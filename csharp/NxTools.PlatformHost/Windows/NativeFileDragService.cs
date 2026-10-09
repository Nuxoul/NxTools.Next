using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;
using ComDataObject = System.Runtime.InteropServices.ComTypes.IDataObject;

namespace NxTools.PlatformHost.Windows;

internal delegate bool AttachThreadInputOperation(
    uint attachThreadId,
    uint attachToThreadId,
    bool attach);

internal static class NativeFileDragService
{
    private const int Succeeded = 0;
    private const int SucceededFalse = 1;
    private const int DataSameFormatEtc = 0x00040130;
    private const int DragDropDrop = 0x00040100;
    private const int DragDropCancel = 0x00040101;
    private const int DragDropUseDefaultCursors = 0x00040102;
    private const int DropEffectCopy = 1;
    private const int LeftButtonMask = 0x0001;
    private const int VirtualKeyLeftButton = 0x01;
    private const int ClipboardFormatDropFiles = 15;
    private const int GlobalMemoryMoveable = 0x0002;
    private const int GlobalMemoryZeroInitialize = 0x0040;
    private const int OleAdviseNotSupported = unchecked((int)0x80040003);
    private const int DataViewFormatEtc = unchecked((int)0x80040064);
    private const int DataViewTymed = unchecked((int)0x80040069);
    private const int NotImplemented = unchecked((int)0x80004001);
    private const int RpcChangedMode = unchecked((int)0x80010106);
    private const uint WmCancelMode = 0x001F;
    private const uint SendMessageAbortIfHung = 0x0002;
    private const uint SourceCaptureReleaseTimeoutMilliseconds = 250;
    private const uint GetAncestorRoot = 2;
    private const int FirstWatchdogDelayMilliseconds = 1_000;
    private const int SecondWatchdogDelayMilliseconds = 4_000;

    private static readonly short PreferredDropEffectFormat =
        RegisterClipboardFormat("Preferred DropEffect");

    [ThreadStatic]
    private static bool _oleInitialized;

    public static bool TryDragFile(
        string path,
        long sourceWindowHandle,
        out int effect,
        out string? error,
        out string terminalStatus,
        NativeFileDragDiagnostic? reportDiagnostic = null)
    {
        effect = 0;
        error = null;
        terminalStatus = "failed";

        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
        {
            error = "The completed download file no longer exists.";
            ReportDiagnostic(reportDiagnostic, "source_file_missing");
            return false;
        }
        var sourceWindow = new IntPtr(sourceWindowHandle);
        if (sourceWindow == IntPtr.Zero || !IsWindow(sourceWindow))
        {
            error = "The completed download window is no longer available.";
            ReportDiagnostic(
                reportDiagnostic,
                "source_window_invalid",
                new Dictionary<string, object?>
                {
                    ["is_zero"] = sourceWindow == IntPtr.Zero,
                    ["is_window"] = sourceWindow != IntPtr.Zero && IsWindow(sourceWindow)
                });
            return false;
        }
        ReportDiagnostic(reportDiagnostic, "source_validated");
        if (!EnsureOleInitialized(out error))
        {
            ReportDiagnostic(
                reportDiagnostic,
                "ole_initialization_failed",
                new Dictionary<string, object?> { ["error"] = error });
            return false;
        }
        if (!TryAttachSourceInputQueue(
                sourceWindow,
                reportDiagnostic,
                out var inputQueueLease,
                out error))
        {
            return false;
        }
        using var inputQueueScope = inputQueueLease;
        ReportInputSnapshot(
            reportDiagnostic,
            "input_before_capture_release",
            sourceWindow);
        if (!TryReleaseSourcePointerCapture(sourceWindow, out error))
        {
            ReportDiagnostic(
                reportDiagnostic,
                "capture_release_failed",
                new Dictionary<string, object?> { ["error"] = error });
            return false;
        }
        ReportInputSnapshot(
            reportDiagnostic,
            "input_after_capture_release",
            sourceWindow);
        var leftButtonState = GetAsyncKeyState(VirtualKeyLeftButton);
        if ((leftButtonState & 0x8000) == 0)
        {
            terminalStatus = "cancelled";
            ReportDiagnostic(
                reportDiagnostic,
                "gesture_ended_before_ole",
                PointerStateFields(leftButtonState));
            return true;
        }

        var telemetry = new OleDragTelemetry(
            GetCurrentThreadId(),
            Environment.CurrentManagedThreadId);
        var dataObject = new FileDropDataObject(
            [Path.GetFullPath(path)],
            telemetry,
            reportDiagnostic);
        var dropSource = new FileDropSource(telemetry, reportDiagnostic);
        var startedTimestamp = Stopwatch.GetTimestamp();
        using var watchdogCancellation = new CancellationTokenSource();
        _ = ReportDragStillActiveAsync(
            sourceWindow,
            telemetry,
            startedTimestamp,
            reportDiagnostic,
            watchdogCancellation.Token);
        ReportInputSnapshot(
            reportDiagnostic,
            "ole_entered",
            sourceWindow,
            telemetry.Snapshot());
        int result;
        try
        {
            result = DoDragDrop(
                dataObject,
                dropSource,
                DropEffectCopy,
                out effect);
        }
        finally
        {
            watchdogCancellation.Cancel();
        }
        var exitFields = telemetry.Snapshot();
        exitFields["result_hresult"] = FormatHResult(result);
        exitFields["drop_effect"] = effect;
        terminalStatus = result == DragDropCancel ? "cancelled" : result is Succeeded or DragDropDrop ? "dropped" : "failed";
        exitFields["terminal_status"] = terminalStatus;
        exitFields["elapsed_ms"] = Stopwatch
            .GetElapsedTime(startedTimestamp)
            .TotalMilliseconds;
        ReportInputSnapshot(
            reportDiagnostic,
            "ole_exited",
            sourceWindow,
            exitFields);
        var terminal = ResultForOleExit(result, effect);
        error = terminal.Error;
        return terminal.Success;
    }

