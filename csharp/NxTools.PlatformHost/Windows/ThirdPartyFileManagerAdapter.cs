using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal interface IThirdPartyFileManagerOperations
{
    string Id { get; }

    bool RequiresPostNavigationActivation => true;

    bool TryCapture(
        ForegroundContext foreground,
        out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure);

    bool CanNavigate(long windowHandle, object paneState);

    WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState);

    bool TryActivate(
        long windowHandle,
        object paneState,
        out string? error);
}

internal sealed record ThirdPartyFileManagerCapture(
    string CurrentDirectory,
    object PaneState);

internal sealed record ThirdPartyFileManagerCaptureAttempt(
    ThirdPartyFileManagerContext? Context,
    WindowsCommandResult? Failure = null);

internal static class ThirdPartyFileManagerAdapter
{
    private static readonly OneCommanderFileManagerOperations OneCommander = new();
    private static readonly ThirdPartyFileManagerSessionRegistry Registry =
        new(
        [
            new DirectoryOpusFileManagerOperations(),
            new TotalCommanderFileManagerOperations(),
            OneCommander,
            new FilesFileManagerOperations()
        ]);

    public static IDisposable BeginLifetime(IPlatformLog log, string role)
    {
        OneCommander.BeginLifetime(log, role);
        return new DirectoryLifetime();
    }

    public static void ResumeLifetime(IPlatformLog log, string role) => OneCommander.BeginLifetime(log, role);
    public static void ObserveForeground(ForegroundContext context) => OneCommander.ObserveForeground(context);
    public static void RetireWindow(long window) => OneCommander.RetireWindow(window);
    public static void ReleaseContext(string? contextId) => Registry.ReleaseContext(contextId);
    public static void Stop()
    {
        Registry.Clear();
        OneCommander.Stop();
    }

    private sealed class DirectoryLifetime : IDisposable
    {
        private int stopped;
        public void Dispose()
        {
            if (Interlocked.Exchange(ref stopped, 1) == 0) Stop();
        }
    }

    public static bool IsPotentialHost(ForegroundContext foreground)
    {
        var processName = Path.GetFileNameWithoutExtension(
            foreground.ProcessName);
        return processName.Equals("dopus", StringComparison.OrdinalIgnoreCase) ||
            processName.Equals("totalcmd", StringComparison.OrdinalIgnoreCase) ||
            processName.Equals("totalcmd64", StringComparison.OrdinalIgnoreCase) ||
            processName.Equals("OneCommander", StringComparison.OrdinalIgnoreCase) ||
            FilesFileManagerOperations.IsFilesProcessName(processName);
    }

    public static ThirdPartyFileManagerCaptureAttempt TryCapture(
        ForegroundContext foreground) =>
        Registry.TryCapture(foreground);

    public static WindowsCommandResult Navigate(
        NavigateThirdPartyFileManagerCommand command) =>
        Registry.Navigate(command, Directory.Exists);
    public static ThirdPartyFileManagerContext? TryRefreshDirectoryOpusSource(
        long windowHandle,
        string? hostContextId,
        ForegroundContext foreground) =>
        Registry.TryRefreshDirectoryOpusSource(windowHandle, hostContextId, foreground);
    public static bool IsDirectoryOpusSource(
        long windowHandle,
        string? hostContextId) =>
        Registry.IsDirectoryOpusSource(windowHandle, hostContextId);
}

internal sealed class ThirdPartyFileManagerSessionRegistry
{
    private const int MaximumSessions = 64;
    private static readonly TimeSpan SessionLifetime =
        TimeSpan.FromMinutes(10);
    private readonly object _gate = new();
    private readonly IReadOnlyList<IThirdPartyFileManagerOperations> _adapters;
    private readonly Dictionary<string, CapturedSession> _sessions =
        new(StringComparer.Ordinal);
    private readonly Dictionary<string, ConsumedDirectoryOpusSource> _consumedDirectoryOpusSources =
        new(StringComparer.Ordinal);
    private readonly LinkedList<string> _sessionOrder = new();
    private readonly Dictionary<string, LinkedListNode<string>>
        _sessionOrderNodes = new(StringComparer.Ordinal);
    public ThirdPartyFileManagerSessionRegistry(
        IReadOnlyList<IThirdPartyFileManagerOperations> adapters)
    {
        _adapters = adapters;
    }

