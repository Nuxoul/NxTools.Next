using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Extensions;

// Four admitted slots include native startup, all three independent pumps, and cleanup.
// A stuck startup/pump/job-empty query retains its slot and handles; no replacement workers.
internal sealed class ExtensionProcessRunner : IDisposable
{
    private readonly object gate = new();
    private readonly Dictionary<Guid, Run> active = [];
    private readonly Action<ExtensionProcessEvent> publish;
    private readonly IPlatformLog? log;
    private readonly Action<ExtensionKernelHandle>? admissionProbe;
    private readonly int outputLimit;
    private readonly bool closeInitialInput;
    private readonly Func<ExtensionProcessEvent, Task>? publishData;
    private bool stopped;
    private readonly HashSet<Guid> retired = [];
    private readonly Queue<Guid> retiredOrder = new(); // Prevent bounded recent run-ID ABA reuse.
    internal int ActiveCount { get { lock (gate) return active.Count; } }

    internal ExtensionProcessRunner(Action<ExtensionProcessEvent> publish, IPlatformLog? log = null,
        Action<ExtensionKernelHandle>? admissionProbe = null, int outputLimit = ExtensionProcessProtocol.StdoutLimit,
        bool closeInitialInput = false, Func<ExtensionProcessEvent, Task>? publishData = null)
    {
        if (outputLimit is < 1 or > ExtensionProcessProtocol.StdoutLimit) throw new ArgumentOutOfRangeException(nameof(outputLimit));
        this.publish = publish; this.log = log; this.admissionProbe = admissionProbe;
        this.outputLimit = outputLimit; this.closeInitialInput = closeInitialInput;
        this.publishData = publishData;
    }

    internal bool Start(ExtensionStartRequest request)
    {
        string? rejection = null;
        try { ExtensionProcessProtocol.Validate(request); }
        catch (ExtensionFailure e) { rejection = e.Reason; }
        Run? run = null;
        lock (gate)
        {
            if (rejection is null)
            {
                if (stopped) rejection = "broker_stopping";
                else if (active.ContainsKey(request.RunId) || retired.Contains(request.RunId)) rejection = "duplicate_run";
                else if (active.Values.Any(r => r.Request.PluginId == request.PluginId)) rejection = "plugin_busy";
                else if (active.Count == 4) rejection = "concurrency_limit";
                else if (active.Values.Sum(r => r.Request.MemoryBytes) + request.MemoryBytes > 2147483648L)
                    rejection = "aggregate_memory_limit";
                else { run = new Run(request); active.Add(request.RunId, run); }
            }
        }
        if (run is null) { Emit(new("rejected", request.RunId, false, rejection)); return false; }
        Diagnostic("accepted", run);
        run.Work = Task.Run(() => ExecuteAsync(run));
        return true;
    }

    internal bool Write(Guid id, string data)
    {
        byte[] bytes;
        try { bytes = ExtensionProcessProtocol.InputFrame(data); }
        catch (ExtensionFailure e) { Emit(new("rejected", id, false, e.Reason)); return false; }
        string? reason = null;
        lock (gate)
        {
            if (!active.TryGetValue(id, out var run)) reason = "unknown_run";
            else if (run.Failure is not null) reason = "run_stopping";
            else if (run.InputBytes + bytes.Length > ExtensionProcessProtocol.InputBudget(run.Request))
            { reason = "input_total_limit"; run.Fail(reason); }
            else if (!run.Input.Writer.TryWrite(bytes)) { reason = "input_queue_limit"; run.Fail(reason); }
            else run.InputBytes += bytes.Length;
        }
        if (reason is null) return true;
        Emit(new("rejected", id, false, reason)); return false;
    }

    internal bool Cancel(Guid id)
    {
        lock (gate)
        {
            if (active.TryGetValue(id, out var run)) { run.Fail("cancelled"); return true; }
        }
        Emit(new("rejected", id, false, "unknown_run")); return false;
    }
    internal void Stop(string reason = "broker_disconnected")
    {
        lock (gate) { stopped = true; foreach (var run in active.Values) run.Fail(reason); }
    }
    internal async Task<bool> DrainAsync(int milliseconds = 5000)
    {
        var clock = Stopwatch.StartNew();
        while (ActiveCount != 0 && clock.ElapsedMilliseconds < milliseconds) await Task.Delay(20);
        return ActiveCount == 0;
    }
    public void Dispose() => Stop(); // Worker owns disposal; caller cannot invalidate leased handles.

