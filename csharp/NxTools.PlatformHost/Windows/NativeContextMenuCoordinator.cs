using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal delegate WindowsCommandResult NativeContextMenuOperation(
    ShowContextMenuCommand command, NativeContextMenuSession session, Action ready);

internal sealed class NativeContextMenuSession(Guid id, IntPtr sourceRoot)
{
    private readonly object _gate = new();
    private Action? _cancel;
    private bool _cancelled;
    private bool _outside;
    internal Guid Id { get; } = id;
    internal IntPtr SourceRoot { get; } = sourceRoot;
    internal volatile IntPtr Host;
    internal volatile uint WorkerThreadId;
    internal volatile bool Tracking;
    internal bool Cancelled { get { lock (_gate) return _cancelled; } }
    internal bool Outside { get { lock (_gate) return _outside; } }
    internal void MarkOutside() { lock (_gate) _outside = true; }
    internal void SetCancelAction(Action? action)
    {
        lock (_gate)
        {
            _cancel = action;
            if (_cancelled) _cancel?.Invoke();
        }
    }
    internal void Cancel()
    {
        // Only posts to the owner; never EndMenu on an IPC/EventWindow thread.
        lock (_gate) { _cancelled = true; _cancel?.Invoke(); }
    }
}

// One global modal menu. Admission never queues; the slot survives cancellation until native cleanup.
internal sealed class NativeContextMenuCoordinator : IDisposable
{
    private static readonly object ActiveGate = new();
    private static NativeContextMenuSession? Active;
    private readonly IPlatformLog _log;
    private readonly NativeContextMenuOperation _operation;
    private readonly Func<ShowContextMenuCommand, bool> _validateSource;
    private readonly Action? _flushPointerObservation;
    private bool _disposed;

    internal NativeContextMenuCoordinator(IPlatformLog? log = null, NativeContextMenuOperation? operation = null,
        Func<ShowContextMenuCommand, bool>? validateSource = null, Action? flushPointerObservation = null)
    {
        _log = log ?? NullPlatformLog.Instance;
        _operation = operation ?? ExecuteNative;
        _validateSource = validateSource ?? ValidateSource;
        _flushPointerObservation = flushPointerObservation;
    }

    internal static void Validate(ShowContextMenuCommand command)
    {
        if (command.PopupWindowHandle != 0)
        {
            if (command.SourceWindowHandle == 0 || command.SourceProcessId == 0 ||
                command.PopupWindowHandle == command.SourceWindowHandle || command.Items is null || command.Items.Count != 0)
                throw new InvalidDataException("Invalid styled context menu surface.");
            return;
        }
        if (command.SourceWindowHandle == 0 || command.SourceProcessId == 0 || command.Items is null ||
            command.Items.Count is < 1 or > 128) throw new InvalidDataException("Invalid native context menu payload.");
        var count = 0;
        var characters = 0;
        var ids = new HashSet<uint>();
        void Visit(IReadOnlyList<NativeContextMenuItem> items, int depth)
        {
            if (depth > 4) throw new InvalidDataException("Invalid native context menu payload.");
            foreach (var item in items)
            {
                if (item is null || ++count > 256 || item.Label is null || item.Label.Length > 256 ||
                    item.Label.IndexOf('\0') >= 0 || item.Children is null || (characters += item.Label.Length) > 32768)
                    throw new InvalidDataException("Invalid native context menu payload.");
                if (item.Separator)
                {
                    if (item.Id != 0 || item.Children.Count != 0 || item.Checked)
                        throw new InvalidDataException("Invalid native context menu payload.");
                }
                else if (item.Children.Count != 0)
                {
                    if (item.Id != 0) throw new InvalidDataException("Invalid native context menu payload.");
                    Visit(item.Children, depth + 1);
                }
                else if (item.Id == 0 || !ids.Add(item.Id))
                    throw new InvalidDataException("Invalid native context menu payload.");
            }
        }
        Visit(command.Items, 1);
    }

