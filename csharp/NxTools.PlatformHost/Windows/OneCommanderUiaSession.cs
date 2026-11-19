using System.Diagnostics;
using System.Runtime.InteropServices;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

internal sealed class OneCommanderPaneState(
    OneCommanderUiaSession owner,
    long identity,
    long windowHandle,
    uint processId,
    DateTimeOffset expiresAt) : IDisposable
{
    private int retired;
    internal OneCommanderUiaSession Owner { get; } = owner;
    internal long Identity { get; } = identity;
    internal long WindowHandle { get; } = windowHandle;
    internal uint ProcessId { get; } = processId;
    internal DateTimeOffset ExpiresAt { get; } = expiresAt;
    internal bool IsRetired => Volatile.Read(ref retired) != 0;
    internal void Retire() => Interlocked.Exchange(ref retired, 1);
    public void Dispose()
    {
        if (Interlocked.Exchange(ref retired, 1) == 0) Owner.Wake();
    }
}

internal sealed record OneCommanderPaneCapture(string CurrentDirectory, OneCommanderPaneState PaneState) : IDisposable
{
    public void Dispose() => PaneState.Dispose();
}

internal sealed class OneCommanderUiaSession
{
    private const int MaximumPanes = 64;
    private static readonly TimeSpan PaneLifetime = TimeSpan.FromMinutes(10);
    private readonly object gate = new();
    private readonly OneCommanderFocusTracker<NativeUiaElement> tracker;
    private readonly Dictionary<long, Pane> panes = [];
    private readonly HashSet<long> retiredWindows = [];
    private long[] observedWindows = [];
    private Target? foreground;
    private NativeUiAutomation? automation;
    private Scope scope = new(NullPlatformLog.Instance, "ordinary", Guid.NewGuid(), Stopwatch.GetTimestamp());
    private Scope? nativeScope;
    private long generation;
    private long paneSequence;
    private bool scopeStopped;
    private int callbackFailure;
    private int reportedFailure;

    private sealed record Target(long Window, uint ProcessId);
    private sealed record Pane(OneCommanderPaneState State, NativeUiaElement Root);
    private sealed record Scope(IPlatformLog Log, string Role, Guid Correlation, long Started);
    private NativeUiAutomation Automation => automation ?? throw new InvalidOperationException("OneCommander native automation is unavailable.");

    internal OneCommanderUiaSession()
    {
        tracker = new(new(
            ResolveFocusWindow,
            OneCommanderNativeWindow.IsMainWindow,
            element => { _ = element.ProcessId; return true; },
            () => Automation.FocusedElement(),
            remember =>
            {
                var subscription = Automation.SubscribeFocus(AcceptCachedIdentity, remember, RecordCallbackFailure);
                Log("tracking", "foreground_target");
                return () =>
                {
                    subscription.Dispose();
                    Log("unsubscribed", "demand_ended");
                };
            },
            Retain: element => element.Clone(),
            Release: element => element.Dispose(),
            StartOwner: StartOwner,
            StopOwner: StopOwner,
            FocusDemand: HasFocusDemand,
            RetireOwner: () => panes.Count == 0 && !HasFocusDemand(),
            MaintainOwner: Maintain,
            SweepOwner: Sweep,
            Failure: ReportFailure), startImmediately: false);
    }

    internal void BeginLifetime(IPlatformLog log, string role)
    {
        lock (gate)
        {
            if (!scopeStopped && ReferenceEquals(scope.Log, log) && scope.Role == role) return;
            scope = new(log, role, Guid.NewGuid(), Stopwatch.GetTimestamp());
            scopeStopped = false;
            foreground = null;
        }
    }

    internal void ObserveForeground(ForegroundContext context)
    {
        Target? target = null;
        if (Path.GetFileNameWithoutExtension(context.ProcessName).Equals("OneCommander", StringComparison.OrdinalIgnoreCase) &&
            OneCommanderNativeWindow.IsMainWindow(context.WindowHandle) &&
            OneCommanderNativeWindow.ProcessId(context.WindowHandle) == context.ProcessId)
            target = new(context.WindowHandle, context.ProcessId);
        lock (gate)
        {
            if (scopeStopped) return;
            foreground = target;
        }
        tracker.Wake(startIfNeeded: target is not null);
    }

