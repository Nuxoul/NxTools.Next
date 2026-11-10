using System.ComponentModel;
using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Tests;

// Opt-in test tool only. Original runner/Job/source/output/cancellation owners are unchanged.
internal static class ArchiveResourceProbe
{
    private static void Check(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    internal static object Field(object instance, string name) => instance.GetType().GetField(name, BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public)!.GetValue(instance)!;
    private static object Run(ArchiveWorkerRunner runner, Guid id) => ((System.Collections.IDictionary)Field(runner, "active"))[id]!;
    [DllImport("kernel32.dll", SetLastError = true)] private static extern uint GetProcessId(IntPtr process);
    [StructLayout(LayoutKind.Sequential)] private struct ProcessBasicInformation
    { internal IntPtr Reserved1, Peb, Reserved2, Reserved3, UniqueProcessId, ParentProcessId; }
    [DllImport("ntdll.dll")] private static extern int NtQueryInformationProcess(IntPtr process, int kind,
        out ProcessBasicInformation information, int size, out int returned);
    private static int? ParentPid()
    {
        using var process = Process.GetCurrentProcess();
        return NtQueryInformationProcess(process.Handle, 0, out var info, Marshal.SizeOf<ProcessBasicInformation>(), out _) == 0
            ? checked((int)info.ParentProcessId) : null;
    }
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GetProcessTimes(IntPtr process,
        out long creation, out long exit, out long kernel, out long user);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool QueryFullProcessImageName(IntPtr process, int flags, System.Text.StringBuilder path, ref int length);
    private static string Image(IntPtr handle)
    {
        var image = new System.Text.StringBuilder(32768); var length = image.Capacity;
        if (!QueryFullProcessImageName(handle, 0, image, ref length)) throw new Win32Exception(Marshal.GetLastWin32Error());
        return image.ToString();
    }
    // Native runner handle, not name-based discovery/PID reuse, supplies both identity and image.
    internal static Process Worker(ArchiveWorkerRunner runner, Guid id, string expected)
    {
        lock (Field(runner, "gate")) {
            var resources = (List<IDisposable>)Field(Run(runner, id), "Resources");
            foreach (var resource in resources.OfType<ExtensionKernelHandle>()) {
                var processId = GetProcessId(resource.DangerousGetHandle());
                if (processId == 0) continue;
                Check(string.Equals(Image(resource.DangerousGetHandle()), expected, StringComparison.OrdinalIgnoreCase), "owned worker image mismatch");
                var process = Process.GetProcessById((int)processId);
                try {
                    Check(GetProcessTimes(resource.DangerousGetHandle(), out var originalCreated, out _, out _, out _) &&
                        GetProcessTimes(process.Handle, out var observedCreated, out _, out _, out _) && originalCreated == observedCreated &&
                        GetProcessId(process.Handle) == processId && string.Equals(Image(process.Handle), expected, StringComparison.OrdinalIgnoreCase), "worker handle identity mismatch");
                    return process;
                } catch { process.Dispose(); throw; }
            }
        }
        throw new InvalidOperationException("owned native process handle absent");
    }
    private static bool SourceWritable(string path)
    {
        try { using var source = File.Open(path, FileMode.Open, FileAccess.Write, FileShare.None); return true; }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
    }
    internal sealed record ToolError(string message, int? native_code);
    internal sealed record Identity(int pid, int? parent_pid, string role, string executable, Guid operation_id);
    internal sealed record Cleanup(string status, bool confirmed, bool? native_terminal, bool? worker_exited,
        bool? job_and_pumps_drained, int? active_count, bool? source_writable, bool? helper_removed, bool? partial_absent, bool? output_directory_removed = null);
    internal sealed class Control
    {
        internal readonly string DirectoryPath, RunId;
        internal string State { get; private set; } = "Running";
        internal string? Reason { get; private set; }
        internal bool Requested => Reason is not null;
        internal bool CancelDispatched { get; private set; }
        internal bool? CancelAccepted { get; private set; }
        internal readonly List<ToolError> Errors = [];
        private readonly List<Identity> identities = [];
        private readonly string helper;
        private bool closed;
        internal bool StatusFailed { get; private set; }
        private int invalidIds;
        internal Cleanup CleanupFacts = new("Unknown", false, null, null, null, null, null, null, null);
        internal object Barrier = new { };
        internal Control(string directory, string runId, string helper, Guid operationId)
        {
            Check(Path.IsPathFullyQualified(directory) && Guid.TryParseExact(runId, "D", out _), "absolute control directory and UUID required");
            DirectoryPath = Path.GetFullPath(directory); RunId = runId; this.helper = helper;
            Check(Directory.Exists(DirectoryPath) && !File.Exists(Path.Combine(DirectoryPath, "status.json")) &&
                !File.Exists(Path.Combine(DirectoryPath, "status.tmp")), "fresh control directory required");
            Check((File.GetAttributes(DirectoryPath) & FileAttributes.ReparsePoint) == 0, "control reparse directory refused");
            identities.Add(new(Environment.ProcessId, ParentPid(), "probe_owner", Environment.ProcessPath!, operationId));
        }
        internal void Error(string message, Exception? failure = null)
        {
            if (Errors.Count < 16) Errors.Add(new(message, failure is Win32Exception native ? native.NativeErrorCode : failure is null ? null : failure.HResult & 0xffff));
        }
        internal void AddWorker(Process process, Guid operationId)
        {
            if (identities.Count < 32 && !identities.Any(value => value.role == "actual_worker"))
                identities.Add(new(process.Id, Environment.ProcessId, "actual_worker", helper, operationId));
        }
        internal bool Poll()
        {
            if (closed || Requested) return Requested;
            try {
                var path = Path.Combine(DirectoryPath, "stop.json");
                FileStream input;
                try { input = new(path, FileMode.Open, FileAccess.Read, FileShare.Read | FileShare.Delete); }
                catch (FileNotFoundException) { return false; }
                using (input) {
                    if (input.Length is < 1 or > 16384) throw new InvalidDataException();
                    var bytes = new byte[16385]; var size = 0;
                    while (size < bytes.Length) { var read = input.Read(bytes, size, bytes.Length - size); if (read == 0) break; size += read; }
                    if (size > 16384) throw new InvalidDataException();
                    using var document = JsonDocument.Parse(bytes.AsMemory(0, size), new JsonDocumentOptions { MaxDepth = 8 });
                    var value = document.RootElement;
                    if (value.GetProperty("run_id").GetString() != RunId) { invalidIds++; return false; }
                    var reason = value.GetProperty("reason").GetString();
                    Request(reason is "physical_memory" or "commit_headroom" or "fixture_disk" or "sampler_deadline" or "sampler_failure" or
                        "driver_exception" or "injected_threshold" or "injected_partial_threshold" or "physical_threshold" or "commit_threshold" or
                        "disk_threshold" or "driver_wait_deadline" or "deadline" or "threshold" ? reason : "driver_stop");
                }
            } catch (Exception failure) { Error("control_read_failed", failure); Request("control_read_failed"); }
            return Requested;
        }
        internal void Request(string reason) { if (!closed && !Requested) { Reason = reason; State = "StopRequested"; } }
        internal void Cancel(Action cancel)
        {
            if (!Requested || CancelDispatched || closed) return;
            CancelDispatched = true; State = "Cancelling";
            try { cancel(); } catch (Exception failure) { Error("cancel_dispatch_failed", failure); }
        }
        internal void Accepted(bool accepted) => CancelAccepted = accepted;
        internal void Draining() { if (!closed) State = "Draining"; }
        internal void Finish(Cleanup facts) {
            var actual = facts.confirmed && facts.output_directory_removed != false;
            CleanupFacts = facts with { confirmed = actual, status = actual ? "Confirmed" : "Incomplete" };
            State = actual ? "Cleaned" : "CleanupIncomplete"; closed = true;
        }
        internal object StopResult() => new { requested = Requested, reason = Reason, cancel_dispatched = CancelDispatched,
            cancel_accepted = CancelAccepted, forced = false, ignored_run_ids = invalidIds };
        internal void Publish()
        {
            try {
                var bytes = JsonSerializer.SerializeToUtf8Bytes(new { schema = 2, run_id = RunId, state = State,
                    identities, expected_images = new { actual_worker = helper }, cleanup = CleanupFacts, stop = StopResult(), barrier = Barrier, errors = Errors });
                Check(bytes.Length <= 65536, "bounded tool status");
                var temporary = Path.Combine(DirectoryPath, "status.tmp");
                using (var output = new FileStream(temporary, FileMode.Create, FileAccess.Write, FileShare.None)) { output.Write(bytes); output.Flush(true); }
                File.Move(temporary, Path.Combine(DirectoryPath, "status.json"), true);
            } catch (Exception failure) { StatusFailed = true; Error("control_status_failed", failure); Request("control_status_failed"); }
        }
    }
    // Kept behind a counter seam so deterministic denied/partial-read cases use the real collector.
    internal sealed class Samples : IDisposable
    {
        private readonly Process? process;
        private readonly Func<long> working, privateBytes;
        internal readonly string Role, Path;
        internal readonly int ProcessId;
        internal long BaselineWorking, BaselinePrivate, PeakWorking, PeakPrivate, LastWorking, LastPrivate;
        internal int Count;
        private int attempts, failures;
        private readonly List<ToolError> errors = [];
        internal double MaxIntervalMs;
        private readonly Stopwatch clock = Stopwatch.StartNew();
        private double last;
        internal Samples(Process process, string role)
        {
            this.process = process; Role = role; ProcessId = process.Id; Path = Image(process.Handle);
            working = () => { process.Refresh(); return process.WorkingSet64; }; privateBytes = () => process.PrivateMemorySize64;
            Take();
        }
        internal Samples(Func<long> working, Func<long> privateBytes) { this.working = working; this.privateBytes = privateBytes; Role = "fake"; Path = "fake"; }
        internal void Take()
        {
            if (attempts >= 10000) return;
            attempts++;
            try {
                // Do not commit a partial pair, baseline, timestamp, count or new peak.
                var nextWorking = working(); var nextPrivate = privateBytes();
                var now = clock.Elapsed.TotalMilliseconds;
                if (Count > 0) MaxIntervalMs = Math.Max(MaxIntervalMs, now - last);
                else { BaselineWorking = nextWorking; BaselinePrivate = nextPrivate; }
                LastWorking = nextWorking; LastPrivate = nextPrivate;
                PeakWorking = Math.Max(PeakWorking, nextWorking); PeakPrivate = Math.Max(PeakPrivate, nextPrivate);
                last = now; Count++;
            } catch (Exception failure) when (failure is InvalidOperationException or Win32Exception or NotSupportedException) {
                failures++;
                if (errors.Count < 16) errors.Add(new("memory_counters_unavailable", failure is Win32Exception native ? native.NativeErrorCode : failure.HResult & 0xffff));
            }
        }
        private long? Available(long value) => Count > 0 ? value : null;
        internal object Result() => new { Role, ProcessId, Path, status = Count > 0 ? "Complete" : "Incomplete",
            BaselineWorking = Available(BaselineWorking), BaselinePrivate = Available(BaselinePrivate),
            PeakWorking = Available(PeakWorking), PeakPrivate = Available(PeakPrivate), LastWorking = Available(LastWorking), LastPrivate = Available(LastPrivate),
            Count, attempts, failures, successes = Count, errors, requested_interval_ms = 20,
            MaxIntervalMs = Count > 1 ? MaxIntervalMs : (double?)null, possible_missed_peaks = true };
        public void Dispose() => process?.Dispose();
    }
    internal sealed class DispatchClock(Func<double> now, double? setupOrigin = null)
    {
        private readonly double entered = setupOrigin ?? now();
        private double origin = now();
        internal double? SetupMs { get; private set; }
        internal void Dispatch() { origin = now(); SetupMs = origin - entered; }
        internal double OperationMs => now() - origin;
    }
    // ZIP has no source-data checkpoint. This test-only stream is installed at the existing
    // BeginFile diagnostic boundary, on the original output pump, before its first native write.
    // No alternate output writer, codec, runtime, cancellation or publication path is introduced.
    private sealed class DataBarrier : IPlatformLog, IDisposable
    {
        internal readonly ManualResetEventSlim Reached = new(), Release = new();
        internal Func<ArchiveOutputSession>? Session;
        internal long Bytes;
        private bool installed;
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (eventName != "archive.output.started" || installed) return;
            var session = Session!();
            var entries = (System.Collections.IDictionary)Field(session, "entries");
            var entry = entries.Values.Cast<object>().Single();
            var file = (SafeFileHandle)Field(entry, "File");
            entry.GetType().GetField("Stream", BindingFlags.Instance | BindingFlags.NonPublic)!.SetValue(entry, new BarrierStream(file, this));
            installed = true;
        }
        private sealed class BarrierStream(SafeFileHandle handle, DataBarrier barrier) : FileStream(handle, FileAccess.ReadWrite, 65536, false)
        {
            private bool held;
            public override void Write(ReadOnlySpan<byte> buffer)
            {
                base.Write(buffer); Flush();
                if (held) return;
                held = true; Interlocked.Exchange(ref barrier.Bytes, Length); barrier.Reached.Set();
                if (!barrier.Release.Wait(10000)) throw new IOException("tool data barrier deadline");
            }
        }
        public void Dispose() { Release.Set(); Reached.Dispose(); Release.Dispose(); }
    }
    internal static async Task<int> RunAsync(string scenario, string root)
    {
        var entered = Stopwatch.StartNew();
        try {
            Check(Path.IsPathFullyQualified(root) && File.Exists(System.IO.Path.Combine(root, "owned-resource-fixtures")), "owned fixture marker required");
            var scratch = System.IO.Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!);
            Check(string.Equals(System.IO.Path.GetDirectoryName(System.IO.Path.GetFullPath(root)), scratch, StringComparison.OrdinalIgnoreCase), "session scratch root required");
            Check(scenario is "runner100k" or "cancel1gib" or "cancel1gib-hold" or "tool-stop-small", "unknown probe");
            return await Measure(scenario, root, new DispatchClock(() => entered.Elapsed.TotalMilliseconds, 0));
        } catch (Exception failure) {
            Console.Error.WriteLine("RESOURCE_FAIL " + failure.GetType().Name);
            return 1;
        }
    }
    private static async Task<int> Measure(string scenario, string root, DispatchClock clock)
    {
        var helper = System.IO.Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        var id = Guid.NewGuid();
        var control = new Control(Environment.GetEnvironmentVariable("NXTOOLS_RESOURCE_CONTROL")!, Environment.GetEnvironmentVariable("NXTOOLS_RESOURCE_RUN_ID")!, helper, id);
        Check(string.Equals(System.IO.Path.GetDirectoryName(control.DirectoryPath), System.IO.Path.GetFullPath(root), StringComparison.OrdinalIgnoreCase), "control must belong to fixture root");
        ArchiveWorkerRunner? runner = null; StreamReader? manifest = null;
        Samples? callerSample = null, workerSample = null; Process? actualWorker = null;
        var output = System.IO.Path.Combine(root, "output-" + Guid.NewGuid().ToString("D"));
        var path = System.IO.Path.Combine(root, scenario == "runner100k" ? "100k.zip" : scenario == "tool-stop-small" ? "small.zip" : "1gib.zip");
        var helperOwned = false; var outputOwned = false; var dispatchAttempted = false; var admitted = false;
        var failed = false; var operationOk = false;
        var identityAttempts = 0; var identityFailures = 0;
        Exception? callbackFailure = null; ArchiveWorkerEvent? terminal = null;
        long count = 0, partialBytes = 0;
        double? firstBatch = null, cancelAt = null, processExitAt = null, cleanupAt = null, terminalAt = null, workerCancelledAt = null;
        string? observedWorkerFailure = null;
        var heldSlot = false; var heldSource = false; var heldJobEmpty = false;
        using var cleanupReached = new ManualResetEventSlim(); using var releaseCleanup = new ManualResetEventSlim();
        using var started = new ManualResetEventSlim(); using var terminalReached = new ManualResetEventSlim();
        using var barrier = new DataBarrier();
        void CancelOwned()
        {
            control.Cancel(() => {
                cancelAt ??= dispatchAttempted ? clock.OperationMs : null;
                control.Publish();
                control.Accepted(runner?.Cancel(id) ?? false);
                runner?.Stop("cancelled");
                // Request original cancellation BEFORE unblocking the original native data pump.
                barrier.Release.Set(); releaseCleanup.Set();
            });
        }
        control.Publish();
        try {
            var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
            Check(!string.IsNullOrEmpty(worker) && File.Exists(worker), "explicit built actual worker required");
            Check(!File.Exists(helper), "refuse existing helper");
            Check(File.Exists(path), "owned source required");
            manifest = new StreamReader(System.IO.Path.Combine(root, "entries.jsonl"));
            callerSample = new Samples(Process.GetCurrentProcess(), "probe_owner");
            // Claim the destination as soon as creation succeeds, so a failed copy is still owned.
            using (var destination = new FileStream(helper, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                helperOwned = true; using var source = File.OpenRead(worker!); source.CopyTo(destination);
            }
            runner = ArchiveWorkerRunner.ForPackage(item => {
                try {
                    if (item.Kind == "started") started.Set();
                    if (item.Kind == "frame" && item.Frame is { } frame) {
                        if (frame.GetProperty("op").GetString() == "entry_batch") {
                            firstBatch ??= clock.OperationMs;
                            foreach (var entry in frame.GetProperty("entries").EnumerateArray()) {
                                using var expected = JsonDocument.Parse(manifest.ReadLine() ?? throw new InvalidDataException());
                                foreach (var key in new[] { "ordinal", "name", "kind", "size", "packed_size" })
                                    Check(entry.GetProperty(key).ToString() == expected.RootElement.GetProperty(key).ToString(), "streamed manifest mismatch");
                                Interlocked.Increment(ref count);
                            }
                        }
                        if (frame.GetProperty("op").GetString() == "failure") {
                            observedWorkerFailure = frame.GetProperty("error_code").GetString();
                            if (observedWorkerFailure == "Cancelled") workerCancelledAt = clock.OperationMs;
                        }
                    }
                    if (item.Kind == "exited") { terminal = item; terminalAt = clock.OperationMs; terminalReached.Set(); }
                } catch (Exception failure) { Interlocked.CompareExchange(ref callbackFailure, failure, null); }
            }, scenario == "tool-stop-small" ? barrier : null);
            barrier.Session = () => ((ArchiveWorkerOutputBridge)Field(Run(runner, id), "Output")).CreationOutput;
            runner.CleanupCheckpointProbe = () => {
                cleanupAt = clock.OperationMs;
                if (scenario == "cancel1gib-hold") { cleanupReached.Set(); releaseCleanup.Wait(10000); }
            };
            if (scenario != "runner100k") { Directory.CreateDirectory(output); outputOwned = true; }
            if (control.Poll()) { CancelOwned(); throw new OperationCanceledException(); }
            var request = new ArchiveWorkerRequest(id, scenario == "runner100k" ? "open" : "extract", path, 0,
                new { max_entries = 100000UL, max_metadata_bytes = 256UL << 20, max_working_bytes = 64UL << 20 },
                TimeoutMs: 120000, OutputRoot: scenario == "runner100k" ? null : output);
            dispatchAttempted = true; clock.Dispatch();
            admitted = runner.Start(request);
            Check(admitted, "probe admission");
            while (!terminalReached.IsSet) {
                if (control.Poll()) { control.Publish(); CancelOwned(); break; }
                if (workerSample is null && started.IsSet && identityAttempts < 50) {
                    identityAttempts++;
                    try {
                        actualWorker ??= Worker(runner, id, helper);
                        control.AddWorker(actualWorker, id); workerSample = new Samples(actualWorker, "actual_worker"); control.Publish();
                    }
                    catch (Exception failure) { identityFailures++; control.Error("worker_identity_unavailable", failure); }
                }
                callerSample.Take(); workerSample?.Take();
                if (actualWorker?.HasExited == true && processExitAt is null) processExitAt = clock.OperationMs;
                if (scenario == "tool-stop-small" && barrier.Reached.IsSet && partialBytes == 0) {
                    partialBytes = Interlocked.Read(ref barrier.Bytes);
                    heldSlot = runner.ActiveCount == 1; heldSource = !SourceWritable(path);
                    Check(partialBytes is > 0 and < 262144 && heldSlot && heldSource, "small native data barrier");
                    control.Barrier = new { partial_bytes = partialBytes, source_held = heldSource, slot_held = heldSlot };
                    control.Publish();
                }
                if (scenario is "cancel1gib" or "cancel1gib-hold" && cancelAt is null) {
                    partialBytes = Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Select(file => new FileInfo(file).Length).DefaultIfEmpty().Max();
                    if (partialBytes > 0) {
                        Check(partialBytes < 1L << 30 && !SourceWritable(path), "active data/source lease required");
                        cancelAt = clock.OperationMs; Check(runner.Cancel(id), "original cancel admission");
                    }
                }
                if (cleanupReached.IsSet && !releaseCleanup.IsSet) {
                    heldSlot = runner.ActiveCount == 1; heldSource = !SourceWritable(path);
                    var resources = (List<IDisposable>)Field(Run(runner, id), "Resources");
                    heldJobEmpty = ExtensionNative.JobEmpty(resources.OfType<ExtensionKernelHandle>().First());
                    Check(heldSlot && heldSource && heldJobEmpty && actualWorker?.HasExited == true, "cleanup held source/slot/Job");
                    releaseCleanup.Set();
                }
                Check(clock.OperationMs < 125000, "resource operation deadline");
                await Task.Delay(20);
            }
            if (callbackFailure is not null) control.Request("callback_failed");
            if (callbackFailure is not null) throw new InvalidOperationException("callback assertion", callbackFailure);
        } catch (Exception failure) {
            failed = true; control.Error("probe_operation_failed", failure); control.Request("probe_failure"); CancelOwned();
        } finally {
            // This guard starts before helper copy, manifest/counters and runner construction.
            if (control.Poll()) CancelOwned();
            barrier.Release.Set(); releaseCleanup.Set();
            control.Draining(); control.Publish();
            var drained = runner is null;
            try { runner?.Stop(control.Requested ? "cancelled" : "disconnected"); if (runner is not null) drained = await runner.DrainAsync(); }
            catch (Exception failure) { control.Error("runner_drain_failed", failure); }
            if (drained && dispatchAttempted && !terminalReached.IsSet) terminalReached.Wait(1000);
            bool? exited = actualWorker is null ? null : false;
            try { if (actualWorker is not null) { exited = actualWorker.HasExited; if (exited == true) processExitAt ??= clock.OperationMs; } }
            catch (Exception failure) { control.Error("worker_exit_unavailable", failure); }
            var activeCount = runner?.ActiveCount ?? 0;
            // CleanupComplete is the original native process + Job + ALL pumps drainage fact.
            var nativeConfirmed = !dispatchAttempted || terminal?.CleanupComplete == true;
            var sourceWritable = File.Exists(path) ? SourceWritable(path) : (bool?)null;
            bool? partialAbsent = !outputOwned;
            try { if (outputOwned) partialAbsent = Directory.Exists(output) ? !Directory.EnumerateFileSystemEntries(output).Any() : true; }
            catch (Exception failure) { partialAbsent = null; control.Error("output_observation_failed", failure); }
            bool? helperRemoved = helperOwned ? false : File.Exists(helper) ? null : true;
            bool? outputRemoved = !outputOwned;
            if (drained && nativeConfirmed && activeCount == 0 && (!admitted || exited == true)) {
                try { if (helperOwned) { File.Delete(helper); helperRemoved = !File.Exists(helper); } }
                catch (Exception failure) { helperRemoved = false; control.Error("helper_remove_failed", failure); }
                // Nonrecursive only, only our empty output directory, after original cleanup.
                try {
                    if (outputOwned && partialAbsent == true && Directory.Exists(output)) Directory.Delete(output, false);
                    if (outputOwned) outputRemoved = !Directory.Exists(output);
                }
                catch (Exception failure) { control.Error("empty_output_remove_failed", failure); }
            }
            var confirmed = drained && nativeConfirmed && activeCount == 0 && (!admitted || exited == true) &&
                (!dispatchAttempted || sourceWritable == true) && helperRemoved == true && partialAbsent == true && outputRemoved == true;
            control.Finish(new(confirmed ? "Confirmed" : "Incomplete", confirmed, dispatchAttempted ? terminal?.CleanupComplete : null,
                exited, dispatchAttempted ? terminal?.CleanupComplete : null, activeCount, sourceWritable, helperRemoved, partialAbsent, outputRemoved));
            operationOk = !failed && admitted && terminal is not null && (scenario == "runner100k"
                ? terminal.Category == "stdout_total_limit" && count is > 0 and < 100000
                : terminal.Category == "cancelled" && cancelAt is not null);
            control.Publish();
            var measurementComplete = callerSample?.Count > 0 && workerSample?.Count > 0;
            try { Console.WriteLine("RESOURCE " + JsonSerializer.Serialize(new { schema = 2, run_id = control.RunId, scenario, operation_id = id, debug = true,
                setup_ms = clock.SetupMs, timing_origin = "runner_dispatch",
                operation = new { status = operationOk ? "Expected" : "Failed", expected_outcome = scenario == "runner100k" ? "stdout_total_limit" : "cancelled",
                    category = terminal?.Category, exit_code = terminal?.ExitCode, first_batch_ms = firstBatch, full_terminal_ms = terminalAt,
                    cancel_at_ms = cancelAt, process_exit_observed_ms = processExitAt, cleanup_predisposal_ms = cleanupAt,
                    cancel_to_exit_upper_ms = processExitAt - cancelAt, cancel_to_cleanup_ms = terminalAt - cancelAt,
                    observed_entries = count, partial_bytes_at_cancel = partialBytes, heldSlot, heldSource, heldJobEmpty,
                    worker_cancel_frame = observedWorkerFailure, worker_cancel_accept_timestamp = "not_exposed", worker_cancel_observe_frame_ms = workerCancelledAt },
                measurement = new { status = measurementComplete ? "Complete" : "Incomplete", required_roles = new[] { "probe_owner", "actual_worker" },
                    identity_attempts = identityAttempts, identity_failures = identityFailures, identity_successes = actualWorker is null ? 0 : 1,
                    caller = callerSample?.Result() ?? Unavailable("probe_owner"), worker = workerSample?.Result() ?? Unavailable("actual_worker"), possible_missed_peaks = true },
                cleanup = control.CleanupFacts, stop = control.StopResult(), errors = control.Errors })); }
            catch (Exception failure) { failed = true; control.Error("resource_output_failed", failure); control.Publish(); }
            try { manifest?.Dispose(); } finally {
                try { callerSample?.Dispose(); } finally {
                    try { workerSample?.Dispose(); if (workerSample is null) actualWorker?.Dispose(); } finally { runner?.Dispose(); }
                }
            }
        }
        return !failed && operationOk && control.CleanupFacts.confirmed && !control.Requested && !control.StatusFailed && callerSample?.Count > 0 && workerSample?.Count > 0 ? 0 : 1;
    }
    private static object Unavailable(string role) => new { Role = role, status = "Incomplete", attempts = 0, failures = 0, successes = 0,
        Count = 0, BaselineWorking = (long?)null, BaselinePrivate = (long?)null, PeakWorking = (long?)null, PeakPrivate = (long?)null,
        LastWorking = (long?)null, LastPrivate = (long?)null };
}
