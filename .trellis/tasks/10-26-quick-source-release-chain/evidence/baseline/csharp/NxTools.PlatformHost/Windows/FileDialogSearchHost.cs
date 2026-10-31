using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal sealed record FileDialogSearchHostSnapshot(
    long WindowHandle,
    string? CurrentDirectory,
    int Left,
    int Top,
    int Width,
    int Height,
    uint MonitorDpi);

internal static class FileDialogSearchHost
{
    private const int DwmExtendedFrameBounds = 9;

    public static bool CanReuseForeground(
        bool searchEnabled,
        bool hostVisible,
        FileDialogSearchHostSnapshot? snapshot,
        long foregroundWindowHandle)
        => searchEnabled &&
           hostVisible &&
           snapshot is not null &&
           snapshot.WindowHandle == foregroundWindowHandle;
    internal static bool ShouldRefreshDirectorySourceOnReuse(
        bool autoQuickSwitchEnabled,
        bool searchEnabled,
        bool hostVisible,
        FileDialogSearchHostSnapshot? snapshot,
        long foregroundWindowHandle,
        bool trackedSourceIsDirectoryOpus) =>
        autoQuickSwitchEnabled &&
        trackedSourceIsDirectoryOpus &&
        CanReuseForeground(
            searchEnabled,
            hostVisible,
            snapshot,
            foregroundWindowHandle);

    public static FileDialogSearchHostSnapshot? TryCapture(
        FileDialogContext dialog)
        => TryCapture(dialog.WindowHandle, dialog.CurrentDirectory);

    public static FileDialogSearchHostSnapshot RefreshGeometry(
        FileDialogSearchHostSnapshot snapshot,
        NativeWindowRectangle bounds)
    {
        var dpi = GetDpiForWindow(new IntPtr(snapshot.WindowHandle));
        return snapshot with
        {
            Left = bounds.Left,
            Top = bounds.Top,
            Width = bounds.Width,
            Height = bounds.Height,
            MonitorDpi = dpi == 0 ? snapshot.MonitorDpi : dpi
        };
    }

    private static FileDialogSearchHostSnapshot? TryCapture(
        long windowHandle,
        string? currentDirectory)
    {
        var handle = new IntPtr(windowHandle);
        if (handle == IntPtr.Zero ||
            !TryGetVisibleBounds(handle, out var bounds))
        {
            return null;
        }
        var width = bounds.Right - bounds.Left;
        var height = bounds.Bottom - bounds.Top;
        if (width <= 0 || height <= 0)
        {
            return null;
        }
        var dpi = GetDpiForWindow(handle);
        return new FileDialogSearchHostSnapshot(
            windowHandle,
            currentDirectory,
            bounds.Left,
            bounds.Top,
            width,
            height,
            dpi == 0 ? 96 : dpi);
    }

    private static bool TryGetVisibleBounds(IntPtr windowHandle, out Rect bounds)
    {
        bounds = default;
        try
        {
            if (DwmGetWindowAttribute(
                    windowHandle,
                    DwmExtendedFrameBounds,
                    out bounds,
                    Marshal.SizeOf<Rect>()) == 0)
            {
                return true;
            }
        }
        catch (DllNotFoundException)
        {
        }
        catch (EntryPointNotFoundException)
        {
        }
        return GetWindowRect(windowHandle, out bounds);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(
        IntPtr windowHandle,
        out Rect bounds);

    [DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(
        IntPtr windowHandle,
        int attribute,
        out Rect value,
        int valueSize);

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr windowHandle);
}
