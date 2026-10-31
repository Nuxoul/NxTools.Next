using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Admission/control is immediate. All source/cache/process work is detached; a materialize
// command's one result waits for native exited/actualcleanup, never for an external Shell app.
internal sealed partial class ArchiveEntrySession : IAsyncDisposable
{
    private readonly object gate = new();
    private readonly ArchiveWorkerRunner runner;
    private readonly ArchiveEntryCache cache;
    private readonly IPlatformLog? log;
    private readonly Dictionary<Guid, Pending> active = [];
    private readonly HashSet<Guid> retired = [];
    private readonly System.Collections.Concurrent.ConcurrentDictionary<Guid, (Guid Operation, Guid Session, ulong Generation)> previewLeases = new();
    private readonly Queue<Guid> recent = new();
    private bool stopped;
    internal Action<ProtocolEnvelope>? PasswordPrompt { get; set; }
    private sealed class Pending(ArchiveEntryCommand command,ulong outputLimit)
    {
        internal readonly ArchiveEntryCommand Command = command;
        internal readonly ulong OutputLimit=outputLimit;
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly TaskCompletionSource<WindowsCommandResult> Done = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal ArchiveSourceActivity.Reservation? SourceActivity;
        internal ArchiveEntryCache.Lease? Lease;
        internal string? Relative, Failure;
        internal ArchivePasswordTicket? PasswordTicket;
        internal ProtectedSecret? SuppliedSecret;
        internal Guid? AuthorizationId;
        internal CancellationTokenRegistration AuthorizationStop;
        internal int? FailureNative;
        internal bool Complete, Cancelled, Finishing;
    }
    internal ArchiveEntrySession(IPlatformLog? log = null) : this(ArchiveReadHelper.Production(), ArchiveEntryCache.Production(log), log) { }
    private ArchiveEntrySession(ArchiveReadHelper helper, ArchiveEntryCache cache, IPlatformLog? log, Func<DateTimeOffset>? utcNow = null)
    {
        this.cache = cache; this.log = log; this.utcNow = utcNow ?? (() => DateTimeOffset.UtcNow);
        runner = ArchiveWorkerRunner.ForBrowserEntry(helper, OnRunner, log);
        authorizationTimer = new System.Threading.Timer(_ => ExpireAuthorizations(), null, TimeSpan.FromSeconds(1), TimeSpan.FromSeconds(1));
    }
    internal static ArchiveEntrySession ForOwnedTestPackage(string root, IPlatformLog? log = null, Func<DateTimeOffset>? utcNow = null) => new(ArchiveReadHelper.OwnedPackage(root), ArchiveEntryCache.ForOwnedTestCache(root), log, utcNow);
    internal void ProbeWriteForFixture(Action<int> probe)=>runner.OutputWriteCheckpointProbe=probe;
    internal const ulong MaterializeEntryLimit=32UL<<30;
    internal const ulong MaterializeOutputLimit=100UL<<30;
    // Internal caller may only narrow original worker/output authority; never a public wire budget.
    internal Task<WindowsCommandResult> Dispatch(ProtocolEnvelope envelope,ulong outputLimit=MaterializeOutputLimit, Guid? authorizationId = null)
    {
        ArchiveEntryCommand command;
        try { command = ArchiveEntryWire.Parse(envelope); }
        catch { return Task.FromResult(ArchiveEntryWire.Result(new(1, Guid.Empty, Guid.Empty, 0, false, null, null, "invalid_request"))); }
        lock (gate) {
            if(outputLimit==0||outputLimit>MaterializeOutputLimit)return Task.FromResult(ArchiveEntryWire.Result(new(1,command.OperationId,command.SessionId,command.Generation,false,null,null,"resource_limit")));
            // Purpose is immutable admission state; previews cannot inherit materialize authority.
            outputLimit = Math.Min(outputLimit, command.Action switch { "preview" => 8ul << 20, "preview_text" => 64ul << 10, _ => MaterializeOutputLimit });
            WindowsCommandResult Result(bool success, string? code = null, Guid? lease = null, bool retained = false) => ArchiveEntryWire.Result(new(1, command.OperationId, command.SessionId, command.Generation, success, lease, null, code, Retained: retained));
            if (stopped) return Task.FromResult(Result(false, "disconnected"));
            ExpireAuthorizationsLocked();
            if (command.Action == "revoke") return Task.FromResult(RevokeAuthorization(command));
            if (command.Action == "open") return previewLeases.ContainsKey(command.LeaseId!.Value) ? Task.FromResult(Result(false, "unsupported_operation")) : OpenLease(command);
            if (command.Action == "release") {
                if (active.ContainsKey(command.OperationId)) return Task.FromResult(Result(false, "busy"));
                return Task.Run(() => { lock (cache) {
                    if (!cache.OwnsOrReleased(command.LeaseId!.Value, command.OperationId, command.SessionId, command.Generation)) return Result(false, "invalid_request");
                    var removed = cache.Release(command.LeaseId.Value);
                    if (removed) previewLeases.TryRemove(command.LeaseId.Value, out _);
                    return Result(true, lease: command.LeaseId, retained: !removed);
                } });
            }
            if (command.Action == "cancel") {
                if (!active.TryGetValue(command.OperationId, out var prior) || prior.Command.SessionId != command.SessionId || prior.Command.Generation != command.Generation || prior.Cancelled) return Task.FromResult(Result(false, "invalid_request"));
                prior.Cancelled = true; prior.PasswordTicket = null; ClearSuppliedSecret(prior); runner.Cancel(command.OperationId); return Task.FromResult(Result(true));
            }
            if (active.ContainsKey(command.OperationId) || retired.Contains(command.OperationId) || authorizations.ContainsKey(command.OperationId)) return Task.FromResult(Result(false, "invalid_request"));
            if (active.Count >= 2) return Task.FromResult(Result(false, "busy"));
            if (command.Action == "authorize" && authorizations.Count + active.Values.Count(p => p.Command.Action == "authorize") >= 8) return Task.FromResult(Result(false, "resource_limit"));
            if (authorizationId is { } grantId && (command.Action != "materialize" || command.PasswordInteraction || !TryAuthorization(grantId, command, out _))) return Task.FromResult(Result(false, "authorization_invalid"));
            ArchiveSourceActivity.Reservation sourceActivity;
            try { sourceActivity = ArchiveSourceLease.ReserveActivity(command.Path!, false, command.OperationId); }
            catch (ArchiveOutputException failure) { return Task.FromResult(ArchiveEntryWire.Result(new(1, command.OperationId, command.SessionId, command.Generation, false, null, null,
                failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal) ? "source_unavailable" : ArchiveReadSession.Map(failure.Stage), failure.NativeCode))); }
            var pending = new Pending(command,outputLimit) { AuthorizationId = authorizationId, SourceActivity = sourceActivity }; active.Add(command.OperationId, pending); Diagnostic("accepted", pending);
            if (authorizationId is { } boundId) pending.AuthorizationStop = authorizations[boundId].Token.Register(() => CancelAuthorizedEntry(pending));
            _ = Task.Run(() => Start(pending)); return pending.Done.Task;
        }
    }
    private void Start(Pending pending)
    {
        try {
            lock (gate) { if (stopped || pending.Cancelled) { Finish(pending, "cancelled", true); return; } }
            if (pending.Command.Action == "authorize") {
                var authorize = pending.Command;
                runner.Start(new(authorize.OperationId, "test", authorize.Path!, (int)authorize.Codepage,
                    new ArchiveAdmittedLimits(100000, 134217728, 67108864, MaterializeEntryLimit, MaterializeOutputLimit),
                    PasswordInteraction: true, BrowserAuthorize: authorize.Browser));
                lock (gate) { if (stopped || pending.Cancelled) runner.Cancel(authorize.OperationId); }
                return;
            }
            ArchiveEntryCache.Lease lease;
            lock (cache) {
                lease = cache.Create(pending.Command.OperationId, pending.Command.SessionId, pending.Command.Generation, pending.Command.LeaseId, pending.Command.TempPolicy);
                foreach (var item in previewLeases) if (!cache.Owns(item.Key, item.Value.Operation, item.Value.Session, item.Value.Generation)) previewLeases.TryRemove(item.Key, out _);
            }
            lock (gate) pending.Lease = lease;
            var command = pending.Command;
            // Keep the private worker's original shape/schema. Pending retains preview purpose.
            if (command.Action is "preview" or "preview_text") command = command with { Action = "materialize" };
            byte[]? password = null;
            try {
                lock (gate) {
                    if (pending.AuthorizationId is { } id) {
                        ExpireAuthorizationsLocked();
                        if (!TryAuthorization(id, command, out var grant)) { Finish(pending, "authorization_invalid", true); return; }
                        password = grant.Secret.CopyPlaintext();
                    }
                }
                runner.Start(new(command.OperationId, "extract", command.Path!, (int)command.Codepage,
                    new ArchiveAdmittedLimits(100000, 134217728, 67108864, Math.Min(pending.OutputLimit,MaterializeEntryLimit), pending.OutputLimit), OutputRoot: Path.Combine(lease.Root, "files"),
                    ExtractOptions: new(command.CopyMotw ?? true), ActiveTemporarySource: true, BrowserEntry: command, PasswordInteraction: command.PasswordInteraction), browserPassword: password);
            } finally { if (password is not null) System.Security.Cryptography.CryptographicOperations.ZeroMemory(password); }
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
            if (!active.TryGetValue(operation, out var pending) || pending.Command.Action == "authorize" || stopped || pending.Cancelled || pending.Finishing ||
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
                    if (pending.Command.Action == "authorize" && (state.GetProperty("state").GetString() == "awaiting_password" ||
                        state.GetProperty("state").GetString() == "attempt_finished" && state.TryGetProperty("error_code", out var attemptError) && attemptError.ValueKind == JsonValueKind.String)) ClearSuppliedSecret(pending);
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
                if (frame.GetProperty("op").GetString() == "complete") pending.Complete = pending.Command.Action == "authorize"
                    ? frame.GetProperty("files_verified").GetUInt64() > 0 : frame.GetProperty("files_verified").GetUInt64() == 1;
                return;
            }
            if (item.Kind == "rejected") { Finish(pending, ArchiveReadSession.Map(item.Category), true, item.NativeErrorCode); return; }
            if (item.Kind != "exited") return;
            var code = !item.CleanupComplete ? "cleanup_incomplete" : stopped ? "disconnected" : pending.Cancelled ? "cancelled" :
                item.Category is "source_changed" or "source_unavailable" ? item.Category : pending.Failure ??
                (item.Category == "success" && pending.Complete && (pending.Command.Action == "authorize" ? item.FilesCommitted == 0 && pending.Relative is null && pending.Lease is null && pending.SuppliedSecret is not null : item.FilesCommitted == 1 && pending.Relative is not null) ? null : ArchiveReadSession.Map(item.Category));
            if (code == "source_changed" && pending.AuthorizationId is { } changedId && authorizations.Remove(changedId, out var changedGrant)) {
                changedGrant.Dispose(); AuthorizationDiagnostic("source_changed", changedGrant.Command);
            }
            Finish(pending, code, item.CleanupComplete, pending.FailureNative ?? item.NativeErrorCode);
        }
    }
    private void Finish(Pending pending, string? code, bool cleanup, int? native = null)
    {
        pending.PasswordTicket = null;
        if (code is not null) ClearSuppliedSecret(pending);
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
            if (stopped || pending.Cancelled && code != "source_changed") { code = stopped ? "disconnected" : "cancelled"; path = null; }
            if (pending.AuthorizationId is { } authorizationId) {
                ExpireAuthorizationsLocked();
                if (code is null && !TryAuthorization(authorizationId, pending.Command, out _)) { code = "authorization_invalid"; path = null; }
            }
            var retained = false;
            if (code is not null && cleanup && pending.Lease is { } failedLease) {
                lock (cache) retained = !cache.Abort(failedLease.Id);
                pending.Lease = null;
            }
            Guid? authorization = null;
            if (pending.Command.Action == "authorize") {
                if (code is null && cleanup && pending.Complete && pending.SuppliedSecret is { } secret && authorizations.Count < 8) {
                    authorization = pending.Command.OperationId;
                    authorizations.Add(authorization.Value, new(pending.Command, secret, utcNow().AddSeconds(120)));
                    pending.SuppliedSecret = null;
                } else code ??= "authorization_failed";
            }
            ClearSuppliedSecret(pending);
            // Never wait for token callbacks while holding this owner's gate.
            pending.AuthorizationStop.Unregister();
            if (code is null && cleanup && pending.Lease is { } sourceBoundLease && pending.SourceActivity is { } sourceActivity) {
                lock (cache) sourceBoundLease.BindSourceActivity(sourceActivity);
                pending.SourceActivity = null;
            }
            if (code is null && pending.Lease is { } previewLease && pending.Command.Action is "preview" or "preview_text") previewLeases.TryAdd(previewLease.Id, (pending.Command.OperationId, pending.Command.SessionId, pending.Command.Generation));
            var result = new ArchiveEntryResult(1, pending.Command.OperationId, pending.Command.SessionId, pending.Command.Generation, code is null,
                authorization ?? pending.Lease?.Id, path, code, native, cleanup, retained || code is not null && pending.Lease is not null);
            pending.Done.TrySetResult(ArchiveEntryWire.Result(result)); Diagnostic("terminal", pending, result); Diagnostic("cleanup", pending, result);
            if (cleanup) pending.SourceActivity?.Dispose();
            if (cleanup) active.Remove(pending.Command.OperationId); retired.Add(pending.Command.OperationId); recent.Enqueue(pending.Command.OperationId);
            if (recent.Count > 4096) retired.Remove(recent.Dequeue());
        }
    }
    private void Diagnostic(string phase, Pending pending, ArchiveEntryResult? result = null) {
        try { log?.Write("info", "archive.entry." + phase, pending.Command.OperationId, new Dictionary<string, object?> {
            ["generation"] = pending.Command.Generation, ["elapsed_ms"] = pending.Clock.ElapsedMilliseconds,
            ["purpose"] = pending.Command.Action,
            ["category"] = result?.Code, ["native_error_code"] = result?.NativeErrorCode, ["cleanup_complete"] = result?.CleanupComplete }); } catch { }
    }
    internal IntPtr OpenVirtualStream(ArchiveEntryResult result,long? expectedSize,CancellationToken cancellationToken,out long actualSize)
    {
        if(!result.Success||result.LeaseId is not {} id||!result.CleanupComplete)throw new ArchiveOutputException("source_unavailable");
        lock(cache)return cache.OpenVirtualStream(id,result.OperationId,result.SessionId,result.Generation,expectedSize,cancellationToken,out actualSize);
    }
    internal void Stop() {
        lock (gate) {
            if (stopped) return;
            stopped = true; externalStop.Cancel(); authorizationTimer.Dispose();
            foreach (var use in external.Values) use.Done.TrySetResult(OpenResult(use, false, "disconnected"));
            foreach (var grant in authorizations.Values) grant.Dispose(); authorizations.Clear();
            foreach (var item in active.Values) { item.Cancelled = true; item.PasswordTicket = null; ClearSuppliedSecret(item); } runner.Stop();
            // A disconnected Rust decoder may still hold a read handle. Original cache Release
            // retains locked output for its bounded later sweep; never force-delete or recurse.
            var abandoned = previewLeases.Keys.ToArray();
            previewCleanup = Task.Run(() => { foreach (var id in abandoned) { lock (cache) cache.Release(id); } });
            previewLeases.Clear();
        }
    }
    private Task previewCleanup = Task.CompletedTask;
    public async ValueTask DisposeAsync() { Stop(); if (await runner.DrainAsync()) { Task[] pending; lock (gate) pending = active.Values.Select(p => p.Done.Task).ToArray(); await Task.WhenAll(pending); await previewCleanup; lock (cache) cache.Dispose(); } runner.Dispose(); }
}
