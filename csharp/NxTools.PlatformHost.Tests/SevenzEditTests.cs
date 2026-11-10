using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: strict DTO/capability validation only; native fixtures below require an explicitly
// frozen matching worker and operate exclusively in GUID-owned session scratch directories.
internal static class SevenzEditTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static ArchiveBrowserContext Context(string format = "SevenZ") =>
        new(Guid.NewGuid(), format, "", new("v1:synthetic-wire-source", new uint[32], 100, 0));
    private static ArchiveJobRequest Edit(string action, ulong[]? ordinals = null, string? value = null, string[]? paths = null, string format = "SevenZ") =>
        new() { Kind = "modify", Paths = paths ?? [@"C:\owned\editable.7z"], Modify = new(Context(format), action, ordinals ?? [], value) };
    private static ArchiveJobCommand Start(ArchiveJobRequest request) =>
        new() { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    private static ArchiveJobCommand Parse(ArchiveJobRequest request) => ArchiveJobWire.Parse(Wire(Start(request)));
    private static void Reject(ArchiveJobRequest request)
    {
        try { Parse(request); }
        catch (Exception failure) when (failure is InvalidDataException or JsonException) { return; }
        throw new InvalidOperationException("invalid edit shape accepted");
    }

    internal static Task OrdinarySevenzReplaceAccepted()
    {
        var request = Edit("replace_content", [7], paths: [@"C:\owned\editable.7z", @"C:\owned\replacement.txt"]);
        var parsed = Parse(request).Request!;
        Check(parsed.Modify is { Action: "replace_content", Value: null } && parsed.Modify.Browser.Format == "SevenZ" &&
            parsed.Modify.Ordinals.SequenceEqual(new ulong[] { 7 }) && parsed.Paths.SequenceEqual(request.Paths), "strict SevenZ replacement roundtrip");
        return Task.CompletedTask;
    }

    internal static Task StrictShapesPreserveZipAndBounds()
    {
        foreach (var format in new[] { "SevenZ", "Zip" })
        {
            Parse(Edit("add", paths: [@"C:\owned\editable.7z", @"C:\owned\input"], format: format));
            Parse(Edit("add", value: "nested/", paths: [@"C:\owned\editable.7z", @"C:\owned\input"], format: format));
            Parse(Edit("delete", [1, 2], format: format));
            Parse(Edit("rename", [1], "renamed.txt", format: format));
            Parse(Edit("rename", [1], "renamed/", format: format));
            Parse(Edit("new_directory", value: "new/", format: format));
            if (format == "SevenZ") {
                var implicitRename = Edit("rename", [1], "目录/");
                var withPrefix = implicitRename with { Modify = implicitRename.Modify! with { SourcePrefix = "a/" } };
                Check(Parse(withPrefix).Request!.Modify!.SourcePrefix == "a/", "implicit source prefix survives strict wire");
                Reject(withPrefix with { Modify = withPrefix.Modify! with { SourcePrefix = "../a/" } });
            }
            Reject(Edit("delete", [1, 1], format: format));
            Reject(Edit("delete", [1], "ignored", format: format));
            Reject(Edit("rename", [1, 2], "renamed.txt", format: format));
            Reject(Edit("rename", [1], "../unsafe", format: format));
            Reject(Edit("new_directory", value: "not-a-directory", format: format));
            Reject(Edit("add", [1], paths: [@"C:\owned\editable.7z", @"C:\owned\input"], format: format));
        }
        Parse(Edit("comment", value: "ZIP comment", format: "Zip"));
        Reject(Edit("comment", value: "not7z"));
        Reject(Edit("replace_content", [1], paths: [@"C:\owned\editable.zip", @"C:\owned\input"], format: "Zip"));
        Reject(Edit("replace_content", [1]));
        Reject(Edit("replace_content", [], paths: [@"C:\owned\editable.7z", @"C:\owned\input"]));
        Reject(Edit("replace_content", [1, 2], paths: [@"C:\owned\editable.7z", @"C:\owned\input"]));
        Reject(Edit("replace_content", [1], "forbidden", [@"C:\owned\editable.7z", @"C:\owned\input"]));
        Reject(Edit("replace_content", [1], paths: [@"C:\owned\editable.7z", @"C:\owned\input", @"C:\owned\extra"]));
        Reject(Edit("delete", [1], paths: [@"C:\owned\editable.7z", @"C:\owned\extra"]));
        Reject(Edit("delete", [1], format: "TarBz2"));
        Parse(Edit("add", paths: Enumerable.Range(0, 32).Select(i => @"C:\owned\input" + i).ToArray()));
        Reject(Edit("add", paths: Enumerable.Range(0, 33).Select(i => @"C:\owned\input" + i).ToArray()));
        var command = Wire(Start(Edit("replace_content", [1], paths: [@"C:\owned\editable.7z", @"C:\owned\input"])));
        var node = JsonSerializer.SerializeToNode(command, JsonDefaults.Options)!;
        node["payload"]!["data"]!["request"]!["modify"]!["target_path"] = @"C:\unauthorized";
        try { ArchiveJobWire.Parse(node.Deserialize<ProtocolEnvelope>(JsonDefaults.Options)!); }
        catch (JsonException) { return Task.CompletedTask; }
        throw new InvalidOperationException("unknown target authority accepted");
    }
    internal static Task AdmissionSnapshotAndCapabilities()
    {
        var paths = new List<string> { @"C:\owned\replacement.txt" };
        var ordinals = new ulong[] { 3 };
        var digest = new uint[32];
        var edit = new ArchiveJobModify(Context() with { Source = Context().Source with { Digest = digest } }, "replace_content", ordinals, null);
        var request = new ArchiveWorkerRequest(Guid.NewGuid(), "modify_7z", @"C:\owned\editable.7z", 0,
            new ArchiveAdmittedLimits(100, 4UL << 20, 128UL << 20, 1UL << 20, 4UL << 20), OutputRoot: @"C:\owned",
            SourcePaths: paths, JobExecution: true, JobModify: edit, PasswordInteraction: true, ConflictPolicy: "ask");
        var admitted = ArchiveWorkerAdmission.Capture(request);
        paths[0] = @"C:\unauthorized\replacement.txt"; ordinals[0] = 9; digest[0] = 255;
        Check(admitted.SourcePaths!.Single() == @"C:\owned\replacement.txt" && admitted.JobModify!.Ordinals.Single() == 3 &&
            admitted.JobModify.Browser.Source.Digest[0] == 0, "owned immutable edit admission");
        void Refuse(ArchiveWorkerRequest invalid)
        {
            try { ArchiveWorkerAdmission.Capture(invalid); }
            catch (ArchiveOutputException failure) when (failure.Stage == "invalid_edit_request") { return; }
            throw new InvalidOperationException("invalid private edit admission accepted");
        }
        Refuse(admitted with { JobExecution = false });
        Refuse(admitted with { Operation = "modify_zip" });
        Refuse(admitted with { SourcePaths = null });
        Refuse(admitted with { SourcePaths = [@"C:\owned\a", @"C:\owned\b"] });
        Refuse(admitted with { JobModify = admitted.JobModify! with { Action = "delete" } });
        Refuse(admitted with { JobModify = admitted.JobModify! with { Action = "add", Ordinals = [] }, SourcePaths = Enumerable.Range(0, 32).Select(i => @"C:\owned\input" + i).ToArray() });
        var capabilities = new[] { "source_lease_v1", "budget_reservations_v1", "partial_read_v1", "source_tree_v1", "sevenz_modify_v1" };
        Check(ArchiveWorkerRunner.SupportsCapabilities("modify_7z", JsonSerializer.SerializeToElement(capabilities)), "7z capabilities");
        foreach (var required in capabilities)
            Check(!ArchiveWorkerRunner.SupportsCapabilities("modify_7z", JsonSerializer.SerializeToElement(capabilities.Where(c => c != required))), "missing required edit capability");
        Check(!ArchiveWorkerRunner.SupportsCapabilities("modify_zip", JsonSerializer.SerializeToElement(capabilities)), "7z capability cannot substitute ZIP edit capability");
        Check(ArchiveWorkerRunner.SupportsCapabilities("modify_zip", JsonSerializer.SerializeToElement(capabilities.Select(c => c == "sevenz_modify_v1" ? "zip_modify_v1" : c))), "ZIP named capability retained");
        return Task.CompletedTask;
    }

    private sealed class CaptureLog : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        internal readonly System.Collections.Concurrent.ConcurrentQueue<string> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Records.Enqueue(JsonSerializer.Serialize(new { eventName, correlationId, fields }));
    }
    private sealed class NativeFixture
    {
        internal readonly string Root;
        internal readonly string Archive;
        internal readonly CaptureLog Log = new();
        private int sequence, oracleSequence;
        internal NativeFixture(string label)
        {
            var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
            var expected = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_SEVENZ_EDIT_WORKER_SHA256");
            var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
            var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
            var oracle = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE");
            if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || expected is not { Length: 64 })
                throw new TestSkippedException("Explicit frozen SevenZ-edit worker path and SHA256 marker required.");
            Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected, StringComparison.OrdinalIgnoreCase), "frozen worker identity mismatch");
            if (string.IsNullOrEmpty(python) || !File.Exists(python) || string.IsNullOrEmpty(fixtures) || !File.Exists(Path.Combine(fixtures, "verify-created-tree.py")) || string.IsNullOrEmpty(oracle) || !File.Exists(oracle))
                throw new TestSkippedException("Explicit existing independent Python/tree/7Zip oracle required.");
            var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Scratch required");
            Root = Path.Combine(Path.GetTempPath(), "archive-product-" + Guid.NewGuid().ToString());
            Directory.CreateDirectory(Root);
            File.Copy(worker, Path.Combine(Root, ArchiveReadHelper.DeploymentName));
            Directory.CreateDirectory(Path.Combine(Root, "archive"));
            Directory.CreateDirectory(Path.Combine(Root, "evidence"));
            Archive = Path.Combine(Root, "archive", "editable.7z");
            File.WriteAllText(Path.Combine(Root, "evidence", "worker.sha256"), expected);
        }
        internal string Input(string name, string content)
        {
            var path = Path.Combine(Root, "inputs", name);
            Directory.CreateDirectory(Path.GetDirectoryName(path)!); File.WriteAllText(path, content, new UTF8Encoding(false)); return path;
        }
        internal ArchiveJobSession Jobs() => ArchiveJobSession.ForOwnedTestPackage(Root, Log);
        internal async Task Seed(ArchiveJobSession jobs, bool solid = false, string? password = null, bool header = false)
        {
            var seed = Path.Combine(Root, "seed"); Directory.CreateDirectory(Path.Combine(seed, "folder"));
            File.WriteAllText(Path.Combine(seed, "keep.txt"), "kept-data", new UTF8Encoding(false));
            File.WriteAllText(Path.Combine(seed, "folder", "child.txt"), "child-data", new UTF8Encoding(false));
            Published(await Run(jobs, new() { Kind = "create", Paths = [seed], Destination = Path.GetDirectoryName(Archive), Create = new() {
                Format = "seven_z", Name = "editable.7z", Level = "fast", Solid = solid, EncryptHeader = header,
                Password = password is null ? null : Encoding.UTF8.GetBytes(password), StripSingleRoot = true, ExcludePatterns = [], TestAfterCreate = false } }, password));
        }
        internal async Task<ArchiveJobTerminal> Run(ArchiveJobSession jobs, ArchiveJobRequest request, string? password = null, bool conflict = false, Action<ArchiveJobCommand>? beforeDispatch = null, Action? afterTerminal = null, string conflictDecision = "overwrite", bool applyToAll = false, int? expectedConflictPrompts = null, bool sourceRefusal = false)
        {
            var index = ++sequence;
            if (request.Kind == "modify") {
                File.Copy(Archive, Path.Combine(Root, "evidence", index + ".before.7z"));
                File.WriteAllText(Path.Combine(Root, "evidence", index + ".before.sha256"), Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Archive))));
            }
            var command = Start(request); beforeDispatch?.Invoke(command);
            var admission = jobs.Dispatch(Wire(command));
            Check(sourceRefusal ? !admission.Success && admission.Error == "source_unavailable" : admission.Success, "native job admission");
            var prompts = 0; var passwordPrompts = 0; var conflictPrompts = 0;
            var phases = new List<string>();
            while (true)
            {
                var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                if (item.Progress is { } progress && (phases.Count == 0 || phases[^1] != progress.Phase)) phases.Add(progress.Phase);
                if (item.Prompt is { } prompt)
                {
                    Check(++prompts <= 4, "bounded fixture prompt count");
                    Check(prompt.Kind == "password" && password is not null || prompt.Kind == "conflict" && conflict, "unexpected edit prompt");
                    if(prompt.Kind=="password")passwordPrompts++;else conflictPrompts++;
                    var reply = new ArchiveJobReply { Ticket = prompt.Ticket, Kind = prompt.Kind, ApplyToAll = prompt.Kind == "conflict" && applyToAll,
                        Password = prompt.Kind == "password" ? Encoding.UTF8.GetBytes(password!) : null, Decision = prompt.Kind == "conflict" ? conflictDecision : null };
                    Check(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = reply })).Success, "original correlated interaction reply");
                }
                if (item.Terminal is not { } terminal) continue;
                afterTerminal?.Invoke();
                if (expectedConflictPrompts is { } expectedCount) Check(conflictPrompts == expectedCount, "exact original conflict prompt count");
                if(request.Kind=="modify" && terminal.FilesCommitted > 0 && (terminal.Outcome is "succeeded" or "succeeded_with_warnings"))Check(terminal.Journal is {Count:1} && terminal.Journal[0].Replaced,"same-path edit replacement journal");
                if (request.Kind == "modify" && terminal.FilesCommitted == 1 && terminal.Outcome == "succeeded")
                    Check(new[] { "edit_planning", "edit_reading", "edit_reencoding", "edit_verifying", "edit_committing" }.All(phases.Contains), "actual worker phases cross original task event route: " + string.Join(",", phases));
                if(request.Kind=="modify" && (terminal.Outcome is "succeeded" or "succeeded_with_warnings"))Check((password is null || passwordPrompts>0) && (!conflict || conflictPrompts>0),"expected original private interaction observed");
                File.WriteAllText(Path.Combine(Root, "evidence", index + ".terminal.json"), JsonSerializer.Serialize(terminal, JsonDefaults.Options));
                File.WriteAllText(Path.Combine(Root, "evidence", index + ".after.sha256"), Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(Archive))));
                Check(Log.Records.All(line => !line.Contains(Root) && (password is null || !line.Contains(password))), "production diagnostic privacy");
                File.WriteAllLines(Path.Combine(Root, "evidence", "diagnostics.jsonl"), Log.Records);
                return terminal;
            }
        }
        internal async Task<(ArchiveBrowserContext Context, ArchiveReadEntry[] Entries)> Browse(string? password = null)
        {
            await using var reads = ArchiveReadSession.ForOwnedTestPackage(Root, Log);
            var wire = ArchiveProductReadTests.Command(path: Archive, version: 2, limits: new() { BrowserContext = true });
            var command = ArchiveReadWire.Parse(wire); var entries = new List<ArchiveReadEntry>();
            Check(reads.Dispatch(wire, browserInteraction: password is not null).Success, "original browser admission");
            while (true)
            {
                var envelope = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                if (envelope.Payload.GetProperty("event").GetString() == "archive_password")
                {
                    var prompt = envelope.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!;
                    Check(password is not null && reads.RespondPassword(prompt, password), "original browser password route"); continue;
                }
                var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) { entries.AddRange(batch.Entries); Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: 2)).Success, "original metadata credit"); }
                if (item.Terminal is not { } terminal) continue;
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser?.Format == "SevenZ", "native browser terminal "+terminal.Status+":"+terminal.Code+":"+terminal.NativeErrorCode);
                return (terminal.Browser!, entries.ToArray());
            }
        }
        internal ArchiveJobRequest Modify(ArchiveBrowserContext context, string action, ulong[]? ids = null, string? value = null, string[]? additions = null, string? sourcePrefix = null) =>
            new() { Kind = "modify", Paths = new[] { Archive }.Concat(additions ?? []).ToArray(), Modify = new(context, action, ids ?? [], value, sourcePrefix) };
        internal void Unchanged(byte[] before, ArchiveJobTerminal terminal)
        {
            Check(terminal.FilesCommitted == 0 && terminal.CleanupComplete && File.ReadAllBytes(Archive).SequenceEqual(before), "precommit refusal changed original or leaked resources");
            Check(!Directory.GetFiles(Path.GetDirectoryName(Archive)!, ".nxpart-*").Any(), "unpublished owned partial retained");
        }
        internal async Task Oracle(Dictionary<string, byte[]> expected, string? password = null, bool header = false)
        {
            if (expected.Count == 0)
            {
                var emptyStart = new ProcessStartInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE")!) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
                foreach (var arg in new[] { "l", "-slt", Archive }) emptyStart.ArgumentList.Add(arg);
                using var empty = Process.Start(emptyStart)!; var stdout = empty.StandardOutput.ReadToEndAsync(); var stderr = empty.StandardError.ReadToEndAsync();
                await empty.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); var output = (await stdout).Replace("\r\n", "\n"); await stderr;
                File.WriteAllText(Path.Combine(Root, "evidence", "oracle-" + ++oracleSequence + ".json"), JsonSerializer.Serialize(new { exit = empty.ExitCode, expected_entries = 0 }));
                Check(empty.ExitCode == 0 && !output.Split("----------\n").Skip(1).Any(section => section.Contains("Path = ")), "independent empty 7z listing");
                return;
            }
            var start = new ProcessStartInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE")!) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
            start.ArgumentList.Add(Path.Combine(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES")!, "verify-created-tree.py"));
            using var child = Process.Start(start)!;
            var manifest = expected.ToDictionary(pair => pair.Key, pair => new { size = pair.Value.Length, sha256 = Convert.ToHexString(SHA256.HashData(pair.Value)).ToLowerInvariant() });
            await child.StandardInput.WriteAsync(JsonSerializer.Serialize(new { path = Archive, entries = manifest, format = "7z", password, encrypt_header = header })); child.StandardInput.Close();
            var result = child.StandardOutput.ReadToEndAsync(); var failure = child.StandardError.ReadToEndAsync();
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(60)); await failure;
            File.WriteAllText(Path.Combine(Root, "evidence", "oracle-" + ++oracleSequence + ".json"), JsonSerializer.Serialize(new { exit = child.ExitCode, expected_entries = expected.Count }));
            Check(child.ExitCode == 0, "independent exact 7z content/encryption oracle");
            using var parsed = JsonDocument.Parse(await result); Check(parsed.RootElement.GetProperty("verified_entries").GetInt32() == expected.Count, "independent entry count");
        }
    }
    private static void Published(ArchiveJobTerminal terminal) => Check(terminal.Outcome == "succeeded" && terminal.FilesCommitted == 1 &&
        terminal.CleanupComplete && terminal.Journal is { Count: 1 } && terminal.Journal[0] is { State: "Committed", Published: true }, "actual native publication: " + terminal.Code);
    private static Dictionary<string, byte[]> SeedManifest() => new() { ["keep.txt"] = Encoding.UTF8.GetBytes("kept-data"), ["folder/"] = [], ["folder/child.txt"] = Encoding.UTF8.GetBytes("child-data") };

    internal static async Task ActualActionsAndEmptyResult()
    {
        foreach (var solid in new[] { false, true })
        {
            var fixture = new NativeFixture(solid ? "solid-actions" : "plain-actions"); await using var jobs = fixture.Jobs();
            await fixture.Seed(jobs, solid); var expected = SeedManifest(); await fixture.Oracle(expected);
            var current = await fixture.Browse();
            Published(await fixture.Run(jobs, fixture.Modify(current.Context, "rename", [current.Entries.Single(e => e.Name == "folder/" || e.Name == "folder").Ordinal], "renamed/")));
            expected.Remove("folder/"); expected.Remove("folder/child.txt"); expected["renamed/"] = []; expected["renamed/child.txt"] = Encoding.UTF8.GetBytes("child-data"); await fixture.Oracle(expected);
            current = await fixture.Browse(); var replacement = fixture.Input("replacement.bin", "replacement-content");
            Published(await fixture.Run(jobs, fixture.Modify(current.Context, "replace_content", [current.Entries.Single(e => e.Name == "keep.txt").Ordinal], additions: [replacement])));
            expected["keep.txt"] = Encoding.UTF8.GetBytes("replacement-content"); await fixture.Oracle(expected);
            current = await fixture.Browse(); var addition = fixture.Input("added.txt", "added-content");
            Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [addition])));
            expected["added.txt"] = Encoding.UTF8.GetBytes("added-content"); await fixture.Oracle(expected);
            current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "new_directory", value: "empty/")));
            expected["empty/"] = []; await fixture.Oracle(expected);
            current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "rename", [current.Entries.Single(e => e.Name == "added.txt").Ordinal], "renamed.txt")));
            expected["renamed.txt"] = expected["added.txt"]; expected.Remove("added.txt"); await fixture.Oracle(expected);
            current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "delete", [current.Entries.Single(e => e.Name == "renamed/" || e.Name == "renamed").Ordinal])));
            expected.Remove("renamed/"); expected.Remove("renamed/child.txt"); await fixture.Oracle(expected);
            current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "delete", current.Entries.Select(e => e.Ordinal).ToArray())));
            Check((await fixture.Browse()).Entries.Length == 0, "all-delete publishes valid empty archive"); await fixture.Oracle(new());
        }
    }

    internal static async Task ActualEncryptedReplacementAndConflict()
    {
        foreach (var header in new[] { false, true })
        {
            var fixture = new NativeFixture(header ? "header-encrypted" : "aes"); await using var jobs = fixture.Jobs();
            var password = "synthetic-seven-edit-" + Guid.NewGuid().ToString("N");
            await fixture.Seed(jobs, true, password, header); var expected = SeedManifest(); await fixture.Oracle(expected, password, header);
            var current = await fixture.Browse(password); var replacement = fixture.Input("replacement.txt", "encrypted-replacement");
            Published(await fixture.Run(jobs, fixture.Modify(current.Context, "replace_content", [current.Entries.Single(e => e.Name == "keep.txt").Ordinal], additions: [replacement]), password));
            expected["keep.txt"] = Encoding.UTF8.GetBytes("encrypted-replacement"); await fixture.Oracle(expected, password, header);
            current = await fixture.Browse(password); var collision = fixture.Input("keep.txt", "explicit-conflict-replacement");
            Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [collision]), password, conflict: true));
            expected["keep.txt"] = Encoding.UTF8.GetBytes("explicit-conflict-replacement"); await fixture.Oracle(expected, password, header);
        }
    }

    internal static async Task ActualSourceCancellationAndHandoff()
    {
        var fixture = new NativeFixture("safety"); await using var jobs = fixture.Jobs(); await fixture.Seed(jobs);
        var current = await fixture.Browse(); var directory = Path.Combine(fixture.Root, "replacement-directory"); Directory.CreateDirectory(directory);
        var before = File.ReadAllBytes(fixture.Archive);
        var invalid = await fixture.Run(jobs, fixture.Modify(current.Context, "replace_content", [current.Entries.Single(e => e.Name == "keep.txt").Ordinal], additions: [directory]));
        Check(invalid.Outcome == "failed", "directory is not replacement content"); fixture.Unchanged(before, invalid);
        FileStream? writer = null;
        try {
            var refused = await fixture.Run(jobs, fixture.Modify(current.Context, "delete", [current.Entries.First().Ordinal]),
                beforeDispatch: _ => writer = new FileStream(fixture.Archive, FileMode.Open, FileAccess.ReadWrite, FileShare.ReadWrite),
                afterTerminal: () => { writer?.Dispose(); writer = null; }, sourceRefusal: true);
            Check(refused.Outcome == "rejected" && refused.Code == "source_unavailable" && refused.NativeErrorCode is 32 or 33, "held writable source refused at reservation"); fixture.Unchanged(before, refused);
        } finally { writer?.Dispose(); }
        var stale = current.Context;
        Published(await fixture.Run(jobs, fixture.Modify(current.Context, "rename", [current.Entries.Single(e => e.Name == "keep.txt").Ordinal], "renamed.txt")));
        before = File.ReadAllBytes(fixture.Archive);
        var changed = await fixture.Run(jobs, fixture.Modify(stale, "delete", [current.Entries.First().Ordinal]));
        Check(changed.Code == "source_changed", "stale source revision refused"); fixture.Unchanged(before, changed);
        current = await fixture.Browse(); var request = fixture.Modify(current.Context, "new_directory", value: "cancelled/");
        var observed = false;
        var cancelled = await fixture.Run(jobs, request, beforeDispatch: command => jobs.ProbeOutputCommitForFixture(stage => {
            if (stage != "edit_source_rechecked") return;
            observed = true;
            try { using var write = File.Open(fixture.Archive, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source write protection lost before handoff"); } catch (IOException) { }
            try { Directory.Move(Path.GetDirectoryName(fixture.Archive)!, Path.Combine(fixture.Root, "moved-parent")); throw new InvalidOperationException("parent protection lost before handoff"); } catch (IOException) { }
            Check(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original prehandoff cancel");
        }));
        Check(observed && cancelled.Outcome == "cancelled", "prehandoff cancellation reached original boundary"); fixture.Unchanged(before, cancelled);
        jobs.ProbeOutputCommitForFixture(_ => { });
        current = await fixture.Browse(); request = fixture.Modify(current.Context, "new_directory", value: "unknown/");
        var nativeCalls = 0;
        jobs.ProbeOutputCommitForFixture(stage => { if (stage == "commit_in_flight") { nativeCalls++; throw new ArchiveOutputException("replace_file", 64); } });
        var unknown = await fixture.Run(jobs, request);
        Check(nativeCalls == 1 && unknown.Code == "commit_outcome_unknown" && unknown.FilesCommitted == 0 && unknown.CommitState == "CommitOutcomeUnknown" &&
            unknown.Journal is { Count: 1 } && unknown.Journal[0].Published is null && File.ReadAllBytes(fixture.Archive).SequenceEqual(before), "unknown commit evidence retained without retry");
        Check(Directory.GetFiles(Path.GetDirectoryName(fixture.Archive)!, ".nxpart-*").Length == 1, "unknown candidate retained as evidence");
    }

    // All oracle processes are test-only, use the explicit independent 7-Zip executable,
    // and never receive a secret through argv (encrypted cases use Oracle's stdin route).
    private static async Task<Dictionary<string, string>> IndependentMetadata(NativeFixture fixture, string member)
    {
        var start = new ProcessStartInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE")!) {
            UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var arg in new[] { "l", "-slt", fixture.Archive }) start.ArgumentList.Add(arg);
        using var child = Process.Start(start)!;
        var stdout = child.StandardOutput.ReadToEndAsync(); var stderr = child.StandardError.ReadToEndAsync();
        await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30));
        var output = await stdout; var error = await stderr;
        Check(child.ExitCode == 0, "independent metadata listing: " + child.ExitCode + ":" + error);
        var fields = new List<Dictionary<string, string>>();
        var current = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var line in output.Replace("\r\n", "\n").Split('\n')) {
            if (line.StartsWith("Path = ", StringComparison.Ordinal) && current.ContainsKey("Path")) { fields.Add(current); current = new(StringComparer.Ordinal); }
            var delimiter = line.IndexOf(" = ", StringComparison.Ordinal);
            if (delimiter > 0) current[line[..delimiter]] = line[(delimiter + 3)..];
        }
        if (current.Count > 0) fields.Add(current);
        return fields.Single(record => record.GetValueOrDefault("Path") == member);
    }

    internal static async Task ActualEmptyInputsSubtreeImplicitAndNames()
    {
        var fixture = new NativeFixture("empty-implicit-names"); await using var jobs = fixture.Jobs();
        await fixture.Seed(jobs); var expected = SeedManifest();
        var empty = fixture.Input("zero.txt", "");
        var current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [empty])));
        expected["zero.txt"] = []; await fixture.Oracle(expected);
        current = await fixture.Browse(); var replacement = fixture.Input("zero-replacement.txt", "");
        Published(await fixture.Run(jobs, fixture.Modify(current.Context, "replace_content", [current.Entries.Single(e => e.Name == "keep.txt").Ordinal], additions: [replacement])));
        expected["keep.txt"] = []; await fixture.Oracle(expected);
        var tree = Path.Combine(fixture.Root, "inputs", "a"); Directory.CreateDirectory(Path.Combine(tree, "b"));
        File.WriteAllText(Path.Combine(tree, "b", "c.txt"), "implicit-leaf", new UTF8Encoding(false));
        Directory.CreateDirectory(Path.Combine(tree, "empty"));
        current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [tree])));
        expected["a/"] = []; expected["a/b/"] = []; expected["a/b/c.txt"] = Encoding.UTF8.GetBytes("implicit-leaf"); expected["a/empty/"] = [];
        await fixture.Oracle(expected);
        var peer = fixture.Input("ab/peer.txt", "separate-prefix");
        current = await fixture.Browse(); Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [Path.GetDirectoryName(peer)!])));
        expected["ab/"] = []; expected["ab/peer.txt"] = Encoding.UTF8.GetBytes("separate-prefix"); await fixture.Oracle(expected);
        current = await fixture.Browse(); var implicitLeaf = current.Entries.Single(e => e.Name == "a/b/c.txt");
        Published(await fixture.Run(jobs, fixture.Modify(current.Context, "rename", [implicitLeaf.Ordinal], "隐式/", sourcePrefix: "a/")));
        foreach (var name in expected.Keys.Where(name => name.StartsWith("a/", StringComparison.Ordinal)).ToArray()) {
            expected["隐式/" + name[2..]] = expected[name]; expected.Remove(name);
        }
        await fixture.Oracle(expected);
        current = await fixture.Browse(); var directory = current.Entries.Single(e => e.Name == "隐式/" || e.Name == "隐式");
        Published(await fixture.Run(jobs, fixture.Modify(current.Context, "rename", [directory.Ordinal], "目录/")));
        foreach (var name in expected.Keys.Where(name => name.StartsWith("隐式/", StringComparison.Ordinal)).ToArray()) {
            expected["目录/" + name[3..]] = expected[name]; expected.Remove(name);
        }
        await fixture.Oracle(expected);
        current = await fixture.Browse();
        Check(current.Entries.Any(e => e.Name == "ab/peer.txt") && current.Entries.Any(e => e.Name == "目录/b/c.txt") &&
            !current.Entries.Any(e => e.Name.StartsWith("a/", StringComparison.Ordinal) || e.Name.StartsWith("隐式/", StringComparison.Ordinal)), "a versus ab, source_prefix and Unicode rename boundaries");
    }

    internal static async Task ActualIndependentMetadataRetention()
    {
        var fixture = new NativeFixture("metadata"); await using var jobs = fixture.Jobs();
        var root = Path.Combine(fixture.Root, "independent"); Directory.CreateDirectory(root);
        var source = Path.Combine(root, "keep.txt"); File.WriteAllText(source, "metadata-payload", new UTF8Encoding(false));
        var created = new DateTime(2020, 2, 3, 4, 5, 6, DateTimeKind.Utc);
        var accessed = new DateTime(2021, 3, 4, 5, 6, 7, DateTimeKind.Utc);
        var modified = new DateTime(2022, 4, 5, 6, 7, 8, DateTimeKind.Utc);
        File.SetCreationTimeUtc(source, created); File.SetLastAccessTimeUtc(source, accessed); File.SetLastWriteTimeUtc(source, modified);
        File.SetAttributes(source, FileAttributes.ReadOnly | FileAttributes.Archive);
        var start = new ProcessStartInfo(Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE")!) {
            WorkingDirectory = root, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var arg in new[] { "a", "-t7z", "-mx=1", "-mtc=on", "-mta=on", "-mtm=on", fixture.Archive, "keep.txt" }) start.ArgumentList.Add(arg);
        using (var child = Process.Start(start)!) {
            var stdout = child.StandardOutput.ReadToEndAsync(); var stderr = child.StandardError.ReadToEndAsync();
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(30)); await stdout; var error = await stderr;
            Check(child.ExitCode == 0, "independent 7z metadata seed: " + child.ExitCode + ":" + error);
        }
        var baseline = await IndependentMetadata(fixture, "keep.txt");
        foreach (var (field, value) in new[] { ("Created", created), ("Accessed", accessed), ("Modified", modified) })
            Check(baseline.TryGetValue(field, out var actual) && actual.StartsWith(value.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss", System.Globalization.CultureInfo.InvariantCulture), StringComparison.Ordinal), "independent seed " + field);
        Check(baseline.TryGetValue("Attributes", out var attributes) && attributes.Contains('R'), "independent seed read-only attribute");
        var expected = new Dictionary<string, byte[]> { ["keep.txt"] = Encoding.UTF8.GetBytes("metadata-payload") };
        await fixture.Oracle(expected);
        var current = await fixture.Browse(); var addition = fixture.Input("extra.txt", "new");
        Published(await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [addition])));
        expected["extra.txt"] = Encoding.UTF8.GetBytes("new"); await fixture.Oracle(expected);
        var after = await IndependentMetadata(fixture, "keep.txt");
        foreach (var field in new[] { "Created", "Accessed", "Modified", "Attributes" })
            Check(after.GetValueOrDefault(field) == baseline[field], "existing member metadata changed: " + field);
    }

    internal static async Task ActualConflictWarningsAndNativeCommitTruth()
    {
        var fixture = new NativeFixture("conflict-native"); await using var jobs = fixture.Jobs(); await fixture.Seed(jobs);
        var expected = SeedManifest(); var same = fixture.Input("keep.txt", "rejected-collision");
        var other = fixture.Input("folder/child.txt", "other-collision");
        var current = await fixture.Browse();
        var skipped = await fixture.Run(jobs, fixture.Modify(current.Context, "add", additions: [same, Path.GetDirectoryName(other)!]), conflict: true, conflictDecision: "skip", applyToAll: true, expectedConflictPrompts: 1);
        Check(skipped.Outcome == "succeeded_with_warnings" && skipped.FilesSkipped == 2 && skipped.FilesCommitted == 0 && skipped.CleanupComplete, "all-skipped add truthful warning counts");
        await fixture.Oracle(expected);
        current = await fixture.Browse(); var before = File.ReadAllBytes(fixture.Archive);
        var nativeCalls = 0;
        jobs.ProbeOutputCommitForFixture(stage => { if (stage == "commit_in_flight") { nativeCalls++; throw new ArchiveOutputException("replace_file", 5); } });
        var failed = await fixture.Run(jobs, fixture.Modify(current.Context, "new_directory", value: "denied/"));
        Check(nativeCalls == 1 && failed.Outcome == "failed" && failed.FilesCommitted == 0 && failed.NativeErrorCode == 5 && failed.CommitState == "FailedBeforeCommit", "native denied edit truth, no retry");
        fixture.Unchanged(before, failed);
        jobs.ProbeOutputCommitForFixture(_ => { });
        current = await fixture.Browse(); var late = false;
        var commandRequest = fixture.Modify(current.Context, "new_directory", value: "published/");
        var terminal = await fixture.Run(jobs, commandRequest, beforeDispatch: command => jobs.ProbeOutputCommitForFixture(stage => {
            if (stage != "native_published") return;
            late = true; Check(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "late native cancel accepted");
        }));
        Check(late && terminal.FilesCommitted == 1 && terminal.Journal is { Count: 1 } && terminal.Journal[0].Published == true &&
            (terminal.Outcome == "succeeded" || terminal.Outcome == "cancelled" && terminal.Code == "cancelled_with_committed_output"), "published output cannot become removable on late cancel");
        expected["published/"] = []; await fixture.Oracle(expected);
    }
}
