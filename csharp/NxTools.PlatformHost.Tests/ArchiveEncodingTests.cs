using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only explicit tiny Python fixtures, GUID-owned outputs and the original runner/Job.
internal static class ArchiveEncodingTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    public static async Task AutoAndManualNativeOutput()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ENCODING_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit worker and independent encoding fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Check(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-encoding-"); var copied = false;
        try {
            File.Copy(worker, helper, false); copied = true;
            foreach (var (file, cp, expected, failure) in new (string, int, string?, string?)[] {
                ("auto-gbk.zip", 0, "中文文件资料.txt", null),
                ("ambiguous.zip", 936, "中文.txt", null),
                ("switch-unsafe.zip", 1252, null, "UnsafePath"),
                ("switch-case.zip", 1252, null, "OutputCollision"),
                ("switch-prefix.zip", 1252, null, "OutputCollision") }) {
                var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var began = 0;
                runner.OutputCommitCheckpointProbe = _ => began++;
                Check(runner.Start(new(Guid.NewGuid(), "extract", Path.Combine(fixtures, file), cp, new { max_entries = 100 }, OutputRoot: output.FullName)), "encoding admission");
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited")) { Check(clock.ElapsedMilliseconds < 15000, "encoding deadline"); await Task.Delay(20); }
                Check(await runner.DrainAsync(), "encoding drain");
                var terminal = events.Single(e => e.Kind == "exited"); Check(terminal.CleanupComplete, "encoding cleanup");
                var frames = events.Where(e => e.Kind == "frame" && e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                if (failure is null) {
                    Check(terminal.Category == "success" && terminal.FilesCommitted == 1, "encoding terminal");
                    var files = Directory.GetFiles(output.FullName, "*", SearchOption.AllDirectories);
                    Check(files.Length == 1 && Path.GetFileName(files[0]) == expected, "actual output name differs: " + file);
                    Check(File.ReadAllBytes(files[0]).SequenceEqual("independent encoding payload\n"u8.ToArray()), "independent content differs");
                } else {
                    Check(terminal.FilesCommitted == 0 && began == 0, "unsafe decoding reached commit boundary");
                    Check(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == failure, "typed decoding rejection");
                    Check(!Directory.EnumerateFileSystemEntries(output.FullName).Any(), "rejected decoding wrote output");
                }
            }
        } finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
