using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;
using NxTools.PlatformHost.Diagnostics;
using IDataObject = System.Runtime.InteropServices.ComTypes.IDataObject;

namespace NxTools.PlatformHost.Windows;

// Immutable captured representations. Routing belongs to Rust, not this Windows adapter.
internal sealed record FloatingAssistantDropData(string? Text, string? Url, string? Html,
    byte[]? ImageBytes, string? ImageFormat, string[] Files, Guid CorrelationId = default);

[StructLayout(LayoutKind.Sequential)]
internal readonly record struct FloatingAssistantDropPoint(int X, int Y);

[ComImport, Guid("00000122-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IFloatingAssistantDropTarget
{
    [PreserveSig] int DragEnter([MarshalAs(UnmanagedType.Interface)] IDataObject data, uint keys,
        FloatingAssistantDropPoint point, ref uint effect);
    [PreserveSig] int DragOver(uint keys, FloatingAssistantDropPoint point, ref uint effect);
    [PreserveSig] int DragLeave();
    [PreserveSig] int Drop([MarshalAs(UnmanagedType.Interface)] IDataObject data, uint keys,
        FloatingAssistantDropPoint point, ref uint effect);
}

[ComVisible(true), ClassInterface(ClassInterfaceType.None)]
internal sealed class FloatingAssistantDropTarget : IFloatingAssistantDropTarget
{
    private readonly Func<bool> _active;
    private readonly Func<FloatingAssistantDropData, bool> _receive;
    private readonly IPlatformLog _log;
    private Guid? _offer;
    private long _started;
    private static readonly short UrlFormat = Register("UniformResourceLocatorW");
    private static readonly short HtmlFormat = Register("HTML Format");
    private static readonly short PngFormat = Register("PNG");
    private const int ImageLimit = 16 * 1024 * 1024;

    internal FloatingAssistantDropTarget(Func<bool> active, Func<FloatingAssistantDropData, bool> receive,
        IPlatformLog? log = null)
    { _active = active; _receive = receive; _log = log ?? NullPlatformLog.Instance; }

    public int DragEnter(IDataObject data, uint keys, FloatingAssistantDropPoint point, ref uint effect)
    {
        Cancel();
        var allowed = effect & 1;
        effect = 0;
        try
        {
            if (allowed == 0 || !_active() || !Supported(data)) return 0;
            _offer = Guid.NewGuid();
            _started = System.Diagnostics.Stopwatch.GetTimestamp();
            Write("accepted", "supported");
            effect = 1;
        }
        catch (Exception error)
        { Write("rejected", "format_unavailable", error.HResult); }
        return 0;
    }

    public int DragOver(uint keys, FloatingAssistantDropPoint point, ref uint effect)
    {
        try { effect = _offer.HasValue && _active() ? effect & 1 : 0; }
        catch (Exception error) { effect = 0; Write("rejected", "offer_failed", error.HResult); }
        if (effect == 0) Cancel();
        return 0;
    }

    public int DragLeave() { Cancel(); return 0; }

    public int Drop(IDataObject data, uint keys, FloatingAssistantDropPoint point, ref uint effect)
    {
        bool admitted;
        try { admitted = _offer.HasValue && _active() && (effect & 1) != 0; }
        catch (Exception error) { Write("rejected", "offer_failed", error.HResult); admitted = false; }
        effect = 0;
        if (!admitted) { Cancel(); return 0; }
        // Retire admission before any callback; a duplicate Drop/reentrant call cannot repeat it.
        var offer = _offer;
        _offer = null;
        try
        {
            var captured = Capture(data);
            if (_active() && _receive(captured with { CorrelationId = offer!.Value })) effect = 1;
            Write("terminal", effect == 1 ? "captured" : "retired_or_busy", id: offer);
        }
        catch (Exception error)
        {
            // Never let an external IDataObject or consumer unwind into OLE's source application.
            Write("terminal", "capture_failed", error is Win32Exception native ? native.NativeErrorCode : error.HResult, offer);
        }
        return 0;
    }

    internal void Cancel()
    {
        if (_offer is not { } id) return;
        _offer = null;
        Write("terminal", "cancelled", id: id);
    }

    private void Write(string phase, string reason, int? nativeCode = null, Guid? id = null)
    {
        try
        {
            _log.Write("info", "floating_assistant.drop." + phase, id ?? _offer, new Dictionary<string, object?>
            {
                ["reason"] = reason, ["native_error_code"] = nativeCode,
                ["elapsed_ms"] = _started == 0 ? 0 : System.Diagnostics.Stopwatch.GetElapsedTime(_started).TotalMilliseconds,
            });
        }
        catch { /* Diagnostics must not alter a drop result. */ }
    }

    private static bool Supported(IDataObject data) => new short[] { 13, UrlFormat, HtmlFormat, PngFormat, 17, 8, 2, 15 }
        .Any(id => Has(data, id, id == 2 ? TYMED.TYMED_GDI : TYMED.TYMED_HGLOBAL));

    private static FloatingAssistantDropData Capture(IDataObject data)
    {
        var text = Decode(Read(data, 13, 65536), unicode: true);
        var url = Decode(Read(data, UrlFormat, 16386), unicode: true);
        if (url is not null && Encoding.UTF8.GetByteCount(url) > 8192) throw new InvalidDataException("drop_url_budget");
        var html = Decode(Read(data, HtmlFormat, 262144), unicode: false);
        byte[]? image = null;
        string? imageFormat = null;
        foreach (var (id, name) in new[] { (PngFormat, "png"), ((short)17, "dib_v5"), ((short)8, "dib") })
        {
            image = Read(data, id, ImageLimit);
            if (image is null) continue;
            imageFormat = name;
            break;
        }
        if (image is null && Has(data, 2, TYMED.TYMED_GDI))
        {
            var format = Format(2, TYMED.TYMED_GDI);
            data.GetData(ref format, out var medium);
            try
            {
                if (medium.tymed != TYMED.TYMED_GDI || medium.unionmember == IntPtr.Zero ||
                    Native.GetObjectW(medium.unionmember, Marshal.SizeOf<Native.BitmapInfo>(), out var bitmap) == 0 ||
                    bitmap.Width is < 1 or > 4096 || bitmap.Height is < 1 or > 4096 ||
                    (long)bitmap.Width * bitmap.Height * 4 > ImageLimit)
                    throw new InvalidDataException("drop_bitmap_budget");
                using var source = System.Drawing.Image.FromHbitmap(medium.unionmember);
                using var output = new MemoryStream();
                source.Save(output, System.Drawing.Imaging.ImageFormat.Png);
                if (output.Length > ImageLimit) throw new InvalidDataException("drop_image_budget");
                image = output.ToArray();
                imageFormat = "png";
            }
            finally { Native.ReleaseStgMedium(ref medium); }
        }
        var files = Array.Empty<string>();
        if (Has(data, 15, TYMED.TYMED_HGLOBAL))
        {
            var format = Format(15, TYMED.TYMED_HGLOBAL);
            data.GetData(ref format, out var medium);
            try
            {
                if (medium.tymed != TYMED.TYMED_HGLOBAL || medium.unionmember == IntPtr.Zero ||
                    Native.GlobalSize(medium.unionmember).ToUInt64() > 65536)
                    throw new InvalidDataException("drop_file_budget");
                var count = Native.DragQueryFileW(medium.unionmember, uint.MaxValue, null, 0);
                if (count is < 1 or > 32) throw new InvalidDataException("drop_file_count");
                files = new string[count];
                for (uint index = 0; index < count; index++)
                {
                    var length = Native.DragQueryFileW(medium.unionmember, index, null, 0);
                    if (length is < 1 or > 32767) throw new InvalidDataException("drop_file_length");
                    var buffer = new StringBuilder((int)length + 1);
                    if (Native.DragQueryFileW(medium.unionmember, index, buffer, length + 1) != length)
                        throw new InvalidDataException("drop_file_read");
                    files[index] = buffer.ToString();
                }
            }
            finally { Native.ReleaseStgMedium(ref medium); }
        }
        if (text is null && url is null && html is null && image is null && files.Length == 0)
            throw new InvalidDataException("drop_empty");
        return new(text, url, html, image, imageFormat, files);
    }

    private static string? Decode(byte[]? bytes, bool unicode)
    {
        if (bytes is null) return null;
        if (unicode && bytes.Length % 2 != 0) throw new InvalidDataException("drop_utf16");
        Encoding encoding = unicode ? new UnicodeEncoding(false, false, true) : new UTF8Encoding(false, true);
        var value = encoding.GetString(bytes).TrimEnd('\0');
        if (value.Contains('\0')) throw new InvalidDataException("drop_embedded_nul");
        return value.Length == 0 ? null : value;
    }

    private static byte[]? Read(IDataObject data, short id, int limit)
    {
        if (!Has(data, id, TYMED.TYMED_HGLOBAL)) return null;
        var format = Format(id, TYMED.TYMED_HGLOBAL);
        data.GetData(ref format, out var medium);
        try
        {
            if (medium.tymed != TYMED.TYMED_HGLOBAL || medium.unionmember == IntPtr.Zero)
                throw new InvalidDataException("drop_medium");
            var size = Native.GlobalSize(medium.unionmember).ToUInt64();
            if (size == 0 || size > (ulong)limit) throw new InvalidDataException("drop_payload_budget");
            var pointer = Native.GlobalLock(medium.unionmember);
            if (pointer == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try
            {
                var bytes = new byte[(int)size];
                Marshal.Copy(pointer, bytes, 0, bytes.Length);
                return bytes;
            }
            finally { _ = Native.GlobalUnlock(medium.unionmember); }
        }
        finally { Native.ReleaseStgMedium(ref medium); }
    }

    private static bool Has(IDataObject data, short id, TYMED medium)
    { var format = Format(id, medium); return data.QueryGetData(ref format) == 0; }
    private static FORMATETC Format(short id, TYMED medium) => new()
    { cfFormat = id, lindex = -1, dwAspect = DVASPECT.DVASPECT_CONTENT, tymed = medium };
    private static short Register(string name) => unchecked((short)Native.RegisterClipboardFormatW(name));

    private static class Native
    {
        [StructLayout(LayoutKind.Sequential)] internal struct BitmapInfo
        { internal int Type, Width, Height, WidthBytes; internal ushort Planes, BitsPixel; internal IntPtr Bits; }
        [DllImport("user32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)] internal static extern uint RegisterClipboardFormatW(string name);
        [DllImport("kernel32.dll", ExactSpelling = true)] internal static extern UIntPtr GlobalSize(IntPtr memory);
        [DllImport("kernel32.dll", ExactSpelling = true, SetLastError = true)] internal static extern IntPtr GlobalLock(IntPtr memory);
        [DllImport("kernel32.dll", ExactSpelling = true)] [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool GlobalUnlock(IntPtr memory);
        [DllImport("ole32.dll", ExactSpelling = true)] internal static extern void ReleaseStgMedium(ref STGMEDIUM medium);
        [DllImport("shell32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)] internal static extern uint DragQueryFileW(IntPtr drop, uint index, StringBuilder? buffer, uint count);
        [DllImport("gdi32.dll", ExactSpelling = true)] internal static extern int GetObjectW(IntPtr bitmap, int size, out BitmapInfo info);
    }
}
