using System.Collections.Concurrent;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    private static async Task WithActualWorker(Func<string, Task> exercise)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit matching isolated-codecs Rust worker required.");
        if (Environment.ProcessorCount < 4)
            throw new TestSkippedException("At least two reference-derived ZIP lanes required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Check(!File.Exists(installed), "actual fixture cannot replace an existing worker");
        var owned = Directory.CreateTempSubdirectory("archive-parallel-actual-");
        var copied = false;
        try
        {
            File.Copy(worker, installed, false); copied = true;
            await exercise(owned.FullName);
        }
        finally
        {
            if (copied) File.Delete(installed);
            owned.Delete(true);
        }
    }

    private static async Task<ArchiveWorkerEvent> ActualTerminal(ArchiveWorkerRunner runner,
        ConcurrentQueue<ArchiveWorkerEvent> events)
    {
        var deadline = DateTime.UtcNow.AddSeconds(25);
        while (!events.Any(item => item.Kind == "exited") && DateTime.UtcNow < deadline) await Task.Delay(10);
        Check(events.Any(item => item.Kind == "exited"), "actual worker terminal deadline");
        Check(await runner.DrainAsync(20000), "actual worker/native tasks/pipes did not drain");
        var terminal = events.Single(item => item.Kind == "exited");
        Check(terminal.CleanupComplete && runner.ActiveCount == 0, "actual original Job cleanup and task join");
        return terminal;
    }

    private static void ActualSourceReleased(string source)
    {
        using var exclusive = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
    }

    public static Task ActualZipMemberOverlapAndBytes() => WithActualWorker(async root =>
    {
        var source = Path.Combine(root, "mixed.zip"); var destination = Path.Combine(root, "output");
        var expected = Enumerable.Range(0, 8).ToDictionary(index => $"data/file{index}.bin",
            index => Enumerable.Range(0, 70001).Select(offset => (byte)((offset * 17 + offset / 251 + index) % 256)).ToArray());
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
            foreach (var (name, bytes) in expected)
                using (var stream = zip.CreateEntry(name, name.Contains("file0") || name.Contains("file2") ||
                    name.Contains("file4") || name.Contains("file6") ? CompressionLevel.NoCompression : CompressionLevel.Optimal).Open())
                    stream.Write(bytes);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new TerminalLog();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
        using var secondEntered = new ManualResetEventSlim();
        var active = 0; var highWater = 0; var began = 0; var timedOut = 0;
        runner.OutputBeginCheckpointProbe = _ =>
        {
            var current = Interlocked.Increment(ref active); ObservePeak(ref highWater, current);
            var ordinal = Interlocked.Increment(ref began);
            try
            {
                if (ordinal == 1 && !secondEntered.Wait(TimeSpan.FromSeconds(5))) Interlocked.Exchange(ref timedOut, 1);
                else if (ordinal == 2) secondEntered.Set();
            }
            finally { Interlocked.Decrement(ref active); }
        };
        try
        {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: destination, TimeoutMs: 20000, ParallelExtraction: true)), "actual mixed ZIP admission");
            var terminal = await ActualTerminal(runner, events);
            Check(terminal.Category == "success" && terminal.FilesCommitted == 8 && terminal.BytesCommitted == 8UL * 70001,
                $"actual mixed ZIP terminal: {terminal.Category}/{terminal.FilesCommitted}/{terminal.BytesCommitted}");
            Check(timedOut == 0 && highWater >= 2 && began == 8, "actual Rust decoders did not overlap native begins");
            Check(Convert.ToInt32(log.Fields!["native_pending_high_water"]) is >= 2 and <= 32 &&
                Convert.ToInt64(log.Fields["output_write_ack_frames"]) == 0, "bounded actual native controls and zero ordinary payload ACKs");
            var complete = events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "complete").Frame!.Value;
            var work = complete.GetProperty("work");
            Check(work.GetProperty("parallel_workers_high_water").GetUInt64() >= 2 &&
                work.GetProperty("parallel_workers_planned").GetUInt64() <= 32 &&
                work.GetProperty("parallel_members_completed").GetUInt64() == 8, "actual whole-member scheduler observations");
            Check(log.WorkFields is not null && new[] { "parallel_workers_planned", "parallel_workers_high_water", "parallel_members_completed" }
                .All(name => log.WorkFields.TryGetValue(name, out var value) && Convert.ToUInt64(value) == work.GetProperty(name).GetUInt64()),
                "production work diagnostic must retain actual parallel numeric aggregates");
            Check(Directory.GetFiles(destination, "*", SearchOption.AllDirectories).Length == expected.Count, "actual exact file set");
            foreach (var (name, bytes) in expected)
                Check(File.ReadAllBytes(Path.Combine(destination, name)).SequenceEqual(bytes), "actual Stored/Deflate bytes: " + name);
            ActualSourceReleased(source);
        }
        finally { secondEntered.Set(); runner.Stop(); Check(await runner.DrainAsync(20000), "actual success final drain"); }
    });

    public static Task ActualCancelMultipleIncomplete() => WithActualWorker(async root =>
    {
        if (Environment.ProcessorCount < 6) throw new TestSkippedException("Three actual ZIP lanes required for completed plus two pending outputs.");
        var source = Path.Combine(root, "cancel.zip"); var destination = Path.Combine(root, "output");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
        {
            using (var first = zip.CreateEntry("first.bin", CompressionLevel.NoCompression).Open()) first.Write(new byte[] { 1, 2, 3 });
            foreach (var name in new[] { "second.bin", "third.bin" })
                using (var stream = zip.CreateEntry(name, CompressionLevel.NoCompression).Open())
                {
                    var block = Enumerable.Repeat((byte)0x53, 65536).ToArray();
                    for (var index = 0; index < 64; index++) stream.Write(block);
                }
        }
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        using var firstClosed = new ManualResetEventSlim(); using var bothBegins = new ManualResetEventSlim();
        var pendingBegins = 0; var timedOut = 0; var cancelled = 0; var cleanup = false; var leaseHeld = false;
        var id = Guid.NewGuid();
        runner.OutputCommitCheckpointProbe = stage => { if (stage == "native_closed") firstClosed.Set(); };
        runner.OutputBeginCheckpointProbe = path =>
        {
            if (path == "first.bin") return;
            if (Interlocked.Increment(ref pendingBegins) == 2) bothBegins.Set();
            if (!firstClosed.Wait(TimeSpan.FromSeconds(5)) || !bothBegins.Wait(TimeSpan.FromSeconds(5)))
                Interlocked.Exchange(ref timedOut, 1);
        };
        runner.OutputWriteCheckpointProbe = _ =>
        {
            if (!firstClosed.IsSet || Interlocked.CompareExchange(ref cancelled, 1, 0) != 0) return;
            var deadline = DateTime.UtcNow.AddSeconds(5);
            while ((!File.Exists(Path.Combine(destination, "second.bin")) || !File.Exists(Path.Combine(destination, "third.bin"))) &&
                DateTime.UtcNow < deadline) Thread.Sleep(1);
            Check(File.Exists(Path.Combine(destination, "second.bin")) && File.Exists(Path.Combine(destination, "third.bin")),
                "actual cancellation requires two simultaneously created incomplete files");
            Check(runner.Cancel(id), "cancel actual worker with two native incomplete outputs");
        };
        runner.CleanupCheckpointProbe = () =>
        {
            cleanup = true;
            try { ActualSourceReleased(source); } catch (IOException) { leaseHeld = true; }
        };
        try
        {
            Check(runner.Start(new(id, "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: destination, TimeoutMs: 20000, ParallelExtraction: true)), "actual parallel cancellation admission");
            var terminal = await ActualTerminal(runner, events);
            Check(timedOut == 0 && pendingBegins == 2 && cancelled == 1 && cleanup && leaseHeld,
                "actual cancellation overlap/cleanup/source lease checkpoints");
            Check(terminal.Category == "cancelled_with_committed_output" && terminal.FilesCommitted == 1 &&
                terminal.Journal?.Count(file => file.Published == true) == 1,
                $"actual cancellation terminal: {terminal.Category}/{terminal.FilesCommitted}");
            Check(Directory.GetFiles(destination, "*", SearchOption.AllDirectories).Length == 1 &&
                File.ReadAllBytes(Path.Combine(destination, "first.bin")).SequenceEqual(new byte[] { 1, 2, 3 }) &&
                !File.Exists(Path.Combine(destination, "second.bin")) && !File.Exists(Path.Combine(destination, "third.bin")),
                "actual cancellation retains completed bytes and removes every incomplete file");
            ActualSourceReleased(source);
        }
        finally { firstClosed.Set(); bothBegins.Set(); runner.Stop(); Check(await runner.DrainAsync(20000), "actual cancellation final drain"); }
    });

    public static Task ActualCrcFailureRetainsCompleted() => WithActualWorker(async root =>
    {
        var source = Path.Combine(root, "crc.zip"); var destination = Path.Combine(root, "output");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
        {
            using (var first = zip.CreateEntry("first.bin", CompressionLevel.NoCompression).Open()) first.Write(new byte[] { 1, 2, 3 });
            using (var bad = zip.CreateEntry("bad.bin", CompressionLevel.NoCompression).Open()) bad.Write(Enumerable.Repeat((byte)0x57, 40001).ToArray());
        }
        var raw = File.ReadAllBytes(source);
        var payload = raw.AsSpan().IndexOf(Enumerable.Repeat((byte)0x57, 32).ToArray());
        Check(payload >= 0, "owned Stored payload corruption location"); raw[payload + 17] ^= 1; File.WriteAllBytes(source, raw);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        using var firstClosed = new ManualResetEventSlim();
        var timedOut = 0; var cleanup = false; var leaseHeld = false;
        runner.OutputCommitCheckpointProbe = stage => { if (stage == "native_closed") firstClosed.Set(); };
        runner.OutputBeginCheckpointProbe = path =>
        {
            if (path == "bad.bin" && !firstClosed.Wait(TimeSpan.FromSeconds(5))) Interlocked.Exchange(ref timedOut, 1);
        };
        runner.CleanupCheckpointProbe = () =>
        {
            cleanup = true;
            try { ActualSourceReleased(source); } catch (IOException) { leaseHeld = true; }
        };
        try
        {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: destination, TimeoutMs: 20000, ParallelExtraction: true)), "actual parallel CRC admission");
            var terminal = await ActualTerminal(runner, events);
            var failure = events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure").Frame!.Value;
            Check(timedOut == 0 && cleanup && leaseHeld && failure.GetProperty("error_code").GetString() == "CorruptArchive" &&
                terminal.Category == "worker_failure" && terminal.FilesCommitted == 1 && terminal.Journal?.Count(file => file.Published == true) == 1,
                $"actual CRC first failure/completed journal: {terminal.Category}/{terminal.FilesCommitted}");
            Check(Directory.GetFiles(destination, "*", SearchOption.AllDirectories).Length == 1 &&
                File.ReadAllBytes(Path.Combine(destination, "first.bin")).SequenceEqual(new byte[] { 1, 2, 3 }) &&
                !File.Exists(Path.Combine(destination, "bad.bin")), "actual CRC failure retains completed bytes and deletes corrupt output");
            ActualSourceReleased(source);
        }
        finally { firstClosed.Set(); runner.Stop(); Check(await runner.DrainAsync(20000), "actual CRC final drain"); }
    });
}
