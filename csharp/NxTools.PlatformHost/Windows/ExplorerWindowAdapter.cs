using System.Runtime.InteropServices;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal interface IExplorerWindowOperations
{
    bool IsExplorerWindow(long windowHandle);
    bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        CancellationToken cancellationToken,
        out string? error);
    bool TryActivate(long windowHandle, out string? error);
}

internal static class ExplorerWindowAdapter
{
    private sealed record CurrentDirectoryCapture(
        bool Success,
        string? CurrentDirectory,
        string? Error);

    public static ExplorerContext? TryCapture(ForegroundContext foreground) =>
        TryCapture(foreground, new ComExplorerWindowOperations());

    internal static ExplorerContext? TryCapture(
        ForegroundContext foreground,
        IExplorerWindowOperations operations)
    {
        if (foreground.WindowHandle == 0 ||
            !foreground.ProcessName.Equals("explorer", StringComparison.OrdinalIgnoreCase) &&
            !foreground.ProcessName.Equals("explorer.exe", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }
        if (!operations.IsExplorerWindow(foreground.WindowHandle))
        {
            return null;
        }
        return new ExplorerContext(foreground.WindowHandle, null);
    }

    public static WindowsCommandResult Navigate(NavigateExplorerCommand command)
    {
        if (!BoundedStaOperationRunner.TryRun(
            "NxTools.ExplorerNavigation",
            TimeSpan.FromSeconds(2),
            cancellationToken => Navigate(
                command,
                new ComExplorerWindowOperations(),
                Directory.Exists,
                cancellationToken),
            out WindowsCommandResult? result,
            out var error))
        {
            return TimedOut(error);
        }
        return result ??
            new WindowsCommandResult(
                false,
                "Explorer navigation did not return a result.");
    }

    internal static WindowsCommandResult TimedOut(string? error) =>
        new(
            false,
            error ?? "Explorer navigation timed out.",
            Fatal: true);

    internal static bool TryGetCurrentDirectory(
        long windowHandle,
        out string? currentDirectory,
        out string? error)
    {
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.ExplorerCurrentDirectory",
                TimeSpan.FromMilliseconds(750),
                cancellationToken =>
                {
                    var operations = new ComExplorerWindowOperations();
                    var success = operations.TryGetCurrentDirectory(
                        windowHandle,
                        cancellationToken,
                        out var captured,
                        out var captureError);
                    return new CurrentDirectoryCapture(
                        success,
                        captured,
                        captureError);
                },
                out CurrentDirectoryCapture? result,
                out var boundedError))
        {
            currentDirectory = null;
            error = boundedError;
            return false;
        }
        currentDirectory = result?.CurrentDirectory;
        error = result?.Error;
        return result?.Success == true &&
            !string.IsNullOrWhiteSpace(currentDirectory);
    }

    internal static WindowsCommandResult Navigate(
        NavigateExplorerCommand command,
        IExplorerWindowOperations operations,
        Func<string, bool> directoryExists,
        CancellationToken cancellationToken = default)
    {
        if (command.WindowHandle == 0)
        {
            return new WindowsCommandResult(false, "The captured Explorer window is unavailable.");
        }
        if (string.IsNullOrWhiteSpace(command.TargetDirectory) ||
            !directoryExists(command.TargetDirectory))
        {
            return new WindowsCommandResult(false, "The target directory no longer exists.");
        }
        if (!operations.TryNavigate(
                command.WindowHandle,
                command.TargetDirectory,
                cancellationToken,
                out var navigationError))
        {
            return new WindowsCommandResult(
                false,
                navigationError ?? "The captured Explorer window could not be navigated.");
        }
        cancellationToken.ThrowIfCancellationRequested();
        if (!operations.TryActivate(command.WindowHandle, out var activationError))
        {
            return new WindowsCommandResult(
                false,
                activationError ?? "The navigated Explorer window could not be activated.");
        }
        return new WindowsCommandResult(true);
    }
}

internal sealed class ComExplorerWindowOperations : IExplorerWindowOperations
{
    private static readonly Guid ShellWindowsClassId =
        new("9BA05972-F6A8-11CF-A442-00A0C90A8F39");

    public bool IsExplorerWindow(long windowHandle)
    {
        var className = new char[256];
        var length = GetClassName(new IntPtr(windowHandle), className, className.Length);
        if (length <= 0)
        {
            return false;
        }
        var value = new string(className, 0, length);
        return value.Equals("CabinetWClass", StringComparison.Ordinal) ||
               value.Equals("ExploreWClass", StringComparison.Ordinal);
    }

