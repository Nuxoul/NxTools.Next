using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardTextReader
{
    // Even JSON's six-byte escaping fits inside the existing one-MiB result frame.
    internal const int MaximumTextLength = 64 * 1024;

    internal static WindowsCommandResult ReadOnSta(CancellationToken cancellation, Func<string>? readText = null)
    {
        if (cancellation.IsCancellationRequested) return new(false, "cancelled");
        return BoundedStaOperationRunner.TryRun(
            "NxTools clipboard text", TimeSpan.FromSeconds(2),
            token => Read(token, readText), out WindowsCommandResult? result, out _, cancellation)
            && result is not null
            ? result
            : new(false, cancellation.IsCancellationRequested ? "cancelled" : "clipboard_unavailable");
    }

    private static WindowsCommandResult Read(CancellationToken cancellation, Func<string>? readText)
    {
        if (cancellation.IsCancellationRequested) return new(false, "cancelled");
        try
        {
            var text = readText is null ? Clipboard.GetText(TextDataFormat.UnicodeText) : readText();
            if (cancellation.IsCancellationRequested) return new(false, "cancelled");
            return text.Length <= MaximumTextLength
                ? new(true, Data: text)
                : new(false, "clipboard_text_too_large");
        }
        catch (ExternalException failure)
        {
            return new(false, "clipboard_unavailable", NativeErrorCode: failure.ErrorCode);
        }
        catch
        {
            return new(false, "clipboard_failed");
        }
    }
}

internal sealed partial class WindowsPlatformAdapter
{
    public Task<WindowsCommandResult> ReadClipboardTextAsync(CancellationToken cancellation)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return Task.FromResult(ClipboardTextReader.ReadOnSta(cancellation));
    }
}
