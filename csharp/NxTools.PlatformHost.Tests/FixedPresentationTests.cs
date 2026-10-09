using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class FixedPresentationTests
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(uint extendedStyle, string className, string name, uint style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetClientRect(IntPtr window, out Rectangle bounds);
    [StructLayout(LayoutKind.Sequential)]
    private struct Rectangle { public int Left, Top, Right, Bottom; }

    public static Task FixedOneShotPreservesAnchorAndCancel()
    {
        var anchor = CreateWindowEx(0, "STATIC", "synthetic anchor", 0x80000000, 380, 310, 560, 176,
            IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
        var target = CreateWindowEx(0, "STATIC", "synthetic target", 0x80000000, 0, 0, 100, 80,
            IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
        if (anchor == IntPtr.Zero || target == IntPtr.Zero) throw new InvalidOperationException("Synthetic HWND creation failed.");
        try
        {
            using var cancelled = new CancellationTokenSource();
            cancelled.Cancel();
            var reject = WindowsPlatformAdapter.PresentFixedWindowOverAnchorForTest(target.ToInt64(), anchor.ToInt64(), 560, 278, cancelled.Token);
            Assert(!reject.Success && !IsWindowVisible(target), "cancelled target must remain hidden");
            var result = WindowsPlatformAdapter.PresentFixedWindowOverAnchorForTest(target.ToInt64(), anchor.ToInt64(), 560, 278);
            Assert(result.Success && IsWindowVisible(target), "one fixed apply must show the target");
            Assert(GetClientRect(target, out var confirmation) && confirmation.Right == 560 && confirmation.Bottom == 278,
                "fixed client must be278 without a replay loop");
            Assert(GetClientRect(anchor, out var compact) && compact.Right == 560 && compact.Bottom == 176,
                "fixed presentation must never resize old compact window");
            var envelope = ProtocolEnvelope.Create("platform_command", new {
                command = "present_fixed_window_over_anchor",
                data = new { window_handle = target.ToInt64(), anchor_window_handle = anchor.ToInt64(), width = 560, height = 278 }
            });
            var parsed = PlatformCommandParser.ParsePresentFixedWindowOverAnchor(envelope);
            Assert(parsed.Width == 560 && parsed.Height == 278 && parsed.AnchorWindowHandle == anchor.ToInt64(), "fixed wire contract");
        }
        finally { _ = DestroyWindow(target); _ = DestroyWindow(anchor); }
        return Task.CompletedTask;
    }
    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
