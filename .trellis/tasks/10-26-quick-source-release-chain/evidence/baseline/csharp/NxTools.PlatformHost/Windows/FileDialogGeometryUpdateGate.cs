namespace NxTools.PlatformHost.Windows;

internal sealed class FileDialogGeometryUpdateGate
{
    private const uint EventObjectLocationChange = 0x800B;
    private const int ObjIdWindow = 0;
    private readonly object _gate = new();
    private long _pendingRootWindowHandle;

    public bool TrySchedule(
        long activeRootWindowHandle,
        long eventRootWindowHandle,
        long eventWindowHandle,
        int objectId,
        uint eventType)
    {
        if (activeRootWindowHandle == 0 ||
            eventType != EventObjectLocationChange ||
            objectId != ObjIdWindow ||
            eventRootWindowHandle != activeRootWindowHandle ||
            eventWindowHandle != activeRootWindowHandle)
        {
            return false;
        }
        lock (_gate)
        {
            if (_pendingRootWindowHandle != 0)
            {
                return false;
            }
            _pendingRootWindowHandle = activeRootWindowHandle;
            return true;
        }
    }

    public bool TryTake(out long rootWindowHandle)
    {
        lock (_gate)
        {
            rootWindowHandle = _pendingRootWindowHandle;
            _pendingRootWindowHandle = 0;
            return rootWindowHandle != 0;
        }
    }

    public void Cancel(long rootWindowHandle)
    {
        lock (_gate)
        {
            if (_pendingRootWindowHandle == rootWindowHandle)
            {
                _pendingRootWindowHandle = 0;
            }
        }
    }
}
