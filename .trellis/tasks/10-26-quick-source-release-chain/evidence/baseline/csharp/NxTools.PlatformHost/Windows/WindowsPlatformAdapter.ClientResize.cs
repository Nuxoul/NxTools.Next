using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    private const uint ClientResizeSwpNoZOrder = 0x0004;
    private const uint ClientResizeSwpNoActivate = 0x0010;
    private const uint ClientResizeSwpFrameChanged = 0x0020;
    private const uint ClientResizeSwpNoMove = 0x0002;
    private const uint ClientResizeWmWindowPosChanged = 0x0047;

    [DllImport("user32.dll", EntryPoint = "GetWindowRect", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientResizeGetWindowRect(
        IntPtr window,
        out NativeWindowRectangle bounds);

    [DllImport("user32.dll", EntryPoint = "GetClientRect")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientResizeGetClientRect(
        IntPtr window,
        out NativeWindowRectangle bounds);

    [DllImport("user32.dll", EntryPoint = "ClientToScreen")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientResizeClientToScreen(
        IntPtr window,
        ref ClientResizePoint point);

    [DllImport("user32.dll", EntryPoint = "SetWindowPos", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientResizeSetWindowPos(
        IntPtr window,
        IntPtr insertAfter,
        int x,
        int y,
        int width,
        int height,
        uint flags);

    [StructLayout(LayoutKind.Sequential)]
    private struct ClientResizeWindowPosition
    {
        public IntPtr Window;
        public IntPtr InsertAfter;
        public int X;
        public int Y;
        public int Width;
        public int Height;
        public uint Flags;
    }

    [DllImport("user32.dll", EntryPoint = "SendMessageW", SetLastError = true)]
    private static extern IntPtr ClientResizeSendMessage(
        IntPtr window, uint message, IntPtr wParam, ref ClientResizeWindowPosition position);


    [StructLayout(LayoutKind.Sequential)]
    private struct ClientResizePoint
    {
        public int X;
        public int Y;
    }

    private static bool TryGetClientResizeScreenBounds(
        IntPtr window,
        out NativeWindowRectangle bounds)
    {
        bounds = default;
        if (!ClientResizeGetClientRect(window, out var client))
        {
            return false;
        }

        var topLeft = new ClientResizePoint { X = client.Left, Y = client.Top };
        var bottomRight = new ClientResizePoint { X = client.Right, Y = client.Bottom };
        if (!ClientResizeClientToScreen(window, ref topLeft) ||
            !ClientResizeClientToScreen(window, ref bottomRight))
        {
            return false;
        }

        bounds = new NativeWindowRectangle(
            topLeft.X,
            topLeft.Y,
            bottomRight.X,
            bottomRight.Y);
        return true;
    }

    private static WindowsCommandResult ApplyClientResizeWithFrameChange(
        IntPtr window,
        int requestedClientWidth,
        int requestedClientHeight)
    {
        if (window == IntPtr.Zero || !ClientResizeGetWindowRect(window, out var outer) ||
            !TryGetClientResizeScreenBounds(window, out var client))
        {
            return new WindowsCommandResult(false, "The GPUI client rectangle is unavailable.");
        }

        var visible = NativeMethods.CaptureWindowPresentation(window).Bounds;
        if (visible.Width <= 0 || visible.Height <= 0)
        {
            return new WindowsCommandResult(false, "The GPUI visible rectangle is unavailable.");
        }

        var insets = NativeWindowInsets.From(outer, client);
        var centerX = visible.Left + (visible.Right - visible.Left) / 2.0;
        var centerY = visible.Top + (visible.Bottom - visible.Top) / 2.0;
        var clientLeft = (int)Math.Round(centerX - requestedClientWidth / 2.0);
        var clientTop = (int)Math.Round(centerY - requestedClientHeight / 2.0);
        var x = clientLeft - insets.Left;
        var y = clientTop - insets.Top;
        var width = requestedClientWidth + insets.Left + insets.Right;
        var height = requestedClientHeight + insets.Top + insets.Bottom;
        var applied = ClientResizeSetWindowPos(
            window,
            IntPtr.Zero,
            x,
            y,
            width,
            height,
            ClientResizeSwpNoZOrder | ClientResizeSwpNoActivate | ClientResizeSwpFrameChanged);
        if (!applied)
        {
            var nativeError = Marshal.GetLastWin32Error();
            return new WindowsCommandResult(false,
                $"SetWindowPos(client/frame-changed) failed with Win32={nativeError}.", NativeErrorCode: nativeError);
        }

        if (!TryGetClientResizeScreenBounds(window, out var after) ||
            after.Width != requestedClientWidth ||
            after.Height != requestedClientHeight)
        {
            return new WindowsCommandResult(false, "Windows did not apply the requested GPUI client size.");
        }

        return new WindowsCommandResult(true);
    }

    internal static WindowsCommandResult RefreshNativeClientSizeForTest(long windowHandle) =>
        RefreshCurrentClientSize(new IntPtr(windowHandle));

    private static WindowsCommandResult RefreshCurrentClientSize(IntPtr window)
    {
        // DefWindowProc derives WM_SIZE from the live client on its owner thread.
        // Wait for processing to finish so the marshalled WINDOWPOS remains alive;
        // a timed send may return while the receiver still reads that parameter.
        // Only fixed resize workers call this in production. A hung HWND retains
        // its worker/slot, never blocks the reader or starts a replacement worker.
        var position = new ClientResizeWindowPosition
        {
            Window = window,
            Flags = ClientResizeSwpNoZOrder | ClientResizeSwpNoActivate | ClientResizeSwpNoMove
        };
        _ = ClientResizeSendMessage(window, ClientResizeWmWindowPosChanged, IntPtr.Zero, ref position);
        var nativeError = Marshal.GetLastWin32Error();
        if (nativeError != 0)
            return new WindowsCommandResult(false, "Current client refresh failed.", NativeErrorCode: nativeError);
        return NativeMethods.CaptureWindowPresentation(window).Exists
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(false, "Current client refresh window is unavailable.");
    }

    private static WindowsCommandResult EnsureRequestedClientSize(
        IntPtr window,
        int requestedClientWidth,
        int requestedClientHeight,
        Guid correlationId)
    {
        var current = NativeMethods.CaptureWindowPresentation(window);
        if (!current.Exists)
        {
            return new WindowsCommandResult(true);
        }
        if (NativeMethods.IsDraggingOrSizing(window))
        {
            NativeMethods.ParkCenteredResizeForRecovery(
                window,
                requestedClientWidth,
                requestedClientHeight,
                correlationId);
            return new WindowsCommandResult(true);
        }

        if (current.ClientBounds.Width == requestedClientWidth &&
            current.ClientBounds.Height == requestedClientHeight)
        {
            // Refresh from the native client at dispatch time, without queueing
            // a requested size that may become stale during move/size.
            return RefreshCurrentClientSize(window);
        }

        return ApplyClientResizeWithFrameChange(
            window,
            requestedClientWidth,
            requestedClientHeight);
    }
}
