using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

// Explicit administrator maintenance. Release state/snapshots and publisher key never change here.
internal sealed class TrustedBootstrapMaintenance(string updaterRoot, byte[] publicKey,
    TrustedBootstrapMaintenance.IService service, IPlatformLog log)
{
    internal interface IService
    {
        void Validate(string expectedBootstrapHash, string expectedPublicKeyHash);
        void Stop();
        IDisposable EnterDeploymentGate();
        void SetBootstrapHash(string hash);
        void SetTrustIdentity(string expectedBootstrapHash, string expectedPublicKeyHash,
            string newBootstrapHash, string newPublicKeyHash);
        void SetTrustKeyHash(string hash);
        void Start();
        void ConfirmReady();
    }
    private sealed record Enrollment(TrustedUpdateEnrollment.Identity Identity, bool Completed);
    private sealed record Version(long Sequence, Guid ReleaseId, string ManifestSha256, string BootstrapSha256, int ProtocolVersion = 1);
    private sealed record Journal(string OldHash, string NewHash, string KeyHash, string EnrollmentBase64,
        string ReleaseStateSha256, Version Next);
    private sealed record RotationJournal(string OldBootstrapHash, string NewBootstrapHash, string OldKeyHash,
        string NewKeyHash, string EnrollmentBase64, string ReleaseStateSha256, Version Next);
    private string Root => Path.GetFullPath(updaterRoot);
    private string Bootstrap => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.exe");
    private string Backup => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.maintenance-old");
    private string Candidate => Path.Combine(Root, "Bootstrap", "NxTools.PlatformHost.maintenance-new");
    private string JournalPath => Path.Combine(Root, "bootstrap-maintenance.json");
    private string RotationJournalPath => Path.Combine(Root, "bootstrap-trust-rotation.json");
    private string EnrollmentPath => Path.Combine(Root, "enrollment.json");
    private string StatePath => Path.Combine(Root, "Releases", "state.json");
    private string VersionPath => Path.Combine(Root, "bootstrap-maintenance-state.json");
    internal static void RecordRepairedEnrollment(string root, VerifiedRelease release)
    {
        if (!new TrustedUpdateEnrollment(root).IsComplete())
            throw new InvalidDataException("Completed enrollment required before protocol state.");
        var bootstrap = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe");
        using var binary = IndexerServiceInstaller.TrustedBinary.Open(
            Path.Combine(root, "Bootstrap", "NxTools.PlatformHost.exe"), bootstrap.Sha256);
        using var output = new FileStream(Path.Combine(root, "bootstrap-maintenance-state.json"),
            FileMode.CreateNew, FileAccess.Write, FileShare.None);
        JsonSerializer.Serialize(output, new Version(release.Manifest.ReleaseSequence,
            release.Manifest.ReleaseId, release.ManifestSha256, bootstrap.Sha256));
        output.Flush(true);
    }

    internal void Rotate(string package, Guid correlationId)
    {
        var clock = Stopwatch.StartNew();
        Event("rotation_accepted", correlationId, clock);
        try
        {
            using var operation = new FileStream(Path.Combine(Root, "bootstrap-maintenance.lock"),
                FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
            var newKeyHash = Hash(publicKey);
            var enrollmentBytes = ReadBytes(EnrollmentPath);
            var enrollment = JsonSerializer.Deserialize<Enrollment>(enrollmentBytes)
                ?? throw new InvalidDataException("Missing enrollment.");
            if (!enrollment.Completed)
                throw new InvalidDataException("Completed enrollment required before trust rotation.");
            var oldKeyHash = enrollment.Identity.PublicKeySha256;
            var oldBootstrapHash = enrollment.Identity.BootstrapSha256;
            if (File.Exists(RotationJournalPath))
            {
                var prior = Read<RotationJournal>(RotationJournalPath);
                if (!Same(prior.OldKeyHash, oldKeyHash) || !Same(prior.OldBootstrapHash, oldBootstrapHash) ||
                    File.Exists(Backup) || !Same(FileHash(Bootstrap), oldBootstrapHash) || !File.Exists(Candidate))
                    throw new InvalidDataException("Unfinished trust rotation requires rollback inspection.");
                RequireHash(Candidate, prior.NewBootstrapHash);
                File.Delete(Candidate);
                File.Delete(RotationJournalPath);
                Event("rotation_recovered_precommit", correlationId, clock);
            }
            if (Same(oldKeyHash, newKeyHash))
            {
                Apply(package, correlationId);
                return;
            }
            service.Validate(oldBootstrapHash, oldKeyHash);
            var state = Read<TrustedUpdateTransaction.State>(StatePath);
            if (state.Sequence <= 0 || state.ReleaseId == Guid.Empty || !IsHash(state.ManifestSha256))
                throw new InvalidDataException("Invalid current release state.");
            if (File.Exists(Path.Combine(Root, "Releases", "transaction.json")))
                throw new InvalidOperationException("An unfinished product update must recover before trust rotation.");
            if (File.Exists(RotationJournalPath) || File.Exists(Backup) || File.Exists(Candidate))
                throw new InvalidDataException("Unowned trust rotation files require inspection.");
            using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, state.Sequence);
            var bootstrap = release.Files.Single(file => file.Path == "NxTools.PlatformHost.exe");
            var next = new Version(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId,
                release.ManifestSha256, bootstrap.Sha256);
            if (next.Sequence <= state.Sequence)
                throw new InvalidDataException("Trust rotation downgrade/replay rejected.");
            var journal = new RotationJournal(oldBootstrapHash, bootstrap.Sha256, oldKeyHash,
                newKeyHash, Convert.ToBase64String(enrollmentBytes), FileHash(StatePath), next);
            var candidateOwned = false;
            try
            {
                using var source = IndexerServiceInstaller.TrustedBinary.Open(Path.Combine(package, bootstrap.Path), bootstrap.Sha256);
                using (var output = new FileStream(Candidate, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                {
                    candidateOwned = true;
                    source.CopyTo(output);
                    output.Flush(true);
                }
                Write(RotationJournalPath, journal);
            }
            catch
            {
                if (!File.Exists(RotationJournalPath) && candidateOwned) File.Delete(Candidate);
                throw;
            }
            Event("rotation_prepared", correlationId, clock);
            var committed = false;
            try
            {
                service.Stop();
                using var gate = service.EnterDeploymentGate();
                service.Validate(oldBootstrapHash, oldKeyHash);
                if (!Same(FileHash(StatePath), journal.ReleaseStateSha256)) throw new InvalidDataException("Release state changed during trust rotation.");
                RequireHash(Bootstrap, oldBootstrapHash);
                RequireHash(Candidate, bootstrap.Sha256);
                File.Move(Bootstrap, Backup);
                File.Move(Candidate, Bootstrap);
                service.SetTrustIdentity(oldBootstrapHash, oldKeyHash, bootstrap.Sha256, newKeyHash);
                var updated = enrollment with
                {
                    Identity = enrollment.Identity with
                    {
                        PublicKeySha256 = newKeyHash,
                        BootstrapSha256 = bootstrap.Sha256,
                        Sequence = next.Sequence,
                        ReleaseId = next.ReleaseId
                    }
                };
                WriteBytes(EnrollmentPath, JsonSerializer.SerializeToUtf8Bytes(updated));
                service.Start();
                service.ConfirmReady();
                RequireHash(Bootstrap, bootstrap.Sha256);
                if (!Same(FileHash(StatePath), journal.ReleaseStateSha256)) throw new InvalidDataException("Release state changed during trust rotation.");
                Write(VersionPath, next);
                committed = true;
                if (File.Exists(Backup)) { RequireHash(Backup, oldBootstrapHash); File.Delete(Backup); }
                if (File.Exists(RotationJournalPath)) File.Delete(RotationJournalPath);
                Event("rotation_committed", correlationId, clock);
            }
            catch
            {
                if (committed) throw;
                try
                {
                    service.Stop();
                    using var rollbackGate = service.EnterDeploymentGate();
                    if (File.Exists(Bootstrap)) { RequireHash(Bootstrap, bootstrap.Sha256); File.Delete(Bootstrap); }
                    if (File.Exists(Backup)) { RequireHash(Backup, oldBootstrapHash); File.Move(Backup, Bootstrap); }
                    if (File.Exists(Candidate)) { RequireHash(Candidate, bootstrap.Sha256); File.Delete(Candidate); }
                    service.SetTrustIdentity(bootstrap.Sha256, newKeyHash, oldBootstrapHash, oldKeyHash);
                    WriteBytes(EnrollmentPath, enrollmentBytes);
                    service.Start();
                    service.ConfirmReady();
                    if (File.Exists(RotationJournalPath)) File.Delete(RotationJournalPath);
                    Event("rotation_rolled_back", correlationId, clock);
                }
                catch (Exception rollbackFailure)
                {
                    throw new AggregateException("Trust rotation failed and rollback was incomplete.", rollbackFailure);
                }
                throw;
            }
            Event("rotation_completed", correlationId, clock);
        }
        catch (Exception failure)
        {
            Event("rotation_failed", correlationId, clock, failure.HResult);
            throw;
        }
    }

    internal void Apply(string package, Guid correlationId)
    {
        var clock = Stopwatch.StartNew();
        Event("accepted", correlationId, clock);
        try
        {
            using var operation = new FileStream(Path.Combine(Root, "bootstrap-maintenance.lock"),
                FileMode.OpenOrCreate, FileAccess.ReadWrite, FileShare.None);
            var keyHash = Hash(publicKey);
            // Recover a previous invocation using only its durable, exact identities, before new admission.
            if (File.Exists(JournalPath)) Recover(keyHash, correlationId, clock);
            var enrollmentBytes = ReadBytes(EnrollmentPath);
            var enrollment = JsonSerializer.Deserialize<Enrollment>(enrollmentBytes) ?? throw new InvalidDataException("Missing enrollment.");
            if (!enrollment.Completed || !Same(enrollment.Identity.PublicKeySha256, keyHash))
                throw new CryptographicException("Maintenance cannot change the enrolled trust.");
            var oldHash = FileHash(Bootstrap);
            if (!Same(oldHash, enrollment.Identity.BootstrapSha256))
                throw new InvalidDataException("Bootstrap enrollment identity mismatch.");
            service.Validate(oldHash, keyHash);
            var stateBytes = ReadBytes(StatePath);
            var state = JsonSerializer.Deserialize<TrustedUpdateTransaction.State>(stateBytes) ?? throw new InvalidDataException("Missing release state.");
            if (state.Sequence <= 0 || state.ReleaseId == Guid.Empty || !IsHash(state.ManifestSha256))
                throw new InvalidDataException("Invalid current release state.");
            if (File.Exists(Path.Combine(Root, "Releases", "transaction.json")))
                throw new InvalidOperationException("An unfinished product update must recover before maintenance.");
            using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
            var file = release.Files.Single(item => item.Path == "NxTools.PlatformHost.exe");
            var next = new Version(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId, release.ManifestSha256, file.Sha256);
            var previous = File.Exists(VersionPath) ? Read<Version>(VersionPath) : null;
            if (previous == next && Same(oldHash, next.BootstrapSha256))
            {
                service.Start(); service.ConfirmReady(); Event("already_current", correlationId, clock); return;
            }
            if (next.Sequence <= Math.Max(state.Sequence, previous?.Sequence ?? 0))
                throw new InvalidDataException("Bootstrap maintenance downgrade/replay rejected.");
            using (var snapshot = VerifyRecoverySnapshot(Path.Combine(Root, "Releases", "active"),
                state.Sequence, state.ReleaseId, state.ManifestSha256)) { }
            if (File.Exists(Backup) || File.Exists(Candidate)) throw new InvalidDataException("Unowned maintenance files require inspection.");
            var journal = new Journal(oldHash, file.Sha256, keyHash, Convert.ToBase64String(enrollmentBytes), Hash(stateBytes), next);
            // Stable source held through copy. Before journal publication this invocation owns the
            // freshly created candidate; after publication recovery owns it, including write-then-throw.
            var candidateOwned = false;
            try
            {
                using var source = IndexerServiceInstaller.TrustedBinary.Open(Path.Combine(package, file.Path), file.Sha256);
                using (var output = new FileStream(Candidate, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                { candidateOwned = true; source.CopyTo(output); output.Flush(true); }
                Write(JournalPath, journal);
            }
            catch
            {
                if (!File.Exists(JournalPath) && candidateOwned) File.Delete(Candidate);
                throw;
            }
            Event("prepared", correlationId, clock);
            var bootstrapMoved = false;
            try
            {
                service.Stop();
                using var gate = service.EnterDeploymentGate();
                RequireStateUnchanged(journal);
                RequireHash(Bootstrap, journal.OldHash);
                RequireHash(Candidate, journal.NewHash);
                File.Move(Bootstrap, Backup);
                bootstrapMoved = true;
                File.Move(Candidate, Bootstrap);
                service.SetBootstrapHash(journal.NewHash);
                var updated = enrollment with { Identity = enrollment.Identity with { BootstrapSha256 = journal.NewHash } };
                Write(EnrollmentPath, updated);
                service.Start();
                service.ConfirmReady();
                RequireHash(Bootstrap, journal.NewHash);
                RequireStateUnchanged(journal);
                // Point of no return: durable maintenance version only after the service is ready.
                Write(VersionPath, next);
                Event("committed", correlationId, clock);
            }
            catch
            {
                // A mutation-then-throw on version publication may already have committed.
                if (File.Exists(VersionPath) && Read<Version>(VersionPath) == next)
                    throw new IOException("Maintenance committed; cleanup will resume on retry.");
                if (!bootstrapMoved && !File.Exists(Backup) && Same(FileHash(Bootstrap), journal.OldHash))
                {
                    // An old broker may have completed an update before it stopped. No bootstrap
                    // mutation occurred: abort only our preparation, never restore its release state.
                    if (File.Exists(Candidate)) { RequireHash(Candidate, journal.NewHash); File.Delete(Candidate); }
                    service.Validate(journal.OldHash, keyHash);
                    File.Delete(JournalPath);
                    throw;
                }
                Recover(keyHash, correlationId, clock);
                throw;
            }
            CleanupCommitted(journal);
            Event("completed", correlationId, clock);
        }
        catch (Exception failure)
        {
            Event("failed", correlationId, clock, failure.HResult); throw;
        }
    }

    private void Recover(string keyHash, Guid correlation, Stopwatch clock)
    {
        var journal = Read<Journal>(JournalPath);
        ValidateJournal(journal, keyHash);
        if (File.Exists(VersionPath) && Read<Version>(VersionPath) == journal.Next)
        {
            service.Validate(journal.NewHash, keyHash);
            service.Start(); service.ConfirmReady(); CleanupCommitted(journal);
            Event("recovered_committed", correlation, clock); return;
        }
        Event("rollback_started", correlation, clock);
        service.Stop();
        using var gate = service.EnterDeploymentGate();
        RequireStateUnchanged(journal);
        // Either the rename has not happened, or the only removable bootstrap is exactly ours.
        if (File.Exists(Backup))
        {
            RequireHash(Backup, journal.OldHash);
            if (File.Exists(Bootstrap)) { RequireHash(Bootstrap, journal.NewHash); File.Delete(Bootstrap); }
            File.Move(Backup, Bootstrap);
        }
        RequireHash(Bootstrap, journal.OldHash);
        if (File.Exists(Candidate)) { RequireHash(Candidate, journal.NewHash); File.Delete(Candidate); }
        var oldEnrollment = Convert.FromBase64String(journal.EnrollmentBase64);
        WriteBytes(EnrollmentPath, oldEnrollment);
        service.SetBootstrapHash(journal.OldHash);
        service.Validate(journal.OldHash, keyHash);
        File.Delete(JournalPath);
        Event("rolled_back", correlation, clock);
    }

    private void CleanupCommitted(Journal journal)
    {
        RequireHash(Bootstrap, journal.NewHash);
        if (File.Exists(Backup)) { RequireHash(Backup, journal.OldHash); File.Delete(Backup); }
        if (File.Exists(Candidate)) throw new InvalidDataException("Unexpected committed candidate.");
        File.Delete(JournalPath);
    }
    private static void ValidateJournal(Journal value, string keyHash)
    {
        if (!Same(value.KeyHash, keyHash) || !IsHash(value.OldHash) || !IsHash(value.NewHash) ||
            !IsHash(value.ReleaseStateSha256) || value.Next.Sequence <= 0 || value.Next.ReleaseId == Guid.Empty ||
            !IsHash(value.Next.ManifestSha256) || !Same(value.NewHash, value.Next.BootstrapSha256))
            throw new InvalidDataException("Invalid maintenance recovery identity.");
        var enrollment = JsonSerializer.Deserialize<Enrollment>(Convert.FromBase64String(value.EnrollmentBase64));
        if (enrollment is null || !enrollment.Completed || !Same(enrollment.Identity.PublicKeySha256, keyHash) ||
            !Same(enrollment.Identity.BootstrapSha256, value.OldHash)) throw new InvalidDataException("Invalid prior enrollment.");
    }
    private void RequireStateUnchanged(Journal value)
    { if (!Same(FileHash(StatePath), value.ReleaseStateSha256)) throw new InvalidDataException("Release state changed during maintenance."); }
    private VerifiedRelease VerifyRecoverySnapshot(string path, long sequence, Guid releaseId, string manifestSha256)
    {
        try
        {
            return TrustedReleaseManifest.VerifyRecoverySnapshot(path, publicKey, sequence, releaseId, manifestSha256);
        }
        catch (Exception) when (!Same(Hash(publicKey), Hash(Convert.FromBase64String(LegacyUpdateTrust.PublicKeySpkiBase64))))
        {
            return TrustedReleaseManifest.VerifyRecoverySnapshot(path,
                Convert.FromBase64String(LegacyUpdateTrust.PublicKeySpkiBase64), sequence, releaseId, manifestSha256);
        }
    }
    private static bool Same(string? a, string? b) => string.Equals(a, b, StringComparison.OrdinalIgnoreCase);
    private static bool IsHash(string? value) => value is { Length: 64 } && value.All(char.IsAsciiHexDigit);
    private static string Hash(byte[] bytes) => Convert.ToHexString(SHA256.HashData(bytes));
    private static string FileHash(string path) { using var source = IndexerServiceInstaller.TrustedBinary.Open(path); return source.Sha256; }
    private static void RequireHash(string path, string hash) { using var source = IndexerServiceInstaller.TrustedBinary.Open(path, hash); }
    private static byte[] ReadBytes(string path)
    { using var stream = File.OpenRead(path); if (stream.Length > 16 * 1024) throw new InvalidDataException("Maintenance record too large."); var bytes = new byte[(int)stream.Length]; stream.ReadExactly(bytes); return bytes; }
    private static T Read<T>(string path) => JsonSerializer.Deserialize<T>(ReadBytes(path)) ?? throw new InvalidDataException("Missing maintenance record.");
    private static void Write<T>(string path, T value) => WriteBytes(path, JsonSerializer.SerializeToUtf8Bytes(value));
    private static void WriteBytes(string path, byte[] bytes)
    {
        var temp = path + ".maintenance-tmp";
        using (var stream = new FileStream(temp, FileMode.Create, FileAccess.Write, FileShare.None)) { stream.Write(bytes); stream.Flush(true); }
        File.Move(temp, path, true);
    }
    private void Event(string stage, Guid correlation, Stopwatch clock, int? native = null)
    { try { log.Write("info", "trusted_update.maintenance", correlation, new Dictionary<string, object?> { ["stage"] = stage, ["native_error_code"] = native, ["elapsed_ms"] = clock.ElapsedMilliseconds }); } catch { } }
}
