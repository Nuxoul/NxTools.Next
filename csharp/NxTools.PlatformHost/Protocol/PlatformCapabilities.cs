namespace NxTools.PlatformHost.Protocol;

internal static class PlatformCapabilities
{
    public const string ForegroundContext = "foreground_context";
    public const string ClipboardTextCapture = "clipboard_text_capture";
    public const string ClipboardImageCapture = "clipboard_image_capture";
    public const string ClipboardFilesCapture = "clipboard_files_capture";
    public const string ClipboardNativeFormats = "clipboard_native_formats";
    public const string PasteText = "paste_text";
    public const string PasteImage = "paste_image";
    public const string PasteFiles = "paste_files";
    public const string ClipboardPanelHotKey = "clipboard_panel_hotkey";
    public const string QuickLaunchPanelHotKey = "quick_launch_panel_hotkey";
    public const string QuickLaunchConfiguration = "quick_launch_configuration";
    public const string ClipboardConfiguration = "clipboard_configuration";
    public const string ClipboardArchivePicker = "clipboard_archive_picker";
    public const string ClipboardStoragePicker = "clipboard_storage_picker";
    public const string WindowTopmost = "window_topmost";
    public const string WindowNoActivate = "window_no_activate";
    public const string WindowBounds = "window_bounds";
    public const string TaskbarProgress = "taskbar_progress";
    public const string TaskbarProgressOverlay = "taskbar_progress_overlay";
    public const string NativeFileDrag = "native_file_drag";
    public const string WindowVisibility = "window_visibility";
    public const string WindowMinimized = "window_minimized";
    public const string WindowPosition = "window_position";
    public const string ShellIconExtraction = "shell_icon_extraction";
    public const string ShellContextActions = "shell_context_actions";
    public const string DirectoryNavigation = "directory_navigation";
    public const string FileDialogSearch = "file_dialog_search";
    public const string WindowActivation = "window_activation";
    public const string IndexerServiceManagement = "indexer_service_management";
    public const string BrowserNativeMessaging = "browser_native_messaging";
    public const string DesktopShell = "desktop_shell";
    public const string TrayEvents = "tray_events";
    // Adapter availability only. Each worker must independently negotiate its read contract.
    public const string ArchiveRead = "archive_read_v1";
    public const string ArchiveEntry = "archive_entry_v1";

    public static string[] Create(bool clipboardHotKeyAvailable, bool quickLaunchHotKeyAvailable = false)
    {
        var capabilities = new List<string>
        {
            ArchiveRead,
            ArchiveEntry,
            ForegroundContext,
            ClipboardTextCapture,
            ClipboardImageCapture,
            ClipboardFilesCapture,
            ClipboardNativeFormats,
            PasteText,
            PasteImage,
            PasteFiles,
            ClipboardConfiguration,
            QuickLaunchConfiguration,
            ClipboardArchivePicker,
            ClipboardStoragePicker,
            WindowTopmost,
            WindowNoActivate,
            WindowBounds,
            TaskbarProgress,
            TaskbarProgressOverlay,
            NativeFileDrag,
            WindowVisibility,
            WindowMinimized,
            WindowPosition,
            ShellIconExtraction,
            ShellContextActions,
            DirectoryNavigation,
            FileDialogSearch,
            WindowActivation,
            IndexerServiceManagement,
            BrowserNativeMessaging,
            DesktopShell,
            TrayEvents
        };
        if (clipboardHotKeyAvailable)
        {
            capabilities.Add(ClipboardPanelHotKey);
        }
        if (quickLaunchHotKeyAvailable)
        {
            capabilities.Add(QuickLaunchPanelHotKey);
        }
        return capabilities.ToArray();
    }
}
