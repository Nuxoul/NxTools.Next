using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Private fixture stdio adapter for output_readmission::execute. No approval/retry/budget policy.
// Parent registration: --sevenz-output-readmission-host <owned-output-root>.
internal static class SevenzReadmissionHost
{
    internal static async Task<int> RunAsync(string outputRoot)
    {
        SevenzFixturePackage? package = null;
        try {
            var mode = Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_HOST_MODE") ?? "none";
            // No approximate native-output-limit simulation: that mode requires a real native seam.
            if (mode is not ("none" or "cleanup_barrier" or "cancel_after_commit")) return 2;
            var evidence = Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_EVIDENCE_ROOT");
            if (string.IsNullOrWhiteSpace(evidence) || !Path.IsPathFullyQualified(outputRoot)
                || !Path.GetFullPath(outputRoot).StartsWith(Path.GetFullPath(evidence).TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase)) return 2;
            package = SevenzFixturePackage.Create("output_readmission_host", requireFixtures: false);
            var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
            var attempts = 0;
            var ids = new HashSet<Guid>();
            while (await ReadAsync(input) is { } request) {
                if (request.TryGetProperty("fixture_control", out var fixture) && fixture.GetString() == "capture_source") {
                    var path = request.GetProperty("path").GetString()!;
                    var evidenceRoot = Path.GetFullPath(evidence).TrimEnd('\\') + "\\";
                    if (!Path.GetFullPath(path).StartsWith(evidenceRoot, StringComparison.OrdinalIgnoreCase)) return 3;
                    using var source = ArchiveSourceLease.Open(path);
                    using var stream = File.OpenRead(path);
                    var digest = SHA256.HashData(stream).Select(b => (uint)b).ToArray();
                    var modified = checked((ulong)(File.GetLastWriteTimeUtc(path).Ticks - DateTime.UnixEpoch.Ticks) * 100);
                    var browser = new ArchiveBrowserContext(Guid.NewGuid(), "SevenZ", "", new(source.ReadmissionIdentity(), digest, (ulong)stream.Length, modified));
                    await WriteAsync(output, new { browser = JsonSerializer.SerializeToElement(browser, JsonDefaults.Options) });
                    continue;
                }
                if (request.TryGetProperty("fixture_control", out _) || ++attempts > 2) return 3;
                var command = Command(request, outputRoot);
                if (!ids.Add(command.OperationId)) return 3;
                var result = await AttemptAsync(package, command, attempts, mode, input, output);
                await WriteAsync(output, result);
            }
            // Every attempt has already stopped and drained its original runner before reading EOF.
            return 0;
        }
        catch (Exception failure) {
            if (package is not null) File.WriteAllText(Path.Combine(package.Root, "host-failure.json"), JsonSerializer.Serialize(new {
                category = "fixture_host_failure", exception_type = failure.GetType().Name, retained = true
            }));
            return 5;
        }
    }