    private sealed class Run
    {
        internal readonly ExtensionStartRequest Request;
        internal readonly Stopwatch Clock = Stopwatch.StartNew();
        internal readonly Channel<byte[]> Input = Channel.CreateBounded<byte[]>(new BoundedChannelOptions(8)
            { FullMode = BoundedChannelFullMode.Wait, SingleReader = true });
        internal readonly CancellationTokenSource InputStop = new();
        internal ExtensionFailure? Failure;
        internal int InputBytes;
        internal int OutputBytes;
        internal Task? Work;
        internal readonly List<IDisposable> Resources = [];
        internal int Terminal;
        internal Run(ExtensionStartRequest request)
        {
            Request = request;
            var input = ExtensionProcessProtocol.InputFrame(request.Stdin);
            InputBytes = input.Length; Input.Writer.TryWrite(input);
        }
        internal void Fail(string reason, int? native = null) =>
            Interlocked.CompareExchange(ref Failure, new ExtensionFailure(reason, native), null);
    }

    private async Task ExecuteAsync(Run run)
    {
        ExtensionKernelHandle? job = null, process = null;
        FileStream? input = null, output = null, error = null;
        Task outPump = Task.CompletedTask, errPump = Task.CompletedTask, inPump = Task.CompletedTask;
        uint? exit = null;
        var drained = false;
        var assigned = false;
        try
        {
            ExtensionNative.Ordinary(ExtensionNative.CurrentProcess());
            if (run.Failure is not null) throw run.Failure;
            job = ExtensionNative.MakeJob(run.Request.MemoryBytes, run.Request.MaxProcesses);
            run.Resources.Add(job);
            var stdinPipe = ExtensionNative.Pipe(false);
            using var childInput = stdinPipe.Child;
            var ourInput = stdinPipe.Ours; run.Resources.Add(ourInput);
            var stdoutPipe = ExtensionNative.Pipe(true);
            using var childOutput = stdoutPipe.Child;
            var ourOutput = stdoutPipe.Ours; run.Resources.Add(ourOutput);
            var stderrPipe = ExtensionNative.Pipe(true);
            using var childError = stderrPipe.Child;
            var ourError = stderrPipe.Ours; run.Resources.Add(ourError);
            var child = ExtensionNative.Spawn(run.Request, childInput, childOutput, childError);
            process = child.Process;
            run.Resources.Add(process);
            using var thread = child.Thread;
            // Any exception from assignment/token/probe/resume enters owned suspended termination.
            ExtensionNative.Require(ExtensionNative.AssignJob(job, process.DangerousGetHandle()), "assign_job");
            assigned = true;
            ExtensionNative.Ordinary(process.DangerousGetHandle());
            admissionProbe?.Invoke(process);
            if (run.Clock.ElapsedMilliseconds >= run.Request.TimeoutMs) run.Fail("timeout");
            if (run.Failure is not null) throw run.Failure;
            input = new FileStream(ourInput, FileAccess.Write, 8192, false);
            run.Resources.Add(input);
            output = new FileStream(ourOutput, FileAccess.Read, 8192, false);
            run.Resources.Add(output);
            error = new FileStream(ourError, FileAccess.Read, 8192, false);
            run.Resources.Add(error);
            if (ExtensionNative.Resume(thread) == uint.MaxValue)
                throw new ExtensionFailure("resume_thread", Marshal.GetLastWin32Error());
            childInput.Dispose(); childOutput.Dispose(); childError.Dispose();
            Emit(new("started", run.Request.RunId, ProcessId: child.Pid, ElapsedMs: run.Clock.ElapsedMilliseconds));
            Diagnostic("started", run);
            outPump = Task.Run(() => ReadAsync(run, output, "stdout",
                outputLimit == ExtensionProcessProtocol.StdoutLimit && !closeInitialInput
                    ? ExtensionProcessProtocol.StdoutBudget(run.Request) : outputLimit));
            errPump = Task.Run(() => ReadAsync(run, error, "stderr", Math.Min(outputLimit, ExtensionProcessProtocol.StderrLimit)));
            inPump = Task.Run(() => WriteAsync(run, input));
            if (closeInitialInput) run.Input.Writer.TryComplete();
            while (run.Failure is null)
            {
                var wait = ExtensionNative.Wait(process, 0);
                if (wait == 0) break;
                if (wait == uint.MaxValue) throw new ExtensionFailure("wait_process", Marshal.GetLastWin32Error());
                if (run.Clock.ElapsedMilliseconds >= run.Request.TimeoutMs) run.Fail("timeout");
                await Task.Delay(20);
            }
        }
        catch (ExtensionFailure e) { run.Fail(e.Reason, e.NativeCode); }
        catch (Exception) { run.Fail("runner_failure"); }
        finally
        {
            run.Input.Writer.TryComplete(); run.InputStop.Cancel();
            // Always kill remaining descendants on root exit, including pipe-holding children.
            // Failed assignment still owns a suspended process handle: terminate it explicitly.
            try
            {
                if (job is not null) ExtensionNative.Require(ExtensionNative.TerminateJob(job, 1), "terminate_job");
                if (!assigned && process is not null && ExtensionNative.Wait(process, 0) != 0)
                    ExtensionNative.Require(ExtensionNative.TerminateProcess(process, 1), "terminate_process");
                var cleanupClock = Stopwatch.StartNew();
                var pumps = Task.WhenAll(outPump, errPump, inPump);
                while (cleanupClock.ElapsedMilliseconds < 5000)
                {
                    if (run.Clock.ElapsedMilliseconds >= run.Request.TimeoutMs) run.Fail("timeout");
                    var empty = job is null || ExtensionNative.JobEmpty(job);
                    var rootGone = process is null || ExtensionNative.Wait(process, 0) == 0;
                    if (empty && rootGone && pumps.IsCompleted) { await pumps; drained = true; break; }
                    await Task.Delay(20);
                }
                if (drained && process is not null)
                {
                    ExtensionNative.Require(ExtensionNative.ExitCode(process, out var code), "exit_code"); exit = code;
                    if (code != 0) run.Fail("process_exit");
                }
            }
            catch (ExtensionFailure e) { run.Fail(e.Reason, e.NativeCode); }
            catch (Exception) { run.Fail("cleanup_failure"); }
            if (drained)
            {
                foreach (var resource in run.Resources) resource.Dispose();
                run.Resources.Clear(); run.InputStop.Dispose();
                lock (gate)
                {
                    active.Remove(run.Request.RunId); retired.Add(run.Request.RunId); retiredOrder.Enqueue(run.Request.RunId);
                    if (retiredOrder.Count > 4096) retired.Remove(retiredOrder.Dequeue());
                }
            }
            else
            {
                run.Fail("cleanup_stuck");
                // Keep exact resources rooted in the occupied slot, even after this callback returns.
                // run.Resources stays reachable from active; no new task/thread for stuck cleanup.
            }
            Interlocked.Exchange(ref run.Terminal, 1);
            var failure = run.Failure;
            Emit(new("exited", run.Request.RunId, failure is null && drained, failure?.Reason,
                failure?.NativeCode, exit, ElapsedMs: run.Clock.ElapsedMilliseconds, CleanupComplete: drained));
            Diagnostic(failure is null ? "completed" : failure.Reason == "cancelled" ? "cancelled" : "failed", run);
        }
    }

