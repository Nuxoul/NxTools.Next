using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned tiny ZIP/output, actual original runner/Job/worker; no user files or Shell.
internal static class ArchiveConflictTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
            => Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    private static JsonElement[] Prompts(ConcurrentQueue<ArchiveWorkerEvent> events) => events
        .Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "conflict_prompt")
        .Select(e => e.Frame!.Value.GetProperty("conflict_prompt")).ToArray();
    private static async Task Until(Func<bool> done, string detail)
    {
        var clock = Stopwatch.StartNew();
        while (!done()) { Check(clock.ElapsedMilliseconds < 10000, detail); await Task.Delay(10); }
    }
    public static async Task ActualZipConflictDecisions()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit matching isolated worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Check(!File.Exists(helper), "refuse existing helper");
        var root = Directory.CreateTempSubdirectory("archive-conflict-"); var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            ArchivePasswordTicket? priorJobTicket = null;
            foreach (var mode in new[] { "ask_overwrite", "skip", "rename", "overwrite_policy", "skip_policy", "rename_policy", "apply_overwrite", "apply_skip", "apply_rename", "next_job", "changed", "cancel", "cancel_after_commit", "rename_race", "internal_collision", "reserved_rename" }) {
                var owned = Directory.CreateDirectory(Path.Combine(root.FullName, mode));
                var source = Path.Combine(owned.FullName, "tiny.zip");
                var two = mode.StartsWith("apply_", StringComparison.Ordinal) || mode is "next_job" or "cancel" or "cancel_after_commit" or "internal_collision" or "reserved_rename";
                using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) {
                    var names = mode == "internal_collision" ? new[] { "a.txt", "A.TXT" } : mode == "reserved_rename" ? new[] { "a.txt", "a (2).txt" } : two ? new[] { "a.txt", "b.txt" } : new[] { "a.txt" };
                    foreach (var name in names) { using var stream = zip.CreateEntry(name, CompressionLevel.NoCompression).Open(); stream.Write(Encoding.UTF8.GetBytes("incoming-" + name)); }
                }
                var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
                var oldA = Path.Combine(output.FullName, "a.txt"); var oldB = Path.Combine(output.FullName, "b.txt");
                if (mode != "cancel_after_commit") File.WriteAllText(oldA, "original-a");
                if (two && mode is not ("internal_collision" or "reserved_rename")) File.WriteAllText(oldB, "original-b");
                var foreignPartial = Path.Combine(output.FullName, ".nxpart-foreign"); File.WriteAllText(foreignPartial, "foreign");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
                var id = Guid.NewGuid();
                var policy = mode == "overwrite_policy" ? "overwrite" : mode == "skip_policy" ? "skip" : mode == "rename_policy" ? "auto_rename" : "ask";
                var raced = false;
                runner.OutputCommitCheckpointProbe = stage => {
                    if (mode == "rename_race" && stage == "before_flush" && !raced) { raced = true; File.WriteAllText(Path.Combine(output.FullName, "a (2).txt"), "external-racer"); }
                };
                Check(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: output.FullName, ConflictPolicy: policy)), "actual ZIP conflict admission");
                var expectsPrompt = policy == "ask" && mode != "internal_collision";
                if (expectsPrompt) {
                    await Until(() => Prompts(events).Length == 1 || events.Any(e => e.Kind == "exited"), "first conflict deadline");
                    Check(Prompts(events).Length == 1, "ordinary target did not enter AwaitingConflict: " + mode);
                    Check(runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "original slot held while pending");
                    var prompt = Prompts(events)[0]; var ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Check(ticket.OperationId == id, "original logical task ticket");
                    Check(prompt.GetProperty("existing_bytes").GetUInt64() == 10, "observed target size");
                    Check(prompt.GetProperty("display").GetProperty("incoming_member").GetString()==(mode=="cancel_after_commit"?"b.txt":"a.txt") && prompt.GetProperty("display").GetProperty("target").GetString()==(mode=="cancel_after_commit"?"b.txt":"a.txt"),"current worker object snapshot");
                    if (mode == "next_job" && priorJobTicket is not null) Check(!runner.RespondConflict(id, priorJobTicket, "overwrite", true), "prior task ticket rejected");
                    if (mode == "changed") File.WriteAllText(oldA, "external-change");
                    if (mode is "cancel" or "cancel_after_commit") {
                        Check(runner.RespondConflict(id, ticket, null), "typed conflict Cancel");
                    } else {
                        var decision = mode is "skip" or "apply_skip" or "next_job" ? "skip" : mode is "rename" or "rename_race" or "apply_rename" or "reserved_rename" ? "auto_rename" : "overwrite";
                        Check(runner.RespondConflict(id, ticket, decision, mode.StartsWith("apply_", StringComparison.Ordinal)), "typed decision");
                        if (mode == "apply_overwrite") priorJobTicket = ticket;
                        if (mode == "next_job") {
                            await Until(() => Prompts(events).Length == 2 || events.Any(e => e.Kind == "exited"), "next conflict deadline");
                            Check(Prompts(events).Length == 2, "apply-all leaked from prior logical task");
                            var next = Prompts(events)[1].GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                            Check(next != ticket, "fresh prompt ticket");
                            Check(Prompts(events)[1].GetProperty("display").GetProperty("incoming_member").GetString()=="b.txt" && Prompts(events)[1].GetProperty("display").GetProperty("target").GetString()=="b.txt","second ticket has second object, not previous size/progress");
                            Check(runner.RespondConflict(id, ticket, "overwrite"), "old answer delivered for Rust rejection");
                            Check(runner.RespondConflict(id, next, "skip"), "current second Skip");
                        }
                    }
                }
                await Until(() => events.Any(e => e.Kind == "exited"), "original terminal deadline: " + mode);
                Check(await runner.DrainAsync(), "original resource drain");
                var terminal = events.Single(e => e.Kind == "exited");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0, "actual cleanup: " + mode);
                Check(!runner.RespondConflict(id, Prompts(events).FirstOrDefault().ValueKind == JsonValueKind.Object ? Prompts(events)[0].GetProperty("ticket").Deserialize<ArchivePasswordTicket>()! : new(id, Guid.NewGuid(), 1), "overwrite"), "retired ticket rejected");
                var skipped = mode is "skip" or "skip_policy" ? 1UL : mode is "apply_skip" or "next_job" ? 2UL : 0UL;
                Check(terminal.FilesSkipped == skipped, "accurate Skip count: " + mode);
                if (mode == "changed") {
                    Check(terminal.Category == "target_changed" && terminal.FilesCommitted == 0 && File.ReadAllText(oldA) == "external-change", "old approval cannot replace changed target");
                } else if (mode is "cancel" or "cancel_after_commit") {
                    Check(terminal.FilesCommitted == (mode == "cancel_after_commit" ? 1 : 0) && terminal.Category == (mode == "cancel_after_commit" ? "cancelled_with_committed_output" : "cancelled"), "truthful cancellation count");
                    Check(File.ReadAllText(oldA) == (mode == "cancel_after_commit" ? "incoming-a.txt" : "original-a") && File.ReadAllText(oldB) == "original-b", "cancel retains previous commits and old targets");
                } else if (mode is "internal_collision" or "rename_race") {
                    Check(Prompts(events).Length == (mode == "internal_collision" ? 0 : 1) && terminal.FilesCommitted == 0, "refusal before publication");
                    var failure = events.Single(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure").Frame!.Value;
                    Check(failure.GetProperty("error_code").GetString() == "OutputCollision", "typed internal/raced collision");
                    if (mode == "rename_race") Check(File.ReadAllText(Path.Combine(output.FullName, "a (2).txt")) == "external-racer", "no-replace race retained external output");
                } else {
                    Check(terminal.Category == (skipped > 0 ? "succeeded_with_warnings" : "success") && terminal.FilesCommitted == (two ? 2 - (int)skipped : 1 - (int)skipped), "truthful output terminal: " + mode);
                    if (skipped > 0) Check(File.ReadAllText(oldA) == "original-a", "Skip retained old bytes");
                    else if (mode is "rename" or "rename_policy" or "apply_rename" or "reserved_rename") {
                        Check(File.ReadAllText(oldA) == "original-a", "rename retained original");
                        Check(File.ReadAllText(Path.Combine(output.FullName, mode == "reserved_rename" ? "a (3).txt" : "a (2).txt")) == "incoming-a.txt", "deterministic same-directory numbered name");
                        if (mode == "apply_rename") Check(File.ReadAllText(Path.Combine(output.FullName, "b (2).txt")) == "incoming-b.txt", "rename apply-all");
                        if (mode == "reserved_rename") Check(File.ReadAllText(Path.Combine(output.FullName, "a (2).txt")) == "incoming-a (2).txt", "planned original name not stolen by rename");
                    } else {
                        Check(File.ReadAllText(oldA) == "incoming-a.txt", "native overwrite exact bytes");
                        if (two) Check(File.ReadAllText(oldB) == "incoming-b.txt", "second native overwrite exact bytes");
                    }
                    var summary = events.Single(e => e.Frame is { } f && f.GetProperty("op").GetString() == "extraction_summary").Frame!.Value.GetProperty("extraction_summary");
                    Check(summary.GetProperty("files_skipped").GetUInt64() == skipped && summary.GetProperty("all_members_written").GetBoolean() == (skipped == 0), "Rust summary exact coverage");
                }
                if (mode.StartsWith("apply_", StringComparison.Ordinal)) Check(Prompts(events).Length == 1, "apply-all asks once per logical task");
                if (policy != "ask") Check(Prompts(events).Length == 0, "explicit strategy should not Ask");
                Check(File.ReadAllText(foreignPartial) == "foreign" && Directory.GetFiles(output.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 1, "owned partial cleanup preserves foreign file");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                var text = JsonSerializer.Serialize(log.Records.Select(r => new { r.Name, r.Id, r.Fields }));
                Check(!text.Contains(root.FullName) && !text.Contains("original-a") && !text.Contains("incoming-a"), "conflict diagnostic privacy");
            }
        } finally { if (copied) File.Delete(helper); root.Delete(true); }
    }
}
