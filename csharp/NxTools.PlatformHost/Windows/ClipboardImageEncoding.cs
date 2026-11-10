using System.Drawing;
using System.Drawing.Imaging;

namespace NxTools.PlatformHost.Windows;

internal sealed class ClipboardImageBudgetExceededException(string reason, ulong observed, ulong maximum)
    : IOException("Clipboard image budget exceeded.")
{
    internal string Reason { get; } = reason;
    internal ulong Observed { get; } = observed;
    internal ulong Maximum { get; } = maximum;
}

internal static class ClipboardImageEncoding
{
    internal static Size ReadDimensions(Stream input)
    {
        Size size;
        using (var metadata = Image.FromStream(input, useEmbeddedColorManagement: false, validateImageData: false))
            size = metadata.Size;
        input.Position = 0;
        return size;
    }

    internal static void SavePng(Image image, string path, ulong maximumBytes)
    {
        using var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
        using var bounded = new PngOutputStream(output, maximumBytes);
        try { image.Save(bounded, ImageFormat.Png); }
        catch (Exception) when (bounded.Rejection is { } rejected) { throw rejected; }
        // Native codecs may replace or consume a managed IStream write failure.
        if (bounded.Rejection is { } failure) throw failure;
    }

    internal static void CheckPixels(int width, int height, ulong maximumPixels)
    {
        var pixels = checked((ulong)width * (ulong)height);
        if (pixels > maximumPixels)
            throw new ClipboardImageBudgetExceededException("max_image_pixels", pixels, maximumPixels);
    }

    private sealed class PngOutputStream(Stream output, ulong maximumBytes) : Stream
    {
        private readonly long maximum = checked((long)maximumBytes);
        internal ClipboardImageBudgetExceededException? Rejection { get; private set; }
        public override bool CanRead => false;
        public override bool CanSeek => output.CanSeek;
        public override bool CanWrite => output.CanWrite;
        public override long Length => output.Length;
        public override long Position
        {
            get => output.Position;
            set { CheckEnd(value); output.Position = value; }
        }
        public override void Flush() => output.Flush();
        public override int Read(byte[] buffer, int offset, int count) => throw new NotSupportedException();
        public override void Write(byte[] buffer, int offset, int count)
        {
            CheckEnd(checked(output.Position + count));
            output.Write(buffer, offset, count);
        }
        public override void SetLength(long value) { CheckEnd(value); output.SetLength(value); }
        public override long Seek(long offset, SeekOrigin origin)
        {
            var end = origin switch
            {
                SeekOrigin.Begin => offset,
                SeekOrigin.Current => checked(output.Position + offset),
                SeekOrigin.End => checked(output.Length + offset),
                _ => throw new ArgumentOutOfRangeException(nameof(origin))
            };
            CheckEnd(end);
            return output.Seek(offset, origin);
        }
        private void CheckEnd(long end)
        {
            if (end <= maximum && Rejection is null) return;
            Rejection ??= new("max_image_bytes", checked((ulong)end), checked((ulong)maximum));
            throw Rejection;
        }
    }
}
