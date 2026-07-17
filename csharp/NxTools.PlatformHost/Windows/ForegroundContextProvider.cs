using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal sealed record ForegroundContext(
    long WindowHandle,
    long FocusWindowHandle,
    uint ProcessId,
    string ProcessName,
    string WindowTitle);

internal sealed class ForegroundContextProvider
{
    public ForegroundContext Capture()
    {
        return Capture(NativeMethods.GetForegroundWindow());
    }

    public ForegroundContext Capture(IntPtr window)
    {
        if (window == IntPtr.Zero)
        {
            return new ForegroundContext(0, 0, 0, string.Empty, string.Empty);
        }

        var windowThread = NativeMethods.GetWindowThreadProcessId(window, out var processId);
        var focusWindow = window;
        var threadInfo = new NativeMethods.GuiThreadInfo
        {
            Size = (uint)Marshal.SizeOf<NativeMethods.GuiThreadInfo>()
        };
        if (windowThread != 0 &&
            NativeMethods.GetGUIThreadInfo(windowThread, ref threadInfo) &&
            threadInfo.FocusWindow != IntPtr.Zero)
        {
            focusWindow = threadInfo.FocusWindow;
        }
        var titleLength = NativeMethods.GetWindowTextLength(window);
        var title = new StringBuilder(Math.Max(titleLength + 1, 1));
        _ = NativeMethods.GetWindowText(window, title, title.Capacity);

        string processName;
        try
        {
            using var process = Process.GetProcessById(checked((int)processId));
            processName = process.ProcessName;
        }
        catch (ArgumentException)
        {
            processName = string.Empty;
        }

        return new ForegroundContext(
            window.ToInt64(),
            focusWindow.ToInt64(),
            processId,
            processName,
            title.ToString());
    }

    private static class NativeMethods
    {
        [DllImport("user32.dll")]
        internal static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool GetGUIThreadInfo(uint threadId, ref GuiThreadInfo info);

        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        internal static extern int GetWindowText(IntPtr window, StringBuilder text, int maxCount);

        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        internal static extern int GetWindowTextLength(IntPtr window);

        [StructLayout(LayoutKind.Sequential)]
        internal struct GuiThreadInfo
        {
            public uint Size;
            public uint Flags;
            public IntPtr ActiveWindow;
            public IntPtr FocusWindow;
            public IntPtr CaptureWindow;
            public IntPtr MenuOwnerWindow;
            public IntPtr MoveSizeWindow;
            public IntPtr CaretWindow;
            public Rect CaretRect;
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Rect
        {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;
        }
    }
}
