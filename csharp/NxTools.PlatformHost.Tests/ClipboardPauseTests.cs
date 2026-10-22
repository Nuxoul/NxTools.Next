using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// The actual EventWindow WndProc/application/read boundary, with every Windows effect injected.
// No HWND, listener, timer, clipboard, filesystem, registry, service or global input is used.
internal static class ClipboardPauseTests
{
    private const int PauseMessage = 0x8010;
    private const int UpdateMessage = 0x031D;
    private const int TimerMessage = 0x0113;
    private const int RetryTimer = 1;

    private sealed class Log : IPlatformLog
    {
        internal readonly List<(string Name, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Entries = [];
        internal bool Throw;
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (Throw) throw new IOException("synthetic private diagnostic payload");
            Entries.Add((eventName, correlationId, fields ?? new Dictionary<string, object?>()));
        }
    }

    private sealed class Fixture : IDisposable
    {
        internal readonly Log Log = new();
        internal readonly WindowsPlatformAdapter.EventWindow Window;
        internal uint Sequence = 1;
        internal int SequenceReads, SourceReads, ContentReads, Posts, TimerStarts, TimerStops;
        internal int PostError;
        internal bool Busy, FailTimer, FailSequence, TimerActive;
        internal Action? DuringTimerStop;
        internal readonly List<uint> CapturedSequences = [];
        internal Fixture()
        {
            Window = new(Channel.CreateBounded<WindowsPlatformEvent>(8).Writer, Log,
                new ClipboardCaptureOperations(
                    () => { Posts++; return PostError; },
                    () =>
                    {
                        SequenceReads++;
                        if (FailSequence) throw new Win32Exception(5, "synthetic private sequence payload");
                        return Sequence;
                    },
                    () => { SourceReads++; return new(10, 11, 12, "synthetic", "synthetic title"); },
                    (_, sequence) =>
                    {
                        ContentReads++;
                        if (Busy) throw new ExternalException("synthetic private clipboard payload");
                        CapturedSequences.Add(sequence);
                    },
                    () =>
                    {
                        TimerStops++;
                        DuringTimerStop?.Invoke();
                        if (FailTimer) throw new Win32Exception(87, "synthetic private timer payload");
                        TimerActive = false;
                    },
                    () => { TimerStarts++; TimerActive = true; }));
        }
        internal Task<WindowsCommandResult> Enqueue(bool paused, Guid? correlation = null, CancellationToken cancellation = default) =>
            Window.EnqueueClipboardPause(new(paused), correlation ?? Guid.NewGuid(), cancellation);
        internal void Apply() => Window.DispatchClipboardMessage(PauseMessage);
        internal void Update() => Window.DispatchClipboardMessage(UpdateMessage);
        internal void Retry() => Window.DispatchClipboardMessage(TimerMessage, RetryTimer);
        public void Dispose()
        {
            FailTimer = false;
            DuringTimerStop = null;
            Window.StopClipboardCapture();
        }
    }

    public static async Task PauseCommandAndReadGate()
    {
        foreach (var paused in new[] { true, false })
        {
            var envelope = ProtocolEnvelope.Create("platform_command",
                new { command = "set_clipboard_capture_paused", data = new { paused } });
            Assert(PlatformCommandParser.ParseSetClipboardCapturePaused(envelope).Paused == paused, "typed bool roundtrip");
        }
        foreach (var data in new[] { "{}", "null", "{\"paused\":null}", "{\"paused\":1}", "{\"paused\":\"true\"}", "{\"paused\":true,\"enabled\":false}" })
        {
            using var json = JsonDocument.Parse(data);
            var envelope = ProtocolEnvelope.Create("platform_command",
                new { command = "set_clipboard_capture_paused", data = json.RootElement });
            try
            {
                PlatformCommandParser.ParseSetClipboardCapturePaused(envelope);
                throw new InvalidOperationException("malformed pause payload admitted");
            }
            catch (Exception error) when (error is JsonException or InvalidDataException) { }
        }

        using var f = new Fixture();
        f.Update();
        Assert(f.ContentReads == 1, "startup uses existing capture owner");
        var pause = f.Enqueue(true);
        Assert(!pause.IsCompleted && !f.Window.ClipboardCaptureState.Paused, "admission is not applied ACK");
        f.Apply();
        var result = await pause;
        AssertApplied(result, true);
        var request = ProtocolEnvelope.Create("platform_command",
            new { command = "set_clipboard_capture_paused", data = new { paused = true } });
        var reply = PlatformPipeClient.CreateCommandResultEnvelope(request, result);
        Assert(reply.CorrelationId == request.Id && reply.Payload.GetProperty("data").ValueKind == JsonValueKind.String,
            "unchanged command-result envelope carries correlated JSON string, not a JSON object");
        Assert(reply.Payload.GetProperty("data").GetString() == result.Data, "applied JSON string survives wire projection");
        Assert(f.Window.ClipboardCaptureState.Paused, "gate set before ACK");
        var counts = (f.SequenceReads, f.SourceReads, f.ContentReads);
        f.Sequence++;
        f.Update(); f.Retry();
        Assert(counts == (f.SequenceReads, f.SourceReads, f.ContentReads), "paused dispatch cannot even sample clipboard sequence/source");
        using var newOwner = new Fixture();
        newOwner.Update();
        Assert(newOwner.ContentReads == 1, "runtime pause is not persisted into a replacement owner");
        var dispatcher = new PlatformCommandSequenceDispatcher();
        Task<WindowsCommandResult>? queued = null;
        var admitted = dispatcher.DispatchAsync("set_clipboard_capture_paused", async () =>
        {
            queued = f.Enqueue(false);
            AssertApplied(await queued, false);
        });
        Assert(admitted.IsCompletedSuccessfully && queued is { IsCompleted: false },
            "synchronous bounded admission must not block the serial command reader on event-owner ACK");
        var laterRan = false;
        await dispatcher.DispatchAsync("set_window_visibility", () => { laterRan = true; return Task.CompletedTask; });
        Assert(laterRan, "later independent command stays responsive");
        f.Apply();
        await dispatcher.DrainAsync(TimeSpan.FromSeconds(5));
        Assert(queued!.IsCompletedSuccessfully, "detached confirmation drains after actual apply");
    }

