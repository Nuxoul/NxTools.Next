using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Fixture-only transport for original Rust output_readmission::execute, never an approval,
// decoder, retry or process/Job implementation. Parent alone may register
// --tar-stream-output-readmission-host <owned-output-root> and NXTOOLS_TAR_STREAM_HOST.
internal static class TarStreamReadmissionHost
{
    internal static async Task<int> RunAsync(string outputRoot)
    {
        TarStreamFixturePackage? package = null;
        try {
            var mode = Environment.GetEnvironmentVariable("NXTOOLS_TAR_STREAM_HOST_MODE") ?? "none";
            if (mode is not ("none" or "cleanup_barrier" or "cancel_after_commit")) return 2;
            var evidence = TarStreamFixturePackage.EvidenceRoot();
            var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_TAR_STREAM_FIXTURES");
            if (!TarStreamFixturePackage.IsWithin(outputRoot, evidence)
                || !string.IsNullOrWhiteSpace(fixtures) && TarStreamFixturePackage.IsWithin(outputRoot, fixtures)) return 2;
            package = TarStreamFixturePackage.Create("output_readmission_host", requireFixtures: false);
            var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
            var attempts = 0; var captures = 0; string? originalSource = null;
            var ids = new HashSet<Guid>();
            while (await ReadAsync(input) is { } request) {
                if (request.TryGetProperty("fixture_control", out var fixture) && fixture.GetString() == "capture_source") {
                    if (++captures > 2) return 3;
                    var path = request.GetProperty("path").GetString()!;
                    ValidateSourcePath(path);
                    using var source = ArchiveSourceLease.Open(path);
                    using var stream = File.OpenRead(path);
                    var format = TarStreamFixturePackage.SignatureFormat(stream, path);
                    stream.Position = 0;
                    var digest = SHA256.HashData(stream).Select(b => (uint)b).ToArray();
                    var modified = checked((ulong)(File.GetLastWriteTimeUtc(path).Ticks - DateTime.UnixEpoch.Ticks) * 100);
                    // Original serializer and held native identity enable source-bound selection.
                    // Suffix+outer signature is not an integrity claim; the original worker owns it.
                    var browser = new ArchiveBrowserContext(Guid.NewGuid(), format, "", new(source.ReadmissionIdentity(), digest, (ulong)stream.Length, modified));
                    await WriteAsync(output, new { browser = JsonSerializer.SerializeToElement(browser, JsonDefaults.Options) });
                    continue;
                }
                if (request.TryGetProperty("fixture_control", out _) || ++attempts > 2) return 3;
                var command = Command(request, outputRoot);
                if (!ids.Add(command.OperationId)) return 3;
                if (originalSource is null) originalSource = command.Path;
                if (!string.Equals(originalSource, command.Path, StringComparison.OrdinalIgnoreCase)) return 3;
                // Never copy source_path between attempts or manufacture a replacement source binding.
                await WriteAsync(output, await AttemptAsync(package, command, attempts, mode, input, output));
            }
            return 0; // Every returned AttemptResult follows actual original exited/drain/release.
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
        if (!TarStreamFixturePackage.IsWithin(source, TarStreamFixturePackage.EvidenceRoot()))
            throw new InvalidDataException("Only explicit source paths inside current fixture evidence are admitted.");
        _ = TarStreamFixturePackage.SuffixFormat(source);
        // No stream read or extra decode on every command: original runner/source owner validates
        // the actual format and expected_source; capture_source alone inspects an outer signature.
    }

    private static ArchiveWorkerRequest Command(JsonElement request, string outputRoot)
    {
        if (request.GetProperty("schema").GetInt32() != 2 || request.GetProperty("op").GetString() != "start"
            || request.GetProperty("operation").GetString() != "extract") throw new InvalidDataException("Original extraction request required.");
        var id = request.GetProperty("operation_id").GetGuid();
        if (id == Guid.Empty) throw new InvalidDataException("A fresh original operation ID is required.");
        var source = request.GetProperty("path").GetString()!;
        ValidateSourcePath(source);
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

    private static async Task<object> AttemptAsync(TarStreamFixturePackage package, ArchiveWorkerRequest command, int attempt,
        string mode, Stream input, Stream output)
    {
        var root = Directory.CreateDirectory(Path.Combine(package.Root, "attempt-" + attempt)).FullName;
        File.WriteAllText(Path.Combine(root, "source-attribution.json"), JsonSerializer.Serialize(new {
            operation_id = command.OperationId, source_path = command.Path,
            source_sha256 = File.Exists(command.Path) ? TarStreamFixturePackage.Hash(command.Path) : null, output_path = command.OutputRoot
        }));
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        var audit = new TarStreamFixtureAudit { CancelAfterCommit = mode == "cancel_after_commit" };
        using var runner = ArchiveWorkerRunner.ForProductRead(package.Helper, events.Enqueue, (id, frame, cancellation) => {
            events.Enqueue(new("frame", id, Frame: frame.Clone()));
            return Task.FromResult<string?>(null); // Business frames only; never intercept native handling/approval.
        }, audit);
        audit.Install(runner, command.OperationId, command.OutputRoot!);
        using var cleanupReached = new ManualResetEventSlim(false);
        using var releaseCleanup = new ManualResetEventSlim(false);
        var cleanupTimedOut = 0;
        var cleanupBarrier = mode == "cleanup_barrier" && attempt == 1;
        if (cleanupBarrier) runner.CleanupCheckpointProbe = () => {
            // Callback is outside runner.gate; do not query ActiveCount from the callback.
            cleanupReached.Set();
            if (!releaseCleanup.Wait(10000)) Interlocked.Exchange(ref cleanupTimedOut, 1);
        };
        var existedBefore = Directory.Exists(command.OutputRoot);
        object? result = null;
        try {
            TarStreamSafetyTests.Check(runner.Start(command), "original finite TAR/stream native admission");
            if (cleanupBarrier) {
                try {
                    await TarStreamSafetyTests.Until(() => cleanupReached.IsSet || events.Any(e => e.Kind == "exited"), "original cleanup checkpoint deadline");
                    TarStreamSafetyTests.Check(cleanupReached.IsSet && runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "cleanup-held reservation retains active1 and no original exited");
                    var sourceProtected = TarStreamFixturePackage.IsProtected(command.Path);
                    var helperProtected = TarStreamFixturePackage.IsProtected(package.Helper.Executable);
                    TarStreamSafetyTests.Check(sourceProtected && helperProtected, "cleanup-held original source/helper remain protected");
                    var observation = new {
                        observation = "cleanup_held", operation_id = command.OperationId, active = runner.ActiveCount,
                        source_protected = sourceProtected, helper_protected = helperProtected, no_terminal = !events.Any(e => e.Kind == "exited"),
                        source_acquired = audit.SourceAcquired, checkpoints = audit.Checkpoints.ToArray()
                    };
                    File.WriteAllText(Path.Combine(root, "cleanup-held.json"), JsonSerializer.Serialize(observation));
                    await WriteAsync(output, observation);
                    var release = await ReadAsync(input);
                    TarStreamSafetyTests.Check(release is { } control && control.TryGetProperty("fixture_control", out var instruction)
                        && instruction.GetString() == "release_cleanup"
                        && (!control.TryGetProperty("operation_id", out var releaseId) || releaseId.GetGuid() == command.OperationId), "explicit matching stdio cleanup release required");
                    TarStreamSafetyTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "explicit release met finite cleanup gate");
                }
                finally { releaseCleanup.Set(); }
            }
            await TarStreamSafetyTests.Until(() => events.Any(e => e.Kind == "exited"), "actual original TAR/stream exited deadline");
            TarStreamSafetyTests.Check(await runner.DrainAsync(), "actual original TAR/stream drain");
            TarStreamSafetyTests.Check(Volatile.Read(ref cleanupTimedOut) == 0, "cleanup gate did not expire");
            var terminal = events.Single(e => e.Kind == "exited");
            TarStreamSafetyTests.Check(terminal.CleanupComplete && runner.ActiveCount == 0 && events.Count(e => e.Kind == "started") == 1
                && events.All(e => e.OperationId == command.OperationId), "one correlated original worker/Job, actual cleanup, no C# rerun");
            var sourceReleased = TarStreamFixturePackage.IsReleased(command.Path);
            var helperReleased = TarStreamFixturePackage.IsReleased(package.Helper.Executable);
            TarStreamSafetyTests.Check(sourceReleased && helperReleased, "exclusive source/helper reopen before original AttemptResult");
            var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
            var shortfall = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "output_budget_shortfall");
            var failure = frames.SingleOrDefault(f => f.GetProperty("op").GetString() == "failure");
            var failureCode = failure.ValueKind == JsonValueKind.Object ? failure.GetProperty("error_code").GetString() : null;
            var errorCode = terminal.Category is "success" or "succeeded_with_warnings" ? null
                : terminal.Category is "worker_failure" or "resource_limit" ? failureCode ?? terminal.Category : terminal.Category;
            if (shortfall.ValueKind == JsonValueKind.Object) TarStreamSafetyTests.Check(terminal.FilesCommitted == 0
                && terminal.BytesCommitted == 0 && audit.Writes == 0 && audit.Published == 0
                && (terminal.Journal?.Count ?? 0) == 0 && (existedBefore || !Directory.Exists(command.OutputRoot)),
                "original shortfall must precede writes/publication/root effects");
            // Explicit allowlist only: no secret-bearing request/frame/diagnostic serialization.
            var safeAudit = JsonSerializer.SerializeToNode(audit.Snapshot(command, terminal, runner.ActiveCount, existedBefore))!.AsObject();
            safeAudit["attempt"] = attempt; safeAudit["started"] = events.Count(e => e.Kind == "started");
            safeAudit["source_released"] = sourceReleased; safeAudit["helper_released"] = helperReleased;
            safeAudit["output_exists"] = Directory.Exists(command.OutputRoot);
            safeAudit["events"] = JsonSerializer.SerializeToNode(events.Select(TarStreamFixtureAudit.SafeEvent).ToArray());
            result = new {
                operation_id = command.OperationId, cleanup_complete = terminal.CleanupComplete, files_committed = terminal.FilesCommitted,
                error_code = errorCode,
                // Exact original WorkerFrame shortage, never guessed from a name/format/partial decode.
                shortfall = shortfall.ValueKind == JsonValueKind.Object ? (JsonElement?)shortfall.GetProperty("output_budget_shortfall").Clone() : null,
                audit = safeAudit
            };
            File.WriteAllText(Path.Combine(root, "attempt-result.json"), JsonSerializer.Serialize(result));
            return result;
        }
        finally {
            releaseCleanup.Set(); runner.Stop(); var drained = await runner.DrainAsync();
            File.WriteAllText(Path.Combine(root, "drain.json"), JsonSerializer.Serialize(new {
                operation_id = command.OperationId, drained, active = runner.ActiveCount, result_written = result is not null,
                source_released = TarStreamFixturePackage.IsReleased(command.Path), helper_released = TarStreamFixturePackage.IsReleased(package.Helper.Executable),
                cleanup_gate_timed_out = Volatile.Read(ref cleanupTimedOut) != 0,
                audit = audit.Snapshot(command, events.LastOrDefault(e => e.Kind == "exited"), runner.ActiveCount, existedBefore),
                events = events.Select(TarStreamFixtureAudit.SafeEvent).ToArray(), retained = true
            }));
            TarStreamSafetyTests.Check(drained, "original cleanup incomplete; all TAR/stream fixture evidence retained");
        }
    }

    private static JsonElement? Optional(JsonElement request, string name) => request.TryGetProperty(name, out var value)
        && value.ValueKind != JsonValueKind.Null ? value.Clone() : null;

    // One stdin owner, hard1MiB and whole-frame10s. WaitAsync bounds redirected streams
    // which do not honor cancellation; EOF/control/error still drains the original native runner.
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
