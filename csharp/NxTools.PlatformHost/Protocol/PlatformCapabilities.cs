namespace NxTools.PlatformHost.Protocol;

internal static class PlatformCapabilities
{
    public const string ForegroundContext = "foreground_context";
    public const string ClipboardTextCapture = "clipboard_text_capture";
    public const string ClipboardImageCapture = "clipboard_image_capture";
    public const string ClipboardFilesCapture = "clipboard_files_capture";
    public const string PasteText = "paste_text";
    public const string PasteImage = "paste_image";
    public const string PasteFiles = "paste_files";
    public const string ClipboardPanelHotKey = "clipboard_panel_hotkey";
    public const string WindowTopmost = "window_topmost";
    public const string WindowVisibility = "window_visibility";

    public static string[] Create(bool clipboardHotKeyAvailable)
    {
        var capabilities = new List<string>
        {
            ForegroundContext,
            ClipboardTextCapture,
            ClipboardImageCapture,
            ClipboardFilesCapture,
            PasteText,
            PasteImage,
            PasteFiles,
            WindowTopmost,
            WindowVisibility
        };
        if (clipboardHotKeyAvailable)
        {
            capabilities.Add(ClipboardPanelHotKey);
        }
        return capabilities.ToArray();
    }
}