    internal void RetireWindow(long window)
    {
        lock (gate)
        {
            if (foreground?.Window != window && !Volatile.Read(ref observedWindows).Contains(window)) return;
            if (foreground?.Window == window) foreground = null;
            retiredWindows.Add(window);
        }
        tracker.Wake();
    }

    internal void Wake() => tracker.Wake();

    internal void Stop()
    {
        lock (gate)
        {
            if (scopeStopped) return;
            scopeStopped = true;
            foreground = null;
            Interlocked.Increment(ref generation);
        }
        tracker.RequestStop();
        if (!tracker.WaitForStop(TimeSpan.FromMilliseconds(500)))
            Log("retirement_pending", "native_call_in_flight");
    }

    private void StartOwner()
    {
        lock (gate)
        {
            if (scopeStopped) throw new InvalidOperationException("The OneCommander directory session has stopped.");
            nativeScope = scope;
        }
        automation = new NativeUiAutomation();
        automation.SetTimeouts(120);
        reportedFailure = 0;
        Interlocked.Exchange(ref callbackFailure, 0);
        Log("started", "target_demand");
    }

    private void StopOwner()
    {
        try
        {
            foreach (var pane in panes.Values)
            {
                pane.State.Retire();
                pane.Root.Dispose();
            }
            panes.Clear();
            Volatile.Write(ref observedWindows, []);
            var client = automation;
            automation = null;
            client?.Dispose();
            Log("retired", "owner_stopped");
        }
        finally
        {
            nativeScope = null;
        }
    }

    private bool HasFocusDemand()
    {
        Target? target;
        lock (gate) target = scopeStopped ? null : foreground;
        return target is not null && OneCommanderNativeWindow.IsForeground(target.Window, target.ProcessId);
    }

    private bool AcceptCachedIdentity(int processId, long window)
    {
        Target? target;
        lock (gate) target = scopeStopped ? null : foreground;
        if (target is null || processId <= 0 || processId != target.ProcessId)
        {
            tracker.Wake();
            return false;
        }
        if (window != 0 && !OneCommanderNativeWindow.IsWithin(window, target.Window, target.ProcessId))
        {
            tracker.Wake();
            return false;
        }
        return true;
    }

    private long ResolveFocusWindow(NativeUiaElement element)
    {
        Target? target;
        lock (gate) target = scopeStopped ? null : foreground;
        if (target is null) return 0;
        return OneCommanderFocusProcessGate.ResolveTopLevelWindow(
            element.ProcessId,
            processId => processId == target.ProcessId,
            () =>
            {
                using var walker = Automation.RawViewWalker();
                NativeUiaElement? node = element.Clone();
                try
                {
                    for (var depth = 0; depth < 64 && node is not null; depth++)
                    {
                        if (tracker.StopRequested) return 0;
                        var window = node.NativeWindowHandle;
                        if (window != 0 && OneCommanderNativeWindow.IsWithin(window, target.Window, target.ProcessId))
                            return target.Window;
                        var parent = walker.Parent(node);
                        node.Dispose();
                        node = parent;
                    }
                    return 0;
                }
                finally { node?.Dispose(); }
            },
            window => window == target.Window && OneCommanderNativeWindow.IsMainWindow(window));
    }

    internal OneCommanderPaneCapture? Capture(long window, long focusWindow, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var requestGeneration = RequestGeneration();
        return tracker.Invoke(() => CaptureOnOwner(window, focusWindow, requestGeneration, cancellationToken), cancellationToken);
    }

