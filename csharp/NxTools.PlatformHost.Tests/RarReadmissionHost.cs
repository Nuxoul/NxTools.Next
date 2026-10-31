using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Fixture-only transport for original output_readmission::execute; no approval/retry/budget policy.
// Deliberately unregistered. Owning lane may wire --rar-output-readmission-host <owned-output-root>.
internal static class RarReadmissionHost
{
    internal static async Task<int> RunAsync(string outputRoot)
    {
        RarFixturePackage? package = null;
        try {
            var mode = Environment.GetEnvironmentVariable("NXTOOLS_RAR_HOST_MODE") ?? "none";
            if (mode is not ("none" or "cleanup_barrier" or "cancel_after_commit")) return 2;
            var evidence = Environment.GetEnvironmentVariable("NXTOOLS_RAR_EVIDENCE_ROOT");
            if (string.IsNullOrWhiteSpace(evidence) || !RarFixturePackage.IsWithin(outputRoot, evidence)
                || !string.IsNullOrWhiteSpace(Environment.GetEnvironmentVariable("NXTOOLS_RAR_FIXTURES"))
                    && RarFixturePackage.IsWithin(outputRoot, Environment.GetEnvironmentVariable("NXTOOLS_RAR_FIXTURES")!)) return 2;
            package = RarFixturePackage.Create("output_readmission_host", requireFixtures: false);
            var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
            var attempts = 0; var captures = 0;
            var ids = new HashSet<Guid>();
            while (await ReadAsync(input) is { } request) {
                if (request.TryGetProperty("fixture_control", out var fixture) && fixture.GetString() == "capture_source") {
                    if (++captures > 2) return 3;
                    var path = request.GetProperty("path").GetString()!;
                    ValidateSourcePath(path);
                    using var source = ArchiveSourceLease.Open(path);
                    using var stream = File.OpenRead(path);
                    var format = RarFixturePackage.SignatureFormat(stream);
                    stream.Position = 0;
                    var digest = SHA256.HashData(stream).Select(b => (uint)b).ToArray();
                    var modified = checked((ulong)(File.GetLastWriteTimeUtc(path).Ticks - DateTime.UnixEpoch.Ticks) * 100);
                    // Reuse original held identity and original browser-context serializer for selection.
                    var browser = new ArchiveBrowserContext(Guid.NewGuid(), format, "", new(source.ReadmissionIdentity(), digest, (ulong)stream.Length, modified));
                    await WriteAsync(output, new { browser = JsonSerializer.SerializeToElement(browser, JsonDefaults.Options) });
                    continue;
                }
                if (request.TryGetProperty("fixture_control", out _) || ++attempts > 2) return 3;
                var command = Command(request, outputRoot);
                if (!ids.Add(command.OperationId)) return 3;
                var result = await AttemptAsync(package, command, attempts, mode, input, output);
                await WriteAsync(output, result);
            }
            return 0; // Every original runner has actually exited/drained before EOF.
        }
        catch (Exception failure) {
            if (package is not null) File.WriteAllText(Path.Combine(package.Root, "host-failure.json"), JsonSerializer.Serialize(new {
                category = "fixture_host_failure", exception_type = failure.GetType().Name, retained = true
            }));
            return 5;
        }
    }

