using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class FloatingAssistantCoordinatorTests
{
    private static void Assert(bool condition, string reason)
    { if (!condition) throw new InvalidOperationException(reason); }

    private static FloatingAssistantCommand Command(IntPtr renderer, IntPtr source, string operation = "prepare") =>
        new(1, Guid.NewGuid(), 1, renderer.ToInt64(), source.ToInt64(), (uint)Environment.ProcessId,
            operation, 100, 100, 80, 48, false, false, null);
    private static ProtocolEnvelope Envelope(FloatingAssistantCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "floating_assistant", data = command });
    private static void Reject(ProtocolEnvelope envelope)
    {
        try { _ = FloatingAssistantContract.Parse(envelope); }
        catch (InvalidDataException) { return; }
        throw new InvalidOperationException("malformed command admitted");
    }
    internal static Task StrictWireAndEvents()
    {
        var command = Command(new IntPtr(11), new IntPtr(12));
        var envelope = Envelope(command);
        Assert(FloatingAssistantContract.Parse(envelope) == command, "exact versioned wire");
        var picker = command with { Operation = "pick_image", WindowHandle = 0 };
        Assert(FloatingAssistantContract.Parse(Envelope(picker)) == picker,
            "choosing an image does not require an enabled/prepared floating renderer");
        Assert(!FloatingAssistantContract.Parse(ProtocolEnvelope.Create("platform_command", new { command = "floating_assistant", data = new {
            version = 1, session_id = command.SessionId, generation = 1, window_handle = 11L,
            source_window_handle = 12L, source_process_id = (uint)Environment.ProcessId, operation = "update",
            x = 100, y = 100, size = 80, opacity = 48, click_through = false, position_locked = false
        } })).EdgeHidden, "legacy wire omitting edge_hidden defaults to full clamp");
        Assert(FloatingAssistantContract.Parse(Envelope(command with { EdgeHidden = true })).EdgeHidden,
            "explicit edge_hidden true survives strict wire");
        Assert(FloatingAssistantContract.Parse(Envelope(command with { EdgeHidden = false })) == command,
            "explicit false retains the default command shape");
        Reject(ProtocolEnvelope.Create("platform_command", new { command = "floating_assistant", data = new {
            version = 1, session_id = command.SessionId, generation = 1, window_handle = 11L,
            source_window_handle = 12L, source_process_id = (uint)Environment.ProcessId, operation = "update",
            x = 100, y = 100, size = 80, opacity = 48, click_through = false, position_locked = false,
            edge_hidden = "true"
        } }));
        foreach (var bad in new[] { command with { Version = 2 }, command with { Size = 301 },
            command with { Opacity = 0 }, command with { SessionId = Guid.Empty },
            command with { Generation = 0 }, command with { WindowHandle = command.SourceWindowHandle },
            command with { Operation = "show_now" }, command with { DropId = Guid.NewGuid() },
            command with { Operation = "release_drop" } }) Reject(Envelope(bad));
        Reject(ProtocolEnvelope.Create("platform_command", new { command = "floating_assistant", data = new {
            version = 1, session_id = command.SessionId, generation = 1, window_handle = 11,
            source_window_handle = 12, source_process_id = (uint)Environment.ProcessId, operation = "show",
            x = "100", y = 100, size = 80, opacity = 48, click_through = false, position_locked = false } }));
        Reject(ProtocolEnvelope.Create("platform_command", new { command = "floating_assistant", data = new {
            version = 1, session_id = command.SessionId, generation = 1, window_handle = 11L,
            source_window_handle = 12L, source_process_id = (uint)Environment.ProcessId, operation = "update",
            x = 100, y = 100, size = 80, opacity = 48, click_through = false, position_locked = false,
            edge_hidden = (bool?)null
        } }));
        var id = Guid.NewGuid();
        var evt = FloatingAssistantContract.Event(command, id, 3, "wheel", delta: 120);
        Assert(evt.Type == "platform_event" && evt.Payload.GetProperty("event").GetString() == "floating_assistant" &&
            evt.Payload.GetProperty("data").GetProperty("correlation_id").GetGuid() == id &&
            evt.Payload.GetProperty("data").GetProperty("files").GetArrayLength() == 0 &&
            evt.Payload.GetProperty("data").GetProperty("text").ValueKind == JsonValueKind.Null &&
            evt.Payload.GetProperty("data").GetProperty("delta").GetInt32() == 120, "complete passive event wire");
        return Task.CompletedTask;
    }

    internal static Task EnvironmentSignatureRetriesAndDeduplicates()
    {
        var tracker = new FloatingAssistantEnvironmentTracker();
        var original = new FloatingAssistantEnvironmentSignature(1, 0, 0, 1920, 1080, 0, 0, 1920, 1040, 96, false);
        var count = 0;
        Assert(!tracker.TryPublish(original, () => { count++; return false; }), "failed publication cannot consume initial snapshot");
        Assert(tracker.TryPublish(original, () => { count++; return true; }) && count == 2,
            "initial snapshot retries after full outgoing queue");
        Assert(!tracker.TryPublish(original, () => { count++; return true; }) && count == 2,
            "unchanged signature has no polling event");
        foreach (var changed in new[] { original with { MonitorHandle = 2 },
            original with { WorkBottom = 1030 }, original with { WorkLeft = -80 },
            original with { Dpi = 144 }, original with { Fullscreen = true },
            original with { MonitorRight = 1600 }, original with { MonitorTop = -100 } })
        {
            Assert(!tracker.TryPublish(changed, () => { count++; return false; }), "changed snapshot retries on failed delivery");
            Assert(tracker.TryPublish(changed, () => { count++; return true; }),
                "monitor, work area, DPI and fullscreen each notify independently");
            Assert(!tracker.TryPublish(changed, () => { count++; return true; }), "successful signature deduplicates");
        }
        tracker.Reset();
        Assert(tracker.TryPublish(original, () => true), "new renderer lease starts with a fresh environment snapshot");
        return Task.CompletedTask;
    }

    internal static Task FullscreenPolicyRejectsDesktopAndFramedWindows()
    {
        bool Evaluate(bool valid, bool visible, bool minimized, bool desktop, long style, bool covers, string expected)
        {
            var result = FloatingAssistantFullscreenPolicy.Evaluate(valid, visible, minimized, desktop, style, covers, out var reason);
            Assert(reason == expected, "fullscreen policy reports only its fixed reason");
            return result;
        }
        foreach (var shellClass in new[] { "Progman", "WorkerW" })
        {
            var desktop = Win32DesktopWindowOperations.IsShellDesktopIdentity(true, shellClass, true, true);
            Assert(!Evaluate(true, true, false, desktop, 0, true, "shell_desktop"),
                "Shell desktop covering a monitor is not a fullscreen application");
        }
        Assert(Evaluate(true, true, false, false, 0x80000000, true, "borderless_fullscreen"), "borderless fullscreen hides");
        Assert(!Evaluate(true, true, false, false, 0x00CF0000, true, "framed_window"), "normal maximized frame does not hide");
        Assert(!Evaluate(false, true, false, false, 0, true, "invalid_foreground"), "invalid foreground does not hide");
        Assert(!Evaluate(true, false, false, false, 0, true, "hidden_foreground"), "hidden foreground does not hide");
        Assert(!Evaluate(true, true, true, false, 0, true, "minimized_foreground"), "minimized foreground does not hide");
        Assert(!Evaluate(true, true, false, false, 0, false, "not_covering_monitor"), "ordinary borderless window does not hide");
        return Task.CompletedTask;
    }

    internal static async Task PickerWaitingForUserIsNotBackgroundTimeout()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var waiting = Task.Run(() => FloatingAssistantCoordinator.RunImagePicker(token => {
            entered.Set();
            release.Wait();
            return new WindowsCommandResult(true, Data: "{\"selected_path\":null}");
        }, CancellationToken.None));
        Assert(entered.Wait(TimeSpan.FromSeconds(2)), "native picker entered its owned STA");
        try {
            await Task.Delay(100);
            Assert(!waiting.IsCompleted, "waiting for user input must not emit image selection failure");
        } finally { release.Set(); await waiting.WaitAsync(TimeSpan.FromSeconds(2)); }
        using var cancelled = new CancellationTokenSource();
        using var enteredCancellation = new ManualResetEventSlim();
        var retired = Task.Run(() => FloatingAssistantCoordinator.RunImagePicker(token => {
            enteredCancellation.Set();
            token.WaitHandle.WaitOne();
            token.ThrowIfCancellationRequested();
            return new WindowsCommandResult(true);
        }, cancelled.Token));
        Assert(enteredCancellation.Wait(TimeSpan.FromSeconds(2)), "cancellable picker entered");
        cancelled.Cancel();
        var terminal = await retired.WaitAsync(TimeSpan.FromSeconds(2));
        Assert(!terminal.Success && terminal.Error == "picker_retired", "stop retires picker without a false native failure");
    }

    internal static async Task InvalidPickerPeerFailsBeforeDialog()
    {
        using var windows = new PumpedWindows();
        using var coordinator = new FloatingAssistantCoordinator((uint)Environment.ProcessId,
            _ => true, NullPlatformLog.Instance);
        var picker = Command(IntPtr.Zero, windows.Source, "pick_image");
        var invalid = await coordinator.Dispatch(Envelope(picker with { SourceWindowHandle = -1 }))
            .WaitAsync(TimeSpan.FromSeconds(3));
        Assert(!invalid.Success && invalid.Error == "invalid_source", "picker verifies live peer HWND before opening dialog");
        var wrongPeer = await coordinator.Dispatch(Envelope(picker with { SourceProcessId = (uint)Environment.ProcessId + 1 }));
        Assert(!wrongPeer.Success, "picker cannot trust caller-supplied process identity");
    }
    internal static async Task OwnedHiddenShowRetire()
    {
        using var windows = new PumpedWindows();
        var command = Command(windows.Renderer, windows.Source);
        var events = new List<ProtocolEnvelope>();
        var log = new RepairLog();
        using var coordinator = new FloatingAssistantCoordinator((uint)Environment.ProcessId,
            evt => { lock (events) events.Add(evt); return true; }, log);
        var wrong = await coordinator.Dispatch(Envelope(command with { SourceProcessId = (uint)Environment.ProcessId + 1 }));
        Assert(!wrong.Success, "pipe peer PID cannot be supplied by caller");
        var prepared = await coordinator.Dispatch(Envelope(command)).WaitAsync(TimeSpan.FromSeconds(5));
        Assert(prepared.Success && !Native.IsWindowVisible(windows.Renderer), "prepare sets final bounds while hidden");
        Assert(windows.RendererShowTransitions == 0, "prepare never temporarily shows the renderer before GPUI frame readiness");
        using var bounds = JsonDocument.Parse(prepared.Data!);
        Assert(bounds.RootElement.GetProperty("width").GetInt32() > 0 &&
            bounds.RootElement.GetProperty("work_right").GetInt32() > bounds.RootElement.GetProperty("work_left").GetInt32(),
            "client/work-area readback");
        var workLeft = bounds.RootElement.GetProperty("work_left").GetInt32();
        var workTop = bounds.RootElement.GetProperty("work_top").GetInt32();
        var workRight = bounds.RootElement.GetProperty("work_right").GetInt32();
        var workBottom = bounds.RootElement.GetProperty("work_bottom").GetInt32();
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "retire" }))).Success,
            "first lease retired before testing restored offscreen placement");
        var restored = command with { SessionId = Guid.NewGuid(), Generation = 2, X = workLeft - 300,
            Y = workTop - 300, Operation = "prepare", EdgeHidden = true };
        var restoredResult = await coordinator.Dispatch(Envelope(restored));
        Assert(restoredResult.Success, "offscreen stored anchor must still prepare hidden");
        using var restoredBounds = JsonDocument.Parse(restoredResult.Data!);
        Assert(restoredBounds.RootElement.GetProperty("x").GetInt32() >= workLeft &&
            restoredBounds.RootElement.GetProperty("y").GetInt32() >= workTop &&
            restoredBounds.RootElement.GetProperty("x").GetInt32() + restoredBounds.RootElement.GetProperty("width").GetInt32() <= workRight &&
            restoredBounds.RootElement.GetProperty("y").GetInt32() + restoredBounds.RootElement.GetProperty("height").GetInt32() <= workBottom,
            "prepare restores whole client within workarea, not eight visible pixels");
        var normal = await coordinator.Dispatch(Envelope(restored with { Operation = "update", X = workRight + 100,
            Y = workTop, EdgeHidden = false }));
        Assert(normal.Success, "ordinary update restores full visibility");
        using var normalBounds = JsonDocument.Parse(normal.Data!);
        Assert(normalBounds.RootElement.GetProperty("x").GetInt32() == workRight - normalBounds.RootElement.GetProperty("width").GetInt32(),
            "ordinary update never silently uses eight-pixel edge-hide");
        var edge = await coordinator.Dispatch(Envelope(restored with { Operation = "update", X = workRight + 100,
            Y = workTop, EdgeHidden = true }));
        Assert(edge.Success, "explicit edge-hidden update accepted");
        using var edgeBounds = JsonDocument.Parse(edge.Data!);
        Assert(edgeBounds.RootElement.GetProperty("x").GetInt32() == workRight - 8,
            "explicit edge-hide keeps eight device pixels visible");
        var recovered = await coordinator.Dispatch(Envelope(restored with { Operation = "update", X = workRight + 100,
            Y = workTop, EdgeHidden = false }));
        using var recoveredBounds = JsonDocument.Parse(recovered.Data!);
        Assert(recovered.Success && recoveredBounds.RootElement.GetProperty("x").GetInt32() ==
            workRight - recoveredBounds.RootElement.GetProperty("width").GetInt32(), "ordinary update reverses edge-hide");
        Assert((await coordinator.Dispatch(Envelope(restored with { Operation = "retire" }))).Success,
            "restored session retires without pending native receiver");
        // Continue the existing show/retire coverage using a fresh lease.
        Assert((await coordinator.Dispatch(Envelope(command))).Success, "original identity may prepare after retirement");
        var exStyle = Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64();
        Assert((exStyle & (8 | 0x80000 | 0x20 | 0x8000000 | 0x80)) == (8 | 0x80000 | 0x20 | 0x8000000 | 0x80),
            "layered transparent noactivate tool renderer");
        Assert(!(await coordinator.Dispatch(Envelope(command with { Generation = 2, Operation = "show" }))).Success,
            "stale generation cannot reveal");
        var show = Envelope(command with { Operation = "show" });
        Assert((await coordinator.Dispatch(show)).Success && Native.IsWindowVisible(windows.Renderer),
            "only explicit show reveals renderer");
        // Only mutate the receiver after proving it is this fixture's owned HWND.
        var input = Native.GetWindow(windows.Renderer, 3); // preceding z-order window
        _ = Native.GetWindowThreadProcessId(input, out var inputProcess);
        Assert(input != IntPtr.Zero && Native.GetWindow(input, 4) == windows.Renderer &&
            inputProcess == (uint)Environment.ProcessId, "fixture receiver identity and z-order");
        Assert(Native.GetWindowRect(windows.Renderer, out var rendererBounds) &&
            Native.GetWindowRect(input, out _), "owned visible bounds available");
        _ = Native.GetWindowRect(input, out var inputBounds);
        var showTransitions = windows.RendererShowTransitions;
        var foreground = Native.GetForegroundWindow();
        Assert(Native.SetWindowPos(windows.Renderer, new IntPtr(-2), 0, 0, 0, 0, 0x0013),
            "owned fixture demotes renderer without moving/activating it");
        Assert((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) == 0,
            "regression actually removed WS_EX_TOPMOST");
        var recoveryDeadline = DateTime.UtcNow.AddSeconds(3);
        while ((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) == 0 && DateTime.UtcNow < recoveryDeadline)
            await Task.Delay(25);
        Assert((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) != 0,
            "existing bounded observation restores lost native topmost without another settings command");
        Assert((Native.GetWindowLongPtrW(input, -20).ToInt64() & 8) != 0 &&
            Native.GetWindow(windows.Renderer, 3) == input, "repair restores both surfaces with receiver above renderer");
        Assert(Native.GetWindowRect(windows.Renderer, out var rendererAfter) && rendererAfter == rendererBounds &&
            Native.GetWindowRect(input, out var inputAfter) && inputAfter == inputBounds &&
            windows.RendererShowTransitions == showTransitions && Native.GetForegroundWindow() == foreground,
            "topmost repair never moves resizes shows or activates");
        Assert(Native.SetWindowPos(input, new IntPtr(-2), 0, 0, 0, 0, 0x0013) &&
            (Native.GetWindowLongPtrW(input, -20).ToInt64() & 8) == 0, "input-only demotion actually loses topmost");
        recoveryDeadline = DateTime.UtcNow.AddSeconds(3);
        while ((Native.GetWindowLongPtrW(input, -20).ToInt64() & 8) == 0 && DateTime.UtcNow < recoveryDeadline)
            await Task.Delay(25);
        Assert((Native.GetWindowLongPtrW(input, -20).ToInt64() & 8) != 0 &&
            (Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) != 0 &&
            Native.GetWindow(windows.Renderer, 3) == input, "input-only loss repairs the pair in order");
        await Task.Delay(650);
        Assert(log.Repairs.Count == 2 && log.Repairs.All(entry => entry.Correlation == show.Id && entry.Success),
            "two native losses produce two correlated transitions, not per-tick logs");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "update", ClickThrough = true }))).Success,
            "same generation updates pass-through without hiding renderer");
        Assert(Native.IsWindowVisible(windows.Renderer), "update retains visibility");
        Assert(!Native.IsWindowVisible(input), "click-through hides the receiver");
        Assert(Native.SetWindowPos(windows.Renderer, new IntPtr(-2), 0, 0, 0, 0, 0x0013), "demote click-through renderer");
        recoveryDeadline = DateTime.UtcNow.AddSeconds(3);
        while ((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) == 0 && DateTime.UtcNow < recoveryDeadline)
            await Task.Delay(25);
        Assert((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) != 0 && !Native.IsWindowVisible(input),
            "click-through repairs visible renderer only, never reviving hidden receiver");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "hide" }))).Success &&
            !Native.IsWindowVisible(windows.Renderer), "hide releases hit target");
        var repairsBeforeHidden = log.Repairs.Count;
        Assert(Native.SetWindowPos(windows.Renderer, new IntPtr(-2), 0, 0, 0, 0, 0x0013), "demote hidden owned renderer");
        await Task.Delay(1100);
        Assert((Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) == 0 &&
            !Native.IsWindowVisible(windows.Renderer) && !Native.IsWindowVisible(input) &&
            log.Repairs.Count == repairsBeforeHidden, "observer never repairs or revives an explicitly hidden session");
        Assert(!(await coordinator.Dispatch(Envelope(command with { Operation = "release_drop", DropId = Guid.NewGuid() }))).Success,
            "unknown drop token cannot authorize filesystem deletion");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "retire" }))).Success &&
            !(await coordinator.Dispatch(Envelope(command with { Operation = "show" }))).Success,
            "retirement blocks replay");
        await Task.Delay(650);
        Assert(!Native.IsWindowVisible(windows.Renderer) &&
            (Native.GetWindowLongPtrW(windows.Renderer, -20).ToInt64() & 8) == 0 &&
            log.Repairs.Count == repairsBeforeHidden, "retired session cannot be repaired or revived");
        var replacement = command with { SessionId = Guid.NewGuid(), Generation = 2, Operation = "prepare" };
        Assert((await coordinator.Dispatch(Envelope(replacement))).Success, "retired lease permits new hidden generation");
        Assert(!(await coordinator.Dispatch(Envelope(command with { Operation = "show" }))).Success,
            "old session cannot reveal replacement renderer");
        Assert((await coordinator.Dispatch(Envelope(replacement with { Operation = "retire" }))).Success,
            "replacement retirement cleans its own input surface");
        lock (events) Assert(events.Any(e => e.Payload.GetProperty("data").GetProperty("kind").GetString() == "retired"),
            "retirement notification delivered");
    }
    // Exercise the coordinator's real OLE consumer with synthetic, self-owned image bytes;
    // neither the user's clipboard nor a foreign dropped file is read or changed.
    internal static async Task OwnedDropLifetimeAndCorrelation()
    {
        using var windows = new PumpedWindows();
        var command = Command(windows.Renderer, windows.Source);
        ProtocolEnvelope? captured = null;
        using var coordinator = new FloatingAssistantCoordinator((uint)Environment.ProcessId,
            evt => { if (evt.Payload.GetProperty("data").GetProperty("kind").GetString() == "drop") captured = evt; return true; },
            NullPlatformLog.Instance);
        Assert((await coordinator.Dispatch(Envelope(command))).Success, "self-owned receiver prepared");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "show" }))).Success, "receiver visible");
        var dropId = Guid.NewGuid();
        var data = new FloatingAssistantDropData("fixture", null, null, [1, 2, 3], "png", [], dropId);
        Assert(await coordinator.ReceiveDropForFixture(data), "native consumer accepts synthetic bounded image");
        var evt = captured!.Payload.GetProperty("data");
        Assert(evt.GetProperty("correlation_id").GetGuid() == dropId && evt.GetProperty("drop_id").GetGuid() == dropId,
            "OLE accepted correlation is the emitted event/drop token");
        var stage = evt.GetProperty("image_path").GetString()!;
        Assert(File.Exists(stage), "owned staged image survives event admission");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "hide" }))).Success && File.Exists(stage),
            "hide cancels new OLE admission but preserves already accepted image pending Rust action");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "release_drop", DropId = dropId }))).Success &&
            !File.Exists(stage), "exact release removes only the accepted staged file");
        Assert(!(await coordinator.Dispatch(Envelope(command with { Operation = "release_drop", DropId = dropId }))).Success,
            "duplicate release is rejected");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "retire" }))).Success, "receiver retired");
    }
    internal static async Task FullEventQueueCleansExactStaging()
    {
        using var windows = new PumpedWindows();
        var command = Command(windows.Renderer, windows.Source);
        string? rejectedStage = null;
        using var coordinator = new FloatingAssistantCoordinator((uint)Environment.ProcessId,
            evt =>
            {
                var data = evt.Payload.GetProperty("data");
                if (data.GetProperty("kind").GetString() == "drop")
                {
                    rejectedStage = data.GetProperty("image_path").GetString();
                    return false;
                }
                return true;
            }, NullPlatformLog.Instance);
        Assert((await coordinator.Dispatch(Envelope(command))).Success &&
            (await coordinator.Dispatch(Envelope(command with { Operation = "show" }))).Success,
            "receiver prepared before full event queue");
        var id = Guid.NewGuid();
        Assert(!await coordinator.ReceiveDropForFixture(new("fixture", null, null, [1, 2, 3], "png", [], id)),
            "full outgoing queue refuses drop admission");
        Assert(rejectedStage is not null && !File.Exists(rejectedStage) &&
            !(await coordinator.Dispatch(Envelope(command with { Operation = "release_drop", DropId = id }))).Success,
            "failed delivery cleans exact staging and never installs a live drop token");
        Assert(await coordinator.ReceiveDropForFixture(new("text", null, null, null, null, [], Guid.NewGuid())) == false,
            "full transport rejects text-only event without a staged token");
        Assert((await coordinator.Dispatch(Envelope(command with { Operation = "retire" }))).Success,
            "failed event does not prevent native retirement");
    }

    internal static async Task SourceDestroyedRetiresInput()
    {
        using var windows = new PumpedWindows();
        var command = Command(windows.Renderer, windows.Source);
        var events = new List<ProtocolEnvelope>();
        using var coordinator = new FloatingAssistantCoordinator((uint)Environment.ProcessId,
            evt => { lock (events) events.Add(evt); return true; }, NullPlatformLog.Instance);
        var prepared = await coordinator.Dispatch(Envelope(command));
        Assert(prepared.Success, $"receiver prepared before source removal: {prepared.Error} native={prepared.NativeErrorCode}");
        var shown = await coordinator.Dispatch(Envelope(command with { Operation = "show" }));
        Assert(shown.Success, $"receiver shown before source removal: {shown.Error} native={shown.NativeErrorCode}");
        windows.DestroySource();
        await Task.Delay(1100);
        Assert(!Native.IsWindowVisible(windows.Renderer) &&
            !(await coordinator.Dispatch(Envelope(command with { Operation = "show" }))).Success,
            "source disappearance hides renderer and rejects stale show");
        lock (events) Assert(events.Any(evt => evt.Payload.GetProperty("data").GetProperty("kind").GetString() == "retired"),
            "source destruction emits terminal retired event");
    }

    private sealed class RepairLog : IPlatformLog
    {
        private readonly List<(Guid? Correlation, bool Success)> _repairs = new();
        internal List<(Guid? Correlation, bool Success)> Repairs { get { lock (_repairs) return [.. _repairs]; } }
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (eventName == "floating_assistant.window.topmost_repair")
                lock (_repairs) _repairs.Add((correlationId, fields?["success"] is true));
        }
    }

    private sealed class PumpedWindows : IDisposable
    {
        private readonly Thread _thread;
        private readonly TaskCompletionSource<(IntPtr, IntPtr)> _handles = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private ApplicationContext? _context;
        internal IntPtr Source { get; }
        internal IntPtr Renderer { get; }
        internal int RendererShowTransitions => Volatile.Read(ref _showTransitions);
        private int _showTransitions;
        internal void DestroySource()
        {
            if (!Native.PostMessageW(Source, 0x0010, IntPtr.Zero, IntPtr.Zero))
                throw new InvalidOperationException("Cannot close owned source HWND");
        }
        internal PumpedWindows()
        {
            _thread = new Thread(() =>
            {
                using var source = new Form { StartPosition = FormStartPosition.Manual, Bounds = new System.Drawing.Rectangle(50, 50, 40, 40) };
                using var renderer = new ObservedRenderer(() => Interlocked.Increment(ref _showTransitions));
                using var context = new ApplicationContext();
                _context = context;
                _handles.TrySetResult((source.Handle, renderer.Handle));
                Application.Run(context);
            }) { IsBackground = true, Name = "NxTools.FloatingRendererFixture" };
            _thread.SetApartmentState(ApartmentState.STA);
            _thread.Start();
            (Source, Renderer) = _handles.Task.WaitAsync(TimeSpan.FromSeconds(5)).GetAwaiter().GetResult();
        }
        public void Dispose()
        {
            _context?.ExitThread();
            _thread.Join(TimeSpan.FromSeconds(3));
        }
    }
    private sealed class ObservedRenderer(Action show) : Form
    {
        protected override bool ShowWithoutActivation => true;
        protected override CreateParams CreateParams
        {
            get
            {
                var value = base.CreateParams;
                value.ExStyle |= 0x08000080;
                return value;
            }
        }
        protected override void OnHandleCreated(EventArgs args)
        {
            // Unlike GPUI, Form replays its managed TopMost policy on first show.
            // Match the passive product renderer instead of downgrading its owned popup.
            TopMost = true;
            base.OnHandleCreated(args);
        }
        protected override void WndProc(ref Message message)
        {
            if (message.Msg == 0x0047 && message.LParam != IntPtr.Zero &&
                (Marshal.PtrToStructure<WindowPosition>(message.LParam).Flags & 0x40) != 0) show();
            base.WndProc(ref message);
        }
        [StructLayout(LayoutKind.Sequential)] private struct WindowPosition
        { internal IntPtr Window, After; internal int X, Y, Width, Height; internal uint Flags; }
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal readonly record struct Rect(int Left, int Top, int Right, int Bottom);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetWindow(IntPtr window, uint command);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetWindowRect(IntPtr window, out Rect rect);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
        internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool PostMessageW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    }
}
