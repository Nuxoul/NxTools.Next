using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Phase A — Engine contracts and safety foundation. Deliberately unregistered; the parent
// owns exact test names/effects and the matching isolated-codecs worker/independent fixtures.
// No GUI, Shell, installation, production helper replacement, approval or retry policy.
internal static class TarStreamSafetyTests
{
    internal static void Check(bool value, string detail)
    { if (!value) throw new InvalidOperationException(detail); }

    internal static async Task Until(Func<bool> condition, string detail)
    {
        var clock = Stopwatch.StartNew();
        while (!condition()) {
            Check(clock.ElapsedMilliseconds < 10000, detail);
            await Task.Delay(10);
        }
    }

    internal static readonly TarStreamFixtureSpec[] Tar = [
        new("plain.tar", "Tar", true), new("plain.tgz", "TarGz", true),
        new("plain.tbz2", "TarBz2", true), new("plain.txz", "TarXz", true),
        new("plain.tzst", "TarZstd", true)
    ];
    internal static readonly TarStreamFixtureSpec[] Streams = [
        new("single.gz", "Gzip", false), new("single.bz2", "Bzip2", false),
        new("single.xz", "Xz", false), new("single.zst", "Zstd", false)
    ];
    internal static byte[] CyclicPayload() => Enumerable.Range(0, 4096).Select(i => (byte)(i % 256)).ToArray();
    private static byte[] Payload(string name) => name == "alpha.bin" ? CyclicPayload()
        : name == "beta.bin" ? Enumerable.Repeat((byte)'B', 2048).ToArray()
        : throw new InvalidDataException("Unexpected independently specified TAR member.");
    internal static JsonElement Binding(Guid id) => JsonSerializer.SerializeToElement(new {
        logical_id = id, expected_source = (object?)null
    });

