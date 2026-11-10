using System.Collections.Concurrent;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class OneCommanderFocusTrackerTests
{
    private sealed class Element(long window, string pane)
    {
        internal long Window { get; } = window;
        internal string Pane { get; } = pane;
        internal bool Available = true;
    }

    private sealed class Platform(Element? initial = null)
    {
        internal readonly ConcurrentDictionary<long, bool> Windows = new();
        internal Action<Element>? Callback, LastCallback;
        internal Func<Element, long>? Resolve;
        internal Func<Element, bool>? Validate;
        internal int Subscriptions, Unsubscriptions, Samples, WindowChecks, ElementChecks;
        internal OneCommanderFocusOperations<Element> Operations => new(
            element => Resolve?.Invoke(element) ?? element.Window,
            window => { Interlocked.Increment(ref WindowChecks); return Windows.TryGetValue(window, out var live) && live; },
            element => { Interlocked.Increment(ref ElementChecks); return Validate?.Invoke(element) ?? element.Available; },
            () => { Interlocked.Increment(ref Samples); return initial; },
            remember =>
            {
                LastCallback = remember;
                Volatile.Write(ref Callback, remember);
                Interlocked.Increment(ref Subscriptions);
                return () => { Volatile.Write(ref Callback, null); Interlocked.Increment(ref Unsubscriptions); };
            });
        internal void Focus(Element element)
        {
            Windows[element.Window] = true;
            Volatile.Read(ref Callback)?.Invoke(element);
        }
    }

    private sealed class Fixture : IDisposable
    {
        internal readonly Platform Platform;
        internal readonly OneCommanderFocusTracker<Element> Tracker;
        internal Fixture(Element? initial = null, bool periodic = true)
        {
            Platform = new(initial);
            if (initial is not null) Platform.Windows[initial.Window] = true;
            Tracker = new(Platform.Operations, periodic ? TimeSpan.FromMilliseconds(20) : TimeSpan.FromHours(1));
        }
        public void Dispose() => Tracker.Dispose();
    }

    public static Task SampleRememberAndReplace()
    {
        Assert(OneCommanderFocusTracker<Element>.DefaultSweepPeriod == TimeSpan.FromSeconds(60), "production uses only a low-frequency native-window sweep");
        var first = new Element(10, "left");
        using var f = new Fixture(first);
        f.Tracker.EnsureStarted(); f.Tracker.EnsureStarted();
        Assert(f.Platform.Subscriptions == 1 && f.Platform.Samples == 1 && ReferenceEquals(f.Tracker.TryGet(10), first),
            "one owner subscribes and samples current focus exactly once");
        var frozen = f.Tracker.TryGet(10);
        var replacement = new Element(10, "right");
        f.Platform.Focus(replacement);
        f.Platform.Focus(new(20, "other"));
        Assert(f.Tracker.Count == 2 && ReferenceEquals(f.Tracker.TryGet(10), replacement), "same window keeps only its latest focus");
        Assert(ReferenceEquals(frozen, first) && frozen?.Pane == "left", "a previously frozen pane remains independent of cache replacement");
        f.Platform.Focus(new(0, "unrelated"));
        Assert(f.Tracker.Count == 2, "unresolved or unrelated HWND never enters the cache");
        return Task.CompletedTask;
    }

    public static Task ClosedWindowRetiresWithoutLookup()
    {
        using var f = new Fixture(new(10, "closed"));
        f.Platform.Focus(new(20, "live"));
        Assert(f.Tracker.SweepPeriod == TimeSpan.FromMilliseconds(20), "fixture advances the same background lifecycle with synthetic native operations");
        var uiReads = f.Platform.ElementChecks;
        f.Platform.Windows[10] = false;
        Assert(SpinWait.SpinUntil(() => f.Tracker.Count == 1, TimeSpan.FromSeconds(1)),
            "closed HWND must retire without another TryGet for that HWND");
        Assert(f.Platform.ElementChecks == uiReads, "idle retirement must not query UIA elements");
        Assert(f.Tracker.TryGet(20)?.Pane == "live", "valid last focus has no age expiry");
        return Task.CompletedTask;
    }

    public static Task CapacityKeepsMostRecentFocus()
    {
        using var f = new Fixture(periodic: false);
        for (var window = 1; window <= 64; window++) f.Platform.Focus(new(window, "pane"));
        var newestFirst = new Element(1, "new first");
        f.Platform.Focus(newestFirst);
        f.Platform.Focus(new(65, "new window"));
        Assert(f.Tracker.Count == 64, "focus cache must remain bounded at 64 windows");
        Assert(f.Tracker.TryGet(2) is null && ReferenceEquals(f.Tracker.TryGet(1), newestFirst) && f.Tracker.TryGet(65) is not null,
            "least recently focused window is evicted, not the recently replaced anchor");
        return Task.CompletedTask;
    }

    public static Task InvalidLookupRetiresRecord()
    {
        using var f = new Fixture(new(10, "invalid native window"), periodic: false);
        f.Platform.Windows[10] = false;
        Assert(f.Tracker.TryGet(10) is null && f.Tracker.Count == 0, "unavailable HWND cannot return a still-readable element wrapper");
        var element = new Element(20, "invalid UIA element");
        f.Platform.Focus(element);
        element.Available = false;
        Assert(f.Tracker.TryGet(20) is null && f.Tracker.Count == 0, "invalid UIA anchor is removed on the bounded query path");
        return Task.CompletedTask;
    }

    public static async Task StaleValidationPreservesReplacement()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var first = new Element(10, "old");
        using var f = new Fixture(first, periodic: false);
        f.Platform.Validate = element =>
        {
            if (!ReferenceEquals(element, first)) return true;
            entered.Set(); Wait(release); return false;
        };
        var read = Task.Run(() => f.Tracker.TryGet(10));
        var latest = new Element(10, "latest");
        try { Wait(entered); f.Platform.Focus(latest); }
        finally { release.Set(); }
        Assert(await read is null, "old invalid query returns no element");
        Assert(ReferenceEquals(f.Tracker.TryGet(10), latest), "stale query cannot erase a newer focus record");
    }

    public static Task WindowReuseAndStop()
    {
        using var f = new Fixture();
        for (var cycle = 0; cycle < 5; cycle++)
        {
            var element = new Element(10, "pane " + cycle);
            f.Platform.Focus(element);
            Assert(ReferenceEquals(f.Tracker.TryGet(10), element), "reused window remembers its new anchor");
            f.Platform.Windows[10] = false;
            Assert(SpinWait.SpinUntil(() => f.Tracker.Count == 0, TimeSpan.FromSeconds(1)), "repeated close retires the anchor without lookup");
        }
        var callback = f.Platform.LastCallback ?? throw new InvalidOperationException("fixture subscription missing");
        f.Tracker.Dispose();
        callback(new(30, "late"));
        Assert(f.Tracker.Count == 0 && f.Platform.Unsubscriptions == 1, "stopped tracker clears records and removes its exact subscription");
        return Task.CompletedTask;
    }

    public static async Task InFlightFocusCannotReviveStoppedOwner()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var f = new Fixture(periodic: false);
        f.Platform.Resolve = element => { entered.Set(); Wait(release); return element.Window; };
        var callback = Task.Run(() => f.Platform.Focus(new(10, "late resolve")));
        try { Wait(entered); f.Tracker.Dispose(); }
        finally { release.Set(); }
        await callback;
        Assert(f.Tracker.Count == 0 && f.Platform.Unsubscriptions == 1, "in-flight focus resolution cannot repopulate a stopped tracker");
    }

    private static void Wait(ManualResetEventSlim gate)
    {
        if (!gate.Wait(TimeSpan.FromSeconds(3))) throw new TimeoutException("fixture tracker boundary");
    }
    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
