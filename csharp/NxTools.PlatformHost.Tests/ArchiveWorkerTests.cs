using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveWorkerTests
{
    private static ArchiveWorkerRequest Request(string mode, int memory = 0, int timeout = 3000) =>
        new(Guid.NewGuid(), "open", mode, 65001, memory == 0 ? new { max_working_bytes = 512UL << 20, max_metadata_bytes = 256UL << 20 } :
            new { max_working_bytes = (ulong)Math.Max(0L, (long)memory - (320L << 20)), max_metadata_bytes = 256UL << 20 }, null, memory, timeout);
    private static ArchiveWorkerRunner Runner(Action<ArchiveWorkerEvent> publish, IPlatformLog? log = null,
        Action<ExtensionKernelHandle>? probe = null) =>
        ArchiveWorkerRunner.ForFixture(Environment.ProcessPath!, publish, log, probe);
    private static void Assert(bool result, string detail) { if (!result) throw new InvalidOperationException(detail); }
    private static async Task Until(Func<bool> check)
    {
        var clock = Stopwatch.StartNew();
        while (!check())
        { if (clock.ElapsedMilliseconds > 10000) throw new InvalidOperationException("fixture deadline"); await Task.Delay(20); }
    }
    public static async Task SuspendedAdmissionAndJobLimits()
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var marker = Path.Combine(Directory.CreateTempSubdirectory("archive-worker-").FullName, "marker");
        try
        {
            using var denied = Runner(events.Enqueue, probe: _ =>
            { Assert(!File.Exists(marker), "child ran before admission"); throw new InvalidOperationException("deny admission"); });
            var request = Request(marker);
            Assert(denied.Start(request), "admit failed");
            await Until(() => events.Any(e => e.OperationId == request.OperationId && e.Kind == "exited"));
            Assert(!File.Exists(marker), "suspended process ran despite denied admission");
            using var runner = Runner(events.Enqueue, probe: process => Assert(ExtensionNative.InJob(process), "not assigned before resume"));
            var limits = Request("job");
            Assert(runner.Start(limits), "job fixture rejected");
            await Until(() => events.Any(e => e.OperationId == limits.OperationId && e.Kind == "exited"));
            var frame = events.Single(e => e.OperationId == limits.OperationId && e.Kind == "frame" && e.Frame!.Value.GetProperty("op").GetString() == "accepted").Frame!.Value;
            var job = frame.GetProperty("data");
            Assert(job.GetProperty("flags").GetUInt32() == 0x2208 && job.GetProperty("processes").GetUInt32() == 1 &&
                job.GetProperty("memory").GetUInt64() == (832UL << 20), "actual job flags/budget/process cap");
        }
        finally { Directory.Delete(Path.GetDirectoryName(marker)!, true); }
    }
    public static async Task FramesAndPrivacy()
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var log = new RejectingLog();
        using var runner = Runner(events.Enqueue, log);
        foreach (var (mode, reason) in new[] { ("badlength", "stdout_frame_limit"), ("badjson", "stdout_invalid_frame"),
            ("badid", "stdout_invalid_frame"), ("truncated", "stdout_invalid_frame"), ("badutf8", "stdout_invalid_frame") })
        {
            var request = Request(mode);
            Assert(runner.Start(request), "failed fixture admission");
            await Until(() => events.Any(e => e.OperationId == request.OperationId && e.Kind == "exited"));
            Assert(events.Last(e => e.OperationId == request.OperationId).Category == reason, "wrong frame failure: " + mode);
        }
        foreach (var (mode, reason) in new[] { ("missing", "stdout_missing_terminal"), ("failure", "worker_failure") })
        {
            var request = Request(mode);
            Assert(runner.Start(request), "terminal fixture rejected");
            await Until(() => events.Any(e => e.OperationId == request.OperationId && e.Kind == "exited"));
            Assert(events.Last(e => e.OperationId == request.OperationId).Category == reason, "wrong terminal: " + mode);
        }
        var secret = "DO_NOT_LOG_PASSWORD_" + Guid.NewGuid();
        var stderr = Request("stderr") with { Password = secret };
        Assert(runner.Start(stderr), "stderr fixture admission");
        await Until(() => events.Any(e => e.OperationId == stderr.OperationId && e.Kind == "exited"));
        Assert(events.Last(e => e.OperationId == stderr.OperationId).Category == "success", "log failure changed result");
        Assert(log.Text.All(text => !text.Contains(secret) && !text.Contains("stderr") && !text.Contains("SSSS")), "diagnostic leaked data");
    }
    public static async Task StuckAdmissionRetainsSlots()
    {
        using var release = new ManualResetEventSlim();
        var entered = 0;
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = Runner(events.Enqueue, probe: _ =>
        { Interlocked.Increment(ref entered); release.Wait(); });
        var first = Request("sleep", timeout: 5000);
        var second = Request("sleep", timeout: 5000);
        try
        {
            Assert(runner.Start(first) && runner.Start(second), "two suspended slots");
            await Until(() => Volatile.Read(ref entered) == 2);
            Assert(runner.Cancel(first.OperationId) && runner.Cancel(second.OperationId), "cancel suspended slots");
            Assert(!await runner.DrainAsync(100) && !runner.Start(Request("job")) && runner.ActiveCount == 2,
                "blocked admission created replacement");
        }
        finally { release.Set(); runner.Stop(); Assert(await runner.DrainAsync(), "admission drain"); }
        Assert(events.Count(e => e.Kind == "exited") == 2 && !events.Any(e => e.Kind == "started"),
            "suspended cancellation must not execute worker");
    }
    public static async Task CancellationAndAdmission()
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = Runner(events.Enqueue);
        var first = Request("sleep", 1024 * 1024 * 1024, 5000);
        var second = Request("sleep", 1024 * 1024 * 1024, 5000);
        Assert(runner.Start(first) && runner.Start(second), "two slots");
        Assert(!runner.Start(first) && !runner.Start(Request("sleep")), "duplicate/third admission");
        Assert(runner.ActiveCount == 2, "unexpected capacity");
        Assert(runner.Cancel(first.OperationId), "cancel failed");
        await Until(() => events.Any(e => e.OperationId == first.OperationId && e.Kind == "exited"));
        Assert(events.Last(e => e.OperationId == first.OperationId).Category == "cancelled", "cancel category");
        var next = Request("job");
        Assert(runner.Start(next), "later job not responsive");
        await Until(() => events.Any(e => e.OperationId == next.OperationId && e.Kind == "exited"));
        Assert(events.Last(e => e.OperationId == next.OperationId).Category == "success", "later job failed");
        Assert(!runner.Start(first), "replayed operation id");
        runner.Stop(); Assert(await runner.DrainAsync(), "owned workers not drained");
        Assert(runner.ActiveCount == 0, "slot retained");
        var controls = Request("sleep", timeout: 5000);
        using var controlRunner = Runner(events.Enqueue);
        Assert(controlRunner.Start(controls), "control fixture rejected");
        await Until(() => events.Any(e => e.OperationId == controls.OperationId && e.Kind == "started"));
        using var message = JsonDocument.Parse(JsonSerializer.Serialize(new
        { schema = 1, operation_id = controls.OperationId, op = "write", payload = "bounded" }));
        Assert(controlRunner.Write(controls.OperationId, message.RootElement), "bounded control rejected");
        await Until(() => events.Any(e => e.OperationId == controls.OperationId && e.Kind == "frame" &&
            e.Frame!.Value.GetProperty("op").GetString() == "progress"));
        Assert(controlRunner.Cancel(controls.OperationId), "control worker cancel");
        Assert(await controlRunner.DrainAsync(), "control worker drain");
        using var timeoutRunner = Runner(events.Enqueue);
        var timed = Request("sleep", timeout: 300);
        Assert(!timeoutRunner.Start(Request("job", 67108863)), "below minimum memory accepted");
        Assert(!timeoutRunner.Start(Request("job", timeout: 120001)), "unbounded timeout accepted");
        Assert(timeoutRunner.Start(timed), "timeout fixture rejected");
        await Until(() => events.Any(e => e.OperationId == timed.OperationId && e.Kind == "exited"));
        Assert(events.Last(e => e.OperationId == timed.OperationId).Category == "timeout", "timeout terminal category");
    }
    public static async Task ActualRustWorkerPrivateProtocol()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(source) || !File.Exists(source)) throw new TestSkippedException("Explicit built Rust archive fixture worker is not supplied.");
        var ownedRoot = Directory.CreateTempSubdirectory("archive-rust-integration-").FullName;
        var installedName = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Assert(!File.Exists(installedName), "fixture must not overwrite an existing worker");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var createdWorker = false;
        try
        {
            File.Copy(source!, installedName, false);
            createdWorker = true;
            var zipPath = Path.Combine(ownedRoot, "fixture.zip");
            using (var zip = System.IO.Compression.ZipFile.Open(zipPath, System.IO.Compression.ZipArchiveMode.Create))
            {
                var entry = zip.CreateEntry("资料/说明.txt");
                using var writer = new StreamWriter(entry.Open()); writer.Write("isolated-cross-language");
            }
            using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var request = new ArchiveWorkerRequest(Guid.NewGuid(), "open", zipPath, 0, new { max_entries = 100 });
            Assert(runner.Start(request), "admit actual Rust worker");
            await Until(() => events.Any(e => e.OperationId == request.OperationId && e.Kind == "exited"));
            var terminal = events.Last(e => e.OperationId == request.OperationId);
            Assert(terminal.Category == "success" && terminal.CleanupComplete, "actual Rust completion/cleanup: " + terminal.Category);
            Assert(events.Any(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "entry_batch" &&
                frame.GetProperty("entries")[0].GetProperty("name").GetString() == "资料/说明.txt"), "actual Unicode metadata lost across C#/Rust pipe");
            var missing = request with { OperationId = Guid.NewGuid(), Path = Path.Combine(ownedRoot, "missing.rar") };
            Assert(runner.Start(missing), "admit typed failure");
            await Until(() => events.Any(e => e.OperationId == missing.OperationId && e.Kind == "exited"));
            Assert(events.Any(e => e.OperationId == missing.OperationId && e.Frame is { } frame && frame.GetProperty("op").GetString() == "failure"), "actual Rust typed failure absent");
            Assert(await runner.DrainAsync(), "actual workers/pumps must drain");
        }
        finally { if (createdWorker) File.Delete(installedName); Directory.Delete(ownedRoot, true); }
    }

    private sealed class RejectingLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Text = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { Text.Enqueue(eventName + JsonSerializer.Serialize(fields)); throw new InvalidOperationException("log sink down"); }
    }
}
