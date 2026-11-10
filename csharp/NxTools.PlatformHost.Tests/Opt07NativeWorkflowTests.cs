using System.Collections.Concurrent;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned AES ZIP copies, original Job session, worker processes and native output owners.
internal static class Opt07NativeWorkflowTests
{
    private static void Check(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }

    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Records = new();
        public void Write(string level, string name, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) =>
            Records.Enqueue(name + " " + correlationId + " " + JsonSerializer.Serialize(fields));
    }

    private static ProtocolEnvelope Wire(ArchiveJobCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);

    public static async Task ActualBatchPasswordReuseAndRetirement()
    {
        var scratch = Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ??
            throw new InvalidOperationException("Session scratch required"));
        var root = Path.Combine(scratch, "archive-product-opt07-native-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        try {
            var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE") ??
                throw new InvalidOperationException("Parent-built worker required");
            Check(Path.IsPathFullyQualified(worker) && File.Exists(worker), "Explicit current worker required");
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var fixture = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory,
                "../../../../../rust/crates/nxtools-archive/tests/fixtures/combo-ae2-store-256.zip"));
            var sourceBytes = File.ReadAllBytes(fixture);
            var paths = new[] { "first.zip", "second.zip", "isolated.zip" }
                .Select(name => Path.Combine(root, name)).ToArray();
            foreach (var path in paths) File.WriteAllBytes(path, sourceBytes);
            var output = Path.Combine(root, "output");
            Directory.CreateDirectory(output);
            var batch = Guid.NewGuid();
            var log = new CaptureLog();
            await using (var jobs = ArchiveJobSession.ForOwnedTestPackage(root, log)) {
                ArchiveJobCommand Start(string path, Guid batchId, string kind = "test", ulong generation = 1, Guid? session = null) => new() {
                    Version = 1, OperationId = Guid.NewGuid(), SessionId = session ?? Guid.NewGuid(), Generation = generation,
                    Action = "start", BatchId = batchId,
                    Request = new() { Kind = kind, Paths = [path], Destination = kind == "extract" ? output : null,
                        Extract = kind == "extract" ? new() { Strategy = "here", SingleFileFolder = false, Conflict = "ask", CopyMotw = false } : null }
                };
                async Task<ArchiveJobTerminal> Run(ArchiveJobCommand command, bool expectPrompt, bool remember = false)
                {
                    Check(jobs.Dispatch(Wire(command)).Success, "Original native member admission");
                    var prompts = 0;
                    ArchiveJobPrompt? answered = null;
                    while (true) {
                        var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20));
                        var current = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                        Check(current.OperationId == command.OperationId && current.SessionId == command.SessionId &&
                            current.Generation == command.Generation, "Exact original member/attempt event");
                        if (current.Prompt is {} prompt) {
                            prompts++;
                            Check(expectPrompt && prompts == 1 && prompt.Kind == "password" && prompt.ErrorCode == "PasswordRequired",
                                "Only the expected original password ticket may be published");
                            answered = prompt;
                            var reply = new ArchiveJobReply { Ticket = prompt.Ticket, Kind = "password", ApplyToAll = false,
                                Password = Encoding.UTF8.GetBytes("synthetic-private-ZIP-secret"), RememberBatchPassword = remember };
                            Check(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = reply })).Success,
                                "Original protected password response");
                        }
                        if (current.Terminal is not {} terminal) continue;
                        Check(prompts == (expectPrompt ? 1 : 0), "Explicit prompt count, not inferred reuse");
                        Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete && terminal.FilesVerified > 0,
                            "Actual worker/codec completion and cleanup: " + terminal.Code);
                        if (answered is not null) {
                            var stale = command with { Action = "reply", Request = null,
                                Reply = new() { Ticket = answered.Ticket, Kind = "password", Password = Encoding.UTF8.GetBytes("synthetic-private-ZIP-secret"),
                                    ApplyToAll = false, RememberBatchPassword = remember } };
                            Check(!jobs.Dispatch(Wire(stale)).Success, "Finished native ticket cannot be reused");
                        }
                        return terminal;
                    }
                }
                var first = Start(paths[0], batch);
                var tested = await Run(first, expectPrompt: true, remember: true);
                Check(tested.OutputPublished == false && tested.FilesCommitted == 0, "Test does not publish output");
                var extracted = await Run(Start(paths[1], batch, "extract"), expectPrompt: false);
                Check(extracted.OutputPublished == true && extracted.FilesCommitted > 0 && extracted.ResultLocation is not null,
                    "Reused password reaches original actual extraction and location facts");
                await Run(Start(paths[2], Guid.NewGuid()), expectPrompt: true);
                var retire = new ArchiveJobCommand { Version = 1, OperationId = batch, SessionId = batch, Generation = 1,
                    Action = "retire_batch", BatchId = batch };
                Check(jobs.Dispatch(Wire(retire)).Success, "Original serialized batch retirement");
                await Run(Start(paths[0], batch), expectPrompt: true);
                await Run(Start(paths[0], batch, generation: 2, session: first.SessionId), expectPrompt: true);
            }
            foreach (var path in paths) {
                Check(File.ReadAllBytes(path).AsSpan().SequenceEqual(sourceBytes), "Owned encrypted source retained byte-for-byte");
                using var reopened = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            var outputs = Directory.GetFiles(output, "*", SearchOption.AllDirectories);
            Check(outputs.Length > 0, "Actual extracted files retained");
            foreach (var path in outputs) {
                using var reopened = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            Check(!Directory.EnumerateFiles(root, "*.partial", SearchOption.AllDirectories).Any(), "No retained partial output");
            Check(log.Records.Count(line => line.StartsWith("archive.job.batch_password_remembered ", StringComparison.Ordinal)) == 1 &&
                log.Records.Count(line => line.StartsWith("archive.job.batch_password_reused ", StringComparison.Ordinal)) == 1,
                "Exactly one protected candidate and one actual member reuse");
            Check(log.Records.All(line => !line.Contains("synthetic-private-ZIP-secret", StringComparison.Ordinal) &&
                !line.Contains(root, StringComparison.OrdinalIgnoreCase) && !line.Contains("first.zip", StringComparison.Ordinal)),
                "Native lifecycle diagnostics contain no secret or source path");
        }
        finally { Directory.Delete(root, true); }
    }
}
