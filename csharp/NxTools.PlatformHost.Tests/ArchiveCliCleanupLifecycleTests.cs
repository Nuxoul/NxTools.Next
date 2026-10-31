using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.ExceptionServices;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Original runner ownership only, not a public CLI/console-signal outcome or a session DisposeAsync defect.
// Tiny independent input/evidence are retained. Only our unchanged helper copy may be removed after drain.
internal static class ArchiveCliCleanupLifecycleTests
{
    private static readonly TimeSpan Bound = TimeSpan.FromSeconds(8);
    private sealed record AssertionFact(string Id, bool Passed);
    private sealed record ExclusiveOpenFact(bool Opened, int? NativeErrorCode = null, int? HResult = null);
    private static TaskCompletionSource<bool> Signal() => new(TaskCreationOptions.RunContinuationsAsynchronously);
    private static string Hash(string path)
    {
        using var stream = File.OpenRead(path);
        return Convert.ToHexString(SHA256.HashData(stream));
    }
    private static int? NativeCode(Exception failure) => (failure.HResult & unchecked((int)0xffff0000)) == unchecked((int)0x80070000)
        ? failure.HResult & 0xffff : null;
    private static ExclusiveOpenFact OpenExclusive(string path)
    {
        try {
            using var stream = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            return new(true);
        }
        catch (IOException failure) { return new(false, NativeCode(failure), failure.HResult); }
        catch (UnauthorizedAccessException failure) { return new(false, NativeCode(failure), failure.HResult); }
    }
    private static object SafeEvent(ArchiveWorkerEvent item) => new {
        kind = item.Kind, operation_id = item.OperationId, category = item.Category,
        native_error_code = item.NativeErrorCode, elapsed_ms = item.ElapsedMs, exit_code = item.ExitCode,
        cleanup_complete = item.CleanupComplete, files_committed = item.FilesCommitted,
        bytes_committed = item.BytesCommitted, files_skipped = item.FilesSkipped, commit_state = item.CommitState,
        // Retain original events in memory, but never serialize raw worker frames/names/paths/tokens.
        frame_op = item.Frame is { } frame && frame.TryGetProperty("op", out var op) ? op.GetString() : null
    };

    public static async Task ActualCleanupTimeoutRetainsOwnership()
    {
        var parent = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT");
        if (string.IsNullOrWhiteSpace(parent) || !Path.IsPathFullyQualified(parent) || !Directory.Exists(parent))
            throw new InvalidOperationException("explicit_existing_lifecycle_evidence_root_required");
        var evidenceId = Guid.NewGuid();
        var root = Path.Combine(Path.GetFullPath(parent), "archive-cli-cleanup-" + evidenceId);
        if (Directory.Exists(root) || File.Exists(root)) throw new InvalidOperationException("fresh_evidence_child_required");
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "small.zip");
        var helper = Path.Combine(AppContext.BaseDirectory, ArchiveReadHelper.DeploymentName);
        var first = Guid.NewGuid(); var second = Guid.NewGuid(); var third = Guid.NewGuid();
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var checkpoints = new ConcurrentQueue<object>();
        var observations = new List<object>();
        var assertions = new List<AssertionFact>();
        var failures = new List<(string Stage, Exception Failure)>();
        var bothCleanup = Signal(); var firstExited = Signal(); var secondExited = Signal(); var thirdExited = Signal();
        var releaseCleanup = new ManualResetEventSlim();
        var clock = Stopwatch.StartNew();
        ArchiveWorkerRunner? runner = null;
        string? inputBefore = null, inputAfter = null, helperExpected = null, helperAfter = null;
        var copied = false; var helperRemoved = false; var bodyCompleted = false;
        var cleanupCalls = 0; var cleanupTimeouts = 0; var cleanupReturns = 0;
        bool? shortDrain = null, releasedDrain = null, finalDrain = null;
        long? shortDrainMs = null, releasedDrainMs = null, finalDrainMs = null;

