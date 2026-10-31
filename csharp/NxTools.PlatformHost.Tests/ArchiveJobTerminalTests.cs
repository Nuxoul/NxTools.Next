using System.Collections;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Exact production private boundaries, invoked in a fixed order. Only an owned scratch directory
// and in-memory Run/Job state; no Start, worker/native API, archive mutation or product fault switch.
internal static class ArchiveJobTerminalTests
{
    private const BindingFlags Instance = BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic;
    private static void Check(bool value, string message) { if (!value) throw new InvalidOperationException(message); }
    private static object Invoke(Type type, object? target, string name, params object?[] arguments) =>
        (type.GetMethod(name, target is null ? BindingFlags.Static | BindingFlags.NonPublic : Instance)
            ?? throw new InvalidOperationException("Missing production boundary: " + name)).Invoke(target, arguments)
            ?? throw new InvalidOperationException("Null production boundary: " + name);
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly List<string> Records = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Records.Add(JsonSerializer.Serialize(new { eventName, correlationId, fields }));
    }
    private static ArchiveWorkerEvent OwnerTerminal(Guid id, string category, int? nativeCode, ArchiveCommitState state,
        bool nativeFailure, bool cleanup = true, string operation = "modify_7z", string action = "commit")
    {
        var type = typeof(ArchiveWorkerRunner).GetNestedType("Run", BindingFlags.NonPublic)
            ?? throw new InvalidOperationException("Missing real Run");
        var run = Activator.CreateInstance(type, Instance, null,
            [new ArchiveWorkerRequest(id, operation, @"C:\owned\fixture.7z", 0, new { }, JobExecution: true),
             ArchiveWorkerRunner.DefaultMemory, Array.Empty<byte>()], null) ?? throw new InvalidOperationException("Missing Run constructor");
        try
        {
            // Actual first-failure CAS precedes the consequential worker Cancelled frame.
            if (nativeFailure)
                type.GetMethod("FailNative", Instance)!.Invoke(run,
                    [new ArchiveNativeOutputFailure(id, Guid.NewGuid(), action, Guid.NewGuid(), category, nativeCode, state)]);
            else type.GetMethod("Fail", Instance)!.Invoke(run, [category, nativeCode]);
            type.GetField("Journal", Instance)!.SetValue(run,
                new ArchiveJobCommit[] { new(0, state.ToString(), false, 203, "owned-fixture-identity", false, "partial_removed") });
            type.GetField("OutputPublished", Instance)!.SetValue(run, false);
            return (ArchiveWorkerEvent)Invoke(typeof(ArchiveWorkerRunner), null, "ResolveTerminal", run, cleanup, (uint?)1);
        }
        finally
        {
            foreach (var field in type.GetFields(Instance))
                if (field.GetValue(run) is CancellationTokenSource source) source.Dispose();
        }
    }
    private static async Task<ArchiveJobTerminal> Project(string label, ArchiveWorkerEvent item, string frameCode = "Cancelled")
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var log = new CaptureLog();
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root, log);
        var command = new ArchiveJobCommand { Version = 1, Action = "start", OperationId = item.OperationId,
            SessionId = Guid.NewGuid(), Generation = 1 };
        var operationType = typeof(ArchiveJobSession).GetNestedType("Operation", BindingFlags.NonPublic)!;
        var operation = Activator.CreateInstance(operationType, Instance, null, [command], null)!;
        var active = (IDictionary)typeof(ArchiveJobSession).GetField("active", Instance)!.GetValue(jobs)!;
        active.Add(item.OperationId, operation);
        var frame = JsonSerializer.SerializeToElement(new { op = "failure", error_code = frameCode });
        var accepted = await (Task<string?>)Invoke(typeof(ArchiveJobSession), jobs, "OnFrame", item.OperationId, frame, CancellationToken.None);
        Check(accepted is null, "real worker failure frame accepted");
        // Invoke returns null for this real void callback, so call its MethodInfo directly.
        var exit = typeof(ArchiveJobSession).GetMethod("OnRunnerEvent", Instance)!;
        exit.Invoke(jobs, [item]);
        Check(jobs.Events.TryRead(out var envelope), "original Job must publish terminal");
        var terminal = envelope!.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!.Terminal!;
        Check(terminal is not null && !jobs.Events.TryRead(out _), "one original terminal only");
        exit.Invoke(jobs, [item]);
        Check(!jobs.Events.TryRead(out _), "retired operation cannot emit another terminal");
        var diagnostic = log.Records.Single(record => record.Contains("archive.job.terminal", StringComparison.Ordinal));
        Check(!diagnostic.Contains(root, StringComparison.Ordinal), "terminal diagnostics cannot contain path");
        File.WriteAllText(Path.Combine(root, "projection.json"), JsonSerializer.Serialize(new { label, runner = item, terminal, diagnostic }, JsonDefaults.Options));
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_TERM01_EVIDENCE");
        if (evidence is not null)
        {
            var scratch = Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!) + Path.DirectorySeparatorChar;
            Check(Path.GetFullPath(evidence).StartsWith(scratch, StringComparison.OrdinalIgnoreCase), "owned evidence only");
            Directory.CreateDirectory(evidence);
            File.WriteAllText(Path.Combine(evidence, label + ".json"), JsonSerializer.Serialize(new { root, runner = item, terminal }, JsonDefaults.Options));
        }
        return terminal!;
    }
    public static Task ExtractionProgressWire()
    {
        var legacy = "{\"phase\":\"running\",\"bytes_done\":0,\"bytes_total\":null,\"files_done\":0,\"files_total\":null}";
        var parsed = JsonSerializer.Deserialize<ArchiveJobProgress>(legacy, ArchiveJobWire.Strict)!;
        Check(parsed.Valid && parsed.CurrentBytesDone is null && parsed.CurrentBytesTotal is null, "legacy progress remains valid and unknown");
        var wire = JsonSerializer.Serialize(parsed, JsonDefaults.Options);
        Check(!wire.Contains("current_bytes_", StringComparison.Ordinal), "missing member facts omitted");
        var measured = parsed with { CurrentMember = "first.txt", CurrentBytesDone = 4, CurrentBytesTotal = 10, BytesDone = 4, BytesTotal = 30, FilesTotal = 2 };
        Check(measured.Valid && JsonSerializer.Deserialize<ArchiveJobProgress>(JsonSerializer.Serialize(measured, JsonDefaults.Options), ArchiveJobWire.Strict) == measured, "dual progress roundtrip");
        Check(!(measured with { CurrentBytesDone = 11 }).Valid && !(measured with { CurrentBytesDone = null }).Valid, "known member total cannot overrun or omit done");
        Check(!(measured with { CurrentMember = null }).Valid && (measured with { CurrentBytesTotal = null, CurrentBytesDone = 11, BytesDone = 31 }).Valid, "unknown member total retains processed bytes and aggregate overrun");
        return Task.CompletedTask;
    }
    public static async Task NativeFailureBeforeWorkerCancelled()
    {
        var runner = OwnerTerminal(Guid.NewGuid(), "replace_file", 5, ArchiveCommitState.FailedBeforeCommit, true);
        Check(runner.Category == "replace_file" && runner.NativeErrorCode == 5 && runner.CommitState == "FailedBeforeCommit",
            "owning Runner must already retain exact native cause");
        var terminal = await Project("native5-worker-cancelled-first", runner);
        Check(terminal.Outcome == "failed" && terminal.Code == "replace_file",
            "TERM01 expected failed/replace_file but actual " + terminal.Outcome + "/" + terminal.Code);
        Check(terminal.NativeErrorCode == 5 && terminal.FilesCommitted == 0 && terminal.CommitState == "FailedBeforeCommit" &&
            terminal.CleanupComplete && terminal.OutputPublished is false && terminal.Journal!.SequenceEqual(runner.Journal!),
            "owning native code/count/state/cleanup/publication/journal must survive Job projection");
        Console.WriteLine("TERM01 deterministic native5 -> worker Cancelled frame -> Runner exit -> Job failed/replace_file PASS");
        foreach (var (category, action) in new[] { ("write_file", "write"), ("flush_file", "commit"), ("flush_or_commit", "commit") }) {
            var io = OwnerTerminal(Guid.NewGuid(), category, 112, ArchiveCommitState.FailedBeforeCommit, true, operation: "extract", action: action);
            Check(io.HasNativeFailure && io.Category == category && io.NativeErrorCode == 112, "ordinary output I/O retains its exact native owner");
            var projected = await Project("ordinary-" + category, io, "OutputIo");
            Check(projected.Outcome == "failed" && projected.Code == category && projected.NativeErrorCode == 112, "ordinary I/O cause survives generic worker frame");
        }
    }
    public static async Task TerminalAuthorityControls()
    {
        var scenarios = new List<(string Name, ArchiveWorkerEvent Event, string Frame, string Outcome, string Code)>();
        ArchiveWorkerEvent Native(string category, int code, ArchiveCommitState state) => OwnerTerminal(Guid.NewGuid(), category, code, state, true);
        scenarios.Add(("genuine-cancel", OwnerTerminal(Guid.NewGuid(), "cancelled", null, ArchiveCommitState.FailedBeforeCommit, false), "Cancelled", "cancelled", "cancelled"));
        scenarios.Add(("native-cancelled", Native("cancelled", 995, ArchiveCommitState.FailedBeforeCommit), "Cancelled", "cancelled", "cancelled"));
        scenarios.Add(("incidental-code-no-native-failure", OwnerTerminal(Guid.NewGuid(), "worker_failure", 5, ArchiveCommitState.FailedBeforeCommit, false), "Cancelled", "cancelled", "cancelled"));
        scenarios.Add(("source-changed", Native("source_changed", 2, ArchiveCommitState.FailedBeforeCommit), "Cancelled", "failed", "source_changed"));
        scenarios.Add(("target-denied", Native("target_denied", 5, ArchiveCommitState.FailedBeforeCommit), "Cancelled", "failed", "target_denied"));
        scenarios.Add(("unknown64", Native("commit_outcome_unknown", 64, ArchiveCommitState.CommitOutcomeUnknown) with { OutputPublished = null }, "Cancelled", "failed", "commit_outcome_unknown"));
        scenarios.Add(("timeout", OwnerTerminal(Guid.NewGuid(), "timeout", null, ArchiveCommitState.FailedBeforeCommit, false), "Cancelled", "failed", "timeout"));
        scenarios.Add(("cleanup-incomplete", OwnerTerminal(Guid.NewGuid(), "replace_file", 5, ArchiveCommitState.FailedBeforeCommit, true, false), "Cancelled", "failed", "cleanup_incomplete"));
        scenarios.Add(("domain-corrupt", OwnerTerminal(Guid.NewGuid(), "worker_failure", null, ArchiveCommitState.FailedBeforeCommit, false), "CorruptArchive", "failed", "corrupt_archive"));
        var published = new ArchiveWorkerEvent("exited", Guid.NewGuid(), "cancelled_with_committed_output", CleanupComplete: true,
            FilesCommitted: 1, CommitState: "Committed", Journal: [new(0, "Committed", true, 203, "owned-published", true, "retained_replacement")],
            BytesCommitted: 203, ResultLocation: new(@"C:\owned\result", "open"), OutputPublished: true);
        scenarios.Add(("late-published-cancel", published, "Cancelled", "cancelled", "cancelled_with_committed_output"));
        scenarios.Add(("published-postverify-failure", published with { OperationId = Guid.NewGuid(), Category = "committed_but_verification_failed", NativeErrorCode = 5, CommitState = "CommittedButVerificationFailed" }, "Cancelled", "failed", "committed_but_verification_failed"));
        scenarios.Add(("published-target-changed", published with { OperationId = Guid.NewGuid(), Category = "target_changed_after_commit", NativeErrorCode = 5, CommitState = "TargetChangedAfterCommit" }, "Cancelled", "failed", "target_changed_after_commit"));
        foreach (var scenario in scenarios)
        {
            var terminal = await Project(scenario.Name, scenario.Event, scenario.Frame);
            Check(terminal.Outcome == scenario.Outcome && terminal.Code == scenario.Code, "authority control " + scenario.Name + ": " + terminal.Outcome + "/" + terminal.Code);
            Check(terminal.NativeErrorCode == scenario.Event.NativeErrorCode && terminal.FilesCommitted == (ulong)scenario.Event.FilesCommitted &&
                terminal.BytesCommitted == scenario.Event.BytesCommitted && terminal.CommitState == scenario.Event.CommitState &&
                terminal.CleanupComplete == scenario.Event.CleanupComplete && terminal.OutputPublished == scenario.Event.OutputPublished &&
                terminal.ResultLocation == scenario.Event.ResultLocation && terminal.Journal!.SequenceEqual(scenario.Event.Journal!),
                "authority facts changed: " + scenario.Name);
        }
        Console.WriteLine("TERM01 authority controls=" + scenarios.Count + " PASS; not actual native publication/cancellation acceptance");
    }
}
