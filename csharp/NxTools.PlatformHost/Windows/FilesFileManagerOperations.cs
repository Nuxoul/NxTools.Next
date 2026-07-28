using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Automation;

namespace NxTools.PlatformHost.Windows;

internal interface IFilesWindowOperations
{
    bool IsMainWindow(long windowHandle);

    bool TryCapture(
        long windowHandle,
        out string? currentDirectory,
        out object? windowState,
        out WindowsCommandResult? failure);

    WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object windowState);

    bool TryActivate(long windowHandle, out string? error);
}

internal sealed class FilesFileManagerOperations :
    IThirdPartyFileManagerOperations
{
    private readonly IFilesWindowOperations _windows;

    public FilesFileManagerOperations()
        : this(new UiaFilesWindowOperations())
    {
    }

    internal FilesFileManagerOperations(IFilesWindowOperations windows)
    {
        _windows = windows;
    }

    public string Id => "Files";

    internal static bool IsFilesProcessName(string? processName) =>
        string.Equals(
            Path.GetFileNameWithoutExtension(processName),
            "Files",
            StringComparison.OrdinalIgnoreCase);

    public bool TryCapture(
        ForegroundContext foreground,
        out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure)
    {
        capture = null;
        failure = null;
        if (!IsFilesProcessName(foreground.ProcessName) ||
            !_windows.IsMainWindow(foreground.WindowHandle) ||
            !_windows.TryCapture(
                foreground.WindowHandle,
                out var capturedDirectory,
                out var windowState,
                out failure) ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                capturedDirectory) ||
            windowState is null)
        {
            return false;
        }
        capture = new ThirdPartyFileManagerCapture(
            capturedDirectory!,
            windowState);
        return true;
    }

    public bool CanNavigate(long windowHandle, object paneState) =>
        _windows.IsMainWindow(windowHandle);

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState) =>
        _windows.Navigate(windowHandle, targetDirectory, paneState);

    public bool TryActivate(
        long windowHandle,
        object paneState,
        out string? error) =>
        _windows.TryActivate(windowHandle, out error);
}

internal sealed record FilesWindowState(AutomationElement BridgeRoot);

internal interface IFilesPathControls
{
    bool TryWrite(
        string targetDirectory,
        CancellationToken cancellationToken);

    string? Read(CancellationToken cancellationToken);
}

internal sealed record FilesPathNavigationResult(
    bool Success,
    string? Error);

internal static class FilesPathNavigation
{
    public static FilesPathNavigationResult Navigate(
        IFilesPathControls controls,
        string targetDirectory,
        TimeSpan confirmationTimeout,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (!controls.TryWrite(targetDirectory, cancellationToken))
        {
            return new FilesPathNavigationResult(
                false,
                "Files did not expose the captured window's directory control.");
        }

        var deadline = Stopwatch.StartNew();
        do
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (ThirdPartyFileManagerPath.SameDirectory(
                    controls.Read(cancellationToken),
                    targetDirectory))
            {
                return new FilesPathNavigationResult(true, null);
            }
            if (deadline.Elapsed >= confirmationTimeout)
            {
                break;
            }
            Thread.Sleep(25);
        }
        while (true);

        return new FilesPathNavigationResult(
            false,
            "Files did not switch the captured window to the requested directory.");
    }
}

internal sealed class UiaFilesWindowOperations : IFilesWindowOperations
{
    private const string BridgeClassName =
        "Microsoft.UI.Content.DesktopChildSiteBridge";
    private const string CurrentPathGetter = "CurrentPathGet";
    private const string CurrentPathSetter = "CurrentPathSet";
    private static readonly TimeSpan CaptureTimeout =
        TimeSpan.FromMilliseconds(150);
    private static readonly TimeSpan NavigationTimeout =
        TimeSpan.FromMilliseconds(1_500);

    public bool IsMainWindow(long windowHandle)
    {
        var window = new IntPtr(windowHandle);
        return window != IntPtr.Zero &&
            IsWindow(window) &&
            WindowBelongsToFiles(window);
    }

