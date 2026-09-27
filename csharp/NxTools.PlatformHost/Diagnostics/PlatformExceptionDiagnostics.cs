using System.Windows.Forms;

namespace NxTools.PlatformHost.Diagnostics;

internal static class PlatformExceptionDiagnostics
{
    public static void Register(PlatformLog log, CancellationTokenSource shutdown)
    {
        AppDomain.CurrentDomain.UnhandledException += (_, args) =>
        {
            if (args.ExceptionObject is Exception exception)
            {
                Record(log, "process.unhandled_exception", exception,
                    new Dictionary<string, object?>
                    {
                        ["is_terminating"] = args.IsTerminating
                    });
            }
            else
            {
                SafeWrite(log, "critical", "process.unhandled_exception", new Dictionary<string, object?>
                {
                    ["exception_type"] = args.ExceptionObject?.GetType().FullName ?? "unknown",
                    ["is_terminating"] = args.IsTerminating
                });
            }
        };
        TaskScheduler.UnobservedTaskException += (_, args) =>
        {
            Record(log, "process.unobserved_task_exception", args.Exception);
            args.SetObserved();
        };
        Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
        Application.ThreadException += (_, args) =>
        {
            Record(log, "process.ui_thread_exception", args.Exception);
            shutdown.Cancel();
        };
    }

    public static void Record(
        IPlatformLog log,
        string eventName,
        Exception exception,
        IReadOnlyDictionary<string, object?>? extraFields = null)
    {
        var fields = new Dictionary<string, object?>
        {
            ["exception_type"] = exception.GetType().FullName ?? exception.GetType().Name,
            ["base_exception_type"] = exception.GetBaseException().GetType().FullName,
            ["hresult"] = $"0x{exception.HResult:X8}",
            ["stack_trace"] = exception.StackTrace
        };
        if (extraFields is not null)
        {
            foreach (var field in extraFields)
            {
                fields[field.Key] = field.Value;
            }
        }
        SafeWrite(log, "critical", eventName, fields);
    }

    private static void SafeWrite(
        IPlatformLog log,
        string level,
        string eventName,
        IReadOnlyDictionary<string, object?> fields)
    {
        try
        {
            log.Write(level, eventName, fields: fields);
        }
        catch
        {
            // Exception diagnostics must never create a second process failure.
        }
    }
}
