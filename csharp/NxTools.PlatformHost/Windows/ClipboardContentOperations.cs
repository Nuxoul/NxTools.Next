using System.Drawing;
using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

internal delegate string? ClipboardSourceIcon(uint processId, out string? error);

// Windows reads and fallible file writes; capture ordering/ownership stays in EventWindow.
internal sealed record ClipboardContentOperations(
    Func<IntPtr, IReadOnlyList<CapturedClipboardNativeFormat>> Formats,
    Func<string[]> Files,
    Func<Image?> Image,
    Func<string?> Text,
    Action<Image, string, ulong> SaveImage,
    Action<string, byte[]> WriteFormat,
    ClipboardSourceIcon SourceIcon)
{
    internal static ClipboardContentOperations Windows(ForegroundContextProvider foreground) => new(
        NativeClipboardFormats.Capture,
        () => Clipboard.ContainsFileDropList() ? Clipboard.GetFileDropList().Cast<string>()
            .Where(path => !string.IsNullOrWhiteSpace(path)).Take(1024).ToArray() : [],
        () => Clipboard.ContainsImage() ? Clipboard.GetImage() : null,
        () => Clipboard.ContainsText(TextDataFormat.UnicodeText) ? Clipboard.GetText(TextDataFormat.UnicodeText) : null,
        ClipboardImageEncoding.SavePng, File.WriteAllBytes, foreground.TryStageProcessIcon);
}
