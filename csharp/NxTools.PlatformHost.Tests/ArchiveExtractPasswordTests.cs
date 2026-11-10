using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Original runner/Job/worker/native outputs, attributed tiny encrypted fixtures; no Shell/delete.
internal static class ArchiveExtractPasswordTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog {
        internal readonly ConcurrentQueue<string> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
            => Records.Enqueue(JsonSerializer.Serialize(new { eventName, correlationId, fields }));
    }
    private static JsonElement[] States(ConcurrentQueue<ArchiveWorkerEvent> events, string state) => events
        .Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "password_state" && f.GetProperty("password_state").GetProperty("state").GetString() == state)
        .Select(e => e.Frame!.Value.GetProperty("password_state")).ToArray();
    private static async Task Until(Func<bool> done) {
        var clock = Stopwatch.StartNew(); while (!done()) { Check(clock.ElapsedMilliseconds < 15000, "encrypted extraction deadline"); await Task.Delay(10); }
    }
    public static async Task ActualEncryptedExtraction()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        var codec = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures) || string.IsNullOrEmpty(codec))
            throw new TestSkippedException("Matching worker and attributed tiny encrypted fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Check(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-extract-password-"); var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            foreach (var mode in new[] { "zip", "sevenz", "rar4", "rar5", "wrong_then_correct", "cancel", "zip_conflict", "damaged", "after_commit_cancel" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, mode));
                var zip = mode is "zip" or "zip_conflict" or "after_commit_cancel";
                var sevenz = mode == "sevenz";
                var original = zip ? Path.Combine(fixtures, "independent-ae2-256.zip") : Path.Combine(codec, sevenz ? "encrypted.7z" : mode == "rar4" ? "rar4-encrypted.rar" : "rar5-header-encrypted.rar");
                var source = Path.Combine(root.FullName, "private" + Path.GetExtension(original)); File.Copy(original, source);
                if (mode == "damaged") { var data = File.ReadAllBytes(source); data[^30] ^= 1; File.WriteAllBytes(source, data); }
                var output = Directory.CreateDirectory(Path.Combine(root.FullName, "output"));
                var foreign = Path.Combine(output.FullName, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                var old = Path.Combine(output.FullName, "old-target.txt"); File.WriteAllText(old, "old-target");
                if (mode == "zip_conflict") { Directory.CreateDirectory(Path.Combine(output.FullName, "资料")); File.WriteAllText(Path.Combine(output.FullName, "资料", "payload.bin"), "original-payload"); }
                var before = Directory.GetFileSystemEntries(output.FullName, "*", SearchOption.AllDirectories).Order().ToArray();
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
                var id = Guid.NewGuid(); var writes = 0; var published = 0;
                runner.OutputWriteCheckpointProbe = _ => { Check(States(events, "extracting").Length == 1, "only password-approved single extraction can write"); writes++; };
                runner.OutputCommitCheckpointProbe = stage => {
                    if (stage == "native_closed") { published++; if (mode == "after_commit_cancel" && published == 1) Check(runner.Cancel(id), "cancel after actual close"); }
                };
                Check(runner.Start(new(id, "extract", source, 936, new { max_entries = 100, max_entry_bytes = 1048576, max_output_bytes = 1048576 },
                    OutputRoot: output.FullName, PasswordInteraction: true, ConflictPolicy: mode == "zip_conflict" ? "auto_rename" : "ask", ExtractOptions: new(false))), "encrypted extraction password admission");
                await Until(() => States(events, "awaiting_password").Length == 1 || events.Any(e => e.Kind == "exited"));
                Check(States(events, "awaiting_password").Length == 1, "encrypted extract must enter real AwaitingPassword: " + mode);
                Check(writes == 0 && published == 0 && States(events, "extracting").Length == 0 && before.SequenceEqual(Directory.GetFileSystemEntries(output.FullName, "*", SearchOption.AllDirectories).Order()), "password preparation has zero output effects");
                Check(runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "original slot held during password wait");
                try { using var denied = File.Open(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source lease released during password wait"); } catch (IOException) { }
                try { using var denied = File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None); throw new InvalidOperationException("helper released during password wait"); } catch (IOException) { }
                var first = States(events, "awaiting_password")[0];
                Check(first.GetProperty("error_code").GetString() == "PasswordRequired", "explicit missing-password category");
                var ticket = first.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                var secret = zip ? "synthetic-private-ZIP-secret" : sevenz ? "sevenz-rust" : mode == "rar4" ? "unrar" : "password";
                Check(runner.RespondPassword(id, ticket, mode == "cancel" ? null : mode == "wrong_then_correct" ? "incorrect-extract-secret" : secret), "original typed password answer");
                if (mode == "wrong_then_correct") {
                    await Until(() => States(events, "awaiting_password").Length == 2 || events.Any(e => e.Kind == "exited"));
                    Check(States(events, "awaiting_password").Length == 2 && writes == 0 && published == 0, "explicit WrongPassword retries before all output");
                    var next = States(events, "awaiting_password")[1];
                    Check(next.GetProperty("error_code").GetString() == "WrongPassword", "RAR5 exact WrongPassword correction");
                    var current = next.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!; Check(current != ticket, "fresh password ticket");
                    Check(runner.RespondPassword(id, ticket, "stale-extract-secret"), "old ticket transported for existing Rust rejection");
                    Check(runner.RespondPassword(id, current, secret), "current corrected password");
                }
                await Until(() => events.Any(e => e.Kind == "exited")); Check(await runner.DrainAsync(), "actual encrypted extraction drain");
                var terminal = events.Single(e => e.Kind == "exited");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0 && events.Count(e => e.Kind == "started") == 1, "one worker/Job and actual resource cleanup");
                Check(events.All(e => e.OperationId == id), "native/business correlation retains original operation");
                Check(States(events, "running").Length == (mode == "cancel" ? 1 : mode == "wrong_then_correct" ? 3 : 2), "finite preparation attempts");
                Check(States(events, "running").Select(s => s.GetProperty("attempt_id").GetGuid()).Distinct().Count() == States(events, "running").Length && States(events, "running").All(s => s.GetProperty("attempt_id").GetGuid() != id), "new preparation IDs link original task");
                var extractionCount = States(events, "extracting").Length;
                Check(extractionCount == (mode is "cancel" or "damaged" ? 0 : 1), "actual extraction starts once and is never retried");
                if (mode is "cancel" or "damaged") {
                    Check(terminal.FilesCommitted == 0 && published == 0 && writes == 0 && before.SequenceEqual(Directory.GetFileSystemEntries(output.FullName, "*", SearchOption.AllDirectories).Order()), "failed/cancelled preparation never extracts");
                    var failure = events.Single(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure").Frame!.Value;
                    Check(failure.GetProperty("error_code").GetString() == (mode == "cancel" ? "Cancelled" : "AuthenticationOrDataError"), "truthful preparation terminal, no ambiguity retry");
                } else {
                    Check(terminal.Category == (mode == "after_commit_cancel" ? "cancelled_with_committed_output" : "success") && terminal.FilesCommitted == (mode == "after_commit_cancel" ? 1 : zip || sevenz ? 2 : 1), "native extraction terminal/count");
                    if (zip) {
                        var expected = Encoding.UTF8.GetBytes("独立 ZIP crypto fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
                        var payload = Path.Combine(output.FullName, "资料", mode == "zip_conflict" ? "payload (2).bin" : "payload.bin");
                        Check(File.ReadAllBytes(payload).SequenceEqual(expected), "AES ZIP independent native bytes");
                        if (mode == "zip_conflict") Check(File.ReadAllText(Path.Combine(output.FullName, "资料", "payload.bin")) == "original-payload", "original conflict strategy retained after password");
                    } else if (sevenz) {
                        foreach (var (name, size, hash) in new[] { ("7zFormat.txt", 2247L, "09ff37c5cdb9d30953442827f21457a8263baf7868471e5ae8317c743c84303b"), ("7ziplogo.png", 1417L, "f8ac2201f43c4989eb95b2184eee17a705a3107e43852859d5ce01e239d16ada") }) {
                            var data = File.ReadAllBytes(Path.Combine(output.FullName, "encripted", name));
                            Check(data.LongLength == size && Convert.ToHexString(SHA256.HashData(data)).ToLowerInvariant() == hash, "7z independent hash/native bytes");
                        }
                    } else Check(File.ReadAllText(Path.Combine(output.FullName, ".gitignore")) == "target\nCargo.lock\n", "RAR independent native bytes");
                }
                Check(File.ReadAllText(old) == "old-target" && File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(output.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 1, "owned cleanup preserves old/external files");
                var observable = JsonSerializer.Serialize(events.Where(e => e.Frame is not null).Select(e => e.Frame)) + string.Join("\n", log.Records);
                Check(!observable.Contains("incorrect-extract-secret") && !observable.Contains("stale-extract-secret") && !observable.Contains("synthetic-private-ZIP-secret") && !observable.Contains("\"password\":"), "no secret in events/diagnostics");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                using (File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            }
        } finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
