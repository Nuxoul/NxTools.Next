using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Windows.Forms;

using NxTools.PlatformHost.Diagnostics;
namespace NxTools.PlatformHost.Windows;

internal enum FloatingAssistantInputKind { LeftClick, RightClick, MoveCompleted, HoverEnter, HoverLeave, Wheel }
internal sealed record FloatingAssistantInputEvent(FloatingAssistantInputKind Kind, int X = 0, int Y = 0, int Delta = 0);

// Input only: WS_EX_NOREDIRECTIONBITMAP has no DWM backing surface. GPUI remains
// the sole renderer. This object and its HWND/OLE registration belong to one STA.
internal sealed class FloatingAssistantInputSurface : NativeWindow, IDisposable
{
    private readonly IntPtr _renderer;
    private readonly int _ownerThread = Environment.CurrentManagedThreadId;
    private bool _disposed;
    private readonly FloatingAssistantDropTarget _dropTarget;
    private bool _interactive;
    private bool _oleInitialized;
    private bool _registered;
    private readonly Action<FloatingAssistantInputEvent> _input;
    private int _pressedButton;
    private Native.Point _pressPoint;
    private Native.Rectangle _pressBounds;
    private long _pressedAt;
    private bool _moved;
    private bool _hovered;

    internal FloatingAssistantInputSurface(IntPtr renderer, uint peerProcessId,
        Func<FloatingAssistantDropData, bool>? receive = null, Action<FloatingAssistantInputEvent>? input = null,
        IPlatformLog? log = null)
    {
        _input = input ?? (_ => { });
        if (Thread.CurrentThread.GetApartmentState() != ApartmentState.STA)
            throw new InvalidOperationException("Floating input requires an STA owner.");
        _ = Native.GetWindowThreadProcessId(renderer, out var actualProcess);
        if (renderer == IntPtr.Zero || !Native.IsWindow(renderer) || peerProcessId == 0 || actualProcess != peerProcessId ||
            !Native.ProcessIdToSessionId(peerProcessId, out var peerSession) ||
            !Native.ProcessIdToSessionId((uint)Environment.ProcessId, out var ownSession) || peerSession != ownSession)
            throw new InvalidDataException("Invalid floating renderer identity.");
        _renderer = renderer;
        _dropTarget = new FloatingAssistantDropTarget(
            () => !_disposed && _interactive && Native.IsWindow(_renderer), receive ?? (_ => false), log);
        var initialize = Native.OleInitialize(IntPtr.Zero);
        if (initialize < 0) throw new COMException("floating_ole_initialize", initialize);
        _oleInitialized = true;
        try
        {
        CreateHandle(new CreateParams
        {
            Caption = "NxTools floating input",
            Style = unchecked((int)0x80000000), // WS_POPUP, hidden
            ExStyle = 0x00200000 | 0x08000000 | 0x00000080, // NOREDIRECTIONBITMAP, NOACTIVATE, TOOLWINDOW
            Parent = renderer, // Owner for a WS_POPUP; not a cross-process SetParent.
            X = 0, Y = 0, Width = 1, Height = 1,
        });
            var registered = Native.RegisterDragDrop(Handle, _dropTarget);
            if (registered < 0) throw new COMException("floating_drop_registration", registered);
            _registered = true;
        }
        catch
        {
            if (Handle != IntPtr.Zero) DestroyHandle();
            Native.OleUninitialize();
            _oleInitialized = false;
            throw;
        }
    }

    internal bool PositionLocked { get; set; }

    internal bool TopmostLost => _interactive && Native.IsWindowVisible(Handle) &&
        (Native.GetWindowLongPtrW(Handle, -20).ToInt64() & 8) == 0;

    internal void RepairTopmost()
    {
        VerifyOwner();
        if (!_interactive || !Native.IsWindowVisible(Handle)) return;
        if (!Native.SetWindowPos(Handle, new IntPtr(-1), 0, 0, 0, 0, 0x0013))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        if (TopmostLost) throw new InvalidOperationException("Floating input topmost did not settle.");
    }

