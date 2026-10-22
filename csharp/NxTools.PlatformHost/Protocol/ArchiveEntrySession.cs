using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Admission/control is immediate. All source/cache/process work is detached; a materialize
// command's one result waits for native exited/actualcleanup, never for an external Shell app.
internal sealed class ArchiveEntrySession : IAsyncDisposable
{
    private readonly object gate = new();
    private readonly ArchiveWorkerRunner runner;
    private readonly ArchiveEntryCache cache;
    private readonly IPlatformLog? log;
    private readonly Dictionary<Guid, Pending> active = [];
    private readonly HashSet<Guid> retired = [];
    private readonly Queue<Guid> recent = new();
    private bool stopped;
    internal Action<ProtocolEnvelope>? PasswordPrompt { get; set; }
    private sealed class Pending(ArchiveEntryCommand command)
    {
        internal readonly ArchiveEntryCommand Command = command;
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly TaskCompletionSource<WindowsCommandResult> Done = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal ArchiveEntryCache.Lease? Lease;
        internal string? Relative, Failure;
        internal ArchivePasswordTicket? PasswordTicket;
        internal bool Complete, Cancelled, Finishing;
    }
    internal ArchiveEntrySession(IPlatformLog? log = null) : this(ArchiveReadHelper.Production(), ArchiveEntryCache.Production(log), log) { }
    private ArchiveEntrySession(ArchiveReadHelper helper, ArchiveEntryCache cache, IPlatformLog? log)
    { this.cache = cache; this.log = log; runner = ArchiveWorkerRunner.ForBrowserEntry(helper, OnRunner, log); }
    internal static ArchiveEntrySession ForOwnedTestPackage(string root, IPlatformLog? log = null) => new(ArchiveReadHelper.OwnedPackage(root), ArchiveEntryCache.ForOwnedTestCache(root), log);
    internal Task<WindowsCommandResult> Dispatch(ProtocolEnvelope envelope)
    {
        ArchiveEntryCommand command;
        try { command = ArchiveEntryWire.Parse(envelope); }
        catch { return Task.FromResult(ArchiveEntryWire.Result(new(1, Guid.Empty, Guid.Empty, 0, false, null, null, "invalid_request"))); }
        lock (gate) {
            WindowsCommandResult Result(bool success, string? code = null, Guid? lease = null, bool retained = false) => ArchiveEntryWire.Result(new(1, command.OperationId, command.SessionId, command.Generation, success, lease, null, code, Retained: retained));
            if (stopped) return Task.FromResult(Result(false, "disconnected"));
            if (command.Action == "release") {
                if (active.ContainsKey(command.OperationId)) return Task.FromResult(Result(false, "busy"));
                return Task.Run(() => { lock (cache) {
                    if (!cache.Owns(command.LeaseId!.Value, command.OperationId, command.SessionId, command.Generation)) return Result(false, "invalid_request");
                    var removed = cache.Release(command.LeaseId.Value); return Result(true, lease: command.LeaseId, retained: !removed);
                } });
            }
            if (command.Action == "cancel") {
                if (!active.TryGetValue(command.OperationId, out var prior) || prior.Command.SessionId != command.SessionId || prior.Command.Generation != command.Generation || prior.Cancelled) return Task.FromResult(Result(false, "invalid_request"));
                prior.Cancelled = true; prior.PasswordTicket = null; runner.Cancel(command.OperationId); return Task.FromResult(Result(true));
            }
            if (active.ContainsKey(command.OperationId) || retired.Contains(command.OperationId)) return Task.FromResult(Result(false, "invalid_request"));
            if (active.Count >= 2) return Task.FromResult(Result(false, "busy"));
            var pending = new Pending(command); active.Add(command.OperationId, pending); Diagnostic("accepted", pending);
            _ = Task.Run(() => Start(pending)); return pending.Done.Task;
        }
    }
    private void Start(Pending pending)
    {
        try {
            lock (gate) { if (stopped || pending.Cancelled) { Finish(pending, "cancelled", true); return; } }
            ArchiveEntryCache.Lease lease;
            lock (cache) lease = cache.Create(pending.Command.OperationId, pending.Command.SessionId, pending.Command.Generation, pending.Command.LeaseId);
            lock (gate) pending.Lease = lease;
            var command = pending.Command;
            runner.Start(new(command.OperationId, "extract", command.Path!, (int)command.Codepage,
                new ArchiveAdmittedLimits(100000, 134217728, 67108864, 536870912, 536870912), OutputRoot: Path.Combine(lease.Root, "files"),
                ExtractOptions: new(command.CopyMotw ?? true), ActiveTemporarySource: true, BrowserEntry: command, PasswordInteraction: command.PasswordInteraction));
            lock (gate) { if (stopped || pending.Cancelled) runner.Cancel(command.OperationId); }
        } catch (ArchiveOutputException failure) { lock (gate) Finish(pending, ArchiveReadSession.Map(failure.Stage), true, failure.NativeCode); }
        catch { lock (gate) Finish(pending, "io_failure", true); }
    }
    // This transport accepts only the currently installed worker ticket; never carries the
    // secret in a public archive command, result, diagnostic or persistence record.
    internal ArchivePasswordTicket? PendingPassword(Guid operation) {
        lock (gate) return active.TryGetValue(operation, out var pending) && !stopped && !pending.Cancelled && !pending.Finishing ? pending.PasswordTicket : null;
    }
    internal bool RespondPassword(Guid operation, Guid session, ulong generation, ArchivePasswordTicket ticket, string? password) {
        lock (gate) {
            if (!active.TryGetValue(operation, out var pending) || stopped || pending.Cancelled || pending.Finishing ||
                pending.Command.SessionId != session || pending.Command.Generation != generation ||
                pending.PasswordTicket != ticket) return false;
            if (!runner.RespondPassword(operation, ticket, password)) return false;
            pending.PasswordTicket = null;
            return true;
        }
    }
    private void OnRunner(ArchiveWorkerEvent item)
    {
        lock (gate) {
            if (!active.TryGetValue(item.OperationId, out var pending)) return;
            if (item.Kind is "accepted" or "started") { Diagnostic(item.Kind, pending); return; }
            if (item.Kind == "frame" && item.Frame is { } passwordFrame && passwordFrame.GetProperty("op").GetString() == "password_state") {
                try {
                    var state = passwordFrame.GetProperty("password_state");
                    pending.PasswordTicket = !pending.Cancelled && !pending.Finishing &&
                        state.GetProperty("state").GetString() == "awaiting_password" &&
                        state.TryGetProperty("ticket", out var ticket) && ticket.ValueKind == JsonValueKind.Object
                        ? ticket.Deserialize<ArchivePasswordTicket>() : null;
                    if (pending.PasswordTicket is not null) {
                        var prompt = ArchiveBrowserPasswordPrompt.From(passwordFrame, item.OperationId, pending.Command.SessionId, pending.Command.Generation);
                        if (prompt is not null) PasswordPrompt?.Invoke(prompt.Envelope());
                    }
                } catch { pending.Failure = "protocol_violation"; runner.Cancel(item.OperationId); }
                return;
            }
            if (item.Kind == "browser_output" && item.Frame is { } output) {
                if (pending.Relative is not null) { pending.Failure = "protocol_violation"; runner.Cancel(item.OperationId); }
                else pending.Relative = output.GetProperty("path").GetString();
                return;
            }
            if (item.Kind == "frame" && item.Frame is { } frame) {
                if (frame.GetProperty("op").GetString() == "failure") pending.Failure = ArchiveReadSession.Map(frame.GetProperty("error_code").GetString());
                if (frame.GetProperty("op").GetString() == "complete") pending.Complete = frame.GetProperty("files_verified").GetUInt64() == 1;
                return;
            }
            if (item.Kind == "rejected") { Finish(pending, ArchiveReadSession.Map(item.Category), true, item.NativeErrorCode); return; }
            if (item.Kind != "exited") return;
            var code = !item.CleanupComplete ? "cleanup_incomplete" : stopped ? "disconnected" : pending.Cancelled ? "cancelled" :
                item.Category is "source_changed" or "source_unavailable" ? item.Category : pending.Failure ??
                (item.Category == "success" && pending.Complete && item.FilesCommitted == 1 && pending.Relative is not null ? null : ArchiveReadSession.Map(item.Category));
            Finish(pending, code, item.CleanupComplete, item.NativeErrorCode);
        }
    }
    private void Finish(Pending pending, string? code, bool cleanup, int? native = null)
    {
        pending.PasswordTicket = null;
        if (pending.Finishing) return;
        pending.Finishing = true;
        _ = Task.Run(() => Complete(pending, code, cleanup, native));
    }
    private void Complete(Pending pending, string? code, bool cleanup, int? native)
    {
        string? path = null;
        if (code is null && pending.Lease is { } lease && pending.Relative is { } relative) {
            try { path = Path.Combine(lease.Root, "files", relative.Replace('/', '\\')); lock (cache) lease.Seal(path); }
            catch { code = "io_failure"; path = null; }
        }
        lock (gate) {
            if (stopped || pending.Cancelled) { code = stopped ? "disconnected" : "cancelled"; path = null; }
            var retained = false;
            if (code is not null && cleanup && pending.Lease is { } failedLease) {
                lock (cache) retained = !cache.Abort(failedLease.Id);
                pending.Lease = null;
            }
            var result = new ArchiveEntryResult(1, pending.Command.OperationId, pending.Command.SessionId, pending.Command.Generation, code is null,
                pending.Lease?.Id, path, code, native, cleanup, retained || code is not null && pending.Lease is not null);
            pending.Done.TrySetResult(ArchiveEntryWire.Result(result)); Diagnostic("terminal", pending, result); Diagnostic("cleanup", pending, result);
            active.Remove(pending.Command.OperationId); retired.Add(pending.Command.OperationId); recent.Enqueue(pending.Command.OperationId);
            if (recent.Count > 4096) retired.Remove(recent.Dequeue());
        }
    }
    private void Diagnostic(string phase, Pending pending, ArchiveEntryResult? result = null) {
        try { log?.Write("info", "archive.entry." + phase, pending.Command.OperationId, new Dictionary<string, object?> {
            ["generation"] = pending.Command.Generation, ["elapsed_ms"] = pending.Clock.ElapsedMilliseconds,
            ["category"] = result?.Code, ["native_error_code"] = result?.NativeErrorCode, ["cleanup_complete"] = result?.CleanupComplete }); } catch { }
    }
    internal void Stop() { lock (gate) { stopped = true; foreach (var item in active.Values) { item.Cancelled = true; item.PasswordTicket = null; } runner.Stop(); } }
    public async ValueTask DisposeAsync() { Stop(); if (await runner.DrainAsync()) { Task[] pending; lock (gate) pending = active.Values.Select(p => p.Done.Task).ToArray(); await Task.WhenAll(pending); lock (cache) cache.Dispose(); } runner.Dispose(); }
}
