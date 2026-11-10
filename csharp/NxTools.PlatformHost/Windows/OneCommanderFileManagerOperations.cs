using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Automation;

namespace NxTools.PlatformHost.Windows;

internal interface IOneCommanderWindowOperations
{
    bool IsMainWindow(long windowHandle);

    bool TryCaptureActivePane(
        long windowHandle,
        long focusWindowHandle,
        out string? currentDirectory,
        out object? paneState,
        out WindowsCommandResult? failure);

    WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState);

    bool TryActivate(long windowHandle, out string? error);
}

internal sealed class OneCommanderFileManagerOperations :
    IThirdPartyFileManagerOperations
{
    private readonly IOneCommanderWindowOperations _windows;

    public OneCommanderFileManagerOperations()
        : this(new UiaOneCommanderWindowOperations())
    {
    }

    internal OneCommanderFileManagerOperations(
        IOneCommanderWindowOperations windows)
    {
        _windows = windows;
    }

    public string Id => "OneCommander";

    public bool TryCapture(
        ForegroundContext foreground,
        out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure)
    {
        capture = null;
        failure = null;
        if (!foreground.ProcessName.Equals(
                "OneCommander",
                StringComparison.OrdinalIgnoreCase) ||
            !_windows.IsMainWindow(foreground.WindowHandle) ||
            !_windows.TryCaptureActivePane(
                foreground.WindowHandle,
                foreground.FocusWindowHandle,
                out var capturedDirectory,
                out var paneState,
                out failure) ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                capturedDirectory) ||
            paneState is null)
        {
            return false;
        }
        capture = new ThirdPartyFileManagerCapture(
            capturedDirectory!,
            paneState);
        return true;
    }

    public bool CanNavigate(long windowHandle, object paneState) =>
        _windows.IsMainWindow(windowHandle);

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState) =>
        _windows.Navigate(
            windowHandle,
            targetDirectory,
            paneState);

    public bool TryActivate(
        long windowHandle,
        object paneState,
        out string? error) =>
        _windows.TryActivate(windowHandle, out error);
}

internal sealed record OneCommanderPaneState(
    AutomationElement PaneRoot);

