using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Actual worker/native boundary. Accepted is emitted before the asynchronous bridge/source acquire.
internal static class ArchiveAdmissionTests
{
    private static void Require(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit actual worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-admission-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<ArchiveWorkerEvent> Run(ArchiveWorkerRequest request, Action accepted,
        Action<ArchiveWorkerRunner>? setup = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var mutated = false;
        using var runner = ArchiveWorkerRunner.ForPackage(value => {
            events.Enqueue(value);
            if (value.Kind == "accepted") { accepted(); mutated = true; }
        });
        setup?.Invoke(runner);
        try {
            Require(runner.Start(request), "admission rejected");
            var clock = Stopwatch.StartNew();
            while (!events.Any(value => value.Kind == "exited")) {
                Require(clock.ElapsedMilliseconds < 30000, "worker deadline"); await Task.Delay(20);
            }
            Require(await runner.DrainAsync(), "actual cleanup");
            Require(mutated, "accepted mutation not reached");
            var terminal = events.Single(value => value.Kind == "exited");
            Require(terminal.CleanupComplete, "incomplete cleanup");
            Require(Directory.GetFiles(request.OutputRoot!, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "owned partial leaked");
            return terminal;
        }
        finally { runner.Stop(); Require(await runner.DrainAsync(), "finally cleanup"); }
    }
    private static void VerifyZip(string path, IReadOnlyDictionary<string, byte[]> expected)
    {
        using var zip = ZipFile.OpenRead(path);
        Require(zip.Entries.Select(entry => entry.FullName).Order().SequenceEqual(expected.Keys.Order()), "accepted source names changed");
        foreach (var entry in zip.Entries) {
            using var payload = entry.Open(); using var bytes = new MemoryStream(); payload.CopyTo(bytes);
            Require(bytes.ToArray().SequenceEqual(expected[entry.FullName]), "accepted source bytes changed");
        }
    }
    public static Task SourceElements() => Sources("element");
    public static Task SourceCount() => Sources("add");
    public static Task SourceClear() => Sources("clear");
    private static Task Sources(string mode) => WithWorker(async owned => {
        var a = Path.Combine(owned.FullName, "approved.txt"); var b = Path.Combine(owned.FullName, "replacement.txt");
        var original = "approved-source"u8.ToArray(); File.WriteAllBytes(a, original); File.WriteAllText(b, "unconfirmed-source");
        var paths = new List<string> { a }; var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
        var terminal = await Run(new(Guid.NewGuid(), "create_zip", a, 0, new { max_entries = 100 },
            OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal", SourcePaths: paths), () => {
                if (mode == "element") paths[0] = b; else if (mode == "add") paths.Add(b); else paths.Clear();
            });
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1, "accepted roots lost: " + terminal.Category);
        VerifyZip(Path.Combine(output.FullName, "created.zip"), new Dictionary<string, byte[]> { ["approved.txt"] = original });
    });
    public static Task Exclusions() => WithWorker(async owned => {
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input"));
        File.WriteAllText(Path.Combine(source.FullName, "keep.txt"), "keep"); File.WriteAllText(Path.Combine(source.FullName, "omit.txt"), "omit");
        var patterns = new List<string> { "omit.txt" }; var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
        var terminal = await Run(new(Guid.NewGuid(), "create_zip", source.FullName, 0, new { max_entries = 100 },
            OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal",
            ZipOptions: new(StripSingleRoot: true, ExcludePatterns: patterns)), () => { patterns[0] = "keep.txt"; patterns.Clear(); patterns.Add("*"); });
        Require(terminal.Category == "success" && terminal.FilesCommitted == 1, "exclusion mutation terminal");
        VerifyZip(Path.Combine(output.FullName, "created.zip"), new Dictionary<string, byte[]> { ["keep.txt"] = "keep"u8.ToArray() });
    });
    public static Task Budgets() => WithWorker(async owned => {
        foreach (var mode in new[] { "source", "output" }) {
            var source = Path.Combine(owned.FullName, mode + ".txt"); File.WriteAllText(source, "approved-budget");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
            var limits = new Dictionary<string, ulong> { ["max_entries"] = 100, ["max_entry_bytes"] = 1048576, ["max_output_bytes"] = 1048576 };
            var terminal = await Run(new(Guid.NewGuid(), "create_zip", source, 0, limits, OutputRoot: output.FullName,
                CreateName: "created.zip", CompressionLevel: "normal"), () => limits[mode == "source" ? "max_entry_bytes" : "max_output_bytes"] = 1);
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1, "accepted budget changed: " + mode + "/" + terminal.Category);
            VerifyZip(Path.Combine(output.FullName, "created.zip"), new Dictionary<string, byte[]> { [mode + ".txt"] = "approved-budget"u8.ToArray() });
        }
    });
    public static Task OverwriteAuthorization() => WithWorker(async owned => {
        foreach (var mode in new[] { "replace", "remove", "add" }) {
            var source = Path.Combine(owned.FullName, mode + ".txt"); File.WriteAllText(source, "approved-source");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, mode)); var final = Path.Combine(output.FullName, "created.zip");
            File.WriteAllText(final, "original-target");
            var confirmations = new Dictionary<string, string>();
            if (mode != "add") { using var capture = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid()); confirmations["created.zip"] = capture.CaptureTargetIdentity("created.zip"); }
            var terminal = await Run(new(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName,
                CreateName: "created.zip", CompressionLevel: "normal", ConfirmedOverwrites: confirmations), () => {
                if (mode == "replace") { File.Move(final, final + ".old"); File.WriteAllText(final, "new-unconfirmed-target"); }
                if (mode == "remove") confirmations.Clear();
                else { using var capture = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid()); confirmations["created.zip"] = capture.CaptureTargetIdentity("created.zip"); }
            });
            if (mode == "remove") {
                Require(terminal.Category == "success" && terminal.FilesCommitted == 1, "removed caller confirmation revoked accepted authorization");
                VerifyZip(final, new Dictionary<string, byte[]> { [mode + ".txt"] = "approved-source"u8.ToArray() });
            } else {
                Require(terminal.Category == (mode == "replace" ? "target_changed" : "worker_failure") && terminal.FilesCommitted == 0, "mutable confirmation authorized new target: " + terminal.Category);
                Require(terminal.CommitState == "FailedBeforeCommit", "precommit state: " + mode + "/" + terminal.CommitState);
                Require(File.ReadAllText(final) == (mode == "replace" ? "new-unconfirmed-target" : "original-target"), "unconfirmed target overwritten");
            }
        }
    });
    public static Task LegacySharedAuthorization() => WithWorker(async owned => {
        foreach (var operation in new[] { "extract", "create_gzip", "create_xz" }) {
            var source = Path.Combine(owned.FullName, operation + (operation == "extract" ? ".zip" : ".txt"));
            const string outputName = "payload.txt";
            if (operation == "extract") { using var zip = ZipFile.Open(source, ZipArchiveMode.Create); using var writer = new StreamWriter(zip.CreateEntry(outputName).Open()); writer.Write("approved-source"); }
            else File.WriteAllText(source, "approved-source");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, operation)); var final = Path.Combine(output.FullName, outputName);
            File.WriteAllText(final, "original-target");
            var confirmations = new Dictionary<string, string>();
            using (var capture = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid())) confirmations[outputName] = capture.CaptureTargetIdentity(outputName);
            var terminal = await Run(new(Guid.NewGuid(), operation, source, 0, new { max_entries = 100 }, OutputRoot: output.FullName,
                CreateName: operation == "extract" ? null : outputName, CompressionLevel: operation == "extract" ? null : "fast", ConfirmedOverwrites: confirmations), () => {
                File.Move(final, final + ".old"); File.WriteAllText(final, "new-unconfirmed-target");
                using var capture = ArchiveOutputSession.Open(output.FullName, Guid.NewGuid()); confirmations[outputName] = capture.CaptureTargetIdentity(outputName);
            });
            Require(terminal.Category == "target_changed" && terminal.FilesCommitted == 0 && terminal.CommitState == "FailedBeforeCommit", "legacy mutable authorization: " + operation + "/" + terminal.Category);
            Require(File.ReadAllText(final) == "new-unconfirmed-target", "legacy target overwritten");
        }
    });
    private sealed class ChangingPaths(string first, string second) : IReadOnlyList<string>
    {
        public int Count => 1;
        public string this[int index] => second;
        public IEnumerator<string> GetEnumerator() { yield return first; }
        System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator() => GetEnumerator();
    }
    private sealed class MutatingLimits(Action mutate)
    {
        public ulong max_entries { get { mutate(); return 100; } }
    }
    public static Task AdmissionBounds() => WithWorker(owned => {
        var source = Path.Combine(owned.FullName, "source.txt"); File.WriteAllText(source, "source");
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
        var valid = new ArchiveWorkerRequest(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal");
        foreach (var request in new[] {
            valid with { SourcePaths = Enumerable.Repeat(source, 33).ToArray() },
            valid with { ZipOptions = new(ExcludePatterns: new[] { new string('a', 1025) }) },
            valid with { SourcePaths = new ChangingPaths(source, source + ".changed") },
            valid with { ConfirmedOverwrites = new Dictionary<string, string> { ["created.zip"] = new string('a', 257) } },
            valid with { Limits = new Dictionary<string, ulong> { ["unknown"] = 1 } },
        }) {
            var events = new List<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Add);
            Require(!runner.Start(request with { OperationId = Guid.NewGuid() }) && runner.ActiveCount == 0, "unstable/oversized request accepted");
            Require(events.Count == 1 && events[0].Kind == "rejected", "observable acceptance before capture");
        }
        foreach (var useArray in new[] { false, true }) {
            var array = new[] { source }; var list = new List<string> { source };
            var unstable = valid with { SourcePaths = useArray ? array : list,
                Limits = new MutatingLimits(() => { if (useArray) array[0] = source + ".changed"; else list[0] = source + ".changed"; }) };
            var events = new List<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Add);
            Require(!runner.Start(unstable with { OperationId = Guid.NewGuid() }) && events.Single().Category == "request_changed", "concurrent source mutation not rejected before acceptance");
        }
        return Task.CompletedTask;
    });
    private sealed class CountChanges(string path) : IReadOnlyList<string>
    {
        private bool moved;
        public int Count => moved ? 2 : 1;
        public string this[int index] => path;
        public IEnumerator<string> GetEnumerator() { moved = true; yield return path; }
        System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator() => GetEnumerator();
    }
    public static Task ErrorCompatibility() => WithWorker(owned => {
        var source = Path.Combine(owned.FullName, "source.txt"); File.WriteAllText(source, "source");
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
        var valid = new ArchiveWorkerRequest(Guid.NewGuid(), "create_zip", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName, CreateName: "created.zip", CompressionLevel: "normal");
        foreach (var (request, category) in new[] {
            (valid with { CreateName = new string('a', 256) }, "invalid_component"),
            (valid with { Password = new string('p', 4097), ZipOptions = new(Encryption: "aes256") }, "invalid_create_request"),
            (valid with { SourcePaths = new CountChanges(source) }, "request_changed"),
        }) {
            var events = new List<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Add);
            Require(!runner.Start(request with { OperationId = Guid.NewGuid() }) && events.Single().Category == category, "admission category: expected " + category + "/actual " + events.Last().Category);
        }
        return Task.CompletedTask;
    });
}