    public bool TryCapture(
        long windowHandle,
        out string? currentDirectory,
        out object? windowState,
        out WindowsCommandResult? failure)
    {
        currentDirectory = null;
        windowState = null;
        failure = null;
        if (!IsMainWindow(windowHandle))
        {
            return false;
        }
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.FilesCapture",
                CaptureTimeout,
                cancellationToken => CaptureWindow(
                    windowHandle,
                    cancellationToken),
                out var captured,
                out var runnerError))
        {
            failure = TimedOut(runnerError);
            return false;
        }
        if (captured is null ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                captured.CurrentDirectory))
        {
            return false;
        }
        currentDirectory = captured.CurrentDirectory;
        windowState = captured.WindowState;
        return true;
    }

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object windowState)
    {
        if (!IsMainWindow(windowHandle))
        {
            return new WindowsCommandResult(
                false,
                "The captured Files window is no longer available.");
        }
        if (windowState is not FilesWindowState filesWindow)
        {
            return new WindowsCommandResult(
                false,
                "The captured Files window context is invalid.");
        }
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.FilesNavigation",
                NavigationTimeout,
                cancellationToken => NavigateWindow(
                    windowHandle,
                    targetDirectory,
                    filesWindow,
                    cancellationToken),
                out var result,
                out var runnerError))
        {
            return TimedOut(runnerError);
        }
        return result is { Success: true }
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                result?.Error ??
                "Files did not switch the captured window to the requested directory.");
    }

    internal static WindowsCommandResult TimedOut(string? error) =>
        new(
            false,
            error ??
            "Files did not finish directory navigation before timeout.",
            Fatal: true);

    public bool TryActivate(long windowHandle, out string? error)
    {
        if (WindowsPlatformAdapter.TryActivateNativeWindow(windowHandle))
        {
            error = null;
            return true;
        }
        error = "Windows did not restore the captured Files window.";
        return false;
    }

    private static FilesCaptureResult? CaptureWindow(
        long windowHandle,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var root = AutomationElement.FromHandle(new IntPtr(windowHandle));
        cancellationToken.ThrowIfCancellationRequested();
        if (!IsFilesRoot(root))
        {
            return null;
        }
        var bridge = FindBridge(root, cancellationToken);
        if (bridge is null ||
            FindPathElement(
                bridge,
                CurrentPathSetter,
                cancellationToken) is null)
        {
            return null;
        }
        var getter = FindPathElement(
            bridge,
            CurrentPathGetter,
            cancellationToken);
        var currentDirectory = ReadValue(getter, cancellationToken);
        return ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                currentDirectory)
            ? new FilesCaptureResult(
                currentDirectory!,
                new FilesWindowState(bridge))
            : null;
    }

    private static FilesPathNavigationResult NavigateWindow(
        long windowHandle,
        string targetDirectory,
        FilesWindowState windowState,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var root = AutomationElement.FromHandle(new IntPtr(windowHandle));
        if (!IsFilesRoot(root) ||
            !IsInsideRoot(windowState.BridgeRoot, root))
        {
            return new FilesPathNavigationResult(
                false,
                "The captured Files content host is no longer available.");
        }
        return FilesPathNavigation.Navigate(
            new UiaFilesPathControls(windowState.BridgeRoot),
            targetDirectory,
            TimeSpan.FromSeconds(1),
            cancellationToken);
    }

    internal static bool IsFilesRootName(string? name) =>
        !string.IsNullOrEmpty(name) &&
        (name.Equals("Files", StringComparison.Ordinal) ||
         name.Contains("- Files", StringComparison.Ordinal));

    private static bool IsFilesRoot(AutomationElement root) =>
        IsFilesRootName(root.Current.Name);

    private static AutomationElement? FindBridge(
        AutomationElement root,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return root.FindFirst(
            TreeScope.Children,
            new PropertyCondition(
                AutomationElement.ClassNameProperty,
                BridgeClassName));
    }

    private static AutomationElement? FindPathElement(
        AutomationElement bridge,
        string automationId,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return bridge.FindFirst(
            TreeScope.Children,
            new PropertyCondition(
                AutomationElement.AutomationIdProperty,
                automationId));
    }

    private static string? ReadValue(
        AutomationElement? element,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (element is null ||
            !element.TryGetCurrentPattern(
                ValuePattern.Pattern,
                out var pattern) ||
            pattern is not ValuePattern valuePattern)
        {
            return null;
        }
        cancellationToken.ThrowIfCancellationRequested();
        return valuePattern.Current.Value;
    }

    private sealed class UiaFilesPathControls(
        AutomationElement bridge) : IFilesPathControls
    {
        public bool TryWrite(
            string targetDirectory,
            CancellationToken cancellationToken)
        {
            var setter = FindPathElement(
                bridge,
                CurrentPathSetter,
                cancellationToken);
            if (setter is null ||
                !setter.TryGetCurrentPattern(
                    ValuePattern.Pattern,
                    out var setterPattern) ||
                setterPattern is not ValuePattern setterValue)
            {
                return false;
            }
            cancellationToken.ThrowIfCancellationRequested();
            setterValue.SetValue(targetDirectory);
            return true;
        }

        public string? Read(CancellationToken cancellationToken) =>
            ReadValue(
                FindPathElement(
                    bridge,
                    CurrentPathGetter,
                    cancellationToken),
                cancellationToken);
    }

    private static bool IsInsideRoot(
        AutomationElement candidate,
        AutomationElement root)
    {
        var walker = TreeWalker.RawViewWalker;
        AutomationElement? node = candidate;
        for (var depth = 0; depth < 64 && node is not null; depth++)
        {
            try
            {
                if (System.Windows.Automation.Automation.Compare(node, root))
                {
                    return true;
                }
                node = walker.GetParent(node);
            }
            catch (ElementNotAvailableException)
            {
                return false;
            }
        }
        return false;
    }

    private static bool WindowBelongsToFiles(IntPtr window)
    {
        _ = GetWindowThreadProcessId(window, out var processId);
        if (processId == 0)
        {
            return false;
        }
        try
        {
            using var process = Process.GetProcessById((int)processId);
            return process.ProcessName.Equals(
                "Files",
                StringComparison.OrdinalIgnoreCase);
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    private sealed record FilesCaptureResult(
        string CurrentDirectory,
        FilesWindowState WindowState);

}
