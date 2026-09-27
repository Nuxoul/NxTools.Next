using System.Diagnostics;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal delegate void NativeFileDragDiagnostic(
    string stage,
    IReadOnlyDictionary<string, object?>? fields = null);

internal delegate WindowsCommandResult NativeFileDragOperation(
    BeginFileDragCommand command,
    Guid correlationId,
    NativeFileDragDiagnostic reportDiagnostic);

internal sealed class NativeFileDragCoordinator
{
    private readonly IPlatformLog _log;
    private readonly NativeFileDragOperation _operation;
    private readonly object _activeGate = new();
    private Guid? _activeCorrelationId;
    private long _activeStartedTimestamp;

    public NativeFileDragCoordinator(
        IPlatformLog? log = null,
        NativeFileDragOperation? operation = null)
    {
        _log = log ?? NullPlatformLog.Instance;
        _operation = operation ?? ExecuteNativeDrag;
    }

    public Task<WindowsCommandResult> BeginAsync(
        BeginFileDragCommand command,
        Guid correlationId,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return Task.FromCanceled<WindowsCommandResult>(cancellationToken);
        }

        Guid? activeCorrelationId = null;
        double activeElapsedMilliseconds = 0;
        var startedTimestamp = Stopwatch.GetTimestamp();
        lock (_activeGate)
        {
            if (_activeCorrelationId is Guid active)
            {
                activeCorrelationId = active;
                activeElapsedMilliseconds = Stopwatch
                    .GetElapsedTime(_activeStartedTimestamp)
                    .TotalMilliseconds;
            }
            else
            {
                _activeCorrelationId = correlationId;
                _activeStartedTimestamp = startedTimestamp;
            }
        }
        if (activeCorrelationId is not null)
        {
            WriteStage(
                command,
                correlationId,
                "request_rejected_active",
                new Dictionary<string, object?>
                {
                    ["active_correlation_id"] = activeCorrelationId.Value,
                    ["active_elapsed_ms"] = activeElapsedMilliseconds
                });
            return Task.FromResult(new WindowsCommandResult(
                false,
                "A native file drag operation is already active."));
        }

        var completion = new TaskCompletionSource<WindowsCommandResult>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var thread = new Thread(() =>
        {
            try
            {
                WriteStage(
                    command,
                    correlationId,
                    "worker_started",
                    new Dictionary<string, object?>
                    {
                        ["managed_thread_id"] = Environment.CurrentManagedThreadId,
                        ["apartment_state"] = Thread.CurrentThread.GetApartmentState().ToString(),
                        ["is_thread_pool_thread"] = Thread.CurrentThread.IsThreadPoolThread
                    });
                if (cancellationToken.IsCancellationRequested)
                {
                    WriteStage(command, correlationId, "worker_cancelled_before_operation");
                    completion.TrySetCanceled(cancellationToken);
                    return;
                }
                completion.TrySetResult(_operation(
                    command,
                    correlationId,
                    (stage, fields) =>
                        WriteStage(command, correlationId, stage, fields)));
            }
            catch (Exception error)
            {
                WriteStage(
                    command,
                    correlationId,
                    "worker_faulted",
                    new Dictionary<string, object?>
                    {
                        ["exception_type"] = error.GetType().FullName,
                        ["exception_message"] = error.Message
                    });
                completion.TrySetResult(new WindowsCommandResult(false, error.Message));
            }
            finally
            {
                NativeFileDragService.ReleaseOleForCurrentThread();
                lock (_activeGate)
                {
                    if (_activeCorrelationId == correlationId)
                    {
                        _activeCorrelationId = null;
                        _activeStartedTimestamp = 0;
                    }
                }
                WriteStage(
                    command,
                    correlationId,
                    "worker_exited",
                    new Dictionary<string, object?>
                    {
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(startedTimestamp)
                            .TotalMilliseconds
                    });
            }
        })
        {
            IsBackground = true,
            Name = "NxTools.NativeFileDrag"
        };
        thread.SetApartmentState(ApartmentState.STA);
        try
        {
            thread.Start();
        }
        catch (Exception error)
        {
            lock (_activeGate)
            {
                if (_activeCorrelationId == correlationId)
                {
                    _activeCorrelationId = null;
                    _activeStartedTimestamp = 0;
                }
            }
            WriteStage(
                command,
                correlationId,
                "worker_start_failed",
                new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["exception_message"] = error.Message
                });
            completion.TrySetResult(new WindowsCommandResult(false, error.Message));
        }
        return completion.Task;
    }

    private void WriteStage(
        BeginFileDragCommand command,
        Guid correlationId,
        string stage,
        IReadOnlyDictionary<string, object?>? details = null)
    {
        try
        {
            var fields = new Dictionary<string, object?>
            {
                ["stage"] = stage,
                ["source_window_handle"] = command.SourceWindowHandle,
            };
            if (details is not null)
            {
                foreach (var detail in details)
                {
                    fields[detail.Key] = detail.Value;
                }
            }
            _log.Write(
                "info",
                "file_drag.stage",
                correlationId,
                fields);
        }
        catch
        {
            // Diagnostics must not terminate the native drag worker.
        }
    }

    private static WindowsCommandResult ExecuteNativeDrag(
        BeginFileDragCommand command,
        Guid correlationId,
        NativeFileDragDiagnostic reportDiagnostic)
    {
        _ = correlationId;
        var path = Path.GetFullPath(command.Path);
        if (!File.Exists(path))
        {
            reportDiagnostic("source_file_missing");
            return new WindowsCommandResult(
                false,
                "The completed download file no longer exists.");
        }
        var dragged = NativeFileDragService.TryDragFile(
            path,
            command.SourceWindowHandle,
            out var effect,
            out var error,
            reportDiagnostic);
        reportDiagnostic(
            "operation_completed",
            new Dictionary<string, object?>
            {
                ["success"] = dragged,
                ["drop_effect"] = effect,
                ["error"] = error
            });
        return new WindowsCommandResult(
            dragged,
            dragged ? null : error);
    }
}
