using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Only owned STATIC HWNDs and GDI regions; no adapter startup or global input.
internal static class WindowBoundsCornerTests
{
    internal static Task StrictOptionalWire()
    {
        Assert(Parse(1, 60, "").CornerRadius is null, "legacy missing radius");
        Assert(Parse(1, 60, ",\"corner_radius\":null").CornerRadius is null, "null radius");
        Assert(Parse(1, 60, ",\"corner_radius\":0").CornerRadius == 0, "explicit reset");
        Assert(Parse(1, 60, ",\"corner_radius\":12").CornerRadius == 12, "device pixel radius");
        foreach (var value in new[] { "-1", "1.5", "true", "\"8\"", "2147483648" })
        {
            var rejected = false;
            try { _ = Parse(1, 60, ",\"corner_radius\":" + value); }
            catch (Exception exception) when (exception is JsonException or InvalidDataException) { rejected = true; }
            Assert(rejected, "invalid radius rejected: " + value);
        }
        return Task.CompletedTask;
    }

    internal static Task OwnedRegionResizeAndReset()
    {
        var dpiScope = SetThreadDpiAwarenessContext(new IntPtr(-4));
        // This entry uses no instance state. Avoid the ordinary adapter's hook/clipboard startup.
        var adapter = (WindowsPlatformAdapter)RuntimeHelpers.GetUninitializedObject(typeof(WindowsPlatformAdapter));
        var window = IntPtr.Zero;
        var framed = IntPtr.Zero;
        try
        {
            window = CreateWindowExW(0, "STATIC", "bounds corner fixture", 0x80000000,
                120, 140, 620, 60, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            framed = CreateWindowExW(0, "STATIC", "bounds frame fixture", 0x00CF0000,
                120, 140, 620, 180, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Assert(window != IntPtr.Zero && framed != IntPtr.Zero, "owned windows created");
            var foreground = GetForegroundWindow();
            foreach (var height in new[] { 60, 530, 60 })
            {
                Apply(adapter, window, height, ",\"corner_radius\":8");
                AssertRounded(window, 620, height);
                Assert(GetForegroundWindow() == foreground, "bounds/region never activate");
                Assert(!IsWindowVisible(window), "hidden preparation stays hidden");
            }
            _ = ShowWindow(window, 4); // SW_SHOWNOACTIVATE, only this fixture's popup.
            Assert(IsWindowVisible(window) && GetForegroundWindow() == foreground, "owned passive reveal");
            foreach (var height in new[] { 530, 60 })
            {
                Apply(adapter, window, height, ",\"corner_radius\":8");
                AssertRounded(window, 620, height);
                Assert(GetForegroundWindow() == foreground, "visible resize/region never activate");
            }
            _ = ShowWindow(window, 0);
            // Missing/null leaves an existing region exactly as it is, not a reset or replacement.
            var saved = ReadRegion(window);
            try
            {
                Apply(adapter, window, 60, "");
                AssertSameRegion(window, saved);
                Apply(adapter, window, 60, ",\"corner_radius\":null");
                AssertSameRegion(window, saved);
            }
            finally { Assert(DeleteObject(saved), "delete owned region snapshot"); }
            Apply(adapter, window, 60, ",\"corner_radius\":0");
            var empty = CreateRectRgn(0, 0, 0, 0);
            try { Assert(GetWindowRgn(window, empty) == 0, "explicit zero clears native region"); }
            finally { Assert(DeleteObject(empty), "delete reset readback region"); }

            // Actual outer dimensions, not requested client dimensions, define the shape.
            Apply(adapter, framed, 60, ",\"corner_radius\":8");
            AssertRounded(framed, 620, 60);
            // Huge valid input is clamped before doubling; narrow windows must keep their center.
            var tiny = Parse(window.ToInt64(), 5, ",\"corner_radius\":2147483647") with { Width = 9 };
            Assert(adapter.SetWindowBounds(tiny).Success, "small bounds accept clamped radius");
            AssertRounded(window, 9, 5);
            var invalid = Parse(window.ToInt64(), 60, ",\"corner_radius\":8") with { Width = 0 };
            Assert(!adapter.SetWindowBounds(invalid).Success, "invalid client size fails");
            Assert(!adapter.SetWindowBounds(Parse(0, 60, ",\"corner_radius\":8")).Success, "invalid HWND fails");
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

    private static SetWindowBoundsCommand Parse(long window, int height, string radius)
    {
        using var data = JsonDocument.Parse($"{{\"window_handle\":{window},\"x\":120,\"y\":140,\"width\":620,\"height\":{height}{radius}}}");
        return PlatformCommandParser.ParseSetWindowBounds(ProtocolEnvelope.Create("platform_command",
            new { Command = "set_window_bounds", Data = data.RootElement.Clone() }));
    }

    private static void Apply(WindowsPlatformAdapter adapter, IntPtr window, int height, string radius)
    {
        var result = adapter.SetWindowBounds(Parse(window.ToInt64(), height, radius));
        Assert(result.Success, "production bounds application: " + result.Error);
    }

    private static void AssertRounded(IntPtr window, int width, int height)
    {
        Assert(GetWindowRect(window, out var outer), "outer readback");
        Assert(outer.Left == 120 && outer.Top == 140, "outer placement preserved");
        Assert(GetClientRect(window, out var client) && client.Right == width && client.Bottom == height,
            "actual client dimensions preserved");
        var region = ReadRegion(window);
        try
        {
            var w = outer.Right - outer.Left;
            var h = outer.Bottom - outer.Top;
            Assert(GetRgnBox(region, out var box) != 0 && box.Left == 0 && box.Top == 0 && box.Right == w && box.Bottom == h,
                "region covers current outer bounds after resize");
            Assert(!PtInRegion(region, 0, 0) && !PtInRegion(region, w - 1, 0) &&
                !PtInRegion(region, 0, h - 1) && !PtInRegion(region, w - 1, h - 1), "all four actual corners excluded");
            Assert(PtInRegion(region, w / 2, h / 2), "center retained");
            Assert(PtInRegion(region, w / 2, h - 1), "new bottom edge retained after resize");
        }
        finally { Assert(DeleteObject(region), "delete owned readback region"); }
    }

    private static IntPtr ReadRegion(IntPtr window)
    {
        var region = CreateRectRgn(0, 0, 0, 0);
        Assert(region != IntPtr.Zero, "allocate owned region");
        if (GetWindowRgn(window, region) == 0)
        {
            _ = DeleteObject(region);
            throw new InvalidOperationException("actual HWND must have a rounded native region");
        }
        return region;
    }

    private static void AssertSameRegion(IntPtr window, IntPtr expected)
    {
        var actual = ReadRegion(window);
        try { Assert(EqualRgn(expected, actual), "unspecified radius leaves native shape untouched"); }
        finally { Assert(DeleteObject(actual), "delete comparison region"); }
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
    [DllImport("gdi32.dll")] private static extern bool PtInRegion(IntPtr region, int x, int y);
    [DllImport("gdi32.dll")] private static extern bool EqualRgn(IntPtr first, IntPtr second);
    [DllImport("gdi32.dll")] private static extern int GetRgnBox(IntPtr region, out Rect box);
}
