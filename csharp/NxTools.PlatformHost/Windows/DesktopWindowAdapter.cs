using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal interface IDesktopWindowOperations
{
    bool IsDesktopWindow(long windowHandle);
    bool TryOpenDirectory(string targetDirectory, out string? error);
}

internal static class DesktopWindowAdapter
{
    public static DesktopContext? TryCapture(ForegroundContext foreground) =>
        TryCapture(
            foreground,
            new Win32DesktopWindowOperations(),
            () => Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory));

    internal static DesktopContext? TryCapture(
        ForegroundContext foreground,
        IDesktopWindowOperations operations,
        Func<string> desktopDirectory)
    {
        if (foreground.WindowHandle == 0 ||
            !foreground.ProcessName.Equals("explorer", StringComparison.OrdinalIgnoreCase) &&
            !foreground.ProcessName.Equals("explorer.exe", StringComparison.OrdinalIgnoreCase) ||
            !operations.IsDesktopWindow(foreground.WindowHandle))
        {
            return null;
        }

        var currentDirectory = desktopDirectory();
        return new DesktopContext(
            foreground.WindowHandle,
            string.IsNullOrWhiteSpace(currentDirectory) ? null : currentDirectory);
    }

    public static WindowsCommandResult Navigate(NavigateDesktopCommand command) =>
        Navigate(command, new Win32DesktopWindowOperations(), Directory.Exists);

    internal static WindowsCommandResult Navigate(
        NavigateDesktopCommand command,
        IDesktopWindowOperations operations,
        Func<string, bool> directoryExists)
    {
        if (command.WindowHandle == 0 ||
            !operations.IsDesktopWindow(command.WindowHandle))
        {
            return new WindowsCommandResult(false, "The captured Shell desktop is unavailable.");
        }
        if (string.IsNullOrWhiteSpace(command.TargetDirectory) ||
            !directoryExists(command.TargetDirectory))
        {
            return new WindowsCommandResult(false, "The target directory no longer exists.");
        }
        return operations.TryOpenDirectory(command.TargetDirectory, out var error)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                error ?? "Windows Shell could not open the target directory.");
    }
}

internal sealed class Win32DesktopWindowOperations : IDesktopWindowOperations
{
    public bool IsDesktopWindow(long windowHandle)
    {
        var hwnd = new IntPtr(windowHandle);
        if (hwnd == IntPtr.Zero || !IsWindow(hwnd))
        {
            return false;
        }

        var className = GetWindowClass(hwnd);
        return IsShellDesktopIdentity(
            hwnd == GetShellWindow(),
            className,
            className.Equals("WorkerW", StringComparison.OrdinalIgnoreCase) &&
            FindWindowEx(hwnd, IntPtr.Zero, "SHELLDLL_DefView", null) != IntPtr.Zero,
            IsExplorerProcess(hwnd));
    }

    internal static bool IsShellDesktopIdentity(
        bool isShellWindow,
        string className,
        bool hasShellDefView,
        bool isExplorerProcess)
    {
        if (!isExplorerProcess)
        {
            return false;
        }
        var isProgramManager =
            className.Equals("Progman", StringComparison.OrdinalIgnoreCase);
        var isDesktopWorker =
            className.Equals("WorkerW", StringComparison.OrdinalIgnoreCase);
        if (isShellWindow)
        {
            return isProgramManager || isDesktopWorker;
        }
        return isDesktopWorker && hasShellDefView;
    }

    public bool TryOpenDirectory(string targetDirectory, out string? error)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = targetDirectory,
                UseShellExecute = true
            });
            error = null;
            return true;
        }
        catch (Exception exception)
        {
            error = $"Windows Shell could not open the target directory: {exception.Message}";
            return false;
        }
    }

    private static string GetWindowClass(IntPtr hwnd)
    {
        var className = new StringBuilder(256);
        return GetClassName(hwnd, className, className.Capacity) > 0
            ? className.ToString()
            : string.Empty;
    }

    private static bool IsExplorerProcess(IntPtr hwnd)
    {
        _ = GetWindowThreadProcessId(hwnd, out var processId);
        if (processId == 0)
        {
            return false;
        }
        try
        {
            using var process = Process.GetProcessById(unchecked((int)processId));
            return process.ProcessName.Equals("explorer", StringComparison.OrdinalIgnoreCase);
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
    private static extern bool IsWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    private static extern IntPtr GetShellWindow();

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr hwnd,
        out uint processId);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr hwnd,
        StringBuilder className,
        int maxCount);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindowEx(
        IntPtr parent,
        IntPtr childAfter,
        string className,
        string? windowName);
}
