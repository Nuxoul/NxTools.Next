using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

internal enum ArchiveNativeReadStage
{
    EditOpen, EditVerification, EditReadsComplete, EditHandoff, Prepublication,
    TargetAdmission, TargetBegin, OutputFingerprint, OutputPostpublication
}
internal enum ArchiveNativeReadKind { Source, Target, Output }

// Worker-thread-owned counters only. No paths, identities, hashes or reusable authority are retained.
internal sealed class ArchiveNativeReadObservation
{
    internal sealed record Counts(string Stage, string Kind, long Attempts, long FullReads, long BytesRead, long CompletedBytes);
    private struct Counter
    {
        internal long Attempts, FullReads, BytesRead, CompletedBytes;
    }
    private readonly Counter[,] counters = new Counter[9, 3];
    internal long PartialReadCalls { get; private set; }
    internal long PartialReadBytes { get; private set; }
    internal long FullReads { get; private set; }
    internal long BytesRead { get; private set; }
    // Instance-only deterministic cancellation seam; never installed by production callers.
    internal Action<ArchiveNativeReadStage, ArchiveNativeReadKind, long>? ReadChunkProbeForFixture { get; set; }

    // Count bytes returned by the real stream, not the file's advertised length. A full read is
    // credited only after EOF and digest completion; interrupted attempts retain their actual bytes.
    internal byte[] Hash(Stream stream, ArchiveNativeReadStage stage, ArchiveNativeReadKind kind,
        CancellationToken cancellationToken = default)
    {
        long bytes = 0;
        var complete = false;
        try
        {
            using var digest = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            var buffer = new byte[65536];
            while (true)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var count = stream.Read(buffer);
                if (count == 0) break;
                bytes += count;
                digest.AppendData(buffer, 0, count);
                ReadChunkProbeForFixture?.Invoke(stage, kind, bytes);
            }
            var hash = digest.GetHashAndReset();
            complete = true;
            return hash;
        }
        finally
        {
            ref var counter = ref counters[(int)stage, (int)kind];
            counter.Attempts++;
            counter.BytesRead += bytes;
            BytesRead += bytes;
            if (complete)
            {
                counter.FullReads++;
                counter.CompletedBytes += bytes;
                FullReads++;
            }
        }
    }
    internal void PartialRead(int count)
    {
        PartialReadCalls++;
        PartialReadBytes += count;
    }
    internal Counts[] Snapshot()
    {
        var result = new List<Counts>();
        foreach (var stage in Enum.GetValues<ArchiveNativeReadStage>())
            foreach (var kind in Enum.GetValues<ArchiveNativeReadKind>())
            {
                var counter = counters[(int)stage, (int)kind];
                if (counter.Attempts != 0)
                    result.Add(new(stage.ToString(), kind.ToString(), counter.Attempts, counter.FullReads,
                        counter.BytesRead, counter.CompletedBytes));
            }
        return result.ToArray();
    }
}
