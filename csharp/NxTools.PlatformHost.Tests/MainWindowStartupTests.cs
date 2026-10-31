using System.Runtime.InteropServices;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Only self-owned hidden STATIC HWND; no tray/registry/input/user windows.
internal static class MainWindowStartupTests
{
    internal static Task OwnedHiddenFirstShowRetainsClientSize()
    {
        var scope = SetThreadDpiAwarenessContext(new IntPtr(-4));
        var window = IntPtr.Zero;
        try
        {
            window = CreateWindowExW(0, "STATIC", "synthetic main startup", 0x00CF0000,
                int.MinValue, int.MinValue, int.MinValue, int.MinValue,
                IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Assert(window != IntPtr.Zero && !IsWindowVisible(window), "self-owned hidden window created");
            var foregroundBeforePrepare = GetForegroundWindow();
            var prepared = WindowsPlatformAdapter.PrepareMainWindowForTest(new NxTools.PlatformHost.Protocol.PrepareMainWindowCommand(window.ToInt64(), (uint)Environment.ProcessId, 1060, 650));
            Assert(prepared.Success, "production hidden preparation: " + prepared.Error);
            Assert(!IsWindowVisible(window), "prepare never shows the main window");
            Assert(GetForegroundWindow() == foregroundBeforePrepare, "hidden prepare must not steal foreground");
            var foreground = GetForegroundWindow();
            Assert(WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(window.ToInt64(), true).Success, "original native reveal");
            Assert(GetClientRect(window, out var client), "read actual first-visible client");
            var dpi = GetDpiForWindow(window);
            Assert(client.Right == (int)Math.Round(1060 * dpi / 96.0) && client.Bottom == (int)Math.Round(650 * dpi / 96.0),
                $"first visible client must use intended1060x650DIP; actual={client.Right}x{client.Bottom} dpi={dpi}");
            Assert(WindowsPlatformAdapter.SetNativeWindowBoundsForTest(window.ToInt64(), 120, 140, 1200, 700).Success, "simulate user's own resize after first show");
            Assert(WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(window.ToInt64(), false).Success, "original hide");
            Assert(WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(window.ToInt64(), true).Success, "later restore without preparation");
            Assert(GetClientRect(window, out var restored) && restored.Right == 1200 && restored.Bottom == 700, "later restore preserves user client size");
            Assert(GetForegroundWindow() == foreground, "nonactivating original show");
        }
        finally
        {
            if (window != IntPtr.Zero) _ = DestroyWindow(window);
            if (scope != IntPtr.Zero) _ = SetThreadDpiAwarenessContext(scope);
        }
        return Task.CompletedTask;
    }
    internal static Task DpiReadbackFailureAndCancellation()
    {
        var command = new NxTools.PlatformHost.Protocol.PrepareMainWindowCommand(1, 123, 1060, 650);
        foreach (var dpi in new uint[] { 96, 120, 144, 192 }) {
            var native = new Fake(dpi);
            var result = MainWindowPreparation.Prepare(command, native, CancellationToken.None);
            Assert(result.Success && native.Applied.Width == (int)Math.Round(1060 * dpi / 96.0, MidpointRounding.AwayFromZero) && native.Applied.Height == (int)Math.Round(650 * dpi / 96.0, MidpointRounding.AwayFromZero), "logical size converts once at target DPI");
            Assert(native.Calls.SequenceEqual(new[] { "scope", "validate", "work", "stage", "validate", "dpi", "apply", "validate", "read" }), "stage precedes target DPI and actual readback owns success");
        }
        foreach (var native in new[] { new Fake(96) { Valid = false }, new Fake(96) { Mismatch = true }, new Fake(96) { Visible = true }, new Fake(96) { RetireOnStage = true } }) {
            Assert(!MainWindowPreparation.Prepare(command, native, CancellationToken.None).Success, "invalid source or final readback fails closed");
            if (!native.Valid) Assert(!native.Calls.Contains("apply"), "invalid source never applies final bounds");
        }
        using var cancelled = new CancellationTokenSource(); cancelled.Cancel();
        var untouched = new Fake(96);
        Assert(!MainWindowPreparation.Prepare(command, untouched, cancelled.Token).Success && untouched.Calls.Count == 0, "pre-cancel does not touch native window");
        var envelope = NxTools.PlatformHost.Protocol.ProtocolEnvelope.Create("platform_command", new { command = "prepare_main_window", data = command });
        Assert(NxTools.PlatformHost.Protocol.PlatformCommandParser.ParsePrepareMainWindow(envelope) == command, "strict native wire matches Rust");
        NxTools.PlatformHost.Protocol.PlatformPipeClient.ValidateMainWindowPeer(envelope, 123);
        foreach (var peer in new uint[] { 0, 124 }) {
            try { NxTools.PlatformHost.Protocol.PlatformPipeClient.ValidateMainWindowPeer(envelope, peer); }
            catch (InvalidDataException) { continue; }
            throw new InvalidOperationException("unverified peer cannot prepare a main window");
        }
        foreach (var malformed in new object[] { new { window_handle = 1, source_process_id = 123, width = "1060", height = 650 }, new { window_handle = 1, source_process_id = 123, width = 1060, height = 650, injected = true }, new { window_handle = 0, source_process_id = 123, width = 1060, height = 650 } }) {
            try { _ = NxTools.PlatformHost.Protocol.PlatformCommandParser.ParsePrepareMainWindow(NxTools.PlatformHost.Protocol.ProtocolEnvelope.Create("platform_command", new { command = "prepare_main_window", data = malformed })); }
            catch (InvalidDataException) { continue; }
            throw new InvalidOperationException("malformed preparation must fail before native mutation");
        }
        return Task.CompletedTask;
    }
    internal static async Task DetachedPreparationDoesNotBlockLaterCommands()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var dispatcher = new NxTools.PlatformHost.Protocol.PlatformCommandSequenceDispatcher();
        await dispatcher.DispatchAsync("prepare_main_window", () => { entered.Set(); release.Wait(); return Task.CompletedTask; });
        try {
            Assert(entered.Wait(TimeSpan.FromSeconds(2)), "bounded detached preparation started");
            var later = false;
            await dispatcher.DispatchAsync("set_window_visibility", () => { later = true; return Task.CompletedTask; });
            Assert(later, "main preparation cannot park serial platform reader");
        } finally { release.Set(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(2)); }
    }