    private OneCommanderPaneCapture? CaptureOnOwner(long window, long focusWindow, long requestGeneration, CancellationToken cancellationToken)
    {
        CheckRequest(requestGeneration, cancellationToken);
        Automation.SetTimeouts(120);
        var processId = OneCommanderNativeWindow.ProcessId(window);
        using var root = Automation.ElementFromHandle(window);
        using var walker = Automation.RawViewWalker();
        using var getterCondition = Automation.AutomationIdCondition("CurrentPathGet");
        using var setterCondition = Automation.AutomationIdCondition("CurrentPathSet");
        NativeUiaElement? focused = null;
        try
        {
            if (focusWindow != 0 && focusWindow != window && OneCommanderNativeWindow.IsWithin(focusWindow, window, processId))
            {
                try { focused = Automation.ElementFromHandle(focusWindow); }
                catch (Exception error) when (error is COMException or InvalidOperationException) { ReportFailure(error); }
            }
            if (focused is null || Automation.Compare(focused, root) || !IsInside(focused, root, walker, requestGeneration, cancellationToken))
            {
                focused?.Dispose();
                focused = tracker.TryGet(window);
            }
            if (focused is null && OneCommanderNativeWindow.IsForeground(window, processId))
                focused = Automation.FocusedElement();
            CheckRequest(requestGeneration, cancellationToken);
            if (focused is null || focused.ProcessId != processId || !IsInside(focused, root, walker, requestGeneration, cancellationToken)) return null;
            using var paneRoot = NearestPane(focused, root, walker, getterCondition, setterCondition, requestGeneration, cancellationToken);
            if (paneRoot is null) return null;
            using var getter = paneRoot.FindFirst(getterCondition);
            using var value = getter?.ValuePattern();
            var path = value?.Value;
            CheckRequest(requestGeneration, cancellationToken);
            if (!ThirdPartyFileManagerPath.LooksLikeRootedDirectory(path)) return null;
            while (panes.Count >= MaximumPanes) RetirePane(panes.MinBy(pair => pair.Key).Key);
            var state = new OneCommanderPaneState(this, ++paneSequence, window, processId, DateTimeOffset.UtcNow + PaneLifetime);
            var savedRoot = paneRoot.Clone();
            try { panes.Add(state.Identity, new(state, savedRoot)); }
            catch { savedRoot.Dispose(); throw; }
            PublishObservedWindows();
            if (cancellationToken.IsCancellationRequested || requestGeneration != Interlocked.Read(ref generation))
            {
                RetirePane(state.Identity);
                CheckRequest(requestGeneration, cancellationToken);
            }
            return new(path!, state);
        }
        finally { focused?.Dispose(); }
    }

    internal WindowsCommandResult Navigate(OneCommanderPaneState state, string targetDirectory, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var requestGeneration = RequestGeneration();
        return tracker.Invoke(() =>
        {
            CheckRequest(requestGeneration, cancellationToken);
            Automation.SetTimeouts(1_500);
            if (!ReferenceEquals(state.Owner, this) || state.IsRetired ||
                !panes.TryGetValue(state.Identity, out var pane) || pane.State != state ||
                !OneCommanderNativeWindow.IsWithin(state.WindowHandle, state.WindowHandle, state.ProcessId))
                return new WindowsCommandResult(false, "The captured OneCommander pane is no longer available.");
            using var savedPane = pane.Root.Clone();
            using var root = Automation.ElementFromHandle(state.WindowHandle);
            using var walker = Automation.RawViewWalker();
            if (!IsInside(savedPane, root, walker, requestGeneration, cancellationToken))
                return new WindowsCommandResult(false, "The captured OneCommander pane is no longer available.");
            using var getterCondition = Automation.AutomationIdCondition("CurrentPathGet");
            using var setterCondition = Automation.AutomationIdCondition("CurrentPathSet");
            using var setter = savedPane.FindFirst(setterCondition);
            using var value = setter?.ValuePattern();
            if (value is null)
                return new WindowsCommandResult(false, "OneCommander did not expose the captured pane's directory control.");
            CheckRequest(requestGeneration, cancellationToken);
            value.SetValue(targetDirectory);
            var elapsed = Stopwatch.StartNew();
            while (elapsed.ElapsedMilliseconds < 1_000)
            {
                CheckRequest(requestGeneration, cancellationToken);
                using var getter = savedPane.FindFirst(getterCondition);
                using var currentValue = getter?.ValuePattern();
                if (ThirdPartyFileManagerPath.SameDirectory(currentValue?.Value, targetDirectory))
                    return new WindowsCommandResult(true);
                Thread.Sleep(25);
            }
            return new WindowsCommandResult(false, "OneCommander did not switch the captured pane to the requested directory.");
        }, cancellationToken);
    }

