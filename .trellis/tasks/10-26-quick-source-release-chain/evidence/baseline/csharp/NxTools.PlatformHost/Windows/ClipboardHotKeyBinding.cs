using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record ClipboardHotKeyBinding(uint Modifiers, uint VirtualKey, string Display)
{
    private const uint ModAlt = 0x0001;
    private const uint ModControl = 0x0002;
    private const uint ModShift = 0x0004;
    private const uint ModWin = 0x0008;

    public static bool TryCreate(
        ConfigureClipboardCommand command,
        out ClipboardHotKeyBinding binding,
        out string? error)
        => TryCreate(
            command.HotkeyModifiers,
            command.HotkeyKey,
            command.HotkeyDisplay,
            "clipboard",
            out binding,
            out error);

    public static bool TryCreate(
        ConfigureQuickLaunchCommand command,
        out ClipboardHotKeyBinding binding,
        out string? error)
        => TryCreate(
            command.HotkeyModifiers,
            command.HotkeyKey,
            command.HotkeyDisplay,
            "quick-launch",
            out binding,
            out error);

    private static bool TryCreate(
        IReadOnlyList<string> hotkeyModifiers,
        string hotkeyKey,
        string hotkeyDisplay,
        string feature,
        out ClipboardHotKeyBinding binding,
        out string? error)
    {
        binding = null!;
        error = null;
        var modifiers = 0U;
        foreach (var modifier in hotkeyModifiers.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            if (!TryResolveModifier(modifier, out var resolvedModifier))
            {
                error = $"The {feature} hotkey modifier '{modifier}' is unsupported.";
                return false;
            }
            modifiers |= resolvedModifier;
        }
        if (modifiers == 0 || string.IsNullOrWhiteSpace(hotkeyDisplay))
        {
            error = $"The {feature} hotkey requires a modifier and display label.";
            return false;
        }
        if (!TryResolveVirtualKey(hotkeyKey, out var virtualKey))
        {
            error = $"The {feature} hotkey key '{hotkeyKey}' is unsupported.";
            return false;
        }

        binding = new ClipboardHotKeyBinding(modifiers, virtualKey, hotkeyDisplay.Trim());
        return true;
    }

    private static bool TryResolveModifier(string modifier, out uint value)
    {
        value = modifier.Trim().ToLowerInvariant() switch
        {
            "control" => ModControl,
            "alt" => ModAlt,
            "shift" => ModShift,
            "win" => ModWin,
            _ => 0U
        };
        return value != 0;
    }

    private static bool TryResolveVirtualKey(string key, out uint virtualKey)
    {
        var value = key.Trim();
        if (value.Length == 1 && value[0] is <= '\x7f' && char.IsLetterOrDigit(value[0]))
        {
            virtualKey = char.ToUpperInvariant(value[0]);
            return true;
        }
        if (value.Length is 2 or 3 && value[0] is 'f' or 'F' &&
            uint.TryParse(value.AsSpan(1), out var functionNumber) &&
            functionNumber is >= 1 and <= 24)
        {
            virtualKey = 0x70 + functionNumber - 1;
            return true;
        }

        virtualKey = value.ToLowerInvariant() switch
        {
            "space" => 0x20,
            "tab" => 0x09,
            "insert" => 0x2D,
            "delete" => 0x2E,
            "home" => 0x24,
            "end" => 0x23,
            "pageup" => 0x21,
            "pagedown" => 0x22,
            "up" => 0x26,
            "down" => 0x28,
            "left" => 0x25,
            "right" => 0x27,
            _ => 0
        };
        return virtualKey != 0;
    }
}
