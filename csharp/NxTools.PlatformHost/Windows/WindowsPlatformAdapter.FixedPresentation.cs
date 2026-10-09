using System.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    public Task<WindowsCommandResult> PresentFixedWindowOverAnchorAsync(
        PresentFixedWindowOverAnchorCommand command, Guid correlationId, CancellationToken cancellationToken) =>
        Task.Run(() =>
        {
            var started = Stopwatch.StartNew();
            WriteFixedPresentationDiagnostic("download.fixed_window.started", correlationId, command, null, started);
            WindowsCommandResult result;
            var completed = BoundedStaOperationRunner.TryRun(
                "NxTools.FixedWindowPresentation", TimeSpan.FromSeconds(5), 2,
                token => NativeMethods.PresentFixedWindowOverAnchor(new IntPtr(command.WindowHandle),
                    new IntPtr(command.AnchorWindowHandle), command.Width, command.Height, token),
                out WindowsCommandResult? applied, out _, cancellationToken);
            result = completed && applied is not null
                ? applied : new WindowsCommandResult(false, "Fixed window presentation was cancelled, unavailable or timed out.");
            WriteFixedPresentationDiagnostic(result.Success ? "download.fixed_window.completed" : "download.fixed_window.failed",
                correlationId, command, result, started);
            return result;
        }, CancellationToken.None);

    internal static WindowsCommandResult PresentFixedWindowOverAnchorForTest(
        long window, long anchor, int width, int height, CancellationToken cancellationToken = default) =>
        NativeMethods.PresentFixedWindowOverAnchor(new IntPtr(window), new IntPtr(anchor), width, height, cancellationToken);

    private void WriteFixedPresentationDiagnostic(string eventName, Guid correlationId,
        PresentFixedWindowOverAnchorCommand command, WindowsCommandResult? result, Stopwatch started)
    {
        try
        {
            _log.Write(result is { Success: false } ? "warning" : "info", eventName, correlationId,
                new Dictionary<string, object?>
                {
                    ["window_handle"] = command.WindowHandle,
                    ["anchor_window_handle"] = command.AnchorWindowHandle,
                    ["target_client_width"] = command.Width,
                    ["target_client_height"] = command.Height,
                    ["success"] = result?.Success,
                    ["native_error_code"] = result?.NativeErrorCode,
                    ["elapsed_ms"] = started.ElapsedMilliseconds
                });
        }
        catch { /* Bounded diagnostics never change presentation results. */ }
    }
}