    internal Task<WindowsCommandResult> ShowAsync(ShowContextMenuCommand command, Guid sessionId, CancellationToken token)
    {
        var started = Stopwatch.GetTimestamp();
        try { Validate(command); }
        catch (InvalidDataException) { return Reject("Invalid native context menu payload."); }
        if (sessionId == Guid.Empty) return Reject("Invalid native context menu session.");
        if (token.IsCancellationRequested)
        {
            var cancelled = CancelledResult();
            Write("terminal", sessionId, started, command, cancelled);
            return Task.FromResult(cancelled);
        }
        if (!_validateSource(command)) return Reject("Invalid native context menu source.");
        var session = new NativeContextMenuSession(sessionId, Native.Root(new(command.SourceWindowHandle)));
        lock (ActiveGate)
        {
            if (_disposed) return Reject("Native context menu coordinator is unavailable.");
            if (Active is not null) return Reject("A native context menu is already active.");
            Active = session;
        }
        Write("accepted", sessionId, started, command);
        var completion = new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
        var worker = new Thread(() =>
        {
            WindowsCommandResult result;
            try
            {
                using var lifetime = CancellationTokenSource.CreateLinkedTokenSource(token);
                using var registration = lifetime.Token.Register(session.Cancel);
                lifetime.CancelAfter(TimeSpan.FromSeconds(120));
                Write("start", sessionId, started, command);
                result = session.Cancelled ? CancelledResult() : _operation(command, session,
                    () => Write("ready", sessionId, started, command));
            }
            catch (Exception error)
            {
                result = new(false, "Native context menu failed.", NativeErrorCode:
                    error is Win32Exception native ? native.NativeErrorCode : null);
                Write("exception", sessionId, started, command, result, error.GetType().FullName);
            }
            finally
            {
                // ExecuteNative's finally destroyed HMENU and the worker HWND before this retirement.
                lock (ActiveGate) { if (ReferenceEquals(Active, session)) Active = null; }
            }
            Write("terminal", sessionId, started, command, result);
            completion.TrySetResult(result);
        }) { IsBackground = true, Name = "NxTools.NativeContextMenu" };
        try { worker.SetApartmentState(ApartmentState.STA); worker.Start(); }
        catch (Exception error)
        {
            lock (ActiveGate) { if (ReferenceEquals(Active, session)) Active = null; }
            var result = new WindowsCommandResult(false, "Native context menu worker failed.");
            Write("terminal", sessionId, started, command, result, error.GetType().FullName);
            completion.TrySetResult(result);
        }
        return completion.Task;

        Task<WindowsCommandResult> Reject(string error)
        {
            var result = new WindowsCommandResult(false, error);
            Write("terminal", sessionId, started, command, result);
            return Task.FromResult(result);
        }
    }

    internal WindowsCommandResult Cancel(Guid sessionId)
    {
        lock (ActiveGate) { if (Active?.Id == sessionId) Active.Cancel(); }
        return new(true); // Idempotent acknowledgement, never cancels a different session.
    }

    internal void ObservePointerPress(long windowHandle, long rootWindowHandle)
    {
        lock (ActiveGate)
        {
            var session = Active;
            if (session is null || !session.Tracking || session.Cancelled) return;
            var window = new IntPtr(windowHandle);
            var root = new IntPtr(rootWindowHandle);
            if (root == session.SourceRoot || window == session.Host || root == session.Host ||
                Native.IsWorkerMenu(window, session.WorkerThreadId) || Native.IsWorkerMenu(root, session.WorkerThreadId)) return;
            session.MarkOutside();
        }
    }

    public void Dispose()
    {
        lock (ActiveGate) { _disposed = true; Active?.Cancel(); }
    }

    private static bool ValidateSource(ShowContextMenuCommand command)
    {
        var window = new IntPtr(command.SourceWindowHandle);
        var root = Native.Root(window);
        _ = Native.GetWindowThreadProcessId(root, out var process);
        return Native.IsWindow(window) && Native.IsWindowVisible(window) && Native.IsWindowVisible(root) &&
            process == command.SourceProcessId &&
            Native.ProcessIdToSessionId(process, out var sourceSession) && sourceSession != 0 &&
            Native.ProcessIdToSessionId((uint)Environment.ProcessId, out var hostSession) && sourceSession == hostSession;
    }

    private static WindowsCommandResult CancelledResult() => Result(null, false);
    private static WindowsCommandResult Result(uint? selected, bool outside) => new(true, Data:
        JsonSerializer.Serialize(new { selected_id = selected, dismissed_outside = outside }));

