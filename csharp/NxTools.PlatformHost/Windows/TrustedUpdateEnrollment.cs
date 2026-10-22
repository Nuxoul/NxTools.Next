using System.Security.Cryptography;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows;

// Enrollment is the one explicit authorization boundary. Its durable identity cannot change
// on a retry: a partially provisioned bootstrap is not yet an enrolled update service.
internal sealed class TrustedUpdateEnrollment(string root)
{
    internal sealed record Identity(string PublicKeySha256, string BootstrapSha256, long Sequence, Guid ReleaseId);
    private sealed record Record(Identity Identity, bool Completed);
    private string RecordPath => Path.Combine(root, "enrollment.json");

    internal bool IsComplete()
    {
        if (!File.Exists(RecordPath)) return false;
        return Read().Completed;
    }

    internal void ValidateCandidate(Identity identity)
    {
        if (File.Exists(RecordPath) && Read().Identity != identity)
            throw new CryptographicException("Enrollment identity changed during recovery.");
    }

    internal void Run(Identity identity, Action prepareProtectedRoot, Action initialize,
        Action registerAndStart, Action validate)
    {
        // Harden first. No user-writeable marker is ever accepted as existing trust.
        prepareProtectedRoot();
        Record record;
        if (File.Exists(RecordPath))
        {
            record = Read();
            if (record.Identity != identity) throw new CryptographicException("Enrollment identity changed during recovery.");
            if (record.Completed) { validate(); return; }
        }
        else
        {
            record = new Record(identity, false);
            Write(record); // Before bootstrap copy, transaction state, or SCM mutation.
        }
        initialize(); // Idempotent, bound to this exact signed release.
        registerAndStart(); // Idempotent exact-owned service configuration/start.
        validate();
        Write(record with { Completed = true });
    }

    private Record Read()
    {
        using var stream = new FileStream(RecordPath, FileMode.Open, FileAccess.Read, FileShare.Read);
        if (stream.Length > 16 * 1024) throw new InvalidDataException("Enrollment record too large.");
        return JsonSerializer.Deserialize<Record>(stream) ?? throw new InvalidDataException("Enrollment record missing.");
    }
    private void Write(Record record)
    {
        var temporary = RecordPath + ".tmp";
        using (var stream = new FileStream(temporary, FileMode.Create, FileAccess.Write, FileShare.None))
        {
            JsonSerializer.Serialize(stream, record);
            stream.Flush(true);
        }
        File.Move(temporary, RecordPath, true);
    }
}
