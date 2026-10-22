using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows;

// One durable owner: signed staging -> deploy -> publish. Bootstrap runs separately from replaced binaries.
internal sealed class TrustedUpdateTransaction(string directory, byte[] publicKey,
    TrustedUpdateTransaction.IDeployment deployment, IPlatformLog log)
{
    internal interface IDeployment
    {
        Task DeployAsync(string verifiedPackage, Guid correlationId, CancellationToken cancellationToken);
    }
    internal sealed record State(long Sequence, Guid ReleaseId, string ManifestSha256);
    private sealed record Journal(State Previous, State Next);
    private string Active => Path.Combine(directory, "active");
    private string Pending => Path.Combine(directory, "pending");
    private string Backup => Path.Combine(directory, "backup");
    private string StatePath => Path.Combine(directory, "state.json");
    private string JournalPath => Path.Combine(directory, "transaction.json");

    internal State ReadState() => Read<State>(StatePath);

    internal void Initialize(string package)
    {
        using var release = TrustedReleaseManifest.VerifyPackage(package, publicKey, 0);
        var state = new State(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId, release.ManifestSha256);
        if (File.Exists(StatePath))
        {
            if (ReadState() != state) throw new InvalidDataException("Enrollment snapshot changed.");
            VerifySnapshot(Active, state);
            return;
        }
        if (Directory.Exists(Active))
        {
            VerifySnapshot(Active, state);
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
            VerifySnapshot(Active, journal.Next);
            Clean(Backup);
            Clean(Pending);
            File.Delete(JournalPath);
            Event("recovered_committed", correlationId, 0);
            return;
        }
        if (current != journal.Previous) throw new InvalidDataException("Update recovery state mismatch.");
        var previous = Directory.Exists(Backup) ? Backup : Active;
        VerifySnapshot(previous, journal.Previous);
        await deployment.DeployAsync(previous, correlationId, cancellationToken);
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
        var started = System.Diagnostics.Stopwatch.GetTimestamp();
        Event("accepted", correlationId, started);
        await RecoverAsync(correlationId, cancellationToken);
        cancellationToken.ThrowIfCancellationRequested();
        var previous = ReadState();
        var next = new State(release.Manifest.ReleaseSequence, release.Manifest.ReleaseId, release.ManifestSha256);
        if (next == previous)
        {
            VerifySnapshot(Active, previous);
            Event("already_current", correlationId, started);
            return previous;
        }
        if (next.Sequence <= previous.Sequence) throw new InvalidDataException("Release downgrade/replay rejected.");
        VerifySnapshot(Active, previous);
        Clean(Pending);
        Directory.CreateDirectory(Pending);
        release.CopyTo(Pending);
        VerifySnapshot(Pending, next);
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

    private void VerifySnapshot(string path, State expected)
    {
        using var verified = TrustedReleaseManifest.VerifyPackage(path, publicKey, 0);
        if (verified.Manifest.ReleaseSequence != expected.Sequence || verified.Manifest.ReleaseId != expected.ReleaseId ||
            verified.ManifestSha256 != expected.ManifestSha256)
            throw new InvalidDataException("Recovery snapshot identity mismatch.");
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
