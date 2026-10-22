using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Small own scratch ZIPs/targets/Jobs only. Inputs and outcomes retained; no source delete/recycle.
internal static class ArchiveCancellationBoundaryTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static TaskCompletionSource<bool> Signal() => new(TaskCreationOptions.RunContinuationsAsynchronously);
    private static void SourceProtected(string path)
    {
        try { using var write = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source released before actual cleanup"); }
        catch (IOException) { }
    }
    public static async Task StreamingReceiveObserveCleanup()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        Check(!string.IsNullOrEmpty(worker) && File.Exists(worker), "explicit matching actual worker required, no SKIP");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!;
        var root = Directory.CreateDirectory(Path.Combine(scratch, "archive-cancel-boundary-" + Guid.NewGuid())).FullName;
        File.WriteAllText(Path.Combine(root, "owned-cancel-boundary"), "v1\n");
        var source = Path.Combine(root, "small.zip");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) {
            using var content = zip.CreateEntry("payload.bin", CompressionLevel.NoCompression).Open();
            var block = Enumerable.Range(0, 256).Select(index => (byte)index).ToArray();
            for (var index = 0; index < 1024; index++) content.Write(block);
        }
        var sourceHash = SHA256.HashData(File.ReadAllBytes(source));
        var output = Directory.CreateDirectory(Path.Combine(root, "output")).FullName;
        var final = Path.Combine(output, "payload.bin"); File.WriteAllText(final, "original target");
        var foreign = Path.Combine(output, ".nxpart-external"); File.WriteAllText(foreign, "external partial");
        string confirmation;
        using (var capture = ArchiveOutputSession.Open(output, Guid.NewGuid())) confirmation = capture.CaptureTargetIdentity("payload.bin");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Check(!File.Exists(helper), "refuse preexisting helper");
        var helperHash = SHA256.HashData(File.ReadAllBytes(worker!));
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var firstWrite = Signal(); var cancelWritten = Signal(); var firstCleanup = Signal(); var bothCleanup = Signal();
        using var releaseWrite = new ManualResetEventSlim(); using var releaseCleanup = new ManualResetEventSlim();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        var writeCalls = 0; var writeBytes = 0; var cleanupCalls = 0; var commitCalls = 0;
        var id = Guid.NewGuid(); var second = Guid.NewGuid(); var clock = new Stopwatch();
        var copied = false;
        try {
            File.Copy(worker!, helper, false); copied = true;
            runner.OutputWriteCheckpointProbe = count => {
                Check(Interlocked.Increment(ref writeCalls) == 1 && count == 65536, "unexpected write after cancelled first chunk");
                Interlocked.Add(ref writeBytes, count); firstWrite.TrySetResult(true);
                Check(releaseWrite.Wait(TimeSpan.FromSeconds(5)), "first actual native write barrier timed out");
            };
            runner.ControlInputCheckpointProbe = op => { if (op == "cancel") cancelWritten.TrySetResult(true); };
            runner.OutputCommitCheckpointProbe = _ => Interlocked.Increment(ref commitCalls);
            runner.CleanupCheckpointProbe = () => {
                var count = Interlocked.Increment(ref cleanupCalls);
                if (count == 1) firstCleanup.TrySetResult(true);
                if (count == 2) bothCleanup.TrySetResult(true);
                if (count <= 2) Check(releaseCleanup.Wait(TimeSpan.FromSeconds(5)), "actual cleanup barrier timed out");
            };
            object limits = new { max_entries = 100, max_metadata_bytes = 4194304, max_working_bytes = 67108864, max_entry_bytes = 1048576, max_output_bytes = 1048576 };
            Check(runner.Start(new(id, "extract", source, 0, limits, OutputRoot: output, ConfirmedOverwrites: new Dictionary<string, string> { ["payload.bin"] = confirmation })), "real extract admission");
            await firstWrite.Task.WaitAsync(TimeSpan.FromSeconds(5));
            SourceProtected(source); Check(runner.ActiveCount == 1 && !events.Any(item => item.Kind == "exited"), "premature terminal at streaming barrier");
            clock.Start(); Check(runner.Cancel(id), "original runner cancellation admission");
            await cancelWritten.Task.WaitAsync(TimeSpan.FromSeconds(2));
            var sendMs = clock.Elapsed.TotalMilliseconds; releaseWrite.Set();
            await firstCleanup.Task.WaitAsync(TimeSpan.FromSeconds(5));
            var cleanupBarrierMs = clock.Elapsed.TotalMilliseconds;
            Check(runner.ActiveCount == 1 && !events.Any(item => item.Kind == "exited"), "process/pump exit released native slot before leases");
            SourceProtected(source);
            var partials = Directory.GetFiles(output, ".nxpart-*").Where(path => path != foreign).ToArray();
            Check(partials.Length == 1 && new FileInfo(partials[0]).Length == 65536, "real native partial size at cleanup barrier");
            Check(File.ReadAllText(final) == "original target" && File.ReadAllText(foreign) == "external partial", "precommit cancellation changed target/foreign partial");
            var cancelledFrame = events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure").Frame!.Value;
            Check(cancelledFrame.GetProperty("error_code").GetString() == "Cancelled", "actual worker must cooperatively observe cancel, not only parent projection");
            Check(runner.Start(new(second, "test", source, 0, limits)), "second actual small worker occupies other slot");
            await bothCleanup.Task.WaitAsync(TimeSpan.FromSeconds(5));
            var blocked = Guid.NewGuid();
            Check(runner.ActiveCount == 2 && !runner.Start(new(blocked, "test", source, 0, limits)), "cleanup-held slots falsely re-admitted a third request");
            Check(events.Single(item => item.Kind == "rejected" && item.OperationId == blocked).Category == "concurrency_limit", "real busy slot rejection category");
            SourceProtected(source); releaseCleanup.Set();
            Check(await runner.DrainAsync(), "original process/Job/pumps/leases/slots actual cleanup");
            var terminal = events.Single(item => item.Kind == "exited" && item.OperationId == id);
            Check(terminal.Category == "cancelled" && terminal.CleanupComplete && terminal.FilesCommitted == 0 && terminal.ExitCode == 0, "cooperative original terminal/cleanup facts");
            Check(writeCalls == 1 && writeBytes == 65536 && commitCalls == 0, "postcancel native writes/commits occurred");
            Check(Directory.GetFiles(output, ".nxpart-*").SequenceEqual(new[] { foreign }), "own partial cleanup removed foreign or retained own");
            Check(File.ReadAllText(final) == "original target" && File.ReadAllText(foreign) == "external partial", "cancel cleanup changed previous output");
            using (new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            Check(SHA256.HashData(File.ReadAllBytes(source)).SequenceEqual(sourceHash), "source bytes changed");
            var followup = Guid.NewGuid();
            Check(runner.Start(new(followup, "test", source, 0, limits)) && await runner.DrainAsync(), "cleanup must permit actual fresh request");
            Check(events.Single(item => item.Kind == "exited" && item.OperationId == followup) is { Category: "success", CleanupComplete: true }, "actual postcleanup request failed");
            // Red-capable observation gap: do not invent reception from caller send or parent Cancelled.
            Check(cancelledFrame.TryGetProperty("cancellation", out var observation), "missing real worker receive/observe facts");
            var received = observation.GetProperty("received_us").GetUInt64(); var observed = observation.GetProperty("observed_us").GetUInt64();
            Check(observed >= received && observed - received <= 200000, "bounded cooperative worker observation target");
            Check(observation.GetProperty("boundary").GetString() is "native_reply" or "checkpoint", "real work observation boundary");
            using var record = new FileStream(Path.Combine(root, "evidence.json"), FileMode.CreateNew, FileAccess.Write);
            JsonSerializer.Serialize(record, new { schema = 1, operation_id = id, status = "passed", caller_send_ms = sendMs, caller_cleanup_barrier_ms = cleanupBarrierMs,
                worker_cancellation = observation, no_cross_clock_subtraction = true, write_calls = writeCalls, write_bytes = writeBytes, commit_calls = commitCalls,
                source_protected_before_cleanup = true, partial_held_before_cleanup = true, two_slots_held_before_cleanup = true, third_busy = true,
                original_target_retained = true, foreign_partial_retained = true, source_unchanged = true, actual_followup_success = true,
                terminal.Category, terminal.FilesCommitted, terminal.CleanupComplete, terminal.ExitCode }, new JsonSerializerOptions { WriteIndented = true });
        }
        finally {
            releaseWrite.Set(); releaseCleanup.Set(); runner.Stop();
            var drained = await runner.DrainAsync();
            if (copied && drained) {
                Check(SHA256.HashData(File.ReadAllBytes(helper)).SequenceEqual(helperHash), "retain changed helper rather than deleting unknown copy");
                File.Delete(helper);
            }
        }
    }
}
