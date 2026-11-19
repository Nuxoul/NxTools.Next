using System.Buffers.Binary;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveVolumeSources
{
    private readonly record struct SfxDescriptor(ulong Prefix, int Disks);
    private bool AcquireSfx(ulong metadata, ulong encoded, bool outputPreflight)
    {
        var requested = PrimaryPath;
        var finalEntry = Path.GetExtension(requested).Equals(".zip", StringComparison.OrdinalIgnoreCase);
        if (finalEntry && ZipDiskCount(requested, metadata) == 1) return false;
        SfxDescriptor? descriptor;
        if (finalEntry)
        {
            string executable;
            try { executable = ResolveSibling(Path.ChangeExtension(requested, ".exe")); }
            catch (ArchiveOutputException failure) when (failure.Stage == "missing_volume") { return false; }
            try { if (ReadSfxDescriptor(executable, metadata, hint: true) is null) return false; }
            catch (ArchiveOutputException failure) when (failure.Stage == "corrupt_archive") { return false; }
            if (outputPreflight) encoded = Math.Max(encoded, ArchiveWorkerBudget.Capture(new { }).max_output_bytes);
            Add(executable, encoded);
            descriptor = ReadSfxDescriptor(paths[1], metadata) ?? throw new ArchiveOutputException("source_changed");
        }
        else descriptor = ReadSfxDescriptor(PrimaryPath, metadata);
        if (descriptor is not { } declared) return false;
        if (outputPreflight) encoded = Math.Max(encoded, ArchiveWorkerBudget.Capture(new { }).max_output_bytes);
        if (finalEntry && declared.Disks == 1) throw new ArchiveOutputException("corrupt_archive");
        var primary = finalEntry ? paths[1] : PrimaryPath;
        for (var disk = 1; disk < declared.Disks; disk++)
        {
            var extension = disk + 1 == declared.Disks ? ".zip" : ".z" + (disk + 1).ToString("D2");
            if (finalEntry && disk + 1 == declared.Disks) continue;
            Add(ResolveSibling(Path.ChangeExtension(primary, extension)), encoded);
        }
        if (finalEntry)
        {
            var finalLease = leases[0]; var finalPath = paths[0];
            leases.RemoveAt(0); paths.RemoveAt(0); leases.Add(finalLease); paths.Add(finalPath);
        }
        if (paths.Count != declared.Disks) throw new ArchiveOutputException("missing_volume");
        CheckEncoded(encoded);
        if (declared.Disks > 1)
        {
            using var first = File.OpenRead(PrimaryPath);
            first.Position = checked((long)declared.Prefix);
            var marker = SfxRead(first, 4);
            if (!marker.AsSpan().SequenceEqual("PK\x07\x08"u8)) throw new ArchiveOutputException("corrupt_archive");
            var stem = Path.Combine(Path.GetDirectoryName(PrimaryPath)!, Path.GetFileNameWithoutExtension(PrimaryPath));
            var numbered = EnumerateFamily(PrimaryPath, candidate => TryZipOrdinal(candidate, out var candidateStem, out _) && string.Equals(candidateStem, stem, StringComparison.OrdinalIgnoreCase));
            if (numbered.Count != declared.Disks - 2) throw new ArchiveOutputException("corrupt_archive");
        }
        ValidateSfxEnd(paths[^1], metadata, declared.Disks);
        return true;
    }
    private static byte[] SfxRead(FileStream file, int count)
    {
        var bytes = new byte[count];
        try { file.ReadExactly(bytes); }
        catch (EndOfStreamException) { throw new ArchiveOutputException("corrupt_archive"); }
        return bytes;
    }
    private static SfxDescriptor? ReadSfxDescriptor(string path, ulong metadata, bool hint = false)
    {
        using var file = File.OpenRead(path);
        if (file.Length < 2 || !SfxRead(file, 2).AsSpan().SequenceEqual("MZ"u8)) return null;
        if (!hint && metadata < 64) throw new ArchiveOutputException("resource_limit");
        if (!hint && (ulong)file.Length > uint.MaxValue) throw new ArchiveOutputException("resource_limit");
        file.Position = 0;
        var dos = SfxRead(file, 64);
        var pe = BinaryPrimitives.ReadUInt32LittleEndian(dos.AsSpan(60));
        if (pe is < 64 or > 1048576) return null;
        file.Position = pe; var coff = SfxRead(file, 24);
        if (!coff.AsSpan(0, 4).SequenceEqual("PE\0\0"u8) || BinaryPrimitives.ReadUInt16LittleEndian(coff.AsSpan(4)) != 0x8664) return null;
        var sections = BinaryPrimitives.ReadUInt16LittleEndian(coff.AsSpan(6));
        var optionalBytes = BinaryPrimitives.ReadUInt16LittleEndian(coff.AsSpan(20));
        if (sections is < 1 or > 96 || optionalBytes < 240) return null;
        if (!hint && 64UL + 24 + 240 + (ulong)sections * 40 + 48 > metadata) throw new ArchiveOutputException("resource_limit");
        var optional = SfxRead(file, 240);
        if (BinaryPrimitives.ReadUInt16LittleEndian(optional) != 0x20b || BinaryPrimitives.ReadUInt32LittleEndian(optional.AsSpan(108)) > 16 || BinaryPrimitives.ReadUInt64LittleEndian(optional.AsSpan(144)) != 0) return null;
        var headers = (ulong)BinaryPrimitives.ReadUInt32LittleEndian(optional.AsSpan(60));
        var sectionStart = (ulong)pe + 24 + optionalBytes;
        if (sectionStart + (ulong)sections * 40 > headers || headers > (ulong)file.Length) return null;
        var end = headers;
        for (var index = 0; index < sections; index++)
        {
            file.Position = checked((long)(sectionStart + (ulong)index * 40)); var section = SfxRead(file, 40);
            var size = (ulong)BinaryPrimitives.ReadUInt32LittleEndian(section.AsSpan(16));
            var offset = (ulong)BinaryPrimitives.ReadUInt32LittleEndian(section.AsSpan(20));
            if (size != 0) { if (offset < headers) return null; end = Math.Max(end, offset + size); }
        }
        if (end + 48 > (ulong)file.Length) return null;
        file.Position = checked((long)end); var descriptor = SfxRead(file, 48);
        if (!descriptor.AsSpan(0, 16).SequenceEqual("NXTOOLS-ZIP-SFX1"u8)) return null;
        var disks = BinaryPrimitives.ReadUInt32LittleEndian(descriptor.AsSpan(24));
        if (hint) return new(end + 48, 0);
        if (BinaryPrimitives.ReadUInt32LittleEndian(descriptor.AsSpan(16)) != 1 || BinaryPrimitives.ReadUInt32LittleEndian(descriptor.AsSpan(20)) != 48 || BinaryPrimitives.ReadUInt32LittleEndian(descriptor.AsSpan(28)) != 0 || BinaryPrimitives.ReadUInt64LittleEndian(descriptor.AsSpan(32)) != end + 48 || BinaryPrimitives.ReadUInt64LittleEndian(descriptor.AsSpan(40)) != 0 || disks == 0) throw new ArchiveOutputException("corrupt_archive");
        if (disks > MaxVolumes) throw new ArchiveOutputException("resource_limit");
        return new(end + 48, (int)disks);
    }
    private static void ValidateSfxEnd(string path, ulong metadata, int disks)
    {
        using var file = File.OpenRead(path);
        var length = checked((int)Math.Min(file.Length, Math.Min(65557L, (long)Math.Min(metadata, long.MaxValue))));
        if (length < 22) throw new ArchiveOutputException("resource_limit");
        file.Seek(-length, SeekOrigin.End); var tail = SfxRead(file, length);
        for (var at = length - 22; at >= 0; at--)
        {
            if (!tail.AsSpan(at, 4).SequenceEqual("PK\x05\x06"u8) || at + 22 + BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 20)) != length) continue;
            var disk = BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 4));
            var central = BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 6));
            if (disk != ushort.MaxValue && disk != disks - 1 || central != ushort.MaxValue && central >= disks) throw new ArchiveOutputException("corrupt_archive");
            if (at >= 20 && tail.AsSpan(at - 20, 4).SequenceEqual("PK\x06\x07"u8))
            {
                if (BinaryPrimitives.ReadUInt32LittleEndian(tail.AsSpan(at - 4)) != disks) throw new ArchiveOutputException("corrupt_archive");
            }
            else if (disk == ushort.MaxValue || central == ushort.MaxValue || BinaryPrimitives.ReadUInt16LittleEndian(tail.AsSpan(at + 10)) == ushort.MaxValue || BinaryPrimitives.ReadUInt32LittleEndian(tail.AsSpan(at + 12)) == uint.MaxValue || BinaryPrimitives.ReadUInt32LittleEndian(tail.AsSpan(at + 16)) == uint.MaxValue) throw new ArchiveOutputException("corrupt_archive");
            return;
        }
        throw new ArchiveOutputException("corrupt_archive");
    }
}
