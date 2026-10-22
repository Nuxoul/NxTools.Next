using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Bounded GUID-owned sources/output; explicitly supplied built worker and independent oracles only.
internal static class ArchiveTreeCreationTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit actual Rust worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-tree-create-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<(ArchiveWorkerEvent Terminal, JsonElement[] Frames)> Run(ArchiveWorkerRequest request, Action<ArchiveWorkerRunner>? setup = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue); setup?.Invoke(runner);
        Require(runner.Start(request), "tree creation admission: " + string.Join(",", events.Select(e => e.Category)));
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 60000, "tree create deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "tree create cleanup");
        return (events.Single(e => e.Kind == "exited"), events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray());
    }
    private static async Task Independent(string path, IReadOnlyDictionary<string, byte[]> expected, string format, string? password = null, bool? solid = null, bool? encryptHeader = null)
    {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit Python/tree oracle required.");
        var start = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add(Path.Combine(fixtures, "verify-created-tree.py")); using var process = Process.Start(start)!;
        var manifest = expected.ToDictionary(pair => pair.Key, pair => new { size = pair.Value.Length, sha256 = Convert.ToHexString(SHA256.HashData(pair.Value)).ToLowerInvariant() });
        await process.StandardInput.WriteAsync(JsonSerializer.Serialize(new { path, entries = manifest, format, password, solid, encrypt_header = encryptHeader })); process.StandardInput.Close();
        var stdout = process.StandardOutput.ReadToEndAsync(); var stderr = process.StandardError.ReadToEndAsync();
        using var deadline = new CancellationTokenSource(60000); await process.WaitForExitAsync(deadline.Token);
        Require(process.ExitCode == 0 && (await stderr).Length == 0, "independent tree oracle failed: " + await stderr);
        using var result = JsonDocument.Parse(await stdout); Require(result.RootElement.GetProperty("verified_entries").GetInt32() == expected.Count, "independent exact tree count");
    }
    private static async Task ReadProduced(string path, IReadOnlyDictionary<string, byte[]> expected, string? password = null) {
        foreach (var operation in new[] { "open", "test", "extract" }) {
            var destination = operation == "extract" ? Path.Combine(Path.GetDirectoryName(path)!, "decoded-" + Guid.NewGuid().ToString("N")) : null;
            var (terminal, frames) = await Run(new(Guid.NewGuid(), operation, path, 0, new { max_entries = 100 }, Password: password, OutputRoot: destination));
            Require(terminal.Category == "success" && terminal.CleanupComplete, "created archive read lifecycle: " + operation + "/" + terminal.Category);
            var complete = frames.Single(frame => frame.GetProperty("op").GetString() == "complete");
            if (operation == "open") Require(frames.Where(frame => frame.GetProperty("op").GetString() == "entry_batch").Sum(frame => frame.GetProperty("entries").GetArrayLength()) == expected.Count, "created archive listed count");
            else Require(complete.GetProperty("files_verified").GetUInt64() == (ulong)expected.Keys.Count(name => !name.EndsWith('/')) && complete.GetProperty("bytes_verified").GetUInt64() == (ulong)expected.Values.Sum(bytes => bytes.Length), "created archive verified counts");
            if (destination is not null) {
                var actual = Directory.GetFileSystemEntries(destination, "*", SearchOption.AllDirectories).Select(item => Path.GetRelativePath(destination, item).Replace('\\', '/') + (Directory.Exists(item) ? "/" : "")).Order().ToArray();
                Require(actual.SequenceEqual(expected.Keys.Order()), "created archive exact native extraction names/kinds");
                foreach (var pair in expected.Where(pair => !pair.Key.EndsWith('/'))) Require(File.ReadAllBytes(Path.Combine(destination, pair.Key)).SequenceEqual(pair.Value), "created archive exact native extraction bytes");
            }
        }
    }
    private static (string Source, Dictionary<string, byte[]> Expected) Source(DirectoryInfo owned)
    {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "资料"));
        Directory.CreateDirectory(Path.Combine(source.FullName, "空目录"));
        var first = Enumerable.Range(0, 131072).Select(i => (byte)(i * 31)).ToArray();
        var second = Encoding.UTF8.GetBytes("independent tree source 合成\n");
        File.WriteAllBytes(Path.Combine(source.FullName, "内容.bin"), first);
        File.WriteAllBytes(Path.Combine(source.FullName, "second.txt"), second);
        File.WriteAllBytes(Path.Combine(source.FullName, "empty.bin"), []);
        File.WriteAllText(Path.Combine(source.FullName, "Thumbs.db"), "excluded");
        return (source.FullName, new() { ["资料/"] = [], ["资料/空目录/"] = [], ["资料/内容.bin"] = first, ["资料/second.txt"] = second, ["资料/empty.bin"] = [] });
    }
    public static Task TarXz() => WithWorker(async owned => {
        var (source, expected) = Source(owned);
        foreach (var level in new[] { "fast", "normal", "maximum" }) {
            var output = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"));
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_txz", source, 0, new { max_entries = 100 }, OutputRoot: output, CreateName: "created.txz", CompressionLevel: level));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "TXZ terminal: " + terminal.Category);
            await Independent(Path.Combine(output, "created.txz"), expected, "txz");
            if (level == "normal") await ReadProduced(Path.Combine(output, "created.txz"), expected);
            Require(Directory.GetFiles(output, ".nxpart-*").Length == 0, "TXZ partial leak");
        }
    });
    public static Task TarGzip() => WithWorker(async owned => {
        var (source, expected) = Source(owned);
        foreach (var level in new[] { "fast", "normal", "maximum" }) {
            var output = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"));
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_tgz", source, 0, new { max_entries = 100 }, OutputRoot: output, CreateName: "created.tgz", CompressionLevel: level));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "TGZ terminal: " + terminal.Category);
            await Independent(Path.Combine(output, "created.tgz"), expected, "tgz");
            if (level == "normal") await ReadProduced(Path.Combine(output, "created.tgz"), expected);
            Require(Directory.GetFiles(output, ".nxpart-*").Length == 0, "TGZ partial leak");
        }
    });
    public static Task TarPlain() => WithWorker(async owned => {
        var (source, expected) = Source(owned); var output = Path.Combine(owned.FullName, "missing", "nested");
        var (terminal, _) = await Run(new(Guid.NewGuid(), "create_tar", source, 0, new { max_entries = 100 }, OutputRoot: output, CreateName: "created.tar"));
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "TAR terminal: " + terminal.Category);
        await Independent(Path.Combine(output, "created.tar"), expected, "tar");
        await ReadProduced(Path.Combine(output, "created.tar"), expected);
        Require(Directory.GetFiles(output, ".nxpart-*").Length == 0, "TAR partial leak");
    });
    public static Task SevenzPlain() => WithWorker(async owned => {
        var (source, expected) = Source(owned); var output = Path.Combine(owned.FullName, "missing", "nested");
        Require(!Directory.Exists(output), "tree output must initially be absent");
        var (terminal, _) = await Run(new(Guid.NewGuid(), "create_7z", source, 0, new { max_entries = 100 }, OutputRoot: output, CreateName: "created.7z", CompressionLevel: "normal"));
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed" && terminal.CleanupComplete, "7z creation terminal: " + terminal.Category);
        await Independent(Path.Combine(output, "created.7z"), expected, "7z", solid: false, encryptHeader: false);
        await ReadProduced(Path.Combine(output, "created.7z"), expected);
        Require(Directory.GetFiles(output, ".nxpart-*").Length == 0, "tree partial leak");
    });
    public static Task SevenzOptions() => WithWorker(async owned => {
        var (source, original) = Source(owned);
        var expected = original.Where(pair => pair.Key != "资料/").ToDictionary(pair => pair.Key[3..], pair => pair.Value);
        foreach (var level in new[] { "fast", "normal", "maximum" })
        foreach (var solid in new[] { false, true })
        foreach (var encryption in new[] { "none", "aes256", "header" }) {
            var output = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"));
            var password = encryption == "none" ? null : "synthetic-tree-secret";
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_7z", source, 0, new { max_entries = 100 }, Password: password,
                OutputRoot: output, CreateName: "created.7z", CompressionLevel: level, TreeOptions: new(StripSingleRoot: true),
                SevenzOptions: new(solid, password is null ? "none" : "aes256", encryption == "header")));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "7z options terminal: " + level + "/" + solid + "/" + encryption + "/" + terminal.Category);
            await Independent(Path.Combine(output, "created.7z"), expected, "7z", password, solid, encryption == "header");
            if (level == "normal") await ReadProduced(Path.Combine(output, "created.7z"), expected, password);
            Require(Directory.GetFiles(output, ".nxpart-*").Length == 0, "7z options partial leak");
        }
    });
    public static Task SevenzSafety() => WithWorker(owned => Safety(owned, "7z"));
    public static Task TarSafety() => WithWorker(async owned => {
        foreach (var format in new[] { "tar", "tgz", "txz" }) await Safety(owned, format);
    });
    private static async Task Safety(DirectoryInfo owned, string format) {
        foreach (var mode in new[] { "internal", "output", "cancel", "changed", "existing", "overwrite" }.Concat(format is "7z" or "txz" ? new[] { "work" } : []).Concat(format == "7z" ? [] : new[] { "overhead" })) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var (source, expected) = Source(local); var output = mode == "internal" ? Path.Combine(source, "missing", "nested") : Path.Combine(local.FullName, "output");
            var name = "created." + format;
            var before = Directory.GetFileSystemEntries(source, "*", SearchOption.AllDirectories).Order().ToArray();
            var existing = mode is "existing" or "overwrite"; if (existing) { Directory.CreateDirectory(output); File.WriteAllText(Path.Combine(output, name), "original"); }
            IReadOnlyDictionary<string, string>? confirmed = null;
            if (mode == "overwrite") { using var session = ArchiveOutputSession.Open(output, Guid.NewGuid()); confirmed = new Dictionary<string, string> { [name] = session.CaptureTargetIdentity(name) }; }
            var id = Guid.NewGuid();
            var (terminal, frames) = await Run(new(id, "create_" + format, source, 0, new { max_entries = 100, max_working_bytes = mode == "work" ? 67108864 : 536870912, max_output_bytes = mode == "output" ? 1 : mode == "overhead" ? expected.Values.Sum(bytes => bytes.Length) : 1048576 },
                OutputRoot: output, CreateName: name, CompressionLevel: format == "tar" ? null : "normal", ConfirmedOverwrites: confirmed), runner => {
                runner.SourceCheckpointProbe = stage => {
                    if (stage == "source_precommit" && mode == "cancel") Require(runner.Cancel(id), "7z cancel");
                    if (stage == "source_precommit" && mode == "changed") File.WriteAllText(Path.Combine(source, "second.txt"), "changed");
                };
            });
            Require(terminal.CleanupComplete && terminal.FilesCommitted == (mode == "overwrite" ? 1 : 0), "7z safety commit/cleanup: " + mode);
            Require(terminal.Category == (mode == "overwrite" ? "success" : mode is "work" or "output" or "overhead" ? "resource_limit" : mode == "cancel" ? "cancelled" : "worker_failure"), "tree safety category: " + format + "/" + mode + "/" + terminal.Category);
            if (mode is "internal" or "changed" or "existing") Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == (mode == "changed" ? "SourceChanged" : "OutputCollision"), "7z safety typed error");
            if (mode == "internal") Require(!Directory.Exists(Path.Combine(source, "missing")) && Directory.GetFileSystemEntries(source, "*", SearchOption.AllDirectories).Order().SequenceEqual(before), "7z internal request changed source");
            if (mode == "overwrite") await Independent(Path.Combine(output, name), expected, format);
            else Require(existing ? File.ReadAllText(Path.Combine(output, name)) == "original" : !File.Exists(Path.Combine(output, name)), "tree failure changed target");
            Require(!Directory.Exists(output) || Directory.GetFiles(output, ".nxpart-*").Length == 0, "7z safety partial leak");
        }
    }
    public static Task LayoutAndAdmission() => WithWorker(async owned => {
        foreach (var format in new[] { "7z", "tar", "tgz", "txz" }) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var (source, original) = Source(local); var longName = new string('长', 45) + ".bin";
            File.WriteAllBytes(Path.Combine(source, longName), "long UTF8 payload"u8.ToArray());
            original["资料/" + longName] = "long UTF8 payload"u8.ToArray();
            var other = Path.Combine(local.FullName, "other.bin"); File.WriteAllBytes(other, "second root"u8.ToArray()); original["other.bin"] = "second root"u8.ToArray();
            var sources = new List<string> { source, other }; var exclusions = new List<string> { "Thumbs.db", "desktop.ini" };
            var output = Path.Combine(local.FullName, "output"); var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = ArchiveWorkerRunner.ForPackage(e => {
                events.Enqueue(e); if (e.Kind == "accepted") { sources.Clear(); sources.Add(other); exclusions.Add("*"); }
            });
            Require(runner.Start(new(Guid.NewGuid(), "create_" + format, source, 0, new { max_entries = 100 }, OutputRoot: output, CreateName: "created." + format,
                CompressionLevel: format == "tar" ? null : "normal", SourcePaths: sources, TreeOptions: new(true, exclusions))), "tree immutable admission");
            var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 60000, "tree layout deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync() && events.Single(e => e.Kind == "exited").Category == "success", "tree layout terminal: " + format + "/" + events.Single(e => e.Kind == "exited").Category + "/" + string.Join(",", events.Where(e => e.Frame is { } frame && frame.TryGetProperty("error_code", out _)).Select(e => e.Frame!.Value.GetProperty("error_code").GetString())));
            await Independent(Path.Combine(output, "created." + format), original, format);
            var empty = Directory.CreateDirectory(Path.Combine(local.FullName, "empty")); var emptyOutput = Path.Combine(local.FullName, "empty-output");
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_" + format, empty.FullName, 0, new { max_entries = 100 }, OutputRoot: emptyOutput, CreateName: "empty." + format,
                CompressionLevel: format == "tar" ? null : "normal", TreeOptions: new(true)));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "empty tree terminal: " + format);
            await Independent(Path.Combine(emptyOutput, "empty." + format), new Dictionary<string, byte[]>(), format);
            var limitedOutput = Path.Combine(local.FullName, "empty-limited");
            var (limited, _) = await Run(new(Guid.NewGuid(), "create_" + format, empty.FullName, 0, new { max_entries = 100, max_output_bytes = 16 }, OutputRoot: limitedOutput, CreateName: "limited." + format,
                CompressionLevel: format == "tar" ? null : "normal", TreeOptions: new(true)));
            Require(limited.Category == "resource_limit" && limited.FilesCommitted == 0 && limited.CleanupComplete && !File.Exists(Path.Combine(limitedOutput, "limited." + format)), "empty tree output budget typing: " + format + "/" + limited.Category);
        }
    });
    private sealed class CaptureLog : NxTools.PlatformHost.Diagnostics.IPlatformLog {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) => Records.Enqueue((name, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    public static Task OptionsAndDiagnostics() => WithWorker(async owned => {
        var source = Path.Combine(owned.FullName, "PRIVATE_SOURCE_SENTINEL.bin"); File.WriteAllText(source, "PRIVATE_CONTENT_SENTINEL");
        foreach (var format in new[] { "7z", "tar", "tgz", "txz" }) {
            var output = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")); var operation = "create_" + format;
            var valid = new ArchiveWorkerRequest(Guid.NewGuid(), operation, source, 0, new { max_entries = 100 }, OutputRoot: output,
                CreateName: "PRIVATE_OUTPUT_SENTINEL." + format, CompressionLevel: format == "tar" ? null : "normal");
            var rejected = new ConcurrentQueue<ArchiveWorkerEvent>(); using var reject = ArchiveWorkerRunner.ForPackage(rejected.Enqueue);
            foreach (var request in new[] { valid with { ZipOptions = new() }, valid with { Password = "PRIVATE_PASSWORD_SENTINEL" }, valid with { CompressionLevel = "store" }, valid with { SevenzOptions = new(Encryption: "aes128") }, valid with { CreateName = "../bad" }, valid with { SourcePaths = Array.Empty<string>() } })
                Require(!reject.Start(request with { OperationId = Guid.NewGuid() }), "unsupported tree option admitted: " + format);
            Require(reject.ActiveCount == 0 && !Directory.Exists(output) && rejected.All(e => e.Kind == "rejected"), "invalid tree options touched runtime");
            var capability = format == "7z" ? "sevenz_create_v1" : format + "_create_v1";
            var capabilities = new[] { "source_lease_v1", "budget_reservations_v1", "partial_read_v1", "source_tree_v1", capability };
            Require(ArchiveWorkerRunner.SupportsCapabilities(operation, JsonSerializer.SerializeToElement(capabilities)), "tree capabilities");
            foreach (var missing in capabilities) Require(!ArchiveWorkerRunner.SupportsCapabilities(operation, JsonSerializer.SerializeToElement(capabilities.Where(value => value != missing).ToArray())), "tree missing capability accepted");
            var log = new CaptureLog(); var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
            Require(runner.Start(valid), "tree diagnostics admission");
            var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 60000, "tree diagnostics deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync(), "tree diagnostics cleanup"); var terminal = events.Single(e => e.Kind == "exited"); Require(terminal.Category == "success", "tree diagnostics terminal");
            foreach (var phase in new[] { "accepted", "started", "creation_planning", "creation_encoding", "creation_verifying", "creation_committing", "completed" }) Require(log.Records.Count(r => r.Name == "archive.worker." + phase && r.Id == valid.OperationId) == 1, "tree diagnostic phase: " + phase);
            var record = log.Records.Single(r => r.Name == "archive.worker.completed");
            Require(Equals(record.Fields["category"], terminal.Category) && Equals(record.Fields["files_committed"], terminal.FilesCommitted) && Equals(record.Fields["elapsed_ms"], terminal.ElapsedMs), "tree diagnostic terminal projection");
            Require(!JsonSerializer.Serialize(log.Records.Select(r => new { r.Name, r.Id, r.Fields })).Contains("SENTINEL", StringComparison.Ordinal) && log.Records.Count < 30, "tree diagnostic privacy/frequency");
        }
    });
}
