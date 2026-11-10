using System.Diagnostics;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: marked GUID-owned PI_SCRATCH_DIR ZIP fixtures only; real SourceLease/OutputSession,
// held-partial readback, FILE-only handoff and native replacement. No worker/reference process,
// GUI, Shell, clipboard, registry, service, user archive, source deletion or reclamation.
internal static partial class Opt05NativeObservationTests
{
    private const int PayloadSize = 8 << 20;
    private const string Target = "editable.zip";
    private const string RootPrefix = "archive-product-opt05-native-";
    private static readonly string[] EquivalentStages =
        ["EditOpen", "EditVerification", "EditReadsComplete", "EditHandoff", "Prepublication"];
    private sealed record Phase(string Name, double ElapsedMs);
    private sealed record Trial(int Number, Guid OperationId, double OperationMs, double CleanupMs, Phase[] Phases,
        ArchiveNativeReadObservation.Counts[] SourceReads, ArchiveNativeReadObservation.Counts[] OutputReads,
        long PartialReadCalls, long PartialReadBytes, int NativePublications, bool FileOnlyHandoff,
        bool ReaderClosed, bool SourceExclusivelyReopenedAtHandoff, bool FinalExclusivelyReopened,
        bool CleanupComplete, string CommitState);
    private sealed class WorkLog : IPlatformLog
    {
        internal Guid? Correlation;
        internal IReadOnlyDictionary<string, object?>? Completed;
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (name != "archive.output.completed") return;
            Check(Completed is null, "one output completion diagnostic");
            Correlation = correlationId;
            Completed = fields;
        }
    }
    private static void Check(bool condition, string detail)
    { if (!condition) throw new InvalidOperationException(detail); }

    internal static Task EquivalentObservations()
    {
        // Test-only baseline control; production source/output owners never read this variable.
        var baseline = Environment.GetEnvironmentVariable("NXTOOLS_OPT05_NATIVE_BASELINE") == "1";
        var root = Root(baseline);
        var configuration = typeof(Opt05NativeObservationTests).Assembly.GetCustomAttribute<AssemblyConfigurationAttribute>()?.Configuration;
        if (!baseline)
        {
            using var before = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "measurements-before.json")));
            Check(before.RootElement.GetProperty("build").GetProperty("configuration").GetString() == configuration,
                "baseline and regression must use the same build configuration");
        }
        var payload = new byte[PayloadSize];
        new Random(505).NextBytes(payload);
        var original = CreateZip(payload, "original.bin");
        var candidate = CreateZip(payload, "renamed.bin");
        var fixture = Path.Combine(root, "fixture.zip");
        if (!File.Exists(fixture))
        {
            Check(baseline, "regression cannot regenerate the baseline fixture");
            File.WriteAllBytes(fixture, original);
        }
        Check(File.ReadAllBytes(fixture).AsSpan().SequenceEqual(original), "same retained baseline fixture bytes");
        VerifyZip(original, payload, "original.bin");
        VerifyZip(candidate, payload, "renamed.bin");
        Console.WriteLine("ARCHIVE_OPT05_NATIVE_PAIR " + JsonSerializer.Serialize(new {
            root, fixture, fixture_bytes = original.Length, configuration,
            host = typeof(ArchiveSourceLease).Assembly.Location, tests = typeof(Opt05NativeObservationTests).Assembly.Location,
            route = "ArchiveSourceLease/ArchiveOutputSession/native_rename_replace", worker = (string?)null,
            cache_policy = "no_system_cache_flush"
        }));
        var trials = new List<Trial>();
        for (var trial = 1; trial <= 3; trial++) trials.Add(RunTrial(root, fixture, original, candidate, payload, trial));
        var failures = trials.SelectMany(t => EquivalentStages.Select(stage => new {
            trial = t.Number, stage,
            reads = t.SourceReads.Concat(t.OutputReads).Where(c => c.Stage == stage).Sum(c => c.FullReads),
            bytes = t.SourceReads.Concat(t.OutputReads).Where(c => c.Stage == stage).Sum(c => c.CompletedBytes)
        })).Where(c => c.reads != 1 || c.bytes != original.Length).ToArray();
        var report = new {
            schema = 1, mode = baseline ? "before" : "regression", completed_trials = trials.Count,
            build = new { configuration, host = typeof(ArchiveSourceLease).Assembly.Location,
                tests = typeof(Opt05NativeObservationTests).Assembly.Location },
            fixture_bytes = original.Length, payload_bytes = PayloadSize,
            route = "ArchiveSourceLease/ArchiveOutputSession/native_rename_replace",
            cache_policy = "no_system_cache_flush", total_io_bytes = (long?)null,
            counter_semantics = "actual_stream_bytes; full_read_requires_EOF_and_digest_completion; source_target_and_output_roles_separate; partial_readback_separate",
            timing_semantics = "owned_native_route_with_fixture_staging_and_checkpoint_assertions; not_exclusive_codec_or_production_latency",
            median_operation_ms = trials.Select(t => t.OperationMs).Order().ElementAt(1),
            trials, equivalent_stage_failures = failures
        };
        var reportName = baseline ? "measurements-before.json" : "measurements-regression-" + Guid.NewGuid().ToString("N") + ".json";
        var reportPath = Path.Combine(root, reportName);
        using (var file = new FileStream(reportPath, FileMode.CreateNew, FileAccess.Write, FileShare.Read))
            JsonSerializer.Serialize(file, report, new JsonSerializerOptions { WriteIndented = true });
        Console.WriteLine("ARCHIVE_OPT05_NATIVE_REPORT " + reportPath);
        Check(trials.Count == 3 && trials.All(t => t.CleanupComplete && t.FinalExclusivelyReopened), "three completed native trials required");
        if (!baseline)
            Check(failures.Length == 0, "OPT05 each equivalent source/target must be fully read once per fresh native stage: " +
                JsonSerializer.Serialize(failures));
        return Task.CompletedTask;
    }

    internal static Task IdentityAndCancellation()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch), "absolute PI_SCRATCH_DIR required");
        var root = Directory.CreateDirectory(Path.Combine(scratch!, RootPrefix + Guid.NewGuid().ToString("D"))).FullName;
        File.WriteAllText(Path.Combine(root, "owned-opt05-native-fixture"), "OPT05 native identity and cancellation controls");
        var payload = new byte[PayloadSize]; new Random(505).NextBytes(payload);
        var original = CreateZip(payload, "original.bin");
        var candidate = CreateZip(payload, "renamed.bin");
        try
        {
            DifferentObjectCannotReuse(root, original, candidate);
            foreach (var complete in new[] { false, true }) SourceReadCancellation(root, original, complete);
            ChangedVersionCannotReuse(root, original, candidate);
            foreach (var mode in new[] { "handoff", "prepublication", "fingerprint", "ordinary_final_target" })
                CommitReadCancellation(root, original, candidate, mode);
            foreach (var begin in new[] { false, true }) TargetReadCancellation(root, original, begin);
            var cancelledSource = Path.Combine(root, "cancelled-open.zip");
            File.WriteAllBytes(cancelledSource, original);
            using var cancellation = new CancellationTokenSource(); cancellation.Cancel();
            ExpectCancellation(() => { using var lease = ArchiveSourceLease.OpenForEdit(cancelledSource, cancellation.Token); }, cancellation.Token);
            using (var reopened = File.Open(cancelledSource, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                Check(reopened.Length == original.Length, "cancelled acquisition leaves no source FILE lease");
        }
        finally { Directory.Delete(root, recursive: true); }
        Console.WriteLine("ARCHIVE_OPT05_NATIVE_CONTROLS identity_version=2 interrupted_reads=8 cancelled_open=1 cleanup_complete=true");
        return Task.CompletedTask;
    }

    private static void DifferentObjectCannotReuse(string root, byte[] original, byte[] candidate)
    {
        var parent = Directory.CreateDirectory(Path.Combine(root, "different-object")).FullName;
        var source = Path.Combine(parent, Target);
        var other = Path.Combine(parent, "other.zip");
        File.WriteAllBytes(source, original); File.WriteAllBytes(other, original);
        File.SetLastWriteTimeUtc(other, File.GetLastWriteTimeUtc(source));
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(parent, Guid.NewGuid());
        var sourceIdentity = output.CaptureTargetIdentity(Target);
        var otherIdentity = output.CaptureTargetIdentity("other.zip");
        var sourceFields = sourceIdentity.Split(':'); var otherFields = otherIdentity.Split(':');
        Check(sourceFields[2] != otherFields[2] && sourceFields[3] == otherFields[3] && sourceFields[5] == otherFields[5],
            "same-size same-bytes target with a different actual full native ID required");
        var token = StageControlCandidate(output, "other.zip", otherIdentity, candidate);
        var completion = lease.CompleteEditReads();
        try
        {
            output.CommitVerifiedEdit(token, lease, completion, CancellationToken.None, expectedSourceTargetIdentity: sourceIdentity);
            throw new InvalidOperationException("different protected target reused source digest");
        }
        catch (ArchiveOutputException failure) { Check(failure.Stage == "target_changed", "different-object target refusal"); }
        Check(!lease.EditFileReleased && !completion.IsConsumed && output.OutputPublished() != true,
            "different-object refusal must precede source handoff/publication");
        Check(File.ReadAllBytes(source).AsSpan().SequenceEqual(original) && File.ReadAllBytes(other).AsSpan().SequenceEqual(original),
            "different-object refusal preserves both actual files");
        Check(output.Cancel(token).Category == "partial_removed", "different-object candidate cleanup");
        RequireProtectedFile(source); RequireProtectedParent(parent);
        Check(!Directory.EnumerateFiles(parent, ".nxpart-*").Any(), "different-object owned partial retired");
        output.Dispose(); lease.Dispose();
        using var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
    }

    private static void ChangedVersionCannotReuse(string root, byte[] original, byte[] candidate)
    {
        var parent = Directory.CreateDirectory(Path.Combine(root, "changed-version")).FullName;
        var source = Path.Combine(parent, Target); File.WriteAllBytes(source, original);
        var attributes = File.GetAttributes(source);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(parent, Guid.NewGuid());
        var token = StageControlCandidate(output, Target, output.CaptureTargetIdentity(Target), candidate);
        var completion = lease.CompleteEditReads();
        // Metadata writes do not require data-write access. Version freshness must include them
        // even though the native object, encoded length and every payload byte are unchanged.
        File.SetAttributes(source, attributes ^ FileAttributes.Hidden);
        try
        {
            output.CommitVerifiedEdit(token, lease, completion, CancellationToken.None);
            throw new InvalidOperationException("changed source version reused an old observation");
        }
        catch (ArchiveOutputException failure) { Check(failure.Stage == "source_changed", "same-object version refusal"); }
        Check(!lease.EditFileReleased && !completion.IsConsumed && File.ReadAllBytes(source).AsSpan().SequenceEqual(original),
            "same-object version refusal preserves source and read-completion ownership");
        Check(output.Cancel(token).Category == "partial_removed", "changed-version candidate cleanup");
        RequireProtectedFile(source); RequireProtectedParent(parent);
        output.Dispose(); lease.Dispose();
        Check(output.OutputPublished() == false && !Directory.EnumerateFiles(parent, ".nxpart-*").Any(), "changed-version zero-publication cleanup");
        using (var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
            Check(reopened.Length == original.Length, "changed-version source reader actually closed");
        File.SetAttributes(source, attributes);
    }


    private static void SourceReadCancellation(string root, byte[] original, bool complete)
    {
        var parent = Directory.CreateDirectory(Path.Combine(root, complete ? "completion-cancel" : "verification-cancel")).FullName;
        var source = Path.Combine(parent, Target); File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(parent, Guid.NewGuid());
        var expected = output.CaptureTargetIdentity(Target);
        var stage = complete ? ArchiveNativeReadStage.EditReadsComplete : ArchiveNativeReadStage.EditVerification;
        using var cancellation = new CancellationTokenSource();
        var before = ReadCounts(lease.ReadObservation, stage, ArchiveNativeReadKind.Source);
        CancelFirstChunk(lease.ReadObservation, stage, ArchiveNativeReadKind.Source, cancellation);
        if (complete) ExpectCancellation(() => lease.CompleteEditReads(cancellation.Token), cancellation.Token);
        else ExpectCancellation(() => lease.VerifyEditSource(expected, cancellationToken: cancellation.Token), cancellation.Token);
        AssertInterrupted(lease.ReadObservation, stage, ArchiveNativeReadKind.Source, before);
        Check(!lease.EditFileReleased && output.CommitJournal().Count == 0, "interrupted source read grants no handoff/publication");
        RequireProtectedFile(source); RequireProtectedParent(parent);
        lease.ReadObservation.ReadChunkProbeForFixture = null;
        Check(!lease.CompleteEditReads().IsConsumed, "interrupted completion minted no pending read-completion evidence");
        output.Dispose(); lease.Dispose();
        using var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        Check(reopened.Length == original.Length, "interrupted source reader actually closed");
    }

    private static void CommitReadCancellation(string root, byte[] original, byte[] candidate, string mode)
    {
        var parent = Directory.CreateDirectory(Path.Combine(root, mode + "-cancel")).FullName;
        var source = Path.Combine(parent, Target); File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(parent, Guid.NewGuid());
        var token = StageControlCandidate(output, Target, output.CaptureTargetIdentity(Target), candidate);
        var completion = lease.CompleteEditReads();
        var stage = mode == "handoff" ? ArchiveNativeReadStage.EditHandoff : mode == "fingerprint"
            ? ArchiveNativeReadStage.OutputFingerprint : ArchiveNativeReadStage.Prepublication;
        var kind = mode == "fingerprint" ? ArchiveNativeReadKind.Output : mode == "ordinary_final_target"
            ? ArchiveNativeReadKind.Target : ArchiveNativeReadKind.Source;
        var observation = kind == ArchiveNativeReadKind.Source ? lease.ReadObservation : output.ReadObservation;
        var before = ReadCounts(observation, stage, kind);
        using var cancellation = new CancellationTokenSource();
        CancelFirstChunk(observation, stage, kind, cancellation);
        var result = mode == "ordinary_final_target" ? output.Commit(token, cancellation.Token)
            : output.CommitVerifiedEdit(token, lease, completion, cancellation.Token);
        AssertInterrupted(observation, stage, kind, before);
        Check(result is { Category: "cancelled", State: ArchiveCommitState.FailedBeforeCommit } &&
            !lease.EditFileReleased && !completion.IsConsumed, "mid-read cancellation cannot release source or publish: " + mode);
        Check(File.ReadAllBytes(source).AsSpan().SequenceEqual(original) && !Directory.EnumerateFiles(parent, ".nxpart-*").Any(),
            "mid-read cancellation preserves original and removes owned candidate: " + mode);
        Check(output.CommitJournal().Single() is { Published: false, Cleanup: "partial_removed" }, "mid-read cleanup journal: " + mode);
        RequireProtectedFile(source); RequireProtectedParent(parent);
        output.Dispose(); lease.Dispose();
        Check(output.OutputPublished() == false, "actual zero-publication cleanup: " + mode);
        using var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        Check(reopened.Length == original.Length, "mid-read source reader actually closed: " + mode);
    }

    private static void TargetReadCancellation(string root, byte[] original, bool begin)
    {
        var parent = Directory.CreateDirectory(Path.Combine(root, begin ? "begin-cancel" : "admission-cancel")).FullName;
        var source = Path.Combine(parent, Target); File.WriteAllBytes(source, original);
        using var output = ArchiveOutputSession.Open(parent, Guid.NewGuid());
        var expected = output.CaptureTargetIdentity(Target);
        var stage = begin ? ArchiveNativeReadStage.TargetBegin : ArchiveNativeReadStage.TargetAdmission;
        var before = ReadCounts(output.ReadObservation, stage, ArchiveNativeReadKind.Target);
        using var cancellation = new CancellationTokenSource();
        CancelFirstChunk(output.ReadObservation, stage, ArchiveNativeReadKind.Target, cancellation);
        if (begin) ExpectCancellation(() => output.BeginFile(Target, expected, overwrite: true, cancellationToken: cancellation.Token), cancellation.Token);
        else ExpectCancellation(() => output.CaptureTargetIdentity(Target, cancellation.Token), cancellation.Token);
        AssertInterrupted(output.ReadObservation, stage, ArchiveNativeReadKind.Target, before);
        Check(output.CommitJournal().Count == 0 && !Directory.EnumerateFiles(parent, ".nxpart-*").Any(), "cancelled target observation created no candidate");
        output.Dispose();
        using var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        Check(reopened.Length == original.Length, "cancelled target reader actually closed");
    }

    private static Guid StageControlCandidate(ArchiveOutputSession output, string name, string expected, byte[] candidate)
    {
        var token = output.BeginFile(name, expected, overwrite: true);
        for (var offset = 0; offset < candidate.Length; offset += 65536)
        {
            var chunk = candidate.AsMemory(offset, Math.Min(65536, candidate.Length - offset));
            Check(output.Write(token, chunk), "control held candidate write");
            Check(output.ReadPartial(token, offset, chunk.Length).AsSpan().SequenceEqual(chunk.Span), "control exact held-partial readback");
        }
        output.MarkCandidateVerified(token);
        return token;
    }
    private static ArchiveNativeReadObservation.Counts ReadCounts(ArchiveNativeReadObservation observation,
        ArchiveNativeReadStage stage, ArchiveNativeReadKind kind) => observation.Snapshot().SingleOrDefault(c =>
            c.Stage == stage.ToString() && c.Kind == kind.ToString()) ?? new(stage.ToString(), kind.ToString(), 0, 0, 0, 0);
    private static void CancelFirstChunk(ArchiveNativeReadObservation observation, ArchiveNativeReadStage stage,
        ArchiveNativeReadKind kind, CancellationTokenSource cancellation) => observation.ReadChunkProbeForFixture = (actualStage, actualKind, bytes) => {
            if (actualStage == stage && actualKind == kind && bytes == 65536) cancellation.Cancel();
        };
    private static void AssertInterrupted(ArchiveNativeReadObservation observation, ArchiveNativeReadStage stage,
        ArchiveNativeReadKind kind, ArchiveNativeReadObservation.Counts before)
    {
        var after = ReadCounts(observation, stage, kind);
        Check(after.Attempts - before.Attempts == 1 && after.FullReads == before.FullReads &&
            after.BytesRead - before.BytesRead == 65536 && after.CompletedBytes == before.CompletedBytes,
            "original cancellation token must interrupt the actual native read after one 64KiB chunk: " + stage + "/" + kind);
    }
    private static void ExpectCancellation(Action action, CancellationToken cancellation)
    {
        try { action(); }
        catch (OperationCanceledException stopped) { Check(stopped.CancellationToken == cancellation, "original cancellation token retained"); return; }
        throw new InvalidOperationException("native owner ignored the original cancellation token");
    }

    private static string Root(bool baseline)
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch), "absolute PI_SCRATCH_DIR required");
        var configured = Environment.GetEnvironmentVariable("NXTOOLS_OPT05_NATIVE_ROOT");
        Check(baseline || !string.IsNullOrWhiteSpace(configured), "regression requires NXTOOLS_OPT05_NATIVE_ROOT from the completed baseline");
        var root = string.IsNullOrWhiteSpace(configured)
            ? Path.Combine(scratch!, RootPrefix + Guid.NewGuid().ToString("D")) : Path.GetFullPath(configured);
        Check(string.Equals(Path.GetDirectoryName(root), Path.GetFullPath(scratch!).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) &&
            Path.GetFileName(root).StartsWith(RootPrefix, StringComparison.Ordinal) &&
            Guid.TryParseExact(Path.GetFileName(root)[RootPrefix.Length..], "D", out _), "GUID-owned session scratch root required");
        var marker = Path.Combine(root, "owned-opt05-native-fixture");
        if (baseline)
        {
            Check(!File.Exists(Path.Combine(root, "measurements-before.json")), "before evidence must not be overwritten");
            Check(!Directory.Exists(root) || File.Exists(marker), "existing root must be fixture-owned");
            Directory.CreateDirectory(root);
            File.WriteAllText(marker, "OPT05 native same-stage observation fixture");
        }
        else Check(File.Exists(marker) && File.Exists(Path.Combine(root, "measurements-before.json")), "completed owned baseline required");
        return root;
    }

    private static Trial RunTrial(string root, string fixture, byte[] original, byte[] candidate, byte[] payload, int number)
    {
        var caseRoot = Directory.CreateDirectory(Path.Combine(root, "trial-" + number + "-" + Guid.NewGuid().ToString("N"))).FullName;
        var source = Path.Combine(caseRoot, Target);
        File.Copy(fixture, source);
        var foreign = Path.Combine(caseRoot, ".nxpart-foreign");
        File.WriteAllText(foreign, "foreign-owned evidence");
        ArchiveSourceLease? lease = null;
        ArchiveOutputSession? output = null;
        try
        {
            var operation = Guid.NewGuid();
            var phases = new List<Phase>();
            var clock = Stopwatch.StartNew();
            var previous = 0.0;
            void PhaseDone(string name)
            {
                var elapsed = clock.Elapsed.TotalMilliseconds;
                phases.Add(new(name, elapsed - previous));
                previous = elapsed;
            }
            lease = ArchiveSourceLease.OpenForEdit(source);
            PhaseDone("edit_open");
            var originalIdentity = lease.ReadmissionIdentity();
            var log = new WorkLog();
            output = ArchiveOutputSession.Open(caseRoot, operation, log);
            var expected = output.CaptureTargetIdentity(Target);
            PhaseDone("target_admission");
            lease.VerifyEditSource(expected); // Same owner route as the original worker BindEditSource.
            PhaseDone("edit_verification");
            RequireProtectedFile(source);
            RequireProtectedParent(caseRoot);
            var token = output.BeginFile(Target, expected, overwrite: true);
            for (var offset = 0; offset < candidate.Length; offset += 65536)
                Check(output.Write(token, candidate.AsMemory(offset, Math.Min(65536, candidate.Length - offset))), "held candidate write");
            var readback = new byte[candidate.Length];
            for (var offset = 0; offset < readback.Length; offset += 65536)
            {
                var chunk = output.ReadPartial(token, offset, Math.Min(65536, readback.Length - offset));
                Check(chunk.Length == Math.Min(65536, readback.Length - offset), "complete held-partial readback");
                chunk.CopyTo(readback, offset);
            }
            Check(readback.AsSpan().SequenceEqual(candidate), "held candidate exact bytes");
            VerifyZip(readback, payload, "renamed.bin");
            output.MarkCandidateVerified(token);
            PhaseDone("candidate_write_readback_verify");
            // This is a real file reader; its lexical disposal precedes read-completion evidence.
            using (var reader = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.Read))
                Check(reader.Length == original.Length && reader.ReadByte() == original[0], "source reader used before completion");
            var completion = lease.CompleteEditReads();
            Check(!completion.IsConsumed && !lease.EditFileReleased, "completion cannot release source FILE early");
            PhaseDone("complete_edit_reads");
            var publications = 0;
            var fileOnly = false;
            var exclusiveAtHandoff = false;
            var checkpoints = new List<string>();
            output.CommitCheckpointProbe = stage => {
                Check(checkpoints.Count < 32, "bounded native commit checkpoints");
                checkpoints.Add(stage);
                if (stage is "edit_source_rechecked" or "before_recheck" or "target_rechecked")
                {
                    Check(!lease.EditFileReleased && !completion.IsConsumed, "source protected through final checks");
                    RequireProtectedFile(source);
                    RequireProtectedParent(caseRoot);
                }
                if (stage == "commit_in_flight")
                {
                    Check(lease.EditFileReleased && completion.IsConsumed, "FILE-only release after single-use reader completion");
                    using (var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                        Check(reopened.Length == original.Length, "all original source readers closed before publication");
                    exclusiveAtHandoff = true;
                    RequireProtectedParent(caseRoot);
                    var partial = Directory.GetFiles(caseRoot, ".nxpart-" + operation.ToString("N") + "-*").Single();
                    RequireProtectedFile(partial);
                    fileOnly = true;
                }
                if (stage == "native_published")
                {
                    publications++;
                    var journal = output.CommitJournal().Single();
                    Check(journal.State == "Committed" && journal.Published == true && journal.Replaced &&
                        journal.Bytes == (ulong)candidate.Length && output.OutputPublished() == true, "native publication facts precede close/postcheck");
                }
            };
            var result = output.CommitVerifiedEdit(token, lease, completion, CancellationToken.None);
            PhaseDone("verified_native_commit");
            clock.Stop();
            Check(result is { Category: "committed", NativeCode: null, State: ArchiveCommitState.Committed }, "real native same-path commit");
            Check(publications == 1 && fileOnly && exclusiveAtHandoff && completion.IsConsumed, "single native FILE-only handoff/publication");
            Check(checkpoints.Count(c => c == "before_flush") == 1 && checkpoints.Count(c => c == "target_rechecked") == 1 &&
                checkpoints.Count(c => c == "native_committed") == 1, "original flush/final-target/postpublication boundaries retained");
            Check(lease.ReadmissionIdentity() == originalIdentity, "released source retains its original readmission identity");
            RequireProtectedParent(caseRoot);
            var published = File.ReadAllBytes(source);
            Check(published.AsSpan().SequenceEqual(candidate), "native published bytes");
            VerifyZip(published, payload, "renamed.bin");
            Check(output.Cancel(token, cleanupNewOutput: true).Category == "retained_replacement", "published replacement cannot be cleaned as a new file");
            var sourceCounts = lease.ReadObservation.Snapshot();
            var outputCounts = output.ReadObservation.Snapshot();
            AssertFreshStages(sourceCounts, outputCounts, original.Length, candidate.Length);
            Check(output.ReadObservation.PartialReadBytes == candidate.Length && output.ReadObservation.PartialReadCalls == (candidate.Length + 65535L) / 65536,
                "actual held-partial readback counted separately");
            Check(log.Correlation == operation && log.Completed is not null &&
                (long)log.Completed["native_full_reads"]! == output.ReadObservation.FullReads &&
                (long)log.Completed["native_read_bytes"]! == output.ReadObservation.BytesRead &&
                (long)log.Completed["partial_read_bytes"]! == candidate.Length, "original correlated completion diagnostics match actual output-owner counters");
            var cleanupClock = Stopwatch.StartNew();
            output.Dispose();
            lease.Dispose();
            using (var reopened = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                Check(reopened.Length == candidate.Length, "final source exclusively reopened after actual cleanup");
            cleanupClock.Stop();
            Check(!Directory.EnumerateFiles(caseRoot, ".nxpart-" + operation.ToString("N") + "-*").Any() &&
                File.ReadAllText(foreign) == "foreign-owned evidence", "only owned partials retired");
            Check(output.OutputPublished() == true && output.CommitJournal().Single().State == "Committed", "publication history survives cleanup");
            var trial = new Trial(number, operation, clock.Elapsed.TotalMilliseconds, cleanupClock.Elapsed.TotalMilliseconds,
                phases.ToArray(), sourceCounts, outputCounts, output.ReadObservation.PartialReadCalls, output.ReadObservation.PartialReadBytes,
                publications, fileOnly, true, exclusiveAtHandoff, true, true, result.State.ToString());
            Directory.Delete(caseRoot, recursive: true);
            Console.WriteLine("ARCHIVE_OPT05_NATIVE_TRIAL " + JsonSerializer.Serialize(trial));
            return trial;
        }
        finally { output?.Dispose(); lease?.Dispose(); }
    }

    private static void AssertFreshStages(ArchiveNativeReadObservation.Counts[] source, ArchiveNativeReadObservation.Counts[] output,
        long originalBytes, long candidateBytes)
    {
        foreach (var counts in source.Concat(output))
        {
            var length = counts.Kind == "Output" ? candidateBytes : originalBytes;
            Check(counts.Attempts == counts.FullReads && counts.FullReads > 0 &&
                counts.BytesRead == length * counts.FullReads && counts.CompletedBytes == counts.BytesRead,
                "successful full-read counts must be real EOF bytes: " + counts.Stage + "/" + counts.Kind);
        }
        foreach (var stage in EquivalentStages)
            Check(source.Concat(output).Where(c => c.Stage == stage).Sum(c => c.FullReads) >= 1,
                "separately fresh source/target stage required: " + stage);
        foreach (var stage in new[] { "TargetAdmission", "TargetBegin", "OutputFingerprint", "OutputPostpublication" })
            Check(output.Where(c => c.Stage == stage).Sum(c => c.FullReads) == 1, "independent output boundary preserved: " + stage);
    }
    private static void RequireProtectedFile(string path)
    {
        try { using var writer = File.Open(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); }
        catch (IOException) { return; }
        throw new InvalidOperationException("protected source/candidate accepted a writer");
    }
    private static void RequireProtectedParent(string root)
    {
        var moved = root + ".moved";
        try { Directory.Move(root, moved); }
        catch (IOException) { return; }
        Directory.Move(moved, root);
        throw new InvalidOperationException("held parent moved during FILE-only handoff");
    }
    private static byte[] CreateZip(byte[] payload, string name)
    {
        using var bytes = new MemoryStream();
        using (var archive = new ZipArchive(bytes, ZipArchiveMode.Create, leaveOpen: true))
        {
            var entry = archive.CreateEntry(name, CompressionLevel.NoCompression);
            entry.LastWriteTime = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
            using var stream = entry.Open();
            stream.Write(payload);
        }
        return bytes.ToArray();
    }
    private static void VerifyZip(byte[] bytes, byte[] payload, string name)
    {
        using var memory = new MemoryStream(bytes, writable: false);
        using var archive = new ZipArchive(memory, ZipArchiveMode.Read);
        var entry = archive.Entries.Single();
        Check(entry.FullName == name && entry.Length == payload.Length, "independent ZIP name/size");
        using var stream = entry.Open();
        var buffer = new byte[65536];
        var offset = 0;
        while (true)
        {
            var count = stream.Read(buffer);
            if (count == 0) break;
            Check(offset <= payload.Length - count && buffer.AsSpan(0, count).SequenceEqual(payload.AsSpan(offset, count)), "independent ZIP payload bytes");
            offset += count;
        }
        Check(offset == payload.Length, "independent ZIP complete EOF");
    }
}
