using System.ComponentModel;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct NotificationWindowReadback(NativeWindowRectangle Client, uint Dpi,
    bool Visible, bool NoActivate, bool ToolWindow, bool Topmost);

// The same orchestration drives production Win32 calls and controlled work-area/DPI regressions.
internal interface INotificationWindowNative
{
    IDisposable EnterDpiScope();
    bool ValidateWindows(PrepareNotificationWindowCommand command);
    NativeWindowRectangle GetWorkArea(long source);
    void SetPassivePolicy(long target);
    void PrepareMonitor(long target, NativeWindowRectangle workArea);
    uint GetDpi(long target);
    void ApplyClientBounds(long target, NativeWindowRectangle clientBounds);
    NotificationWindowReadback Readback(long target);
}

internal static class NotificationWindowPlacement
{
    // This is a small bounded stack, never a full-screen input-catching overlay.
    internal const int MaximumLogicalWidth = 360;
    internal const int MaximumLogicalHeight = 400;

    internal static void Validate(PrepareNotificationWindowCommand command)
    {
        if (command.WindowHandle == 0 || command.SourceWindowHandle == 0 || command.SourceProcessId == 0 ||
            command.WindowHandle == command.SourceWindowHandle || command.Width is <= 0 or > MaximumLogicalWidth ||
            command.Height is <= 0 or > MaximumLogicalHeight || command.Position is not
                ("top_left" or "top_center" or "top_right" or "bottom_left" or "bottom_center" or "bottom_right"))
            throw new InvalidDataException("Invalid notification window parameters.");
    }

    internal static WindowsCommandResult Prepare(PrepareNotificationWindowCommand command,
        INotificationWindowNative native, CancellationToken cancellationToken)
    {
        try
        {
            Validate(command);
            cancellationToken.ThrowIfCancellationRequested();
            using var dpiScope = native.EnterDpiScope();
            if (!native.ValidateWindows(command)) return new(false, "Invalid notification window source or hidden target.");
            var work = native.GetWorkArea(command.SourceWindowHandle);
            if ((long)work.Right - work.Left is <= 0 or > 32768 || (long)work.Bottom - work.Top is <= 0 or > 32768)
                return new(false, "Invalid notification monitor work area.");
            cancellationToken.ThrowIfCancellationRequested();
            if (!command.KeepVisible)
            {
                native.SetPassivePolicy(command.WindowHandle);
                native.PrepareMonitor(command.WindowHandle, work);
            }
            cancellationToken.ThrowIfCancellationRequested();
            // GetDpiForWindow is meaningful only after the hidden target has moved to the source monitor.
            var dpi = native.GetDpi(command.WindowHandle);
            if (dpi is < 48 or > 960) return new(false, "Invalid notification target DPI.");
            var width = Math.Min(work.Width, (int)Math.Round(command.Width * dpi / 96.0, MidpointRounding.AwayFromZero));
            var height = Math.Min(work.Height, (int)Math.Round(command.Height * dpi / 96.0, MidpointRounding.AwayFromZero));
            // Rust already supplies 12 DIPs of container padding: the popup CLIENT itself touches the work-area edge.
            var x = command.Position.EndsWith("left", StringComparison.Ordinal) ? work.Left :
                command.Position.EndsWith("right", StringComparison.Ordinal) ? work.Right - width : work.Left + (work.Width - width) / 2;
            var y = command.Position.StartsWith("top", StringComparison.Ordinal) ? work.Top : work.Bottom - height;
            var requested = new NativeWindowRectangle(x, y, x + width, y + height);
            cancellationToken.ThrowIfCancellationRequested();
            if (!native.ValidateWindows(command)) return new(false, "Notification source or hidden target retired.");
            native.ApplyClientBounds(command.WindowHandle, requested);
            cancellationToken.ThrowIfCancellationRequested();
            if (!native.ValidateWindows(command)) return new(false, "Notification source or hidden target retired.");
            var actual = native.Readback(command.WindowHandle);
            if (actual.Client != requested || actual.Dpi != dpi || actual.Visible != command.KeepVisible || !actual.NoActivate || !actual.ToolWindow || !actual.Topmost)
                return new(false, "Notification client placement or passive policy readback mismatch.");
            return new(true, Data: JsonSerializer.Serialize(new { x = actual.Client.Left, y = actual.Client.Top,
                width = actual.Client.Width, height = actual.Client.Height, dpi = actual.Dpi }));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        { return new(false, "Notification preparation was cancelled."); }
        catch (Win32Exception error)
        { return new(false, "Notification native preparation failed.", NativeErrorCode: error.NativeErrorCode); }
        catch (InvalidDataException)
        { return new(false, "Invalid notification window parameters."); }
    }
}