    public static async Task ActualLowTotalPreflight()
    {
        var package = TarStreamFixturePackage.Create("low_total_preflight");
        var failures = new List<string>();
        // Collect all five real native results even while source admission is still RED.
        foreach (var spec in Tar) {
            try {
                await using var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-total1", spec);
                Check(!Directory.Exists(attempt.Output), "fresh extraction root absent");
                Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936,
                    TarStreamFixtureAttempt.Limits(8192, 1), OutputRoot: attempt.Output,
                    OutputReadmission: Binding(attempt.Id), ExtractOptions: new(false))), "original low-total native admission");
                var terminal = await attempt.TerminalAsync();
                var shortages = attempt.Frames("output_budget_shortfall");
                Check(shortages.Length == 1, "total1 must reach original TAR declaration rather than reject encoded source");
                var shortfall = shortages.Single().GetProperty("output_budget_shortfall");
                Check(shortfall.GetProperty("current").GetProperty("entry_bytes").GetUInt64() == 8192
                    && shortfall.GetProperty("current").GetProperty("output_bytes").GetUInt64() == 1
                    && shortfall.GetProperty("required").GetProperty("entry_bytes").GetUInt64() == 4096
                    && shortfall.GetProperty("required").GetProperty("output_bytes").GetUInt64() == 6144,
                    "unchanged admission and independent largest4096/total6144 declaration");
                var source = shortfall.GetProperty("source");
                using (var stream = File.OpenRead(attempt.Source))
                    Check(source.GetProperty("digest").EnumerateArray().Select(b => b.GetByte()).SequenceEqual(SHA256.HashData(stream)), "original Rust digest binds actual source bytes");
                using (var lease = ArchiveSourceLease.Open(attempt.Source))
                    Check(source.GetProperty("native").GetString() == lease.ReadmissionIdentity(), "original native source identity/version binding");
                NoPublished(attempt, terminal);
                Check(terminal.Category == "resource_limit" && Failure(attempt) == "ResourceLimit"
                    && attempt.Audit.SourceAcquired && attempt.Audit.PreflightChecks > 0
                    && attempt.Audit.PreflightEffects == 0 && attempt.Audit.Writes == 0
                    && (terminal.Journal?.Count ?? 0) == 0 && !Directory.Exists(attempt.Output), "reliable declaration shortfall precedes all native writes/root effects");
                NoRisk(attempt);
            }
            catch (Exception failure) {
                failures.Add(spec.Format + ":" + failure.GetType().Name);
                File.WriteAllText(Path.Combine(package.Root, spec.Format + "-failure.json"), JsonSerializer.Serialize(new {
                    format = spec.Format, exception_type = failure.GetType().Name,
                    assertion = failure is InvalidOperationException ? failure.Message : null, retained = true
                }));
            }
        }
        File.WriteAllText(Path.Combine(package.Root, "low-total-results.json"), JsonSerializer.Serialize(new {
            scenarios = Tar.Select(s => s.Format).ToArray(), failures, retained = true
        }));
        Check(failures.Count == 0, "TAR/compound native low-total failures (all five collected): " + string.Join(", ", failures));
    }

    public static async Task ActualMetadataAndNoRisk()
    {
        var package = TarStreamFixturePackage.Create("metadata_and_no_risk");
        // Nine listings; one extraction per actual codec/compound boundary, not a resource matrix.
        foreach (var spec in Tar.Concat(Streams)) {
            await using (var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-open", spec)) {
                Check(attempt.Start(new(attempt.Id, "open", attempt.Source, 936, TarStreamFixtureAttempt.Limits())), "original metadata admission");
                var terminal = await attempt.TerminalAsync();
                Check(terminal.Category == "success", "ordinary open success"); NoPublished(attempt, terminal); NoRisk(attempt);
                Check(attempt.Audit.Writes == 0 && !Directory.Exists(attempt.Output), "open has no output authority");
                var batches = attempt.Frames("entry_batch");
                Check(batches.Length > 0 && batches.All(f => f.GetProperty("format").GetString() == spec.Format), "actual worker format attribution");
                var entries = batches.SelectMany(f => f.GetProperty("entries").EnumerateArray()).ToArray();
                Check(entries.Length == (spec.IsTar ? 2 : 1), "exact independent logical member count");
                foreach (var entry in entries) {
                    Check(entry.GetProperty("kind").GetString() == "File" && !entry.GetProperty("encrypted").GetBoolean(), "ordinary regular unencrypted member");
                    var name = entry.GetProperty("name").GetString()!;
                    if (spec.IsTar) {
                        Check(name is "alpha.bin" or "beta.bin", "independent TAR member name");
                        Check(entry.GetProperty("size").GetUInt64() == (ulong)Payload(name).Length
                            && entry.GetProperty("packed_size").ValueKind == JsonValueKind.Null,
                            "TAR regular sizes known; outer compressed bytes never attributed per member");
                    } else {
                        Check(name == Path.GetFileNameWithoutExtension(spec.FileName)
                            && entry.GetProperty("size").ValueKind == JsonValueKind.Null
                            && entry.GetProperty("packed_size").GetUInt64() == (ulong)new FileInfo(attempt.Source).Length,
                            "standalone public size remains unknown, including XZ/Zstd; exact single encoded stream packed bytes");
                    }
                }
                if (spec.IsTar) Check(entries.Select(e => e.GetProperty("name").GetString()).Distinct().Count() == 2, "both independent TAR members listed");
                File.WriteAllText(Path.Combine(attempt.Root, "metadata-observation.json"), JsonSerializer.Serialize(new {
                    format = spec.Format, fields = entries.Select(e => new {
                        name = e.GetProperty("name").GetString(), kind = e.GetProperty("kind").GetString(),
                        size = e.GetProperty("size").Clone(), packed_size = e.GetProperty("packed_size").Clone(), encrypted = false
                    }).ToArray()
                }));
            }
            await using (var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-extract", spec)) {
                Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(),
                    OutputRoot: attempt.Output, ConflictPolicy: "overwrite", ExtractOptions: new(false))), "ordinary original native extraction");
                var terminal = await attempt.TerminalAsync();
                var count = spec.IsTar ? 2 : 1; var bytes = spec.IsTar ? 6144UL : 4096UL;
                Check(terminal.Category == "success" && terminal.FilesCommitted == count && terminal.BytesCommitted == bytes
                    && attempt.Audit.Published == count && attempt.Audit.WriteBytes == (long)bytes, "exact ordinary native bytes/publications");
                var files = Directory.GetFiles(attempt.Output, "*", SearchOption.AllDirectories);
                Check(files.Length == count, "no additional native files");
                if (spec.IsTar) foreach (var name in new[] { "alpha.bin", "beta.bin" }) AssertHash(Path.Combine(attempt.Output, name), Payload(name));
                else AssertHash(Path.Combine(attempt.Output, Path.GetFileNameWithoutExtension(spec.FileName)), CyclicPayload());
                PublishedJournal(terminal, count, bytes); NoPartials(attempt); NoRisk(attempt);
                Check(attempt.Frames("output_budget_shortfall").Length == 0, "ordinary extraction never creates a budget prompt");
            }
        }
        // Representative no-output TESTs: TAR structure, full outer compound and single stream.
        foreach (var spec in new[] { Tar[0], Tar[3], Streams[3] }) {
            await using var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-test", spec);
            Check(attempt.Start(new(attempt.Id, "test", attempt.Source, 936, TarStreamFixtureAttempt.Limits())), "original no-output TEST admission");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "success", "ordinary TEST success"); NoPublished(attempt, terminal); NoRisk(attempt);
            Check(attempt.Audit.Writes == 0 && !Directory.Exists(attempt.Output), "TEST has no extraction effects");
            var complete = attempt.Frames("complete").Single();
            Check(complete.GetProperty("files_verified").GetUInt64() == (spec.IsTar ? 2UL : 1UL)
                && complete.GetProperty("bytes_verified").GetUInt64() == (spec.IsTar ? 6144UL : 4096UL), "exact independent TEST counts (TAR is not a payload checksum)");
        }
    }

    public static async Task ActualUnknownAndCorrupt()
    {
        var package = TarStreamFixturePackage.Create("unknown_and_corrupt");
        foreach (var spec in new[] { Streams[0], Streams[1], new TarStreamFixtureSpec("unknown.zst", "Zstd", false), new TarStreamFixtureSpec("mixed.zst", "Zstd", false) }) {
            await using var attempt = new TarStreamFixtureAttempt(package, spec.FileName + "-actual-overflow", spec);
            Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(spec.FileName == "mixed.zst" ? 6144UL : 3072UL, 1UL << 20),
                OutputRoot: attempt.Output, OutputReadmission: Binding(attempt.Id), ExtractOptions: new(false))), "unknown-total native actual-byte admission");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "resource_limit" && Failure(attempt) == "ResourceLimit"
                && attempt.Audit.SourceAcquired && attempt.Audit.PreflightChecks > 0 && Directory.Exists(attempt.Output),
                "unknown declaration passes original preflight then actual decoder counters reject the admitted entry limit");
            NoPublished(attempt, terminal); NoRisk(attempt);
            Check(attempt.Frames("output_budget_shortfall").Length == 0, "Gzip/Bzip2/unknown or mixed Zstd cannot finance a guessed retry");
        }
        foreach (var spec in Tar.Skip(1)) {
            await using var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-metadata2048", spec);
            Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(8192, 1, 2048),
                OutputRoot: attempt.Output, OutputReadmission: Binding(attempt.Id), ExtractOptions: new(false))), "finite compound metadata/spool admission");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "resource_limit" && Failure(attempt) == "ResourceLimit" && attempt.Audit.SourceAcquired
                && attempt.Audit.PreflightChecks == 0 && attempt.Audit.Writes == 0 && !Directory.Exists(attempt.Output),
                "legal metadata2048 reservation rejects full outer spool before inner declaration/target effects");
            NoPublished(attempt, terminal); NoRisk(attempt);
            Check(attempt.Frames("output_budget_shortfall").Length == 0, "incomplete compound discovery never invents known output");
        }
        foreach (var spec in new[] { Streams[0], Tar[1] }) {
            // Explicit structural mutation, not an independent valid producer or invented ratio.
            await using var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-bad-crc", spec, path => {
                var data = File.ReadAllBytes(path); Check(data.Length >= 18, "complete gzip fixture framing");
                data[^8] ^= 1; File.WriteAllBytes(path, data);
            });
            Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(),
                OutputRoot: attempt.Output, OutputReadmission: Binding(attempt.Id), ExtractOptions: new(false))), "bad-CRC original extraction admission");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "worker_failure" && Failure(attempt) == "CorruptArchive", "actual outer CRC rejection");
            NoPublished(attempt, terminal); NoRisk(attempt);
            Check(attempt.Frames("output_budget_shortfall").Length == 0, "corruption cannot authorize readmission");
            if (spec.IsTar) Check(attempt.Audit.Writes == 0 && !Directory.Exists(attempt.Output), "bad compound integrity precedes all inner publication/root effects");
        }
    }

    public static async Task ActualPostcommitCancellation()
    {
        var package = TarStreamFixturePackage.Create("postcommit_cancellation");
        await using var attempt = new TarStreamFixtureAttempt(package, "Tar-cancel-first-native-publication", Tar[0]);
        attempt.Audit.CancelAfterCommit = true;
        Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(),
            OutputRoot: attempt.Output, OutputReadmission: Binding(attempt.Id), ConflictPolicy: "overwrite", ExtractOptions: new(false))), "postcommit native cancellation admission");
        var terminal = await attempt.TerminalAsync();
        Check(attempt.Audit.CancelAccepted && terminal.Category == "cancelled_with_committed_output"
            && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed" && attempt.Audit.Published == 1,
            "first actual native publication retained after shared cancellation; one original attempt");
        // TAR stages both tokens; commit order is token-ordered, NOT assumed to be archive order.
        Check(attempt.Audit.WriteBytes == 6144, "both independent TAR payloads staged before publication");
        var files = Directory.GetFiles(attempt.Output, "*", SearchOption.AllDirectories);
        Check(files.Length == 1, "exactly one published output retained, unpublished token cleaned");
        var name = Path.GetFileName(files.Single()); var expected = Payload(name);
        AssertHash(files.Single(), expected);
        Check(terminal.BytesCommitted == (ulong)expected.Length, "retained native byte count matches whichever token actually committed");
        PublishedJournal(terminal, 1, (ulong)expected.Length); NoPartials(attempt); NoRisk(attempt);
        Check(attempt.Frames("output_budget_shortfall").Length == 0 && File.Exists(attempt.Source), "no upgrade/rerun/rollback/source deletion after publication");
    }

    internal static string? Failure(TarStreamFixtureAttempt attempt) => attempt.Frames("failure").Single().GetProperty("error_code").GetString();
    internal static void NoRisk(TarStreamFixtureAttempt attempt) => Check(attempt.Frames("risk_prompt").Length == 0
        && attempt.Frames("risk_confirmed").Length == 0, "no fabricated tiny risk ratio or C# risk approval");
    internal static void NoPartials(TarStreamFixtureAttempt attempt) => Check(!Directory.Exists(attempt.Output)
        || Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "owned native partials actually cleaned");
    internal static void NoPublished(TarStreamFixtureAttempt attempt, ArchiveWorkerEvent terminal)
    {
        Check(terminal.FilesCommitted == 0 && terminal.BytesCommitted == 0 && attempt.Audit.Published == 0
            && !(terminal.Journal?.Any(j => j.Published == true) ?? false), "zero native publication/commits");
        NoPartials(attempt);
        Check(!Directory.Exists(attempt.Output) || Directory.GetFiles(attempt.Output, "*", SearchOption.AllDirectories).Length == 0, "no invalid final output");
    }
    private static void AssertHash(string path, byte[] literal) => Check(File.ReadAllBytes(path).SequenceEqual(literal)
        && TarStreamFixturePackage.Hash(path) == Convert.ToHexString(SHA256.HashData(literal)).ToLowerInvariant(), "independent literal bytes and exact SHA256");
    private static void PublishedJournal(ArchiveWorkerEvent terminal, int count, ulong bytes)
    {
        var published = terminal.Journal?.Where(j => j.Published == true).ToArray() ?? [];
        Check(published.Length == count && published.Aggregate(0UL, (n, j) => n + j.Bytes) == bytes
            && published.All(j => j.State == "Committed" && j.Cleanup is ("retained" or "retained_replacement")), "native retained journal exactly matches publication truth");
    }
}

