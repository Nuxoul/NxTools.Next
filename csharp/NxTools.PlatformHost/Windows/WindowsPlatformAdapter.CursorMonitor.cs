using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    public WindowsCommandResult QueryCursorMonitor(Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        WriteCursorMonitorDiagnostic("started", correlationId, started, null);
        var snapshot = NativeMethods.GetCursorLocation().Monitor;
        var result = snapshot is null
            ? new WindowsCommandResult(false, "The cursor monitor work area is unavailable.",
                NativeErrorCode: Marshal.GetLastPInvokeError())
            : new WindowsCommandResult(true, Data: JsonSerializer.Serialize(snapshot, JsonDefaults.Options));
        WriteCursorMonitorDiagnostic("completed", correlationId, started, result);
        return result;
    }

    private void WriteCursorMonitorDiagnostic(string stage, Guid correlationId, long started,
        WindowsCommandResult? result)
    {
        try
        {
            _log.Write(result?.Success == false ? "warning" : "info", "ui.cursor_monitor." + stage,
                correlationId, new Dictionary<string, object?>
                {
                    ["success"] = result?.Success,
                    ["native_error_code"] = result?.NativeErrorCode,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
        }
        catch { /* Diagnostics do not affect the sampled monitor. */ }
    }
}
