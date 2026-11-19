using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal interface IOneCommanderWindowOperations
{
    bool IsMainWindow(long windowHandle);
    bool TryCaptureActivePane(long windowHandle, long focusWindowHandle,
        out string? currentDirectory, out object? paneState, out WindowsCommandResult? failure);
    WindowsCommandResult Navigate(long windowHandle, string targetDirectory, object paneState);
    bool TryActivate(long windowHandle, out string? error);
}

internal sealed class OneCommanderFileManagerOperations : IThirdPartyFileManagerOperations
{
    private readonly IOneCommanderWindowOperations _windows;
    public OneCommanderFileManagerOperations() : this(new UiaOneCommanderWindowOperations()) { }
    internal OneCommanderFileManagerOperations(IOneCommanderWindowOperations windows) => _windows = windows;
    public string Id => "OneCommander";

    public bool TryCapture(ForegroundContext foreground, out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure)
    {
        capture = null;
        failure = null;
        if (!foreground.ProcessName.Equals("OneCommander", StringComparison.OrdinalIgnoreCase) ||
            !_windows.IsMainWindow(foreground.WindowHandle)) return false;
        if (!_windows.TryCaptureActivePane(foreground.WindowHandle, foreground.FocusWindowHandle,
                out var directory, out var pane, out failure)) return false;
        if (!ThirdPartyFileManagerPath.LooksLikeRootedDirectory(directory) || pane is null)
        {
            (pane as IDisposable)?.Dispose();
            return false;
        }
        capture = new(directory!, pane);
        return true;
    }

    public bool CanNavigate(long windowHandle, object paneState) => _windows.IsMainWindow(windowHandle);
    public WindowsCommandResult Navigate(long windowHandle, string targetDirectory, object paneState) =>
        _windows.Navigate(windowHandle, targetDirectory, paneState);
    public bool TryActivate(long windowHandle, object paneState, out string? error) =>
        _windows.TryActivate(windowHandle, out error);

    internal void BeginLifetime(IPlatformLog log, string role)
    {
        if (_windows is UiaOneCommanderWindowOperations native) native.BeginLifetime(log, role);
    }
    internal void ObserveForeground(ForegroundContext context)
    {
        if (_windows is UiaOneCommanderWindowOperations native) native.ObserveForeground(context);
    }
    internal void RetireWindow(long window)
    {
        if (_windows is UiaOneCommanderWindowOperations native) native.RetireWindow(window);
    }
    internal void Stop()
    {
        if (_windows is UiaOneCommanderWindowOperations native) native.Stop();
    }
}

internal sealed class UiaOneCommanderWindowOperations : IOneCommanderWindowOperations
{
    private static readonly TimeSpan CaptureTimeout = TimeSpan.FromMilliseconds(120);
    private static readonly TimeSpan NavigationTimeout = TimeSpan.FromMilliseconds(1_500);
    private readonly OneCommanderUiaSession session = new();

    internal void BeginLifetime(IPlatformLog log, string role) => session.BeginLifetime(log, role);
    internal void ObserveForeground(ForegroundContext context) => session.ObserveForeground(context);
    internal void RetireWindow(long window) => session.RetireWindow(window);
    internal void Stop() => session.Stop();
    public bool IsMainWindow(long windowHandle) => OneCommanderNativeWindow.IsMainWindow(windowHandle);

