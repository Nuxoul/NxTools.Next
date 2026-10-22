using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveSourceBindingTests
{
    private static void Require(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit actual worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-source-binding-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<(ArchiveWorkerEvent Terminal, JsonElement[] Frames)> Run(ArchiveWorkerRequest request,
        Action<ArchiveWorkerRunner>? setup = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        setup?.Invoke(runner);
        try {
            Require(runner.Start(request), "source fixture admission"); var clock = Stopwatch.StartNew();
            while (!events.Any(value => value.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 30000, "worker deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync(), "source fixture cleanup");
            return (events.Single(value => value.Kind == "exited"), events.Where(value => value.Frame is not null).Select(value => value.Frame!.Value).ToArray());
        }
        finally { runner.Stop(); Require(await runner.DrainAsync(), "finally cleanup"); }
    }
    private static void Verify(string path, IReadOnlyDictionary<string, byte[]> expected)
    {
        using var zip = ZipFile.OpenRead(path);
        Require(zip.Entries.Select(entry => entry.FullName).Order().SequenceEqual(expected.Keys.Order()), "independent names");
        foreach (var entry in zip.Entries) { using var stream = entry.Open(); using var bytes = new MemoryStream(); stream.CopyTo(bytes); Require(bytes.ToArray().SequenceEqual(expected[entry.FullName]), "independent bytes"); }
    }
    public static Task ParentOutput() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input"));
        File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "sibling-payload"); Directory.CreateDirectory(Path.Combine(source.FullName, "empty"));
        foreach (var overwrite in new[] { false, true }) {
            var name = overwrite ? "overwrite.zip" : "created.zip";
            IReadOnlyDictionary<string, string>? confirmed = null;
            if (overwrite) { File.WriteAllText(Path.Combine(owned.FullName, name), "confirmed-old"); using var capture = ArchiveOutputSession.Open(owned.FullName, Guid.NewGuid()); confirmed = new Dictionary<string, string> { [name] = capture.CaptureTargetIdentity(name) }; }
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 }, OutputRoot: owned.FullName,
                CreateName: name, CompressionLevel: "normal", ConfirmedOverwrites: confirmed));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "safe parent output rejected: " + terminal.Category);
            Verify(Path.Combine(owned.FullName, name), new Dictionary<string, byte[]> { ["input/"] = [], ["input/empty/"] = [], ["input/keep.txt"] = "sibling-payload"u8.ToArray() });
            Require(Directory.GetFiles(owned.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "parent partial leak");
        }
    });
    public static Task SelfInclusion() => WithWorker(async owned => {
        foreach (var mode in new[] { "inside", "equal", "multi", "same_file", "hardlink", "junction" }) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, mode)); var source = Directory.CreateDirectory(Path.Combine(local.FullName, "input"));
            var file = Path.Combine(source.FullName, "keep.txt"); File.WriteAllText(file, "protected-source");
            var output = mode == "inside" || mode == "multi" ? Directory.CreateDirectory(Path.Combine(source.FullName, "output")) : mode == "equal" ? source : Directory.CreateDirectory(Path.Combine(local.FullName, "output"));
            var sourcePath = mode is "same_file" or "hardlink" ? file : source.FullName;
            var name = mode == "same_file" ? "keep.txt" : "created.zip";
            if (mode == "same_file") output = source;
            var link = Path.Combine(output.FullName, name);
            if (mode is "hardlink" or "junction") {
                var arguments = mode == "hardlink" ? $"/d /c mklink /H \"{link}\" \"{file}\"" : $"/d /c mklink /J \"{link}\" \"{source.FullName}\"";
                using var process = Process.Start(new ProcessStartInfo("cmd.exe", arguments) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true })!;
                await process.StandardOutput.ReadToEndAsync(); await process.StandardError.ReadToEndAsync(); await process.WaitForExitAsync(); Require(process.ExitCode == 0, "owned alias creation");
            }
            var first = Path.Combine(local.FullName, "other.txt"); File.WriteAllText(first, "other");
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            try {
                var accepted = runner.Start(new(Guid.NewGuid(), "create_zip", mode == "multi" ? first : sourcePath, 0, new { max_entries = 100 }, OutputRoot: output.FullName,
                    CreateName: name, CompressionLevel: "normal", SourcePaths: mode == "multi" ? new[] { first, sourcePath } : null));
                if (accepted) {
                    var clock = Stopwatch.StartNew(); while (!events.Any(value => value.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 30000, "collision deadline"); await Task.Delay(20); }
                    Require(await runner.DrainAsync(), "collision cleanup"); var terminal = events.Single(value => value.Kind == "exited");
                    Require(terminal.FilesCommitted == 0 && terminal.Category != "success" && terminal.CleanupComplete, "self inclusion published: " + mode);
                    if (mode is "inside" or "equal" or "multi" or "hardlink") Require(events.Single(value => value.Frame?.GetProperty("op").GetString() == "failure").Frame!.Value.GetProperty("error_code").GetString() == "OutputCollision", "collision typing: " + mode);
                } else Require(mode == "same_file" && events.Single().Category == "source_output_collision", "unexpected collision admission");
                Require(File.ReadAllText(file) == "protected-source" && Directory.GetFiles(output.FullName, ".nxpart-*", SearchOption.TopDirectoryOnly).Length == 0, "source/partial ownership");
            }
            finally { runner.Stop(); Require(await runner.DrainAsync(), "collision finally cleanup"); if (mode == "junction" && Directory.Exists(link)) Directory.Delete(link); }
        }
    });
    public static Task NamespaceReplacement() => WithWorker(async owned => {
        foreach (var nested in new[] { false, true }) {
            var source = Directory.CreateDirectory(Path.Combine(owned.FullName, nested ? "nested-source" : "source"));
            var target = nested ? Directory.CreateDirectory(Path.Combine(source.FullName, "child")) : source;
            File.WriteAllText(Path.Combine(target.FullName, "common.txt"), "approved-common");
            File.WriteAllText(Path.Combine(target.FullName, "must-not-disappear.txt"), "approved-only");
            var substitute = Directory.CreateDirectory(Path.Combine(owned.FullName, nested ? "nested-substitute" : "substitute"));
            File.WriteAllText(Path.Combine(substitute.FullName, "common.txt"), "namespace-other");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, nested ? "nested-output" : "output")); var injected = false;
            var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 },
                OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal"), runner => {
                    runner.SourceNamespacePathProbe = path => {
                        if (nested && !path.EndsWith("\\child", StringComparison.Ordinal)) return path;
                        injected = true; return "\\\\?\\" + substitute.FullName;
                    };
                });
            Require(injected, "namespace replacement seam not exercised");
            Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && terminal.Category == "worker_failure", "namespace mismatch silently published omitted source members");
            Require(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "SourceChanged", "namespace identity mismatch typing");
            Require(!File.Exists(Path.Combine(output.FullName, "created.zip")) && Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "namespace failure leaked output");
        }
    });
    public static Task HeldOutputNamespace() => WithWorker(async owned => {
        var source = Path.Combine(owned.FullName, "source.txt"); File.WriteAllText(source, "protected-original");
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "actual-output"));
        var substitute = Directory.CreateDirectory(Path.Combine(owned.FullName, "substitute-output"));
        var final = Path.Combine(output.FullName, "created.zip");
        using (var link = Process.Start(new ProcessStartInfo("cmd.exe", $"/d /c mklink /H \"{final}\" \"{source}\"") { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true })!) {
            await link.StandardOutput.ReadToEndAsync(); await link.StandardError.ReadToEndAsync(); await link.WaitForExitAsync(); Require(link.ExitCode == 0, "owned output alias");
        }
        var confirmed = new Dictionary<string, string>();
        using (var capture = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid())) confirmed["created.zip"] = capture.CaptureTargetIdentity("created.zip");
        var injected = false;
        var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName,
            CreateName: "created.zip", CompressionLevel: "normal", ConfirmedOverwrites: confirmed), runner => {
                runner.SourceOutputNamespacePathProbe = _ => { injected = true; return substitute.FullName; };
            });
        Require(injected && terminal.FilesCommitted == 0 && terminal.CleanupComplete, "held output alias published");
        Require(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "OutputCollision", "collision inspected substituted namespace instead of held output");
        Require(File.ReadAllText(final) == "protected-original" && File.ReadAllText(source) == "protected-original", "source alias altered");
        Require(Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "held output partial leaked");
    });
    private static string[] Members(string root) => Directory.GetFileSystemEntries(root, "*", SearchOption.AllDirectories)
        .Select(path => Path.GetRelativePath(root, path).Replace('\\', '/') + (Directory.Exists(path) ? "/" : ":" +
            Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(path)))))
        .Order(StringComparer.Ordinal).ToArray();

    public static Task MissingInternalOutput() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input"));
        File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "protected-source");
        Directory.CreateDirectory(Path.Combine(source.FullName, "empty"));
        var output = Path.Combine(source.FullName, "new-output", "nested");
        var before = Members(source.FullName);
        Require(!Directory.Exists(Path.Combine(source.FullName, "new-output")) && !Directory.Exists(output), "output must initially be absent");
        var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 },
            OutputRoot: output, CreateName: "created.zip", CompressionLevel: "normal"));
        var failure = frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString();
        var after = Members(source.FullName);
        Console.WriteLine($"PREPARE internal before=[{string.Join(",", before)}] after=[{string.Join(",", after)}] typed={failure} committed={terminal.FilesCommitted} cleanup={terminal.CleanupComplete} output_created={Directory.Exists(output)}");
        Require(failure == "OutputCollision" && terminal.FilesCommitted == 0 && terminal.CleanupComplete, "internal output typed rejection/cleanup");
        Require(!File.Exists(Path.Combine(output, "created.zip")) && Directory.GetFiles(source.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "internal output archive/partial leak");
        Require(before.SequenceEqual(after) && !Directory.Exists(Path.Combine(source.FullName, "new-output")), "rejected request changed source members");
    });
    public static Task MissingInternalMultiAndIdentity() => WithWorker(async owned => {
        foreach (var mode in new[] { "multi", "held_identity" }) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
            var source = Directory.CreateDirectory(Path.Combine(local.FullName, "input"));
            File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "protected-source");
            var other = Path.Combine(local.FullName, "other.txt"); File.WriteAllText(other, "other-source");
            var output = Path.Combine(source.FullName, "new-output", "nested");
            var before = Members(source.FullName);
            Require(!Directory.Exists(Path.Combine(source.FullName, "new-output")), "multi output must initially be absent");
            var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", other, 0, new { max_entries = 100 },
                OutputRoot: output, CreateName: "created.zip", CompressionLevel: "normal", SourcePaths: [other, source.FullName]), runner => {
                    if (mode == "held_identity") runner.SourceOutputNamespacePathProbe = _ => Path.Combine(local.FullName, "harmless-missing");
                });
            Require(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "OutputCollision", "multi/held ancestor collision typing");
            Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && before.SequenceEqual(Members(source.FullName)) &&
                !Directory.Exists(Path.Combine(source.FullName, "new-output")) && File.ReadAllText(other) == "other-source", "multi/identity rejection changed sources");
            Require(!File.Exists(Path.Combine(output, "created.zip")) && Directory.GetFiles(local.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "multi/identity partial leak");
        }
    });

    public static Task ExistingInternalUnchanged() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input"));
        var output = Directory.CreateDirectory(Path.Combine(source.FullName, "output"));
        File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "protected-source");
        File.WriteAllText(Path.Combine(output.FullName, "foreign.txt"), "preexisting-output-content");
        var before = Members(source.FullName);
        var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 },
            OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal"));
        Require(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "OutputCollision", "existing internal collision typing");
        Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && before.SequenceEqual(Members(source.FullName)), "existing internal content changed");
    });

    public static Task MissingExternalOutput() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input"));
        File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "external-output-payload");
        Directory.CreateDirectory(Path.Combine(source.FullName, "empty"));
        var before = Members(source.FullName);
        var output = Path.Combine(owned.FullName, "new-output", "nested");
        Require(!Directory.Exists(Path.Combine(owned.FullName, "new-output")), "external output must initially be absent");
        var (terminal, _) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 },
            OutputRoot: output, CreateName: "created.zip", CompressionLevel: "normal"));
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "legal missing output failed: " + terminal.Category);
        Verify(Path.Combine(output, "created.zip"), new Dictionary<string, byte[]> { ["input/"] = [], ["input/empty/"] = [], ["input/keep.txt"] = "external-output-payload"u8.ToArray() });
        Require(before.SequenceEqual(Members(source.FullName)) && Directory.GetFiles(owned.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "external output changed source/partial leak");
    });

    public static Task PreparationFailures() => WithWorker(async owned => {
        foreach (var mode in new[] { "cancel_before", "denied", "source_changed", "cancel_after_create", "raced_component" }) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
            var source = Directory.CreateDirectory(Path.Combine(local.FullName, "input"));
            File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "protected-source");
            var before = Members(source.FullName);
            var first = Path.Combine(local.FullName, "new-output"); var output = Path.Combine(first, "nested");
            Require(!Directory.Exists(first), "failure output must initially be absent");
            var id = Guid.NewGuid(); var checkpoints = 0; var replaceDenied = false;
            var (terminal, frames) = await Run(new(id, "create_zip", source.FullName, 0, new { max_entries = 100 },
                OutputRoot: output, CreateName: "created.zip", CompressionLevel: "normal"), runner => {
                    runner.SourceCheckpointProbe = stage => {
                        if (stage != "creation_output_preparing") return;
                        checkpoints++;
                        if (mode == "cancel_before" || mode == "cancel_after_create" && checkpoints == 2) {
                            if (checkpoints == 2) File.WriteAllText(Path.Combine(first, "foreign.txt"), "external-added-content");
                            Require(runner.Cancel(id), "preparation cancellation");
                        }
                        if (mode == "denied") throw new ArchiveOutputException("source_unavailable", 5);
                        if (mode == "raced_component" && checkpoints == 1) File.WriteAllText(first, "external-raced-file");
                        if (mode == "source_changed") {
                            try { Directory.Move(source.FullName, source.FullName + "-replaced"); }
                            catch (IOException) { replaceDenied = true; }
                            File.WriteAllText(Path.Combine(source.FullName, "external.txt"), "external-source-change");
                        }
                    };
                });
            Require(checkpoints > 0 && terminal.Category != "success" && terminal.FilesCommitted == 0 && terminal.CleanupComplete, "preparation failure published/leaked: " + mode);
            if (mode == "source_changed") {
                Require(replaceDenied && File.ReadAllText(Path.Combine(source.FullName, "keep.txt")) == "protected-source" &&
                    File.ReadAllText(Path.Combine(source.FullName, "external.txt")) == "external-source-change", "source lease/change ownership");
                Require(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "SourceChanged", "preparation source change typing");
            } else Require(before.SequenceEqual(Members(source.FullName)), "preparation changed protected source: " + mode);
            if (mode == "cancel_after_create") Require(Directory.Exists(first) && !Directory.Exists(output) &&
                File.ReadAllText(Path.Combine(first, "foreign.txt")) == "external-added-content", "cleanup deleted external directory/content");
            else Require(!Directory.Exists(first), "precreation failure created directory: " + mode);
            if (mode == "raced_component") Require(File.ReadAllText(first) == "external-raced-file", "raced output component deleted");
            Require(!File.Exists(Path.Combine(output, "created.zip")) && Directory.GetFiles(local.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "failure final/partial leak");
        }
    });
}
