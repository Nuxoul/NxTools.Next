using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only bounded GUID-owned synthetic sources/output and the explicitly built worker.
internal static class ArchiveZipCreationTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit actual Rust worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-zip-create-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<ArchiveWorkerEvent> Finish(ArchiveWorkerRunner runner, ConcurrentQueue<ArchiveWorkerEvent> events, int maximumMs = 20000)
    {
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < maximumMs, "ZIP create deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "ZIP create cleanup"); return events.Single(e => e.Kind == "exited");
    }
    internal static async Task Independent(string path, IReadOnlyDictionary<string, byte[]> expected, int method, string? password = null) {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit Python/ZIP oracle required.");
        var start = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add(Path.Combine(fixtures, "verify-created-zip.py"));
        using var process = Process.Start(start)!;
        var manifest = expected.ToDictionary(pair => pair.Key, pair => new { size = pair.Value.Length, sha256 = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(pair.Value)).ToLowerInvariant() });
        await process.StandardInput.WriteAsync(System.Text.Json.JsonSerializer.Serialize(new { path, entries = manifest, method, password })); process.StandardInput.Close();
        var stdout = process.StandardOutput.ReadToEndAsync(); var stderr = process.StandardError.ReadToEndAsync();
        using var deadline = new CancellationTokenSource(30000); await process.WaitForExitAsync(deadline.Token);
        Require(process.ExitCode == 0 && (await stderr).Length == 0, "independent ZIP oracle failed");
        using var result = System.Text.Json.JsonDocument.Parse(await stdout); Require(result.RootElement.GetProperty("verified_entries").GetInt32() == expected.Count, "independent exact oracle count");
    }
    public static Task PlainSingleFile() => WithWorker(async owned => {
        var source = Path.Combine(owned.FullName, "资料.bin"); var expected = Encoding.UTF8.GetBytes("independent ZIP source 合成\n");
        File.WriteAllBytes(source, expected);
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        Require(runner.Start(new(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal")), "ZIP creation admission");
        var terminal = await Finish(runner, events);
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed" && terminal.CleanupComplete, "ZIP create terminal: " + terminal.Category);
        using var archive = ZipFile.OpenRead(Path.Combine(output.FullName, "created.zip"));
        Require(archive.Entries.Count == 1 && archive.Entries[0].FullName == "资料.bin", "independent ZIP exact names");
        using var payload = archive.Entries[0].Open(); using var bytes = new MemoryStream(); await payload.CopyToAsync(bytes);
        Require(bytes.ToArray().SequenceEqual(expected), "independent ZIP exact bytes");
        Require(Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "ZIP create partial leak");
        await Independent(Path.Combine(output.FullName, "created.zip"), new Dictionary<string, byte[]> { ["资料.bin"] = expected }, 8);
    });
    public static Task DirectoryLevelsAndRoots() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "资料"));
        Directory.CreateDirectory(Path.Combine(source.FullName, "空目录"));
        File.WriteAllBytes(Path.Combine(source.FullName, "空文件.txt"), []);
        File.WriteAllText(Path.Combine(source.FullName, "Thumbs.db"), "excluded");
        var expected = Enumerable.Range(0, 65536).Select(i => (byte)(i * 31)).ToArray();
        File.WriteAllBytes(Path.Combine(source.FullName, "payload.bin"), expected);
        foreach (var level in new[] { "store", "fast", "normal", "maximum" })
        foreach (var strip in new[] { false, true }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            Require(runner.Start(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: level,
                ZipOptions: new(level == "store" ? "store" : "deflate", StripSingleRoot: strip))), "tree ZIP admission");
            var terminal = await Finish(runner, events); Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "tree ZIP terminal: " + terminal.Category);
            using var archive = ZipFile.OpenRead(Path.Combine(output.FullName, "created.zip")); var prefix = strip ? "" : "资料/";
            var expectedNames = new[] { prefix + "空目录/", prefix + "空文件.txt", prefix + "payload.bin" }.Concat(strip ? [] : new[] { "资料/" }).Order().ToArray();
            Require(archive.Entries.Select(e => e.FullName).Order().SequenceEqual(expectedNames), "tree ZIP names/root/exclusions");
            using var payload = archive.GetEntry(prefix + "payload.bin")!.Open(); using var bytes = new MemoryStream(); await payload.CopyToAsync(bytes);
            Require(bytes.ToArray().SequenceEqual(expected), "tree independent content");
            Require(archive.GetEntry(prefix + "空文件.txt")!.Length == 0 && archive.GetEntry(prefix + "空目录/")!.Length == 0, "empty entries");
            var manifest = expectedNames.ToDictionary(name => name, name => name.EndsWith("payload.bin", StringComparison.Ordinal) ? expected : Array.Empty<byte>());
            await Independent(Path.Combine(output.FullName, "created.zip"), manifest, level == "store" ? 0 : 8);
        }
    });
    private static async Task<(ArchiveWorkerEvent Terminal, System.Text.Json.JsonElement[] Frames)> Run(ArchiveWorkerRequest request, Action<ArchiveWorkerRunner>? setup = null) {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue); setup?.Invoke(runner);
        Require(runner.Start(request), "ZIP safety admission"); var terminal = await Finish(runner, events);
        return (terminal, events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray());
    }
    public static Task MultipleSourcesAndCollisions() => WithWorker(async owned => {
        foreach (var mode in new[] { "normal", "case", "prefix" }) {
            var first = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var second = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var a = Path.Combine(first.FullName, "one.bin"); File.WriteAllText(a, "first");
            var b = Path.Combine(second.FullName, mode == "case" ? "ONE.BIN" : mode == "prefix" ? "one.bin" : "two.bin");
            if (mode == "prefix") Directory.CreateDirectory(b); else File.WriteAllText(b, "second");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", a, 0, new { max_entries = 100 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal",
                ZipOptions: new(StripSingleRoot: true), SourcePaths: new[] { a, b }));
            Require(terminal.Category == (mode == "normal" ? "success" : "worker_failure") && terminal.FilesCommitted == (mode == "normal" ? 1 : 0) && terminal.CleanupComplete, "multi terminal: " + mode);
            if (mode == "normal") { using var archive = ZipFile.OpenRead(Path.Combine(output.FullName, "created.zip")); Require(archive.Entries.Select(e => e.FullName).Order().SequenceEqual(new[] { "one.bin", "two.bin" }), "multi exact names"); }
            else Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "OutputCollision", "multi collision typing");
            Require(Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "multi partial leak");
        }
    });
    public static Task SourceChangesBudgetsAndPublication() => WithWorker(async owned => {
        foreach (var mode in new[] { "replace", "delete", "add", "changed_after_read", "count", "metadata", "entry", "output", "cancel", "existing", "overwrite", "cancel_during" }) {
            var source = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var path = Path.Combine(source.FullName, "payload.bin"); File.WriteAllBytes(path, new byte[131072]);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var final = Path.Combine(output.FullName, "created.zip"); var userPartial = Path.Combine(output.FullName, ".nxpart-user"); File.WriteAllText(userPartial, "user partial");
            var existing = mode is "existing" or "overwrite" or "cancel_during"; if (existing) File.WriteAllText(final, "original");
            IReadOnlyDictionary<string, string>? confirmed = null;
            if (mode is "overwrite" or "cancel_during") { using var session = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid()); confirmed = new Dictionary<string, string> { ["created.zip"] = session.CaptureTargetIdentity("created.zip") }; }
            var id = Guid.NewGuid(); var injected = false;
            var (terminal, frames) = await Run(new(id, "create_zip", source.FullName, 0, new { max_entries = mode == "count" ? 1 : 100, max_metadata_bytes = mode == "metadata" ? 1 : 268435456,
                    max_entry_bytes = mode == "entry" ? 1 : 1048576, max_output_bytes = mode == "output" ? 1 : 1048576 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal", ConfirmedOverwrites: confirmed), runner => {
                runner.SourceCheckpointProbe = stage => {
                    if (!injected && (stage == "source_validated" && mode is "replace" or "delete" or "add" || stage == "source_precommit" && mode == "changed_after_read")) {
                        injected = true;
                        if (mode == "replace") { File.Move(path, path + ".old"); File.WriteAllBytes(path, new byte[131072]); }
                        if (mode == "delete") File.Delete(path);
                        if (mode == "add") File.WriteAllText(Path.Combine(source.FullName, "added.txt"), "new");
                        if (mode == "changed_after_read") File.WriteAllBytes(path, Enumerable.Repeat((byte)1, 131072).ToArray());
                    }
                    if (stage == "source_precommit" && mode == "cancel") Require(runner.Cancel(id), "cancel ZIP");
                };
                runner.OutputCommitCheckpointProbe = stage => { if (stage == "commit_in_flight" && mode == "cancel_during") Require(runner.Cancel(id), "cancel commit"); };
            });
            var published = mode is "overwrite" or "cancel_during";
            Require(terminal.FilesCommitted == (published ? 1 : 0) && terminal.CleanupComplete, "safety publication: " + mode + "/" + terminal.Category);
            Require(terminal.Category == (mode == "overwrite" ? "success" : mode == "cancel_during" ? "cancelled_with_committed_output" : mode == "cancel" ? "cancelled" : mode is "count" or "metadata" or "entry" or "output" ? "resource_limit" : "worker_failure"), "safety terminal: " + mode + "/" + terminal.Category);
            if (mode is "replace" or "delete" or "add" or "changed_after_read") Require(injected && frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "SourceChanged", "source mutation typing: " + mode);
            if (published) { using var archive = ZipFile.OpenRead(final); Require(archive.Entries.Count == 2, "published archive validation"); }
            else Require(existing ? File.ReadAllText(final) == "original" : !File.Exists(final), "failure altered target: " + mode);
            Require(Directory.GetFiles(output.FullName, ".nxpart-*").SequenceEqual(new[] { userPartial }) && File.ReadAllText(userPartial) == "user partial", "owned partial isolation: " + mode);
        }
    });
    public static Task AesCreationInterop() => WithWorker(async owned => {
        const string password = "synthetic-ZIP-create-secret";
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "加密资料"));
        var expected = Enumerable.Range(0, 131072).Select(i => (byte)(i * 19)).ToArray();
        File.WriteAllBytes(Path.Combine(source.FullName, "内容.bin"), expected); File.WriteAllBytes(Path.Combine(source.FullName, "empty.bin"), []);
        Directory.CreateDirectory(Path.Combine(source.FullName, "空目录"));
        foreach (var level in new[] { "store", "fast", "normal", "maximum" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var (terminal, _) = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 }, Password: password, OutputRoot: output.FullName,
                CreateName: "created.zip", CompressionLevel: level, ZipOptions: new(level == "store" ? "store" : "deflate", "aes256", true)));
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "AES creation terminal: " + terminal.Category);
            await Independent(Path.Combine(output.FullName, "created.zip"), new Dictionary<string, byte[]> { ["内容.bin"] = expected, ["empty.bin"] = [], ["空目录/"] = [] }, level == "store" ? 0 : 8, password);
            foreach (var supplied in new string?[] { null, "wrong synthetic" }) {
                var (testTerminal, testFrames) = await Run(new(Guid.NewGuid(), "test", Path.Combine(output.FullName, "created.zip"), 0, new { max_entries = 100 }, Password: supplied));
                var failureCode = testFrames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString();
                Require(testTerminal.FilesCommitted == 0 && failureCode == (supplied is null ? "PasswordRequired" : "AuthenticationOrDataError"), "created AES missing/wrong password: " + failureCode);
            }
        }
    });
    public static Task NativeSourceLimitsAndReparse() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "source")); var path = Path.Combine(source.FullName, "file.bin"); File.WriteAllBytes(path, "abcdef"u8.ToArray());
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")); var operation = Guid.NewGuid();
        using (var sources = ArchiveCreationSources.Open(new[] { source.FullName }, output.FullName, "out.zip", 2, 1048576, 1048576, 1048576)) {
            System.Text.Json.JsonElement Call(string action, Guid? token = null, ulong? offset = null, int? length = null) => System.Text.Json.JsonSerializer.SerializeToElement(sources.Execute(operation,
                System.Text.Json.JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action, token, offset, length })));
            var roots = Call("source_roots"); using var rootData = System.Text.Json.JsonDocument.Parse(roots.GetProperty("read_data").GetString()!); var root = rootData.RootElement[0].GetProperty("id").GetGuid();
            var first = Call("source_next", root, 0); Require(first.GetProperty("success").GetBoolean(), "first bounded child");
            using var childData = System.Text.Json.JsonDocument.Parse(first.GetProperty("read_data").GetString()!); var file = childData.RootElement.GetProperty("id").GetGuid();
            var end = Call("source_next", root, 1); Require(end.GetProperty("success").GetBoolean() && end.GetProperty("read_data").GetString() == "null", "exact entry limit must permit EOF");
            Require(Call("source_open", file).GetProperty("success").GetBoolean(), "open protected planned file");
            try { using var write = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source writable while read"); } catch (IOException) { }
            try { File.Move(path, path + ".moved"); throw new InvalidOperationException("source replaceable while read"); } catch (IOException) { }
            Require(!Call("source_open", file).GetProperty("success").GetBoolean(), "only one active file");
            Require(!Call("source_read", file, 0, 65537).GetProperty("success").GetBoolean(), "bounded source read");
            var read = Call("source_read", file, 0, 6); Require(Convert.FromBase64String(read.GetProperty("read_data").GetString()!).SequenceEqual("abcdef"u8.ToArray()), "same protected handle bytes");
            Require(Call("source_close", file).GetProperty("success").GetBoolean(), "close verifies source");
            File.WriteAllText(path, "edited"); Require(!Call("source_check", file).GetProperty("success").GetBoolean(), "closed source change detected");
        }
        var roots32 = Enumerable.Range(0, 32).Select(index => Path.Combine(owned.FullName, "root-" + index + ".bin")).ToArray();
        foreach (var rootPath in roots32) File.WriteAllBytes(rootPath, []);
        using (var maximumRoots = ArchiveCreationSources.Open(roots32, output.FullName, "roots.zip", 32, 1048576, 1048576, 1048576)) {
            var reply = System.Text.Json.JsonSerializer.SerializeToElement(maximumRoots.Execute(operation, System.Text.Json.JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "source_roots" })));
            using var data = System.Text.Json.JsonDocument.Parse(reply.GetProperty("read_data").GetString()!); Require(data.RootElement.GetArrayLength() == 32, "exact source root ceiling");
        }
        try { using var excess = ArchiveCreationSources.Open(roots32.Concat(new[] { path }).ToArray(), output.FullName, "roots.zip", 100, 1048576, 1048576, 1048576); throw new InvalidOperationException("33 source roots admitted"); }
        catch (ArchiveOutputException failure) { Require(failure.Stage == "session_entry_limit", "root budget typing"); }
        foreach (var mode in new[] { "self", "junction", "invalid_glob" }) {
            var local = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))); File.WriteAllText(Path.Combine(local.FullName, "keep.txt"), "source");
            var localOutput = Directory.CreateDirectory(Path.Combine(mode == "self" ? local.FullName : owned.FullName, Guid.NewGuid().ToString("N")));
            var junction = Path.Combine(local.FullName, "loop");
            try {
                if (mode == "junction") { using var link = Process.Start(new ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{junction}\" \"{local.FullName}\"") { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!; await link.StandardOutput.ReadToEndAsync(); await link.StandardError.ReadToEndAsync(); await link.WaitForExitAsync(); Require(link.ExitCode == 0, "owned junction creation"); }
                var (terminal, frames) = await Run(new(Guid.NewGuid(), "create_zip", local.FullName, 0, new { max_entries = 100 }, OutputRoot: localOutput.FullName, CreateName: "out.zip", CompressionLevel: "normal",
                    ZipOptions: new(ExcludePatterns: mode == "invalid_glob" ? new[] { "[" } : null)));
                Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && !File.Exists(Path.Combine(localOutput.FullName, "out.zip")), "unsafe source published: " + mode);
                Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == (mode == "self" ? "OutputCollision" : "UnsafePath"), "unsafe source typing: " + mode);
                Require(Directory.GetFiles(localOutput.FullName, ".nxpart-*").Length == 0, "unsafe owned partial leak");
            } finally { if (Directory.Exists(junction)) Directory.Delete(junction); }
        }
    });
    private sealed class CaptureLog : NxTools.PlatformHost.Diagnostics.IPlatformLog {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new(); internal bool Reject;
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) { if (Reject) throw new InvalidOperationException("synthetic sink"); Records.Enqueue((name, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>()))); }
    }
    public static Task DiagnosticsAndOptions() => WithWorker(async owned => {
        var source = Path.Combine(owned.FullName, "PRIVATE_SOURCE_SENTINEL.bin"); File.WriteAllText(source, "PRIVATE_CONTENT_SENTINEL");
        foreach (var mode in new[] { "success", "cancel", "throw_log" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))); var log = new CaptureLog { Reject = mode == "throw_log" };
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log); var id = Guid.NewGuid();
            runner.SourceCheckpointProbe = stage => { if (mode == "cancel" && stage == "source_precommit") Require(runner.Cancel(id), "diagnostic cancel"); };
            Require(runner.Start(new(id, "create_zip", source, 0, new { max_entries = 100 }, Password: "PRIVATE_PASSWORD_SENTINEL", OutputRoot: output.FullName, CreateName: "PRIVATE_OUTPUT_SENTINEL.zip", CompressionLevel: "normal", ZipOptions: new(Encryption: "aes256"))), "diagnostic create");
            var terminal = await Finish(runner, events); Require(terminal.Category == (mode == "cancel" ? "cancelled" : "success"), "sink changed creation result");
            if (!log.Reject) {
                var record = log.Records.Single(r => r.Name is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed");
                Require(record.Id == id && Equals(record.Fields["category"], terminal.Category) && Equals(record.Fields["commit_state"], terminal.CommitState) && Equals(record.Fields["files_committed"], terminal.FilesCommitted) && Equals(record.Fields["elapsed_ms"], terminal.ElapsedMs), "creation log/terminal projection");
                var text = System.Text.Json.JsonSerializer.Serialize(log.Records.Select(r => new { r.Name, r.Id, r.Fields }));
                Require(!text.Contains("SENTINEL", StringComparison.Ordinal) && !text.Contains(owned.FullName, StringComparison.Ordinal) && log.Records.Count < 30, "creation diagnostic leak/frequency");
                foreach (var phase in new[] { "accepted", "started", "creation_planning", "creation_encoding", "creation_verifying", "creation_committing" }) Require(log.Records.Count(r => r.Name == "archive.worker." + phase) == 1, "creation lifecycle phase: " + phase);
            }
        }
        var rejected = new ConcurrentQueue<ArchiveWorkerEvent>(); using var reject = ArchiveWorkerRunner.ForPackage(rejected.Enqueue);
        var root = Directory.CreateDirectory(Path.Combine(owned.FullName, "rejected"));
        var valid = new ArchiveWorkerRequest(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName, CreateName: "valid.zip", CompressionLevel: "normal");
        foreach (var request in new[] { valid with { OperationId = Guid.NewGuid(), ZipOptions = new(Encryption: "aes128") }, valid with { OperationId = Guid.NewGuid(), ZipOptions = new(Encryption: "aes256") }, valid with { OperationId = Guid.NewGuid(), CompressionLevel = "store" }, valid with { OperationId = Guid.NewGuid(), CreateName = "../escape" }, valid with { OperationId = Guid.NewGuid(), SourcePaths = Array.Empty<string>() }, valid with { OperationId = Guid.NewGuid(), ConfirmedOverwrites = new Dictionary<string,string> { ["other.zip"] = "unrelated" } } }) Require(!reject.Start(request), "invalid ZIP request launched");
        Require(reject.ActiveCount == 0, "rejected ZIP active job");
        Require(!reject.Start(valid with { OperationId = Guid.NewGuid(), TimeoutMs = 600001 }), "unbounded ZIP timeout admitted");
        var capabilities = new[] { "source_lease_v1", "budget_reservations_v1", "partial_read_v1", "source_tree_v1", "zip_create_v1" };
        Require(ArchiveWorkerRunner.SupportsCapabilities("create_zip", System.Text.Json.JsonSerializer.SerializeToElement(capabilities)), "ZIP capabilities exact set");
        foreach (var missing in capabilities) Require(!ArchiveWorkerRunner.SupportsCapabilities("create_zip", System.Text.Json.JsonSerializer.SerializeToElement(capabilities.Where(value => value != missing).ToArray())), "ZIP missing capability admitted");
        Require(ArchiveWorkerRunner.SupportsCapabilities("test", System.Text.Json.JsonSerializer.SerializeToElement(capabilities.Take(2).ToArray())), "legacy schema2 capability compatibility");
        using var bridge = new ArchiveWorkerOutputBridge(valid, null);
        foreach (var name in new[] { "other.zip", "../escape", "sub/valid.zip" }) {
            var reply = System.Text.Json.JsonSerializer.SerializeToElement(bridge.Execute(valid.OperationId, System.Text.Json.JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "begin", path = name }), out _));
            Require(!reply.GetProperty("success").GetBoolean(), "create token escaped exact output name");
        }
        Require(Directory.GetFileSystemEntries(root.FullName).Length == 0, "rejected native create touched output");
    });
    public static Task LargeZip64Streaming() => WithWorker(async owned => {
        const long length = (1L << 32) + 17;
        var source = Path.Combine(owned.FullName, "large.bin");
        using (var file = new FileStream(source, FileMode.CreateNew, FileAccess.Write, FileShare.None)) file.SetLength(length);
        using var expected = System.Security.Cryptography.IncrementalHash.CreateHash(System.Security.Cryptography.HashAlgorithmName.SHA256);
        var block = new byte[65536];
        for (long offset = 0; offset < length; offset += block.Length) expected.AppendData(block, 0, (int)Math.Min(block.Length, length - offset));
        var hash = Convert.ToHexString(expected.GetHashAndReset()).ToLowerInvariant();
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")); var final = Path.Combine(output.FullName, "large.zip");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        Require(runner.Start(new(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 10 }, OutputRoot: output.FullName, CreateName: "large.zip", CompressionLevel: "fast", TimeoutMs: 600000)), "large ZIP admission");
        var terminal = await Finish(runner, events, 610000);
        if (terminal.Category != "success") Require(terminal.FilesCommitted == 0 && !File.Exists(final) && Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "large failed output cleanup");
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CleanupComplete, "large ZIP terminal: " + terminal.Category);
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE"); var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit streaming Python oracle required.");
        var start = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add(Path.Combine(fixtures, "verify-created-zip.py")); using var oracle = Process.Start(start)!;
        await oracle.StandardInput.WriteAsync(System.Text.Json.JsonSerializer.Serialize(new { path = final, method = 8, entries = new Dictionary<string, object> { ["large.bin"] = new { size = length, sha256 = hash } } })); oracle.StandardInput.Close();
        var stdout = oracle.StandardOutput.ReadToEndAsync(); var stderr = oracle.StandardError.ReadToEndAsync();
        using var deadline = new CancellationTokenSource(60000); await oracle.WaitForExitAsync(deadline.Token);
        Require(oracle.ExitCode == 0 && (await stderr).Length == 0 && (await stdout).Contains("verified_entries", StringComparison.Ordinal), "independent streaming ZIP64 exact hash");
        Require(Directory.GetFiles(output.FullName, ".nxpart-*").Length == 0, "large partial leak");
    });
}