    public ThirdPartyFileManagerCaptureAttempt TryCapture(
        ForegroundContext foreground)
    {
        if (foreground.WindowHandle == 0)
        {
            return new ThirdPartyFileManagerCaptureAttempt(null);
        }
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
        }
        foreach (var adapter in _adapters)
        {
            if (!adapter.TryCapture(
                    foreground,
                    out var capture,
                    out var failure) ||
                capture is null)
            {
                if (failure?.Fatal == true)
                {
                    return new ThirdPartyFileManagerCaptureAttempt(
                        null,
                        failure);
                }
                continue;
            }
            var contextId = Guid.NewGuid().ToString("N");
            lock (_gate)
            {
                Cleanup(DateTimeOffset.UtcNow);
                _sessions[contextId] = new CapturedSession(
                    adapter,
                    foreground.WindowHandle,
                    foreground.ProcessId,
                    capture.PaneState,
                    DateTimeOffset.UtcNow);
                _sessionOrderNodes[contextId] =
                    _sessionOrder.AddLast(contextId);
                TrimToCapacity();
            }
            return new ThirdPartyFileManagerCaptureAttempt(
                new ThirdPartyFileManagerContext(
                    foreground.WindowHandle,
                    capture.CurrentDirectory,
                    contextId));
        }
        return new ThirdPartyFileManagerCaptureAttempt(null);
    }

    public ThirdPartyFileManagerContext? TryRefreshDirectoryOpusSource(
        long windowHandle,
        string? hostContextId,
        ForegroundContext foreground)
    {
        if (windowHandle == 0 ||
            foreground.WindowHandle != windowHandle ||
            foreground.ProcessId == 0 ||
            string.IsNullOrWhiteSpace(hostContextId))
        {
            return null;
        }

        CapturedSession? previous = null;
        ConsumedDirectoryOpusSource? consumed = null;
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
            if (_sessions.TryGetValue(hostContextId, out var active) &&
                active.WindowHandle == windowHandle &&
                active.ProcessId == foreground.ProcessId &&
                active.Adapter.Id.Equals("Directory Opus", StringComparison.Ordinal))
            {
                previous = active;
            }
            else if (_consumedDirectoryOpusSources.TryGetValue(hostContextId, out var retired) &&
                     retired.WindowHandle == windowHandle &&
                     retired.ProcessId == foreground.ProcessId)
            {
                consumed = retired;
            }
            else
            {
                return null;
            }
        }

        var adapter = previous?.Adapter ?? consumed!.Adapter;
        if (!adapter.TryCapture(foreground, out var capture, out _) || capture is null)
        {
            return null;
        }

        lock (_gate)
        {
            if (previous is not null)
            {
                if (!_sessions.TryGetValue(hostContextId, out var current) ||
                    !ReferenceEquals(current, previous))
                {
                    return null;
                }
                _sessions[hostContextId] = previous with
                {
                    PaneState = capture.PaneState,
                    CapturedAt = DateTimeOffset.UtcNow
                };
                return new ThirdPartyFileManagerContext(
                    windowHandle,
                    capture.CurrentDirectory,
                    hostContextId);
            }

            if (!_consumedDirectoryOpusSources.TryGetValue(hostContextId, out var currentConsumed) ||
                !ReferenceEquals(currentConsumed, consumed))
            {
                return null;
            }
            _consumedDirectoryOpusSources.Remove(hostContextId);
            var refreshedContextId = Guid.NewGuid().ToString("N");
            _sessions[refreshedContextId] = new CapturedSession(
                adapter,
                windowHandle,
                foreground.ProcessId,
                capture.PaneState,
                DateTimeOffset.UtcNow);
            _sessionOrderNodes[refreshedContextId] =
                _sessionOrder.AddLast(refreshedContextId);
            TrimToCapacity();
            return new ThirdPartyFileManagerContext(
                windowHandle,
                capture.CurrentDirectory,
                refreshedContextId);
        }
    }


    public bool IsDirectoryOpusSource(
        long windowHandle,
        string? hostContextId)
    {
        if (windowHandle == 0 || string.IsNullOrWhiteSpace(hostContextId))
        {
            return false;
        }
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
            return (_sessions.TryGetValue(hostContextId, out var session) &&
                    session.WindowHandle == windowHandle &&
                    session.Adapter.Id.Equals("Directory Opus", StringComparison.Ordinal)) ||
                (_consumedDirectoryOpusSources.TryGetValue(hostContextId, out var consumed) &&
                 consumed.WindowHandle == windowHandle);
        }
    }


    public WindowsCommandResult Navigate(
        NavigateThirdPartyFileManagerCommand command,
        Func<string, bool> directoryExists)
    {
        if (command.WindowHandle == 0)
        {
            return new WindowsCommandResult(
                false,
                "The captured third-party file manager window is unavailable.");
        }
        if (string.IsNullOrWhiteSpace(command.TargetDirectory) ||
            !directoryExists(command.TargetDirectory))
        {
            return new WindowsCommandResult(
                false,
                "The directory selected in Quick Launch no longer exists.");
        }
        CapturedSession? session;
        lock (_gate)
        {
            Cleanup(DateTimeOffset.UtcNow);
            if (string.IsNullOrWhiteSpace(command.HostContextId) ||
                !_sessions.TryGetValue(
                    command.HostContextId,
                    out session) ||
                session.WindowHandle != command.WindowHandle ||
                !session.Adapter.CanNavigate(
                    command.WindowHandle,
                    session.PaneState))
            {
                return new WindowsCommandResult(
                    false,
                    "The captured third-party file manager pane is no longer available; reopen Quick Launch from that pane.");
            }
        }
        var navigation = session.Adapter.Navigate(
                command.WindowHandle,
                command.TargetDirectory,
                session.PaneState);
        if (!navigation.Success)
        {
            if (navigation.Fatal)
            {
                lock (_gate)
                {
                    RemoveSession(command.HostContextId!);
                }
            }
            return navigation;
        }
        if (session.Adapter.RequiresPostNavigationActivation &&
            !session.Adapter.TryActivate(
                command.WindowHandle,
                session.PaneState,
                out var activationError))
        {
            return new WindowsCommandResult(
                false,
                activationError ??
                $"The navigated {session.Adapter.Id} window could not be activated.");
        }
        lock (_gate)
        {
            if (session.Adapter.Id.Equals("Directory Opus", StringComparison.Ordinal))
            {
                _consumedDirectoryOpusSources[command.HostContextId!] =
                    new ConsumedDirectoryOpusSource(
                        session.Adapter,
                        session.WindowHandle,
                        session.ProcessId,
                        DateTimeOffset.UtcNow);
                TrimConsumedDirectoryOpusSources();
            }
            RemoveSession(command.HostContextId!);
        }
        return new WindowsCommandResult(true);
    }

    internal int SessionCount
    {
        get
        {
            lock (_gate)
            {
                return _sessions.Count;
            }
        }
    }

    internal int SessionOrderCount
    {
        get
        {
            lock (_gate)
            {
                return _sessionOrder.Count;
            }
        }
    }

    private void Cleanup(DateTimeOffset now)
    {
        foreach (var (contextId, session) in _sessions.ToArray())
        {
            if (now - session.CapturedAt > SessionLifetime)
            {
                RemoveSession(contextId);
            }
        }
        foreach (var (contextId, consumed) in _consumedDirectoryOpusSources.ToArray())
        {
            if (now - consumed.ConsumedAt > SessionLifetime)
            {
                _consumedDirectoryOpusSources.Remove(contextId);
            }
        }
        TrimToCapacity();
    }

    private void TrimToCapacity()
    {
        while (_sessions.Count > MaximumSessions &&
               _sessionOrder.First is { } oldest)
        {
            RemoveSession(oldest.Value);
        }
    }

    private void TrimConsumedDirectoryOpusSources()
    {
        while (_consumedDirectoryOpusSources.Count > MaximumSessions)
        {
            var oldest = _consumedDirectoryOpusSources
                .MinBy(static pair => pair.Value.ConsumedAt);
            if (oldest.Key is null)
            {
                return;
            }
            _consumedDirectoryOpusSources.Remove(oldest.Key);
        }
    }

    internal void ReleaseContext(string? contextId)
    {
        if (string.IsNullOrWhiteSpace(contextId)) return;
        lock (_gate) RemoveSession(contextId);
    }

    internal void Clear()
    {
        lock (_gate)
        {
            foreach (var contextId in _sessions.Keys.ToArray()) RemoveSession(contextId);
            _consumedDirectoryOpusSources.Clear();
        }
    }

    private void RemoveSession(string contextId)
    {
        if (_sessions.Remove(contextId, out var session) && session.PaneState is OneCommanderPaneState pane)
            pane.Dispose();
        if (_sessionOrderNodes.Remove(contextId, out var node))
        {
            _sessionOrder.Remove(node);
        }
    }

    private sealed record CapturedSession(
        IThirdPartyFileManagerOperations Adapter,
        long WindowHandle,
        uint ProcessId,
        object PaneState,
        DateTimeOffset CapturedAt);

    private sealed record ConsumedDirectoryOpusSource(
        IThirdPartyFileManagerOperations Adapter,
        long WindowHandle,
        uint ProcessId,
        DateTimeOffset ConsumedAt);
}


internal static class ThirdPartyFileManagerPath
{
    public static bool LooksLikeRootedDirectory(string? path) =>
        !string.IsNullOrWhiteSpace(path) &&
        Path.IsPathRooted(path) &&
        (path.Contains(@":\", StringComparison.Ordinal) ||
         path.StartsWith(@"\\", StringComparison.Ordinal));

    public static bool SameDirectory(string? left, string? right)
    {
        if (!LooksLikeRootedDirectory(left) ||
            !LooksLikeRootedDirectory(right))
        {
            return false;
        }
        return string.Equals(
            Path.TrimEndingDirectorySeparator(left!),
            Path.TrimEndingDirectorySeparator(right!),
            StringComparison.OrdinalIgnoreCase);
    }
}
