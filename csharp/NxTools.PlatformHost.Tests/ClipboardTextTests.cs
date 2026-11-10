using System.Runtime.InteropServices;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ClipboardTextTests
{
    internal static Task ReadOnlyStaAndResults()
    {
        var reads = 0;
        var result = ClipboardTextReader.ReadOnSta(CancellationToken.None, () =>
        {
            reads++;
            if (Thread.CurrentThread.GetApartmentState() != ApartmentState.STA)
                throw new InvalidOperationException("Clipboard text must be read on the bounded STA.");
            return "https://synthetic.invalid/file?token=fixture";
        });
        Check(result.Success && result.Data == "https://synthetic.invalid/file?token=fixture" && reads == 1,
            "Original text must cross the native result without probing or rewriting.");
        var empty = ClipboardTextReader.ReadOnSta(CancellationToken.None, () => string.Empty);
        Check(empty.Success && empty.Data == string.Empty, "No text is an empty successful read.");
        var unavailable = ClipboardTextReader.ReadOnSta(CancellationToken.None,
            () => throw new ExternalException("private clipboard payload", unchecked((int)0x800401D0)));
        Check(!unavailable.Success && unavailable.Error == "clipboard_unavailable"
            && unavailable.NativeErrorCode == unchecked((int)0x800401D0) && unavailable.Data is null,
            "Native failure must retain HRESULT without payload or raw exception text.");
        return Task.CompletedTask;
    }

    internal static Task CancellationAndFrameBound()
    {
        var reads = 0;
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var cancelled = ClipboardTextReader.ReadOnSta(cancellation.Token, () => { reads++; return "unused"; });
        Check(!cancelled.Success && cancelled.Error == "cancelled" && reads == 0,
            "Precancelled read must never invoke native clipboard access.");
        var oversized = ClipboardTextReader.ReadOnSta(CancellationToken.None,
            () => new string('界', ClipboardTextReader.MaximumTextLength + 1));
        Check(!oversized.Success && oversized.Error == "clipboard_text_too_large" && oversized.Data is null,
            "Oversized clipboard text must not enter the one-MiB result frame.");
        var maximum = ClipboardTextReader.ReadOnSta(CancellationToken.None,
            () => new string('界', ClipboardTextReader.MaximumTextLength));
        Check(maximum.Success && maximum.Data?.Length == ClipboardTextReader.MaximumTextLength,
            "The declared text boundary remains accepted.");
        Check(PlatformCommandSequenceDispatcherIsDetached(), "Clipboard access must not block the command reader.");
        return Task.CompletedTask;
    }

    private static bool PlatformCommandSequenceDispatcherIsDetached()
    {
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var dispatch = dispatcher.DispatchAsync("read_clipboard_text", async () =>
        {
            entered.TrySetResult();
            await release.Task;
        });
        try
        {
            return dispatch.IsCompletedSuccessfully && entered.Task.Wait(TimeSpan.FromSeconds(2));
        }
        finally
        {
            release.TrySetResult();
            dispatcher.DrainAsync().GetAwaiter().GetResult();
        }
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
