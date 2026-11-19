using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal sealed record OneCommanderFocusOperations<TElement>(
    Func<TElement, long> ResolveWindow,
    Func<long, bool> WindowAvailable,
    Func<TElement, bool> ElementAvailable,
    Func<TElement?> FocusedElement,
    Func<Action<TElement>, Action> SubscribeFocus,
    Func<TElement, TElement>? Retain = null,
    Action<TElement>? Release = null,
    Action? StartOwner = null,
    Action? StopOwner = null,
    Func<bool>? FocusDemand = null,
    Func<bool>? RetireOwner = null,
    Action? MaintainOwner = null,
    Action? SweepOwner = null,
    Action<Exception>? Failure = null) where TElement : class;

internal sealed class OneCommanderFocusTracker<TElement> : IDisposable where TElement : class
{
    internal const int MaximumWindows = 64;
    internal static readonly TimeSpan DefaultSweepPeriod = TimeSpan.FromSeconds(60);
    private readonly OneCommanderFocusOperations<TElement> operations;
    private readonly object gate = new();
    private readonly Dictionary<long, Anchor> anchors = [];
    private RunState? running;
    private long focusSequence;
    private long wakeSequence;
    private bool disposed;

    private sealed record Anchor(TElement Element, long Sequence);

    private sealed class RunState
    {
        internal Thread Thread = null!;
        internal readonly TaskCompletionSource Started = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal Action? Work;
        internal Action<Exception>? RejectWork;
        internal TElement? PendingFocus;
        internal Exception? Failure;
        internal bool NativeCallActive;
        internal bool StopRequested;
    }

    internal TimeSpan SweepPeriod { get; }
    internal int Count
    {
        get
        {
            lock (gate) return IsStoppingLocked() ? 0 : anchors.Count;
        }
    }

    internal bool StopRequested
    {
        get { lock (gate) return IsStoppingLocked(); }
    }

    internal OneCommanderFocusTracker(
        OneCommanderFocusOperations<TElement> operations,
        TimeSpan? sweepPeriod = null,
        bool startImmediately = true)
    {
        this.operations = operations;
        SweepPeriod = sweepPeriod ?? DefaultSweepPeriod;
        if (startImmediately) EnsureStarted();
    }

    internal void EnsureStarted()
    {
        RunState state;
        lock (gate)
        {
            if (disposed) return;
            state = StartLocked();
        }
        state.Started.Task.Wait(TimeSpan.FromMilliseconds(500));
    }

    internal void Wake(bool startIfNeeded = false)
    {
        lock (gate)
        {
            if (disposed) return;
            if (startIfNeeded) _ = StartLocked();
            wakeSequence++;
            Monitor.PulseAll(gate);
        }
    }

