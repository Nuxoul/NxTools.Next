using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;
// Native observation signature: no payload/window title or per-tick diagnostic output.
// Logical placement/recovery stays with Rust; this only notifies after a changed snapshot.
internal readonly record struct FloatingAssistantEnvironmentSignature(
    long MonitorHandle, int MonitorLeft, int MonitorTop, int MonitorRight, int MonitorBottom,
    int WorkLeft, int WorkTop, int WorkRight, int WorkBottom, uint Dpi, bool Fullscreen);

internal sealed class FloatingAssistantEnvironmentTracker
{
    private FloatingAssistantEnvironmentSignature? _published;

    internal bool TryPublish(FloatingAssistantEnvironmentSignature snapshot, Func<bool> publish)
    {
        if (_published == snapshot) return false;
        if (!publish()) return false;
        _published = snapshot;
        return true;
    }

    internal void Reset() => _published = null;
}

// Geometry alone also matches Shell desktop and ordinary maximized framed windows.
internal static class FloatingAssistantFullscreenPolicy
{
    internal static bool Evaluate(bool valid, bool visible, bool minimized, bool desktop,
        long style, bool coversMonitor, out string reason)
    {
        reason = !valid ? "invalid_foreground" : !visible ? "hidden_foreground" :
            minimized ? "minimized_foreground" : desktop ? "shell_desktop" :
            (style & 0x00C40000) != 0 ? "framed_window" :
            !coversMonitor ? "not_covering_monitor" : "borderless_fullscreen";
        return reason == "borderless_fullscreen";
    }
}

// One queue, one persistent OLE STA and one renderer lease per connected pipe. Native calls
// never execute on the pipe reader or the shared EventWindow thread.
internal sealed class FloatingAssistantCoordinator : IDisposable
{
    private readonly object _gate = new();
    private readonly Queue<Work> _pending = new();
    private Work? _cleanupPending;
    private readonly Queue<(FloatingAssistantDropData Data, TaskCompletionSource<bool> Completion)> _fixtureCallbacks = new();
    private readonly uint _peer;
    private readonly Func<ProtocolEnvelope, bool> _publish;
    private readonly IPlatformLog _log;
    private readonly Thread _thread;
    private readonly CancellationTokenSource _pickerStop = new();
    private readonly ManualResetEventSlim _ready = new();
    private FloatingAssistantCommand? _lease;
    private FloatingAssistantInputSurface? _input;
    // A stopped pipe's failed deletion still consumes the same process-wide image slot.
    private static int _dropSlot;
    private readonly Dictionary<Guid, DropLease> _drops = new();
    private sealed class DropLease(FloatingAssistantCommand command, Guid correlation)
    {
        internal FloatingAssistantCommand Command { get; } = command;
        internal Guid Correlation { get; } = correlation;
        internal long Started { get; } = Stopwatch.GetTimestamp();
        internal string? Directory { get; set; }
        internal bool Retired { get; set; }
        internal bool PendingLogged { get; set; }
    }
    private bool _visible;
    private readonly FloatingAssistantEnvironmentTracker _environment = new();
    private readonly Win32DesktopWindowOperations _desktopWindows = new();
    private Guid _leaseCorrelation;
    private bool? _lastFullscreen;
    private bool _topmostRepairFailed;
    private long _lastTopmostFailure;
    private ulong _sequence;
    private volatile bool _stopping;
    private int _ownerThread;
    private int _queued;
    private sealed record Work(FloatingAssistantCommand Command, Guid Correlation, long Started,
        TaskCompletionSource<WindowsCommandResult> Completion);

    internal FloatingAssistantCoordinator(uint peer, Func<ProtocolEnvelope, bool> publish, IPlatformLog log)
    {
        _peer = peer;
        _publish = publish;
        _log = log;
        _thread = new Thread(Loop) { IsBackground = true, Name = "NxTools.FloatingAssistant.OLE" };
        _thread.SetApartmentState(ApartmentState.STA);
        _thread.Start();
        if (!_ready.Wait(TimeSpan.FromSeconds(5))) { _stopping = true; throw new InvalidOperationException("Floating STA failed to start."); }
    }