    internal static WindowsCommandResult ResultForOleExit(int result, int effect)
    {
        var success = result is Succeeded or DragDropDrop or DragDropCancel;
        var status = result == DragDropCancel ? "cancelled" : success ? "dropped" : "failed";
        return new WindowsCommandResult(success,
            success ? null : $"Windows DoDragDrop failed with HRESULT 0x{result:X8}.",
            Data: System.Text.Json.JsonSerializer.Serialize(new { status, drop_effect = effect }),
            NativeErrorCode: success ? null : result);
    }


    internal static bool TryAttachInputQueues(
        uint currentThreadId,
        uint sourceThreadId,
        AttachThreadInputOperation attachOperation,
        out IDisposable? lease,
        Action<bool>? onDetach = null)
    {
        ArgumentNullException.ThrowIfNull(attachOperation);
        lease = null;
        if (currentThreadId == sourceThreadId)
        {
            return true;
        }
        if (!attachOperation(currentThreadId, sourceThreadId, true))
        {
            return false;
        }
        lease = new InputQueueLease(
            currentThreadId,
            sourceThreadId,
            attachOperation,
            onDetach);
        return true;
    }

    private static bool TryAttachSourceInputQueue(
        IntPtr sourceWindow,
        NativeFileDragDiagnostic? reportDiagnostic,
        out IDisposable? lease,
        out string? error)
    {
        lease = null;
        error = null;
        var sourceThreadId = GetWindowThreadProcessId(
            sourceWindow,
            out var sourceProcessId);
        if (sourceThreadId == 0)
        {
            error = "Windows could not identify the completed download input thread.";
            ReportDiagnostic(
                reportDiagnostic,
                "input_queue_attach_failed",
                new Dictionary<string, object?>
                {
                    ["error"] = error,
                    ["last_error"] = Marshal.GetLastWin32Error()
                });
            return false;
        }

        var currentThreadId = GetCurrentThreadId();
        _ = PeekMessage(
            out _,
            IntPtr.Zero,
            0,
            0,
            0);
        var attached = TryAttachInputQueues(
            currentThreadId,
            sourceThreadId,
            AttachThreadInput,
            out lease,
            detachSucceeded => ReportDiagnostic(
                reportDiagnostic,
                "input_queue_detached",
                new Dictionary<string, object?>
                {
                    ["success"] = detachSucceeded,
                    ["last_error"] = detachSucceeded
                        ? 0
                        : Marshal.GetLastWin32Error(),
                    ["ole_native_thread_id"] = currentThreadId,
                    ["source_thread_id"] = sourceThreadId
                }));
        if (!attached)
        {
            var lastError = Marshal.GetLastWin32Error();
            error = $"Windows could not attach the drag input queue (error {lastError}).";
            ReportDiagnostic(
                reportDiagnostic,
                "input_queue_attach_failed",
                new Dictionary<string, object?>
                {
                    ["error"] = error,
                    ["last_error"] = lastError,
                    ["ole_native_thread_id"] = currentThreadId,
                    ["source_thread_id"] = sourceThreadId,
                    ["source_process_id"] = sourceProcessId
                });
            return false;
        }
        ReportDiagnostic(
            reportDiagnostic,
            currentThreadId == sourceThreadId
                ? "input_queue_already_shared"
                : "input_queue_attached",
            new Dictionary<string, object?>
            {
                ["ole_native_thread_id"] = currentThreadId,
                ["source_thread_id"] = sourceThreadId,
                ["source_process_id"] = sourceProcessId
            });
        return true;
    }

