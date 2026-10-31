using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

// Observation/execution only: no construction, qualification, source replacement or backend input.
internal interface IArchiveRecycleAttempt
{
    Guid OperationId { get; }
    Guid Token { get; }
    Task<ArchiveSourceDeletionResult> ExecuteAsync(Guid operationId, Guid token);
}

internal sealed partial class ArchiveWorkerRunner
{
    // Only this private Run owner can capture/complete/retire a session. Wire evidence is data,
    // never a factory for authority. At most the runner's admitted operations retain sessions.
    private sealed class RecycleSession : IArchiveRecycleAttempt
    {
        private enum Phase { Captured, Qualified, Refused, InFlight, Completed, Retired }
        private static readonly SemaphoreSlim ExecutionSlots = new(2, 2);
        private readonly object sessionGate = new();
        private readonly Run owner;
        private readonly ArchiveRecycleSource[] sources;
        private readonly IArchiveRecycleBackend? backend;
        private readonly IPlatformLog? log;
        private Phase phase;
        private readonly Guid token = Guid.NewGuid();
        internal RecycleSession(Run owner, IReadOnlyList<ArchiveRecycleSource> sources, IArchiveRecycleBackend? backend, IPlatformLog? log)
        { this.owner = owner; this.sources = sources.ToArray(); this.backend = backend; this.log = log; }
        public Guid OperationId => owner.Request.OperationId;
        public Guid Token => token;

        internal string Complete(Run completingOwner, ArchiveWorkerEvent terminal)
        {
            lock (sessionGate) {
                if (!ReferenceEquals(completingOwner, owner)) return "authorization_mismatch";
                if (phase == Phase.Retired) return "retired";
                if (phase != Phase.Captured) return "already_qualified";
                phase = Phase.Refused; // Every completion attempt is terminal, including refusal.
                var request = owner.Request;
                if (request.DeleteSource != true) return "consent_missing";
                if (request.ActiveTemporarySource) return "active_temporary_source";
                if (terminal.Category != "success" || !owner.SawTerminal || terminal.ExitCode != 0 ||
                    (request.Operation != "extract" && !IsCreation(request.Operation)) ||
                    owner.Output?.LastCommitState is not (null or ArchiveCommitState.Committed)) return "archive_not_fully_succeeded";
                if (owner.DeleteEvidence is not { } value) return "worker_evidence_missing";
                try {
                    if (value.ValueKind != JsonValueKind.Object || value.EnumerateObject().Count() != 3 ||
                        value.EnumerateObject().Any(field => field.Name is not ("operation_id" or "reason" or "source_ids")) ||
                        value.GetProperty("operation_id").GetGuid() != OperationId) return "invalid_evidence";
                    var reason = value.GetProperty("reason").GetString();
                    if (reason != "eligible") return reason is "consent_missing" or "cancelled" or "selected_only" or "members_skipped" or
                        "empty_source_set" or "coverage_incomplete" or "members_excluded" or "directory_coverage_unproven" or "lifecycle_incomplete" or "already_consumed" ? reason : "invalid_evidence";
                    var ids = value.GetProperty("source_ids");
                    if (ids.ValueKind != JsonValueKind.Array || ids.GetArrayLength() is < 1 or > 32 || ids.GetArrayLength() != sources.Length) return "source_set_mismatch";
                    var expected = sources.Select(source => source.Id).ToHashSet();
                    if (expected.Count != sources.Length) return "source_set_mismatch";
                    foreach (var id in ids.EnumerateArray()) if (!expected.Remove(id.GetGuid())) return "source_set_mismatch";
                    foreach (var source in sources) source.Verify();
                    phase = Phase.Qualified;
                    return "eligible";
                }
                catch (ArchiveOutputException failure) { return failure.Stage == "source_changed" ? "source_changed" : "source_unavailable"; }
                catch (Exception) { return "invalid_evidence"; }
            }
        }

        internal void Retire()
        {
            lock (sessionGate) {
                // The runner calls only after actual execution has returned; never release leases
                // for a timed-out/still-running backend. Retired observation handles cannot revive it.
                phase = Phase.Retired;
                Array.Clear(sources);
            }
        }

