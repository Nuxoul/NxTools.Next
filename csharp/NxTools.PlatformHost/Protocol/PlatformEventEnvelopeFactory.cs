using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

internal static class PlatformEventEnvelopeFactory
{
    public static ProtocolEnvelope Create(WindowsPlatformEvent platformEvent) =>
        platformEvent switch
        {
            ForegroundChangedEvent foreground => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "foreground_changed",
                Data = foreground.Context
            }),
            ClipboardTextCapturedEvent clipboard => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "clipboard_text_captured",
                Data = new
                {
                    clipboard.Text,
                    clipboard.Source,
                    clipboard.CapturedAtUnixMs
                }
            }),
            ClipboardPanelRequestedEvent panel => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "clipboard_panel_requested",
                Data = new
                {
                    panel.Target,
                    panel.CursorX,
                    panel.CursorY,
                    panel.CursorDisplayIndex,
                    panel.CursorMonitorDpi,
                    panel.RequestedAtUnixMs
                }
            }),
            _ => throw new ArgumentOutOfRangeException(nameof(platformEvent))
        };
}
