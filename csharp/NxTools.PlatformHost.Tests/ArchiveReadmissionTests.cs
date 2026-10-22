using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Actual original runner/Job/worker; synthetic4096byte ZIP, missing target root.
internal static class ArchiveReadmissionTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    public static async Task PreflightHasNoOutputEffects()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit matching worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Check(!File.Exists(helper), "refuse existing helper");
        var root = Directory.CreateTempSubdirectory("archive-readmission-"); var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            var source = Path.Combine(root.FullName, "tiny.zip");
            using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) { using var stream = zip.CreateEntry("payload.bin", CompressionLevel.Optimal).Open(); stream.Write(new byte[4096]); }
            var output = Path.Combine(root.FullName, "missing", "output");
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var id = Guid.NewGuid(); Check(runner.Start(new(id, "extract", source, 936, new { max_entries = 100, max_entry_bytes = 1024, max_output_bytes = 8192 }, OutputRoot: output, OutputReadmission: JsonSerializer.SerializeToElement(new { logical_id = id, expected_source = (object?)null }))), "original admission");
            var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { Check(clock.ElapsedMilliseconds < 10000, "original exit"); await Task.Delay(10); }
            Check(await runner.DrainAsync() && events.Single(e => e.Kind == "exited").CleanupComplete, "actual original cleanup");
            Check(events.Any(e => e.Frame is { } f && f.GetProperty("op").GetString() == "output_budget_shortfall"), "declared output shortage must expose current and required finite quotas");
            Check(!Directory.Exists(Path.Combine(root.FullName, "missing")), "preflight shortage must not create target directories");
        } finally { if (copied) File.Delete(helper); root.Delete(true); }
    }
}
