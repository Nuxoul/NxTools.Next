using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed partial class WindowsPlatformAdapter
{
    internal sealed partial class EventWindow
    {
        internal const int MaxPendingClipboardPauseRequests = 32;
        private readonly ClipboardCapturePause _clipboardPause = new();
        private readonly object _clipboardPauseSync = new();
        private readonly LinkedList<ClipboardPauseRequest> _clipboardPauseRequests = new();
        private readonly ClipboardCaptureOperations _clipboardCapture;
        private bool _clipboardCaptureStopped;
        private bool _clipboardCaptureCleaned;
        private bool _clipboardRetryArmed;

        private sealed class ClipboardPauseRequest(bool paused, Guid correlationId, CancellationToken cancellation)
        {
            internal readonly bool Paused = paused;
            internal readonly Guid CorrelationId = correlationId;
            internal readonly CancellationToken Cancellation = cancellation;
            internal readonly TaskCompletionSource<WindowsCommandResult> Completion = new(TaskCreationOptions.RunContinuationsAsynchronously);
            internal readonly long StartedAt = Stopwatch.GetTimestamp();
            internal LinkedListNode<ClipboardPauseRequest>? Node;
            internal CancellationTokenRegistration Registration;
        }

        private ClipboardCaptureOperations CreateClipboardCaptureOperations() => new(
            () =>
            {
                if (NativeMethods.PostMessage(Handle, WmClipboardPause, IntPtr.Zero, IntPtr.Zero)) return 0;
                var errorCode = Marshal.GetLastWin32Error();
                return errorCode != 0 ? errorCode : 31;
            },
            NativeMethods.GetClipboardSequenceNumber,
            () => _foregroundContextProvider.Capture(),
            ReadClipboardContent,
            () => { _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId); },
            () =>
            {
                if (NativeMethods.SetTimer(Handle, (UIntPtr)RetryTimerId, 40, IntPtr.Zero) == UIntPtr.Zero)
                    throw new Win32Exception(Marshal.GetLastWin32Error());
            });

        // No HWND/hooks/listener/COM/global input. Only the OS operations are substituted;
        // WndProc, admission, application, read guard, retry and shutdown are production code.
        internal EventWindow(ChannelWriter<WindowsPlatformEvent> events, IPlatformLog log, ClipboardCaptureOperations operations,
            ClipboardContentOperations? content = null, ClipboardStagingDirectory? staging = null, ClipboardCaptureLimits? limits = null,
            long? publicationByteLimit = null)
        {
            _events = events;
            _log = log;
            _foregroundContextProvider = new ForegroundContextProvider();
            _elevatedDirectoryNavigation = null!;
            _clipboardCaptureDropDiagnosticGate = new();
            _foregroundCallback = OnForegroundChanged;
            _fileDialogWindowCallback = OnFileDialogWindowChanged;
            _deferExplorerDelivery = _ => throw new InvalidOperationException("Clipboard fixture has no Explorer delivery.");
            _clipboardCapture = operations;
            if (content is not null) _clipboardContent = content;
            if (staging is not null) _clipboardStaging = new(() => staging);
            if (limits is { } captureLimits) _captureLimits = captureLimits;
            if (publicationByteLimit is { } byteLimit) _clipboardPublicationByteLimit = byteLimit;
        }

        internal (bool Paused, bool HasPendingSource, int Attempts, int PendingCommands) ClipboardCaptureState
        {
            get
            {
                lock (_clipboardPauseSync)
                    return (_clipboardPause.IsPaused, _pendingClipboardSource is not null, _clipboardReadAttempts, _clipboardPauseRequests.Count);
            }
        }

        internal void DispatchClipboardMessage(int messageId, int wParam = 0)
        {
            if (messageId != WmClipboardPause && messageId != WmClipboardUpdate &&
                !(messageId == WmTimer && wParam == RetryTimerId))
                throw new ArgumentOutOfRangeException(nameof(messageId));
            var message = Message.Create(IntPtr.Zero, messageId, new IntPtr(wParam), IntPtr.Zero);
            WndProc(ref message);
        }

        public Task<WindowsCommandResult> EnqueueClipboardPause(SetClipboardCapturePausedCommand command,
            Guid correlationId, CancellationToken cancellation)
        {
            var request = new ClipboardPauseRequest(command.Paused, correlationId, cancellation);
            // PostMessage does not invoke the owner synchronously. Holding this short lock ensures
            // a failed post/cancel cannot race the owner into applying an unadmitted request.
            lock (_clipboardPauseSync)
            {
                if (cancellation.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(cancellation);
                    WriteClipboardPauseDiagnostic("cancelled", request);
                }
                else if (_clipboardCaptureStopped || _clipboardPauseRequests.Count >= MaxPendingClipboardPauseRequests)
                {
                    request.Completion.TrySetResult(new(false, _clipboardCaptureStopped
                        ? "Windows adapter is stopping." : "Clipboard capture pause queue is full."));
                    WriteClipboardPauseDiagnostic("rejected", request);
                }
                else
                {
                    request.Node = _clipboardPauseRequests.AddLast(request);
                    WriteClipboardPauseDiagnostic("accepted", request);
                    try
                    {
                        var registration = cancellation.Register(() => CancelClipboardPauseRequest(request));
                        request.Registration = registration;
                        if (request.Node is null)
                        {
                            registration.Unregister();
                            return request.Completion.Task;
                        }
                        var errorCode = _clipboardCapture.PostPause();
                        if (errorCode != 0)
                            CompleteClipboardPauseRequest(request,
                                new(false, "Could not schedule clipboard capture pause.", NativeErrorCode: errorCode), "failed");
                    }
                    catch (Exception error)
                    {
                        CompleteClipboardPauseRequest(request,
                            new(false, "Could not schedule clipboard capture pause.", NativeErrorCode: ClipboardPauseNativeCode(error)), "failed");
                    }
                }
            }
            return request.Completion.Task;
        }

        private void RemoveClipboardPauseRequest(ClipboardPauseRequest request)
        {
            if (request.Node is not null) _clipboardPauseRequests.Remove(request.Node);
            request.Node = null;
            // Dispose could wait for a callback blocked on this lock. Unregister never waits.
            request.Registration.Unregister();
        }

        private void CompleteClipboardPauseRequest(ClipboardPauseRequest request, WindowsCommandResult result, string terminal)
        {
            RemoveClipboardPauseRequest(request);
            WriteClipboardPauseDiagnostic(terminal, request, result.NativeErrorCode);
            request.Completion.TrySetResult(result);
        }

        private void CancelClipboardPauseRequest(ClipboardPauseRequest request)
        {
            lock (_clipboardPauseSync)
            {
                if (request.Node is null) return; // Applying/terminal already owns the outcome.
                RemoveClipboardPauseRequest(request);
                WriteClipboardPauseDiagnostic("cancelled", request);
                request.Completion.TrySetCanceled(request.Cancellation);
            }
        }

        private void DrainClipboardPauseRequests()
        {
            // Bound each owner turn even if other producers keep submitting commands.
            for (var i = 0; i < MaxPendingClipboardPauseRequests; i++)
            {
                lock (_clipboardPauseSync)
                {
                    if (_clipboardCaptureStopped || _clipboardPauseRequests.First is not { } node) return;
                    var request = node.Value;
                    if (request.Cancellation.IsCancellationRequested)
                    {
                        CancelClipboardPauseRequest(request);
                        continue;
                    }
                    RemoveClipboardPauseRequest(request); // Application wins later cancellation.
                    WriteClipboardPauseDiagnostic("start", request);
                    try
                    {
                        var wasPaused = _clipboardPause.IsPaused;
                        _clipboardPause.Close();
                        ClearPendingClipboardRead();
                        // Resume samples metadata, never content. Discard queued updates for this
                        // sequence; only a later clipboard change may start another read.
                        var resumeSequence = !request.Paused && wasPaused ? _clipboardCapture.Sequence() : (uint?)null;
                        if (resumeSequence == 0)
                            throw new InvalidOperationException("Clipboard resume sequence is unavailable.");
                        var result = _clipboardPause.Apply(request.Paused, resumeSequence);
                        CompleteClipboardPauseRequest(request, result, "applied");
                    }
                    catch (Exception error)
                    {
                        _clipboardPause.Close();
                        CompleteClipboardPauseRequest(request,
                            new(false, "Could not apply clipboard capture pause.", NativeErrorCode: ClipboardPauseNativeCode(error)), "failed");
                    }
                }
            }
        }

        // Safe on the disposing/pipe thread; no Windows operations or owner-state mutation.
        // Also called before posting WM_CLOSE, so a failed close post cannot strand command tasks.
        internal void CancelClipboardPauseRequests()
        {
            lock (_clipboardPauseSync)
            {
                Volatile.Write(ref _clipboardCaptureStopped, true);
                while (_clipboardPauseRequests.First is { } node)
                    CompleteClipboardPauseRequest(node.Value, new(false, "Windows adapter is stopping."), "stopped");
            }
        }

        // Event-owner shutdown, also used when the message loop exits abnormally.
        internal void StopClipboardCapture()
        {
            CancelClipboardPauseRequests();
            if (_clipboardCaptureCleaned) return;
            _clipboardCaptureCleaned = true;
            _clipboardPause.Close();
            try { ClearPendingClipboardRead(); }
            catch (Exception error)
            {
                WriteClipboardCaptureFailure("clipboard.capture_stop_failed", error);
            }
            if (_clipboardStaging.IsValueCreated)
            {
                var staging = _clipboardStaging.Value;
                _ = _capturePublishTail.ContinueWith(_ => staging.Dispose(), CancellationToken.None,
                    TaskContinuationOptions.ExecuteSynchronously, TaskScheduler.Default);
            }
        }

        private void ClearPendingClipboardRead()
        {
            _pendingClipboardSource = null;
            _clipboardReadAttempts = 0;
            _clipboardRetryArmed = false;
            _clipboardCapture.StopRetry();
        }

        private void HandleClipboardUpdate()
        {
            if (_suppressNextClipboardUpdate)
            {
                _suppressNextClipboardUpdate = false;
                return;
            }
            if (Volatile.Read(ref _clipboardCaptureStopped) || !_clipboardPause.MayRead) return;
            try { ClearPendingClipboardRead(); }
            catch (Exception error)
            {
                WriteClipboardCaptureFailure("clipboard.capture_failed", error);
                return;
            }
            TryPublishClipboardContent();
        }

        private void HandleClipboardRetry()
        {
            if (Volatile.Read(ref _clipboardCaptureStopped) || !_clipboardPause.MayRead || !_clipboardRetryArmed) return;
            _clipboardRetryArmed = false;
            try { _clipboardCapture.StopRetry(); }
            catch (Exception error)
            {
                _pendingClipboardSource = null;
                _clipboardReadAttempts = 0;
                WriteClipboardCaptureFailure("clipboard.capture_failed", error);
                return;
            }
            TryPublishClipboardContent();
        }

        private void TryPublishClipboardContent()
        {
            // BEFORE sequence/native formats/Contains/Get/source sampling, including retries.
            if (Volatile.Read(ref _clipboardCaptureStopped) || !_clipboardPause.MayRead) return;
            var retry = false;
            try
            {
                var sequence = _clipboardCapture.Sequence();
                if (!_clipboardPause.MayReadSequence(sequence)) return;
                var source = _pendingClipboardSource ?? _clipboardCapture.Source();
                _pendingClipboardSource = source;
                if (_captureExclusions.Contains(source.ProcessName))
                {
                    try
                    {
                        _log.Write("info", "clipboard.capture_excluded", null,
                            new Dictionary<string, object?> { ["source_process_name"] = source.ProcessName });
                    }
                    catch { /* Diagnostics cannot change exclusion. */ }
                    return;
                }
                _clipboardCapture.Read(source, sequence);
            }
            catch (ExternalException) when (++_clipboardReadAttempts < MaxClipboardReadAttempts)
            {
                try
                {
                    _clipboardCapture.StartRetry();
                    retry = _clipboardRetryArmed = true;
                }
                catch (Exception error) { WriteClipboardCaptureFailure("clipboard.capture_failed", error); }
            }
            catch (Exception error) { WriteClipboardCaptureFailure("clipboard.capture_failed", error); }
            finally
            {
                if (!retry)
                {
                    _pendingClipboardSource = null;
                    _clipboardReadAttempts = 0;
                    _clipboardRetryArmed = false;
                }
            }
        }

        private static int? ClipboardPauseNativeCode(Exception error) => error is Win32Exception native ? native.NativeErrorCode : null;

        private void WriteClipboardCaptureFailure(string name, Exception error)
        {
            try
            {
                _log.Write("error", name, null, new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["hresult"] = $"0x{error.HResult:X8}",
                    ["native_error_code"] = ClipboardPauseNativeCode(error)
                });
            }
            catch { /* Diagnostics cannot change capture state. */ }
        }

        private void WriteClipboardPauseDiagnostic(string stage, ClipboardPauseRequest request, int? nativeCode = null)
        {
            try
            {
                _log.Write(stage is "failed" or "rejected" ? "error" : "info", "clipboard.capture_pause." + stage,
                    request.CorrelationId, new Dictionary<string, object?>
                    {
                        ["paused"] = request.Paused,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(request.StartedAt).TotalMilliseconds,
                        ["native_error_code"] = nativeCode
                    });
            }
            catch { /* Observational diagnostics cannot change admission/application/results. */ }
        }
    }
}
