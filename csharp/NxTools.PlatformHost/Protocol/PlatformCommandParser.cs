using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record PasteTextCommand(
    string Text,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool PasteAfterWrite = true);
internal sealed record PasteFilesCommand(
    IReadOnlyList<string> Paths,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool PasteAfterWrite = true);
internal sealed record PasteImageCommand(
    string ImagePath,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool IncludeFileDropFallback = true,
    bool PasteAfterWrite = true);
internal sealed record ClipboardNativeFormatData(
    uint FormatId,
    string FormatName,
    string DataPath,
    ulong SizeBytes,
    uint SortOrder);
internal sealed record PasteNativeFormatsCommand(
    IReadOnlyList<ClipboardNativeFormatData> Formats,
    bool PasteAfterWrite,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    string? ImagePathFallback = null);
internal sealed record ConfigureClipboardCommand(
    IReadOnlyList<string> HotkeyModifiers,
    string HotkeyKey,
    string HotkeyDisplay,
    IReadOnlyList<string> ExcludedProcesses);
internal sealed record SetClipboardHotKeyRecordingCommand(bool Recording);
internal sealed record ConfigureQuickLaunchCommand(
    bool Enabled,
    IReadOnlyList<string> HotkeyModifiers,
    string HotkeyKey,
    string HotkeyDisplay,
    bool AutoQuickSwitchEnabled = true,
    bool FileDialogSearchEnabled = true);
internal sealed record SetQuickLaunchHotKeyRecordingCommand(bool Recording);
internal sealed record ConfigureDesktopShellCommand(
    bool StartOnLoginSilently,
    bool TrayIconVisible);

internal sealed record SetWindowTopmostCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowNoActivateCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowShadowCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowBoundsCommand(long WindowHandle, int X, int Y, int Width, int Height);
internal sealed record SetWindowBoundsCenteredCommand(long WindowHandle, int Width, int Height);
internal sealed record MoveWindowCenteredOverCommand(
    long WindowHandle,
    long AnchorWindowHandle,
    int Width,
    int Height);
internal sealed record StashWindowBoundsCommand(long WindowHandle, string Key);
internal sealed record MoveWindowCenteredOverKeyCommand(
    long WindowHandle,
    string Key,
    int Width,
    int Height);
internal sealed record SetTaskbarProgressCommand(
    long WindowHandle,
    string State,
    ulong Completed,
    ulong Total);
internal sealed record PlaceTaskbarProgressOverlayCommand(
    long WindowHandle,
    bool Enabled,
    bool Visible,
    string Anchor,
    int OffsetX,
    int OffsetY,
    int Width,
    int Height,
    bool Interactive = false);
internal sealed record SetTaskbarProgressOverlayVisibilityCommand(
    long WindowHandle,
    bool Visible);
internal sealed record PlaceFileDialogSearchWindowCommand(
    long WindowHandle,
    long OwnerWindowHandle,
    int VisibleWidth,
    int VisibleHeight,
    int BottomOverlap);
internal sealed record SetWindowOwnerCommand(long WindowHandle, long OwnerWindowHandle);
internal sealed record SetWindowMinimizedCommand(long WindowHandle);
internal sealed record ToggleWindowMaximizedCommand(long WindowHandle);
internal sealed record SetWindowVisibilityCommand(long WindowHandle, bool Visible);
internal sealed record SetWindowPositionCommand(long WindowHandle, int X, int Y);
internal sealed record ActivateWindowCommand(long WindowHandle);
internal sealed record BeginWindowResizeCommand(long WindowHandle);
internal sealed record BeginWindowMoveCommand(long WindowHandle);
internal sealed record BeginFileDragCommand(string Path, long SourceWindowHandle);
internal sealed record OpenShellTargetCommand(string Target, string Action);
internal sealed record ShutdownSystemCommand(ushort DelaySeconds);
internal sealed record ExportBrowserExtensionCommand(string Format, string Destination);
internal sealed record LaunchProcessCommand(
    string Executable,
    IReadOnlyList<string> Arguments,
    string? WorkingDirectory,
    bool RunAsAdmin = false,
    bool RunSilently = false);
internal sealed record NavigateDirectoryCommand(
    string Host,
    long WindowHandle,
    string TargetDirectory,
    string? HostContextId = null);
internal sealed record NavigateExplorerCommand(long WindowHandle, string TargetDirectory);
internal sealed record NavigateDesktopCommand(long WindowHandle, string TargetDirectory);
internal sealed record NavigateFileDialogCommand(long WindowHandle, string TargetDirectory);
internal sealed record OpenFileInDialogCommand(long WindowHandle, string FilePath);
internal sealed record NavigateThirdPartyFileManagerCommand(
    long WindowHandle,
    string TargetDirectory,
    string? HostContextId);
