using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    private sealed class TerminalLog : IPlatformLog
    {
        internal IReadOnlyDictionary<string, object?>? Fields;
        internal IReadOnlyDictionary<string, object?>? WorkFields;
        public void Write(string level, string name, Guid? id = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled") Fields = fields;
            if (name == "archive.worker.work") WorkFields = fields;
        }
    }

    public static async Task MissingParallelCapabilityHasNoOutput()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-capability-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "missing-cap.zip");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) using (var entry = zip.CreateEntry("item.bin").Open()) entry.WriteByte(1);
        var destination = Path.Combine(root, "output");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, null, null], null)!;
        try {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001, new { max_entries = 100 },
                OutputRoot: destination, TimeoutMs: 10000, ParallelExtraction: true)), "capability fixture admission");
            Check(await runner.DrainAsync(15000), "capability process/Job cleanup");
            var terminal = events.Single(item => item.Kind == "exited");
            Check(terminal.CleanupComplete && terminal.Category == "worker_capability_mismatch" && terminal.FilesCommitted == 0 &&
                !Directory.Exists(destination), "missing parallel capability must refuse before output mutation");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        } finally { runner.Stop(); Check(await runner.DrainAsync(15000), "capability final drain"); Directory.Delete(root, true); }
    }

    public static async Task NativeBeginCap()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-cap-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "cap.zip");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
            for (var index = 0; index < 33; index++) using (var entry = zip.CreateEntry($"file{index}.bin").Open()) entry.WriteByte(1);
        var destination = Path.Combine(root, "output");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new TerminalLog();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, log, null], null)!;
        using var release = new ManualResetEventSlim();
        var active = 0; var peak = 0; var entered = 0; var timedOut = 0;
        ThreadPool.GetMinThreads(out var originalWorkers, out var originalIo);
        Check(ThreadPool.SetMinThreads(Math.Max(originalWorkers, 34), originalIo), "owned bounded native task fixture threads");
        runner.OutputBeginCheckpointProbe = _ => {
            var now = Interlocked.Increment(ref active);
            ObservePeak(ref peak, now);
            if (Interlocked.Increment(ref entered) == 32) release.Set();
            if (!release.Wait(TimeSpan.FromSeconds(20))) Interlocked.Exchange(ref timedOut, 1);
            Interlocked.Decrement(ref active);
        };
        try
        {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: destination, TimeoutMs: 30000, ParallelExtraction: true)), "cap source admission");
            Check(await runner.DrainAsync(35000), "cap original Job/pipe/native tasks did not drain");
            var terminal = events.Single(item => item.Kind == "exited");
            Check(terminal.CleanupComplete && runner.ActiveCount == 0 && terminal.Category == "success" && terminal.FilesCommitted == 33,
                $"cap source/output/cleanup: {terminal.Category}/{terminal.FilesCommitted}/{terminal.CleanupComplete}");
            Check(entered == 33 && timedOut == 0 && peak == 32 && Convert.ToInt32(log.Fields!["native_pending_high_water"]) == 32,
                $"native cap/overlap: entered={entered}, peak={peak}, timeout={timedOut}, pending={log.Fields?["native_pending_high_water"]}");
            Check(Directory.GetFiles(destination).Length == 33 && Directory.GetFiles(destination).All(file => new FileInfo(file).Length == 0),
                "distinct 33 native final names");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        }
        finally {
            release.Set(); runner.Stop();
            try { Check(await runner.DrainAsync(35000), "cap final task/lease drain"); Directory.Delete(root, true); }
            finally { ThreadPool.SetMinThreads(originalWorkers, originalIo); }
        }
    }

    private static async Task<int> PeerCap(Stream input, Stream output, Guid id)
    {
        var calls = Enumerable.Range(0, 33).Select(index => Guid.NewGuid()).ToArray();
        for (var index = 0; index < calls.Length; index++)
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = calls[index], action = "begin", path = $"file{index}.bin" } });
        var tokens = new Dictionary<Guid, Guid>();
        foreach (var _ in calls) {
            var reply = await Receive(input);
            Check(reply.GetProperty("success").GetBoolean(), "cap begin result");
            var call = reply.GetProperty("call_id").GetGuid();
            Check(calls.Contains(call) && !tokens.ContainsKey(call), "cap distinct original call routing");
            tokens.Add(call, reply.GetProperty("token").GetGuid());
        }
        foreach (var call in calls) {
            var close = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = close, action = "commit", token = tokens[call], metadata = new { modified = (object?)null, attributes = (object?)null } } });
            var reply = await Receive(input);
            Check(reply.GetProperty("call_id").GetGuid() == close && reply.GetProperty("success").GetBoolean(), "cap close result");
        }
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 33, bytes_verified = 0 });
        return 0;
    }
}
