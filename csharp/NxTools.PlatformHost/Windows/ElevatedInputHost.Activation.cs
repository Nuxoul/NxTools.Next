using System.ComponentModel;
using System.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class ElevatedInputHost
{
    public WindowsCommandResult Activate(ElevatedWindowActivation input)
    {
        var started = Stopwatch.GetTimestamp();
        Write("info", "elevated_input.activation.accepted", input.CorrelationId, "accepted", started);
        var current = Volatile.Read(ref _configuration);
        var target = new IntPtr(input.WindowHandle);
        if (Volatile.Read(ref _disposed) != 0 || current is null ||
            !DestinationOwned(current.Value.DestinationWindow) || current.Value.DesktopProcessId == 0 ||
            _native.WindowProcessId(target) != current.Value.DesktopProcessId ||
            !_native.IsSameSession(current.Value.DesktopProcessId) || _native.RootWindow(target) != target)
            return Finish("activation", input.CorrelationId, started,
                new(false, "The activation target is not owned by the connected Desktop."), "target_invalid");
        try
        {
            if (!ReferenceEquals(current, Volatile.Read(ref _configuration)))
                return Finish("activation", input.CorrelationId, started,
                    new(false, "The Desktop activation configuration was retired."), "retired");
            var success = _native.ActivateWindow(target);
            return Finish("activation", input.CorrelationId, started,
                new(success, success ? null : "Windows did not activate and focus the Desktop window."),
                success ? "completed" : "activation_failed");
        }
        catch (Exception error)
        {
            return Finish("activation", input.CorrelationId, started,
                new(false, "Desktop window activation failed.",
                    NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult),
                "native_failure");
        }
    }

    private sealed partial class WindowsElevatedInputNative
    {
        public bool ActivateWindow(IntPtr window)
        {
            WindowsKeyboardHookPlatform.EnsureThreadMessageQueue();
            return WindowsPlatformAdapter.TryActivateNativeWindow(window.ToInt64(), window.ToInt64());
        }
    }
}
