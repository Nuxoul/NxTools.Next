using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: explicit matching worker, GUID-owned PI_SCRATCH_DIR package and synthetic input only.
// Original browser/Job/Run/native publication; no GUI, Shell, picker, clipboard or user data.
internal static partial class Opt06BulkTransferTests
{
    private const string Marker = "opt06-binary-baseline-v1";
    private const string Member = "seed.bin";
    private const string Addition = "addition.bin";
    private const int InputSize = 1 << 20;

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class MeasurementLog : IPlatformLog, IDisposable
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Events = new();
        private ArchiveWorkerRunner? runner;
        private Guid operation;
        private string? image;
        private ArchiveResourceProbe.Samples? owner, worker;
        private Task? sampler;
        private volatile bool stopped;
        private string? sampleError;
        private double? cpuStart, cpuEnd;

        internal void Bind(ArchiveJobSession session, Guid id, string root)
        {
            runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(session, "runner");
            operation = id;
            image = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        }

        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (correlationId != operation) return;
            if (name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled") stopped = true;
            Events.Enqueue((name, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
            if (name != "archive.worker.started") return;
            try
            {
                owner = new(Process.GetCurrentProcess(), "probe_owner");
                worker = new(ArchiveResourceProbe.Worker(runner!, operation, image!), "actual_worker");
                cpuStart = Cpu();
                sampler = Task.Run(async () => {
                    while (!stopped)
                    {
                        owner.Take(); worker.Take(); cpuEnd = Cpu();
                        await Task.Delay(20);
                    }
                });
            }
            catch (Exception error) { sampleError = error.GetType().Name; }
        }

        private double? Cpu()
        {
            try { return ((Process)ArchiveResourceProbe.Field(worker!, "process")).TotalProcessorTime.TotalMilliseconds; }
            catch (Exception error) when (error is InvalidOperationException or System.ComponentModel.Win32Exception) { return null; }
        }

        internal object Snapshot()
        {
            stopped = true;
            sampler?.GetAwaiter().GetResult();
            Check(sampleError is null && owner?.Count > 0 && worker?.Count > 0 &&
                string.Equals(worker.Path, image, StringComparison.OrdinalIgnoreCase), "original worker handle and sampled memory required: " + sampleError);
            var terminal = Events.Where(e => e.Id == operation && e.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled").ToArray();
            Check(terminal.Length == 1, "one original worker terminal diagnostic required");
            var fields = terminal[0].Fields;
            foreach (var key in new[] { "elapsed_ms", "files_committed", "files_skipped", "transport_stdin_json_bytes", "transport_stdout_json_bytes",
                "native_source_attempts", "native_source_full_reads", "native_source_read_bytes", "native_source_completed_bytes",
                "native_target_attempts", "native_target_full_reads", "native_target_read_bytes", "native_target_completed_bytes",
                "native_output_attempts", "native_output_full_reads", "native_output_read_bytes", "native_output_completed_bytes",
                "partial_read_calls", "partial_read_bytes", "source_payload_read_calls", "source_payload_read_bytes",
                "output_payload_write_calls", "output_payload_write_bytes" })
                Check(fields.ContainsKey(key) && fields[key] is not null, "missing original terminal counter: " + key);
            Check(Convert.ToInt64(fields["transport_stdin_json_bytes"]) > 0 && Convert.ToInt64(fields["transport_stdout_json_bytes"]) > 0,
                "actual private transport required");
            long Counter(string key) => Convert.ToInt64(fields[key]);
            var readBytes = Counter("source_payload_read_bytes") + Counter("partial_read_bytes");
            var readFrames = Counter("source_payload_read_calls") + Counter("partial_read_calls");
            var writeBytes = Counter("output_payload_write_bytes");
            var writeFrames = Counter("output_payload_write_calls");
            Check(Counter("transport_stdin_binary_frames") == readFrames &&
                Counter("transport_stdin_binary_bytes") == readBytes + 80 * readFrames &&
                Counter("transport_stdout_binary_frames") == writeFrames &&
                Counter("transport_stdout_binary_bytes") == writeBytes + 80 * writeFrames, "exact raw payload/wire/frame accounting");
            Check(Counter("payload_conversion_input_bytes") == 0 && Counter("transport_payload_copy_bytes") == readBytes,
                "no modern payload conversion; one bounded read-envelope copy");
            Check(Counter("transport_reserved_bytes") == (1 << 20) &&
                Counter("transport_payload_buffers_high_water") is >= 1 and <= 2 &&
                Counter("transport_payload_buffer_bytes_high_water") is > 0 and <= 131072,
                "original fixed overhead and at-most-two payload buffers");
            var work = Events.Single(e => e.Name == "archive.worker.work" && e.Id == operation).Fields;
            foreach (var key in new[] { "native_write_ms", "native_commit_ms", "native_fingerprint_ms", "native_postpublication_ms" })
                Check(fields.ContainsKey(key) && fields[key] is not null, "missing native phase: " + key);
            foreach (var key in new[] { "zip_inflate_us", "zip_inflate_calls", "zip_payload_read_us", "output_ipc_ack_us", "output_write_calls", "output_write_bytes" })
                Check(work.ContainsKey(key), "missing worker phase: " + key);
            Check(Counter("native_write_ms") >= 0 && Counter("native_commit_ms") >= 0 &&
                Counter("native_fingerprint_ms") <= Counter("native_commit_ms"), "native phase nesting");
            Check(Convert.ToUInt64(work["output_write_bytes"]) == (ulong)writeBytes &&
                Convert.ToUInt64(work["output_write_calls"]) == (ulong)writeFrames,
                "Rust ACK-bound output must match original native write observations");
            return new { terminal_event = terminal[0].Name, terminal_fields = fields, owner_memory = owner!.Result(), worker_memory = worker!.Result(),
                worker_cpu_sampled_ms = cpuStart is { } start && cpuEnd is { } end ? Math.Max(0, end - start) : (double?)null,
                cpu_semantics = "sampled_worker_process_interval_not_exclusive_codec_cpu" };
        }

        public void Dispose()
        {
            stopped = true;
            sampler?.GetAwaiter().GetResult();
            owner?.Dispose(); worker?.Dispose();
        }
    }

    private static byte[] Bytes(int seed)
    {
        var bytes = new byte[InputSize];
        new Random(seed).NextBytes(bytes);
        return bytes;
    }

    private static byte[] IndependentZip(byte[] payload)
    {
        using var memory = new MemoryStream();
        using (var archive = new ZipArchive(memory, ZipArchiveMode.Create, true))
        {
            var entry = archive.CreateEntry(Member, CompressionLevel.NoCompression);
            entry.LastWriteTime = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
            using var output = entry.Open(); output.Write(payload);
        }
        return memory.ToArray();
    }

    private static string OwnedRoot()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        var configured = Environment.GetEnvironmentVariable("NXTOOLS_OPT06_ROOT");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch) &&
            !string.IsNullOrWhiteSpace(configured) && Path.IsPathFullyQualified(configured), "explicit absolute scratch and OPT06 root required");
        var root = Path.GetFullPath(configured!);
        var leaf = Path.GetFileName(root);
        Check(string.Equals(Path.GetDirectoryName(root), Path.GetFullPath(scratch!).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) &&
            leaf.StartsWith("archive-product-opt06-", StringComparison.Ordinal) &&
            Guid.TryParseExact(leaf["archive-product-opt06-".Length..], "D", out _), "GUID-owned direct scratch child required");
        if (Environment.GetEnvironmentVariable("NXTOOLS_OPT06_BASELINE") == "1")
        {
            Check(!Directory.Exists(root), "baseline root already exists; refuse input regeneration");
            Directory.CreateDirectory(root);
        }
        ArchiveReadHelper.GuardOwnedRoot(root);
        return root;
    }

    private static void Inputs(string root, bool baseline, byte[] seed, byte[] addition, byte[] zip)
    {
        var inputRoot = Path.Combine(root, "inputs");
        var marker = Path.Combine(root, "baseline.marker");
        if (baseline)
        {
            Directory.CreateDirectory(inputRoot);
            File.WriteAllBytes(Path.Combine(inputRoot, Member), seed);
            File.WriteAllBytes(Path.Combine(inputRoot, Addition), addition);
            File.WriteAllBytes(Path.Combine(inputRoot, "independent.zip"), zip);
            using var output = new FileStream(marker, FileMode.CreateNew, FileAccess.Write, FileShare.None);
            using var writer = new StreamWriter(output); writer.Write(Marker);
        }
        Check(File.Exists(marker) && File.ReadAllText(marker) == Marker, "baseline marker required");
        Check(File.ReadAllBytes(Path.Combine(inputRoot, Member)).AsSpan().SequenceEqual(seed) &&
            File.ReadAllBytes(Path.Combine(inputRoot, Addition)).AsSpan().SequenceEqual(addition) &&
            File.ReadAllBytes(Path.Combine(inputRoot, "independent.zip")).AsSpan().SequenceEqual(zip), "baseline inputs changed");
    }

    private static async Task<ArchiveBrowserContext> Browser(string root, string archive)
    {
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: archive, version: 2, limits: new() { BrowserContext = true });
        var command = ArchiveReadWire.Parse(wire);
        Check(reads.Dispatch(wire).Success, "original browser dispatch");
        while (true)
        {
            var item = (await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch)
                Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation,
                    sequence: batch.Sequence, version: 2)).Success, "original metadata credit");
            if (item.Terminal is not { } terminal) continue;
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "fresh original browser binding");
            return terminal.Browser!;
        }
    }

    private static ProtocolEnvelope Wire(ArchiveJobCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);

    private static async Task<object> Trial(string root, string operation, int index, byte[] seed, byte[] added)
    {
        var caseRoot = Path.Combine(root, "cases", operation + "-" + index);
        var source = Path.Combine(caseRoot, "archive", "independent.zip");
        var destination = Path.Combine(caseRoot, "output");
        Directory.CreateDirectory(Path.GetDirectoryName(source)!);
        Directory.CreateDirectory(destination);
        if (operation != "create_zip_store") File.Copy(Path.Combine(root, "inputs", "independent.zip"), source);
        ArchiveBrowserContext? browser = operation == "modify_zip_add" ? await Browser(root, source) : null;
        var request = operation switch
        {
            "create_zip_store" => new ArchiveJobRequest { Kind = "create", Paths = [Path.Combine(root, "inputs", Member)], Destination = destination,
                Create = new() { Format = "zip", Name = "created.zip", Level = "store", Method = "store", StripSingleRoot = false,
                    ExcludePatterns = [], TestAfterCreate = false } },
            "extract_zip" => new ArchiveJobRequest { Kind = "extract", Paths = [source], Destination = destination,
                Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "skip", CopyMotw = false } },
            "modify_zip_add" => new ArchiveJobRequest { Kind = "modify", Paths = [source, Path.Combine(root, "inputs", Addition)],
                Modify = new(browser!, "add", [], null) },
            _ => throw new InvalidOperationException("unknown measurement route")
        };
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(),
            Generation = 1, Action = "start", Request = request };
        using var log = new MeasurementLog();
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root, log);
        log.Bind(jobs, command.OperationId, root);
        var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(jobs, "runner");
        var publications = 0;
        jobs.ProbeOutputCommitForFixture(stage => { if (stage == (operation == "extract_zip" ? "native_closed" : "native_published")) Interlocked.Increment(ref publications); });
        var clock = Stopwatch.StartNew();
        Check(jobs.Dispatch(Wire(command)).Success, "original Job admission");
        ArchiveJobTerminal? terminal = null;
        while (terminal is null)
        {
            var item = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Check(item.OperationId == command.OperationId && item.Prompt is null, "matching prompt-free Job");
            terminal = item.Terminal;
        }
        Check(await runner.DrainAsync() && runner.ActiveCount == 0, "actual Run drain and zero active leases");
        clock.Stop(); // Independent decoding and fixture reads are outside the measured operation.
        Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete && terminal.OutputPublished == true &&
            terminal.FilesCommitted == 1 && terminal.Journal is { Count: 1 } && terminal.Journal[0].State == "Committed" &&
            terminal.Journal[0].Published == true && publications == 1, "one actual publication and complete native cleanup");
        var resultPath = operation switch { "create_zip_store" => Path.Combine(destination, "created.zip"),
            "extract_zip" => Path.Combine(destination, Member), _ => source };
        if (operation == "extract_zip") Check(File.ReadAllBytes(resultPath).AsSpan().SequenceEqual(seed), "independent extracted bytes");
        else
        {
            using var archive = ZipFile.OpenRead(resultPath);
            Check(archive.Entries.Count == (operation == "modify_zip_add" ? 2 : 1), "independent ZIP member count");
            foreach (var expected in operation == "modify_zip_add" ? new[] { (Member, seed), (Addition, added) } : new[] { (Member, seed) })
            {
                var entry = archive.GetEntry(expected.Item1);
                Check(entry is not null && entry.Length == InputSize, "independent ZIP entry");
                using var decoded = entry!.Open(); using var actual = new MemoryStream(); decoded.CopyTo(actual);
                Check(actual.ToArray().AsSpan().SequenceEqual(expected.Item2), "independent ZIP decoded bytes");
            }
        }
        foreach (var path in new[] { Path.Combine(root, "inputs", Member), Path.Combine(root, "inputs", Addition), source, resultPath }.Distinct())
            if (File.Exists(path)) using (var reopened = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                Check(reopened.Length > 0, "exclusive reopen after actual cleanup");
        Check(!Directory.EnumerateFiles(caseRoot, ".nxpart-*", SearchOption.AllDirectories).Any(), "no owned unpublished partial");
        Check(File.ReadAllBytes(Path.Combine(root, "inputs", Member)).AsSpan().SequenceEqual(seed) &&
            File.ReadAllBytes(Path.Combine(root, "inputs", Addition)).AsSpan().SequenceEqual(added), "native source inputs remain exact");
        if (operation == "extract_zip")
            Check(File.ReadAllBytes(source).AsSpan().SequenceEqual(File.ReadAllBytes(Path.Combine(root, "inputs", "independent.zip"))),
                "original independent ZIP source remains exact");
        var observation = log.Snapshot();
        var workerFields = log.Events.Single(e => e.Id == command.OperationId &&
            e.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled").Fields;
        Check(Convert.ToInt64(workerFields["source_payload_read_bytes"]) == (operation == "extract_zip" ? 0 : InputSize) &&
            Convert.ToInt64(workerFields["output_payload_write_bytes"]) > 0, "actual native source and output payload counters");
        if (operation != "extract_zip")
            Check(Convert.ToInt64(workerFields["partial_read_bytes"]) > 0 &&
                Convert.ToInt64(workerFields["partial_read_calls"]) > 0, "actual candidate/created held-partial readback required");
        var record = new { operation, trial = index, command.OperationId, operation_wall_ms = clock.Elapsed.TotalMilliseconds,
            cleanup_ms = (double?)null, cleanup_semantics = "included_in_operation_wall_not_separately_timed",
            terminal.Outcome, terminal.Code, terminal.FilesCommitted, terminal.BytesCommitted, terminal.FilesVerified,
            terminal.BytesVerified, terminal.NativeErrorCode, terminal.CommitState, terminal.OutputPublished, terminal.CleanupComplete,
            journal = new { count = terminal.Journal!.Count, terminal.Journal[0].State, terminal.Journal[0].Published, terminal.Journal[0].Replaced },
            publications, observation };
        // Only per-trial archive/output are removed; immutable inputs and historical reports remain.
        if (File.Exists(source)) File.Delete(source);
        if (File.Exists(resultPath)) File.Delete(resultPath);
        return record;
    }

#if DEBUG
    internal static async Task Measurements()
#else
    internal static Task Measurements()
#endif
    {
#if !DEBUG
        throw new InvalidOperationException("OPT06 native measurements require Debug.");
#else
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")
            ?? throw new InvalidOperationException("explicit native worker required");
        var expected = Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_WORKER_SHA256");
        if (!Path.IsPathFullyQualified(worker) || !File.Exists(worker) || expected is not { Length: 64 } ||
            !Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("explicit matching native worker/hash required");
        var root = OwnedRoot();
        var baseline = Environment.GetEnvironmentVariable("NXTOOLS_OPT06_BASELINE") == "1";
        var seed = Bytes(606); var added = Bytes(607); var zip = IndependentZip(seed);
        Inputs(root, baseline, seed, added, zip);
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        if (!baseline) Check(File.Exists(helper), "existing stable worker execution path required");
        if (!baseline) using (var released = File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
            Check(released.Length > 0, "stable worker path must be released before replacement");
        // Each invocation runs after all earlier Jobs have drained; never replace the execution path during a Run.
        File.Copy(worker, helper, !baseline);
        var requested = Environment.GetEnvironmentVariable("NXTOOLS_OPT06_REPORT");
        var report = requested ?? Path.Combine(root, "reports", Guid.NewGuid().ToString("D") + ".json");
        Check(Path.IsPathFullyQualified(report) &&
            string.Equals(Path.GetDirectoryName(Path.GetFullPath(report)), Path.Combine(root, "reports"), StringComparison.OrdinalIgnoreCase) &&
            !File.Exists(report), "new report must be inside owned reports directory");
        Directory.CreateDirectory(Path.GetDirectoryName(report)!);
        using var reserved = new FileStream(report, FileMode.CreateNew, FileAccess.Write, FileShare.None);
        var run = Guid.NewGuid().ToString("D");
        var results = new List<object>();
        string? failureType = null;
        try
        {
            foreach (var operation in new[] { "create_zip_store", "extract_zip", "modify_zip_add" })
                for (var trial = 0; trial < 3; trial++)
                    results.Add(await Trial(root, operation, trial, seed, added));
        }
        catch (Exception failure) { failureType = failure.GetType().Name; throw; }
        finally
        {
            var output = JsonSerializer.SerializeToUtf8Bytes(new { schema = 1, route = "original_job_runner_native",
                configuration = "Debug", host = typeof(ArchiveWorkerRunner).Assembly.Location,
                tests = typeof(Opt06BulkTransferTests).Assembly.Location, worker_execution_path = helper,
                worker_sha256 = expected, root, run, input_bytes = InputSize, disk = Path.GetPathRoot(root),
                cache_policy = "first_measured_use_then_reused_source_no_cache_flush",
                complete = failureType is null && results.Count == 9, failure_type = failureType, results });
            reserved.Write(output); reserved.Flush(true);
            Console.WriteLine("ARCHIVE_OPT06_REPORT " + report);
        }
#endif
    }
}