    private static async Task ReportDragStillActiveAsync(
        IntPtr sourceWindow,
        OleDragTelemetry telemetry,
        long startedTimestamp,
        NativeFileDragDiagnostic? reportDiagnostic,
        CancellationToken cancellationToken)
    {
        try
        {
            await Task.Delay(
                    FirstWatchdogDelayMilliseconds,
                    cancellationToken)
                .ConfigureAwait(false);
            var firstFields = telemetry.Snapshot();
            firstFields["elapsed_ms"] = Stopwatch
                .GetElapsedTime(startedTimestamp)
                .TotalMilliseconds;
            ReportInputSnapshot(
                reportDiagnostic,
                "ole_still_active_1s",
                sourceWindow,
                firstFields);

            await Task.Delay(
                    SecondWatchdogDelayMilliseconds,
                    cancellationToken)
                .ConfigureAwait(false);
            var secondFields = telemetry.Snapshot();
            secondFields["elapsed_ms"] = Stopwatch
                .GetElapsedTime(startedTimestamp)
                .TotalMilliseconds;
            ReportInputSnapshot(
                reportDiagnostic,
                "ole_still_active_5s",
                sourceWindow,
                secondFields);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (Exception error)
        {
            ReportDiagnostic(
                reportDiagnostic,
                "watchdog_faulted",
                new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["exception_message"] = error.Message
                });
        }
    }

    private static void ReportInputSnapshot(
        NativeFileDragDiagnostic? reportDiagnostic,
        string stage,
        IntPtr sourceWindow,
        IReadOnlyDictionary<string, object?>? additionalFields = null)
    {
        var sourceThreadId = GetWindowThreadProcessId(
            sourceWindow,
            out var sourceProcessId);
        var foregroundWindow = GetForegroundWindow();
        var cursorAvailable = GetCursorPos(out var cursor);
        var cursorWindow = cursorAvailable
            ? WindowFromPoint(cursor)
            : IntPtr.Zero;
        var leftButtonState = GetAsyncKeyState(VirtualKeyLeftButton);
        var fields = new Dictionary<string, object?>
        {
            ["probe_native_thread_id"] = GetCurrentThreadId(),
            ["probe_managed_thread_id"] = Environment.CurrentManagedThreadId,
            ["probe_apartment_state"] = Thread.CurrentThread.GetApartmentState().ToString(),
            ["source_process_id"] = sourceProcessId,
            ["source_thread_id"] = sourceThreadId,
            ["source_window_valid"] = IsWindow(sourceWindow),
            ["foreground_window_handle"] = foregroundWindow.ToInt64(),
            ["foreground_root_window_handle"] = foregroundWindow == IntPtr.Zero
                ? 0
                : GetAncestor(foregroundWindow, GetAncestorRoot).ToInt64(),
            ["current_thread_capture_window_handle"] = GetCapture().ToInt64(),
            ["cursor_available"] = cursorAvailable,
            ["cursor_x"] = cursorAvailable ? cursor.X : (int?)null,
            ["cursor_y"] = cursorAvailable ? cursor.Y : (int?)null,
            ["cursor_window_handle"] = cursorWindow.ToInt64(),
            ["cursor_root_window_handle"] = cursorWindow == IntPtr.Zero
                ? 0
                : GetAncestor(cursorWindow, GetAncestorRoot).ToInt64()
        };
        foreach (var pointerField in PointerStateFields(leftButtonState))
        {
            fields[pointerField.Key] = pointerField.Value;
        }

        if (sourceThreadId != 0 &&
            TryGetGuiThreadInformation(sourceThreadId, out var sourceInformation))
        {
            fields["source_active_window_handle"] =
                sourceInformation.ActiveWindow.ToInt64();
            fields["source_focus_window_handle"] =
                sourceInformation.FocusWindow.ToInt64();
            fields["source_capture_window_handle"] =
                sourceInformation.CaptureWindow.ToInt64();
            fields["source_menu_owner_window_handle"] =
                sourceInformation.MenuOwnerWindow.ToInt64();
            fields["source_move_size_window_handle"] =
                sourceInformation.MoveSizeWindow.ToInt64();
            fields["source_gui_flags"] = sourceInformation.Flags;
        }
        else
        {
            fields["source_gui_thread_info_available"] = false;
        }

        if (additionalFields is not null)
        {
            foreach (var field in additionalFields)
            {
                fields[field.Key] = field.Value;
            }
        }
        ReportDiagnostic(reportDiagnostic, stage, fields);
    }