    public bool TryCaptureActivePane(long windowHandle, long focusWindowHandle,
        out string? currentDirectory, out object? paneState, out WindowsCommandResult? failure)
    {
        currentDirectory = null;
        paneState = null;
        failure = null;
        if (!IsMainWindow(windowHandle)) return false;
        using var handoff = new CaptureHandoff();
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.OneCommanderCapture", CaptureTimeout,
                cancellationToken => handoff.Publish(session.Capture(windowHandle, focusWindowHandle, cancellationToken), cancellationToken),
                out OneCommanderPaneCapture? captured, out var runnerError))
        {
            failure = TimedOut(runnerError);
            return false;
        }
        var accepted = handoff.Take(captured);
        if (accepted is null) return false;
        currentDirectory = accepted.CurrentDirectory;
        paneState = accepted.PaneState;
        return true;
    }

    public WindowsCommandResult Navigate(long windowHandle, string targetDirectory, object paneState)
    {
        if (!IsMainWindow(windowHandle))
            return new(false, "The captured OneCommander window is no longer available.");
        if (paneState is not OneCommanderPaneState pane || pane.WindowHandle != windowHandle || pane.IsRetired)
            return new(false, "The captured OneCommander pane context is invalid.");
        if (!TryActivate(windowHandle, out var activationError)) return new(false, activationError);
        return BoundedStaOperationRunner.TryRun(
                "NxTools.OneCommanderNavigation", NavigationTimeout,
                cancellationToken => session.Navigate(pane, targetDirectory, cancellationToken),
                out WindowsCommandResult? result, out var runnerError)
            ? result ?? new(false, "OneCommander did not finish directory navigation.")
            : TimedOut(runnerError);
    }

    internal static WindowsCommandResult TimedOut(string? error) =>
        new(false, error ?? "OneCommander did not finish directory navigation before timeout.", Fatal: true);

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

    private sealed class CaptureHandoff : IDisposable
    {
        private readonly object gate = new();
        private OneCommanderPaneCapture? pending;
        private bool closed;

        internal OneCommanderPaneCapture? Publish(OneCommanderPaneCapture? capture, CancellationToken cancellationToken)
        {
            if (capture is null) return null;
            lock (gate)
            {
                if (!closed && !cancellationToken.IsCancellationRequested)
                {
                    pending = capture;
                    return capture;
                }
            }
            capture.Dispose();
            return null;
        }

        internal OneCommanderPaneCapture? Take(OneCommanderPaneCapture? capture)
        {
            lock (gate)
            {
                if (closed || capture is null || !ReferenceEquals(pending, capture)) return null;
                pending = null;
                return capture;
            }
        }

        public void Dispose()
        {
            OneCommanderPaneCapture? retired;
            lock (gate)
            {
                closed = true;
                retired = pending;
                pending = null;
            }
            retired?.Dispose();
        }
    }
}

internal static class OneCommanderNativeWindow
{
    internal static bool IsMainWindow(long window)
    {
        if (window == 0 || !IsWindow(new IntPtr(window))) return false;
        var name = new StringBuilder(512);
        if (GetClassName(new IntPtr(window), name, name.Capacity) == 0 ||
            !name.ToString().Contains("OneCommander", StringComparison.OrdinalIgnoreCase)) return false;
        var processId = ProcessId(window);
        if (processId == 0 || processId > int.MaxValue) return false;
        try
        {
            using var process = Process.GetProcessById((int)processId);
            return process.ProcessName.Equals("OneCommander", StringComparison.OrdinalIgnoreCase);
        }
        catch (Exception error) when (error is ArgumentException or InvalidOperationException or System.ComponentModel.Win32Exception)
        {
            return false;
        }
    }

    internal static uint ProcessId(long window)
    {
        _ = GetWindowThreadProcessId(new IntPtr(window), out var processId);
        return processId;
    }

    internal static bool IsWithin(long window, long root, uint processId) =>
        window != 0 && root != 0 && processId != 0 && IsWindow(new IntPtr(window)) &&
        IsWindow(new IntPtr(root)) && ProcessId(window) == processId && ProcessId(root) == processId &&
        (window == root || GetAncestor(new IntPtr(window), 2).ToInt64() == root);

    internal static bool IsForeground(long window, uint processId) =>
        IsWithin(GetForegroundWindow().ToInt64(), window, processId);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder value, int maximum);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")]
    private static extern IntPtr GetAncestor(IntPtr window, uint flags);
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
}

internal static class OneCommanderFocusProcessGate
{
    internal static long ResolveTopLevelWindow(int processId, Func<int, bool> isExpectedProcess,
        Func<long> findTopLevelWindow, Func<long, bool> windowBelongsToExpectedProcess)
    {
        if (processId <= 0 || !isExpectedProcess(processId)) return 0;
        var window = findTopLevelWindow();
        return window != 0 && windowBelongsToExpectedProcess(window) ? window : 0;
    }
}
