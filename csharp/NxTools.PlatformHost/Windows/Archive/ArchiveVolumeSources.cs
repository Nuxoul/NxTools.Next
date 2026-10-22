using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;

namespace NxTools.PlatformHost.Windows.Archive;

// The only owner of every source lease in a read operation. No path is handed to a decoder
// until the entire deterministic family has been acquired and checked under these leases.
internal sealed class ArchiveVolumeSources : IDisposable
{
    private const int MaxVolumes = 1024;
    private readonly List<ArchiveSourceLease> leases = [];
    private readonly List<string> paths = [];
    private readonly Guid verificationId = Guid.NewGuid();
    private string? admissionIdentity;
    internal string PrimaryPath => paths[0];
    internal IReadOnlyList<string> Paths => paths.AsReadOnly();
    internal ArchiveSourceLease Primary => leases[0];

    private ArchiveVolumeSources(string first, ulong maxMetadata, ulong maxEncoded)
    {
        try
        {
            if (maxMetadata < 32 || maxEncoded == 0) throw new ArchiveOutputException("resource_limit");
            // The single-source fallback is deliberately format-neutral. Never classify a RAR
            // or ZIP as split from its suffix without inspecting its actual structural flags.
            var suffix = Path.GetExtension(first);
            var numbered7z = TrySevenOrdinal(first, out var sevenStem, out var sevenOrdinal);
            var zipPart = TryZipOrdinal(first, out var zipStem, out var zipOrdinal);
            var rarPart = TryRarPart(first, out var rarStem, out var rarOrdinal);
            if (numbered7z && sevenOrdinal != 1 || zipPart && zipOrdinal != 1 || rarPart && rarOrdinal != 1)
                throw new ArchiveOutputException("missing_volume");
            if (!zipPart && (suffix.Equals(".r00", StringComparison.OrdinalIgnoreCase) ||
                TryOldRarOrdinal(first, out _, out _) || TryOldRarLater(first, out _)))
                throw new ArchiveOutputException("missing_volume");
            if (zipPart) first = ResolveSibling(zipStem + ".zip");
            var rarWidth=rarPart?Path.GetFileName(first)[(Path.GetFileName(first).LastIndexOf(".part",StringComparison.OrdinalIgnoreCase)+5)..^4].Length:1;
            if (rarPart) first = ResolveSibling(rarStem + ".part" + 1.ToString("D"+rarWidth) + ".rar");
            if (numbered7z) first = ResolveSibling(first);
            Add(first, maxEncoded);
            if (numbered7z)
            {
                using var stream = File.OpenRead(PrimaryPath);
                Span<byte> signature = stackalloc byte[32];
                if (stream.Read(signature) != 32 || !signature[..6].SequenceEqual(new byte[]{0x37,0x7a,0xbc,0xaf,0x27,0x1c}))
                    throw new ArchiveOutputException("invalid_source");
                var candidates = EnumerateFamily(first, name => TrySevenOrdinal(name, out var stem, out _) &&
                    string.Equals(stem, sevenStem, StringComparison.OrdinalIgnoreCase));
                if (candidates.Count == 0) throw new ArchiveOutputException("missing_volume");
                var max = candidates.Select(name => { TrySevenOrdinal(name, out _, out var index); return index; }).Max();
                if (max < 1 || max > MaxVolumes) throw new ArchiveOutputException("missing_volume");
                for (var index = 2; index <= max; index++) Add(ResolveSibling(sevenStem + "." + index.ToString("D3")), maxEncoded);
                if (paths.Count != candidates.Count) throw new ArchiveOutputException("invalid_source");
                var expectedLength=checked(32UL+BinaryPrimitives.ReadUInt64LittleEndian(signature[12..20])+BinaryPrimitives.ReadUInt64LittleEndian(signature[20..28]));
                var actualLength=paths.Aggregate(0UL,(total,path)=>checked(total+(ulong)new FileInfo(path).Length));
                if(actualLength<expectedLength)throw new ArchiveOutputException("missing_volume");
                if(actualLength!=expectedLength)throw new ArchiveOutputException("invalid_source");
            }
            else if (suffix.Equals(".zip", StringComparison.OrdinalIgnoreCase) || zipPart)
            {
                var disks = ZipDiskCount(PrimaryPath, maxMetadata);
                if (disks > 1)
                {
                    var final = PrimaryPath;
                    // Each successful acquisition immediately transfers ownership to this set.
                    for (var index = 1; index < disks; index++)
                    {
                        var member = ResolveSibling(Path.Combine(Path.GetDirectoryName(final)!,
                            Path.GetFileNameWithoutExtension(final) + ".z" + index.ToString("D2")));
                        Add(member, maxEncoded);
                    }
                    var lastLease = leases[0];
                    var lastPath = paths[0];
                    leases.RemoveAt(0); paths.RemoveAt(0);
                    leases.Add(lastLease); paths.Add(lastPath);
                    CheckEncoded(maxEncoded);
                    var candidates = EnumerateFamily(final, name => TryZipOrdinal(name, out var stem, out _) &&
                        string.Equals(stem, Path.Combine(Path.GetDirectoryName(final)!, Path.GetFileNameWithoutExtension(final)), StringComparison.OrdinalIgnoreCase));
                    if (candidates.Count != disks - 1) throw new ArchiveOutputException("missing_volume");
                }
                else if (zipPart) throw new ArchiveOutputException("missing_volume");
            }
            else if (suffix.Equals(".rar", StringComparison.OrdinalIgnoreCase))
            {
                var next = RarNext(PrimaryPath, maxMetadata, out var isVolume, out var modern);
                if (rarPart && !isVolume) throw new ArchiveOutputException("invalid_source");
                if (isVolume)
                {
                    var stem = rarPart ? rarStem : Path.Combine(Path.GetDirectoryName(first)!, Path.GetFileNameWithoutExtension(first));
                    if (modern && !rarPart) throw new ArchiveOutputException("missing_volume");
                    if (!modern && rarPart) throw new ArchiveOutputException("invalid_source");
                    for (var index = 1; next; index++)
                    {
                        if (index >= MaxVolumes || !modern && index > 900) throw new ArchiveOutputException("resource_limit");
                        var member = modern ? stem + ".part" + (index + 1).ToString("D"+rarWidth) + ".rar" :
                            stem + "." + (char)('r' + (index - 1) / 100) + ((index - 1) % 100).ToString("D2");
                        Add(ResolveSibling(member), maxEncoded);
                        next = RarNext(member, maxMetadata, out var volume, out var numbering);
                        if (!volume || numbering != modern) throw new ArchiveOutputException("invalid_source");
                    }
                    var candidates = EnumerateFamily(first, name => modern
                        ? TryRarPart(name, out var partStem, out _) && string.Equals(partStem, stem, StringComparison.OrdinalIgnoreCase)
                        : TryOldRarLater(name, out var oldStem) && string.Equals(oldStem, stem, StringComparison.OrdinalIgnoreCase));
                    if (candidates.Count != paths.Count - (modern ? 0 : 1)) throw new ArchiveOutputException("invalid_source");
                }
            }
            Verify();
            admissionIdentity = IdentityNow();
        }
        catch { Dispose(); throw; }
    }
    internal static ArchiveVolumeSources Open(string path, ulong maxMetadata, ulong maxEncoded) => new(path, maxMetadata, maxEncoded);
    internal string Identity() { Verify(); return admissionIdentity!; }
    private string IdentityNow()
    {
        if (leases.Count == 1) return Primary.ReadmissionIdentity();
        using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        foreach (var lease in leases)
        {
            var bytes = Encoding.UTF8.GetBytes(lease.ReadmissionIdentity());
            hash.AppendData(BitConverter.GetBytes(bytes.Length)); hash.AppendData(bytes);
        }
        return "v1:" + Convert.ToHexString(hash.GetHashAndReset());
    }
    internal void Verify()
    {
        if (leases.Count == 0) throw new ArchiveOutputException("source_unavailable");
        foreach (var lease in leases) lease.RecycleSource(verificationId).Verify();
        if (admissionIdentity is not null && !string.Equals(IdentityNow(), admissionIdentity, StringComparison.Ordinal))
            throw new ArchiveOutputException("source_changed");
    }
    internal ReadOnlyMemory<byte>? CaptureZoneIdentifier(Func<bool, uint, uint>? flagsProbe = null) => Primary.CaptureZoneIdentifier(flagsProbe);
    public void Dispose() { foreach (var lease in leases) lease.Dispose(); leases.Clear(); paths.Clear(); }