internal sealed record TarStreamFixtureSpec(string FileName, string Format, bool IsTar);

internal sealed class TarStreamFixturePackage
{
    internal string Root { get; }
    internal string Fixtures { get; }
    internal ArchiveReadHelper Helper { get; }
    private TarStreamFixturePackage(string root, string fixtures, ArchiveReadHelper helper)
    { Root = root; Fixtures = fixtures; Helper = helper; }

    internal static TarStreamFixturePackage Create(string label, bool requireFixtures = true)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var evidence = EvidenceRoot();
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_TAR_STREAM_FIXTURES");
        if (string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker)
            || requireFixtures && (string.IsNullOrWhiteSpace(fixtures) || !Directory.Exists(fixtures) || !IsWithin(fixtures, evidence)))
            throw new TestSkippedException("Explicit matching isolated worker and independent TAR/stream fixtures inside current scratch evidence required.");
        var root = Directory.CreateDirectory(Path.Combine(evidence, "archive-product-" + Guid.NewGuid())).FullName;
        var deployment = Directory.CreateDirectory(Path.Combine(root, "package")).FullName;
        ArchiveReadHelper.GuardOwnedRoot(deployment);
        var destination = Path.Combine(deployment, ArchiveReadHelper.DeploymentName);
        File.Copy(worker, destination, overwrite: false);
        var helper = ArchiveReadHelper.OwnedPackage(deployment);
        var originalHash = Hash(worker);
        using (helper.Acquire()) TarStreamSafetyTests.Check(Hash(destination) == originalHash, "matching owned helper fingerprint under original native lease");
        File.WriteAllText(Path.Combine(root, "attribution.json"), JsonSerializer.Serialize(new {
            scenario = label, worker_path = Path.GetFullPath(worker), worker_sha256 = originalHash,
            staged_helper_path = destination, staged_helper_sha256 = Hash(destination), retained = true
        }));
        return new(root, fixtures ?? "", helper);
    }

    internal static string EvidenceRoot()
    {
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_TAR_STREAM_EVIDENCE_ROOT");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        if (string.IsNullOrWhiteSpace(evidence) || string.IsNullOrWhiteSpace(scratch)
            || !Directory.Exists(evidence) || !Directory.Exists(scratch) || !IsWithin(evidence, scratch))
            throw new TestSkippedException("Explicit existing current TAR/stream scratch evidence root required.");
        return Path.GetFullPath(evidence);
    }
    internal static string Hash(string path) { using var stream = File.OpenRead(path); return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant(); }
    internal static bool IsWithin(string path, string root) => Path.IsPathFullyQualified(path) && Path.IsPathFullyQualified(root)
        && Path.GetFullPath(path).StartsWith(Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
    internal static bool IsProtected(string path)
    {
        try { using var stream = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); return false; }
        catch (IOException failure) when ((failure.HResult & 0xffff) is 32 or 33) { return true; }
    }
    internal static bool IsReleased(string path)
    {
        try { using var stream = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); return true; }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
    }
    internal static string SuffixFormat(string path)
    {
        var name = path.ToLowerInvariant();
        if (name.EndsWith(".tar.gz", StringComparison.Ordinal) || name.EndsWith(".tgz", StringComparison.Ordinal)) return "TarGz";
        if (name.EndsWith(".tar.bz2", StringComparison.Ordinal) || name.EndsWith(".tbz2", StringComparison.Ordinal)) return "TarBz2";
        if (name.EndsWith(".tar.xz", StringComparison.Ordinal) || name.EndsWith(".txz", StringComparison.Ordinal)) return "TarXz";
        if (name.EndsWith(".tar.zst", StringComparison.Ordinal) || name.EndsWith(".tar.zstd", StringComparison.Ordinal) || name.EndsWith(".tzst", StringComparison.Ordinal)) return "TarZstd";
        return Path.GetExtension(name) switch {
            ".tar" => "Tar", ".gz" => "Gzip", ".bz2" => "Bzip2", ".xz" => "Xz", ".zst" => "Zstd",
            _ => throw new InvalidDataException("Only explicit TAR/stream fixture suffixes are admitted.")
        };
    }
    internal static string SignatureFormat(Stream stream, string path)
    {
        var format = SuffixFormat(path); var header = new byte[512];
        var length = 0;
        while (length < header.Length) { var count = stream.Read(header, length, header.Length - length); if (count == 0) break; length += count; }
        // Compound attribution follows the original suffix choice plus actual outer signature;
        // only the original worker validates the inner TAR/full outer integrity, never this host.
        var valid = format switch {
            "Tar" => length == 512 && header.AsSpan(257, 5).SequenceEqual("ustar"u8),
            "TarGz" or "Gzip" => length >= 3 && header.AsSpan(0, 3).SequenceEqual(new byte[] { 0x1f, 0x8b, 8 }),
            "TarBz2" or "Bzip2" => length >= 4 && header.AsSpan(0, 3).SequenceEqual("BZh"u8) && header[3] is >= (byte)'1' and <= (byte)'9',
            "TarXz" or "Xz" => length >= 6 && header.AsSpan(0, 6).SequenceEqual(new byte[] { 0xfd, 0x37, 0x7a, 0x58, 0x5a, 0 }),
            "TarZstd" or "Zstd" => length >= 4 && header.AsSpan(0, 4).SequenceEqual(new byte[] { 0x28, 0xb5, 0x2f, 0xfd }),
            _ => false
        };
        TarStreamSafetyTests.Check(valid, "actual fixture signature must agree with original TAR/stream format choice");
        return format;
    }
}

