using System.Buffers.Binary;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

internal static class ArchiveReclamationNative
{
    internal const string MarkerName = ":NxTools.HZ5";
    [StructLayout(LayoutKind.Sequential)]
    private struct StandardInfo { internal long Allocation, Length; internal uint Links; internal byte DeletePending, Directory; }
    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool Standard(SafeFileHandle file, int kind, out StandardInfo value, uint size);
    [DllImport("kernel32.dll", EntryPoint = "GetVolumeInformationByHandleW", ExactSpelling = true, SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool Volume(SafeFileHandle file, IntPtr name, uint nameLength, out uint serial, out uint componentLength, out uint flags, StringBuilder fs, uint fsLength);
    [DllImport("kernel32.dll", EntryPoint = "DeviceIoControl", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool Control(SafeFileHandle file, uint code, byte[]? input, uint inputLength, byte[]? output, uint outputLength, out uint returned, IntPtr overlapped);
    internal static string Identity(SafeFileHandle file, bool length = false)
    {
        var value = OutputNative.Identity(file);
        return $"v1:{value.Volume:X}:{value.IdHigh:X16}{value.IdLow:X16}" + (length ? $":{value.Size:X}" : "");
    }
    internal static void RequireCapability(SafeFileHandle file, bool continuing)
    {
        var fs = new StringBuilder(32);
        if (!Volume(file, IntPtr.Zero, 0, out _, out _, out var flags, fs, 32)) Fail("capability");
        if (fs.ToString() != "NTFS" || (flags & (0x40000u | 0x40u)) != (0x40000u | 0x40u)) throw new ArchiveOutputException("reclamation_unsupported_filesystem");
        var attrs = ArchiveMetadataNative.Attributes(file);
        if ((attrs & (0x400u | 0x800u | 0x4000u | 0x10u | 0x1000u | 0x40000u | (continuing ? 0u : 0x200u))) != 0)
            throw new ArchiveOutputException("reclamation_source_attributes");
        var info = Info(file);
        if (info.Links != 1 || info.DeletePending != 0 || info.Directory != 0) throw new ArchiveOutputException("reclamation_source_links");
    }
    private static StandardInfo Info(SafeFileHandle file)
    {
        if (!Standard(file, 1, out var info, (uint)Marshal.SizeOf<StandardInfo>())) Fail("allocation");
        if (info.Allocation < 0 || info.Length < 0) throw new ArchiveOutputException("reclamation_allocation_invalid");
        return info;
    }
    internal static ulong Allocation(SafeFileHandle file) => (ulong)Info(file).Allocation;
    internal static ulong AllocatedIn(SafeFileHandle file, ulong start, ulong end)
    {
        if (end < start || end > long.MaxValue) throw new ArchiveOutputException("reclamation_range_invalid");
        if (end == start) return 0;
        var input = new byte[16]; BinaryPrimitives.WriteUInt64LittleEndian(input, start); BinaryPrimitives.WriteUInt64LittleEndian(input.AsSpan(8), end - start);
        var output = new byte[65536];
        if (!Control(file, 0x940cf, input, 16, output, (uint)output.Length, out var count, IntPtr.Zero)) Fail("query_ranges");
        if (count % 16 != 0 || count > output.Length) throw new ArchiveOutputException("reclamation_allocation_invalid");
        ulong sum = 0, previous = 0;
        for (var at = 0; at < count; at += 16)
        {
            var first = BinaryPrimitives.ReadUInt64LittleEndian(output.AsSpan(at)); var size = BinaryPrimitives.ReadUInt64LittleEndian(output.AsSpan(at + 8));
            if (size > long.MaxValue || first > long.MaxValue - size || first < previous) throw new ArchiveOutputException("reclamation_allocation_invalid");
            previous = first + size;
            var low = Math.Max(start, first); var high = Math.Min(end, previous);
            if (high > low) sum = checked(sum + high - low);
        }
        return sum;
    }
    internal static void Punch(SafeFileHandle file, ulong start, ulong end)
    {
        if (end == start) return;
        if (!Control(file, 0x900c4, null, 0, null, 0, out _, IntPtr.Zero)) Fail("set_sparse");
        var input = new byte[16]; BinaryPrimitives.WriteUInt64LittleEndian(input, start); BinaryPrimitives.WriteUInt64LittleEndian(input.AsSpan(8), end);
        if (!Control(file, 0x980c8, input, 16, null, 0, out _, IntPtr.Zero)) Fail("punch");
    }
    internal static SafeFileHandle OpenMarker(SafeFileHandle file, bool create)
    {
        var identity = Identity(file);
        var stream = OutputNative.Open(file, MarkerName, directory: false, exclusive: create, readOnly: !create, zoneStream: true);
        try { if (identity != Identity(stream)) throw new ArchiveOutputException("reclamation_marker_identity"); return stream; }
        catch { stream.Dispose(); throw; }
    }
    internal static void RefuseRecoverySource(SafeFileHandle file)
    {
        try { using var marker = OpenMarker(file, false); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3 or 123 or 1 or 50) { return; }
        throw new ArchiveOutputException("reclamation_recovery_required");
    }
    internal static byte[] ReadExact(SafeFileHandle file, long offset, int length)
    {
        var result = new byte[length]; var read = 0;
        while (read < length) { var count = RandomAccess.Read(file, result.AsSpan(read), offset + read); if (count == 0) throw new ArchiveOutputException("reclamation_record_truncated"); read += count; }
        return result;
    }
    private static readonly uint[] CrcTable = BuildCrcTable();
    private static uint[] BuildCrcTable() {
        var table = new uint[256];
        for (uint index = 0; index < table.Length; index++) { var value = index; for (var bit = 0; bit < 8; bit++) value = (value >> 1) ^ ((value & 1) != 0 ? 0xedb88320u : 0u); table[index] = value; }
        return table;
    }
    internal static uint Crc32(ReadOnlySpan<byte> bytes, uint crc = uint.MaxValue, bool finish = true)
    {
        foreach (var b in bytes) crc = (crc >> 8) ^ CrcTable[(crc ^ b) & 255];
        return finish ? ~crc : crc;
    }
    internal static uint FileCrc32(SafeFileHandle file, CancellationToken stop = default)
    {
        var buffer = new byte[65536]; long offset = 0; uint crc = uint.MaxValue;
        while (true) { stop.ThrowIfCancellationRequested(); var count = RandomAccess.Read(file, buffer, offset); if (count == 0) return ~crc; crc = Crc32(buffer.AsSpan(0, count), crc, false); offset += count; }
    }
    private static void Fail(string stage) => throw new ArchiveOutputException("reclamation_" + stage + "_failed", Marshal.GetLastWin32Error());
}

// Reservations extend native sharing through deferred content and the edit FILE handoff.
// There is no queue or retry here; the original operation owner retains admission/cleanup.
internal static class ArchiveSourceActivity
{
    private static readonly object Gate = new();
    private static readonly Dictionary<string, (int Count, bool Reclaim, Guid Owner)> Active = [];
    internal static Reservation Acquire(string identity, bool reclaim, Guid owner = default)
    {
        lock (Gate)
        {
            Active.TryGetValue(identity, out var value);
            if (value.Count != 0 && (value.Reclaim || reclaim) && !(reclaim && value.Reclaim && owner != Guid.Empty && owner == value.Owner))
                throw new ArchiveOutputException("reclamation_source_busy");
            Active[identity] = (value.Count + 1, reclaim, owner);
            return new Reservation(identity);
        }
    }
    internal sealed class Reservation(string identity) : IDisposable
    {
        internal string Identity => identity;
        private bool disposed;
        public void Dispose()
        {
            lock (Gate)
            {
                if (disposed) return; disposed = true;
                var value = Active[identity];
                if (value.Count == 1) Active.Remove(identity); else Active[identity] = (value.Count - 1, value.Reclaim, value.Owner);
            }
        }
    }
}