    internal Task<WindowsCommandResult> Dispatch(ProtocolEnvelope envelope)
    {
        FloatingAssistantCommand command;
        try { command = FloatingAssistantContract.Parse(envelope); }
        catch (InvalidDataException) { return Task.FromResult(new WindowsCommandResult(false, "invalid_request")); }
        var id = envelope.CorrelationId is { } correlation && correlation != Guid.Empty ? correlation : envelope.Id;
        if (id == Guid.Empty) return Task.FromResult(new WindowsCommandResult(false, "invalid_request"));
        var started = Stopwatch.GetTimestamp();
        var cleanup = command.Operation is "release_drop" or "retire";
        Task<WindowsCommandResult> result;
        lock (_gate)
        {
            if (_stopping || (_queued >= 16 && !cleanup) || command.SourceProcessId != _peer || _peer == 0 ||
                (command.Operation == "prepare" ? _lease is not null : command.Operation != "pick_image" && !Matches(command)))
                return Task.FromResult(new WindowsCommandResult(false, "stale_or_busy"));
            if (cleanup && _cleanupPending is { } previous)
            {
                if (previous.Command.SessionId != command.SessionId || previous.Command.Generation != command.Generation)
                    return Task.FromResult(new WindowsCommandResult(false, "stale_or_busy"));
                if (previous.Command.Operation == "retire" ||
                    (command.Operation == "release_drop" && previous.Command.DropId == command.DropId))
                    return previous.Completion.Task;
                if (command.Operation != "retire") return Task.FromResult(new WindowsCommandResult(false, "stale_or_busy"));
                // Retirement transfers this session's release to the same continuing STA owner.
                previous.Completion.TrySetResult(new(true));
                Log("terminal", previous.Correlation, previous.Command, previous.Started, new(true), "retirement_owns_cleanup");
            }
            // Picker completion is detached by the pipe, not by this native queue; it
            // uses its own existing bounded STA runner and returns selected_path directly.
            if (command.Operation == "pick_image")
            {
                if (Interlocked.CompareExchange(ref _pickerBusy, 1, 0) != 0)
                    return Task.FromResult(new WindowsCommandResult(false, "picker_busy"));
                // Picker uses its own fixed STA; no HWND/message pump calls are held under this lock.
                return Task.Run(() => PickImage(command, id));
            }
            // Install lease synchronously so update/retire cannot overtake a queued preparation.
            if (command.Operation == "prepare") _lease = command;
            var completion = new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
            var work = new Work(command, id, started, completion);
            if (cleanup && (_queued >= 16 || _cleanupPending is not null)) _cleanupPending = work;
            else { _pending.Enqueue(work); _queued++; }
            result = completion.Task;
        }
        Log("accepted", id, command, started);
        return result;
    }

    private bool Matches(FloatingAssistantCommand command) => _lease is { } current &&
        command.SessionId == current.SessionId && command.Generation == current.Generation &&
        command.WindowHandle == current.WindowHandle && command.SourceWindowHandle == current.SourceWindowHandle &&
        command.SourceProcessId == current.SourceProcessId;

    // Synthetic drop-snapshot fixture uses the same fixed STA and production consumer;
    // it does not call the user's clipboard or simulate an external OLE gesture.
    internal Task<bool> ReceiveDropForFixture(FloatingAssistantDropData captured)
    {
        var completion = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        lock (_gate)
        {
            if (_stopping || _queued >= 16) return Task.FromResult(false);
            _queued++;
            _fixtureCallbacks.Enqueue((captured, completion));
        }
        return completion.Task;
    }
    private void Loop()
    {
        _ownerThread = Environment.CurrentManagedThreadId;
        using var context = new ApplicationContext();
        using var timer = new System.Windows.Forms.Timer { Interval = 25 };
        var stopCleaned = false;
        timer.Tick += (_, _) =>
        {
            if (_stopping)
            {
                if (!stopCleaned)
                {
                    try { Cleanup(); }
                    catch (Exception error) when (error is COMException or Win32Exception or InvalidOperationException)
                    { /* Cleanup's finally retains staged ownership; stopped receipts still drain. */ }
                    finally { DrainStoppedRequests(); stopCleaned = true; }
                }
                if (Environment.TickCount64 - _lastObservation >= 500)
                {
                    _lastObservation = Environment.TickCount64;
                    RetryDropCleanup();
                }
                // The original STA remains the cleanup owner after Dispose's bounded join.
                if (_drops.Count == 0) context.ExitThread();
                return;
            }
            // Retire/stop invalidates admission on the reader; only this STA touches native state.
            Work? work;
            (FloatingAssistantDropData Data, TaskCompletionSource<bool> Completion)? fixture;
            lock (_gate)
            {
                fixture = _fixtureCallbacks.Count > 0 ? _fixtureCallbacks.Dequeue() : null;
                work = fixture is null && _pending.Count > 0 ? _pending.Dequeue() : null;
                if (fixture is not null || work is not null) _queued--;
                if (_cleanupPending is { } cleanup && _queued < 16)
                {
                    _pending.Enqueue(cleanup);
                    _cleanupPending = null;
                    _queued++;
                }
            }
            if (fixture is { } request)
            {
                try { request.Completion.TrySetResult(ReceiveDrop(request.Data)); }
                catch (Exception error) { request.Completion.TrySetException(error); }
            }
            if (work is not null) Execute(work);
            // Native observation is bounded at 500ms, independent of hidden/fullscreen input.
            if (Environment.TickCount64 - _lastObservation >= 500)
            {
                _lastObservation = Environment.TickCount64;
                RetryDropCleanup();
                Observe();
            }
        };
        timer.Start();
        _ready.Set();
        Application.Run(context);
        timer.Stop();
        try { Cleanup(); } catch (Exception) { /* native teardown must still complete queued requests */ }
        DrainStoppedRequests();
    }

