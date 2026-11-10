using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

/// <summary>Owns one blocking hook thread. The callback only translates input and posts a gesture.</summary>
internal sealed class LowLevelKeyboardMonitor : IDisposable
{
    private static readonly TimeSpan ThreadWaitBudget = TimeSpan.FromSeconds(2);
    private readonly Action<int, uint> _keyDown;
    private readonly Action<int> _keyUp;
    private readonly IKeyboardHookPlatform _platform;
    private readonly KeyboardHookProcedure _procedure;
    private readonly Action<QuickLaunchKeyboardHealthChangedEvent>? _healthChanged;
    private readonly IPlatformLog _log;
    private readonly object _lifecycle = new();
    private Run? _run;
    private IntPtr _hook; // Hook-thread owned, including installation and uninstallation.
    private int _installed;
    private int _disposed;
    private int _lastNativeError;
    private QuickLaunchKeyboardHealthChangedEvent? _healthSnapshot;
    private bool _attempted;

    private sealed class Run(Guid correlationId, bool retry)
    {
        public readonly Guid CorrelationId = correlationId;
        public readonly bool Retry = retry;
        public readonly long StartedAt = Stopwatch.GetTimestamp();
        public readonly TaskCompletionSource<bool> Started = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public Thread Thread = null!;
        public uint ThreadId;
        public int StopRequested;
    }

    public LowLevelKeyboardMonitor(
        Action<int, uint> keyDown,
        Action<int> keyUp,
        IKeyboardHookPlatform? platform = null,
        Action<QuickLaunchKeyboardHealthChangedEvent>? healthChanged = null,
        IPlatformLog? log = null,
        Guid? correlationId = null)
    {
        _keyDown = keyDown;
        _keyUp = keyUp;
        _platform = platform ?? new WindowsKeyboardHookPlatform();
        _healthChanged = healthChanged;
        _log = log ?? NullPlatformLog.Instance;
        _procedure = HookCallback; // Rooted until the owned thread has unhooked.
        TryInstall(correlationId ?? Guid.NewGuid());
    }

    // Observable ownership only: Windows does not notify us about silent hook removal.
    public bool IsInstalled => Volatile.Read(ref _disposed) == 0 && Volatile.Read(ref _installed) != 0;
    public int LastNativeErrorCode => Volatile.Read(ref _lastNativeError);
    public QuickLaunchKeyboardHealthChangedEvent? HealthSnapshot => Volatile.Read(ref _healthSnapshot);

    public bool TryInstall(Guid correlationId)
    {
        lock (_lifecycle)
        {
            if (Volatile.Read(ref _disposed) != 0) return false;
            if (IsInstalled) return true;
            // An exited run may still be publishing its terminal health transition.
            if (_run is { } previous && !previous.Thread.Join(ThreadWaitBudget)) return false;
            var run = new Run(correlationId, _attempted);
            _attempted = true;
            run.Thread = new Thread(() => RunHook(run)) { IsBackground = true, Name = "NxTools.KeyboardHook" };
            _run = run;
            WriteDiagnostic("info", "quick_launch.keyboard_hook.install_started", run, 0);
            run.Thread.Start();
            if (run.Started.Task.Wait(ThreadWaitBudget)) return run.Started.Task.Result && IsInstalled;
            Volatile.Write(ref run.StopRequested, 1);
            var threadId = Volatile.Read(ref run.ThreadId);
            if (threadId != 0) _ = _platform.PostQuit(threadId);
            Publish(QuickLaunchKeyboardHealthState.InstallFailed, run, 1460);
            return false;
        }
    }

    private void RunHook(Run run)
    {
        var installed = false;
        var nativeError = 0;
        try
        {
            Volatile.Write(ref run.ThreadId, _platform.InitializeThread());
            if (Volatile.Read(ref run.StopRequested) != 0) return;
            _hook = _platform.Install(_procedure);
            installed = true;
            if (Volatile.Read(ref run.StopRequested) != 0) return;
            Volatile.Write(ref _lastNativeError, 0);
            Volatile.Write(ref _installed, 1);
            if (run.Retry) Publish(QuickLaunchKeyboardHealthState.Reinstalled, run, 0);
            else WriteDiagnostic("info", "quick_launch.keyboard_hook.installed", run, 0);
            run.Started.TrySetResult(true);
            _platform.RunMessageLoop();
        }
        catch (Exception error)
        {
            nativeError = error is Win32Exception native ? native.NativeErrorCode : 0;
        }
        finally
        {
            if (installed)
            {
                _platform.Uninstall(_hook);
                _hook = IntPtr.Zero;
            }
            Volatile.Write(ref _installed, 0);
            if (Volatile.Read(ref run.StopRequested) == 0)
                Publish(installed ? QuickLaunchKeyboardHealthState.ThreadExited : QuickLaunchKeyboardHealthState.InstallFailed,
                    run, nativeError);
            run.Started.TrySetResult(false);
        }
    }

    public void Dispose()
    {
        lock (_lifecycle)
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
            if (_run is { } run)
            {
                Volatile.Write(ref run.StopRequested, 1);
                var threadId = Volatile.Read(ref run.ThreadId);
                if (threadId != 0 && run.Thread.IsAlive) _ = _platform.PostQuit(threadId);
                if (!run.Thread.Join(ThreadWaitBudget))
                    WriteDiagnostic("warning", "quick_launch.keyboard_hook.stop_deferred", run, 0);
            }
        }
        GC.SuppressFinalize(this);
    }

    private void Publish(QuickLaunchKeyboardHealthState state, Run run, int nativeError)
    {
        Volatile.Write(ref _lastNativeError, nativeError);
        var snapshot = new QuickLaunchKeyboardHealthChangedEvent(run.CorrelationId, state, nativeError);
        Volatile.Write(ref _healthSnapshot, snapshot);
        var suffix = state switch
        {
            QuickLaunchKeyboardHealthState.InstallFailed => "install_failed",
            QuickLaunchKeyboardHealthState.ThreadExited => "thread_exited",
            _ => "reinstalled"
        };
        WriteDiagnostic(state == QuickLaunchKeyboardHealthState.Reinstalled ? "info" : "error",
            $"quick_launch.keyboard_hook.{suffix}", run, nativeError);
        try { _healthChanged?.Invoke(snapshot); }
        catch { /* A consumer cannot change native thread cleanup or installation. */ }
    }

    private void WriteDiagnostic(string level, string eventName, Run run, int nativeError)
    {
        try
        {
            _log.Write(level, eventName, run.CorrelationId, new Dictionary<string, object?>
            {
                ["native_error_code"] = nativeError,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(run.StartedAt).TotalMilliseconds
            });
        }
        catch { /* The bounded diagnostic sink cannot change hook ownership. */ }
    }

    private IntPtr HookCallback(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0)
        {
            var key = Marshal.PtrToStructure<KeyboardHookData>(data);
            if ((key.Flags & 0x12) == 0) // LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED
            {
                switch (message.ToInt32())
                {
                    case 0x0100:
                    case 0x0104:
                        _keyDown(unchecked((int)key.VirtualKey), key.Timestamp);
                        break;
                    case 0x0101:
                    case 0x0105:
                        _keyUp(unchecked((int)key.VirtualKey));
                        break;
                }
            }
        }
        return _platform.CallNext(_hook, code, message, data);
    }
}
