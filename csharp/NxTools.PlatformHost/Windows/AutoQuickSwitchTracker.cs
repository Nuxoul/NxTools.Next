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

    public void DiscardPendingSource()
    {
        lock (_gate)
        {
            if (!_sourceDirty)
            {
                return;
            }
            _sourceDirty = false;
            AdvanceGeneration();
        }
    }

    /// <summary>
    /// The latest source window was activated again but its current folder is not yet readable
    /// (for example a Directory Opus lister whose focus is outside a file pane). The tracked
    /// source controller stays authoritative; the dialog return re-reads its folder.
    /// </summary>
    public bool MarkSourcePending(long windowHandle)
    {
        if (windowHandle == 0)
        {
            return false;
        }
        lock (_gate)
        {
            if (!_trackSources ||
                _latestSource?.Context.WindowHandle != windowHandle)
            {
                return false;
            }
            _sourceDirty = _autoSwitchEnabled;
            AdvanceGeneration();
            return true;
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

    public bool HasPendingDirectorySourceForWindow(long windowHandle)
    {
        lock (_gate)
        {
            return _sourceDirty &&
                windowHandle != 0 &&
                _latestSource?.Context.WindowHandle == windowHandle;
        }
    }
    public bool IsKnownFileDialog(long windowHandle)
    {
        if (windowHandle == 0)
        {
            return false;
        }
        lock (_gate)
        {
            return _knownDialogs.Contains(windowHandle);
        }
    }

    public bool RefreshDirectorySource(
        DirectoryNavigationContext context,
        string displayName)
    {
        if (context.WindowHandle == 0 ||
            !context.Host.Equals(
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                StringComparison.Ordinal))
        {
            return false;
        }
        lock (_gate)
        {
            if (!_trackSources ||
                _latestSource is not { } previous ||
                previous.Context.WindowHandle != context.WindowHandle ||
                !previous.Context.Host.Equals(context.Host, StringComparison.Ordinal))
            {
                return false;
            }
            var changed = !ThirdPartyFileManagerPath.SameDirectory(
                previous.Context.CurrentDirectory,
                context.CurrentDirectory);
            _latestSource = new AutoQuickSwitchDirectorySource(context, displayName);
            if (changed)
            {
                _sourceDirty = _autoSwitchEnabled;
                AdvanceGeneration();
            }
            return changed;
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
