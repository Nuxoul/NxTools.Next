using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;
using static NxTools.PlatformHost.Tests.ElevatedInputTests;
using static NxTools.PlatformHost.Tests.ElevatedInputChannelTests;

namespace NxTools.PlatformHost.Tests;

internal static class ElevatedInputHostTests
{
    public static async Task MessageRoutingAndOwnedCleanup()
    {
        using var keyboard = new KeyboardMonitorTests.Platform();
        var native = new Native();
        var host = new ElevatedInputHost(42, NullPlatformLog.Instance, native, keyboard, new PasteOperations());
        try
        {
            Assert(host.Configure(Config(1)).Success && host.IsHookInstalled, "helper owns one fake keyboard monitor");
            keyboard.Tap(0xA2, 100); keyboard.Tap(0xA2, 300); keyboard.Drain();
            await Until(() => native.Decisions >= 1);
            Assert(native.Forwarded.IsEmpty, "ordinary foreground is never forwarded by elevated helper");
            native.Elevated = true;
            keyboard.Tap(0xA2, 1000); keyboard.Tap(0xA2, 1200); keyboard.Drain();
            await Until(() => native.Forwarded.Count == 1);
            var message = native.Forwarded.Single();
            Assert(message.Message == ElevatedInputMessages.Gesture && message.Generation == 1 &&
                unchecked((uint)((ulong)message.Value >> 32)) == 1200 &&
                unchecked((int)(uint)message.Value) == Environment.ProcessId,
                "forward includes exact hook timestamp, helper PID and caller configuration generation");
            Assert(native.QueryThreads.All(thread => thread != keyboard.InstallThread),
                "foreground, privilege and ownership queries never run on the hook callback thread");
            Assert(host.Configure(Config(2, recording: true)).Success, "recording config accepted");
            keyboard.Tap(0xA2, 1600); keyboard.Tap(0xA2, 1800); keyboard.Drain();
            Assert(native.Forwarded.Count == 1 && keyboard.QuitRequests == 0,
                "recording suppresses callbacks without replacing the still-owned hook thread");
            Assert(host.Configure(Config(3)).Success, "recording exit resets detector");
            native.HoldGestures = true;
            keyboard.Tap(0xA2, 2100); keyboard.Tap(0xA2, 2300); keyboard.Drain();
            Assert(host.Configure(Config(4)).Success, "new configuration retires queued old epoch");
            native.ReleaseGestures();
            keyboard.Tap(0xA2, 2500); keyboard.Tap(0xA2, 2700); keyboard.Drain();
            await Until(() => native.Forwarded.Any(item =>
                unchecked((uint)((ulong)item.Value >> 32)) == 2700));
            Assert(native.Forwarded.Count == 2 && native.Forwarded.Last().Generation == 4,
                "queued old epoch never forwards after configuration reset");
            keyboard.ExitWithError(123);
            await Until(() => native.Forwarded.Any(item => item.Message == ElevatedInputMessages.Unavailable));
            Assert(!host.IsHookInstalled && native.Forwarded.Last().Generation == 4,
                "unexpected hook exit notifies ordinary owner without replacing ordinary health");
        }
        finally { host.Dispose(); }
        Assert(keyboard.Owner?.IsAlive == false && !host.IsHookInstalled &&
            native.OwnedWindow != IntPtr.Zero && !IsWindow(native.OwnedWindow),
            "dispose joins fake hook owner and destroys its self-owned hidden message HWND");
    }

    public static Task PasteTargetAndDestinationValidation()
    {
        using var keyboard = new KeyboardMonitorTests.Platform();
        var native = new Native { Elevated = true };
        var operations = new PasteOperations();
        using var host = new ElevatedInputHost(42, NullPlatformLog.Instance, native, keyboard, operations);
        Assert(!host.Configure(Config(1) with { DestinationWindow = 98 }).Success,
            "destination HWND must belong to authenticated ordinary pipe-server process");
        Assert(host.Paste(new(10, 11, Guid.NewGuid())).Success && operations.Writes == 0 && operations.Shortcuts == 1,
            "same-session elevated target executes shared input stage without clipboard writes");
        Assert(!host.Paste(new(10, 12, Guid.NewGuid())).Success && operations.Shortcuts == 1,
            "foreign focus never reaches activation or SendInput");
        native.SameSession = false;
        Assert(!host.Paste(new(10, 11, Guid.NewGuid())).Success, "cross-session target rejected");
        native.SameSession = true; native.Elevated = false;
        Assert(!host.Paste(new(10, 11, Guid.NewGuid())).Success && operations.Shortcuts == 1,
            "ordinary target cannot enter elevated input execution");
        return Task.CompletedTask;
    }

    public static Task ActivationRequiresConfiguredDesktopOwnership()
    {
        using var keyboard = new KeyboardMonitorTests.Platform();
        var native = new Native { Elevated = true };
        using var host = new ElevatedInputHost(42, NullPlatformLog.Instance, native, keyboard, new PasteOperations());
        Assert(host.Configure(Config(1) with { DesktopProcessId = 55 }).Success,
            "helper binds the original Desktop peer through authenticated configuration");
        var id = Guid.NewGuid();
        Assert(host.Activate(new(20, id)).Success && native.Activations == 1,
            "administrator helper must activate and verify the configured Desktop root window");
        Assert(!host.Activate(new(10, Guid.NewGuid())).Success && native.Activations == 1,
            "an administrator source target cannot be activated through the Desktop-only route");
        Assert(!host.Activate(new(21, Guid.NewGuid())).Success && native.Activations == 1,
            "only a root HWND owned by the current Desktop peer is admitted");
        native.SameSession = false;
        Assert(!host.Activate(new(20, Guid.NewGuid())).Success && native.Activations == 1,
            "cross-session activation is refused");
        native.SameSession = true;
        native.ActivationSucceeds = false;
        Assert(!host.Activate(new(20, Guid.NewGuid())).Success && native.Activations == 2,
            "a failed native foreground/focus readback cannot publish activation success");
        return Task.CompletedTask;
    }

