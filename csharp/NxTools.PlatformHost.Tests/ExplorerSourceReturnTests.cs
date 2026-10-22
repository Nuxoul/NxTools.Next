using System.Collections.Concurrent;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// All identities/paths/capture/foreground samples are synthetic. No native handle is created.
internal static class ExplorerSourceReturnTests
{
    private sealed class Fixture : IDisposable
    {
        internal readonly Channel<WindowsPlatformEvent> Events = Channel.CreateUnbounded<WindowsPlatformEvent>();
        internal readonly ConcurrentQueue<Action> Work = new();
        internal readonly ConcurrentQueue<Action> Delivery = new();
        internal readonly Log Log = new();
        internal readonly WindowsPlatformAdapter.EventWindow Window;
        internal readonly ManualResetEventSlim Entered = new(false);
        internal readonly ManualResetEventSlim Release = new(false);
        internal int Calls;
        internal int BusyRejects;
        private int _occupied;
        internal long Foreground = 20;
        internal bool Fail = false;
        internal int BlockCall = 1;
        internal bool ThrowCapture = false;
        internal Fixture(int? eventCapacity = null)
        {
            if (eventCapacity is { } capacity)
                Events = Channel.CreateBounded<WindowsPlatformEvent>(capacity);
            Window = new(Events.Writer, Log, Capture, Work.Enqueue, Delivery.Enqueue, hwnd => hwnd == Foreground);
        }
        public void Dispose() { Release.Set(); Entered.Dispose(); Release.Dispose(); }
        private string? Capture(long hwnd)
        {
            Interlocked.Increment(ref Calls);
            if (Interlocked.Exchange(ref _occupied, 1) != 0)
            {
                Interlocked.Increment(ref BusyRejects);
                return null;
            }
            try
            {
                if (ThrowCapture) throw new IOException("C:/private/payload-must-not-be-logged");
                if (Calls == BlockCall)
                {
                    Entered.Set();
                    if (!Release.Wait(TimeSpan.FromSeconds(5))) throw new TimeoutException("synthetic capture barrier");
                    return Fail ? null : "C:/synthetic/activation";
                }
                return Fail ? null : $"C:/synthetic/fresh-{hwnd}";
            }
            finally { Volatile.Write(ref _occupied, 0); }
        }
        internal void KnownDialog() => Window.ProcessRecognizedFileDialog(new(20, "C:/synthetic/dialog"));
        internal void Explorer(long hwnd = 10) => Window.ObserveExplorerDirectorySource(new("explorer", hwnd, null), "explorer");
        internal async Task<Task> StartCapture()
        {
            Assert(Work.TryDequeue(out var work), "activation queues capture");
            var running = Task.Run(work!);
            await Task.Run(() => Assert(Entered.Wait(TimeSpan.FromSeconds(5)), "capture entered barrier"));
            return running;
        }
        internal void Pump()
        {
            for (var i = 0; i < 20; i++)
            {
                var advanced = false;
                while (Delivery.TryDequeue(out var delivery)) { delivery(); advanced = true; }
                while (Work.TryDequeue(out var work)) { work(); advanced = true; }
                if (!advanced) return;
            }
            throw new InvalidOperationException("unbounded caller scheduling");
        }
        internal QuickLaunchAutoSwitchRequestedEvent[] Requests()
        {
            var requests = new List<QuickLaunchAutoSwitchRequestedEvent>();
            while (Events.Reader.TryRead(out var value))
                if (value is QuickLaunchAutoSwitchRequestedEvent request) requests.Add(request);
            return requests.ToArray();
        }
    }

    internal sealed class Log : IPlatformLog
    {
        internal readonly List<(string Name, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Entries = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { lock (Entries) Entries.Add((eventName, correlationId, fields ?? new Dictionary<string, object?>())); }
    }

    public static async Task InFlightKnownReturnCompletesOnce()
    {
        using var f = new Fixture();
        f.KnownDialog(); f.Explorer();
        var running = await f.StartCapture();
        try
        {
            f.KnownDialog();
            // The old caller starts another read while the key is occupied: deterministic busy rejection.
            while (f.Work.TryDequeue(out var work)) work();
        }
        finally { f.Release.Set(); await running.WaitAsync(TimeSpan.FromSeconds(5)); }
        f.Pump();
        var requests = f.Requests();
        Assert(requests.Length == 1, "known return must emit one switch after held activation capture completes");
        Assert(f.BusyRejects == 0, "return cannot duplicate an occupied capture");
        Assert(f.Calls == 2, "return obtains one serialized fresh read, not a stale activation snapshot");
        Assert(requests[0].Target.WindowHandle == 20 && requests[0].SourceDirectory == "C:/synthetic/fresh-10", "frozen target and fresh source");
        var accepted = f.Log.Entries.Single(entry => entry.Name.EndsWith("accepted") && Equals(entry.Fields["outcome"], "return"));
        var lifecycle = f.Log.Entries.Where(entry => entry.Correlation == accepted.Correlation).ToArray();
        Assert(accepted.Correlation is not null && lifecycle.Select(entry => entry.Name).SequenceEqual(new[]
        {
            "quick_launch.explorer_source_accepted", "quick_launch.explorer_source_start",
            "quick_launch.explorer_source_ready", "quick_launch.explorer_source_terminal"
        }), "one correlated return accepted/start/source-ready/terminal lifecycle");
        Assert(lifecycle.All(entry => entry.Fields.ContainsKey("elapsed_ms") && entry.Fields.Count == 3), "bounded elapsed and nonpayload fields");
        f.KnownDialog(); f.Pump();
        Assert(f.Requests().Length == 0, "same-generation duplicate dialog wake cannot navigate twice");
    }

