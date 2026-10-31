using Microsoft.Win32.SafeHandles;
using System.Security.Cryptography;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows.Archive;

// Deliberately not a general ADS API. The only stream name is fixed at the native owner.
internal static class ArchiveMotw
{
    internal const int MaximumBytes = 65536;
    [DllImport("kernel32.dll", EntryPoint = "GetVolumeInformationByHandleW", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool VolumeInfo(SafeFileHandle file, IntPtr name, uint nameLength, out uint serial, out uint componentLength, out uint flags, IntPtr fileSystem, uint fileSystemLength);
    internal static void EnsureSupported(SafeFileHandle file, bool writing, Func<bool, uint, uint>? flagsProbe = null)
    {
        if (!VolumeInfo(file, IntPtr.Zero, 0, out _, out _, out var flags, IntPtr.Zero, 0))
            throw Classify("capability", Marshal.GetLastWin32Error());
        flags = flagsProbe?.Invoke(writing, flags) ?? flags;
        if ((flags & 0x40000) == 0) throw new ArchiveOutputException("motw_not_supported");
    }
    internal static byte[]? Read(SafeFileHandle source, bool shareDelete = false)
    {
        SafeFileHandle stream;
        try { stream = OutputNative.OpenZoneIdentifier(source, create: false, shareDelete: shareDelete); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return null; }
        catch (ArchiveOutputException failure) { throw Classify("read", failure.NativeCode); }
        using (stream) {
            byte[]? bytes = null;
            try {
                var length = RandomAccess.GetLength(stream);
                if (length is < 0 or > MaximumBytes) throw new ArchiveOutputException("motw_budget");
                bytes = new byte[(int)length];
                var offset = 0;
                while (offset < bytes.Length) {
                    var count = RandomAccess.Read(stream, bytes.AsSpan(offset), offset);
                    if (count == 0) throw new ArchiveOutputException("motw_source_changed");
                    offset += count;
                }
                Span<byte> extra = stackalloc byte[1];
                if (RandomAccess.Read(stream, extra, length) != 0 || RandomAccess.GetLength(stream) != length)
                    throw new ArchiveOutputException("motw_source_changed");
                var result = bytes; bytes = null; return result;
            }
            catch (IOException failure) { throw Classify("read", failure.HResult & 0xffff); }
            finally { if (bytes is not null) CryptographicOperations.ZeroMemory(bytes); }
        }
    }
    internal static void Write(SafeFileHandle partial, ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length > MaximumBytes) throw new ArchiveOutputException("motw_budget");
        try {
            using var stream = OutputNative.OpenZoneIdentifier(partial, create: true);
            RandomAccess.Write(stream, bytes, 0);
            OutputNative.Flush(stream);
        }
        catch (ArchiveOutputException failure) { throw Classify("write", failure.NativeCode); }
        catch (IOException failure) { throw Classify("write", failure.HResult & 0xffff); }
    }
    internal static bool Matches(SafeFileHandle file, byte[]? expectedHash)
    {
        var bytes = Read(file, shareDelete: true);
        if (bytes is null) return expectedHash is null;
        try { return expectedHash is not null && CryptographicOperations.FixedTimeEquals(SHA256.HashData(bytes), expectedHash); }
        finally { CryptographicOperations.ZeroMemory(bytes); }
    }
    private static ArchiveOutputException Classify(string stage, int? code) =>
        new(code is 1 or 50 or 120 ? "motw_not_supported" : "motw_" + stage + "_failed", code);
}