    internal void SetInteraction(bool visible, bool clickThrough)
    {
        VerifyOwner();
        if (!visible || clickThrough)
        {
            _interactive = false;
            _hovered = false;
            CancelGesture();
            _dropTarget.Cancel();
            _ = Native.ShowWindow(Handle, 0);
            if (Native.IsWindowVisible(Handle)) throw new InvalidOperationException("Floating input did not hide.");
            return;
        }
        if (!Native.IsWindow(_renderer) || !Native.GetWindowRect(_renderer, out var bounds))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        var region = Native.CreateEllipticRgn(0, 0, bounds.Right - bounds.Left, bounds.Bottom - bounds.Top);
        if (region == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        if (Native.SetWindowRgn(Handle, region, false) == 0)
        {
            _ = Native.DeleteObject(region);
            throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        if (!Native.SetWindowPos(Handle, new IntPtr(-1), bounds.Left, bounds.Top,
                bounds.Right - bounds.Left, bounds.Bottom - bounds.Top, 0x0010 | 0x0040))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        if (!Native.IsWindowVisible(Handle) || (Native.GetWindowLongPtrW(Handle, -20).ToInt64() & 8) == 0 ||
            !Native.GetWindowRect(Handle, out var actual) || actual != bounds)
            throw new InvalidOperationException("Floating input bounds did not settle.");
        _interactive = true;
        TrackHover();
    }

    private void VerifyOwner()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        if (Environment.CurrentManagedThreadId != _ownerThread)
            throw new InvalidOperationException("Floating input must run on its STA owner.");
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg == 0x0021) { message.Result = new IntPtr(3); return; } // MA_NOACTIVATE
        if (message.Msg == 0x0014) { message.Result = new IntPtr(1); return; } // no background erase
        if (message.Msg == 0x02A3 && _hovered) // WM_MOUSELEAVE
        {
            _hovered = false;
            if (_interactive) _input(new(FloatingAssistantInputKind.HoverLeave));
            return;
        }
        if (_interactive && message.Msg == 0x020A) // WM_MOUSEWHEEL
        {
            _input(new(FloatingAssistantInputKind.Wheel, Delta: unchecked((short)(message.WParam.ToInt64() >> 16))));
            return;
        }
        if (_interactive && message.Msg == 0x0200 && !_hovered)
        {
            _hovered = true;
            TrackHover();
            _input(new(FloatingAssistantInputKind.HoverEnter));
        }
        if (message.Msg == 0x0215 && _pressedButton != 0) // WM_CAPTURECHANGED
        {
            var moved = _interactive && _moved && _pressedButton == 0x0201 && !PositionLocked;
            // Capture was lost externally; cancel first so a later button-up cannot click.
            _pressedButton = 0;
            _moved = false;
            if (moved && Native.GetWindowRect(_renderer, out var settled))
                _input(new(FloatingAssistantInputKind.MoveCompleted, settled.Left, settled.Top));
        }
        if (_interactive && (message.Msg is 0x0201 or 0x0204))
        {
            CancelGesture();
            _pressPoint = ScreenPoint(message.LParam);
            if (!Native.GetWindowRect(_renderer, out _pressBounds)) return;
            _pressedButton = message.Msg;
            _pressedAt = System.Diagnostics.Stopwatch.GetTimestamp();
            _moved = false;
            _ = Native.SetCapture(Handle);
            return;
        }
        if (_interactive && _pressedButton != 0 && message.Msg == 0x0200)
        {
            var point = ScreenPoint(message.LParam);
            var dx = point.X - _pressPoint.X;
            var dy = point.Y - _pressPoint.Y;
            var dpi = Math.Max(96, Native.GetDpiForWindow(Handle));
            _moved |= Math.Abs((long)dx) + Math.Abs((long)dy) >= 6L * dpi / 96;
            if (_moved && _pressedButton == 0x0201 && !PositionLocked)
            {
                var x = checked(_pressBounds.Left + dx);
                var y = checked(_pressBounds.Top + dy);
                // Sole native move owner; terminal position is the only IPC-worthy event.
                if (!Native.SetWindowPos(_renderer, IntPtr.Zero, x, y, 0, 0, 0x0015) ||
                    !Native.SetWindowPos(Handle, IntPtr.Zero, x, y, 0, 0, 0x0015))
                    CancelGesture();
            }
            return;
        }
        if (_interactive && (message.Msg is 0x0202 or 0x0205) && _pressedButton != 0)
        {
            var button = _pressedButton;
            if (message.Msg != (button == 0x0201 ? 0x0202 : 0x0205)) return;
            var moved = _moved;
            var elapsed = System.Diagnostics.Stopwatch.GetElapsedTime(_pressedAt);
            CancelGesture();
            if (moved && button == 0x0201 && !PositionLocked && Native.GetWindowRect(_renderer, out var bounds))
                _input(new(FloatingAssistantInputKind.MoveCompleted, bounds.Left, bounds.Top));
            else if (!moved && elapsed < TimeSpan.FromMilliseconds(450))
                _input(new(button == 0x0201 ? FloatingAssistantInputKind.LeftClick : FloatingAssistantInputKind.RightClick));
            return;
        }
        base.WndProc(ref message);
    }

