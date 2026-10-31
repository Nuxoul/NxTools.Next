using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

// Only immutable values, never paths/raw attribute masks/ADS names from a worker.
internal sealed record ArchiveOutputMetadata(long? Modified, bool? ReadOnly = null, bool? Hidden = null)
{
    internal static ArchiveOutputMetadata Parse(JsonElement value)
    {
        try {
            Fields(value, "modified", "attributes");
            bool? readOnly = null, hidden = null;
            var attributes = value.GetProperty("attributes");
            if (attributes.ValueKind != JsonValueKind.Null) {
                Fields(attributes, "read_only", "hidden");
                readOnly = attributes.GetProperty("read_only").GetBoolean(); hidden = attributes.GetProperty("hidden").GetBoolean();
            }
            return new(ParseTime(value.GetProperty("modified")), readOnly, hidden);
        }
        catch (Exception failure) when (failure is ArgumentException or InvalidOperationException or FormatException or OverflowException or KeyNotFoundException)
        { throw new ArchiveOutputException("metadata_invalid"); }
    }
    private static long? ParseTime(JsonElement modified)
    {
        if (modified.ValueKind == JsonValueKind.Null) return null;
        switch (modified.GetProperty("kind").GetString()) {
            case "dos_local":
                Fields(modified, "kind", "date", "time");
                var date = modified.GetProperty("date").GetUInt16(); var time = modified.GetProperty("time").GetUInt16();
                var local = new DateTime(1980 + (date >> 9), (date >> 5) & 15, date & 31,
                    time >> 11, (time >> 5) & 63, (time & 31) * 2, DateTimeKind.Unspecified);
                if (TimeZoneInfo.Local.IsAmbiguousTime(local) || TimeZoneInfo.Local.IsInvalidTime(local))
                    throw new ArchiveOutputException("metadata_local_time_ambiguous");
                return TimeZoneInfo.ConvertTimeToUtc(local, TimeZoneInfo.Local).ToFileTimeUtc();
            case "filetime_utc":
                Fields(modified, "kind", "ticks");
                var ticks = modified.GetProperty("ticks").GetUInt64();
                if (ticks > 2650467743999999999UL) throw new ArchiveOutputException("metadata_invalid");
                return (long)ticks;
            case "unix_utc":
                Fields(modified, "kind", "seconds", "nanoseconds", "precision_ns");
                var seconds = modified.GetProperty("seconds").GetInt64(); var nanos = modified.GetProperty("nanoseconds").GetUInt32();
                var precision = modified.GetProperty("precision_ns").GetUInt32();
                if (seconds is < -11644473600L or > 253402300799L || nanos >= 1000000000 ||
                    precision is not (1 or 10 or 100 or 1000 or 10000 or 100000 or 1000000 or 10000000 or 100000000 or 1000000000))
                    throw new ArchiveOutputException("metadata_invalid");
                return checked((seconds + 11644473600L) * 10000000L + nanos / 100);
            default: throw new ArchiveOutputException("metadata_invalid");
        }
    }
    private static void Fields(JsonElement value, params string[] names)
    {
        if (value.ValueKind != JsonValueKind.Object || value.EnumerateObject().Count() != names.Length ||
            value.EnumerateObject().Any(field => !names.Contains(field.Name, StringComparer.Ordinal)) ||
            names.Any(name => value.EnumerateObject().Count(field => field.NameEquals(name)) != 1))
            throw new ArchiveOutputException("metadata_invalid");
    }
}

internal static class ArchiveMetadataNative
{
    [StructLayout(LayoutKind.Sequential)]
    private struct BasicInfo { internal long Creation, Access, Write, Change; internal uint Attributes; }
    [DllImport("kernel32.dll", EntryPoint = "SetFileTime", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetTime(SafeFileHandle file, IntPtr creation, IntPtr access, ref long write);
    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetBasic(SafeFileHandle file, int kind, out BasicInfo info, uint size);
    [DllImport("kernel32.dll", EntryPoint = "SetFileInformationByHandle", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetBasic(SafeFileHandle file, int kind, ref BasicInfo info, uint size);
    internal static void Apply(SafeFileHandle file, ArchiveOutputMetadata metadata)
    {
        if (metadata.Modified is { } modified) {
            if (!SetTime(file, IntPtr.Zero, IntPtr.Zero, ref modified))
                throw new ArchiveOutputException("metadata_time_failed", Marshal.GetLastWin32Error());
            // Zero is valid archive FILETIME, but Windows treats it as "leave unchanged".
            // Never report restoration merely because this API returned success.
            if (OutputNative.Identity(file).Modified != modified)
                throw new ArchiveOutputException("metadata_time_not_supported");
        }
        if (metadata.ReadOnly is { } readOnly && metadata.Hidden is { } hidden) {
            var current = Attributes(file);
            SetAttributes(file, (current & ~3u) | (readOnly ? 1u : 0u) | (hidden ? 2u : 0u));
        }
    }
    internal static uint Attributes(SafeFileHandle file)
    {
        if (!GetBasic(file, 0, out var info, (uint)Marshal.SizeOf<BasicInfo>()))
            throw new ArchiveOutputException("metadata_attributes_failed", Marshal.GetLastWin32Error());
        return info.Attributes;
    }
    // Edit-only ordinary DOS file attributes; filesystem-managed compression/encryption,
    // reparse and directory flags are never transplanted to another object.
    internal static void PreserveEditAttributes(SafeFileHandle file, uint sourceAttributes) =>
        SetAttributes(file, (Attributes(file) & ~0x27u) | (sourceAttributes & 0x27u));
    private static void SetAttributes(SafeFileHandle file, uint attributes)
    {
        var info = new BasicInfo { Attributes = (attributes & ~0x80u) == 0 ? 0x80u : attributes & ~0x80u };
        if (!SetBasic(file, 0, ref info, (uint)Marshal.SizeOf<BasicInfo>()))
            throw new ArchiveOutputException("metadata_attributes_failed", Marshal.GetLastWin32Error());
    }
    // Call only for an exact owned unpublished partial, or after all existing cleanup credentials match.
    internal static void ClearOwnedReadOnly(SafeFileHandle file)
    {
        var attributes = Attributes(file);
        if ((attributes & 1) != 0) SetAttributes(file, attributes & ~1u);
    }
}
