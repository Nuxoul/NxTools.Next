namespace NxTools.PlatformHost.Windows;

// Message-thread owned. Both hooks observe the same native key timestamp.
internal sealed class QuickLaunchInputRouter
{
    private uint? _lastAccepted;

    public bool Accept(bool helperSource, bool elevatedForeground, bool helperOwns, uint timestamp)
    {
        if (_lastAccepted == timestamp ||
            !WindowsPlatformAdapter.AcceptInputSource(helperSource, elevatedForeground, helperOwns)) return false;
        _lastAccepted = timestamp;
        return true;
    }
}