    private Native.Point ScreenPoint(IntPtr packed)
    {
        var point = new Native.Point(unchecked((short)packed.ToInt64()), unchecked((short)(packed.ToInt64() >> 16)));
        if (!Native.ClientToScreen(Handle, ref point)) throw new Win32Exception(Marshal.GetLastWin32Error());
        return point;
    }

    private void TrackHover()
    {
        var tracking = new Native.TrackMouse { Size = Marshal.SizeOf<Native.TrackMouse>(), Flags = 2, Window = Handle };
        if (_interactive) _ = Native.TrackMouseEvent(ref tracking);
    }

    private void CancelGesture()
    {
        _pressedButton = 0;
        _moved = false;
        if (Native.GetCapture() == Handle) _ = Native.ReleaseCapture();
    }

    public void Dispose()
    {
        if (_disposed) return;
        VerifyOwner();
        _interactive = false;
        CancelGesture();
        _dropTarget.Cancel();
        if (_registered) { _ = Native.RevokeDragDrop(Handle); _registered = false; }
        _ = Native.ShowWindow(Handle, 0);
        DestroyHandle();
        _disposed = true;
        if (_oleInitialized) { Native.OleUninitialize(); _oleInitialized = false; }
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal readonly record struct Rectangle(int Left, int Top, int Right, int Bottom);
        [StructLayout(LayoutKind.Sequential)] internal record struct Point(int X, int Y);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr SetCapture(IntPtr window);
        [StructLayout(LayoutKind.Sequential)] internal struct TrackMouse
        { internal int Size; internal uint Flags; internal IntPtr Window; internal uint HoverTime; }
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool TrackMouseEvent(ref TrackMouse tracking);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetCapture();
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ReleaseCapture();
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ClientToScreen(IntPtr window, ref Point point);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern int GetDpiForWindow(IntPtr window);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int OleInitialize(IntPtr reserved);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern void OleUninitialize();
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int RegisterDragDrop(IntPtr window, IFloatingAssistantDropTarget target);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern int RevokeDragDrop(IntPtr window);
        [DllImport("gdi32.dll", ExactSpelling = true, SetLastError = true)] internal static extern IntPtr CreateEllipticRgn(int left, int top, int right, int bottom);
        [DllImport("gdi32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool DeleteObject(IntPtr value);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] internal static extern int SetWindowRgn(IntPtr window, IntPtr region, [MarshalAs(UnmanagedType.Bool)] bool redraw);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsWindow(IntPtr window);
        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)] internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
        [DllImport("kernel32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ProcessIdToSessionId(uint processId, out uint sessionId);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetWindowRect(IntPtr window, out Rectangle bounds);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ShowWindow(IntPtr window, int show);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    }
}