internal sealed class TarStreamFixtureAudit : IPlatformLog
{
    internal readonly ConcurrentQueue<object> Logs = new();
    internal readonly ConcurrentQueue<string> Checkpoints = new();
    private long writeBytes;
    private int writes, published, nativeCount, sourceAcquired, preflightChecks, preflightEffects;
    internal long WriteBytes => Interlocked.Read(ref writeBytes);
    internal int Writes => Volatile.Read(ref writes);
    internal int Published => Volatile.Read(ref published);
    internal int NativeCount => Volatile.Read(ref nativeCount);
    internal bool SourceAcquired => Volatile.Read(ref sourceAcquired) != 0;
    internal int PreflightChecks => Volatile.Read(ref preflightChecks);
    internal int PreflightEffects => Volatile.Read(ref preflightEffects);
    internal bool CancelAfterCommit, CancelAccepted;
    public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
    {
        // Do not retain request/log payloads, names, secrets or native tokens.
        Logs.Enqueue(new { name = eventName, operation_id = correlationId });
        if (eventName == "archive.worker.source_acquired") { Interlocked.Exchange(ref sourceAcquired, 1); Checkpoints.Enqueue("source_acquired"); }
    }
    internal void Install(ArchiveWorkerRunner runner, Guid id, string output)
    {
        runner.ControlInputCheckpointProbe = op => { if (op == "native_result") Interlocked.Increment(ref nativeCount); };
        // Generic TAR/stream paths do not emit RAR's source_validated/source_decoding callbacks.
        // Observe their original preflight source_precommit and real native writes instead.
        runner.SourceCheckpointProbe = stage => {
            Checkpoints.Enqueue(stage);
            if (stage == "source_precommit" && Writes == 0) {
                Interlocked.Increment(ref preflightChecks);
                if (Directory.Exists(output)) Interlocked.Increment(ref preflightEffects);
            }
        };
        runner.OutputWriteCheckpointProbe = count => {
            TarStreamSafetyTests.Check(SourceAcquired, "original held source acquisition precedes native writes");
            Interlocked.Add(ref writeBytes, count); Interlocked.Increment(ref writes);
        };
        runner.OutputCommitCheckpointProbe = stage => {
            if (stage != "native_closed") return;
            var count = Interlocked.Increment(ref published);
            if (CancelAfterCommit && count == 1) CancelAccepted = runner.Cancel(id);
        };
    }
    internal object Snapshot(ArchiveWorkerRequest command, ArchiveWorkerEvent? terminal, int active, bool existedBefore)
    {
        var budget = ArchiveWorkerBudget.Approve(command); var job = command.JobExtract; var output = command.OutputRoot;
        return new {
            codepage = command.Codepage, conflict_policy = command.ConflictPolicy, copy_motw = command.ExtractOptions?.CopyMotw ?? true,
            extract_options_present = command.ExtractOptions is not null, has_password = command.Password is not null,
            password_interaction = command.PasswordInteraction, job_execution = command.JobExecution,
            job_extract = job is null ? null : (object)new {
                strategy = job.Strategy, single_file_folder = job.SingleFileFolder, conflict = job.Conflict, copy_motw = job.CopyMotw,
                selection_ordinals = job.Selection?.Ordinals,
                snapshot_sha256 = Convert.ToHexString(SHA256.HashData(JsonSerializer.SerializeToUtf8Bytes(job, JsonDefaults.Options))).ToLowerInvariant()
            },
            limits = budget.Limits, memory_budget = budget.Memory, output_readmission = command.OutputReadmission,
            active, source_acquired = SourceAcquired, native_count = NativeCount, native_count_kind = "original_control_native_result_written",
            category = terminal?.Category, native_error_code = terminal?.NativeErrorCode, elapsed_ms = terminal?.ElapsedMs,
            files_committed = terminal?.FilesCommitted, bytes_committed = terminal?.BytesCommitted, commit_state = terminal?.CommitState,
            files_skipped = terminal?.FilesSkipped, journal = SafeJournal(terminal), cancel_after_commit_accepted = CancelAccepted,
            output_effects = new {
                existed_before = existedBefore, exists_after = Directory.Exists(output), preflight_source_checks = PreflightChecks,
                preflight_root_effects = PreflightEffects, native_write_count = Writes, native_write_bytes = WriteBytes,
                native_publications = Published,
                partials_remaining = Directory.Exists(output) ? Directory.GetFiles(output!, ".nxpart-*", SearchOption.AllDirectories).Length : 0
            }, checkpoints = Checkpoints.ToArray(), diagnostics = Logs.ToArray()
        };
    }
    internal static object SafeEvent(ArchiveWorkerEvent e) => new {
        kind = e.Kind, operation_id = e.OperationId, category = e.Category, native_error_code = e.NativeErrorCode,
        elapsed_ms = e.ElapsedMs, cleanup_complete = e.CleanupComplete, files_committed = e.FilesCommitted,
        bytes_committed = e.BytesCommitted, files_skipped = e.FilesSkipped, commit_state = e.CommitState,
        frame_op = e.Frame is { } f && f.TryGetProperty("op", out var op) ? op.GetString() : null,
        error_code = e.Frame is { } failure && failure.TryGetProperty("error_code", out var error) ? error.GetString() : null,
        shortfall = e.Frame is { } shortage && shortage.TryGetProperty("output_budget_shortfall", out var shortfall) ? shortfall.Clone() : (JsonElement?)null
    };
    internal static object[] SafeJournal(ArchiveWorkerEvent? terminal) => terminal?.Journal?.Select(j => (object)new {
        ordinal = j.Ordinal, state = j.State, replaced = j.Replaced, bytes = j.Bytes, published = j.Published, cleanup = j.Cleanup
    }).ToArray() ?? [];
}

