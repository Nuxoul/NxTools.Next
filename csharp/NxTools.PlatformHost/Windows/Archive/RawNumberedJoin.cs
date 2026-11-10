using System.Globalization;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed record RawJoinPart(string Name, long Length, string Version);
internal sealed record RawJoinPreview(Guid PreviewId, string SelectedPath, string SuggestedOutput, RawJoinPart[] Parts, long TotalBytes);
internal sealed record RawJoinResult(string Outcome, string? Code, long BytesWritten, bool CleanupComplete, int? NativeErrorCode = null);

// Observes actual transfer boundaries in owned fixtures; no content or per-chunk telemetry.
internal sealed class RawJoinObservation
{
    internal int BufferBytes, MaxRead, MaxWrite, Reads, Writes;
    internal long BytesRead, BytesWritten;
    internal Action<ArchiveOutputSession>? ConfigureOutput;
    internal Action<string>? Checkpoint;
}

internal static class RawNumberedJoin
{
    internal const int MaximumParts = 1024;
    internal const long MaximumBytes = 34359738368L;
    private const int BufferSize = 65536;

    private static (string Directory, string Stem) Family(string selected)
    {
        var name = Path.GetFileName(selected);
        var dot = name.LastIndexOf('.');
        if (dot < 1 || !Ordinal(name[(dot + 1)..], out _)) throw new ArchiveOutputException("invalid_family");
        return (Path.GetDirectoryName(selected) ?? throw new ArchiveOutputException("invalid_source"), name[..dot]);
    }
    private static bool Ordinal(string suffix, out int ordinal)
    {
        ordinal = 0;
        return suffix.Length >= 3 && suffix.All(char.IsAsciiDigit) &&
            int.TryParse(suffix, NumberStyles.None, CultureInfo.InvariantCulture, out ordinal) &&
            ordinal is >= 1 and <= MaximumParts && suffix == ordinal.ToString("D3", CultureInfo.InvariantCulture);
    }
    private static string[] Names(string directory, string stem, CancellationToken cancellationToken)
    {
        var names = new SortedDictionary<int, string>();
        var nameBytes = 0;
        foreach (var path in Directory.EnumerateFileSystemEntries(directory))
        {
            cancellationToken.ThrowIfCancellationRequested();
            var name = Path.GetFileName(path);
            if (!name.StartsWith(stem + ".", StringComparison.OrdinalIgnoreCase)) continue;
            var suffix = name[(stem.Length + 1)..];
            if (suffix.Length == 0 || !suffix.All(char.IsAsciiDigit)) continue;
            if (!Ordinal(suffix, out var ordinal) || !names.TryAdd(ordinal, name)) throw new ArchiveOutputException("ambiguous_family");
            nameBytes = checked(nameBytes + System.Text.Encoding.UTF8.GetByteCount(name));
            if (nameBytes > 262144) throw new ArchiveOutputException("family_limit");
            if (names.Count > MaximumParts) throw new ArchiveOutputException("family_limit");
        }
        if (names.Count == 0 || names.Keys.Where((ordinal, index) => ordinal != index + 1).Any())
            throw new ArchiveOutputException("missing_part");
        return names.Values.ToArray();
    }
    internal static RawJoinPreview Inspect(string selected, CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var selectedLease = ArchiveSourceLease.Open(selected);
        var (directory, stem) = Family(selected);
        var names = Names(directory, stem, cancellationToken);
        if (!names.Contains(Path.GetFileName(selected), StringComparer.OrdinalIgnoreCase)) throw new ArchiveOutputException("invalid_family");
        var parts = new List<RawJoinPart>(names.Length);
        long total = 0;
        foreach (var name in names)
        {
            cancellationToken.ThrowIfCancellationRequested();
            using var lease = ArchiveSourceLease.Open(Path.Combine(directory, name));
            var length = OutputNative.Identity(lease.InspectionSource).Size;
            if (length < 0 || length > MaximumBytes - total) throw new ArchiveOutputException("length_limit");
            total += length;
            parts.Add(new(name, length, lease.ReadmissionIdentity()));
        }
        if (!Names(directory, stem, cancellationToken).SequenceEqual(names, StringComparer.Ordinal)) throw new ArchiveOutputException("source_changed");
        return new(Guid.NewGuid(), selected, Path.Combine(directory, stem), parts.ToArray(), total);
    }
    internal static RawJoinResult Join(RawJoinPreview preview, string outputPath, Guid operation,
        CancellationToken cancellationToken = default, RawJoinObservation? observation = null)
    {
        var sources = new List<ArchiveSourceLease>();
        ArchiveOutputSession? output = null;
        Guid? token = null;
        long written = 0;
        var committed = false;
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var current = Inspect(preview.SelectedPath, cancellationToken);
            if (!current.Parts.SequenceEqual(preview.Parts) || current.TotalBytes != preview.TotalBytes) throw new ArchiveOutputException("source_changed");
            var (directory, stem) = Family(preview.SelectedPath);
            var outputParent = Path.GetDirectoryName(outputPath);
            var outputName = Path.GetFileName(outputPath);
            if (string.Equals(Path.GetFullPath(outputParent ?? ""), Path.GetFullPath(directory), StringComparison.OrdinalIgnoreCase) &&
                outputName.StartsWith(stem + ".", StringComparison.OrdinalIgnoreCase) &&
                outputName[(stem.Length + 1)..] is { Length: > 0 } outputSuffix && outputSuffix.All(char.IsAsciiDigit))
                throw new ArchiveOutputException("output_is_part");
            foreach (var part in preview.Parts)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var lease = ArchiveSourceLease.Open(Path.Combine(directory, part.Name));
                sources.Add(lease);
                if (lease.ReadmissionIdentity() != part.Version) throw new ArchiveOutputException("source_changed");
            }
            observation?.Checkpoint?.Invoke("sources_admitted");
            var parent = Path.GetDirectoryName(outputPath) ?? throw new ArchiveOutputException("invalid_root");
            output = ArchiveOutputSession.Prepare(parent, operation);
            observation?.ConfigureOutput?.Invoke(output);
            var destination = output.InspectDestination();
            if (destination.Missing) throw new ArchiveOutputException("destination_missing");
            if (destination.Available < (ulong)preview.TotalBytes) throw new ArchiveOutputException("insufficient_space");
            cancellationToken.ThrowIfCancellationRequested();
            token = output.BeginExtractedFile(Path.GetFileName(outputPath), overwrite: false, cancellationToken);
            var buffer = new byte[BufferSize];
            if (observation is not null) observation.BufferBytes = buffer.Length;
            for (var index = 0; index < sources.Count; index++)
            {
                var source = sources[index]; var part = preview.Parts[index]; long offset = 0;
                while (offset < part.Length)
                {
                    cancellationToken.ThrowIfCancellationRequested();
                    var read = RandomAccess.Read(source.InspectionSource, buffer.AsSpan(0, (int)Math.Min(BufferSize, part.Length - offset)), offset);
                    if (read == 0) throw new ArchiveOutputException("source_changed");
                    if (observation is not null) { observation.Reads++; observation.BytesRead += read; observation.MaxRead = Math.Max(observation.MaxRead, read); }
                    observation?.Checkpoint?.Invoke("read");
                    cancellationToken.ThrowIfCancellationRequested();
                    output.Write(token.Value, buffer.AsMemory(0, read));
                    offset += read; written += read;
                    if (observation is not null) { observation.Writes++; observation.BytesWritten += read; observation.MaxWrite = Math.Max(observation.MaxWrite, read); }
                    observation?.Checkpoint?.Invoke("write");
                }
                if (source.ReadmissionIdentity() != part.Version) throw new ArchiveOutputException("source_changed");
            }
            if (!Names(directory, stem, cancellationToken).SequenceEqual(preview.Parts.Select(p => p.Name), StringComparer.Ordinal)) throw new ArchiveOutputException("source_changed");
            observation?.Checkpoint?.Invoke("before_commit");
            var result = output.Commit(token.Value, cancellationToken);
            if (result.Category != "committed") return new(result.Category == "cancelled" ? "cancelled" : "failed", result.Category, written, result.Category == "cancelled", result.NativeCode);
            committed = true;
            observation?.Checkpoint?.Invoke("committed");
            return new("succeeded", null, written, true);
        }
        catch (Exception failure) when (failure is ArchiveOutputException or OperationCanceledException or IOException or UnauthorizedAccessException or OverflowException or ArgumentException)
        {
            if (committed) return new("succeeded", null, written, true);
            var code = failure switch { ArchiveOutputException native => native.Stage, OperationCanceledException => "cancelled", OverflowException => "length_limit", ArgumentException => "invalid_source", _ => "file_io" };
            int? nativeCode = failure is ArchiveOutputException e ? e.NativeCode : failure is IOException or UnauthorizedAccessException ? failure.HResult & 0xffff : null;
            if (output is not null && token is {} owned)
            {
                var cleanup = output.Cancel(owned);
                if (cleanup.Category != "partial_removed") return new("failed", "cleanup_failed", written, false, cleanup.NativeCode);
            }
            return new(code == "cancelled" ? "cancelled" : "failed", code, written, true, nativeCode);
        }
        finally
        {
            output?.Dispose();
            foreach (var source in sources) source.Dispose();
        }
    }
}
