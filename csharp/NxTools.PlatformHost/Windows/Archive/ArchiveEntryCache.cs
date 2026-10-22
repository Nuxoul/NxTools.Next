using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

// One selected file per GUID lease. Never recursive cleanup or Shell-process lifetime inference.
internal sealed class ArchiveEntryCache : IDisposable
{
    internal static readonly TimeSpan Retention = TimeSpan.FromHours(24);
    private readonly string cacheRoot;
    private readonly TimeProvider clock;
    private readonly IPlatformLog? log;
    private readonly Dictionary<Guid, Lease> leases = [];
    private readonly HashSet<Guid> released = [];
    private readonly Queue<Guid> releasedOrder = new();
    private readonly Queue<Guid> localRetirements = new();
    private IEnumerator<string>? sweep;
    private bool disposed;
    internal Task StartupCleanup { get; }

    private ArchiveEntryCache(string root, TimeProvider? clock = null, IPlatformLog? log = null)
    {
        cacheRoot = Path.Combine(root, "archive", "sessions");
        this.clock = clock ?? TimeProvider.System; this.log = log;
        StartupCleanup = Task.Run(() => { lock (this) { if (!disposed) Sweep(); } });
    }
    internal static ArchiveEntryCache Production(IPlatformLog? log = null)
    {
        var local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        if (!Path.IsPathFullyQualified(local)) throw new ArchiveOutputException("source_unavailable");
        return new(Path.Combine(local, "NxTools", "NxTools.Next", "cache"), log: log);
    }
    internal static ArchiveEntryCache ForOwnedTestCache(string root, TimeProvider? clock = null)
    { ArchiveReadHelper.GuardOwnedRoot(root); return new(root, clock); }

    private sealed record FileId(ulong Volume, ulong Low, ulong High)
    {
        internal static FileId Read(SafeFileHandle handle) { var id = OutputNative.Identity(handle); return new(id.Volume, id.IdLow, id.IdHigh); }
    }
    private sealed record DirectoryStamp(string Relative, FileId Identity);
    private sealed record OutputStamp(string Relative, FileId Identity, long Size, long Modified,
        string Digest, uint Attributes, bool ZonePresent, string? ZoneDigest);
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record ManifestRecord(int Version, Guid LeaseId, Guid OperationId, Guid SessionId,
        ulong Generation, long ExpiresUtcTicks, FileId RootIdentity, OutputStamp? Output,
        DirectoryStamp[] Directories, bool OutputRemoved, bool Retired);

    internal sealed class Lease : IDisposable
    {
        internal Guid Id => record.LeaseId;
        internal Guid Operation => record.OperationId;
        internal Guid Session => record.SessionId;
        internal ulong Generation => record.Generation;
        internal string Root { get; }
        private readonly List<SafeFileHandle> parents = [];
        private SafeFileHandle? root;
        private FileStream? manifest;
        private ManifestRecord record = null!;
        private byte[] manifestBytes = [];
        private bool retirementRecord;
        private bool disposed;
        private SafeFileHandle Base => parents[^1];
        private SafeFileHandle RootHandle => root ?? throw new ArchiveOutputException("source_unavailable");
        private string RetiredName => $"{Id:D}.retire.json";

