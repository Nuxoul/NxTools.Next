using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media.Imaging;

namespace NxTools.PlatformHost.Windows;

[StructLayout(LayoutKind.Sequential)]
internal readonly record struct ShellImageSize(int Width, int Height);

// Only the native leaf is replaceable; STA admission, resolution, cache and PNG publication stay upstream.
internal interface IShellItemImageNative
{
    int Create(string target, Guid interfaceId, out IntPtr factory);
    int GetImage(IntPtr factory, ShellImageSize size, uint flags, out IntPtr bitmap);
    void Release(IntPtr factory);
    void DeleteBitmap(IntPtr bitmap);
}

internal sealed class ShellItemImageFactory(IShellItemImageNative? native = null)
{
    private static readonly Guid InterfaceId = new("bcc18b79-ba16-442f-80c4-8a59c30c463b");
    private const uint IconOnly = 4;
    private const int Pending = unchecked((int)0x8000000A);
    private readonly IShellItemImageNative _native = native ?? new ShellItemImageNative();

    internal BitmapSource? Load(string target, uint size, CancellationToken cancellationToken, out int status)
    {
        cancellationToken.ThrowIfCancellationRequested();
        IntPtr factory = IntPtr.Zero;
        try
        {
            status = _native.Create(target, InterfaceId, out factory);
            if (status < 0) return null;
            if (factory == IntPtr.Zero)
            {
                status = unchecked((int)0x80004003); // E_POINTER
                return null;
            }
            for (var attempt = 0; ; attempt++)
            {
                cancellationToken.ThrowIfCancellationRequested();
                IntPtr bitmap = IntPtr.Zero;
                try
                {
                    status = _native.GetImage(factory, new ShellImageSize((int)size, (int)size), IconOnly, out bitmap);
                    cancellationToken.ThrowIfCancellationRequested();
                    if (status >= 0 && bitmap != IntPtr.Zero)
                    {
                        var image = Imaging.CreateBitmapSourceFromHBitmap(bitmap, IntPtr.Zero,
                            Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
                        image.Freeze();
                        return image;
                    }
                    if (status >= 0) status = unchecked((int)0x80004003);
                    if (status != Pending || attempt >= 5) return null;
                }
                finally
                {
                    if (bitmap != IntPtr.Zero) _native.DeleteBitmap(bitmap);
                }
                if (cancellationToken.WaitHandle.WaitOne(TimeSpan.FromMilliseconds(200)))
                    cancellationToken.ThrowIfCancellationRequested();
            }
        }
        finally
        {
            if (factory != IntPtr.Zero) _native.Release(factory);
        }
    }
}

internal sealed class ShellItemImageNative : IShellItemImageNative
{
    public int Create(string target, Guid interfaceId, out IntPtr factory) =>
        SHCreateItemFromParsingName(target, IntPtr.Zero, ref interfaceId, out factory);

    public int GetImage(IntPtr factory, ShellImageSize size, uint flags, out IntPtr bitmap)
    {
        // IShellItemImageFactory derives directly from IUnknown; GetImage is native slot 3.
        var table = Marshal.ReadIntPtr(factory);
        var getImage = Marshal.GetDelegateForFunctionPointer<GetImageDelegate>(Marshal.ReadIntPtr(table, 3 * IntPtr.Size));
        return getImage(factory, size, flags, out bitmap);
    }

    public void Release(IntPtr factory) => _ = Marshal.Release(factory);
    public void DeleteBitmap(IntPtr bitmap) => _ = DeleteObject(bitmap);

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int GetImageDelegate(IntPtr factory, ShellImageSize size, uint flags, out IntPtr bitmap);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = true)]
    private static extern int SHCreateItemFromParsingName(string target, IntPtr bindContext,
        ref Guid interfaceId, out IntPtr factory);

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DeleteObject(IntPtr bitmap);
}
