using System.ComponentModel;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal readonly record struct MainWindowReadback(NativeWindowRectangle Client, uint Dpi, bool Visible);
internal interface IMainWindowNative
{
    IDisposable EnterDpiScope();
    bool Validate(PrepareMainWindowCommand command);
    NativeWindowRectangle PrimaryWorkArea();
    void Stage(long window, NativeWindowRectangle work);
    uint Dpi(long window);
    void Apply(long window, NativeWindowRectangle client);
    MainWindowReadback Read(long window);
}
internal static class MainWindowPreparation
{
    internal static void Validate(PrepareMainWindowCommand command)
    {
        if (command.WindowHandle == 0 || command.SourceProcessId == 0 || command.Width is < 920 or > 4096 || command.Height is < 480 or > 4096)
            throw new InvalidDataException("Invalid main-window preparation.");
    }
    internal static WindowsCommandResult Prepare(PrepareMainWindowCommand command, IMainWindowNative native, CancellationToken cancellation)
    {
        try
        {
            Validate(command);
            cancellation.ThrowIfCancellationRequested();
            using var scope = native.EnterDpiScope();
            if (!native.Validate(command)) return new(false, "main_window_source_retired");
            var work = native.PrimaryWorkArea();
            if (work.Width <= 0 || work.Height <= 0) return new(false, "main_window_monitor_unavailable");
            native.Stage(command.WindowHandle, work);
            cancellation.ThrowIfCancellationRequested();
            if (!native.Validate(command)) return new(false, "main_window_source_retired");
            var dpi = native.Dpi(command.WindowHandle);
            if (dpi is < 48 or > 960) return new(false, "main_window_dpi_invalid");
            var width = checked((int)Math.Round(command.Width * dpi / 96.0, MidpointRounding.AwayFromZero));
            var height = checked((int)Math.Round(command.Height * dpi / 96.0, MidpointRounding.AwayFromZero));
            var left = work.Left + (work.Width - width) / 2;
            var top = work.Top + (work.Height - height) / 2;
            var target = new NativeWindowRectangle(left, top, checked(left + width), checked(top + height));
            cancellation.ThrowIfCancellationRequested();
            native.Apply(command.WindowHandle, target);
            cancellation.ThrowIfCancellationRequested();
            if (!native.Validate(command)) return new(false, "main_window_source_retired");
            var actual = native.Read(command.WindowHandle);
            if (actual.Visible || actual.Dpi != dpi || actual.Client != target)
                return new(false, "main_window_readback_mismatch");
            return new(true, Data: JsonSerializer.Serialize(new { x = actual.Client.Left, y = actual.Client.Top,
                width = actual.Client.Width, height = actual.Client.Height, dpi = actual.Dpi }));
        }
        catch (OperationCanceledException) { return new(false, "main_window_prepare_cancelled"); }
        catch (Exception failure) when (failure is Win32Exception or InvalidDataException or OverflowException)
        { return new(false, "main_window_prepare_failed", NativeErrorCode: failure is Win32Exception nativeError ? nativeError.NativeErrorCode : failure.HResult); }
    }
}
