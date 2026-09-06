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

internal sealed partial class WindowsPlatformAdapter : IDisposable
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
}
