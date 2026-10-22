using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using System.Windows.Automation;

namespace NxTools.PlatformHost.Tests;

internal static class NativeContextMenuTests
{
    private static NativeContextMenuItem Leaf(uint id = 1) => new(id, "Synthetic", true, false, false, []);
    private static ShowContextMenuCommand Command() => new(1, 1, 299, 30, [Leaf()]);
    private static void Assert(bool value, string message)
    { if (!value) throw new InvalidOperationException(message); }
    private static void Reject(ShowContextMenuCommand command)
    {
        try { NativeContextMenuCoordinator.Validate(command); }
        catch (InvalidDataException) { return; }
        throw new InvalidOperationException("Invalid menu admitted");
    }

    public static Task SemanticValidationAndWire()
    {
        var command = Command();
        NativeContextMenuCoordinator.Validate(command);
        var envelope = ProtocolEnvelope.Create("platform_command", new { command = "show_context_menu", data = command });
        var parsed = PlatformCommandParser.ParseShowContextMenu(envelope);
        Assert(parsed.Items[0].Id == 1 && parsed.X == 299, "source-client device pixels and IDs survive wire");
        var styled = command with { PopupWindowHandle = 2, Items = [] };
        var styledWire = PlatformCommandParser.ParseShowContextMenu(ProtocolEnvelope.Create("platform_command",
            new { command = "show_context_menu", data = styled }));
        Assert(styledWire.PopupWindowHandle == 2 && styledWire.Items.Count == 0,
            "styled GPUI HWND placement carries no menu text or callbacks to native renderer");
        Reject(styled with { PopupWindowHandle = styled.SourceWindowHandle });
        Assert(PlatformPipeClient.ResolveDiagnosticCorrelationId(envelope with { CorrelationId = Guid.NewGuid() }) == envelope.Id,
            "menu diagnostics use the menu session envelope ID");
        foreach (var invalid in new object[] { new { }, new { source_window_handle = 1, source_process_id = 1, x = 1, y = 1, items = new[] { new { id = 1 } } } })
        {
            try
            {
                PlatformCommandParser.ParseShowContextMenu(ProtocolEnvelope.Create("platform_command", new { command = "show_context_menu", data = invalid }));
                throw new InvalidOperationException("Incomplete native menu wire admitted");
            }
            catch (InvalidDataException) { }
        }
        Reject(command with { Items = [Leaf(), Leaf()] });
        Reject(command with { Items = [Leaf(0)] });
        Reject(command with { Items = [Leaf() with { Label = new string('x', 257) }] });
        Reject(command with { Items = [Leaf() with { Separator = true }] });
        Reject(command with { Items = [Leaf() with { Children = [Leaf(2)] }] });
        Reject(command with { Items = Enumerable.Range(1, 129).Select(i => Leaf((uint)i)).ToArray() });
        var deep = Leaf();
        for (var i = 0; i < 4; i++) deep = new(0, "Group", true, false, false, [deep]);
        Reject(command with { Items = [deep] });
        var branches = Enumerable.Range(0, 2).Select(group => new NativeContextMenuItem(0, "Group", true, false, false,
            Enumerable.Range(1, 128).Select(i => Leaf((uint)(group * 128 + i))).ToArray())).ToArray();
        Reject(command with { Items = branches });
        var labels = Enumerable.Range(1, 128).Select(i => Leaf((uint)i) with { Label = new string('x', 256) }).ToArray();
        Reject(command with { Items = [new(0, "Group", true, false, false, labels)] });
        NativeContextMenuCoordinator.Validate(command with { Items = [Leaf() with { Enabled = false }] });
        var cancel = ProtocolEnvelope.Create("platform_command", new { command = "cancel_context_menu", data = new { session_id = envelope.Id } });
        Assert(PlatformCommandParser.ParseCancelContextMenu(cancel).SessionId == envelope.Id, "cancel uses exact session GUID");
        return Task.CompletedTask;
    }

