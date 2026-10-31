using System.ComponentModel;
using System.Diagnostics;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

/// <summary>One fixed Windows Settings destination. Requests cannot supply a URI or path.</summary>
internal sealed class DefaultAppsSettingsOpener(IPlatformLog log, Action? nativeOperation = null)
{
    internal WindowsCommandResult Open(OpenDefaultAppsSettingsCommand command, Guid correlationId, CancellationToken cancellation)
    {
        ArgumentNullException.ThrowIfNull(command);
        var started = Stopwatch.GetTimestamp();
        Write("accepted", correlationId, started);
        WindowsCommandResult result;
        try
        {
            cancellation.ThrowIfCancellationRequested();
            Write("started", correlationId, started);
            cancellation.ThrowIfCancellationRequested();
            (nativeOperation ?? OpenFixedTarget)();
            // Shell acceptance is all this narrow operation observes, not Settings UI readiness.
            result = new(true, Data: "opened");
        }
        catch (OperationCanceledException)
        {
            result = new(false, "Opening Windows default apps settings was cancelled.", Data: "cancelled");
        }
        catch (Win32Exception failure)
        {
            result = new(false, "Windows default apps settings could not be opened.", NativeErrorCode: failure.NativeErrorCode);
        }
        catch (Exception failure)
        {
            result = new(false, "Windows default apps settings could not be opened.", NativeErrorCode: failure.HResult);
        }
        Write("terminal", correlationId, started, result);
        return result;
    }

    private static void OpenFixedTarget()
    {
        using var launched = Process.Start(new ProcessStartInfo("ms-settings:defaultapps") { UseShellExecute = true });
    }

    private void Write(string stage, Guid correlationId, long started, WindowsCommandResult? result = null)
    {
        try
        {
            log.Write("info", "default_apps_settings." + stage, correlationId, new Dictionary<string, object?>
            {
                ["state"] = result is null ? stage : result.Success ? "opened" : result.Data == "cancelled" ? "cancelled" : "failed",
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                ["native_error_code"] = result?.NativeErrorCode,
            });
        }
        catch (Exception) { /* Observational diagnostics cannot affect launch or result. */ }
    }
}