    public static Task ReadySourceStillRefreshesAtReturn()
    {
        using var f = new Fixture();
        f.Release.Set(); f.KnownDialog(); f.Explorer(); f.Pump();
        f.KnownDialog(); f.Pump();
        var requests = f.Requests();
        Assert(f.Calls == 2 && requests.Length == 1 && requests[0].SourceDirectory == "C:/synthetic/fresh-10",
            "completed activation cannot be reused as stale return source");
        return Task.CompletedTask;
    }

    public static async Task FirstObservationNeverNavigatesHistory()
    {
        using var f = new Fixture();
        f.Explorer(); var running = await f.StartCapture();
        try { f.KnownDialog(); }
        finally { f.Release.Set(); await running.WaitAsync(TimeSpan.FromSeconds(5)); }
        f.Pump(); f.KnownDialog(); f.Pump();
        Assert(f.Calls == 1 && f.Requests().Length == 0, "new dialog cannot consume historical source on completion/repeated wake");
    }

    public static async Task RetiredReturnsDropLateCompletion()
    {
        foreach (var stage in new[] { "activation", "return" })
        foreach (var retire in new[] { "cancel", "stop", "source_destroy", "target_destroy", "foreground", "target_replace" })
        {
            using var f = new Fixture();
            f.KnownDialog(); f.Explorer();
            if (stage == "return")
            {
                f.Release.Set(); f.Pump();
                f.Release.Reset(); f.Entered.Reset(); f.BlockCall = 2;
                f.KnownDialog();
            }
            var running = await f.StartCapture();
            try
            {
                f.KnownDialog();
                switch (retire)
                {
                    case "cancel": f.Window.CancelExplorerDirectoryCapture("cancelled"); break;
                    case "stop": f.Window.CancelExplorerDirectoryCapture("stopped", stop: true); break;
                    case "source_destroy": f.Window.ExplorerCaptureWindowUnavailable(10); break;
                    case "target_destroy": f.Window.ExplorerCaptureWindowUnavailable(20); break;
                    case "foreground": f.Foreground = 99; break;
                    case "target_replace": f.Window.ProcessRecognizedFileDialog(new(21, null)); break;
                }
            }
            finally { f.Release.Set(); await running.WaitAsync(TimeSpan.FromSeconds(5)); }
            f.Pump();
            Assert(f.Requests().Length == 0 && f.Calls == (stage == "activation" ? 1 : 2) && f.BusyRejects == 0,
                $"{stage}/{retire} retires intent without stale navigation or extra read");
            var returnAccepted = f.Log.Entries.Single(entry => entry.Name.EndsWith("accepted") && Equals(entry.Fields["outcome"], "return"));
            Assert(f.Log.Entries.Count(entry => entry.Correlation == returnAccepted.Correlation && entry.Name.EndsWith("terminal")) == 1,
                "cancelled return emits exactly one terminal even after late completion");
        }
    }

    public static async Task SourceSupersessionSerializesAndRejectsSameHwndAba()
    {
        foreach (var nextHwnd in new long[] { 10, 11 })
        {
            using var f = new Fixture();
            f.KnownDialog(); f.Explorer(); var running = await f.StartCapture();
            try
            {
                f.KnownDialog(); f.Explorer(nextHwnd); f.KnownDialog();
                Assert(f.Work.IsEmpty, "superseded source must coalesce behind occupied capture");
            }
            finally { f.Release.Set(); await running.WaitAsync(TimeSpan.FromSeconds(5)); }
            f.Pump();
            var requests = f.Requests();
            Assert(f.Calls == 2 && f.BusyRejects == 0 && requests.Length == 1 &&
                requests[0].SourceDirectory == $"C:/synthetic/fresh-{nextHwnd}", "only latest source epoch can publish, including same-HWND ABA");
            Assert(f.Log.Entries.Any(entry => Equals(entry.Fields["outcome"], "stale")), "old epoch has explicit stale terminal");
        }
    }

    public static Task FailuresAndFullQueueRetireWithoutPayload()
    {
        foreach (var failure in new[] { "empty", "exception", "queue_full" })
        {
            using var f = new Fixture(failure == "queue_full" ? 1 : null);
            f.Release.Set(); f.KnownDialog(); f.Explorer(); f.Pump();
            if (failure == "empty") f.Fail = true;
            if (failure == "exception") f.ThrowCapture = true;
            if (failure == "queue_full") f.Events.Writer.TryWrite(new ForegroundChangedEvent(new(99, 99, 0, "fake", "fake")));
            f.KnownDialog(); f.Pump();
            Assert(f.Requests().Length == 0, "failure never navigates");
            f.Fail = false; f.ThrowCapture = false; f.KnownDialog(); f.Pump();
            Assert(f.Requests().Length == 0 && f.Calls == 2, "failed candidate is terminal, not an unbounded retry");
            var diagnostic = string.Join("|", f.Log.Entries.SelectMany(entry => entry.Fields.Values));
            Assert(!diagnostic.Contains("C:/") && !diagnostic.Contains("private"), "no paths/raw exception payload in diagnostics");
            var outcome = failure switch { "empty" => "capture_failed", "exception" => "IOException", _ => "event_queue_full" };
            Assert(f.Log.Entries.Any(entry => Equals(entry.Fields["outcome"], outcome)), "failure has fixed terminal category");
        }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    { if (!condition) throw new InvalidOperationException(message); }
}
