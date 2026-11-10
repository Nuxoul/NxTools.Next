using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

internal sealed partial class ArchiveEntrySession
{
    private readonly Dictionary<Guid, VirtualMaterialization> virtualRuns = [];
    internal sealed record VirtualMember(ArchiveEntryCommand Owner, Guid LeaseId, long Size, bool Borrowed = false, Guid DragOperation = default);
    internal sealed record VirtualMemberProgress(Guid RequestId, int Index, ulong Ordinal, ulong BytesDone, ulong? BytesTotal);
    internal sealed class VirtualMaterialization(ArchiveVirtualPlan plan, Guid operation, Guid? authorization, Action<VirtualMemberProgress>? progress)
    {
        internal readonly ArchiveVirtualPlan Plan = plan;
        internal readonly Guid Operation = operation;
        internal readonly Guid? Authorization = authorization;
        internal readonly Action<VirtualMemberProgress>? Progress = progress;
        internal long SealTicks, CursorTicks;
        internal int Seals, Cursors;
        internal readonly Dictionary<int, VirtualPending> Members = [];
        internal readonly List<VirtualMember> Borrowed = [];
        internal readonly TaskCompletionSource<ArchiveWorkerEvent> Terminal = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal VirtualPending? Current;
        internal bool Started, Closing, Cancelled, Complete;
        internal string? Failure;
    }
    internal sealed class VirtualPending(int index, ArchiveEntryCommand command, ArchiveEntryCache.Lease lease, ulong limit)
    {
        internal readonly int Index = index;
        internal readonly ArchiveEntryCommand Command = command;
        internal readonly ArchiveEntryCache.Lease Lease = lease;
        internal readonly ulong Limit = limit;
        internal readonly TaskCompletionSource<VirtualMember> Done = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal string? Relative;
        internal bool Ready;
        internal ulong BytesProcessed;
        internal ulong? BytesTotal;
    }
    internal VirtualMaterialization CreateVirtualMaterialization(ArchiveVirtualPlan plan, Guid? authorization, Guid? operation = null, Action<VirtualMemberProgress>? progress = null)
    {
        lock (gate) {
            if (stopped || !TryCaptureVirtualAuthorization(authorization, plan.Items, out _)) throw new ArchiveOutputException("authorization_invalid");
            var context = new VirtualMaterialization(plan, operation ?? Guid.NewGuid(), authorization, progress);
            if (active.ContainsKey(context.Operation) || retired.Contains(context.Operation) || !virtualRuns.TryAdd(context.Operation, context))
                throw new ArchiveOutputException("invalid_request");
            return context;
        }
    }
    internal Task<VirtualMember> DemandVirtual(VirtualMaterialization context, int index, ulong limit)
    {
        lock (gate) {
            if (stopped || context.Closing || context.Cancelled || context.Terminal.Task.IsCompleted ||
                !virtualRuns.TryGetValue(context.Operation, out var current) || !ReferenceEquals(context, current)) throw new ArchiveOutputException("source_unavailable");
            var command = context.Plan[index].Entry;
            if (context.Plan[index].Directory || context.Members.ContainsKey(index) || context.Current is not null || limit is 0 or > MaterializeOutputLimit)
                throw new ArchiveOutputException("invalid_request");
            if (!TryCaptureVirtualAuthorization(context.Authorization, context.Plan.Items, out _)) throw new ArchiveOutputException("authorization_invalid");
            if (TryAdoptPreview(context, command, limit) is { } preview) return Task.FromResult(preview);
            if (!context.Started && active.Count + virtualRuns.Values.Count(run => run.Started) >= 2) throw new ArchiveOutputException("busy");
            ArchiveEntryCache.Lease lease;
            lock (cache) lease = cache.Create(command.OperationId, command.SessionId, command.Generation);
            var pending = new VirtualPending(index, command, lease, limit);
            context.Members.Add(index, pending); context.Current = pending;
            var demand = new ArchiveDragDemand(command.OperationId, index, command.Ordinal!.Value, limit, Path.Combine(lease.Root, "files"));
            if (!context.Started) {
                context.Started = true;
                _ = Task.Run(() => StartVirtual(context, pending, demand));
            } else if (!runner.DemandVirtual(context.Operation, demand)) {
                context.Failure = "source_unavailable"; context.Cancelled = true; runner.Cancel(context.Operation);
            }
            return pending.Done.Task;
        }
    }
    private void StartVirtual(VirtualMaterialization context, VirtualPending first, ArchiveDragDemand demand)
    {
        byte[]? password = null;
        try {
            lock (gate) {
                if (stopped || context.Cancelled) { CompleteVirtualRejected(context, "cancelled"); return; }
                if (context.Authorization is { } id) {
                    ExpireAuthorizationsLocked();
                    if (!TryAuthorization(id, first.Command, out var grant)) { CompleteVirtualRejected(context, "authorization_invalid"); return; }
                    password = grant.Secret.CopyPlaintext();
                }
            }
            var command = first.Command with { OperationId = context.Operation };
            runner.Start(new(context.Operation, "extract", command.Path!, (int)command.Codepage,
                new ArchiveAdmittedLimits(100000, 134217728, 67108864, MaterializeEntryLimit, MaterializeOutputLimit),
                OutputRoot: demand.OutputRoot, ExtractOptions: new(command.CopyMotw ?? true), ActiveTemporarySource: true,
                BrowserEntry: command, DragMaterialization: new(context.Plan.Members, demand)), browserPassword: password);
            lock (gate) if (stopped || context.Cancelled) runner.Cancel(context.Operation);
        } catch (ArchiveOutputException failure) { lock (gate) CompleteVirtualRejected(context, ArchiveReadSession.Map(failure.Stage), failure.NativeCode); }
        catch { lock (gate) CompleteVirtualRejected(context, "io_failure"); }
        finally { if (password is not null) System.Security.Cryptography.CryptographicOperations.ZeroMemory(password); }
    }
    private void CompleteVirtualRejected(VirtualMaterialization context, string code, int? native = null)
    {
        context.Failure = code;
        context.Current?.Done.TrySetException(new ArchiveOutputException(code, native));
        context.Terminal.TrySetResult(new("rejected", context.Operation, code, native, CleanupComplete: true));
    }
    // Presentation snapshots never authorize output or change the worker's terminal outcome.
    private static void ObserveVirtualProgress(VirtualMaterialization context, System.Text.Json.JsonElement frame)
    {
        if (context.Cancelled || context.Terminal.Task.IsCompleted || context.Current is not { Ready:false }) return;
        var member = context.Current!;
        try {
            var progress = frame.GetProperty("progress");
            var demand = progress.GetProperty("drag_demand");
            if (demand.GetProperty("request_id").GetGuid() != member.Command.OperationId ||
                demand.GetProperty("index").GetInt32() != member.Index ||
                demand.GetProperty("ordinal").GetUInt64() != member.Command.Ordinal ||
                !progress.TryGetProperty("current_bytes_done", out var processed) || processed.ValueKind != JsonValueKind.Number) return;
            var done = processed.GetUInt64();
            ulong? total = null;
            if (progress.TryGetProperty("current_bytes_total", out var totalField)) {
                if (totalField.ValueKind == JsonValueKind.Number) total = totalField.GetUInt64();
                else if (totalField.ValueKind != JsonValueKind.Null) return;
            }
            if (done < member.BytesProcessed || done > member.Limit || total is { } known && done > known ||
                member.BytesTotal is { } previous && total != previous) return;
            member.BytesProcessed = done; member.BytesTotal = total;
            try { context.Progress?.Invoke(new(member.Command.OperationId, member.Index, member.Command.Ordinal!.Value, done, total)); } catch { }
        } catch (Exception) { } // Malformed or stale presentation frames cannot fail materialization.
    }
    private bool OnVirtualRunner(ArchiveWorkerEvent item)
    {
        lock (gate) {
            if (!virtualRuns.TryGetValue(item.OperationId, out var context)) return false;
            if (item.Kind == "browser_output" && item.Frame is { } output) {
                if (context.Current is not { } member || member.Relative is not null) { context.Failure = "protocol_violation"; runner.Cancel(item.OperationId); }
                else member.Relative = output.GetProperty("path").GetString();
            } else if (item.Kind == "virtual_member_ready" && item.Frame is { } frame) {
                try {
                    var ready = frame.GetProperty("member_ready");
                    if (context.Current is not { } member || context.Cancelled || stopped || member.Relative is null ||
                        ready.GetProperty("request_id").GetGuid() != member.Command.OperationId || ready.GetProperty("index").GetInt32() != member.Index ||
                        ready.GetProperty("ordinal").GetUInt64() != member.Command.Ordinal) throw new ArchiveOutputException("protocol_violation");
                    long size;
                    var sealStarted = Stopwatch.GetTimestamp();
                    try { lock (cache) size = member.Lease.Seal(Path.Combine(member.Lease.Root, "files", member.Relative.Replace('/', '\\'))); }
                    finally { context.SealTicks += Stopwatch.GetTimestamp() - sealStarted; context.Seals++; }
                    if (size < 0 || (ulong)size != ready.GetProperty("bytes").GetUInt64() || (ulong)size > member.Limit) throw new ArchiveOutputException("resource_limit");
                    sealedCheckpointForFixture?.Invoke();
                    if (context.Authorization is { } id && !TryAuthorization(id, member.Command, out _)) throw new ArchiveOutputException("authorization_invalid");
                    runner.ConfirmVirtualMember(context.Operation, member.Command.OperationId, () => {
                        member.Ready = true; context.Current = null;
                        VirtualDiagnostic("member_ready", context, member.Index, bytes: (ulong)size);
                        member.Done.TrySetResult(new(member.Command, member.Lease.Id, size, DragOperation: context.Operation));
                    });
                } catch (Exception failure) {
                    context.Failure = failure is ArchiveOutputException outputFailure ? outputFailure.Stage : "io_failure";
                    context.Current?.Done.TrySetException(new ArchiveOutputException(context.Failure)); runner.Cancel(context.Operation);
                }
            } else if (item.Kind == "frame" && item.Frame is { } terminalFrame) {
                var op = terminalFrame.GetProperty("op").GetString();
                if (op == "progress") ObserveVirtualProgress(context, terminalFrame);
                if (op == "failure") context.Failure ??= ArchiveReadSession.Map(terminalFrame.GetProperty("error_code").GetString());
                if (op == "complete") context.Complete = context.Closing && context.Current is null &&
                    terminalFrame.GetProperty("files_verified").GetUInt64() == (ulong)context.Members.Values.Count(member => member.Ready);
            } else if (item.Kind == "rejected") CompleteVirtualRejected(context, ArchiveReadSession.Map(item.Category), item.NativeErrorCode);
            else if (item.Kind == "exited") {
                var code = !item.CleanupComplete ? "cleanup_incomplete" : stopped ? "disconnected" : context.Cancelled ? "cancelled" :
                    context.Failure ?? (item.Category == "success" && context.Complete ? null : ArchiveReadSession.Map(item.Category));
                if (context.Current is { } member) member.Done.TrySetException(new ArchiveOutputException(code ?? "protocol_violation", item.NativeErrorCode));
                VirtualDiagnostic("terminal", context, cleanup: item.CleanupComplete, code: code, native: item.NativeErrorCode);
                context.Terminal.TrySetResult(item with { Category = code ?? "success" });
            }
            return true;
        }
    }
    internal void CancelVirtual(VirtualMaterialization context)
    {
        lock (gate) {
            context.Cancelled = true; context.Closing = true;
            if (context.Started) runner.Cancel(context.Operation);
        }
    }
    internal async Task CloseVirtual(VirtualMaterialization context)
    {
        lock (gate) {
            context.Closing = true;
            if (context.Started && !context.Terminal.Task.IsCompleted && !context.Cancelled && !runner.EndVirtual(context.Operation)) runner.Cancel(context.Operation);
        }
        if (context.Started) {
            var terminal = await context.Terminal.Task;
            if (!terminal.CleanupComplete) throw new ArchiveOutputException("cleanup_incomplete");
        }
        lock (gate) {
            foreach (var pending in context.Members.Values) {
                lock (cache) {
                    if (!pending.Ready && pending.Relative is { } relative) {
                        var path = Path.Combine(pending.Lease.Root, "files", relative.Replace('/', '\\'));
                        if (File.Exists(path)) pending.Lease.Seal(path); // Cleanup stamp only; never grants member readiness.
                    }
                    cache.Release(pending.Lease.Id);
                }
            }
            foreach (var member in context.Borrowed) lock (cache) cache.EndConsumer(member.LeaseId);
            VirtualDiagnostic("cleanup", context);
            virtualRuns.Remove(context.Operation);
            retired.Add(context.Operation); recent.Enqueue(context.Operation);
            if (recent.Count > 4096) retired.Remove(recent.Dequeue());
        }
    }
    internal IntPtr OpenVirtualStream(VirtualMember member, long? expected, CancellationToken stop, out long actual)
    {
        var owner = member.Owner;
        var started = Stopwatch.GetTimestamp();
        try {
            lock (cache) return member.Borrowed
                ? cache.OpenConsumerVirtualStream(member.LeaseId, owner.OperationId, owner.SessionId, owner.Generation, expected, stop, out actual)
                : cache.OpenVirtualStream(member.LeaseId, owner.OperationId, owner.SessionId, owner.Generation, expected, stop, out actual);
        } finally {
            lock (gate) if (virtualRuns.TryGetValue(member.DragOperation, out var context)) {
                context.CursorTicks += Stopwatch.GetTimestamp() - started; context.Cursors++;
            }
        }
    }
    private VirtualMember? TryAdoptPreview(VirtualMaterialization context, ArchiveEntryCommand command, ulong limit)
    {
        foreach (var pair in previewLeases) {
            var preview = pair.Value;
            if (!Matches(preview.Command, command) || preview.Command.Ordinal != command.Ordinal || preview.Encrypted && context.Authorization is null) continue;
            lock (cache) {
                if (!cache.HoldConsumer(pair.Key, preview.Command.OperationId, preview.Command.SessionId, preview.Command.Generation, out var size)) continue;
                if (size < 0 || (ulong)size > limit) { cache.EndConsumer(pair.Key); throw new ArchiveOutputException("resource_limit"); }
                var member = new VirtualMember(preview.Command, pair.Key, size, true, context.Operation); context.Borrowed.Add(member);
                VirtualDiagnostic("preview_adopted", context, bytes: (ulong)size); return member;
            }
        }
        return null;
    }
    private void VirtualDiagnostic(string phase, VirtualMaterialization context, int? index = null, ulong? bytes = null, bool? cleanup = null, string? code = null, int? native = null)
    {
        try {
            if (log is null) return;
            var fields = new Dictionary<string, object?> {
                ["index"] = index, ["bytes"] = bytes, ["elapsed_ms"] = context.Clock.ElapsedMilliseconds,
                ["cleanup_complete"] = cleanup, ["category"] = code, ["native_error_code"] = native };
            if (phase is "terminal" or "cleanup") {
                fields["seal_count"] = context.Seals; fields["seal_ms"] = context.SealTicks * 1000 / Stopwatch.Frequency;
                fields["cursor_count"] = context.Cursors; fields["cursor_ms"] = context.CursorTicks * 1000 / Stopwatch.Frequency;
            }
            log.Write("info", "archive.entry.drag." + phase, context.Operation, fields);
        } catch { }
    }
}
