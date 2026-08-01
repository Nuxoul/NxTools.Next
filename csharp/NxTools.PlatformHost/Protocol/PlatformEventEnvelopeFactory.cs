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
            GlobalPointerPressedEvent pointer => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "global_pointer_pressed",
                Data = new
                {
                    pointer.WindowHandle,
                    pointer.RootWindowHandle,
                    pointer.ProcessId
                }
            }),
            ClipboardCapturedEvent clipboard => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "clipboard_captured",
                Data = new
                {
                    clipboard.Text,
                    clipboard.Image,
                    clipboard.Paths,
                    NativeFormats = clipboard.NativeFormats ?? [],
                    clipboard.SourceIconStagingPath,
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
            QuickLaunchPanelRequestedEvent panel => ProtocolEnvelope.Create("platform_event", new
            {
                Event = "quick_launch_panel_requested",
                Data = new
                {
                    panel.Target,
                    panel.CursorX,
                    panel.CursorY,
                    panel.CursorDisplayIndex,
                    panel.CursorMonitorDpi,
                    panel.RequestedAtUnixMs,
                    panel.DirectoryNavigationContext
                }
            }),
            QuickLaunchDirectoryContextResolvedEvent resolved =>
                ProtocolEnvelope.Create("platform_event", new
                {
                    Event = "quick_launch_directory_context_resolved",
                    Data = new
                    {
                        resolved.TargetWindowHandle,
                        resolved.RequestedAtUnixMs,
                        resolved.DirectoryNavigationContext
                    }
                }),
            QuickLaunchAutoSwitchRequestedEvent autoSwitch =>
                ProtocolEnvelope.Create("platform_event", new
                {
                    Event = "quick_launch_auto_switch_requested",
                    Data = new
                    {
                        autoSwitch.Target,
                        autoSwitch.SourceDirectory,
                        autoSwitch.RequestedAtUnixMs
                    }
                }),
            QuickLaunchFileDialogSearchHostChangedEvent searchHost =>
                ProtocolEnvelope.Create("platform_event", new
                {
                    Event = "quick_launch_file_dialog_search_host_changed",
                    Data = new
                    {
                        searchHost.WindowHandle,
                        searchHost.CurrentDirectory,
                        searchHost.Left,
                        searchHost.Top,
                        searchHost.Width,
                        searchHost.Height,
                        searchHost.MonitorDpi,
                        searchHost.Visible,
                        searchHost.ChangedAtUnixMs,
                        searchHost.QuickSwitchSource
                    }
                }),
            TaskbarOverlayEditGestureFinishedEvent gesture =>
                ProtocolEnvelope.Create("platform_event", new
                {
                    Event = "taskbar_overlay_edit_gesture_finished",
                    Data = new
                    {
                        gesture.WindowHandle
                    }
                }),
            ElevatedDirectoryHelperRequestedEvent helper => ProtocolEnvelope.Create(
                "platform_event",
                new
                {
                    Event = "elevated_directory_helper_requested",
                    Data = new
                    {
                        helper.PipeName,
                        helper.AuthenticationToken
                    }
                }),
            _ => throw new ArgumentOutOfRangeException(nameof(platformEvent))
        };
}
