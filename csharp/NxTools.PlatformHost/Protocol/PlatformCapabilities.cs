namespace NxTools.PlatformHost.Protocol;

internal static class PlatformCapabilities
{
    public const string ForegroundContext = "foreground_context";
    public const string ClipboardTextCapture = "clipboard_text_capture";
    public const string PasteText = "paste_text";
    public const string ClipboardPanelHotKey = "clipboard_panel_hotkey";

    public static string[] Create(bool clipboardHotKeyAvailable)
    {
        var capabilities = new List<string>
        {
            ForegroundContext,
            ClipboardTextCapture,
            PasteText
        };
        if (clipboardHotKeyAvailable)
        {
            capabilities.Add(ClipboardPanelHotKey);
        }
        return capabilities.ToArray();
    }
}