        void Require(bool value, string id)
        {
            assertions.Add(new(id, value));
            if (!value) throw new InvalidOperationException(id);
        }
        void Expect(bool value, string id)
        {
            assertions.Add(new(id, value));
            if (!value) failures.Add(("assertion", new InvalidOperationException(id)));
        }
        void Publish(ArchiveWorkerEvent item)
        {
            events.Enqueue(item);
            if (item.Kind != "exited") return;
            if (item.OperationId == first) firstExited.TrySetResult(true);
            if (item.OperationId == second) secondExited.TrySetResult(true);
            if (item.OperationId == third) thirdExited.TrySetResult(true);
        }
        void Exclusive(string stage, bool held)
        {
            var input = OpenExclusive(source); var image = OpenExclusive(helper);
            observations.Add(new { stage, elapsed_ms = clock.ElapsedMilliseconds, source = input, helper = image });
            Expect(held ? !input.Opened && input.NativeErrorCode is 5 or 32 or 33 : input.Opened, stage + "_source_exclusive");
            Expect(held ? !image.Opened && image.NativeErrorCode is 5 or 32 or 33 : image.Opened, stage + "_helper_exclusive");
        }
        void Held(string stage)
        {
            var snapshot = events.ToArray();
            var count = runner!.ActiveCount;
            var exited = snapshot.Count(item => item.Kind == "exited");
            var cleaned = snapshot.Count(item => item.CleanupComplete);
            observations.Add(new { stage, elapsed_ms = clock.ElapsedMilliseconds, active_count = count,
                exited_count = exited, cleanup_complete_count = cleaned, checkpoint_count = Volatile.Read(ref cleanupCalls) });
            Expect(count == 2, stage + "_two_active_slots");
            Expect(exited == 0 && cleaned == 0, stage + "_no_terminal_or_cleanup_credit");
            Exclusive(stage, held: true);
        }

