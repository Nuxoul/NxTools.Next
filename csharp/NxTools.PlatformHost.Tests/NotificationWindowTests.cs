using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class NotificationWindowTests
{
    private static readonly string[] Positions = ["top_left", "top_center", "top_right", "bottom_left", "bottom_center", "bottom_right"];
    private static PrepareNotificationWindowCommand Command(string position = "bottom_right") => new(2, 1, 123, position, 360, 400);
    private static void Assert(bool value, string message)
    { if (!value) throw new InvalidOperationException(message); }
    private static void Reject(Action operation)
    {
        try { operation(); }
        catch (InvalidDataException) { return; }
        throw new InvalidOperationException("Invalid notification command admitted.");
    }

    public static Task WireAndPeerValidation()
    {
        foreach (var position in Positions)
        {
            var command = Command(position);
            var envelope = ProtocolEnvelope.Create("platform_command", new { command = "prepare_notification_window", data = command });
            Assert(PlatformCommandParser.ParsePrepareNotificationWindow(envelope) == command, "exact typed DIP command wire");
            var correlation = Guid.NewGuid();
            Assert(PlatformPipeClient.ResolveDiagnosticCorrelationId(envelope with { CorrelationId = correlation }) == correlation,
                "notification diagnostic correlation survives the pipe boundary");
            var reply = PlatformPipeClient.CreateCommandResultEnvelope(envelope, new(true, Data: "{\"x\":-360,\"y\":40,\"width\":360,\"height\":400,\"dpi\":96}"));
            Assert(reply.CorrelationId == envelope.Id && reply.Payload.GetProperty("success").GetBoolean() &&
                JsonDocument.Parse(reply.Payload.GetProperty("data").GetString()!).RootElement.GetProperty("x").GetInt32() == -360,
                "result routes by command ID with JSON Data, not a second protocol shape");
            PlatformPipeClient.ValidateNotificationWindowPeer(envelope, 123);
            Reject(() => PlatformPipeClient.ValidateNotificationWindowPeer(envelope, 124));
            Reject(() => PlatformPipeClient.ValidateNotificationWindowPeer(envelope, 0));
        }
        foreach (var command in new[] { Command() with { WindowHandle = 0 }, Command() with { SourceWindowHandle = 0 },
            Command() with { WindowHandle = 1 }, Command() with { SourceProcessId = 0 }, Command("middle"),
            Command() with { Width = 0 }, Command() with { Height = -1 }, Command() with { Width = 1920 }, Command() with { Height = 1080 } })
            Reject(() => NotificationWindowPlacement.Validate(command));
        foreach (var data in new object[] { new { }, new { window_handle = 2, source_window_handle = 1,
            source_process_id = 123, position = "top_left", width = "360", height = 400 },
            new { window_handle = 2, source_window_handle = 1, source_process_id = 123, position = (string?)null, width = 360, height = 400 } })
            Reject(() => PlatformCommandParser.ParsePrepareNotificationWindow(ProtocolEnvelope.Create("platform_command",
                new { command = "prepare_notification_window", data })));
        return Task.CompletedTask;
    }

    public static Task SixAnchorsDpiAndReadback()
    {
        foreach (var work in new[] { new NativeWindowRectangle(0, 40, 1920, 1040),
            new NativeWindowRectangle(-1920, -1080, 0, -40), new NativeWindowRectangle(1920, 120, 3200, 1080),
            new NativeWindowRectangle(-50, 25, 150, 175) })
        foreach (var dpi in new uint[] { 96, 144, 192 })
        foreach (var position in Positions)
        {
            var native = new FakeNative(work, dpi);
            var result = NotificationWindowPlacement.Prepare(Command(position), native, CancellationToken.None);
            Assert(result.Success && result.Data is not null, "six-anchor prepare must succeed");
            using var data = JsonDocument.Parse(result.Data!);
            var actual = data.RootElement;
            var width = Math.Min(work.Width, 360 * (int)dpi / 96);
            var height = Math.Min(work.Height, 400 * (int)dpi / 96);
            var x = position.EndsWith("left", StringComparison.Ordinal) ? work.Left :
                position.EndsWith("right", StringComparison.Ordinal) ? work.Right - width : work.Left + (work.Width - width) / 2;
            var y = position.StartsWith("top", StringComparison.Ordinal) ? work.Top : work.Bottom - height;
            Assert(actual.GetProperty("x").GetInt32() == x && actual.GetProperty("y").GetInt32() == y &&
                actual.GetProperty("width").GetInt32() == width && actual.GetProperty("height").GetInt32() == height &&
                actual.GetProperty("dpi").GetUInt32() == dpi, "device client extents, zero extra native margin and work-area anchors");
            Assert(native.Calls.SequenceEqual(new[] { "scope", "validate", "work", "policy", "monitor", "dpi", "validate", "apply", "validate", "readback" }),
                "monitor must precede target DPI and final readback must own success");
        }
        foreach (var native in new[] { new FakeNative() { Valid = false }, new FakeNative() { Mismatch = true },
            new FakeNative() { Visible = true }, new FakeNative() { NoActivate = false }, new FakeNative() { Topmost = false },
            new FakeNative() { ToolWindow = false }, new FakeNative() { ThrowNative = true } })
        {
            var result = NotificationWindowPlacement.Prepare(Command(), native, CancellationToken.None);
            Assert(!result.Success && result.Data is null, "invalid source/policy/readback must fail closed");
            if (native.ThrowNative) Assert(result.NativeErrorCode == 5, "preserve actual API error");
            if (!native.Valid) Assert(!native.Calls.Contains("policy"), "invalid source must not mutate target");
        }
        using var cancellation = new CancellationTokenSource(); cancellation.Cancel();
        var cancelled = new FakeNative();
        Assert(!NotificationWindowPlacement.Prepare(Command(), cancelled, cancellation.Token).Success && cancelled.Calls.Count == 0,
            "pre-cancel never touches native HWNDs");
        var retired = new FakeNative() { RetireAfterMonitor = true };
        Assert(!NotificationWindowPlacement.Prepare(Command(), retired, CancellationToken.None).Success && !retired.Calls.Contains("apply"),
            "source retirement after monitor staging cannot commit final placement");
        return Task.CompletedTask;
    }

    public static async Task BoundedDetachedAndDiagnostics()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var cancellation = new CancellationTokenSource();
        var native = new FakeNative() { Entered = entered, Release = release };
        var log = new FixtureLog();
        var id = Guid.NewGuid();
        var dispatcher = new PlatformCommandSequenceDispatcher();
        Task<WindowsCommandResult>? first = null;
        await dispatcher.DispatchAsync("prepare_notification_window", async () =>
        {
            first = WindowsPlatformAdapter.PrepareNotificationWindowCoreAsync(Command(), id, log, cancellation.Token, native);
            await first;
        });
        try
        {
            Assert(entered.Wait(TimeSpan.FromSeconds(3)), "detached bounded STA must start");
            var later = false;
            await dispatcher.DispatchAsync("set_window_visibility", () => { later = true; return Task.CompletedTask; });
            Assert(later && first is { IsCompleted: false }, "native placement must not block later platform commands");
            cancellation.Cancel();
            Assert(!(await first!.WaitAsync(TimeSpan.FromSeconds(3))).Success, "logical cancellation reports failure");
            var second = await WindowsPlatformAdapter.PrepareNotificationWindowCoreAsync(Command(), Guid.NewGuid(), log,
                CancellationToken.None, new FakeNative());
            Assert(!second.Success, "stuck native call retains the sole runner slot; no replacement worker");
            Assert(native.Apartment == ApartmentState.STA, "native API work uses original bounded STA runner");
        }
        finally { release.Set(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(3)); }
        Assert(log.Events.Any(e => e.Id == id && e.Name == "ui.notification.surface.accepted") &&
            log.Events.Any(e => e.Id == id && e.Name == "ui.notification.surface.start") &&
            log.Events.Count(e => e.Id == id && e.Name == "ui.notification.surface.terminal") == 1,
            "correlated accepted/start/exactly one terminal");
        Assert(log.Events.All(e => e.Fields.ContainsKey("elapsed_ms") && !e.Fields.ContainsKey("data") && !e.Fields.ContainsKey("message")),
            "metadata only diagnostics, no result payload or exception messages");
    }

    public static Task OwnedHwndHiddenPolicyAndClientReadback()
    {
        var previous = SetThreadDpiAwarenessContext(new IntPtr(-4));
        Assert(previous != IntPtr.Zero, "owned fixture PMv2 scope");
        var source = IntPtr.Zero;
        var target = IntPtr.Zero;
        try
        {
            source = CreateWindowExW(0, "STATIC", "synthetic source", 0x80000000, 100, 100, 200, 100,
                IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            // Caption/frame intentionally exercise actual client-inset compensation.
            target = CreateWindowExW(0, "STATIC", "synthetic popup", 0x80C40000, 0, 0, 100, 80,
                IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Assert(source != IntPtr.Zero && target != IntPtr.Zero, "owned HWND fixture creation");
            var foreground = GetForegroundWindow();
            var monitor = MonitorFromWindow(source, 2);
            var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
            Assert(GetMonitorInfoW(monitor, ref info), "source root work area");
            foreach (var position in Positions)
            {
                var command = new PrepareNotificationWindowCommand(target.ToInt64(), source.ToInt64(), (uint)Environment.ProcessId, position, 300, 200);
                var result = WindowsPlatformAdapter.PrepareNotificationWindowForTest(command);
                Assert(result.Success, "actual owned HWND prepare: " + result.Error);
                using var data = JsonDocument.Parse(result.Data!);
                var actual = data.RootElement;
                Assert(GetClientRect(target, out var client), "native client readback");
                var origin = new Point();
                Assert(ClientToScreen(target, ref origin), "native screen client origin");
                var dpi = GetDpiForWindow(target);
                var width = Math.Min(info.Work.Width, (int)Math.Round(300 * dpi / 96.0, MidpointRounding.AwayFromZero));
                var height = Math.Min(info.Work.Height, (int)Math.Round(200 * dpi / 96.0, MidpointRounding.AwayFromZero));
                var x = position.EndsWith("left", StringComparison.Ordinal) ? info.Work.Left : position.EndsWith("right", StringComparison.Ordinal)
                    ? info.Work.Right - width : info.Work.Left + (info.Work.Width - width) / 2;
                var y = position.StartsWith("top", StringComparison.Ordinal) ? info.Work.Top : info.Work.Bottom - height;
                Assert(origin.X == x && origin.Y == y && client.Width == width && client.Height == height,
                    "all six real API client anchors use source work area");
                Assert(actual.GetProperty("x").GetInt32() == origin.X && actual.GetProperty("y").GetInt32() == origin.Y &&
                    actual.GetProperty("width").GetInt32() == client.Width && actual.GetProperty("height").GetInt32() == client.Height &&
                    actual.GetProperty("dpi").GetUInt32() == dpi, "JSON is actual device client readback");
                Assert(!IsWindowVisible(target) && GetForegroundWindow() == foreground, "prepare never shows or activates");
                Assert((GetWindowLongPtrW(target, -20).ToInt64() & 0x08000088) == 0x08000088, "NOACTIVATE TOOLWINDOW TOPMOST actual styles");
                Assert((GetWindowLongPtrW(target, -16).ToInt64() & 0x00C40000L) == 0,
                    "notification popup must not keep a native caption, border or resize frame outside the card");
                Assert(!WindowsPlatformAdapter.PrepareNotificationWindowForTest(command with { SourceProcessId = uint.MaxValue }).Success,
                    "foreign PID rejected at actual API seam");
                Assert(!WindowsPlatformAdapter.PrepareNotificationWindowForTest(command with { SourceWindowHandle = target.ToInt64() }).Success,
                    "same source/target rejected");
                Assert(!WindowsPlatformAdapter.PrepareNotificationWindowForTest(command with { SourceWindowHandle = 0x123 }).Success,
                    "invalid source rejected");
            }
            var valid = new PrepareNotificationWindowCommand(target.ToInt64(), source.ToInt64(), (uint)Environment.ProcessId, "top_left", 300, 200);
            // Explicit reveal remains a separate, existing SW_SHOWNOACTIVATE boundary.
            Assert(WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(target.ToInt64(), true).Success &&
                IsWindowVisible(target) && GetForegroundWindow() == foreground, "separate reveal is nonactivating");
            Assert(!WindowsPlatformAdapter.PrepareNotificationWindowForTest(valid).Success, "visible target cannot be prepared");
            Assert(WindowsPlatformAdapter.PrepareNotificationWindowForTest(valid with { KeepVisible = true, Height = 240 }).Success &&
                IsWindowVisible(target) && GetForegroundWindow() == foreground, "visible notification resizes in place without hide/show or focus theft");
            Assert(WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(target.ToInt64(), false).Success, "reuse hides through existing boundary");
        }
        finally
        {
            if (target != IntPtr.Zero) _ = DestroyWindow(target);
            if (source != IntPtr.Zero) _ = DestroyWindow(source);
            _ = SetThreadDpiAwarenessContext(previous);
        }
        return Task.CompletedTask;
    }

    private sealed class FakeNative(NativeWindowRectangle? work = null, uint dpi = 144) : INotificationWindowNative
    {
        internal readonly List<string> Calls = [];
        internal ManualResetEventSlim? Entered, Release;
        internal bool Valid = true, Visible, NoActivate = true, ToolWindow = true, Topmost = true, Mismatch, ThrowNative, RetireAfterMonitor;
        internal ApartmentState Apartment;
        private NativeWindowRectangle _bounds;
        public IDisposable EnterDpiScope() { Calls.Add("scope"); return new EmptyScope(); }
        public bool ValidateWindows(PrepareNotificationWindowCommand command) { Calls.Add("validate"); return Valid; }
        public NativeWindowRectangle GetWorkArea(long source) { Calls.Add("work"); return work ?? new(0, 0, 1920, 1040); }
        public void SetPassivePolicy(long target) { Calls.Add("policy"); if (ThrowNative) throw new Win32Exception(5, "synthetic secret payload"); }
        public void PrepareMonitor(long target, NativeWindowRectangle area)
        {
            if (RetireAfterMonitor) Valid = false;
            Calls.Add("monitor"); Apartment = Thread.CurrentThread.GetApartmentState(); Entered?.Set(); Release?.Wait();
        }
        public uint GetDpi(long target) { Calls.Add("dpi"); return dpi; }
        public void ApplyClientBounds(long target, NativeWindowRectangle bounds) { Calls.Add("apply"); _bounds = bounds; }
        public NotificationWindowReadback Readback(long target)
        {
            Calls.Add("readback");
            return new(Mismatch ? _bounds with { Left = _bounds.Left + 1 } : _bounds, dpi, Visible, NoActivate, ToolWindow, Topmost);
        }
        private sealed class EmptyScope : IDisposable { public void Dispose() { } }
    }
    private sealed class FixtureLog : IPlatformLog
    {
        internal readonly List<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Events = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { lock (Events) Events.Add((eventName, correlationId, fields!)); }
    }

    [StructLayout(LayoutKind.Sequential)] private struct Point { internal int X, Y; }
    [StructLayout(LayoutKind.Sequential)] private struct MonitorInfo { internal int Size; internal NativeWindowRectangle Monitor, Work; internal uint Flags; }
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern IntPtr CreateWindowExW(uint exStyle, string className, string name, uint style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out NativeWindowRectangle bounds);
    [DllImport("user32.dll")] private static extern bool ClientToScreen(IntPtr window, ref Point point);
    [DllImport("user32.dll")] private static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
}
