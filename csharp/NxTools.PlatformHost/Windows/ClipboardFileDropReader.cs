using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed class ClipboardFileDropException(string code, int? native = null) : Exception(code)
{
    internal string Code { get; } = code;
    internal int? NativeCode { get; } = native;
}

// CF_HDROP belongs to the clipboard. Only owned test handles may be freed by their caller.
internal interface IClipboardFileDropAccess
{
    bool Open(IntPtr owner);
    bool HasDrop();
    IntPtr GetDrop();
    void Close();
}

internal static partial class ClipboardFileDropReader
{
    private const uint CfHdrop = 15;
    private const int MaxPathCount = 32;
    private const int MaxBytes = 131072;
    private const int MaxListBytes = 32 * (32767 + 1) * 2 + 24;

    private sealed class NativeAccess : IClipboardFileDropAccess
    {
        public bool Open(IntPtr owner) => OpenClipboard(owner);
        public bool HasDrop() => IsClipboardFormatAvailable(CfHdrop);
        public IntPtr GetDrop() => GetClipboardData(CfHdrop);
        public void Close() => _ = CloseClipboard();
    }
    internal static string[] ReadClipboard(IntPtr owner, IClipboardFileDropAccess? access = null)
    {
        access ??= new NativeAccess();
        if (!access.Open(owner)) throw new ClipboardFileDropException("clipboard_unavailable", Marshal.GetLastWin32Error());
        try
        {
            if (!access.HasDrop()) return [];
            var handle = access.GetDrop();
            if (handle == IntPtr.Zero) throw new ClipboardFileDropException("clipboard_failed", Marshal.GetLastWin32Error());
            return ReadHandle(handle);
        }
        finally { access.Close(); }
    }

    // The handle is borrowed. Validate the DROPFILES header and complete double-terminated
    // list within GlobalSize before asking Shell32 to convert ANSI/Unicode paths.
    internal static string[] ReadHandle(IntPtr handle)
    {
        var size = GlobalSize(handle).ToUInt64();
        if (size < 22) throw new ClipboardFileDropException("invalid_clipboard_data");
        var pointer = GlobalLock(handle);
        if (pointer == IntPtr.Zero) throw new ClipboardFileDropException("clipboard_failed", Marshal.GetLastWin32Error());
        try
        {
            var offset = (uint)Marshal.ReadInt32(pointer);
            var wide = Marshal.ReadInt32(pointer, 16);
            if (offset < 20 || offset >= size || wide is not (0 or 1)) throw new ClipboardFileDropException("invalid_clipboard_data");
            var stride = wide == 1 ? 2 : 1;
            if (wide == 1 && (offset & 1) != 0) throw new ClipboardFileDropException("invalid_clipboard_data");
            var end = Math.Min(size, (ulong)MaxListBytes);
            var position = (ulong)offset;
            var count = 0;
            var terminated = false;
            while (position + (ulong)stride <= end)
            {
                var start = position;
                while (position + (ulong)stride <= end && (wide == 1
                    ? Marshal.ReadInt16(pointer, checked((int)position)) != 0
                    : Marshal.ReadByte(pointer, checked((int)position)) != 0)) position += (ulong)stride;
                if (position + (ulong)stride > end) break;
                if (position == start)
                {
                    if (count == 0 && position + (ulong)(2 * stride) > end)
                        throw new ClipboardFileDropException("invalid_clipboard_data");
                    terminated = true; break;
                }
                if (++count > MaxPathCount) throw new ClipboardFileDropException("clipboard_limit");
                if ((position - start) / (ulong)stride > 32767) throw new ClipboardFileDropException("clipboard_limit");
                position += (ulong)stride;
            }
            if (!terminated) throw new ClipboardFileDropException(end < size ? "clipboard_limit" : "invalid_clipboard_data");
            if (DragQueryFile(handle, uint.MaxValue, null, 0) != count)
                throw new ClipboardFileDropException("invalid_clipboard_data");
            var paths = new string[count];
            var bytes = 0;
            for (uint index = 0; index < count; ++index)
            {
                var length = DragQueryFile(handle, index, null, 0);
                if (length == 0 || length > 32767) throw new ClipboardFileDropException("invalid_clipboard_data");
                var buffer = new StringBuilder(checked((int)length + 1));
                if (DragQueryFile(handle, index, buffer, checked(length + 1)) != length)
                    throw new ClipboardFileDropException("invalid_clipboard_data");
                var path = buffer.ToString();
                if (!ArchiveJobWire.PathValid(path)) throw new ClipboardFileDropException("invalid_clipboard_data");
                bytes += Encoding.UTF8.GetByteCount(path);
                if (bytes > MaxBytes) throw new ClipboardFileDropException("clipboard_limit");
                paths[index] = path;
            }
            return paths;
        }
        finally { _ = GlobalUnlock(handle); }
    }

    [DllImport("user32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenClipboard(IntPtr owner);
    [DllImport("user32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseClipboard();
    [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsClipboardFormatAvailable(uint format);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr GetClipboardData(uint format);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern UIntPtr GlobalSize(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GlobalLock(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GlobalUnlock(IntPtr handle);
    [DllImport("shell32.dll", EntryPoint = "DragQueryFileW", CharSet = CharSet.Unicode)]
    private static extern uint DragQueryFile(IntPtr drop, uint index, StringBuilder? path, uint capacity);
}
