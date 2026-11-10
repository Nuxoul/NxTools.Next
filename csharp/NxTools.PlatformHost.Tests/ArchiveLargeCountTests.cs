using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only GUID-owned source copies, original Job/worker/native output, and synthetic result facts.
internal static class ArchiveLargeCountTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CountLog : IPlatformLog
    {
        internal int OutputEvents;
        internal IReadOnlyDictionary<string, object?>? TerminalMetrics;
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (eventName.StartsWith("archive.output.", StringComparison.Ordinal)) Interlocked.Increment(ref OutputEvents);
            if (eventName is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed") TerminalMetrics = fields;
        }
    }
    private static string Root()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (worker is null || !File.Exists(worker)) throw new TestSkippedException("Explicit matching isolated worker required");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Scratch required");
        var root = Path.Combine(scratch, "archive-product-" + Guid.NewGuid()); Directory.CreateDirectory(root);
        File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName)); return root;
    }
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    private static async Task<(ArchiveJobTerminal Terminal, byte[] Envelope)> Extract(string root, string path, string output, CountLog log,
        int cancelAfter = 0, bool largeCount = true, ArchiveJobLimits? limits = null)
    {
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root, log, largeCount: largeCount);
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
            Request = new() { Kind = "extract", Paths = [path], Destination = output, Limits = limits ?? new(),
                Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false } } };
        var published = 0; var cancelAccepted = 0;
        if (cancelAfter > 0) jobs.ProbeOutputCommitForFixture(stage => {
            if (stage == "native_closed" && Interlocked.Increment(ref published) == cancelAfter)
                Interlocked.Exchange(ref cancelAccepted, jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success ? 1 : -1);
        });
        Check(jobs.Dispatch(Wire(command)).Success, "original large extraction admission");
        while (true) {
            var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromMinutes(15));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Check(item.OperationId == command.OperationId && item.SessionId == command.SessionId, "original operation correlation");
            Check(item.Prompt is null, "fresh owned output needs no consent prompt");
            if (item.Terminal is { } terminal) {
                if (cancelAfter > 0) Check(cancelAccepted == 1, "cancel reaches original operation after close boundary");
                var bytes = JsonSerializer.SerializeToUtf8Bytes(envelope, JsonDefaults.Options);
                Check(bytes.Length <= ProtocolConstants.MaxFrameBytes, "large terminal stays within original public frame");
                if (largeCount) Check(terminal.JournalSummary is { } summary && summary.TotalRecords >= terminal.FilesCommitted &&
                    summary.DetailComplete == (summary.TotalRecords == (ulong)(terminal.Journal?.Count ?? 0)) &&
                    summary.Outcomes.Where(outcome => outcome.Published == true).Aggregate(0UL, (sum, outcome) => sum + outcome.Files) == terminal.FilesCommitted &&
                    terminal.Journal is { Count: <= 256 }, "complete native aggregates and explicitly bounded detail");
                else Check(terminal.JournalSummary is null, "legacy session omits the new summary");
                return (terminal, bytes);
            }
        }
    }
    internal static async Task LegacyAndMetadataAdmission()
    {
        var root = Root(); var passed = false;
        try {
            var path = Path.Combine(root, "admission.zip");
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create)) {
                for (var index = 0; index < 5000; index++) { using var entry = zip.CreateEntry($"item-{index:D5}.txt", CompressionLevel.NoCompression).Open(); entry.WriteByte(7); }
            }
            foreach (var largeCount in new[] { false, true }) {
                var output = Path.Combine(root, largeCount ? "bounded" : "legacy"); Directory.CreateDirectory(output);
                File.WriteAllText(Path.Combine(output, "foreign.txt"), "unchanged");
                var result = await Extract(root, path, output, new CountLog(), largeCount: largeCount,
                    limits: largeCount ? new() { MaxMetadataBytes = 12UL << 20 } : null);
                Check(result.Terminal is { Outcome: "failed", Code: "resource_limit", FilesCommitted: 0, CleanupComplete: true }, "count/metadata refusal before native publications");
                Check(Directory.GetFiles(output).Length == 1 && File.ReadAllText(Path.Combine(output, "foreign.txt")) == "unchanged", "no partial or changed foreign output");
            }
            passed = true;
        } finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED OWNED LARGE-COUNT FIXTURE " + root); }
    }
    internal static async Task SuppliedSeventyThousand()
    {
        var corpus = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_COMPAT_CORPUS");
        if (corpus is null) throw new TestSkippedException("Explicit supplied corpus required");
        var root = Root(); var passed = false;
        try {
            var path = Path.Combine(root, "many.zip");
            File.Copy(Path.Combine(corpus, "samples", "zip", "zip-zip64-70000-entries.zip"), path);
            var output = Path.Combine(root, "out"); Directory.CreateDirectory(output);
            var log = new CountLog(); var clock = Stopwatch.StartNew();
            var result = await Extract(root, path, output, log); var terminal = result.Terminal;
            Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete && terminal.FilesCommitted == 70000 && terminal.BytesCommitted == 70000,
                $"complete supplied extraction: outcome={terminal.Outcome} code={terminal.Code} committed={terminal.FilesCommitted} elapsed_ms={terminal.ElapsedMs}");
            var extractionMs = clock.Elapsed.TotalMilliseconds;
            using var source = ZipFile.OpenRead(path);
            Check(source.Entries.Count == 70000 && source.Entries.All(entry => entry.Length == 1), "independent supplied tiny-file corpus");
            var names = Directory.GetFiles(output, "*", SearchOption.AllDirectories).Select(file => Path.GetRelativePath(output, file).Replace('\\', '/')).ToHashSet(StringComparer.Ordinal);
            Check(names.SetEquals(source.Entries.Select(entry => entry.FullName)), "all 70000 names without extras or omission");
            foreach (var entry in source.Entries) {
                using var expected = entry.Open(); var bytes = File.ReadAllBytes(Path.Combine(output, entry.FullName));
                Check(bytes.Length == 1 && bytes[0] == expected.ReadByte() && expected.ReadByte() == -1, "exact independent member bytes");
            }
            Check(!Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "no unpublished partials after cleanup");
            Check(log.OutputEvents <= 32, "output diagnostics stay bounded rather than one event per tiny file");
            File.WriteAllBytes(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-large-count-terminal.json"), result.Envelope);
            Console.WriteLine($"large_count files={terminal.FilesCommitted} bytes={terminal.BytesCommitted} extract_ms={extractionMs:F3} terminal_bytes={result.Envelope.Length} output_events={log.OutputEvents}");
            passed = true;
        } finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED OWNED LARGE-COUNT FIXTURE " + root); }
    }
    internal static async Task CancellationBeyondLegacyBoundary()
    {
        var root = Root(); var passed = false;
        try {
            var path = Path.Combine(root, "cancel.zip");
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create)) {
                for (var index = 0; index < 5000; index++) { using var entry = zip.CreateEntry($"item-{index:D5}.txt", CompressionLevel.NoCompression).Open(); entry.WriteByte((byte)(index % 251)); }
            }
            var output = Path.Combine(root, "out"); Directory.CreateDirectory(output);
            var result = await Extract(root, path, output, new CountLog(), 4200); var terminal = result.Terminal;
            Check(terminal.Outcome == "cancelled" && terminal.FilesCommitted >= 4200 && terminal.FilesCommitted < 5000 && terminal.CleanupComplete, "large cancellation keeps actual committed facts and cleanup");
            Check((ulong)Directory.GetFiles(output, "*", SearchOption.AllDirectories).Length == terminal.FilesCommitted, "terminal count matches surviving native files");
            Check(!Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "cancellation removes only its unpublished partials");
            Console.WriteLine($"large_cancel committed={terminal.FilesCommitted} terminal_bytes={result.Envelope.Length}"); passed = true;
            File.WriteAllBytes(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-large-cancel-terminal.json"), result.Envelope);
        } finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED OWNED LARGE-COUNT FIXTURE " + root); }
    }
    internal static async Task CorruptionBeyondLegacyBoundary()
    {
        var root = Root(); var passed = false;
        try {
            var path = Path.Combine(root, "corrupt.zip");
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create)) {
                for (var index = 0; index < 5000; index++) { using var entry = zip.CreateEntry($"item-{index:D5}.txt", CompressionLevel.NoCompression).Open(); entry.WriteByte((byte)(index % 251)); }
            }
            var archive = File.ReadAllBytes(path);
            var name = System.Text.Encoding.UTF8.GetBytes("item-04999.txt");
            var offset = archive.AsSpan().IndexOf(name);
            Check(offset >= 30 && BitConverter.ToUInt32(archive, offset - 30) == 0x04034b50, "independent last local header");
            var extra = BitConverter.ToUInt16(archive, offset - 2);
            archive[offset + name.Length + extra] ^= 0x40;
            File.WriteAllBytes(path, archive);
            var output = Path.Combine(root, "out"); Directory.CreateDirectory(output);
            var log = new CountLog();
            using var sampleStop = new CancellationTokenSource();
            long peakPrivate = 0;
            var sample = Task.Run(async () => {
                while (!sampleStop.IsCancellationRequested) {
                    using var process = Process.GetCurrentProcess();
                    peakPrivate = Math.Max(peakPrivate, process.PrivateMemorySize64);
                    try { await Task.Delay(100, sampleStop.Token); } catch (OperationCanceledException) { break; }
                }
            });
            (ArchiveJobTerminal Terminal, byte[] Envelope) result;
            try { result = await Extract(root, path, output, log); }
            finally { sampleStop.Cancel(); await sample; }
            var terminal = result.Terminal;
            Check(terminal.Outcome == "failed" && terminal.Code == "corrupt_archive" && terminal.FilesCommitted == 4999 && terminal.CleanupComplete,
                $"corrupt later member retains prior outputs: outcome={terminal.Outcome} code={terminal.Code} committed={terminal.FilesCommitted}");
            Check((ulong)Directory.GetFiles(output).Length == terminal.FilesCommitted && !File.Exists(Path.Combine(output, "item-04999.txt")), "only CRC-verified members published");
            Check(!Directory.EnumerateFiles(output, ".nxpart-*").Any(), "failed current partial cleaned");
            Check(terminal.JournalSummary!.TotalRecords == 5000 && terminal.Journal!.Any(fact => fact.Published == false && fact.Cleanup == "partial_removed"), "failed member detail retained ahead of ordinary commits");
            File.WriteAllBytes(Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-large-corrupt-terminal.json"), result.Envelope);
            Console.WriteLine($"large_corrupt committed={terminal.FilesCommitted} elapsed_ms={terminal.ElapsedMs} host_peak_private_bytes={peakPrivate} terminal_bytes={result.Envelope.Length}");
            Console.WriteLine("large_native_metrics " + JsonSerializer.Serialize(log.TerminalMetrics));
            passed = true;
        } finally { if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED OWNED LARGE-COUNT FIXTURE " + root); }
    }
}
