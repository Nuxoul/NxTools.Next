namespace NxTools.PlatformHost.Windows;

// The existing EventWindow preference retry timer owns this latest-only slot.
internal sealed class PendingMaterialPublication
{
    private SystemMaterialCapabilitiesChangedEvent? _pending;
    private bool _stopped;
    internal bool HasPending => _pending is not null;
    internal Guid? PendingCorrelationId => _pending?.CorrelationId;
    internal void Stage(SystemMaterialCapabilitiesChangedEvent? value)
    {
        if (!_stopped && value is not null) _pending = value;
    }
    internal SystemMaterialCapabilitiesChangedEvent? TryPublish(
        Guid currentSession, Func<SystemMaterialCapabilitiesChangedEvent, bool> publish)
    {
        var pending = _pending;
        if (_stopped || pending is null) return null;
        if (pending.SessionId != currentSession) { _pending = null; return null; }
        if (!publish(pending)) return null;
        _pending = null;
        return pending;
    }
    internal void Stop() { _stopped = true; _pending = null; }
}
