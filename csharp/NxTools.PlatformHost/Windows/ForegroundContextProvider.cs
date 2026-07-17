using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal sealed record ForegroundContext(
    uint ProcessId,
    string ProcessName,
    string WindowTitle);

internal sealed class ForegroundContextProvider
{
    public ForegroundContext Capture()
    {
        var window = NativeMethods.GetForegroundWindow();
        if (window == IntPtr.Zero)
        {
            return new ForegroundContext(0, string.Empty, string.Empty);
        }

        _ = NativeMethods.GetWindowThreadProcessId(window, out var processId);
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

        return new ForegroundContext(processId, processName, title.ToString());
    }

    private static class NativeMethods
    {
        [DllImport("user32.dll")]
        internal static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        internal static extern int GetWindowText(IntPtr window, StringBuilder text, int maxCount);

        [DllImport("user32.dll", CharSet = CharSet.Unicode)]
        internal static extern int GetWindowTextLength(IntPtr window);
    }
}
