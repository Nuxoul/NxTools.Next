using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct TaskbarOverlayBounds(
    int Left,
    int Top,
    int Right,
    int Bottom)
{
    public int Width => Right - Left;
    public int Height => Bottom - Top;
}

internal static class TaskbarOverlayPlacement
{
    private static readonly IntPtr HwndTopmost = new(-1);
    private const int GwlExStyle = -20;
    private const long WsExTopmost = 0x00000008L;
    private const long WsExTransparent = 0x00000020L;
    private const long WsExToolWindow = 0x00000080L;
    private const long WsExLayered = 0x00080000L;
    private const long WsExAppWindow = 0x00040000L;
    private const long WsExNoActivate = 0x08000000L;
    private const uint SwpNoActivate = 0x0010;
    private const uint SwpShowWindow = 0x0040;
    private const uint SwpNoMove = 0x0002;
    private const uint SwpNoSize = 0x0001;
    private const int SwHide = 0;
    private const int SwShowNoActivate = 4;

    internal static TaskbarOverlayBounds CalculateBounds(
        TaskbarOverlayBounds taskbar,
        uint dpi,
        string anchor,
        int offsetX,
        int offsetY,
        int width,
        int height)
    {
        var safeDpi = Math.Max(96U, dpi);
        var scaledWidth = Scale(width, safeDpi);
        var scaledHeight = Scale(height, safeDpi);
        var scaledOffsetX = Scale(offsetX, safeDpi);
        var scaledOffsetY = Scale(offsetY, safeDpi);
        scaledWidth = Math.Clamp(scaledWidth, 1, Math.Max(1, taskbar.Width));
        scaledHeight = Math.Clamp(scaledHeight, 1, Math.Max(1, taskbar.Height));

        var desiredX = anchor.Equals("right", StringComparison.OrdinalIgnoreCase)
            ? taskbar.Right - scaledOffsetX - scaledWidth
            : taskbar.Left + scaledOffsetX;
        var desiredY = taskbar.Top + scaledOffsetY;
        var x = Math.Clamp(desiredX, taskbar.Left, taskbar.Right - scaledWidth);
        var y = Math.Clamp(desiredY, taskbar.Top, taskbar.Bottom - scaledHeight);
        return new TaskbarOverlayBounds(x, y, x + scaledWidth, y + scaledHeight);
    }

    internal static WindowsCommandResult Place(PlaceTaskbarProgressOverlayCommand command)
    {
        var window = new IntPtr(command.WindowHandle);
        if (window == IntPtr.Zero || !IsWindow(window))
        {
            return new WindowsCommandResult(false, "The GPUI taskbar overlay window handle is invalid.");
        }
        if (!command.Enabled)
        {
            _ = ShowWindow(window, SwHide);
            return new WindowsCommandResult(true, Data: "hidden");
        }
        if (!command.Anchor.Equals("left", StringComparison.OrdinalIgnoreCase) &&
            !command.Anchor.Equals("right", StringComparison.OrdinalIgnoreCase))
        {
            return new WindowsCommandResult(false, "Taskbar overlay anchor must be 'left' or 'right'.");
        }
        if (command.Width <= 0 || command.Height <= 0)
        {
            return new WindowsCommandResult(false, "Taskbar overlay dimensions must be positive.");
        }

        var taskbar = FindWindow("Shell_TrayWnd", null);
        if (taskbar == IntPtr.Zero || !TryGetTaskbarBounds(taskbar, out var taskbarBounds))
        {
            return new WindowsCommandResult(
                false,
                $"Could not locate the primary Windows taskbar (Win32={Marshal.GetLastWin32Error()}).");
        }

        var currentStyle = GetWindowLongPtr(window, GwlExStyle).ToInt64();
        var desiredStyle = (currentStyle | WsExToolWindow | WsExLayered) & ~WsExAppWindow;
        if (command.Interactive)
        {
            desiredStyle &= ~(WsExNoActivate | WsExTransparent);
        }
        else
        {
            desiredStyle |= WsExNoActivate | WsExTransparent;
        }
        if (desiredStyle != currentStyle)
        {
            Marshal.SetLastPInvokeError(0);
            var previous = SetWindowLongPtr(window, GwlExStyle, new IntPtr(desiredStyle));
            var styleError = Marshal.GetLastPInvokeError();
            if (previous == IntPtr.Zero && styleError != 0)
            {
                return new WindowsCommandResult(
                    false,
                    $"Could not apply taskbar overlay window styles (Win32={styleError}).");
            }
        }

        var dpi = TryGetDpi(taskbar);
        var bounds = CalculateBounds(
            taskbarBounds,
            dpi,
            command.Anchor,
            command.OffsetX,
            command.OffsetY,
            command.Width,
            command.Height);
        var positionFlags = SwpNoActivate;
        if (command.Visible)
        {
            positionFlags |= SwpShowWindow;
        }
        if (!SetWindowPos(
                window,
                HwndTopmost,
                bounds.Left,
                bounds.Top,
                bounds.Width,
                bounds.Height,
                positionFlags))
        {
            return new WindowsCommandResult(
                false,
                $"Could not position the taskbar overlay (Win32={Marshal.GetLastWin32Error()}).");
        }
        if (!command.Visible)
        {
            _ = ShowWindow(window, SwHide);
        }
        if ((command.Visible && !IsWindowVisible(window)) || !IsTopmost(window))
        {
            return new WindowsCommandResult(
                false,
                "Windows did not preserve the requested taskbar overlay visible/topmost state.");
        }
        return new WindowsCommandResult(
            true,
            Data: $"{bounds.Left},{bounds.Top},{bounds.Width},{bounds.Height};dpi={dpi};visible={command.Visible};topmost=True");
    }

