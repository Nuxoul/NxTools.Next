using System.Collections.Concurrent;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct NativeWindowRectangle(
    int Left,
    int Top,
    int Right,
    int Bottom)
{
    public int Width => Right - Left;
    public int Height => Bottom - Top;
}

internal readonly record struct NativeWindowInsets(
    int Left,
    int Top,
    int Right,
    int Bottom)
{
    public static NativeWindowInsets From(
        NativeWindowRectangle windowBounds,
        NativeWindowRectangle visibleBounds)
        => new(
            Math.Max(0, visibleBounds.Left - windowBounds.Left),
            Math.Max(0, visibleBounds.Top - windowBounds.Top),
            Math.Max(0, windowBounds.Right - visibleBounds.Right),
            Math.Max(0, windowBounds.Bottom - visibleBounds.Bottom));
}

internal static class AnchoredWindowPlacement
{
    private const int DwmExtendedFrameBounds = 9;
    private const uint SwpNoZOrder = 0x0004;
    private const uint SwpNoActivate = 0x0010;
    private static readonly ConcurrentDictionary<IntPtr, NativeWindowInsets> FrameInsets = new();

    public static NativeWindowRectangle Calculate(
        NativeWindowRectangle ownerVisibleBounds,
        NativeWindowRectangle windowBounds,
        NativeWindowRectangle windowVisibleBounds,
        int visibleWidth,
        int visibleHeight,
        int bottomOverlap)
    {
        if (ownerVisibleBounds.Width <= 0 ||
            ownerVisibleBounds.Height <= 0 ||
            visibleWidth <= 0 ||
            visibleHeight <= 0 ||
            bottomOverlap < 0)
        {
            throw new ArgumentOutOfRangeException(
                nameof(visibleWidth),
                "Anchored window dimensions and owner bounds must be valid.");
        }

        return Calculate(
            ownerVisibleBounds,
            NativeWindowInsets.From(windowBounds, windowVisibleBounds),
            visibleWidth,
            visibleHeight,
            bottomOverlap);
    }

    public static NativeWindowRectangle Calculate(
        NativeWindowRectangle ownerVisibleBounds,
        NativeWindowInsets frameInsets,
        int visibleWidth,
        int visibleHeight,
        int bottomOverlap)
    {
        if (ownerVisibleBounds.Width <= 0 ||
            ownerVisibleBounds.Height <= 0 ||
            visibleWidth <= 0 ||
            visibleHeight <= 0 ||
            bottomOverlap < 0)
        {
            throw new ArgumentOutOfRangeException(
                nameof(visibleWidth),
                "Anchored window dimensions and owner bounds must be valid.");
        }

        var visibleLeft = ownerVisibleBounds.Left +
            (ownerVisibleBounds.Width - visibleWidth) / 2;
        var visibleTop = ownerVisibleBounds.Bottom - bottomOverlap;

        return new NativeWindowRectangle(
            visibleLeft - frameInsets.Left,
            visibleTop - frameInsets.Top,
            visibleLeft + visibleWidth + frameInsets.Right,
            visibleTop + visibleHeight + frameInsets.Bottom);
    }

