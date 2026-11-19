using System.Diagnostics;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// Per connected pipe: admission/control never waits for a process or metadata consumer.
internal sealed class ArchiveReadSession : IAsyncDisposable
{
    private readonly object gate = new();
    private readonly Channel<ProtocolEnvelope> events = Channel.CreateBounded<ProtocolEnvelope>(8);
    private readonly Dictionary<Guid, Operation> active = [];
    private readonly HashSet<Guid> retired = [];
    private readonly Queue<Guid> recent = new();
    private readonly ArchiveWorkerRunner runner;
    private readonly IPlatformLog? log;
    private bool stopped;
    internal ChannelReader<ProtocolEnvelope> Events => events.Reader;
    internal int ActiveCount { get { lock (gate) return active.Count; } }
    private sealed class Operation(ArchiveReadCommand command)
    {
        internal readonly ArchiveReadCommand Command = command;
        internal readonly ArchiveReadLimits Limits = command.Limits ?? new();
        internal readonly ArchiveReadMetadata Metadata = new(command.Limits ?? new(), command.Codepage, command.Version);
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal TaskCompletionSource<bool>? Credit;
        internal ulong? WaitingSequence;
        internal bool Negotiated, Complete;
        internal string? Failure;
        internal ulong Files, Bytes;
        internal int? Native;
        internal bool BrowserInteraction;
        internal ArchiveBrowserPasswordPrompt? PasswordPrompt;
    }
    internal ArchiveReadSession(IPlatformLog? log = null) : this(ArchiveReadHelper.Production(), log) { }
    private ArchiveReadSession(ArchiveReadHelper helper, IPlatformLog? log)
    { this.log = log; runner = ArchiveWorkerRunner.ForProductRead(helper, OnRunnerEvent, OnFrameAsync, log); }
    internal static ArchiveReadSession ForOwnedTestPackage(string root, IPlatformLog? log = null) =>
        new(ArchiveReadHelper.OwnedPackage(root), log);
    internal static ArchiveReadSession ForOwnedWorkerFixture(string root, IPlatformLog? log = null) =>
        new(ArchiveReadHelper.OwnedPackage(root, fixture: true), log);