        internal Lease(string baseRoot, Guid id, Guid operation, Guid session, ulong generation, DateTimeOffset expires)
        {
            Root = Path.Combine(baseRoot, id.ToString("D"));
            try {
                using var prepared = ArchiveOutputSession.Open(baseRoot, operation);
                OpenBase(baseRoot);
                root = OutputNative.Open(Base, id.ToString("D"), true, create: true, exclusive: true);
                using var files = OutputNative.Open(root, "files", true, create: true, exclusive: true);
                record = new(2, id, operation, session, generation, expires.UtcTicks, FileId.Read(root), null,
                    [new("files", FileId.Read(files))], false, false);
                manifest = new FileStream(OutputNative.Open(root, "manifest.json", false, create: true, exclusive: true), FileAccess.ReadWrite);
                Persist();
            } catch { Dispose(); throw; }
        }
        private Lease(string baseRoot, Guid id, bool retired)
        {
            Root = Path.Combine(baseRoot, id.ToString("D")); retirementRecord = retired;
            try {
                OpenBase(baseRoot);
                if (!retired) root = OutputNative.Open(Base, id.ToString("D"), true);
                manifest = new FileStream(OutputNative.Open(retired ? Base : RootHandle,
                    retired ? $"{id:D}.retire.json" : "manifest.json", false), FileAccess.ReadWrite);
                if (manifest.Length is < 1 or > 65536) throw new InvalidDataException();
                manifestBytes = new byte[(int)manifest.Length]; manifest.ReadExactly(manifestBytes);
                record = JsonSerializer.Deserialize<ManifestRecord>(manifestBytes) ?? throw new InvalidDataException();
                Validate(id);
                if (retired) {
                    if (!record.OutputRemoved || !record.Retired) throw new InvalidDataException();
                    try { root = OutputNative.Open(Base, id.ToString("D"), true); }
                    catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) { }
                }
                if (root is not null && FileId.Read(root) != record.RootIdentity) throw new ArchiveOutputException("source_changed");
            } catch { Dispose(); throw; }
        }
        private void OpenBase(string baseRoot)
        {
            var full = Path.GetFullPath(baseRoot);
            parents.Add(OutputNative.Open(null, "\\??\\" + char.ToUpperInvariant(full[0]) + ":\\", true));
            foreach (var part in full[3..].Split('\\')) { ArchiveOutputSession.Validate(part); parents.Add(OutputNative.Open(parents[^1], part, true)); }
        }
        private void Validate(Guid id)
        {
            if (record.Version != 2 || record.LeaseId != id || record.OperationId == Guid.Empty || record.SessionId == Guid.Empty ||
                record.Generation == 0 || record.ExpiresUtcTicks <= DateTimeOffset.MinValue.UtcTicks || record.ExpiresUtcTicks > DateTimeOffset.MaxValue.UtcTicks ||
                record.RootIdentity is null || record.Directories is null || record.Directories.Length > 64 || record.OutputRemoved && !record.Retired) throw new InvalidDataException();
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var directory in record.Directories) {
                if (directory is null || directory.Identity is null) throw new InvalidDataException();
                ValidateRelative(directory.Relative); if (!names.Add(directory.Relative)) throw new InvalidDataException();
            }
            if (record.Output is { } output) {
                ValidateRelative(output.Relative);
                if (output.Identity is null || output.Size is < 0 or > 536870912 || output.Digest is null || output.Digest.Length != 64 ||
                    !output.Digest.All(Uri.IsHexDigit) || output.ZonePresent != (output.ZoneDigest is { Length: 64 }) ||
                    output.ZoneDigest is not null && !output.ZoneDigest.All(Uri.IsHexDigit)) throw new InvalidDataException();
            } else if (record.OutputRemoved && !record.Retired) throw new InvalidDataException();
        }
        private static string[] ValidateRelative(string name)
        {
            if (name is null || name.Length is < 1 or > 32767 || name.Contains('\\')) throw new InvalidDataException();
            var parts = name.Split('/'); if (parts.Length > 64) throw new InvalidDataException();
            foreach (var part in parts) ArchiveOutputSession.Validate(part);
            return parts;
        }
        private bool MatchesManifest()
        {
            if (manifest is null || manifest.Length != manifestBytes.Length) return false;
            manifest.Position = 0; var actual = new byte[manifestBytes.Length]; manifest.ReadExactly(actual);
            return actual.AsSpan().SequenceEqual(manifestBytes);
        }
        private void Persist()
        {
            manifestBytes = JsonSerializer.SerializeToUtf8Bytes(record);
            if (manifestBytes.Length > 65536) throw new InvalidDataException();
            var stream = manifest ?? throw new InvalidDataException();
            stream.Position = 0; stream.SetLength(0); stream.Write(manifestBytes); stream.Flush(true);
        }
        private SafeFileHandle OpenOutput(string name, bool readOnly)
        {
            var parts = ValidateRelative(name); var held = new List<SafeFileHandle>();
            try {
                var parent = RootHandle;
                if (FileId.Read(parent) != record.RootIdentity) throw new ArchiveOutputException("source_changed");
                for (var index = 0; index < parts.Length - 1; index++) {
                    var next = OutputNative.Open(parent, parts[index], true); held.Add(next); parent = next;
                    var relative = string.Join('/', parts[..(index + 1)]);
                    var expected = record.Directories.FirstOrDefault(item => item.Relative == relative);
                    if ((record.Output is not null || expected is not null) && (expected is null || FileId.Read(next) != expected.Identity)) throw new ArchiveOutputException("source_changed");
                }
                return OutputNative.Open(parent, parts[^1], false, readOnly: readOnly, metadataCleanup: !readOnly);
            } finally { foreach (var handle in held.AsEnumerable().Reverse()) handle.Dispose(); }
        }
        internal void Seal(string output)
        {
            var prefix = Root + "\\";
            if (record.Retired || !output.StartsWith(prefix, StringComparison.OrdinalIgnoreCase)) throw new ArchiveOutputException("invalid_component");
            var name = output[prefix.Length..].Replace('\\', '/'); var parts = ValidateRelative(name);
            using var handle = OpenOutput(name, true);
            var baseline = OutputNative.Identity(handle);
            if (baseline.Size is < 0 or > 536870912) throw new ArchiveOutputException("resource_limit");
            using var stream = new FileStream(handle, FileAccess.Read);
            var attributes = ArchiveMetadataNative.Attributes(handle);
            var mark = ArchiveMotw.Read(handle); string? zoneDigest;
            try { zoneDigest = mark is null ? null : Convert.ToHexString(SHA256.HashData(mark)); }
            finally { if (mark is not null) CryptographicOperations.ZeroMemory(mark); }
            var stamp = new OutputStamp(name, new(baseline.Volume, baseline.IdLow, baseline.IdHigh), baseline.Size, baseline.Modified,
                Convert.ToHexString(SHA256.HashData(stream)), attributes, zoneDigest is not null, zoneDigest);
            var directories = record.Directories.ToDictionary(item => item.Relative, StringComparer.Ordinal);
            var held = new List<SafeFileHandle>();
            try {
                var parent = RootHandle;
                for (var index = 0; index < parts.Length - 1; index++) {
                    var next = OutputNative.Open(parent, parts[index], true); held.Add(next); parent = next;
                    var relative = string.Join('/', parts[..(index + 1)]); var identity = FileId.Read(next);
                    if (directories.TryGetValue(relative, out var original) && original.Identity != identity) throw new ArchiveOutputException("source_changed");
                    directories[relative] = new(relative, identity);
                }
            } finally { foreach (var directory in held.AsEnumerable().Reverse()) directory.Dispose(); }
            record = record with { Output = stamp, Directories = directories.Values.ToArray() }; Persist();
        }
        internal bool Eligible(DateTimeOffset now) => record.Retired || record.ExpiresUtcTicks <= now.UtcTicks;
        internal bool Release()
        {
            try {
                if (!MatchesManifest()) return false;
                if (!record.Retired) { record = record with { Retired = true }; Persist(); }
                if (!record.OutputRemoved && record.Output is { } output) {
                    SafeFileHandle handle;
                    try { handle = OpenOutput(output.Relative, false); }
                    catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) {
                        // The exact owned manifest records the only output; no replacement is followed.
                        record = record with { OutputRemoved = true }; Persist(); return RetireEmpty();
                    }
                    using (handle) {
                        var baseline = OutputNative.Identity(handle);
                        if (new FileId(baseline.Volume, baseline.IdLow, baseline.IdHigh) != output.Identity || baseline.Size != output.Size ||
                            baseline.Modified != output.Modified || ArchiveMetadataNative.Attributes(handle) != output.Attributes) return false;
                        var mark = ArchiveMotw.Read(handle, shareDelete: true);
                        try { if ((mark is not null) != output.ZonePresent || mark is not null && !CryptographicOperations.FixedTimeEquals(SHA256.HashData(mark), Convert.FromHexString(output.ZoneDigest!))) return false; }
                        finally { if (mark is not null) CryptographicOperations.ZeroMemory(mark); }
                        using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
                        var bytes = new byte[65536]; long offset = 0;
                        while (offset < output.Size) { var read = RandomAccess.Read(handle, bytes, offset); if (read == 0) return false; hash.AppendData(bytes, 0, read); offset += read; }
                        if (!CryptographicOperations.FixedTimeEquals(hash.GetHashAndReset(), Convert.FromHexString(output.Digest)) || OutputNative.Identity(handle) != baseline) return false;
                        ArchiveMetadataNative.ClearOwnedReadOnly(handle); OutputNative.DeleteHandle(handle);
                    }
                }
                if (!record.OutputRemoved) { record = record with { OutputRemoved = true }; Persist(); }
                return RetireEmpty();
            } catch (Exception failure) when (Recoverable(failure)) { return false; }
        }
        internal bool Abort() => Release();
        private bool RetireEmpty()
        {
            if (!MatchesManifest() || !record.OutputRemoved || !record.Retired) return false;
            if (root is null && retirementRecord) {
                try { root = OutputNative.Open(Base, Id.ToString("D"), true); }
                catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) { }
                if (root is not null && FileId.Read(root) != record.RootIdentity) throw new ArchiveOutputException("source_changed");
            }
            if (root is not null) {
                if (FileId.Read(root) != record.RootIdentity) throw new ArchiveOutputException("source_changed");
                // Preserve recovery authority before any directory deletion can fail.
                if (!retirementRecord) {
                    OutputNative.RenameNoReplace(manifest!.SafeFileHandle, Base, RetiredName);
                    retirementRecord = true;
                }
                foreach (var directory in record.Directories.OrderByDescending(item => item.Relative.Count(c => c == '/'))) RetireDirectory(directory);
                root.Dispose(); root = null;
                try {
                    using var current = OutputNative.Open(Base, Id.ToString("D"), true, retireDirectory: true);
                    if (FileId.Read(current) != record.RootIdentity) throw new ArchiveOutputException("source_changed");
                    OutputNative.DeleteHandle(current);
                } catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) { }
            }
            // A record survives every failed root retirement; no GUID-name ownership inference.
            if (!retirementRecord) return false;
            OutputNative.DeleteHandle(manifest!.SafeFileHandle); manifest.Dispose(); manifest = null;
            return true;
        }
        private void RetireDirectory(DirectoryStamp target)
        {
            var parts = ValidateRelative(target.Relative); var held = new List<SafeFileHandle>();
            try {
                var parent = RootHandle;
                for (var index = 0; index < parts.Length - 1; index++) {
                    var next = OutputNative.Open(parent, parts[index], true); held.Add(next); parent = next;
                    var relative = string.Join('/', parts[..(index + 1)]);
                    var expected = record.Directories.FirstOrDefault(item => item.Relative == relative);
                    if (expected is null || FileId.Read(next) != expected.Identity) throw new ArchiveOutputException("source_changed");
                }
                using var current = OutputNative.Open(parent, parts[^1], true, retireDirectory: true);
                if (FileId.Read(current) != target.Identity) throw new ArchiveOutputException("source_changed");
                OutputNative.DeleteHandle(current); // Windows refuses unknown nonempty directories.
            } catch (ArchiveOutputException missing) when (missing.NativeCode is 2 or 3) { }
            finally { foreach (var directory in held.AsEnumerable().Reverse()) directory.Dispose(); }
        }
        internal static Lease Reopen(string baseRoot, Guid id, bool retired) => new(baseRoot, id, retired);
        public void Dispose()
        {
            if (disposed) return; disposed = true;
            manifest?.Dispose(); root?.Dispose();
            foreach (var parent in parents.AsEnumerable().Reverse()) parent.Dispose(); parents.Clear();
        }
    }
    private static bool Recoverable(Exception failure) => failure is IOException or InvalidDataException or UnauthorizedAccessException or ArchiveOutputException or JsonException or InvalidOperationException or KeyNotFoundException or FormatException or OverflowException or ArgumentException;

    // One startup/admission batch, max8 filesystem entries; no timer or replacement worker.
    internal void Sweep()
    {
        lock (this) {
            if (disposed) return;
            var timer = Stopwatch.StartNew(); var inspected = 0; var removed = 0; var eligible = 0;
            try {
                var localBatchSize = Math.Min(8, localRetirements.Count);
                for (var index = 0; index < localBatchSize; index++) {
                    var id = localRetirements.Dequeue();
                    if (!leases.TryGetValue(id, out var inactive)) continue;
                    if (inactive.Release()) { leases.Remove(id); inactive.Dispose(); removed++; }
                    else localRetirements.Enqueue(id);
                }
                using var protectedRoot = ArchiveOutputSession.Open(cacheRoot, Guid.NewGuid());
                sweep ??= Directory.EnumerateFileSystemEntries(cacheRoot).GetEnumerator();
                while (inspected < 8) {
                    if (!sweep.MoveNext()) { sweep.Dispose(); sweep = null; break; }
                    inspected++; var name = Path.GetFileName(sweep.Current);
                    var retired = name.EndsWith(".retire.json", StringComparison.Ordinal);
                    var rawId = retired ? name[..^12] : name;
                    if (!Guid.TryParseExact(rawId, "D", out var id) || leases.ContainsKey(id)) continue;
                    try {
                        using var inactive = Lease.Reopen(cacheRoot, id, retired);
                        if (!inactive.Eligible(clock.GetUtcNow())) continue;
                        eligible++; if (inactive.Release()) removed++;
                    } catch (Exception failure) when (Recoverable(failure)) { }
                }
            } catch (Exception failure) when (Recoverable(failure)) { sweep?.Dispose(); sweep = null; }
            try { log?.Write("info", "archive.cache.sweep", Guid.NewGuid(), new Dictionary<string, object?> {
                ["inspected"] = inspected, ["eligible"] = eligible, ["removed"] = removed, ["elapsed_ms"] = timer.ElapsedMilliseconds }); } catch { }
        }
    }
    internal Lease Create(Guid operation, Guid session, ulong generation, Guid? requestedId = null)
    {
        lock (this) {
            if (disposed) throw new ObjectDisposedException(nameof(ArchiveEntryCache));
            Sweep();
            if (leases.Count >= 32) throw new ArchiveOutputException("resource_limit");
            var id = requestedId ?? Guid.NewGuid();
            if (id == Guid.Empty || leases.ContainsKey(id) || released.Contains(id)) throw new ArchiveOutputException("invalid_request");
            var lease = new Lease(cacheRoot, id, operation, session, generation, clock.GetUtcNow() + Retention); leases.Add(id, lease); return lease;
        }
    }
    internal bool Owns(Guid id, Guid operation, Guid session, ulong generation)
    { lock (this) return !released.Contains(id) && leases.TryGetValue(id, out var lease) && lease.Operation == operation && lease.Session == session && lease.Generation == generation; }
    internal bool Release(Guid id)
    {
        lock (this) {
            if (!leases.TryGetValue(id, out var lease)) return false;
            RememberReleased(id);
            if (!lease.Release()) return false;
            leases.Remove(id); lease.Dispose(); return true;
        }
    }
    private void RememberReleased(Guid id)
    {
        if (!released.Add(id)) return;
        releasedOrder.Enqueue(id);
        if (leases.ContainsKey(id)) localRetirements.Enqueue(id);
        if (releasedOrder.Count > 4096) { var oldest = releasedOrder.Dequeue(); if (!leases.ContainsKey(oldest)) released.Remove(oldest); }
    }
    internal bool Abort(Guid id)
    {
        lock (this) {
            if (!leases.Remove(id, out var lease)) return false;
            RememberReleased(id); var removed = lease.Abort(); lease.Dispose(); return removed;
        }
    }
    public void Dispose()
    {
        lock (this) {
            if (disposed) return; disposed = true; sweep?.Dispose(); sweep = null;
            foreach (var lease in leases.Values) { if (released.Contains(lease.Id)) lease.Release(); lease.Dispose(); }
            leases.Clear();
        }
    }
}
