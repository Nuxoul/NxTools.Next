using System.Collections.Concurrent;
using System.Diagnostics;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

// Exact current test apphost, early synthetic mode, GUID temp files, and owned trees only.
internal static class ExtensionProcessTests
{
    public static async Task OwnedEchoAndSuspendedAdmission()
    {
        var events = new ConcurrentQueue<ExtensionProcessEvent>();
        var marker = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid() + ".txt");
        try
        {
            using var runner = new ExtensionProcessRunner(events.Enqueue, admissionProbe: process =>
            {
                Assert(!File.Exists(marker), "fixture executed before Job assignment/resume");
                Assert(ExtensionNative.InJob(process), "suspended fixture must already belong to its Job");
            });
            var request = Request("echo", marker);
            Assert(runner.Start(request), "owned fixture should be admitted");
            await Until(() => events.Any(e => e.Op == "exited"));
            Assert(events.Any(e => e.Op == "stdout" && e.Data == "fixture-ready"), "actual child output missing");
            Assert(events.Last().Success && runner.ActiveCount == 0, "successful run must drain before releasing slot");
        }
        finally { File.Delete(marker); }
    }

    public static async Task OutputBoundsAndStrictUtf8()
    {
        foreach (var (mode, reason) in new[] { ("badutf8", "stdout_invalid_utf8"), ("oversized", "stdout_line_limit"),
            ("stdoutflood", "stdout_total_limit"), ("stderrflood", "stderr_total_limit") })
        {
            var events = new ConcurrentQueue<ExtensionProcessEvent>();
            using var runner = new ExtensionProcessRunner(events.Enqueue);
            Assert(runner.Start(Request(mode)), "fixture admission failed");
            await Until(() => events.Any(e => e.Op == "exited"));
            Assert(events.Last().Reason == reason && !events.Last().Success && runner.ActiveCount == 0,
                mode + " must fail at the real byte pump and drain: " + events.Last().Reason + " active=" + runner.ActiveCount + " elapsed=" + events.Last().ElapsedMs);
        }
    }

    public static async Task TimeoutCancellationAndPipeHoldingTree()
    {
        foreach (var mode in new[] { "sleep", "tree", "pipeholder" })
        {
            var marker = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid() + ".txt");
            var events = new ConcurrentQueue<ExtensionProcessEvent>();
            using var runner = new ExtensionProcessRunner(events.Enqueue);
            var request = Request(mode, marker) with { TimeoutMs = mode == "sleep" ? 300 : 3000 };
            try
            {
                Assert(runner.Start(request), "fixture admission failed");
                if (mode == "tree")
                { await Until(() => File.Exists(marker)); Assert(runner.Cancel(request.RunId), "cancel failed"); }
                await Until(() => events.Any(e => e.Op == "exited"));
                Assert(runner.ActiveCount == 0, "tree and all pumps must drain");
                if (mode == "sleep") Assert(events.Last().Reason == "timeout", "timeout must include child IO");
                if (mode == "tree") Assert(events.Last().Reason == "cancelled", "cancel must be terminal");
                if (mode == "pipeholder") Assert(File.Exists(marker) && events.Last().Success,
                    "fixture must really create a pipe-inheriting child; root exit drains it without timeout");
                if (File.Exists(marker))
                    foreach (var text in File.ReadAllLines(marker)) Assert(Gone(int.Parse(text)), "owned descendant remains alive");
            }
            finally { runner.Stop(); await runner.DrainAsync(); File.Delete(marker); }
        }
    }

    public static async Task AdmissionConcurrencyMemoryAndDuplex()
    {
        var events = new ConcurrentQueue<ExtensionProcessEvent>();
        using var runner = new ExtensionProcessRunner(events.Enqueue);
        var first = Request("sleep") with { MemoryBytes = 1073741824 };
        var second = Request("sleep") with { MemoryBytes = 1073741824 };
        Assert(runner.Start(first) && runner.Start(second), "two jobs at aggregate limit");
        Assert(!runner.Start(Request("sleep")), "aggregate commit budget must reject");
        Assert(!runner.Start(first with { RunId = Guid.NewGuid() }), "one run per plugin");
        runner.Stop(); Assert(await runner.DrainAsync(), "cancelled startup must drain");
        using var next = new ExtensionProcessRunner(events.Enqueue);
        var requests = Enumerable.Range(0, 4).Select(_ => Request("sleep")).ToArray();
        foreach (var request in requests) Assert(next.Start(request), "four fixed slots");
        var clock = Stopwatch.StartNew();
        Assert(!next.Start(Request("sleep")), "fifth run must reject");
        Assert(next.Cancel(requests[0].RunId) && clock.ElapsedMilliseconds < 500, "control must not wait for IO");
        next.Stop(); Assert(await next.DrainAsync(), "all slots must drain");
        using var duplex = new ExtensionProcessRunner(events.Enqueue);
        var call = Request("duplex"); Assert(duplex.Start(call), "duplex start");
        await Until(() => events.Any(e => e.RunId == call.RunId && e.Data == "host-call"));
        Assert(duplex.Write(call.RunId, "host-result"), "host-call reply write");
        await Until(() => events.Any(e => e.RunId == call.RunId && e.Op == "exited"));
        Assert(events.Any(e => e.RunId == call.RunId && e.Data == "host-result"), "real reply must reach child");
    }

    public static async Task FailureBeforeResumeAndOrdinaryToken()
    {
        var marker = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid() + ".txt");
        var events = new ConcurrentQueue<ExtensionProcessEvent>();
        try
        {
            using var runner = new ExtensionProcessRunner(events.Enqueue, admissionProbe: _ => throw new ExtensionFailure("fixture_admission_denied", 5));
            Assert(runner.Start(Request("echo", marker)), "admitted native fixture");
            await Until(() => events.Any(e => e.Op == "exited"));
            Assert(!File.Exists(marker) && events.Last().NativeErrorCode == 5 && runner.ActiveCount == 0,
                "failed native admission must terminate suspended process without running marker");
            using var tokenRunner = new ExtensionProcessRunner(events.Enqueue);
            var token = Request("token"); tokenRunner.Start(token);
            await Until(() => events.Any(e => e.RunId == token.RunId && e.Op == "exited"));
            Assert(events.Any(e => e.RunId == token.RunId && e.Data == "8192:False"), "actual child must have ordinary medium token");
            var missing = Request("sleep") with { Executable = marker + ".exe" };
            tokenRunner.Start(missing);
            await Until(() => events.Any(e => e.RunId == missing.RunId && e.Op == "exited"));
            Assert(events.Last().Reason == "create_process" && events.Last().NativeErrorCode != 0, "missing executable returns fixed native failure");
        }
        finally { File.Delete(marker); }
    }

    public static async Task InputBoundsAndNativeProcessLimit()
    {
        var events = new ConcurrentQueue<ExtensionProcessEvent>();
        using var runner = new ExtensionProcessRunner(events.Enqueue);
        Assert(!runner.Start(Request("sleep") with { TimeoutMs = 120001 }), "hard timeout cap");
        Assert(!runner.Start(Request("sleep") with { MaxProcesses = 9 }), "hard process cap");
        Assert(!runner.Start(Request("sleep") with { MemoryBytes = 1073741825 }), "hard memory cap");
        var limits = Request("joblimits"); runner.Start(limits);
        await Until(() => events.Any(e => e.RunId == limits.RunId && e.Op == "exited"));
        Assert(events.Any(e => e.RunId == limits.RunId && e.Data == "8712:536870912:8"),
            "actual child Job must use aggregate (not per-process) commit, process cap, kill-on-close and no breakaway");
        Assert(!runner.Start(limits), "terminal run-ID replay must reject");
        var consume = Request("consume"); runner.Start(consume);
        await Until(() => events.Any(e => e.RunId == consume.RunId && e.Op == "started"));
        for (var i = 0; i < 4; i++) { runner.Write(consume.RunId, new string('x', 240000)); await Task.Delay(100); }
        Assert(!runner.Write(consume.RunId, new string('x', 240000)), "input total budget");
        await Until(() => events.Any(e => e.RunId == consume.RunId && e.Op == "exited"));
        Assert(events.Last().Reason == "input_total_limit", "stdin must stay finite");
        var marker = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid() + ".txt");
        try
        {
            var request = Request("processlimit", marker) with { MaxProcesses = 2, TimeoutMs = 500 };
            runner.Start(request);
            await Until(() => events.Any(e => e.RunId == request.RunId && e.Op == "exited"));
            Assert(File.ReadAllLines(marker).Length == 1, "native Job active-process cap must deny excess descendants");
            Assert(File.ReadAllLines(marker).All(text => Gone(int.Parse(text))), "limited descendants must drain");
        }
        finally { runner.Stop(); await runner.DrainAsync(); File.Delete(marker); }
    }

    public static async Task StuckAdmissionRetainsBoundedSlotAndPrivacy()
    {
        using var release = new ManualResetEventSlim();
        var entered = 0;
        var events = new ConcurrentQueue<ExtensionProcessEvent>();
        var diagnostics = new ExtensionFixtureLog();
        using var runner = new ExtensionProcessRunner(events.Enqueue, diagnostics, _ =>
        { Interlocked.Increment(ref entered); release.Wait(); });
        var runs = Enumerable.Range(0, 4).Select(_ => Request("sleep")).ToArray();
        try
        {
            foreach (var run in runs) Assert(runner.Start(run), "admit fixed native startup slot");
            await Until(() => Volatile.Read(ref entered) == 4);
            foreach (var run in runs) runner.Cancel(run.RunId);
            Assert(!await runner.DrainAsync(100), "blocked native startup must not release slots");
            Assert(!runner.Start(Request("sleep")) && runner.ActiveCount == 4, "cancel must not create unlimited replacements");
        }
        finally { release.Set(); runner.Stop(); Assert(await runner.DrainAsync(), "released owned suspended children must terminate"); }
        Assert(events.Count(e => e.Op == "exited") == 4 && !events.Any(e => e.Op == "started"), "cancelled suspended callbacks must never resume");
        Assert(diagnostics.Entries.All(e => !e.Contains(AppContext.BaseDirectory) && !e.Contains("invoke") && !e.Contains("sleep")),
            "native lifecycle logs must never contain path, argv, stdin or child output");
        Assert(diagnostics.Entries.Any(e => e.Contains("extension.run.accepted")) && diagnostics.Entries.Any(e => e.Contains("extension.run.cancelled")),
            "correlated accepted and terminal milestones are required");
    }

    private sealed class ExtensionFixtureLog : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Entries = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { Entries.Enqueue(eventName + " " + correlationId + " " + System.Text.Json.JsonSerializer.Serialize(fields)); }
    }

    internal static bool Gone(int pid)
    { try { using var process = Process.GetProcessById(pid); return process.HasExited; } catch (ArgumentException) { return true; } }

    internal static ExtensionStartRequest Request(string mode, params string[] args) => new()
    {
        RunId = Guid.NewGuid(), PluginId = Guid.NewGuid().ToString(), Executable = Environment.ProcessPath!,
        Arguments = new[] { "--extension-fixture-child", mode }.Concat(args).ToArray(),
        WorkingDirectory = AppContext.BaseDirectory, TimeoutMs = 3000, Stdin = "invoke"
    };

    internal static async Task Until(Func<bool> condition, int timeout = 10000)
    {
        var clock = Stopwatch.StartNew();
        while (!condition())
        {
            if (clock.ElapsedMilliseconds > timeout) throw new InvalidOperationException("owned fixture deadline exceeded");
            await Task.Delay(20);
        }
    }
    internal static void Assert(bool value, string message)
    { if (!value) throw new InvalidOperationException(message); }
}
