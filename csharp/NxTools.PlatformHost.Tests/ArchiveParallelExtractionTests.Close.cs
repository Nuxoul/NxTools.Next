using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    public static async Task NativeClosesOverlapAfterPayloadDrain()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-close-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "close-overlap.zip"); var destination = Path.Combine(root, "output");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
            foreach (var name in new[] { "first.bin", "second.bin" })
                using (var entry = zip.CreateEntry(name).Open()) entry.WriteByte(1);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, null, null], null)!;
        using var secondEntered = new ManualResetEventSlim();
        var active = 0; var peak = 0; var timedOut = 0;
        runner.OutputCommitCheckpointProbe = stage => {
            if (stage != "before_metadata") return;
            var current = Interlocked.Increment(ref active);
            ObservePeak(ref peak, current);
            try {
                if (current == 1) { if (!secondEntered.Wait(TimeSpan.FromSeconds(5))) Interlocked.Exchange(ref timedOut, 1); }
                else secondEntered.Set();
            } finally { Interlocked.Decrement(ref active); }
        };
        try {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001, new { max_entries = 100 },
                OutputRoot: destination, TimeoutMs: 12000, ParallelExtraction: true)), "two native close admission");
            Check(await runner.DrainAsync(17000), "native closes and Job did not drain");
            var terminal = events.Single(item => item.Kind == "exited");
            Check(terminal.CleanupComplete && terminal.Category == "success" && terminal.FilesCommitted == 2 && runner.ActiveCount == 0,
                $"two real closes did not publish: {terminal.Category}/{terminal.FilesCommitted}/{terminal.CleanupComplete}");
            Check(peak == 2 && timedOut == 0, $"native close overlap missing: high_water={peak}, first_wait_timed_out={timedOut}");
            Check(File.ReadAllBytes(Path.Combine(destination, "first.bin")).SequenceEqual(new byte[] { 1, 2, 3 }) &&
                File.ReadAllBytes(Path.Combine(destination, "second.bin")).SequenceEqual(new byte[] { 4, 5, 6 }), "preceding payload drained before independent close");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        } finally { runner.Stop(); Check(await runner.DrainAsync(17000), "native close final drain"); Directory.Delete(root, true); }
    }

    public static async Task LaterPayloadAfterCloseIsRejected()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-late-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "late-payload.zip"); var destination = Path.Combine(root, "output");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) using (var entry = zip.CreateEntry("first.bin").Open()) entry.WriteByte(1);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, null, null], null)!;
        try {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001, new { max_entries = 100 },
                OutputRoot: destination, TimeoutMs: 12000, ParallelExtraction: true)), "late payload fixture admission");
            Check(await runner.DrainAsync(17000), "late payload original Job/native drain");
            var terminal = events.Single(item => item.Kind == "exited");
            Check(terminal.CleanupComplete && terminal.Category == "stdout_invalid_frame" && runner.ActiveCount == 0,
                $"payload after token close must be rejected: {terminal.Category}/{terminal.CleanupComplete}");
            var path = Path.Combine(destination, "first.bin");
            Check(!File.Exists(path) || File.ReadAllBytes(path).SequenceEqual(new byte[] { 1, 2, 3 }),
                "late payload appended to an incomplete or already closed output");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        } finally { runner.Stop(); Check(await runner.DrainAsync(17000), "late payload final drain"); Directory.Delete(root, true); }
    }

    private static async Task<int> PeerLatePayload(Stream input, Stream output, Guid id)
    {
        var begin = Guid.NewGuid();
        await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
            call_id = begin, action = "begin", path = "first.bin" } });
        var reply = await Receive(input);
        Check(reply.GetProperty("call_id").GetGuid() == begin && reply.GetProperty("success").GetBoolean(), "late payload begin");
        var token = reply.GetProperty("token").GetGuid();
        await output.WriteAsync(Binary(id, token, 1, [1, 2, 3]));
        await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
            call_id = Guid.NewGuid(), action = "commit", token,
            metadata = new { modified = (object?)null, attributes = (object?)null } } });
        await output.WriteAsync(Binary(id, token, 2, [4], 3));
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 1, bytes_verified = 4 });
        return 0;
    }

    private static async Task<int> PeerClose(Stream input, Stream output, Guid id)
    {
        var begins = new[] { Guid.NewGuid(), Guid.NewGuid() };
        for (var index = 0; index < begins.Length; index++)
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = begins[index], action = "begin", path = index == 0 ? "first.bin" : "second.bin" } });
        var tokens = new Dictionary<Guid, Guid>();
        for (var index = 0; index < begins.Length; index++) {
            var reply = await Receive(input);
            Check(reply.GetProperty("success").GetBoolean(), "two native begin replies");
            tokens.Add(reply.GetProperty("call_id").GetGuid(), reply.GetProperty("token").GetGuid());
        }
        Check(begins.All(tokens.ContainsKey), "close fixture original begin identity");
        await output.WriteAsync(Binary(id, tokens[begins[0]], 1, [1, 2, 3]));
        await output.WriteAsync(Binary(id, tokens[begins[1]], 2, [4, 5, 6]));
        var closes = new[] { Guid.NewGuid(), Guid.NewGuid() };
        for (var index = 0; index < closes.Length; index++)
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = closes[index], action = "commit", token = tokens[begins[index]],
                metadata = new { modified = (object?)null, attributes = (object?)null } } });
        var replied = new HashSet<Guid>();
        foreach (var _ in closes) {
            var result = await Receive(input);
            Check(result.GetProperty("success").GetBoolean(), "native close result");
            Check(closes.Contains(result.GetProperty("call_id").GetGuid()) && replied.Add(result.GetProperty("call_id").GetGuid()),
                "close reply bound to original call");
        }
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 2, bytes_verified = 6 });
        return 0;
    }
}
