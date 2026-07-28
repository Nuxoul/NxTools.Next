namespace NxTools.PlatformHost.Windows;

internal sealed class FileDialogRecognitionRetry
{
    private readonly object _gate = new();
    private readonly int _maxAttempts;
    private long _windowHandle;
    private int _attempts;

    public FileDialogRecognitionRetry(int maxAttempts)
    {
        if (maxAttempts <= 0)
        {
            throw new ArgumentOutOfRangeException(nameof(maxAttempts));
        }
        _maxAttempts = maxAttempts;
    }

    public bool Begin(long windowHandle)
    {
        if (windowHandle == 0)
        {
            return false;
        }
        lock (_gate)
        {
            if (_windowHandle != windowHandle)
            {
                _windowHandle = windowHandle;
                _attempts = 0;
            }
            return _attempts < _maxAttempts;
        }
    }

    public bool TryBeginAttempt(long foregroundWindowHandle, out long windowHandle)
    {
        lock (_gate)
        {
            windowHandle = _windowHandle;
            if (_windowHandle == 0)
            {
                return false;
            }
            if (_attempts >= _maxAttempts)
            {
                CancelLocked();
                windowHandle = 0;
                return false;
            }
            _attempts++;
            return foregroundWindowHandle == _windowHandle;
        }
    }

    public bool HasPendingAttempt(long windowHandle)
    {
        lock (_gate)
        {
            return _windowHandle == windowHandle && _attempts < _maxAttempts;
        }
    }

    public bool ShouldRetryForStructureChange(long rootWindowHandle)
    {
        lock (_gate)
        {
            return rootWindowHandle != 0 &&
                _windowHandle == rootWindowHandle &&
                _attempts < _maxAttempts;
        }
    }

    public void Complete(long windowHandle)
    {
        lock (_gate)
        {
            if (_windowHandle == windowHandle)
            {
                CancelLocked();
            }
        }
    }

    public void Cancel()
    {
        lock (_gate)
        {
            CancelLocked();
        }
    }

    private void CancelLocked()
    {
        _windowHandle = 0;
        _attempts = 0;
    }
}
