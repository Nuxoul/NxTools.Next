using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned scratch ZIPs and explicit matching worker; original browser/Run/Job/native
// replacement and deterministic established-lease cancellation only. No GUI/Shell/user sources.
internal static partial class Opt05NativeObservationTests
{
    private sealed class TerminalLog(bool reject) : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        internal int TerminalAttempts;
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (name is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed")
                Interlocked.Increment(ref TerminalAttempts);
            if (reject) throw new InvalidOperationException("owned diagnostic sink rejection");
            Records.Enqueue((name, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
        }
    }

    internal static async Task ActualTerminalObservations()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var expectedWorker = Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_WORKER_SHA256");
        if (worker is null || !File.Exists(worker) || expectedWorker is not { Length: 64 })
            throw new TestSkippedException("Explicit matching frozen worker required.");
        Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expectedWorker, StringComparison.OrdinalIgnoreCase), "matching worker");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch), "absolute scratch required");
        var failures = new List<string>();
        foreach (var mode in new[] { "success", "cancel_verification", "reject_log" })
        {
            var root = Path.Combine(scratch!, "archive-product-opt05-terminal-" + Guid.NewGuid());
            var archiveRoot = Directory.CreateDirectory(Path.Combine(root, "archive")).FullName;
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            ArchiveReadHelper.GuardOwnedRoot(root);
            var source = Path.Combine(archiveRoot, Target);
            var payload = new byte[256 << 10]; new Random(505).NextBytes(payload);
            var original = CreateZip(payload, "original.bin");
            File.WriteAllBytes(source, original);
            var browser = await TerminalBrowser(root, source);
            var id = Guid.NewGuid();
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            var log = new TerminalLog(mode == "reject_log");
            using var runner = ArchiveWorkerRunner.ForProductJob(ArchiveReadHelper.OwnedPackage(root), events.Enqueue,
                (_, _, _) => Task.FromResult<string?>(null), log);
            var interrupted = false;
            var publications = 0;
            runner.EditSourceReadChunkProbeForFixture = (stage, kind, bytes) => {
                if (mode != "cancel_verification" || stage != ArchiveNativeReadStage.EditVerification) return;
                Check(!interrupted && kind == ArchiveNativeReadKind.Source && bytes == 65536, "first established-lease verification chunk");
                interrupted = true;
                Check(runner.Cancel(id), "original cancellation admission");
            };
            runner.OutputCommitCheckpointProbe = stage => { if (stage == "native_published") publications++; };
            Check(runner.Start(new(id, "modify_zip", source, 0, new { max_entries = 100 }, OutputRoot: archiveRoot,
                JobExecution: true, PasswordInteraction: true, ConflictPolicy: "ask",
                JobModify: new(browser, "rename", [0], "renamed.bin"))), "actual edit admission");
            var deadline = System.Diagnostics.Stopwatch.StartNew();
            while (!events.Any(e => e.Kind == "exited"))
            {
                Check(deadline.Elapsed < TimeSpan.FromSeconds(40), "actual terminal deadline");
                await Task.Delay(20);
            }
            Check(await runner.DrainAsync() && runner.ActiveCount == 0, "actual Run drain");
            var terminal = events.Single(e => e.Kind == "exited");
            var records = log.Records.Select(r => new { r.Name, r.Id, r.Fields }).ToArray();
            File.WriteAllText(Path.Combine(root, "terminal.json"), JsonSerializer.Serialize(new { mode, terminal, interrupted, publications, original_bytes = original.Length }));
            File.WriteAllText(Path.Combine(root, "diagnostics.json"), JsonSerializer.Serialize(records));
            Console.WriteLine("ARCHIVE_OPT05_TERMINAL " + root);
            try
            {
                var cancelled = mode == "cancel_verification";
                Check(terminal.Category == (cancelled ? "cancelled" : "success") && terminal.NativeErrorCode is null &&
                    terminal.FilesCommitted == (cancelled ? 0 : 1) && terminal.CleanupComplete &&
                    publications == (cancelled ? 0 : 1) && interrupted == cancelled, "original terminal/publication/cancellation facts: " + mode);
                var published = File.ReadAllBytes(source);
                if (cancelled) Check(published.AsSpan().SequenceEqual(original), "interrupted source remains unchanged");
                else VerifyZip(published, payload, "renamed.bin");
                using (var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                    Check(reopened.Length == published.Length, "exclusive reopen after actual cleanup");
                Check(!Directory.EnumerateFiles(archiveRoot, ".nxpart-*").Any(), "actual owned partial retirement");
                Check(log.TerminalAttempts == 1, "one original terminal diagnostic attempt");
                if (mode == "reject_log") { Check(log.Records.IsEmpty, "rejecting sink cannot change successful result"); continue; }
                var record = log.Records.Single(r => r.Name is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed");
                var fields = record.Fields;
                Check(record.Id == id && Equals(fields["category"], terminal.Category) &&
                    Equals(fields["elapsed_ms"], terminal.ElapsedMs) && Equals(fields["native_error_code"], terminal.NativeErrorCode) &&
                    Equals(fields["commit_state"], terminal.CommitState) && Equals(fields["files_committed"], terminal.FilesCommitted) &&
                    Equals(fields["cleanup_complete"], terminal.CleanupComplete), "resolved terminal/log agreement");
                AssertRole(fields, "source", cancelled ? 2 : 5, cancelled ? 1 : 5,
                    cancelled ? original.Length + 65536L : original.Length * 5L, original.Length * (cancelled ? 1L : 5L));
                AssertRole(fields, "target", cancelled ? 1 : 2, cancelled ? 1 : 2,
                    original.Length * (cancelled ? 1L : 2L), original.Length * (cancelled ? 1L : 2L));
                AssertRole(fields, "output", cancelled ? 0 : 2, cancelled ? 0 : 2,
                    cancelled ? 0 : published.Length * 2L, cancelled ? 0 : published.Length * 2L);
                Check(Convert.ToInt64(fields["partial_read_bytes"]) == (cancelled ? 0 : published.Length) &&
                    Convert.ToInt64(fields["partial_read_calls"]) == (cancelled ? 0 : (published.Length + 65535L) / 65536 + 1), "held-partial readback is separate");
                var serialized = JsonSerializer.Serialize(records);
                Check(!serialized.Contains(root, StringComparison.Ordinal) && !serialized.Contains("original.bin", StringComparison.Ordinal) &&
                    !serialized.Contains("renamed.bin", StringComparison.Ordinal) && !serialized.Contains(browser.Source.Native, StringComparison.Ordinal), "diagnostic private-data leak");
            }
            catch (Exception failure) when (failure is InvalidOperationException or KeyNotFoundException)
            { failures.Add(mode + ": " + failure.Message); }
        }
        Check(failures.Count == 0, string.Join("; ", failures));
    }

    private static void AssertRole(IReadOnlyDictionary<string, object?> fields, string role, long attempts, long fullReads, long bytes, long completedBytes)
    {
        var prefix = "native_" + role;
        Check(fields.TryGetValue(prefix + "_attempts", out var actualAttempts) &&
            fields.TryGetValue(prefix + "_full_reads", out var actualReads) &&
            fields.TryGetValue(prefix + "_read_bytes", out var actualBytes) &&
            fields.TryGetValue(prefix + "_completed_bytes", out var actualCompleted) &&
            Convert.ToInt64(actualAttempts) == attempts && Convert.ToInt64(actualReads) == fullReads &&
            Convert.ToInt64(actualBytes) == bytes && Convert.ToInt64(actualCompleted) == completedBytes,
            "terminal native " + role + " actual/completed read counters");
    }

    private static async Task<ArchiveBrowserContext> TerminalBrowser(string root, string source)
    {
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
        var command = ArchiveReadWire.Parse(wire);
        Check(reads.Dispatch(wire).Success, "original browser admission");
        while (true)
        {
            var envelope = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch)
                Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: 2)).Success, "browser metadata ACK");
            if (item.Terminal is not { } terminal) continue;
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "fresh browser source binding");
            return terminal.Browser!;
        }
    }
}
