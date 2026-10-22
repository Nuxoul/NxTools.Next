using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: only GUID-owned files below the approved session scratch tree and the
// existing handle-relative ArchiveSourceLease/ArchiveOutputSession native boundaries.
internal static class ArchiveZipEditCommitTests
{
    private const string TargetName = "editable.zip";

    private static readonly IReadOnlyDictionary<string, string> OriginalManifest =
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["original.txt"] = "original archive payload"
        };

    private static readonly IReadOnlyDictionary<string, string> EditedManifest =
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["original.txt"] = "edited archive payload",
            ["added/phase-e.txt"] = "record-preserving rewrite candidate"
        };

    internal static Task HeldSourceSamePathReplacementPublishesValidZip()
    {
        var root = CreateFixtureRoot("commit");
        var completed = false;
        try
        {
            var source = Path.Combine(root, TargetName);
            var originalBytes = CreateZip(OriginalManifest);
            var candidateBytes = CreateZip(EditedManifest);
            File.WriteAllBytes(source, originalBytes);
            ValidateZip(File.ReadAllBytes(source), OriginalManifest, "original source ZIP");

            var operationId = Guid.NewGuid();
            using var sourceLease = ArchiveSourceLease.OpenForEdit(source);
            var heldIdentity = sourceLease.ReadmissionIdentity();
            using var output = ArchiveOutputSession.Open(root, operationId);
            var expectedTarget = output.CaptureTargetIdentity(TargetName);
            var token = StageCompleteCandidate(output, expectedTarget, candidateBytes);
            AssertSourceProtected(source, heldIdentity, sourceLease, root);

            var evidence = sourceLease.CompleteEditReads();
            Require(!evidence.IsConsumed, "read-complete evidence consumed before commit");
            var result = output.CommitVerifiedEdit(token, sourceLease, evidence, CancellationToken.None);

            Require(evidence.IsConsumed, "successful handoff evidence not consumed");
            Require(result is { Category: "committed", NativeCode: null, State: ArchiveCommitState.Committed },
                $"exact commit result category={result.Category} native={result.NativeCode} state={result.State}");
            Require(sourceLease.ReadmissionIdentity() == heldIdentity, "released edit source lost immutable original identity");
            Require(sourceLease.EditFileReleased, "successful edit did not release source file handle");
            AssertCommittedJournal(output, candidateBytes.Length);
            RequireParentsProtected(root);
            var publishedBytes = File.ReadAllBytes(source);
            Require(publishedBytes.SequenceEqual(candidateBytes), "published candidate bytes mismatch");
            ValidateZip(publishedBytes, EditedManifest, "published edited ZIP");
            Require(OwnedPartials(root, operationId).Length == 0, "published candidate partial remained");
            Require(output.Cancel(token, cleanupNewOutput: true).Category == "retained_replacement",
                "published replacement was treated as removable new output");

            completed = true;
        }
        finally { FinishFixture(root, completed); }
        return Task.CompletedTask;
    }

    internal static Task CancellationBeforeAndAfterPublicationPreservesTruth()
    {
        var root = CreateFixtureRoot("cancel");
        var completed = false;
        try
        {
            BeforeHandoffCancellation(root);
            LateCancellation(root);
            completed = true;
        }
        finally { FinishFixture(root, completed); }
        return Task.CompletedTask;
    }

    internal static Task ChangedRevisionIsRefusedAfterLeaseRetires()
    {
        var root = CreateFixtureRoot("revision");
        var completed = false;
        try
        {
            var source = Path.Combine(root, TargetName);
            var retired = Path.Combine(root, "retired-original.zip");
            var foreignPartial = Path.Combine(root, ".nxpart-foreign-" + Guid.NewGuid().ToString("N"));
            var originalBytes = CreateZip(OriginalManifest);
            var candidateBytes = CreateZip(EditedManifest);
            File.WriteAllBytes(source, originalBytes);
            File.WriteAllText(foreignPartial, "foreign evidence", Encoding.UTF8);

            var operationId = Guid.NewGuid();
            using var output = ArchiveOutputSession.Open(root, operationId);
            string expectedTarget;
            Guid token;
            using (var sourceLease = ArchiveSourceLease.OpenForEdit(source))
            {
                expectedTarget = output.CaptureTargetIdentity(TargetName);
                token = StageCompleteCandidate(output, expectedTarget, candidateBytes);
                sourceLease.RecycleSource(operationId).Verify();
            }

            File.Move(source, retired);
            File.WriteAllBytes(source, originalBytes);
            var replacementTarget = output.CaptureTargetIdentity(TargetName);
            var oldFields = expectedTarget.Split(':');
            var newFields = replacementTarget.Split(':');
            Require(oldFields.Length == 6 && newFields.Length == 6 &&
                oldFields[1] == newFields[1] && oldFields[2] != newFields[2] &&
                oldFields[3] == newFields[3] && oldFields[5] == newFields[5],
                "fixture did not create a same-volume same-bytes different target revision");

            var failure = CaptureFailure(() => output.Commit(token));
            Require(failure is { Stage: "target_changed", NativeCode: null, State: ArchiveCommitState.FailedBeforeCommit },
                $"changed revision result stage={failure.Stage} native={failure.NativeCode} state={failure.State}");
            var journal = output.CommitJournal().Single();
            Require(journal.State == "FailedBeforeCommit" && !journal.Replaced && journal.Bytes == (ulong)candidateBytes.Length,
                "changed revision commit facts");
            Require(File.ReadAllBytes(source).SequenceEqual(originalBytes), "changed target was overwritten");
            Require(File.ReadAllBytes(retired).SequenceEqual(originalBytes), "retired held original changed");
            Require(output.Cancel(token).Category == "partial_removed", "changed revision candidate cleanup");
            Require(OwnedPartials(root, operationId).Length == 0, "changed revision owned partial remained");
            Require(File.ReadAllText(foreignPartial, Encoding.UTF8) == "foreign evidence", "changed revision cleanup removed foreign partial");
            ValidateZip(File.ReadAllBytes(source), OriginalManifest, "replacement target after revision refusal");
            ValidateZip(File.ReadAllBytes(retired), OriginalManifest, "retired original after revision refusal");

            completed = true;
        }
        finally { FinishFixture(root, completed); }
        return Task.CompletedTask;
    }

    internal static Task HandoffGatesSourceTargetCancellationAndOwnership()
    {
        var root = CreateFixtureRoot("handoff-gates");
        var completed = false;
        try
        {
            MissingReadCompletionRejects(root);
            SourceMismatchRejects(root);
            TargetMismatchRejects(root);
            HandoffCancellationRejects(root);
            OrdinaryLeaseRemainsHeld(root);
            completed = true;
        }
        finally { FinishFixture(root, completed); }
        return Task.CompletedTask;
    }

    internal static Task HandoffNativeFailureUnknownAndNoRetry()
    {
        var root = CreateFixtureRoot("handoff-native-failures");
        var completed = false;
        try
        {
            ReaderClosureIsRequired(root);
            CandidateVerificationGateRejects(root);
            foreach (var mode in new[] { "denied", "unknown" })
            {
                var caseRoot = Directory.CreateDirectory(Path.Combine(root, mode + "-" + Guid.NewGuid().ToString("N"))).FullName;
                var source = Path.Combine(caseRoot, TargetName);
                var originalBytes = CreateZip(OriginalManifest);
                var candidateBytes = CreateZip(EditedManifest);
                File.WriteAllBytes(source, originalBytes);
                var operationId = Guid.NewGuid();
                using var sourceLease = ArchiveSourceLease.OpenForEdit(source);
                using var output = ArchiveOutputSession.Open(caseRoot, operationId);
                var expected = output.CaptureTargetIdentity(TargetName);
                var token = StageCompleteCandidate(output, expected, candidateBytes);
                var evidence = sourceLease.CompleteEditReads();
                var calls = 0;
                output.CommitCheckpointProbe = stage =>
                {
                    if (stage != "commit_in_flight") return;
                    calls++;
                    throw new ArchiveOutputException("replace_file", mode == "unknown" ? 64 : 5);
                };

                if (mode == "unknown")
                {
                    var result = output.CommitVerifiedEdit(token, sourceLease, evidence, CancellationToken.None);
                    Require(result is { Category: "commit_outcome_unknown", NativeCode: 64, State: ArchiveCommitState.CommitOutcomeUnknown },
                        "unknown native fact changed");
                    Require(output.Cancel(token).Category == "retained_uncertain", "unknown evidence was cleaned");
                }
                else
                {
                    var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, sourceLease, evidence, CancellationToken.None));
                    Require(failure is { Stage: "replace_file", NativeCode: 5, State: ArchiveCommitState.FailedBeforeCommit },
                        "denied native fact changed");
                    Require(output.Cancel(token).Category == "partial_removed", "denied candidate cleanup");
                }
                Require(calls == 1, "native replace was retried");
                Require(evidence.IsConsumed, "handoff evidence not consumed on native attempt");
                Require(File.ReadAllBytes(source).SequenceEqual(originalBytes), "native failure changed original bytes");
                RequireParentsProtected(caseRoot);
            }
            completed = true;
        }
        finally { FinishFixture(root, completed); }
        return Task.CompletedTask;
    }

    private static void ReaderClosureIsRequired(string parent)
    {
        var root = NewCaseRoot(parent, "reader-must-close");
        var source = Path.Combine(root, TargetName);
        var original = CreateZip(OriginalManifest);
        File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), CreateZip(EditedManifest));
        using var workerReader = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.Read);
        var evidence = lease.CompleteEditReads();
        var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, lease, evidence, CancellationToken.None));
        Require(failure is { Stage: "replace_file", NativeCode: 32, State: ArchiveCommitState.FailedBeforeCommit },
            $"open worker reader native result {failure.Stage}/{failure.NativeCode}/{failure.State}");
        Require(lease.EditFileReleased && evidence.IsConsumed, "handoff did not reach actual reader conflict");
        Require(File.ReadAllBytes(source).SequenceEqual(original), "open reader failure changed source");
        Require(CaptureFailure(() => output.Commit(token)).Stage == "invalid_state", "reader conflict was automatically retryable");
        Require(output.Cancel(token).Category == "partial_removed", "reader conflict candidate cleanup");
        RequireParentsProtected(root);
    }
    private static void CandidateVerificationGateRejects(string parent)
    {
        var root = NewCaseRoot(parent, "candidate-gate");
        var source = Path.Combine(root, TargetName);
        File.WriteAllBytes(source, CreateZip(OriginalManifest));
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var expected = output.CaptureTargetIdentity(TargetName);
        var token = output.BeginFile(TargetName, expected, overwrite: true);
        Require(output.Write(token, CreateZip(EditedManifest)), "candidate gate write");
        var evidence = lease.CompleteEditReads();
        var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, lease, evidence, CancellationToken.None));
        Require(failure.Stage == "candidate_not_verified" && !lease.EditFileReleased && !evidence.IsConsumed,
            "unverified candidate authorized handoff");
        output.Cancel(token);
    }
    private static void MissingReadCompletionRejects(string parent)
    {
        var root = NewCaseRoot(parent, "missing-read-complete");
        var source = Path.Combine(root, TargetName);
        var candidate = CreateZip(EditedManifest);
        File.WriteAllBytes(source, CreateZip(OriginalManifest));
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), candidate);
        var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, lease, null!, CancellationToken.None));
        Require(failure.Stage == "edit_reads_incomplete" && !lease.EditFileReleased, "missing read completion authorized handoff");
        AssertSourceProtected(source, lease.ReadmissionIdentity(), lease, root);
        output.Cancel(token);
    }

    private static void SourceMismatchRejects(string parent)
    {
        var root = NewCaseRoot(parent, "source-mismatch");
        var source = Path.Combine(root, TargetName);
        var original = CreateZip(OriginalManifest);
        File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), CreateZip(EditedManifest));
        using var alias = ArchiveSourceLease.OpenForEdit(source);
        var wrongEvidence = alias.CompleteEditReads();
        var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, lease, wrongEvidence, CancellationToken.None));
        Require(failure.Stage == "edit_source_mismatch" && !lease.EditFileReleased && !wrongEvidence.IsConsumed,
            "foreign close evidence authorized source handoff");
        Require(File.ReadAllBytes(source).SequenceEqual(original), "foreign evidence changed source");
        output.Cancel(token);
    }

    private static void TargetMismatchRejects(string parent)
    {
        var root = NewCaseRoot(parent, "target-mismatch");
        var source = Path.Combine(root, TargetName);
        var original = CreateZip(OriginalManifest);
        File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), CreateZip(EditedManifest));
        var evidence = lease.CompleteEditReads();
        var failure = CaptureFailure(() => output.CommitVerifiedEdit(token, lease, evidence, CancellationToken.None,
            expectedSourceTargetIdentity: "v2:0:00000000000000000000000000000000:0:0:00"));
        Require(failure.Stage == "edit_target_mismatch" && !lease.EditFileReleased && !evidence.IsConsumed,
            "target mismatch released source file");
        Require(File.ReadAllBytes(source).SequenceEqual(original), "target mismatch changed source");
        output.Cancel(token);
    }

    private static void HandoffCancellationRejects(string parent)
    {
        var root = NewCaseRoot(parent, "handoff-cancel");
        var source = Path.Combine(root, TargetName);
        var original = CreateZip(OriginalManifest);
        File.WriteAllBytes(source, original);
        using var lease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), CreateZip(EditedManifest));
        var evidence = lease.CompleteEditReads();
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var result = output.CommitVerifiedEdit(token, lease, evidence, cancellation.Token);
        Require(result is { Category: "cancelled", State: ArchiveCommitState.FailedBeforeCommit } &&
            !lease.EditFileReleased && !evidence.IsConsumed, "pre-handoff cancellation released source");
        Require(File.ReadAllBytes(source).SequenceEqual(original), "pre-handoff cancellation changed source");
        Require(OwnedPartials(root, output.OperationId).Length == 0, "pre-handoff cancellation retained owned partial");
    }

    private static void OrdinaryLeaseRemainsHeld(string parent)
    {
        var root = NewCaseRoot(parent, "ordinary-lease");
        var source = Path.Combine(root, TargetName);
        File.WriteAllBytes(source, CreateZip(OriginalManifest));
        using var lease = ArchiveSourceLease.Open(source);
        var identity = lease.ReadmissionIdentity();
        AssertSourceProtected(source, identity, lease, root);
        Require(!lease.EditFileReleased, "ordinary lease unexpectedly released file");
    }

    private static void BeforeHandoffCancellation(string parent)
    {
        var root = NewCaseRoot(parent, "before-cancel");
        var source = Path.Combine(root, TargetName);
        var foreignPartial = Path.Combine(root, ".nxpart-foreign-" + Guid.NewGuid().ToString("N"));
        var originalBytes = CreateZip(OriginalManifest);
        File.WriteAllBytes(source, originalBytes);
        File.WriteAllText(foreignPartial, "foreign evidence", Encoding.UTF8);
        var operationId = Guid.NewGuid();
        using var sourceLease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, operationId);
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), CreateZip(EditedManifest));
        var evidence = sourceLease.CompleteEditReads();
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var result = output.CommitVerifiedEdit(token, sourceLease, evidence, cancellation.Token);
        Require(result is { Category: "cancelled", State: ArchiveCommitState.FailedBeforeCommit }, "before-handoff cancellation result");
        Require(!sourceLease.EditFileReleased && !evidence.IsConsumed, "before-handoff cancellation consumed lease handoff");
        Require(File.ReadAllBytes(source).SequenceEqual(originalBytes), "before-handoff cancellation changed original");
        Require(OwnedPartials(root, operationId).Length == 0, "before-handoff cancellation retained owned partial");
        Require(File.ReadAllText(foreignPartial, Encoding.UTF8) == "foreign evidence", "before-handoff cancellation removed foreign partial");
    }

    private static void LateCancellation(string parent)
    {
        var root = NewCaseRoot(parent, "late-cancel");
        var source = Path.Combine(root, TargetName);
        var candidateBytes = CreateZip(EditedManifest);
        File.WriteAllBytes(source, CreateZip(OriginalManifest));
        var operationId = Guid.NewGuid();
        using var sourceLease = ArchiveSourceLease.OpenForEdit(source);
        using var output = ArchiveOutputSession.Open(root, operationId);
        var token = StageCompleteCandidate(output, output.CaptureTargetIdentity(TargetName), candidateBytes);
        var evidence = sourceLease.CompleteEditReads();
        using var cancellation = new CancellationTokenSource();
        var published = 0;
        output.CommitCheckpointProbe = stage =>
        {
            if (stage != "native_published") return;
            published++;
            cancellation.Cancel();
            AssertCommittedJournal(output, candidateBytes.Length);
        };
        var result = output.CommitVerifiedEdit(token, sourceLease, evidence, cancellation.Token);
        Require(published == 1 && cancellation.IsCancellationRequested, "late cancellation checkpoint not reached once");
        Require(result is { Category: "committed", NativeCode: null, State: ArchiveCommitState.Committed },
            "late cancellation rewrote committed truth");
        Require(sourceLease.EditFileReleased && evidence.IsConsumed, "successful handoff lifecycle missing");
        ValidateZip(File.ReadAllBytes(source), EditedManifest, "published ZIP after late cancellation");
        RequireParentsProtected(root);
    }

    private static Guid StageCompleteCandidate(ArchiveOutputSession output, string expectedTarget, byte[] candidateBytes)
    {
        var token = output.BeginFile(TargetName, expectedTarget, overwrite: true);
        for (var offset = 0; offset < candidateBytes.Length;)
        {
            var count = Math.Min(257, candidateBytes.Length - offset);
            Require(output.Write(token, candidateBytes.AsMemory(offset, count)), "candidate write rejected");
            offset += count;
        }
        var staged = new byte[candidateBytes.Length];
        for (var offset = 0; offset < staged.Length;)
        {
            var chunk = output.ReadPartial(token, offset, Math.Min(251, staged.Length - offset));
            Require(chunk.Length > 0, "candidate read-back ended early");
            chunk.CopyTo(staged, offset);
            offset += chunk.Length;
        }
        Require(staged.SequenceEqual(candidateBytes), "exclusive candidate read-back mismatch");
        ValidateZip(staged, EditedManifest, "exclusive complete candidate");
        output.MarkCandidateVerified(token);
        return token;
    }

    private static void AssertCommittedJournal(ArchiveOutputSession output, int byteCount)
    {
        var fact = output.CommitJournal().Single();
        Require(fact.Ordinal == 0 && fact.State == "Committed" && fact.Replaced && fact.Bytes == (ulong)byteCount,
            $"commit journal ordinal={fact.Ordinal} state={fact.State} replaced={fact.Replaced} bytes={fact.Bytes}");
    }

    private static void AssertSourceProtected(string path, string identity, ArchiveSourceLease lease, string root)
    {
        Require(lease.ReadmissionIdentity() == identity, "source identity changed while protected");
        try
        {
            using var writer = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete);
            throw new InvalidOperationException("held edit source accepted writer");
        }
        catch (IOException) { }
        RequireParentsProtected(root);
    }

    private static void RequireParentsProtected(string root)
    {
        var moved = root + ".moved";
        try
        {
            Directory.Move(root, moved);
            Directory.Move(moved, root);
            throw new InvalidOperationException("held parent directory moved");
        }
        catch (IOException) { }
    }

    private static ArchiveOutputException CaptureFailure(Action action)
    {
        try { action(); }
        catch (ArchiveOutputException failure) { return failure; }
        throw new InvalidOperationException("expected ArchiveOutputException");
    }

    private static byte[] CreateZip(IReadOnlyDictionary<string, string> manifest)
    {
        using var bytes = new MemoryStream();
        using (var archive = new ZipArchive(bytes, ZipArchiveMode.Create, leaveOpen: true))
        {
            foreach (var item in manifest.OrderBy(item => item.Key, StringComparer.Ordinal))
            {
                var entry = archive.CreateEntry(item.Key, CompressionLevel.Optimal);
                entry.LastWriteTime = new DateTimeOffset(2020, 1, 2, 3, 4, 6, TimeSpan.Zero);
                using var writer = new StreamWriter(entry.Open(), new UTF8Encoding(false));
                writer.Write(item.Value);
            }
        }
        return bytes.ToArray();
    }

    private static void ValidateZip(byte[] bytes, IReadOnlyDictionary<string, string> expected, string detail)
    {
        using var input = new MemoryStream(bytes, writable: false);
        using var archive = new ZipArchive(input, ZipArchiveMode.Read);
        Require(archive.Entries.Count == expected.Count, detail + " entry count");
        var observed = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var entry in archive.Entries)
        {
            using var reader = new StreamReader(entry.Open(), Encoding.UTF8, detectEncodingFromByteOrderMarks: true);
            Require(observed.TryAdd(entry.FullName, reader.ReadToEnd()), detail + " duplicate entry");
        }
        Require(observed.Count == expected.Count && expected.All(item =>
            observed.TryGetValue(item.Key, out var value) && value == item.Value), detail + " manifest/content");
    }

    private static string[] OwnedPartials(string root, Guid operationId) =>
        Directory.GetFiles(root, ".nxpart-" + operationId.ToString("N") + "-*", SearchOption.TopDirectoryOnly);

    private static string NewCaseRoot(string parent, string name) =>
        Directory.CreateDirectory(Path.Combine(parent, name + "-" + Guid.NewGuid().ToString("N"))).FullName;

    private static string CreateFixtureRoot(string scenario)
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        if (string.IsNullOrWhiteSpace(scratch))
            throw new InvalidOperationException("PI_SCRATCH_DIR is required for the isolated native fixture");
        var parent = Path.Combine(scratch, "phase-e-delivery-20261022-130740", "native-handoff", "fixtures");
        Directory.CreateDirectory(parent);
        return NewCaseRoot(parent, scenario);
    }

    private static void FinishFixture(string root, bool completed)
    {
        if (completed) Directory.Delete(root, recursive: true);
        else Console.Error.WriteLine("PRESERVED FIXTURE " + root);
    }

    private static void Require(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }
}