    private static ArchiveWorkerRequest Command(JsonElement request, string outputRoot)
    {
        if (request.GetProperty("schema").GetInt32() != 2 || request.GetProperty("op").GetString() != "start"
            || request.GetProperty("operation").GetString() != "extract") throw new InvalidDataException("Fixture requires original extraction request.");
        var id = request.GetProperty("operation_id").GetGuid();
        var source = request.GetProperty("path").GetString()!;
        if (id == Guid.Empty || !Path.IsPathFullyQualified(source) || !string.Equals(Path.GetExtension(source), ".7z", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Only explicit fixture 7z sources are admitted.");
        // Keep the actual original path: copying between attempts would invalidate native binding.
        var evidence = Path.GetFullPath(Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_EVIDENCE_ROOT")!).TrimEnd('\\') + "\\";
        var fixtureRoot = Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_FIXTURES");
        var ownedFixture = !string.IsNullOrWhiteSpace(fixtureRoot) && Path.IsPathFullyQualified(fixtureRoot)
            && Path.GetFullPath(source).StartsWith(Path.GetFullPath(fixtureRoot).TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase);
        if (!Path.GetFullPath(source).StartsWith(evidence, StringComparison.OrdinalIgnoreCase) && !ownedFixture)
            throw new InvalidDataException("Source must be self-owned fixture evidence.");
        var password = Optional(request, "password") is { } secret ? secret.GetString() : null;
        var policy = Optional(request, "conflict_policy") is { } conflict ? conflict.GetString() : null;
        var extract = Optional(request, "extract_options");
        var job = Optional(request, "job_extract");
        var jobExtract = job?.Deserialize<ArchiveJobExtract>(JsonDefaults.Options);
        var jobExecution = Optional(request, "job_execution")?.GetBoolean() ?? false;
        var passwordInteraction = Optional(request, "password_interaction")?.GetBoolean() ?? false;
        return new(id, "extract", source, request.GetProperty("codepage").GetInt32(), request.GetProperty("limits").Clone(),
            Password: password, OutputRoot: outputRoot,
            ExtractOptions: extract is { } options ? new(options.GetProperty("copy_motw").GetBoolean()) : null,
            PasswordInteraction: passwordInteraction, ConflictPolicy: policy,
            OutputReadmission: Optional(request, "output_readmission"), JobExecution: jobExecution, JobExtract: jobExtract);
    }

    private static async Task<object> AttemptAsync(SevenzFixturePackage package, ArchiveWorkerRequest command, int attempt,
        string mode, Stream input, Stream output)
    {
        var root = Directory.CreateDirectory(Path.Combine(package.Root, "attempt-" + attempt)).FullName;
        File.WriteAllText(Path.Combine(root, "source-attribution.json"), JsonSerializer.Serialize(new {
            operation_id = command.OperationId, source_path = command.Path,
            source_sha256 = File.Exists(command.Path) ? SevenzFixturePackage.Hash(command.Path) : null,
            output_path = command.OutputRoot
        }));
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var audit = new SevenzFixtureAudit { CancelAfterCommit = mode == "cancel_after_commit" };
        // JobExecution routes non-native frames to readFrame instead of Emit; observe and return
        // null even for failure/shortfall. Never bypass native handling or force a ReadContract.
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
            // Original callback is OUTSIDE runner.gate. Signal only; never call ActiveCount/stdio
            // here or hold the runner gate while waiting. Main request owns the only stdin reader.
            cleanupReached.Set();
            if (!releaseCleanup.Wait(10000)) Interlocked.Exchange(ref cleanupTimedOut, 1);
        };
        var existedBefore = Directory.Exists(command.OutputRoot);
        object? result = null;
        try {
            SevenzRiskReadmissionTests.Check(runner.Start(command), "original finite request admission");
            if (cleanupBarrier) {
                try {
                    await SevenzRiskReadmissionTests.Until(() => cleanupReached.IsSet || events.Any(e => e.Kind == "exited"), "cleanup checkpoint deadline");
                    SevenzRiskReadmissionTests.Check(cleanupReached.IsSet && runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "original cleanup still owns native reservation");
                    var sourceProtected = SevenzFixturePackage.IsProtected(command.Path);
                    var helperProtected = SevenzFixturePackage.IsProtected(package.Helper.Executable);
                    SevenzRiskReadmissionTests.Check(sourceProtected && helperProtected, "cleanup barrier retains original source/helper leases");
                    var observation = new {
                        observation = "cleanup_held", operation_id = command.OperationId, active = runner.ActiveCount,
                        source_protected = sourceProtected, helper_protected = helperProtected, no_terminal = !events.Any(e => e.Kind == "exited")
                    };
                    File.WriteAllText(Path.Combine(root, "cleanup-held.json"), JsonSerializer.Serialize(observation));
                    await WriteAsync(output, observation);
                    var release = await ReadAsync(input);
                    SevenzRiskReadmissionTests.Check(release is { } control && control.TryGetProperty("fixture_control", out var instruction)
                        && instruction.GetString() == "release_cleanup"
                        && (!control.TryGetProperty("operation_id", out var releaseId) || releaseId.GetGuid() == command.OperationId), "explicit cleanup release required on original stdin");
                    SevenzRiskReadmissionTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "cleanup release arrived within finite fixture gate");
                }
                finally { releaseCleanup.Set(); }
            }
            await SevenzRiskReadmissionTests.Until(() => events.Any(e => e.Kind == "exited"), "actual original exited deadline");
            SevenzRiskReadmissionTests.Check(await runner.DrainAsync(), "actual original runner drain");
            SevenzRiskReadmissionTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "finite cleanup gate did not expire before explicit release");
            var terminal = events.Single(e => e.Kind == "exited");
            SevenzRiskReadmissionTests.Check(terminal.CleanupComplete && runner.ActiveCount == 0 && events.Count(e => e.Kind == "started") == 1, "cleanup-bound original worker, no C# rerun");
            var sourceReleased = SevenzFixturePackage.IsReleased(command.Path);
            var helperReleased = SevenzFixturePackage.IsReleased(package.Helper.Executable);
            SevenzRiskReadmissionTests.Check(sourceReleased && helperReleased, "exclusive source/helper reopen before AttemptResult");
            var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
            var shortfall = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "output_budget_shortfall");
            var failure = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "failure");
            var failureCode = failure.ValueKind == JsonValueKind.Object ? failure.GetProperty("error_code").GetString() : null;
            // Native source/publication failures outrank a later generic worker IoFailure/Cancelled.
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
                // Exact original WorkerFrame payload, not a reconstructed/guessed shortage.
                shortfall = shortfall.ValueKind == JsonValueKind.Object ? (JsonElement?)shortfall.GetProperty("output_budget_shortfall").Clone() : null,
                audit = new {
                    attempt, started = events.Count(e => e.Kind == "started"), active = runner.ActiveCount,
                    source_released = sourceReleased, helper_released = helperReleased,
                    source_acquired = audit.SourceAcquired, category = terminal.Category, native_error_code = terminal.NativeErrorCode,
                    elapsed_ms = terminal.ElapsedMs, bytes_committed = terminal.BytesCommitted, commit_state = terminal.CommitState,
                    files_skipped = terminal.FilesSkipped, journal = SevenzFixtureAudit.SafeJournal(terminal),
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
                    cancel_after_commit_accepted = audit.CancelAccepted,
                    checkpoints = audit.Checkpoints.ToArray(), events = events.Select(SevenzFixtureAudit.SafeEvent).ToArray(), diagnostics = audit.Logs.ToArray()
                }
            };
            File.WriteAllText(Path.Combine(root, "attempt-result.json"), JsonSerializer.Serialize(result));
            return result;
        }
        finally {
            // Also covers malformed control, EOF, timeout, callback/transport failure; no recursive
            // deletion and no replacement process. Original Run owns cooperative/Job termination.
            releaseCleanup.Set(); runner.Stop(); var drained = await runner.DrainAsync();
            File.WriteAllText(Path.Combine(root, "drain.json"), JsonSerializer.Serialize(new {
                operation_id = command.OperationId, drained, active = runner.ActiveCount, result_written = result is not null,
                source_released = SevenzFixturePackage.IsReleased(command.Path), helper_released = SevenzFixturePackage.IsReleased(package.Helper.Executable),
                native_count = audit.NativeCount, native_write_bytes = audit.WriteBytes, native_publications = audit.Published,
                cleanup_gate_timed_out = Volatile.Read(ref cleanupTimedOut) != 0,
                events = events.Select(SevenzFixtureAudit.SafeEvent).ToArray(), diagnostics = audit.Logs.ToArray()
            }));
            SevenzRiskReadmissionTests.Check(drained, "original cleanup incomplete; all fixture evidence retained");
        }
    }

    private static JsonElement? Optional(JsonElement request, string name) => request.TryGetProperty(name, out var value)
        && value.ValueKind != JsonValueKind.Null ? value.Clone() : null;

    // One synchronous request/control sequence, one reader, no background stdin consumer.
    private static async Task<JsonElement?> ReadAsync(Stream stream)
    {
        using var deadline = new CancellationTokenSource(TimeSpan.FromSeconds(10));
        var header = new byte[4];
        if (await stream.ReadAsync(header.AsMemory(0, 1), deadline.Token) == 0) return null;
        await stream.ReadExactlyAsync(header.AsMemory(1), deadline.Token);
        var size = BinaryPrimitives.ReadUInt32LittleEndian(header);
        if (size is 0 or > 1048576) throw new InvalidDataException("Invalid fixture frame size.");
        var body = new byte[size];
        try {
            await stream.ReadExactlyAsync(body, deadline.Token);
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
        await stream.WriteAsync(header, deadline.Token); await stream.WriteAsync(body, deadline.Token); await stream.FlushAsync(deadline.Token);
    }
}
