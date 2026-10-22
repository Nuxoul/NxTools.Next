using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Original read -> original single-entry Job/worker/cache. No Shell or executable payload.
internal static class ArchiveEncryptedBrowserTests
{
    private static void Check(bool condition, string reason) { if (!condition) throw new InvalidOperationException(reason); }
    private sealed class Capture : NxTools.PlatformHost.Diagnostics.IPlatformLog {
        internal readonly System.Collections.Concurrent.ConcurrentQueue<string> Events = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) {
            if (eventName is "archive.read.frame_rejected" or "archive.worker.terminal") Events.Enqueue(eventName + ":" + JsonSerializer.Serialize(fields));
        }
    }

    public static async Task EncryptedSelectedZipUsesOriginalPrivatePasswordWait()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Matching isolated worker and tiny encrypted ZIP fixture required.");
        var root = ArchiveProductReadTests.Root();
        try {
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var source = Path.Combine(root, "encrypted.zip");
            File.Copy(Path.Combine(fixtures, "independent-ae2-256.zip"), source);
            await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
            var open = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
            var identity = ArchiveReadWire.Parse(open);
            Check(read.Dispatch(open).Success, "encrypted metadata read admission");
            ArchiveReadTerminal? terminal = null;
            ulong? selected = null;
            while (terminal is null) {
                var frame = await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) {
                    selected ??= batch.Entries.FirstOrDefault(entry => entry.Encrypted == true && entry.Kind == "File" && entry.Name.EndsWith("payload.bin", StringComparison.Ordinal))?.Ordinal;
                    Check(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original read credit");
                }
                terminal = item.Terminal;
            }
            Check(selected is not null && terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "encrypted browser identity");
            Check(terminal.Browser!.Format == "Zip", "small ZIP source binding");
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = identity.Generation,
                Action = "materialize", Path = source, Ordinal = selected, Browser = terminal.Browser, CopyMotw = false, PasswordInteraction = true };
            ProtocolEnvelope Envelope(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
            var pending = entries.Dispatch(Envelope(command));
            var clock = Stopwatch.StartNew();
            while (entries.PendingPassword(command.OperationId) is null && !pending.IsCompleted) {
                Check(clock.Elapsed < TimeSpan.FromSeconds(15), "missing original private password prompt");
                await Task.Delay(10);
            }
            var ticket = entries.PendingPassword(command.OperationId) ?? throw new InvalidOperationException("password wait ended before prompt");
            Check(ticket.OperationId == command.OperationId && ticket.Generation == 1 && !pending.IsCompleted, "original ticket and held worker");
            Check(!entries.RespondPassword(command.OperationId, command.SessionId, command.Generation, ticket with { PromptId = Guid.NewGuid() }, "wrong-ticket"), "stale/foreign ticket accepted");
            Check(entries.RespondPassword(command.OperationId, command.SessionId, command.Generation, ticket, "synthetic-private-ZIP-secret"), "private typed answer");
            var reply = await pending.WaitAsync(TimeSpan.FromSeconds(40));
            var result = JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
            Check(!entries.RespondPassword(command.OperationId, command.SessionId, command.Generation, ticket, "late-secret"), "retired prompt accepted");
            Check(reply.Success && result.CleanupComplete && result.Path is not null && File.Exists(result.Path), "actual selected encrypted output: " + result.Code);
            var expected = System.Text.Encoding.UTF8.GetBytes("独立 ZIP crypto fixture\n").Concat(Enumerable.Range(0, 1024).Select(index => (byte)index)).ToArray();
            Check(File.ReadAllBytes(result.Path!).SequenceEqual(expected), "selected AES payload bytes and integrity");
            Check(!Directory.EnumerateFiles(Path.GetDirectoryName(result.Path!)!).Any(path => Path.GetFileName(path) != Path.GetFileName(result.Path)), "unselected file published");
            var release = command with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, PasswordInteraction = false, LeaseId = result.LeaseId };
            Check((await entries.Dispatch(Envelope(release))).Success && !File.Exists(result.Path), "original owned lease retirement");
        } finally { Directory.Delete(root, true); }
    }
    public static async Task HeaderEncryptedRarReadCancelAndSelected()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Matching worker and tiny RAR5 fixture required.");
        var root = ArchiveProductReadTests.Root();
        try {
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var source = Path.Combine(root, "encrypted.rar"); File.Copy(Path.Combine(fixtures, "rar5-header-encrypted.rar"), source);
            foreach (var cancel in new[] { true, false }) {
                var capture = new Capture();
                await using var read = ArchiveReadSession.ForOwnedTestPackage(root, capture);
                await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
                var open = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
                var identity = ArchiveReadWire.Parse(open); Check(read.Dispatch(open, browserInteraction: true).Success, "browser private read admission");
                ArchiveReadTerminal? terminal = null; ulong? selected = null; var prompts = 0; ArchiveBrowserPasswordPrompt? old = null;
                while (terminal is null) {
                    var frame = await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                    if (frame.Payload.GetProperty("event").GetString() == "archive_password") {
                        var prompt = frame.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!;
                        prompts++; Check(prompt.OperationId == identity.OperationId && prompt.SessionId == identity.SessionId && prompt.Generation == identity.Generation, "read prompt owner");
                        Check(!read.RespondPassword(prompt with { Generation = prompt.Generation + 1 }, "password"), "old view generation accepted");
                        if (old is not null) Check(!read.RespondPassword(old, "password"), "old ticket accepted");
                        Check(prompt.ErrorCode == (prompts == 1 ? "PasswordRequired" : "WrongPassword"), "typed password classification");
                        var text = cancel ? null : prompts == 1 ? "incorrect-fixture-secret" : "password";
                        var reply = ProtocolEnvelope.Create("platform_command", new { command = "archive_password_reply", data = new { prompt, password = text is null ? null : System.Text.Encoding.UTF8.GetBytes(text).Select(value => (int)value).ToArray() } }, prompt.OperationId);
                        Check(ArchiveBrowserPassword.Respond(reply, read, entries), "existing private ticket response"); old = prompt;
                    } else {
                        var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                        if (item.Batch is { } batch) { selected ??= batch.Entries.FirstOrDefault(entry => entry.Kind == "File")?.Ordinal; Check(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "credit"); }
                        terminal = item.Terminal;
                    }
                }
                Check(terminal.CleanupComplete && prompts == (cancel ? 1 : 2), "finite password read cleanup");
                if (cancel) { Check(terminal.Status == "cancelled" && terminal.Browser is null, "cancel must not publish browse authority"); continue; }
                Check(terminal.Status == "succeeded" && terminal.Browser is not null && selected is not null, "header decrypted metadata: " + terminal.Status + "/" + terminal.Code + "/browser=" + (terminal.Browser is not null) + "/selected=" + selected.HasValue + "/" + string.Join(";", capture.Events));
                var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = identity.Generation, Action = "materialize", Path = source, Ordinal = selected, Browser = terminal.Browser, CopyMotw = false, PasswordInteraction = true };
                ProtocolEnvelope Envelope(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
                var pending = entries.Dispatch(Envelope(command)); var timer = Stopwatch.StartNew();
                while (entries.PendingPassword(command.OperationId) is null && !pending.IsCompleted) { Check(timer.Elapsed < TimeSpan.FromSeconds(20), "materialize prompt"); await Task.Delay(10); }
                var ticket = entries.PendingPassword(command.OperationId) ?? throw new InvalidOperationException("selected RAR private prompt missing");
                Check(entries.RespondPassword(command.OperationId, command.SessionId, command.Generation, ticket, "password"), "selected RAR response");
                var result = JsonSerializer.Deserialize<ArchiveEntryResult>((await pending.WaitAsync(TimeSpan.FromSeconds(40))).Data!, JsonDefaults.Options)!;
                Check(result.Success && result.CleanupComplete && result.Path is not null && File.Exists(result.Path), "selected RAR success: " + result.Code);
                var release = command with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, PasswordInteraction = false, LeaseId = result.LeaseId };
                Check((await entries.Dispatch(Envelope(release))).Success, "selected RAR release");
            }
        } finally { Directory.Delete(root, true); }
    }
}