    private async Task ReadAsync(Run run, Stream stream, string op, int limit)
    {
        try
        {
            var reader = new ExtensionLineReader(stream, limit, op, count =>
            {
                // The scoped runtime probe shares one 4KiB budget across both pipes.
                if (closeInitialInput && Interlocked.Add(ref run.OutputBytes, count) > outputLimit)
                    throw new ExtensionFailure("output_total_limit");
            });
            while (await reader.ReadAsync() is { } line)
            {
                if (Volatile.Read(ref run.Terminal) != 0) break;
                var value = new ExtensionProcessEvent(op, run.Request.RunId, Data: line);
                if (op == "stdout" && run.Request.DataPlane == 2 && publishData is not null) await publishData(value);
                else Emit(value);
            }
        }
        catch (ExtensionFailure e) { run.Fail(e.Reason, e.NativeCode); }
        catch (Exception) { run.Fail(op + "_io_failure"); }
    }
    private async Task WriteAsync(Run run, Stream stream)
    {
        try
        {
            await foreach (var bytes in run.Input.Reader.ReadAllAsync(run.InputStop.Token))
            {
                // A scoped version probe has no stdin payload. Its validated empty JSONL
                // seed means EOF, avoiding a racing write to an interpreter that ignores stdin.
                if (closeInitialInput && bytes.Length == 1 && bytes[0] == 10) continue;
                await stream.WriteAsync(bytes, run.InputStop.Token);
            }
            if (closeInitialInput) stream.Dispose(); // Version probes never await a plugin reply.
        }
        catch (OperationCanceledException) when (run.InputStop.IsCancellationRequested) { }
        catch (Exception) { run.Fail("stdin_io_failure"); }
    }
    private void Emit(ExtensionProcessEvent value)
    {
        try { publish(value); }
        catch (Exception) { Stop("output_transport_failure"); }
    }
    private void Diagnostic(string phase, Run run)
    {
        try
        {
            log?.Write("info", "extension.run." + phase, run.Request.RunId, new Dictionary<string, object?>
            {
                ["elapsed_ms"] = run.Clock.ElapsedMilliseconds, ["reason"] = run.Failure?.Reason,
                ["native_error_code"] = run.Failure?.NativeCode
            });
        }
        catch (Exception) { /* Diagnostics never change the observed operation. */ }
    }
}