    private static Dictionary<string, object?> PointerStateFields(
        short leftButtonState) =>
        new()
        {
            ["async_left_button_state"] =
                $"0x{unchecked((ushort)leftButtonState):X4}",
            ["async_left_button_down"] = (leftButtonState & 0x8000) != 0,
            ["async_left_button_pressed_since_last_query"] =
                (leftButtonState & 0x0001) != 0
        };

    private static void ReportDiagnostic(
        NativeFileDragDiagnostic? reportDiagnostic,
        string stage,
        IReadOnlyDictionary<string, object?>? fields = null)
    {
        try
        {
            reportDiagnostic?.Invoke(stage, fields);
        }
        catch
        {
            // Diagnostics must never alter OLE drag behavior.
        }
    }

    private static string FormatHResult(int result) =>
        $"0x{unchecked((uint)result):X8}";

    private static string DragResultName(int result) =>
        result switch
        {
            Succeeded => "continue",
            DragDropDrop => "drop",
            DragDropCancel => "cancel",
            _ => "other"
        };

    private static string ClipboardFormatName(short format) =>
        format switch
        {
            ClipboardFormatDropFiles => "CF_HDROP",
            _ when format == PreferredDropEffectFormat =>
                "Preferred DropEffect",
            _ => "other"
        };

    internal static int QueryContinueDrag(bool escapePressed, int keyState)
        => QueryContinueDrag(
            escapePressed,
            keyState,
            (GetAsyncKeyState(VirtualKeyLeftButton) & 0x8000) != 0);

    internal static int QueryContinueDrag(
        bool escapePressed,
        int keyState,
        bool leftButtonCurrentlyPressed)
    {
        // AttachThreadInput resets the queue-local key state. The asynchronous
        // physical state remains authoritative for this cross-process gesture.
        _ = keyState;
        if (escapePressed)
        {
            return DragDropCancel;
        }
        return !leftButtonCurrentlyPressed
            ? DragDropDrop
            : Succeeded;
    }

    internal static void ReleaseOleForCurrentThread()
    {
        if (!_oleInitialized)
        {
            return;
        }
        OleUninitialize();
        _oleInitialized = false;
    }

    private static bool TryReleaseSourcePointerCapture(
        IntPtr sourceWindow,
        out string? error)
    {
        error = null;
        var sourceThread = GetWindowThreadProcessId(sourceWindow, out var sourceProcess);
        if (sourceThread == 0 || sourceProcess == 0)
        {
            error = "Windows could not identify the completed download window.";
            return false;
        }

        if (!TryGetThreadCapture(sourceThread, out var captureWindow))
        {
            error = "Windows could not inspect the completed download pointer state.";
            return false;
        }
        if (captureWindow == IntPtr.Zero)
        {
            return true;
        }

        _ = GetWindowThreadProcessId(captureWindow, out var captureProcess);
        if (captureProcess != sourceProcess)
        {
            error = "Another application owns the active pointer capture.";
            return false;
        }

        if (!SendMessageTimeout(
                captureWindow,
                WmCancelMode,
                IntPtr.Zero,
                IntPtr.Zero,
                SendMessageAbortIfHung,
                SourceCaptureReleaseTimeoutMilliseconds,
                out _))
        {
            error = "The completed download window did not release pointer capture.";
            return false;
        }

        if (!TryGetThreadCapture(sourceThread, out captureWindow))
        {
            error = "Windows could not verify the completed download pointer state.";
            return false;
        }
        if (captureWindow != IntPtr.Zero)
        {
            error = "The completed download window retained pointer capture.";
            return false;
        }
        return true;
    }