    private sealed class Fake(uint dpi) : IMainWindowNative
    {
        internal readonly List<string> Calls = [];
        internal bool Valid = true, Mismatch, Visible, RetireOnStage;
        internal NativeWindowRectangle Applied;
        public IDisposable EnterDpiScope() { Calls.Add("scope"); return new Scope(); }
        public bool Validate(NxTools.PlatformHost.Protocol.PrepareMainWindowCommand command) { Calls.Add("validate"); return Valid; }
        public NativeWindowRectangle PrimaryWorkArea() { Calls.Add("work"); return new(-2560, -200, 0, 1200); }
        public void Stage(long window, NativeWindowRectangle work) { Calls.Add("stage"); if (RetireOnStage) Valid = false; }
        public uint Dpi(long window) { Calls.Add("dpi"); return dpi; }
        public void Apply(long window, NativeWindowRectangle client) { Calls.Add("apply"); Applied = client; }
        public MainWindowReadback Read(long window) { Calls.Add("read"); return new(Mismatch ? new(0, 0, 1904, 1021) : Applied, dpi, Visible); }
        private sealed class Scope : IDisposable { public void Dispose() { } }
    }

    private static void Assert(bool value, string message) { if (!value) throw new InvalidOperationException(message); }
    [StructLayout(LayoutKind.Sequential)] private struct Rect { internal int Left, Top, Right, Bottom; }
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr CreateWindowExW(uint extended, string cls, string title, uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
}