internal sealed class TarStreamFixtureAttempt : IAsyncDisposable
{
    internal TarStreamFixturePackage Package { get; }
    internal Guid Id { get; } = Guid.NewGuid();
    internal string Root { get; }
    internal string Source { get; }
    internal string Output { get; }
    internal ConcurrentQueue<ArchiveWorkerEvent> Events { get; } = new();
    internal TarStreamFixtureAudit Audit { get; } = new();
    internal ArchiveWorkerRunner Runner { get; }
    private ArchiveWorkerRequest? command;
    private bool existedBefore;
    internal TarStreamFixtureAttempt(TarStreamFixturePackage package, string label, TarStreamFixtureSpec fixture, Action<string>? mutate = null)
    {
        Package = package; Root = Directory.CreateDirectory(Path.Combine(package.Root, label)).FullName;
        Source = Path.Combine(Root, fixture.FileName); Output = Path.Combine(Root, "output");
        var original = Path.Combine(package.Fixtures, fixture.FileName);
        TarStreamSafetyTests.Check(File.Exists(original), "required independent fixture missing: " + fixture.FileName);
        File.Copy(original, Source, overwrite: false); mutate?.Invoke(Source);
        using (var stream = File.OpenRead(Source)) TarStreamSafetyTests.Check(TarStreamFixturePackage.SignatureFormat(stream, Source) == fixture.Format, "actual signature attribution");
        File.WriteAllText(Path.Combine(Root, "source-attribution.json"), JsonSerializer.Serialize(new {
            fixture_path = original, source_path = Source, original_sha256 = TarStreamFixturePackage.Hash(original),
            source_sha256 = TarStreamFixturePackage.Hash(Source), format = fixture.Format, structural_mutation = mutate is not null,
            expected_regular_sizes = fixture.IsTar ? new[] { 4096, 2048 } : null
        }));
        Runner = ArchiveWorkerRunner.ForProductRead(package.Helper, Events.Enqueue, (id, frame, cancellation) => {
            Events.Enqueue(new("frame", id, Frame: frame.Clone()));
            return Task.FromResult<string?>(null); // Observe business frames only; original runner owns all native calls.
        }, Audit);
        Audit.Install(Runner, Id, Output);
    }
    internal static object Limits(ulong entry = 8192, ulong output = 1UL << 20, ulong metadata = 4UL << 20) => new {
        max_entries = 100UL, max_metadata_bytes = metadata, max_working_bytes = 64UL << 20,
        max_entry_bytes = entry, max_output_bytes = output
    };
    internal bool Start(ArchiveWorkerRequest request)
    {
        TarStreamSafetyTests.Check(command is null && request.OperationId == Id && request.Path == Source, "one original correlated attempt only");
        command = request; existedBefore = Directory.Exists(Output); return Runner.Start(request);
    }
    internal JsonElement[] Frames(string op) => Events.Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == op).Select(e => e.Frame!.Value).ToArray();
    internal async Task<ArchiveWorkerEvent> TerminalAsync()
    {
        await TarStreamSafetyTests.Until(() => Events.Any(e => e.Kind == "exited"), "actual original TAR/stream exited deadline");
        TarStreamSafetyTests.Check(await Runner.DrainAsync(), "actual original TAR/stream drain");
        var terminal = Events.Single(e => e.Kind == "exited");
        TarStreamSafetyTests.Check(terminal.CleanupComplete && Runner.ActiveCount == 0 && Events.Count(e => e.Kind == "started") == 1
            && Events.All(e => e.OperationId == Id), "one correlated original runner/Job, actual cleanup, no C# replacement");
        TarStreamSafetyTests.Check(Audit.NativeCount > 0 && Audit.NativeCount >= Audit.Writes + Audit.Published, "actual original native_result acknowledgements observed");
        TarStreamSafetyTests.Check(TarStreamFixturePackage.IsReleased(Source) && TarStreamFixturePackage.IsReleased(Package.Helper.Executable), "exclusive original source/helper reopen after actual cleanup");
        return terminal;
    }
    public async ValueTask DisposeAsync()
    {
        Runner.Stop(); var drained = await Runner.DrainAsync();
        try {
            var terminal = Events.LastOrDefault(e => e.Kind == "exited");
            File.WriteAllText(Path.Combine(Root, "events.json"), JsonSerializer.Serialize(new {
                operation_id = Id, drained, active = Runner.ActiveCount, started = Events.Count(e => e.Kind == "started"),
                source_released = TarStreamFixturePackage.IsReleased(Source), helper_released = TarStreamFixturePackage.IsReleased(Package.Helper.Executable),
                audit = command is null ? null : Audit.Snapshot(command, terminal, Runner.ActiveCount, existedBefore),
                events = Events.Select(TarStreamFixtureAudit.SafeEvent).ToArray(), retained = true
            }));
        }
        finally { Runner.Dispose(); }
        TarStreamSafetyTests.Check(drained, "original cleanup incomplete; all TAR/stream evidence retained");
    }
}