    private static bool TryGetThreadCapture(uint threadId, out IntPtr captureWindow)
    {
        if (!TryGetGuiThreadInformation(threadId, out var information))
        {
            captureWindow = IntPtr.Zero;
            return false;
        }
        captureWindow = information.CaptureWindow;
        return true;
    }

    private static bool TryGetGuiThreadInformation(
        uint threadId,
        out GuiThreadInfo information)
    {
        information = new GuiThreadInfo
        {
            Size = Marshal.SizeOf<GuiThreadInfo>()
        };
        return GetGUIThreadInfo(threadId, ref information);
    }

    private static bool EnsureOleInitialized(out string? error)
    {
        error = null;
        if (_oleInitialized)
        {
            return true;
        }

        var result = OleInitialize(IntPtr.Zero);
        if (result == RpcChangedMode)
        {
            error = "The Windows adapter thread does not support OLE file dragging.";
            return false;
        }
        if (result < 0)
        {
            error = $"OleInitialize failed with HRESULT 0x{result:X8}.";
            return false;
        }

        _oleInitialized = true;
        return true;
    }

    private static FORMATETC CreateDropFilesFormat() =>
        new()
        {
            cfFormat = ClipboardFormatDropFiles,
            ptd = IntPtr.Zero,
            dwAspect = DVASPECT.DVASPECT_CONTENT,
            lindex = -1,
            tymed = TYMED.TYMED_HGLOBAL
        };

    private static FORMATETC CreatePreferredDropEffectFormat() =>
        new()
        {
            cfFormat = PreferredDropEffectFormat,
            ptd = IntPtr.Zero,
            dwAspect = DVASPECT.DVASPECT_CONTENT,
            lindex = -1,
            tymed = TYMED.TYMED_HGLOBAL
        };

    private static bool IsDropFilesFormat(FORMATETC format) =>
        format.cfFormat == ClipboardFormatDropFiles &&
        format.dwAspect == DVASPECT.DVASPECT_CONTENT &&
        format.lindex == -1 &&
        (format.tymed & TYMED.TYMED_HGLOBAL) != 0;

    private static bool IsPreferredDropEffectFormat(FORMATETC format) =>
        format.cfFormat == PreferredDropEffectFormat &&
        format.dwAspect == DVASPECT.DVASPECT_CONTENT &&
        format.lindex == -1 &&
        (format.tymed & TYMED.TYMED_HGLOBAL) != 0;

    private static IntPtr CreateDropFilesHandle(IReadOnlyList<string> paths)
    {
        var payload = string.Join('\0', paths.Select(Path.GetFullPath)) + "\0\0";
        var payloadBytes = Encoding.Unicode.GetBytes(payload);
        var headerSize = Marshal.SizeOf<DropFilesHeader>();
        var totalSize = checked(headerSize + payloadBytes.Length);
        var handle = GlobalAlloc(
            GlobalMemoryMoveable | GlobalMemoryZeroInitialize,
            (UIntPtr)totalSize);
        if (handle == IntPtr.Zero)
        {
            throw new OutOfMemoryException("GlobalAlloc failed for CF_HDROP.");
        }

        var locked = IntPtr.Zero;
        try
        {
            locked = GlobalLock(handle);
            if (locked == IntPtr.Zero)
            {
                throw new InvalidOperationException("GlobalLock failed for CF_HDROP.");
            }

            var header = new DropFilesHeader
            {
                FileOffset = headerSize,
                Wide = 1
            };
            Marshal.StructureToPtr(header, locked, false);
            Marshal.Copy(
                payloadBytes,
                0,
                IntPtr.Add(locked, headerSize),
                payloadBytes.Length);
            return handle;
        }
        catch
        {
            _ = GlobalFree(handle);
            throw;
        }
        finally
        {
            if (locked != IntPtr.Zero)
            {
                _ = GlobalUnlock(handle);
            }
        }
    }