    public bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        CancellationToken cancellationToken,
        out string? error) =>
        TryWithWindow(windowHandle, cancellationToken, window =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            object url = targetDirectory;
            object flags = Type.Missing;
            object targetFrameName = Type.Missing;
            object postData = Type.Missing;
            object headers = Type.Missing;
            ((IExplorerBrowserDispatch)window).Navigate2(
                ref url,
                ref flags,
                ref targetFrameName,
                ref postData,
                ref headers);
            return true;
        }, out error);

    public bool TryActivate(long windowHandle, out string? error)
    {
        if (WindowsPlatformAdapter.TryActivateNativeWindow(windowHandle))
        {
            error = null;
            return true;
        }
        error = "Windows did not restore the navigated Explorer window to the foreground.";
        return false;
    }

    internal bool TryResolveWindow(long windowHandle, out string? error) =>
        TryWithWindow(windowHandle, CancellationToken.None, _ => true, out error);

    internal bool TryGetCurrentDirectory(
        long windowHandle,
        out string? currentDirectory,
        out string? error) =>
        TryGetCurrentDirectory(
            windowHandle,
            CancellationToken.None,
            out currentDirectory,
            out error);

    internal bool TryGetCurrentDirectory(
        long windowHandle,
        CancellationToken cancellationToken,
        out string? currentDirectory,
        out string? error)
    {
        string? captured = null;
        var resolved = TryWithWindow(windowHandle, cancellationToken, window =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            var locationUrl = ((IExplorerBrowserDispatch)window).LocationUrl;
            if (Uri.TryCreate(locationUrl, UriKind.Absolute, out Uri? uri) &&
                uri is { IsFile: true })
            {
                captured = uri!.LocalPath;
                return true;
            }
            return false;
        }, out error);
        currentDirectory = captured;
        return resolved && !string.IsNullOrWhiteSpace(captured);
    }

    private static bool TryWithWindow(
        long requestedWindowHandle,
        CancellationToken cancellationToken,
        Func<object, bool> operation,
        out string? error)
    {
        error = null;
        object? shellWindows = null;
        try
        {
            var shellWindowsType = Type.GetTypeFromCLSID(ShellWindowsClassId);
            if (shellWindowsType is null)
            {
                error = "Windows Shell automation is unavailable.";
                return false;
            }
            shellWindows = Activator.CreateInstance(shellWindowsType);
            if (shellWindows is null)
            {
                error = "Windows Shell automation could not be started.";
                return false;
            }

            dynamic windows = shellWindows;
            var count = Convert.ToInt32(windows.Count);
            for (var index = 0; index < count; index++)
            {
                cancellationToken.ThrowIfCancellationRequested();
                object? candidate = null;
                try
                {
                    candidate = windows.Item(index);
                    if (candidate is null)
                    {
                        continue;
                    }
                    IExplorerBrowserDispatch browser;
                    try
                    {
                        browser = (IExplorerBrowserDispatch)candidate;
                    }
                    catch (InvalidCastException)
                    {
                        continue;
                    }
                    if (browser.WindowHandle != requestedWindowHandle)
                    {
                        continue;
                    }
                    cancellationToken.ThrowIfCancellationRequested();
                    return operation(candidate);
                }
                finally
                {
                    ReleaseComObject(candidate);
                }
            }
            error = "The Explorer window captured when Quick Launch opened is no longer available.";
            return false;
        }
        catch (OperationCanceledException)
            when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            error = $"Explorer navigation failed: {exception.Message}";
            return false;
        }
        finally
        {
            ReleaseComObject(shellWindows);
        }
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr windowHandle,
        [Out] char[] className,
        int maxCount);

    private static void ReleaseComObject(object? value)
    {
        if (value is not null && Marshal.IsComObject(value))
        {
            _ = Marshal.FinalReleaseComObject(value);
        }
    }
}

[ComImport]
[Guid("D30C1661-CDAF-11D0-8A3E-00C04FC9E26E")]
[InterfaceType(ComInterfaceType.InterfaceIsIDispatch)]
internal interface IExplorerBrowserDispatch
{
    [DispId(-515)]
    int WindowHandle { get; }

    [DispId(211)]
    string LocationUrl { get; }

    [DispId(500)]
    void Navigate2(
        [In] ref object url,
        [In, Optional] ref object flags,
        [In, Optional] ref object targetFrameName,
        [In, Optional] ref object postData,
        [In, Optional] ref object headers);
}
