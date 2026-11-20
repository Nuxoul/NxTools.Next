using System.Collections.Concurrent;
using System.IO.Pipes;
using System.Text;
using System.Threading.Channels;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ProcessWindowPickTests
{
    private static readonly Guid Session = Guid.Parse("7ec90369-2ac0-47e7-82ec-9764719b12f4");
    private static ProtocolEnvelope Wire(string data) => ProtocolEnvelope.Create("platform_command", new
    { command = "process_window_pick", data = JsonSerializer.Deserialize<JsonElement>(data) });
    private static ProcessWindowPickCommand StartCommand() => new("start", Session, 10, 20);
    private static void Check(bool value, string reason) { if (!value) throw new InvalidOperationException(reason); }

    internal static Task StrictWire()
    {
        var prefix = $"\"session_id\":\"{Session}\"";
        Check(ProcessWindowPickContract.Parse(Wire($"{{\"action\":\"start\",{prefix},\"source_window_handle\":10,\"overlay_window_handle\":20}}")) == StartCommand(), "exact start fields");
        foreach (var action in new[] { "show_overlay", "cancel" })
            Check(ProcessWindowPickContract.Parse(Wire($"{{\"action\":\"{action}\",{prefix}}}")).Action == action, "exact simple action");
        foreach (var data in new[] { "{}", $"{{\"action\":\"unknown\",{prefix}}}", $"{{\"action\":\"start\",{prefix}}}",
            $"{{\"action\":\"cancel\",{prefix},\"source_window_handle\":10}}", $"{{\"action\":\"cancel\",{prefix},\"action\":\"cancel\"}}",
            "{\"action\":\"cancel\",\"session_id\":\"00000000-0000-0000-0000-000000000000\"}",
            $"{{\"action\":\"start\",{prefix},\"source_window_handle\":10,\"overlay_window_handle\":10}}" })
        {
            try { ProcessWindowPickContract.Parse(Wire(data)); throw new Exception("malformed shape accepted"); }
            catch (InvalidDataException) { }
        }
        return Task.CompletedTask;
    }

    internal static Task OwnershipAndStartupFailure()
    {
        foreach (var mode in new[] { "source_hidden", "source_foreign", "overlay_foreign", "overlay_visible", "prepare", "hooks" })
        {
            using var fixture = new Fixture(); fixture.Native.Failure = mode;
            var result = fixture.Controller.Execute(StartCommand(), 42, CancellationToken.None);
            Check(!result.Success && !fixture.Controller.Occupied && fixture.Native.Hooks == 0, "startup failure releases admission/hooks");
            Check(!fixture.Native.Visible && fixture.Native.ShowCalls == 0, "startup never reveals overlay");
            if (mode == "hooks") Check(result.NativeErrorCode == 5 && fixture.Native.HideCalls > 0, "failed partial hook install preserves native code and hides");
        }
        return Task.CompletedTask;
    }

    internal static Task ViewportHoverAndNames()
    {
        using var f = new Fixture(); f.Start();
        f.Controller.Tick(); f.Controller.Tick();
        Check(f.Events.Count == 1 && f.Events[0].Viewport == f.Native.Viewport && f.Events[0].Candidate?.ProcessName == "private-app.exe", "hover carries negative desktop origin and actual executable");
        Check(f.Native.Resolves == 1 && f.Native.PreparedViewport == f.Native.Viewport, "unchanged hover coalesces before process IO");
        f.Native.Hit = f.Native.Hit! with { Left = -501 }; f.Controller.Tick();
        Check(f.Events.Count == 2 && f.Events[1].Candidate?.Left == -501, "rectangle change produces hover");
        f.Native.Name = null; f.Native.Hit = f.Native.Hit! with { WindowHandle = 31 }; f.Controller.Tick();
        Check(f.Events[^1].Candidate is null && f.Events[^1].NativeErrorCode == 5 && f.Events[^1].Viewport is not null, "denied process query is meaningful no-selection, not fabricated name");
        f.Native.Hit = null; f.Controller.Tick();
        Check(f.Events[^1].Candidate is null, "no eligible root clears highlight");
        return Task.CompletedTask;
    }

    internal static Task ClickAndReleaseSuppression()
    {
        using var f = new Fixture(); f.Native.LeftPressed = true; f.Start();
        Check(!f.Controller.Input(ProcessPickInput.LeftUp, 30), "initial initiating release passes through");
        Check(!f.Controller.Input(ProcessPickInput.LeftDown, 10) && !f.Controller.Input(ProcessPickInput.LeftUp, 10), "settings source is pass-through");
        f.Controller.Tick();
        f.Native.Hit = f.Native.Hit! with { WindowHandle = 31, ProcessId = 71 };
        Check(f.Controller.Input(ProcessPickInput.LeftDown, 31), "fresh clicked target down is swallowed");
        Check(f.Native.Resolves == 1 && f.Events.Count == 1, "hook decision performs no process IO or publication");
        Check(!f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "a click already captured by hook cannot reveal a late frame");
        f.Controller.Tick();
        Check(f.Events[^1].State == "selected" && f.Events[^1].Candidate?.WindowHandle == 31 && f.Events[^1].Candidate?.ProcessId == 71, "selection resolves click root, not cached hover");
        Check(f.Controller.Occupied && f.Native.Hooks == 1 && !f.Native.Visible, "terminal hides but retains intercepted release owner");
        Check(!f.Controller.Execute(new("start", Guid.NewGuid(), 10, 20), 42, CancellationToken.None).Success, "no replacement before release cleanup");
        Check(f.Controller.Input(ProcessPickInput.LeftUp, 31), "matching target release is swallowed after terminal");
        f.Controller.Tick();
        Check(!f.Controller.Occupied && f.Native.Hooks == 0 && f.Events.Count(e => e.State == "selected") == 1, "one terminal and hook cleanup");
        return Task.CompletedTask;
    }

    internal static Task EscapeAndCancellation()
    {
        using var f = new Fixture(); f.Start();
        Check(f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success && f.Native.Visible, "current rendered frame may reveal");
        Check(f.Controller.Input(ProcessPickInput.EscapeDown, 0), "Escape down swallowed");
        Check(!f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "Escape retires reveal identity before deferred cancellation publication");
        Check(f.Events.Count == 0, "hook emits nothing");
        f.Controller.Tick();
        Check(f.Events.Single().State == "cancelled" && !f.Native.Visible && f.Controller.Occupied, "Escape terminal retains key-up owner");
        Check(f.Controller.Input(ProcessPickInput.EscapeDown, 0) && f.Controller.Input(ProcessPickInput.EscapeUp, 0), "Escape repeat and up swallowed");
        f.Controller.Tick(); Check(!f.Controller.Occupied, "Escape cleanup");
        Check(!f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "retired frame cannot reveal");
        return Task.CompletedTask;
    }

    internal static Task StaleCancelAndBoundedAdmission()
    {
        using var f = new Fixture(); f.Start();
        var next = Guid.NewGuid();
        Check(!f.Controller.Execute(new("start", next, 10, 20), 42, CancellationToken.None).Success, "single bounded session");
        Check(f.Controller.Execute(new("cancel", next), 42, CancellationToken.None).Success && f.Controller.Occupied, "foreign cancellation never retires current");
        f.Controller.Execute(new("cancel", Session), 42, CancellationToken.None);
        Check(!f.Controller.Occupied && !f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "cancel before late show");
        Check(f.Controller.Execute(new("start", next, 10, 20), 42, CancellationToken.None).Success, "cleaned slot reusable");
        f.Controller.Execute(new("cancel", Session), 42, CancellationToken.None);
        Check(f.Controller.Occupied && f.Events.Count == 1, "old session cancel does not affect replacement");
        return Task.CompletedTask;
    }

    internal static Task OwnerDisconnectDisposeAndDeadline()
    {
        foreach (var reason in new[] { "owner", "disconnect", "dispose", "deadline" })
        {
            using var f = new Fixture(); using var cancel = new CancellationTokenSource();
            Check(f.Controller.Execute(StartCommand(), 42, cancel.Token).Success, "admitted");
            if (reason == "owner") f.Native.OwnerAlive = false;
            if (reason == "disconnect") cancel.Cancel();
            if (reason == "dispose") f.Controller.Stop();
            if (reason == "deadline") f.Native.Now = 120_001;
            Check(!f.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "cancelled/dead/expired owner rejects reveal before timer cleanup");
            f.Controller.Tick();
            Check(!f.Controller.Occupied && f.Native.Hooks == 0 && !f.Native.Visible && f.Events.Count == 1 && f.Events[0].State is "cancelled" or "failed", "lifetime ends once and cleans");
        }
        using var retained = new Fixture(); retained.Start();
        retained.Controller.Input(ProcessPickInput.LeftDown, 30); retained.Controller.Tick(); retained.Controller.Stop();
        Check(retained.Controller.Occupied, "dispose does not forward intercepted release");
        retained.Controller.Input(ProcessPickInput.LeftUp, 30); retained.Controller.Tick();
        Check(!retained.Controller.Occupied, "dispose drains release then cleans");
        return Task.CompletedTask;
    }

    internal static Task DiagnosticsAndEventEnvelope()
    {
        using var f = new Fixture(); f.Start(); f.Controller.Tick();
        var envelope = PlatformEventEnvelopeFactory.Create(f.Events.Single());
        Check(envelope.ProtocolVersion == 1 && envelope.Type == "platform_event" && envelope.CorrelationId == Session, "original envelope version and session correlation");
        Check(envelope.Payload.GetProperty("event").GetString() == "process_window_pick_changed", "exact event spelling");
        var data = envelope.Payload.GetProperty("data");
        Check(data.GetProperty("session_id").GetGuid() == Session && data.GetProperty("state").GetString() == "hover" && data.GetProperty("viewport").GetProperty("left").GetInt32() == -1920, "exact snake case viewport");
        Check(data.GetProperty("candidate").GetProperty("process_name").GetString() == "private-app.exe", "candidate executable fact");
        f.Controller.Execute(new("cancel", Session), 42, CancellationToken.None);
        var terminal = PlatformEventEnvelopeFactory.Create(f.Events[^1]).Payload.GetProperty("data");
        Check(!terminal.TryGetProperty("candidate", out _) && !terminal.TryGetProperty("viewport", out _), "optional absent terminal facts omitted");
        Check(f.Log.Events.Select(e => e.Name).SequenceEqual(new[] { "blacklist.window_pick.accepted", "blacklist.window_pick.started", "blacklist.window_pick.completed" }), "finite accepted/started/terminal only, never hover");
        Check(f.Log.Events.All(e => e.Id == Session && e.Fields.Keys.Order().SequenceEqual(new[] { "elapsed_ms", "native_error_code", "reason", "state" })), "diagnostics exact payload-free fields");
        Check(!JsonSerializer.Serialize(f.Log.Events).Contains("private-app"), "no process names in diagnostics");
        Check(!PlatformPipeClient.ShouldLogCommandLifecycle("process_window_pick"), "render-qualified show commands never turn hover into generic command logs");
        return Task.CompletedTask;
    }

    internal static async Task ActualPipeRouteAndReader()
    {
        using var f = new Fixture();
        var adapter = (WindowsPlatformAdapter)RuntimeHelpers.GetUninitializedObject(typeof(WindowsPlatformAdapter));
        var coordinator = new ProcessWindowPickCoordinator(f.Controller, action => { action(); return true; });
        typeof(WindowsPlatformAdapter).GetField("_processWindowPick", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(adapter, coordinator);
        var client = new PlatformPipeClient("fixture-unused", adapter, NullPlatformLog.Instance);
        var execute = typeof(PlatformPipeClient).GetMethod("ExecutePlatformCommandAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
        var wire = Wire($"{{\"action\":\"start\",\"session_id\":\"{Session}\",\"source_window_handle\":10,\"overlay_window_handle\":20}}");
        var result = await (Task<WindowsCommandResult>)execute.Invoke(client, [wire, "process_window_pick", Session, CancellationToken.None, 42u])!;
        Check(result.Success && f.Controller.Occupied, "actual pipe route reaches injected native controller with connected peer");
        var reply = PlatformPipeClient.CreateCommandResultEnvelope(wire, result);
        Check(reply.CorrelationId == wire.Id && reply.Payload.GetProperty("success").GetBoolean(), "original command ACK");
        Check(PlatformPipeClient.ResolveDiagnosticCorrelationId(wire) == Session, "session rather than request correlation");
        f.Controller.Tick();
        f.Controller.Input(ProcessPickInput.LeftDown, 30); f.Controller.Tick();
        Check(f.Controller.Input(ProcessPickInput.LeftUp, 30), "selected release suppressed before pipe projection"); f.Controller.Tick();
        await AssertPipeEvents(adapter, client, reply, f.Events);
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var order = new List<string>();
        await dispatcher.DispatchAsync("process_window_pick", () => { order.Add("admitted"); return completion.Task; });
        await dispatcher.DispatchAsync("later", () => { order.Add("later"); return Task.CompletedTask; });
        Check(order.SequenceEqual(new[] { "admitted", "later" }), "bounded inline admission, asynchronous ACK never blocks reader");
        completion.SetResult(); await dispatcher.DrainAsync();
    }

    private static async Task AssertPipeEvents(WindowsPlatformAdapter adapter, PlatformPipeClient client,
        ProtocolEnvelope reply, IReadOnlyList<ProcessWindowPickChangedEvent> events)
    {
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        var source = Channel.CreateBounded<WindowsPlatformEvent>(8);
        typeof(WindowsPlatformAdapter).GetField("_events", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(adapter, source);
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(8);
        foreach (var item in events) Check(source.Writer.TryWrite(item), "owned event queue");
        source.Writer.Complete();
        var forward = typeof(PlatformPipeClient).GetMethod("ForwardEventsAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
        await (Task)forward.Invoke(client, [outgoing.Writer, new ConcurrentDictionary<Guid, ClipboardCapturedEvent>(), false, null, deadline.Token])!;
        var pipeName = "nxtools-window-pick-fixture-" + Guid.NewGuid().ToString("N");
        using var server = new NamedPipeServerStream(pipeName, PipeDirection.Out, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        using var readerPipe = new NamedPipeClientStream(".", pipeName, PipeDirection.In, PipeOptions.Asynchronous);
        var connected = server.WaitForConnectionAsync(deadline.Token);
        await readerPipe.ConnectAsync(deadline.Token); await connected;
        using var writer = new StreamWriter(server, new UTF8Encoding(false), 4096, leaveOpen: true) { AutoFlush = true };
        var write = typeof(PlatformPipeClient).GetMethod("WriteAsync", BindingFlags.Static | BindingFlags.NonPublic)!;
        var frameReader = new BoundedFrameReader(readerPipe);
        var readingAck = frameReader.ReadAsync(deadline.Token);
        await (Task)write.Invoke(null, [writer, reply, deadline.Token])!;
        var ack = JsonSerializer.Deserialize<ProtocolEnvelope>((await readingAck)!, JsonDefaults.Options)!;
        Check(ack.Type == "platform_command_result" && ack.Payload.GetProperty("success").GetBoolean(), "actual original ACK over owned pipe");
        var received = new List<string>();
        while (outgoing.Reader.TryRead(out var envelope))
        {
            var readingEvent = frameReader.ReadAsync(deadline.Token);
            await (Task)write.Invoke(null, [writer, envelope, deadline.Token])!;
            var frame = JsonSerializer.Deserialize<ProtocolEnvelope>((await readingEvent)!, JsonDefaults.Options)!;
            var data = frame.Payload.GetProperty("data");
            Check(frame.ProtocolVersion == 1 && frame.Type == "platform_event" && frame.CorrelationId == Session &&
                frame.Payload.GetProperty("event").GetString() == "process_window_pick_changed" && data.GetProperty("session_id").GetGuid() == Session,
                "production event forwarding/framing preserves exact opt-in protocol1 shape");
            received.Add(data.GetProperty("state").GetString()!);
            if (received[^1] == "hover") Check(data.GetProperty("viewport").GetProperty("top").GetInt32() == -200, "negative viewport survives real pipe");
            if (received[^1] == "selected") Check(data.GetProperty("candidate").GetProperty("process_name").GetString() == "private-app.exe", "selected candidate required over real pipe");
        }
        Check(received.SequenceEqual(new[] { "hover", "selected" }), "event ordering is hover then one selected");
    }

    internal static Task NativePolicyAndHookBoundary()
    {
        var api = new FakeApi();
        var native = new ProcessWindowPickNative(api);
        Check(native.ValidateOwners(10, 20, 42), "actual ownership policy accepts connected visible root and hidden same-process overlay");
        foreach (var facts in new[] { api.Windows[10] with { Visible = false }, api.Windows[10] with { ProcessId = 99 },
            api.Windows[10] with { Root = 11 }, api.Windows[10] with { Exists = false }, api.Windows[10] with { Cloaked = true } })
        {
            var original = api.Windows[10]; api.Windows[10] = facts;
            Check(!native.ValidateOwners(10, 20, 42), "source identity facts are enforced by production native policy"); api.Windows[10] = original;
        }
        api.Windows[20] = api.Windows[20] with { ProcessId = 99 };
        Check(!native.ValidateOwners(10, 20, 42), "overlay must share actual source PID");
        api.Windows[20] = api.Windows[20] with { ProcessId = 42 };
        Check(!native.ValidateOwners(10, 10, 42) && !native.ValidateOwners(10, 20, 0), "same HWND and missing peer rejected");
        api.Windows[10] = api.Windows[10] with { ProcessId = 70 }; api.Windows[20] = api.Windows[20] with { ProcessId = 70 };
        Check(!native.ValidateOwners(10, 20, 70), "PlatformHost itself cannot become the Desktop visibility owner");
        api.Windows[10] = api.Windows[10] with { ProcessId = 42 }; api.Windows[20] = api.Windows[20] with { ProcessId = 42 };
        var events = new List<ProcessWindowPickChangedEvent>();
        var controller = new ProcessWindowPickController(native, e => { events.Add(e); return true; }, NullPlatformLog.Instance);
        Check(controller.Execute(StartCommand(), 42, CancellationToken.None).Success, "production native preparation with injected OS calls");
        Check(!api.Windows[10].Visible && !api.Windows[11].Visible, "admitted picker hides settings and main Desktop windows");
        Check(api.Mutations.All(m => m is 10 or 11 or 13 or 14 or 20) && api.Styles.Keys.All(k => k.Item1 == 20) &&
            !api.Windows[20].Visible && api.Windows[20].Left == -1920 && api.Windows[20].Top == -200,
            "only overlay is styled/placed; source windows receive visibility-only changes");
        Check((api.Styles[(20, -20)] & 0x080800A0) == 0x080800A0 && (api.Styles[(20, -16)] & 0x00CF0000) == 0,
            "layered transparent noactivate toolwindow and borderless native style");
        Check(api.LayeredInitialized, "layered alpha initialized while hidden before native placement");
        foreach (var root in new long[] { 0, 10, 20, 40, 41, 42, 43, 44 })
        { api.PointerRoot = root; Check(!native.HandleMouse(true, -100, 100), "ineligible/source/overlay down is passed through"); }
        api.PointerRoot = 30;
        Check(native.HandleMouse(true, -100, 100) && api.ProcessReads == 0 && events.Count == 0, "actual native callback has no process reads or IPC");
        controller.Tick();
        Check(events.Single().Candidate?.WindowHandle == 30 && events.Single().State == "selected" && api.Installed.Count == 2, "actual clicked root selected with both releases owned");
        Check(native.HandleMouse(false, -100, 100), "native matching release swallowed"); controller.Tick();
        Check(api.Installed.Count == 0 && !controller.Occupied && !api.Windows[20].Visible, "actual native hook handles removed after release");
        NativeDesktopVisibilityLifecycle();
        return Task.CompletedTask;
    }

    internal static Task NativePartialInstallAndLostRelease()
    {
        foreach (var hook in new[] { 13, 14 })
        {
            var api = new FakeApi { FailedHook = hook };
            var controller = new ProcessWindowPickController(new ProcessWindowPickNative(api), _ => true, NullPlatformLog.Instance);
            var result = controller.Execute(StartCommand(), 42, CancellationToken.None);
            Check(!result.Success && result.NativeErrorCode == 5 && api.Installed.Count == 0 && !api.Windows[20].Visible,
                "actual first/second hook failure removes every installed handle and hides overlay");
        }
        var mismatchApi = new FakeApi { ClientMismatch = true };
        var mismatch = new ProcessWindowPickController(new ProcessWindowPickNative(mismatchApi), _ => true, NullPlatformLog.Instance)
            .Execute(StartCommand(), 42, CancellationToken.None);
        Check(!mismatch.Success && mismatch.NativeErrorCode == 13 && mismatchApi.Installed.Count == 0 && !mismatchApi.Windows[20].Visible,
            "client/outer viewport mismatch fails before hooks or reveal");
        var layeredApi = new FakeApi { FailLayered = true };
        var layered = new ProcessWindowPickController(new ProcessWindowPickNative(layeredApi), _ => true, NullPlatformLog.Instance)
            .Execute(StartCommand(), 42, CancellationToken.None);
        Check(!layered.Success && layered.NativeErrorCode == 5 && layeredApi.Installed.Count == 0 && !layeredApi.Windows[20].Visible,
            "layered initialization failure never installs hooks or reveals overlay");
        using var f = new Fixture(); f.Start();
        f.Native.LeftPressed = true; f.Controller.Input(ProcessPickInput.LeftDown, 30); f.Controller.Tick();
        f.Native.Now = 3000; f.Controller.Tick();
        Check(f.Controller.Occupied, "held intercepted input retains original owner, no replacement worker");
        f.Native.LeftPressed = false; f.Controller.Tick();
        Check(!f.Controller.Occupied && f.Native.Hooks == 0, "lost release cleans only after native physical-up observation");
        using var bounded = new Fixture(); bounded.Start(); bounded.Native.LeftPressed = true;
        bounded.Controller.Input(ProcessPickInput.LeftDown, 30); bounded.Controller.Tick(); bounded.Native.Now = 120_001; bounded.Controller.Tick();
        Check(bounded.Controller.Occupied && !bounded.Controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success,
            "deadline cannot reveal or replace the release-only owner");
        Check(bounded.Controller.Input(ProcessPickInput.LeftUp, 30), "deadline still swallows intercepted release"); bounded.Controller.Tick();
        Check(!bounded.Controller.Occupied && bounded.Native.Hooks == 0, "deadline release cleanup");
        NativeVisibilityRollback();
        return Task.CompletedTask;
    }

    internal static Task CancelBeforeProcessingAndFailureFacts()
    {
        using var f = new Fixture(); f.Start();
        Check(f.Controller.Input(ProcessPickInput.LeftDown, 30), "intercepted target down");
        f.Controller.Execute(new("cancel", Session), 42, CancellationToken.None);
        Check(f.Events.Single().State == "cancelled" && f.Native.Resolves == 0 && f.Controller.Occupied, "cancel before deferred target resolution wins without process IO");
        Check(f.Controller.Input(ProcessPickInput.LeftUp, 30), "cancel still swallows release"); f.Controller.Tick();
        Check(!f.Controller.Occupied && f.Events.Count == 1, "one cancelled terminal");
        using var denied = new Fixture(); denied.Start(); denied.Native.Name = null;
        denied.Controller.Input(ProcessPickInput.LeftDown, 30); denied.Controller.Tick();
        Check(denied.Events.Single().State == "failed" && denied.Events.Single().Candidate is null && denied.Events.Single().NativeErrorCode == 5,
            "selected denied process never fabricates executable name");
        denied.Controller.Input(ProcessPickInput.LeftUp, 30); denied.Controller.Tick();
        var native = new FakeNative();
        var throwing = new ProcessWindowPickController(native, _ => false, new ThrowingLog());
        Check(throwing.Execute(StartCommand(), 42, CancellationToken.None).Success, "diagnostic failure cannot change admission");
        throwing.Tick(); Check(!throwing.Occupied && native.Hooks == 0, "event backpressure fails with input cleanup and no diagnostic dependency");
        return Task.CompletedTask;
    }

    private static void NativeDesktopVisibilityLifecycle()
    {
        foreach (var reason in new[] { "selected", "escape", "cancel", "cancel_down", "cancel_escape", "disconnect", "dispose",
            "deadline", "show_deadline", "source_gone", "overlay_gone", "process_exit", "reused_handle", "source_foreign",
            "display_changed", "queue_failed", "show_failed", "overlay_hide_failed", "target_gone" })
        {
            var api = new FakeApi();
            var before = new Dictionary<long, ProcessPickWindowFacts>(api.Windows);
            var showStates = new Dictionary<long, string>(api.ShowStates);
            var native = new ProcessWindowPickNative(api);
            var events = new List<ProcessWindowPickChangedEvent>();
            using var cancellation = new CancellationTokenSource();
            var controller = new ProcessWindowPickController(native, e =>
            {
                if (e.State != "hover") api.Order.Add("terminal");
                events.Add(e); return reason != "queue_failed" || e.State != "hover";
            }, NullPlatformLog.Instance);
            Check(controller.Execute(StartCommand(), 42, cancellation.Token).Success, reason + ": admitted");
            foreach (var handle in new long[] { 10, 11, 13, 14 })
                Check(!api.Windows[handle].Visible && (api.Windows[handle] with { Visible = true }) == before[handle], reason + ": captured state untouched while hidden");
            Check(!api.Windows[12].Visible && api.Windows[30].Visible && api.Windows[15].Visible && !api.Windows[20].Visible,
                reason + ": originally hidden/foreign/child/overlay excluded from source hiding");
            Check(native.ValidateOwners(10, 20, 42) && native.ReadWindow(11, 10, 20) is null && native.ReadWindow(41, 10, 20) is null &&
                native.ReadWindow(30, 10, 20) is not null, reason + ": intentionally hidden source remains alive; hidden windows cannot be selected");
            Check(controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success && api.Windows[20].Visible,
                reason + ": original passive overlay can show while Desktop is hidden");
            Check(controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "repeated current Show does not recapture source");
            if (reason != "queue_failed")
            {
                controller.Tick(); controller.Tick();
                Check(events.Count == 1 && events[0].State == "hover" && api.ProcessReads == 1, "hidden-source hover coalesces");
            }
            if (reason is "selected" or "cancel_down" or "target_gone")
                Check(native.HandleMouse(true, 0, 0), reason + ": paired down suppressed");
            if (reason is "escape" or "cancel_escape") Check(native.HandleEscape(true), "Escape down suppressed");
            if (reason is "cancel" or "cancel_down" or "cancel_escape" or "reused_handle")
            {
                if (reason == "reused_handle") api.Windows[11] = api.Windows[11] with { ProcessId = 99 };
                controller.Execute(new("cancel", Session), 42, CancellationToken.None);
            }
            if (reason == "disconnect") cancellation.Cancel();
            if (reason == "dispose") new ProcessWindowPickCoordinator(controller, action => { action(); return true; }).Dispose();
            if (reason is "deadline" or "show_deadline") api.Now = 120_000;
            if (reason == "show_deadline") Check(!controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "Show enforces native deadline");
            if (reason == "source_gone") api.Windows[10] = api.Windows[10] with { Exists = false };
            if (reason == "overlay_gone") api.Windows[20] = api.Windows[20] with { Exists = false };
            if (reason == "source_foreign") api.Windows[10] = api.Windows[10] with { ProcessId = 99 };
            if (reason == "display_changed") api.DisplayChanged = true;
            if (reason == "process_exit")
            {
                api.ProcessAlive = false;
                api.Windows[10] = api.Windows[10] with { Visible = true };
                Check(!native.ValidateOwners(10, 20, 42), "exited retained process rejects even visible HWND/PID facts");
                api.Windows[10] = api.Windows[10] with { Visible = false };
            }
            if (reason == "show_failed")
            { api.FailShow = true; Check(!controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "Show failure retires"); }
            if (reason == "overlay_hide_failed")
            { api.FailOverlayHide = true; controller.Execute(new("cancel", Session), 42, CancellationToken.None); }
            if (reason == "target_gone") api.Windows[30] = api.Windows[30] with { Exists = false };
            controller.Tick();
            var terminal = events.Where(e => e.State != "hover").ToArray();
            var failed = reason is "queue_failed" or "show_failed" or "overlay_hide_failed" or "target_gone";
            Check(terminal.Length == 1 && terminal[0].State == (reason == "selected" ? "selected" : failed ? "failed" : "cancelled"), reason + ": exactly one truthful terminal");
            foreach (var handle in new long[] { 10, 11, 13, 14 })
            {
                var eligible = reason != "process_exit" && api.Windows[handle].Exists && api.Windows[handle].ProcessId == 42;
                Check(api.Windows[handle].Visible == eligible, reason + ": only surviving original Desktop windows restored");
                var restored = api.VisibilityCalls.Where(c => c.Handle == handle && (c.Flags & 0x40) != 0).ToArray();
                Check(restored.Length == (eligible ? 1 : 0), reason + ": restore exactly once, never a foreign/dead-process window");
                if (eligible) Check(api.Windows[handle] == before[handle], reason + ": exact bounds and visibility restored");
            }
            Check(api.ShowStates.OrderBy(p => p.Key).SequenceEqual(showStates.OrderBy(p => p.Key)), reason + ": normal/maximized/minimized state preserved");
            Check(!api.Windows[12].Visible && api.VisibilityCalls.All(c => c.Handle is 10 or 11 or 13 or 14 or 20), "no originally hidden/foreign/child window touched");
            Check(api.OwnerHandle?.IsClosed == true, reason + ": process lease released at terminal");
            Check(api.Order.FindLastIndex(s => s.StartsWith("restore:")) < api.Order.IndexOf("terminal"), "restore precedes terminal publication and Rust focus handoff");
            Check(!controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, reason + ": late Show cannot revive");
            if (reason is "selected" or "cancel_down" or "target_gone") Check(native.HandleMouse(false, 0, 0), "terminal preserves paired mouse release");
            if (reason is "escape" or "cancel_escape") Check(native.HandleEscape(false), "terminal preserves paired Escape release");
            controller.Tick();
            Check(!controller.Occupied && api.Installed.Count == 0, reason + ": release owner cleans");
            var calls = api.VisibilityCalls.Count;
            controller.Stop(); controller.Execute(new("cancel", Session), 42, CancellationToken.None); controller.Tick();
            Check(api.VisibilityCalls.Count == calls && events.Count(e => e.State != "hover") == 1, "late cancellation/disposal cannot restore twice");
        }
    }

    private static void NativeVisibilityRollback()
    {
        foreach (var reason in new[] { "layered", "viewport", "mouse_hook", "keyboard_hook", "open", "enumerate", "first_hide", "partial_hide" })
        {
            var api = new FakeApi
            {
                FailLayered = reason == "layered", ClientMismatch = reason == "viewport",
                FailedHook = reason == "mouse_hook" ? 14 : reason == "keyboard_hook" ? 13 : 0,
                FailOpen = reason == "open", FailEnumeration = reason == "enumerate",
                FailHide = reason == "first_hide" ? 10 : reason == "partial_hide" ? 11 : 0
            };
            var before = new Dictionary<long, ProcessPickWindowFacts>(api.Windows);
            var controller = new ProcessWindowPickController(new ProcessWindowPickNative(api), _ => true, NullPlatformLog.Instance);
            var result = controller.Execute(StartCommand(), 42, CancellationToken.None);
            Check(!result.Success && !controller.Occupied && api.Installed.Count == 0 && !api.Windows[20].Visible, reason + ": failed admission cleans hooks/overlay");
            Check(result.NativeErrorCode == (reason == "viewport" ? 13 : 5), "startup failure retains native error");
            foreach (var handle in new long[] { 10, 11, 12, 13, 14, 15, 30 })
                Check(api.Windows[handle] == before[handle], reason + ": failure unwinds complete captured Desktop visibility");
            Check(api.OwnerHandle is null || api.OwnerHandle.IsClosed, reason + ": failed Start releases retained process");
            Check(!controller.Execute(new("show_overlay", Session), 42, CancellationToken.None).Success, "failed Start cannot reveal late frame");
        }
        var restoreApi = new FakeApi { FailRestore = 10 };
        var events = new List<ProcessWindowPickChangedEvent>();
        var log = new RecordingLog();
        var restoreController = new ProcessWindowPickController(new ProcessWindowPickNative(restoreApi), e => { events.Add(e); return true; }, log);
        Check(restoreController.Execute(StartCommand(), 42, CancellationToken.None).Success, "restore failure fixture admitted");
        restoreController.Execute(new("cancel", Session), 42, CancellationToken.None);
        Check(events.Single().State == "failed" && events[0].NativeErrorCode == 5 && log.Events[^1].Fields["reason"]?.ToString() == "restore_failed",
            "native restore failure is reported rather than successful cancellation");
        Check(restoreApi.Windows[11].Visible && restoreApi.Windows[13].Visible && restoreApi.Windows[14].Visible &&
            restoreApi.OwnerHandle?.IsClosed == true && restoreApi.Installed.Count == 0, "one restore failure cannot prevent remaining restores or native cleanup");
    }

    private sealed class FakeApi : ProcessPickApi
    {
        internal readonly Dictionary<long, ProcessPickWindowFacts> Windows = new()
        {
            [10] = new(true, true, false, 42, 10, false, 0, 0, 900, 600),
            [11] = new(true, true, false, 42, 11, false, 100, 50, 1000, 700),
            [20] = new(true, false, false, 42, 20, false, 0, 0, 1, 1),
            [12] = new(true, false, false, 42, 12, false, 200, 80, 400, 300),
            [13] = new(true, true, false, 42, 13, false, -900, -40, 1920, 1080),
            [14] = new(true, true, false, 42, 14, false, -32000, -32000, 160, 28),
            [15] = new(true, true, false, 42, 10, false, 0, 0, 20, 20),
            [30] = new(true, true, false, 70, 30, false, -100, 100, 700, 400),
            [40] = new(true, true, false, 70, 40, true, 0, 0, 700, 400),
            [41] = new(true, false, false, 70, 41, false, 0, 0, 700, 400),
            [42] = new(true, true, false, 70, 42, false, 0, 0, 0, 400),
            [43] = new(false, true, false, 70, 43, false, 0, 0, 700, 400),
            [44] = new(true, true, true, 70, 44, false, 0, 0, 700, 400)
        };
        internal readonly Dictionary<(long, int), long> Styles = new();
        internal readonly List<long> Mutations = [];
        internal readonly List<(long Handle, uint Flags)> VisibilityCalls = [];
        internal readonly Dictionary<long, string> ShowStates = new() { [10] = "normal", [11] = "normal", [13] = "maximized", [14] = "minimized" };
        internal readonly List<string> Order = [];
        internal SafeProcessHandle? OwnerHandle;
        internal bool ProcessAlive = true, FailEnumeration, FailOpen;
        internal long FailHide, FailRestore, Now;
        internal bool FailShow, FailOverlayHide;
        internal override long ElapsedMilliseconds => Now;
        internal override uint CurrentProcessId => 70;
        internal override SafeProcessHandle OpenOwner(uint pid)
        {
            Check(pid == 42, "only connected Desktop process is pinned");
            if (FailOpen) throw new System.ComponentModel.Win32Exception(5);
            return OwnerHandle = new SafeProcessHandle(new IntPtr(123), ownsHandle: false);
        }
        internal override bool OwnerAlive(SafeProcessHandle process)
        { Check(ReferenceEquals(process, OwnerHandle) && !process.IsClosed, "original process handle owns visibility lifetime"); return ProcessAlive; }
        internal override IReadOnlyList<long> TopLevelWindows()
        {
            Check(Installed.Count == 2 && LayeredInitialized, "source capture follows prepared overlay and both admitted hooks");
            if (FailEnumeration) throw new System.ComponentModel.Win32Exception(5);
            return Windows.Keys.ToArray();
        }
        internal readonly HashSet<IntPtr> Installed = [];
        internal int FailedHook, ProcessReads;
        internal bool ClientMismatch, DisplayChanged;
        internal bool LayeredInitialized = false;
        internal bool FailLayered;
        internal long PointerRoot = 30;
        internal override int LastError => 5;
        internal override ProcessPickWindowFacts Facts(long handle) => Windows.GetValueOrDefault(handle) ?? new(false, false, false, 0, 0, false, 0, 0, 0, 0);
        internal override ProcessPickViewport Viewport() => new(-1920, -200, DisplayChanged ? 3000u : 3840u, 1280);
        internal override ProcessPickViewport ClientViewport(long handle) => ClientMismatch ? Viewport() with { Left = -1919 } : Viewport();
        internal override long GetStyle(long hwnd, int index) => Styles.GetValueOrDefault((hwnd, index), index == -16 ? 0x00CF0000 : 0);
        internal override void SetStyle(long hwnd, int index, long value) { Mutations.Add(hwnd); Styles[(hwnd, index)] = value; }
        internal override void InitializeLayeredOverlay(long hwnd)
        {
            Check(!Windows[hwnd].Visible && Windows[hwnd].Width == 1 && (Styles[(hwnd, -20)] & 0x80000) != 0,
                "initialize layered alpha after style and before placement while hidden");
            if (FailLayered) throw new System.ComponentModel.Win32Exception(5);
            Mutations.Add(hwnd); LayeredInitialized = true;
        }
        internal override void Place(long hwnd, ProcessPickViewport viewport, bool visible)
        { Mutations.Add(hwnd); Windows[hwnd] = Windows[hwnd] with { Left = viewport.Left, Top = viewport.Top, Width = viewport.Width, Height = viewport.Height, Visible = visible }; }
        internal override void Show(long hwnd)
        { if (FailShow) throw new System.ComponentModel.Win32Exception(5); Mutations.Add(hwnd); Windows[hwnd] = Windows[hwnd] with { Visible = true }; }
        internal override void Visibility(long hwnd, uint flags)
        {
            VisibilityCalls.Add((hwnd, flags)); Mutations.Add(hwnd);
            var visible = (flags & 0x40) != 0;
            if (visible) Check((flags & 0x217) == 0x217 && (flags & 0x80) == 0, "restore never moves/sizes/reorders/activates a Desktop HWND");
            if (visible && hwnd == FailRestore || !visible && hwnd == 20 && FailOverlayHide && Windows[20].Width != 1)
                throw new System.ComponentModel.Win32Exception(5);
            Windows[hwnd] = Windows[hwnd] with { Visible = visible };
            Order.Add((visible ? "restore:" : "hide:") + hwnd);
            if (!visible && hwnd == FailHide) throw new System.ComponentModel.Win32Exception(5);
        }
        internal override bool Pressed(int key) => false;
        internal override long RootAtPointer() => PointerRoot;
        internal override long RootAt(int x, int y) => PointerRoot;
        internal override string? ProcessName(uint pid, out int error) { ProcessReads++; error = 0; return "owned-fixture.exe"; }
        internal override IntPtr Install(int kind, HookProc callback)
        { Check(Windows[10].Visible && Windows[11].Visible, "hook admission precedes source hiding"); if (kind == FailedHook) return IntPtr.Zero; var handle = new IntPtr(kind); Installed.Add(handle); return handle; }
        internal override void Unhook(IntPtr hook) { Installed.Remove(hook); }
        internal override IntPtr Next(int code, IntPtr message, IntPtr data) => IntPtr.Zero;
    }

    private sealed class ThrowingLog : IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) => throw new IOException("private diagnostic fixture");
    }

    private sealed class Fixture : IDisposable
    {
        internal readonly FakeNative Native = new();
        internal readonly List<ProcessWindowPickChangedEvent> Events = [];
        internal readonly RecordingLog Log = new();
        internal readonly ProcessWindowPickController Controller;
        internal Fixture() { Controller = new(Native, e => { Events.Add(e); return true; }, Log); }
        internal void Start() { Check(Controller.Execute(StartCommand(), 42, CancellationToken.None).Success, "start admitted"); }
        public void Dispose() { Native.LeftPressed = false; Native.EscapePressed = false; Controller.Stop(); Controller.Input(ProcessPickInput.LeftUp, 0); Controller.Input(ProcessPickInput.EscapeUp, 0); Controller.Tick(); }
    }

    private sealed class FakeNative : IProcessWindowPickNative
    {
        internal string? Failure;
        internal bool OwnerAlive = true, LeftPressed, EscapePressed, Visible;
        internal int Hooks, HideCalls, ShowCalls, Resolves;
        internal long Now;
        internal string? Name = "private-app.exe";
        internal ProcessPickViewport Viewport = new(-1920, -200, 3840, 1280);
        internal ProcessPickViewport? PreparedViewport;
        internal ProcessPickWindow? Hit = new(30, 70, -500, 20, 600, 400);
        public long ElapsedMilliseconds => Now;
        public bool IsPressed(ProcessPickInput input) => input == ProcessPickInput.LeftDown ? LeftPressed : EscapePressed;
        public bool ValidateOwners(long source, long overlay, uint peer) => OwnerAlive && source == 10 && overlay == 20 && peer == 42 && Failure is not ("source_hidden" or "source_foreign" or "overlay_foreign" or "overlay_visible");
        public ProcessPickViewport GetViewport() => Viewport;
        public void PrepareOverlay(long overlay, ProcessPickViewport viewport) { PreparedViewport = viewport; if (Failure == "prepare") throw new System.ComponentModel.Win32Exception(5); Visible = false; }
        public void ShowOverlay(long overlay) { Visible = true; ShowCalls++; }
        public void HideOverlay(long overlay) { Visible = false; HideCalls++; }
        public void InstallHooks(Func<ProcessPickInput, long, bool> input, long source, long overlay) { Hooks = 1; if (Failure == "hooks") throw new System.ComponentModel.Win32Exception(5); }
        public void RemoveHooks() { Hooks = 0; }
        public void HideSourceWindows(long source, long overlay, uint peer) { }
        public void RestoreSourceWindows() { }
        public ProcessPickWindow? HitTest(long source, long overlay) => Hit;
        public ProcessPickWindow? ReadWindow(long handle, long source, long overlay) => Hit?.WindowHandle == handle ? Hit : null;
        public string? ResolveProcess(uint pid, out int error) { Resolves++; error = Name is null ? 5 : 0; return Name; }
    }

    private sealed record Diagnostic(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields);
    private sealed class RecordingLog : IPlatformLog
    {
        internal readonly List<Diagnostic> Events = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) => Events.Add(new(eventName, correlationId, fields!));
    }
}
