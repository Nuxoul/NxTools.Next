namespace NxTools.PlatformHost.Windows;

// Event-STA owned single slot: a full output queue never retires the latest state.
internal sealed class PendingMotionPublication
{
    private SystemMotionPreferenceChangedEvent? _pending;
    private bool _stopped;
    internal bool HasPending => _pending is not null;
    internal Guid? PendingCorrelationId => _pending?.CorrelationId;
    internal void Stage(MotionPreferenceSnapshot snapshot)
    {
        if (!_stopped) _pending = new(Guid.NewGuid(), snapshot.ReducedMotion, snapshot.NativeErrorCode);
    }
    internal SystemMotionPreferenceChangedEvent? TryPublish(Func<SystemMotionPreferenceChangedEvent, bool> publish)
    {
        var pending = _pending;
        if (_stopped || pending is null || !publish(pending)) return null;
        _pending = null;
        return pending;
    }
    internal void Stop() { _stopped = true; _pending = null; }
}