    public static async Task RetryResumeNeverBackfills()
    {
        using var f = new Fixture { Busy = true };
        f.Update();
        Assert(f.TimerActive && f.TimerStarts == 1 && f.Window.ClipboardCaptureState.Attempts == 1 &&
            f.Window.ClipboardCaptureState.HasPendingSource, "real capture failure arms bounded retry");
        var pause = f.Enqueue(true);
        f.Apply();
        AssertApplied(await pause, true);
        Assert(!f.TimerActive && !f.Window.ClipboardCaptureState.HasPendingSource &&
            f.Window.ClipboardCaptureState.Attempts == 0, "pause clears pending source, attempts and timer before success");
        f.Sequence = 20;
        f.Busy = false;
        f.Update();
        var reads = f.ContentReads;
        var resume = f.Enqueue(false);
        Assert(!resume.IsCompleted, "resume also waits for event owner");
        f.Apply();
        AssertApplied(await resume, false);
        Assert(f.ContentReads == reads && !f.TimerActive, "resume does not read or schedule content");
        f.Retry(); f.Update(); f.Retry();
        Assert(f.ContentReads == reads && !f.Window.ClipboardCaptureState.HasPendingSource,
            "already queued retry/update cannot backfill the paused clipboard sequence");
        var repeatedResume = f.Enqueue(false);
        f.Apply(); AssertApplied(await repeatedResume, false);
        f.Update(); f.Retry();
        Assert(f.ContentReads == reads, "idempotent resume must retain paused-sequence exclusion");
        f.Sequence++;
        f.Update();
        Assert(f.ContentReads == reads + 1 && f.CapturedSequences.SequenceEqual(new uint[] { 21 }),
            "only a fresh post-resume change reaches content boundary");
        f.Retry();
        Assert(f.ContentReads == reads + 1, "retired retry cannot duplicate successful capture");
    }

    public static async Task BoundedAdmissionAndPostFailure()
    {
        using var f = new Fixture();
        var pending = Enumerable.Range(0, WindowsPlatformAdapter.EventWindow.MaxPendingClipboardPauseRequests)
            .Select(_ => f.Enqueue(true)).ToArray();
        var rejected = await f.Enqueue(false);
        Assert(!rejected.Success && rejected.Data is null && f.Posts == pending.Length, "full admission rejects without posting");
        f.Apply();
        foreach (var task in pending) AssertApplied(await task, true);
        Assert(f.Window.ClipboardCaptureState.PendingCommands == 0, "drain releases all bounded slots");
        f.PostError = 1400;
        for (var i = 0; i < pending.Length * 2; i++)
        {
            var failed = await f.Enqueue(false);
            Assert(!failed.Success && failed.NativeErrorCode == 1400 && failed.Data is null,
                "PostMessage failure preserves native code and never sends applied data");
            Assert(f.Window.ClipboardCaptureState.PendingCommands == 0, "failed post physically removes request");
        }
        f.Apply();
        Assert(f.Window.ClipboardCaptureState.Paused, "stale wake cannot apply a failed resume");
        f.PostError = 0;
        var resume = f.Enqueue(false);
        f.Apply();
        AssertApplied(await resume, false);
    }

