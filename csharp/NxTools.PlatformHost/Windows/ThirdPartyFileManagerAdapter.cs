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
    private static readonly ThirdPartyFileManagerSessionRegistry Registry =
        new(
        [
            new DirectoryOpusFileManagerOperations(),
            new TotalCommanderFileManagerOperations(),
            new OneCommanderFileManagerOperations(),
            new FilesFileManagerOperations()
        ]);

    public static void WarmUp() =>
        UiaOneCommanderWindowOperations.WarmUpFocusTracker();

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

    private void RemoveSession(string contextId)
    {
        _sessions.Remove(contextId);
        if (_sessionOrderNodes.Remove(contextId, out var node))
        {
            _sessionOrder.Remove(node);
        }
    }

    private sealed record CapturedSession(
        IThirdPartyFileManagerOperations Adapter,
        long WindowHandle,
        object PaneState,
        DateTimeOffset CapturedAt);
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