    public static async Task AdmissionCancellationAndNonblocking()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var cancelled = new ManualResetEventSlim();
        var workerApartment = ApartmentState.Unknown;
        using var coordinator = new NativeContextMenuCoordinator(operation: (command, session, ready) =>
        {
            workerApartment = Thread.CurrentThread.GetApartmentState();
            session.SetCancelAction(() => cancelled.Set());
            ready(); entered.Set(); release.Wait();
            return new(true, Data: "{\"selected_id\":null,\"dismissed_outside\":false}");
        }, validateSource: _ => true);
        var id = Guid.NewGuid();
        var first = coordinator.ShowAsync(Command(), id, CancellationToken.None);
        try
        {
            Assert(entered.Wait(TimeSpan.FromSeconds(3)), "dedicated worker starts");
            Assert(!first.IsCompleted && workerApartment == ApartmentState.STA, "reader is not menu worker");
            Assert(!(await coordinator.ShowAsync(Command(), Guid.NewGuid(), CancellationToken.None)).Success, "busy rejects without queue");
            coordinator.Cancel(Guid.NewGuid());
            Assert(!cancelled.IsSet, "wrong session cannot cancel active loop");
            coordinator.Cancel(id);
            Assert(cancelled.Wait(TimeSpan.FromSeconds(2)), "exact session reaches cancellation boundary");
            Assert(!(await coordinator.ShowAsync(Command(), Guid.NewGuid(), CancellationToken.None)).Success, "cancel does not release stuck native slot");
        }
        finally { release.Set(); await first.WaitAsync(TimeSpan.FromSeconds(3)); }
        using (var disconnected = new CancellationTokenSource())
        using (var disconnectEntered = new ManualResetEventSlim())
        using (var disconnectReached = new ManualResetEventSlim())
        using (var disconnectCoordinator = new NativeContextMenuCoordinator(operation: (_, session, ready) =>
        {
            session.SetCancelAction(() => disconnectReached.Set());
            session.Tracking = true;
            ready(); disconnectEntered.Set(); disconnectReached.Wait(TimeSpan.FromSeconds(3));
            return new(true, Data: JsonSerializer.Serialize(new { selected_id = (uint?)null, dismissed_outside = session.Outside }));
        }, validateSource: _ => true))
        {
            var disconnectWork = disconnectCoordinator.ShowAsync(Command(), Guid.NewGuid(), disconnected.Token);
            Assert(disconnectEntered.Wait(TimeSpan.FromSeconds(3)), "disconnect worker entered");
            disconnectCoordinator.ObservePointerPress(123, 123);
            disconnected.Cancel();
            var terminal = await disconnectWork.WaitAsync(TimeSpan.FromSeconds(3));
            Assert(disconnectReached.IsSet && JsonDocument.Parse(terminal.Data!).RootElement.GetProperty("dismissed_outside").GetBoolean(),
                "session disconnect cancels actual worker and preserves observed external press");
        }
        using var cancellation = new CancellationTokenSource(); cancellation.Cancel();
        var result = await coordinator.ShowAsync(Command(), Guid.NewGuid(), cancellation.Token);
        Assert(result.Success && JsonDocument.Parse(result.Data!).RootElement.GetProperty("selected_id").ValueKind == JsonValueKind.Null,
            "pre-cancel is terminal cancellation, not allocation");
        var dispatcher = new PlatformCommandSequenceDispatcher();
        using var started = new ManualResetEventSlim();
        var later = false;
        try
        {
            var pending = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            await dispatcher.DispatchAsync("show_context_menu", () => { started.Set(); return pending.Task; });
            Assert(started.IsSet, "bounded admission occurs before a following cancel can overtake it");
            await dispatcher.DispatchAsync("cancel_context_menu", () => { later = true; return Task.CompletedTask; });
            pending.SetResult();
            Assert(later, "native modal loop never blocks later exact cancel");
        }
        finally { await dispatcher.DrainAsync(); }
    }

    // Explicit native category: owns every HWND, no SendInput, hooks, services or real user windows.
    public static async Task OwnedHwndOverflowAndExactCancel()
    {
        var source = new TaskCompletionSource<(IntPtr Handle, Action Stop)>(TaskCreationOptions.RunContinuationsAsynchronously);
        var thread = new Thread(() =>
        {
            using var form = new Form { ClientSize = new(300, 440), StartPosition = FormStartPosition.Manual,
                Location = new(100, 100), AutoScaleMode = AutoScaleMode.None, ShowInTaskbar = false, FormBorderStyle = FormBorderStyle.None };
            form.Shown += (_, _) => source.SetResult((form.Handle, () => form.BeginInvoke(new Action(form.Close))));
            Application.Run(form);
        }) { IsBackground = true };
        thread.SetApartmentState(ApartmentState.STA); thread.Start();
        var owned = await source.Task.WaitAsync(TimeSpan.FromSeconds(5));
        using var coordinator = new NativeContextMenuCoordinator();
        var id = Guid.NewGuid();
        Task<WindowsCommandResult>? pending = null;
        try
        {
            Assert(GetClientRect(owned.Handle, out var client) && client.Right == 300 && client.Bottom == 440,
                "fixture source is exactly 300x440 client device pixels");
            // Same real native boundary used by production: GPUI supplies a hidden popup HWND.
            var popupReady = new TaskCompletionSource<(IntPtr Handle, Action Stop)>(TaskCreationOptions.RunContinuationsAsynchronously);
            var popupThread = new Thread(() =>
            {
                using var popup = new Form { ShowInTaskbar = false, FormBorderStyle = FormBorderStyle.None,
                    StartPosition = FormStartPosition.Manual, Location = new(1, 1), Size = new(10, 10) };
                _ = popup.Handle;
                popupReady.SetResult((popup.Handle, () => popup.BeginInvoke(new Action(Application.ExitThread))));
                Application.Run();
            }) { IsBackground = true };
            popupThread.SetApartmentState(ApartmentState.STA); popupThread.Start();
            var styledPopup = await popupReady.Task.WaitAsync(TimeSpan.FromSeconds(3));
            try
            {
                var styledResult = await coordinator.ShowAsync(new(owned.Handle.ToInt64(),
                    (uint)Environment.ProcessId, 299, 30, [], styledPopup.Handle.ToInt64()), Guid.NewGuid(), CancellationToken.None)
                    .WaitAsync(TimeSpan.FromSeconds(5));
                Assert(styledResult.Success, "production styled surface native prepare succeeds");
                using var placement = JsonDocument.Parse(styledResult.Data!);
                Assert(GetClientRect(styledPopup.Handle, out var styledClient) &&
                    styledClient.Right == placement.RootElement.GetProperty("width").GetInt32() &&
                    styledClient.Bottom == placement.RootElement.GetProperty("height").GetInt32(), "actual styled client matches work area readback");
                GetWindowRect(styledPopup.Handle, out var styledBounds);
                GetWindowRect(owned.Handle, out var sourceBounds);
                Assert(styledBounds.Right > sourceBounds.Right || styledBounds.Left < sourceBounds.Left,
                    "independent styled viewport genuinely extends outside source HWND");
                Assert(GetWindow(styledPopup.Handle, 4) == owned.Handle && !IsWindowVisible(styledPopup.Handle),
                    "popup is native-owned by source, still hidden until Rust model placement");
            }
            finally { styledPopup.Stop(); popupThread.Join(TimeSpan.FromSeconds(3)); }
            var command = new ShowContextMenuCommand(owned.Handle.ToInt64(), (uint)Environment.ProcessId, 299, 30,
                [new(0, "Synthetic cascading group", true, false, false, [Leaf()]), Leaf(2)]);
            pending = coordinator.ShowAsync(command, id, CancellationToken.None);
            var overflow = false;
            IntPtr nativePopup = IntPtr.Zero;
            var until = DateTime.UtcNow.AddSeconds(5);
            while (DateTime.UtcNow < until && !pending.IsCompleted && !overflow)
            {
                GetWindowRect(owned.Handle, out var panel);
                EnumWindows((window, _) =>
                {
                    var name = new System.Text.StringBuilder(64);
                    GetClassNameW(window, name, name.Capacity);
                    GetWindowThreadProcessId(window, out var process);
                    if (process == (uint)Environment.ProcessId && name.ToString() == "#32768" && IsWindowVisible(window))
                    {
                        GetWindowRect(window, out var bounds);
                        nativePopup = window;
                        overflow |= bounds.Right > panel.Right || bounds.Left < panel.Left || bounds.Bottom > panel.Bottom || bounds.Top < panel.Top;
                    }
                    return true;
                }, IntPtr.Zero);
                await Task.Delay(25);
            }
            Assert(overflow && IsWindowVisible(owned.Handle), "actual native #32768 menu must extend outside live 300x440 source HWND");
            IntPtr menuHost = IntPtr.Zero;
            EnumWindows((window, _) =>
            {
                GetWindowThreadProcessId(window, out var process);
                if (process != (uint)Environment.ProcessId) return true;
                var caption = new System.Text.StringBuilder(64);
                GetWindowTextW(window, caption, caption.Capacity);
                if (caption.ToString() == "NxTools.NativeContextMenu") menuHost = window;
                return true;
            }, IntPtr.Zero);
            Assert(menuHost != IntPtr.Zero, "tracking owner is a worker-owned surrogate, not source HWND");
            // Expand only the owned system menu via its accessibility provider; no global input.
            var accessible = AutomationElement.FromHandle(nativePopup);
            var group = accessible.FindFirst(TreeScope.Descendants,
                new PropertyCondition(AutomationElement.NameProperty, "Synthetic cascading group"));
            Assert(group is not null, "owned menu exposes its cascading group");
            ((ExpandCollapsePattern)group!.GetCurrentPattern(ExpandCollapsePattern.Pattern)).Expand();
            var cascadeVisible = false;
            until = DateTime.UtcNow.AddSeconds(3);
            while (DateTime.UtcNow < until && !pending.IsCompleted && !cascadeVisible)
            {
                var menus = 0;
                EnumWindows((window, _) =>
                {
                    GetWindowThreadProcessId(window, out var process);
                    if (process != (uint)Environment.ProcessId || !IsWindowVisible(window)) return true;
                    var name = new System.Text.StringBuilder(64);
                    GetClassNameW(window, name, name.Capacity);
                    if (name.ToString() == "#32768") menus++;
                    return true;
                }, IntPtr.Zero);
                cascadeVisible = menus >= 2;
                await Task.Delay(25);
            }
            Assert(cascadeVisible && IsWindowVisible(owned.Handle), "accessibility expansion opens actual cascading #32768 HWND, source remains alive");
            coordinator.Cancel(Guid.NewGuid());
            Assert(!pending.IsCompleted, "wrong native cancellation leaves menu alive");
            coordinator.Cancel(id);
            var result = await pending.WaitAsync(TimeSpan.FromSeconds(5));
            using var data = JsonDocument.Parse(result.Data!);
            Assert(result.Success && data.RootElement.GetProperty("selected_id").ValueKind == JsonValueKind.Null &&
                !data.RootElement.GetProperty("dismissed_outside").GetBoolean() && IsWindowVisible(owned.Handle), "exact cancellation closes menu, not clipboard");
        }
        finally
        {
            coordinator.Cancel(id);
            try { if (pending is not null) await pending.WaitAsync(TimeSpan.FromSeconds(5)); }
            finally { owned.Stop(); thread.Join(TimeSpan.FromSeconds(3)); }
        }
    }
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left, Top, Right, Bottom; }
    private delegate bool EnumCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out Rect rectangle);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassNameW(IntPtr window, System.Text.StringBuilder name, int count);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out Rect rectangle);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowTextW(IntPtr window, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] private static extern IntPtr GetWindow(IntPtr window, uint command);
}