internal sealed class UiaOneCommanderWindowOperations :
    IOneCommanderWindowOperations
{
    private const string CurrentPathGetter = "CurrentPathGet";
    private const string CurrentPathSetter = "CurrentPathSet";
    private static readonly TimeSpan CaptureTimeout =
        TimeSpan.FromMilliseconds(120);
    private static readonly TimeSpan NavigationTimeout =
        TimeSpan.FromMilliseconds(1_500);
    private static readonly Lazy<OneCommanderFocusTracker<AutomationElement>> FocusTracker = new(CreateFocusTracker);

    public static void WarmUpFocusTracker() =>
        FocusTracker.Value.EnsureStarted();

    public bool IsMainWindow(long windowHandle)
    {
        var hwnd = new IntPtr(windowHandle);
        return hwnd != IntPtr.Zero &&
            IsWindow(hwnd) &&
            GetWindowClass(hwnd).Contains(
                "OneCommander",
                StringComparison.OrdinalIgnoreCase) &&
            WindowBelongsToProcess(hwnd, "OneCommander");
    }

    public bool TryCaptureActivePane(
        long windowHandle,
        long focusWindowHandle,
        out string? currentDirectory,
        out object? paneState,
        out WindowsCommandResult? failure)
    {
        currentDirectory = null;
        paneState = null;
        failure = null;
        if (!IsMainWindow(windowHandle))
        {
            return false;
        }
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.OneCommanderCapture",
                CaptureTimeout,
                cancellationToken => CapturePane(
                    windowHandle,
                    focusWindowHandle,
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
        paneState = captured.PaneState;
        return true;
    }

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState)
    {
        if (!IsMainWindow(windowHandle))
        {
            return new WindowsCommandResult(
                false,
                "The captured OneCommander window is no longer available.");
        }
        if (paneState is not OneCommanderPaneState oneCommanderPane)
        {
            return new WindowsCommandResult(
                false,
                "The captured OneCommander pane context is invalid.");
        }
        if (!TryActivate(windowHandle, out var activationError))
        {
            return new WindowsCommandResult(false, activationError);
        }
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.OneCommanderNavigation",
                NavigationTimeout,
                cancellationToken => NavigatePane(
                    windowHandle,
                    targetDirectory,
                    oneCommanderPane,
                    cancellationToken),
                out var result,
                out var runnerError))
        {
            return TimedOut(runnerError);
        }
        if (result is null || !result.Success)
        {
            return new WindowsCommandResult(
                false,
                result?.Error ??
                "OneCommander did not switch the captured pane to the requested directory.");
        }
        return new WindowsCommandResult(true);
    }

    internal static WindowsCommandResult TimedOut(string? error) =>
        new(
            false,
            error ??
            "OneCommander did not finish directory navigation before timeout.",
            Fatal: true);

    public bool TryActivate(long windowHandle, out string? error)
    {
        if (WindowsPlatformAdapter.TryActivateNativeWindow(windowHandle))
        {
            error = null;
            return true;
        }
        error = "Windows did not restore the captured OneCommander window.";
        return false;
    }

    private static OneCommanderCaptureResult? CapturePane(
        long windowHandle,
        long focusWindowHandle,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var root = AutomationElement.FromHandle(new IntPtr(windowHandle));
        cancellationToken.ThrowIfCancellationRequested();
        AutomationElement? focused = null;
        if (focusWindowHandle != 0)
        {
            try
            {
                focused = AutomationElement.FromHandle(
                    new IntPtr(focusWindowHandle));
            }
            catch (ElementNotAvailableException)
            {
            }
            catch (InvalidOperationException)
            {
            }
            catch (COMException)
            {
            }
        }
        if (focused is null ||
            System.Windows.Automation.Automation.Compare(focused, root) ||
            !IsInsideRoot(focused, root))
        {
            focused = FocusTracker.Value.TryGet(windowHandle);
        }
        if (focused is null ||
            !IsInsideRoot(focused, root))
        {
            return null;
        }
        var paneRoot = FindNearestPaneRoot(
            root,
            focused,
            cancellationToken);
        if (paneRoot is null)
        {
            return null;
        }
        var getter = FindPaneElement(
            paneRoot,
            CurrentPathGetter,
            cancellationToken);
        var currentDirectory = ReadValue(getter, cancellationToken);
        return ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                currentDirectory)
            ? new OneCommanderCaptureResult(
                currentDirectory!,
                new OneCommanderPaneState(paneRoot))
            : null;
    }

    private static OneCommanderNavigationResult NavigatePane(
        long windowHandle,
        string targetDirectory,
        OneCommanderPaneState paneState,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var root = AutomationElement.FromHandle(new IntPtr(windowHandle));
        if (!IsInsideRoot(paneState.PaneRoot, root))
        {
            return new OneCommanderNavigationResult(
                false,
                "The captured OneCommander pane is no longer available.");
        }
        var setter = FindPaneElement(
            paneState.PaneRoot,
            CurrentPathSetter,
            cancellationToken);
        if (setter is null ||
            !setter.TryGetCurrentPattern(
                ValuePattern.Pattern,
                out var pattern) ||
            pattern is not ValuePattern valuePattern)
        {
            return new OneCommanderNavigationResult(
                false,
                "OneCommander did not expose the captured pane's directory control.");
        }
        cancellationToken.ThrowIfCancellationRequested();
        valuePattern.SetValue(targetDirectory);

        var deadline = Stopwatch.StartNew();
        while (deadline.ElapsedMilliseconds < 1_000)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var getter = FindPaneElement(
                paneState.PaneRoot,
                CurrentPathGetter,
                cancellationToken);
            var currentDirectory = ReadValue(
                getter,
                cancellationToken);
            if (ThirdPartyFileManagerPath.SameDirectory(
                    currentDirectory,
                    targetDirectory))
            {
                return new OneCommanderNavigationResult(true, null);
            }
            Thread.Sleep(25);
        }
        return new OneCommanderNavigationResult(
            false,
            "OneCommander did not switch the captured pane to the requested directory.");
    }

    private static AutomationElement? FindNearestPaneRoot(
        AutomationElement root,
        AutomationElement anchor,
        CancellationToken cancellationToken)
    {
        var walker = TreeWalker.RawViewWalker;
        AutomationElement? node = anchor;
        for (var depth = 0; depth < 25 && node is not null; depth++)
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                if (System.Windows.Automation.Automation.Compare(node, root))
                {
                    break;
                }
                if (FindPaneElement(
                        node,
                        CurrentPathGetter,
                        cancellationToken) is not null &&
                    FindPaneElement(
                        node,
                        CurrentPathSetter,
                        cancellationToken) is not null)
                {
                    return node;
                }
                node = walker.GetParent(node);
            }
            catch (ElementNotAvailableException)
            {
                break;
            }
        }
        return null;
    }

    private static AutomationElement? FindPaneElement(
        AutomationElement paneRoot,
        string automationId,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var condition = new PropertyCondition(
            AutomationElement.AutomationIdProperty,
            automationId);
        return paneRoot.FindFirst(TreeScope.Subtree, condition);
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

    private static bool IsInsideRoot(
        AutomationElement candidate,
        AutomationElement root)
    {
        var walker = TreeWalker.RawViewWalker;
        AutomationElement? node = candidate;
        for (var depth = 0; depth < 25 && node is not null; depth++)
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

    private static OneCommanderFocusTracker<AutomationElement> CreateFocusTracker()
    {
        var tracker = new OneCommanderFocusTracker<AutomationElement>(new(
            element => OneCommanderFocusProcessGate.ResolveTopLevelWindow(
                element.Current.ProcessId, IsOneCommanderProcessId,
                () => FindTopLevelWindowHandle(element),
                window => WindowBelongsToProcess(new IntPtr(window), "OneCommander")),
            window => IsWindow(new IntPtr(window)) && WindowBelongsToProcess(new IntPtr(window), "OneCommander"),
            element => { _ = element.Current.ProcessId; return true; },
            () => AutomationElement.FocusedElement,
            remember =>
            {
                AutomationFocusChangedEventHandler handler = (source, _) =>
                {
                    if (source is AutomationElement element) remember(element);
                };
                try { Automation.AddAutomationFocusChangedEventHandler(handler); }
                catch
                {
                    try { Automation.RemoveAutomationFocusChangedEventHandler(handler); } catch { }
                    throw;
                }
                return () => Automation.RemoveAutomationFocusChangedEventHandler(handler);
            }));
        AppDomain.CurrentDomain.ProcessExit += (_, _) => tracker.Dispose();
        return tracker;
    }

    private static long FindTopLevelWindowHandle(AutomationElement element)
    {
        var walker = TreeWalker.RawViewWalker;
        AutomationElement? node = element;
        long lastNativeWindow = 0;
        for (var depth = 0; depth < 64 && node is not null; depth++)
        {
            var nativeWindow = node.Current.NativeWindowHandle;
            if (nativeWindow != 0) lastNativeWindow = nativeWindow;
            node = walker.GetParent(node);
        }
        return lastNativeWindow;
    }

    private static bool WindowBelongsToProcess(
        IntPtr window,
        string expectedProcessName)
    {
        _ = GetWindowThreadProcessId(window, out var processId);
        return processId <= int.MaxValue &&
            IsProcessIdOwnedByProcess((int)processId, expectedProcessName);
    }

    private static bool IsOneCommanderProcessId(int processId) =>
        IsProcessIdOwnedByProcess(processId, "OneCommander");

    private static bool IsProcessIdOwnedByProcess(
        int processId,
        string expectedProcessName)
    {
        if (processId <= 0)
        {
            return false;
        }
        try
        {
            using var process = Process.GetProcessById(processId);
            return process.ProcessName.Equals(
                expectedProcessName,
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
        catch (System.ComponentModel.Win32Exception)
        {
            return false;
        }
    }

    private static string GetWindowClass(IntPtr window)
    {
        var value = new StringBuilder(512);
        return GetClassName(window, value, value.Capacity) > 0
            ? value.ToString()
            : string.Empty;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr window,
        StringBuilder value,
        int maximum);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    private sealed record OneCommanderCaptureResult(
        string CurrentDirectory,
        OneCommanderPaneState PaneState);

    private sealed record OneCommanderNavigationResult(
        bool Success,
        string? Error);
}

internal static class OneCommanderFocusProcessGate
{
    internal static long ResolveTopLevelWindow(
        int processId,
        Func<int, bool> isExpectedProcess,
        Func<long> findTopLevelWindow,
        Func<long, bool> windowBelongsToExpectedProcess)
    {
        if (processId <= 0 || !isExpectedProcess(processId))
        {
            return 0;
        }
        var windowHandle = findTopLevelWindow();
        return windowHandle != 0 && windowBelongsToExpectedProcess(windowHandle)
            ? windowHandle
            : 0;
    }
}
