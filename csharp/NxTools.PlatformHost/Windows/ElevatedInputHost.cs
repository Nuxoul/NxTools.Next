using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal interface IElevatedInputNative
{
    uint WindowProcessId(IntPtr window);
    IntPtr RootWindow(IntPtr window);
    IntPtr ForegroundWindow();
    bool IsSameSession(uint processId);
    bool IsElevated(uint processId);
    bool AllowForeground(uint processId);
    bool Post(IntPtr window, int message, IntPtr wParam, IntPtr lParam);
    bool IsPointerButtonDown(int virtualKey);
    (long WindowHandle, long RootWindowHandle, uint ProcessId) PointerTarget();
    void StartPointerMonitor(IntPtr window);
    void StopPointerMonitor(IntPtr window);
    bool ActivateWindow(IntPtr window);
}

internal sealed partial class ElevatedInputHost : IDisposable
{
    private const int WmGesture = 0x8014;
    private const int WmHookFailure = 0x8015;
    private const int WmStop = 0x8016;
    private static readonly TimeSpan ThreadWait = TimeSpan.FromSeconds(2);
    private readonly uint _authenticatedPipeServerPid;
    private readonly IPlatformLog _log;
    private readonly IElevatedInputNative _native;
    private readonly IKeyboardHookPlatform? _keyboardPlatform;
    private readonly IWindowsPasteOperations? _pasteOperations;
    private readonly object _lifecycle = new();
    private readonly QuickLaunchKeyboardGesture _gesture = new();
    private readonly TaskCompletionSource<IntPtr> _ready = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly Thread _thread;
    private LowLevelKeyboardMonitor? _monitor;
    private Configuration? _configuration;
    private IntPtr _windowHandle;
    private long _nextEpoch;
    private int _disposed;
    private long _lastUnavailableEpoch;

    private sealed record Configuration(ElevatedInputConfiguration Value, long Epoch);

    public ElevatedInputHost(uint authenticatedPipeServerPid, IPlatformLog log)
        : this(authenticatedPipeServerPid, log, new WindowsElevatedInputNative()) { }

    internal ElevatedInputHost(uint authenticatedPipeServerPid, IPlatformLog log, IElevatedInputNative native,
        IKeyboardHookPlatform? keyboardPlatform = null, IWindowsPasteOperations? pasteOperations = null)
    {
        _authenticatedPipeServerPid = authenticatedPipeServerPid;
        _log = log;
        _native = native;
        _keyboardPlatform = keyboardPlatform;
        _pasteOperations = pasteOperations;
        _thread = new Thread(RunWindow) { Name = "NxTools.ElevatedInputWindow", IsBackground = true };
        _thread.SetApartmentState(ApartmentState.STA);
        _thread.Start();
    }

    internal bool IsHookInstalled => Volatile.Read(ref _disposed) == 0 &&
        Volatile.Read(ref _monitor)?.IsInstalled == true;

