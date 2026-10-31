namespace NxTools.PlatformHost.Windows;

internal static class DirectoryOpusSourceRefresh
{
    internal const uint EventObjectNameChange = 0x800C;
    internal const uint EventObjectFocus = 0x8005;
    internal const uint EventObjectSelection = 0x8006;
    internal const int ObjectIdWindow = 0;
    internal const int ObjectIdClient = -4;
    internal const uint EventObjectLocationChange = 0x800B;

    internal static bool ShouldRefresh(
        uint eventType,
        int objectId,
        long rootWindowHandle,
        long trackedListerWindowHandle)
    {
        if (rootWindowHandle == 0 || rootWindowHandle != trackedListerWindowHandle)
        {
            return false;
        }
        if (eventType is EventObjectNameChange or EventObjectSelection)
        {
            return objectId is ObjectIdWindow or ObjectIdClient;
        }

        return eventType == EventObjectFocus &&
            objectId is ObjectIdWindow or ObjectIdClient;
    }

    internal static long PendingFocusRefreshWindow(
        long latestSourceWindowHandle,
        bool latestSourceIsDirectoryOpus,
        long eventRootWindowHandle,
        uint eventType,
        int objectId) =>
        latestSourceIsDirectoryOpus &&
        ShouldRefresh(
            eventType,
            objectId,
            eventRootWindowHandle,
            latestSourceWindowHandle)
            ? latestSourceWindowHandle
            : 0;

    internal static bool ShouldRefreshPending(
        long pendingWindowHandle,
        long foregroundWindowHandle,
        bool latestSourceIsDirectoryOpus,
        long latestSourceWindowHandle) =>
        pendingWindowHandle != 0 &&
        pendingWindowHandle == foregroundWindowHandle &&
        latestSourceIsDirectoryOpus &&
        pendingWindowHandle == latestSourceWindowHandle;
}