    internal TResult Invoke<TResult>(Func<TResult> operation, CancellationToken cancellationToken)
    {
        var completion = new TaskCompletionSource<TResult>(TaskCreationOptions.RunContinuationsAsynchronously);
        lock (gate)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            var state = StartLocked();
            if (state.StopRequested || state.NativeCallActive)
                throw new InvalidOperationException("OneCommander is still finishing its previous native operation.");
            state.NativeCallActive = true;
            state.RejectWork = error => completion.TrySetException(error);
            state.Work = () =>
            {
                try
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    completion.TrySetResult(operation());
                }
                catch (Exception error)
                {
                    Report(error);
                    completion.TrySetException(error);
                }
            };
            wakeSequence++;
            Monitor.PulseAll(gate);
        }
        // The admitted STA keeps its original physical slot until this native operation returns.
        return completion.Task.GetAwaiter().GetResult();
    }

    private RunState StartLocked()
    {
        if (running is { } previous)
        {
            if (previous.Thread.IsAlive) return previous;
            previous.Thread.Join();
        }
        var state = new RunState();
        state.Thread = new Thread(() => Run(state))
        {
            IsBackground = true,
            Name = "NxTools.OneCommanderFocusTracker"
        };
        state.Thread.SetApartmentState(ApartmentState.MTA);
        running = state;
        try { state.Thread.Start(); }
        catch
        {
            running = null;
            throw;
        }
        return state;
    }

    internal void RememberFocus(TElement element)
    {
        if (operations.StartOwner is null)
        {
            RememberOnOwner(element);
            return;
        }
        var retained = Retain(element);
        TElement? replaced = null;
        lock (gate)
        {
            if (!IsStoppingLocked() && running is { } state)
            {
                replaced = state.PendingFocus;
                state.PendingFocus = retained;
                retained = null!;
                wakeSequence++;
                Monitor.PulseAll(gate);
            }
        }
        Release(replaced);
        Release(retained);
    }

    private void RememberOnOwner(TElement element)
    {
        lock (gate)
        {
            if (IsStoppingLocked()) return;
        }
        TElement? retained = null;
        List<TElement> retired = [];
        try
        {
            var window = operations.ResolveWindow(element);
            if (window == 0 || !operations.WindowAvailable(window)) return;
            retained = Retain(element);
            lock (gate)
            {
                if (IsStoppingLocked()) return;
                if (anchors.Remove(window, out var replaced)) retired.Add(replaced.Element);
                anchors[window] = new(retained, ++focusSequence);
                retained = null;
                if (anchors.Count > MaximumWindows)
                {
                    var oldest = anchors.MinBy(pair => pair.Value.Sequence);
                    anchors.Remove(oldest.Key);
                    retired.Add(oldest.Value.Element);
                }
            }
        }
        catch (Exception error) when (Unavailable(error)) { Report(error); }
        finally
        {
            Release(retained);
            foreach (var elementToRelease in retired) Release(elementToRelease);
        }
    }

    internal TElement? TryGet(long window)
    {
        Anchor? anchor;
        lock (gate)
        {
            if (IsStoppingLocked() || !anchors.TryGetValue(window, out anchor)) return null;
        }
        try
        {
            if (operations.WindowAvailable(window) && operations.ElementAvailable(anchor.Element))
            {
                bool currentAnchor;
                lock (gate)
                    currentAnchor = !IsStoppingLocked() && anchors.TryGetValue(window, out var current) && ReferenceEquals(current, anchor);
                if (currentAnchor) return Retain(anchor.Element);
            }
        }
        catch (Exception error) when (Unavailable(error)) { Report(error); }
        RetireIfCurrent(window, anchor);
        return null;
    }

    private void RetireIfCurrent(long window, Anchor anchor)
    {
        TElement? retired = null;
        lock (gate)
        {
            if (anchors.TryGetValue(window, out var current) && ReferenceEquals(current, anchor))
            {
                anchors.Remove(window);
                retired = anchor.Element;
            }
        }
        Release(retired);
    }

    internal void ForgetWindow(long window)
    {
        Anchor? retired;
        lock (gate) anchors.Remove(window, out retired);
        if (retired is not null) Release(retired.Element);
    }

    private void ClearAnchors()
    {
        TElement[] retired;
        lock (gate)
        {
            retired = anchors.Values.Select(anchor => anchor.Element).ToArray();
            anchors.Clear();
        }
        foreach (var element in retired) Release(element);
    }

    private void SweepUnavailableWindows()
    {
        KeyValuePair<long, Anchor>[] snapshot;
        lock (gate) snapshot = anchors.ToArray();
        foreach (var pair in snapshot)
        {
            lock (gate)
            {
                if (IsStoppingLocked()) return;
            }
            try
            {
                if (!operations.WindowAvailable(pair.Key)) RetireIfCurrent(pair.Key, pair.Value);
            }
            catch (Exception error) when (Unavailable(error))
            {
                Report(error);
                RetireIfCurrent(pair.Key, pair.Value);
            }
        }
    }

    private void Run(RunState state)
    {
        Action? unsubscribe = null;
        var nextSweep = Environment.TickCount64 + (long)SweepPeriod.TotalMilliseconds;
        try
        {
            operations.StartOwner?.Invoke();
            while (true)
            {
                long observedWake;
                lock (gate)
                {
                    if (state.StopRequested || disposed) break;
                    observedWake = wakeSequence;
                }
                operations.MaintainOwner?.Invoke();
                var focusDemand = operations.FocusDemand?.Invoke() ?? true;
                if (focusDemand && unsubscribe is null)
                {
                    unsubscribe = operations.SubscribeFocus(RememberFocus);
                    try
                    {
                        var focused = operations.FocusedElement();
                        if (focused is not null)
                        {
                            try { RememberOnOwner(focused); }
                            finally { Release(focused); }
                        }
                    }
                    catch (Exception error) when (Unavailable(error)) { Report(error); }
                }
                else if (!focusDemand && unsubscribe is not null)
                {
                    unsubscribe();
                    unsubscribe = null;
                    ClearAnchors();
                }
                state.Started.TrySetResult();

                TElement? pending;
                Action? work;
                lock (gate)
                {
                    pending = state.PendingFocus;
                    state.PendingFocus = null;
                    work = state.Work;
                    state.Work = null;
                }
                if (pending is not null)
                {
                    try
                    {
                        if (focusDemand) RememberOnOwner(pending);
                    }
                    finally { Release(pending); }
                }
                if (work is not null)
                {
                    try { work(); }
                    finally
                    {
                        lock (gate)
                        {
                            state.NativeCallActive = false;
                            state.RejectWork = null;
                        }
                    }
                }
                if (Environment.TickCount64 >= nextSweep)
                {
                    SweepUnavailableWindows();
                    operations.SweepOwner?.Invoke();
                    nextSweep = Environment.TickCount64 + (long)SweepPeriod.TotalMilliseconds;
                }
                operations.MaintainOwner?.Invoke();
                var retire = operations.RetireOwner?.Invoke() ?? false;
                lock (gate)
                {
                    if (state.StopRequested || disposed) break;
                    if (wakeSequence != observedWake || state.Work is not null || state.PendingFocus is not null) continue;
                    if (retire)
                    {
                        state.StopRequested = true;
                        break;
                    }
                    var wait = Math.Max(1, nextSweep - Environment.TickCount64);
                    Monitor.Wait(gate, TimeSpan.FromMilliseconds(wait));
                }
            }
        }
        catch (Exception error)
        {
            state.Failure = error;
            Report(error);
        }
        finally
        {
            TElement? pending;
            Action<Exception>? reject;
            lock (gate)
            {
                state.StopRequested = true;
                pending = state.PendingFocus;
                state.PendingFocus = null;
                reject = state.RejectWork;
                state.Work = null;
                state.RejectWork = null;
                state.NativeCallActive = false;
            }
            try { unsubscribe?.Invoke(); }
            catch (Exception error) { state.Failure ??= error; Report(error); }
            Release(pending);
            ClearAnchors();
            try { operations.StopOwner?.Invoke(); }
            catch (Exception error) { state.Failure ??= error; Report(error); }
            reject?.Invoke(state.Failure ?? new InvalidOperationException("OneCommander native ownership is retiring."));
            state.Started.TrySetResult();
        }
    }

    private bool IsStoppingLocked() => disposed || running?.StopRequested == true;
    private TElement Retain(TElement element) => operations.Retain?.Invoke(element) ?? element;
    private void Release(TElement? element)
    {
        if (element is null) return;
        try { operations.Release?.Invoke(element); }
        catch (Exception error) { Report(error); }
    }
    private void Report(Exception error)
    {
        try { operations.Failure?.Invoke(error); }
        catch { }
    }
    private static bool Unavailable(Exception error) =>
        error is InvalidOperationException or COMException or OperationCanceledException;

    internal void RequestStop()
    {
        lock (gate)
        {
            if (running is { } state) state.StopRequested = true;
            wakeSequence++;
            Monitor.PulseAll(gate);
        }
    }

    internal bool WaitForStop(TimeSpan timeout)
    {
        Thread? thread;
        lock (gate) thread = running?.Thread;
        return thread is null || (thread != Thread.CurrentThread && thread.Join(timeout));
    }

    public void Dispose()
    {
        lock (gate)
        {
            if (disposed) return;
            disposed = true;
            if (running is { } state) state.StopRequested = true;
            wakeSequence++;
            Monitor.PulseAll(gate);
        }
        _ = WaitForStop(TimeSpan.FromMilliseconds(500));
    }
}
