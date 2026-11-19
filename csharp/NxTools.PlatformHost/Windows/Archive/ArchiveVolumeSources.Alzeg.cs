using System.Buffers.Binary;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveVolumeSources
{
    private bool AcquireAlzeg(ulong metadata, ulong encoded)
    {
        using var source = File.OpenRead(PrimaryPath);
        if (source.Length < 4) return false;
        Span<byte> signature = stackalloc byte[4];
        source.ReadExactly(signature);
        if (signature.SequenceEqual("ALZ\x01"u8)) AcquireAlz(metadata, encoded);
        else if (signature.SequenceEqual("EGGA"u8)) AcquireEgg(metadata, encoded);
        else return false;
        return true;
    }

    private void AcquireAlz(ulong metadata, ulong encoded)
    {
        ulong retained = 0;
        Span<byte> head = stackalloc byte[8];
        Span<byte> tail = stackalloc byte[16];
        for (var ordinal = 0; ; ordinal++)
        {
            ReserveAlzeg(ref retained, 128, metadata);
            if (ordinal >= 1000) throw new ArchiveOutputException("resource_limit");
            if (ordinal != 0)
            {
                var extension = "." + (char)('a' + (ordinal - 1) / 100) + ((ordinal - 1) % 100).ToString("D2");
                Add(ResolveSibling(Path.ChangeExtension(paths[0], extension)), encoded);
            }
            using var source = File.OpenRead(paths[ordinal]);
            if (source.Length < 24) throw new ArchiveOutputException("corrupt_archive");
            ReadAlzeg(source, head);
            if (!head[..4].SequenceEqual("ALZ\x01"u8)) throw new ArchiveOutputException("corrupt_archive");
            if (BinaryPrimitives.ReadUInt16LittleEndian(head[4..]) != 10) throw new ArchiveOutputException("unsupported_format");
            if (BinaryPrimitives.ReadUInt16LittleEndian(head[6..]) != ordinal)
                throw new ArchiveOutputException(ordinal == 0 ? "missing_volume" : "corrupt_archive");
            source.Position = source.Length - 16;
            ReadAlzeg(source, tail);
            if (!tail[..4].SequenceEqual("CLZ\x01"u8) || tail[8..12].IndexOfAnyExcept((byte)0) >= 0)
                throw new ArchiveOutputException("corrupt_archive");
            if (tail[12..].SequenceEqual("CLZ\x02"u8)) return;
            if (!tail[12..].SequenceEqual("CLZ\x03"u8) || tail[4..12].IndexOfAnyExcept((byte)0) >= 0)
                throw new ArchiveOutputException("corrupt_archive");
        }
    }

    private readonly record struct EggPrefix(uint Id, uint Previous, uint Next, bool Split);

    private void AcquireEgg(ulong metadata, ulong encoded)
    {
        ulong retained = 0;
        ReserveAlzeg(ref retained, 128, metadata);
        var initial = ReadEggPrefix(paths[0], ref retained, metadata);
        if (!initial.Split || initial.Previous == 0 && initial.Next == 0) return;
        var index = EggIndex(paths[0], ref retained, metadata);
        if (index.TryGetValue(initial.Id, out var initialPath) && initialPath is null)
            throw new ArchiveOutputException("corrupt_archive");
        var seen = new HashSet<uint> { initial.Id };

        EggPrefix Acquire(uint id)
        {
            if (!seen.Add(id)) throw new ArchiveOutputException("corrupt_archive");
            if (!index.TryGetValue(id, out var candidate)) throw new ArchiveOutputException("missing_volume");
            if (candidate is null) throw new ArchiveOutputException("corrupt_archive");
            ReserveAlzeg(ref retained, 128, metadata);
            Add(candidate, encoded);
            // The index is only a lookup hint. The original lease now protects this reread.
            var prefix = ReadEggPrefix(candidate, ref retained, metadata);
            if (prefix.Id != id) throw new ArchiveOutputException("source_changed");
            if (!prefix.Split) throw new ArchiveOutputException("corrupt_archive");
            return prefix;
        }

        var current = initial;
        while (current.Previous != 0)
        {
            var previous = Acquire(current.Previous);
            if (previous.Next != current.Id) throw new ArchiveOutputException("corrupt_archive");
            var lease = leases[^1]; var path = paths[^1];
            leases.RemoveAt(leases.Count - 1); paths.RemoveAt(paths.Count - 1);
            leases.Insert(0, lease); paths.Insert(0, path);
            current = previous;
        }
        current = initial;
        while (current.Next != 0)
        {
            var next = Acquire(current.Next);
            if (next.Previous != current.Id) throw new ArchiveOutputException("corrupt_archive");
            current = next;
        }
    }

    private static Dictionary<uint, string?> EggIndex(string requested, ref ulong retained, ulong metadata)
    {
        var result = new Dictionary<uint, string?>();
        var inspected = 0;
        Span<byte> header = stackalloc byte[10];
        foreach (var candidate in Directory.EnumerateFiles(Path.GetDirectoryName(requested)!))
        {
            if (++inspected > 16384) throw new ArchiveOutputException("resource_limit");
            try
            {
                // No authority is derived from this unleased, fixed-size directory probe.
                using var probe = new FileStream(candidate, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete, 1);
                if (probe.Length < header.Length) continue;
                probe.ReadExactly(header);
                if (!header[..4].SequenceEqual("EGGA"u8)) continue;
                var id = BinaryPrimitives.ReadUInt32LittleEndian(header[6..]);
                if (id == 0) continue;
                ReserveAlzeg(ref retained, checked(128UL + (ulong)candidate.Length * 2), metadata);
                if (result.ContainsKey(id)) result[id] = null;
                else
                {
                    if (result.Count >= MaxVolumes) throw new ArchiveOutputException("resource_limit");
                    result.Add(id, candidate);
                }
            }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }
        return result;
    }

    private static EggPrefix ReadEggPrefix(string path, ref ulong retained, ulong metadata)
    {
        using var source = File.OpenRead(path);
        Span<byte> head = stackalloc byte[14];
        ReadAlzeg(source, head);
        if (!head[..4].SequenceEqual("EGGA"u8)) throw new ArchiveOutputException("corrupt_archive");
        if (BinaryPrimitives.ReadUInt16LittleEndian(head[4..]) != 256 || head[10..].IndexOfAnyExcept((byte)0) >= 0)
            throw new ArchiveOutputException("unsupported_format");
        var id = BinaryPrimitives.ReadUInt32LittleEndian(head[6..]);
        if (id == 0) throw new ArchiveOutputException("corrupt_archive");
        uint previous = 0, next = 0;
        var split = false; var solid = false;
        Span<byte> word = stackalloc byte[8];
        while (true)
        {
            ReadAlzeg(source, word[..4]);
            ReserveAlzeg(ref retained, 4, metadata);
            var tag = BinaryPrimitives.ReadUInt32LittleEndian(word);
            if (tag == 0x08e28222) return new(id, previous, next, split);
            if (split && previous != 0) throw new ArchiveOutputException("corrupt_archive");
            if (tag == 0x08d144a8) throw new ArchiveOutputException("unsupported_encryption");
            if (tag == 0xffff0000) throw new ArchiveOutputException("unsupported_method");
            if (tag is not (0x24f5a262 or 0x24e5a060)) throw new ArchiveOutputException("corrupt_archive");
            ReadAlzeg(source, word[..1]);
            var flags = word[0];
            var width = (flags & 1) == 0 ? 2 : 4;
            ReadAlzeg(source, word[..width]);
            var size = width == 2 ? BinaryPrimitives.ReadUInt16LittleEndian(word) : BinaryPrimitives.ReadUInt32LittleEndian(word);
            if ((flags & ~1) != 0 || size > (ulong)(source.Length - source.Position))
                throw new ArchiveOutputException("corrupt_archive");
            ReserveAlzeg(ref retained, 1UL + (ulong)width + size, metadata);
            if (tag == 0x24f5a262)
            {
                if (split || size != 8) throw new ArchiveOutputException("corrupt_archive");
                ReadAlzeg(source, word);
                previous = BinaryPrimitives.ReadUInt32LittleEndian(word);
                next = BinaryPrimitives.ReadUInt32LittleEndian(word[4..]);
                if (solid && previous != 0) throw new ArchiveOutputException("corrupt_archive");
                split = true;
            }
            else
            {
                if (solid || size != 0) throw new ArchiveOutputException("corrupt_archive");
                solid = true;
            }
        }
    }

    private static void ReserveAlzeg(ref ulong retained, ulong bytes, ulong limit)
    {
        if (bytes > limit - Math.Min(retained, limit)) throw new ArchiveOutputException("resource_limit");
        retained += bytes;
    }
    private static void ReadAlzeg(Stream stream, Span<byte> bytes)
    {
        try { stream.ReadExactly(bytes); }
        catch (EndOfStreamException) { throw new ArchiveOutputException("corrupt_archive"); }
    }
}
