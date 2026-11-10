using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerRunner
{
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record MemberReady(Guid request_id, int index, ulong ordinal, ulong bytes);
    private sealed partial class Run
    {
        internal ArchiveDragDemand? VirtualDemand;
        internal bool VirtualClosing, VirtualAcquired;
        internal volatile bool VirtualSealing;
        internal Guid? VirtualReadyRequest;
        internal long VirtualClosingStart;
        internal long VirtualWorkStart;
        internal readonly HashSet<int> VirtualIndices = [];
        internal readonly HashSet<Guid> VirtualRequests = [];
        internal readonly List<ArchiveWorkerOutputBridge> VirtualOutputs = [];
        internal ReadOnlyMemory<byte>? VirtualZone;
        internal long WorkElapsedMs => Request.DragMaterialization is null
            ? Request.JobExecution ? ActiveElapsedMs : Clock.ElapsedMilliseconds
            : VirtualDemand is null && !VirtualSealing ? VirtualClosing ? Clock.ElapsedMilliseconds - Interlocked.Read(ref VirtualClosingStart) : 0
            : Clock.ElapsedMilliseconds - Interlocked.Read(ref VirtualWorkStart);
        internal int PublishedFiles => Request.DragMaterialization is null ? Output?.FilesCommitted ?? 0 : VirtualOutputs.Sum(item => item.FilesCommitted);
        internal ulong PublishedBytes => Request.DragMaterialization is null ? Output?.BytesCommitted ?? BytesCommitted : VirtualOutputs.Aggregate(0ul, (sum, item) => sum + item.BytesCommitted);
        internal void InitializeVirtual()
        {
            if (Request.DragMaterialization is not { } drag) return;
            VirtualDemand = drag.First;
            VirtualIndices.Add(drag.First.Index); VirtualRequests.Add(drag.First.RequestId);
        }
    }
    private static ArchiveDragMaterialization? CaptureVirtual(ArchiveWorkerRequest request)
    {
        if (request.DragMaterialization is not { } drag) return null;
        if (request.Operation != "extract" || request.BrowserEntry is null || request.PasswordInteraction || request.JobExecution ||
            request.ReadContract is not null || request.BrowserAuthorize is not null || request.DeleteSource is not null ||
            request.ExtractOptions?.Reclamation is not null || request.ConflictPolicy is not null || request.OutputReadmission is not null ||
            drag.Members.Length == 0 || drag.First.RequestId == Guid.Empty || drag.First.OutputRoot != request.OutputRoot ||
            drag.First.OutputLimit is 0 or > Protocol.ArchiveEntrySession.MaterializeOutputLimit ||
            request.BrowserEntry.Ordinal != drag.First.Ordinal) throw new ArchiveOutputException("invalid_request");
        var members = drag.Members.ToArray();
        if (members.Any(member => member.index < 0) || members.Select(member => member.index).Distinct().Count() != members.Length ||
            members.Select(member => member.ordinal).Distinct().Count() != members.Length ||
            !members.Any(member => member.index == drag.First.Index && member.ordinal == drag.First.Ordinal)) throw new ArchiveOutputException("invalid_request");
        return new(members, drag.First);
    }
    internal bool DemandVirtual(Guid operation, ArchiveDragDemand demand)
    {
        lock (gate) {
            if (!active.TryGetValue(operation, out var run) || run.Request.DragMaterialization is not { } drag ||
                run.Failure is not null || run.SawTerminal || !run.Negotiated || run.VirtualClosing || run.VirtualDemand is not null ||
                demand.RequestId == Guid.Empty || demand.OutputLimit == 0 || demand.OutputLimit > ((ArchiveAdmittedLimits)run.Request.Limits).max_output_bytes ||
                !drag.Members.Any(member => member.index == demand.Index && member.ordinal == demand.Ordinal) ||
                run.VirtualIndices.Contains(demand.Index) || run.VirtualRequests.Contains(demand.RequestId)) return false;
            run.VirtualDemand = demand; run.VirtualAcquired = false;
            run.VirtualWorkStart = run.Clock.ElapsedMilliseconds;
            run.VirtualIndices.Add(demand.Index); run.VirtualRequests.Add(demand.RequestId);
            return SendVirtualDemand(run);
        }
    }
    private static bool SendVirtualDemand(Run run)
    {
        if (run.VirtualDemand is not { } demand) return false;
        if (run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = run.Request.OperationId, op = "drag_demand",
            demand = new { request_id = demand.RequestId, index = demand.Index, ordinal = demand.Ordinal, output_limit = demand.OutputLimit } }))) {
            run.Fail("input_limit"); return false;
        }
        run.PendingWrites++; return true;
    }
    internal bool EndVirtual(Guid operation)
    {
        lock (gate) {
            if (!active.TryGetValue(operation, out var run) || run.Request.DragMaterialization is null || run.Failure is not null ||
                run.VirtualClosing || run.VirtualDemand is not null || !run.Negotiated) return false;
            run.VirtualClosing = true; run.VirtualClosingStart = run.Clock.ElapsedMilliseconds;
            if (!run.Input.Writer.TryWrite(Pack(new { version = 1, operation_id = operation, op = "drag_end" }))) { run.Fail("input_limit"); return false; }
            return true;
        }
    }
    // Called by the original stdout/native-call pump, never the platform-command reader.
    private object AcquireVirtualMember(Run run, JsonElement call)
    {
        lock (gate) {
            if (run.Request.DragMaterialization is null || run.VirtualDemand is not { } demand || run.VirtualAcquired ||
                run.Source is null || run.Failure is not null || call.GetProperty("token").GetGuid() != demand.RequestId)
                throw new ArchiveFault("stdout_invalid_frame");
            if (run.VirtualOutputs.Count != 0) {
                var limits = (ArchiveAdmittedLimits)run.Request.Limits;
                var request = run.Request with { OutputRoot = demand.OutputRoot, Limits = limits with {
                    max_entry_bytes = Math.Min(limits.max_entry_bytes, demand.OutputLimit), max_output_bytes = demand.OutputLimit } };
                run.Output = new ArchiveWorkerOutputBridge(request, log);
                run.Output.BindZoneIdentifier(run.VirtualZone);
                run.Output.CommitCheckpointProbe = stage => OutputCommitCheckpointProbe?.Invoke(stage);
                run.Output.WriteCheckpointProbe = OutputWriteCheckpointProbe;
                run.Output.CreationOutput.NamedStreamsFlagsProbe = NamedStreamsFlagsProbe;
                run.Resources.Add(run.Output);
            }
            run.VirtualOutputs.Add(run.Output ?? throw new ArchiveFault("stdout_invalid_frame"));
            run.VirtualAcquired = true;
            Diagnostic("member_started", run);
            return new { version = 1, operation_id = run.Request.OperationId, op = "native_result", call_id = call.GetProperty("call_id").GetGuid(), success = true };
        }
    }
    private void ReadyVirtualMember(Run run, JsonElement frame)
    {
        lock (gate) {
            var ready = frame.GetProperty("member_ready").Deserialize<MemberReady>() ?? throw new ArchiveFault("stdout_invalid_frame");
            if (run.Request.DragMaterialization is null || run.VirtualDemand is not { } demand || !run.VirtualAcquired || run.Failure is not null ||
                ready.request_id != demand.RequestId || ready.index != demand.Index || ready.ordinal != demand.Ordinal || ready.bytes > demand.OutputLimit ||
                run.Output?.FilesCommitted != 1 || run.Output.BytesCommitted != ready.bytes || run.Output.LastCommitState != ArchiveCommitState.Committed)
                throw new ArchiveFault("stdout_invalid_frame");
            run.VirtualDemand = null; run.VirtualReadyRequest = demand.RequestId; run.VirtualSealing = true;
        }
        Diagnostic("member_ready", run);
        try { Emit(new("virtual_member_ready", run.Request.OperationId, Frame: frame.Clone(), ElapsedMs: run.Clock.ElapsedMilliseconds)); }
        finally { lock (gate) { run.VirtualSealing = false; run.VirtualReadyRequest = null; } }
    }
    private void CheckWorkTimeout(Run run)
    {
        lock (gate) if (run.Request.TimeoutMs > 0 && run.WorkElapsedMs >= run.Request.TimeoutMs) run.Fail("timeout");
    }
    internal void ConfirmVirtualMember(Guid operation, Guid request, Action publish)
    {
        lock (gate) {
            if (!active.TryGetValue(operation, out var run) || !run.VirtualSealing || run.VirtualReadyRequest != request || run.SawTerminal)
                throw new ArchiveOutputException("source_unavailable");
            CheckWorkTimeout(run);
            if (run.Failure is { } failure) throw new ArchiveOutputException(failure.Category, failure.NativeCode);
            publish();
            run.VirtualSealing = false; run.VirtualReadyRequest = null;
        }
    }
}