    public static async Task AdministratorPointerUsesOriginalMessageWindow()
    {
        using var keyboard = new KeyboardMonitorTests.Platform();
        var native = new Native { Elevated = true };
        var host = new ElevatedInputHost(42, NullPlatformLog.Instance, native, keyboard, new PasteOperations());
        try
        {
            Assert(host.Configure(Config(1)).Success, "original helper configuration accepted");
            var window = (IntPtr)typeof(ElevatedInputHost)
                .GetField("_windowHandle", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!
                .GetValue(host)!;
            Assert(window != IntPtr.Zero, "original helper message window exists");
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            native.PrimaryDown = true;
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(120);
            Assert(native.Forwarded.Count(item => item.Message == 0x8018) == 1,
                "administrator outside press must be forwarded once by the original helper message window");
            var pointer = native.Forwarded.Single(item => item.Message == 0x8018);
            Assert(pointer.Value == 10 && unchecked((uint)pointer.Generation) == (uint)Environment.ProcessId &&
                unchecked((uint)Environment.TickCount64 - (uint)((ulong)pointer.Generation >> 32)) < 1000,
                "forwarded click preserves the sampled root, helper PID and native timestamp");
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            Assert(native.Forwarded.Count(item => item.Message == 0x8018) == 1,
                "a held button must not repeatedly forward pointer presses");
            native.PrimaryDown = false;
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            native.Elevated = false;
            native.PrimaryDown = true;
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            Assert(native.Forwarded.Count(item => item.Message == 0x8018) == 1,
                "ordinary pointer targets retain the original ordinary sampler");
            native.PrimaryDown = false;
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            native.Elevated = true;
            Assert(host.Configure(Config(2, recording: true)).Success, "recording config accepted");
            native.SecondaryDown = true;
            _ = PostMessage(window, 0x0113, new IntPtr(2), IntPtr.Zero);
            await Task.Delay(40);
            Assert(native.Forwarded.Count(item => item.Message == 0x8018) == 2,
                "administrator secondary presses remain observable while keyboard recording suppresses gestures");
        }
        finally { host.Dispose(); }
        Assert(native.PointerMonitorStopped && native.OwnedWindow != IntPtr.Zero && !IsWindow(native.OwnedWindow),
            "pointer observation ends with the original helper HWND lifetime");
    }

    private static ElevatedInputConfiguration Config(long generation, bool recording = false) =>
        new(99, new(true, recording, QuickLaunchTriggerKind.DoubleControl, generation), Guid.NewGuid());

    private sealed class Native : IElevatedInputNative
    {
        public bool Elevated, SameSession = true;
        public bool PrimaryDown, SecondaryDown, PointerMonitorStopped;
        public bool IsPointerButtonDown(int key) => key == 1 ? PrimaryDown : SecondaryDown;
        public (long WindowHandle, long RootWindowHandle, uint ProcessId) PointerTarget() => (11, 10, 77);
        public void StartPointerMonitor(IntPtr window) => OwnedWindow = window;
        public void StopPointerMonitor(IntPtr window) => PointerMonitorStopped = true;
        public int Activations;
        public bool ActivationSucceeds = true;
        public bool ActivateWindow(IntPtr window) { Activations++; return ActivationSucceeds; }
        public int Decisions;
        public IntPtr OwnedWindow;
        public bool HoldGestures;
        private readonly ConcurrentQueue<(IntPtr Window, int Message, IntPtr WParam, IntPtr LParam)> _held = new();
        public void ReleaseGestures()
        {
            HoldGestures = false;
            while (_held.TryDequeue(out var item))
                _ = PostMessage(item.Window, item.Message, item.WParam, item.LParam);
        }
        public readonly ConcurrentQueue<int> QueryThreads = new();
        public readonly ConcurrentQueue<(int Message, long Value, long Generation)> Forwarded = new();
        public uint WindowProcessId(IntPtr window)
        {
            QueryThreads.Enqueue(Environment.CurrentManagedThreadId);
            return window.ToInt64() switch { 99 => 42, 10 or 11 => 77, 12 => 88, 20 or 21 => 55, _ => 0 };
        }
        public IntPtr RootWindow(IntPtr window) => window.ToInt64() is 10 or 11 ? new(10) : window.ToInt64() == 21 ? new(20) : window;
        public IntPtr ForegroundWindow() { QueryThreads.Enqueue(Environment.CurrentManagedThreadId); return new(10); }
        public bool IsSameSession(uint processId) => SameSession;
        public bool IsElevated(uint processId)
        {
            QueryThreads.Enqueue(Environment.CurrentManagedThreadId);
            var elevated = Elevated;
            Interlocked.Increment(ref Decisions);
            return elevated;
        }
        public bool AllowForeground(uint processId) => false;
        public bool Post(IntPtr window, int message, IntPtr wParam, IntPtr lParam)
        {
            if (window.ToInt64() == 99)
            { Forwarded.Enqueue((message, wParam.ToInt64(), lParam.ToInt64())); return true; }
            OwnedWindow = window;
            if (HoldGestures && message == 0x8014)
            { _held.Enqueue((window, message, wParam, lParam)); return true; }
            return PostMessage(window, message, wParam, lParam);
        }
    }

    [DllImport("user32.dll", EntryPoint = "PostMessageW")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessage(IntPtr window, int message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);
}
