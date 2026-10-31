using System.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct CenteredResizeReplayResult(
    bool HadPendingResize,
    Guid CorrelationId,
    long WindowHandle,
    long StartedAtTimestamp,
    int AttemptCount,
    bool Success,
    bool StillPending,
    int? NativeErrorCode)
{
    internal long Generation { get; init; }
    internal int RequestedWidth { get; init; }
    internal int RequestedHeight { get; init; }
    internal int ReadbackWidth { get; init; }
    internal int ReadbackHeight { get; init; }
    internal bool MatchesCurrentClient(NativeWindowPresentationState current) =>
        Success && current.Exists && RequestedWidth > 0 && RequestedHeight > 0 &&
        current.ClientBounds.Width == RequestedWidth && current.ClientBounds.Height == RequestedHeight;
}

internal readonly record struct CenteredResizeRecoverySnapshot(
    IntPtr RootWindow,
    Guid CorrelationId,
    long WindowHandle,
    long StartedAtTimestamp,
    int AttemptCount)
{
    internal int Width { get; init; }
    internal int Height { get; init; }
    internal long Generation { get; init; }
}

internal enum CenteredResizeRecoveryDecision
{
    NoPendingResize,
    Retry,
    WaitingForMoveEnd,
    Completed,
    Failed,
    Exhausted,
    TimedOut
}

internal sealed class CenteredResizeRecovery
{
    internal const int MaximumAttempts = 8;
    internal const long MaximumWaitMilliseconds = 30_000;
    private readonly Dictionary<IntPtr, CenteredResizeRecoverySnapshot> _pending = [];

    internal IReadOnlyList<IntPtr> PendingRoots => _pending.Keys.ToArray();
    internal bool HasPending => _pending.Count > 0;

    internal void TrackPending(CenteredResizeRecoverySnapshot snapshot)
    {
        if (snapshot.RootWindow != IntPtr.Zero && snapshot.WindowHandle != 0)
        {
            _pending[snapshot.RootWindow] = snapshot;
        }
    }
    internal CenteredResizeRecoveryDecision WaitForMoveEnd(
        IntPtr rootWindow,
        long nowTimestamp,
        bool isDraggingOrSizing)
    {
        if (!_pending.TryGetValue(rootWindow, out var pending))
        {
            return CenteredResizeRecoveryDecision.NoPendingResize;
        }
        if (Stopwatch.GetElapsedTime(pending.StartedAtTimestamp, nowTimestamp).TotalMilliseconds >=
            MaximumWaitMilliseconds)
        {
            _pending.Remove(rootWindow);
            return CenteredResizeRecoveryDecision.TimedOut;
        }
        return isDraggingOrSizing
            ? CenteredResizeRecoveryDecision.WaitingForMoveEnd
            : CenteredResizeRecoveryDecision.Retry;
    }

    internal CenteredResizeRecoveryDecision Observe(
        IntPtr rootWindow,
        CenteredResizeReplayResult replay) =>
        OnMoveSizeEnd(rootWindow, replay);

    internal CenteredResizeRecoveryDecision OnMoveSizeEnd(
        IntPtr rootWindow,
        CenteredResizeReplayResult replay)
    {
        if (_pending.TryGetValue(rootWindow, out var tracked) && tracked.Generation != 0 &&
            replay.Generation < tracked.Generation)
            return CenteredResizeRecoveryDecision.NoPendingResize;
        if (rootWindow == IntPtr.Zero || !replay.HadPendingResize)
        {
            _pending.Remove(rootWindow);
            return CenteredResizeRecoveryDecision.NoPendingResize;
        }
        if (!replay.StillPending)
        {
            _pending.Remove(rootWindow);
            return replay.Success
                ? CenteredResizeRecoveryDecision.Completed
                : CenteredResizeRecoveryDecision.Failed;
        }
        if (replay.AttemptCount >= MaximumAttempts)
        {
            _pending.Remove(rootWindow);
            return CenteredResizeRecoveryDecision.Exhausted;
        }
        _pending[rootWindow] = (_pending.TryGetValue(rootWindow, out var previous) &&
            previous.CorrelationId == replay.CorrelationId && previous.Generation == replay.Generation
            ? previous
            : new CenteredResizeRecoverySnapshot(rootWindow, replay.CorrelationId, replay.WindowHandle,
                replay.StartedAtTimestamp, replay.AttemptCount)) with
            { AttemptCount = replay.AttemptCount, Generation = replay.Generation };
        return CenteredResizeRecoveryDecision.Retry;
    }

    internal bool TryGetSnapshot(
        IntPtr rootWindow,
        out CenteredResizeRecoverySnapshot snapshot) =>
        _pending.TryGetValue(rootWindow, out snapshot);

    internal void Cancel(IntPtr rootWindow) => _pending.Remove(rootWindow);

    internal int AttemptsFor(IntPtr rootWindow) =>
        _pending.TryGetValue(rootWindow, out var pending) ? pending.AttemptCount : 0;
}