internal sealed record QueryShellActionsCommand(string Target);
internal sealed record QueryShellSubmenuCommand(string SessionId, uint SubmenuId);
internal sealed record InvokeShellActionCommand(
    string SessionId,
    uint CommandId,
    long OwnerWindowHandle);
internal sealed record ExtractShellIconCommand(string Target, uint Size);
internal sealed record PickClipboardArchivePathCommand(string Mode, string SuggestedFileName);
internal sealed record PickClipboardStorageDirectoryCommand(string CurrentDirectory);
internal sealed record PickDirectoryCommand(string CurrentDirectory, string Title);
internal sealed record PickProgramCommand(string CurrentPath);
internal sealed record EnsureIndexerServiceCommand;
internal sealed record EnsureBrowserNativeMessagingCommand(
    bool Enabled,
    string ChromiumExtensionId,
    string FirefoxExtensionId);

internal static class PlatformCommandParser
{
    public static PasteTextCommand ParsePasteText(ProtocolEnvelope envelope)
    {
        var command = GetCommandName(envelope);
        if (command != "paste_text")
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }

        var data = envelope.Payload.GetProperty("data");
        var parsed = data.Deserialize<PasteTextCommand>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException("Paste command payload was empty.");
    }

    public static PasteFilesCommand ParsePasteFiles(ProtocolEnvelope envelope) =>
        Parse<PasteFilesCommand>(envelope, "paste_files", "Files paste command payload was empty.");

    public static PasteImageCommand ParsePasteImage(ProtocolEnvelope envelope) =>
        Parse<PasteImageCommand>(envelope, "paste_image", "Image paste command payload was empty.");

    public static PasteNativeFormatsCommand ParsePasteNativeFormats(ProtocolEnvelope envelope) =>
        Parse<PasteNativeFormatsCommand>(
            envelope,
            "paste_native_formats",
            "Native formats paste command payload was empty.");

    public static ConfigureClipboardCommand ParseConfigureClipboard(ProtocolEnvelope envelope) =>
        Parse<ConfigureClipboardCommand>(
            envelope,
            "configure_clipboard",
            "Clipboard configuration command payload was empty.");

    public static SetClipboardHotKeyRecordingCommand ParseSetClipboardHotKeyRecording(
        ProtocolEnvelope envelope) =>
        Parse<SetClipboardHotKeyRecordingCommand>(
            envelope,
            "set_clipboard_hot_key_recording",
            "Clipboard hotkey recording command payload was empty.");

    public static ConfigureQuickLaunchCommand ParseConfigureQuickLaunch(ProtocolEnvelope envelope) =>
        Parse<ConfigureQuickLaunchCommand>(
            envelope,
            "configure_quick_launch",
            "Quick-launch configuration command payload was empty.");

    public static SetQuickLaunchHotKeyRecordingCommand ParseSetQuickLaunchHotKeyRecording(
        ProtocolEnvelope envelope) =>
        Parse<SetQuickLaunchHotKeyRecordingCommand>(
            envelope,
            "set_quick_launch_hot_key_recording",
            "Quick-launch hotkey recording command payload was empty.");

    private static T Parse<T>(ProtocolEnvelope envelope, string expected, string emptyMessage)
    {
        var command = GetCommandName(envelope);
        if (command != expected)
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }
        var parsed = envelope.Payload.GetProperty("data").Deserialize<T>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException(emptyMessage);
    }

    public static SetWindowTopmostCommand ParseSetWindowTopmost(ProtocolEnvelope envelope)
    {
        var command = GetCommandName(envelope);
        if (command != "set_window_topmost")
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }

        var data = envelope.Payload.GetProperty("data");
        var parsed = data.Deserialize<SetWindowTopmostCommand>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException("Topmost command payload was empty.");
    }

    public static ConfigureDesktopShellCommand ParseConfigureDesktopShell(ProtocolEnvelope envelope) =>
        Parse<ConfigureDesktopShellCommand>(
            envelope,
            "configure_desktop_shell",
            "Desktop-shell command payload was empty.");

    public static SetWindowVisibilityCommand ParseSetWindowVisibility(ProtocolEnvelope envelope)
    {
        var command = GetCommandName(envelope);
        if (command != "set_window_visibility")
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }

        var data = envelope.Payload.GetProperty("data");
        var parsed = data.Deserialize<SetWindowVisibilityCommand>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException("Visibility command payload was empty.");
    }

    public static SetWindowNoActivateCommand ParseSetWindowNoActivate(ProtocolEnvelope envelope) =>
        Parse<SetWindowNoActivateCommand>(
            envelope,
            "set_window_no_activate",
            "No-activate command payload was empty.");

