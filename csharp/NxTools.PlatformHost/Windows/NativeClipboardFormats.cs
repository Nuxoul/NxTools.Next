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
        string? imagePathFallback,
        out string? error,
        out int hresult)
    {
        error = null;
        hresult = 0;
        try
        {
            var payloads = LoadPayloads(formats);
            if (!string.IsNullOrWhiteSpace(imagePathFallback))
            {
                if (!File.Exists(imagePathFallback))
                {
                    error = "The fallback clipboard image file no longer exists.";
                    return false;
                }
                payloads.RemoveAll(static payload => payload.FormatId == 15);
                payloads.Add(new NativePayload(
                    15,
                    ClipboardFileDropPayload.Encode([Path.GetFullPath(imagePathFallback)]),
                    payloads.Count == 0 ? 0 : checked(payloads.Max(static item => item.SortOrder) + 1)));
            }
            return TryWritePayloads(owner, payloads, out error, out hresult);
        }
        catch (Exception exception)
        {
            hresult = exception.HResult;
            error = exception.Message;
            return false;
        }
    }

    public static bool TryRewriteWithPng(
        IntPtr owner,
        IReadOnlyList<CapturedClipboardNativeFormat> formats,
        byte[] pngData,
        out uint pngFormat,
        out string? error,
        out int hresult)
    {
        pngFormat = 0;
        error = null;
        hresult = 0;
        try
        {
            if (pngData.Length == 0)
            {
                error = "The PNG compatibility payload was empty.";
                return false;
            }
            pngFormat = RegisterClipboardFormat("PNG");
            if (pngFormat == 0)
            {
                hresult = Marshal.GetHRForLastWin32Error();
                error = new Win32Exception(Marshal.GetLastWin32Error()).Message;
                return false;
            }

            var payloads = new List<NativePayload>(formats.Count + 1)
            {
                new(pngFormat, pngData, 0)
            };
            payloads.AddRange(formats
                .Where(format => format.Data.Length > 0 &&
                    !string.Equals(format.FormatName, "PNG", StringComparison.OrdinalIgnoreCase))
                .Select(format => new NativePayload(
                    format.FormatId,
                    format.Data,
                    checked(format.SortOrder + 1))));
            return TryWritePayloads(owner, payloads, out error, out hresult);
        }
        catch (Exception exception)
        {
            hresult = exception.HResult;
            error = exception.Message;
            return false;
        }
    }

    public static bool TryRewriteWithImageFallbacks(
        IntPtr owner,
        IReadOnlyList<CapturedClipboardNativeFormat> formats,
        byte[] pngData,
        string imagePathFallback,
        out IReadOnlyList<CapturedClipboardNativeFormat> rewrittenFormats,
        out string? error,
        out int hresult)
    {
        rewrittenFormats = formats;
        error = null;
        hresult = 0;
        try
        {
            var pngFormat = formats
                .FirstOrDefault(static format => string.Equals(
                    format.FormatName, "PNG", StringComparison.OrdinalIgnoreCase))?.FormatId ?? 0;
            if (pngFormat == 0)
            {
                if (pngData.Length == 0)
                {
                    error = "The PNG compatibility payload was empty.";
                    return false;
                }
                pngFormat = RegisterClipboardFormat("PNG");
                if (pngFormat == 0)
                {
                    hresult = Marshal.GetHRForLastWin32Error();
                    error = new Win32Exception(Marshal.GetLastWin32Error()).Message;
                    return false;
                }
            }

            var rewritten = formats.ToList();
            ClipboardImageCompatibility.AddCompatibilityFallbacks(
                rewritten, pngFormat, pngData, Path.GetFullPath(imagePathFallback));
            var payloads = rewritten
                .Where(static format => format.Data.Length > 0)
                .Select(static format => new NativePayload(
                    format.FormatId, format.Data, format.SortOrder))
                .ToList();
            if (!TryWritePayloads(owner, payloads, out error, out hresult))
            {
                return false;
            }
            rewrittenFormats = rewritten;
            return true;
        }
        catch (Exception exception)
        {
            hresult = exception.HResult;
            error = exception.Message;
            return false;
        }
    }

    private static bool TryWritePayloads(
        IntPtr owner,
        IReadOnlyList<NativePayload> payloads,
        out string? error,
        out int hresult)
    {
        error = null;
        hresult = 0;
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

    // Handle-backed formats cannot be serialized as movable global-memory bytes.
    // CF_DIB (8) and CF_DIBV5 (17) are intentionally retained for original image paste.
    private static bool ShouldSkip(uint format) => format is
        2 or 3 or 9 or 14 or 16 or
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
