using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

// One durable owner: signed staging -> deploy -> publish. Bootstrap runs separately from replaced binaries.
internal sealed class TrustedUpdateTransaction(string directory, byte[] publicKey,
    TrustedUpdateTransaction.IDeployment deployment, IPlatformLog log, byte[]? legacyPublicKey = null)
{
    internal interface IDeployment
    {
        Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken);
        // Recovery passes the authenticated durable snapshot rather than re-admitting it as a new package.
        Task DeployAsync(string verifiedPackage, VerifiedRelease verified, Guid correlationId, CancellationToken cancellationToken)
            => DeployAsync(verifiedPackage, correlationId, cancellationToken);
        Task RemoveSupersededAsync(VerifiedRelease previous, VerifiedRelease failed, CancellationToken cancellationToken)
            => Task.CompletedTask;
    }
    internal sealed record State(long Sequence, Guid ReleaseId, string ManifestSha256,
        [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)] bool DeploymentPending = false);
    private sealed record Journal(State Previous, State Next);
    private string Active => Path.Combine(directory, "active");
    private string Pending => Path.Combine(directory, "pending");
    private string Backup => Path.Combine(directory, "backup");
    private string StatePath => Path.Combine(directory, "state.json");
    private string JournalPath => Path.Combine(directory, "transaction.json");

    internal State ReadState() => Read<State>(StatePath);

    internal void Initialize(string package, bool requireInitialDeployment = false)
    {
        using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
        var state = new State(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId, release.ManifestSha256,
            requireInitialDeployment);
        if (File.Exists(StatePath))
        {
            var existing = ReadState();
            if (existing with { DeploymentPending = requireInitialDeployment } != state ||
                requireInitialDeployment && !existing.DeploymentPending)
                throw new InvalidDataException("Enrollment snapshot changed.");
            VerifySnapshot(Active, existing).Dispose();
            return;
        }
        if (Directory.Exists(Active))
        {
            VerifySnapshot(Active, state).Dispose();
            Write(StatePath, state);
            return;
        }
        Clean(Pending);
        Directory.CreateDirectory(Pending);
        release.CopyTo(Pending);
        Directory.Move(Pending, Active);
        Write(StatePath, state);
    }

    internal async Task RecoverAsync(Guid correlationId, CancellationToken cancellationToken)
    {
        if (!File.Exists(JournalPath)) return;
        var journal = Read<Journal>(JournalPath);
        var current = ReadState();
        if (current == journal.Next)
        {
            // State commits only after deployment succeeded and the verified active snapshot was published.
            VerifySnapshot(Active, journal.Next).Dispose();
            Clean(Backup);
            Clean(Pending);
            File.Delete(JournalPath);
            Event("recovered_committed", correlationId, 0);
            return;
        }
        if (current != journal.Previous) throw new InvalidDataException("Update recovery state mismatch.");
        var previous = Directory.Exists(Backup) ? Backup : Active;
        var failedPath = Directory.Exists(Pending) ? Pending : Active;
        using (var verified = VerifySnapshot(previous, journal.Previous))
        using (var failed = VerifySnapshot(failedPath, journal.Next))
        {
            await deployment.DeployAsync(previous, verified, correlationId, cancellationToken);
            await deployment.RemoveSupersededAsync(verified, failed, cancellationToken);
        }
        if (previous == Backup)
        {
            Clean(Active);
            Directory.Move(Backup, Active);
        }
        Clean(Pending);
        File.Delete(JournalPath);
        Event("recovered_previous", correlationId, 0);
    }

    internal async Task<State> ApplyAsync(string package, Guid correlationId, CancellationToken cancellationToken)
    {
        using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
        return await ApplyVerifiedAsync(release, correlationId, cancellationToken);
    }

    internal async Task<State> ApplyVerifiedAsync(VerifiedRelease release,
        Guid correlationId, CancellationToken cancellationToken)
    {
        // Even an internal recovery VerifiedRelease must never become new-request admission.
        TrustedReleaseManifest.ValidateApplicationRelease(release, publicKey);
        var started = System.Diagnostics.Stopwatch.GetTimestamp();
        Event("accepted", correlationId, started);
        await RecoverAsync(correlationId, cancellationToken);
        cancellationToken.ThrowIfCancellationRequested();
        var previous = ReadState();
        var next = new State(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId, release.ManifestSha256);
        if (previous.DeploymentPending)
        {
            if (next.Sequence < previous.Sequence || next.Sequence == previous.Sequence &&
                (next.ReleaseId != previous.ReleaseId || next.ManifestSha256 != previous.ManifestSha256))
                throw new InvalidDataException("Release downgrade/replay rejected.");
            // A repair snapshot is an ordinary complete signed package, not recovery-only admission.
            using (var initial = TrustedReleaseManifest.VerifyPackage(Active, publicKey, 0))
            {
                if (initial.Manifest.ReleaseSequence != previous.Sequence ||
                    initial.Manifest.ReleaseId != previous.ReleaseId || initial.ManifestSha256 != previous.ManifestSha256)
                    throw new InvalidDataException("Initial deployment snapshot changed.");
                Event("initial_deployment_started", correlationId, started);
                await deployment.DeployAsync(Active, initial, correlationId, CancellationToken.None);
            }
            previous = previous with { DeploymentPending = false };
            Write(StatePath, previous);
            Event("initial_deployment_committed", correlationId, started);
        }
        if (next == previous)
        {
            VerifySnapshot(Active, previous).Dispose();
            Event("already_current", correlationId, started);
            return previous;
        }
        if (next.Sequence <= previous.Sequence) throw new InvalidDataException("Release downgrade/replay rejected.");
        VerifySnapshot(Active, previous).Dispose();
        Clean(Pending);
        Directory.CreateDirectory(Pending);
        release.CopyTo(Pending);
        VerifySnapshot(Pending, next).Dispose();
        cancellationToken.ThrowIfCancellationRequested();
        // Journal is durable before any deployment mutation. Once prepared, finish or recover;
        // losing the requesting UI/pipe does not cancel a privileged deployment midway.
        Write(JournalPath, new Journal(previous, next));
        Event("prepared", correlationId, started);
        try
        {
            await deployment.DeployAsync(Pending, correlationId, CancellationToken.None);
            Directory.Move(Active, Backup);
            Directory.Move(Pending, Active);
            Write(StatePath, next);
            Event("committed", correlationId, started);
        }
        catch
        {
            await RecoverAsync(correlationId, CancellationToken.None);
            Event("rolled_back", correlationId, started);
            throw;
        }
        // Cleanup failure is recoverable from the committed journal; never roll back after commit.
        await RecoverAsync(correlationId, CancellationToken.None);
        return next;
    }

    private VerifiedRelease VerifySnapshot(string path, State expected)
    {
        try
        {
            return TrustedReleaseManifest.VerifyRecoverySnapshot(path, publicKey, expected.Sequence, expected.ReleaseId, expected.ManifestSha256);
        }
        catch (Exception) when (legacyPublicKey is not null)
        {
            return TrustedReleaseManifest.VerifyRecoverySnapshot(path, legacyPublicKey, expected.Sequence, expected.ReleaseId, expected.ManifestSha256);
        }
    }

    private static T Read<T>(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        if (stream.Length > 16 * 1024) throw new InvalidDataException("Update state is oversized.");
        return JsonSerializer.Deserialize<T>(stream) ?? throw new InvalidDataException("Update state is missing.");
    }

    private static void Write<T>(string path, T value)
    {
        var temporary = path + ".tmp";
        using (var stream = new FileStream(temporary, FileMode.Create, FileAccess.Write, FileShare.None))
        {
            JsonSerializer.Serialize(stream, value);
            stream.Flush(true);
        }
        File.Move(temporary, path, true);
    }

    private static void Clean(string path)
    {
        if (!Directory.Exists(path)) return;
        RejectReparseTree(path);
        Directory.Delete(path, true);
    }
    internal static void RejectReparseTree(string root)
    {
        if ((File.GetAttributes(root) & FileAttributes.ReparsePoint) != 0)
            throw new InvalidDataException("Reparse update storage rejected.");
        foreach (var entry in Directory.EnumerateFileSystemEntries(root))
        {
            var attributes = File.GetAttributes(entry);
            if ((attributes & FileAttributes.ReparsePoint) != 0)
                throw new InvalidDataException("Reparse update storage rejected.");
            if ((attributes & FileAttributes.Directory) != 0) RejectReparseTree(entry);
        }
    }
    private void Event(string stage, Guid correlationId, long started)
    {
        try
        {
            log.Write("info", "trusted_update.transaction", correlationId, new Dictionary<string, object?>
            {
                ["stage"] = stage,
                ["elapsed_ms"] = started == 0 ? 0 : System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        }
        catch { /* Observability cannot alter deployment. */ }
    }
}
