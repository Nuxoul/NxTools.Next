using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record PasteTextCommand(
    string Text,
    long TargetWindowHandle,
    long TargetFocusWindowHandle);

internal static class PlatformCommandParser
{
    public static PasteTextCommand ParsePasteText(ProtocolEnvelope envelope)
    {
        if (envelope.Type != "platform_command")
        {
            throw new InvalidDataException("Envelope is not a platform command.");
        }

        var command = envelope.Payload.GetProperty("command").GetString();
        if (command != "paste_text")
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }

        var data = envelope.Payload.GetProperty("data");
        var parsed = data.Deserialize<PasteTextCommand>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException("Paste command payload was empty.");
    }
}
