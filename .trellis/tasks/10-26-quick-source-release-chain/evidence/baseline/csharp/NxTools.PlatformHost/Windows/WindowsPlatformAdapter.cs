using System.Collections.Concurrent;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Channels;
using System.Windows.Forms;
using Microsoft.Win32;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record WindowsCommandResult(
    bool Success,
    string? Error = null,
    string? Data = null,
    bool Fatal = false,
    int? NativeErrorCode = null);

internal readonly record struct DesktopTrayCandidate(NotifyIcon TrayIcon, Icon OwnedIcon);

internal readonly record struct NativeWindowPresentationState(
    bool Exists,
    bool Visible,
    bool Topmost,
    bool Cloaked,
    long OwnerWindowHandle,
    long ForegroundWindowHandle,
    NativeWindowRectangle Bounds,
    NativeWindowRectangle ClientBounds);

 internal sealed class ClipboardCaptureDropDiagnosticGate
 {
     private const long LogIntervalMilliseconds = 1_000;
     private readonly object _gate = new();
     private bool _hasLogged;
     private long _lastLogAtMilliseconds;
     private long _suppressedDropCount;

     internal bool TryAcquire(out long suppressedDropCount)
     {
         lock (_gate)
         {
             var now = Environment.TickCount64;
             if (!_hasLogged || now - _lastLogAtMilliseconds >= LogIntervalMilliseconds)
             {
                 _hasLogged = true;
                 _lastLogAtMilliseconds = now;
                 suppressedDropCount = _suppressedDropCount;
                 _suppressedDropCount = 0;
                 return true;
             }
             _suppressedDropCount++;
             suppressedDropCount = 0;
             return false;
         }
     }
 }

internal sealed partial class WindowsPlatformAdapter : IDisposable
{
    private const long QuickLaunchContextMaximumAgeMilliseconds = 1_000;
    internal static int NativeInputSize => NativeMethods.InputSize;
    internal static int BottomRightResizeSystemCommand => 0xF008;
    internal static int MoveCaptionSystemCommand => 0xF012;
    internal const string DesktopExecutableEnvironmentVariable = "NXTOOLS_DESKTOP_EXECUTABLE";