    public static WindowsCommandResult Place(
        IntPtr window,
        IntPtr owner,
        int visibleWidth,
        int visibleHeight,
        int bottomOverlap)
    {
        if (window == IntPtr.Zero || owner == IntPtr.Zero ||
            !IsWindow(window) || !IsWindow(owner))
        {
            return new WindowsCommandResult(
                false,
                "The GPUI search window or its file-dialog owner is unavailable.");
        }
        if (visibleWidth <= 0 || visibleHeight <= 0 || bottomOverlap < 0)
        {
            return new WindowsCommandResult(
                false,
                "Anchored window dimensions must be positive.");
        }
        if (!TryGetVisibleBounds(owner, out var ownerVisible) ||
            ownerVisible.Width <= 0 ||
            ownerVisible.Height <= 0)
        {
            return new WindowsCommandResult(
                false,
                "Windows did not expose the file dialog's visible bounds.");
        }
        if (!GetWindowRect(window, out var windowBounds))
        {
            return new WindowsCommandResult(
                false,
                $"GetWindowRect(search) failed with Win32={Marshal.GetLastWin32Error()}.");
        }
        if (!TryGetVisibleBounds(window, out var windowVisible))
        {
            windowVisible = windowBounds;
        }

        var frameInsets = NativeWindowInsets.From(windowBounds, windowVisible);
        var placement = Calculate(
            ownerVisible,
            frameInsets,
            visibleWidth,
            visibleHeight,
            bottomOverlap);
        if (!Apply(window, placement))
        {
            return new WindowsCommandResult(
                false,
                $"SetWindowPos(anchored bounds) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        // Windows can finalize a GPUI popup's non-client frame only after its first
        // resize. Re-read that frame and apply one bounded correction so the visible
        // surface, rather than the requested outer HWND, is actually centered.
        if (GetWindowRect(window, out var appliedWindow) &&
            TryGetVisibleBounds(window, out var appliedVisible))
        {
            frameInsets = NativeWindowInsets.From(appliedWindow, appliedVisible);
            var corrected = Calculate(
                ownerVisible,
                frameInsets,
                visibleWidth,
                visibleHeight,
                bottomOverlap);
            if (corrected != appliedWindow && !Apply(window, corrected))
            {
                return new WindowsCommandResult(
                    false,
                    $"SetWindowPos(visible-bounds correction) failed with Win32={Marshal.GetLastWin32Error()}.");
            }
        }
        FrameInsets[window] = frameInsets;
        var expectedVisibleLeft = ownerVisible.Left +
            (ownerVisible.Width - visibleWidth) / 2;
        var expectedVisibleTop = ownerVisible.Bottom - bottomOverlap;
        return new WindowsCommandResult(
            true,
            Data:
                $"owner={ownerVisible.Left},{ownerVisible.Top},{ownerVisible.Right},{ownerVisible.Bottom};" +
                $"window={expectedVisibleLeft},{expectedVisibleTop}," +
                $"{expectedVisibleLeft + visibleWidth},{expectedVisibleTop + visibleHeight}");
    }

    public static WindowsCommandResult Follow(
        IntPtr window,
        IntPtr owner,
        int visibleWidth,
        int visibleHeight,
        int bottomOverlap,
        out NativeWindowRectangle ownerVisible)
    {
        ownerVisible = default;
        if (window == IntPtr.Zero || owner == IntPtr.Zero ||
            !IsWindow(window) || !IsWindow(owner))
        {
            return new WindowsCommandResult(
                false,
                "The GPUI search window or its file-dialog owner is unavailable.");
        }
        if (visibleWidth <= 0 || visibleHeight <= 0 || bottomOverlap < 0)
        {
            return new WindowsCommandResult(
                false,
                "Anchored window dimensions must be positive.");
        }
        if (!TryGetVisibleBounds(owner, out ownerVisible) ||
            ownerVisible.Width <= 0 ||
            ownerVisible.Height <= 0)
        {
            return new WindowsCommandResult(
                false,
                "Windows did not expose the file dialog's visible bounds.");
        }
        if (!FrameInsets.TryGetValue(window, out var frameInsets))
        {
            if (!GetWindowRect(window, out var windowBounds))
            {
                return new WindowsCommandResult(
                    false,
                    $"GetWindowRect(search) failed with Win32={Marshal.GetLastWin32Error()}.");
            }
            if (!TryGetVisibleBounds(window, out var windowVisible))
            {
                windowVisible = windowBounds;
            }
            frameInsets = NativeWindowInsets.From(windowBounds, windowVisible);
            FrameInsets[window] = frameInsets;
        }

        var placement = Calculate(
            ownerVisible,
            frameInsets,
            visibleWidth,
            visibleHeight,
            bottomOverlap);
        return Apply(window, placement)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                $"SetWindowPos(anchored follow) failed with Win32={Marshal.GetLastWin32Error()}.");
    }

    public static void Forget(IntPtr window) => FrameInsets.TryRemove(window, out _);

    private static bool Apply(IntPtr window, NativeWindowRectangle bounds) =>
        SetWindowPos(
            window,
            IntPtr.Zero,
            bounds.Left,
            bounds.Top,
            bounds.Width,
            bounds.Height,
            SwpNoZOrder | SwpNoActivate);

    internal static bool TryGetVisibleBounds(
        IntPtr window,
        out NativeWindowRectangle bounds)
    {
        try
        {
            if (DwmGetWindowAttribute(
                    window,
                    DwmExtendedFrameBounds,
                    out bounds,
                    Marshal.SizeOf<NativeWindowRectangle>()) == 0)
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
        return GetWindowRect(window, out bounds);
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(
        IntPtr window,
        out NativeWindowRectangle bounds);

    [DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(
        IntPtr window,
        int attribute,
        out NativeWindowRectangle value,
        int valueSize);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(
        IntPtr window,
        IntPtr insertAfter,
        int x,
        int y,
        int width,
        int height,
        uint flags);
}
