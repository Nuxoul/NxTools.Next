using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class DirectoryOpusDialogReturnRefresh
{
    internal static AutoQuickSwitchCandidate? Observe(
        AutoQuickSwitchTracker tracker,
        DirectoryNavigationContext target,
        Func<AutoQuickSwitchDirectorySource, bool> isDirectoryOpusSource,
        Func<AutoQuickSwitchDirectorySource, bool> refreshDirectoryOpusSource)
    {
        if (target.WindowHandle == 0 ||
            !target.Host.Equals(
                DirectoryNavigationAdapter.FileDialogHost,
                StringComparison.Ordinal))
        {
            return null;
        }

        if (tracker.IsKnownFileDialog(target.WindowHandle) &&
            tracker.LatestSource is { } source &&
            isDirectoryOpusSource(source) &&
            !refreshDirectoryOpusSource(source))
        {
            tracker.DiscardPendingSource();
        }

        return tracker.ObserveFileDialog(target);
    }
}