    public WindowsCommandResult Configure(ElevatedInputConfiguration configuration)
    {
        var started = Stopwatch.GetTimestamp();
        Write("info", "elevated_input.configure.accepted", configuration.CorrelationId, "accepted", started);
        lock (_lifecycle)
        {
            if (Volatile.Read(ref _disposed) != 0)
                return Finish("configure", configuration.CorrelationId, started, new(false, "Elevated input has stopped."), "stopped");

            // Retire the old epoch even if the new destination or native installation fails.
            var epoch = ++_nextEpoch;
            var current = new Configuration(configuration, epoch);
            _gesture.Publish(configuration.Keyboard with { ResetGeneration = epoch });
            Volatile.Write(ref _configuration, current);
            if (!SpinWait.SpinUntil(() => _ready.Task.IsCompleted, ThreadWait) ||
                !_ready.Task.IsCompletedSuccessfully || _ready.Task.Result == IntPtr.Zero ||
                !_thread.IsAlive)
                return Finish("configure", configuration.CorrelationId, started, new(false, "Elevated input window is unavailable."), "window_unavailable");

            if (!DestinationOwned(configuration.DestinationWindow))
            {
                return Finish("configure", configuration.CorrelationId, started, new(false, "Input destination is not owned by the authenticated host."), "destination_invalid");
            }

            var needsHook = configuration.Keyboard.Enabled && !configuration.Keyboard.Recording &&
                configuration.Keyboard.Trigger == QuickLaunchTriggerKind.DoubleControl;
            if (!needsHook)
            {
                return Finish("configure", configuration.CorrelationId, started, new(true), "inactive");
            }

            try
            {
                if (_monitor is null)
                    _monitor = new LowLevelKeyboardMonitor(OnKeyDown, OnKeyUp, _keyboardPlatform,
                        healthChanged: OnHookHealthChanged, log: _log,
                        correlationId: configuration.CorrelationId);
                else if (!_monitor.IsInstalled)
                    _monitor.TryInstall(configuration.CorrelationId);
            }
            catch (Exception error)
            {
                var code = error is Win32Exception nativeError ? nativeError.NativeErrorCode : 0;
                NotifyUnavailable(epoch);
                return Finish("configure", configuration.CorrelationId, started,
                    new(false, "Elevated keyboard hook could not start.", NativeErrorCode: code), "install_failed");
            }
            if (!IsHookInstalled)
            {
                var code = _monitor?.LastNativeErrorCode ?? 0;
                NotifyUnavailable(epoch);
                return Finish("configure", configuration.CorrelationId, started,
                    new(false, "Elevated keyboard hook is unavailable.", NativeErrorCode: code), "install_failed");
            }
            return Finish("configure", configuration.CorrelationId, started, new(true), "ready");
        }
    }

    public WindowsCommandResult Paste(ElevatedPasteInput input)
    {
        var started = Stopwatch.GetTimestamp();
        Write("info", "elevated_input.paste.accepted", input.CorrelationId, "accepted", started);
        if (Volatile.Read(ref _disposed) != 0)
            return Finish("paste", input.CorrelationId, started, new(false, "Elevated input has stopped."), "stopped");
        try
        {
            var target = new IntPtr(input.TargetWindowHandle);
            var processId = _native.WindowProcessId(target);
            var root = _native.RootWindow(target);
            if (processId == 0 || root == IntPtr.Zero || !_native.IsSameSession(processId) ||
                !_native.IsElevated(processId))
                return Finish("paste", input.CorrelationId, started,
                    new(false, "The elevated paste target is unavailable."), "target_invalid");
            if (input.TargetFocusWindowHandle != 0)
            {
                var focus = new IntPtr(input.TargetFocusWindowHandle);
                if (_native.WindowProcessId(focus) != processId || _native.RootWindow(focus) != root)
                    return Finish("paste", input.CorrelationId, started,
                        new(false, "The paste focus does not belong to the target."), "focus_invalid");
            }
            var result = new WindowsPasteCoordinator(
                _pasteOperations ?? WindowsPlatformAdapter.CreatePasteInputOperations(), _log).ExecuteInput(input);
            return Finish("paste", input.CorrelationId, started, result,
                result.Success ? "completed" : "input_failed");
        }
        catch (Exception error)
        {
            var code = error is Win32Exception nativeError ? nativeError.NativeErrorCode : 0;
            return Finish("paste", input.CorrelationId, started,
                new(false, "Elevated paste input failed.", NativeErrorCode: code), "input_failed");
        }
    }

    private void OnKeyDown(int key, uint timestamp)
    {
        var current = Volatile.Read(ref _configuration);
        if (current is not null && _gesture.OnKeyDown(key, timestamp) &&
            ReferenceEquals(current, Volatile.Read(ref _configuration)))
            PostGesture(current);
    }

