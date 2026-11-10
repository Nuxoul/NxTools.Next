using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    public static async Task IncompleteCancellationAndFirstNativeError()
    {
        foreach (var mode in new[] { "cancel", "first-error", "cancel-first-error", "cancel-worker-error" })
        {
            var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-lifecycle-" + Guid.NewGuid());
            Directory.CreateDirectory(root);
            var source = Path.Combine(root, mode + ".zip"); var destination = Path.Combine(root, "output");
            using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
                foreach (var name in new[] { "first.bin", "second.bin", "third.bin" })
                    using (var entry = zip.CreateEntry(name).Open()) entry.WriteByte(1);
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new TerminalLog();
            using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
                null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, log, null], null)!;
            var cleanupObserved = false; var leaseHeldAtCleanup = false; var metadataCalls = 0;
            var id = Guid.NewGuid();
            runner.CleanupCheckpointProbe = () => {
                cleanupObserved = true;
                try { using var exclusive = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None); }
                catch (IOException) { leaseHeldAtCleanup = true; }
            };
            if (mode is "first-error" or "cancel-first-error") runner.OutputCommitCheckpointProbe = stage => {
                if (stage == "before_metadata" && Interlocked.Increment(ref metadataCalls) == 2) {
                    if (mode == "cancel-first-error") Check(runner.Cancel(id), "cancel while native close is already executing");
                    throw new ArchiveOutputException("target_denied", 5);
                }
            };
            try
            {
                Check(runner.Start(new(id, "extract", source, 65001,
                    new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                    OutputRoot: destination, TimeoutMs: 15000, ParallelExtraction: true)), "lifecycle admission");
                var deadline = DateTime.UtcNow.AddSeconds(10);
                while (!events.Any(item => item.Frame is { } frame && frame.TryGetProperty("marker", out var marker) && marker.GetString() == "two_incomplete") &&
                    !events.Any(item => item.Kind == "exited") && DateTime.UtcNow < deadline) await Task.Delay(10);
                Check(events.Any(item => item.Frame is { } frame && frame.TryGetProperty("marker", out var marker) && marker.GetString() == "two_incomplete"),
                    "two incomplete files not reached before terminal");
                Check(File.ReadAllBytes(Path.Combine(destination, "first.bin")).SequenceEqual(new byte[] { 1, 2, 3 }) &&
                    VisibleBytes(Path.Combine(destination, "second.bin")).SequenceEqual(new byte[] { 4, 5, 6 }) &&
                    VisibleBytes(Path.Combine(destination, "third.bin")).SequenceEqual(new byte[] { 7, 8, 9 }),
                    "first complete plus two simultaneous incomplete native files");
                if (mode is "cancel" or "cancel-worker-error") Check(runner.Cancel(id), "cancel accepted against original active Run");
                else Check(runner.Write(id, JsonSerializer.SerializeToElement(new { schema = 1, operation_id = id, op = "write", fixture_continue = true })),
                    "release native first-error checkpoint");
                Check(await runner.DrainAsync(20000), "lifecycle process/pipes/native tasks did not drain");
                var terminalDeadline = DateTime.UtcNow.AddSeconds(3);
                while (!events.Any(item => item.Kind == "exited") && DateTime.UtcNow < terminalDeadline) await Task.Delay(10);
                var terminal = events.Single(item => item.Kind == "exited");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0 && terminal.FilesCommitted == 1 &&
                    terminal.Category == (mode == "cancel" ? "cancelled_with_committed_output" : mode == "cancel-worker-error" ? "worker_failure" : "target_denied") &&
                    (mode is not ("first-error" or "cancel-first-error") || terminal.NativeErrorCode == 5),
                    $"first native failure/cancel result: {mode}/{terminal.Category}/{terminal.NativeErrorCode}/{terminal.FilesCommitted}/{terminal.CleanupComplete}");
                Check(cleanupObserved && leaseHeldAtCleanup, "native work must join before original source lease release");
                Check(File.Exists(Path.Combine(destination, "first.bin")) && !File.Exists(Path.Combine(destination, "second.bin")) &&
                    !File.Exists(Path.Combine(destination, "third.bin")), "completed output retained; both incomplete owned outputs deleted");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                Check(Convert.ToInt32(log.Fields!["native_pending_high_water"]) is >= 1 and <= 32, "bounded native pending diagnostic");
            }
            finally { runner.Stop(); Check(await runner.DrainAsync(20000), "lifecycle final drain"); Directory.Delete(root, true); }
        }
    }

    private static byte[] VisibleBytes(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        using var buffer = new MemoryStream(); stream.CopyTo(buffer); return buffer.ToArray();
    }

    private static async Task<int> PeerLifecycle(Stream input, Stream output, Guid id, string mode)
    {
        async Task<JsonElement> Call(string action, string? path = null, Guid? token = null)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = call, action, path, token, metadata = action == "commit" ? new { modified = (object?)null, attributes = (object?)null } : null } });
            var reply = await Receive(input);
            if (mode == "cancel-first-error" && reply.GetProperty("op").GetString() == "cancel") reply = await Receive(input);
            Check(reply.GetProperty("call_id").GetGuid() == call, "lifecycle bound native reply");
            return reply;
        }
        var first = (await Call("begin", "first.bin")).GetProperty("token").GetGuid();
        await output.WriteAsync(Binary(id, first, 1, [1, 2, 3]));
        Check((await Call("commit", token: first)).GetProperty("success").GetBoolean(), "first close before failure");
        var calls = new[] { Guid.NewGuid(), Guid.NewGuid() };
        for (var index = 0; index < 2; index++)
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = calls[index], action = "begin", path = index == 0 ? "second.bin" : "third.bin" } });
        var tokens = new Dictionary<Guid, Guid>();
        for (var index = 0; index < 2; index++) {
            var reply = await Receive(input);
            Check(reply.GetProperty("success").GetBoolean(), "two incomplete begins");
            tokens.Add(reply.GetProperty("call_id").GetGuid(), reply.GetProperty("token").GetGuid());
        }
        Check(tokens.Count == 2 && calls.All(tokens.ContainsKey), "lifecycle separate replies");
        await output.WriteAsync(Binary(id, tokens[calls[0]], 2, [4, 5, 6]));
        await output.WriteAsync(Binary(id, tokens[calls[1]], 3, [7, 8, 9]));
        await Send(output, new { schema = 1, operation_id = id, op = "progress", marker = "two_incomplete" });
        if (mode is "first-error" or "cancel-first-error") {
            var proceed = await Receive(input);
            Check(proceed.GetProperty("op").GetString() == "write" && proceed.GetProperty("fixture_continue").GetBoolean(), "native error checkpoint released");
            var close = await Call("commit", token: tokens[calls[0]]);
            Check(!close.GetProperty("success").GetBoolean() && close.GetProperty("native_error_code").GetInt32() == 5,
                "first real native failure delivered unchanged");
        }
        else {
            var cancel = await Receive(input);
            Check(cancel.GetProperty("op").GetString() == "cancel", "original cancellation input");
        }
        await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = mode == "cancel-worker-error" ? "CorruptArchive" : "Cancelled" });
        return 0;
    }
}