    private void DrainStoppedRequests()
    {
        lock (_gate)
        {
            if (_cleanupPending is { } cleanup)
            {
                Log("terminal", cleanup.Correlation, cleanup.Command, cleanup.Started, new(false, "disconnected"));
                cleanup.Completion.TrySetResult(new(false, "disconnected"));
                _cleanupPending = null;
            }
            while (_fixtureCallbacks.TryDequeue(out var request)) request.Completion.TrySetResult(false);
            while (_pending.TryDequeue(out var work))
            {
                Log("terminal", work.Correlation, work.Command, work.Started, new(false, "disconnected"));
                work.Completion.TrySetResult(new(false, "disconnected"));
            }
            _queued = 0;
            _lease = null;
        }
    }
    private long _lastObservation;

    private void Execute(Work work)
    {
        var command = work.Command;
        Log("start", work.Correlation, command, work.Started);
        WindowsCommandResult result;
        try
        {
            bool current;
            lock (_gate) current = !_stopping && Matches(command);
            if (!current)
            {
                result = new(false, "stale_session");
                Log("terminal", work.Correlation, command, work.Started, result);
                work.Completion.TrySetResult(result);
                return;
            }
            result = command.Operation switch
            {
                "prepare" => Prepare(command),
                "update" => Apply(command, null),
                "show" => Apply(command, true),
                "hide" => Hide(command),
                "retire" => Retire(command, work.Correlation),
                "release_drop" => ReleaseDrop(command),
                _ => new(false, "invalid_request")
            };
        }
        catch (Exception error) when (error is Win32Exception or InvalidDataException or COMException or InvalidOperationException or IOException or OverflowException)
        {
            result = new(false, "native_failure", NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
            if (command.Operation is "show" or "update") Publish(command, work.Correlation, "failed");
        }
        if (command.Operation == "prepare" && !result.Success)
        {
            try { Cleanup(); } catch (Exception) { /* original native failure wins; cleanup still attempted */ }
            lock (_gate) { if (Matches(command)) _lease = null; }
        }
        if (command.Operation is "show" or "update" && !result.Success) SafeHide();
        if (result.Success && command.Operation is "prepare" or "update" or "show")
            _leaseCorrelation = work.Correlation;
        Log("terminal", work.Correlation, command, work.Started, result);
        work.Completion.TrySetResult(result);
    }

    private WindowsCommandResult Prepare(FloatingAssistantCommand command)
    {
        var target = new IntPtr(command.WindowHandle);
        if (!Identity(target, command.SourceProcessId) ||
            !Identity(new IntPtr(command.SourceWindowHandle), command.SourceProcessId) ||
            Native.IsWindowVisible(target)) return new(false, "invalid_window_identity");
        SetStyle(target, -20, 0x00080000 | 0x00000020 | 0x08000000 | 0x00000080, 0x00040000 | 0x00000100);
        SetStyle(target, -16, unchecked((int)0x80000000), 0x00C00000 | 0x00040000);
        if (!Native.SetLayeredWindowAttributes(target, 0, Alpha(command.Opacity), 2)) throw LastError();
        _input = new FloatingAssistantInputSurface(target, _peer, ReceiveDrop, ReceiveInput, _log);
        _visible = false;
        return Apply(command, false);
    }
    private WindowsCommandResult Apply(FloatingAssistantCommand command, bool? reveal)
    {
        var target = new IntPtr(command.WindowHandle);
        if (_input is null || !Identity(target, _peer) || !Identity(new IntPtr(command.SourceWindowHandle), _peer))
        { SafeHide(); return new(false, "source_retired"); }
        var monitor = Native.MonitorFromPoint(new Native.Point(command.X, command.Y), 2);
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        if (monitor == IntPtr.Zero || !Native.GetMonitorInfoW(monitor, ref info)) throw LastError();
        var dpi = Native.GetDpiForMonitor(monitor, 0, out var dpiX, out _) == 0 ? dpiX : Native.GetDpiForWindow(target);
        if (dpi is < 48 or > 960 || info.Work.Right <= info.Work.Left || info.Work.Bottom <= info.Work.Top) throw new InvalidDataException("invalid_monitor");
        var size = checked((int)Math.Round(command.Size * (double)dpi / 96, MidpointRounding.AwayFromZero));
        size = Math.Min(size, Math.Min(info.Work.Right - info.Work.Left, info.Work.Bottom - info.Work.Top));
        // Only explicit edge-hidden updates can leave eight device pixels visible.
        // Prepare and every ordinary show/update recover the complete client in work area.
        var edgeHidden = command.Operation != "prepare" && command.EdgeHidden;
        var x = (int)Math.Clamp((long)command.X, (long)info.Work.Left + (edgeHidden ? 8 - size : 0),
            (long)info.Work.Right - (edgeHidden ? 8 : size));
        var y = (int)Math.Clamp((long)command.Y, (long)info.Work.Top + (edgeHidden ? 8 - size : 0),
            (long)info.Work.Bottom - (edgeHidden ? 8 : size));
        if (!Native.SetLayeredWindowAttributes(target, 0, Alpha(command.Opacity), 2) ||
            !Native.SetWindowPos(target, new IntPtr(-1), x, y, size, size, 0x0010 | 0x0020)) throw LastError();
        var rect = ReadClient(target);
        var settledMonitor = Native.MonitorFromWindow(target, 2);
        if (settledMonitor != monitor || rect.Left != x || rect.Top != y ||
            rect.Right - rect.Left != size || rect.Bottom - rect.Top != size ||
            !Policy(target)) throw new InvalidDataException("client_policy_mismatch");
        _input.PositionLocked = command.PositionLocked;
        if (reveal.HasValue) _visible = reveal.Value;
        if (_visible)
        {
            _ = Native.ShowWindow(target, 8); // SW_SHOWNA, never activate
            if (!Native.IsWindowVisible(target)) throw new InvalidDataException("renderer_not_visible");
        }
        else _ = Native.ShowWindow(target, 0);
        _input.SetInteraction(_visible, command.ClickThrough);
        if (Native.IsWindowVisible(_input.Handle) != (_visible && !command.ClickThrough))
            throw new InvalidDataException("input_policy_mismatch");
        lock (_gate) { if (Matches(command)) _lease = command; }
        return BoundsResult(target, dpi, info);
    }

    private WindowsCommandResult Hide(FloatingAssistantCommand command)
    {
        SafeHide();
        var target = new IntPtr(command.WindowHandle);
        if (!Identity(target, _peer)) return new(false, "source_retired");
        var monitor = Native.MonitorFromWindow(target, 2);
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        if (!Native.GetMonitorInfoW(monitor, ref info)) throw LastError();
        return BoundsResult(target, Native.GetDpiForWindow(target), info);
    }

    private WindowsCommandResult Retire(FloatingAssistantCommand command, Guid correlation)
    {
        Cleanup();
        Publish(command, correlation, "retired");
        lock (_gate) { if (Matches(command)) _lease = null; }
        return new(true);
    }

    private WindowsCommandResult ReleaseDrop(FloatingAssistantCommand command)
    {
        if (!_drops.TryGetValue(command.DropId!.Value, out var drop) || drop.Retired ||
            drop.Command.SessionId != command.SessionId || drop.Command.Generation != command.Generation)
            return new(false, "stale_drop");
        drop.Retired = true;
        DeleteOwned(command.DropId.Value, drop);
        return new(true);
    }

    private WindowsCommandResult PickImage(FloatingAssistantCommand command, Guid id)
    {
        var started = Stopwatch.GetTimestamp();
        Log("accepted", id, command, started);
        Log("start", id, command, started);
        WindowsCommandResult result;
        try
        {
            if (!Identity(new IntPtr(command.SourceWindowHandle), _peer))
                result = new(false, "invalid_source");
            else
            {
                result = RunImagePicker(token => StaDialogRunner.RunInline(() =>
                    RunImagePickerWithOwner(command.SourceWindowHandle, command.SourceProcessId, owner =>
                    {
                        using var dialog = new OpenFileDialog { Title = "选择悬浮图片",
                            Filter = "图片|*.png;*.gif;*.jpg;*.jpeg;*.bmp;*.webp",
                            CheckFileExists = true, CheckPathExists = true, Multiselect = false, RestoreDirectory = true };
                        return new WindowsCommandResult(true, Data: JsonSerializer.Serialize(new {
                            selected_path = dialog.ShowDialog(owner) == DialogResult.OK ? dialog.FileName : null }));
                    }, token), token), _pickerStop.Token);
                lock (_gate) if (_stopping) result = new(false, "picker_retired");
            }
        }
        catch (Exception error) { result = new(false, "picker_failed", NativeErrorCode: error.HResult); }
        finally { Interlocked.Exchange(ref _pickerBusy, 0); }
        Log("terminal", id, command, started, result, result.Success ? "completed" : result.Error is "invalid_source" or "picker_retired" or "picker_busy" ? result.Error : "native_failure");
        return result;
    }
    private int _pickerBusy;

    internal static WindowsCommandResult RunImagePicker(Func<CancellationToken, WindowsCommandResult> dialog,
        CancellationToken cancellation)
    {
        // Waiting for a person is not a background-operation timeout. The single bounded
        // slot stays owned until the actual native modal call exits, including cancellation.
        var ok = BoundedStaOperationRunner.TryRun("NxTools floating image picker", Timeout.InfiniteTimeSpan,
            dialog, out WindowsCommandResult? picked, out _, cancellation);
        return cancellation.IsCancellationRequested ? new(false, "picker_retired")
            : ok && picked is not null ? picked : new(false, "picker_busy");
    }

    internal static WindowsCommandResult RunImagePickerWithOwner(
        long sourceWindowHandle,
        uint sourceProcessId,
        Func<IWin32Window, WindowsCommandResult> present,
        CancellationToken cancellation)
    {
        return PickerOwner.Run(sourceWindowHandle, sourceProcessId, cancellation, present);
    }

    private void ReceiveInput(FloatingAssistantInputEvent input)
    {
        var command = _lease;
        if (command is null || !_visible) return;
        Publish(command, Guid.NewGuid(), input.Kind switch
        {
            FloatingAssistantInputKind.LeftClick => "left_click", FloatingAssistantInputKind.RightClick => "right_click",
            FloatingAssistantInputKind.MoveCompleted => "move_completed", FloatingAssistantInputKind.HoverEnter => "hover_enter",
            FloatingAssistantInputKind.HoverLeave => "hover_leave", _ => "wheel"
        }, input.X, input.Y, input.Delta);
    }
    private bool ReceiveDrop(FloatingAssistantDropData drop)
    {
        var command = _lease;
        if (command is null || !_visible || command.ClickThrough || _drops.Count >= 1 || drop.CorrelationId == Guid.Empty)
            return false;
        var id = drop.CorrelationId;
        var owned = new DropLease(command, id);
        if (Interlocked.CompareExchange(ref _dropSlot, 1, 0) != 0) return false;
        _drops.Add(id, owned);
        var published = false;
        string? image = null;
        try
        {
            if (drop.ImageBytes is { } bytes)
            {
                if (bytes.Length is < 1 or > 16 * 1024 * 1024 || drop.ImageFormat is not ("png" or "dib" or "dib_v5")) return false;
                owned.Directory = Path.Combine(Path.GetTempPath(), "NxTools-floating-" + Guid.NewGuid().ToString("N"));
                // Install ownership before the first fallible filesystem mutation.
                Directory.CreateDirectory(owned.Directory);
                image = Path.Combine(owned.Directory, "image." + drop.ImageFormat);
                using var stream = new FileStream(image, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
                stream.Write(bytes);
            }
            if (!Publish(command, id, "drop", drop: drop with { ImageBytes = null }, imagePath: image, dropId: id)) return false;
            published = true;
            return true;
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException) { return false; }
        finally
        {
            if (!published && _drops.ContainsKey(id))
            {
                owned.Retired = true;
                DeleteOwned(id, owned);
            }
        }
    }

    private bool Publish(FloatingAssistantCommand command, Guid id, string kind, int x = 0, int y = 0,
        int delta = 0, bool fullscreen = false, FloatingAssistantDropData? drop = null,
        string? imagePath = null, Guid? dropId = null)
    {
        // Only the fixed STA calls Publish; rejected delivery retires its exact staging owner.
        if (_stopping) return false;
        try
        {
            CursorMonitorSnapshot? cursorMonitor = null;
            if (kind == "drop" && Native.GetCursorPos(out var cursor))
            {
                x = cursor.X;
                y = cursor.Y;
                var monitor = Native.MonitorFromPoint(cursor, 2);
                var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
                if (monitor != IntPtr.Zero && Native.GetMonitorInfoW(monitor, ref info))
                {
                    var dpi = Native.GetDpiForMonitor(monitor, 0, out var dpiX, out _) == 0
                        ? dpiX : Native.GetDpiForWindow(new IntPtr(command.WindowHandle));
                    cursorMonitor = new CursorMonitorSnapshot(unchecked((ulong)monitor.ToInt64()),
                        info.Work.Left, info.Work.Top, (uint)(info.Work.Right - info.Work.Left),
                        (uint)(info.Work.Bottom - info.Work.Top), Math.Max(96u, dpi));
                }
            }
            return _publish(FloatingAssistantContract.Event(command, id, ++_sequence, kind, x, y,
                delta, fullscreen, drop, imagePath, dropId, cursorMonitor));
        }
        catch { return false; }
    }

    private void Observe()
    {
        var command = _lease;
        if (command is null) return;
        if (!Identity(new IntPtr(command.WindowHandle), _peer) || !Identity(new IntPtr(command.SourceWindowHandle), _peer))
        {
            Cleanup();
            Publish(command, Guid.NewGuid(), "retired");
            lock (_gate) { if (Matches(command)) _lease = null; }
            return;
        }
        RepairTopmost(command);
        var foreground = Native.GetAncestor(Native.GetForegroundWindow(), 2);
        var foregroundRect = Native.GetWindowRect(foreground, out var area);
        var monitor = Native.MonitorFromWindow(new IntPtr(command.WindowHandle), 2);
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        // An unavailable monitor snapshot must not be mistaken for "not fullscreen".
        // Keep the last delivered signature so the next 500ms sample retries.
        if (monitor == IntPtr.Zero || !Native.GetMonitorInfoW(monitor, ref info)) return;
        var dpi = Native.GetDpiForMonitor(monitor, 0, out var effectiveDpi, out _) == 0
            ? effectiveDpi : Native.GetDpiForWindow(new IntPtr(command.WindowHandle));
        if (dpi is < 48 or > 960 || info.Work.Right <= info.Work.Left || info.Work.Bottom <= info.Work.Top) return;
        var validForeground = foreground != IntPtr.Zero && Native.IsWindow(foreground) &&
            foreground != new IntPtr(command.WindowHandle) && foreground != _input?.Handle &&
            foreground != new IntPtr(command.SourceWindowHandle) && foregroundRect &&
            area.Right > area.Left && area.Bottom > area.Top;
        var fullscreen = FloatingAssistantFullscreenPolicy.Evaluate(validForeground,
            Native.IsWindowVisible(foreground), Native.IsIconic(foreground),
            validForeground && _desktopWindows.IsDesktopWindow(foreground.ToInt64()),
            Native.GetWindowLongPtrW(foreground, -16).ToInt64(),
            area.Left <= info.Monitor.Left && area.Top <= info.Monitor.Top &&
            area.Right >= info.Monitor.Right && area.Bottom >= info.Monitor.Bottom, out var reason);
        var signature = new FloatingAssistantEnvironmentSignature(monitor.ToInt64(),
            info.Monitor.Left, info.Monitor.Top, info.Monitor.Right, info.Monitor.Bottom,
            info.Work.Left, info.Work.Top, info.Work.Right, info.Work.Bottom, dpi, fullscreen);
        var correlation = Guid.NewGuid();
        var started = Stopwatch.GetTimestamp();
        if (_environment.TryPublish(signature, () => Publish(command, correlation, "environment", fullscreen: fullscreen)) &&
            _lastFullscreen != fullscreen)
        {
            _lastFullscreen = fullscreen;
            Log("environment_changed", correlation, command, started, new(true), reason);
        }
    }

    private void RepairTopmost(FloatingAssistantCommand command)
    {
        var renderer = new IntPtr(command.WindowHandle);
        var input = _input;
        // Never show/reposition anything here. Hidden, retired and disconnected
        // sessions must not acquire a new native presentation intent.
        if (!_visible || _stopping || input is null || !Matches(command) ||
            !Native.IsWindowVisible(renderer) || !Identity(renderer, _peer) ||
            !Identity(new IntPtr(command.SourceWindowHandle), _peer) ||
            !Identity(input.Handle, (uint)Environment.ProcessId)) return;
        var rendererLost = (Native.GetWindowLongPtrW(renderer, -20).ToInt64() & 8) == 0;
        var inputLost = !command.ClickThrough && input.TopmostLost;
        if (!rendererLost && !inputLost) return;
        var started = Stopwatch.GetTimestamp();
        WindowsCommandResult result;
        try
        {
            // Owner first, then its visible receiving popup above it. NOACTIVATE,
            // NOMOVE, NOSIZE only: no SHOWWINDOW, opacity or interaction reset.
            if (!Native.SetWindowPos(renderer, new IntPtr(-1), 0, 0, 0, 0, 0x0013)) throw LastError();
            if (!command.ClickThrough) input.RepairTopmost();
            if ((Native.GetWindowLongPtrW(renderer, -20).ToInt64() & 8) == 0 ||
                (!command.ClickThrough && input.TopmostLost))
                throw new InvalidDataException("topmost_repair_mismatch");
            result = new(true);
        }
        catch (Exception error) when (error is Win32Exception or InvalidDataException or InvalidOperationException)
        {
            result = new(false, "topmost_repair_failed",
                NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
        var now = Environment.TickCount64;
        if (result.Success || !_topmostRepairFailed || now - _lastTopmostFailure >= 30_000)
        {
            Log("topmost_repair", _leaseCorrelation, command, started, result,
                rendererLost ? (inputLost ? "renderer_and_input" : "renderer") : "input");
            if (!result.Success) _lastTopmostFailure = now;
        }
        _topmostRepairFailed = !result.Success;
    }

    private void SafeHide()
    {
        _visible = false;
        try { _input?.SetInteraction(false, true); } catch (Exception) { /* continue renderer hide */ }
        var command = _lease;
        if (command is not null && Identity(new IntPtr(command.WindowHandle), _peer))
            _ = Native.ShowWindow(new IntPtr(command.WindowHandle), 0);
    }

    private void Cleanup()
    {
        SafeHide();
        try { _input?.Dispose(); }
        finally
        {
            _input = null;
            foreach (var drop in _drops.Values) drop.Retired = true;
            RetryDropCleanup();
            _environment.Reset();
            _lastFullscreen = null;
            _leaseCorrelation = Guid.Empty;
            _topmostRepairFailed = false;
            _lastTopmostFailure = 0;
        }
    }

    private void RetryDropCleanup()
    {
        foreach (var (id, drop) in _drops.ToArray())
            if (drop.Retired) DeleteOwned(id, drop);
    }

    private void DeleteOwned(Guid id, DropLease drop)
    {
        try
        {
            if (drop.Directory is not null) Directory.Delete(drop.Directory, recursive: true);
        }
        catch (DirectoryNotFoundException) { /* already physically absent */ }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException)
        {
            if (!drop.PendingLogged)
            {
                Log("drop_cleanup_pending", drop.Correlation, drop.Command, drop.Started,
                    new(false, "delete_failed", NativeErrorCode: error.HResult), "delete_failed");
                drop.PendingLogged = true;
            }
            return;
        }
        if (_drops.Remove(id)) Interlocked.Exchange(ref _dropSlot, 0);
        Log("drop_cleanup_complete", drop.Correlation, drop.Command, drop.Started, new(true));
    }

    internal void Stop()
    {
        _stopping = true;
        _pickerStop.Cancel();
        // The fixed STA observes stop on its next turn. Never wait for a foreign HWND
        // or OLE callback on the pipe reader; its slot retains ownership until cleanup.
    }
    public void Dispose() { Stop(); if (Environment.CurrentManagedThreadId != _ownerThread) _thread.Join(TimeSpan.FromSeconds(2)); }

    private void Log(string phase, Guid id, FloatingAssistantCommand command, long started,
        WindowsCommandResult? result = null, string? reason = null)
    {
        try { _log.Write(result?.Success == false ? "warning" : "info", "floating_assistant.window." + phase, id,
            new Dictionary<string, object?> { ["operation"] = command.Operation, ["session_id"] = command.SessionId,
                ["generation"] = command.Generation, ["success"] = result?.Success,
                ["reason"] = reason,
                ["native_error_code"] = result?.NativeErrorCode,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds }); }
        catch { /* diagnostics cannot change command results */ }
    }

    private static byte Alpha(byte opacity) => (byte)Math.Round(opacity * 255.0 / 100);
    private static Win32Exception LastError() => new(Marshal.GetLastWin32Error());
    private bool Identity(IntPtr window, uint peer) => window != IntPtr.Zero && Native.IsWindow(window) &&
        Native.GetWindowThreadProcessId(window, out var actual) != 0 && actual == peer &&
        Native.ProcessIdToSessionId(peer, out var session) && Native.ProcessIdToSessionId((uint)Environment.ProcessId, out var own) && session == own;
    private static void SetStyle(IntPtr window, int index, int add, int remove)
    {
        var style = Native.GetWindowLongPtrW(window, index).ToInt64();
        Marshal.SetLastPInvokeError(0);
        var previous = Native.SetWindowLongPtrW(window, index, new IntPtr((style | (uint)add) & ~(uint)remove));
        if (previous == IntPtr.Zero && Marshal.GetLastPInvokeError() != 0) throw LastError();
    }
    private static bool Policy(IntPtr window)
    {
        var style = Native.GetWindowLongPtrW(window, -20).ToInt64();
        return (style & (8 | 0x80000 | 0x20 | 0x8000000 | 0x80)) == (8 | 0x80000 | 0x20 | 0x8000000 | 0x80) &&
            (style & 0x40000) == 0;
    }
    private static Native.Rect ReadClient(IntPtr window)
    {
        if (!Native.GetClientRect(window, out var client)) throw LastError();
        var point = new Native.Point(0, 0);
        if (!Native.ClientToScreen(window, ref point)) throw LastError();
        return new(point.X, point.Y, checked(point.X + client.Right), checked(point.Y + client.Bottom));
    }
    private static WindowsCommandResult BoundsResult(IntPtr target, uint dpi, Native.MonitorInfo info)
    {
        var rect = ReadClient(target);
        return new(true, Data: JsonSerializer.Serialize(new {
            x = rect.Left, y = rect.Top, width = rect.Right - rect.Left, height = rect.Bottom - rect.Top,
            dpi, work_left = info.Work.Left, work_top = info.Work.Top, work_right = info.Work.Right,
            work_bottom = info.Work.Bottom, monitor_left = info.Monitor.Left, monitor_top = info.Monitor.Top,
            monitor_right = info.Monitor.Right, monitor_bottom = info.Monitor.Bottom }));
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal record struct Point(int X, int Y);
        [StructLayout(LayoutKind.Sequential)] internal record struct Rect(int Left, int Top, int Right, int Bottom);
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] internal struct MonitorInfo
        { internal int Size; internal Rect Monitor; internal Rect Work; internal uint Flags; }
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr MonitorFromPoint(Point point, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetCursorPos(out Point point);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
        [DllImport("shcore.dll", ExactSpelling = true)] internal static extern int GetDpiForMonitor(IntPtr monitor, int type, out uint x, out uint y);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern uint GetDpiForWindow(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsWindow(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool IsIconic(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)] internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("kernel32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ProcessIdToSessionId(uint process, out uint session);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetWindowRect(IntPtr window, out Rect rect);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GetClientRect(IntPtr window, out Rect rect);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ClientToScreen(IntPtr window, ref Point point);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ShowWindow(IntPtr window, int show);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool SetLayeredWindowAttributes(IntPtr window, uint key, byte alpha, uint flags);
        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)] internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
        [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)] internal static extern IntPtr SetWindowLongPtrW(IntPtr window, int index, IntPtr value);
    }
}
