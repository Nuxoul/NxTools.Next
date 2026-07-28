using System.Drawing;
using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

internal sealed class ClipboardImageDataObject : IDisposable
{
    private readonly Bitmap _bitmap;
    private readonly MemoryStream _pngStream;

    private ClipboardImageDataObject(DataObject data, Bitmap bitmap, MemoryStream pngStream)
    {
        Data = data;
        _bitmap = bitmap;
        _pngStream = pngStream;
    }

    public DataObject Data { get; }

    public static ClipboardImageDataObject Load(string path, bool includeFileDrop = true)
    {
        using var source = Image.FromFile(path);
        var bitmap = new Bitmap(source);
        var pngStream = new MemoryStream();
        bitmap.Save(pngStream, System.Drawing.Imaging.ImageFormat.Png);
        pngStream.Position = 0;
        var data = new DataObject();
        data.SetData("PNG", autoConvert: false, pngStream);
        data.SetData(DataFormats.Bitmap, autoConvert: true, bitmap);
        if (includeFileDrop)
        {
            data.SetData(DataFormats.FileDrop, autoConvert: true, new[] { Path.GetFullPath(path) });
        }
        return new ClipboardImageDataObject(data, bitmap, pngStream);
    }

    public void Dispose()
    {
        _pngStream.Dispose();
        _bitmap.Dispose();
    }
}
