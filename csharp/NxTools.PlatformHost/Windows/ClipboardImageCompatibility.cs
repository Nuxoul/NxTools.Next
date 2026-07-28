namespace NxTools.PlatformHost.Windows;

internal static class ClipboardImageCompatibility
{
    private const string PngFormatName = "PNG";
    private const uint FileDropFormatId = 15;

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
}
