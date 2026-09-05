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
    NativeWindowRectangle Bounds);

internal sealed class WindowsPlatformAdapter : IDisposable
{
    private const long QuickLaunchContextMaximumAgeMilliseconds = 1_000;
    internal static int NativeInputSize => NativeMethods.InputSize;
    internal static int BottomRightResizeSystemCommand => 0xF008;
    internal static int MoveCaptionSystemCommand => 0xF012;

    private readonly Channel<WindowsPlatformEvent> _events = Channel.CreateBounded<WindowsPlatformEvent>(
        new BoundedChannelOptions(512)
        {
            FullMode = BoundedChannelFullMode.DropOldest,
            SingleReader = true,
            SingleWriter = false
        });
    private readonly ManualResetEventSlim _started = new(false);
    private readonly Thread _thread;
    private readonly IPlatformLog _log;
    private readonly ShellContextMenuSessionManager _shellActions = new();
    private readonly IElevatedDirectoryNavigationChannel _elevatedDirectoryNavigation;
    private readonly NativeFileDragCoordinator _fileDragCoordinator;
    private readonly TaskbarProgressLogGate _taskbarProgressLogGate = new();
    private readonly ICompletionSystemActions _completionSystemActions;
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

    public bool ClipboardHotKeyAvailable => _window?.ClipboardHotKeyAvailable ?? false;
    public bool QuickLaunchHotKeyAvailable => _window?.QuickLaunchHotKeyAvailable ?? false;

