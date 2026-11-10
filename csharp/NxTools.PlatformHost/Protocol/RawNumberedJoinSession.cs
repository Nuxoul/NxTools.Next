using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

// One operation and frozen preview per original pipe. No work or resource ownership in the reader.
internal sealed class RawNumberedJoinSession(IPlatformLog? log = null) : IAsyncDisposable
{
    private readonly object gate = new();
    private Task<WindowsCommandResult>? active;
    private CancellationTokenSource? cancellation;
    private RawNumberedJoinCommand? admitted;
    private (RawNumberedJoinCommand Command, RawJoinPreview Preview)? preview;
    private bool stopped;
    internal RawJoinObservation? TransferObservation { get; set; }
    private static bool Same(RawNumberedJoinCommand a, RawNumberedJoinCommand b) =>
        a.OperationId == b.OperationId && a.SessionId == b.SessionId && a.Generation == b.Generation;
    internal Task<WindowsCommandResult> Dispatch(ProtocolEnvelope envelope)
    {
        RawNumberedJoinCommand command;
        try { command = RawNumberedJoinCommand.Parse(envelope); }
        catch { return Task.FromResult(new WindowsCommandResult(false, "invalid_request")); }
        lock (gate)
        {
            if (stopped) return Task.FromResult(new WindowsCommandResult(false, "session_closed"));
            if (command.Action == "cancel")
            {
                if (admitted is null || !Same(admitted, command)) return Task.FromResult(new WindowsCommandResult(false, "stale_operation"));
                cancellation?.Cancel();
                Diagnostic(command, "cancelling", 0);
                return Task.FromResult(new WindowsCommandResult(true)); // Never a terminal/resource-release receipt.
            }
            if (active is not null) return Task.FromResult(new WindowsCommandResult(false, "busy"));
            RawJoinPreview? frozen = null;
            if (command.Action == "start")
            {
                if (preview is not {} saved || !Same(saved.Command, command) || saved.Preview.PreviewId != command.PreviewId)
                    return Task.FromResult(new WindowsCommandResult(false, "stale_preview"));
                frozen = saved.Preview;
            }
            preview = null;
            admitted = command;
            cancellation = new CancellationTokenSource();
            var ownedCancellation = cancellation;
            var completion = new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
            active = completion.Task; // Installed before later cancel can be read.
            Diagnostic(command, "accepted", 0);
            _ = Task.Run(() => Execute(command, frozen, ownedCancellation, completion));
            return completion.Task;
        }
    }
    private void Execute(RawNumberedJoinCommand command, RawJoinPreview? frozen, CancellationTokenSource ownedCancellation,
        TaskCompletionSource<WindowsCommandResult> completion)
    {
        var clock = Stopwatch.StartNew();
        RawJoinPreview? inspected = null;
        RawJoinResult terminal = new("failed", "worker_unavailable", 0, true);
        Diagnostic(command, command.Action == "inspect" ? "inspecting" : "started", clock.ElapsedMilliseconds);
        // The outer wait is never logically cancelled/timed out: actual worker finally owns the slot.
        // Only the operation token is cancelled, and its finite transfer checks that token.
        var ran = BoundedStaOperationRunner.TryRun("NxTools.RawNumberedJoin", Timeout.InfiniteTimeSpan,
            _ => {
                try
                {
                    if (command.Action == "inspect")
                    {
                        inspected = RawNumberedJoin.Inspect(command.SelectedPath!, ownedCancellation.Token);
                        ownedCancellation.Token.ThrowIfCancellationRequested();
                        return new RawJoinResult("preview_ready", null, 0, true);
                    }
                    return RawNumberedJoin.Join(frozen!, command.OutputPath!, command.OperationId, ownedCancellation.Token, TransferObservation);
                }
                catch (OperationCanceledException) { return new RawJoinResult("cancelled", "cancelled", 0, true); }
                catch (ArchiveOutputException failure) { return new RawJoinResult("failed", failure.Stage, 0, true, failure.NativeCode); }
                catch (Exception failure) when (failure is IOException or UnauthorizedAccessException or ArgumentException)
                { return new RawJoinResult("failed", "file_io", 0, true, failure.HResult & 0xffff); }
            }, out RawJoinResult? result, out _);
        if (ran && result is not null) terminal = result;
        RawJoinPreviewData? data = null;
        lock (gate)
        {
            if (!stopped && terminal.Outcome == "preview_ready" && inspected is not null)
            {
                preview = (command, inspected);
                data = new(inspected.PreviewId, inspected.SuggestedOutput,
                    inspected.Parts.Select(part => new RawJoinPreviewPart(part.Name, part.Length)).ToArray(), inspected.TotalBytes);
            }
            active = null; admitted = null; cancellation = null;
        }
        ownedCancellation.Dispose();
        Diagnostic(command, terminal.Outcome == "preview_ready" ? "preview_ready" : "terminal", clock.ElapsedMilliseconds, terminal);
        var reply = new RawNumberedJoinReply(1, command.OperationId, command.SessionId, command.Generation, data,
            terminal.Outcome == "preview_ready" ? null : terminal);
        completion.SetResult(new WindowsCommandResult(terminal.Outcome is "preview_ready" or "succeeded", terminal.Code,
            NativeErrorCode: terminal.NativeErrorCode, Data: JsonSerializer.Serialize(reply, JsonDefaults.Options)));
    }
    private void Diagnostic(RawNumberedJoinCommand command, string phase, long elapsed, RawJoinResult? terminal = null)
    {
        try { log?.Write("info", "tools.raw_numbered_join." + phase, command.OperationId,
            new Dictionary<string, object?> { ["session_id"] = command.SessionId, ["generation"] = command.Generation,
                ["elapsed_ms"] = elapsed, ["outcome"] = terminal?.Outcome, ["code"] = terminal?.Code,
                ["bytes_written"] = terminal?.BytesWritten, ["cleanup_complete"] = terminal?.CleanupComplete,
                ["native_error_code"] = terminal?.NativeErrorCode }); }
        catch { /* Bounded diagnostics never change file operation results. */ }
    }
    internal void Stop()
    {
        lock (gate) { stopped = true; preview = null; cancellation?.Cancel(); }
    }
    public async ValueTask DisposeAsync()
    {
        Task<WindowsCommandResult>? finishing;
        lock (gate) { stopped = true; preview = null; cancellation?.Cancel(); finishing = active; }
        if (finishing is not null) await finishing;
    }
}