    private static void ValidateSourcePath(string source)
    {
        if (!Path.IsPathFullyQualified(source) || !string.Equals(Path.GetExtension(source), ".rar", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Only explicit fixture RAR paths are admitted.");
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_RAR_EVIDENCE_ROOT");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_RAR_FIXTURES");
        if (!(evidence is not null && RarFixturePackage.IsWithin(source, evidence))
            && !(fixtures is not null && RarFixturePackage.IsWithin(source, fixtures)))
            throw new InvalidDataException("Source must belong to the fixture evidence tree.");
    }

    private static ArchiveWorkerRequest Command(JsonElement request, string outputRoot)
    {
        if (request.GetProperty("schema").GetInt32() != 2 || request.GetProperty("op").GetString() != "start"
            || request.GetProperty("operation").GetString() != "extract") throw new InvalidDataException("Original extraction request required.");
        var id = request.GetProperty("operation_id").GetGuid();
        if (id == Guid.Empty) throw new InvalidDataException("A fresh original operation ID is required.");
        var source = request.GetProperty("path").GetString()!;
        ValidateSourcePath(source); // Never copy between attempts: retain original identity/version binding.
        var password = Optional(request, "password") is { } secret ? secret.GetString() : null;
        var conflict = Optional(request, "conflict_policy") is { } policy ? policy.GetString() : null;
        var extract = Optional(request, "extract_options");
        var job = Optional(request, "job_extract")?.Deserialize<ArchiveJobExtract>(JsonDefaults.Options);
        return new(id, "extract", source, request.GetProperty("codepage").GetInt32(), request.GetProperty("limits").Clone(),
            Password: password, OutputRoot: outputRoot,
            ExtractOptions: extract is { } options ? new(options.GetProperty("copy_motw").GetBoolean()) : null,
            PasswordInteraction: Optional(request, "password_interaction")?.GetBoolean() ?? false, ConflictPolicy: conflict,
            OutputReadmission: Optional(request, "output_readmission"),
            JobExecution: Optional(request, "job_execution")?.GetBoolean() ?? false, JobExtract: job);
    }

    private static async Task<object> AttemptAsync(RarFixturePackage package, ArchiveWorkerRequest command, int attempt,
        string mode, Stream input, Stream output)
    {
        var root = Directory.CreateDirectory(Path.Combine(package.Root, "attempt-" + attempt)).FullName;
        File.WriteAllText(Path.Combine(root, "source-attribution.json"), JsonSerializer.Serialize(new {
            operation_id = command.OperationId, source_path = command.Path,
            source_sha256 = File.Exists(command.Path) ? RarFixturePackage.Hash(command.Path) : null, output_path = command.OutputRoot
        }));
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var audit = new RarFixtureAudit { CancelAfterCommit = mode == "cancel_after_commit" };
        // JobExecution delivers business frames to readFrame. Observe them and return null, including
        // failure/shortfall; never bypass native handling or synthesize a read contract/result.
        using var runner = ArchiveWorkerRunner.ForProductRead(package.Helper, events.Enqueue, (id, frame, cancellation) => {
            events.Enqueue(new("frame", id, Frame: frame.Clone()));
            return Task.FromResult<string?>(null);
        }, audit);
        audit.Install(runner, command.OperationId, command.OutputRoot!);
        using var cleanupReached = new ManualResetEventSlim(false);
        using var releaseCleanup = new ManualResetEventSlim(false);
        var cleanupTimedOut = 0;
        var cleanupBarrier = mode == "cleanup_barrier" && attempt == 1;
        if (cleanupBarrier) runner.CleanupCheckpointProbe = () => {
            // Original callback is outside runner.gate. Only signal here; the sole stdin reader
            // and all ActiveCount/lease observations belong to the main attempt, never this callback.
            cleanupReached.Set();
            if (!releaseCleanup.Wait(10000)) Interlocked.Exchange(ref cleanupTimedOut, 1);
        };
        var existedBefore = Directory.Exists(command.OutputRoot);
        object? result = null;
        try {
            RarRiskReadmissionTests.Check(runner.Start(command), "original finite RAR request admission");
            if (cleanupBarrier) {
                try {
                    await RarRiskReadmissionTests.Until(() => cleanupReached.IsSet || events.Any(e => e.Kind == "exited"), "original RAR cleanup checkpoint deadline");
                    RarRiskReadmissionTests.Check(cleanupReached.IsSet && runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "cleanup-held retains original reservation and has no exited result");
                    var sourceProtected = RarFixturePackage.IsProtected(command.Path);
                    var helperProtected = RarFixturePackage.IsProtected(package.Helper.Executable);
                    RarRiskReadmissionTests.Check(sourceProtected && helperProtected, "cleanup-held retains original source/helper leases");
                    var observation = new {
                        observation = "cleanup_held", operation_id = command.OperationId, active = runner.ActiveCount,
                        source_protected = sourceProtected, helper_protected = helperProtected, no_terminal = !events.Any(e => e.Kind == "exited"),
                        source_acquired = audit.SourceAcquired, checkpoints = audit.Checkpoints.ToArray()
                    };
                    File.WriteAllText(Path.Combine(root, "cleanup-held.json"), JsonSerializer.Serialize(observation));
                    await WriteAsync(output, observation);
                    var release = await ReadAsync(input);
                    RarRiskReadmissionTests.Check(release is { } control && control.TryGetProperty("fixture_control", out var instruction)
                        && instruction.GetString() == "release_cleanup"
                        && (!control.TryGetProperty("operation_id", out var releaseId) || releaseId.GetGuid() == command.OperationId), "explicit matching stdio cleanup release required");
                    RarRiskReadmissionTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "explicit cleanup release met original finite gate");
                }
                finally { releaseCleanup.Set(); }
            }
            await RarRiskReadmissionTests.Until(() => events.Any(e => e.Kind == "exited"), "actual original RAR exited deadline");
            RarRiskReadmissionTests.Check(await runner.DrainAsync(), "actual original RAR drain");
            RarRiskReadmissionTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "cleanup gate did not expire before explicit release");
            var terminal = events.Single(e => e.Kind == "exited");
            RarRiskReadmissionTests.Check(terminal.CleanupComplete && runner.ActiveCount == 0 && events.Count(e => e.Kind == "started") == 1
                && events.All(e => e.OperationId == command.OperationId), "one original correlated worker/Job, actual cleanup, no C# rerun");
            var sourceReleased = RarFixturePackage.IsReleased(command.Path);
            var helperReleased = RarFixturePackage.IsReleased(package.Helper.Executable);
            RarRiskReadmissionTests.Check(sourceReleased && helperReleased, "exclusive original source/helper reopen before AttemptResult");
            var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
            var shortfall = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "output_budget_shortfall");
            var failure = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "failure");
            var failureCode = failure.ValueKind == JsonValueKind.Object ? failure.GetProperty("error_code").GetString() : null;
            // Preserve actual native source/publication failures over generic worker IoFailure/Cancelled.
            var errorCode = terminal.Category is "success" or "succeeded_with_warnings" ? null
                : terminal.Category is "worker_failure" or "resource_limit" ? failureCode ?? terminal.Category
                : terminal.Category;
            var budget = ArchiveWorkerBudget.Approve(command);
            var job = command.JobExtract;
            var outputEffects = new {
                existed_before = existedBefore, exists_after = Directory.Exists(command.OutputRoot),
                preflight_source_checks = audit.PreflightChecks, preflight_root_effects = audit.PreflightEffects,
                native_write_count = audit.Writes, native_write_bytes = audit.WriteBytes, native_publications = audit.Published,
                partials_remaining = Directory.Exists(command.OutputRoot) ? Directory.GetFiles(command.OutputRoot!, ".nxpart-*", SearchOption.AllDirectories).Length : 0
            };
            result = new {
                operation_id = command.OperationId, cleanup_complete = terminal.CleanupComplete, files_committed = terminal.FilesCommitted,
                error_code = errorCode,
                // Exact original WorkerFrame, not a guessed/reconstructed shortage.
                shortfall = shortfall.ValueKind == JsonValueKind.Object ? (JsonElement?)shortfall.GetProperty("output_budget_shortfall").Clone() : null,
                audit = new {
                    attempt, started = events.Count(e => e.Kind == "started"), active = runner.ActiveCount,
                    source_released = sourceReleased, helper_released = helperReleased, source_acquired = audit.SourceAcquired,
                    category = terminal.Category, native_error_code = terminal.NativeErrorCode, elapsed_ms = terminal.ElapsedMs,
                    bytes_committed = terminal.BytesCommitted, commit_state = terminal.CommitState,
                    files_skipped = terminal.FilesSkipped, journal = RarFixtureAudit.SafeJournal(terminal),
                    codepage = command.Codepage, conflict_policy = command.ConflictPolicy, copy_motw = command.ExtractOptions?.CopyMotw ?? true,
                    extract_options_present = command.ExtractOptions is not null,
                    has_password = command.Password is not null, password_interaction = command.PasswordInteraction,
                    job_execution = command.JobExecution,
                    job_extract = job is null ? null : (object)new {
                        strategy = job.Strategy, single_file_folder = job.SingleFileFolder, conflict = job.Conflict, copy_motw = job.CopyMotw,
                        selection_ordinals = job.Selection?.Ordinals,
                        snapshot_sha256 = Convert.ToHexString(SHA256.HashData(JsonSerializer.SerializeToUtf8Bytes(job, JsonDefaults.Options))).ToLowerInvariant()
                    },
                    limits = budget.Limits, memory_budget = budget.Memory, output_readmission = command.OutputReadmission,
                    native_count = audit.NativeCount, native_count_kind = "original_control_native_result_written",
                    output_exists = Directory.Exists(command.OutputRoot), output_effects = outputEffects,
                    cancel_after_commit_accepted = audit.CancelAccepted, checkpoints = audit.Checkpoints.ToArray(),
                    events = events.Select(RarFixtureAudit.SafeEvent).ToArray(), diagnostics = audit.Logs.ToArray()
                }
            };
            File.WriteAllText(Path.Combine(root, "attempt-result.json"), JsonSerializer.Serialize(result));
            return result;
        }
        finally {
            // Covers malformed control, EOF, deadline/callback/transport failure. Never create a
            // replacement worker or recursively delete evidence; original Run owns stop/Job cleanup.
            releaseCleanup.Set(); runner.Stop(); var drained = await runner.DrainAsync();
            File.WriteAllText(Path.Combine(root, "drain.json"), JsonSerializer.Serialize(new {
                operation_id = command.OperationId, drained, active = runner.ActiveCount, result_written = result is not null,
                source_released = RarFixturePackage.IsReleased(command.Path), helper_released = RarFixturePackage.IsReleased(package.Helper.Executable),
                source_acquired = audit.SourceAcquired, native_count = audit.NativeCount, native_write_bytes = audit.WriteBytes,
                native_publications = audit.Published, cleanup_gate_timed_out = Volatile.Read(ref cleanupTimedOut) != 0,
                checkpoints = audit.Checkpoints.ToArray(), journal = RarFixtureAudit.SafeJournal(events.LastOrDefault(e => e.Kind == "exited")),
                events = events.Select(RarFixtureAudit.SafeEvent).ToArray(), diagnostics = audit.Logs.ToArray()
            }));
            RarRiskReadmissionTests.Check(drained, "original cleanup incomplete; all RAR fixture evidence retained");
        }
    }

    private static JsonElement? Optional(JsonElement request, string name) => request.TryGetProperty(name, out var value)
        && value.ValueKind != JsonValueKind.Null ? value.Clone() : null;

    // One reader and one original request/control sequence. Hard 1MiB frames, whole-frame 10s
    // deadlines, and WaitAsync also bounds waiting when a redirected stdio stream ignores cancellation.
    private static async Task<JsonElement?> ReadAsync(Stream stream)
    {
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        var header = new byte[4];
        if (await stream.ReadAsync(header.AsMemory(0, 1), deadline.Token).AsTask().WaitAsync(deadline.Token) == 0) return null;
        await stream.ReadExactlyAsync(header.AsMemory(1), deadline.Token).AsTask().WaitAsync(deadline.Token);
        var size = BinaryPrimitives.ReadUInt32LittleEndian(header);
        if (size is 0 or > 1048576) throw new InvalidDataException("Invalid fixture frame size.");
        var body = new byte[size];
        try {
            await stream.ReadExactlyAsync(body.AsMemory(), deadline.Token).AsTask().WaitAsync(deadline.Token);
            using var document = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 32 });
            return document.RootElement.Clone();
        }
        finally { CryptographicOperations.ZeroMemory(body); }
    }
    private static async Task WriteAsync(Stream stream, object packet)
    {
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        var body = JsonSerializer.SerializeToUtf8Bytes(packet); var header = new byte[4];
        if (body.Length is 0 or > 1048576) throw new InvalidDataException("Fixture response exceeds frame limit.");
        BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)body.Length);
        await stream.WriteAsync(header.AsMemory(), deadline.Token).AsTask().WaitAsync(deadline.Token);
        await stream.WriteAsync(body.AsMemory(), deadline.Token).AsTask().WaitAsync(deadline.Token);
        await stream.FlushAsync(deadline.Token).WaitAsync(deadline.Token);
    }
}
