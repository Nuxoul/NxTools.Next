using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record AutoQuickSwitchDirectorySource(
    DirectoryNavigationContext Context,
    string DisplayName);

internal sealed record AutoQuickSwitchCandidate(
    long Generation,
    DirectoryNavigationContext Source,
    DirectoryNavigationContext Target)
{
    public long SourceWindowHandle => Source.WindowHandle;
}

internal sealed class AutoQuickSwitchTracker
{
    private const int KnownDialogLimit = 128;

    private readonly object _gate = new();
    private readonly HashSet<long> _knownDialogs = [];
    private readonly Queue<long> _knownDialogOrder = [];
    private bool _autoSwitchEnabled;
    private bool _trackSources;
    private bool _sourceDirty;
    private AutoQuickSwitchDirectorySource? _latestSource;
    private long _generation;

    public AutoQuickSwitchDirectorySource? LatestSource
    {
        get
        {
            lock (_gate)
            {
                return _latestSource;
            }
        }
    }

    public void Configure(bool enabled)
    {
        Configure(enabled, enabled);
    }

    public void Configure(bool autoSwitchEnabled, bool trackSources)
    {
        lock (_gate)
        {
            _autoSwitchEnabled = autoSwitchEnabled;
            _trackSources = trackSources;
            _sourceDirty = false;
            if (!trackSources)
            {
                _latestSource = null;
            }
            AdvanceGeneration();
        }
    }

    public void ObserveExplorer(long windowHandle)
    {
        if (windowHandle == 0)
        {
            return;
        }
        ObserveDirectorySource(
            new DirectoryNavigationContext(
                DirectoryNavigationAdapter.ExplorerHost,
                windowHandle,
                null),
            "File Explorer");
    }

    public void ObserveDirectorySource(
        DirectoryNavigationContext context,
        string displayName)
    {
        if (context.WindowHandle == 0 ||
            context.Host.Equals(
                DirectoryNavigationAdapter.FileDialogHost,
                StringComparison.Ordinal))
        {
            return;
        }
        lock (_gate)
        {
            if (!_trackSources)
            {
                return;
            }
            _latestSource = new AutoQuickSwitchDirectorySource(
                context,
                displayName);
            _sourceDirty = _autoSwitchEnabled;
            AdvanceGeneration();
        }
    }

    public bool ResolveDirectorySource(
        long windowHandle,
        string directory,
        string? displayName = null)
    {
        if (windowHandle == 0 ||
            string.IsNullOrWhiteSpace(directory))
        {
            return false;
        }
        lock (_gate)
        {
            if (!_trackSources ||
                _latestSource is not { } latest ||
                latest.Context.WindowHandle != windowHandle)
            {
                return false;
            }
            _latestSource = latest with
            {
                Context = latest.Context with
                {
                    CurrentDirectory = directory
                },
                DisplayName = string.IsNullOrWhiteSpace(displayName)
                    ? latest.DisplayName
                    : displayName
            };
            return true;
        }
    }

    public void ObserveOther()
    {
        lock (_gate)
        {
            if (!_autoSwitchEnabled || !_sourceDirty)
            {
                return;
            }
            _sourceDirty = false;
            AdvanceGeneration();
        }
    }

    public AutoQuickSwitchCandidate? ObserveFileDialog(
        DirectoryNavigationContext target)
    {
        if (target.WindowHandle == 0 ||
            !target.Host.Equals(
                DirectoryNavigationAdapter.FileDialogHost,
                StringComparison.Ordinal))
        {
            return null;
        }
        lock (_gate)
        {
            var firstObservation = RememberDialog(target.WindowHandle);
            if (!_autoSwitchEnabled)
            {
                return null;
            }
            if (firstObservation)
            {
                _sourceDirty = false;
                AdvanceGeneration();
                return null;
            }
            if (!_sourceDirty || _latestSource is not { } source)
            {
                return null;
            }

            _sourceDirty = false;
            AdvanceGeneration();
            return new AutoQuickSwitchCandidate(
                _generation,
                source.Context,
                target);
        }
    }

    public bool IsCurrent(AutoQuickSwitchCandidate candidate)
    {
        lock (_gate)
        {
            return _autoSwitchEnabled && candidate.Generation == _generation;
        }
    }

    private bool RememberDialog(long windowHandle)
    {
        if (!_knownDialogs.Add(windowHandle))
        {
            return false;
        }
        _knownDialogOrder.Enqueue(windowHandle);
        while (_knownDialogOrder.Count > KnownDialogLimit)
        {
            _knownDialogs.Remove(_knownDialogOrder.Dequeue());
        }
        return true;
    }

    private void AdvanceGeneration()
    {
        _generation = _generation == long.MaxValue
            ? 1
            : _generation + 1;
    }
}
