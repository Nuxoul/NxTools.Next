using System.Runtime.InteropServices;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// NativeAcceptance only: real global hooks and uniquely marked SendInput, never user-app input.
internal static class ProcessWindowPickInputNativeTests
{
    internal static Task MarkedLeftSelectsWithoutDelivery() => Run(false);
    internal static Task MarkedEscapeCancelsWithoutDelivery() => Run(true);

    private static async Task Run(bool escape)
    {
        RequireDesktop();
        var originalForeground = Native.GetForegroundWindow();
        if (!Native.GetPhysicalCursorPos(out var originalCursor)) throw new TestSkippedException("probe cursor unavailable");
        RequireReleasedKeys();
        var ui = new OwnedWindows();
        SafetyApi? api = null;
        ProcessWindowPickCoordinator? coordinator = null;
        Thread? hookOwner = null;
        using var cancellation = new CancellationTokenSource();
        uint sent = 0;
        var moved = false;
        var point = new ProcessPickApi.Point();
        var terminalCount = 0;
        var stage = "ui_ready";
        try
        {
            await ui.Ready.Task.WaitAsync(TimeSpan.FromSeconds(5));
            api = new SafetyApi(ui.Source, ui.Target);
            var terminal = new TaskCompletionSource<ProcessWindowPickChangedEvent>(TaskCreationOptions.RunContinuationsAsynchronously);
            var controller = new ProcessWindowPickController(new ProcessWindowPickNative(api), value =>
            {
                if (value.State != "hover") { Interlocked.Increment(ref terminalCount); terminal.TrySetResult(value); }
                return true;
            }, NullPlatformLog.Instance);
            coordinator = new ProcessWindowPickCoordinator(controller); // Real Loop, not fixture enqueue.
            var session = Guid.NewGuid();
            var start = coordinator.ExecuteAsync(new("start", session, ui.Source, ui.Overlay),
                (uint)Environment.ProcessId, cancellation.Token);
            // Observe the original owner solely to JOIN actual Loop cleanup, including startup failures.
            var ownerField = typeof(ProcessWindowPickCoordinator).GetField("_thread",
                System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
                ?? throw new InvalidOperationException("probe coordinator owner field missing");
            hookOwner = ownerField.GetValue(coordinator) as Thread;
            Check(hookOwner is not null, "probe coordinator did not create its real STA owner");
            stage = "picker_start";
            var started = await start.WaitAsync(TimeSpan.FromSeconds(3));
            if (!started.Success && started.NativeErrorCode is 5 or 1314)
                throw new TestSkippedException("probe hook/desktop permission unavailable");
            Check(started.Success, "probe start failed");
            Check(api.OwnerThread == hookOwner && hookOwner?.GetApartmentState() == ApartmentState.STA,
                "probe hooks must belong to real coordinator STA");
            Native.Input placement = default;
            Check(!Native.IsWindowVisible(new(ui.Source)) && !Native.IsWindowVisible(new(ui.Overlay)),
                "probe source must hide and overlay must remain hidden");
            stage = "placement_prepare";
            await ui.Invoke(() =>
            {
                RequireReleasedKeys();
                if (Native.GetForegroundWindow() != new IntPtr(ui.Target))
                    throw new TestSkippedException("probe owned foreground unavailable");
                var bounds = new ProcessPickApi().ClientViewport(ui.Target);
                point = new() { X = bounds.Left + (int)bounds.Width / 2, Y = bounds.Top + (int)bounds.Height / 2 };
                placement = api.Placement(point);
            });
            stage = "placement_callback";
            moved = Native.SendInput(1, [placement], Marshal.SizeOf<Native.Input>()) == 1;
            if (!moved) throw new TestSkippedException("probe cursor placement denied");
            await api.Placed.Task.WaitAsync(TimeSpan.FromSeconds(2));
            Check(!api.CallbackFailed, "probe placement callback failed");
            if (api.Aborted) throw new TestSkippedException("probe owned target obstructed; movement blocked");
            stage = "input_arm";
            await ui.Invoke(() =>
            {
                if (!api.SafeTarget()) throw new TestSkippedException("probe owned target obstructed");
                api.Armed = true;
            });
            stage = "input_pair_callback";
            // Both transitions are inserted by ONE call; OS injection flags and packet pointers stay untouched.
            sent = Native.SendInput(2, Pair(escape, api.Marker), Marshal.SizeOf<Native.Input>());
            if (sent == 0) throw new TestSkippedException("probe SendInput denied");
            Check(sent == 2, "probe incomplete input pair");
            await api.Pair.Task.WaitAsync(TimeSpan.FromSeconds(2));
            Check(!api.CallbackFailed, "probe native callback failed");
            if (api.Aborted) throw new TestSkippedException("probe ownership changed; marked input blocked");
            // Drain the target queue so forwarded packets are counted before assertions.
            stage = "target_queue_drain";
            await Task.Delay(150);
            await ui.Invoke(() => { });
            if (api.Intervened) throw new TestSkippedException("probe interrupted by non-probe input");
            Check(api.Down == 1 && api.Up == 1 && api.Injected == 2, "probe must observe two actual injected hook packets");
            Check(api.Consumed == 2, "production callback must consume both marked transitions");
            Check(ui.LeftDown == 0 && ui.LeftUp == 0 && ui.EscapeDown == 0 && ui.EscapeUp == 0,
                "owned target received intercepted input");
            stage = "terminal";
            var result = await terminal.Task.WaitAsync(TimeSpan.FromSeconds(3));
            Check(result.SessionId == session && result.State == (escape ? "cancelled" : "selected"),
                "probe must reach the actual controller terminal");
            Check(escape ? result.Candidate is null : result.Candidate is { } candidate &&
                candidate.WindowHandle == ui.Target && candidate.ProcessId == (uint)Environment.ProcessId,
                "probe terminal target mismatch");
            Check(Native.IsWindowVisible(new(ui.Source)) && !Native.IsWindowVisible(new(ui.Overlay)),
                "probe terminal must restore source without showing overlay");
        }
        catch (TimeoutException error)
        {
            throw new InvalidOperationException("probe timed out at " + stage, error);
        }
        finally
        {
            // A partial SendInput still owns its release. Keep the safety hook and UI alive until it drains.
            if (sent == 1 && api is not null)
                _ = Native.SendInput(1, [Pair(escape, api.Marker)[1]], Marshal.SizeOf<Native.Input>());
            cancellation.Cancel();
            coordinator?.Dispose();
            var hookStopped = hookOwner is null || hookOwner.Join(TimeSpan.FromSeconds(5));
            // Exact self-owned handles only; fallback disarms callbacks before removing a failed owner's hooks.
            api?.DisarmAndUnhook();
            try
            {
                if (ui.Ready.Task.IsCompletedSuccessfully)
                    await ui.Invoke(() =>
                    {
                        var untouched = api is null || !api.Intervened;
                        if (moved && untouched && Native.GetCursorPos(out var current) && current.X == point.X && current.Y == point.Y)
                            Check(Native.SetCursorPos(originalCursor.X, originalCursor.Y), "probe cursor restore denied");
                        var foreground = Native.GetForegroundWindow();
                        if (untouched && (foreground == new IntPtr(ui.Target) || foreground == new IntPtr(ui.Source)) && Native.IsWindow(originalForeground))
                        {
                            _ = Native.SetForegroundWindow(originalForeground);
                            Check(Native.GetForegroundWindow() == originalForeground, "probe foreground restore denied");
                        }
                    });
            }
            finally
            {
                await ui.Stop();
                Console.WriteLine($"PROBE cleanup hook_stopped={hookStopped} down={api?.Down ?? 0} up={api?.Up ?? 0} injected={api?.Injected ?? 0} consumed={api?.Consumed ?? 0} terminal={terminalCount} left_down={ui.LeftDown} left_up={ui.LeftUp} escape_down={ui.EscapeDown} escape_up={ui.EscapeUp}");
                GC.KeepAlive(api);
            }
            Check(hookStopped, "probe hook owner did not exit");
            Check(api is null || api.Unhooked, "probe native hook cleanup failed");
            if (coordinator is not null && ui.Ready.Task.IsCompletedSuccessfully)
                Check(terminalCount <= 1, "probe duplicate terminal");
        }
    }

    private static Native.Input[] Pair(bool escape, UIntPtr marker) => escape
        ? [new() { Type = 1, Data = new() { Keyboard = new() { Key = 27, Extra = marker } } },
           new() { Type = 1, Data = new() { Keyboard = new() { Key = 27, Flags = 2, Extra = marker } } }]
        : [new() { Type = 0, Data = new() { Mouse = new() { Flags = 2, Extra = marker } } },
           new() { Type = 0, Data = new() { Mouse = new() { Flags = 4, Extra = marker } } }];

    private static void RequireReleasedKeys()
    {
        foreach (var key in new[] { 1, 2, 4, 16, 17, 18, 27, 91, 92 })
            if ((Native.GetAsyncKeyState(key) & 0x8000) != 0)
                throw new TestSkippedException("probe requires released buttons/modifiers/Escape");
    }
    private static void RequireDesktop()
    {
        if (!Environment.UserInteractive || Native.GetForegroundWindow() == IntPtr.Zero)
            throw new TestSkippedException("probe requires interactive input desktop");
        var desktop = Native.OpenInputDesktop(0, false, 1);
        if (desktop == IntPtr.Zero) throw new TestSkippedException("probe input desktop unavailable");
        _ = Native.CloseDesktop(desktop);
    }
    private static void Check(bool value, string stage)
    { if (!value) throw new InvalidOperationException(stage); }

    private sealed class SafetyApi(long source, long target) : ProcessPickApi
    {
        // Only ownership/enumeration are substituted: no foreign HWND enumeration/hiding.
        // All facts, process leases, styles, visibility and process-name resolution remain real.
        internal override uint CurrentProcessId
        {
            get
            {
                OwnerThread = Thread.CurrentThread;
                Check(Native.SetThreadDpiAwarenessContext(new(-4)) != IntPtr.Zero, "probe hook DPI unavailable");
                return 0; // Permit three self-owned HWNDs in this one test process.
            }
        }
        internal override IReadOnlyList<long> TopLevelWindows() => [source];
        // Positive32-bit markers survive the native input path without sign-extension ambiguity.
        internal readonly UIntPtr Marker = new((BitConverter.ToUInt32(Guid.NewGuid().ToByteArray()) & 0x7FFF_FFFEu) | 1u);
        internal readonly TaskCompletionSource Pair = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal readonly TaskCompletionSource Placed = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private ProcessPickApi.Point _placement;
        private volatile bool _placing;
        private readonly List<(IntPtr Handle, HookProc Callback)> _hooks = [];
        internal Thread? OwnerThread;
        internal volatile bool Armed;
        private volatile bool _disarmed;
        private int _aborted, _callbackFailed, _intervened, _down, _up, _injected, _consumed;
        internal bool Aborted => Volatile.Read(ref _aborted) != 0;
        internal bool CallbackFailed => Volatile.Read(ref _callbackFailed) != 0;
        internal bool Intervened => Volatile.Read(ref _intervened) != 0;
        internal int Down => Volatile.Read(ref _down);
        internal int Up => Volatile.Read(ref _up);
        internal int Injected => Volatile.Read(ref _injected);
        internal int Consumed => Volatile.Read(ref _consumed);
        internal bool Unhooked { get; private set; }

        internal bool SafeTarget() => SafeForeground() && RootAtPointer() == target;
        private bool SafeForeground()
        {
            var info = new Native.GuiThreadInfo { Size = (uint)Marshal.SizeOf<Native.GuiThreadInfo>() };
            return Native.GetForegroundWindow() == new IntPtr(target) && Native.GetGUIThreadInfo(0, ref info) &&
                Native.GetAncestor(info.Focus, 2) == new IntPtr(target) &&
                (info.Capture == IntPtr.Zero || Native.GetAncestor(info.Capture, 2) == new IntPtr(target));
        }
        internal Native.Input Placement(ProcessPickApi.Point point)
        {
            _placement = point; _placing = true;
            var viewport = Viewport();
            return new() { Data = new() { Mouse = new()
            {
                X = checked((int)(((point.X - (long)viewport.Left) * 65536 + 32768) / viewport.Width)),
                Y = checked((int)(((point.Y - (long)viewport.Top) * 65536 + 32768) / viewport.Height)),
                Flags = 0xC001, Extra = Marker // MOVE | ABSOLUTE | VIRTUALDESK, destination owned and guarded.
            } } };
        }
        internal override IntPtr Install(int kind, HookProc callback)
        {
            HookProc guarded = (code, message, data) =>
            {
                if (code < 0) return base.Next(code, message, data);
                var mouse = kind == 14 ? Marshal.PtrToStructure<MouseData>(data) : default;
                var keyboard = kind == 13 ? Marshal.PtrToStructure<KeyboardData>(data) : default;
                if ((kind == 14 ? mouse.Extra : keyboard.Extra) != Marker)
                {
                    Interlocked.Exchange(ref _intervened, 1);
                    return base.Next(code, message, data); // Physical/user/non-probe input is never intercepted.
                }
                try
                {
                    if (kind == 14 && message.ToInt64() == 0x200)
                    {
                        if (_disarmed || Aborted || !_placing || mouse.X != _placement.X || mouse.Y != _placement.Y ||
                            !SafeForeground() || RootAt(mouse.X, mouse.Y) != target)
                        {
                            Interlocked.Exchange(ref _aborted, 1); Placed.TrySetResult();
                            return new IntPtr(1);
                        }
                        _placing = false;
                        var movement = callback(code, message, data);
                        Placed.TrySetResult();
                        return movement;
                    }
                    var down = kind == 14 ? message.ToInt64() == 0x201 : message.ToInt64() == 0x100;
                    var up = kind == 14 ? message.ToInt64() == 0x202 : message.ToInt64() == 0x101;
                    if (_disarmed || Aborted || !Armed || !(down || up) || (kind == 13 && keyboard.Key != 27) ||
                        !SafeTarget() || (kind == 14 && RootAt(mouse.X, mouse.Y) != target))
                    {
                        Interlocked.Exchange(ref _aborted, 1);
                        Pair.TrySetResult();
                        return new IntPtr(1); // Never deliver a marked probe to a foreign window.
                    }
                    if (down) Interlocked.Increment(ref _down); else Interlocked.Increment(ref _up);
                    if ((kind == 14 ? mouse.Flags & 1 : keyboard.Flags & 0x10) != 0) Interlocked.Increment(ref _injected);
                    var result = callback(code, message, data); // Raw OS packet, including injected flags, unmodified.
                    if (result == IntPtr.Zero && !SafeTarget())
                    {
                        Interlocked.Exchange(ref _aborted, 1);
                        Pair.TrySetResult();
                        return new IntPtr(1);
                    }
                    if (result != IntPtr.Zero) Interlocked.Increment(ref _consumed);
                    if (Down + Up >= 2) Pair.TrySetResult();
                    return result;
                }
                catch
                {
                    Interlocked.Exchange(ref _callbackFailed, 1);
                    Interlocked.Exchange(ref _aborted, 1);
                    Pair.TrySetResult();
                    Placed.TrySetResult();
                    return new IntPtr(1); // No managed exception crosses the native callback.
                }
            };
            var handle = base.Install(kind, guarded); // Actual WH_MOUSE_LL / WH_KEYBOARD_LL on coordinator STA.
            if (handle != IntPtr.Zero) _hooks.Add((handle, guarded)); // Root wrappers AND production delegates.
            return handle;
        }
        internal void DisarmAndUnhook()
        {
            _disarmed = true;
            Unhooked = true;
            foreach (var hook in _hooks)
            {
                // After the real owner exits, ERROR_INVALID_HOOK_HANDLE proves native RemoveHooks completed.
                var removed = Native.UnhookWindowsHookEx(hook.Handle);
                var code = Marshal.GetLastWin32Error();
                if (removed || code != 1404) Unhooked = false;
                GC.KeepAlive(hook.Callback);
            }
            GC.KeepAlive(_hooks);
        }
    }

    private sealed class OwnedWindows : IMessageFilter
    {
        internal readonly TaskCompletionSource Ready = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly Thread _thread;
        private volatile bool _stop;
        internal long Source, Overlay, Target;
        internal int LeftDown, LeftUp, EscapeDown, EscapeUp;
        private Form? _targetForm;
        internal OwnedWindows()
        {
            _thread = new Thread(Loop) { IsBackground = true, Name = "NxTools.PickerInputProbe.UI" };
            _thread.SetApartmentState(ApartmentState.STA);
            _thread.Start();
        }
        private void Loop()
        {
            var dpi = Native.SetThreadDpiAwarenessContext(new(-4));
            try
            {
                if (dpi == IntPtr.Zero) throw new TestSkippedException("probe UI DPI unavailable");
                using var context = new ApplicationContext();
                using var source = MakeForm(40, 40);
                using var overlay = MakeForm(40, 40);
                using var target = MakeForm(40, 160);
                _targetForm = target;
                Source = source.Handle.ToInt64(); Overlay = overlay.Handle.ToInt64(); Target = target.Handle.ToInt64();
                Application.AddMessageFilter(this);
                try
                {
                    source.Show(); target.Show(); target.Activate();
                    _ = Native.SetForegroundWindow(target.Handle);
                    _ = Native.SetFocus(target.Handle);
                    using var timer = new System.Windows.Forms.Timer { Interval = 20 };
                    timer.Tick += (_, _) => { if (_stop) context.ExitThread(); };
                    timer.Start();
                    target.BeginInvoke(new Action(() => Ready.TrySetResult()));
                    Application.Run(context);
                }
                finally { Application.RemoveMessageFilter(this); }
            }
            catch (Exception error) { Ready.TrySetException(error); }
            finally { if (dpi != IntPtr.Zero) _ = Native.SetThreadDpiAwarenessContext(dpi); }
        }
        private static Form MakeForm(int x, int y) => new()
        {
            StartPosition = FormStartPosition.Manual, Location = new(x, y), ClientSize = new(240, 100),
            FormBorderStyle = FormBorderStyle.None, AutoScaleMode = AutoScaleMode.None, ShowInTaskbar = false
        };
        public bool PreFilterMessage(ref Message message)
        {
            if (message.HWnd.ToInt64() != Target) return false;
            if (message.Msg == 0x201) Interlocked.Increment(ref LeftDown);
            if (message.Msg == 0x202) Interlocked.Increment(ref LeftUp);
            if (message.WParam.ToInt64() == 27)
            {
                if (message.Msg is 0x100 or 0x104) Interlocked.Increment(ref EscapeDown);
                if (message.Msg is 0x101 or 0x105) Interlocked.Increment(ref EscapeUp);
            }
            return false; // Count actual queue delivery before WinForms dialog-key preprocessing.
        }
        internal async Task Invoke(Action action)
        {
            var completion = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var target = _targetForm ?? throw new InvalidOperationException("probe UI not ready");
            target.BeginInvoke(new Action(() =>
            {
                try { action(); completion.TrySetResult(); }
                catch (Exception error) { completion.TrySetException(error); }
            }));
            await completion.Task.WaitAsync(TimeSpan.FromSeconds(2));
        }
        internal Task Stop()
        {
            _stop = true;
            Check(_thread.Join(TimeSpan.FromSeconds(3)), "probe UI owner did not exit");
            Check(!Native.IsWindow(new(Source)) && !Native.IsWindow(new(Overlay)) && !Native.IsWindow(new(Target)),
                "probe owned HWND cleanup incomplete");
            return Task.CompletedTask;
        }
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal struct Input { internal uint Type; internal InputUnion Data; }
        [StructLayout(LayoutKind.Explicit)] internal struct InputUnion
        {
            [FieldOffset(0)] internal MouseInput Mouse;
            [FieldOffset(0)] internal KeyboardInput Keyboard;
        }
        [StructLayout(LayoutKind.Sequential)] internal struct MouseInput
        { internal int X, Y; internal uint Data, Flags, Time; internal UIntPtr Extra; }
        [StructLayout(LayoutKind.Sequential)] internal struct KeyboardInput
        { internal ushort Key, Scan; internal uint Flags, Time; internal UIntPtr Extra; }
        [StructLayout(LayoutKind.Sequential)] internal struct GuiThreadInfo
        {
            internal uint Size, Flags;
            internal IntPtr Active, Focus, Capture, MenuOwner, MoveSize, Caret;
            internal ProcessPickApi.Rect CaretRect;
        }
        [DllImport("user32.dll")] internal static extern bool GetPhysicalCursorPos(out ProcessPickApi.Point point);
        [DllImport("user32.dll", SetLastError = true)] internal static extern uint SendInput(uint count, Input[] input, int size);
        [DllImport("user32.dll")] internal static extern bool GetCursorPos(out ProcessPickApi.Point point);
        [DllImport("user32.dll")] internal static extern bool SetCursorPos(int x, int y);
        [DllImport("user32.dll")] internal static extern short GetAsyncKeyState(int key);
        [DllImport("user32.dll")] internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll")] internal static extern bool SetForegroundWindow(IntPtr hwnd);
        [DllImport("user32.dll")] internal static extern IntPtr SetFocus(IntPtr hwnd);
        [DllImport("user32.dll")] internal static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
        [DllImport("user32.dll")] internal static extern bool GetGUIThreadInfo(uint thread, ref GuiThreadInfo info);
        [DllImport("user32.dll")] internal static extern bool IsWindow(IntPtr hwnd);
        [DllImport("user32.dll")] internal static extern bool IsWindowVisible(IntPtr hwnd);
        [DllImport("user32.dll", SetLastError = true)] internal static extern bool UnhookWindowsHookEx(IntPtr hook);
        [DllImport("user32.dll")] internal static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
        [DllImport("user32.dll", SetLastError = true)] internal static extern IntPtr OpenInputDesktop(uint flags, bool inherit, uint access);
        [DllImport("user32.dll")] internal static extern bool CloseDesktop(IntPtr desktop);
    }
}
