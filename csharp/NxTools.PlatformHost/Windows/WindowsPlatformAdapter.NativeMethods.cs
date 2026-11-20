using System.Collections.Concurrent;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    public WindowsCommandResult QueryWindowGeometry(QueryWindowGeometryCommand command, uint desktopProcessId) =>
        NativeMethods.QueryWindowGeometry(new IntPtr(command.WindowHandle), desktopProcessId);

    internal static WindowsCommandResult WindowGeometryResult(NativeWindowRectangle outer,
        NativeWindowRectangle client, ulong displayId, uint dpi, bool visible)
    {
        if (outer.Width <= 0 || outer.Height <= 0 || client.Width <= 0 || client.Height <= 0 || dpi == 0)
            return new WindowsCommandResult(false, "window_geometry_unavailable");

        return new WindowsCommandResult(true, Data: System.Text.Json.JsonSerializer.Serialize(new
        {
            outer = new { left = outer.Left, top = outer.Top, right = outer.Right, bottom = outer.Bottom },
            client = new { left = client.Left, top = client.Top, right = client.Right, bottom = client.Bottom },
            display_id = displayId,
            dpi,
            visible
        }));
    }

    private static partial class NativeMethods
    {
        internal static int InputSize => Marshal.SizeOf<Input>();

        private const int SwRestore = 9;
        private const int SwMaximize = 3;
        private const uint InputKeyboard = 1;
        private const uint KeyeventfKeyup = 0x0002;
        private const ushort VkControl = 0x11;
        private const ushort VkV = 0x56;
        private const ushort VkShift = 0x10;
        private const ushort VkMenu = 0x12;
        private const ushort VkLWin = 0x5B;
        private const ushort VkRWin = 0x5C;
        private const ushort VkLShift = 0xA0;
        private const ushort VkRShift = 0xA1;
        private const ushort VkLControl = 0xA2;
        private const ushort VkRControl = 0xA3;
        private const ushort VkLMenu = 0xA4;
        private const ushort VkRMenu = 0xA5;
        private const int WmPaste = 0x0302;
        private const int WmCancelMode = 0x001F;
        private const int WmSysCommand = 0x0112;
        private const uint GaRoot = 2;
        private const uint SwpNoSize = 0x0001;
        private const uint SwpNoMove = 0x0002;
        private const uint SwpNoZOrder = 0x0004;
        private const uint SwpNoActivate = 0x0010;
        private const uint SwpFrameChanged = 0x0020;
        private const int GwlExStyle = -20;
        private const int GwlHwndParent = -8;
        private const int DwmCloaked = 14;
        private const long WsExTopmost = 0x00000008L;
        private const long WsExNoActivate = 0x08000000L;
        private const uint MonitorDefaultToNearest = 2;
        private const int MonitorDpiTypeEffective = 0;
        private const uint DefaultDpi = 96;
        private const int SwHide = 0;
        private const int SwShowNoActivate = 4;
        private const int SwShowMinNoActive = 7;

        [UnmanagedFunctionPointer(CallingConvention.Winapi)]
        internal delegate void WinEventDelegate(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool AddClipboardFormatListener(IntPtr window);


        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr hwnd, IntPtr processId);

        [StructLayout(LayoutKind.Sequential)]
        private struct GUITHREADINFO
        {
            public uint cbSize;
            public uint flags;
            public IntPtr hwndActive;
            public IntPtr hwndCapture;
            public IntPtr hwndMenuOwner;
            public IntPtr hwndMoveSize;
            public IntPtr hwndCaret;
            public NativeWindowRectangle rcCaret;
        }

        private const uint GUI_INMOVESIZE = 0x00000002;

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetGUIThreadInfo(uint threadId, ref GUITHREADINFO info);

        [DllImport("dwmapi.dll")]
        private static extern int DwmSetWindowAttribute(
            IntPtr hwnd,
            int attribute,
            ref int attributeValue,
            int attributeSize);
[DllImport("dwmapi.dll")]
        private static extern int DwmGetWindowAttribute(
            IntPtr window,
            int attribute,
            out int value,
            int valueSize);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RemoveClipboardFormatListener(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RegisterHotKey(
            IntPtr window,
            int id,
            uint modifiers,
            uint virtualKey);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnregisterHotKey(IntPtr window, int id);

        [DllImport("user32.dll", SetLastError = true)]
        internal static extern IntPtr SetWinEventHook(
            uint eventMin,
            uint eventMax,
            IntPtr module,
            WinEventDelegate callback,
            uint processId,
            uint threadId,
            uint flags);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnhookWinEvent(IntPtr hook);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool PostMessage(IntPtr window, int message, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll", SetLastError = true)]
        internal static extern UIntPtr SetTimer(IntPtr window, UIntPtr timerId, uint milliseconds, IntPtr callback);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool KillTimer(IntPtr window, UIntPtr timerId);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindow(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowVisible(IntPtr window);

        [DllImport("user32.dll")]
        internal static extern uint GetClipboardSequenceNumber();

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        internal static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll")]
        private static extern short GetAsyncKeyState(int virtualKey);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetWindowPos(
            IntPtr window,
            IntPtr insertAfter,
            int x,
            int y,
            int width,
            int height,
            uint flags);
        [DllImport("user32.dll", SetLastError = true)]
[return: MarshalAs(UnmanagedType.Bool)]
private static extern bool GetClientRect(
    IntPtr window,
    out NativeWindowRectangle bounds);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ClientToScreen(
            IntPtr window,
            ref Point point);

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
        private static extern IntPtr GetWindowLongPtr(IntPtr window, int index);

        [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
        private static extern IntPtr SetWindowLongPtr(IntPtr window, int index, IntPtr value);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetCursorPos(out Point point);

        [DllImport("user32.dll")]
        private static extern IntPtr WindowFromPoint(Point point);

        [DllImport("user32.dll")]
        private static extern IntPtr MonitorFromPoint(Point point, uint flags);

        [DllImport("user32.dll")]
        private static extern IntPtr MonitorFromRect(in NativeWindowRectangle rectangle, uint flags);

        [DllImport("shcore.dll")]
        private static extern int GetDpiForMonitor(
            IntPtr monitor,
            int dpiType,
            out uint dpiX,
            out uint dpiY);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool EnumDisplayMonitors(
            IntPtr deviceContext,
            IntPtr clipRect,
            MonitorEnumDelegate callback,
            IntPtr data);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, bool isAttached);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ShowWindow(IntPtr window, int command);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsIconic(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsZoomed(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);

        [DllImport("user32.dll")]
        private static extern IntPtr GetFocus();

        [DllImport("user32.dll")]
        private static extern IntPtr SetFocus(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

        internal static bool ActivateWindow(IntPtr target, IntPtr focusWindow)
        {
            if (IsIconic(target))
            {
                _ = ShowWindow(target, SwRestore);
            }
            var currentThread = GetCurrentThreadId();
            var targetThread = GetWindowThreadProcessId(target, out _);
            var foreground = GetForegroundWindow();
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out _);

            var attachedToTarget = targetThread != 0 && targetThread != currentThread
                && AttachThreadInput(currentThread, targetThread, true);
            var attachedToForeground = foregroundThread != 0
                && foregroundThread != currentThread
                && foregroundThread != targetThread
                && AttachThreadInput(currentThread, foregroundThread, true);
            var focusRestored = focusWindow == IntPtr.Zero;
            try
            {
                _ = BringWindowToTop(target);
                _ = SetForegroundWindow(target);
                if (focusWindow != IntPtr.Zero &&
                    IsWindow(focusWindow))
                {
                    if (GetFocus() != focusWindow)
                    {
                        _ = SetFocus(focusWindow);
                    }
                    focusRestored = GetFocus() == focusWindow;
                }
            }
            finally
            {
                if (attachedToForeground)
                {
                    _ = AttachThreadInput(currentThread, foregroundThread, false);
                }
                if (attachedToTarget)
                {
                    _ = AttachThreadInput(currentThread, targetThread, false);
                }
            }
            return IsForegroundWindowForTarget(target) && focusRestored;
        }

        internal static bool IsForegroundWindowForTarget(IntPtr target)
        {
            var foreground = GetForegroundWindow();
            if (foreground == target)
            {
                return true;
            }
            if (foreground == IntPtr.Zero || target == IntPtr.Zero)
            {
                return false;
            }
            var targetRoot = GetAncestor(target, GaRoot);
            var foregroundRoot = GetAncestor(foreground, GaRoot);
            return targetRoot != IntPtr.Zero && targetRoot == foregroundRoot;
        }

        internal static long GetRootWindowHandle(IntPtr window)
        {
            if (window == IntPtr.Zero)
            {
                return 0;
            }
            var root = GetAncestor(window, GaRoot);
            return (root == IntPtr.Zero ? window : root).ToInt64();
        }

        internal static WindowsCommandResult SetWindowOwner(
            IntPtr window,
            IntPtr owner)
        {
            if (window == IntPtr.Zero || owner == IntPtr.Zero ||
                !IsWindow(window) || !IsWindow(owner) || window == owner)
            {
                return new WindowsCommandResult(
                    false,
                    "The GPUI window or file-dialog owner is unavailable.");
            }
            Marshal.SetLastPInvokeError(0);
            var previous = SetWindowLongPtr(window, GwlHwndParent, owner);
            var error = Marshal.GetLastPInvokeError();
            if (previous == IntPtr.Zero && error != 0)
            {
                return new WindowsCommandResult(
                    false,
                    $"Windows could not attach the GPUI search window to its file dialog (error: {error}).");
            }
            return new WindowsCommandResult(true);
        }

        internal static PasteWindowSnapshot CaptureWindowState(IntPtr target)
        {
            var currentThread = GetCurrentThreadId();
            uint targetProcess = 0;
            var targetThread = target == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(target, out targetProcess);
            var foreground = GetForegroundWindow();
            uint foregroundProcess = 0;
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out foregroundProcess);
            return new PasteWindowSnapshot(
                foreground.ToInt64(),
                GetAncestor(foreground, GaRoot).ToInt64(),
                GetAncestor(target, GaRoot).ToInt64(),
                currentThread,
                targetThread,
                foregroundThread,
                targetProcess,
                foregroundProcess);
        }

        internal static ModifierReleaseResult ReleaseActivationModifiers()
        {
            ushort[] keys =
            [
                VkShift, VkControl, VkMenu,
                VkLShift, VkRShift, VkLControl, VkRControl, VkLMenu, VkRMenu,
                VkLWin, VkRWin
            ];
            var initiallyPressed = keys.Where(IsKeyDown).ToArray();
            var inputSteps = new List<KeyboardInputStep>();
            var attempts = 0;
            for (; attempts < 12; attempts++)
            {
                var pressed = keys.Where(IsKeyDown).ToArray();
                if (pressed.Length == 0)
                {
                    return new ModifierReleaseResult(true, initiallyPressed, [], attempts, inputSteps);
                }
                foreach (var key in pressed)
                {
                    inputSteps.Add(SendKeyboardInput("modifier_up", key, KeyeventfKeyup));
                }
                Thread.Sleep(3);
            }
            var finallyPressed = keys.Where(IsKeyDown).ToArray();
            return new ModifierReleaseResult(
                finallyPressed.Length == 0,
                initiallyPressed,
                finallyPressed,
                attempts,
                inputSteps);
        }

        internal static PasteShortcutResult SendPasteShortcut()
        {
            KeyboardInputStep[] steps =
            [
                SendKeyboardInput("ctrl_down", VkControl, 0),
                DelayThenSend(10, "v_down", VkV, 0),
                DelayThenSend(10, "v_up", VkV, KeyeventfKeyup),
                DelayThenSend(5, "ctrl_up", VkControl, KeyeventfKeyup)
            ];
            return new PasteShortcutResult(
                steps.Aggregate(0U, static (sent, step) => sent + step.Sent),
                Marshal.SizeOf<Input>(),
                steps);
        }

        private static KeyboardInputStep DelayThenSend(
            int milliseconds,
            string name,
            ushort key,
            uint flags)
        {
            Thread.Sleep(milliseconds);
            return SendKeyboardInput(name, key, flags);
        }

        internal static PostPasteResult PostPaste(IntPtr target)
        {
            if (target == IntPtr.Zero || !IsWindow(target))
            {
                return new PostPasteResult(false, 1400, "The paste target is no longer a valid window.");
            }
            Marshal.SetLastPInvokeError(0);
            if (PostMessage(target, WmPaste, IntPtr.Zero, IntPtr.Zero))
            {
                return new PostPasteResult(true, 0, null);
            }
            var error = Marshal.GetLastPInvokeError();
            return new PostPasteResult(
                false,
                error,
                $"PostMessage(WM_PASTE) failed with Win32={error}.");
        }

        internal static WindowsCommandResult SetWindowTopmost(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            var insertAfter = enabled ? new IntPtr(-1) : new IntPtr(-2);
            var lastError = 0;
            for (var attempt = 0; attempt < 3; attempt++)
            {
                if (!SetWindowPos(
                    window,
                    insertAfter,
                    0,
                    0,
                    0,
                    0,
                    SwpNoMove | SwpNoSize | SwpNoActivate))
                {
                    lastError = Marshal.GetLastWin32Error();
                }
                else if (IsTopmost(window) == enabled)
                {
                    return new WindowsCommandResult(true);
                }
                Thread.Sleep(10);
            }
            return new WindowsCommandResult(
                false,
                lastError == 0
                    ? "Windows did not apply the requested topmost state."
                    : $"SetWindowPos failed with Win32={lastError}.");
        }

        internal static WindowsCommandResult SetWindowNoActivate(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            var current = GetWindowLongPtr(window, GwlExStyle).ToInt64();
            var desired = enabled ? current | WsExNoActivate : current & ~WsExNoActivate;
            if (desired != current)
            {
                Marshal.SetLastPInvokeError(0);
                var previous = SetWindowLongPtr(window, GwlExStyle, new IntPtr(desired));
                var error = Marshal.GetLastPInvokeError();
                if (previous == IntPtr.Zero && error != 0)
                {
                    return new WindowsCommandResult(
                        false,
                        $"SetWindowLongPtr failed with Win32={error}.");
                }
                if (!SetWindowPos(
                        window,
                        IntPtr.Zero,
                        0,
                        0,
                        0,
                        0,
                        SwpNoMove | SwpNoSize | SwpNoZOrder | SwpNoActivate | SwpFrameChanged))
                {
                    return new WindowsCommandResult(
                        false,
                        $"SetWindowPos(SWP_FRAMECHANGED) failed with Win32={Marshal.GetLastWin32Error()}.");
                }
            }

            var applied = (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExNoActivate) != 0;
            return applied == enabled
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested no-activate state.");
        }
        internal static WindowsCommandResult SetWindowShadow(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            // DWMWA_NCRENDERING_POLICY = 2; DWMNCRP_DISABLED = 1 kills the
            // system drop shadow. The floating hint card reads cleaner flat.
            const int dwmwaNcRenderingPolicy = 2;
            int policy = enabled ? 2 : 1;
            var hr = DwmSetWindowAttribute(window, dwmwaNcRenderingPolicy, ref policy, sizeof(int));
            return hr == 0
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, $"DwmSetWindowAttribute failed HResult={hr}.");
        }

        internal static WindowsCommandResult SetWindowVisibility(IntPtr window, bool visible)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            _ = ShowWindow(window, visible ? SwShowNoActivate : SwHide);
            return IsWindowVisible(window) == visible
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested visibility state.");
        }

        internal static WindowsCommandResult SetWindowMinimized(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            _ = ShowWindow(window, SwShowMinNoActive);
            for (var attempt = 0; attempt < 3; attempt++)
            {
                if (IsWindowVisible(window) && IsIconic(window) && GetForegroundWindow() != window)
                {
                    return new WindowsCommandResult(true);
                }
                Thread.Sleep(10);
            }
            return new WindowsCommandResult(
                false,
                "Windows did not show the requested GPUI window minimized without activation.");
        }

        internal static WindowsCommandResult SetWindowMaximized(IntPtr window, bool maximized)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = ShowWindow(window, maximized ? SwMaximize : SwRestore);
            return IsZoomed(window) == maximized
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested maximized state.");
        }

        internal static WindowsCommandResult ToggleWindowMaximized(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            return SetWindowMaximized(window, !IsZoomed(window));
        }

        internal static NativeWindowPresentationState CaptureWindowPresentation(IntPtr window)
        {
            var exists = window != IntPtr.Zero && IsWindow(window);
            var bounds = default(NativeWindowRectangle);
            var clientBounds = default(NativeWindowRectangle);
            var cloaked = false;
            if (exists)
            {
                _ = AnchoredWindowPlacement.TryGetVisibleBounds(window, out bounds);
                _ = GetClientRect(window, out clientBounds);
                try
                {
                    cloaked = DwmGetWindowAttribute(
                        window,
                        DwmCloaked,
                        out var cloakedValue,
                        sizeof(int)) == 0 && cloakedValue != 0;
                }
                catch (DllNotFoundException)
                {
                }
                catch (EntryPointNotFoundException)
                {
                }
            }
            return new NativeWindowPresentationState(
                exists,
                exists && IsWindowVisible(window),
                exists && IsTopmost(window),
                cloaked,
                exists ? GetWindowLongPtr(window, GwlHwndParent).ToInt64() : 0,
                GetForegroundWindow().ToInt64(),
                bounds,
                clientBounds);
        }

        internal static WindowsCommandResult QueryWindowGeometry(IntPtr window, uint desktopProcessId)
        {
            if (desktopProcessId == 0 || window == IntPtr.Zero ||
                GetWindowThreadProcessId(window, out var ownerProcessId) == 0 || ownerProcessId != desktopProcessId)
                return new WindowsCommandResult(false, "window_geometry_invalid_peer");

            // GetWindowRect includes the real outer frame. The DWM visible rectangle
            // used by CaptureWindowPresentation excludes invisible borders.
            if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer) ||
                !TryGetClientScreenRectangle(window, out var client))
                return new WindowsCommandResult(false, "window_geometry_unavailable",
                    NativeErrorCode: Marshal.GetLastWin32Error());

            const uint MonitorDefaultToNull = 0;
            var monitor = MonitorFromRect(in outer, MonitorDefaultToNull);
            return WindowGeometryResult(outer, client, unchecked((ulong)monitor.ToInt64()),
                GetDpiForWindow(window), IsWindowVisible(window));
        }

        internal static WindowsCommandResult SetWindowBounds(
            IntPtr window,
            int x,
            int y,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }

            // Every GPUI caller supplies a client/content size in device pixels.
            // Keep the existing outer x/y placement contract, but compensate the
            // actual frame rather than shrinking content by the native border.
            if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer) ||
                !TryGetClientScreenRectangle(window, out var client))
                return new WindowsCommandResult(false, "The GPUI client rectangle is unavailable.");
            var insets = NativeWindowInsets.From(outer, client);
            var outerWidth = (long)width + insets.Left + insets.Right;
            var outerHeight = (long)height + insets.Top + insets.Bottom;
            if (outerWidth <= 0 || outerHeight <= 0 || outerWidth > int.MaxValue || outerHeight > int.MaxValue)
                return new WindowsCommandResult(false, "Requested client bounds exceed the native size range.");
            var applied = SetWindowPos(window, IntPtr.Zero, x, y,
                (int)outerWidth, (int)outerHeight,
                SwpNoZOrder | SwpNoActivate | SwpFrameChanged);
            if (!applied)
            {
                var nativeError = Marshal.GetLastWin32Error();
                return new WindowsCommandResult(false, "Native client bounds application failed.", NativeErrorCode: nativeError);
            }
            return TryGetClientScreenRectangle(window, out var after) &&
                after.Width == width && after.Height == height
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested GPUI client bounds.");
        }

        /// <summary>
        /// Resizes the window while preserving its real current visual center.
        /// GPUI sends client/content pixels, so the current outer/client frame
        /// is used to derive the SetWindowPos rectangle.
        /// </summary>
        internal static WindowsCommandResult SetWindowBoundsCentered(
            IntPtr window,
            int width,
            int height,
            Guid correlationId = default,
            int recoveryAttempt = 0,
            double? anchorCenterX = null,
            double? anchorCenterY = null)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var currentVisible) ||
                !AnchoredWindowPlacement.TryGetWindowRectangle(window, out var currentOuter) ||
                !TryGetClientScreenRectangle(window, out var currentClient))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            var clientInsets = NativeWindowInsets.From(currentOuter, currentClient);
            var centeredClient = anchorCenterX.HasValue && anchorCenterY.HasValue
                ? CenteredRectangleAtCenter(anchorCenterX.Value, anchorCenterY.Value, width, height)
                : CenteredRectangle(currentVisible, width, height);
            var targetOuter = (
                centeredClient.Left - clientInsets.Left,
                centeredClient.Top - clientInsets.Top,
                width + clientInsets.Left + clientInsets.Right,
                height + clientInsets.Top + clientInsets.Bottom);
            var appliedOk = SetWindowPos(
                window,
                IntPtr.Zero,
                targetOuter.Item1,
                targetOuter.Item2,
                targetOuter.Item3,
                targetOuter.Item4,
                SwpNoZOrder | SwpNoActivate);
            var nativeError = appliedOk ? (int?)null : Marshal.GetLastWin32Error();
            return appliedOk ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, $"SetWindowPos(centered) failed with Win32={nativeError}.",
                    NativeErrorCode: nativeError);
        }

        internal static CenteredResizeReplayResult ReapplyPendingCenteredResizeFor(
            IntPtr rootWindow,
            Func<CenteredResizeRecoverySnapshot, NativeWindowPresentationState, WindowsCommandResult>? finalizeClientResize = null,
            Action? beforeApply = null,
            Action? beforeCommit = null)
        {
            if (!PendingCenteredResizes.TryGetValue(rootWindow, out var pending)) return default;
            var snapshot = Snapshot(rootWindow, pending);
            beforeApply?.Invoke();
            var attemptCount = pending.AttemptCount + 1;
            var result = new WindowsCommandResult(false, "The resize request was superseded.");
            var clientMatches = false;
            var stable = false;
            if (IsCurrentCenteredResize(rootWindow, snapshot.Generation) && IsWindow(pending.Window))
            {
                // Capture the live moved center at the execution boundary, not at acceptance.
                var before = CaptureWindowPresentation(pending.Window);
                if (!IsDraggingOrSizing(pending.Window))
                {
                    result = SetWindowBoundsCentered(pending.Window, pending.Width, pending.Height,
                        pending.CorrelationId, attemptCount);
                    if (result.Success && IsCurrentCenteredResize(rootWindow, snapshot.Generation) && finalizeClientResize is not null)
                        result = finalizeClientResize(snapshot, before);
                }
            }
            beforeCommit?.Invoke();
            // A position/size message may change the client after application.
            // Read at retirement, not before the commit boundary.
            var final = CaptureWindowPresentation(pending.Window);
            clientMatches = final.Exists && final.ClientBounds.Width == pending.Width &&
                final.ClientBounds.Height == pending.Height;
            stable = result.Success && clientMatches && !IsDraggingOrSizing(pending.Window) && attemptCount >= 2;
            var committed = stable || !IsWindow(pending.Window)
                ? ((ICollection<KeyValuePair<IntPtr, PendingCenteredResize>>)PendingCenteredResizes)
                    .Remove(new(rootWindow, pending))
                : PendingCenteredResizes.TryUpdate(rootWindow, pending with { AttemptCount = attemptCount }, pending);
            if (!committed) return default; // Superseded completion has no terminal authority.
            var stillPending = PendingCenteredResizes.TryGetValue(rootWindow, out var currentPending) && currentPending.Generation == pending.Generation;
            return new CenteredResizeReplayResult(true, pending.CorrelationId, pending.Window.ToInt64(),
                pending.StartedAtTimestamp, attemptCount, result.Success && clientMatches, stillPending, result.NativeErrorCode)
                {
                    Generation = pending.Generation, RequestedWidth = pending.Width, RequestedHeight = pending.Height,
                    ReadbackWidth = final.Exists ? final.ClientBounds.Width : 0,
                    ReadbackHeight = final.Exists ? final.ClientBounds.Height : 0
                };
        }

        internal static bool IsCurrentCenteredResize(IntPtr rootWindow, long generation) =>
            PendingCenteredResizes.TryGetValue(rootWindow, out var pending) && pending.Generation == generation;

        private static CenteredResizeRecoverySnapshot Snapshot(IntPtr rootWindow, PendingCenteredResize pending) =>
            new(rootWindow, pending.CorrelationId, pending.Window.ToInt64(), pending.StartedAtTimestamp, pending.AttemptCount)
            { Width = pending.Width, Height = pending.Height, Generation = pending.Generation };

        internal static bool TryGetPendingCenteredResizeRoot(
            IntPtr window,
            out IntPtr rootWindow)
        {
            rootWindow = window == IntPtr.Zero ? IntPtr.Zero : GetAncestorOrSelf(window);
            return rootWindow != IntPtr.Zero &&
                PendingCenteredResizes.ContainsKey(rootWindow);
        }

        internal static bool TryGetPendingCenteredResize(
            IntPtr rootWindow,
            out CenteredResizeRecoverySnapshot snapshot)
        {
            if (PendingCenteredResizes.TryGetValue(rootWindow, out var pending))
            {
                snapshot = Snapshot(rootWindow, pending);
                return true;
            }
            snapshot = default;
            return false;
        }

        internal static void ParkCenteredResizeForRecovery(
            IntPtr window, int width, int height, Guid correlationId)
        {
            // Corrections never create a request or replace a newer generation.
            // The accepted request remains pending until final readback retires it.
        }

        internal static CenteredResizeRecoverySnapshot BeginCenteredResize(
            IntPtr window, int width, int height, Guid correlationId)
        {
            var rootWindow = GetAncestorOrSelf(window);
            var pending = new PendingCenteredResize(window, width, height, correlationId,
                Stopwatch.GetTimestamp(), 0, Interlocked.Increment(ref CenteredResizeGeneration));
            PendingCenteredResizes.AddOrUpdate(rootWindow, pending,
                (_, current) => current.Generation > pending.Generation ? current : pending);
            return Snapshot(rootWindow, pending);
        }

        internal static WindowsCommandResult ParkCenteredResizeForTest(
            IntPtr window,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window) || width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "The window or size is invalid.");
            }
            _ = BeginCenteredResize(window, width, height, Guid.Empty);
            return new WindowsCommandResult(true);
        }

        internal static void CancelPendingCenteredResize(IntPtr rootWindow, Guid correlationId, long generation = 0)
        {
            if (!PendingCenteredResizes.TryGetValue(rootWindow, out var pending) ||
                pending.CorrelationId != correlationId || (generation != 0 && pending.Generation != generation)) return;
            _ = ((ICollection<KeyValuePair<IntPtr, PendingCenteredResize>>)PendingCenteredResizes)
                .Remove(new(rootWindow, pending));
        }

        private sealed record PendingCenteredResize(
            IntPtr Window,
            int Width,
            int Height,
            Guid CorrelationId,
            long StartedAtTimestamp,
            int AttemptCount,
            long Generation);

        private static long CenteredResizeGeneration;

        private static readonly System.Collections.Concurrent.ConcurrentDictionary<
            IntPtr,
            PendingCenteredResize> PendingCenteredResizes = new();

        private static bool TryGetClientScreenRectangle(
            IntPtr window,
            out NativeWindowRectangle bounds)
        {
            bounds = default;
            if (!GetClientRect(window, out var client))
            {
                return false;
            }
            var topLeft = new Point { X = client.Left, Y = client.Top };
            var bottomRight = new Point { X = client.Right, Y = client.Bottom };
            if (!ClientToScreen(window, ref topLeft) ||
                !ClientToScreen(window, ref bottomRight))
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

        private static bool AppliedSizeTookEffect(
            IntPtr window,
            int x,
            int y,
            int width,
            int height)
        {
            if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var rect))
            {
                return false;
            }
            return Math.Abs(rect.Left - x) <= 2
                && Math.Abs(rect.Top - y) <= 2
                && Math.Abs(rect.Right - rect.Left - width) <= 2
                && Math.Abs(rect.Bottom - rect.Top - height) <= 2;
        }
        internal static IntPtr GetAncestorOrSelf(IntPtr window)
        {
            var root = GetAncestor(window, 2 /* GA_ROOT */);
            return root == IntPtr.Zero ? window : root;
        }

        internal static bool IsDraggingOrSizing(IntPtr window)
        {
            var threadId = GetWindowThreadProcessId(window, IntPtr.Zero);
            var info = new GUITHREADINFO { cbSize = (uint)Marshal.SizeOf<GUITHREADINFO>() };
            return GetGUIThreadInfo(threadId, ref info)
                && (info.flags & GUI_INMOVESIZE) != 0;
        }


        /// <summary>
        /// Applies one fixed client size to a newly created HWND, centers it over
        /// the live predecessor, and shows it without activation. No recovery,
        /// dimension polling or resize of the predecessor is performed.
        /// </summary>
        [DllImport("user32.dll")]
        private static extern uint GetDpiForWindow(IntPtr window);

        internal static WindowsCommandResult PresentFixedWindowOverAnchor(
            IntPtr window, IntPtr anchor, int width, int height, CancellationToken cancellationToken)
        {
            if (cancellationToken.IsCancellationRequested)
                return new WindowsCommandResult(false, "Fixed presentation was cancelled.");
            if (window == IntPtr.Zero || anchor == IntPtr.Zero || window == anchor ||
                !IsWindow(window) || !IsWindow(anchor) || width <= 0 || height <= 0 ||
                !AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer) ||
                !TryGetClientScreenRectangle(window, out var client) ||
                !AnchoredWindowPlacement.TryGetVisibleBounds(anchor, out var anchorBounds))
                return new WindowsCommandResult(false, "Fixed presentation window or anchor is unavailable.");
            // Hidden GPUI creation may still be on the default monitor. Prepare
            // the anchor monitor without sizing, then use that monitor's chrome.
            if (GetDpiForWindow(window) != GetDpiForWindow(anchor))
            {
                if (!SetWindowPos(window, IntPtr.Zero, anchorBounds.Left, anchorBounds.Top, 0, 0,
                    SwpNoSize | SwpNoZOrder | SwpNoActivate))
                    return new WindowsCommandResult(false, "Fixed presentation monitor preparation failed.", NativeErrorCode: Marshal.GetLastWin32Error());
                if (cancellationToken.IsCancellationRequested || !IsWindow(anchor) ||
                    !AnchoredWindowPlacement.TryGetWindowRectangle(window, out outer) ||
                    !TryGetClientScreenRectangle(window, out client))
                    return new WindowsCommandResult(false, "Fixed presentation monitor preparation retired.");
            }
            var insets = NativeWindowInsets.From(outer, client);
            var centerX = (anchorBounds.Left + anchorBounds.Right) / 2.0;
            var centerY = (anchorBounds.Top + anchorBounds.Bottom) / 2.0;
            var x = (int)Math.Round(centerX - width / 2.0) - insets.Left;
            var y = (int)Math.Round(centerY - height / 2.0) - insets.Top;
            if (!SetWindowPos(window, IntPtr.Zero, x, y,
                width + insets.Left + insets.Right, height + insets.Top + insets.Bottom,
                SwpNoZOrder | SwpNoActivate | SwpFrameChanged))
                return new WindowsCommandResult(false, "Fixed window placement failed.", NativeErrorCode: Marshal.GetLastWin32Error());
            if (cancellationToken.IsCancellationRequested || !IsWindow(anchor))
                return new WindowsCommandResult(false, "Fixed presentation anchor retired.");
            _ = ShowWindow(window, SwShowNoActivate);
            var final = CaptureWindowPresentation(window);
            var centered = Math.Abs((final.Bounds.Left + final.Bounds.Right) / 2.0 - centerX) <= 2 &&
                Math.Abs((final.Bounds.Top + final.Bounds.Bottom) / 2.0 - centerY) <= 2;
            if (cancellationToken.IsCancellationRequested || !IsWindow(anchor) || !final.Exists || !final.Visible ||
                final.ClientBounds.Width != width || final.ClientBounds.Height != height || !centered)
            {
                _ = ShowWindow(window, SwHide);
                return new WindowsCommandResult(false, "Fixed presentation did not match its final bounds.");
            }
            return new WindowsCommandResult(true);
        }

        internal static WindowsCommandResult MoveWindowCenteredOver(
            IntPtr window,
            IntPtr anchorWindow,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var target))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            NativeWindowRectangle anchorRect = default;
            var anchorAvailable = anchorWindow != IntPtr.Zero &&
                IsWindow(anchorWindow) &&
                AnchoredWindowPlacement.TryGetVisibleBounds(anchorWindow, out anchorRect);
            var centered = anchorAvailable
                ? CenteredRectangle(anchorRect, width, height)
                : CenteredRectangle(target, width, height);
            var applied = ToOuterRectangle(
                window,
                target,
                centered,
                width,
                height,
                out var failure);
            if (applied == null)
            {
                return failure ?? new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }
            var (x, y, outerWidth, outerHeight) = applied.Value;
            return SetWindowPos(
                window,
                IntPtr.Zero,
                x,
                y,
                outerWidth,
                outerHeight,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(centered-over) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        /// <summary>
        /// Converts a desired VISIBLE rectangle into the OUTER rectangle
        /// SetWindowPos applies. DWM shadows make the outer frame larger than
        /// the visible bounds; ignoring the insets displaced every phase
        /// transition by the shadow border and shrank the client area.
        /// </summary>
        private static (int Left, int Top, int Width, int Height)? ToOuterRectangle(
            IntPtr window,
            NativeWindowRectangle currentVisible,
            NativeWindowRectangle centeredVisible,
            int visibleWidth,
            int visibleHeight,
            out WindowsCommandResult? failure)
        {
            failure = null;
            if (!AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer))
            {
                failure = new WindowsCommandResult(false, "The window rectangle is unavailable.");
                return null;
            }
            var insets = NativeWindowInsets.From(outer, currentVisible);
            return (
                centeredVisible.Left - insets.Left,
                centeredVisible.Top - insets.Top,
                visibleWidth + insets.Left + insets.Right,
                visibleHeight + insets.Top + insets.Bottom);
        }

        internal static NativeWindowRectangle CenteredRectangleAtCenter(
            double centerX,
            double centerY,
            int width,
            int height)
        {
            var left = (int)Math.Round(centerX - width / 2.0);
            var top = (int)Math.Round(centerY - height / 2.0);
            return new NativeWindowRectangle(left, top, left + width, top + height);
        }

        internal static NativeWindowRectangle CenteredRectangle(
            NativeWindowRectangle anchor,
            int width,
            int height)
        {
            var centerX = anchor.Left + (anchor.Right - anchor.Left) / 2.0;
            var centerY = anchor.Top + (anchor.Bottom - anchor.Top) / 2.0;
            var left = (int)Math.Round(centerX - width / 2.0);
            var top = (int)Math.Round(centerY - height / 2.0);
            return new NativeWindowRectangle(left, top, left + width, top + height);
        }

        private const int WindowCenterStashCapacity = 16;
        private static readonly ConcurrentDictionary<string, NativeWindowRectangle>
            WindowCenterStash = new(StringComparer.Ordinal);

        /// <summary>
        /// Captures a window's real visible rectangle under a correlation key so
        /// a replacement window can later inherit that position after this
        /// window is destroyed. GPUI's cached origin never follows user drags,
        /// which is why the geometry is captured here, natively.
        /// </summary>
        internal static WindowsCommandResult StashWindowBounds(IntPtr window, string key)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (string.IsNullOrWhiteSpace(key))
            {
                return new WindowsCommandResult(false, "The window stash key is empty.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var bounds))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            if (WindowCenterStash.Count >= WindowCenterStashCapacity)
            {
                WindowCenterStash.Clear();
            }
            WindowCenterStash[key] = bounds;
            return new WindowsCommandResult(true);
        }

        /// <summary>
        /// Places the window centered over a stashed anchor rectangle and
        /// consumes it. A missing stash entry falls back to an in-place resize
        /// at the current position so the placement never teleports the window.
        /// </summary>
        internal static WindowsCommandResult MoveWindowCenteredOverKey(
            IntPtr window,
            string key,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var target))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            NativeWindowRectangle anchorRect = default;
            var stashHit = !string.IsNullOrWhiteSpace(key) &&
                WindowCenterStash.TryRemove(key, out anchorRect);
            var centered = stashHit
                ? CenteredRectangle(anchorRect, width, height)
                : CenteredRectangle(target, width, height);
            var applied = ToOuterRectangle(
                window,
                target,
                centered,
                width,
                height,
                out var stashFailure);
            if (applied == null)
            {
                return stashFailure ?? new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }
            var (stashX, stashY, stashOuterWidth, stashOuterHeight) = applied.Value;
            return SetWindowPos(
                window,
                IntPtr.Zero,
                stashX,
                stashY,
                stashOuterWidth,
                stashOuterHeight,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(stash-centered) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult SetWindowPosition(IntPtr window, int x, int y)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            return SetWindowPos(
                window,
                IntPtr.Zero,
                x,
                y,
                0,
                0,
                SwpNoSize | SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, $"SetWindowPos failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult BeginWindowResize(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = PostMessage(window, WmCancelMode, IntPtr.Zero, IntPtr.Zero);
            var resizeCommand = new IntPtr(BottomRightResizeSystemCommand);
            return PostMessage(window, WmSysCommand, resizeCommand, IntPtr.Zero)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"PostMessage(WM_SYSCOMMAND/SC_SIZE) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult BeginWindowMove(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = PostMessage(window, WmCancelMode, IntPtr.Zero, IntPtr.Zero);
            return PostMessage(window, WmSysCommand, new IntPtr(MoveCaptionSystemCommand), IntPtr.Zero)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"PostMessage(WM_SYSCOMMAND/SC_MOVE) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        private static bool IsTopmost(IntPtr window) =>
            (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExTopmost) != 0;

        internal static CursorLocation GetCursorLocation()
        {
            var position = GetCursorPos(out var point) ? point : default;
            var monitor = MonitorFromPoint(position, MonitorDefaultToNearest);
            var dpi = DefaultDpi;
            if (monitor != IntPtr.Zero &&
                GetDpiForMonitor(monitor, MonitorDpiTypeEffective, out var dpiX, out _) == 0 &&
                dpiX > 0)
            {
                dpi = dpiX;
            }
            CursorMonitorSnapshot? snapshot = null;
            var info = new NotificationMonitorInfo { Size = Marshal.SizeOf<NotificationMonitorInfo>() };
            if (monitor != IntPtr.Zero && NotificationGetMonitorInfo(monitor, ref info) &&
                info.Work.Width > 0 && info.Work.Height > 0)
            {
                snapshot = new CursorMonitorSnapshot(unchecked((ulong)monitor.ToInt64()),
                    info.Work.Left, info.Work.Top, (uint)info.Work.Width, (uint)info.Work.Height, dpi);
            }
            return new CursorLocation(position, GetDisplayIndex(monitor), dpi, snapshot);
        }

        internal static bool IsPointerButtonDown(int virtualKey) =>
            (GetAsyncKeyState(virtualKey) & 0x8000) != 0;

        internal static PointerTarget GetPointerTarget()
        {
            if (!GetCursorPos(out var point))
            {
                return default;
            }
            var window = WindowFromPoint(point);
            var root = window == IntPtr.Zero ? IntPtr.Zero : GetAncestor(window, GaRoot);
            _ = GetWindowThreadProcessId(root != IntPtr.Zero ? root : window, out var processId);
            return new PointerTarget(window.ToInt64(), root.ToInt64(), processId);
        }

        private static uint GetDisplayIndex(IntPtr targetMonitor)
        {
            uint current = 0;
            uint? result = null;
            _ = EnumDisplayMonitors(
                IntPtr.Zero,
                IntPtr.Zero,
                (monitor, _, _, _) =>
                {
                    if (monitor == targetMonitor)
                    {
                        result = current;
                        return false;
                    }
                    current++;
                    return true;
                },
                IntPtr.Zero);
            return result ?? 0;
        }

        private static Input KeyboardInput(ushort key, uint flags) => new()
        {
            Type = InputKeyboard,
            Union = new InputUnion
            {
                Keyboard = new KeyboardInputData
                {
                    VirtualKey = key,
                    Flags = flags
                }
            }
        };

        private static bool IsKeyDown(ushort key) => (GetAsyncKeyState(key) & 0x8000) != 0;

        private static KeyboardInputStep SendKeyboardInput(string name, ushort key, uint flags)
        {
            var input = new[] { KeyboardInput(key, flags) };
            Marshal.SetLastPInvokeError(0);
            var sent = SendInput(1, input, Marshal.SizeOf<Input>());
            var error = sent == 0 ? Marshal.GetLastPInvokeError() : 0;
            return new KeyboardInputStep(name, key, flags, sent, error);
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Point
        {
            public int X;
            public int Y;
        }

        internal readonly record struct CursorLocation(Point Position, uint DisplayIndex, uint Dpi,
            CursorMonitorSnapshot? Monitor = null);

        internal readonly record struct PointerTarget(
            long WindowHandle,
            long RootWindowHandle,
            uint ProcessId);

        private delegate bool MonitorEnumDelegate(
            IntPtr monitor,
            IntPtr deviceContext,
            IntPtr monitorRect,
            IntPtr data);

        [StructLayout(LayoutKind.Sequential)]
        private struct Input
        {
            public uint Type;
            public InputUnion Union;
        }

        [StructLayout(LayoutKind.Explicit)]
        private struct InputUnion
        {
            [FieldOffset(0)]
            public KeyboardInputData Keyboard;

            [FieldOffset(0)]
            public MouseInputData Mouse;

            [FieldOffset(0)]
            public HardwareInputData Hardware;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct MouseInputData
        {
            public int X;
            public int Y;
            public uint MouseData;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct HardwareInputData
        {
            public uint Message;
            public ushort ParameterLow;
            public ushort ParameterHigh;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct KeyboardInputData
        {
            public ushort VirtualKey;
            public ushort ScanCode;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }
    }
}
