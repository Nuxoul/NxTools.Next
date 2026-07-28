using System.Text;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardFileDropPayload
{
    private const int DropFilesHeaderSize = 20;

    public static byte[] Encode(IReadOnlyList<string> paths)
    {
        if (paths.Count == 0 || paths.Any(string.IsNullOrWhiteSpace))
        {
            throw new ArgumentException("At least one non-empty file path is required.", nameof(paths));
        }

        var fileList = string.Join('\0', paths) + "\0\0";
        var encodedPaths = Encoding.Unicode.GetBytes(fileList);
        var payload = new byte[DropFilesHeaderSize + encodedPaths.Length];
        BitConverter.GetBytes((uint)DropFilesHeaderSize).CopyTo(payload, 0);
        BitConverter.GetBytes(1).CopyTo(payload, 16); // DROPFILES.fWide = TRUE
        encodedPaths.CopyTo(payload, DropFilesHeaderSize);
        return payload;
    }
}
