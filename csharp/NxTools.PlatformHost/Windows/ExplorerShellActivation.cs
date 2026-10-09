using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface IExplorerShellPlatform
{
    IntPtr FindExplorerWindow(string directory);
    void LaunchExplorer(string arguments);
    bool IsWindow(IntPtr window);
    bool IsIconic(IntPtr window);
    void RestoreWindow(IntPtr window);
    void TryActivateWindow(IntPtr window);
    IntPtr GetForegroundWindow();
    IntPtr GetRootWindow(IntPtr window);
    void WaitForNextAttempt();
    int LastErrorCode { get; }
}

internal static class ExplorerShellActivation
{
    private const int MaxAttempts = 8;

    internal static WindowsCommandResult LaunchAndActivate(string target) =>
        LaunchAndActivate(target, new NativeExplorerShellPlatform(), null);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        Action<string>? stageChanged) =>
        LaunchAndActivate(target, new NativeExplorerShellPlatform(), stageChanged);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        IExplorerShellPlatform platform) =>
        LaunchAndActivate(target, platform, null);

    internal static WindowsCommandResult LaunchAndActivate(
        string target,
        IExplorerShellPlatform platform,
        Action<string>? stageChanged)
    {
        var stage = "target_resolution";
        void SetStage(string nextStage)
        {
            if (stage == nextStage)
            {
                return;
            }
            stage = nextStage;
            try
            {
                stageChanged?.Invoke(stage);
            }
            catch
            {
                // Diagnostics must never change Shell behavior.
            }
        }

        try
        {
            var isDirectory = Directory.Exists(target);
            if (!isDirectory && !File.Exists(target))
            {
                return Failure("target_missing", "The Explorer target is no longer available.", 2);
            }
            SetStage("target_resolved");

            var directory = isDirectory ? target : Path.GetDirectoryName(target);
            if (string.IsNullOrWhiteSpace(directory))
            {
                return Failure("target_parent_missing", "The Explorer target has no containing directory.", 3);
            }
            directory = NormalizeDirectory(directory);

            SetStage("existing_window_lookup");
            var candidate = platform.FindExplorerWindow(directory);
            if (isDirectory && candidate != IntPtr.Zero)
            {
                SetStage("existing_window_found");
            }
            else
            {
                SetStage(isDirectory ? "directory_launch_requested" : "selection_requested");
                platform.LaunchExplorer(isDirectory
                    ? $"\"{target}\""
                    : $"/select,\"{target}\"");
            }

            var lastExpectedRoot = IntPtr.Zero;
            for (var attempt = 0; attempt < MaxAttempts; attempt++)
            {
                if (candidate == IntPtr.Zero || !platform.IsWindow(candidate))
                {
                    SetStage("explorer_window_lookup");
                    candidate = platform.FindExplorerWindow(directory);
                }

                if (candidate != IntPtr.Zero && platform.IsWindow(candidate))
                {
                    lastExpectedRoot = platform.GetRootWindow(candidate);
                    if (lastExpectedRoot == IntPtr.Zero)
                    {
                        lastExpectedRoot = candidate;
                    }

                    SetStage("foreground_activation_requested");
                    if (platform.IsIconic(candidate))
                    {
                        platform.RestoreWindow(candidate);
                    }
                    platform.TryActivateWindow(candidate);

                    SetStage("foreground_readback");
                    var foreground = platform.GetForegroundWindow();
                    if (platform.IsWindow(candidate) && foreground != IntPtr.Zero &&
                        platform.IsWindow(foreground) && platform.GetRootWindow(foreground) == lastExpectedRoot)
                    {
                        SetStage("foreground_confirmed");
                        return new WindowsCommandResult(true, Data: "foreground_confirmed");
                    }
                }

                if (attempt + 1 < MaxAttempts)
                {
                    platform.WaitForNextAttempt();
                }
            }

            var failureStage = candidate == IntPtr.Zero
                ? "explorer_window_not_found"
                : "foreground_not_confirmed";
            SetStage(failureStage);
            var message = candidate == IntPtr.Zero
                ? "Explorer did not publish the requested folder window in time."
                : "Windows did not confirm the requested Explorer window in the foreground.";
            return Failure(failureStage, message, platform.LastErrorCode);
        }
        catch (Win32Exception error)
        {
            return Failure(stage, "Windows could not activate the requested Explorer window.", error.NativeErrorCode);
        }
        catch (Exception error)
        {
            return Failure(stage, "Windows could not activate the requested Explorer window.", error.HResult);
        }
    }

    internal static bool IsExpectedForegroundRoot(
        IntPtr expectedRoot,
        IntPtr foregroundWindow,
        Func<IntPtr, IntPtr> getRootWindow)
    {
        return expectedRoot != IntPtr.Zero && foregroundWindow != IntPtr.Zero &&
            getRootWindow(foregroundWindow) == expectedRoot;
    }

    internal static string NormalizeDirectory(string directory)
    {
        var fullPath = Path.GetFullPath(directory);
        var root = Path.GetPathRoot(fullPath);
        return !string.IsNullOrEmpty(root) &&
            fullPath.Length <= root.Length
                ? root
                : fullPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
    }

    private static WindowsCommandResult Failure(
        string stage,
        string message,
        int nativeErrorCode) =>
        new(
            false,
            message,
            Data: stage,
            NativeErrorCode: nativeErrorCode == 0 ? null : nativeErrorCode);

    private sealed class NativeExplorerShellPlatform : IExplorerShellPlatform
    {
        private const int SwRestore = 9;
        private const uint GaRoot = 2;
        public int LastErrorCode => Marshal.GetLastPInvokeError();

        public IntPtr FindExplorerWindow(string directory)
        {
            var normalized = NormalizeDirectory(directory);
            var match = IntPtr.Zero;
            _ = EnumWindows((window, _) =>
            {
                if ((!IsWindowVisible(window) && !IsIconicNative(window)) || !IsExplorerClass(window))
                {
                    return true;
                }
                if (!TryGetExplorerDirectory(window, out var currentDirectory))
                {
                    return true;
                }
                if (string.Equals(
                    NormalizeDirectory(currentDirectory),
                    normalized,
                    StringComparison.OrdinalIgnoreCase))
                {
                    match = window;
                    return false;
                }
                return true;
            }, IntPtr.Zero);
            return match;
        }

        public void LaunchExplorer(string arguments)
        {
            _ = Process.Start(new ProcessStartInfo("explorer.exe", arguments)
            {
                UseShellExecute = true,
            });
        }

        public bool IsWindow(IntPtr window) => IsWindowNative(window);
        public bool IsIconic(IntPtr window) => IsIconicNative(window);
        public void RestoreWindow(IntPtr window) => _ = ShowWindow(window, SwRestore);

        public void TryActivateWindow(IntPtr window)
        {
            var currentThread = GetCurrentThreadId();
            var targetThread = GetWindowThreadProcessId(window, out _);
            var foreground = GetForegroundWindowNative();
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out _);
            var targetAttached = targetThread != 0 && targetThread != currentThread &&
                AttachThreadInput(currentThread, targetThread, true);
            var foregroundAttached = foregroundThread != 0 && foregroundThread != currentThread &&
                foregroundThread != targetThread && AttachThreadInput(currentThread, foregroundThread, true);
            try
            {
                _ = BringWindowToTop(window);
                _ = SetForegroundWindow(window);
            }
            finally
            {
                if (foregroundAttached)
                {
                    _ = AttachThreadInput(currentThread, foregroundThread, false);
                }
                if (targetAttached)
                {
                    _ = AttachThreadInput(currentThread, targetThread, false);
                }
            }
        }

        public IntPtr GetForegroundWindow() => GetForegroundWindowNative();
        public IntPtr GetRootWindow(IntPtr window) => GetAncestor(window, GaRoot);
        public void WaitForNextAttempt() => Thread.Sleep(75);

        private static bool TryGetExplorerDirectory(IntPtr window, out string directory)
        {
            directory = string.Empty;
            try
            {
                var shellWindowsType = Type.GetTypeFromCLSID(new Guid("9BA05972-F6A8-11CF-A442-00A0C90A8F39"));
                if (shellWindowsType is null || Activator.CreateInstance(shellWindowsType) is not object shellWindows)
                {
                    return false;
                }
                try
                {
                    dynamic windows = shellWindows;
                    var count = Convert.ToInt32(windows.Count);
                    for (var index = 0; index < count; index++)
                    {
                        object? candidate = null;
                        try
                        {
                            candidate = windows.Item(index);
                            if (candidate is not IExplorerBrowserDispatch browser ||
                                browser.WindowHandle != window.ToInt64())
                            {
                                continue;
                            }
                            var location = browser.LocationUrl;
                            if (Uri.TryCreate(location, UriKind.Absolute, out var uri) && uri.IsFile)
                            {
                                directory = uri.LocalPath;
                                return true;
                            }
                        }
                        finally
                        {
                            if (candidate is not null && Marshal.IsComObject(candidate))
                            {
                                Marshal.FinalReleaseComObject(candidate);
                            }
                        }
                    }
                }
                finally
                {
                    if (Marshal.IsComObject(shellWindows))
                    {
                        Marshal.FinalReleaseComObject(shellWindows);
                    }
                }
            }
            catch
            {
                return false;
            }
            return false;
        }

        private static bool IsExplorerClass(IntPtr window)
        {
            var className = new char[256];
            var length = GetClassName(window, className, className.Length);
            if (length <= 0)
            {
                return false;
            }
            var value = new string(className, 0, length);
            return value is "CabinetWClass" or "ExploreWClass";
        }

        private delegate bool EnumWindowsProc(IntPtr window, IntPtr data);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr data);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", EntryPoint = "IsWindow", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowNative(IntPtr window);
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern int GetClassName(IntPtr window, [Out] char[] className, int maxCount);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsIconicNative(IntPtr window);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ShowWindow(IntPtr window, int command);
        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, bool isAttached);
        [DllImport("user32.dll", EntryPoint = "GetForegroundWindow", SetLastError = true)]
        private static extern IntPtr GetForegroundWindowNative();
        [DllImport("user32.dll", SetLastError = true)]
        private static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);
    }

    [ComImport]
    [Guid("0002DF05-0000-0000-C000-000000000046")]
    [InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
    private interface IExplorerBrowserDispatch
    {
        [DispId(0x60020003)]
        long WindowHandle { get; }
        [DispId(0x60020008)]
        string LocationUrl { get; }
    }
}
