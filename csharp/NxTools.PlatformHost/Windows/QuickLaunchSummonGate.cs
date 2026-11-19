using System.Diagnostics;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal enum QuickLaunchSummonTrigger
{
    HotKey,
    DoubleControl,
    ClipboardHotKey
}

internal enum FullscreenSuppressionKind
{
    None,
    ExclusiveD3d,
    Borderless
}

internal readonly record struct FullscreenRect(int Left, int Top, int Right, int Bottom)
{
    internal bool Covers(FullscreenRect monitor) =>
        Left <= monitor.Left && Top <= monitor.Top &&
        Right >= monitor.Right && Bottom >= monitor.Bottom;
}

internal readonly record struct FullscreenForegroundSnapshot(
    bool Valid,
    bool Visible,
    bool Minimized,
    bool Desktop,
    long Style,
    FullscreenRect WindowBounds,
    FullscreenRect MonitorBounds)
{
    internal bool CoversMonitor => WindowBounds.Covers(MonitorBounds);
}

internal readonly record struct FullscreenNotificationStateResult(
    bool Succeeded,
    int State,
    int Hresult)
{
    public static FullscreenNotificationStateResult Failure(int hresult) =>
        new(false, 0, hresult);
}

internal readonly record struct FullscreenSummonObservation(
    FullscreenNotificationStateResult Notification,
    bool ForegroundQuerySucceeded,
    FullscreenForegroundSnapshot Foreground,
    int? ForegroundHresult = null);

internal readonly record struct QuickLaunchSummonDecision(
    Guid CorrelationId,
    QuickLaunchSummonTrigger Trigger,
    bool TriggerEnabled,
    bool Suppressed,
    bool Published,
    bool QueryAttempted,
    int? NotificationState,
    int? QueryHresult,
    double ElapsedMilliseconds)
{
    public FullscreenSuppressionKind FullscreenKind { get; init; }
    public string FullscreenReason { get; init; } = "not_checked";
    public int? ForegroundHresult { get; init; }
}

internal static class QuickLaunchSummonGate
{
    public const int RunningD3dFullscreenState = 3;
    private const uint GaRoot = 2;
    private const uint MonitorDefaultToNearest = 2;

    public static FullscreenNotificationStateResult QueryWindowsNotificationState()
    {
        try
        {
            var hresult = SHQueryUserNotificationState(out var state);
            return hresult >= 0
                ? new FullscreenNotificationStateResult(true, state, 0)
                : FullscreenNotificationStateResult.Failure(hresult);
        }
        catch (Exception exception) when (
            exception is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException)
        {
            return FullscreenNotificationStateResult.Failure(exception.HResult);
        }
    }

    internal static FullscreenSummonObservation QueryWindowsFullscreenState()
    {
        var notification = QueryWindowsNotificationState();
        try
        {
            var foreground = Native.GetAncestor(Native.GetForegroundWindow(), GaRoot);
            if (foreground == IntPtr.Zero || !Native.IsWindow(foreground))
            {
                return new FullscreenSummonObservation(notification, false, default, NativeFailure());
            }

            if (!Native.GetWindowRect(foreground, out var windowBounds))
            {
                return new FullscreenSummonObservation(notification, false, default, NativeFailure());
            }

            var monitor = Native.MonitorFromWindow(foreground, MonitorDefaultToNearest);
            var monitorInfo = new Native.MonitorInfo { Size = Marshal.SizeOf<Native.MonitorInfo>() };
            if (monitor == IntPtr.Zero || !Native.GetMonitorInfoW(monitor, ref monitorInfo))
            {
                return new FullscreenSummonObservation(notification, false, default, NativeFailure());
            }

            var snapshot = new FullscreenForegroundSnapshot(
                true,
                Native.IsWindowVisible(foreground),
                Native.IsIconic(foreground),
                new Win32DesktopWindowOperations().IsDesktopWindow(foreground.ToInt64()),
                Native.GetWindowLongPtrW(foreground, -16).ToInt64(),
                new FullscreenRect(windowBounds.Left, windowBounds.Top, windowBounds.Right, windowBounds.Bottom),
                new FullscreenRect(
                    monitorInfo.Monitor.Left,
                    monitorInfo.Monitor.Top,
                    monitorInfo.Monitor.Right,
                    monitorInfo.Monitor.Bottom));
            return new FullscreenSummonObservation(notification, true, snapshot);
        }
        catch (Exception exception) when (
            exception is DllNotFoundException or EntryPointNotFoundException or BadImageFormatException or
            InvalidOperationException)
        {
            return new FullscreenSummonObservation(notification, false, default, exception.HResult);
        }
    }

    public static QuickLaunchSummonDecision Dispatch(
        bool triggerEnabled,
        bool suppressInExclusiveFullscreen,
        QuickLaunchSummonTrigger trigger,
        Func<FullscreenNotificationStateResult> queryFullscreenState,
        Action<Guid> publish)
    {
        ArgumentNullException.ThrowIfNull(queryFullscreenState);
        return DispatchObserved(
            triggerEnabled,
            suppressInExclusiveFullscreen,
            trigger,
            () => new FullscreenSummonObservation(
                queryFullscreenState(), false, default),
            publish);
    }

