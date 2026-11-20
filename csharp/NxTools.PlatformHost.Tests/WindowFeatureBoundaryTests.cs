using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class WindowFeatureBoundaryTests
{
    public static void CursorWireAndHotkeyEnablement()
    {
        var monitor = new CursorMonitorSnapshot(0xCAFE, -1920, -1080, 1920, 1040, 144);
        var target = new ForegroundContext(1, 2, 3, "fixture", "");
        foreach (WindowsPlatformEvent source in new WindowsPlatformEvent[]
        {
            new ClipboardPanelRequestedEvent(target, -100, -100, 1, 144, 1, CursorMonitor: monitor),
            new QuickLaunchPanelRequestedEvent(target, -100, -100, 1, 144, 1, CursorMonitor: monitor)
        })
        {
            var data = PlatformEventEnvelopeFactory.Create(source).Payload.GetProperty("data").GetProperty("cursor_monitor");
            Assert(data.GetProperty("display_id").GetUInt64() == 0xCAFE, "native monitor identity retained");
            Assert(data.GetProperty("work_left").GetInt32() == -1920 && data.GetProperty("work_top").GetInt32() == -1080,
                "negative work-area origin retained");
            Assert(data.GetProperty("dpi").GetUInt32() == 144, "same monitor DPI retained");
        }
        var assistant = new FloatingAssistantCommand(1, Guid.NewGuid(), 1, 1, 2, 3,
            "prepare", 0, 0, 72, 48, false, false, null);
        var assistantMonitor = FloatingAssistantContract.Event(assistant, Guid.NewGuid(), 1, "drop",
            cursorMonitor: monitor).Payload.GetProperty("data").GetProperty("cursor_monitor");
        Assert(assistantMonitor.GetProperty("display_id").GetUInt64() == 0xCAFE &&
            assistantMonitor.GetProperty("dpi").GetUInt32() == 144,
            "assistant search carries the same native monitor snapshot");
        var disabled = ProtocolEnvelope.Create("platform_command", new
        {
            command = "configure_clipboard",
            data = new { hotkey_modifiers = new[] { "alt" }, hotkey_key = "C", hotkey_display = "Alt+C", excluded_processes = Array.Empty<string>(), enabled = false }
        });
        Assert(!PlatformCommandParser.ParseConfigureClipboard(disabled).Enabled, "enabled intent reaches native parser");
        var registrar = new Registrar();
        var binding = new ClipboardHotKeyBinding(1, 0x43, "Alt+C");
        var registration = new ClipboardHotKeyRegistration(registrar, binding);
        Assert(registration.Initialize().Success && registration.IsAvailable, "initial registration");
        Assert(registration.Apply(binding, false).Success && !registration.IsAvailable, "feature disables actual hotkey");
        var registrations = registrar.Registrations;
        Assert(registration.CancelRecording().Success && registrar.Registrations == registrations,
            "recording cancellation cannot reenable disabled hotkey");
        Assert(registration.Apply(binding, true).Success && registration.IsAvailable, "reenable restores original binding");
        registrar.FailUnregister = true;
        var failure = registration.Apply(binding, false);
        Assert(!failure.Success && failure.NativeErrorCode == 5, "unregister failure is not reported as disabled success");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class Registrar : IClipboardHotKeyRegistrar
    {
        public int LastError => 5;
        public int Registrations;
        public bool FailUnregister;
        public bool Register(uint modifiers, uint virtualKey) { Registrations++; return true; }
        public bool Unregister() => !FailUnregister;
    }
}