    private void Write(string stage, Guid id, long started, ShowContextMenuCommand command,
        WindowsCommandResult? result = null, string? exceptionType = null)
    {
        try
        {
            _log.Write(result?.Success == false ? "warning" : "info", "context_menu." + stage, id,
                new Dictionary<string, object?> { ["root_item_count"] = command.Items?.Count ?? 0,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    ["success"] = result?.Success, ["native_error_code"] = result?.NativeErrorCode,
                    ["exception_type"] = exceptionType });
        }
        catch { /* Diagnostics never affect the menu. */ }
    }

    // GPUI owns menu paint/state. This path only prepares its independent, still-hidden HWND.
    private static WindowsCommandResult PrepareStyledSurface(ShowContextMenuCommand command,
        NativeContextMenuSession session, Action ready)
    {
        using var dpi = new DpiScope();
        var popup = new IntPtr(command.PopupWindowHandle);
        if (!ValidateSource(command) || !Native.IsWindow(popup)) return new(false, "Invalid styled menu HWND.");
        _ = Native.GetWindowThreadProcessId(popup, out var process);
        if (process != command.SourceProcessId || Native.IsWindowVisible(popup))
            return new(false, "Styled menu HWND must be hidden and source-owned.");
        if (session.Cancelled) return new(false, "Styled menu preparation cancelled.");
        var point = new Native.Point { X = command.X, Y = command.Y };
        if (!Native.ClientToScreen(new(command.SourceWindowHandle), ref point))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        var monitor = Native.MonitorFromPoint(point, 2);
        var info = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
        if (!Native.GetMonitorInfoW(monitor, ref info)) throw new Win32Exception(Marshal.GetLastWin32Error());
        var width = info.Work.Right - info.Work.Left;
        var height = info.Work.Bottom - info.Work.Top;
        if (width <= 0 || height <= 0 || width > 32768 || height > 32768) return new(false, "Invalid menu work area.");
        // WS_POPUP, no caption/thickframe: exact client viewport, not a clipped WS_CHILD.
        void SetStyle(int index, IntPtr value)
        {
            Marshal.SetLastPInvokeError(0);
            _ = Native.SetWindowLongPtrW(popup, index, value);
            var nativeError = Marshal.GetLastWin32Error();
            if (nativeError != 0) throw new Win32Exception(nativeError);
        }
        SetStyle(-8, session.SourceRoot);
        SetStyle(-16, new IntPtr(unchecked((int)0x80000000)));
        var extended = Native.GetWindowLongPtrW(popup, -20).ToInt64();
        SetStyle(-20, new IntPtr((extended | 0x80) & ~0x08040000L));
        if (!Native.SetWindowPos(popup, new IntPtr(-1), info.Work.Left, info.Work.Top, width, height, 0x0030))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        var clientOrigin = new Native.Point();
        if (!Native.GetClientRect(popup, out var client) || client.Right != width || client.Bottom != height ||
            !Native.ClientToScreen(popup, ref clientOrigin) || clientOrigin.X != info.Work.Left || clientOrigin.Y != info.Work.Top)
            return new(false, "Styled menu client readback mismatch.");
        ready();
        return new(true, Data: JsonSerializer.Serialize(new {
            width, height, anchor_x = Math.Clamp(point.X - info.Work.Left, 0, width - 1),
            anchor_y = Math.Clamp(point.Y - info.Work.Top, 0, height - 1) }));
    }

