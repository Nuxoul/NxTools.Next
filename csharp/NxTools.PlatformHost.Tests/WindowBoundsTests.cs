using NxTools.PlatformHost.Diagnostics;
using System.Threading.Channels;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Only owned STATIC HWNDs; no adapter startup or global input.
internal static class WindowBoundsTests
{
    private sealed class FixtureLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) { }
    }

    internal static Task StrictWire()
    {
        Assert(Parse(1, 60) == new SetWindowBoundsCommand(1, 120, 140, 620, 60), "client bounds wire");
        return Task.CompletedTask;
    }

    internal static Task OwnedClientBoundsWithoutRegion()
    {
        var dpiScope = SetThreadDpiAwarenessContext(new IntPtr(-4));
        // No ordinary adapter startup: only the bounds boundary is used.
        var adapter = (WindowsPlatformAdapter)RuntimeHelpers.GetUninitializedObject(typeof(WindowsPlatformAdapter));
        var window = IntPtr.Zero;
        var framed = IntPtr.Zero;
        try
        {
            window = CreateWindowExW(0, "STATIC", "bounds fixture", 0x80000000,
                120, 140, 620, 60, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            framed = CreateWindowExW(0, "STATIC", "bounds frame fixture", 0x00CF0000,
                120, 140, 620, 180, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Assert(window != IntPtr.Zero && framed != IntPtr.Zero, "owned windows created");
            var foreground = GetForegroundWindow();
            foreach (var height in new[] { 60, 530, 60 })
            {
                Apply(adapter, window, height);
                AssertBounds(window, 620, height);
                Assert(GetForegroundWindow() == foreground, "bounds never activate");
                Assert(!IsWindowVisible(window), "hidden preparation stays hidden");
            }
            _ = ShowWindow(window, 4); // SW_SHOWNOACTIVATE, only this fixture's popup.
            Assert(IsWindowVisible(window) && GetForegroundWindow() == foreground, "owned passive reveal");
            foreach (var height in new[] { 530, 60 })
            {
                Apply(adapter, window, height);
                AssertBounds(window, 620, height);
                Assert(GetForegroundWindow() == foreground, "visible resize never activates");
            }
            _ = ShowWindow(window, 0);

            // Actual client dimensions are honoured through the native frame.
            Apply(adapter, framed, 60);
            AssertBounds(framed, 620, 60);

            Assert(WindowsPlatformAdapter.ParkNativeWindowBoundsCenteredForTest(window.ToInt64(), 620, 410).Success,
                "owned pending centered resize");
            var recoveryRoots = new List<IntPtr>();
            var recovery = new WindowsPlatformAdapter.EventWindow(
                Channel.CreateUnbounded<WindowsPlatformEvent>().Writer, new FixtureLog(),
                _ => null, _ => { }, _ => { }, _ => false,
                queueWindowGeometry: root => { recoveryRoots.Add(root); return true; });
            recovery.RetryCenteredResizeForRoot(window);
            Assert(recoveryRoots.SequenceEqual(new[] { window }),
                "centered resize recovery delegates to the shared geometry work entry");

            var invalid = Parse(window.ToInt64(), 60) with { Width = 0 };
            Assert(!adapter.SetWindowBounds(invalid).Success, "invalid client size fails");
            Assert(!adapter.SetWindowBounds(Parse(0, 60)).Success, "invalid HWND fails");
            Assert(GetForegroundWindow() == foreground, "all bounds calls preserve foreground");
        }
        finally
        {
            if (framed != IntPtr.Zero) Assert(DestroyWindow(framed), "destroy owned framed window");
            if (window != IntPtr.Zero) Assert(DestroyWindow(window), "destroy owned popup");
            if (dpiScope != IntPtr.Zero) _ = SetThreadDpiAwarenessContext(dpiScope);
        }
        return Task.CompletedTask;
    }

    private static SetWindowBoundsCommand Parse(long window, int height)
    {
        using var data = JsonDocument.Parse($"{{\"window_handle\":{window},\"x\":120,\"y\":140,\"width\":620,\"height\":{height}}}");
        return PlatformCommandParser.ParseSetWindowBounds(ProtocolEnvelope.Create("platform_command",
            new { Command = "set_window_bounds", Data = data.RootElement.Clone() }));
    }

    private static void Apply(WindowsPlatformAdapter adapter, IntPtr window, int height)
    {
        var result = adapter.SetWindowBounds(Parse(window.ToInt64(), height));
        Assert(result.Success, "production bounds application: " + result.Error);
    }

    private static void AssertBounds(IntPtr window, int width, int height)
    {
        Assert(GetWindowRect(window, out var outer) && outer.Left == 120 && outer.Top == 140, "outer placement preserved");
        Assert(GetClientRect(window, out var client) && client.Right == width && client.Bottom == height,
            "actual client dimensions applied");
        AssertNoRegion(window);
    }

    // Window shape belongs to DWM (Windows 11) or stays rectangular; bounds never install a GDI region.
    private static void AssertNoRegion(IntPtr window)
    {
        var region = CreateRectRgn(0, 0, 0, 0);
        try { Assert(GetWindowRgn(window, region) == 0, "bounds never install a window region"); }
        finally { Assert(DeleteObject(region), "delete readback region"); }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    [StructLayout(LayoutKind.Sequential)] private struct Rect { internal int Left, Top, Right, Bottom; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowExW(uint exStyle, string className, string name, uint style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern int GetWindowRgn(IntPtr window, IntPtr region);
    [DllImport("user32.dll")] private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("gdi32.dll")] private static extern IntPtr CreateRectRgn(int left, int top, int right, int bottom);
    [DllImport("gdi32.dll")] private static extern bool DeleteObject(IntPtr region);
}
