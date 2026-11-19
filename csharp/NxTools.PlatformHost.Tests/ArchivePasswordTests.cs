using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Actual isolated worker/Job and owned synthetic sources; no Shell or user archives.
internal static class ArchivePasswordTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
            => Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    private static JsonElement[] States(ConcurrentQueue<ArchiveWorkerEvent> events, string state) => events
        .Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "password_state" && f.GetProperty("password_state").GetProperty("state").GetString() == state)
        .Select(e => e.Frame!.Value.GetProperty("password_state")).ToArray();
    private static async Task<JsonElement> Prompt(ConcurrentQueue<ArchiveWorkerEvent> events, int count)
    {
        var clock = Stopwatch.StartNew();
        while (States(events, "awaiting_password").Length < count && !events.Any(e => e.Kind == "exited")) {
            Check(clock.ElapsedMilliseconds < 10000, "actual password pending deadline"); await Task.Delay(10);
        }
        Check(States(events, "awaiting_password").Length == count, "missing real pending password prompt");
        return States(events, "awaiting_password")[count - 1];
    }
    public static async Task ActualPasswordWaitRetryAndCancel()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit isolated worker and upstream codec fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Check(!File.Exists(helper), "refuse existing helper");
        var root = Directory.CreateTempSubdirectory("archive-password-"); var copied = false;
        try {
            File.Copy(worker, helper, false); copied = true;
            foreach (var mode in new[] { "correct", "open", "wrong_then_correct", "cancel_answer", "cancel_operation", "attempt_limit", "ambiguous" }) {
                var source = Path.Combine(root.FullName, mode + ".rar");
                File.Copy(Path.Combine(fixtures, "rar5-header-encrypted.rar"), source);
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
                var id = Guid.NewGuid();
                var request = new ArchiveWorkerRequest(id, mode == "open" ? "open" : "test", source, 936, new { max_entries = 100, max_entry_bytes = 100, max_output_bytes = 1048576 });
                if (mode == "ambiguous") {
                    // Existing independently documented RAR5 damaged-payload mutation, not bad-password guessing.
                    var damaged = File.ReadAllBytes(source); damaged[^30] ^= 1; File.WriteAllBytes(source, damaged);
                    request = request with { Password = "password" };
                }
                Check(runner.Start(request with { PasswordInteraction = true }), "original password operation admission");
                var expectedAttempts = mode == "ambiguous" ? 1 : mode is "cancel_answer" or "cancel_operation" ? 1 : mode is "correct" or "open" ? 2 : 3;
                if (mode != "ambiguous") {
                    var first = await Prompt(events, 1);
                    Check(first.GetProperty("error_code").GetString() == "PasswordRequired", "exact missing password error");
                    try { using var deniedHelper = File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None); throw new InvalidOperationException("helper image released during pending prompt"); } catch (IOException) { }
                    Check(runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "original slot held while awaiting password");
                    try { using var denied = File.Open(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source lease released during prompt"); } catch (IOException) { }
                    var ticket = first.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Check(ticket.OperationId == id && ticket.Generation == 1, "logical task/generation ticket");
                    if (mode == "cancel_operation") Check(runner.Cancel(id), "original cancel");
                    else Check(runner.RespondPassword(id, ticket, mode == "cancel_answer" ? null : mode is "correct" or "open" ? "password" : "incorrect-synthetic-secret"), "typed original response delivery");
                    if (mode is "wrong_then_correct" or "attempt_limit") {
                        var second = await Prompt(events, 2);
                        Check(second.GetProperty("error_code").GetString() == "WrongPassword", "only explicit wrong password permits retry");
                        var next = second.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                        Check(next != ticket && second.GetProperty("attempt_id").GetGuid() != first.GetProperty("attempt_id").GetGuid(), "fresh prompt and actual attempt IDs");
                        Check(runner.RespondPassword(id, ticket, "stale-synthetic-secret"), "old ticket transported for Rust rejection");
                        Check(runner.RespondPassword(id, next, mode == "attempt_limit" ? "incorrect-again" : "password"), "current manual retry");
                    }
                }
                var terminalClock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited")) {
                    Check(terminalClock.ElapsedMilliseconds < 10000, "original exited event deadline: " + mode); await Task.Delay(10);
                }
                Check(await runner.DrainAsync(), "original operation drain");
                var terminal = events.SingleOrDefault(e => e.Kind == "exited") ?? throw new InvalidOperationException("missing original exited terminal: " + mode);
                Check(terminal.CleanupComplete && runner.ActiveCount == 0 && terminal.FilesCommitted == 0, "real original resource/slot cleanup with no publication");
                Check(events.Count(e => e.Kind == "started") == 1 && States(events, "running").Length == expectedAttempts, "one original worker, finite attempts, no retry after cancel");
                var attemptIds = States(events, "running").Select(s => s.GetProperty("attempt_id").GetGuid()).ToArray();
                Check(attemptIds.Distinct().Count() == expectedAttempts && !attemptIds.Contains(id), "all actual attempts have new IDs linked to logical task");
                Check(events.All(e => e.OperationId == id), "original logical correlation retained");
                if (mode is "correct" or "open" or "wrong_then_correct") {
                    Check(terminal.Category == "success", "password operation success");
                    var complete = events.SingleOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "complete")?.Frame ?? throw new InvalidOperationException("missing actual complete frame: " + mode);
                    if (mode == "open") {
                        var batch = events.SingleOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "entry_batch")?.Frame ?? throw new InvalidOperationException("missing actual open batch: " + mode);
                        var entry = batch.GetProperty("entries")[0];
                        Check(entry.GetProperty("name").GetString() == ".gitignore" && entry.GetProperty("size").GetUInt64() == 18, "actual encrypted header metadata");
                    } else Check(complete.GetProperty("files_verified").GetUInt64() == 1 && complete.GetProperty("bytes_verified").GetUInt64() == 18, "actual decrypted content integrity verified, not open-only retry");
                } else {
                    var failure = events.SingleOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure")?.Frame ?? throw new InvalidOperationException("missing actual failure frame: " + mode + "/" + terminal.Category);
                    var expected = mode == "attempt_limit" ? "WrongPassword" : mode == "ambiguous" ? "AuthenticationOrDataError" : "Cancelled";
                    Check(failure.GetProperty("error_code").GetString() == expected, "truthful password terminal category: " + mode);
                    if (expected == "Cancelled") Check(terminal.Category == "cancelled", "password cancellation must remain cancelled at original runner terminal");
                    if (mode == "ambiguous") Check(States(events, "awaiting_password").Length == 0, "ambiguous authentication is not a password retry");
                }
                using (File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                var observed = JsonSerializer.Serialize(events.Where(e => e.Frame is not null).Select(e => e.Frame));
                Check(!observed.Contains("incorrect-synthetic-secret") && !observed.Contains("stale-synthetic-secret") && !observed.Contains("\"password\":"), "secret leaked into metadata/state/terminal");
                var diagnostic = JsonSerializer.Serialize(log.Records.Select(r => new { r.Name, r.Id, r.Fields }));
                Check(!diagnostic.Contains(source) && !diagnostic.Contains("synthetic-secret") && !diagnostic.Contains("\"password\":"), "secret/path leaked to diagnostics");
                Check(log.Records.Any(r => r.Name == "archive.worker.password.running" && r.Id == id), "correlated password lifecycle diagnostics");
            }
            Check(Directory.GetFiles(root.FullName).Length == 7 && !Directory.GetDirectories(root.FullName).Any(), "read-only attempts create no output");
        } finally { if (copied) File.Delete(helper); root.Delete(true); }
    }
}
