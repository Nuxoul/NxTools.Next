using System.Text.Json;
using NxTools.PlatformHost.Windows;

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
    IReadOnlyList<string> ExcludedProcesses,
    ulong MaxTextBytes = 8UL * 1024 * 1024,
    ulong MaxImageBytes = 64UL * 1024 * 1024,
    ulong MaxImagePixels = 100_000_000,
    bool SuppressInFullscreen = true);
internal sealed record SetClipboardCapturePausedCommand(bool Paused);
internal sealed record SetClipboardHotKeyRecordingCommand(bool Recording);
internal sealed record ConfigureQuickLaunchCommand(
    bool Enabled,
    IReadOnlyList<string> HotkeyModifiers,
    string HotkeyKey,
    string HotkeyDisplay,
    bool AutoQuickSwitchEnabled = true,
    bool FileDialogSearchEnabled = true,
    bool SuppressInExclusiveFullscreen = true);
internal sealed record SetQuickLaunchHotKeyRecordingCommand(bool Recording);
internal sealed record ConfigureDesktopShellCommand(
    bool StartOnLoginSilently,
    bool TrayIconVisible);

internal sealed record SetWindowTopmostCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowNoActivateCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowShadowCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowBoundsCommand(long WindowHandle, int X, int Y, int Width, int Height);
internal sealed record SetWindowBoundsCenteredCommand(long WindowHandle, int Width, int Height);
internal sealed record PresentFixedWindowOverAnchorCommand(long WindowHandle, long AnchorWindowHandle, int Width, int Height);
internal sealed record PrepareNotificationWindowCommand(long WindowHandle, long SourceWindowHandle, uint SourceProcessId,
    string Position, int Width, int Height, bool KeepVisible = false);
internal sealed record PrepareMainWindowCommand(long WindowHandle, uint SourceProcessId, int Width, int Height);
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
internal sealed record NativeContextMenuItem(uint Id, string Label, bool Enabled, bool Checked, bool Separator,
    IReadOnlyList<NativeContextMenuItem> Children);
internal sealed record ShowContextMenuCommand(long SourceWindowHandle, uint SourceProcessId, int X, int Y,
    IReadOnlyList<NativeContextMenuItem> Items, long PopupWindowHandle = 0);
