using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

// Finite metadata accounting. Never invent engine identity or encoding defaults.
internal sealed class ArchiveReadMetadata(ArchiveReadLimits limits, uint selectedCodepage, uint version = 1)
{
    internal ulong Entries { get; private set; }
    internal ulong Batches { get; private set; }
    internal ulong Bytes { get; private set; }
    internal ulong MaxBatchEntries { get; private set; }
    internal ulong MaxBatchBytes { get; private set; }
    private Guid? snapshot;
    internal ArchiveBrowserContext? Browser { get; private set; }
    internal void ReadBrowser(JsonElement frame) {
        if (version != 2 || !limits.BrowserContext || Browser is not null || Entries != 0 || Batches != 0) throw new InvalidDataException();
        var browser = frame.GetProperty("browser").Deserialize<ArchiveBrowserContext>(Strict) ?? throw new InvalidDataException();
        if (!browser.Valid) throw new InvalidDataException();
        var bytes = (ulong)JsonSerializer.SerializeToUtf8Bytes(browser, JsonDefaults.Options).Length;
        if (bytes > limits.MaxRetainedBytes - Math.Min(Bytes, limits.MaxRetainedBytes)) throw new ArchiveReadBudgetException();
        Browser = browser; snapshot = browser.SnapshotId; format = browser.Format; Bytes += bytes;
    }
    private string? format;
    private static readonly JsonSerializerOptions Strict = new(JsonDefaults.Options) {
        PropertyNameCaseInsensitive = false, NumberHandling = System.Text.Json.Serialization.JsonNumberHandling.Strict };
    internal ArchiveReadBatch Read(JsonElement frame)
    {
        var id = frame.GetProperty("snapshot_id").GetGuid();
        var sequence = frame.GetProperty("batch_sequence").GetUInt64();
        var name = frame.GetProperty("format").GetString()!;
        var raw = frame.GetProperty("entries");
        if (id == Guid.Empty || sequence != Batches || snapshot is { } prior && prior != id ||
            format is { } previous && previous != name || name is not ("Zip" or "SevenZ" or "Rar4" or "Rar5" or "Tar" or "TarGz" or "TarBz2" or "TarXz" or "TarZstd" or "Gzip" or "Bzip2" or "Xz" or "Zstd" or "Cab" or "Iso" or "Udf" or "Wim" or "Ar" or "Cpio" or "Lzma" or "Lz4" or "Lz4Block" or "Brotli") ||
            raw.ValueKind != JsonValueKind.Array || raw.GetArrayLength() < 1 || raw.GetArrayLength() > (version == 2 ? 256 : 2)) throw new InvalidDataException();
        var entries = new ArchiveReadEntry[raw.GetArrayLength()];
        for (var index = 0; index < entries.Length; index++) {
            var rawEntry = raw[index];
            var ordinal = rawEntry.GetProperty("ordinal").GetUInt64();
            var entry = rawEntry.Deserialize<ArchiveReadEntry>(Strict) ?? throw new InvalidDataException();
            if (ordinal != entry.Ordinal) throw new InvalidDataException();
            if (entry.Modified is { } modified && modified.ValueKind != JsonValueKind.Object) throw new InvalidDataException();
            if (entry.Ordinal != Entries + (ulong)index || entry.Name is null || entry.Kind is not ("File" or "Directory" or "Link" or "Special") ||
                entry.WarningCodes is null || entry.WarningCodes.Length > 32 || entry.WarningCodes.Any(w => w is null || w.Length > 128) ||
                entry.Provenance is not ("Utf8Flag" or "UnicodeExtra" or "FormatUnicode" or "Explicit" or "UncertainCp437" or "Heuristic") ||
                (entry.Provenance == "Heuristic"
                    ? selectedCodepage != 0 || entry.Codepage is not (65001 or 936 or 932)
                    : entry.Codepage != (entry.Provenance == "Explicit" ? selectedCodepage : entry.Provenance == "UncertainCp437" ? 437u : 65001u)) ||
                entry.Provenance == "Explicit" && selectedCodepage == 0) throw new InvalidDataException();
            entries[index] = entry;
        }
        var batch = new ArchiveReadBatch(id, sequence, name, entries);
        var bytes = (ulong)JsonSerializer.SerializeToUtf8Bytes(batch, JsonDefaults.Options).Length;
        if (bytes > (version == 2 ? 262144ul : 524288ul) || bytes > limits.MaxRetainedBytes - Math.Min(Bytes, limits.MaxRetainedBytes) ||
            version == 1 && bytes > limits.MaxMetadataBytes - Math.Min(Bytes, limits.MaxMetadataBytes) ||
            (ulong)entries.Length > limits.MaxEntries - Math.Min(Entries, limits.MaxEntries)) throw new ArchiveReadBudgetException();
        snapshot = id; format = name; Bytes += bytes; Entries += (ulong)entries.Length; Batches++;
        MaxBatchEntries = Math.Max(MaxBatchEntries, (ulong)entries.Length); MaxBatchBytes = Math.Max(MaxBatchBytes, bytes);
        return batch;
    }
    internal bool Complete(JsonElement frame, bool open)
    {
        // Legacy private complete already carries actual open entry count as files_verified.
        // Public terminal batch_count is the number of contiguous original sequences observed here.
        if (open && limits.BrowserContext && Browser is null) return false;
        if (version == 2 && (!frame.TryGetProperty("entry_count", out _) || !frame.TryGetProperty("batch_count", out _))) return false;
        if (frame.TryGetProperty("entry_count", out var entries) && entries.GetUInt64() != Entries ||
            frame.TryGetProperty("batch_count", out var batches) && batches.GetUInt64() != Batches) return false;
        return open ? frame.GetProperty("files_verified").GetUInt64() == Entries : Entries == 0 && Batches == 0;
    }
}
internal sealed class ArchiveReadBudgetException : Exception;
