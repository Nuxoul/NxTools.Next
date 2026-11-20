using System.Text.Json;

namespace NxTools.PlatformHost.Windows;

/// <summary>Runtime-only gate, mutated exclusively by the existing clipboard event owner.</summary>
internal sealed class ClipboardCapturePause
{
    private uint? _resumeSequence;
    private long _publicationEpoch;
    private int _enabled = 1;
    public bool IsEnabled => Volatile.Read(ref _enabled) != 0;
    internal long PublicationEpoch => Volatile.Read(ref _publicationEpoch);
    public bool IsPaused { get; private set; }
    public bool MayRead => IsEnabled && !IsPaused;

    public void SetEnabled(bool enabled, uint? resumeSequence)
    {
        if (enabled && !IsEnabled) _resumeSequence = resumeSequence;
        Volatile.Write(ref _enabled, enabled ? 1 : 0);
        Interlocked.Increment(ref _publicationEpoch);
    }

    // Close before clearing retries/sampling a resume baseline: an application failure stays closed.
    public void Close()
    {
        IsPaused = true;
        Interlocked.Increment(ref _publicationEpoch);
    }

    public WindowsCommandResult Apply(bool paused, uint? resumeSequence)
    {
        // Idempotent resume must retain the discard baseline until a new pause/change.
        if (paused || resumeSequence.HasValue) _resumeSequence = resumeSequence;
        IsPaused = paused;
        return new(true, Data: JsonSerializer.Serialize(new { capture_paused = paused }));
    }

    // WM_CLIPBOARDUPDATE/WM_TIMER may already be queued when resume is applied.
    // The current sequence is metadata only; never read content from that paused interval.
    public bool MayReadSequence(uint sequence) => MayRead && sequence != _resumeSequence;
}

// Production operations and safe fixtures enter the same EventWindow dispatch/read/retry path.
// PostPause returns zero on success, otherwise the native error captured immediately at PostMessage.
internal sealed record ClipboardCaptureOperations(
    Func<int> PostPause,
    Func<uint> Sequence,
    Func<ForegroundContext> Source,
    Action<ForegroundContext, uint> Read,
    Action StopRetry,
    Action StartRetry);
