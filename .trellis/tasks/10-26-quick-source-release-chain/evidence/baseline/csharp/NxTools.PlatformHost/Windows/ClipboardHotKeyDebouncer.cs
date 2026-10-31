namespace NxTools.PlatformHost.Windows;

internal sealed class ClipboardHotKeyDebouncer
{
    internal const long DebounceMilliseconds = 350;

    private long _lastAcceptedAt = long.MinValue;

    public bool TryAccept(long nowMilliseconds)
    {
        var previous = _lastAcceptedAt;
        if (previous != long.MinValue &&
            nowMilliseconds >= previous &&
            nowMilliseconds - previous < DebounceMilliseconds)
        {
            return false;
        }

        _lastAcceptedAt = nowMilliseconds;
        return true;
    }
}