    public Task<WindowsCommandResult> ConfigureClipboardAsync(
        ConfigureClipboardCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueClipboardConfiguration(command, cancellationToken);

    public Task<WindowsCommandResult> SetClipboardHotKeyRecordingAsync(
        SetClipboardHotKeyRecordingCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueClipboardHotKeyRecording(command, cancellationToken);

    public Task<WindowsCommandResult> ConfigureQuickLaunchAsync(
        ConfigureQuickLaunchCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueQuickLaunchConfiguration(command, cancellationToken);

    public Task<WindowsCommandResult> SetQuickLaunchHotKeyRecordingAsync(
        SetQuickLaunchHotKeyRecordingCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueQuickLaunchHotKeyRecording(command, cancellationToken);

    public Task<WindowsCommandResult> ConfigureDesktopShellAsync(
        ConfigureDesktopShellCommand command,
        CancellationToken cancellationToken)
        => (_window ?? throw new InvalidOperationException("Windows message window is unavailable."))
            .EnqueueDesktopShellConfiguration(command, cancellationToken);

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

    public WindowsCommandResult SetWindowNoActivate(SetWindowNoActivateCommand command) =>
        NativeMethods.SetWindowNoActivate(new IntPtr(command.WindowHandle), command.Enabled);

    public WindowsCommandResult SetWindowBounds(SetWindowBoundsCommand command) =>
        NativeMethods.SetWindowBounds(
            new IntPtr(command.WindowHandle),
            command.X,
            command.Y,
            command.Width,
            command.Height);

    public WindowsCommandResult SetWindowBoundsCentered(SetWindowBoundsCenteredCommand command) =>
        NativeMethods.SetWindowBoundsCentered(
            new IntPtr(command.WindowHandle),
            command.Width,
            command.Height);

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

    public WindowsCommandResult SetWindowVisibility(SetWindowVisibilityCommand command)
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
            });
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

    private void LogWindowPresentation(
        string eventName,
        long windowHandle,
        NativeWindowPresentationState before,
        NativeWindowPresentationState after,
        WindowsCommandResult result,
        Dictionary<string, object?> fields)
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
        _log.Write(result.Success ? "info" : "warning", eventName, fields: fields);
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

    public WindowsCommandResult OpenShellTarget(OpenShellTargetCommand command)
    {
        if (!ShellTarget.TryCreate(
                command.Target, command.Action, out var target, out var validationError))
        {
            return new WindowsCommandResult(false, validationError);
        }
        try
        {
            target.Launch();
            return new WindowsCommandResult(true);
        }
        catch (Win32Exception error)
        {
            return new WindowsCommandResult(
                false,
                error.Message,
                NativeErrorCode: error.NativeErrorCode);
        }
        catch (InvalidOperationException error)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public WindowsCommandResult PlaySystemNotificationSound() =>
        _completionSystemActions.PlayNotificationSound();

    public WindowsCommandResult ShutdownSystem(ShutdownSystemCommand command) =>
        _completionSystemActions.RequestShutdown(command.DelaySeconds);

    public WindowsCommandResult ExportBrowserExtension(ExportBrowserExtensionCommand command)
    {
        var started = Stopwatch.GetTimestamp();
        var format = string.Equals(
            command.Format,
            BrowserExtensionPackageService.FirefoxFormat,
            StringComparison.OrdinalIgnoreCase)
            ? BrowserExtensionPackageService.FirefoxFormat
            : BrowserExtensionPackageService.ChromeFormat;
        var destination = command.Destination;
        if (string.IsNullOrWhiteSpace(destination))
        {
            // Empty destination asks the host to show the save dialog; a
            // cancelled dialog reports success with empty data.
            var picked = BrowserExtensionExportDialog.PickDestination(format);
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
                    null,
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
            var actualPath = BrowserExtensionPackageService.Export(format, destination);
            _log.Write(
                "info",
                "browser_extension.exported",
                null,
                new Dictionary<string, object?>
                {
                    ["format"] = format,
                    ["file_name"] = Path.GetFileName(actualPath),
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds,
                });
            return new WindowsCommandResult(true, Data: actualPath);
        }
        catch (Exception error)
        {
            _log.Write(
                "error",
                "browser_extension.export_failed",
                null,
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

    public WindowsCommandResult QueryShellActions(QueryShellActionsCommand command) =>
        _shellActions.QueryActions(command);

    public WindowsCommandResult QueryShellSubmenu(QueryShellSubmenuCommand command) =>
        _shellActions.QuerySubmenu(command);

    public WindowsCommandResult InvokeShellAction(InvokeShellActionCommand command) =>
        _shellActions.InvokeAction(command);

    public WindowsCommandResult ExtractShellIcon(ExtractShellIconCommand command) =>
        ShellIconExtractor.Extract(command);

    public WindowsCommandResult PickClipboardArchivePath(
        PickClipboardArchivePathCommand command) => ClipboardArchiveFilePicker.Pick(command);

    public WindowsCommandResult PickClipboardStorageDirectory(
        PickClipboardStorageDirectoryCommand command) => ClipboardStorageDirectoryPicker.Pick(command);

    public WindowsCommandResult PickDirectory(PickDirectoryCommand command) =>
        DirectoryPicker.Pick(command);

    public WindowsCommandResult PickProgram(PickProgramCommand command) =>
        ProgramPicker.Pick(command);

    public Task<WindowsCommandResult> EnsureIndexerServiceAsync(
        EnsureIndexerServiceCommand command,
        CancellationToken cancellationToken) =>
        IndexerServiceInstaller.EnsureAsync(_log, cancellationToken);

    public WindowsCommandResult EnsureBrowserNativeMessaging(
        EnsureBrowserNativeMessagingCommand command,
        Guid correlationId) =>
        BrowserNativeMessagingRegistrar.Ensure(command, _log, correlationId);

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
                        "Packaged NxTools desktop executable is unavailable.");
                }
                runKey.SetValue("NxTools.Next", $"\"{desktopExecutable}\" --silent-start");
            }
            else
            {
                runKey.DeleteValue("NxTools.Next", throwOnMissingValue: false);
            }
            return new WindowsCommandResult(true);
        }
        catch (Exception error) when (error is IOException or UnauthorizedAccessException or System.Security.SecurityException)
        {
            return new WindowsCommandResult(false, error.Message, NativeErrorCode: error.HResult);
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
                _log);
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
        _shellActions.Dispose();
        _window?.RequestStop();
        if (!_thread.Join(TimeSpan.FromSeconds(5)))
        {
            Console.Error.WriteLine("Windows platform adapter did not stop within five seconds.");
        }
        _elevatedDirectoryNavigation.Dispose();
        _started.Dispose();
    }

    private sealed class EventWindow : NativeWindow, IWindowsPasteOperations
    {
        private const int WmClipboardUpdate = 0x031D;
        private const int WmHotKey = 0x0312;
        private const int WmClose = 0x0010;
        private const int WmTimer = 0x0113;
        private const int WmExecutePaste = 0x8001;
        private const int WmConfigureClipboard = 0x8002;
        private const int WmSetClipboardHotKeyRecording = 0x8003;
        private const int WmConfigureQuickLaunch = 0x8004;
        private const int WmSetQuickLaunchHotKeyRecording = 0x8005;
        private const int WmQuickLaunchGesture = 0x8006;
        private const int WmDirectorySourceResolved = 0x8007;
        private const int WmFileDialogStructureChanged = 0x8008;
        private const int WmFileDialogGeometryChanged = 0x8009;
        private const int WmTaskbarOverlayChanged = 0x800A;
        private const int WmConfigureDesktopShell = 0x800B;
        private const int WmSettingChange = 0x001A;
        private const int WmDisplayChange = 0x007E;
        private const uint EventSystemForeground = 0x0003;
        private const uint EventSystemMoveSizeEnd = 0x000B;
        private const uint EventObjectCreate = 0x8000;
        private const uint EventObjectDestroy = 0x8001;
        private const uint EventObjectShow = 0x8002;
        private const uint EventObjectHide = 0x8003;
        private const uint EventObjectLocationChange = 0x800B;
        private const uint EventObjectNameChange = 0x800C;
        private const int ObjIdWindow = 0;
        private const uint WineventOutOfContext = 0x0000;
        private const uint RetryTimerId = 1;
        private const uint PointerMonitorTimerId = 2;
        private const uint FileDialogRecognitionTimerId = 3;
        private const uint DirectorySourceRecognitionTimerId = 4;
        private const uint PointerMonitorIntervalMs = 40;
        private const uint FileDialogRecognitionIntervalMs = 30;
        private const int FileDialogRecognitionMaxAttempts = 200;
        private const int DirectorySourceRecognitionMaxAttempts = 20;
        private const int MaxClipboardReadAttempts = 5;
        private const int ClipboardHotKeyId = 1;
        private const int QuickLaunchHotKeyId = 2;
        private const uint ModAlt = 0x0001;
        private const uint ModNoRepeat = 0x4000;
        private const uint VkC = 0x43;
        private const uint VkSpace = 0x20;
        private const int VkLeftControl = 0xA2;
        private const int VkRightControl = 0xA3;
        private static readonly IntPtr HwndMessage = new(-3);

        private readonly ChannelWriter<WindowsPlatformEvent> _events;
        private readonly ForegroundContextProvider _foregroundContextProvider;
        private readonly IElevatedDirectoryNavigationChannel _elevatedDirectoryNavigation;
        private readonly IPlatformLog _log;
        private readonly AutoQuickSwitchTracker _autoQuickSwitch = new();
        private readonly FileDialogRecognitionRetry _fileDialogRecognitionRetry =
            new(FileDialogRecognitionMaxAttempts);
        private readonly FileDialogRecognitionRetry _directorySourceRecognitionRetry =
            new(DirectorySourceRecognitionMaxAttempts);
        private readonly FileDialogGeometryUpdateGate _fileDialogGeometryUpdateGate = new();
        private readonly ConcurrentQueue<PasteRequest> _pasteRequests = new();
        private readonly ConcurrentQueue<ClipboardConfigurationRequest> _configurationRequests = new();
        private readonly ConcurrentQueue<ClipboardHotKeyRecordingRequest> _hotKeyRecordingRequests = new();
        private readonly ConcurrentQueue<QuickLaunchConfigurationRequest> _quickLaunchConfigurationRequests = new();
        private readonly ConcurrentQueue<QuickLaunchHotKeyRecordingRequest> _quickLaunchHotKeyRecordingRequests = new();
        private readonly ConcurrentQueue<DesktopShellConfigurationRequest> _desktopShellConfigurationRequests = new();
        private readonly ConcurrentQueue<(long WindowHandle, string Directory, string DisplayName)>
            _resolvedDirectorySources = new();
        private readonly NativeMethods.WinEventDelegate _foregroundCallback;
        private readonly NativeMethods.WinEventDelegate _fileDialogWindowCallback;
        private IntPtr _foregroundHook;
        private IntPtr _fileDialogCreateHook;
        private IntPtr _fileDialogDestroyHook;
        private IntPtr _fileDialogShowHook;
        private IntPtr _fileDialogHideHook;
        private IntPtr _fileDialogLocationHook;
        private IntPtr _fileDialogNameHook;
        private IntPtr _taskbarOverlayMoveSizeEndHook;
        private ForegroundContext? _pendingClipboardSource;
        private Task _capturePublishTail = Task.CompletedTask;
        private int _clipboardReadAttempts;
        private bool _suppressNextClipboardUpdate;
        private bool _primaryPointerDown;
        private bool _secondaryPointerDown;
        private int _fileDialogStructureRetryPosted;
        private ClipboardHotKeyRegistration _hotKeyRegistration = null!;
        private ClipboardHotKeyRegistration _quickLaunchHotKeyRegistration = null!;
        private readonly ModifierDoubleTapDetector _leftControlDoubleTap =
            new(VkLeftControl, 100, 500);
        private readonly ModifierDoubleTapDetector _rightControlDoubleTap =
            new(VkRightControl, 100, 500);
        private LowLevelKeyboardMonitor? _quickLaunchKeyboardMonitor;
        private QuickLaunchTriggerBinding _quickLaunchTrigger = new(
            QuickLaunchTriggerKind.DoubleControl,
            "双击 Ctrl",
            null);
        private bool _quickLaunchTriggerEnabled = true;
        private bool _quickLaunchRecording;
        private bool _fileDialogSearchEnabled;
        private bool _autoQuickSwitchEnabled;
        private FileDialogSearchHostSnapshot? _activeFileDialogSearchHost;
        private AutoQuickSwitchDirectorySource? _publishedFileDialogSearchSource;
        private bool _fileDialogSearchHostVisible;
        private long _fileDialogRecognitionWindowHandle;
        private long _fileDialogRecognitionStartedAt;
        private PlaceFileDialogSearchWindowCommand? _fileDialogSearchPlacement;
        private PlaceTaskbarProgressOverlayCommand? _taskbarOverlayPlacement;
        private Guid? _taskbarOverlayCorrelationId;
        private string? _taskbarOverlayAppliedBounds;
        private ClipboardCaptureExclusions _captureExclusions = ClipboardCaptureExclusions.Create(
            ["KeePass.exe", "1Password.exe", "Bitwarden.exe"]);
        private readonly DesktopTrayController _desktopTray = new();

        public bool ClipboardHotKeyAvailable => _hotKeyRegistration?.IsAvailable == true;
        public bool QuickLaunchHotKeyAvailable => _quickLaunchTriggerEnabled && !_quickLaunchRecording &&
            (_quickLaunchTrigger.Kind == QuickLaunchTriggerKind.DoubleControl
                ? _quickLaunchKeyboardMonitor?.IsInstalled == true
                : _quickLaunchHotKeyRegistration?.IsAvailable == true);

        public EventWindow(
            ChannelWriter<WindowsPlatformEvent> events,
            ForegroundContextProvider foregroundContextProvider,
            IElevatedDirectoryNavigationChannel elevatedDirectoryNavigation,
            IPlatformLog log)
        {
            _events = events;
            _foregroundContextProvider = foregroundContextProvider;
            _elevatedDirectoryNavigation = elevatedDirectoryNavigation;
            _log = log;
            _foregroundCallback = OnForegroundChanged;
            _fileDialogWindowCallback = OnFileDialogWindowChanged;
            CreateHandle(new CreateParams
            {
                Caption = "NxTools.PlatformHost.Events",
                Parent = HwndMessage
            });
            if (!NativeMethods.AddClipboardFormatListener(Handle))
            {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            _foregroundHook = NativeMethods.SetWinEventHook(
                EventSystemForeground,
                EventSystemForeground,
                IntPtr.Zero,
                _foregroundCallback,
                0,
                0,
                WineventOutOfContext);
            if (_foregroundHook == IntPtr.Zero)
            {
                _ = NativeMethods.RemoveClipboardFormatListener(Handle);
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            _fileDialogCreateHook = InstallOptionalWindowEventHook(EventObjectCreate);
            _fileDialogDestroyHook = InstallOptionalWindowEventHook(EventObjectDestroy);
            _fileDialogShowHook = InstallOptionalWindowEventHook(EventObjectShow);
            _fileDialogHideHook = InstallOptionalWindowEventHook(EventObjectHide);
            _fileDialogLocationHook = InstallOptionalWindowEventHook(EventObjectLocationChange);
            _fileDialogNameHook = InstallOptionalWindowEventHook(EventObjectNameChange);
            _taskbarOverlayMoveSizeEndHook = InstallOptionalWindowEventHook(EventSystemMoveSizeEnd);
            SystemEvents.DisplaySettingsChanged += OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged += OnSystemUserPreferenceChanged;

            _hotKeyRegistration = new ClipboardHotKeyRegistration(
                new NativeClipboardHotKeyRegistrar(() => Handle, ClipboardHotKeyId),
                new ClipboardHotKeyBinding(ModAlt, VkC, "Alt+C"));
            var hotKeyInitialization = _hotKeyRegistration.Initialize();
            if (!hotKeyInitialization.Success)
            {
                Console.Error.WriteLine(hotKeyInitialization.Error);
            }
            _quickLaunchHotKeyRegistration = new ClipboardHotKeyRegistration(
                new NativeClipboardHotKeyRegistrar(() => Handle, QuickLaunchHotKeyId),
                new ClipboardHotKeyBinding(ModAlt, VkSpace, "Alt+Space"));
            try
            {
                _quickLaunchKeyboardMonitor = new LowLevelKeyboardMonitor(
                    OnGlobalKeyDown,
                    OnGlobalKeyUp);
            }
            catch (Win32Exception error)
            {
                _quickLaunchTriggerEnabled = false;
                _log.Write("error", "quick_launch.keyboard_hook_failed", null,
                    new Dictionary<string, object?>
                    {
                        ["win32_error"] = error.NativeErrorCode,
                        ["message"] = error.Message
                    });
            }

            _ = NativeMethods.SetTimer(
                Handle,
                (UIntPtr)PointerMonitorTimerId,
                PointerMonitorIntervalMs,
                IntPtr.Zero);

            ProcessForeground(_foregroundContextProvider.Capture());
        }

        public Task<WindowsCommandResult> EnqueuePaste(
            ClipboardPastePayload command,
            Guid correlationId,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _pasteRequests.Enqueue(new PasteRequest(command, correlationId, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmExecutePaste, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(false, "Failed to schedule paste on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueClipboardConfiguration(
            ConfigureClipboardCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _configurationRequests.Enqueue(new ClipboardConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureClipboard, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule clipboard configuration on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueClipboardHotKeyRecording(
            SetClipboardHotKeyRecordingCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _hotKeyRecordingRequests.Enqueue(new ClipboardHotKeyRecordingRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(
                    Handle, WmSetClipboardHotKeyRecording, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule clipboard hotkey recording on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueQuickLaunchConfiguration(
            ConfigureQuickLaunchCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _quickLaunchConfigurationRequests.Enqueue(new QuickLaunchConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureQuickLaunch, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule quick-launch configuration on the Windows thread."));
            }
            return completion.Task;
        }

        public Task<WindowsCommandResult> EnqueueQuickLaunchHotKeyRecording(
            SetQuickLaunchHotKeyRecordingCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _quickLaunchHotKeyRecordingRequests.Enqueue(new QuickLaunchHotKeyRecordingRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(
                    Handle, WmSetQuickLaunchHotKeyRecording, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false, "Failed to schedule quick-launch hotkey recording on the Windows thread."));
            }
            return completion.Task;
        }

        public void RequestStop()
        {
            _ = NativeMethods.PostMessage(Handle, WmClose, IntPtr.Zero, IntPtr.Zero);
        }

        public Task<WindowsCommandResult> EnqueueDesktopShellConfiguration(
            ConfigureDesktopShellCommand command,
            CancellationToken cancellationToken)
        {
            var completion = new TaskCompletionSource<WindowsCommandResult>(
                TaskCreationOptions.RunContinuationsAsynchronously);
            _desktopShellConfigurationRequests.Enqueue(new DesktopShellConfigurationRequest(
                command, cancellationToken, completion));
            if (!NativeMethods.PostMessage(Handle, WmConfigureDesktopShell, IntPtr.Zero, IntPtr.Zero))
            {
                completion.TrySetResult(new WindowsCommandResult(
                    false,
                    "Failed to schedule desktop-shell configuration on the Windows thread.",
                    NativeErrorCode: Marshal.GetLastWin32Error()));
            }
            return completion.Task;
        }

        public void TrackFileDialogSearchPlacement(
            PlaceFileDialogSearchWindowCommand command)
        {
            Volatile.Write(ref _fileDialogSearchPlacement, command);
        }

        public void TrackTaskbarOverlayPlacement(
            PlaceTaskbarProgressOverlayCommand command,
            Guid correlationId,
            string? appliedBounds)
        {
            Volatile.Write(ref _taskbarOverlayPlacement, command);
            _taskbarOverlayCorrelationId = correlationId;
            _taskbarOverlayAppliedBounds = appliedBounds;
        }

        public void TrackTaskbarOverlayVisibility(
            SetTaskbarProgressOverlayVisibilityCommand command)
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement?.WindowHandle == command.WindowHandle)
            {
                Volatile.Write(
                    ref _taskbarOverlayPlacement,
                    placement with { Visible = command.Visible });
            }
        }

        public void ForgetTaskbarOverlayPlacement(long windowHandle)
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement?.WindowHandle != windowHandle ||
                !ReferenceEquals(
                    Interlocked.CompareExchange(ref _taskbarOverlayPlacement, null, placement),
                    placement))
            {
                return;
            }
            _taskbarOverlayCorrelationId = null;
            _taskbarOverlayAppliedBounds = null;
        }

        protected override void WndProc(ref Message message)
        {
            switch (message.Msg)
            {
                case WmClipboardUpdate:
                    if (_suppressNextClipboardUpdate)
                    {
                        _suppressNextClipboardUpdate = false;
                        return;
                    }
                    _pendingClipboardSource = _foregroundContextProvider.Capture();
                    _clipboardReadAttempts = 0;
                    TryPublishClipboardContent();
                    return;
                case WmHotKey when message.WParam.ToInt32() == ClipboardHotKeyId:
                    PublishPanelRequest();
                    return;
                case WmHotKey when message.WParam.ToInt32() == QuickLaunchHotKeyId:
                    PublishQuickLaunchPanelRequest();
                    return;
                case WmTimer when message.WParam.ToInt64() == RetryTimerId:
                    _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
                    TryPublishClipboardContent();
                    return;
                case WmTimer when message.WParam.ToInt64() == PointerMonitorTimerId:
                    MonitorPointerPress();
                    return;
                case WmTimer when message.WParam.ToInt64() == FileDialogRecognitionTimerId:
                    _ = NativeMethods.KillTimer(
                        Handle,
                        (UIntPtr)FileDialogRecognitionTimerId);
                    RetryFileDialogRecognition();
                    return;
                case WmTimer when message.WParam.ToInt64() == DirectorySourceRecognitionTimerId:
                    _ = NativeMethods.KillTimer(
                        Handle,
                        (UIntPtr)DirectorySourceRecognitionTimerId);
                    RetryDirectorySourceRecognition();
                    return;
                case WmFileDialogGeometryChanged:
                    if (_fileDialogGeometryUpdateGate.TryTake(out var geometryRoot))
                    {
                        FollowFileDialogSearchWindow(geometryRoot);
                    }
                    return;
                case WmTaskbarOverlayChanged:
                case WmSettingChange:
                case WmDisplayChange:
                    ReapplyTaskbarOverlayPlacement();
                    if (message.Msg == WmTaskbarOverlayChanged)
                    {
                        return;
                    }
                    break;
                case WmExecutePaste:
                    DrainPasteRequests();
                    return;
                case WmConfigureClipboard:
                    DrainClipboardConfigurationRequests();
                    return;
                case WmSetClipboardHotKeyRecording:
                    DrainClipboardHotKeyRecordingRequests();
                    return;
                case WmConfigureQuickLaunch:
                    DrainQuickLaunchConfigurationRequests();
                    return;
                case WmSetQuickLaunchHotKeyRecording:
                    DrainQuickLaunchHotKeyRecordingRequests();
                    return;
                case WmConfigureDesktopShell:
                    DrainDesktopShellConfigurationRequests();
                    return;
                case WmQuickLaunchGesture:
                    PublishQuickLaunchPanelRequest();
                    return;
                case WmDirectorySourceResolved:
                    DrainResolvedDirectorySources();
                    return;
                case WmFileDialogStructureChanged:
                    Interlocked.Exchange(
                        ref _fileDialogStructureRetryPosted,
                        0);
                    var changedRoot = message.LParam.ToInt64();
                    if (_fileDialogRecognitionRetry
                        .ShouldRetryForStructureChange(changedRoot))
                    {
                        RetryFileDialogRecognition();
                    }
                    else if ((_fileDialogSearchEnabled ||
                              _autoQuickSwitchEnabled) &&
                             NativeMethods.IsForegroundWindowForTarget(
                                 new IntPtr(changedRoot)))
                    {
                        ProcessForeground(
                            _foregroundContextProvider.Capture(
                                new IntPtr(changedRoot)));
                    }
                    return;
                case WmClose:
                    Stop();
                    return;
            }
            base.WndProc(ref message);
        }

        private void TryPublishClipboardContent()
        {
            try
            {
                var source = _pendingClipboardSource ?? _foregroundContextProvider.Capture();
                if (_captureExclusions.Contains(source.ProcessName))
                {
                    _log.Write("info", "clipboard.capture_excluded", null,
                        new Dictionary<string, object?>
                        {
                            ["source_process_name"] = source.ProcessName
                        });
                    return;
                }
                var capturedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                var sequenceBefore = NativeMethods.GetClipboardSequenceNumber();
                var nativeFormats = NativeClipboardFormats.Capture(Handle).ToList();
                var paths = Clipboard.ContainsFileDropList()
                    ? Clipboard.GetFileDropList()
                        .Cast<string>()
                        .Where(static path => !string.IsNullOrWhiteSpace(path))
                        .Take(1024)
                        .ToArray()
                    : [];
                ClipboardImageData? imageData = null;
                var stagingDirectory = Path.Combine(
                    Path.GetTempPath(), "NxTools.Next", "clipboard-staging");
                var promoted = ClipboardImagePromotion.TryPromote(paths, text: null, stagingDirectory);
                if (promoted is not null)
                {
                    imageData = promoted.Image;
                    paths = promoted.RemainingPaths;
                }
                else if (Clipboard.ContainsImage())
                {
                    using var image = Clipboard.GetImage();
                    if (image is not null)
                    {
                        Directory.CreateDirectory(stagingDirectory);
                        var path = Path.Combine(stagingDirectory, $"{Guid.NewGuid():N}.png");
                        image.Save(path, System.Drawing.Imaging.ImageFormat.Png);
                        imageData = new ClipboardImageData(
                            path,
                            checked((uint)image.Width),
                            checked((uint)image.Height));
                    }
                }
                string? text = null;
                if (Clipboard.ContainsText(TextDataFormat.UnicodeText))
                {
                    text = Clipboard.GetText(TextDataFormat.UnicodeText);
                }
                if (imageData is null)
                {
                    promoted = ClipboardImagePromotion.TryPromote(paths, text, stagingDirectory);
                    if (promoted is not null)
                    {
                        imageData = promoted.Image;
                        paths = promoted.RemainingPaths;
                    }
                }
                if (paths.Length > 0 || imageData is not null || !string.IsNullOrEmpty(text))
                {
                    var compatibilityFallbackAdded = TryAddImageCompatibilityFallbacks(
                        sequenceBefore,
                        imageData,
                        paths.Length,
                        text,
                        nativeFormats,
                        source);
                    _log.Write("info", "clipboard.capture_completed", null,
                        new Dictionary<string, object?>
                        {
                            ["sequence_before"] = sequenceBefore,
                            ["sequence_after"] = NativeMethods.GetClipboardSequenceNumber(),
                            ["image_compatibility_fallback_added"] = compatibilityFallbackAdded,
                            ["native_format_count"] = nativeFormats.Count,
                            ["has_image"] = imageData is not null,
                            ["file_count"] = paths.Length,
                            ["has_text"] = !string.IsNullOrEmpty(text),
                            ["source_process_name"] = source.ProcessName
                        });
                    QueueCapturedEvent(text, imageData, paths, nativeFormats, source, capturedAt);
                }
            }
            catch (ExternalException) when (++_clipboardReadAttempts < MaxClipboardReadAttempts)
            {
                _ = NativeMethods.SetTimer(Handle, (UIntPtr)RetryTimerId, 40, IntPtr.Zero);
            }
            catch (Exception error)
            {
                _log.Write("error", "clipboard.capture_failed", null, new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = error.Message
                });
            }
        }

        private bool TryAddImageCompatibilityFallbacks(
            uint expectedSequence,
            ClipboardImageData? imageData,
            int fileCount,
            string? text,
            List<CapturedClipboardNativeFormat> nativeFormats,
            ForegroundContext source)
        {
            var hasText = !string.IsNullOrEmpty(text);
            if (imageData is null ||
                (!ClipboardImageCompatibility.RequiresPngFallback(
                     hasImage: true, hasText, fileCount, nativeFormats) &&
                 !ClipboardImageCompatibility.RequiresFileDropFallback(
                     hasImage: true, hasText, fileCount, nativeFormats)))
            {
                return false;
            }
            var actualSequence = NativeMethods.GetClipboardSequenceNumber();
            if (actualSequence != expectedSequence)
            {
                _log.Write("info", "clipboard.image_fallback_skipped", null,
                    new Dictionary<string, object?>
                    {
                        ["reason"] = "clipboard_changed",
                        ["expected_sequence"] = expectedSequence,
                        ["actual_sequence"] = actualSequence
                    });
                return false;
            }

            var pngData = File.ReadAllBytes(imageData.StagingPath);
            var fallbackPath = ClipboardImageFallbackStore.Persist(imageData.StagingPath);
            _suppressNextClipboardUpdate = true;
            if (NativeClipboardFormats.TryRewriteWithImageFallbacks(
                    Handle,
                    nativeFormats,
                    pngData,
                    fallbackPath,
                    out var rewrittenFormats,
                    out var error,
                    out var hresult))
            {
                nativeFormats.Clear();
                nativeFormats.AddRange(rewrittenFormats);
                _log.Write("info", "clipboard.image_fallback_added", null,
                    new Dictionary<string, object?>
                    {
                        ["sequence_before"] = expectedSequence,
                        ["sequence_after"] = NativeMethods.GetClipboardSequenceNumber(),
                        ["png_bytes"] = pngData.Length,
                        ["file_drop_added"] = true,
                        ["source_process_name"] = source.ProcessName
                    });
                return true;
            }

            _suppressNextClipboardUpdate = false;
            _log.Write("warning", "clipboard.image_fallback_failed", null,
                new Dictionary<string, object?>
                {
                    ["hresult"] = $"0x{hresult:X8}",
                    ["reason"] = error,
                    ["source_process_name"] = source.ProcessName
                });
            return false;
        }

        private void QueueCapturedEvent(
            string? text,
            ClipboardImageData? imageData,
            string[] paths,
            IReadOnlyList<CapturedClipboardNativeFormat> nativeFormats,
            ForegroundContext source,
            long capturedAt)
        {
            _capturePublishTail = _capturePublishTail.ContinueWith(
                _ =>
                {
                    var sourceIconPath = _foregroundContextProvider.TryStageProcessIcon(
                        source.ProcessId,
                        out var iconError);
                    if (iconError is not null)
                    {
                        _log.Write("warning", "clipboard.source_icon_failed", null,
                            new Dictionary<string, object?>
                            {
                                ["source_process_id"] = source.ProcessId,
                                ["source_process_name"] = source.ProcessName,
                                ["reason"] = iconError
                            });
                    }
                    var stagedFormats = StageNativeFormats(nativeFormats);
                    _events.TryWrite(new ClipboardCapturedEvent(
                        text, imageData, paths, sourceIconPath, source, capturedAt, stagedFormats));
                },
                CancellationToken.None,
                TaskContinuationOptions.None,
                TaskScheduler.Default);
        }

        private IReadOnlyList<ClipboardNativeFormatData> StageNativeFormats(
            IReadOnlyList<CapturedClipboardNativeFormat> formats)
        {
            var staged = new List<ClipboardNativeFormatData>(formats.Count);
            var directory = Path.Combine(
                Path.GetTempPath(), "NxTools.Next", "clipboard-format-staging");
            Directory.CreateDirectory(directory);
            foreach (var format in formats)
            {
                try
                {
                    var path = Path.Combine(directory, $"{Guid.NewGuid():N}.bin");
                    File.WriteAllBytes(path, format.Data);
                    staged.Add(new ClipboardNativeFormatData(
                        format.FormatId,
                        format.FormatName,
                        path,
                        checked((ulong)format.Data.LongLength),
                        format.SortOrder));
                }
                catch (Exception error)
                {
                    _log.Write("warning", "clipboard.native_format_stage_failed", null,
                        new Dictionary<string, object?>
                        {
                            ["format_id"] = format.FormatId,
                            ["size_bytes"] = format.Data.LongLength,
                            ["exception_type"] = error.GetType().FullName,
                            ["message"] = error.Message
                        });
                }
            }
            return staged;
        }

        private void OnForegroundChanged(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime)
        {
            if (window != IntPtr.Zero)
            {
                ProcessForeground(_foregroundContextProvider.Capture(window));
            }
        }

        private void ProcessForeground(ForegroundContext context)
        {
            if (FileDialogSearchHost.CanReuseForeground(
                    _fileDialogSearchEnabled,
                    _fileDialogSearchHostVisible,
                    _activeFileDialogSearchHost,
                    context.WindowHandle))
            {
                return;
            }
            PublishForeground(context);
            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                FileDialogWindowAdapter.IsPotentialHost(context) &&
                _fileDialogRecognitionWindowHandle != context.WindowHandle)
            {
                _fileDialogRecognitionWindowHandle = context.WindowHandle;
                _fileDialogRecognitionStartedAt = Stopwatch.GetTimestamp();
            }
            var fileDialog = FileDialogWindowAdapter.TryCapture(context);
            ProcessFileDialogSearchHost(context, fileDialog);

            if (fileDialog is not null)
            {
                CompleteFileDialogRecognition(fileDialog.WindowHandle);
                ProcessRecognizedFileDialog(fileDialog);
                return;
            }

            var directoryAttempt =
                DirectoryNavigationAdapter.TryCaptureLocalAttempt(context);
            var directorySource = directoryAttempt.Context;
            if (directorySource is not null &&
                !directorySource.Host.Equals(
                    DirectoryNavigationAdapter.FileDialogHost,
                    StringComparison.Ordinal))
            {
                CancelFileDialogRecognition();
                var displayName = DirectorySourceDisplayName(
                    context.ProcessName,
                    directorySource);
                _autoQuickSwitch.ObserveDirectorySource(
                    directorySource,
                    displayName);
                if (directorySource.Host.Equals(
                        DirectoryNavigationAdapter.ExplorerHost,
                        StringComparison.Ordinal) &&
                    string.IsNullOrWhiteSpace(
                        directorySource.CurrentDirectory))
                {
                    QueueExplorerDirectoryResolution(
                        directorySource.WindowHandle,
                        context.ProcessName);
                }
                CancelDirectorySourceRecognition();
                return;
            }
            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                directoryAttempt.Failure?.Fatal != true &&
                ThirdPartyFileManagerAdapter.IsPotentialHost(context))
            {
                ScheduleDirectorySourceRecognition(context.WindowHandle);
                return;
            }

            if ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                FileDialogWindowAdapter.IsPotentialHost(context))
            {
                ScheduleFileDialogRecognition(context.WindowHandle);
                return;
            }

            CancelFileDialogRecognition();
            CancelDirectorySourceRecognition();
            if (!IsNxToolsDesktopProcess(context.ProcessName))
            {
                _autoQuickSwitch.ObserveOther();
            }
        }

        private void ScheduleDirectorySourceRecognition(long windowHandle)
        {
            if (!_directorySourceRecognitionRetry.Begin(windowHandle))
            {
                return;
            }
            _ = NativeMethods.SetTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId,
                FileDialogRecognitionIntervalMs,
                IntPtr.Zero);
        }

        private void RetryDirectorySourceRecognition()
        {
            var foreground = _foregroundContextProvider.Capture();
            if (!_directorySourceRecognitionRetry.TryBeginAttempt(
                    foreground.WindowHandle,
                    out var windowHandle))
            {
                return;
            }
            var attempt = DirectoryNavigationAdapter.TryCaptureLocalAttempt(
                foreground);
            var source = attempt.Context;
            if (source is not null &&
                source.Host.Equals(
                    DirectoryNavigationAdapter.ThirdPartyFileManagerHost,
                    StringComparison.Ordinal))
            {
                var displayName = DirectorySourceDisplayName(
                    foreground.ProcessName,
                    source);
                _autoQuickSwitch.ObserveDirectorySource(source, displayName);
                CompleteDirectorySourceRecognition(windowHandle);
                return;
            }
            if (attempt.Failure?.Fatal == true)
            {
                CancelDirectorySourceRecognition();
                return;
            }
            if (_directorySourceRecognitionRetry.HasPendingAttempt(windowHandle))
            {
                ScheduleDirectorySourceRecognition(windowHandle);
            }
        }

        private void CompleteDirectorySourceRecognition(long windowHandle)
        {
            _directorySourceRecognitionRetry.Complete(windowHandle);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
        }

        private void CancelDirectorySourceRecognition()
        {
            _directorySourceRecognitionRetry.Cancel();
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
        }

        private void ProcessRecognizedFileDialog(FileDialogContext fileDialog)
        {
            var candidate = _autoQuickSwitch.ObserveFileDialog(
                new DirectoryNavigationContext(
                    DirectoryNavigationAdapter.FileDialogHost,
                    fileDialog.WindowHandle,
                    fileDialog.CurrentDirectory));
            if (candidate is not null)
            {
                QueueAutoQuickSwitch(candidate);
            }
        }

        private void QueueExplorerDirectoryResolution(
            long windowHandle,
            string processName)
        {
            _ = Task.Run(() =>
            {
                if (!ExplorerWindowAdapter.TryGetCurrentDirectory(
                        windowHandle,
                        out var directory,
                        out _)
                    || string.IsNullOrWhiteSpace(directory))
                {
                    return;
                }
                var context = new DirectoryNavigationContext(
                    DirectoryNavigationAdapter.ExplorerHost,
                    windowHandle,
                    directory);
                _resolvedDirectorySources.Enqueue((
                    windowHandle,
                    directory,
                    DirectorySourceDisplayName(processName, context)));
                _ = NativeMethods.PostMessage(
                    Handle,
                    WmDirectorySourceResolved,
                    IntPtr.Zero,
                    IntPtr.Zero);
            });
        }

        private void DrainResolvedDirectorySources()
        {
            var changed = false;
            while (_resolvedDirectorySources.TryDequeue(out var source))
            {
                changed |= _autoQuickSwitch.ResolveDirectorySource(
                    source.WindowHandle,
                    source.Directory,
                    source.DisplayName);
            }
            if (changed &&
                _fileDialogSearchHostVisible &&
                _activeFileDialogSearchHost is { } snapshot)
            {
                PublishFileDialogSearchHost(snapshot, force: true);
            }
        }

        private static string DirectorySourceDisplayName(
            string processName,
            DirectoryNavigationContext source)
        {
            var provider = source.Host switch
            {
                DirectoryNavigationAdapter.ExplorerHost => "File Explorer",
                DirectoryNavigationAdapter.DesktopHost => "Desktop",
                DirectoryNavigationAdapter.ThirdPartyFileManagerHost =>
                    Path.GetFileNameWithoutExtension(processName)
                        .ToLowerInvariant() switch
                    {
                        "dopus" => "Directory Opus",
                        "totalcmd" or "totalcmd64" => "Total Commander",
                        "onecommander" => "OneCommander",
                        "files" => "Files",
                        _ => "File Manager"
                    },
                _ => "File Manager"
            };
            var directory = source.CurrentDirectory?
                .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            var folder = string.IsNullOrWhiteSpace(directory)
                ? null
                : Path.GetFileName(directory);
            if (string.IsNullOrWhiteSpace(folder))
            {
                folder = directory;
            }
            return string.IsNullOrWhiteSpace(folder)
                ? provider
                : $"{folder} ({provider})";
        }

        private void ScheduleFileDialogRecognition(long windowHandle)
        {
            if (!_fileDialogRecognitionRetry.Begin(windowHandle))
            {
                return;
            }
            if (_fileDialogRecognitionWindowHandle != windowHandle)
            {
                _fileDialogRecognitionWindowHandle = windowHandle;
                _fileDialogRecognitionStartedAt = Stopwatch.GetTimestamp();
            }
            if (NativeMethods.SetTimer(
                    Handle,
                    (UIntPtr)FileDialogRecognitionTimerId,
                    FileDialogRecognitionIntervalMs,
                    IntPtr.Zero) == UIntPtr.Zero)
            {
                _fileDialogRecognitionRetry.Cancel();
                _autoQuickSwitch.ObserveOther();
            }
        }

        private void RetryFileDialogRecognition()
        {
            var foreground = _foregroundContextProvider.Capture();
            if (!_fileDialogRecognitionRetry.TryBeginAttempt(
                    foreground.WindowHandle,
                    out var windowHandle))
            {
                if (windowHandle != 0 &&
                    _fileDialogRecognitionRetry.HasPendingAttempt(windowHandle))
                {
                    ScheduleFileDialogRecognition(windowHandle);
                }
                return;
            }
            var targetContext = foreground.WindowHandle == windowHandle
                ? foreground
                : _foregroundContextProvider.Capture(new IntPtr(windowHandle));
            var fileDialog = FileDialogWindowAdapter.TryCapture(targetContext);
            if (fileDialog is not null)
            {
                CompleteFileDialogRecognition(windowHandle);
                ProcessFileDialogSearchHost(foreground, fileDialog);
                ProcessRecognizedFileDialog(fileDialog);
                return;
            }
            if (_fileDialogRecognitionRetry.HasPendingAttempt(windowHandle))
            {
                ScheduleFileDialogRecognition(windowHandle);
                return;
            }
            _autoQuickSwitch.ObserveOther();
        }

        private void CompleteFileDialogRecognition(long windowHandle)
        {
            _fileDialogRecognitionRetry.Complete(windowHandle);
            if (_fileDialogRecognitionWindowHandle == windowHandle)
            {
                _log.Write(
                    "info",
                    "quick_launch.file_dialog_recognized",
                    fields: new Dictionary<string, object?>
                    {
                        ["window_handle"] = windowHandle,
                        ["elapsed_ms"] = Stopwatch
                            .GetElapsedTime(_fileDialogRecognitionStartedAt)
                            .TotalMilliseconds
                    });
                _fileDialogRecognitionWindowHandle = 0;
                _fileDialogRecognitionStartedAt = 0;
            }
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
        }

        private void CancelFileDialogRecognition()
        {
            _fileDialogRecognitionRetry.Cancel();
            _fileDialogRecognitionWindowHandle = 0;
            _fileDialogRecognitionStartedAt = 0;
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
        }

        private IntPtr InstallOptionalWindowEventHook(uint eventType)
        {
            var hook = NativeMethods.SetWinEventHook(
                eventType,
                eventType,
                IntPtr.Zero,
                _fileDialogWindowCallback,
                0,
                0,
                WineventOutOfContext);
            if (hook == IntPtr.Zero)
            {
                _log.Write("warning", "quick_launch.file_dialog_window_hook_failed", null,
                    new Dictionary<string, object?>
                    {
                        ["event_type"] = eventType,
                        ["win32_error"] = Marshal.GetLastWin32Error()
                    });
            }
            return hook;
        }

        private void ProcessFileDialogSearchHost(
            ForegroundContext foreground,
            FileDialogContext? fileDialog)
        {
            if (!_fileDialogSearchEnabled)
            {
                return;
            }
            if (fileDialog is not null)
            {
                PublishFileDialogSearchHost(fileDialog);
                return;
            }
            if (IsNxToolsDesktopProcess(foreground.ProcessName))
            {
                return;
            }
            HideFileDialogSearchHost();
        }

        private static bool IsNxToolsDesktopProcess(string processName)
        {
            var name = Path.GetFileNameWithoutExtension(processName);
            return name.Equals("NxTools", StringComparison.OrdinalIgnoreCase) ||
                name.Equals("nxtools-desktop", StringComparison.OrdinalIgnoreCase);
        }

        private void PublishFileDialogSearchHost(FileDialogContext fileDialog)
        {
            var snapshot = FileDialogSearchHost.TryCapture(fileDialog);
            if (snapshot is null)
            {
                HideFileDialogSearchHost();
                return;
            }
            PublishFileDialogSearchHost(snapshot);
        }

        private void PublishFileDialogSearchHost(
            FileDialogSearchHostSnapshot snapshot,
            bool force = false)
        {
            var source = _autoQuickSwitch.LatestSource;
            if (_fileDialogSearchHostVisible &&
                _activeFileDialogSearchHost == snapshot &&
                _publishedFileDialogSearchSource == source &&
                !force)
            {
                return;
            }
            _activeFileDialogSearchHost = snapshot;
            _publishedFileDialogSearchSource = source;
            _fileDialogSearchHostVisible = true;
            _events.TryWrite(new QuickLaunchFileDialogSearchHostChangedEvent(
                snapshot.WindowHandle,
                snapshot.CurrentDirectory,
                snapshot.Left,
                snapshot.Top,
                snapshot.Width,
                snapshot.Height,
                snapshot.MonitorDpi,
                true,
                DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                source));
        }

        private void HideFileDialogSearchHost(bool forget = false)
        {
            var snapshot = _activeFileDialogSearchHost;
            if (snapshot is null)
            {
                return;
            }
            if (_fileDialogSearchHostVisible)
            {
                _events.TryWrite(new QuickLaunchFileDialogSearchHostChangedEvent(
                    snapshot.WindowHandle,
                    snapshot.CurrentDirectory,
                    snapshot.Left,
                    snapshot.Top,
                    snapshot.Width,
                    snapshot.Height,
                    snapshot.MonitorDpi,
                    false,
                    DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                    _autoQuickSwitch.LatestSource));
            }
            _fileDialogSearchHostVisible = false;
            if (forget)
            {
                _fileDialogGeometryUpdateGate.Cancel(snapshot.WindowHandle);
                var placement = Volatile.Read(ref _fileDialogSearchPlacement);
                if (placement?.OwnerWindowHandle == snapshot.WindowHandle)
                {
                    AnchoredWindowPlacement.Forget(new IntPtr(placement.WindowHandle));
                    Volatile.Write(ref _fileDialogSearchPlacement, null);
                }
                _activeFileDialogSearchHost = null;
                _publishedFileDialogSearchSource = null;
            }
        }

        private void OnFileDialogWindowChanged(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime)
        {
            if (eventType == EventSystemMoveSizeEnd)
            {
                var overlay = Volatile.Read(ref _taskbarOverlayPlacement);
                if (overlay is not null && window.ToInt64() == overlay.WindowHandle)
                {
                    _events.TryWrite(new TaskbarOverlayEditGestureFinishedEvent(overlay.WindowHandle));
                }
                return;
            }
            var rootWindowHandle = NativeMethods.GetRootWindowHandle(window);
            if (TaskbarOverlayPlacement.IsTaskbarWindow(window) ||
                TaskbarOverlayPlacement.IsTaskbarWindow(new IntPtr(rootWindowHandle)))
            {
                _ = NativeMethods.PostMessage(
                    Handle,
                    WmTaskbarOverlayChanged,
                    IntPtr.Zero,
                    IntPtr.Zero);
            }
            if ((eventType is EventObjectCreate or EventObjectShow) &&
                (_fileDialogRecognitionRetry.ShouldRetryForStructureChange(
                     rootWindowHandle) ||
                 ((_fileDialogSearchEnabled || _autoQuickSwitchEnabled) &&
                  NativeMethods.IsForegroundWindowForTarget(
                      new IntPtr(rootWindowHandle)))) &&
                Interlocked.Exchange(
                    ref _fileDialogStructureRetryPosted,
                    1) == 0)
            {
                if (!NativeMethods.PostMessage(
                        Handle,
                        WmFileDialogStructureChanged,
                        IntPtr.Zero,
                        new IntPtr(rootWindowHandle)))
                {
                    Interlocked.Exchange(
                        ref _fileDialogStructureRetryPosted,
                        0);
                }
                return;
            }
            var snapshot = _activeFileDialogSearchHost;
            if (!_fileDialogSearchEnabled ||
                snapshot is null ||
                rootWindowHandle != snapshot.WindowHandle)
            {
                return;
            }
            if ((eventType is EventObjectDestroy or EventObjectHide) &&
                objectId == ObjIdWindow &&
                window.ToInt64() == snapshot.WindowHandle)
            {
                HideFileDialogSearchHost(forget: eventType == EventObjectDestroy);
                return;
            }
            if (eventType == EventObjectLocationChange)
            {
                if (_fileDialogGeometryUpdateGate.TrySchedule(
                        snapshot.WindowHandle,
                        rootWindowHandle,
                        window.ToInt64(),
                        objectId,
                        eventType) &&
                    !NativeMethods.PostMessage(
                        Handle,
                        WmFileDialogGeometryChanged,
                        IntPtr.Zero,
                        new IntPtr(snapshot.WindowHandle)))
                {
                    _fileDialogGeometryUpdateGate.Cancel(snapshot.WindowHandle);
                }
                return;
            }
            if (eventType == EventObjectNameChange)
            {
                var foreground = _foregroundContextProvider.Capture(
                    new IntPtr(snapshot.WindowHandle));
                var dialog = FileDialogWindowAdapter.TryCapture(foreground);
                if (dialog is not null)
                {
                    PublishFileDialogSearchHost(dialog);
                }
            }
        }

        private void ReapplyTaskbarOverlayPlacement()
        {
            var placement = Volatile.Read(ref _taskbarOverlayPlacement);
            if (placement is null)
            {
                return;
            }
            var result = TaskbarOverlayPlacement.Place(placement);
            if (result.Success && result.Data == _taskbarOverlayAppliedBounds)
            {
                return;
            }
            _taskbarOverlayAppliedBounds = result.Data;
            _log.Write(
                result.Success ? "info" : "warning",
                result.Success
                    ? "download.taskbar_overlay.repositioned"
                    : "download.taskbar_overlay.reposition_failed",
                _taskbarOverlayCorrelationId,
                new Dictionary<string, object?>
                {
                    ["window_handle"] = placement.WindowHandle,
                    ["applied_bounds"] = result.Data,
                    ["error"] = result.Error
                });
            if (!result.Success)
            {
                ForgetTaskbarOverlayPlacement(placement.WindowHandle);
            }
        }

        private void OnSystemDisplaySettingsChanged(object? sender, EventArgs eventArgs) =>
            RequestTaskbarOverlayReposition();

        private void OnSystemUserPreferenceChanged(
            object? sender,
            UserPreferenceChangedEventArgs eventArgs) =>
            RequestTaskbarOverlayReposition();

        private void RequestTaskbarOverlayReposition()
        {
            _ = NativeMethods.PostMessage(
                Handle,
                WmTaskbarOverlayChanged,
                IntPtr.Zero,
                IntPtr.Zero);
        }

        private void FollowFileDialogSearchWindow(long rootWindowHandle)
        {
            var snapshot = _activeFileDialogSearchHost;
            var placement = Volatile.Read(ref _fileDialogSearchPlacement);
            if (!_fileDialogSearchEnabled ||
                !_fileDialogSearchHostVisible ||
                snapshot is null ||
                snapshot.WindowHandle != rootWindowHandle ||
                placement is null ||
                placement.OwnerWindowHandle != rootWindowHandle)
            {
                return;
            }
            var result = AnchoredWindowPlacement.Follow(
                new IntPtr(placement.WindowHandle),
                new IntPtr(placement.OwnerWindowHandle),
                placement.VisibleWidth,
                placement.VisibleHeight,
                placement.BottomOverlap,
                out var ownerVisible);
            if (!result.Success)
            {
                _log.Write(
                    "warning",
                    "quick_launch.file_dialog_search_follow_failed",
                    fields: new Dictionary<string, object?>
                    {
                        ["window_handle"] = placement.WindowHandle,
                        ["owner_window_handle"] = placement.OwnerWindowHandle,
                        ["error"] = result.Error
                    });
                return;
            }
            var refreshed = FileDialogSearchHost.RefreshGeometry(snapshot, ownerVisible);
            if (refreshed.MonitorDpi != snapshot.MonitorDpi)
            {
                PublishFileDialogSearchHost(refreshed);
            }
            else
            {
                _activeFileDialogSearchHost = refreshed;
            }
        }

        private void QueueAutoQuickSwitch(AutoQuickSwitchCandidate candidate)
        {
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            _ = Task.Run(() =>
            {
                if (!_autoQuickSwitch.IsCurrent(candidate))
                {
                    return;
                }
                var sourceDirectory = candidate.Source.CurrentDirectory;
                string? captureError = null;
                if (string.IsNullOrWhiteSpace(sourceDirectory) &&
                    candidate.Source.Host.Equals(
                        DirectoryNavigationAdapter.ExplorerHost,
                        StringComparison.Ordinal) &&
                    ExplorerWindowAdapter.TryGetCurrentDirectory(
                        candidate.SourceWindowHandle,
                        out var resolvedDirectory,
                        out captureError))
                {
                    sourceDirectory = resolvedDirectory;
                    if (!string.IsNullOrWhiteSpace(sourceDirectory))
                    {
                        _resolvedDirectorySources.Enqueue((
                            candidate.SourceWindowHandle,
                            sourceDirectory,
                            DirectorySourceDisplayName(
                                "explorer",
                                candidate.Source with
                                {
                                    CurrentDirectory = sourceDirectory
                                })));
                        _ = NativeMethods.PostMessage(
                            Handle,
                            WmDirectorySourceResolved,
                            IntPtr.Zero,
                            IntPtr.Zero);
                    }
                }
                if (string.IsNullOrWhiteSpace(sourceDirectory))
                {
                    _log.Write("warning", "quick_launch.auto_switch_source_failed", null,
                        new Dictionary<string, object?>
                        {
                            ["source_window_handle"] = candidate.SourceWindowHandle,
                            ["target_window_handle"] = candidate.Target.WindowHandle,
                            ["error"] = captureError
                        });
                    return;
                }
                if (!_autoQuickSwitch.IsCurrent(candidate) ||
                    !NativeMethods.IsForegroundWindowForTarget(
                        new IntPtr(candidate.Target.WindowHandle)))
                {
                    return;
                }
                _events.TryWrite(new QuickLaunchAutoSwitchRequestedEvent(
                    candidate.Target,
                    sourceDirectory,
                    requestedAtUnixMs));
            });
        }

        private void PublishForeground(ForegroundContext context)
        {
            _events.TryWrite(new ForegroundChangedEvent(context));
        }

        private void MonitorPointerPress()
        {
            var primaryDown = NativeMethods.IsPointerButtonDown(0x01);
            var secondaryDown = NativeMethods.IsPointerButtonDown(0x02);
            var pressed = (primaryDown && !_primaryPointerDown) ||
                          (secondaryDown && !_secondaryPointerDown);
            _primaryPointerDown = primaryDown;
            _secondaryPointerDown = secondaryDown;
            if (!pressed)
            {
                return;
            }

            var target = NativeMethods.GetPointerTarget();
            _events.TryWrite(new GlobalPointerPressedEvent(
                target.WindowHandle,
                target.RootWindowHandle,
                target.ProcessId));
        }

        private void PublishPanelRequest()
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            _events.TryWrite(new ClipboardPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()));
        }