    private static IntPtr CreateDropEffectHandle()
    {
        var handle = GlobalAlloc(
            GlobalMemoryMoveable | GlobalMemoryZeroInitialize,
            (UIntPtr)sizeof(int));
        if (handle == IntPtr.Zero)
        {
            throw new OutOfMemoryException(
                "GlobalAlloc failed for Preferred DropEffect.");
        }

        var locked = IntPtr.Zero;
        try
        {
            locked = GlobalLock(handle);
            if (locked == IntPtr.Zero)
            {
                throw new InvalidOperationException(
                    "GlobalLock failed for Preferred DropEffect.");
            }
            Marshal.WriteInt32(locked, DropEffectCopy);
            return handle;
        }
        catch
        {
            _ = GlobalFree(handle);
            throw;
        }
        finally
        {
            if (locked != IntPtr.Zero)
            {
                _ = GlobalUnlock(handle);
            }
        }
    }

    private sealed class FileDropDataObject(
        IReadOnlyList<string> paths,
        OleDragTelemetry telemetry,
        NativeFileDragDiagnostic? reportDiagnostic) : ComDataObject
    {
        private readonly FORMATETC[] _formats =
        [
            CreateDropFilesFormat(),
            CreatePreferredDropEffectFormat()
        ];
        private readonly HashSet<string> _reportedCalls = [];

        public void GetData(ref FORMATETC format, out STGMEDIUM medium)
        {
            Interlocked.Increment(ref telemetry.GetDataCount);
            ReportFormatOnce("data_object_get_data", ref format);
            medium = default;
            if (IsDropFilesFormat(format))
            {
                medium.tymed = TYMED.TYMED_HGLOBAL;
                medium.unionmember = CreateDropFilesHandle(paths);
                medium.pUnkForRelease = null;
                return;
            }
            if (IsPreferredDropEffectFormat(format))
            {
                medium.tymed = TYMED.TYMED_HGLOBAL;
                medium.unionmember = CreateDropEffectHandle();
                medium.pUnkForRelease = null;
                return;
            }
            ThrowCom(
                format.tymed == TYMED.TYMED_HGLOBAL
                    ? DataViewFormatEtc
                    : DataViewTymed);
        }

        public void GetDataHere(ref FORMATETC format, ref STGMEDIUM medium) =>
            ThrowCom(NotImplemented);

        public int QueryGetData(ref FORMATETC format)
        {
            Interlocked.Increment(ref telemetry.QueryGetDataCount);
            ReportFormatOnce("data_object_query_get_data", ref format);
            if (IsDropFilesFormat(format) ||
                IsPreferredDropEffectFormat(format))
            {
                return Succeeded;
            }
            return (format.tymed & TYMED.TYMED_HGLOBAL) == 0
                ? DataViewTymed
                : DataViewFormatEtc;
        }

        public int GetCanonicalFormatEtc(
            ref FORMATETC formatIn,
            out FORMATETC formatOut)
        {
            formatOut = formatIn;
            return DataSameFormatEtc;
        }

        public void SetData(
            ref FORMATETC formatIn,
            ref STGMEDIUM medium,
            bool release) =>
            ThrowCom(NotImplemented);

        public IEnumFORMATETC EnumFormatEtc(DATADIR direction)
        {
            Interlocked.Increment(ref telemetry.EnumFormatCount);
            ReportOnce(
                $"enum:{direction}",
                "data_object_enum_formats",
                new Dictionary<string, object?>
                {
                    ["direction"] = direction.ToString()
                });
            if (direction == DATADIR.DATADIR_GET)
            {
                return new FormatEnumerator(_formats);
            }
            ThrowCom(NotImplemented);
            return null!;
        }

        public int DAdvise(
            ref FORMATETC format,
            ADVF flags,
            IAdviseSink adviseSink,
            out int connection)
        {
            connection = 0;
            return OleAdviseNotSupported;
        }

        public void DUnadvise(int connection) =>
            ThrowCom(OleAdviseNotSupported);

        public int EnumDAdvise(out IEnumSTATDATA enumAdvise)
        {
            enumAdvise = null!;
            return OleAdviseNotSupported;
        }

        private void ReportFormatOnce(
            string stage,
            ref FORMATETC format)
        {
            var formatName = ClipboardFormatName(format.cfFormat);
            ReportOnce(
                $"{stage}:{format.cfFormat}:{format.tymed}:{format.dwAspect}:{format.lindex}",
                stage,
                new Dictionary<string, object?>
                {
                    ["clipboard_format"] = formatName,
                    ["clipboard_format_id"] = unchecked((ushort)format.cfFormat),
                    ["tymed"] = format.tymed.ToString(),
                    ["aspect"] = format.dwAspect.ToString(),
                    ["index"] = format.lindex
                });
        }

        private void ReportOnce(
            string key,
            string stage,
            IReadOnlyDictionary<string, object?> fields)
        {
            if (!_reportedCalls.Add(key))
            {
                return;
            }
            ReportDiagnostic(reportDiagnostic, stage, fields);
        }
    }

