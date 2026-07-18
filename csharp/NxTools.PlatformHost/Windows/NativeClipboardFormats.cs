using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record CapturedClipboardNativeFormat(
    uint FormatId,
    string FormatName,
    byte[] Data,
    uint SortOrder);

internal static class NativeClipboardFormats
{
    private const ulong MaxFormatBytes = 8 * 1024 * 1024;
    private const ulong MaxCaptureBytes = 32 * 1024 * 1024;
    private const int MaxFormatCount = 64;
    private const uint GmemMoveable = 0x0002;

    public static IReadOnlyList<CapturedClipboardNativeFormat> Capture(IntPtr owner)
    {
        if (!OpenClipboard(owner))
        {
            throw new ExternalException(
                "Could not open the Windows clipboard for native format capture.",
                Marshal.GetHRForLastWin32Error());
        }

        try
        {
            var captured = new List<CapturedClipboardNativeFormat>();
            ulong totalBytes = 0;
            uint format = 0;
            while (captured.Count < MaxFormatCount &&
                   (format = EnumClipboardFormats(format)) != 0)
            {
                if (ShouldSkip(format))
                {
                    continue;
                }
                var handle = GetClipboardData(format);
                if (handle == IntPtr.Zero)
                {
                    continue;
                }
                var size = GlobalSize(handle).ToUInt64();
                if (size == 0 || size > MaxFormatBytes || totalBytes + size > MaxCaptureBytes)
                {
                    continue;
                }
                var pointer = GlobalLock(handle);
                if (pointer == IntPtr.Zero)
                {
                    continue;
                }
                try
                {
                    var data = new byte[checked((int)size)];
                    Marshal.Copy(pointer, data, 0, data.Length);
                    captured.Add(new CapturedClipboardNativeFormat(
                        format,
                        GetFormatName(format),
                        data,
                        checked((uint)captured.Count)));
                    totalBytes += size;
                }
                finally
                {
                    _ = GlobalUnlock(handle);
                }
            }
            return captured;
        }
        finally
        {
            _ = CloseClipboard();
        }
    }

    public static bool TryWrite(
        IntPtr owner,
        IReadOnlyList<ClipboardNativeFormatData> formats,
        out string? error,
        out int hresult)
    {
        error = null;
        hresult = 0;
        try
        {
            var payloads = LoadPayloads(formats);
            if (payloads.Count == 0)
            {
                error = "No native clipboard formats were available.";
                return false;
            }
            if (!OpenClipboard(owner))
            {
                hresult = Marshal.GetHRForLastWin32Error();
                error = new Win32Exception(Marshal.GetLastWin32Error()).Message;
                return false;
            }
            try
            {
                if (!EmptyClipboard())
                {
                    hresult = Marshal.GetHRForLastWin32Error();
                    error = new Win32Exception(Marshal.GetLastWin32Error()).Message;
                    return false;
                }
                foreach (var payload in payloads.OrderBy(static item => item.SortOrder))
                {
                    var memory = GlobalAlloc(GmemMoveable, new UIntPtr((uint)payload.Data.Length));
                    if (memory == IntPtr.Zero)
                    {
                        hresult = Marshal.GetHRForLastWin32Error();
                        error = "Could not allocate native clipboard memory.";
                        return false;
                    }
                    var pointer = GlobalLock(memory);
                    if (pointer == IntPtr.Zero)
                    {
                        _ = GlobalFree(memory);
                        hresult = Marshal.GetHRForLastWin32Error();
                        error = "Could not lock native clipboard memory.";
                        return false;
                    }
                    Marshal.Copy(payload.Data, 0, pointer, payload.Data.Length);
                    _ = GlobalUnlock(memory);
                    if (SetClipboardData(payload.FormatId, memory) == IntPtr.Zero)
                    {
                        _ = GlobalFree(memory);
                        hresult = Marshal.GetHRForLastWin32Error();
                        error = new Win32Exception(Marshal.GetLastWin32Error()).Message;
                        return false;
                    }
                }
                return true;
            }
            finally
            {
                _ = CloseClipboard();
            }
        }
        catch (Exception exception)
        {
            hresult = exception.HResult;
            error = exception.Message;
            return false;
        }
    }

    private static List<NativePayload> LoadPayloads(IReadOnlyList<ClipboardNativeFormatData> formats)
    {
        var payloads = new List<NativePayload>();
        ulong totalBytes = 0;
        foreach (var format in formats.Take(MaxFormatCount))
        {
            var info = new FileInfo(format.DataPath);
            if (!info.Exists || info.Length <= 0 || (ulong)info.Length > MaxFormatBytes)
            {
                continue;
            }
            var size = (ulong)info.Length;
            if (format.SizeBytes != size || totalBytes + size > MaxCaptureBytes)
            {
                continue;
            }
            var formatId = format.FormatId >= 0xC000 && !string.IsNullOrWhiteSpace(format.FormatName)
                ? RegisterClipboardFormat(format.FormatName)
                : format.FormatId;
            if (formatId == 0)
            {
                continue;
            }
            payloads.Add(new NativePayload(
                formatId,
                File.ReadAllBytes(format.DataPath),
                format.SortOrder));
            totalBytes += size;
        }
        return payloads;
    }

    private static bool ShouldSkip(uint format) => format is
        2 or 3 or 6 or 7 or 8 or 9 or 14 or 16 or 17 or
        >= 0x0080 and <= 0x008F;

    private static string GetFormatName(uint format)
    {
        if (format < 0xC000)
        {
            return string.Empty;
        }
        var name = new StringBuilder(256);
        return GetClipboardFormatName(format, name, name.Capacity) > 0
            ? name.ToString()
            : string.Empty;
    }

    private sealed record NativePayload(uint FormatId, byte[] Data, uint SortOrder);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenClipboard(IntPtr owner);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseClipboard();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint EnumClipboardFormats(uint format);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr GetClipboardData(uint format);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern int GetClipboardFormatName(uint format, StringBuilder name, int maxCount);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern uint RegisterClipboardFormat(string format);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EmptyClipboard();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetClipboardData(uint format, IntPtr memory);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern UIntPtr GlobalSize(IntPtr memory);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalLock(IntPtr memory);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GlobalUnlock(IntPtr memory);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalAlloc(uint flags, UIntPtr bytes);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalFree(IntPtr memory);
}