        private void PublishQuickLaunchPanelRequest()
        {
            var target = _foregroundContextProvider.Capture();
            var cursor = NativeMethods.GetCursorLocation();
            var requestedAtUnixMs = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            var deferDirectoryCapture =
                DirectoryNavigationAdapter.RequiresElevatedHelper(target);
            var directoryNavigationContext = deferDirectoryCapture
                ? null
                : DirectoryNavigationAdapter.TryCaptureLocal(target);
            _events.TryWrite(new QuickLaunchPanelRequestedEvent(
                target,
                cursor.Position.X,
                cursor.Position.Y,
                cursor.DisplayIndex,
                cursor.Dpi,
                requestedAtUnixMs,
                directoryNavigationContext));
            if (directoryNavigationContext is null &&
                DirectoryNavigationAdapter.IsPotentialElevatedHost(target))
            {
                _ = Task.Run(() =>
                {
                    if (!IsQuickLaunchContextRequestCurrent(
                            requestedAtUnixMs,
                            DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()))
                    {
                        return;
                    }
                    var resolved =
                        _elevatedDirectoryNavigation.TryCapture(target);
                    if (resolved is not null &&
                        IsQuickLaunchContextRequestCurrent(
                            requestedAtUnixMs,
                            DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()))
                    {
                        _events.TryWrite(
                            new QuickLaunchDirectoryContextResolvedEvent(
                                target.WindowHandle,
                                requestedAtUnixMs,
                                resolved));
                    }
                });
            }
        }