    private readonly Channel<WindowsPlatformEvent> _events = Channel.CreateBounded<WindowsPlatformEvent>(
        new BoundedChannelOptions(512)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });
    private readonly ManualResetEventSlim _started = new(false);
    private readonly Thread _thread;
    private readonly IPlatformLog _log;
    private readonly ShellContextMenuSessionManager _shellActions = new();
    private readonly IElevatedDirectoryNavigationChannel _elevatedDirectoryNavigation;
    private readonly NativeFileDragCoordinator _fileDragCoordinator;
    private readonly NativeContextMenuCoordinator _contextMenus;
    private readonly TaskbarProgressLogGate _taskbarProgressLogGate = new();
    private readonly ClipboardCaptureDropDiagnosticGate _clipboardCaptureDropDiagnosticGate = new();
    private readonly ICompletionSystemActions _completionSystemActions;
    private readonly ExtensionRuntimeInspector _extensionRuntimeInspector;
    private static readonly CenteredResizeCoordinator CenteredResizeWorkers = new();
    private EventWindow? _window;
    private Exception? _startupError;
    private bool _disposed;

    public WindowsPlatformAdapter(
        ForegroundContextProvider foregroundContextProvider,
        IPlatformLog? log = null,
        ICompletionSystemActions? completionSystemActions = null)
    {
        _log = log ?? NullPlatformLog.Instance;
        _completionSystemActions = completionSystemActions ?? new WindowsCompletionSystemActions();
        _fileDragCoordinator = new NativeFileDragCoordinator(_log);
        _contextMenus = new NativeContextMenuCoordinator(_log,
            flushPointerObservation: () => _window?.FlushContextMenuPointerPress());
        _extensionRuntimeInspector = new ExtensionRuntimeInspector(_log);
        ThirdPartyFileManagerAdapter.WarmUp();
        _elevatedDirectoryNavigation = new ElevatedDirectoryNavigationClient(
            platformEvent => _events.Writer.TryWrite(platformEvent),
            _log);
        _thread = new Thread(() => RunMessageLoop(
            foregroundContextProvider,
            _elevatedDirectoryNavigation))
        {
            IsBackground = true,
            Name = "NxTools.WindowsPlatformAdapter"
        };
        _thread.SetApartmentState(ApartmentState.STA);
        _thread.Start();
        _started.Wait();
        if (_startupError is not null)
        {
            throw new InvalidOperationException("Windows platform adapter failed to start.", _startupError);
        }
    }

    public ChannelReader<WindowsPlatformEvent> Events => _events.Reader;
    internal void DiscardQueuedEventsForNewSession()
    {
        while (_events.Reader.TryRead(out var platformEvent))
        {
            if (platformEvent is ClipboardCapturedEvent captured)
            {
                HandleClipboardCapturedEventForwardingFailure(
                    captured,
                    "session_replaced",
                    null);
            }
        }
    }

    internal void CancelCurrentShellContextWork() => _shellActions.CancelCurrent();
    internal static bool TryPublishClipboardCapturedEvent(
        ChannelWriter<WindowsPlatformEvent> events,
        ClipboardCapturedEvent captured,
        IPlatformLog log,
        ClipboardCaptureDropDiagnosticGate? dropDiagnosticGate = null)
    {
        if (events.TryWrite(captured))
        {
            return true;
        }
        var cleanup = CleanupClipboardCapturedStaging(captured);
        long suppressedDropCount = 0;
        var shouldLog = dropDiagnosticGate is null ||
            dropDiagnosticGate.TryAcquire(out suppressedDropCount);
        if (!shouldLog)
        {
            return false;
        }
        try
        {
            log.Write(
                "warning",
                "clipboard.capture_event_dropped",
                fields: new Dictionary<string, object?>
                {
                    ["reason"] = "event_queue_full",
                    ["image_staging_file_count"] = cleanup.ImageFileCount,
                    ["native_format_staging_file_count"] = cleanup.NativeFormatFileCount,
                    ["source_icon_staging_file_count"] = cleanup.SourceIconFileCount,
                    ["cleanup_deleted_file_count"] = cleanup.DeletedFileCount,
                    ["cleanup_failed_file_count"] = cleanup.FailedFileCount,
                    ["suppressed_drop_count"] = suppressedDropCount
                });
        }
        catch
        {
            // Event diagnostics must not change the bounded queue result.
        }
        return false;
    }
    internal void HandleClipboardCapturedEventForwardingFailure(
        ClipboardCapturedEvent captured,
        string reason,
        int? frameBytes,
        Exception? error = null)
    {
        var cleanup = CleanupClipboardCapturedStaging(captured);
        if (!_clipboardCaptureDropDiagnosticGate.TryAcquire(out var suppressedDropCount))
        {
            return;
        }
        try
        {
            _log.Write(
                "warning",
                "clipboard.capture_event_dropped",
                fields: new Dictionary<string, object?>
                {
                    ["reason"] = reason,
                    ["frame_bytes"] = frameBytes,
                    ["exception_type"] = error?.GetType().FullName,
                    ["image_staging_file_count"] = cleanup.ImageFileCount,
                    ["native_format_staging_file_count"] = cleanup.NativeFormatFileCount,
                    ["source_icon_staging_file_count"] = cleanup.SourceIconFileCount,
                    ["cleanup_deleted_file_count"] = cleanup.DeletedFileCount,
                    ["cleanup_failed_file_count"] = cleanup.FailedFileCount,
                    ["suppressed_drop_count"] = suppressedDropCount
                });
        }
        catch
        {
            // Event diagnostics must not change forwarding behavior.
        }
    }

    internal static ClipboardCapturedEventCleanup CleanupClipboardCapturedStaging(
        ClipboardCapturedEvent captured)
    {
        var candidates = new List<(string Kind, string Path)>();
        if (captured.Image is { StagingPath: { } imagePath } &&
            !string.IsNullOrWhiteSpace(imagePath))
        {
            candidates.Add(("image", imagePath));
        }
        foreach (var format in captured.NativeFormats ?? [])
        {
            if (!string.IsNullOrWhiteSpace(format.DataPath))
            {
                candidates.Add(("native_format", format.DataPath));
            }
        }
        if (!string.IsNullOrWhiteSpace(captured.SourceIconStagingPath))
        {
            candidates.Add(("source_icon", captured.SourceIconStagingPath));
        }
        var uniquePaths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var deleted = 0;
        var failed = 0;
        foreach (var candidate in candidates)
        {
            if (!uniquePaths.Add(candidate.Path))
            {
                continue;
            }
            try
            {
                if (File.Exists(candidate.Path))
                {
                    File.Delete(candidate.Path);
                    deleted++;
                }
            }
            catch (IOException)
            {
                failed++;
            }
            catch (UnauthorizedAccessException)
            {
                failed++;
            }
            catch (ArgumentException)
            {
                failed++;
            }
            catch (NotSupportedException)
            {
                failed++;
            }
            catch (System.Security.SecurityException)
            {
                failed++;
            }
        }
        return new ClipboardCapturedEventCleanup(
            candidates.Count(candidate => candidate.Kind == "image"),
            candidates.Count(candidate => candidate.Kind == "native_format"),
            candidates.Count(candidate => candidate.Kind == "source_icon"),
            deleted,
            failed);
    }
    internal readonly record struct ClipboardCapturedEventCleanup(
        int ImageFileCount,
        int NativeFormatFileCount,
        int SourceIconFileCount,
        int DeletedFileCount,
        int FailedFileCount);

    public bool ClipboardHotKeyAvailable => _window?.ClipboardHotKeyAvailable ?? false;
    public bool QuickLaunchHotKeyAvailable => _window?.QuickLaunchHotKeyAvailable ?? false;
    internal MotionPreferenceSnapshot SystemMotionSnapshot =>
        _window?.SystemMotionSnapshot ?? new MotionPreferenceSnapshot(true, -1);
    internal ThemePreferenceSnapshot SystemThemeSnapshot =>
        _window?.SystemThemeSnapshot ?? new ThemePreferenceSnapshot(true, -1);

    public Task<WindowsCommandResult> ConfigureClipboardAsync(
        ConfigureClipboardCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueClipboardConfiguration(command, cancellationToken);

    public Task<WindowsCommandResult> SetClipboardCapturePausedAsync(
        SetClipboardCapturePausedCommand command, Guid correlationId, CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueClipboardPause(command, correlationId, cancellationToken);
    }

    public Task<WindowsCommandResult> SetClipboardHotKeyRecordingAsync(
        SetClipboardHotKeyRecordingCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueClipboardHotKeyRecording(command, cancellationToken);

    public Task<WindowsCommandResult> ConfigureQuickLaunchAsync(
        ConfigureQuickLaunchCommand command,
        Guid correlationId,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueQuickLaunchConfiguration(command, correlationId, cancellationToken);

    public Task<WindowsCommandResult> SetQuickLaunchHotKeyRecordingAsync(
        SetQuickLaunchHotKeyRecordingCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueQuickLaunchHotKeyRecording(command, cancellationToken);

    public Task<WindowsCommandResult> ConfigureDesktopShellAsync(
        ConfigureDesktopShellCommand command,
        Guid correlationId,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueDesktopShellConfiguration(command, correlationId, cancellationToken);

    public Task<WindowsCommandResult> PasteTextAsync(
        PasteTextCommand command,
        Guid correlationId,
        CancellationToken cancellationToken)
        => EnqueuePaste(
            new TextClipboardPastePayload(
                command.Text, command.TargetWindowHandle,
                command.TargetFocusWindowHandle, command.PasteAfterWrite),
            correlationId,
            cancellationToken);

    public Task<WindowsCommandResult> PasteFilesAsync(
        PasteFilesCommand command, Guid correlationId, CancellationToken cancellationToken)
        => EnqueuePaste(
            new FilesClipboardPastePayload(
                command.Paths, command.TargetWindowHandle,
                command.TargetFocusWindowHandle, command.PasteAfterWrite),
            correlationId,
            cancellationToken);

    public Task<WindowsCommandResult> PasteImageAsync(
        PasteImageCommand command, Guid correlationId, CancellationToken cancellationToken)
        => EnqueuePaste(
            new ImageClipboardPastePayload(
                command.ImagePath, command.TargetWindowHandle,
                command.TargetFocusWindowHandle, command.IncludeFileDropFallback,
                command.PasteAfterWrite),
            correlationId,
            cancellationToken);

    public Task<WindowsCommandResult> PasteNativeFormatsAsync(
        PasteNativeFormatsCommand command, Guid correlationId, CancellationToken cancellationToken)
        => EnqueuePaste(
            new NativeFormatsClipboardPastePayload(
                command.Formats, command.ImagePathFallback, command.TargetWindowHandle,
                command.TargetFocusWindowHandle, command.PasteAfterWrite),
            correlationId,
            cancellationToken);

    private Task<WindowsCommandResult> EnqueuePaste(
        ClipboardPastePayload payload,
        Guid correlationId,
        CancellationToken cancellationToken)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        return (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueuePaste(payload, correlationId, cancellationToken);
    }

    public WindowsCommandResult SetWindowTopmost(SetWindowTopmostCommand command)
    {
        var window = new IntPtr(command.WindowHandle);
        var before = NativeMethods.CaptureWindowPresentation(window);
        var result = NativeMethods.SetWindowTopmost(window, command.Enabled);
        var after = NativeMethods.CaptureWindowPresentation(window);
        LogWindowPresentation(
            "quick_launch.window_topmost",
            command.WindowHandle,
            before,
            after,
            result,
            new Dictionary<string, object?>
            {
                ["requested_enabled"] = command.Enabled
            });
        return result;
    }

    public WindowsCommandResult SetWindowShadow(SetWindowShadowCommand command) =>
        NativeMethods.SetWindowShadow(
            new IntPtr(command.WindowHandle),
            command.Enabled);

    private static void CorrectNativeWindowVisibleCenter(
        IntPtr window,
        NativeWindowPresentationState before)
    {
        if (!before.Exists ||
            before.Bounds.Width <= 0 ||
            before.Bounds.Height <= 0 ||
            NativeMethods.IsDraggingOrSizing(window))
        {
            return;
        }

        var after = NativeMethods.CaptureWindowPresentation(window);
        if (!after.Exists || after.Bounds.Width <= 0 || after.Bounds.Height <= 0)
        {
            return;
        }
        var correctionX = (int)Math.Round(
            ((before.Bounds.Left + before.Bounds.Right) -
                (after.Bounds.Left + after.Bounds.Right)) / 2.0,
            MidpointRounding.AwayFromZero);
        var correctionY = (int)Math.Round(
            ((before.Bounds.Top + before.Bounds.Bottom) -
                (after.Bounds.Top + after.Bounds.Bottom)) / 2.0,
            MidpointRounding.AwayFromZero);
        if ((correctionX == 0 && correctionY == 0) ||
            !AnchoredWindowPlacement.TryGetWindowRectangle(window, out var outer))
        {
            return;
        }
        _ = NativeMethods.SetWindowPosition(
            window,
            outer.Left + correctionX,
            outer.Top + correctionY);
    }

    private static WindowsCommandResult SetWindowBoundsCenteredPreservingCenter(
        IntPtr window,
        int width,
        int height,
        Guid correlationId = default)
    {
        var before = NativeMethods.CaptureWindowPresentation(window);
        var result = NativeMethods.SetWindowBoundsCentered(
            window,
            width,
            height,
            correlationId);
        if (result.Success)
        {
            NativeMethods.ParkCenteredResizeForRecovery(
                window,
                width,
                height,
                correlationId);
        }
        if (result.Success)
        {
            // Moving a custom-client-chrome HWND can invalidate the client
            // rectangle again. Correct position first, then size/read back.
            CorrectNativeWindowVisibleCenter(window, before);
            var clientCorrection = EnsureRequestedClientSize(
                window, width, height, correlationId);
            if (!clientCorrection.Success)
            {
                NativeMethods.ParkCenteredResizeForRecovery(
                    window,
                    width,
                    height,
                    correlationId);
                return clientCorrection;
            }
            var final = NativeMethods.CaptureWindowPresentation(window);
            if (final.Exists &&
                (final.ClientBounds.Width != width || final.ClientBounds.Height != height))
            {
                NativeMethods.ParkCenteredResizeForRecovery(
                    window,
                    width,
                    height,
                    correlationId);
                return new WindowsCommandResult(false, "Final client size does not match the requested layout.");
            }
        }
        return result;
    }

    internal static CenteredResizeReplayResult ReapplyPendingCenteredResizeFor(IntPtr rootWindow) =>
        NativeMethods.ReapplyPendingCenteredResizeFor(rootWindow, (pending, before) =>
        {
            var window = new IntPtr(pending.WindowHandle);
            if (!NativeMethods.IsCurrentCenteredResize(rootWindow, pending.Generation))
                return new WindowsCommandResult(false, "The resize request was superseded.");
            CorrectNativeWindowVisibleCenter(window, before);
            if (!NativeMethods.IsCurrentCenteredResize(rootWindow, pending.Generation))
                return new WindowsCommandResult(false, "The resize request was superseded.");
            return EnsureRequestedClientSize(
                window, pending.Width, pending.Height, pending.CorrelationId);
        });

    internal static CenteredResizeReplayResult ReplayCenteredResizeBarrierForTest(long window, Action barrier) =>
        NativeMethods.ReapplyPendingCenteredResizeFor(NativeMethods.GetAncestorOrSelf(new IntPtr(window)), beforeApply: barrier);

    internal static CenteredResizeReplayResult ReplayCenteredResizeCommitForTest(long window, Action barrier) =>
        NativeMethods.ReapplyPendingCenteredResizeFor(NativeMethods.GetAncestorOrSelf(new IntPtr(window)), beforeCommit: barrier);

    internal static CenteredResizeRecoverySnapshot PendingCenteredResizeForTest(long window)
    {
        NativeMethods.TryGetPendingCenteredResize(NativeMethods.GetAncestorOrSelf(new IntPtr(window)), out var snapshot);
        return snapshot;
    }

    internal static WindowsCommandResult QueueNativeCenteredResizeForTest(long windowHandle, int width, int height)
    {
        var snapshot = NativeMethods.BeginCenteredResize(new IntPtr(windowHandle), width, height, Guid.NewGuid());
        return new WindowsCommandResult(CenteredResizeWorkers.TrySchedule(snapshot.RootWindow,
            () => { _ = ReapplyPendingCenteredResizeFor(snapshot.RootWindow); }));
    }

    private bool QueueCenteredResize(IntPtr rootWindow) => CenteredResizeWorkers.TrySchedule(rootWindow, () =>
    {
        if (!NativeMethods.TryGetPendingCenteredResize(rootWindow, out var owned)) return;
        CenteredResizeReplayResult replay;
        try { replay = ReapplyPendingCenteredResizeFor(rootWindow); }
        catch (Exception error)
        {
            replay = new(true, owned.CorrelationId, owned.WindowHandle, owned.StartedAtTimestamp,
                owned.AttemptCount, false, false, error.HResult) { Generation = owned.Generation };
            NativeMethods.CancelPendingCenteredResize(rootWindow, owned.CorrelationId, owned.Generation);
        }
        if (replay.HadPendingResize)
        {
            try { _log.Write(replay.Success ? "info" : "warning", "download.window_resize.readback", replay.CorrelationId,
                new Dictionary<string, object?> { ["window_handle"] = replay.WindowHandle, ["generation"] = replay.Generation,
                    ["success"] = replay.Success, ["pending_recovery"] = replay.StillPending,
                    ["attempt_count"] = replay.AttemptCount, ["native_error_code"] = replay.NativeErrorCode,
                    ["requested_client_width"] = replay.RequestedWidth, ["requested_client_height"] = replay.RequestedHeight,
                    ["actual_client_width"] = replay.ReadbackWidth, ["actual_client_height"] = replay.ReadbackHeight,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(replay.StartedAtTimestamp).TotalMilliseconds }); }
            catch { }
        }
        _ = _window?.RequestCenteredResizeRecovery(rootWindow);
    });
    internal static WindowsCommandResult ParkNativeWindowBoundsCenteredForTest(
        long windowHandle,
        int width,
        int height) =>
        NativeMethods.ParkCenteredResizeForTest(
            new IntPtr(windowHandle), width, height);

    internal static CenteredResizeReplayResult ReapplyNativeWindowBoundsCenteredForTest(long windowHandle) =>
        ReapplyPendingCenteredResizeFor(
            NativeMethods.GetAncestorOrSelf(new IntPtr(windowHandle)));
    public WindowsCommandResult SetWindowNoActivate(SetWindowNoActivateCommand command) =>
        NativeMethods.SetWindowNoActivate(new IntPtr(command.WindowHandle), command.Enabled);

    internal static WindowsCommandResult SetNativeWindowNoActivateForTest(long windowHandle, bool enabled) =>
        NativeMethods.SetWindowNoActivate(new IntPtr(windowHandle), enabled);

    internal static WindowsCommandResult SetNativeWindowBoundsForTest(long windowHandle, int x, int y, int width, int height) =>
        NativeMethods.SetWindowBounds(new IntPtr(windowHandle), x, y, width, height);

    internal static WindowsCommandResult SetNativeWindowVisibilityForTest(long windowHandle, bool visible) =>
        NativeMethods.SetWindowVisibility(new IntPtr(windowHandle), visible);

    public WindowsCommandResult SetWindowBounds(SetWindowBoundsCommand command) =>
        NativeMethods.SetWindowBounds(
            new IntPtr(command.WindowHandle),
            command.X,
            command.Y,
            command.Width,
            command.Height);



    public WindowsCommandResult SetWindowBoundsCentered(
        SetWindowBoundsCenteredCommand command,
        Guid correlationId)
    {
        var window = new IntPtr(command.WindowHandle);
        if (window == IntPtr.Zero || !NativeMethods.IsWindow(window) || command.Width <= 0 || command.Height <= 0)
            return new WindowsCommandResult(false, "The window or size is invalid.");
        var snapshot = NativeMethods.BeginCenteredResize(window, command.Width, command.Height, correlationId);
        if (!QueueCenteredResize(snapshot.RootWindow))
        {
            NativeMethods.CancelPendingCenteredResize(snapshot.RootWindow, correlationId, snapshot.Generation);
            return new WindowsCommandResult(false, "The bounded native resize queue is full.");
        }
        _ = _window?.RequestCenteredResizeRecovery(snapshot.RootWindow);
        try { _log.Write("info", "download.window_resize.accepted", correlationId,
            new Dictionary<string, object?> { ["window_handle"] = command.WindowHandle, ["generation"] = snapshot.Generation,
                ["requested_client_width"] = command.Width, ["requested_client_height"] = command.Height }); }
        catch { }
        // Success means accepted, NOT final application. Rust observes its HWND client
        // viewport and keeps the generation pending until that independent readback.
        return new WindowsCommandResult(true, Data: "{\"status\":\"accepted\",\"applied\":false}");
    }


    public WindowsCommandResult MoveWindowCenteredOver(MoveWindowCenteredOverCommand command) =>
        NativeMethods.MoveWindowCenteredOver(
            new IntPtr(command.WindowHandle),
            new IntPtr(command.AnchorWindowHandle),
            command.Width,
            command.Height);

    public WindowsCommandResult StashWindowBounds(StashWindowBoundsCommand command) =>
        NativeMethods.StashWindowBounds(
            new IntPtr(command.WindowHandle),
            command.Key);

    public WindowsCommandResult MoveWindowCenteredOverKey(MoveWindowCenteredOverKeyCommand command) =>
        NativeMethods.MoveWindowCenteredOverKey(
            new IntPtr(command.WindowHandle),
            command.Key,
            command.Width,
            command.Height);

    public WindowsCommandResult SetTaskbarProgress(
        SetTaskbarProgressCommand command,
        Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        var result = TaskbarProgress.Set(
            new IntPtr(command.WindowHandle),
            command.State,
            command.Completed,
            command.Total);
        var decision = _taskbarProgressLogGate.Evaluate(
            command.WindowHandle,
            command.State,
            command.Completed,
            command.Total,
            result,
            Environment.TickCount64);
        if (decision is not null)
        {
            try
            {
                _log.Write(
                    decision.Level,
                    decision.EventName,
                    correlationId,
                    new Dictionary<string, object?>
                    {
                        ["window_handle"] = command.WindowHandle,
                        ["state"] = command.State,
                        ["completed"] = command.Completed,
                        ["total"] = command.Total,
                        ["percent_milestone"] = decision.PercentMilestone,
                        ["success"] = result.Success,
                        ["error"] = result.Error,
                        ["shell_interface"] = "ITaskbarList4",
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(started)
                            .TotalMilliseconds
                    });
            }
            catch
            {
                // Diagnostics must not change taskbar progress behavior.
            }
        }
        return result;
    }

    public WindowsCommandResult PlaceTaskbarProgressOverlay(
        PlaceTaskbarProgressOverlayCommand command,
        Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        // SetWindowPos can synchronously produce taskbar location notifications. Publish the
        // requested placement first so an event-driven reapply cannot race in with stale offsets.
        _window?.TrackTaskbarOverlayPlacement(command, correlationId, appliedBounds: null);
        var result = TaskbarOverlayPlacement.Place(command);
        if (result.Success)
        {
            _window?.TrackTaskbarOverlayPlacement(command, correlationId, result.Data);
        }
        else
        {
            _window?.ForgetTaskbarOverlayPlacement(command.WindowHandle);
        }
        _log.Write(
            result.Success ? "info" : "warning",
            result.Success
                ? "download.taskbar_overlay.configured"
                : "download.taskbar_overlay.configure_failed",
            correlationId,
            new Dictionary<string, object?>
            {
                ["window_handle"] = command.WindowHandle,
                ["enabled"] = command.Enabled,
                ["visible"] = command.Visible,
                ["anchor"] = command.Anchor,
                ["offset_x"] = command.OffsetX,
                ["offset_y"] = command.OffsetY,
                ["width"] = command.Width,
                ["height"] = command.Height,
                ["interactive"] = command.Interactive,
                ["applied_bounds"] = result.Data,
                ["error"] = result.Error,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        return result;
    }

    public WindowsCommandResult SetTaskbarProgressOverlayVisibility(
        SetTaskbarProgressOverlayVisibilityCommand command,
        Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        var result = TaskbarOverlayPlacement.SetVisibility(
            new IntPtr(command.WindowHandle),
            command.Visible);
        if (result.Success)
        {
            _window?.TrackTaskbarOverlayVisibility(command);
        }
        else
        {
            _window?.ForgetTaskbarOverlayPlacement(command.WindowHandle);
        }
        _log.Write(
            result.Success ? "info" : "warning",
            result.Success
                ? "download.taskbar_overlay.visibility_changed"
                : "download.taskbar_overlay.visibility_failed",
            correlationId,
            new Dictionary<string, object?>
            {
                ["window_handle"] = command.WindowHandle,
                ["visible"] = command.Visible,
                ["error"] = result.Error,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        return result;
    }

    public WindowsCommandResult PlaceFileDialogSearchWindow(
        PlaceFileDialogSearchWindowCommand command)
    {
        var result = AnchoredWindowPlacement.Place(
            new IntPtr(command.WindowHandle),
            new IntPtr(command.OwnerWindowHandle),
            command.VisibleWidth,
            command.VisibleHeight,
            command.BottomOverlap);
        _log.Write(
            result.Success ? "info" : "warning",
            "quick_launch.file_dialog_search_placement",
            fields: new Dictionary<string, object?>
            {
                ["window_handle"] = command.WindowHandle,
                ["owner_window_handle"] = command.OwnerWindowHandle,
                ["visible_width"] = command.VisibleWidth,
                ["visible_height"] = command.VisibleHeight,
                ["bottom_overlap"] = command.BottomOverlap,
                ["applied_bounds"] = result.Data,
                ["error"] = result.Error
            });
        if (result.Success)
        {
            _window?.TrackFileDialogSearchPlacement(command);
        }
        return result;
    }

    public WindowsCommandResult SetWindowOwner(SetWindowOwnerCommand command)
    {
        var window = new IntPtr(command.WindowHandle);
        var before = NativeMethods.CaptureWindowPresentation(window);
        var result = NativeMethods.SetWindowOwner(
            window,
            new IntPtr(command.OwnerWindowHandle));
        var after = NativeMethods.CaptureWindowPresentation(window);
        LogWindowPresentation(
            "quick_launch.window_owner",
            command.WindowHandle,
            before,
            after,
            result,
            new Dictionary<string, object?>
            {
                ["requested_owner_window_handle"] = command.OwnerWindowHandle
            });
        return result;
    }

    public WindowsCommandResult SetWindowVisibility(
        SetWindowVisibilityCommand command,
        Guid correlationId)
    {
        var window = new IntPtr(command.WindowHandle);
        var before = NativeMethods.CaptureWindowPresentation(window);
        var result = NativeMethods.SetWindowVisibility(window, command.Visible);
        var after = NativeMethods.CaptureWindowPresentation(window);
        LogWindowPresentation(
            "quick_launch.window_visibility",
            command.WindowHandle,
            before,
            after,
            result,
            new Dictionary<string, object?>
            {
                ["requested_visible"] = command.Visible
            },
            correlationId);
        return result;
    }

    public WindowsCommandResult SetWindowMinimized(
        SetWindowMinimizedCommand command,
        Guid correlationId)
    {
        var started = Stopwatch.GetTimestamp();
        var window = new IntPtr(command.WindowHandle);
        var result = NativeMethods.SetWindowMinimized(window);
        var after = NativeMethods.CaptureWindowPresentation(window);
        _log.Write(
            result.Success ? "info" : "error",
            result.Success
                ? "download.browser_startup.main_window_native_minimized"
                : "download.browser_startup.main_window_native_minimize_failed",
            correlationId,
            new Dictionary<string, object?>
            {
                ["window_handle"] = command.WindowHandle,
                ["success"] = result.Success,
                ["error_present"] = !string.IsNullOrEmpty(result.Error),
                ["after_visible"] = after.Visible,
                ["foreground_window_handle"] = after.ForegroundWindowHandle,
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            });
        return result;
    }

    public WindowsCommandResult ToggleWindowMaximized(ToggleWindowMaximizedCommand command) =>
        NativeMethods.ToggleWindowMaximized(new IntPtr(command.WindowHandle));

    internal static WindowsCommandResult SetNativeWindowMaximizedForTest(
        long windowHandle,
        bool maximized) =>
        NativeMethods.SetWindowMaximized(new IntPtr(windowHandle), maximized);

    internal static WindowsCommandResult ToggleNativeWindowMaximizedForTest(long windowHandle) =>
        NativeMethods.ToggleWindowMaximized(new IntPtr(windowHandle));

    internal static WindowsCommandResult MinimizeNativeWindowForTest(long windowHandle) =>
        NativeMethods.SetWindowMinimized(new IntPtr(windowHandle));

    internal static WindowsCommandResult BeginNativeWindowMoveForTest(long windowHandle) =>
        NativeMethods.BeginWindowMove(new IntPtr(windowHandle));

    internal static NativeWindowRectangle CenteredRectangleForTest(
        NativeWindowRectangle anchor,
        int width,
        int height)
        => NativeMethods.CenteredRectangle(anchor, width, height);

    internal static WindowsCommandResult SetNativeWindowPositionForTest(
        long windowHandle,
        int x,
        int y) =>
        NativeMethods.SetWindowPosition(new IntPtr(windowHandle), x, y);

    internal static WindowsCommandResult SetNativeWindowBoundsCenteredForTest(
        long windowHandle,
        int width,
        int height) =>
        SetWindowBoundsCenteredPreservingCenter(
            new IntPtr(windowHandle), width, height);

    internal static NativeWindowPresentationState CaptureNativeWindowPresentationForTest(
        long windowHandle) =>
        NativeMethods.CaptureWindowPresentation(new IntPtr(windowHandle));

    private void LogWindowPresentation(
        string eventName,
        long windowHandle,
        NativeWindowPresentationState before,
        NativeWindowPresentationState after,
        WindowsCommandResult result,
        Dictionary<string, object?> fields,
        Guid? diagnosticCorrelationId = null)
    {
        fields["window_handle"] = windowHandle;
        fields["success"] = result.Success;
        fields["error"] = result.Error;
        fields["before_visible"] = before.Visible;
        fields["after_visible"] = after.Visible;
        fields["before_topmost"] = before.Topmost;
        fields["after_topmost"] = after.Topmost;
        fields["before_cloaked"] = before.Cloaked;
        fields["after_cloaked"] = after.Cloaked;
        fields["before_owner_window_handle"] = before.OwnerWindowHandle;
        fields["after_owner_window_handle"] = after.OwnerWindowHandle;
        fields["foreground_window_handle"] = after.ForegroundWindowHandle;
        fields["bounds"] =
            $"{after.Bounds.Left},{after.Bounds.Top},{after.Bounds.Right},{after.Bounds.Bottom}";
        _log.Write(
            result.Success ? "info" : "warning",
            eventName,
            diagnosticCorrelationId,
            fields: fields);
    }

    public WindowsCommandResult SetWindowPosition(SetWindowPositionCommand command) =>
        NativeMethods.SetWindowPosition(new IntPtr(command.WindowHandle), command.X, command.Y);

    public WindowsCommandResult ActivateWindow(ActivateWindowCommand command) =>
        NativeMethods.ActivateWindow(
            new IntPtr(command.WindowHandle),
            new IntPtr(command.WindowHandle))
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                "Windows did not activate and focus the requested GPUI window.");

    internal static bool TryActivateNativeWindow(long windowHandle) =>
        TryActivateNativeWindow(windowHandle, 0);

    internal static bool TryActivateNativeWindow(long windowHandle, long focusWindowHandle) =>
        NativeMethods.ActivateWindow(
            new IntPtr(windowHandle),
            new IntPtr(focusWindowHandle));

    public WindowsCommandResult BeginWindowResize(BeginWindowResizeCommand command) =>
        NativeMethods.BeginWindowResize(new IntPtr(command.WindowHandle));

    public WindowsCommandResult BeginWindowMove(BeginWindowMoveCommand command) =>
        NativeMethods.BeginWindowMove(new IntPtr(command.WindowHandle));

    public Task<WindowsCommandResult> BeginFileDragAsync(
        BeginFileDragCommand command,
        Guid correlationId,
        CancellationToken cancellationToken) =>
        _fileDragCoordinator.BeginAsync(command, correlationId, cancellationToken);

    public Task<WindowsCommandResult> ShowContextMenuAsync(ShowContextMenuCommand command, Guid sessionId,
        CancellationToken cancellationToken) => _contextMenus.ShowAsync(command, sessionId, cancellationToken);

    public WindowsCommandResult CancelContextMenu(CancelContextMenuCommand command) =>
        _contextMenus.Cancel(command.SessionId);

    public WindowsCommandResult OpenDefaultAppsSettings(
        OpenDefaultAppsSettingsCommand command, Guid correlationId, CancellationToken cancellationToken) =>
        new DefaultAppsSettingsOpener(_log).Open(command, correlationId, cancellationToken);

    public Task<WindowsCommandResult> OpenShellTargetAsync(
        OpenShellTargetCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken)
    {
        if (!ShellTarget.TryCreate(
                command.Target, command.Action, out var target, out var validationError))
        {
            return Task.FromResult(new WindowsCommandResult(false, validationError));
        }
        if (cancellationToken.IsCancellationRequested)
        {
            return Task.FromResult(new WindowsCommandResult(false, "The platform command was cancelled."));
        }

        var completion = new TaskCompletionSource<WindowsCommandResult>(
            TaskCreationOptions.RunContinuationsAsynchronously);
        try
        {
            _log.Write("info", "shell_target.dispatch_started", diagnosticCorrelationId, fields: new Dictionary<string, object?>
            {
                ["action"] = command.Action
            });
        }
        catch
        {
            // Diagnostics must not change the command lifecycle.
        }
        var launchThread = new Thread(() =>
        {
            var started = Stopwatch.GetTimestamp();
            WindowsCommandResult result;
            try
            {
                result = target.Launch(stage =>
                {
                    try
                    {
                        _log.Write("info", "shell_target.stage", diagnosticCorrelationId,
                            fields: new Dictionary<string, object?> { ["stage"] = stage });
                    }
                    catch
                    {
                        // Bounded diagnostics must not affect the native Shell operation.
                    }
                });
            }
            catch (Win32Exception error)
            {
                result = new WindowsCommandResult(
                    false,
                    error.Message,
                    NativeErrorCode: error.NativeErrorCode);
            }
            catch (Exception error)
            {
                result = new WindowsCommandResult(
                    false,
                    error.Message,
                    NativeErrorCode: error.HResult);
            }
            completion.TrySetResult(result);
            try
            {
                _log.Write("info", "shell_target.dispatch_completed", diagnosticCorrelationId, fields: new Dictionary<string, object?>
                {
                    ["action"] = command.Action,
                    ["success"] = result.Success,
                    ["stage"] = result.Data ?? (result.Success ? "completed" : "unknown"),
                    ["native_error_code"] = result.NativeErrorCode,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            }
            catch
            {
                // Diagnostics must not change the command lifecycle.
            }
        })
        {
            IsBackground = true,
            Name = "NxTools.ShellTarget"
        };
        try
        {
            launchThread.SetApartmentState(ApartmentState.STA);
            launchThread.Start();
        }
        catch (Exception error)
        {
            completion.TrySetResult(new WindowsCommandResult(
                false,
                error.Message,
                NativeErrorCode: error.HResult));
        }
        return completion.Task;
    }

    public WindowsCommandResult PlaySystemNotificationSound() =>
        _completionSystemActions.PlayNotificationSound();

    public WindowsCommandResult ShutdownSystem(ShutdownSystemCommand command) =>
        _completionSystemActions.RequestShutdown(command.DelaySeconds);

    public WindowsCommandResult ExportBrowserExtension(
        ExportBrowserExtensionCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var started = Stopwatch.GetTimestamp();
        var format = BrowserExtensionPackageService.NormalizeFormat(command.Format);
        var destination = command.Destination;
        if (string.IsNullOrWhiteSpace(destination))
        {
            // Empty destination asks the host to show the save dialog; a
            // cancelled dialog reports success with empty data.
            var picked = BrowserExtensionExportDialog.PickDestination(format, cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            if (!picked.Success)
            {
                return picked;
            }
            destination = picked.Data ?? string.Empty;
            if (string.IsNullOrWhiteSpace(destination))
            {
                _log.Write(
                    "info",
                    "browser_extension.export_cancelled",
                    diagnosticCorrelationId,
                    new Dictionary<string, object?>
                    {
                        ["format"] = format,
                        ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                    });
                return new WindowsCommandResult(true, Data: string.Empty);
            }
        }
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var actualPath = BrowserExtensionPackageService.Export(
                format,
                destination,
                cancellationToken);
            cancellationToken.ThrowIfCancellationRequested();
            _log.Write(
                "info",
                "browser_extension.exported",
                diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["format"] = format,
                    ["file_name"] = Path.GetFileName(actualPath),
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
            return new WindowsCommandResult(true, Data: actualPath);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception error)
        {
            _log.Write(
                "error",
                "browser_extension.export_failed",
                diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["format"] = format,
                    ["error"] = error.Message,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public WindowsCommandResult LaunchProcess(LaunchProcessCommand command)
    {
        try
        {
            Process.Start(CreateProcessStartInfo(command));
            return new WindowsCommandResult(true);
        }
        catch (Exception error) when (error is Win32Exception or InvalidOperationException or ArgumentException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }


    internal static ProcessStartInfo CreateProcessStartInfo(LaunchProcessCommand command)
    {
        if (string.IsNullOrWhiteSpace(command.Executable))
        {
            throw new ArgumentException("The process executable is empty.", nameof(command));
        }
        if (command.Arguments.Count > 256 || command.Arguments.Any(argument => argument.Length > 32_768))
        {
            throw new ArgumentException("The process argument list exceeds the supported limit.", nameof(command));
        }
        var startInfo = new ProcessStartInfo
        {
            FileName = command.Executable,
            UseShellExecute = true,
            Verb = command.RunAsAdmin ? "runas" : string.Empty,
            WindowStyle = command.RunSilently
                ? ProcessWindowStyle.Hidden
                : ProcessWindowStyle.Normal
        };
        foreach (var argument in command.Arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }
        if (!string.IsNullOrWhiteSpace(command.WorkingDirectory))
        {
            startInfo.WorkingDirectory = command.WorkingDirectory;
        }
        else
        {
            startInfo.WorkingDirectory = Path.GetDirectoryName(command.Executable) ?? string.Empty;
        }
        return startInfo;
    }

    public WindowsCommandResult NavigateDirectory(NavigateDirectoryCommand command) =>
        DirectoryNavigationAdapter.Navigate(
            command,
            _elevatedDirectoryNavigation);

    public WindowsCommandResult OpenFileInDialog(OpenFileInDialogCommand command) =>
        FileDialogWindowAdapter.OpenFile(command);

    public WindowsCommandResult QueryShellActions(
        QueryShellActionsCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken = default) =>
        ExecuteShellOperation(
            "query_shell_actions",
            diagnosticCorrelationId,
            () => _shellActions.QueryActions(command, cancellationToken),
            cancellationToken);

    public WindowsCommandResult QueryShellSubmenu(
        QueryShellSubmenuCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken = default) =>
        ExecuteShellOperation(
            "query_shell_submenu",
            diagnosticCorrelationId,
            () => _shellActions.QuerySubmenu(command, cancellationToken),
            cancellationToken);

    public WindowsCommandResult InvokeShellAction(
        InvokeShellActionCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken = default) =>
        ExecuteShellOperation(
            "invoke_shell_action",
            diagnosticCorrelationId,
            () => _shellActions.InvokeAction(command, cancellationToken),
            cancellationToken);

    public WindowsCommandResult ExtractShellIcon(
        ExtractShellIconCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken = default) =>
        ExecuteShellOperation(
            "extract_shell_icon",
            diagnosticCorrelationId,
            () => ShellIconExtractor.ExtractOnSta(command, cancellationToken),
            cancellationToken);

    private WindowsCommandResult ExecuteShellOperation(
        string operation,
        Guid diagnosticCorrelationId,
        Func<WindowsCommandResult> action,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var started = Stopwatch.GetTimestamp();
        WriteNativeDiagnostic(
            "info",
            $"shell.{operation}.started",
            diagnosticCorrelationId,
            new Dictionary<string, object?>
            {
                ["operation"] = operation,
            });
        try
        {
            var result = action();
            cancellationToken.ThrowIfCancellationRequested();
            WriteNativeDiagnostic(
                result.Success ? "info" : "error",
                $"shell.{operation}.completed",
                diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["operation"] = operation,
                    ["success"] = result.Success,
                    ["native_error_code"] = result.NativeErrorCode,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
            return result;
        }
        catch (Exception error)
        {
            WriteNativeDiagnostic(
                "error",
                $"shell.{operation}.failed",
                diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["operation"] = operation,
                    ["exception_type"] = error.GetType().FullName,
                    ["native_error_code"] = error.HResult,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
            throw;
        }
    }

    private void WriteNativeDiagnostic(
        string level,
        string eventName,
        Guid correlationId,
        IReadOnlyDictionary<string, object?> fields)
    {
        try
        {
            _log.Write(level, eventName, correlationId, fields);
        }
        catch
        {
            // Diagnostics must not change native operation results.
        }
    }

    public WindowsCommandResult PickClipboardArchivePath(
        PickClipboardArchivePathCommand command,
        CancellationToken cancellationToken = default) =>
        ClipboardArchiveFilePicker.Pick(command, cancellationToken);

    public WindowsCommandResult PickClipboardStorageDirectory(
        PickClipboardStorageDirectoryCommand command,
        CancellationToken cancellationToken = default) =>
        ClipboardStorageDirectoryPicker.Pick(command, cancellationToken);

    public WindowsCommandResult PickDirectory(
        PickDirectoryCommand command,
        CancellationToken cancellationToken = default) =>
        DirectoryPicker.Pick(command, cancellationToken);

    public WindowsCommandResult PickProgram(
        PickProgramCommand command,
        CancellationToken cancellationToken = default) =>
        ProgramPicker.Pick(command, cancellationToken);

    public WindowsCommandResult PickExtensionManifest(
        PickExtensionManifestCommand command,
        CancellationToken cancellationToken = default,
        Guid diagnosticCorrelationId = default)
    {
        var clock = Stopwatch.StartNew();
        ExtensionRuntimeDiagnostics.Write(_log, "manifest_pick", "accepted", diagnosticCorrelationId, clock);
        var result = ExtensionManifestPicker.Pick(command, cancellationToken);
        ExtensionRuntimeDiagnostics.Write(_log, "manifest_pick", result.Success ? "completed" : result.NativeErrorCode == 995 ? "cancelled" : "failed",
            diagnosticCorrelationId, clock, result.Success ? (string.IsNullOrEmpty(result.Data) ? "dismissed" : "selected") : "picker_failure", result.NativeErrorCode);
        return result;
    }

    public WindowsCommandResult PickExtensionRuntime(PickExtensionRuntimeCommand command,
        Guid diagnosticCorrelationId, CancellationToken cancellationToken = default) =>
        ExtensionRuntimePicker.Pick(command, cancellationToken, _log, diagnosticCorrelationId);

    public Task<WindowsCommandResult> InspectExtensionRuntimeAsync(InspectExtensionRuntimeCommand command,
        Guid diagnosticCorrelationId, CancellationToken cancellationToken = default) =>
        _extensionRuntimeInspector.InspectAsync(command, diagnosticCorrelationId, cancellationToken);

    public Task<WindowsCommandResult> EnsureIndexerServiceAsync(
        EnsureIndexerServiceCommand command,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken) =>
        IndexerServiceInstaller.EnsureAsync(_log, diagnosticCorrelationId, cancellationToken);

    public WindowsCommandResult EnsureBrowserNativeMessaging(
        EnsureBrowserNativeMessagingCommand command,
        Guid correlationId) =>
        BrowserNativeMessagingRegistrar.Ensure(command, _log, correlationId);

    /// <summary>
    /// P206 explicit Apply/Revert/Status. The result payload is the observed state; a
    /// gated or failed apply is a successful transport reply carrying a non-applied state.
    /// </summary>
    public WindowsCommandResult ArchiveShellIntegration(
        Archive.ArchiveShellIntegrationCommand command,
        Guid correlationId)
    {
        _archiveIntegration ??= Archive.ArchiveIntegrationService.CreateProduct(_log);
        var result = _archiveIntegration.Execute(command, correlationId);
        return new WindowsCommandResult(true, Data: Archive.ArchiveIntegrationService.Serialize(result));
    }

    private Archive.ArchiveIntegrationService? _archiveIntegration;

    internal static string? ResolveApprovedDesktopExecutablePath()
        => ResolveApprovedDesktopExecutablePath(
            Environment.GetEnvironmentVariable(DesktopExecutableEnvironmentVariable),
            Path.Combine(IndexerServiceInstaller.ProtectedServiceDirectory(), "NxTools.exe"),
            Environment.ProcessPath ?? string.Empty,
            NxTools.PlatformHost.Generated.ReleaseBinaryManifest.DesktopSha256,
            IndexerServiceInstaller.IsApprovedProtectedDesktop);

    internal static string? ResolveApprovedDesktopExecutablePath(
        string? configuredDesktopExecutable,
        string protectedDesktopExecutable,
        string platformHostExecutable,
        string expectedDesktopHash,
        Func<string, bool> isApprovedProtected)
    {
        var currentPackage = ResolveApprovedDesktopExecutablePath(
            configuredDesktopExecutable,
            protectedDesktopExecutable,
            candidate => IsVerifiedCurrentDesktop(candidate, platformHostExecutable, expectedDesktopHash));
        return currentPackage ?? ResolveApprovedDesktopExecutablePath(
            null, protectedDesktopExecutable, isApprovedProtected);
    }

    private static bool IsVerifiedCurrentDesktop(
        string candidate,
        string platformHostExecutable,
        string expectedDesktopHash)
    {
        try
        {
            if (expectedDesktopHash.Length != 64 || !expectedDesktopHash.All(char.IsAsciiHexDigit)
                || string.IsNullOrWhiteSpace(platformHostExecutable)) return false;
            var hostPath = Path.GetFullPath(platformHostExecutable);
            if (!string.Equals(Path.GetFileName(hostPath), "NxTools.PlatformHost.exe", StringComparison.OrdinalIgnoreCase)
                || !File.Exists(hostPath)) return false;
            var desktopPath = Path.GetFullPath(candidate);
            var sibling = Path.Combine(Path.GetDirectoryName(hostPath)!, "NxTools.exe");
            if (!string.Equals(desktopPath, sibling, StringComparison.OrdinalIgnoreCase)) return false;
            return IndexerServiceInstaller.VerifiedPackagedBinaryHashMatches(desktopPath, expectedDesktopHash);
        }
        catch (Exception error) when (error is IOException or InvalidDataException or UnauthorizedAccessException
            or System.Security.SecurityException or ArgumentException or NotSupportedException)
        {
            return false;
        }
    }

    internal static string? ResolveApprovedDesktopExecutablePath(
        string? configuredDesktopExecutable,
        string protectedDesktopExecutable,
        Func<string, bool> isApproved)
    {
        foreach (var candidate in new[] { configuredDesktopExecutable, protectedDesktopExecutable })
        {
            if (string.IsNullOrWhiteSpace(candidate))
            {
                continue;
            }
            string fullPath;
            try
            {
                fullPath = Path.GetFullPath(candidate);
            }
            catch (ArgumentException)
            {
                continue;
            }
            if (File.Exists(fullPath) && isApproved(fullPath))
            {
                return fullPath;
            }
        }
        return null;
    }


    internal static WindowsCommandResult CreateDesktopShellApplyFailure(
        Exception error,
        string failureStage)
    {
        int nativeErrorCode = error switch
        {
            Win32Exception win32 => win32.NativeErrorCode,
            ExternalException external => external.ErrorCode,
            _ => error.HResult
        };
        var (message, stage) = failureStage switch
        {
            "startup_registration" => (
                "Windows could not update the current-user startup registration.",
                "startup_registration_failed"),
            "tray_icon" => (
                "Windows could not create or show the NxTools notification-area icon.",
                "tray_icon_failed"),
            _ => (
                "Windows could not apply desktop shell settings.",
                "windows_shell_apply_failed")
        };
        return new WindowsCommandResult(
            false,
            message,
            NativeErrorCode: nativeErrorCode,
            Data: stage);
    }

    internal static WindowsCommandResult ApplyStartupRegistration(
        RegistryKey runKey,
        string desktopExecutable,
        bool enabled)
    {
        try
        {
            if (enabled)
            {
                if (!File.Exists(desktopExecutable))
                {
                    return new WindowsCommandResult(
                        false,
                        "Packaged NxTools desktop executable is unavailable.",
                        Data: "desktop_executable_unavailable");
                }
                runKey.SetValue("NxTools.Next", $"\"{desktopExecutable}\" --silent-start");
            }
            else
            {
                runKey.DeleteValue("NxTools.Next", throwOnMissingValue: false);
            }
            runKey.DeleteValue("NxTools", throwOnMissingValue: false);
            return new WindowsCommandResult(true);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return new WindowsCommandResult(
                false,
                "Windows could not update the current-user startup registration.",
                NativeErrorCode: error.HResult,
                Data: "startup_registration_failed");
        }
    }

    internal static DesktopTrayCandidate CreateTrayIcon(
        string desktopExecutable,
        Action open,
        Action exit)
        => CreateTrayIconCore(
            () => DesktopExecutableIcon.ExtractProductIcon(desktopExecutable),
            () => new ContextMenuStrip(),
            () => new NotifyIcon(),
            open,
            exit);

    internal static DesktopTrayCandidate CreateTrayIconForTest(
        Action open,
        Action exit,
        Func<ContextMenuStrip> createMenu,
        Func<NotifyIcon> createTrayIcon)
        => CreateTrayIconCore(
            () => (Icon)SystemIcons.Application.Clone(),
            createMenu,
            createTrayIcon,
            open,
            exit);

    private static DesktopTrayCandidate CreateTrayIconCore(
        Func<Icon?> createIcon,
        Func<ContextMenuStrip> createMenu,
        Func<NotifyIcon> createTrayIcon,
        Action open,
        Action exit)
    {
        Icon? icon = null;
        ContextMenuStrip? menu = null;
        NotifyIcon? trayIcon = null;
        try
        {
            icon = createIcon()
                ?? throw new InvalidOperationException(
                    "The NxTools desktop executable does not expose its product icon.");
            menu = createMenu();
            menu.Items.Add(DesktopTrayPresentation.OpenLabel, null, (_, _) => open());
            menu.Items.Add(new ToolStripSeparator());
            menu.Items.Add(DesktopTrayPresentation.ExitLabel, null, (_, _) => exit());
            trayIcon = createTrayIcon();
            trayIcon.Text = DesktopTrayPresentation.Tooltip;
            trayIcon.Icon = icon;
            trayIcon.ContextMenuStrip = menu;
            trayIcon.MouseClick += (_, eventArgs) =>
            {
                if (eventArgs.Button == MouseButtons.Left)
                {
                    open();
                }
            };
            return new DesktopTrayCandidate(trayIcon, icon);
        }
        catch
        {
            DisposeTrayCandidateWithoutThrowing(trayIcon, menu, icon);
            throw;
        }
    }

    private static void DisposeTrayCandidateWithoutThrowing(
        NotifyIcon? trayIcon,
        ContextMenuStrip? menu,
        Icon? icon)
    {
        try { trayIcon?.Dispose(); } catch { }
        try { menu?.Dispose(); } catch { }
        try { icon?.Dispose(); } catch { }
    }

    internal static bool IsQuickLaunchContextRequestCurrent(
        long requestedAtUnixMs,
        long nowUnixMs) =>
        requestedAtUnixMs > 0 &&
        nowUnixMs >= requestedAtUnixMs &&
        nowUnixMs - requestedAtUnixMs <=
            QuickLaunchContextMaximumAgeMilliseconds;

    private void RunMessageLoop(
        ForegroundContextProvider foregroundContextProvider,
        IElevatedDirectoryNavigationChannel elevatedDirectoryNavigation)
    {
        try
        {
            _window = new EventWindow(
                _events.Writer,
                foregroundContextProvider,
                elevatedDirectoryNavigation,
                _log,
                _clipboardCaptureDropDiagnosticGate,
                _contextMenus);
            _started.Set();
            Application.Run();
        }
        catch (Exception error)
        {
            _startupError = error;
            _started.Set();
        }
        finally
        {
            _window?.StopClipboardCapture();
            _events.Writer.TryComplete(_startupError);
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        _window?.CancelClipboardPauseRequests();
        _shellActions.Dispose();
        _contextMenus.Dispose();
        _extensionRuntimeInspector.Dispose();
        _window?.RequestStop();
        if (!_thread.Join(TimeSpan.FromSeconds(5)))
        {
            Console.Error.WriteLine("Windows platform adapter did not stop within five seconds.");
        }
        _elevatedDirectoryNavigation.Dispose();
        _started.Dispose();
    }
}
