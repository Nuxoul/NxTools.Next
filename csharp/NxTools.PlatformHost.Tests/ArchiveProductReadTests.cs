using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveProductReadTests
{
    private static void Check(bool result, string detail) { if (!result) throw new InvalidOperationException(detail); }
    internal static ProtocolEnvelope Command(string action = "open", Guid? operation = null, Guid? session = null,
        ulong generation = 1, string? path = null, ArchiveReadLimits? limits = null, ulong? sequence = null, uint version = 1) {
        var id = operation ?? Guid.NewGuid();
        return ProtocolEnvelope.Create("platform_command", new { command = "archive_read", data = new ArchiveReadCommand {
            Version = version, OperationId = id, SessionId = session ?? Guid.NewGuid(), Generation = generation,
            Action = action, Path = action is "open" or "test" ? path ?? @"C:\synthetic.zip" : null, Limits = limits, Sequence = sequence } }, id);
    }
    public static Task WireValidation()
    {
        var valid = Command();
        var command = ArchiveReadWire.Parse(valid);
        Check(command.Version == 1 && new ArchiveReadLimits().MaxWorkingBytes == 67108864, "exact defaults");
        foreach (var mutation in new Action<JsonObject>[] {
            d => d.Remove("version"), d => d["generation"] = 0, d => d["password"] = "never-authorized",
            d => d["action"] = "extract", d => d["path"] = "relative.zip", d => d["codepage"] = 42,
            d => d["limits"] = new JsonObject { ["max_retained_bytes"] = 4194305 },
            d => d["limits"] = new JsonObject { ["max_entries"] = 0 },
            d => d["limits"] = new JsonObject { ["max_working_bytes"] = 67108863 },
            d => d["limits"] = new JsonObject { ["unknown"] = 1 }, d => d["sequence"] = 0 }) {
            var payload = JsonNode.Parse(valid.Payload.GetRawText())!.AsObject(); mutation(payload["data"]!.AsObject());
            var invalid = valid with { Payload = JsonSerializer.SerializeToElement(payload) };
            try { ArchiveReadWire.Parse(invalid); throw new InvalidOperationException("invalid command accepted"); }
            catch (Exception failure) when (failure is InvalidDataException or JsonException) { }
        }
        Check(ArchiveReadWire.Parse(Command("ack", sequence: 0)).Sequence == 0, "ack zero");
        var receipt = ArchiveReadWire.Result(new(true, null));
        Check(receipt.Success && JsonDocument.Parse(receipt.Data!).RootElement.GetProperty("accepted").GetBoolean(), "independent receipt");
        return Task.CompletedTask;
    }
    public static Task MetadataValidation()
    {
        var id = Guid.NewGuid();
        JsonElement Batch(ulong sequence = 0, ulong ordinal = 0, string provenance = "Utf8Flag", uint codepage = 65001) =>
            JsonSerializer.SerializeToElement(new { snapshot_id = id, batch_sequence = sequence, format = "Zip",
                entries = new[] { new ArchiveReadEntry(ordinal, "literal.txt", "File", 3, 3, false, [], provenance, codepage) } }, JsonDefaults.Options);
        var metadata = new ArchiveReadMetadata(new(), 0);
        var first = metadata.Read(Batch());
        Check(first.SnapshotId == id && first.Entries[0].Name == "literal.txt", "literal snapshot/entry");
        foreach (var bad in new[] { Batch(), Batch(2, 1), Batch(1, 9), Batch(1, 1, "unknown"), Batch(1, 1, "Utf8Flag", 437) }) {
            try { metadata.Read(bad); throw new InvalidOperationException("invalid metadata accepted"); }
            catch (InvalidDataException) { }
        }
        foreach (var cp in new uint[] { 65001, 936, 932 }) {
            Check(new ArchiveReadMetadata(new(), 0).Read(Batch(provenance: "Heuristic", codepage: cp)).Entries[0].Codepage == cp, "heuristic candidate codepage");
        }
        foreach (var (selected, cp) in new (uint, uint)[] { (0, 437), (0, 950), (936, 936) }) {
            try { new ArchiveReadMetadata(new(), selected).Read(Batch(provenance: "Heuristic", codepage: cp)); throw new InvalidOperationException("invalid heuristic metadata accepted"); }
            catch (InvalidDataException) { }
        }
        var tiny = new ArchiveReadMetadata(new() { MaxRetainedBytes = 1 }, 0);
        try { tiny.Read(Batch()); throw new InvalidOperationException("metadata byte ceiling not enforced"); }
        catch (ArchiveReadBudgetException) { }
        Check(metadata.Complete(JsonSerializer.SerializeToElement(new { files_verified = 1, entry_count = 1, batch_count = 1 }), true), "complete counts");
        Check(!metadata.Complete(JsonSerializer.SerializeToElement(new { files_verified = 2, entry_count = 2, batch_count = 1 }), true), "incomplete counts");
        return Task.CompletedTask;
    }
    internal static string Root() => Directory.CreateDirectory(Path.Combine(Path.GetTempPath(), "archive-product-" + Guid.NewGuid())).FullName;
    public static Task HelperProtection()
    {
        var root = Root();
        try {
            var helper = ArchiveReadHelper.OwnedPackage(root);
            Check(helper.Executable == Path.Combine(root, "NxTools.ArchiveWorker.exe"), "fixed deployment sibling");
            try { using var missing = helper.Acquire(); throw new InvalidOperationException("missing accepted"); }
            catch (ArchiveOutputException failure) { Check(failure.Stage == "helper_missing", "missing category"); }
            File.WriteAllBytes(helper.Executable, [1, 2, 3]);
            using (helper.Acquire()) {
                try { File.Delete(helper.Executable); throw new InvalidOperationException("helper delete allowed during lease"); }
                catch (IOException) { }
                try { using var write = File.OpenWrite(helper.Executable); throw new InvalidOperationException("helper write allowed during lease"); }
                catch (IOException) { }
            }
            File.Delete(helper.Executable);
            var outside = Path.GetDirectoryName(root)!;
            try { ArchiveReadHelper.OwnedPackage(outside); throw new InvalidOperationException("outside accepted"); }
            catch (ArgumentException) { }
            var target = Path.Combine(root, "real"); Directory.CreateDirectory(target); File.WriteAllBytes(Path.Combine(target, ArchiveReadHelper.DeploymentName), [1]);
            var junction = Path.Combine(root, "linked");
            var start = new System.Diagnostics.ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{junction}\" \"{target}\"")
                { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
            using (var process = System.Diagnostics.Process.Start(start)!) {
                process.StandardOutput.ReadToEnd(); process.StandardError.ReadToEnd();
                Check(process.WaitForExit(5000) && process.ExitCode == 0, "owned junction creation");
            }
            var linkedRoot = Path.Combine(junction, "archive-product-" + Guid.NewGuid()); Directory.CreateDirectory(linkedRoot);
            File.WriteAllBytes(Path.Combine(linkedRoot, ArchiveReadHelper.DeploymentName), [1]);
            try { using var rejected = ArchiveReadHelper.OwnedPackage(linkedRoot).Acquire(); throw new InvalidOperationException("reparse accepted"); }
            catch (ArchiveOutputException failure) { Check(failure.Stage == "helper_untrusted", "reparse category"); }
            Directory.Delete(junction);
            return Task.CompletedTask;
        } finally { Directory.Delete(root, true); }
    }
}