    private void OnKeyUp(int key)
    {
        var current = Volatile.Read(ref _configuration);
        if (current is not null && _gesture.OnKeyUp(key) &&
            ReferenceEquals(current, Volatile.Read(ref _configuration)))
            PostGesture(current);
    }

    private void PostGesture(Configuration current)
    {
        if (_gesture.Generation != current.Epoch) return;
        _ = _native.Post(Volatile.Read(ref _windowHandle), WmGesture,
            new IntPtr(current.Epoch), new IntPtr(_gesture.Timestamp));
    }

    private void OnHookHealthChanged(QuickLaunchKeyboardHealthChangedEvent health)
    {
        if (health.State == QuickLaunchKeyboardHealthState.Reinstalled) return;
        var current = Volatile.Read(ref _configuration);
        if (current is not null)
            NotifyUnavailable(current.Epoch);
    }

    private void NotifyUnavailable(long epoch) =>
        _ = _native.Post(Volatile.Read(ref _windowHandle), WmHookFailure, new IntPtr(epoch), IntPtr.Zero);

    private bool DestinationOwned(long handle) =>
        handle != 0 && _authenticatedPipeServerPid != 0 &&
        _native.WindowProcessId(new IntPtr(handle)) == _authenticatedPipeServerPid &&
        _native.IsSameSession(_authenticatedPipeServerPid);

    private void DispatchGesture(long epoch, bool failure, uint timestamp)
    {
        var current = Volatile.Read(ref _configuration);
        if (Volatile.Read(ref _disposed) != 0 || current is null || current.Epoch != epoch ||
            !current.Value.Keyboard.Enabled || current.Value.Keyboard.Recording ||
            current.Value.Keyboard.Trigger != QuickLaunchTriggerKind.DoubleControl ||
            !DestinationOwned(current.Value.DestinationWindow)) return;
        if (!failure && !IsHookInstalled) failure = true;
        if (failure)
        {
            if (IsHookInstalled || _lastUnavailableEpoch == epoch) return;
            _lastUnavailableEpoch = epoch;
        }
        else
        {
            var foreground = _native.ForegroundWindow();
            var foregroundPid = _native.WindowProcessId(foreground);
            if (foregroundPid == 0 || !_native.IsSameSession(foregroundPid) ||
                !_native.IsElevated(foregroundPid)) return;
            _ = _native.AllowForeground(_authenticatedPipeServerPid);
        }
        var message = failure ? ElevatedInputMessages.Unavailable : ElevatedInputMessages.Gesture;
        var sent = _native.Post(new IntPtr(current.Value.DestinationWindow), message,
            failure ? new IntPtr(Environment.ProcessId) :
                new IntPtr(unchecked((long)(((ulong)timestamp << 32) | (uint)Environment.ProcessId))),
            new IntPtr(current.Value.Keyboard.ResetGeneration));
        if (failure)
            Write(sent ? "warning" : "error", "elevated_input.hook.unavailable",
                current.Value.CorrelationId, sent ? "notified" : "notification_failed",
                Stopwatch.GetTimestamp(), _monitor?.LastNativeErrorCode ?? 0);
    }

    private void RunWindow()
    {
        InputWindow? window = null;
        try
        {
            window = new InputWindow(this);
            window.CreateHandle(new CreateParams { Parent = new IntPtr(-3) });
            Volatile.Write(ref _windowHandle, window.Handle);
            _native.StartPointerMonitor(window.Handle);
            _ready.TrySetResult(window.Handle);
            if (Volatile.Read(ref _disposed) == 0) Application.Run();
        }
        catch (Exception error)
        {
            _ready.TrySetException(error);
        }
        finally
        {
            var current = Volatile.Read(ref _configuration);
            if (Volatile.Read(ref _disposed) == 0 && current is not null && DestinationOwned(current.Value.DestinationWindow))
                _ = _native.Post(new IntPtr(current.Value.DestinationWindow), ElevatedInputMessages.Unavailable,
                    new IntPtr(Environment.ProcessId), new IntPtr(current.Value.Keyboard.ResetGeneration));
            lock (_lifecycle) { _monitor?.Dispose(); }
            Volatile.Write(ref _windowHandle, IntPtr.Zero);
            if (window is not null) _native.StopPointerMonitor(window.Handle);
            window?.DestroyHandle();
        }
    }