public static SetWindowShadowCommand ParseSetWindowShadow(ProtocolEnvelope envelope) =>
        Parse<SetWindowShadowCommand>(
            envelope,
            "set_window_shadow",
            "Window-shadow command payload was empty.");
    public static SetWindowBoundsCommand ParseSetWindowBounds(ProtocolEnvelope envelope) =>
        Parse<SetWindowBoundsCommand>(
            envelope,
            "set_window_bounds",
            "Window-bounds command payload was empty.");

    public static SetWindowBoundsCenteredCommand ParseSetWindowBoundsCentered(
        ProtocolEnvelope envelope) =>
        Parse<SetWindowBoundsCenteredCommand>(
            envelope,
            "set_window_bounds_centered",
            "Centered window-bounds command payload was empty.");

    public static MoveWindowCenteredOverCommand ParseMoveWindowCenteredOver(
        ProtocolEnvelope envelope) =>
        Parse<MoveWindowCenteredOverCommand>(
            envelope,
            "move_window_centered_over",
            "Window-anchor placement payload was empty.");

    public static StashWindowBoundsCommand ParseStashWindowBounds(ProtocolEnvelope envelope) =>
        Parse<StashWindowBoundsCommand>(
            envelope,
            "stash_window_bounds",
            "Window-stash command payload was empty.");

    public static MoveWindowCenteredOverKeyCommand ParseMoveWindowCenteredOverKey(
        ProtocolEnvelope envelope) =>
        Parse<MoveWindowCenteredOverKeyCommand>(
            envelope,
            "move_window_centered_over_key",
            "Stash-anchored placement payload was empty.");

    public static SetTaskbarProgressCommand ParseSetTaskbarProgress(ProtocolEnvelope envelope) =>
        Parse<SetTaskbarProgressCommand>(
            envelope,
            "set_taskbar_progress",
            "Taskbar-progress command payload was empty.");

    public static PlaceTaskbarProgressOverlayCommand ParsePlaceTaskbarProgressOverlay(
        ProtocolEnvelope envelope) =>
        Parse<PlaceTaskbarProgressOverlayCommand>(
            envelope,
            "place_taskbar_progress_overlay",
            "Taskbar-overlay placement payload was empty.");

    public static SetTaskbarProgressOverlayVisibilityCommand
        ParseSetTaskbarProgressOverlayVisibility(ProtocolEnvelope envelope) =>
        Parse<SetTaskbarProgressOverlayVisibilityCommand>(
            envelope,
            "set_taskbar_progress_overlay_visibility",
            "Taskbar-overlay visibility payload was empty.");

    public static PlaceFileDialogSearchWindowCommand ParsePlaceFileDialogSearchWindow(
        ProtocolEnvelope envelope) =>
        Parse<PlaceFileDialogSearchWindowCommand>(
            envelope,
            "place_file_dialog_search_window",
            "File-dialog search placement payload was empty.");

    public static SetWindowOwnerCommand ParseSetWindowOwner(ProtocolEnvelope envelope) =>
        Parse<SetWindowOwnerCommand>(
            envelope,
            "set_window_owner",
            "Window-owner command payload was empty.");

    public static SetWindowMinimizedCommand ParseSetWindowMinimized(ProtocolEnvelope envelope) =>
        Parse<SetWindowMinimizedCommand>(
            envelope,
            "set_window_minimized",
            "Window-minimized command payload was empty.");

    public static ToggleWindowMaximizedCommand ParseToggleWindowMaximized(ProtocolEnvelope envelope) =>
        Parse<ToggleWindowMaximizedCommand>(
            envelope,
            "toggle_window_maximized",
            "Window maximize-toggle command payload was empty.");

    public static BeginWindowResizeCommand ParseBeginWindowResize(ProtocolEnvelope envelope) =>
        Parse<BeginWindowResizeCommand>(envelope, "begin_window_resize", "Resize command payload was empty.");

    public static BeginWindowMoveCommand ParseBeginWindowMove(ProtocolEnvelope envelope) =>
        Parse<BeginWindowMoveCommand>(envelope, "begin_window_move", "Move command payload was empty.");

    public static BeginFileDragCommand ParseBeginFileDrag(ProtocolEnvelope envelope) =>
        Parse<BeginFileDragCommand>(
            envelope,
            "begin_file_drag",
            "File-drag command payload was empty.");

    public static SetWindowPositionCommand ParseSetWindowPosition(ProtocolEnvelope envelope) =>
        Parse<SetWindowPositionCommand>(envelope, "set_window_position", "Window position payload was empty.");

    public static ActivateWindowCommand ParseActivateWindow(ProtocolEnvelope envelope) =>
        Parse<ActivateWindowCommand>(envelope, "activate_window", "Activate window payload was empty.");

    public static OpenShellTargetCommand ParseOpenShellTarget(ProtocolEnvelope envelope) =>
        Parse<OpenShellTargetCommand>(envelope, "open_shell_target", "Open target command payload was empty.");

    public static ShutdownSystemCommand ParseShutdownSystem(ProtocolEnvelope envelope) =>
        Parse<ShutdownSystemCommand>(envelope, "shutdown_system", "Shutdown command payload was empty.");

    public static ExportBrowserExtensionCommand ParseExportBrowserExtension(ProtocolEnvelope envelope) =>
        Parse<ExportBrowserExtensionCommand>(
            envelope,
            "export_browser_extension",
            "Browser extension export command payload was empty.");

    public static LaunchProcessCommand ParseLaunchProcess(ProtocolEnvelope envelope) =>
        Parse<LaunchProcessCommand>(
            envelope,
            "launch_process",
            "Process launch command payload was empty.");

    public static NavigateDirectoryCommand ParseNavigateDirectory(ProtocolEnvelope envelope) =>
        Parse<NavigateDirectoryCommand>(
            envelope,
            "navigate_directory",
            "Directory navigation command payload was empty.");

    public static OpenFileInDialogCommand ParseOpenFileInDialog(ProtocolEnvelope envelope) =>
        Parse<OpenFileInDialogCommand>(
            envelope,
            "open_file_in_dialog",
            "File-dialog open command payload was empty.");

    public static QueryShellActionsCommand ParseQueryShellActions(ProtocolEnvelope envelope) =>
        Parse<QueryShellActionsCommand>(
            envelope, "query_shell_actions", "Shell action query payload was empty.");

    public static QueryShellSubmenuCommand ParseQueryShellSubmenu(ProtocolEnvelope envelope) =>
        Parse<QueryShellSubmenuCommand>(
            envelope, "query_shell_submenu", "Shell submenu query payload was empty.");

    public static InvokeShellActionCommand ParseInvokeShellAction(ProtocolEnvelope envelope) =>
        Parse<InvokeShellActionCommand>(
            envelope, "invoke_shell_action", "Shell action invocation payload was empty.");

    public static ExtractShellIconCommand ParseExtractShellIcon(ProtocolEnvelope envelope) =>
        Parse<ExtractShellIconCommand>(envelope, "extract_shell_icon", "Shell icon command payload was empty.");

    public static PickClipboardArchivePathCommand ParsePickClipboardArchivePath(
        ProtocolEnvelope envelope) =>
        Parse<PickClipboardArchivePathCommand>(
            envelope,
            "pick_clipboard_archive_path",
            "Clipboard archive picker payload was empty.");

    public static PickClipboardStorageDirectoryCommand ParsePickClipboardStorageDirectory(
        ProtocolEnvelope envelope) =>
        Parse<PickClipboardStorageDirectoryCommand>(
            envelope,
            "pick_clipboard_storage_directory",
            "Clipboard storage picker payload was empty.");

    public static PickDirectoryCommand ParsePickDirectory(ProtocolEnvelope envelope) =>
        Parse<PickDirectoryCommand>(
            envelope,
            "pick_directory",
            "Directory picker payload was empty.");

    public static PickProgramCommand ParsePickProgram(ProtocolEnvelope envelope) =>
        Parse<PickProgramCommand>(
            envelope,
            "pick_program",
            "Program picker payload was empty.");

    public static EnsureIndexerServiceCommand ParseEnsureIndexerService(ProtocolEnvelope envelope)
    {
        var command = GetCommandName(envelope);
        if (command != "ensure_indexer_service")
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }
        return new EnsureIndexerServiceCommand();
    }

    public static EnsureBrowserNativeMessagingCommand ParseEnsureBrowserNativeMessaging(
        ProtocolEnvelope envelope) =>
        Parse<EnsureBrowserNativeMessagingCommand>(
            envelope,
            "ensure_browser_native_messaging",
            "Browser native messaging registration payload was empty.");

    public static string GetCommandName(ProtocolEnvelope envelope)
    {
        if (envelope.Type != "platform_command")
        {
            throw new InvalidDataException("Envelope is not a platform command.");
        }
        return envelope.Payload.GetProperty("command").GetString()
            ?? throw new InvalidDataException("Platform command name was empty.");
    }
}
