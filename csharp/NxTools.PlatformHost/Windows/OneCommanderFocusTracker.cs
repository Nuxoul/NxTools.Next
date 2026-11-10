using System.Runtime.InteropServices;
using System.Windows.Automation;

namespace NxTools.PlatformHost.Windows;

// UIA/native effects are supplied by the adapter; fixtures exercise this same owner without a desktop.
internal sealed record OneCommanderFocusOperations<TElement>(
    Func<TElement, long> ResolveWindow,
    Func<long, bool> WindowAvailable,
    Func<TElement, bool> ElementAvailable,
    Func<TElement?> FocusedElement,
    Func<Action<TElement>, Action> SubscribeFocus) where TElement : class;

internal sealed class OneCommanderFocusTracker<TElement> : IDisposable where TElement : class
{
    internal const int MaximumWindows = 64;
    internal static readonly TimeSpan DefaultSweepPeriod = TimeSpan.FromSeconds(60);
    private readonly OneCommanderFocusOperations<TElement> operations;
    private readonly object gate = new();
    private readonly Dictionary<long, Anchor> anchors = [];
    private readonly ManualResetEventSlim started = new();
    private readonly ManualResetEventSlim stop = new();
    private readonly Thread thread;
    private long focusSequence;
    private int stopped;
    private int disposed;
    private sealed record Anchor(TElement Element, long Sequence);
    internal TimeSpan SweepPeriod { get; }
    internal int Count { get { lock (gate) return anchors.Count; } }

    internal OneCommanderFocusTracker(OneCommanderFocusOperations<TElement> operations, TimeSpan? sweepPeriod = null)
    {
        this.operations = operations;
        SweepPeriod = sweepPeriod ?? DefaultSweepPeriod;
        thread = new Thread(Run) { IsBackground = true, Name = "NxTools.OneCommanderFocusTracker" };
        thread.SetApartmentState(ApartmentState.MTA);
        thread.Start();
        EnsureStarted();
    }

    internal void EnsureStarted()
    {
        if (Volatile.Read(ref stopped) == 0) started.Wait(TimeSpan.FromMilliseconds(500));
    }

    internal void RememberFocus(TElement element)
    {
        if (Volatile.Read(ref stopped) != 0) return;
        try
        {
            var window = operations.ResolveWindow(element);
            if (window == 0 || !operations.WindowAvailable(window)) return;
            lock (gate)
            {
                if (Volatile.Read(ref stopped) != 0) return;
                anchors[window] = new(element, ++focusSequence);
                if (anchors.Count > MaximumWindows)
                    anchors.Remove(anchors.MinBy(pair => pair.Value.Sequence).Key);
            }
        }
        catch (Exception error) when (Unavailable(error)) { }
    }

    internal TElement? TryGet(long window)
    {
        Anchor? anchor;
        lock (gate)
        {
            if (Volatile.Read(ref stopped) != 0 || !anchors.TryGetValue(window, out anchor)) return null;
        }
        // UIA may block; never hold the cache gate while validating an external element.
        try
        {
            if (operations.WindowAvailable(window) && operations.ElementAvailable(anchor.Element))
                return Volatile.Read(ref stopped) == 0 ? anchor.Element : null;
        }
        catch (Exception error) when (Unavailable(error)) { }
        RetireIfCurrent(window, anchor);
        return null;
    }

    private void RetireIfCurrent(long window, Anchor anchor)
    {
        lock (gate)
        {
            if (anchors.TryGetValue(window, out var current) && ReferenceEquals(current, anchor)) anchors.Remove(window);
        }
    }

    private void SweepUnavailableWindows()
    {
        KeyValuePair<long, Anchor>[] snapshot;
        lock (gate) snapshot = anchors.ToArray();
        foreach (var pair in snapshot)
        {
            if (Volatile.Read(ref stopped) != 0) return;
            try
            {
                // Native HWND/process checks only: idle sweeps never query AutomationElement.
                if (!operations.WindowAvailable(pair.Key)) RetireIfCurrent(pair.Key, pair.Value);
            }
            catch (Exception error) when (Unavailable(error)) { RetireIfCurrent(pair.Key, pair.Value); }
        }
    }

    private void Run()
    {
        Action? unsubscribe = null;
        try
        {
            unsubscribe = operations.SubscribeFocus(RememberFocus);
            try
            {
                if (operations.FocusedElement() is { } focused) RememberFocus(focused);
            }
            catch (Exception error) when (Unavailable(error)) { }
            started.Set();
            while (!stop.Wait(SweepPeriod)) SweepUnavailableWindows();
        }
        catch (ThreadInterruptedException) { }
        catch (Exception error) when (Unavailable(error)) { }
        finally
        {
            Volatile.Write(ref stopped, 1);
            lock (gate) anchors.Clear();
            try { unsubscribe?.Invoke(); }
            catch { /* Focus subscription cleanup cannot replace the owner exit. */ }
            started.Set();
        }
    }

    private static bool Unavailable(Exception error) =>
        error is ElementNotAvailableException or InvalidOperationException or COMException;

    public void Dispose()
    {
        if (Interlocked.Exchange(ref disposed, 1) != 0) return;
        Volatile.Write(ref stopped, 1);
        lock (gate) anchors.Clear();
        stop.Set();
        if (thread.Join(TimeSpan.FromMilliseconds(500)))
        {
            started.Dispose();
            stop.Dispose();
        }
    }
}
