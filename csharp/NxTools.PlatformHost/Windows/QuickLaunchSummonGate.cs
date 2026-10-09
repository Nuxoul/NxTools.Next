using System.Diagnostics;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal enum QuickLaunchSummonTrigger
{
    HotKey,
    DoubleControl
}

internal readonly record struct FullscreenNotificationStateResult(
    bool Succeeded,
    int State,
    int Hresult)
{
    public static FullscreenNotificationStateResult Failure(int hresult) =>
        new(false, 0, hresult);
}

internal readonly record struct QuickLaunchSummonDecision(
    Guid CorrelationId,
    QuickLaunchSummonTrigger Trigger,
    bool TriggerEnabled,
    bool Suppressed,
    bool Published,
    bool QueryAttempted,
    int? NotificationState,
    int? QueryHresult,
    double ElapsedMilliseconds);

internal static class QuickLaunchSummonGate
{
    public const int RunningD3dFullscreenState = 3;

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

    public static QuickLaunchSummonDecision Dispatch(
        bool triggerEnabled,
        bool suppressInExclusiveFullscreen,
        QuickLaunchSummonTrigger trigger,
        Func<FullscreenNotificationStateResult> queryFullscreenState,
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
                state: null,
                hresult: null,
                startedAt);
        }

        FullscreenNotificationStateResult? queryResult = null;
        if (suppressInExclusiveFullscreen)
        {
            try
            {
                queryResult = queryFullscreenState();
            }
            catch (Exception exception)
            {
                queryResult = FullscreenNotificationStateResult.Failure(exception.HResult);
            }
            if (queryResult is { Succeeded: true, State: RunningD3dFullscreenState })
            {
                return Decision(
                    correlationId,
                    trigger,
                    triggerEnabled,
                    suppressed: true,
                    published: false,
                    queryAttempted: true,
                    state: queryResult.Value.State,
                    hresult: null,
                    startedAt);
            }
        }

        publish(correlationId);
        return Decision(
            correlationId,
            trigger,
            triggerEnabled,
            suppressed: false,
            published: true,
            queryAttempted: queryResult is not null,
            state: queryResult is { Succeeded: true } ? queryResult.Value.State : null,
            hresult: queryResult is { Succeeded: false } ? queryResult.Value.Hresult : null,
            startedAt);
    }

    private static QuickLaunchSummonDecision Decision(
        Guid correlationId,
        QuickLaunchSummonTrigger trigger,
        bool triggerEnabled,
        bool suppressed,
        bool published,
        bool queryAttempted,
        int? state,
        int? hresult,
        long startedAt) =>
        new(
            correlationId,
            trigger,
            triggerEnabled,
            suppressed,
            published,
            queryAttempted,
            state,
            hresult,
            Stopwatch.GetElapsedTime(startedAt).TotalMilliseconds);

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
            if (decision.QueryHresult is not null)
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
