using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: owned temporary source/helper/output/cache only; original native runner, Job and source leases.
// No GUI, Shell, registry, services, user source mutation, recycling or installation.
internal static partial class ArchiveOptimizationTests
{
    private static void Check(bool condition, string reason) { if (!condition) throw new InvalidOperationException(reason); }
    private sealed class WorkLog : IPlatformLog, IDisposable
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, JsonElement Fields)> Events = new();
        private ArchiveWorkerRunner? runner;
        private string? expectedImage;
        private Guid operationId;
        private ArchiveResourceProbe.Samples? host, worker;
        private Task? sampler;
        private volatile bool stopped;
        private string? samplingFailure;
        internal void Bind(object session, Guid id, string root)
        {
            runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(session, "runner");
            operationId = id;
            expectedImage = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        }
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            Events.Enqueue((name, correlationId, JsonSerializer.SerializeToElement(fields)));
            if (correlationId != operationId) return;
            if (name == "archive.worker.started") {
                try {
                    host = new(Process.GetCurrentProcess(), "probe_owner");
                    worker = new(ArchiveResourceProbe.Worker(runner!, operationId, expectedImage!), "actual_worker");
                    sampler = Task.Run(async () => {
                        while (!stopped) {
                            host.Take(); worker.Take();
                            await Task.Delay(20);
                        }
                    });
                } catch (Exception failure) { samplingFailure = failure.GetType().Name; }
            }
            if (name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled") stopped = true;
        }
        internal JsonElement[] Work => Events.Where(e => e.Name == "archive.worker.work").Select(e => e.Fields).ToArray();
        internal object Snapshot()
        {
            stopped = true; sampler?.GetAwaiter().GetResult();
            Check(samplingFailure is null && host?.Count > 0 && worker?.Count > 0 &&
                string.Equals(worker.Path, expectedImage, StringComparison.OrdinalIgnoreCase), "native handle identity and sampled memory required " + samplingFailure);
            var terminal = Events.Single(e => e.Id == operationId && e.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled").Fields;
            var stdin = terminal.GetProperty("transport_stdin_json_bytes").GetInt64();
            var stdout = terminal.GetProperty("transport_stdout_json_bytes").GetInt64();
            Check(stdin > 0 && stdout > 0, "original pipe transport bytes required");
            var phases = new List<object>(); long? previous = null;
            foreach (var entry in Events.Where(e => e.Id == operationId && e.Name.StartsWith("archive.worker.", StringComparison.Ordinal) &&
                         e.Name != "archive.worker.work" && e.Fields.ValueKind == JsonValueKind.Object && e.Fields.TryGetProperty("elapsed_ms", out _))) {
                var elapsed = entry.Fields.GetProperty("elapsed_ms").GetInt64();
                if (phases.Count < 32) phases.Add(new { phase = entry.Name["archive.worker.".Length..], elapsed_ms = elapsed,
                    interval_since_previous_event_ms = previous is null ? (long?)null : Math.Max(0, elapsed - previous.Value) });
                previous = elapsed;
            }
            Check(phases.Count >= 2, "actual runner phase timestamps required");
            return new { host = host!.Result(), worker = worker!.Result(), transport_stdin_json_bytes = stdin,
                transport_stdout_json_bytes = stdout, phases, phase_semantics = "timestamp_differences_between_observed_runner_notifications_not_exclusive_codec_phase_times" };
        }
        public void Dispose() { stopped = true; sampler?.GetAwaiter().GetResult(); host?.Dispose(); worker?.Dispose(); }
    }
    private sealed record Observation(string Operation, int Trial, double ElapsedMs, JsonElement[] Work,
        int Workers, ulong Files, ulong Bytes, bool Published, bool CleanupComplete);
    private static byte[] Payload(int index) => Enumerable.Range(0, index == 16 ? 16 << 20 : 262144).Select(i => (byte)((i * 31 + index * 17) % 251)).ToArray();
    private static ulong PayloadBytes(ulong members) => Math.Min(members, 16UL) * 262144 + (members > 16 ? 16UL << 20 : 0);
    private static string Root()
    {
        var root = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT_ROOT")!;
        Check(Path.IsPathFullyQualified(root) && Directory.Exists(root) && File.Exists(Path.Combine(root, "owned-opt-fixtures")), "explicit owned OPT fixture root required");
        ArchiveReadHelper.GuardOwnedRoot(root);
        var source = Path.Combine(root, "sixteen.zip");
        if (!File.Exists(source)) {
            using var zip = ZipFile.Open(source, ZipArchiveMode.Create);
            for (var index = 0; index < 17; index++) {
                var entry = zip.CreateEntry($"member-{index:D2}.bin", CompressionLevel.NoCompression);
                entry.LastWriteTime = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
                using var output = entry.Open(); output.Write(Payload(index));
            }
        }
        using (var zip = ZipFile.OpenRead(source)) {
            Check(zip.Entries.Count == 17, "sixteen selected members plus one large unselected member");
            for (var index = 0; index < 17; index++) {
                using var stream = zip.Entries[index].Open(); using var bytes = new MemoryStream(); stream.CopyTo(bytes);
                Check(bytes.ToArray().SequenceEqual(Payload(index)), "fixture bytes");
            }
        }
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")!;
        Check(Path.IsPathFullyQualified(worker) && File.Exists(worker), "explicit current worker required");
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        Check(!File.Exists(helper), "previous helper must be released and removed");
        File.Copy(worker, helper);
        Console.WriteLine("ARCHIVE_OPT_PAIR " + JsonSerializer.Serialize(new {
            configuration = "Debug", host = typeof(ArchiveWorkerRunner).Assembly.Location, tests = typeof(ArchiveOptimizationTests).Assembly.Location,
            worker, worker_bytes = new FileInfo(worker).Length, worker_modified_utc = File.GetLastWriteTimeUtc(worker),
            fixture = source, fixture_bytes = new FileInfo(source).Length, cache_policy = "no_system_cache_flush"
        }));
        return root;
    }
    private static Observation Record(string operation, int trial, Stopwatch clock, WorkLog log, ulong files, ulong bytes, bool published, bool cleanup)
    {
        var work = log.Work;
        Check(work.Length > 0, "current worker work observation required");
        var workers = log.Events.Count(e => e.Name == "archive.worker.started");
        Check(workers > 0, "actual worker start required");
        var result = new Observation(operation, trial, clock.Elapsed.TotalMilliseconds, work, workers, files, bytes, published, cleanup);
        Console.WriteLine("ARCHIVE_OPT " + JsonSerializer.Serialize(new { result, metrics = log.Snapshot(), source_observation = trial == 0 ? "first_measured_use" : "reused_source", source_read_bytes = "revision_only_not_total", block_decodes = "not_instrumented" }));
        return result;
    }
    private static async Task<(ArchiveBrowserContext Context, Observation Observation)> Browse(string root, int trial)
    {
        using var log = new WorkLog(); await using var session = ArchiveReadSession.ForOwnedTestPackage(root, log);
        var wire = ArchiveProductReadTests.Command(path: Path.Combine(root, "sixteen.zip"), version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(wire); log.Bind(session, identity.OperationId, root); var clock = Stopwatch.StartNew();
        Check(session.Dispatch(wire).Success, "browser dispatch");
        ulong count = 0;
        while (true) {
            var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (value.Batch is { } batch) {
                count += (ulong)batch.Entries.Length;
                Check(session.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "metadata credit");
            }
            if (value.Terminal is { } terminal) {
                clock.Stop();
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null && count == 17, "browser terminal " + terminal.Code);
                return (terminal.Browser!, Record("browser_open", trial, clock, log, count, 0, false, terminal.CleanupComplete));
            }
        }
    }
    private static ProtocolEnvelope JobWire(ArchiveJobCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    private static async Task<Observation> Job(string root, ArchiveBrowserContext browser, int trial, int? selected)
    {
        using var log = new WorkLog();
        var destination = Path.Combine(root, "output-" + Guid.NewGuid());
        var test = selected == 0;
        if (!test) Directory.CreateDirectory(destination);
        var request = test ? new ArchiveJobRequest { Kind = "test", Paths = [Path.Combine(root, "sixteen.zip")] }
            : new ArchiveJobRequest { Kind = "extract", Paths = [Path.Combine(root, "sixteen.zip")], Destination = destination,
                Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "skip", CopyMotw = false,
                    Selection = selected is null ? null : new(browser, Enumerable.Range(0, selected.Value).Select(i => (ulong)i).ToArray()) } };
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
        var clock = Stopwatch.StartNew();
        try {
            await using var session = ArchiveJobSession.ForOwnedTestPackage(root, log);
            log.Bind(session, command.OperationId, root);
            Check(session.Dispatch(JobWire(command)).Success, "job dispatch");
            while (true) {
                var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                if (value.Terminal is not { } terminal) continue;
                clock.Stop();
                var members = (ulong)(test || selected is null ? 17 : selected.Value);
                Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete, "job terminal " + terminal.Code);
                if (test) Check(terminal.FilesVerified == members && terminal.BytesVerified == PayloadBytes(members) && terminal.FilesCommitted == 0, "Test output counts");
                else {
                    Check(terminal.FilesCommitted == members && terminal.BytesCommitted == PayloadBytes(members), "committed counts");
                    var files = Directory.GetFiles(destination, "*", SearchOption.AllDirectories);
                    Check(files.Length == (int)members && !files.Any(f => Path.GetFileName(f).StartsWith(".nxpart-")), "only final outputs");
                    for (var index = 0; index < (int)members; index++) Check(File.ReadAllBytes(Path.Combine(destination, $"member-{index:D2}.bin")).SequenceEqual(Payload(index)), "exact output bytes");
                }
                return Record(test ? "test" : selected is null ? "extract_all" : "extract_selected_" + selected, trial, clock, log,
                    test ? terminal.FilesVerified : terminal.FilesCommitted, test ? terminal.BytesVerified : terminal.BytesCommitted, !test && terminal.FilesCommitted > 0, terminal.CleanupComplete);
            }
        }
        finally { if (Directory.Exists(destination)) Directory.Delete(destination, true); }
    }
    private static async Task<Observation> Materialize(string root, ArchiveBrowserContext browser, int trial)
    {
        using var log = new WorkLog(); await using var session = ArchiveEntrySession.ForOwnedTestPackage(root, log);
        var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "materialize",
            Path = Path.Combine(root, "sixteen.zip"), Ordinal = 0, Browser = browser, CopyMotw = false };
        ProtocolEnvelope Wire(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
        var clock = Stopwatch.StartNew();
        log.Bind(session, command.OperationId, root);
        var reply = await session.Dispatch(Wire(command)).WaitAsync(TimeSpan.FromSeconds(90));
        clock.Stop();
        var value = JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
        Check(reply.Success && value.Success && value.CleanupComplete && value.Path is not null, "materialize terminal " + value.Code);
        Check(File.ReadAllBytes(value.Path!).SequenceEqual(Payload(0)), "materialized bytes");
        var result = Record("materialize", trial, clock, log, 1, 262144, true, value.CleanupComplete);
        var release = command with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = value.LeaseId };
        Check((await session.Dispatch(Wire(release))).Success && !File.Exists(value.Path), "materialized lease released");
        return result;
    }
    public static Task CompletedReadBaseline() => Measure(false);
    public static Task SharedReadWork() => Measure(true);
    private static async Task Measure(bool assertWork)
    {
        var root = Root(); var results = new List<Observation>();
        try {
            for (var trial = 0; trial < (assertWork ? 1 : 3); trial++) {
                var browse = await Browse(root, trial); results.Add(browse.Observation);
                results.Add(await Job(root, browse.Context, trial, 0));
                results.Add(await Materialize(root, browse.Context, trial));
                foreach (var selected in new int?[] { null, 1, 2, 16 }) results.Add(await Job(root, browse.Context, trial, selected));
                using var released = File.Open(Path.Combine(root, "sixteen.zip"), FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            foreach (var group in results.GroupBy(r => r.Operation)) {
                var elapsed = group.Select(r => r.ElapsedMs).Order().ToArray();
                Console.WriteLine("ARCHIVE_OPT_MEDIAN " + JsonSerializer.Serialize(new { operation = group.Key, trials = elapsed.Length, median_ms = elapsed[elapsed.Length / 2] }));
            }
            if (assertWork) {
                ulong Count(Observation value, string key) => value.Work.Aggregate(0UL, (sum, work) => sum + work.GetProperty(key).GetUInt64());
                var browser = results.Single(r => r.Operation == "browser_open");
                Check(Count(browser, "catalog_loads") == 1 && Count(browser, "revision_passes") == 1, "browser must share catalog and its one protected source revision with qualification");
                Check(results.Where(r => r.Operation != "browser_open").All(r => Count(r, "eligibility_checks") == 0), "read-only work must not qualify editing");
                var selected = results.Where(r => r.Operation.StartsWith("extract_selected_")).ToArray();
                Check(selected.Select(r => Count(r, "revision_passes")).Distinct().Count() == 1, "native protected revision count must not grow with selected members");
                var jobs = results.Where(r => r.Operation == "test" || r.Operation.StartsWith("extract_", StringComparison.Ordinal)).ToArray();
                Check(jobs.All(r => Count(r, "catalog_loads") == 1 && Count(r, "revision_passes") == 1),
                    "ordinary protected ZIP jobs must reuse password-admission catalog and read the held source once");
            }
        }
        finally {
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
            Check(!Directory.EnumerateDirectories(root, "output-*").Any(), "owned outputs removed");
            Console.WriteLine("ARCHIVE_OPT_CLEANUP helper_removed=true outputs_removed=true fixture_retained=true");
        }
    }
}