    private WindowsCommandResult ExecuteNative(ShowContextMenuCommand command, NativeContextMenuSession session, Action ready)
    {
        if (command.PopupWindowHandle != 0) return PrepareStyledSurface(command, session, ready);
        // TPM's owner must belong to the calling thread. GPUI's foreign-process HWND cannot be
        // used as the tracking owner. A WS_POPUP tool HWND on this STA is owned by the source
        // top-level HWND (not WS_CHILD); native menu windows can then escape all source bounds.
        var source = new IntPtr(command.SourceWindowHandle);
        using var dpi = new DpiScope();
        if (!ValidateSource(command)) return new(false, "Invalid native context menu source.");
        var point = new Native.Point { X = command.X, Y = command.Y };
        if (!Native.ClientToScreen(source, ref point)) throw new Win32Exception(Marshal.GetLastWin32Error());
        var menu = IntPtr.Zero;
        MenuOwner? owner = null;
        try
        {
            menu = BuildMenu(command.Items);
            if (session.Cancelled) return Result(null, session.Outside);
            // Never take foreground away from a genuinely external application.
            if (Native.Root(Native.GetForegroundWindow()) != session.SourceRoot) return CancelledResult();
            owner = new MenuOwner(session, point);
            session.SetCancelAction(owner.PostCancel);
            if (session.Cancelled) return Result(null, session.Outside);
            _ = Native.ShowWindow(owner.Handle, 4); // SW_SHOWNOACTIVATE
            var foregroundThread = Native.GetWindowThreadProcessId(Native.GetForegroundWindow(), out _);
            var workerThread = Native.GetCurrentThreadId();
            var attached = foregroundThread != workerThread && Native.AttachThreadInput(workerThread, foregroundThread, true);
            try
            {
                if (!Native.SetForegroundWindow(owner.Handle))
                    return new(false, "Native context menu activation failed.", NativeErrorCode: Marshal.GetLastWin32Error());
            }
            finally { if (attached) _ = Native.AttachThreadInput(workerThread, foregroundThread, false); }
            if (session.Cancelled) return Result(null, session.Outside);
            Marshal.SetLastPInvokeError(0);
            // TPM_NONOTIFY does not reliably deliver WM_ENTERMENULOOP to this surrogate.
            // Own tracking at the real modal call boundary so exact cancel always reaches EndMenu.
            session.Tracking = true;
            ready();
            var selected = Native.TrackPopupMenuEx(menu, 0x0100 | 0x0080 | 0x0002, point.X, point.Y, owner.Handle, IntPtr.Zero);
            var nativeError = Marshal.GetLastWin32Error();
            // Flush the existing EventWindow sampler before terminal/outside snapshot: the menu
            // loop can return between its regular 40ms ticks. No second hook or pointer reader.
            if (selected == 0 && !session.Cancelled) _flushPointerObservation?.Invoke();
            session.Tracking = false;
            if (selected == 0 && nativeError != 0 && !session.Cancelled)
                return new(false, "Native context menu tracking failed.", NativeErrorCode: nativeError);
            // Disabled leaves are mapped but cannot dispatch. Native cancellation drops any late command.
            uint? choice = selected == 0 || session.Cancelled ? null : selected;
            return Result(choice, session.Outside);
        }
        finally
        {
            session.Tracking = false;
            session.SetCancelAction(null);
            if (owner is not null)
            {
                var foreground = Native.GetForegroundWindow();
                var mayRestore = !session.Outside && Native.IsWindowVisible(source) &&
                    (foreground == owner.Handle || Native.Root(foreground) == session.SourceRoot ||
                     Native.IsWorkerMenu(foreground, session.WorkerThreadId));
                // Restore while the foreground host still exists, never after an external click.
                if (mayRestore) _ = Native.SetForegroundWindow(session.SourceRoot);
                try { owner.Close(); }
                finally { if (menu != IntPtr.Zero) { _ = Native.DestroyMenu(menu); menu = IntPtr.Zero; } }
                session.Host = IntPtr.Zero;
            }
            if (menu != IntPtr.Zero) _ = Native.DestroyMenu(menu); // Recursively owns appended children.
        }
    }

