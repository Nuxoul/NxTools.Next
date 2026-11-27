using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveProductSessionTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string ActualWorker => Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../target/debug/nxtools-archive-worker.exe"));
    private static string Package(bool synthetic = false)
    {
        var root = ArchiveProductReadTests.Root();
        if (!synthetic) {
            Check(File.Exists(ActualWorker), "parent-built actual Rust worker missing");
            File.Copy(ActualWorker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } else {
            foreach (var file in Directory.EnumerateFiles(AppContext.BaseDirectory))
                if (Path.GetExtension(file) is ".dll" or ".json") File.Copy(file, Path.Combine(root, Path.GetFileName(file)));
            File.Copy(Environment.ProcessPath!, Path.Combine(root, ArchiveReadHelper.DeploymentName));
        }
        return root;
    }
    private static string Zip(string root, int count = 3)
    {
        var path = Path.Combine(root, "private-name.zip");
        // CPython is an independent writer; positives are not runner-only fake metadata.
        var start = new ProcessStartInfo("python") { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add("-c");
        start.ArgumentList.Add("import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1],'w',compression=zipfile.ZIP_STORED) as z:\n for i in range(int(sys.argv[2])): z.writestr('literal-%d.txt'%i,b'abc')");
        start.ArgumentList.Add(path); start.ArgumentList.Add(count.ToString());
        using var process = Process.Start(start)!; process.StandardOutput.ReadToEnd(); process.StandardError.ReadToEnd();
        Check(process.WaitForExit(10000) && process.ExitCode == 0, "independent ZIP generation"); return path;
    }
    private static ArchiveReadEvent Decode(ProtocolEnvelope envelope) => envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
    private static async Task<ArchiveReadEvent> Next(ArchiveReadSession session) => Decode(await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(10)));
    private static async Task<(List<ArchiveReadEvent> Events, ArchiveReadTerminal Terminal)> Finish(ArchiveReadSession session, ArchiveReadCommand command, bool acknowledge = true)
    {
        var events = new List<ArchiveReadEvent>();
        while (true) {
            var item = await Next(session); events.Add(item);
            if (item.Batch is { } batch && acknowledge) Check(session.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: command.Version)).Success, "business ack receipt");
            if (item.Terminal is { } terminal) return (events, terminal);
        }
    }
    private static async Task Until(Func<bool> condition) {
        var clock = Stopwatch.StartNew(); while (!condition()) { Check(clock.ElapsedMilliseconds < 10000, "cleanup deadline"); await Task.Delay(20); }
    }
    public static async Task ActualOpenTestAndPrivacy()
    {
        var root = Package(); var log = new RecordingLog();
        try {
            var path = Zip(root); await using var session = ArchiveReadSession.ForOwnedTestPackage(root, log);
            var request = ArchiveProductReadTests.Command(path: path); var command = ArchiveReadWire.Parse(request);
            Check(session.Dispatch(request).Success, "fast admission receipt");
            var result = await Finish(session, command);
            Check(result.Terminal.Status == "succeeded" && result.Terminal.CleanupComplete, "actual open terminal: " + result.Terminal.Code);
            Check(result.Terminal.EntryCount == 3 && result.Terminal.BatchCount == 2, "exact metadata counts");
            var entries = result.Events.Where(e => e.Batch is not null).SelectMany(e => e.Batch!.Entries).ToArray();
            Check(entries.Select(e => e.Name).SequenceEqual(new[] { "literal-0.txt", "literal-1.txt", "literal-2.txt" }) && entries.All(e => e.Kind == "File" && e.Size == 3 && e.Provenance == "UncertainCp437" && e.Codepage == 437), "literal metadata/kind/size/provenance");
            Check(result.Events.Count(e => e.Kind == "accepted") == 1 && result.Events.Count(e => e.Kind == "negotiated") == 1, "accepted/negotiated unique");
            Check(!session.Dispatch(request).Success && !session.Events.TryRead(out _), "retired duplicate created terminal");
            var test = ArchiveProductReadTests.Command("test", path: path); Check(session.Dispatch(test).Success, "test admission");
            var verified = await Finish(session, ArchiveReadWire.Parse(test));
            Check(verified.Terminal.Status == "succeeded" && verified.Terminal.FilesVerified == 3 && verified.Terminal.BytesVerified == 9 && verified.Events.All(e => e.Batch is null), "actual CRC test");
            Check(log.Records.Any(r => r.Contains("archive.read.terminal")) && log.Records.All(r => !r.Contains(root) && !r.Contains("literal-") && !r.Contains("private-name")), "diagnostic privacy");
            using var source = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName)); // actual helper cleanup released before terminal
        } finally { Directory.Delete(root, true); }
    }
    public static async Task SlowCreditBudgetsAndCancellation()
    {
        var root = Package();
        try {
            var path = Zip(root, 5); await using var session = ArchiveReadSession.ForOwnedTestPackage(root);
            var request = ArchiveProductReadTests.Command(path: path, limits: new() { AckTimeoutMs = 1000 }); var command = ArchiveReadWire.Parse(request);
            Check(session.Dispatch(request).Success, "pause admission");
            ArchiveReadEvent first; do { first = await Next(session); } while (first.Batch is null);
            await Task.Delay(150); Check(!session.Events.TryRead(out _), "consumer pause granted extra metadata credit");
            Check(!session.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, 2, sequence: 0)).Success, "stale generation ack");
            Check(!session.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, 1, sequence: 1)).Success, "wrong sequence ack");
            try { File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName)); throw new InvalidOperationException("helper lease released before cleanup"); } catch (IOException) { }
            Check(session.Dispatch(ArchiveProductReadTests.Command("cancel", command.OperationId, command.SessionId)).Success, "cancel waiting metadata");
            Check(session.Dispatch(ArchiveProductReadTests.Command("cancel", command.OperationId, command.SessionId)).Success, "idempotent cancel");
            var cancelled = await Finish(session, command, false); Check(cancelled.Terminal.Code == "cancelled" && cancelled.Terminal.CleanupComplete, "cancel cleanup terminal");
            var slow = ArchiveProductReadTests.Command(path: path, limits: new() { AckTimeoutMs = 200 }); Check(session.Dispatch(slow).Success, "slow admission");
            var expired = await Finish(session, ArchiveReadWire.Parse(slow), false); Check(expired.Terminal.Code == "consumer_too_slow" && expired.Terminal.CleanupComplete, "slow consumer terminal");
            var budget = ArchiveProductReadTests.Command(path: path, limits: new() { MaxRetainedBytes = 1 }); Check(session.Dispatch(budget).Success, "budget admission");
            var bounded = await Finish(session, ArchiveReadWire.Parse(budget)); Check(bounded.Terminal.Code == "metadata_budget" && bounded.Terminal.CleanupComplete, "metadata budget terminal");
            var disconnect = ArchiveProductReadTests.Command(path: path); Check(session.Dispatch(disconnect).Success, "disconnect admission");
            do { first = await Next(session); } while (first.Batch is null);
            session.Stop(); await Until(() => session.ActiveCount == 0);
            Check(!session.Events.TryRead(out _), "late disconnected event");
            using var source = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } finally { Directory.Delete(root, true); }
    }
    public static async Task NegotiationCrashAdmissionAndUniqueTerminal()
    {
        var root = Package(synthetic: true);
        try {
            await using var session = ArchiveReadSession.ForOwnedWorkerFixture(root, new RecordingLog { Throw = true });
            foreach (var (mode, expected) in new[] { ("old-version", "incompatible_version"), ("wrong-version", "incompatible_version"), ("missing-capability", "missing_capability"), ("crash", "worker_crashed") }) {
                var request = ArchiveProductReadTests.Command(path: Path.Combine(root, mode + ".zip")); Check(session.Dispatch(request).Success, "negative fixture admission");
                var result = await Finish(session, ArchiveReadWire.Parse(request));
                Check(result.Terminal.Code == expected && result.Terminal.CleanupComplete, mode + " typed terminal: " + result.Terminal.Code);
                Check(!session.Dispatch(request).Success && !session.Events.TryRead(out _), "duplicate terminal");
            }
            var first = ArchiveProductReadTests.Command(path: Path.Combine(root, "sleep.zip"));
            var second = ArchiveProductReadTests.Command(path: Path.Combine(root, "sleep2.zip"));
            Check(session.Dispatch(first).Success && session.Dispatch(second).Success, "two operations");
            await using (var replacement = ArchiveReadSession.ForOwnedWorkerFixture(root)) {
                var extra = ArchiveProductReadTests.Command(path: Path.Combine(root, "reconnect.zip"));
                Check(!replacement.Dispatch(extra).Success, "new pipe session bypassed two original runner slots");
                Check((await Finish(replacement, ArchiveReadWire.Parse(extra))).Terminal.Code == "busy", "global read admission typed reject");
            }
            var third = ArchiveProductReadTests.Command(path: Path.Combine(root, "third.zip")); Check(!session.Dispatch(third).Success, "third operation not bounded");
            var busy = false; while (!busy) { var item = await Next(session); busy = item.Terminal?.Code == "busy"; }
            Check(!session.Dispatch(first).Success, "live duplicate replaced state");
            var original = ArchiveReadWire.Parse(first);
            Check(!session.Dispatch(ArchiveProductReadTests.Command("cancel", original.OperationId, Guid.NewGuid())).Success, "foreign session cancellation");
            Check(session.Dispatch(ArchiveProductReadTests.Command("cancel", original.OperationId, original.SessionId)).Success, "cancel first");
            var next = ArchiveReadWire.Parse(second); Check(session.Dispatch(ArchiveProductReadTests.Command("cancel", next.OperationId, next.SessionId)).Success, "cancel second");
            var terminals = new HashSet<Guid>(); while (terminals.Count != 2) { var item = await Next(session); if (item.Terminal is not null) Check(terminals.Add(item.OperationId), "second terminal for same operation"); }
            Check(session.ActiveCount == 0, "cleanup retained session slots");
            // Factory-time validation alone cannot authorize a replaced executable at later Start.
            File.WriteAllBytes(Path.Combine(root, ArchiveReadHelper.DeploymentName), [1, 2, 3]);
            var replaced = ArchiveProductReadTests.Command(path: Path.Combine(root, "replacement.zip"));
            Check(!session.Dispatch(replaced).Success, "replaced fixture executable admitted");
            var denied = await Finish(session, ArchiveReadWire.Parse(replaced));
            Check(denied.Terminal.Code == "helper_untrusted" && denied.Terminal.Status == "rejected", "replaced fixture typed reject");
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } finally { Directory.Delete(root, true); }
    }
    public static async Task RejectionsAndTransportBound()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            await using var session = ArchiveReadSession.ForOwnedTestPackage(root);
            var missing = ArchiveProductReadTests.Command();
            Check(!session.Dispatch(missing).Success, "missing helper admitted");
            var rejected = await Finish(session, ArchiveReadWire.Parse(missing));
            Check(rejected.Terminal.Code == "helper_missing" && rejected.Terminal.NativeErrorCode is 2 or 3 && rejected.Terminal.CleanupComplete && session.ActiveCount == 0, "missing helper terminal/native code");
            var version = ArchiveProductReadTests.Command();
            var payload = System.Text.Json.Nodes.JsonNode.Parse(version.Payload.GetRawText())!; payload["data"]!["version"] = 3;
            version = version with { Payload = JsonSerializer.SerializeToElement(payload) };
            Check(!session.Dispatch(version).Success, "version accepted");
            Check((await Next(session)).Terminal?.Code == "incompatible_version", "version typed reject");
            var invalid = ArchiveProductReadTests.Command(); payload = System.Text.Json.Nodes.JsonNode.Parse(invalid.Payload.GetRawText())!; payload["data"]!["password"] = "never-log";
            invalid = invalid with { Payload = JsonSerializer.SerializeToElement(payload) };
            Check(!session.Dispatch(invalid).Success && (await Next(session)).Terminal?.Code == "invalid_request", "invalid typed terminal");
            Check(!session.Dispatch(invalid).Success && !session.Events.TryRead(out _), "invalid duplicate terminal");
            // Fixed event capacity: no consumer means bounded failure, not unbounded terminal retention.
            for (var index = 0; index < 12; index++) Check(!session.Dispatch(ArchiveProductReadTests.Command()).Success, "overflow missing helper accepted");
            var count = 0; while (session.Events.TryRead(out _)) count++;
            Check(count == 8, "transport not bounded to eight events");
            try { await session.Events.Completion; throw new InvalidOperationException("full queue silently lost terminal"); }
            catch (InvalidDataException) { }
        } finally { Directory.Delete(root, true); }
    }
    public static async Task CapturedCancellationFollowsActualCleanup()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            Check(File.Exists(ActualWorker), "parent-built actual worker missing");
            File.Copy(ActualWorker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            File.WriteAllText(Path.Combine(root, "capture-terminal"), "");
            var path = Zip(root);
            await using var session = ArchiveReadSession.ForOwnedTestPackage(root);
            var request = ArchiveProductReadTests.Command(path: path, limits: new() { AckTimeoutMs = 10000 });
            var command = ArchiveReadWire.Parse(request);
            Check(session.Dispatch(request).Success, "original production admission");
            ArchiveReadEvent first; do { first = await Next(session); } while (first.Batch is null);
            try { using var denied = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); throw new InvalidOperationException("source not protected at credit barrier"); }
            catch (IOException) { }
            Check(session.Dispatch(ArchiveProductReadTests.Command("cancel", command.OperationId, command.SessionId, command.Generation)).Success, "production session cancellation, never direct runner.Cancel");
            ProtocolEnvelope envelope; ArchiveReadEvent item;
            do { envelope = await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(5)); item = Decode(envelope); } while (item.Terminal is null);
            ArchiveProductHost.CaptureTerminal(root, envelope);
            var captured = JsonSerializer.Deserialize<ArchiveReadTerminal>(File.ReadAllText(Path.Combine(root, $"terminal-{command.OperationId:D}.json")), JsonDefaults.Options)!;
            Check(captured.Code == "cancelled" && captured.CleanupComplete && session.ActiveCount == 0, "observer must retain actual cleanup facts");
            using (File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } finally { Directory.Delete(root, true); }
    }
    public static async Task TimeoutIsNotUserCancellation()
    {
        var root = Package(synthetic: true);
        try {
            await using var session = ArchiveReadSession.ForOwnedWorkerFixture(root);
            var request = ArchiveProductReadTests.Command(path: Path.Combine(root, "timeout-cancel.zip"));
            Check(session.Dispatch(request).Success, "timeout original admission");
            ArchiveReadEvent item;
            do { item = Decode(await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40))); } while (item.Terminal is null);
            Check(item.Terminal.Status == "failed" && item.Terminal.Code == "io_failure", "runner timeout mislabeled user cancellation: " + item.Terminal.Code);
            Check(item.Terminal.CleanupComplete && session.ActiveCount == 0, "timeout actual cleanup");
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } finally { Directory.Delete(root, true); }
    }
    public static async Task LargeReadNegotiationAndFiniteBudgets()
    {
        var root = Package(synthetic: true);
        try {
            await using var session = ArchiveReadSession.ForOwnedWorkerFixture(root);
            var future=ArchiveProductReadTests.Command(path:Path.Combine(root,"future.zip"),version:3,
                limits:new(){MaxEntries=100000,MaxMetadataBytes=134217728,MaxRetainedBytes=67108864});
            Check(!session.Dispatch(future).Success,"unknown version accepted");
            Check((await Next(session)).Terminal?.Code=="incompatible_version","version must outrank budget validation");
            foreach (var (mode, expected) in new[] { ("old-version", "incompatible_version"), ("wrong-version", "incompatible_version"), ("missing-capability", "missing_capability"), ("legacy-strict", "incompatible_version") }) {
                var request = ArchiveProductReadTests.Command(path: Path.Combine(root, mode + ".zip"), version: 2,
                    limits: new() { MaxEntries = 5000, MaxMetadataBytes = 134217728, MaxRetainedBytes = 67108864 });
                Check(session.Dispatch(request).Success, "v2 negative fixture admission");
                var result = await Finish(session, ArchiveReadWire.Parse(request));
                Check(result.Terminal.Code == expected && result.Terminal.CleanupComplete, "v2 " + mode + ": " + result.Terminal.Code);
                Check(result.Events.All(item => item.Batch is null), "unnegotiated metadata escaped");
            }
            ulong? totalBytes = null;
            foreach (var delta in new int?[] { null, 0, 1 }) {
                var limit = delta is null ? 8388608ul : totalBytes!.Value - (ulong)delta.Value;
                var request = ArchiveProductReadTests.Command("test", path: Path.Combine(root, "transport-boundary.zip"), version: 2,
                    limits: new() { MaxTransportBytes = limit });
                Check(session.Dispatch(request).Success, "deterministic transport boundary admission");
                var result = await Finish(session, ArchiveReadWire.Parse(request));
                Check(result.Terminal.CleanupComplete && session.ActiveCount == 0, "transport boundary releases native owner");
                if (delta is null) {
                    Check(result.Terminal.Status == "succeeded" && result.Terminal.Transfer is { PrivateBytes: > 0 }, "fixed wire baseline");
                    totalBytes = result.Terminal.Transfer!.PrivateBytes;
                } else if (delta == 0) {
                    Check(result.Terminal.Status == "succeeded" && result.Terminal.Transfer?.PrivateBytes == limit, "exact cumulative byte allowance");
                } else {
                    Check(result.Terminal.Status == "failed" && result.Terminal.Code == "metadata_budget", "one byte below cumulative transport rejects");
                }
            }
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
        } finally { Directory.Delete(root, true); }
    }
    public static async Task ActualLargeReadOpenTestAndPrivacy()
    {
        var root=Package(); var log=new RecordingLog();
        try {
            var path=Zip(root,600); await using var session=ArchiveReadSession.ForOwnedTestPackage(root,log);
            var limits=new ArchiveReadLimits {MaxEntries=600,MaxMetadataBytes=134217728,MaxRetainedBytes=67108864,MaxTransportBytes=67108864};
            foreach(var action in new[]{"open","test"}) {
                var request=ArchiveProductReadTests.Command(action,path:path,limits:limits,version:2);
                Check(session.Dispatch(request).Success,"v2 actual admission");
                var result=await Finish(session,ArchiveReadWire.Parse(request));
                Check(result.Terminal.Status=="succeeded" && result.Terminal.CleanupComplete,"v2 actual terminal: "+result.Terminal.Code);
                Check(result.Terminal.Transfer is {PrivateBytes:>0,MaxInflightBatches:<=1,ProcessBudgetBytes:268435456},"v2 bounded transfer/Job counters");
                if(action=="open") Check(result.Terminal.EntryCount==600 && result.Terminal.BatchCount==3,"v2 full batches");
                else Check(result.Terminal.FilesVerified==600 && result.Terminal.BytesVerified==1800 && result.Events.All(e=>e.Batch is null),"v2 test-only exact bytes");
            }
            Check(log.Records.All(row=>!row.Contains(root) && !row.Contains("literal-") && !row.Contains("private-name")),"v2 diagnostic privacy");
            using(File.Open(path,FileMode.Open,FileAccess.ReadWrite,FileShare.None)){}
            File.Delete(Path.Combine(root,ArchiveReadHelper.DeploymentName));
        } finally {Directory.Delete(root,true);}
    }
    private sealed class RecordingLog : IPlatformLog
    {
        internal bool Throw;
        internal readonly System.Collections.Concurrent.ConcurrentQueue<string> Records = new();
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) {
            Records.Enqueue(name + " " + correlationId + " " + JsonSerializer.Serialize(fields)); if (Throw) throw new IOException("private sink failure");
        }
    }
}
