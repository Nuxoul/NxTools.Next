using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

// Original desktop HWND remains the native modal owner for the entire presentation.
internal static class PickerOwner
{
    private const uint NoActivateMoveSize = 0x0013; // SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE
    private static readonly object TopmostGate = new();
    private static readonly Dictionary<IntPtr, (uint Process, int Users)> Suspended = new();
    private sealed record Owner(IntPtr Handle) : IWin32Window;

    internal static bool IsLive(long handle, uint processId) => handle > 0 && processId != 0 &&
        IsWindow(new IntPtr(handle)) && GetWindowThreadProcessId(new IntPtr(handle), out var actual) != 0 && actual == processId;

    internal static WindowsCommandResult Run(long handle, uint processId, CancellationToken cancellation,
        Func<IWin32Window, WindowsCommandResult> present)
    {
        if (cancellation.IsCancellationRequested) return new(false, "picker_cancelled", NativeErrorCode: 995);
        if (!IsLive(handle, processId)) return new(false, "picker_invalid_owner", NativeErrorCode: 1400);
        using var suspended = SuspendTopmost(processId);
        if (cancellation.IsCancellationRequested) return new(false, "picker_cancelled", NativeErrorCode: 995);
        if (!IsLive(handle, processId)) return new(false, "picker_invalid_owner", NativeErrorCode: 1400);
        var result = present(new Owner(new IntPtr(handle)));
        if (cancellation.IsCancellationRequested) return new(false, "picker_cancelled", NativeErrorCode: 995);
        return IsLive(handle, processId) ? result : new(false, "picker_invalid_owner", NativeErrorCode: 1400);
    }

    internal static IDisposable SuspendTopmost(uint processId)
    {
        var held = new HashSet<IntPtr>();
        lock (TopmostGate)
        {
            foreach (var (window, lease) in Suspended.ToArray())
                if (lease.Process == processId)
                {
                    Suspended[window] = (processId, lease.Users + 1);
                    held.Add(window);
                }
            EnumWindows((window, _) =>
            {
                if (!held.Contains(window) && GetWindowThreadProcessId(window, out var actual) != 0 && actual == processId &&
                    IsWindowVisible(window) && (GetWindowLong(window, -20) & 0x8) != 0 &&
                    SetWindowPos(window, new IntPtr(-2), 0, 0, 0, 0, NoActivateMoveSize))
                {
                    Suspended.Add(window, (processId, 1));
                    held.Add(window);
                }
                return true;
            }, IntPtr.Zero);
        }
        return new RestoreTopmost(held, processId);
    }

    private sealed class RestoreTopmost(HashSet<IntPtr> windows, uint processId) : IDisposable
    {
        private int _disposed;
        public void Dispose()
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
            lock (TopmostGate)
                foreach (var window in windows)
                {
                    if (!Suspended.TryGetValue(window, out var lease) || lease.Process != processId) continue;
                    if (lease.Users > 1) { Suspended[window] = (processId, lease.Users - 1); continue; }
                    Suspended.Remove(window);
                    if (IsWindow(window) && GetWindowThreadProcessId(window, out var actual) != 0 && actual == processId && IsWindowVisible(window))
                        _ = SetWindowPos(window, new IntPtr(-1), 0, 0, 0, 0, NoActivateMoveSize);
                }
        }
    }

    private delegate bool EnumerateWindows(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")][return: MarshalAs(UnmanagedType.Bool)] private static extern bool EnumWindows(EnumerateWindows callback, IntPtr parameter);
    [DllImport("user32.dll")][return: MarshalAs(UnmanagedType.Bool)] private static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")][return: MarshalAs(UnmanagedType.Bool)] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")] private static extern int GetWindowLong(IntPtr window, int index);
    [DllImport("user32.dll")][return: MarshalAs(UnmanagedType.Bool)] private static extern bool SetWindowPos(IntPtr window, IntPtr insertAfter, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
}