    private static IntPtr BuildMenu(IReadOnlyList<NativeContextMenuItem> items)
    {
        var menu = Native.CreatePopupMenu();
        if (menu == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        try
        {
            foreach (var item in items)
            {
                var flags = (item.Enabled ? 0u : 0x0003u) | (item.Checked ? 0x0008u : 0u);
                if (item.Separator) flags = 0x0800;
                var child = item.Children.Count == 0 ? IntPtr.Zero : BuildMenu(item.Children);
                if (child != IntPtr.Zero) flags |= 0x0010;
                // Escape '&' so labels remain literal rather than defining arbitrary mnemonics.
                if (!Native.AppendMenuW(menu, flags, child == IntPtr.Zero ? new UIntPtr(item.Id) : (UIntPtr)child,
                    item.Separator ? null : item.Label.Replace("&", "&&", StringComparison.Ordinal)))
                {
                    var code = Marshal.GetLastWin32Error();
                    if (child != IntPtr.Zero) _ = Native.DestroyMenu(child);
                    throw new Win32Exception(code);
                }
            }
            return menu;
        }
        catch { _ = Native.DestroyMenu(menu); throw; }
    }

    private sealed class DpiScope : IDisposable
    {
        private readonly IntPtr _previous;
        internal DpiScope()
        {
            _previous = Native.SetThreadDpiAwarenessContext(new IntPtr(-4)); // PMv2; wire is device pixels.
            if (_previous == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        public void Dispose() { if (_previous != IntPtr.Zero) _ = Native.SetThreadDpiAwarenessContext(_previous); }
    }

    private sealed class MenuOwner : NativeWindow
    {
        private const int CancelMessage = 0x8000 + 0x61;
        private readonly NativeContextMenuSession _session;
        internal MenuOwner(NativeContextMenuSession session, Native.Point point)
        {
            _session = session;
            CreateHandle(new CreateParams { Caption = "NxTools.NativeContextMenu", Parent = session.SourceRoot,
                Style = unchecked((int)0x80000000), ExStyle = 0x00000080,
                X = point.X, Y = point.Y, Width = 1, Height = 1 });
            session.Host = Handle;
            session.WorkerThreadId = Native.GetCurrentThreadId();
        }
        internal void PostCancel() => _ = Native.PostMessageW(Handle, CancelMessage, IntPtr.Zero, IntPtr.Zero);
        internal void Close() => DestroyHandle();
        protected override void WndProc(ref Message message)
        {
            if (message.Msg == CancelMessage)
            {
                if (_session.Tracking) _ = Native.EndMenu();
                return;
            }
            base.WndProc(ref message);
        }
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal struct Point { internal int X, Y; }
        internal static IntPtr Root(IntPtr window) => GetAncestor(window, 2);
        internal static bool IsWorkerMenu(IntPtr window, uint thread)
        {
            if (window == IntPtr.Zero || thread == 0 || GetWindowThreadProcessId(window, out _) != thread) return false;
            var name = new StringBuilder(64);
            _ = GetClassNameW(window, name, name.Capacity);
            return name.ToString() == "#32768";
        }
        [DllImport("user32.dll")] internal static extern bool IsWindow(IntPtr window);
        [DllImport("user32.dll")] internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll")] private static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll")] internal static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("kernel32.dll")] internal static extern bool ProcessIdToSessionId(uint process, out uint session);
        [DllImport("kernel32.dll")] internal static extern uint GetCurrentThreadId();
        // GetLastPInvokeError is explicitly cleared before tracking; labels never enter failures.
        [DllImport("user32.dll", SetLastError = true)] internal static extern bool ClientToScreen(IntPtr window, ref Point point);
        [DllImport("user32.dll", SetLastError = true)] internal static extern IntPtr CreatePopupMenu();
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)] internal static extern bool AppendMenuW(IntPtr menu, uint flags, UIntPtr id, string? text);
        [DllImport("user32.dll")] internal static extern bool DestroyMenu(IntPtr menu);
        [DllImport("user32.dll", SetLastError = true)] internal static extern uint TrackPopupMenuEx(IntPtr menu, uint flags, int x, int y, IntPtr owner, IntPtr parameters);
        [DllImport("user32.dll")] internal static extern bool EndMenu();
        [DllImport("user32.dll")] internal static extern bool PostMessageW(IntPtr window, int message, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll")] internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", SetLastError = true)] internal static extern bool SetForegroundWindow(IntPtr window);
        [DllImport("user32.dll")] internal static extern bool ShowWindow(IntPtr window, int command);
        [DllImport("user32.dll")] internal static extern bool AttachThreadInput(uint from, uint to, bool attach);
        [DllImport("user32.dll", SetLastError = true)] internal static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassNameW(IntPtr window, StringBuilder name, int count);
        [StructLayout(LayoutKind.Sequential)] internal struct Rect { internal int Left, Top, Right, Bottom; }
        [StructLayout(LayoutKind.Sequential)] internal struct MonitorInfo { internal int Size; internal Rect Monitor, Work; internal uint Flags; }
        [DllImport("user32.dll")] internal static extern IntPtr MonitorFromPoint(Point point, uint flags);
        [DllImport("user32.dll", SetLastError = true)] internal static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
        [DllImport("user32.dll", SetLastError = true)] internal static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
        [DllImport("user32.dll")] internal static extern bool GetClientRect(IntPtr window, out Rect rectangle);
        [DllImport("user32.dll", SetLastError = true)] internal static extern IntPtr SetWindowLongPtrW(IntPtr window, int index, IntPtr value);
        [DllImport("user32.dll")] internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
    }
}
