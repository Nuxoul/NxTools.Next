using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Independent tiny inputs, ORIGINAL runner/Job/native output. Never calls a recycle backend.
internal static class ArchiveFormatConflictTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class NeverRecycle : IArchiveRecycleBackend {
        internal int Calls;
        public ArchiveRecycleNativeResult Recycle(ArchiveRecycleSource source, CancellationToken cancellation) {
            Calls++; throw new InvalidOperationException("Skip must never authorize source recycling");
        }
    }
    private static JsonElement[] Prompts(ConcurrentQueue<ArchiveWorkerEvent> events) => events
        .Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "conflict_prompt")
        .Select(e => e.Frame!.Value.GetProperty("conflict_prompt")).ToArray();
    private static async Task Until(Func<bool> done) {
        var clock = Stopwatch.StartNew(); while (!done()) { Check(clock.ElapsedMilliseconds < 15000, "format conflict deadline"); await Task.Delay(10); }
    }
    public static async Task ActualFormatConflicts()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        var codec = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        var sevenzip = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE");
        if (new[] { worker, python, sevenzip }.Any(p => string.IsNullOrEmpty(p) || !File.Exists(p)) || string.IsNullOrEmpty(fixtures) || string.IsNullOrEmpty(codec))
            throw new TestSkippedException("Explicit matching worker, tiny fixtures, CPython and existing stock7Zip required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Check(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-format-conflicts-"); var copied = false;
        try {
            var input = Directory.CreateDirectory(Path.Combine(owned.FullName, "inputs"));
            var producer = new ProcessStartInfo(python!) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
            foreach (var arg in new[] { Path.Combine(fixtures!, "generate-format-conflicts.py"), input.FullName, sevenzip! }) producer.ArgumentList.Add(arg);
            using (var process = Process.Start(producer)!) {
                var stdout = await process.StandardOutput.ReadToEndAsync(); var stderr = await process.StandardError.ReadToEndAsync();
                Check(process.WaitForExit(10000) && process.ExitCode == 0 && stderr.Length == 0 && stdout.Contains("\"solid\": true"), "independent solid/TAR producer");
            }
            File.Copy(worker!, helper); copied = true;
            ArchivePasswordTicket? previous = null;
            foreach (var mode in new[] { "solid_skip", "solid_cancel", "rar4_overwrite", "rar5_overwrite", "rar4_skip", "rar5_skip", "tar_rename", "tgz_rename", "tar_skip", "tar_race", "collision", "unsafe", "prefix" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
                var rar = mode.StartsWith("rar", StringComparison.Ordinal);
                var source = rar ? Path.Combine(codec!, mode.StartsWith("rar4", StringComparison.Ordinal) ? "rar4.rar" : "rar5.rar") : Path.Combine(input.FullName,
                    mode.StartsWith("solid", StringComparison.Ordinal) ? "solid.7z" : mode == "tar_race" ? "single.tar" : mode == "tgz_rename" ? "plain.tar.gz" : mode is "collision" or "unsafe" or "prefix" ? mode + ".tar" : "plain.tar");
                var name = rar ? mode.StartsWith("rar4", StringComparison.Ordinal) ? "VERSION" : "unicodefilename❤️.txt" : "a.txt";
                var expected = rar ? mode.StartsWith("rar4", StringComparison.Ordinal) ? "unrar-0.4.0" : "foobar\n" : "incoming-a.txt";
                var output = Directory.CreateDirectory(Path.Combine(root.FullName, "output"));
                var old = Path.Combine(output.FullName, name); File.WriteAllText(old, "old-target");
                if (mode is "solid_skip" or "tar_skip") File.WriteAllText(Path.Combine(output.FullName, "b.txt"), "old-second");
                var foreign = Path.Combine(output.FullName, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new NeverRecycle();
                JsonElement? eligibility = null;
                var skipping = mode is "solid_skip" or "rar4_skip" or "rar5_skip" or "tar_skip";
                using var runner = skipping ? ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend, evidenceProbe: e => { eligibility = e.Clone(); return e; }) : ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var id = Guid.NewGuid(); var writes = 0; var raced = false;
                runner.OutputWriteCheckpointProbe = _ => writes++;
                runner.SourceCheckpointProbe = stage => { if (stage == "source_precommit" && mode == "solid_skip") Check(Prompts(events).Length == 1 && writes == 1, "solid publication waits full decode and task decisions"); };
                runner.OutputCommitCheckpointProbe = stage => { if (mode == "tar_race" && stage == "before_flush" && !raced) { raced = true; File.WriteAllText(Path.Combine(output.FullName, "a (2).txt"), "racer"); } };
                var policy = mode.Contains("rename") || mode == "tar_race" ? "auto_rename" : mode is "rar4_skip" or "rar5_skip" or "tar_skip" ? "skip" : "ask";
                Check(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName,
                    ConflictPolicy: policy, DeleteSource: skipping ? true : null, ExtractOptions: new(false))), "format conflict admission: " + mode);
                if (policy == "ask" && mode is not ("collision" or "unsafe" or "prefix")) {
                    await Until(() => Prompts(events).Length > 0 || events.Any(e => e.Kind == "exited"));
                    Check(Prompts(events).Length == 1, "real format must enter existing AwaitingConflict: " + mode);
                    Check(runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "original slot/worker remains pending");
                    Check(writes == 0 && Directory.GetFiles(output.FullName, ".nxpart-*").Length == 1 && File.ReadAllText(old) == "old-target", "no current partial/write/publication while waiting");
                    var ticket = Prompts(events)[0].GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    if (previous is not null) Check(!runner.RespondConflict(id, previous, "overwrite", true), "prior task ticket cannot decide new format");
                    if (mode == "solid_cancel") Check(runner.RespondConflict(id, ticket, null), "typed wait cancellation");
                    else Check(runner.RespondConflict(id, ticket, mode == "solid_skip" ? "skip" : "overwrite", mode == "solid_skip"), "original typed conflict reply");
                    previous = ticket;
                }
                await Until(() => events.Any(e => e.Kind == "exited")); Check(await runner.DrainAsync(), "actual original cleanup");
                var terminal = events.Single(e => e.Kind == "exited");
                var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                Check(terminal.CleanupComplete && runner.ActiveCount == 0, "cleanup before resource reuse");
                Check(!runner.RespondConflict(id, previous ?? new(id, Guid.NewGuid(), 1), "overwrite"), "retired ticket rejected");
                if (mode is "collision" or "unsafe" or "prefix" or "tar_race") {
                    var failure = frames.Single(f => f.GetProperty("op").GetString() == "failure");
                    Check(failure.GetProperty("error_code").GetString() == (mode == "unsafe" ? "UnsafePath" : "OutputCollision") && terminal.FilesCommitted == 0 && Prompts(events).Length == 0, "internal/race refusal without prompt/publication");
                    if (mode == "tar_race") Check(File.ReadAllText(Path.Combine(output.FullName, "a (2).txt")) == "racer", "no-replace racing target retained");
                } else if (mode == "solid_cancel") {
                    Check(terminal.Category == "cancelled" && terminal.FilesCommitted == 0 && File.ReadAllText(old) == "old-target" && !File.Exists(Path.Combine(output.FullName, "c.txt")), "cancel stops solid later publication");
                } else {
                    var skipped = mode == "solid_skip" || mode == "tar_skip" ? 2UL : skipping ? 1UL : 0UL;
                    var committed = mode == "solid_skip" ? 1 : mode == "tar_skip" || mode is "rar4_skip" or "rar5_skip" ? 0 : rar ? 1 : 2;
                    Check(terminal.FilesSkipped == skipped && terminal.FilesCommitted == committed && terminal.Category == (skipping ? "succeeded_with_warnings" : "success"), "true skip/commit/category: " + mode);
                    var summary = frames.Single(f => f.GetProperty("op").GetString() == "extraction_summary").GetProperty("extraction_summary");
                    Check(summary.GetProperty("files_skipped").GetUInt64() == skipped && summary.GetProperty("all_members_written").GetBoolean() == !skipping, "format coverage summary");
                    if (skipping) {
                        Check(File.ReadAllText(old) == "old-target" && eligibility?.GetProperty("reason").GetString() == "members_skipped" && backend.Calls == 0, "Skip retains target/source and disqualifies deletion");
                        Check(writes == (mode == "solid_skip" ? 1 : 0), "skipped member creates no partial writes");
                    } else if (rar) Check(File.ReadAllText(old) == expected, "RAR actual native overwrite exact independent bytes");
                    else Check(File.ReadAllText(old) == "old-target" && File.ReadAllText(Path.Combine(output.FullName, "a (2).txt")) == expected && File.ReadAllText(Path.Combine(output.FullName, "b.txt")) == "incoming-b.txt", "TAR numbered native bytes");
                    if (mode == "solid_skip") Check(File.ReadAllText(Path.Combine(output.FullName, "b.txt")) == "old-second" && File.ReadAllText(Path.Combine(output.FullName, "c.txt")) == "incoming-c.txt" && Prompts(events).Length == 1, "solid discard preserves state/apply-all once/following member");
                }
                Check(File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(output.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 1, "only owned partials cleaned");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            }
        } finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
