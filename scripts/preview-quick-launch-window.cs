using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

// Native presentation for the standalone example only; never targets NxTools.
public static class QuickLaunchPreviewWindow
{
    private const string PreviewTitle = "NxTools — Search Surface Preview (F2 / Esc)";
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr window, StringBuilder text, int capacity);
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr window, int attribute, ref int value, int size);
    [DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out int value, int size);

    public static void Prepare(int processId)
    {
        var elapsed = Stopwatch.StartNew();
        using (var process = Process.GetProcessById(processId))
        {
            while (!process.HasExited && elapsed.ElapsedMilliseconds < 5000)
            {
                IntPtr preview = IntPtr.Zero;
                EnumWindows((window, parameter) =>
                {
                    uint owner;
                    GetWindowThreadProcessId(window, out owner);
                    if (owner != (uint)processId) return true;
                    var title = new StringBuilder(128);
                    GetWindowText(window, title, title.Capacity);
                    if (title.ToString() != PreviewTitle) return true;
                    preview = window;
                    return false;
                }, IntPtr.Zero);
                if (preview != IntPtr.Zero)
                {
                    // Same policy as PlatformHost NativeMethods.SetWindowShadow(false).
                    // Leave GPUI's composition, activation, input and bounds owners intact.
                    int policy = 1; // DWMNCRP_DISABLED
                    int result = DwmSetWindowAttribute(preview, 2, ref policy, sizeof(int));
                    if (result != 0) throw new Win32Exception(result, "Preview DWM frame policy failed.");
                    int enabled;
                    result = DwmGetWindowAttribute(preview, 1, out enabled, sizeof(int));
                    if (result != 0) throw new Win32Exception(result, "Preview DWM frame readback failed.");
                    if (enabled != 0) throw new InvalidOperationException("Preview native frame remains enabled.");
                    return;
                }
                Thread.Sleep(20);
            }
        }
        throw new InvalidOperationException("Standalone GPUI preview window was not created within 5 seconds.");
    }
}
