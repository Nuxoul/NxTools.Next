using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal static class TaskbarProgress
{
    public static WindowsCommandResult Set(
        IntPtr window,
        string state,
        ulong completed,
        ulong total)
    {
        if (window == IntPtr.Zero)
        {
            return new WindowsCommandResult(false, "Taskbar progress requires a live window handle.");
        }
        if (total > 0 && completed > total)
        {
            return new WindowsCommandResult(false, "Taskbar progress cannot exceed its total.");
        }
        if (!TryParseState(state, out var nativeState))
        {
            return new WindowsCommandResult(false, $"Unsupported taskbar progress state '{state}'.");
        }

        ITaskbarList4? taskbar = null;
        try
        {
            taskbar = (ITaskbarList4)(object)new CTaskbarList();
            Marshal.ThrowExceptionForHR(taskbar.HrInit());
            Marshal.ThrowExceptionForHR(taskbar.SetProgressState(window, nativeState));
            if (nativeState is TaskbarProgressFlag.Normal
                or TaskbarProgressFlag.Error
                or TaskbarProgressFlag.Paused)
            {
                if (total == 0)
                {
                    return new WindowsCommandResult(
                        false,
                        "Determinate taskbar progress requires a non-zero total.");
                }
                Marshal.ThrowExceptionForHR(taskbar.SetProgressValue(window, completed, total));
            }
            return new WindowsCommandResult(true);
        }
        catch (Exception error)
        {
            return new WindowsCommandResult(false, error.Message);
        }
        finally
        {
            if (taskbar is not null && Marshal.IsComObject(taskbar))
            {
                Marshal.FinalReleaseComObject(taskbar);
            }
        }
    }

    private static bool TryParseState(string state, out TaskbarProgressFlag result)
    {
        result = state switch
        {
            "none" => TaskbarProgressFlag.None,
            "indeterminate" => TaskbarProgressFlag.Indeterminate,
            "normal" => TaskbarProgressFlag.Normal,
            "error" => TaskbarProgressFlag.Error,
            "paused" => TaskbarProgressFlag.Paused,
            _ => (TaskbarProgressFlag)(-1)
        };
        return (int)result >= 0;
    }

    [Flags]
    private enum TaskbarProgressFlag
    {
        None = 0,
        Indeterminate = 0x1,
        Normal = 0x2,
        Error = 0x4,
        Paused = 0x8
    }

    [ComImport]
    [Guid("56FDF344-FD6D-11D0-958A-006097C9A090")]
    [ClassInterface(ClassInterfaceType.None)]
    private sealed class CTaskbarList;

    [ComImport]
    [Guid("C43DC798-95D1-4BEA-9030-BB99E2983A1A")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface ITaskbarList4
    {
        [PreserveSig] int HrInit();
        [PreserveSig] int AddTab(IntPtr hwnd);
        [PreserveSig] int DeleteTab(IntPtr hwnd);
        [PreserveSig] int ActivateTab(IntPtr hwnd);
        [PreserveSig] int SetActiveAlt(IntPtr hwnd);
        [PreserveSig] int MarkFullscreenWindow(IntPtr hwnd, [MarshalAs(UnmanagedType.Bool)] bool fullscreen);
        [PreserveSig] int SetProgressValue(IntPtr hwnd, ulong completed, ulong total);
        [PreserveSig] int SetProgressState(IntPtr hwnd, TaskbarProgressFlag state);
    }
}

internal sealed record TaskbarProgressLogDecision(
    string Level,
    string EventName,
    int? PercentMilestone);

internal sealed class TaskbarProgressLogGate
{
    private const long RepeatedFailureIntervalMilliseconds = 30_000;
    private const int MaximumTrackedWindows = 256;
    private readonly object _gate = new();
    private readonly Dictionary<long, TaskbarProgressLogState> _states = [];

    public TaskbarProgressLogDecision? Evaluate(
        long windowHandle,
        string state,
        ulong completed,
        ulong total,
        WindowsCommandResult result,
        long timestampMilliseconds)
    {
        lock (_gate)
        {
            if (!_states.TryGetValue(windowHandle, out var previous))
            {
                if (_states.Count >= MaximumTrackedWindows)
                {
                    var oldest = _states.MinBy(pair => pair.Value.LastObservedAt);
                    _states.Remove(oldest.Key);
                }
                previous = new TaskbarProgressLogState();
                _states.Add(windowHandle, previous);
            }
            previous.LastObservedAt = timestampMilliseconds;
            if (!result.Success)
            {
                var repeatedTooSoon =
                    string.Equals(
                        previous.LastError,
                        result.Error,
                        StringComparison.Ordinal) &&
                    timestampMilliseconds - previous.LastErrorLoggedAt <
                    RepeatedFailureIntervalMilliseconds;
                previous.LastError = result.Error;
                if (!repeatedTooSoon)
                {
                    previous.LastErrorLoggedAt = timestampMilliseconds;
                }
                return repeatedTooSoon
                    ? null
                    : new TaskbarProgressLogDecision(
                        "error",
                        "taskbar_progress.failed",
                        previous.PercentMilestone);
            }

            var milestone = PercentMilestone(completed, total);
            var changed = !previous.HasSuccessfulState ||
                          !string.Equals(
                              previous.State,
                              state,
                              StringComparison.Ordinal) ||
                          previous.PercentMilestone != milestone ||
                          previous.LastError is not null;
            previous.HasSuccessfulState = true;
            previous.State = state;
            previous.PercentMilestone = milestone;
            previous.LastError = null;
            previous.LastErrorLoggedAt = 0;
            return changed
                ? new TaskbarProgressLogDecision(
                    "info",
                    "taskbar_progress.applied",
                    milestone)
                : null;
        }
    }

    private static int? PercentMilestone(ulong completed, ulong total)
    {
        if (total == 0)
        {
            return null;
        }
        var percent = completed >= total
            ? 100
            : (int)Math.Floor((double)completed * 100 / total);
        return percent / 25 * 25;
    }

    private sealed class TaskbarProgressLogState
    {
        public bool HasSuccessfulState { get; set; }
        public string? State { get; set; }
        public int? PercentMilestone { get; set; }
        public string? LastError { get; set; }
        public long LastErrorLoggedAt { get; set; }
        public long LastObservedAt { get; set; }
    }
}