    internal WindowsCommandResult Dispatch(ProtocolEnvelope envelope, bool browserInteraction = false)
    {
        ArchiveReadCommand command;
        try { command = ArchiveReadWire.Parse(envelope); }
        catch (Exception failure) {
            var identity = ArchiveReadWire.RejectionIdentity(envelope);
            lock (gate) {
                if (identity is not null && !active.ContainsKey(identity.OperationId) && !retired.Contains(identity.OperationId)) return Reject(identity, failure is ArchiveReadVersionException ? "incompatible_version" : "invalid_request");
            }
            return ArchiveReadWire.Result(new(false, failure is ArchiveReadVersionException ? "incompatible_version" : "invalid_request"));
        }
        lock (gate)
        {
            if (command.Action is "ack" or "cancel")
            {
                if (stopped || !active.TryGetValue(command.OperationId, out var operation) || command.Version != operation.Command.Version ||
                    operation.Command.SessionId != command.SessionId || operation.Command.Generation != command.Generation)
                    return ArchiveReadWire.Result(new(false, "invalid_request"));
                if (command.Action == "cancel") {
                    operation.Failure ??= "cancelled"; operation.Credit?.TrySetResult(false);
                    runner.Cancel(command.OperationId); return ArchiveReadWire.Result(new(true, null));
                }
                if (operation.WaitingSequence != command.Sequence || operation.Credit is not { } credit ||
                    operation.Failure is not null || credit.Task.IsCompleted)
                    return ArchiveReadWire.Result(new(false, "invalid_request"));
                operation.WaitingSequence = null;
                credit.TrySetResult(true); return ArchiveReadWire.Result(new(true, null));
            }
            // A rejected duplicate never replaces state or emits a second operation terminal.
            if (active.ContainsKey(command.OperationId) || retired.Contains(command.OperationId))
                return ArchiveReadWire.Result(new(false, "invalid_request"));
            if (stopped) return Reject(command, "disconnected");
            if (command.Version is not (1 or 2)) return Reject(command, "incompatible_version");
            if (active.Count >= 2 || runner.ActiveCount >= 2) return Reject(command, "busy");
            var state = new Operation(command); active.Add(command.OperationId, state);
            state.BrowserInteraction = browserInteraction && command.Version == 2 && command.Action == "open" && limitsForBrowser(command);
            var limits = state.Limits;
            var accepted = runner.Start(new(command.OperationId, command.Action, command.Path!, (int)command.Codepage,
                new ArchiveAdmittedLimits(limits.MaxEntries, limits.MaxMetadataBytes, limits.MaxWorkingBytes,
                    limits.MaxEntryBytes, limits.MaxOutputBytes), ReadContract: (int)command.Version,
                    ReadTransport: command.Version == 2 ? new(limits.MaxTransportBytes, limits.BrowserContext) : null, PasswordInteraction: state.BrowserInteraction));
            return ArchiveReadWire.Result(new(accepted, accepted ? null : state.Failure ?? "io_failure"));
        }
    }
    private static bool limitsForBrowser(ArchiveReadCommand command) => command.Limits?.BrowserContext == true;
    internal bool RespondPassword(ArchiveBrowserPasswordPrompt prompt, string? password) {
        lock (gate) {
            if (stopped || !active.TryGetValue(prompt.OperationId, out var operation) || !operation.BrowserInteraction || operation.Failure is not null ||
                operation.Command.SessionId != prompt.SessionId || operation.Command.Generation != prompt.Generation || operation.PasswordPrompt != prompt) return false;
            if (!runner.RespondPassword(prompt.OperationId, prompt.Ticket, password)) return false;
            operation.PasswordPrompt = null; return true;
        }
    }
    private WindowsCommandResult Reject(ArchiveReadCommand command, string code)
    {
        Retire(command.OperationId);
        var terminal = new ArchiveReadTerminal("rejected", code);
        Publish(ArchiveReadEvent.For(command, "terminal", terminal: terminal));
        Diagnostic("terminal", command, terminal);
        return ArchiveReadWire.Result(new(false, code));
    }
    private void Retire(Guid id) {
        retired.Add(id); recent.Enqueue(id);
        if (recent.Count > 4096) retired.Remove(recent.Dequeue());
    }
    private bool Publish(ArchiveReadEvent item)
    {
        if (stopped) return false;
        if (events.Writer.TryWrite(item.Envelope())) return true;
        // The transport cannot silently lose a terminal. Retire the pipe and cancel originals.
        stopped = true;
        foreach (var operation in active.Values) { operation.Failure ??= "consumer_queue_full"; operation.Credit?.TrySetResult(false); }
        runner.Stop("consumer_queue_full");
        events.Writer.TryComplete(new InvalidDataException("Archive consumer queue full."));
        return false;
    }
    private void OnRunnerEvent(ArchiveWorkerEvent item)
    {
        lock (gate)
        {
            if (!active.TryGetValue(item.OperationId, out var operation)) return;
            if (item.Kind == "accepted") {
                Publish(ArchiveReadEvent.For(operation.Command, "accepted")); Diagnostic("accepted", operation.Command); return;
            }
            if (item.Kind == "started") { Diagnostic("started", operation.Command); return; }
            if (item.Kind == "rejected") {
                operation.Failure = Map(item.Category); active.Remove(item.OperationId); Retire(item.OperationId);
                var rejected = new ArchiveReadTerminal("rejected", operation.Failure, ElapsedMs: (ulong)operation.Clock.ElapsedMilliseconds, NativeErrorCode: item.NativeErrorCode);
                Publish(ArchiveReadEvent.For(operation.Command, "terminal", terminal: rejected)); Diagnostic("terminal", operation.Command, rejected); return;
            }
            if (item.Kind != "exited") return;
            operation.Credit?.TrySetResult(false);
            // Native admission/timeout is the cause; worker termination frames are cleanup consequences.
            var code = !item.CleanupComplete ? "cleanup_incomplete" : item.Category == "timeout" ? "io_failure" : item.Category is "source_unavailable" or "source_changed" or "resource_limit" or "missing_volume" or "corrupt_archive" or "unsupported_format" or "unsupported_method" or "unsupported_encryption" ? item.Category : operation.Failure ??
                (item.Category == "success" ? operation.Complete && operation.Negotiated ? null : "protocol_violation" : Map(item.Category));
            var status = code is null ? "succeeded" : code == "cancelled" ? "cancelled" : code == "disconnected" ? "disconnected" : "failed";
            var terminal = new ArchiveReadTerminal(status, code, operation.Files, operation.Bytes,
                operation.Metadata.Entries, operation.Metadata.Batches, (ulong)item.ElapsedMs, operation.Native ?? item.NativeErrorCode, item.CleanupComplete,
                operation.Command.Version == 2 ? new(item.PrivateBytes ?? 0, operation.Metadata.Bytes, operation.Metadata.MaxBatchEntries,
                    operation.Metadata.MaxBatchBytes, operation.Metadata.Batches > 0 ? 1ul : 0ul, item.ProcessBudgetBytes ?? 0) : null,
                code is null ? operation.Metadata.Browser : null);
            // If native cleanup is stuck, runner retains its slot/leases even after logical retirement.
            active.Remove(item.OperationId); Retire(item.OperationId);
            Publish(ArchiveReadEvent.For(operation.Command, "terminal", terminal: terminal)); Diagnostic("terminal", operation.Command, terminal);
        }
    }
    private async Task<string?> OnFrameAsync(Guid id, JsonElement frame, CancellationToken token)
    {
        Task<bool>? wait = null; Operation operation;
        lock (gate)
        {
            if (!active.TryGetValue(id, out operation!) || stopped) return "disconnected";
            if (operation.Failure is { } failure) return failure;
            try
            {
                switch (frame.GetProperty("op").GetString())
                {
                    case "password_state":
                        if (!operation.BrowserInteraction) return operation.Failure = "protocol_violation";
                        var prompt = ArchiveBrowserPasswordPrompt.From(frame, id, operation.Command.SessionId, operation.Command.Generation);
                        operation.PasswordPrompt = prompt;
                        if (prompt is not null && !events.Writer.TryWrite(prompt.Envelope())) return operation.Failure = "consumer_queue_full";
                        return null;
                    case "accepted":
                        if (operation.Negotiated) return operation.Failure = "protocol_violation";
                        operation.Negotiated = true;
                        Publish(ArchiveReadEvent.For(operation.Command, "negotiated", ArchiveReadWire.Capabilities(operation.Command.Version).ToArray()));
                        Diagnostic("negotiated", operation.Command); return stopped ? "consumer_queue_full" : null;
                    case "browser_context":
                        if (!operation.Negotiated || operation.Complete || operation.Command.Action != "open") return operation.Failure = "protocol_violation";
                        operation.Metadata.ReadBrowser(frame); return null;
                    case "entry_batch":
                        if (!operation.Negotiated || operation.Complete || operation.Command.Action != "open" || operation.Credit is not null) return operation.Failure = "protocol_violation";
                        var batch = operation.Metadata.Read(frame);
                        operation.Credit = new(TaskCreationOptions.RunContinuationsAsynchronously);
                        operation.WaitingSequence = batch.Sequence; wait = operation.Credit.Task;
                        if (!Publish(ArchiveReadEvent.For(operation.Command, "batch", batch: batch))) return "consumer_queue_full";
                        break;
                    case "complete":
                        if (!operation.Negotiated || operation.Complete || !operation.Metadata.Complete(frame, operation.Command.Action == "open")) return operation.Failure = "protocol_violation";
                        operation.Files = frame.GetProperty("files_verified").GetUInt64();
                        operation.Bytes = frame.GetProperty("bytes_verified").GetUInt64();
                        operation.Complete = true; return null;
                    case "failure":
                        operation.Failure = Map(frame.GetProperty("error_code").GetString());
                        if (frame.TryGetProperty("native_error_code", out var native) && native.ValueKind == JsonValueKind.Number) operation.Native = native.GetInt32();
                        return operation.Failure;
                    default: return null; // Coalesced progress is intentionally not a product event/log stream.
                }
            }
            catch (ArchiveReadBudgetException) { return operation.Failure = "metadata_budget"; }
            catch (Exception frameFailure) {
                try { log?.Write("warning", "archive.read.frame_rejected", id, new Dictionary<string, object?> { ["stage"] = frame.GetProperty("op").GetString(), ["exception_type"] = frameFailure.GetType().Name }); } catch { }
                return operation.Failure = "protocol_violation";
            }
        }
        try {
            var acknowledged = await wait!.WaitAsync(TimeSpan.FromMilliseconds(operation.Limits.AckTimeoutMs), token);
            lock (gate) { operation.Credit = null; return acknowledged ? null : operation.Failure ?? "cancelled"; }
        }
        catch (TimeoutException) { lock (gate) return operation.Failure ??= "consumer_too_slow"; }
        catch (OperationCanceledException) { lock (gate) return operation.Failure ??= stopped ? "disconnected" : "cancelled"; }
    }
    internal void Stop()
    {
        lock (gate) {
            if (!stopped) {
                stopped = true;
                foreach (var operation in active.Values) { operation.Failure ??= "disconnected"; operation.Credit?.TrySetResult(false); }
                runner.Stop(); events.Writer.TryComplete();
            }
        }
    }
    public async ValueTask DisposeAsync() { Stop(); await runner.DrainAsync(); runner.Dispose(); }
    private void Diagnostic(string phase, ArchiveReadCommand command, ArchiveReadTerminal? terminal = null)
    {
        try { log?.Write("info", "archive.read." + phase, command.OperationId, new Dictionary<string, object?> {
            ["generation"] = command.Generation, ["kind"] = command.Action, ["category"] = terminal?.Code,
            ["elapsed_ms"] = terminal?.ElapsedMs ?? 0, ["native_error_code"] = terminal?.NativeErrorCode,
            ["entry_count"] = terminal?.EntryCount, ["batch_count"] = terminal?.BatchCount, ["cleanup_complete"] = terminal?.CleanupComplete }); }
        catch { /* Observational sinks cannot alter process/credit/cleanup outcomes. */ }
    }
    internal static string Map(string? code) => code switch {
        "UnsupportedOperation" => "unsupported_operation", "UnsupportedFormat" => "unsupported_format", "UnsupportedMethod" => "unsupported_method",
        "UnsupportedEncryption" => "unsupported_encryption", "PasswordRequired" => "password_required", "WrongPassword" => "wrong_password",
        "AuthenticationOrDataError" => "authentication_or_data_error", "CorruptArchive" => "corrupt_archive", "MissingVolume" => "missing_volume",
        "UnsafePath" => "unsafe_path", "OutputCollision" => "output_collision", "ResourceLimit" => "resource_limit",
        "SelectedOutputLimit" => "selected_output_limit", "SourceUnavailable" => "source_unavailable",
        "SourceChanged" => "source_changed", "Cancelled" => "cancelled", "IoFailure" => "io_failure",
        "helper_missing" or "helper_untrusted" or "incompatible_version" or "missing_capability" or "resource_limit" or "selected_output_limit" or "cancelled" or "disconnected" or
        "protocol_violation" or "consumer_too_slow" or "consumer_queue_full" or "metadata_budget" or "source_unavailable" or "source_changed" or "missing_volume" or
        "corrupt_archive" or "unsupported_format" or "unsupported_method" or "unsupported_encryption" => code,
        "concurrency_limit" or "aggregate_memory_limit" => "busy",
        "stdout_missing_terminal" or "process_exit" => "worker_crashed",
        "stdout_invalid_frame" or "stdout_frame_limit" or "worker_capability_mismatch" => "protocol_violation",
        "stdout_total_limit" => "metadata_budget", "invalid_request" or "invalid_budget" => "invalid_request",
        "cleanup_stuck" or "cleanup_failure" or "dispose_failure" => "cleanup_incomplete", _ => "io_failure"
    };
}