        private void OnGlobalKeyDown(int virtualKey, uint timestamp)
        {
            if (!_quickLaunchTriggerEnabled || _quickLaunchRecording ||
                _quickLaunchTrigger.Kind != QuickLaunchTriggerKind.DoubleControl)
            {
                return;
            }

            bool triggered;
            if (virtualKey == VkLeftControl)
            {
                _rightControlDoubleTap.Reset();
                triggered = _leftControlDoubleTap.OnKeyDown(virtualKey, timestamp);
            }
            else if (virtualKey == VkRightControl)
            {
                _leftControlDoubleTap.Reset();
                triggered = _rightControlDoubleTap.OnKeyDown(virtualKey, timestamp);
            }
            else
            {
                _leftControlDoubleTap.Reset();
                _rightControlDoubleTap.Reset();
                return;
            }

            if (triggered)
            {
                // A low-level hook must return quickly. Defer foreground capture and IPC publication
                // to the existing Windows message loop instead of doing either inside the callback.
                _ = NativeMethods.PostMessage(Handle, WmQuickLaunchGesture, IntPtr.Zero, IntPtr.Zero);
            }
        }

        private void OnGlobalKeyUp(int virtualKey)
        {
            var triggered = _leftControlDoubleTap.OnKeyUp(virtualKey) |
                            _rightControlDoubleTap.OnKeyUp(virtualKey);
            if (triggered)
            {
                // The complete gesture has ended, so the new GPUI window will not inherit an
                // unmatched Ctrl key-up as its first input event.
                _ = NativeMethods.PostMessage(Handle, WmQuickLaunchGesture, IntPtr.Zero, IntPtr.Zero);
            }
        }

