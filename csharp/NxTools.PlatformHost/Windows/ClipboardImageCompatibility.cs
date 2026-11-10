using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardImageCompatibility
{
    private const string PngFormatName = "PNG";
    private const uint FileDropFormatId = 15;
    private const uint DibFormatId = 8;
    private const uint DibV5FormatId = 17;
    private const int DibHeaderSize = 40;
    private const int DibV5HeaderSize = 124;
    private const int MaximumCanonicalBitmapBytes = 32 * 1024 * 1024;

    public static bool TryCreateCanonicalBitmapFormats(
        byte[] pngData,
        out byte[]? dibV5Data,
        out byte[]? dibData)
    {
        dibV5Data = null;
        dibData = null;
        if (pngData.Length == 0)
        {
            return false;
        }

        try
        {
            using var stream = new MemoryStream(pngData, writable: false);
            var size = ClipboardImageEncoding.ReadDimensions(stream);
            var sourcePixelBytes = checked((long)size.Width * size.Height * 4L);
            if (size.Width <= 0 || size.Height <= 0 || sourcePixelBytes <= 0 ||
                sourcePixelBytes > MaximumCanonicalBitmapBytes) return false;
            using var source = Image.FromStream(
                stream, useEmbeddedColorManagement: false, validateImageData: true);

            var rowBytes = checked((int)(size.Width * 4L));
            var pixelBytes = checked((int)sourcePixelBytes);
            using var bitmap = new Bitmap(
                size.Width,
                size.Height,
                PixelFormat.Format32bppArgb);
            using (var graphics = Graphics.FromImage(bitmap))
            {
                graphics.DrawImageUnscaled(source, 0, 0);
            }

            var pixels = ReadBgraPixels(bitmap, rowBytes, pixelBytes);
            var dibV5 = CreateDibV5(bitmap.Width, bitmap.Height, pixels);
            var dib = CreateDib(bitmap.Width, bitmap.Height, pixels);
            dibV5Data = dibV5;
            dibData = dib;
            return true;
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (ExternalException)
        {
            return false;
        }
        catch (IOException)
        {
            return false;
        }
        catch (OverflowException)
        {
            return false;
        }
        catch (OutOfMemoryException)
        {
            return false;
        }

    }

    private static byte[] ReadBgraPixels(Bitmap bitmap, int rowBytes, int pixelBytes)
    {
        var pixels = new byte[pixelBytes];
        var rectangle = new Rectangle(0, 0, bitmap.Width, bitmap.Height);
        var locked = bitmap.LockBits(
            rectangle,
            ImageLockMode.ReadOnly,
            PixelFormat.Format32bppArgb);
        try
        {
            var stride = Math.Abs(locked.Stride);
            for (var row = 0; row < bitmap.Height; row++)
            {
                var sourceRow = locked.Stride >= 0 ? row : bitmap.Height - row - 1;
                Marshal.Copy(
                    IntPtr.Add(locked.Scan0, sourceRow * stride),
                    pixels,
                    row * rowBytes,
                    rowBytes);
            }
        }
        finally
        {
            bitmap.UnlockBits(locked);
        }

        return pixels;
    }

    private static byte[] CreateDib(int width, int height, byte[] pixels)
    {
        var data = new byte[checked(DibHeaderSize + pixels.Length)];
        WriteUInt32(data, 0, DibHeaderSize);
        WriteInt32(data, 4, width);
        WriteInt32(data, 8, -height);
        WriteUInt16(data, 12, 1);
        WriteUInt16(data, 14, 32);
        WriteUInt32(data, 16, 0);
        WriteUInt32(data, 20, checked((uint)pixels.Length));
        Buffer.BlockCopy(pixels, 0, data, DibHeaderSize, pixels.Length);
        return data;
    }

    private static byte[] CreateDibV5(int width, int height, byte[] pixels)
    {
        var data = new byte[checked(DibV5HeaderSize + pixels.Length)];
        WriteUInt32(data, 0, DibV5HeaderSize);
        WriteInt32(data, 4, width);
        WriteInt32(data, 8, -height);
        WriteUInt16(data, 12, 1);
        WriteUInt16(data, 14, 32);
        WriteUInt32(data, 16, 3);
        WriteUInt32(data, 20, checked((uint)pixels.Length));
        WriteUInt32(data, 40, 0x00FF0000);
        WriteUInt32(data, 44, 0x0000FF00);
        WriteUInt32(data, 48, 0x000000FF);
        WriteUInt32(data, 52, 0xFF000000);
        WriteUInt32(data, 56, 0x73524742);
        WriteUInt32(data, 108, 4);
        Buffer.BlockCopy(pixels, 0, data, DibV5HeaderSize, pixels.Length);
        return data;
    }

    private static void WriteUInt16(byte[] data, int offset, ushort value) =>
        BinaryPrimitives.WriteUInt16LittleEndian(data.AsSpan(offset, sizeof(ushort)), value);

    private static void WriteUInt32(byte[] data, int offset, uint value) =>
        BinaryPrimitives.WriteUInt32LittleEndian(data.AsSpan(offset, sizeof(uint)), value);

    private static void WriteInt32(byte[] data, int offset, int value) =>
        BinaryPrimitives.WriteInt32LittleEndian(data.AsSpan(offset, sizeof(int)), value);

    public static bool RequiresPngFallback(
        bool hasImage,
        bool hasText,
        int fileCount,
        IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats)
    {
        if (!hasImage || hasText || fileCount > 0)
        {
            return false;
        }

        return !nativeFormats.Any(static format =>
            string.Equals(format.FormatName, PngFormatName, StringComparison.OrdinalIgnoreCase));
    }

    public static bool RequiresFileDropFallback(
        bool hasImage,
        bool hasText,
        int fileCount,
        IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats)
    {
        if (!hasImage || hasText || fileCount > 0)
        {
            return false;
        }

        return !nativeFormats.Any(static format => format.FormatId == FileDropFormatId);
    }

    public static bool RequiresCanonicalRewrite(
        bool hasImage,
        bool hasText,
        int fileCount,
        IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats)
    {
        if (!hasImage || hasText || fileCount > 0)
        {
            return false;
        }

        return nativeFormats.Any(static format =>
            IsPrivateImageFormat(format) || IsImageMetadataFormat(format));
    }

    private static bool IsPrivateImageFormat(CapturedClipboardNativeFormat format) =>
        format.FormatName.Equals("DataObject", StringComparison.OrdinalIgnoreCase) ||
        format.FormatName.Equals("PixPinData", StringComparison.OrdinalIgnoreCase) ||
        format.FormatName.Equals("Ole Private Data", StringComparison.OrdinalIgnoreCase);

    private static bool IsImageMetadataFormat(CapturedClipboardNativeFormat format)
    {
        var name = format.FormatName;
        return name.Equals("HTML Format", StringComparison.OrdinalIgnoreCase)
            || name.Equals("text/html", StringComparison.OrdinalIgnoreCase)
            || name.Equals("UniformResourceLocator", StringComparison.OrdinalIgnoreCase)
            || name.Equals("UniformResourceLocatorW", StringComparison.OrdinalIgnoreCase)
            || name.Equals("text/uri-list", StringComparison.OrdinalIgnoreCase)
            || name.Equals("text/x-moz-url", StringComparison.OrdinalIgnoreCase)
            || name.StartsWith("Chromium internal source ", StringComparison.OrdinalIgnoreCase)
            || name.Equals("Chromium Web Custom MIME Data Format", StringComparison.OrdinalIgnoreCase);
    }

    public static void AddPngFallback(
        List<CapturedClipboardNativeFormat> nativeFormats,
        uint pngFormatId,
        byte[] pngData)
    {
        for (var index = 0; index < nativeFormats.Count; index++)
        {
            nativeFormats[index] = nativeFormats[index] with
            {
                SortOrder = checked(nativeFormats[index].SortOrder + 1)
            };
        }
        nativeFormats.Insert(0, new CapturedClipboardNativeFormat(
            pngFormatId, PngFormatName, pngData, 0));
    }

    public static void AddCompatibilityFallbacks(
        List<CapturedClipboardNativeFormat> nativeFormats,
        uint pngFormatId,
        byte[] pngData,
        string imagePath)
    {
        if (!nativeFormats.Any(static format =>
                string.Equals(format.FormatName, PngFormatName, StringComparison.OrdinalIgnoreCase)))
        {
            AddPngFallback(nativeFormats, pngFormatId, pngData);
        }
        if (nativeFormats.Any(static format => format.FormatId == FileDropFormatId))
        {
            return;
        }

        nativeFormats.Add(new CapturedClipboardNativeFormat(
            FileDropFormatId,
            string.Empty,
            ClipboardFileDropPayload.Encode([imagePath]),
            checked((uint)nativeFormats.Count)));
    }

    public static void ReplaceWithCanonicalImageFormats(
        List<CapturedClipboardNativeFormat> nativeFormats,
        uint pngFormatId,
        byte[] pngData,
        string imagePath,
        byte[]? dibV5Data,
        byte[]? dibData)
    {
        var canonical = new List<CapturedClipboardNativeFormat>(capacity: nativeFormats.Count + 4);
        if (dibV5Data is { Length: > 0 })
        {
            canonical.Add(new CapturedClipboardNativeFormat(
                DibV5FormatId,
                string.Empty,
                dibV5Data,
                checked((uint)canonical.Count)));
        }
        if (dibData is { Length: > 0 })
        {
            canonical.Add(new CapturedClipboardNativeFormat(
                DibFormatId,
                string.Empty,
                dibData,
                checked((uint)canonical.Count)));
        }
        if (pngData.Length > 0)
        {
            canonical.Add(new CapturedClipboardNativeFormat(
                pngFormatId,
                PngFormatName,
                pngData,
                checked((uint)canonical.Count)));
        }
        if (!string.IsNullOrWhiteSpace(imagePath))
        {
            canonical.Add(new CapturedClipboardNativeFormat(
                FileDropFormatId,
                string.Empty,
                ClipboardFileDropPayload.Encode([Path.GetFullPath(imagePath)]),
                checked((uint)canonical.Count)));
        }
        foreach (var format in nativeFormats.Where(static format =>
            !IsPrivateImageFormat(format) && !IsImageMetadataFormat(format)))
        {
            if (format.FormatId is DibFormatId or DibV5FormatId or FileDropFormatId ||
                string.Equals(format.FormatName, PngFormatName, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }
            canonical.Add(format with { SortOrder = checked((uint)canonical.Count) });
        }

        nativeFormats.Clear();
        nativeFormats.AddRange(canonical);
    }
}
