namespace NxTools.PlatformHost.Windows;

// Event-STA owned single slot; existing preference retry timer delivers only the latest state.
internal sealed class PendingThemePublication
{
    private SystemThemePreferenceChangedEvent? _pending;
    private bool _stopped;
    internal bool HasPending => _pending is not null;
    internal Guid? PendingCorrelationId => _pending?.CorrelationId;
    internal void Stage(ThemePreferenceSnapshot snapshot)
    {
        if (!_stopped) _pending = new(Guid.NewGuid(), snapshot.Dark, snapshot.NativeErrorCode);
    }
    internal SystemThemePreferenceChangedEvent? TryPublish(Func<SystemThemePreferenceChangedEvent, bool> publish)
    {
        var pending = _pending;
        if (_stopped || pending is null || !publish(pending)) return null;
        _pending = null;
        return pending;
    }
    internal void Stop() { _stopped = true; _pending = null; }
}