    private sealed class FormatEnumerator(
        IReadOnlyList<FORMATETC> formats) : IEnumFORMATETC
    {
        private int _index;

        public int Next(int count, FORMATETC[] items, int[] fetchedCount)
        {
            var fetched = 0;
            while (fetched < count && _index < formats.Count)
            {
                items[fetched++] = formats[_index++];
            }
            if (fetchedCount is { Length: > 0 })
            {
                fetchedCount[0] = fetched;
            }
            return fetched == count ? Succeeded : SucceededFalse;
        }

        public int Skip(int count)
        {
            _index = Math.Min(formats.Count, _index + count);
            return _index < formats.Count ? Succeeded : SucceededFalse;
        }

        public int Reset()
        {
            _index = 0;
            return Succeeded;
        }

        public void Clone(out IEnumFORMATETC newEnumerator)
        {
            newEnumerator = new FormatEnumerator(formats)
            {
                _index = _index
            };
        }
    }

    [ComVisible(true)]
    [ClassInterface(ClassInterfaceType.None)]
    private sealed class FileDropSource(
        OleDragTelemetry telemetry,
        NativeFileDragDiagnostic? reportDiagnostic) : IDropSource
    {
        private readonly long _startedTimestamp = Stopwatch.GetTimestamp();
        private bool _hasQueryState;
        private bool _previousEscapePressed;
        private int _previousKeyState;
        private bool _previousAsyncLeftButtonDown;
        private int? _previousFeedbackEffect;

        public int QueryContinueDrag(bool escapePressed, int keyState)
        {
            var callbackCount = Interlocked.Increment(
                ref telemetry.QueryContinueCount);
            var leftButtonState = GetAsyncKeyState(VirtualKeyLeftButton);
            var asyncLeftButtonDown = (leftButtonState & 0x8000) != 0;
            var result = NativeFileDragService.QueryContinueDrag(
                escapePressed,
                keyState,
                asyncLeftButtonDown);
            var stateChanged = !_hasQueryState ||
                               _previousEscapePressed != escapePressed ||
                               _previousKeyState != keyState ||
                               _previousAsyncLeftButtonDown != asyncLeftButtonDown;
            var terminal = result != Succeeded;
            if (stateChanged || terminal)
            {
                ReportDiagnostic(
                    reportDiagnostic,
                    "query_continue_drag",
                    new Dictionary<string, object?>
                    {
                        ["callback_count"] = callbackCount,
                        ["reason"] = !_hasQueryState
                            ? terminal ? "first_terminal" : "first"
                            : terminal ? "terminal" : "state_changed",
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(_startedTimestamp)
                            .TotalMilliseconds,
                        ["escape_pressed"] = escapePressed,
                        ["ole_key_state"] =
                            $"0x{unchecked((uint)keyState):X8}",
                        ["ole_key_state_left_down"] =
                            (keyState & LeftButtonMask) != 0,
                        ["async_left_button_state"] =
                            $"0x{unchecked((ushort)leftButtonState):X4}",
                        ["async_left_button_down"] = asyncLeftButtonDown,
                        ["result_hresult"] = FormatHResult(result),
                        ["result_name"] = DragResultName(result)
                    });
            }
            _hasQueryState = true;
            _previousEscapePressed = escapePressed;
            _previousKeyState = keyState;
            _previousAsyncLeftButtonDown = asyncLeftButtonDown;
            return result;
        }

        public int GiveFeedback(int effect)
        {
            var callbackCount = Interlocked.Increment(
                ref telemetry.GiveFeedbackCount);
            if (_previousFeedbackEffect != effect)
            {
                ReportDiagnostic(
                    reportDiagnostic,
                    "give_feedback",
                    new Dictionary<string, object?>
                    {
                        ["callback_count"] = callbackCount,
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(_startedTimestamp)
                            .TotalMilliseconds,
                        ["drop_effect"] = effect
                    });
                _previousFeedbackEffect = effect;
            }
            return DragDropUseDefaultCursors;
        }
    }

