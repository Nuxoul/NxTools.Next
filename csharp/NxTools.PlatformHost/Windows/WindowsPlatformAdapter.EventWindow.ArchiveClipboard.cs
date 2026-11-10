using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal sealed partial class EventWindow
    {
        private readonly ConcurrentQueue<(ArchiveClipboardCommand Command, CancellationToken Cancellation,
            TaskCompletionSource<WindowsCommandResult> Completion, long Started)> _fileDropReads = new();
        private int _fileDropReadCount;
        private int _fileDropReadsStopped;

        internal Task<WindowsCommandResult> EnqueueFileDropRead(ArchiveClipboardCommand command, CancellationToken cancellation)
        {
            if (cancellation.IsCancellationRequested || Volatile.Read(ref _fileDropReadsStopped) != 0)
                return Task.FromResult(FileDropResult(command, "cancelled"));
            if (Interlocked.Increment(ref _fileDropReadCount) > 64)
            {
                Interlocked.Decrement(ref _fileDropReadCount);
                return Task.FromResult(FileDropResult(command, "clipboard_unavailable"));
            }
            var completion = new TaskCompletionSource<WindowsCommandResult>(TaskCreationOptions.RunContinuationsAsynchronously);
            var started = Stopwatch.GetTimestamp();
            _fileDropReads.Enqueue((command, cancellation, completion, started));
            FileDropDiagnostic(command, "accepted", null, null, started);
            if (!NativeMethods.PostMessage(Handle, WmReadClipboardFileDrop, IntPtr.Zero, IntPtr.Zero))
            {
                var native = System.Runtime.InteropServices.Marshal.GetLastWin32Error();
                var result = FileDropResult(command, "clipboard_failed", native);
                completion.TrySetResult(result);
                FileDropDiagnostic(command, "failed", "clipboard_failed", native, started);
            }
            return completion.Task;
        }
        private void DrainFileDropReads()
        {
            while (_fileDropReads.TryDequeue(out var request))
            {
                Interlocked.Decrement(ref _fileDropReadCount);
                if (request.Completion.Task.IsCompleted) continue;
                var (command, cancellation, completion, started) = request;
                if (cancellation.IsCancellationRequested || Volatile.Read(ref _fileDropReadsStopped) != 0)
                {
                    completion.TrySetResult(FileDropResult(command, "cancelled"));
                    FileDropDiagnostic(command, "failed", "cancelled", null, started);
                    continue;
                }
                FileDropDiagnostic(command, "started", null, null, started);
                try
                {
                    var paths = ClipboardFileDropReader.ReadClipboard(Handle);
                    var result = cancellation.IsCancellationRequested || Volatile.Read(ref _fileDropReadsStopped) != 0 ? FileDropResult(command, "cancelled") :
                        new WindowsCommandResult(true, Data: JsonSerializer.Serialize(new ArchiveClipboardResult(1,
                            command.OperationId, command.SessionId, command.Generation, paths), JsonDefaults.Options));
                    completion.TrySetResult(result);
                    FileDropDiagnostic(command, result.Success ? "finished" : "failed", result.Error, result.NativeErrorCode, started);
                }
                catch (ClipboardFileDropException failure)
                {
                    completion.TrySetResult(FileDropResult(command, failure.Code, failure.NativeCode));
                    FileDropDiagnostic(command, "failed", failure.Code, failure.NativeCode, started);
                }
                catch
                {
                    completion.TrySetResult(FileDropResult(command, "clipboard_failed"));
                    FileDropDiagnostic(command, "failed", "clipboard_failed", null, started);
                }
            }
        }
        private void StopFileDropReads()
        {
            Volatile.Write(ref _fileDropReadsStopped, 1);
            while (_fileDropReads.TryDequeue(out var request))
            {
                Interlocked.Decrement(ref _fileDropReadCount);
                if (request.Completion.TrySetResult(FileDropResult(request.Command, "cancelled")))
                    FileDropDiagnostic(request.Command, "failed", "cancelled", null, request.Started);
            }
        }
        private static WindowsCommandResult FileDropResult(ArchiveClipboardCommand command, string code, int? native = null) =>
            new(false, code, NativeErrorCode: native, Data: JsonSerializer.Serialize(new ArchiveClipboardResult(1,
                command.OperationId, command.SessionId, command.Generation, [], code, native), JsonDefaults.Options));
        private void FileDropDiagnostic(ArchiveClipboardCommand command, string phase, string? code, int? native, long started)
        {
            try { _log.Write("info", "archive.clipboard." + phase, command.OperationId,
                new Dictionary<string, object?> { ["session_id"] = command.SessionId, ["generation"] = command.Generation,
                    ["code"] = code, ["native_error_code"] = native, ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds }); }
            catch { /* Diagnostics cannot change the native result. */ }
        }
    }
}
