using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class ElevatedInputHost
{
    private bool _primaryPointerDown;
    private bool _secondaryPointerDown;
    private long _lastPointerFailure;

    private void MonitorPointerPress()
    {
        try
        {
            var primary = _native.IsPointerButtonDown(1);
            var secondary = _native.IsPointerButtonDown(2);
            var pressed = (primary && !_primaryPointerDown) || (secondary && !_secondaryPointerDown);
            _primaryPointerDown = primary;
            _secondaryPointerDown = secondary;
            var current = Volatile.Read(ref _configuration);
            if (!pressed || current is null || Volatile.Read(ref _disposed) != 0 ||
                !DestinationOwned(current.Value.DestinationWindow)) return;
            var target = _native.PointerTarget();
            if (target.RootWindowHandle == 0 || target.ProcessId == 0 ||
                !_native.IsSameSession(target.ProcessId) || !_native.IsElevated(target.ProcessId) ||
                !ReferenceEquals(current, Volatile.Read(ref _configuration))) return;
            var timestamp = unchecked((uint)Environment.TickCount64);
            _ = _native.Post(new IntPtr(current.Value.DestinationWindow), ElevatedInputMessages.PointerPressed,
                new IntPtr(target.RootWindowHandle),
                new IntPtr(unchecked((long)(((ulong)timestamp << 32) | (uint)Environment.ProcessId))));
        }
        catch (Exception error)
        {
            var now = Environment.TickCount64;
            if (_lastPointerFailure != 0 && now - _lastPointerFailure < 30_000) return;
            _lastPointerFailure = now;
            Write("warning", "elevated_input.pointer.unavailable", Guid.Empty, "native_failure",
                Stopwatch.GetTimestamp(), error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
    }

    private sealed partial class WindowsElevatedInputNative
    {
        public bool IsPointerButtonDown(int virtualKey) => (GetAsyncKeyState(virtualKey) & 0x8000) != 0;
        public (long WindowHandle, long RootWindowHandle, uint ProcessId) PointerTarget()
        {
            if (!GetCursorPos(out var point)) return default;
            var window = WindowFromPoint(point);
            var root = window == IntPtr.Zero ? IntPtr.Zero : GetAncestor(window, 2);
            _ = GetWindowThreadProcessId(root, out var processId);
            return (window.ToInt64(), root.ToInt64(), processId);
        }
        public void StartPointerMonitor(IntPtr window)
        {
            if (SetTimer(window, new UIntPtr(2), 40, IntPtr.Zero) == UIntPtr.Zero)
                throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        public void StopPointerMonitor(IntPtr window) => _ = KillTimer(window, new UIntPtr(2));

        [StructLayout(LayoutKind.Sequential)] private struct PointerPoint { public int X, Y; }
        [DllImport("user32.dll")] private static extern short GetAsyncKeyState(int key);
        [DllImport("user32.dll")] private static extern bool GetCursorPos(out PointerPoint point);
        [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(PointerPoint point);
        [DllImport("user32.dll", SetLastError = true)]
        private static extern UIntPtr SetTimer(IntPtr window, UIntPtr id, uint interval, IntPtr callback);
        [DllImport("user32.dll")] private static extern bool KillTimer(IntPtr window, UIntPtr id);
    }
}