        public Task<ArchiveSourceDeletionResult> ExecuteAsync(Guid operationId, Guid presentedToken)
        {
            lock (sessionGate) {
                var reason = phase == Phase.Retired ? "retired" : operationId != OperationId || presentedToken != token ? "authorization_mismatch" :
                    phase is Phase.InFlight or Phase.Completed ? "already_consumed" : phase == Phase.Qualified ? "owner_required" : "not_qualified";
                return Task.FromResult(new ArchiveSourceDeletionResult(reason, []));
            }
        }

        internal Task<ArchiveSourceDeletionResult> ExecuteOwnedAsync()
        {
            var operationId = OperationId;
            var presentedToken = token;
            lock (sessionGate) {
                if (phase == Phase.Retired) return Task.FromResult(new ArchiveSourceDeletionResult("retired", []));
                if (operationId != OperationId || presentedToken != token) return Task.FromResult(new ArchiveSourceDeletionResult("authorization_mismatch", []));
                if (phase is Phase.InFlight or Phase.Completed) return Task.FromResult(new ArchiveSourceDeletionResult("already_consumed", []));
                if (phase != Phase.Qualified) return Task.FromResult(new ArchiveSourceDeletionResult("not_qualified", []));
                phase = Phase.InFlight;
            }
            var clock = Stopwatch.StartNew();
            ArchiveRecycleDiagnostics.SafeLog(log, "accepted", OperationId, clock, "eligible", null, sources.Length);
            if (backend is null) return FinishImmediate("native_protection_unproven", clock);
            if (!ExecutionSlots.Wait(0)) return FinishImmediate("execution_busy", clock);
            var completion = new TaskCompletionSource<ArchiveSourceDeletionResult>(TaskCreationOptions.RunContinuationsAsynchronously);
            var thread = new Thread(() => {
                var outcomes = new List<ArchiveRecycleOutcome>(sources.Length);
                try {
                    ArchiveRecycleDiagnostics.SafeLog(log, "started", OperationId, clock, "executing", null, 0);
                    var stop = false;
                    foreach (var source in sources) {
                        if (stop || owner.RecycleCancellation.IsCancellationRequested) { outcomes.Add(new(source.Id, ArchiveRecycleKind.NotAttempted)); continue; }
                        try { source.Verify(); }
                        catch (ArchiveOutputException failure) { outcomes.Add(new(source.Id, ArchiveRecycleKind.NotAttempted, failure.NativeCode)); stop = true; continue; }
                        catch (Exception) { outcomes.Add(new(source.Id, ArchiveRecycleKind.NotAttempted)); stop = true; continue; }
                        try {
                            var outcome = backend.Recycle(source, owner.RecycleCancellation.Token);
                            if (!Enum.IsDefined(outcome.Kind)) outcome = new(ArchiveRecycleKind.OutcomeUnknown);
                            outcomes.Add(new(source.Id, outcome.Kind, outcome.NativeCode));
                            if (outcome.Kind is ArchiveRecycleKind.Cancelled or ArchiveRecycleKind.OutcomeUnknown) stop = true;
                        }
                        catch (Exception) { outcomes.Add(new(source.Id, ArchiveRecycleKind.OutcomeUnknown)); stop = true; }
                    }
                    var result = new ArchiveSourceDeletionResult("completed", outcomes.AsReadOnly());
                    ArchiveRecycleDiagnostics.SafeLog(log, "terminal", OperationId, clock, result.Reason, outcomes.FirstOrDefault(item => item.NativeCode is not null)?.NativeCode, outcomes.Count);
                    lock (sessionGate) phase = Phase.Completed;
                    completion.TrySetResult(result);
                }
                finally { ExecutionSlots.Release(); }
            }) { IsBackground = true, Name = "archive-recycle-isolated" };
            try { thread.SetApartmentState(ApartmentState.STA); thread.Start(); }
            catch (Exception) { ExecutionSlots.Release(); completion.TrySetResult(FinishImmediate("not_started", clock).GetAwaiter().GetResult()); }
            return completion.Task;
        }
        private Task<ArchiveSourceDeletionResult> FinishImmediate(string reason, Stopwatch clock)
        {
            var result = new ArchiveSourceDeletionResult(reason, sources.Select(source => new ArchiveRecycleOutcome(source.Id, ArchiveRecycleKind.NotAttempted)).ToArray());
            ArchiveRecycleDiagnostics.SafeLog(log, "terminal", OperationId, clock, reason, null, sources.Length);
            lock (sessionGate) phase = Phase.Completed;
            return Task.FromResult(result);
        }
    }
}