        private void ResetQuickLaunchGesture()
        {
            _leftControlDoubleTap.Reset();
            _rightControlDoubleTap.Reset();
        }

        private void DrainPasteRequests()
        {
            while (_pasteRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(
                    new WindowsPasteCoordinator(this, _log).Execute(
                        request.Command,
                        request.CorrelationId));
            }
        }

        private void DrainClipboardConfigurationRequests()
        {
            while (_configurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyClipboardConfiguration(request.Command));
            }
        }

        private void DrainClipboardHotKeyRecordingRequests()
        {
            while (_hotKeyRecordingRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                var result = request.Command.Recording
                    ? _hotKeyRegistration.BeginRecording()
                    : _hotKeyRegistration.CancelRecording();
                request.Completion.TrySetResult(result);
            }
        }

        private WindowsCommandResult ApplyClipboardConfiguration(ConfigureClipboardCommand command)
        {
            var requestedExclusions = ClipboardCaptureExclusions.Create(command.ExcludedProcesses);
            if (!ClipboardHotKeyBinding.TryCreate(command, out var binding, out var error))
            {
                return new WindowsCommandResult(false, error);
            }

            // Privacy exclusions do not depend on whether the requested global hotkey is available.
            _captureExclusions = requestedExclusions;

            var registration = _hotKeyRegistration.Apply(binding);
            if (!registration.Success)
            {
                return registration;
            }
            _log.Write("info", "clipboard.configuration_applied", null,
                new Dictionary<string, object?>
                {
                    ["hotkey"] = binding.Display,
                    ["excluded_process_count"] = _captureExclusions.ProcessNames.Count
                });
            return new WindowsCommandResult(true);
        }

        private void DrainQuickLaunchConfigurationRequests()
        {
            while (_quickLaunchConfigurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyQuickLaunchConfiguration(request.Command));
            }
        }

        private void DrainQuickLaunchHotKeyRecordingRequests()
        {
            while (_quickLaunchHotKeyRecordingRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                var result = SetQuickLaunchRecording(request.Command.Recording);
                request.Completion.TrySetResult(result);
            }
        }

        private void DrainDesktopShellConfigurationRequests()
        {
            while (_desktopShellConfigurationRequests.TryDequeue(out var request))
            {
                if (request.CancellationToken.IsCancellationRequested)
                {
                    request.Completion.TrySetCanceled(request.CancellationToken);
                    continue;
                }
                request.Completion.TrySetResult(ApplyDesktopShellConfiguration(request.Command));
            }
        }

        private WindowsCommandResult ApplyDesktopShellConfiguration(ConfigureDesktopShellCommand command)
        {
            try
            {
                var desktopExecutable = Path.Combine(AppContext.BaseDirectory, "NxTools.exe");
                using (var runKey = Registry.CurrentUser.CreateSubKey(
                           @"Software\Microsoft\Windows\CurrentVersion\Run",
                           writable: true))
                {
                    if (runKey is null)
                    {
                        return new WindowsCommandResult(false, "Windows startup registry is unavailable.");
                    }
                    var startup = ApplyStartupRegistration(
                        runKey,
                        desktopExecutable,
                        command.StartOnLoginSilently);
                    if (!startup.Success)
                    {
                        return startup;
                    }
                }

                return ConfigureTrayIcon(command.TrayIconVisible, desktopExecutable);
            }
            catch (Exception error) when (error is IOException or UnauthorizedAccessException or System.Security.SecurityException)
            {
                return new WindowsCommandResult(false, error.Message, NativeErrorCode: error.HResult);
            }
        }

        private WindowsCommandResult ConfigureTrayIcon(bool visible, string desktopExecutable)
        {
            return _desktopTray.Configure(
                visible,
                desktopExecutable,
                () => _events.TryWrite(new TrayActionRequestedEvent(Guid.NewGuid(), "open")),
                () => _events.TryWrite(new TrayActionRequestedEvent(Guid.NewGuid(), "exit")));
        }

        private WindowsCommandResult ApplyQuickLaunchConfiguration(ConfigureQuickLaunchCommand command)
        {
            _autoQuickSwitchEnabled = command.Enabled && command.AutoQuickSwitchEnabled;
            var fileDialogSearchEnabled =
                command.Enabled && command.FileDialogSearchEnabled;
            _autoQuickSwitch.Configure(
                _autoQuickSwitchEnabled,
                _autoQuickSwitchEnabled || fileDialogSearchEnabled);
            if (_fileDialogSearchEnabled && !fileDialogSearchEnabled)
            {
                HideFileDialogSearchHost(forget: true);
            }
            _fileDialogSearchEnabled = fileDialogSearchEnabled;
            if (_fileDialogSearchEnabled || _autoQuickSwitchEnabled)
            {
                ProcessForeground(_foregroundContextProvider.Capture());
            }
            else
            {
                CancelFileDialogRecognition();
            }
            if (!command.Enabled)
            {
                _quickLaunchHotKeyRegistration.Stop();
                _quickLaunchTriggerEnabled = false;
                _quickLaunchRecording = false;
                ResetQuickLaunchGesture();
                return new WindowsCommandResult(true);
            }
            if (!QuickLaunchTriggerBinding.TryCreate(command, out var binding, out var error))
            {
                return new WindowsCommandResult(false, error);
            }

            var previous = _quickLaunchTrigger;
            var previousEnabled = _quickLaunchTriggerEnabled;
            WindowsCommandResult registration;
            if (binding.Kind == QuickLaunchTriggerKind.DoubleControl)
            {
                if (_quickLaunchKeyboardMonitor?.IsInstalled != true)
                {
                    return new WindowsCommandResult(
                        false,
                        "The global keyboard monitor required by double Ctrl is unavailable.");
                }
                if (previous.Kind == QuickLaunchTriggerKind.HotKey)
                {
                    _quickLaunchHotKeyRegistration.Stop();
                }
                _quickLaunchTrigger = binding;
                _quickLaunchTriggerEnabled = true;
                _quickLaunchRecording = false;
                ResetQuickLaunchGesture();
                registration = new WindowsCommandResult(true);
            }
            else
            {
                _quickLaunchTriggerEnabled = false;
                ResetQuickLaunchGesture();
                registration = _quickLaunchHotKeyRegistration.Apply(binding.HotKey!);
                if (registration.Success)
                {
                    _quickLaunchTrigger = binding;
                    _quickLaunchTriggerEnabled = true;
                    _quickLaunchRecording = false;
                }
                else
                {
                    _quickLaunchTrigger = previous;
                    _quickLaunchTriggerEnabled = previousEnabled;
                }
            }
            if (registration.Success)
            {
                _log.Write("info", "quick_launch.configuration_applied", null,
                    new Dictionary<string, object?> { ["hotkey"] = binding.Display });
            }
            return registration;
        }

        private WindowsCommandResult SetQuickLaunchRecording(bool recording)
        {
            if (_quickLaunchRecording == recording)
            {
                return new WindowsCommandResult(true);
            }

            if (recording)
            {
                var suspended = _quickLaunchTrigger.Kind == QuickLaunchTriggerKind.HotKey
                    ? _quickLaunchHotKeyRegistration.BeginRecording()
                    : new WindowsCommandResult(true);
                if (!suspended.Success)
                {
                    return suspended;
                }
                _quickLaunchRecording = true;
                _quickLaunchTriggerEnabled = false;
                ResetQuickLaunchGesture();
                return new WindowsCommandResult(true);
            }

            var restored = _quickLaunchTrigger.Kind == QuickLaunchTriggerKind.HotKey
                ? _quickLaunchHotKeyRegistration.CancelRecording()
                : _quickLaunchKeyboardMonitor?.IsInstalled == true
                    ? new WindowsCommandResult(true)
                    : new WindowsCommandResult(false, "The global keyboard monitor is unavailable.");
            if (restored.Success)
            {
                _quickLaunchRecording = false;
                _quickLaunchTriggerEnabled = true;
                ResetQuickLaunchGesture();
            }
            return restored;
        }

        private ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload)
        {
            var attempts = new List<ClipboardWriteAttempt>();
            for (var attempt = 0; attempt < 5; attempt++)
            {
                try
                {
                    _suppressNextClipboardUpdate = true;
                    switch (payload)
                    {
                        case TextClipboardPastePayload text:
                            Clipboard.SetText(text.Text, TextDataFormat.UnicodeText);
                            break;
                        case FilesClipboardPastePayload files:
                            var paths = new System.Collections.Specialized.StringCollection();
                            paths.AddRange(files.Paths.Where(static path =>
                                File.Exists(path) || Directory.Exists(path)).ToArray());
                            if (paths.Count == 0)
                            {
                                _suppressNextClipboardUpdate = false;
                                return new ClipboardWriteResult(false, "No pasted file paths still exist.", attempts);
                            }
                            Clipboard.SetFileDropList(paths);
                            break;
                        case ImageClipboardPastePayload imagePayload:
                            if (!File.Exists(imagePayload.ImagePath))
                            {
                                _suppressNextClipboardUpdate = false;
                                return new ClipboardWriteResult(false, "The clipboard image file no longer exists.", attempts);
                            }
                            using (var imageData = ClipboardImageDataObject.Load(
                                       imagePayload.ImagePath,
                                       imagePayload.IncludeFileDropFallback))
                            {
                                Clipboard.SetDataObject(imageData.Data, copy: true, retryTimes: 5, retryDelay: 20);
                            }
                            break;
                        case NativeFormatsClipboardPastePayload native:
                            if (!NativeClipboardFormats.TryWrite(
                                    Handle, native.Formats, native.ImagePathFallback,
                                    out var nativeError, out var nativeHresult))
                            {
                                _suppressNextClipboardUpdate = false;
                                throw new ExternalException(
                                    nativeError ?? "Could not restore native clipboard formats.",
                                    nativeHresult);
                            }
                            break;
                        default:
                            _suppressNextClipboardUpdate = false;
                            return new ClipboardWriteResult(false, "Unsupported clipboard paste payload.", attempts);
                    }
                    attempts.Add(new ClipboardWriteAttempt(attempt + 1, true, 0, null));
                    return new ClipboardWriteResult(true, null, attempts);
                }
                catch (ExternalException exception)
                {
                    _suppressNextClipboardUpdate = false;
                    attempts.Add(new ClipboardWriteAttempt(
                        attempt + 1,
                        false,
                        exception.ErrorCode,
                        exception.Message));
                    if (attempt == 4)
                    {
                        return new ClipboardWriteResult(
                            false,
                            $"Could not open the Windows clipboard: {exception.Message}",
                            attempts);
                    }
                    Thread.Sleep(20);
                }
                catch (Exception exception)
                {
                    _suppressNextClipboardUpdate = false;
                    attempts.Add(new ClipboardWriteAttempt(
                        attempt + 1,
                        false,
                        exception.HResult,
                        exception.Message));
                    return new ClipboardWriteResult(false, exception.Message, attempts);
                }
            }
            return new ClipboardWriteResult(false, "Could not open the Windows clipboard.", attempts);
        }

        bool IWindowsPasteOperations.IsWindow(long windowHandle) =>
            NativeMethods.IsWindow(new IntPtr(windowHandle));

        uint IWindowsPasteOperations.GetClipboardSequenceNumber() =>
            NativeMethods.GetClipboardSequenceNumber();

        ClipboardWriteResult IWindowsPasteOperations.WriteClipboard(ClipboardPastePayload payload) =>
            WriteClipboard(payload);

        bool IWindowsPasteOperations.TryActivateWindow(
            long targetWindowHandle,
            long focusWindowHandle) =>
            NativeMethods.ActivateWindow(
                new IntPtr(targetWindowHandle),
                new IntPtr(focusWindowHandle));

        bool IWindowsPasteOperations.IsForegroundWindowForTarget(long targetWindowHandle) =>
            NativeMethods.IsForegroundWindowForTarget(new IntPtr(targetWindowHandle));

        PasteWindowSnapshot IWindowsPasteOperations.CaptureWindowState(long targetWindowHandle) =>
            NativeMethods.CaptureWindowState(new IntPtr(targetWindowHandle));

        ModifierReleaseResult IWindowsPasteOperations.ReleaseActivationModifiers() =>
            NativeMethods.ReleaseActivationModifiers();

        PasteShortcutResult IWindowsPasteOperations.SendPasteShortcut() =>
            NativeMethods.SendPasteShortcut();

        PostPasteResult IWindowsPasteOperations.PostPaste(long targetWindowHandle) =>
            NativeMethods.PostPaste(new IntPtr(targetWindowHandle));

        void IWindowsPasteOperations.Delay(int milliseconds) => Thread.Sleep(milliseconds);

        private void Stop()
        {
            ConfigureTrayIcon(false, string.Empty);
            SystemEvents.DisplaySettingsChanged -= OnSystemDisplaySettingsChanged;
            SystemEvents.UserPreferenceChanged -= OnSystemUserPreferenceChanged;
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)RetryTimerId);
            _ = NativeMethods.KillTimer(Handle, (UIntPtr)PointerMonitorTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)FileDialogRecognitionTimerId);
            _ = NativeMethods.KillTimer(
                Handle,
                (UIntPtr)DirectorySourceRecognitionTimerId);
            _ = NativeMethods.RemoveClipboardFormatListener(Handle);
            _hotKeyRegistration.Stop();
            _quickLaunchHotKeyRegistration.Stop();
            _quickLaunchTriggerEnabled = false;
            _quickLaunchKeyboardMonitor?.Dispose();
            _quickLaunchKeyboardMonitor = null;
            ResetQuickLaunchGesture();
            if (_foregroundHook != IntPtr.Zero)
            {
                _ = NativeMethods.UnhookWinEvent(_foregroundHook);
                _foregroundHook = IntPtr.Zero;
            }
            CancelFileDialogRecognition();
            foreach (var hook in new[]
                     {
                         _fileDialogCreateHook,
                         _fileDialogDestroyHook,
                         _fileDialogShowHook,
                         _fileDialogHideHook,
                         _fileDialogLocationHook,
                         _fileDialogNameHook,
                         _taskbarOverlayMoveSizeEndHook
                     })
            {
                if (hook != IntPtr.Zero)
                {
                    _ = NativeMethods.UnhookWinEvent(hook);
                }
            }
            _fileDialogCreateHook = IntPtr.Zero;
            _fileDialogDestroyHook = IntPtr.Zero;
            _fileDialogShowHook = IntPtr.Zero;
            _fileDialogHideHook = IntPtr.Zero;
            _fileDialogLocationHook = IntPtr.Zero;
            _fileDialogNameHook = IntPtr.Zero;
            _taskbarOverlayMoveSizeEndHook = IntPtr.Zero;
            while (_pasteRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(false, "Windows adapter is stopping."));
            }
            while (_configurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_hotKeyRecordingRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_quickLaunchConfigurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_quickLaunchHotKeyRecordingRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            while (_desktopShellConfigurationRequests.TryDequeue(out var request))
            {
                request.Completion.TrySetResult(new WindowsCommandResult(
                    false, "Windows adapter is stopping."));
            }
            DestroyHandle();
            Application.ExitThread();
        }

        private sealed class NativeClipboardHotKeyRegistrar(Func<IntPtr> windowHandle, int hotKeyId)
            : IClipboardHotKeyRegistrar
        {
            public int LastError => Marshal.GetLastWin32Error();

            public bool Register(uint modifiers, uint virtualKey) =>
                NativeMethods.RegisterHotKey(
                    windowHandle(), hotKeyId, modifiers | ModNoRepeat, virtualKey);

            public bool Unregister() =>
                NativeMethods.UnregisterHotKey(windowHandle(), hotKeyId);
        }
    }

    private sealed record PasteRequest(
        ClipboardPastePayload Command,
        Guid CorrelationId,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record ClipboardConfigurationRequest(
        ConfigureClipboardCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record ClipboardHotKeyRecordingRequest(
        SetClipboardHotKeyRecordingCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record QuickLaunchConfigurationRequest(
        ConfigureQuickLaunchCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record QuickLaunchHotKeyRecordingRequest(
        SetQuickLaunchHotKeyRecordingCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private sealed record DesktopShellConfigurationRequest(
        ConfigureDesktopShellCommand Command,
        CancellationToken CancellationToken,
        TaskCompletionSource<WindowsCommandResult> Completion);

    private static class NativeMethods
    {
        internal static int InputSize => Marshal.SizeOf<Input>();

        private const int SwRestore = 9;
        private const int SwMaximize = 3;
        private const uint InputKeyboard = 1;
        private const uint KeyeventfKeyup = 0x0002;
        private const ushort VkControl = 0x11;
        private const ushort VkV = 0x56;
        private const ushort VkShift = 0x10;
        private const ushort VkMenu = 0x12;
        private const ushort VkLWin = 0x5B;
        private const ushort VkRWin = 0x5C;
        private const ushort VkLShift = 0xA0;
        private const ushort VkRShift = 0xA1;
        private const ushort VkLControl = 0xA2;
        private const ushort VkRControl = 0xA3;
        private const ushort VkLMenu = 0xA4;
        private const ushort VkRMenu = 0xA5;
        private const int WmPaste = 0x0302;
        private const int WmCancelMode = 0x001F;
        private const int WmSysCommand = 0x0112;
        private const uint GaRoot = 2;
        private const uint SwpNoSize = 0x0001;
        private const uint SwpNoMove = 0x0002;
        private const uint SwpNoZOrder = 0x0004;
        private const uint SwpNoActivate = 0x0010;
        private const uint SwpFrameChanged = 0x0020;
        private const int GwlExStyle = -20;
        private const int GwlHwndParent = -8;
        private const int DwmCloaked = 14;
        private const long WsExTopmost = 0x00000008L;
        private const long WsExNoActivate = 0x08000000L;
        private const uint MonitorDefaultToNearest = 2;
        private const int MonitorDpiTypeEffective = 0;
        private const uint DefaultDpi = 96;
        private const int SwHide = 0;
        private const int SwShowNoActivate = 4;
        private const int SwShowMinNoActive = 7;

        [UnmanagedFunctionPointer(CallingConvention.Winapi)]
        internal delegate void WinEventDelegate(
            IntPtr hook,
            uint eventType,
            IntPtr window,
            int objectId,
            int childId,
            uint eventThread,
            uint eventTime);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool AddClipboardFormatListener(IntPtr window);

        [DllImport("dwmapi.dll")]
        private static extern int DwmGetWindowAttribute(
            IntPtr window,
            int attribute,
            out int value,
            int valueSize);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RemoveClipboardFormatListener(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool RegisterHotKey(
            IntPtr window,
            int id,
            uint modifiers,
            uint virtualKey);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnregisterHotKey(IntPtr window, int id);

        [DllImport("user32.dll", SetLastError = true)]
        internal static extern IntPtr SetWinEventHook(
            uint eventMin,
            uint eventMax,
            IntPtr module,
            WinEventDelegate callback,
            uint processId,
            uint threadId,
            uint flags);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool UnhookWinEvent(IntPtr hook);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool PostMessage(IntPtr window, int message, IntPtr wParam, IntPtr lParam);

        [DllImport("user32.dll")]
        internal static extern UIntPtr SetTimer(IntPtr window, UIntPtr timerId, uint milliseconds, IntPtr callback);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool KillTimer(IntPtr window, UIntPtr timerId);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        internal static extern bool IsWindow(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsWindowVisible(IntPtr window);

        [DllImport("user32.dll")]
        internal static extern uint GetClipboardSequenceNumber();

        [DllImport("user32.dll")]
        private static extern IntPtr GetForegroundWindow();

        [DllImport("user32.dll")]
        private static extern IntPtr GetAncestor(IntPtr window, uint flags);

        [DllImport("user32.dll")]
        private static extern short GetAsyncKeyState(int virtualKey);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetWindowPos(
            IntPtr window,
            IntPtr insertAfter,
            int x,
            int y,
            int width,
            int height,
            uint flags);

        [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
        private static extern IntPtr GetWindowLongPtr(IntPtr window, int index);

        [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
        private static extern IntPtr SetWindowLongPtr(IntPtr window, int index, IntPtr value);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetCursorPos(out Point point);

        [DllImport("user32.dll")]
        private static extern IntPtr WindowFromPoint(Point point);

        [DllImport("user32.dll")]
        private static extern IntPtr MonitorFromPoint(Point point, uint flags);

        [DllImport("shcore.dll")]
        private static extern int GetDpiForMonitor(
            IntPtr monitor,
            int dpiType,
            out uint dpiX,
            out uint dpiY);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool EnumDisplayMonitors(
            IntPtr deviceContext,
            IntPtr clipRect,
            MonitorEnumDelegate callback,
            IntPtr data);

        [DllImport("user32.dll")]
        private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

        [DllImport("kernel32.dll")]
        private static extern uint GetCurrentThreadId();

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool AttachThreadInput(uint attach, uint attachTo, bool isAttached);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool ShowWindow(IntPtr window, int command);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsIconic(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool IsZoomed(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool BringWindowToTop(IntPtr window);

        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool SetForegroundWindow(IntPtr window);

        [DllImport("user32.dll")]
        private static extern IntPtr GetFocus();

        [DllImport("user32.dll")]
        private static extern IntPtr SetFocus(IntPtr window);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

        internal static bool ActivateWindow(IntPtr target, IntPtr focusWindow)
        {
            if (IsIconic(target))
            {
                _ = ShowWindow(target, SwRestore);
            }
            var currentThread = GetCurrentThreadId();
            var targetThread = GetWindowThreadProcessId(target, out _);
            var foreground = GetForegroundWindow();
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out _);

            var attachedToTarget = targetThread != 0 && targetThread != currentThread
                && AttachThreadInput(currentThread, targetThread, true);
            var attachedToForeground = foregroundThread != 0
                && foregroundThread != currentThread
                && foregroundThread != targetThread
                && AttachThreadInput(currentThread, foregroundThread, true);
            var focusRestored = focusWindow == IntPtr.Zero;
            try
            {
                _ = BringWindowToTop(target);
                _ = SetForegroundWindow(target);
                if (focusWindow != IntPtr.Zero &&
                    IsWindow(focusWindow))
                {
                    if (GetFocus() != focusWindow)
                    {
                        _ = SetFocus(focusWindow);
                    }
                    focusRestored = GetFocus() == focusWindow;
                }
            }
            finally
            {
                if (attachedToForeground)
                {
                    _ = AttachThreadInput(currentThread, foregroundThread, false);
                }
                if (attachedToTarget)
                {
                    _ = AttachThreadInput(currentThread, targetThread, false);
                }
            }
            return IsForegroundWindowForTarget(target) && focusRestored;
        }

        internal static bool IsForegroundWindowForTarget(IntPtr target)
        {
            var foreground = GetForegroundWindow();
            if (foreground == target)
            {
                return true;
            }
            if (foreground == IntPtr.Zero || target == IntPtr.Zero)
            {
                return false;
            }
            var targetRoot = GetAncestor(target, GaRoot);
            var foregroundRoot = GetAncestor(foreground, GaRoot);
            return targetRoot != IntPtr.Zero && targetRoot == foregroundRoot;
        }

        internal static long GetRootWindowHandle(IntPtr window)
        {
            if (window == IntPtr.Zero)
            {
                return 0;
            }
            var root = GetAncestor(window, GaRoot);
            return (root == IntPtr.Zero ? window : root).ToInt64();
        }

        internal static WindowsCommandResult SetWindowOwner(
            IntPtr window,
            IntPtr owner)
        {
            if (window == IntPtr.Zero || owner == IntPtr.Zero ||
                !IsWindow(window) || !IsWindow(owner) || window == owner)
            {
                return new WindowsCommandResult(
                    false,
                    "The GPUI window or file-dialog owner is unavailable.");
            }
            Marshal.SetLastPInvokeError(0);
            var previous = SetWindowLongPtr(window, GwlHwndParent, owner);
            var error = Marshal.GetLastPInvokeError();
            if (previous == IntPtr.Zero && error != 0)
            {
                return new WindowsCommandResult(
                    false,
                    $"Windows could not attach the GPUI search window to its file dialog (error: {error}).");
            }
            return new WindowsCommandResult(true);
        }

        internal static PasteWindowSnapshot CaptureWindowState(IntPtr target)
        {
            var currentThread = GetCurrentThreadId();
            uint targetProcess = 0;
            var targetThread = target == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(target, out targetProcess);
            var foreground = GetForegroundWindow();
            uint foregroundProcess = 0;
            var foregroundThread = foreground == IntPtr.Zero
                ? 0
                : GetWindowThreadProcessId(foreground, out foregroundProcess);
            return new PasteWindowSnapshot(
                foreground.ToInt64(),
                GetAncestor(foreground, GaRoot).ToInt64(),
                GetAncestor(target, GaRoot).ToInt64(),
                currentThread,
                targetThread,
                foregroundThread,
                targetProcess,
                foregroundProcess);
        }

        internal static ModifierReleaseResult ReleaseActivationModifiers()
        {
            ushort[] keys =
            [
                VkShift, VkControl, VkMenu,
                VkLShift, VkRShift, VkLControl, VkRControl, VkLMenu, VkRMenu,
                VkLWin, VkRWin
            ];
            var initiallyPressed = keys.Where(IsKeyDown).ToArray();
            var inputSteps = new List<KeyboardInputStep>();
            var attempts = 0;
            for (; attempts < 12; attempts++)
            {
                var pressed = keys.Where(IsKeyDown).ToArray();
                if (pressed.Length == 0)
                {
                    return new ModifierReleaseResult(true, initiallyPressed, [], attempts, inputSteps);
                }
                foreach (var key in pressed)
                {
                    inputSteps.Add(SendKeyboardInput("modifier_up", key, KeyeventfKeyup));
                }
                Thread.Sleep(3);
            }
            var finallyPressed = keys.Where(IsKeyDown).ToArray();
            return new ModifierReleaseResult(
                finallyPressed.Length == 0,
                initiallyPressed,
                finallyPressed,
                attempts,
                inputSteps);
        }

        internal static PasteShortcutResult SendPasteShortcut()
        {
            KeyboardInputStep[] steps =
            [
                SendKeyboardInput("ctrl_down", VkControl, 0),
                DelayThenSend(10, "v_down", VkV, 0),
                DelayThenSend(10, "v_up", VkV, KeyeventfKeyup),
                DelayThenSend(5, "ctrl_up", VkControl, KeyeventfKeyup)
            ];
            return new PasteShortcutResult(
                steps.Aggregate(0U, static (sent, step) => sent + step.Sent),
                Marshal.SizeOf<Input>(),
                steps);
        }

        private static KeyboardInputStep DelayThenSend(
            int milliseconds,
            string name,
            ushort key,
            uint flags)
        {
            Thread.Sleep(milliseconds);
            return SendKeyboardInput(name, key, flags);
        }

        internal static PostPasteResult PostPaste(IntPtr target)
        {
            if (target == IntPtr.Zero || !IsWindow(target))
            {
                return new PostPasteResult(false, 1400, "The paste target is no longer a valid window.");
            }
            Marshal.SetLastPInvokeError(0);
            if (PostMessage(target, WmPaste, IntPtr.Zero, IntPtr.Zero))
            {
                return new PostPasteResult(true, 0, null);
            }
            var error = Marshal.GetLastPInvokeError();
            return new PostPasteResult(
                false,
                error,
                $"PostMessage(WM_PASTE) failed with Win32={error}.");
        }

        internal static WindowsCommandResult SetWindowTopmost(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            var insertAfter = enabled ? new IntPtr(-1) : new IntPtr(-2);
            var lastError = 0;
            for (var attempt = 0; attempt < 3; attempt++)
            {
                if (!SetWindowPos(
                    window,
                    insertAfter,
                    0,
                    0,
                    0,
                    0,
                    SwpNoMove | SwpNoSize | SwpNoActivate))
                {
                    lastError = Marshal.GetLastWin32Error();
                }
                else if (IsTopmost(window) == enabled)
                {
                    return new WindowsCommandResult(true);
                }
                Thread.Sleep(10);
            }
            return new WindowsCommandResult(
                false,
                lastError == 0
                    ? "Windows did not apply the requested topmost state."
                    : $"SetWindowPos failed with Win32={lastError}.");
        }

        internal static WindowsCommandResult SetWindowNoActivate(IntPtr window, bool enabled)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            var current = GetWindowLongPtr(window, GwlExStyle).ToInt64();
            var desired = enabled ? current | WsExNoActivate : current & ~WsExNoActivate;
            if (desired != current)
            {
                Marshal.SetLastPInvokeError(0);
                var previous = SetWindowLongPtr(window, GwlExStyle, new IntPtr(desired));
                var error = Marshal.GetLastPInvokeError();
                if (previous == IntPtr.Zero && error != 0)
                {
                    return new WindowsCommandResult(
                        false,
                        $"SetWindowLongPtr failed with Win32={error}.");
                }
                if (!SetWindowPos(
                        window,
                        IntPtr.Zero,
                        0,
                        0,
                        0,
                        0,
                        SwpNoMove | SwpNoSize | SwpNoZOrder | SwpNoActivate | SwpFrameChanged))
                {
                    return new WindowsCommandResult(
                        false,
                        $"SetWindowPos(SWP_FRAMECHANGED) failed with Win32={Marshal.GetLastWin32Error()}.");
                }
            }

            var applied = (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExNoActivate) != 0;
            return applied == enabled
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested no-activate state.");
        }

        internal static WindowsCommandResult SetWindowVisibility(IntPtr window, bool visible)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            _ = ShowWindow(window, visible ? SwShowNoActivate : SwHide);
            return IsWindowVisible(window) == visible
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested visibility state.");
        }

        internal static WindowsCommandResult SetWindowMinimized(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            _ = ShowWindow(window, SwShowMinNoActive);
            for (var attempt = 0; attempt < 3; attempt++)
            {
                if (IsWindowVisible(window) && IsIconic(window) && GetForegroundWindow() != window)
                {
                    return new WindowsCommandResult(true);
                }
                Thread.Sleep(10);
            }
            return new WindowsCommandResult(
                false,
                "Windows did not show the requested GPUI window minimized without activation.");
        }

        internal static WindowsCommandResult SetWindowMaximized(IntPtr window, bool maximized)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = ShowWindow(window, maximized ? SwMaximize : SwRestore);
            return IsZoomed(window) == maximized
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, "Windows did not apply the requested maximized state.");
        }

        internal static WindowsCommandResult ToggleWindowMaximized(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            return SetWindowMaximized(window, !IsZoomed(window));
        }

        internal static NativeWindowPresentationState CaptureWindowPresentation(IntPtr window)
        {
            var exists = window != IntPtr.Zero && IsWindow(window);
            var bounds = default(NativeWindowRectangle);
            var cloaked = false;
            if (exists)
            {
                _ = AnchoredWindowPlacement.TryGetVisibleBounds(window, out bounds);
                try
                {
                    cloaked = DwmGetWindowAttribute(
                        window,
                        DwmCloaked,
                        out var cloakedValue,
                        sizeof(int)) == 0 && cloakedValue != 0;
                }
                catch (DllNotFoundException)
                {
                }
                catch (EntryPointNotFoundException)
                {
                }
            }
            return new NativeWindowPresentationState(
                exists,
                exists && IsWindowVisible(window),
                exists && IsTopmost(window),
                cloaked,
                exists ? GetWindowLongPtr(window, GwlHwndParent).ToInt64() : 0,
                GetForegroundWindow().ToInt64(),
                bounds);
        }

        internal static WindowsCommandResult SetWindowBounds(
            IntPtr window,
            int x,
            int y,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }

            return SetWindowPos(
                window,
                IntPtr.Zero,
                x,
                y,
                width,
                height,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(bounds) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        /// <summary>
        /// Resizes the window while preserving its real current visual center.
        /// GPUI's cached window origin never follows a user drag, so the center
        /// must come from the native rectangle, not from the Rust side.
        /// </summary>
        internal static WindowsCommandResult SetWindowBoundsCentered(
            IntPtr window,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var current))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            var centered = CenteredRectangle(current, width, height);
            return SetWindowPos(
                window,
                IntPtr.Zero,
                centered.Left,
                centered.Top,
                width,
                height,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(centered) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        /// <summary>
        /// Places the window centered over the anchor window's real rectangle,
        /// preserving the anchor's visual center across a window replacement.
        /// A missing anchor falls back to resizing in place at the current
        /// position so the handoff never teleports the window.
        /// </summary>
        internal static WindowsCommandResult MoveWindowCenteredOver(
            IntPtr window,
            IntPtr anchorWindow,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var target))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            NativeWindowRectangle anchorRect = default;
            var anchorAvailable = anchorWindow != IntPtr.Zero &&
                IsWindow(anchorWindow) &&
                AnchoredWindowPlacement.TryGetVisibleBounds(anchorWindow, out anchorRect);
            var centered = anchorAvailable
                ? CenteredRectangle(anchorRect, width, height)
                : CenteredRectangle(target, width, height);
            return SetWindowPos(
                window,
                IntPtr.Zero,
                centered.Left,
                centered.Top,
                width,
                height,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(centered-over) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static NativeWindowRectangle CenteredRectangle(
            NativeWindowRectangle anchor,
            int width,
            int height)
        {
            var centerX = anchor.Left + (anchor.Right - anchor.Left) / 2.0;
            var centerY = anchor.Top + (anchor.Bottom - anchor.Top) / 2.0;
            var left = (int)Math.Round(centerX - width / 2.0);
            var top = (int)Math.Round(centerY - height / 2.0);
            return new NativeWindowRectangle(left, top, left + width, top + height);
        }

        private const int WindowCenterStashCapacity = 16;
        private static readonly ConcurrentDictionary<string, NativeWindowRectangle>
            WindowCenterStash = new(StringComparer.Ordinal);

        /// <summary>
        /// Captures a window's real visible rectangle under a correlation key so
        /// a replacement window can later inherit that position after this
        /// window is destroyed. GPUI's cached origin never follows user drags,
        /// which is why the geometry is captured here, natively.
        /// </summary>
        internal static WindowsCommandResult StashWindowBounds(IntPtr window, string key)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (string.IsNullOrWhiteSpace(key))
            {
                return new WindowsCommandResult(false, "The window stash key is empty.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var bounds))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            if (WindowCenterStash.Count >= WindowCenterStashCapacity)
            {
                WindowCenterStash.Clear();
            }
            WindowCenterStash[key] = bounds;
            return new WindowsCommandResult(true);
        }

        /// <summary>
        /// Places the window centered over a stashed anchor rectangle and
        /// consumes it. A missing stash entry falls back to an in-place resize
        /// at the current position so the placement never teleports the window.
        /// </summary>
        internal static WindowsCommandResult MoveWindowCenteredOverKey(
            IntPtr window,
            string key,
            int width,
            int height)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            if (width <= 0 || height <= 0)
            {
                return new WindowsCommandResult(false, "Window bounds must have a positive size.");
            }
            if (!AnchoredWindowPlacement.TryGetVisibleBounds(window, out var target))
            {
                return new WindowsCommandResult(false, "The window rectangle is unavailable.");
            }

            NativeWindowRectangle anchorRect = default;
            var stashHit = !string.IsNullOrWhiteSpace(key) &&
                WindowCenterStash.TryRemove(key, out anchorRect);
            var centered = stashHit
                ? CenteredRectangle(anchorRect, width, height)
                : CenteredRectangle(target, width, height);
            return SetWindowPos(
                window,
                IntPtr.Zero,
                centered.Left,
                centered.Top,
                width,
                height,
                SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"SetWindowPos(stash-centered) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult SetWindowPosition(IntPtr window, int x, int y)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }

            return SetWindowPos(
                window,
                IntPtr.Zero,
                x,
                y,
                0,
                0,
                SwpNoSize | SwpNoZOrder | SwpNoActivate)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(false, $"SetWindowPos failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult BeginWindowResize(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = PostMessage(window, WmCancelMode, IntPtr.Zero, IntPtr.Zero);
            var resizeCommand = new IntPtr(BottomRightResizeSystemCommand);
            return PostMessage(window, WmSysCommand, resizeCommand, IntPtr.Zero)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"PostMessage(WM_SYSCOMMAND/SC_SIZE) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        internal static WindowsCommandResult BeginWindowMove(IntPtr window)
        {
            if (window == IntPtr.Zero || !IsWindow(window))
            {
                return new WindowsCommandResult(false, "The GPUI window handle is invalid.");
            }
            _ = PostMessage(window, WmCancelMode, IntPtr.Zero, IntPtr.Zero);
            return PostMessage(window, WmSysCommand, new IntPtr(MoveCaptionSystemCommand), IntPtr.Zero)
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    $"PostMessage(WM_SYSCOMMAND/SC_MOVE) failed with Win32={Marshal.GetLastWin32Error()}.");
        }

        private static bool IsTopmost(IntPtr window) =>
            (GetWindowLongPtr(window, GwlExStyle).ToInt64() & WsExTopmost) != 0;

        internal static CursorLocation GetCursorLocation()
        {
            var position = GetCursorPos(out var point) ? point : default;
            var monitor = MonitorFromPoint(position, MonitorDefaultToNearest);
            var dpi = DefaultDpi;
            if (monitor != IntPtr.Zero &&
                GetDpiForMonitor(monitor, MonitorDpiTypeEffective, out var dpiX, out _) == 0 &&
                dpiX > 0)
            {
                dpi = dpiX;
            }
            return new CursorLocation(position, GetDisplayIndex(monitor), dpi);
        }

        internal static bool IsPointerButtonDown(int virtualKey) =>
            (GetAsyncKeyState(virtualKey) & 0x8000) != 0;

        internal static PointerTarget GetPointerTarget()
        {
            if (!GetCursorPos(out var point))
            {
                return default;
            }
            var window = WindowFromPoint(point);
            var root = window == IntPtr.Zero ? IntPtr.Zero : GetAncestor(window, GaRoot);
            _ = GetWindowThreadProcessId(root != IntPtr.Zero ? root : window, out var processId);
            return new PointerTarget(window.ToInt64(), root.ToInt64(), processId);
        }

        private static uint GetDisplayIndex(IntPtr targetMonitor)
        {
            uint current = 0;
            uint? result = null;
            _ = EnumDisplayMonitors(
                IntPtr.Zero,
                IntPtr.Zero,
                (monitor, _, _, _) =>
                {
                    if (monitor == targetMonitor)
                    {
                        result = current;
                        return false;
                    }
                    current++;
                    return true;
                },
                IntPtr.Zero);
            return result ?? 0;
        }

        private static Input KeyboardInput(ushort key, uint flags) => new()
        {
            Type = InputKeyboard,
            Union = new InputUnion
            {
                Keyboard = new KeyboardInputData
                {
                    VirtualKey = key,
                    Flags = flags
                }
            }
        };

        private static bool IsKeyDown(ushort key) => (GetAsyncKeyState(key) & 0x8000) != 0;

        private static KeyboardInputStep SendKeyboardInput(string name, ushort key, uint flags)
        {
            var input = new[] { KeyboardInput(key, flags) };
            Marshal.SetLastPInvokeError(0);
            var sent = SendInput(1, input, Marshal.SizeOf<Input>());
            var error = sent == 0 ? Marshal.GetLastPInvokeError() : 0;
            return new KeyboardInputStep(name, key, flags, sent, error);
        }

        [StructLayout(LayoutKind.Sequential)]
        internal struct Point
        {
            public int X;
            public int Y;
        }

        internal readonly record struct CursorLocation(Point Position, uint DisplayIndex, uint Dpi);

        internal readonly record struct PointerTarget(
            long WindowHandle,
            long RootWindowHandle,
            uint ProcessId);

        private delegate bool MonitorEnumDelegate(
            IntPtr monitor,
            IntPtr deviceContext,
            IntPtr monitorRect,
            IntPtr data);

        [StructLayout(LayoutKind.Sequential)]
        private struct Input
        {
            public uint Type;
            public InputUnion Union;
        }

        [StructLayout(LayoutKind.Explicit)]
        private struct InputUnion
        {
            [FieldOffset(0)]
            public KeyboardInputData Keyboard;

            [FieldOffset(0)]
            public MouseInputData Mouse;

            [FieldOffset(0)]
            public HardwareInputData Hardware;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct MouseInputData
        {
            public int X;
            public int Y;
            public uint MouseData;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct HardwareInputData
        {
            public uint Message;
            public ushort ParameterLow;
            public ushort ParameterHigh;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct KeyboardInputData
        {
            public ushort VirtualKey;
            public ushort ScanCode;
            public uint Flags;
            public uint Time;
            public UIntPtr ExtraInfo;
        }
    }
}