    internal static QuickLaunchSummonDecision DispatchObserved(
        bool triggerEnabled,
        bool suppressInFullscreen,
        QuickLaunchSummonTrigger trigger,
        Func<FullscreenSummonObservation> queryFullscreenState,
        Action<Guid> publish)
    {
        ArgumentNullException.ThrowIfNull(queryFullscreenState);
        ArgumentNullException.ThrowIfNull(publish);

        var correlationId = Guid.NewGuid();
        var startedAt = Stopwatch.GetTimestamp();
        if (!triggerEnabled)
        {
            return Decision(
                correlationId,
                trigger,
                triggerEnabled,
                suppressed: false,
                published: false,
                queryAttempted: false,
                observation: null,
                startedAt);
        }

        FullscreenSummonObservation? observation = null;
        if (suppressInFullscreen)
        {
            try
            {
                observation = queryFullscreenState();
            }
            catch (Exception exception)
            {
                observation = new FullscreenSummonObservation(
                    FullscreenNotificationStateResult.Failure(exception.HResult),
                    false,
                    default,
                    exception.HResult);
            }

            var classification = Classify(observation.Value);
            if (classification.Kind != FullscreenSuppressionKind.None)
            {
                return Decision(
                    correlationId,
                    trigger,
                    triggerEnabled,
                    suppressed: true,
                    published: false,
                    queryAttempted: true,
                    observation,
                    startedAt,
                    classification);
            }
        }

        publish(correlationId);
        return Decision(
            correlationId,
            trigger,
            triggerEnabled,
            suppressed: false,
            published: true,
            queryAttempted: observation is not null,
            observation,
            startedAt,
            observation is { } current ? Classify(current) : default);
    }

    internal static (FullscreenSuppressionKind Kind, string Reason) Classify(
        FullscreenSummonObservation observation)
    {
        if (observation.Notification is { Succeeded: true, State: RunningD3dFullscreenState })
        {
            return (FullscreenSuppressionKind.ExclusiveD3d, "exclusive_d3d_fullscreen");
        }

        if (!observation.ForegroundQuerySucceeded)
        {
            return (FullscreenSuppressionKind.None, "foreground_observation_failed");
        }

        var borderless = FloatingAssistantFullscreenPolicy.Evaluate(
            observation.Foreground.Valid,
            observation.Foreground.Visible,
            observation.Foreground.Minimized,
            observation.Foreground.Desktop,
            observation.Foreground.Style,
            observation.Foreground.CoversMonitor,
            out var reason);
        return borderless
            ? (FullscreenSuppressionKind.Borderless, reason)
            : (FullscreenSuppressionKind.None, reason);
    }

    private static QuickLaunchSummonDecision Decision(
        Guid correlationId,
        QuickLaunchSummonTrigger trigger,
        bool triggerEnabled,
        bool suppressed,
        bool published,
        bool queryAttempted,
        FullscreenSummonObservation? observation,
        long startedAt,
        (FullscreenSuppressionKind Kind, string Reason) classification = default) =>
        new(
            correlationId,
            trigger,
            triggerEnabled,
            suppressed,
            published,
            queryAttempted,
            observation is { Notification.Succeeded: true } current ? current.Notification.State : null,
            observation is { Notification.Succeeded: false } failed ? failed.Notification.Hresult : null,
            Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds)
        {
            FullscreenKind = classification.Kind,
            FullscreenReason = string.IsNullOrEmpty(classification.Reason) ? "not_fullscreen" : classification.Reason,
            ForegroundHresult = observation?.ForegroundHresult
        };

    private static int NativeFailure() =>
        Marshal.GetLastWin32Error() is var error && error != 0
            ? error
            : unchecked((int)0x80004005);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect
    {
        internal int Left;
        internal int Top;
        internal int Right;
        internal int Bottom;
    }

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)]
        internal struct MonitorInfo
        {
            internal int Size;
            internal NativeRect Monitor;
            internal NativeRect Work;
            internal uint Flags;
        }

        [DllImport("user32.dll", ExactSpelling = true)]
        internal static extern IntPtr GetAncestor(IntPtr window, uint flags);
        [DllImport("user32.dll", ExactSpelling = true)]
        internal static extern IntPtr GetForegroundWindow();
        [DllImport("user32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindow(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindowVisible(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsIconic(IntPtr window);
        [DllImport("user32.dll", ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool GetWindowRect(IntPtr window, out NativeRect rect);
        [DllImport("user32.dll", ExactSpelling = true)]
        internal static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
        [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", ExactSpelling = true, SetLastError = true)]
        internal static extern IntPtr GetWindowLongPtrW(IntPtr window, int index);
    }

    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern int SHQueryUserNotificationState(out int notificationState);
}

internal sealed class QuickLaunchSummonDiagnosticGate
{
    private const long FailureLogIntervalMilliseconds = 60_000;
    private readonly object _gate = new();
    private readonly Dictionary<QuickLaunchSummonTrigger, bool> _lastSuppressed = new();
    private long _lastFailureLogAt;
    private long _suppressedFailureCount;

    public bool TryAcquire(QuickLaunchSummonDecision decision, out long suppressedFailureCount)
    {
        lock (_gate)
        {
            if (decision.QueryHresult is not null || decision.ForegroundHresult is not null)
            {
                var now = Environment.TickCount64;
                if (_lastFailureLogAt == 0 || now - _lastFailureLogAt >= FailureLogIntervalMilliseconds)
                {
                    _lastFailureLogAt = now;
                    suppressedFailureCount = _suppressedFailureCount;
                    _suppressedFailureCount = 0;
                    return true;
                }
                _suppressedFailureCount++;
                suppressedFailureCount = 0;
                return false;
            }
            if (decision.Published)
            {
                _lastSuppressed[decision.Trigger] = false;
                suppressedFailureCount = 0;
                return true;
            }

            if (_lastSuppressed.TryGetValue(decision.Trigger, out var previous) &&
                previous == decision.Suppressed)
            {
                suppressedFailureCount = 0;
                return false;
            }

            _lastSuppressed[decision.Trigger] = decision.Suppressed;
            suppressedFailureCount = 0;
            return true;
        }
    }
}
