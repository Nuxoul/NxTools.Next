using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal sealed record PromotedClipboardImage(ClipboardImageData Image, string[] RemainingPaths);

internal static class ClipboardImagePromotion
{
    public static PromotedClipboardImage? TryPromote(
        IReadOnlyList<string> paths,
        string? text,
        string stagingDirectory)
    {
        var candidate = SinglePath(paths) ?? SingleTextPath(text);
        if (candidate is null || !File.Exists(candidate))
        {
            return null;
        }

        try
        {
            using var stream = new FileStream(
                candidate, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
            if (!LooksLikeImage(candidate, stream))
            {
                return null;
            }
            stream.Position = 0;
            using var source = Image.FromStream(
                stream, useEmbeddedColorManagement: false, validateImageData: true);
            Directory.CreateDirectory(stagingDirectory);
            var stagedPath = Path.Combine(stagingDirectory, $"{Guid.NewGuid():N}.png");
            source.Save(stagedPath, ImageFormat.Png);
            return new PromotedClipboardImage(
                new ClipboardImageData(
                    stagedPath,
                    checked((uint)source.Width),
                    checked((uint)source.Height)),
                []);
        }
        catch (Exception error) when (
            error is ArgumentException or ExternalException or IOException
                or UnauthorizedAccessException or OutOfMemoryException)
        {
            return null;
        }
    }

    private static string? SinglePath(IReadOnlyList<string> paths) =>
        paths.Count == 1 && !string.IsNullOrWhiteSpace(paths[0])
            ? paths[0].Trim().Trim('"')
            : null;

    private static string? SingleTextPath(string? text)
    {
        if (string.IsNullOrWhiteSpace(text) || text.IndexOfAny(['\r', '\n']) >= 0)
        {
            return null;
        }
        var candidate = text.Trim().Trim('"');
        return Path.IsPathFullyQualified(candidate) ? candidate : null;
    }

    private static bool LooksLikeImage(string path, Stream stream)
    {
        var extension = Path.GetExtension(path);
        if (extension.Equals(".png", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".jpg", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".jpeg", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".bmp", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".gif", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".webp", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".tif", StringComparison.OrdinalIgnoreCase)
            || extension.Equals(".tiff", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        Span<byte> header = stackalloc byte[12];
        var read = stream.Read(header);
        return read >= 8 && header[..8].SequenceEqual(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 })
            || read >= 3 && header[0] == 0xFF && header[1] == 0xD8 && header[2] == 0xFF
            || read >= 6 && (header[..6].SequenceEqual("GIF87a"u8) || header[..6].SequenceEqual("GIF89a"u8))
            || read >= 2 && header[0] == (byte)'B' && header[1] == (byte)'M'
            || read >= 12 && header[..4].SequenceEqual("RIFF"u8) && header[8..12].SequenceEqual("WEBP"u8);
    }
}
