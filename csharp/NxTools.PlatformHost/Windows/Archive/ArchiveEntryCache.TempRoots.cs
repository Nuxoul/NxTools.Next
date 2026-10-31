using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveEntryCache
{
    internal const int MaxCustomRoots = 16;
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record RootRecord(int Version, string BaseDirectory, FileId Identity);
    private readonly Dictionary<string, FileId> discoveredRoots = new(StringComparer.OrdinalIgnoreCase);
    private sealed class RootProtection(List<SafeFileHandle> held) : IDisposable { public void Dispose() => CloseChain(held); }
    private IDisposable ProtectSweepRoot(string path)
    {
        var held = OpenRootChain(path);
        try {
            if (discoveredRoots.TryGetValue(path, out var expected) && FileId.Read(held[^1]) != expected) throw new ArchiveOutputException("source_changed");
            return new RootProtection(held);
        } catch { CloseChain(held); throw; }
    }

    private static List<SafeFileHandle> OpenRootChain(string path)
    {
        var held = new List<SafeFileHandle>();
        try {
            held.Add(OutputNative.Open(null, "\\??\\" + char.ToUpperInvariant(path[0]) + ":\\", true));
            foreach (var component in path[3..].Split('\\')) {
                ArchiveOutputSession.Validate(component); held.Add(OutputNative.Open(held[^1], component, true));
            }
            return held;
        } catch { CloseChain(held); throw; }
    }
    private static void CloseChain(List<SafeFileHandle> held) { foreach (var handle in held.AsEnumerable().Reverse()) handle.Dispose(); }
    private string InventoryRoot => Path.Combine(Path.GetDirectoryName(cacheRoot)!, "temp-roots");
    // Fixed slots, not a growing directory scan. Existing records are NEVER overwritten or
    // evicted: a crash during a new slot cannot corrupt the discovery authority for old roots.
    private RootRecord? ReadRoot(SafeFileHandle parent, int slot)
    {
        FileStream stream;
        try { stream = new FileStream(OutputNative.Open(parent, slot + ".json", false), FileAccess.ReadWrite); }
        catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) { return null; }
        using (stream) {
            if (stream.Length is < 1 or > 262144) throw new InvalidDataException();
            var bytes = new byte[(int)stream.Length]; stream.ReadExactly(bytes);
            var record = JsonSerializer.Deserialize<RootRecord>(bytes) ?? throw new InvalidDataException();
            if (record.Version != 1 || record.Identity is null || string.IsNullOrEmpty(record.BaseDirectory)) throw new InvalidDataException();
            new ArchiveTempPolicy(record.BaseDirectory, 24, false).Validate();
            return record;
        }
    }
    private string AdmitCustomRoot(ArchiveTempPolicy policy)
    {
        // Prepare examines real existing no-reparse ancestry BEFORE any custom creation.
        using var prepared = ArchiveOutputSession.Prepare(policy.SessionsRoot, Guid.NewGuid());
        using var inventory = ArchiveOutputSession.Open(InventoryRoot, Guid.NewGuid());
        var parents = OpenRootChain(InventoryRoot);
        try {
            // Cross-pipe admission serializes at one exact owned file; busy fails closed.
            using var admission = new FileStream(OutputNative.Open(parents[^1], "admission.lock", false, create: true), FileAccess.ReadWrite);
            var empty = -1; RootRecord? prior = null;
            for (var slot = 0; slot < MaxCustomRoots; slot++) {
                var record = ReadRoot(parents[^1], slot);
                if (record is null) { if (empty < 0) empty = slot; continue; }
                if (string.Equals(record.BaseDirectory.TrimEnd('\\'), policy.BaseDirectory.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase)) prior = record;
            }
            if (prior is null && empty < 0) throw new ArchiveOutputException("resource_limit");
            prepared.CreatePreparedRoot();
            var custom = OpenRootChain(policy.SessionsRoot);
            try {
                var identity = FileId.Read(custom[^1]);
                if (prior is not null) { if (identity != prior.Identity) throw new ArchiveOutputException("source_changed"); }
                else {
                    var normalized = policy.BaseDirectory.TrimEnd('\\'); if (normalized.Length == 2) normalized += "\\";
                    var bytes = JsonSerializer.SerializeToUtf8Bytes(new RootRecord(1, normalized, identity));
                    if (bytes.Length > 262144) throw new ArchiveOutputException("resource_limit");
                    using var record = new FileStream(OutputNative.Open(parents[^1], empty + ".json", false, create: true, exclusive: true), FileAccess.ReadWrite);
                    record.Write(bytes); record.Flush(true); // Durable discovery precedes any lease/manifest.
                }
            } finally { CloseChain(custom); }
            return policy.SessionsRoot;
        } finally { CloseChain(parents); }
    }
    private string[] InventoryRoots()
    {
        discoveredRoots.Clear();
        var roots = new List<string> { cacheRoot };
        List<SafeFileHandle> parents;
        try { parents = OpenRootChain(InventoryRoot); }
        catch (Exception failure) when (Recoverable(failure)) { return roots.ToArray(); }
        try {
            for (var slot = 0; slot < MaxCustomRoots; slot++) {
                try {
                    var record = ReadRoot(parents[^1], slot); if (record is null) continue;
                    var path = new ArchiveTempPolicy(record.BaseDirectory, 24, false).SessionsRoot;
                    var custom = OpenRootChain(path);
                    try {
                        if (FileId.Read(custom[^1]) == record.Identity && !roots.Contains(path, StringComparer.OrdinalIgnoreCase)) { roots.Add(path); discoveredRoots[path] = record.Identity; }
                    }
                    finally { CloseChain(custom); }
                } catch (Exception failure) when (Recoverable(failure)) { }
            }
        } finally { CloseChain(parents); }
        return roots.ToArray();
    }
}