    private void Add(string path, ulong maxEncoded)
    {
        if (paths.Count >= MaxVolumes || paths.Any(value => string.Equals(value, path, StringComparison.OrdinalIgnoreCase)))
            throw new ArchiveOutputException("resource_limit");
        var lease = ArchiveSourceLease.Open(path);
        leases.Add(lease); paths.Add(path);
        CheckEncoded(maxEncoded);
    }
    private void CheckEncoded(ulong limit)
    {
        ulong total = 0;
        foreach (var path in paths) total = checked(total + (ulong)new FileInfo(path).Length);
        if (total > limit) throw new ArchiveOutputException("resource_limit");
    }
    private static string ResolveSibling(string path)
    {
        var parent = Path.GetDirectoryName(path)!;
        var expected = Path.GetFileName(path);
        var inspected = 0;
        string? found = null;
        foreach (var candidate in Directory.EnumerateFileSystemEntries(parent))
        {
            if (++inspected > 16384) throw new ArchiveOutputException("resource_limit");
            if (!string.Equals(Path.GetFileName(candidate), expected, StringComparison.OrdinalIgnoreCase)) continue;
            if (found is not null) throw new ArchiveOutputException("invalid_source");
            found = candidate;
        }
        return found ?? throw new ArchiveOutputException("missing_volume");
    }
    private static List<string> EnumerateFamily(string path, Func<string, bool> matches)
    {
        var found = new List<string>();
        var inspected = 0;
        foreach (var name in Directory.EnumerateFiles(Path.GetDirectoryName(path)!))
        {
            if (++inspected > 16384) throw new ArchiveOutputException("resource_limit");
            if (!matches(name)) continue;
            if (found.Count >= MaxVolumes) throw new ArchiveOutputException("resource_limit");
            found.Add(name);
        }
        return found;
    }
    private static bool Numbered(string path, string prefix, int minDigits, out string stem, out int ordinal)
    {
        stem = path; ordinal = 0;
        var name = Path.GetFileName(path); var dot = name.LastIndexOf('.');
        if (dot < 1) return false;
        var ext = name[(dot + 1)..];
        if (!ext.StartsWith(prefix, StringComparison.OrdinalIgnoreCase) || ext.Length < prefix.Length + minDigits ||
            !ext[prefix.Length..].All(char.IsAsciiDigit) || !int.TryParse(ext[prefix.Length..], out ordinal)) return false;
        stem = path[..^(ext.Length + 1)]; return true;
    }
    private static bool TrySevenOrdinal(string path, out string stem, out int ordinal)
    {
        stem = path; ordinal = 0;
        var marker = path.LastIndexOf(".7z.", StringComparison.OrdinalIgnoreCase);
        if (marker < 0 || path.Length - marker < 7 || !int.TryParse(path[(marker + 4)..], out ordinal) ||
            !path[(marker + 4)..].All(char.IsAsciiDigit)) return false;
        stem = path[..(marker + 3)]; return true;
    }
    private static bool TryZipOrdinal(string path, out string stem, out int ordinal) => Numbered(path, "z", 2, out stem, out ordinal);
    private static bool TryOldRarOrdinal(string path, out string stem, out int ordinal) => Numbered(path, "r", 2, out stem, out ordinal);
    private static bool TryOldRarLater(string path, out string stem)
    {
        stem = path;
        var ext = Path.GetExtension(path);
        if (ext.Length != 4 || ext[0] != '.' || char.ToLowerInvariant(ext[1]) is < 'r' or > 'z' ||
            !char.IsAsciiDigit(ext[2]) || !char.IsAsciiDigit(ext[3])) return false;
        stem = path[..^4]; return true;
    }
    private static bool TryRarPart(string path, out string stem, out int ordinal)
    {
        stem = path; ordinal = 0;
        if (!path.EndsWith(".rar", StringComparison.OrdinalIgnoreCase)) return false;
        var body = path[..^4]; var marker = body.LastIndexOf(".part", StringComparison.OrdinalIgnoreCase);
        if (marker < 0 || !int.TryParse(body[(marker + 5)..], out ordinal) || ordinal < 1 ||
            !body[(marker + 5)..].All(char.IsAsciiDigit)) return false;
        stem = body[..marker]; return true;
    }
    private static int ZipDiskCount(string path, ulong metadata)
    {
        using var file = File.OpenRead(path);
        if (file.Length < 22) return 1;
        var length = (int)Math.Min(file.Length, Math.Min(65557, (long)Math.Min(metadata, long.MaxValue)));
        if (length < 22) throw new ArchiveOutputException("resource_limit");
        var tail = new byte[length]; file.Seek(-length, SeekOrigin.End); file.ReadExactly(tail);
        for (var at = length - 22; at >= 0; at--)
        {
            if (!tail.AsSpan(at, 4).SequenceEqual("PK\x05\x06"u8) || at + 22 + BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 20)) != length) continue;
            var disk = BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 4));
            var central = BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 6));
            if (disk == ushort.MaxValue || central == ushort.MaxValue) throw new ArchiveOutputException("resource_limit");
            if (central > disk || disk >= MaxVolumes) throw new ArchiveOutputException("invalid_source");
            return disk + 1;
        }
        return 1; // Arbitrary single-file archive / invalid input remains the codec's concern.
    }
    // RAR4/5 end-of-archive NEXT_VOLUME bit, not file extension or a guessed byte split.
    // Each block's declared data area is skipped by seek; header allocation is metadata-capped.
    private static bool RarNext(string path, ulong metadata, out bool volume, out bool modern)
    {
        using var file = File.OpenRead(path);
        Span<byte> marker = stackalloc byte[8];
        if (file.Read(marker) < 7) { volume = false; modern = false; return false; }
        var rar5 = marker.SequenceEqual("Rar!\x1a\x07\x01\0"u8);
        if (!rar5 && !marker[..7].SequenceEqual("Rar!\x1a\x07\x00"u8)) { volume = false; modern = false; return false; }
        modern = false; volume = false;
        file.Position = rar5 ? 8 : 7;
        ulong budget = 0;
        while (file.Position < file.Length)
        {
            if (rar5)
            {
                var start = file.Position;
                var crc = ReadExact(file, 4); // CRC32 is checked by UnRAR on actual decode.
                _ = crc;
                var size = Vint(file); if (size > metadata || size < 2) throw new ArchiveOutputException("invalid_source");
                budget = checked(budget + size + 16);
                if (budget > metadata) throw new ArchiveOutputException("resource_limit");
                var header = ReadExact(file, checked((int)size));
                var offset = 0; var type = Vint(header, ref offset); var flags = Vint(header, ref offset);
                var extra = (flags & 1) != 0 ? Vint(header, ref offset) : 0;
                var data = (flags & 2) != 0 ? Vint(header, ref offset) : 0;
                if (extra > (ulong)(header.Length - offset)) throw new ArchiveOutputException("invalid_source");
                if (type == 4 && !volume) return false; // Encrypted ordinary headers: codec/password owner decides,not speculative discovery.
                if (type == 1)
                {
                    var main = Vint(header, ref offset);
                    volume = (main & 1) != 0;
                    modern = volume;
                }
                if (type == 5) return volume && (Vint(header, ref offset) & 1) != 0;
                if (data > (ulong)(file.Length - file.Position)) throw new ArchiveOutputException("invalid_source");
                file.Position += (long)data;
                if (file.Position <= start) throw new ArchiveOutputException("invalid_source");
            }
            else
            {
                var fixedHeader = ReadExact(file, 7);
                var kind = fixedHeader[2]; var flags = BinaryPrimitives.ReadUInt16LittleEndian(fixedHeader.AsSpan(3));
                var size = BinaryPrimitives.ReadUInt16LittleEndian(fixedHeader.AsSpan(5));
                if (size < 7 || size > metadata) throw new ArchiveOutputException("invalid_source");
                budget = checked(budget + size);
                if (budget > metadata) throw new ArchiveOutputException("resource_limit");
                var header = ReadExact(file, size - 7);
                if (kind == 0x73) { volume = (flags & 1) != 0; modern = (flags & 0x10) != 0; }
                if (kind == 0x7b) return volume && (flags & 1) != 0;
                if ((flags & 0x8000) != 0)
                {
                    if (header.Length < 4) throw new ArchiveOutputException("invalid_source");
                    var data = BinaryPrimitives.ReadUInt32LittleEndian(header);
                    if (data > file.Length - file.Position) throw new ArchiveOutputException("invalid_source");
                    file.Position += data;
                }
            }
        }
        // A declared volume without an end header cannot authorize a last member.
        if (volume) throw new ArchiveOutputException("missing_volume");
        return false;
    }
    private static byte[] ReadExact(Stream stream, int count) { var bytes = new byte[count]; stream.ReadExactly(bytes); return bytes; }
    private static ulong Vint(Stream stream)
    {
        ulong result = 0;
        for (var index = 0; index < 10; index++)
        {
            var next = stream.ReadByte(); if (next < 0) throw new ArchiveOutputException("missing_volume");
            result |= (ulong)(next & 127) << (index * 7);
            if ((next & 128) == 0) return result;
        }
        throw new ArchiveOutputException("invalid_source");
    }
    private static ulong Vint(byte[] data, ref int offset)
    {
        ulong result = 0;
        for (var index = 0; index < 10 && offset < data.Length; index++)
        {
            var next = data[offset++]; result |= (ulong)(next & 127) << (index * 7);
            if ((next & 128) == 0) return result;
        }
        throw new ArchiveOutputException("invalid_source");
    }
}