    private sealed class OleDragTelemetry(
        uint nativeThreadId,
        int managedThreadId)
    {
        public int QueryContinueCount;
        public int GiveFeedbackCount;
        public int QueryGetDataCount;
        public int GetDataCount;
        public int EnumFormatCount;

        public Dictionary<string, object?> Snapshot() =>
            new()
            {
                ["ole_native_thread_id"] = nativeThreadId,
                ["ole_managed_thread_id"] = managedThreadId,
                ["query_continue_count"] = Volatile.Read(ref QueryContinueCount),
                ["give_feedback_count"] = Volatile.Read(ref GiveFeedbackCount),
                ["query_get_data_count"] = Volatile.Read(ref QueryGetDataCount),
                ["get_data_count"] = Volatile.Read(ref GetDataCount),
                ["enum_format_count"] = Volatile.Read(ref EnumFormatCount)
            };
    }

    private sealed class InputQueueLease(
        uint currentThreadId,
        uint sourceThreadId,
        AttachThreadInputOperation attachOperation,
        Action<bool>? onDetach) : IDisposable
    {
        private int _disposed;

        public void Dispose()
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0)
            {
                return;
            }
            var detached = attachOperation(
                currentThreadId,
                sourceThreadId,
                false);
            onDetach?.Invoke(detached);
        }
    }

    [ComImport]
    [Guid("00000121-0000-0000-C000-000000000046")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IDropSource
    {
        [PreserveSig]
        int QueryContinueDrag(
            [MarshalAs(UnmanagedType.Bool)] bool escapePressed,
            int keyState);

        [PreserveSig]
        int GiveFeedback(int effect);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DropFilesHeader
    {
        public int FileOffset;
        public int X;
        public int Y;
        public int NonClient;
        public int Wide;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct GuiThreadInfo
    {
        public int Size;
        public int Flags;
        public IntPtr ActiveWindow;
        public IntPtr FocusWindow;
        public IntPtr CaptureWindow;
        public IntPtr MenuOwnerWindow;
        public IntPtr MoveSizeWindow;
        public IntPtr CaretWindow;
        public NativeRect CaretBounds;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativePoint
    {
        public int X;
        public int Y;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeMessage
    {
        public IntPtr Window;
        public uint Message;
        public UIntPtr WParam;
        public IntPtr LParam;
        public uint Time;
        public NativePoint Point;
        public uint Private;
    }

    private static void ThrowCom(int result) =>
        throw new COMException(null, result);

    [DllImport("ole32.dll")]
    private static extern int OleInitialize(IntPtr reserved);

    [DllImport("ole32.dll")]
    private static extern void OleUninitialize();

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("ole32.dll")]
    private static extern int DoDragDrop(
        ComDataObject dataObject,
        IDropSource dropSource,
        int allowedEffects,
        out int effect);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetGUIThreadInfo(
        uint threadId,
        ref GuiThreadInfo information);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern IntPtr GetAncestor(
        IntPtr window,
        uint flags);

    [DllImport("user32.dll")]
    private static extern IntPtr GetCapture();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetCursorPos(out NativePoint point);

    [DllImport("user32.dll")]
    private static extern IntPtr WindowFromPoint(NativePoint point);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AttachThreadInput(
        uint attachThreadId,
        uint attachToThreadId,
        [MarshalAs(UnmanagedType.Bool)] bool attach);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PeekMessage(
        out NativeMessage message,
        IntPtr window,
        uint minimumMessage,
        uint maximumMessage,
        uint removeMessage);

    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int virtualKey);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SendMessageTimeout(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        uint flags,
        uint timeoutMilliseconds,
        out UIntPtr result);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern short RegisterClipboardFormat(string format);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalAlloc(int flags, UIntPtr bytes);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalLock(IntPtr handle);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GlobalUnlock(IntPtr handle);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr GlobalFree(IntPtr handle);
}
