using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal enum QuickLaunchTriggerKind
{
    DoubleControl,
    HotKey
}

internal sealed record QuickLaunchTriggerBinding(
    QuickLaunchTriggerKind Kind,
    string Display,
    ClipboardHotKeyBinding? HotKey)
{
    public static bool TryCreate(
        ConfigureQuickLaunchCommand command,
        out QuickLaunchTriggerBinding binding,
        out string? error)
    {
        binding = null!;
        error = null;
        var key = command.HotkeyKey.Trim();
        if (command.HotkeyModifiers.Count == 0 &&
            (key.Equals("control", StringComparison.OrdinalIgnoreCase) ||
             key.Equals("ctrl", StringComparison.OrdinalIgnoreCase)))
        {
            if (string.IsNullOrWhiteSpace(command.HotkeyDisplay))
            {
                error = "The quick-launch double-control gesture requires a display label.";
                return false;
            }
            binding = new QuickLaunchTriggerBinding(
                QuickLaunchTriggerKind.DoubleControl,
                command.HotkeyDisplay.Trim(),
                null);
            return true;
        }

        if (!ClipboardHotKeyBinding.TryCreate(command, out var hotKey, out error))
        {
            return false;
        }
        binding = new QuickLaunchTriggerBinding(QuickLaunchTriggerKind.HotKey, hotKey.Display, hotKey);
        return true;
    }
}