    internal static WindowsCommandResult SetVisibility(IntPtr window, bool visible)
    {
        if (window == IntPtr.Zero || !IsWindow(window))
        {
            return new WindowsCommandResult(
                false,
                "The GPUI taskbar overlay window handle is invalid.");
        }
        if (visible)
        {
            if (!SetWindowPos(
                    window,
                    HwndTopmost,
                    0,
                    0,
                    0,
                    0,
                    SwpNoActivate | SwpNoMove | SwpNoSize | SwpShowWindow))
            {
                return new WindowsCommandResult(
                    false,
                    $"Could not show the taskbar overlay as topmost (Win32={Marshal.GetLastWin32Error()}).");
            }
        }
        else
        {
            _ = ShowWindow(window, SwHide);
        }
        var verifiedVisible = IsWindowVisible(window);
        var verifiedTopmost = IsTopmost(window);
        return verifiedVisible == visible && (!visible || verifiedTopmost)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                $"Windows did not apply the requested taskbar overlay state " +
                $"(visible={verifiedVisible}, topmost={verifiedTopmost}).");
    }

    internal static bool HasPassiveOverlayStyles(IntPtr window)
    {
        if (window == IntPtr.Zero || !IsWindow(window))
        {
            return false;
        }
        var style = GetWindowLongPtr(window, GwlExStyle).ToInt64();
        var required = WsExNoActivate | WsExToolWindow | WsExTransparent | WsExLayered;
        return (style & required) == required && (style & WsExAppWindow) == 0;
    }

    internal static bool HasInteractiveOverlayStyles(IntPtr window)
    {
        if (window == IntPtr.Zero || !IsWindow(window))
        {
            return false;
        }
        var style = GetWindowLongPtr(window, GwlExStyle).ToInt64();
        var required = WsExToolWindow | WsExLayered;
        var forbidden = WsExNoActivate | WsExTransparent | WsExAppWindow;
        return (style & required) == required && (style & forbidden) == 0;
    }

    internal static bool IsTopmost(IntPtr window)
    {
        if (window == IntPtr.Zero || !IsWindow(window))
        {
            return false;
        }
        return (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExTopmost) != 0;
    }

    internal static bool IsVisible(IntPtr window) =>
        window != IntPtr.Zero && IsWindow(window) && IsWindowVisible(window);

    internal static bool IsTaskbarWindow(IntPtr window)
    {
        if (window == IntPtr.Zero)
        {
            return false;
        }
        var className = new StringBuilder(64);
        if (GetClassName(window, className, className.Capacity) == 0)
        {
            return false;
        }
        return className.ToString() is "Shell_TrayWnd" or "Shell_SecondaryTrayWnd";
    }

    private static int Scale(int value, uint dpi) =>
        (int)Math.Round(value * dpi / 96.0, MidpointRounding.AwayFromZero);

    private static uint TryGetDpi(IntPtr window)
    {
        try
        {
            return Math.Max(96U, GetDpiForWindow(window));
        }
        catch (DllNotFoundException)
        {
            return 96;
        }
        catch (EntryPointNotFoundException)
        {
            return 96;
        }
    }

    private static bool TryGetTaskbarBounds(
        IntPtr taskbar,
        out TaskbarOverlayBounds bounds)
    {
        if (GetClientRect(taskbar, out var client))
        {
            var topLeft = new NativePoint(client.Left, client.Top);
            var bottomRight = new NativePoint(client.Right, client.Bottom);
            if (client.Right > client.Left &&
                client.Bottom > client.Top &&
                ClientToScreen(taskbar, ref topLeft) &&
                ClientToScreen(taskbar, ref bottomRight))
            {
                bounds = new TaskbarOverlayBounds(
                    topLeft.X,
                    topLeft.Y,
                    bottomRight.X,
                    bottomRight.Y);
                return true;
            }
        }

        // Explorer variants may expose no usable client rectangle. Falling back to the
        // taskbar window rectangle keeps the overlay bounded instead of abandoning it.
        if (GetWindowRect(taskbar, out var window))
        {
            bounds = new TaskbarOverlayBounds(
                window.Left,
                window.Top,
                window.Right,
                window.Bottom);
            return bounds.Width > 0 && bounds.Height > 0;
        }
        bounds = default;
        return false;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativePoint(int x, int y)
    {
        public int X = x;
        public int Y = y;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr FindWindow(string? className, string? windowName);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr window, out Rect rectangle);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetClientRect(IntPtr window, out Rect rectangle);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientToScreen(IntPtr window, ref NativePoint point);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder className, int maximumCount);

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr window);

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

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowWindow(IntPtr window, int command);

    private static IntPtr GetWindowLongPtr(IntPtr window, int index) =>
        IntPtr.Size == 8 ? GetWindowLongPtr64(window, index) : new IntPtr(GetWindowLong32(window, index));

    private static IntPtr SetWindowLongPtr(IntPtr window, int index, IntPtr value) =>
        IntPtr.Size == 8
            ? SetWindowLongPtr64(window, index, value)
            : new IntPtr(SetWindowLong32(window, index, value.ToInt32()));

    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    private static extern int GetWindowLong32(IntPtr window, int index);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern IntPtr GetWindowLongPtr64(IntPtr window, int index);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongW", SetLastError = true)]
    private static extern int SetWindowLong32(IntPtr window, int index, int value);

    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
    private static extern IntPtr SetWindowLongPtr64(IntPtr window, int index, IntPtr value);
}
