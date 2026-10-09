using System.Runtime.InteropServices;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class GpuiFixedTransitionTests
{
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetClientRect(IntPtr window, out Rectangle rectangle);
    [StructLayout(LayoutKind.Sequential)]
    private struct Rectangle { public int Left, Top, Right, Bottom; }

    public static Task OwnedTaskCompletionPreservesCenter()
    {
        var expected = uint.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_PID") ?? "0");
        var window = new IntPtr(long.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_HWND") ?? "0"));
        _ = GetWindowThreadProcessId(window, out var owner);
        if (expected == 0 || owner != expected) throw new InvalidOperationException("Owned Debug GPUI HWND required.");
        ResizeAndSettle(window, 184);
        if (!SetWindowPos(window, IntPtr.Zero, 340, 290, 0, 0, 0x15)) throw new InvalidOperationException("Owned move failed.");
        Thread.Sleep(100);
        var baseline = Visible(window);
        for (var round = 0; round < 3; round++)
        {
            foreach (var height in new[] { 184, 280, 184, 196, 184 })
            {
                ResizeAndSettle(window, height);
                var after = Visible(window);
                var driftX = (after.Left + after.Right - baseline.Left - baseline.Right) / 2.0;
                var driftY = (after.Top + after.Bottom - baseline.Top - baseline.Bottom) / 2.0;
                Console.WriteLine($"TASK_CENTER height={height} drift_x={driftX} drift_y={driftY}");
                if (Math.Abs(driftX) > 1 || Math.Abs(driftY) > 1)
                    throw new InvalidOperationException($"Progress to completion native center drifted: x={driftX},y={driftY}.");
            }
        }
        return Task.CompletedTask;
    }
    private static Rectangle Visible(IntPtr window)
    {
        if (DwmGetWindowAttribute(window, 9, out var bounds, Marshal.SizeOf<Rectangle>()) != 0)
            throw new InvalidOperationException("Visible bounds missing.");
        return bounds;
    }
    private static void ResizeAndSettle(IntPtr window, int height)
    {
        var width = Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_COMPLETION_CENTER") is null ? 560 : 440;
        var parked = WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), width, height);
        if (!parked.Success) throw new InvalidOperationException("Resize parking failed.");
        for (var turn = 0; turn < 3; turn++)
        {
            _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(window.ToInt64());
            Thread.Sleep(50);
        }
        if (!GetClientRect(window, out var client) || client.Right != width || client.Bottom != height)
            throw new InvalidOperationException($"Task phase client size did not settle: target={width}x{height}, actual={client.Right}x{client.Bottom}.");
    }

    public static Task OwnedProductionProbePresentation()
    {
        var expected = uint.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_PID") ?? "0");
        var source = long.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_HWND") ?? "0");
        var target = long.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TARGET_HWND") ?? "0");
        foreach (var handle in new[] { source, target }) {
            _ = GetWindowThreadProcessId(new IntPtr(handle), out var owner);
            if (expected == 0 || owner != expected) throw new InvalidOperationException("Owned Debug probe HWND/PID required.");
        }
        var result = WindowsPlatformAdapter.PresentFixedWindowOverAnchorForTest(target, source, 560, 278);
        if (!result.Success) throw new InvalidOperationException("Production probe fixed presentation failed.");
        Console.WriteLine("GPUI_PROBE_FIXED_PRESENTED client_height=278");
        return Task.CompletedTask;
    }
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out Rectangle bounds, int size);

    public static Task OwnedGpuiFixedPresentation()
    {
        var process = uint.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_PID") ?? "0");
        var source = new IntPtr(long.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_HWND") ?? "0"));
        var target = new IntPtr(long.Parse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TARGET_HWND") ?? "0"));
        foreach (var window in new[] { source, target })
        {
            _ = GetWindowThreadProcessId(window, out var owner);
            if (process == 0 || owner != process) throw new InvalidOperationException("Owned Debug GPUI presentation target required.");
        }
        if (IsWindowVisible(target)) throw new InvalidOperationException("Confirmation target must start hidden.");
        if (!SetWindowPos(source, IntPtr.Zero, 340, 290, 0, 0, 0x15))
            throw new InvalidOperationException("Owned fixture anchor move failed.");
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        var rejected = WindowsPlatformAdapter.PresentFixedWindowOverAnchorForTest(target.ToInt64(), source.ToInt64(), 560, 278, cancelled.Token);
        if (rejected.Success || IsWindowVisible(target)) throw new InvalidOperationException("Cancelled fixed presentation must stay hidden.");
        var foreground = GetForegroundWindow();
        var result = WindowsPlatformAdapter.PresentFixedWindowOverAnchorForTest(target.ToInt64(), source.ToInt64(), 560, 278);
        if (!result.Success) throw new InvalidOperationException($"Fixed GPUI presentation rejected: {result.Error}");
        if (!GetClientRect(source, out var compact) || compact.Right != 560 || compact.Bottom != 176 ||
            !GetClientRect(target, out var confirmation) || confirmation.Right != 560 || confirmation.Bottom != 278)
            throw new InvalidOperationException("Fixed presentation changed compact size or failed to create278 confirmation.");
        _ = DwmGetWindowAttribute(source, 9, out var anchor, Marshal.SizeOf<Rectangle>());
        _ = DwmGetWindowAttribute(target, 9, out var shown, Marshal.SizeOf<Rectangle>());
        if (Math.Abs(anchor.Left + anchor.Right - shown.Left - shown.Right) > 4 ||
            Math.Abs(anchor.Top + anchor.Bottom - shown.Top - shown.Bottom) > 4)
            throw new InvalidOperationException("Fixed presentation did not inherit moved anchor center.");
        if (GetForegroundWindow() != foreground) throw new InvalidOperationException("Native fixed presentation must not steal focus.");
        Console.WriteLine("GPUI_FIXED_PRESENTATION compact_height=176 confirmation_height=278 center_preserved=true");
        return Task.CompletedTask;
    }

    public static Task OwnedGpuiFixedTransition()
    {
        if (!long.TryParse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_HWND"), out var handle) ||
            !uint.TryParse(Environment.GetEnvironmentVariable("NXTOOLS_PREVIEW_TRANSITION_PID"), out var expectedProcess))
            throw new InvalidOperationException("Explicit owned Debug preview HWND/PID required; no ordinary product target.");
        var window = new IntPtr(handle);
        _ = GetWindowThreadProcessId(window, out var process);
        if (process != expectedProcess || expectedProcess == 0)
            throw new InvalidOperationException("Preview HWND ownership mismatch.");
        if (!GetClientRect(window, out var before) || before.Right != 560 || before.Bottom != 176)
            throw new InvalidOperationException("Expected actual compact client560x176 at native100% scale.");
        var admission = WindowsPlatformAdapter.QueueNativeCenteredResizeForTest(handle, 560, 278);
        if (!admission.Success) throw new InvalidOperationException("Fixed transition queue rejected.");
        var deadline = DateTime.UtcNow.AddSeconds(3);
        do
        {
            Thread.Sleep(50);
            _ = WindowsPlatformAdapter.ReapplyNativeWindowBoundsCenteredForTest(handle);
        } while (DateTime.UtcNow < deadline &&
                 (!GetClientRect(window, out var current) || current.Right != 560 || current.Bottom != 278));
        if (!GetClientRect(window, out var after) || after.Right != 560 || after.Bottom != 278)
            throw new InvalidOperationException($"Actual GPUI fixed confirmation failed: client={after.Right}x{after.Bottom}.");
        Console.WriteLine($"GPUI_FIXED_TRANSITION client_width={after.Right} client_height={after.Bottom}");
        return Task.CompletedTask;
    }
}
