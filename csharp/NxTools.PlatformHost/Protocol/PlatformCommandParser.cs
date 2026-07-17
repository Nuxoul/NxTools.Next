using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record PasteTextCommand(
    string Text,
    long TargetWindowHandle,
    long TargetFocusWindowHandle);

internal sealed record SetWindowTopmostCommand(long WindowHandle, bool Enabled);
internal sealed record SetWindowVisibilityCommand(long WindowHandle, bool Visible);

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