    public static async Task CancellationAndStop()
    {
        using var f = new Fixture();
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var beforeAdmission = f.Enqueue(true, cancellation: cancellation.Token);
        await AssertCancelled(beforeAdmission);
        Assert(f.Posts == 0, "pre-cancel does not post");
        using var queuedCancellation = new CancellationTokenSource();
        var queued = f.Enqueue(true, cancellation: queuedCancellation.Token);
        queuedCancellation.Cancel();
        await AssertCancelled(queued);
        Assert(f.Window.ClipboardCaptureState.PendingCommands == 0, "queued cancel releases slot without a message pump");
        f.Apply();
        Assert(!f.Window.ClipboardCaptureState.Paused, "cancelled pause cannot later apply");
        var pause = f.Enqueue(true); f.Apply(); AssertApplied(await pause, true);
        using var resumeCancellation = new CancellationTokenSource();
        var resume = f.Enqueue(false, cancellation: resumeCancellation.Token);
        resumeCancellation.Cancel();
        await AssertCancelled(resume);
        f.Apply();
        Assert(f.Window.ClipboardCaptureState.Paused, "cancelled resume cannot reopen capture");
        var stopping = f.Enqueue(false);
        // Same cross-thread retirement used by adapter Dispose/RequestStop and message-loop finally.
        f.Window.CancelClipboardPauseRequests();
        Assert(!(await stopping).Success && f.Window.ClipboardCaptureState.PendingCommands == 0,
            "disposal retires queued commands immediately even if close cannot be posted");
        Assert(!(await f.Enqueue(false)).Success, "disposed owner rejects new work");
        f.Apply(); f.Update(); f.Retry();
        Assert(f.ContentReads == 0, "late messages cannot read or apply after retirement");
        f.Window.StopClipboardCapture();
        f.Window.StopClipboardCapture();
        Assert(!f.TimerActive && !f.Window.ClipboardCaptureState.HasPendingSource, "event-owner cleanup is idempotent");
    }

    public static async Task ApplicationFailureAndDiagnostics()
    {
        using var f = new Fixture { FailTimer = true };
        var correlation = Guid.NewGuid();
        var pause = f.Enqueue(true, correlation);
        f.Apply();
        var failed = await pause;
        Assert(!failed.Success && failed.NativeErrorCode == 87 && failed.Data is null &&
            f.Window.ClipboardCaptureState.Paused, "application failure is explicit and fail-closed, not applied ACK");
        Assert(f.Window.ClipboardCaptureState.PendingCommands == 0, "apply failure releases request");
        var lifecycle = f.Log.Entries.Where(entry => entry.Correlation == correlation).ToArray();
        Assert(lifecycle.Select(entry => entry.Name).SequenceEqual(new[]
        {
            "clipboard.capture_pause.accepted", "clipboard.capture_pause.start", "clipboard.capture_pause.failed"
        }), "original correlation spans accepted/start/terminal failure");
        Assert(lifecycle.All(entry => entry.Fields.ContainsKey("elapsed_ms")) &&
            Equals(lifecycle[^1].Fields["native_error_code"], 87), "elapsed/native diagnostics owned at failure boundary");
        Assert(!JsonSerializer.Serialize(f.Log.Entries.Select(entry => entry.Fields)).Contains("private", StringComparison.Ordinal),
            "diagnostics exclude exception payloads");
        f.FailTimer = false;
        f.FailSequence = true;
        var resume = f.Enqueue(false); f.Apply();
        var resumeFailure = await resume;
        Assert(!resumeFailure.Success && resumeFailure.NativeErrorCode == 5 && f.Window.ClipboardCaptureState.Paused,
            "failed resume baseline stays closed");
        f.FailSequence = false;
        f.Sequence = 0;
        var unavailable = f.Enqueue(false); f.Apply();
        Assert(!(await unavailable).Success && f.Window.ClipboardCaptureState.Paused,
            "unavailable resume sequence cannot authorize a possible paused-content backfill");
        f.Sequence = 1;
        f.Log.Throw = true;
        var retry = f.Enqueue(false); f.Apply();
        AssertApplied(await retry, false);
        f.Sequence++; f.Update();
        Assert(f.ContentReads == 1, "throwing diagnostic sink cannot change applied outcome");
    }

    public static async Task CancellationAtApplicationBoundary()
    {
        using var f = new Fixture();
        using var cancellation = new CancellationTokenSource();
        var pause = f.Enqueue(true, cancellation: cancellation.Token);
        // Once application owns the request, cancellation must not publish a cancelled result
        // for a gate which has already changed. Before ownership, CancellationAndStop covers cancel winning.
        f.DuringTimerStop = cancellation.Cancel;
        f.Apply();
        AssertApplied(await pause, true);
        Assert(f.Window.ClipboardCaptureState.Paused, "application wins late cancellation atomically");
    }

    private static void AssertApplied(WindowsCommandResult result, bool paused)
    {
        Assert(result.Success && result.Error is null, "successful application");
        using var document = JsonDocument.Parse(result.Data ?? throw new InvalidOperationException("missing JSON string data"));
        Assert(document.RootElement.EnumerateObject().Count() == 1 &&
            document.RootElement.GetProperty("capture_paused").GetBoolean() == paused, "exact applied ACK JSON string");
    }

    private static async Task AssertCancelled(Task<WindowsCommandResult> task)
    {
        try { await task.WaitAsync(TimeSpan.FromSeconds(5)); }
        catch (OperationCanceledException) { return; }
        throw new InvalidOperationException("expected cancelled pause request");
    }

    private static void Assert(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }
}