        try {
            Require(OperatingSystem.IsWindows(), "windows_boundary_required");
            Require(string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase),
                "original_test_apphost_required");
            var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
            Require(!string.IsNullOrWhiteSpace(worker) && Path.IsPathFullyQualified(worker) && File.Exists(worker),
                "explicit_matching_real_worker_required_no_skip");
            Require(!File.Exists(helper) && !Directory.Exists(helper), "refuse_preexisting_helper");
            helperExpected = Hash(worker!);
            using (var file = new FileStream(source, FileMode.CreateNew, FileAccess.Write))
            using (var zip = new ZipArchive(file, ZipArchiveMode.Create)) {
                var entry = zip.CreateEntry("payload.txt", CompressionLevel.NoCompression);
                entry.LastWriteTime = new DateTimeOffset(2020, 2, 3, 4, 5, 6, TimeSpan.Zero);
                entry.ExternalAttributes = 0;
                using var content = entry.Open();
                content.Write("independent tiny archive cleanup lifecycle payload\n"u8.ToArray());
            }
            inputBefore = Hash(source);
            Require(new FileInfo(source).Length < 1024, "independent_input_is_tiny");
            runner = ArchiveWorkerRunner.ForBrowserEntry(ArchiveReadHelper.Production(), Publish);
            File.Copy(worker!, helper, overwrite: false); copied = true;
            Require(Hash(helper) == helperExpected, "staged_helper_matches_explicit_worker");
            Exclusive("before_start", held: false);
            // Reuse the original production helper selection/held-image lease; legacy worker TEST
            // remains unchanged, while this constructor avoids the historical unleased test shortcut.
            runner.CleanupCheckpointProbe = () => {
                // This ORIGINAL checkpoint is after process/Job/pump drainage, before resources/leases/slots retire.
                var index = Interlocked.Increment(ref cleanupCalls);
                checkpoints.Enqueue(new { stage = "entered", index, elapsed_ms = clock.ElapsedMilliseconds });
                if (index == 2) bothCleanup.TrySetResult(true);
                // Never throw out of the runner's finally or leave an infinite test-owned wait.
                var released = index > 2 || releaseCleanup.Wait(Bound);
                if (!released) Interlocked.Increment(ref cleanupTimeouts);
                checkpoints.Enqueue(new { stage = "returned", index, released, elapsed_ms = clock.ElapsedMilliseconds });
                Interlocked.Increment(ref cleanupReturns);
            };
            // Same finite limits as the original cancellation-boundary fixture; no product budget/deadline changes.
            object limits = new { max_entries = 100, max_metadata_bytes = 4194304, max_working_bytes = 67108864,
                max_entry_bytes = 1048576, max_output_bytes = 1048576 };
            Require(runner.Start(new(first, "test", source, 0, limits)), "first_original_test_admitted");
            Require(runner.Start(new(second, "test", source, 0, limits)), "second_original_test_admitted");
            await bothCleanup.Task.WaitAsync(Bound);
            Expect(Volatile.Read(ref cleanupCalls) == 2 && Volatile.Read(ref cleanupTimeouts) == 0, "both_bounded_cleanup_barriers_reached");
            foreach (var id in new[] { first, second }) {
                Expect(events.Count(item => item.OperationId == id && item.Kind == "accepted") == 1, "one_original_admission_" + id);
                Expect(events.Count(item => item.OperationId == id && item.Kind == "started") == 1, "one_real_worker_started_" + id);
                Expect(events.Count(item => item.OperationId == id && item.Frame is { } frame && frame.GetProperty("op").GetString() == "complete") == 1,
                    "one_real_test_complete_frame_" + id);
                Expect(!events.Any(item => item.OperationId == id && item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure"),
                    "no_worker_failure_frame_" + id);
            }
            Held("before_short_drain");
            var drainClock = Stopwatch.StartNew();
            shortDrain = await runner.DrainAsync(150);
            shortDrainMs = drainClock.ElapsedMilliseconds;
            Expect(shortDrain == false, "original_150ms_drain_returns_false");
            Held("after_short_drain");
            var thirdAdmitted = runner.Start(new(third, "test", source, 0, limits));
            var rejected = events.Where(item => item.OperationId == third && item.Kind == "rejected").ToArray();
            observations.Add(new { stage = "third_admission", operation_id = third, admitted = thirdAdmitted,
                rejected_count = rejected.Length, active_count = runner.ActiveCount, elapsed_ms = clock.ElapsedMilliseconds });
            Expect(!thirdAdmitted && rejected.Length == 1 && rejected[0].Category == "concurrency_limit", "third_original_run_rejected_concurrency_limit");
            Expect(!events.Any(item => item.OperationId == third && item.Kind is "accepted" or "started"), "third_has_no_worker_admission");
            Held("after_third_rejection");
            Expect(Volatile.Read(ref cleanupTimeouts) == 0 && Volatile.Read(ref cleanupReturns) == 0, "barriers_still_held_until_explicit_release");
            releaseCleanup.Set();
            drainClock.Restart();
            releasedDrain = await runner.DrainAsync(8000);
            releasedDrainMs = drainClock.ElapsedMilliseconds;
            Require(releasedDrain == true, "original_drain_true_after_release");
            // ActiveCount is retired just BEFORE exited publication; do not race that original ordering.
            await Task.WhenAll(firstExited.Task, secondExited.Task).WaitAsync(Bound);
            var terminals = events.Where(item => item.Kind == "exited").ToArray();
            Expect(terminals.Length == 2, "exactly_two_original_exited_events");
            foreach (var id in new[] { first, second }) {
                var terminal = terminals.Where(item => item.OperationId == id).ToArray();
                Expect(terminal.Length == 1 && terminal[0] is { CleanupComplete: true, Category: "success", ExitCode: 0, FilesCommitted: 0 },
                    "exact_success_actual_cleanup_terminal_" + id);
            }
            Expect(runner.ActiveCount == 0, "zero_active_after_actual_cleanup");
            Expect(Volatile.Read(ref cleanupCalls) == 2 && Volatile.Read(ref cleanupReturns) == 2 && Volatile.Read(ref cleanupTimeouts) == 0,
                "exactly_two_explicitly_released_bounded_barriers");
            Exclusive("after_actual_cleanup", held: false);
            bodyCompleted = true;
        }
        catch (Exception failure) { failures.Add(("body", failure)); }
        finally {
            releaseCleanup.Set();
            if (runner is not null) {
                try {
                    runner.Stop();
                    var drainClock = Stopwatch.StartNew();
                    finalDrain = await runner.DrainAsync(8000);
                    finalDrainMs = drainClock.ElapsedMilliseconds;
                    Expect(finalDrain == true && runner.ActiveCount == 0, "finally_original_stop_and_actual_drain");
                    if (finalDrain == true) {
                        var terminals = new[] { (first, firstExited.Task), (second, secondExited.Task), (third, thirdExited.Task) }
                            .Where(item => events.Any(value => value.OperationId == item.Item1 && value.Kind == "accepted"))
                            .Select(item => item.Item2);
                        await Task.WhenAll(terminals).WaitAsync(Bound);
                    }
                }
                catch (Exception failure) { failures.Add(("finally_stop_drain", failure)); }
            }
            if (inputBefore is not null) {
                try { inputAfter = Hash(source); Expect(inputBefore == inputAfter, "input_sha256_unchanged"); }
                catch (Exception failure) { failures.Add(("input_preservation", failure)); }
            }
            if (copied) {
                try {
                    helperAfter = Hash(helper);
                    Expect(helperAfter == helperExpected, "helper_sha256_unchanged");
                    if (finalDrain == true && runner?.ActiveCount == 0 && helperAfter == helperExpected) {
                        File.Delete(helper);
                        helperRemoved = !File.Exists(helper);
                        Expect(helperRemoved, "only_unchanged_drained_helper_copy_removed");
                    }
                }
                catch (Exception failure) { failures.Add(("helper_preservation_cleanup", failure)); }
            }
            runner?.Dispose(); // Original Stop only; it is NOT evidence of drainage.
            if (runner is null || finalDrain == true && runner.ActiveCount == 0) releaseCleanup.Dispose();
            try {
                using var output = new FileStream(Path.Combine(root, "evidence.json"), FileMode.CreateNew, FileAccess.Write);
                JsonSerializer.Serialize(output, new {
                    schema = 1, fixture = "ArchiveCliCleanupLifecycleTests.ActualCleanupTimeoutRetainsOwnership", evidence_id = evidenceId,
                    scope = "original_runner_cleanup_ownership_only", public_cli_outcome = "not_exercised",
                    original_checkpoint_boundary = "process_job_pumps_drained_before_resources_leases_slots_retire",
                    status = bodyCompleted && failures.Count == 0 ? "passed" : "failed", body_completed = bodyCompleted,
                    operation_ids = new[] { first, second }, rejected_operation_id = third, elapsed_ms = clock.ElapsedMilliseconds,
                    short_drain_budget_ms = 150, short_drain = shortDrain, short_drain_ms = shortDrainMs,
                    released_drain_budget_ms = 8000, released_drain = releasedDrain, released_drain_ms = releasedDrainMs,
                    final_drain = finalDrain, final_drain_ms = finalDrainMs, active_count = runner?.ActiveCount,
                    checkpoint_count = Volatile.Read(ref cleanupCalls), checkpoint_returns = Volatile.Read(ref cleanupReturns),
                    checkpoint_timeouts = Volatile.Read(ref cleanupTimeouts), checkpoint_wait_bound_ms = 8000,
                    input_sha256_before = inputBefore, input_sha256_after = inputAfter,
                    helper_sha256_expected = helperExpected, helper_sha256_after = helperAfter,
                    helper_copied = copied, helper_removed = helperRemoved, sample_and_evidence_retained = true,
                    original_event_count = events.Count, failed_assertion_count = assertions.Count(item => !item.Passed),
                    assertions, observations, checkpoints = checkpoints.ToArray(), events = events.ToArray().Select(SafeEvent).ToArray(),
                    failures = failures.Select(item => new { stage = item.Stage, type = item.Failure.GetType().Name,
                        hresult = item.Failure.HResult, native_error_code = NativeCode(item.Failure) }).ToArray()
                }, new JsonSerializerOptions { WriteIndented = true, PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower });
            }
            catch (Exception failure) { failures.Add(("evidence_write", failure)); }
        }
        // Keep failed assertions AND original exceptions; a successful finally drain cannot turn them green.
        if (failures.Count == 1) ExceptionDispatchInfo.Capture(failures[0].Failure).Throw();
        if (failures.Count > 1) throw new AggregateException("archive_cli_cleanup_lifecycle_failed", failures.Select(item => item.Failure));
    }
}
