using System.Collections.Concurrent;
using System.ComponentModel;
using System.Diagnostics;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class KeyboardMonitorTests
{
    public static Task DedicatedOwnershipAndDisposal()
    {
        using var platform = new Platform();
        using var gesture = new ManualResetEventSlim();
        var detector = new ModifierDoubleTapDetector(0xA2, 60, 800);
        var callerThread = Environment.CurrentManagedThreadId;
        var callbackThread = 0;
        var monitor = new LowLevelKeyboardMonitor(
            (key, time) => { callbackThread = Environment.CurrentManagedThreadId; detector.OnKeyDown(key, time); },
            key => { if (detector.OnKeyUp(key)) gesture.Set(); }, platform);
        try
        {
            Assert(platform.InstallThread != callerThread, "keyboard hook must install on its dedicated thread, not EventWindow caller");
            var recognition = Task.Run(() =>
            {
                platform.Key(0xA2, true, 100);
                platform.Key(0xA2, false, 110);
                platform.Key(0xA2, true, 300);
                platform.Key(0xA2, false, 310);
                return gesture.Wait(TimeSpan.FromSeconds(1));
            });
            Thread.Sleep(2000); // EventWindow caller does not pump or service the hook.
            Assert(recognition.GetAwaiter().GetResult(), "hook must recognize while EventWindow caller is blocked");
            Assert(callbackThread == platform.InstallThread && platform.LoopThread == platform.InstallThread,
                "install, blocking message loop and callbacks must share the dedicated owner");
        }
        finally { monitor.Dispose(); }
        Assert(platform.Owner?.IsAlive == false, "Dispose must not leave the dedicated thread alive");
        Assert(platform.UninstallThread == platform.InstallThread && platform.LoopExited.IsSet,
            "Dispose must quit and join before same-thread uninstallation completes");
        Assert(!monitor.IsInstalled && platform.QuitRequests == 1, "disposed hook must be unavailable and quit once");
        monitor.Dispose();
        Assert(platform.QuitRequests == 1, "Dispose is idempotent");
        return Task.CompletedTask;
    }

    public static Task ResetGenerationAndInjectedInput()
    {
        using var platform = new Platform();
        var gesture = new QuickLaunchKeyboardGesture();
        var recognized = 0;
        using var monitor = new LowLevelKeyboardMonitor(
            (key, time) => { if (gesture.OnKeyDown(key, time)) recognized++; },
            key => { if (gesture.OnKeyUp(key)) recognized++; }, platform);
        platform.Tap(0xA2, 100);
        platform.Drain();
        gesture.Publish(new(true, false, QuickLaunchTriggerKind.DoubleControl, 1));
        platform.Tap(0xA2, 300);
        platform.Drain();
        Assert(recognized == 0, "next hook callback must reset the old first tap after publication");
        platform.Tap(0xA2, 500);
        platform.Drain();
        Assert(recognized == 1, "two post-reset taps must retain the complete-release gesture");

        gesture.Publish(new(true, false, QuickLaunchTriggerKind.DoubleControl, 2));
        platform.Tap(0xA2, 1000, 0x10);
        platform.Tap(0xA2, 1200, 0x02);
        platform.Drain();
        Assert(recognized == 1, "injected events must not enter the detector");
        platform.Tap(0xA2, 1400);
        platform.Key(0xA2, true, 1600);
        platform.Drain();
        gesture.Publish(new(false, true, QuickLaunchTriggerKind.DoubleControl, 3));
        platform.Key(0xA2, false, 1610);
        platform.Drain();
        Assert(recognized == 1, "recording reset on key-up must cancel an armed gesture");
        gesture.Publish(new(true, false, QuickLaunchTriggerKind.HotKey, 4));
        platform.Tap(0xA2, 1800);
        platform.Tap(0xA2, 2000);
        platform.Drain();
        Assert(recognized == 1, "custom hotkey mode must not emit double Ctrl");
        gesture.Publish(new(true, false, QuickLaunchTriggerKind.DoubleControl, 5));
        platform.Tap(0xA2, 2200);
        platform.Tap(0xA3, 2400);
        platform.Tap(0xA3, 2600);
        platform.Drain();
        Assert(recognized == 2, "left and right Ctrl have independent tap sequences");
        return Task.CompletedTask;
    }

    public static Task ObservableFailureRetryAndThreadExit()
    {
        using var platform = new Platform { InstallFailures = 1 };
        using var health = new BlockingCollection<QuickLaunchKeyboardHealthChangedEvent>();
        var log = new Log();
        var initialId = Guid.NewGuid();
        using var monitor = new LowLevelKeyboardMonitor((_, _) => { }, _ => { }, platform,
            healthChanged: health.Add, log: log, correlationId: initialId);
        Assert(health.TryTake(out var failed, 1000) && failed is
            { State: QuickLaunchKeyboardHealthState.InstallFailed, NativeErrorCode: 5 } && failed.CorrelationId == initialId,
            "native install failure must retain correlation and native error");
        Assert(!monitor.IsInstalled && monitor.LastNativeErrorCode == 5, "failed monitor is unavailable");
        var retryId = Guid.NewGuid();
        Assert(monitor.TryInstall(retryId), "unavailable monitor must install on explicit retry");
        Assert(health.TryTake(out var restored, 1000) && restored is
            { State: QuickLaunchKeyboardHealthState.Reinstalled, NativeErrorCode: 0 } && restored.CorrelationId == retryId,
            "reinstallation must publish the retry correlation exactly once");
        Assert(monitor.TryInstall(Guid.NewGuid()) && health.Count == 0, "healthy retry is idempotent");
        var logCount = log.Events.Count;
        platform.Tap(0xA2, 100);
        platform.Tap(0xA2, 300);
        platform.Drain();
        Assert(log.Events.Count == logCount, "key callbacks must not log");
        platform.ExitWithError(123);
        Assert(health.TryTake(out var exited, 1000) && exited is
            { State: QuickLaunchKeyboardHealthState.ThreadExited, NativeErrorCode: 123 } && exited.CorrelationId == retryId,
            "owned thread exit must publish native failure, not claim silent removal detection");
        Assert(!monitor.IsInstalled, "thread exit must clear observable availability");
        // Hello may have advertised availability before this failure; session queue discard cannot erase it.
        Assert(monitor.HealthSnapshot == exited, "current native lifecycle fact survives consuming the event queue");
        var sessionSnapshot = PlatformPipeClient.CreateKeyboardHealthSnapshotEnvelope(monitor.HealthSnapshot, true);
        Assert(sessionSnapshot?.Payload.GetProperty("data").GetProperty("state").GetString() == "thread_exited" &&
            sessionSnapshot.CorrelationId == retryId, "post-HelloAck snapshot must preserve a hook exit discarded during handshake");
        Assert(PlatformPipeClient.CreateKeyboardHealthSnapshotEnvelope(monitor.HealthSnapshot, false) is null,
            "post-handshake snapshot also requires capability negotiation");
        Assert(monitor.TryInstall(Guid.NewGuid()), "thread exit must allow explicit reinstallation");
        Assert(health.TryTake(out var secondRestore, 1000) && secondRestore.State == QuickLaunchKeyboardHealthState.Reinstalled,
            "a new owned thread must publish recovery");
        var stopWatch = Stopwatch.StartNew();
        monitor.Dispose();
        Assert(stopWatch.Elapsed < TimeSpan.FromSeconds(3) && platform.Owner?.IsAlive == false,
            "cooperative Dispose is bounded and joins the actual thread");
        Assert(health.Count == 0, "intentional disposal must not publish unexpected thread exit");
        Assert(log.Events.All(item => item.Correlation is not null && item.Fields.Keys.All(key =>
            key is "native_error_code" or "elapsed_ms")), "hook diagnostics must be correlated metadata only");
        return Task.CompletedTask;
    }

    public static Task HealthWireAndCapabilityNegotiation()
    {
        var available = PlatformCapabilities.Create(false, true);
        var unavailable = PlatformCapabilities.Create(false, false);
        Assert(available.Contains(PlatformCapabilities.QuickLaunchPanelHotKey) &&
            !unavailable.Contains(PlatformCapabilities.QuickLaunchPanelHotKey) &&
            unavailable.Contains(PlatformCapabilities.QuickLaunchKeyboardHealth),
            "Hello must separate current trigger availability from health protocol support");
        foreach (var (state, wire) in new[] {
            (QuickLaunchKeyboardHealthState.InstallFailed, "install_failed"),
            (QuickLaunchKeyboardHealthState.ThreadExited, "thread_exited"),
            (QuickLaunchKeyboardHealthState.Reinstalled, "reinstalled") })
        {
            var id = Guid.NewGuid();
            var health = new QuickLaunchKeyboardHealthChangedEvent(id, state, state == QuickLaunchKeyboardHealthState.Reinstalled ? 0 : 5);
            var envelope = PlatformEventEnvelopeFactory.Create(health);
            var decoded = JsonSerializer.Deserialize<ProtocolEnvelope>(JsonSerializer.Serialize(envelope, JsonDefaults.Options), JsonDefaults.Options)!;
            decoded.Validate();
            var data = decoded.Payload.GetProperty("data");
            Assert(decoded.CorrelationId == id && decoded.Payload.GetProperty("event").GetString() == "quick_launch_keyboard_health_changed" &&
                data.GetProperty("correlation_id").GetGuid() == id && data.GetProperty("state").GetString() == wire &&
                data.GetProperty("native_error_code").GetInt32() == health.NativeErrorCode, "typed health wire round trip must preserve all native facts");
            var oldAck = ProtocolEnvelope.Create("hello_ack", new { AcceptedCapabilities = new[] { "quick_launch_configuration" } });
            var newAck = ProtocolEnvelope.Create("hello_ack", new { AcceptedCapabilities = new[] { PlatformCapabilities.QuickLaunchKeyboardHealth } });
            Assert(!PlatformPipeClient.AcceptsKeyboardHealth(oldAck) && PlatformPipeClient.AcceptsKeyboardHealth(newAck),
                "only explicit peer capability acceptance enables health events");
            Assert(!PlatformPipeClient.CanForwardEvent(health, false) && PlatformPipeClient.CanForwardEvent(health, true) &&
                PlatformPipeClient.CanForwardEvent(new ForegroundChangedEvent(new(0, 0, 0, "", "")), false),
                "old peers must not receive unknown health events; existing events remain unchanged");
        }
        return Task.CompletedTask;
    }

    public static async Task HealthSnapshotSurvivesFullQueue()
    {
        using var cancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        var events = Channel.CreateBounded<WindowsPlatformEvent>(new BoundedChannelOptions(512)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = true
        });
        var ordinary = Enumerable.Range(0, 512).Select(index =>
            (WindowsPlatformEvent)new SystemThemePreferenceChangedEvent(Guid.NewGuid(), index % 2 == 0, 0)).ToArray();
        foreach (var item in ordinary)
            Assert(events.Writer.TryWrite(item), "fixture must fill the original native queue capacity");
        var failed = new QuickLaunchKeyboardHealthChangedEvent(Guid.NewGuid(), QuickLaunchKeyboardHealthState.ThreadExited, 123);
        Assert(!events.Writer.TryWrite(failed), "full native queue must reject the nonblocking health publication");
        events.Writer.Complete();
        var forwarded = await Read(events.Reader, () => failed, true, null);
        Assert(forwarded.FirstOrDefault() == failed,
            "resumed current-session reader must forward retained health even when the native queue rejected it");
        Assert(forwarded.OfType<QuickLaunchKeyboardHealthChangedEvent>().SequenceEqual(new[] { failed }),
            "one retained lifecycle fact must be forwarded exactly once");
        Assert(forwarded.Where(item => item is not QuickLaunchKeyboardHealthChangedEvent).SequenceEqual(ordinary),
            "health recovery must preserve ordinary event order");

        var restored = new QuickLaunchKeyboardHealthChangedEvent(Guid.NewGuid(), QuickLaunchKeyboardHealthState.Reinstalled, 0);
        var stale = Channel.CreateBounded<WindowsPlatformEvent>(4);
        Assert(stale.Writer.TryWrite(failed) && stale.Writer.TryWrite(ordinary[0]) && stale.Writer.TryWrite(restored),
            "fixture must queue older health before current recovery");
        stale.Writer.Complete();
        var recovered = await Read(stale.Reader, () => restored, true, null);
        Assert(recovered.SequenceEqual(new WindowsPlatformEvent[] { restored, ordinary[0] }),
            "queued stale failure must not replace the latest retained recovery or duplicate it");

        var initial = Channel.CreateBounded<WindowsPlatformEvent>(2);
        Assert(initial.Writer.TryWrite(restored) && initial.Writer.TryWrite(ordinary[1]), "fixture must retain handshake duplicate");
        initial.Writer.Complete();
        var deduplicated = await Read(initial.Reader, () => restored, true, restored);
        Assert(deduplicated.SequenceEqual(new[] { ordinary[1] }),
            "health already sent by the handshake must not be forwarded again");

        var legacy = Channel.CreateBounded<WindowsPlatformEvent>(2);
        Assert(legacy.Writer.TryWrite(failed) && legacy.Writer.TryWrite(ordinary[2]), "fixture must include legacy-peer controls");
        legacy.Writer.Complete();
        var compatible = await Read(legacy.Reader, () => failed, false, null);
        Assert(compatible.SequenceEqual(new[] { ordinary[2] }),
            "peers without health capability must receive only existing events");

        async Task<List<WindowsPlatformEvent>> Read(ChannelReader<WindowsPlatformEvent> reader,
            Func<QuickLaunchKeyboardHealthChangedEvent?> snapshot, bool accepted,
            QuickLaunchKeyboardHealthChangedEvent? lastSent)
        {
            var result = new List<WindowsPlatformEvent>();
            await foreach (var item in PlatformPipeClient.ReadEventsWithKeyboardHealthSnapshots(
                reader, snapshot, accepted, lastSent, cancellation.Token))
                result.Add(item);
            return result;
        }
    }

    private sealed class Log : IPlatformLog
    {
        public readonly ConcurrentQueue<(string Event, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Events = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Events.Enqueue((eventName, correlationId, fields ?? new Dictionary<string, object?>()));
    }

    internal sealed class Platform : IKeyboardHookPlatform, IDisposable
    {
        private readonly BlockingCollection<Action?> _messages = new();
        private KeyboardHookProcedure? _procedure;
        public int InstallThread;
        public int LoopThread;
        public int UninstallThread;
        public int QuitRequests;
        public int InstallFailures;
        public Thread? Owner;
        public readonly ManualResetEventSlim LoopExited = new();
        public uint InitializeThread() => (uint)Environment.CurrentManagedThreadId;
        public IntPtr Install(KeyboardHookProcedure procedure)
        {
            InstallThread = Environment.CurrentManagedThreadId;
            Owner = Thread.CurrentThread;
            if (InstallFailures > 0) { InstallFailures--; throw new Win32Exception(5); }
            _procedure = procedure;
            return new IntPtr(1);
        }
        public void RunMessageLoop()
        {
            LoopThread = Environment.CurrentManagedThreadId;
            try { foreach (var work in _messages.GetConsumingEnumerable()) { if (work is null) break; work(); } }
            finally { LoopExited.Set(); }
        }
        public bool PostQuit(uint threadId) { Interlocked.Increment(ref QuitRequests); _messages.Add(null); return true; }
        public void Uninstall(IntPtr hook) => UninstallThread = Environment.CurrentManagedThreadId;
        public IntPtr CallNext(IntPtr hook, int code, IntPtr message, IntPtr data) => IntPtr.Zero;
        public void Tap(uint key, uint timestamp, uint flags = 0) { Key(key, true, timestamp, flags); Key(key, false, timestamp + 10, flags); }
        public void ExitWithError(int nativeError) => _messages.Add(() => throw new Win32Exception(nativeError));
        public void Drain()
        {
            var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            _messages.Add(() => completion.SetResult());
            Assert(completion.Task.Wait(TimeSpan.FromSeconds(1)), "dedicated loop did not consume queued input");
        }
        public void Key(uint key, bool down, uint timestamp, uint flags = 0) => _messages.Add(() =>
        {
            var pointer = Marshal.AllocHGlobal(Marshal.SizeOf<KeyboardHookData>());
            try
            {
                Marshal.StructureToPtr(new KeyboardHookData { VirtualKey = key, Timestamp = timestamp, Flags = flags }, pointer, false);
                _procedure!(0, new IntPtr(down ? 0x0100 : 0x0101), pointer);
            }
            finally { Marshal.FreeHGlobal(pointer); }
        });
        public void Dispose() { _messages.Dispose(); LoopExited.Dispose(); }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