    public void Dispose()
    {
        lock (_lifecycle)
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
            Volatile.Write(ref _configuration, null);
            _monitor?.Dispose();
            _monitor = null;
            var handle = Volatile.Read(ref _windowHandle);
            if (handle != IntPtr.Zero) _ = _native.Post(handle, WmStop, IntPtr.Zero, IntPtr.Zero);
        }
        if (!_thread.Join(ThreadWait))
            Write("warning", "elevated_input.window.stop_deferred", Guid.Empty,
                "stop_deferred", Stopwatch.GetTimestamp());
        GC.SuppressFinalize(this);
    }

    private WindowsCommandResult Finish(string operation, Guid correlation, long started,
        WindowsCommandResult result, string state)
    {
        Write(result.Success ? "info" : "error", $"elevated_input.{operation}.terminal",
            correlation, state, started, result.NativeErrorCode ?? 0);
        return result;
    }

    private void Write(string level, string name, Guid correlation, string state, long started, int nativeError = 0)
    {
        try
        {
            _log.Write(level, name, correlation == Guid.Empty ? null : correlation,
                new Dictionary<string, object?>
                {
                    ["role"] = "helper", ["state"] = state,
                    ["native_error_code"] = nativeError,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
        }
        catch { /* Diagnostics cannot change native input ownership. */ }
    }

    private sealed class InputWindow(ElevatedInputHost owner) : NativeWindow
    {
        protected override void WndProc(ref Message message)
        {
            if (message.Msg == 0x0113 && message.WParam.ToInt64() == 2)
            {
                owner.MonitorPointerPress();
                return;
            }
            if (message.Msg == WmStop)
            {
                Application.ExitThread();
                return;
            }
            if (message.Msg == WmGesture || message.Msg == WmHookFailure)
            {
                try { owner.DispatchGesture(message.WParam.ToInt64(), message.Msg == WmHookFailure,
                    unchecked((uint)message.LParam.ToInt64())); }
                catch (Exception error)
                {
                    owner.Write("error", "elevated_input.gesture.failure", Guid.Empty,
                        "native_failure", Stopwatch.GetTimestamp(),
                        error is Win32Exception nativeError ? nativeError.NativeErrorCode : 0);
                }
                return;
            }
            base.WndProc(ref message);
        }
    }

    private sealed partial class WindowsElevatedInputNative : IElevatedInputNative
    {
        public uint WindowProcessId(IntPtr window) =>
            window != IntPtr.Zero && GetWindowThreadProcessId(window, out var process) != 0 ? process : 0;
        public IntPtr RootWindow(IntPtr window) => window == IntPtr.Zero ? IntPtr.Zero : GetAncestor(window, 2);
        public IntPtr ForegroundWindow() => GetForegroundWindow();
        public bool IsSameSession(uint processId) =>
            ElevatedDirectoryHelperPeerVerifier.IsSameSession(processId, (uint)Environment.ProcessId);
        public bool IsElevated(uint processId) => ElevatedDirectoryHelperPeerVerifier.IsProcessElevated(processId);
        public bool AllowForeground(uint processId) => AllowSetForegroundWindow(processId);
        public bool Post(IntPtr window, int message, IntPtr wParam, IntPtr lParam) =>
            window != IntPtr.Zero && PostMessage(window, message, wParam, lParam);

        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)]
        private static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AllowSetForegroundWindow(uint processId);
        [DllImport("user32.dll", EntryPoint = "PostMessageW", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool PostMessage(IntPtr window, int message, IntPtr wParam, IntPtr lParam);
    }
}