internal sealed record CancelContextMenuCommand(Guid SessionId);
internal sealed record OpenShellTargetCommand(string Target, string Action);
internal sealed record OpenDefaultAppsSettingsCommand;
internal enum MarkdownAssociationAction { Query, Register, Unregister, OpenDefaults }
internal sealed record MarkdownAssociationCommand(MarkdownAssociationAction Action);
internal sealed record ShutdownSystemCommand(ushort DelaySeconds);
internal sealed record SuspendSystemCommand(bool Hibernate);
internal sealed record ExportBrowserExtensionCommand(string Format, string Destination);
internal sealed record PickBrowserExtensionExportPathCommand(string Format, long SourceWindowHandle, uint SourceProcessId);
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
internal sealed record PickClipboardArchivePathCommand(string Mode, string SuggestedFileName, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickClipboardStorageDirectoryCommand(string CurrentDirectory, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickDirectoryCommand(string CurrentDirectory, string Title, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickProgramCommand(string CurrentPath, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickMarkdownDocumentCommand(ulong SourceWindowHandle, uint SourceProcessId);
internal enum MarkdownExportFormat { Html, Pdf }
internal sealed record PickMarkdownExportCommand(ulong SourceWindowHandle, uint SourceProcessId, MarkdownExportFormat Format, string SuggestedName);
internal sealed record PrintMarkdownPdfCommand(string HtmlPath, string OutputPath);
internal sealed record PickExtensionManifestCommand(string CurrentPath, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickExtensionRuntimeCommand(string Program, string CurrentPath, long SourceWindowHandle, uint SourceProcessId);
internal sealed record PickExtensionSettingPathCommand(long SourceWindowHandle, uint SourceProcessId, string Kind, string CurrentPath);
internal sealed record InspectExtensionRuntimeCommand(string Program, string Executable);
internal sealed record EnsureIndexerServiceCommand;
internal sealed record EnsureBrowserNativeMessagingCommand(
    bool Enabled,
    string ChromiumExtensionId,
    string FirefoxExtensionId);

internal static class PlatformCommandParser
{
    public static PrepareMainWindowCommand ParsePrepareMainWindow(ProtocolEnvelope envelope)
    {
        try {
            if (GetCommandName(envelope) != "prepare_main_window") throw new InvalidDataException();
            var data = envelope.Payload.GetProperty("data");
            if (data.EnumerateObject().Any(property => property.Name is not ("window_handle" or "source_process_id" or "width" or "height"))) throw new InvalidDataException();
            var command = new PrepareMainWindowCommand(data.GetProperty("window_handle").GetInt64(), data.GetProperty("source_process_id").GetUInt32(), data.GetProperty("width").GetInt32(), data.GetProperty("height").GetInt32());
            MainWindowPreparation.Validate(command);
            return command;
        } catch (Exception failure) when (failure is JsonException or InvalidOperationException or KeyNotFoundException or FormatException or OverflowException or InvalidDataException) {
            throw new InvalidDataException("Invalid main-window preparation payload.");
        }
    }

    public static PrepareNotificationWindowCommand ParsePrepareNotificationWindow(ProtocolEnvelope envelope)
    {
        try
        {
            if (GetCommandName(envelope) != "prepare_notification_window") throw new InvalidDataException();
            var data = envelope.Payload.GetProperty("data");
            var command = new PrepareNotificationWindowCommand(
                data.GetProperty("window_handle").GetInt64(), data.GetProperty("source_window_handle").GetInt64(),
                data.GetProperty("source_process_id").GetUInt32(), data.GetProperty("position").GetString()!,
                data.GetProperty("width").GetInt32(), data.GetProperty("height").GetInt32(),
                data.TryGetProperty("keep_visible", out var keepVisible) && keepVisible.GetBoolean());
            Windows.NotificationWindowPlacement.Validate(command);
            return command;
        }
        catch (Exception error) when (error is JsonException or InvalidOperationException or KeyNotFoundException or
            FormatException or OverflowException or InvalidDataException)
        { throw new InvalidDataException("Invalid notification window payload."); }
    }

    public static ShowContextMenuCommand ParseShowContextMenu(ProtocolEnvelope envelope)
    {
        // Reject missing/wrong-typed fields before deserialization can supply defaults.
        try
        {
            if (GetCommandName(envelope) != "show_context_menu") throw new InvalidDataException();
            var data = envelope.Payload.GetProperty("data");
            _ = data.GetProperty("source_window_handle").GetInt64();
            _ = data.GetProperty("source_process_id").GetUInt32();
            _ = data.GetProperty("x").GetInt32();
            _ = data.GetProperty("y").GetInt32();
            var count = 0;
            ValidateMenuWire(data.GetProperty("items"), 1, ref count);
            var command = data.Deserialize<ShowContextMenuCommand>(JsonDefaults.Options)
                ?? throw new InvalidDataException();
            Windows.NativeContextMenuCoordinator.Validate(command);
            return command;
        }
        catch (Exception error) when (error is JsonException or InvalidOperationException or KeyNotFoundException or
            FormatException or OverflowException or InvalidDataException)
        { throw new InvalidDataException("Invalid native context menu payload."); }
    }

    private static void ValidateMenuWire(JsonElement items, int depth, ref int count)
    {
        if (depth > 4 || items.ValueKind != JsonValueKind.Array || (depth == 1 && items.GetArrayLength() > 128))
            throw new InvalidDataException();
        foreach (var item in items.EnumerateArray())
        {
            if (++count > 256) throw new InvalidDataException();
            _ = item.GetProperty("id").GetUInt32();
            if (item.GetProperty("label").ValueKind != JsonValueKind.String) throw new InvalidDataException();
            _ = item.GetProperty("enabled").GetBoolean();
            _ = item.GetProperty("checked").GetBoolean();
            _ = item.GetProperty("separator").GetBoolean();
            var children = item.GetProperty("children");
            if (children.ValueKind != JsonValueKind.Array) throw new InvalidDataException();
            if (children.GetArrayLength() > 0) ValidateMenuWire(children, depth + 1, ref count);
        }
    }

    public static CancelContextMenuCommand ParseCancelContextMenu(ProtocolEnvelope envelope)
    {
        try
        {
            if (GetCommandName(envelope) != "cancel_context_menu") throw new InvalidDataException();
            var id = envelope.Payload.GetProperty("data").GetProperty("session_id").GetGuid();
            if (id == Guid.Empty) throw new InvalidDataException();
            return new(id);
        }
        catch (Exception error) when (error is JsonException or InvalidOperationException or KeyNotFoundException or
            FormatException or InvalidDataException)
        { throw new InvalidDataException("Invalid native context menu cancellation."); }
    }

    public static ArchiveClipboardCommand ParseReadClipboardFileDrop(ProtocolEnvelope envelope) =>
        ArchiveClipboardCommand.Parse(envelope);
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

    public static SetClipboardCapturePausedCommand ParseSetClipboardCapturePaused(ProtocolEnvelope envelope)
    {
        var parsed = Parse<SetClipboardCapturePausedCommand>(envelope,
            "set_clipboard_capture_paused", "Capture pause payload was empty.");
        var data = envelope.Payload.GetProperty("data");
        if (data.ValueKind != JsonValueKind.Object || data.EnumerateObject().Any(property => property.Name != "paused"))
            throw new InvalidDataException("Capture pause contains unsupported fields.");
        if (!data.TryGetProperty("paused", out var paused) ||
            paused.ValueKind is not (JsonValueKind.True or JsonValueKind.False))
            throw new InvalidDataException("Capture pause requires an explicit boolean paused field.");
        return parsed;
    }

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

    public static PresentFixedWindowOverAnchorCommand ParsePresentFixedWindowOverAnchor(ProtocolEnvelope envelope) =>
        Parse<PresentFixedWindowOverAnchorCommand>(envelope, "present_fixed_window_over_anchor", "Fixed window presentation payload was empty.");

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

    public static OpenDefaultAppsSettingsCommand ParseOpenDefaultAppsSettings(ProtocolEnvelope envelope)
    {
        if (envelope.Type != "platform_command" || envelope.Payload.ValueKind != JsonValueKind.Object ||
            !envelope.Payload.TryGetProperty("command", out var command) || command.ValueKind != JsonValueKind.String ||
            command.GetString() != "open_default_apps_settings" ||
            !envelope.Payload.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Object ||
            data.EnumerateObject().Any() ||
            envelope.Payload.EnumerateObject().Count() != 2)
            throw new InvalidDataException("Default apps settings requires its fixed command and an empty data object.");
        return new();
    }

    public static MarkdownAssociationCommand ParseMarkdownAssociation(ProtocolEnvelope envelope)
    {
        if (envelope.Type != "platform_command" || envelope.Payload.ValueKind != JsonValueKind.Object ||
            !envelope.Payload.TryGetProperty("command", out var command) || command.ValueKind != JsonValueKind.String ||
            command.GetString() != "markdown_association" || envelope.Payload.EnumerateObject().Count() != 2 ||
            !envelope.Payload.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Object ||
            data.EnumerateObject().Count() != 1 || !data.TryGetProperty("action", out var action) || action.ValueKind != JsonValueKind.String)
            throw new InvalidDataException("Markdown association requires its fixed command and one action.");
        return new(action.GetString() switch
        {
            "query" => MarkdownAssociationAction.Query,
            "register" => MarkdownAssociationAction.Register,
            "unregister" => MarkdownAssociationAction.Unregister,
            "open_defaults" => MarkdownAssociationAction.OpenDefaults,
            _ => throw new InvalidDataException("Invalid Markdown association action."),
        });
    }

    public static ShutdownSystemCommand ParseShutdownSystem(ProtocolEnvelope envelope) =>
        Parse<ShutdownSystemCommand>(envelope, "shutdown_system", "Shutdown command payload was empty.");

    public static SuspendSystemCommand ParseSuspendSystem(ProtocolEnvelope envelope) =>
        Parse<SuspendSystemCommand>(envelope, "suspend_system", "Suspend command payload was empty.");

    public static ExportBrowserExtensionCommand ParseExportBrowserExtension(ProtocolEnvelope envelope)
    {
        var command = Parse<ExportBrowserExtensionCommand>(envelope, "export_browser_extension",
            "Browser extension export command payload was empty.");
        if (string.IsNullOrWhiteSpace(command.Destination))
            throw new InvalidDataException("Browser extension export requires a destination.");
        return command;
    }

    public static PickBrowserExtensionExportPathCommand ParsePickBrowserExtensionExportPath(ProtocolEnvelope envelope)
    {
        var command = ParsePicker<PickBrowserExtensionExportPathCommand>(envelope, "pick_browser_extension_export_path");
        if (string.IsNullOrWhiteSpace(command.Format))
            throw new InvalidDataException("Browser export picker format is required.");
        return command;
    }

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

    public static PickArchivePathCommand ParsePickArchivePath(ProtocolEnvelope envelope) =>
        Parse<PickArchivePathCommand>(envelope, "pick_archive_path", "Archive picker payload was empty.");

    private static T ParsePicker<T>(ProtocolEnvelope envelope, string name)
    {
        if (GetCommandName(envelope) != name ||
            !envelope.Payload.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Object ||
            !data.TryGetProperty("source_window_handle", out var handle) || !handle.TryGetInt64(out var hwnd) || hwnd <= 0 ||
            !data.TryGetProperty("source_process_id", out var process) || !process.TryGetUInt32(out var pid) || pid == 0)
            throw new InvalidDataException("Invalid picker source identity.");
        return Parse<T>(envelope, name, "Invalid picker payload.");
    }

    public static PickClipboardArchivePathCommand ParsePickClipboardArchivePath(
        ProtocolEnvelope envelope) =>
        ParsePicker<PickClipboardArchivePathCommand>(envelope, "pick_clipboard_archive_path");

    public static PickClipboardStorageDirectoryCommand ParsePickClipboardStorageDirectory(
        ProtocolEnvelope envelope) =>
        ParsePicker<PickClipboardStorageDirectoryCommand>(envelope, "pick_clipboard_storage_directory");

    public static PickDirectoryCommand ParsePickDirectory(ProtocolEnvelope envelope) =>
        ParsePicker<PickDirectoryCommand>(envelope, "pick_directory");

    public static PickProgramCommand ParsePickProgram(ProtocolEnvelope envelope) =>
        ParsePicker<PickProgramCommand>(envelope, "pick_program");

    public static PickMarkdownDocumentCommand ParsePickMarkdownDocument(ProtocolEnvelope envelope) =>
        ParsePicker<PickMarkdownDocumentCommand>(envelope, "pick_markdown_document");

    public static PickMarkdownExportCommand ParsePickMarkdownExport(ProtocolEnvelope envelope)
    {
        var owner = ParsePicker<PickMarkdownDocumentCommand>(envelope, "pick_markdown_export");
        var data = envelope.Payload.GetProperty("data");
        var format = data.GetProperty("format").GetString() switch
        {
            "html" => MarkdownExportFormat.Html,
            "pdf" => MarkdownExportFormat.Pdf,
            _ => throw new InvalidDataException("Invalid Markdown export format.")
        };
        var suggested = data.GetProperty("suggested_name").GetString();
        if (suggested is null || suggested.Length > 255 || suggested.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
            throw new InvalidDataException("Invalid Markdown export name.");
        return new(owner.SourceWindowHandle, owner.SourceProcessId, format, suggested);
    }

    public static PrintMarkdownPdfCommand ParsePrintMarkdownPdf(ProtocolEnvelope envelope) =>
        Parse<PrintMarkdownPdfCommand>(envelope, "print_markdown_pdf", "Invalid Markdown PDF payload.");

    public static PickExtensionManifestCommand ParsePickExtensionManifest(ProtocolEnvelope envelope)
    {
        if (GetCommandName(envelope) != "pick_extension_manifest" ||
            !envelope.Payload.TryGetProperty("data", out var data) ||
            data.ValueKind != JsonValueKind.Object ||
            !data.TryGetProperty("current_path", out var currentPath) ||
            currentPath.ValueKind != JsonValueKind.String)
        {
            throw new InvalidDataException("Invalid extension manifest picker payload.");
        }
        return ParsePicker<PickExtensionManifestCommand>(envelope, "pick_extension_manifest");
    }

    internal static PickExtensionSettingPathCommand ParsePickExtensionSettingPath(ProtocolEnvelope envelope)
    {
        if (GetCommandName(envelope) != "pick_extension_setting_path" || !envelope.Payload.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Object ||
            data.EnumerateObject().Count() != 4 || data.EnumerateObject().Any(p => p.Name is not ("source_window_handle" or "source_process_id" or "kind" or "current_path")) ||
            !data.TryGetProperty("source_window_handle", out var window) || window.ValueKind != JsonValueKind.Number || !window.TryGetInt64(out var hwnd) || hwnd <= 0 ||
            !data.TryGetProperty("source_process_id", out var process) || process.ValueKind != JsonValueKind.Number || !process.TryGetUInt32(out var pid) || pid == 0 ||
            !data.TryGetProperty("kind", out var kind) || kind.ValueKind != JsonValueKind.String || kind.GetString() is not ("file" or "folder") ||
            !data.TryGetProperty("current_path", out var path) || path.ValueKind != JsonValueKind.String || path.GetString()!.Length >= 32767 || path.GetString()!.Contains('\0'))
            throw new InvalidDataException("Invalid extension setting path payload.");
        return new(hwnd, pid, kind.GetString()!, path.GetString()!);
    }

    internal static bool IsExtensionRuntimeProgram(string? program) =>
        program is "python" or "node" or "powershell" or "pwsh";

    public static PickExtensionRuntimeCommand ParsePickExtensionRuntime(ProtocolEnvelope envelope)
    {
        var (program, path) = ParseExtensionRuntime(envelope, "pick_extension_runtime", "current_path");
        var owner = ParsePicker<PickExtensionRuntimeCommand>(envelope, "pick_extension_runtime");
        return new(program, path, owner.SourceWindowHandle, owner.SourceProcessId);
    }

    public static InspectExtensionRuntimeCommand ParseInspectExtensionRuntime(ProtocolEnvelope envelope)
    {
        var (program, path) = ParseExtensionRuntime(envelope, "inspect_extension_runtime", "executable");
        return new(program, path);
    }

    private static (string Program, string Path) ParseExtensionRuntime(
        ProtocolEnvelope envelope, string command, string pathField)
    {
        if (GetCommandName(envelope) != command ||
            !envelope.Payload.TryGetProperty("data", out var data) || data.ValueKind != JsonValueKind.Object ||
            !data.TryGetProperty("program", out var program) || program.ValueKind != JsonValueKind.String ||
            !IsExtensionRuntimeProgram(program.GetString()) ||
            !data.TryGetProperty(pathField, out var path) || path.ValueKind != JsonValueKind.String ||
            path.GetString()!.Length >= 32767 || path.GetString()!.Contains('\0'))
            throw new InvalidDataException("Invalid extension runtime payload.");
        return (program.GetString()!, path.GetString()!);
    }

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

    public static Windows.Archive.ArchiveShellIntegrationCommand ParseArchiveShellIntegration(
        ProtocolEnvelope envelope)
    {
        var parsed = Parse<Windows.Archive.ArchiveShellIntegrationCommand>(
            envelope, "archive_shell_integration", "Archive shell integration payload was empty.");
        if (parsed.Version != 1 || parsed.Action is not ("status" or "apply" or "revert") || parsed.DesiredRevision < 0)
            throw new InvalidDataException("Invalid archive shell integration command.");
        try { Windows.Archive.ArchiveIntegrationPlan.NormalizeIntent(parsed.ToIntent()); }
        catch (ArgumentException) { throw new InvalidDataException("Invalid archive association configuration."); }
        return parsed;
    }

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
