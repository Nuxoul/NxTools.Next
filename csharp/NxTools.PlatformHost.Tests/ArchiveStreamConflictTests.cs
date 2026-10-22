using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Independent tiny stream inputs; original runner/Job/worker/native publication. No Shell/delete.
internal static class ArchiveStreamConflictTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class NeverRecycle : IArchiveRecycleBackend {
        internal int Calls;
        public ArchiveRecycleNativeResult Recycle(ArchiveRecycleSource source, CancellationToken cancellation) {
            Calls++; throw new InvalidOperationException("Skipped stream cannot authorize source recycling");
        }
    }
    private static JsonElement[] Prompts(ConcurrentQueue<ArchiveWorkerEvent> events) => events
        .Where(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "conflict_prompt")
        .Select(e => e.Frame!.Value.GetProperty("conflict_prompt")).ToArray();
    private static async Task Until(Func<bool> done) {
        var clock = Stopwatch.StartNew();
        while (!done()) { Check(clock.ElapsedMilliseconds < 15000, "stream conflict deadline"); await Task.Delay(10); }
    }
    public static async Task ActualStandaloneConflicts()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit matching worker and independent tiny stream fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Check(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-stream-conflict-"); var copied = false;
        var expected = Encoding.UTF8.GetBytes("独立 gzip fixture\n").Concat(Enumerable.Range(0, 256).Select(i => (byte)i)).ToArray();
        try {
            File.Copy(worker, helper); copied = true;
            foreach (var mode in new[] { "gzip_ask", "xz_rename", "bzip_skip", "zstd_overwrite", "gzip_cancel", "gzip_skip_crc", "bzip_skip_budget" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
                var extension = mode.StartsWith("gzip", StringComparison.Ordinal) ? "gz" : mode.StartsWith("bzip", StringComparison.Ordinal) ? "bz2" : mode == "xz_rename" ? "xz" : "zst";
                var input = File.ReadAllBytes(Path.Combine(fixtures, extension == "zst" ? "checked.zst" : "independent." + extension));
                if (mode == "gzip_skip_crc") input[^8] ^= 1;
                var source = Path.Combine(root.FullName, "payload." + extension); File.WriteAllBytes(source, input);
                var output = Directory.CreateDirectory(Path.Combine(root.FullName, "output"));
                var target = Path.Combine(output.FullName, "payload"); File.WriteAllText(target, "old-target");
                var foreign = Path.Combine(output.FullName, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new NeverRecycle();
                JsonElement? eligibility = null;
                var skipping = mode.Contains("skip", StringComparison.Ordinal);
                using var runner = skipping ? ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend,
                    evidenceProbe: evidence => { eligibility = evidence.Clone(); return evidence; }) : ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var writes = 0; runner.OutputWriteCheckpointProbe = _ => writes++;
                var id = Guid.NewGuid();
                var policy = skipping ? "skip" : mode == "xz_rename" ? "auto_rename" : mode == "zstd_overwrite" ? "overwrite" : "ask";
                Check(runner.Start(new(id, "extract", source, 0, new { max_entries = 100, max_entry_bytes = mode == "bzip_skip_budget" ? 1 : 1048576 },
                    OutputRoot: output.FullName, ConflictPolicy: policy, DeleteSource: skipping ? true : null, ExtractOptions: new(false))), "original stream admission: " + mode);
                if (policy == "ask") {
                    await Until(() => Prompts(events).Length != 0 || events.Any(e => e.Kind == "exited"));
                    Check(Prompts(events).Length == 1, "standalone stream must enter existing AwaitingConflict: " + mode);
                    Check(runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited") && writes == 0 &&
                        Directory.GetFiles(output.FullName, ".nxpart-*").Length == 1 && File.ReadAllText(target) == "old-target", "wait retains slot and creates no current partial");
                    var ticket = Prompts(events)[0].GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Check(Prompts(events)[0].GetProperty("incoming_bytes").ValueKind == JsonValueKind.Null, "unknown stream expanded size stays unknown");
                    Check(runner.RespondConflict(id, ticket, mode == "gzip_cancel" ? null : "overwrite"), "typed existing conflict reply");
                }
                await Until(() => events.Any(e => e.Kind == "exited")); Check(await runner.DrainAsync(), "original stream drain");
                var terminal = events.Single(e => e.Kind == "exited");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0, "actual stream cleanup/slot release");
                var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                var summary = frames.Single(frame => frame.GetProperty("op").GetString() == "extraction_summary").GetProperty("extraction_summary");
                Check(terminal.FilesSkipped == (skipping ? 1UL : 0UL) && summary.GetProperty("files_skipped").GetUInt64() == (skipping ? 1UL : 0UL), "exact stream skip count");
                var failure = mode is "gzip_skip_crc" or "bzip_skip_budget";
                var cancelled = mode == "gzip_cancel";
                var publishes = !skipping && !cancelled;
                Check(terminal.FilesCommitted == (publishes ? 1 : 0) && terminal.Category == (failure ? mode == "bzip_skip_budget" ? "resource_limit" : "worker_failure" : cancelled ? "cancelled" : skipping ? "succeeded_with_warnings" : "success"), "true stream terminal: " + mode);
                Check(summary.GetProperty("all_members_written").GetBoolean() == publishes, "skipped/unverified stream is not full coverage");
                if (failure) {
                    Check(frames.Single(frame => frame.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == (mode == "gzip_skip_crc" ? "CorruptArchive" : "ResourceLimit"), "Skip still consumes/checks CRC and actual bytes");
                } else if (!cancelled) {
                    var complete = frames.Single(frame => frame.GetProperty("op").GetString() == "complete");
                    Check(complete.GetProperty("files_verified").GetUInt64() == (publishes ? 1UL : 0UL) && complete.GetProperty("bytes_verified").GetUInt64() == (publishes ? (ulong)expected.Length : 0UL), "published-byte counts do not pretend skipped payload is published");
                }
                if (publishes) {
                    var final = mode == "xz_rename" ? Path.Combine(output.FullName, "payload (2)") : target;
                    Check(File.ReadAllBytes(final).SequenceEqual(expected) && terminal.CommitState == "Committed", "exact independent stream native bytes");
                }
                if (!publishes || mode == "xz_rename") Check(File.ReadAllText(target) == "old-target", "old stream target retained");
                if (skipping) {
                    Check(writes == 0 && backend.Calls == 0 && File.Exists(source), "Skip creates no partial writes or delete authority");
                    if (!failure) Check(eligibility?.GetProperty("reason").GetString() == "members_skipped", "Skip disqualifies source");
                    else Check(eligibility?.GetProperty("reason").GetString() != "eligible", "failed Skip cannot qualify source");
                }
                Check(File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(output.FullName, ".nxpart-*").Length == 1, "owned cleanup retains external partial");
                Check(File.ReadAllBytes(source).SequenceEqual(input), "source unchanged");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            }
        } finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