    private NativeUiaElement? NearestPane(
        NativeUiaElement focused, NativeUiaElement root, NativeUiaTreeWalker walker,
        NativeUiaCondition getter, NativeUiaCondition setter, long requestGeneration, CancellationToken cancellationToken)
    {
        NativeUiaElement? node = focused.Clone();
        try
        {
            for (var depth = 0; depth < 25 && node is not null; depth++)
            {
                CheckRequest(requestGeneration, cancellationToken);
                if (Automation.Compare(node, root)) return null;
                using var read = node.FindFirst(getter);
                using var write = node.FindFirst(setter);
                if (read is not null && write is not null)
                {
                    var result = node;
                    node = null;
                    return result;
                }
                var parent = walker.Parent(node);
                node.Dispose();
                node = parent;
            }
            return null;
        }
        finally { node?.Dispose(); }
    }

    private bool IsInside(NativeUiaElement candidate, NativeUiaElement root, NativeUiaTreeWalker walker,
        long requestGeneration, CancellationToken cancellationToken)
    {
        NativeUiaElement? node = candidate.Clone();
        try
        {
            for (var depth = 0; depth < 25 && node is not null; depth++)
            {
                CheckRequest(requestGeneration, cancellationToken);
                if (Automation.Compare(node, root)) return true;
                var parent = walker.Parent(node);
                node.Dispose();
                node = parent;
            }
            return false;
        }
        finally { node?.Dispose(); }
    }

    private long RequestGeneration()
    {
        lock (gate)
        {
            if (scopeStopped) throw new InvalidOperationException("The OneCommander directory session has stopped.");
            return Interlocked.Read(ref generation);
        }
    }

    private void CheckRequest(long requestGeneration, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (requestGeneration != RequestGeneration() || tracker.StopRequested)
            throw new InvalidOperationException("The OneCommander request belongs to a retired directory session.");
    }

    private void Maintain()
    {
        long[] retired;
        lock (gate)
        {
            retired = retiredWindows.ToArray();
            retiredWindows.Clear();
        }
        var now = DateTimeOffset.UtcNow;
        foreach (var identity in panes.Where(pair => pair.Value.State.IsRetired || pair.Value.State.ExpiresAt <= now ||
                     retired.Contains(pair.Value.State.WindowHandle)).Select(pair => pair.Key).ToArray())
            RetirePane(identity);
        foreach (var window in retired) tracker.ForgetWindow(window);
        var code = Volatile.Read(ref callbackFailure);
        if (code != 0 && code != reportedFailure)
        {
            reportedFailure = code;
            Log("failed", "focus_callback", code);
        }
    }

    private void Sweep()
    {
        foreach (var identity in panes.Where(pair => !OneCommanderNativeWindow.IsWithin(
                     pair.Value.State.WindowHandle, pair.Value.State.WindowHandle, pair.Value.State.ProcessId))
                 .Select(pair => pair.Key).ToArray())
            RetirePane(identity);
    }

    private void RetirePane(long identity)
    {
        if (!panes.Remove(identity, out var pane)) return;
        pane.State.Retire();
        pane.Root.Dispose();
        PublishObservedWindows();
    }

    private void PublishObservedWindows() =>
        Volatile.Write(ref observedWindows, panes.Values.Select(pane => pane.State.WindowHandle).Distinct().ToArray());

    private void RecordCallbackFailure(int code)
    {
        Interlocked.Exchange(ref callbackFailure, code);
        tracker.Wake();
    }

    private void ReportFailure(Exception error)
    {
        var code = error is COMException native ? native.ErrorCode : error.HResult;
        if (Interlocked.Exchange(ref reportedFailure, code) != code) Log("failed", "native_operation", code);
    }

    private void Log(string state, string reason, int? nativeCode = null)
    {
        Scope current;
        lock (gate) current = nativeScope ?? scope;
        try
        {
            current.Log.Write(nativeCode is null ? "info" : "warning", "onecommander.uia.lifecycle", current.Correlation,
                new Dictionary<string, object?>
                {
                    ["process_role"] = current.Role,
                    ["state"] = state,
                    ["reason"] = reason,
                    ["pane_window_count"] = Volatile.Read(ref observedWindows).Length,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(current.Started).TotalMilliseconds,
                    ["native_error_code"] = nativeCode
                });
        }
        catch { }
    }
}
