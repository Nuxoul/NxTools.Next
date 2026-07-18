using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record PasteTextCommand(
    string Text,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool PasteAfterWrite = true);
internal sealed record PasteFilesCommand(
    IReadOnlyList<string> Paths,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool PasteAfterWrite = true);
internal sealed record PasteImageCommand(
    string ImagePath,
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    bool PasteAfterWrite = true);
internal sealed record ClipboardNativeFormatData(
    uint FormatId,
    string FormatName,
    string DataPath,
    ulong SizeBytes,
    uint SortOrder);
internal sealed record PasteNativeFormatsCommand(
    IReadOnlyList<ClipboardNativeFormatData> Formats,
    bool PasteAfterWrite,
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

    public static PasteFilesCommand ParsePasteFiles(ProtocolEnvelope envelope) =>
        Parse<PasteFilesCommand>(envelope, "paste_files", "Files paste command payload was empty.");

    public static PasteImageCommand ParsePasteImage(ProtocolEnvelope envelope) =>
        Parse<PasteImageCommand>(envelope, "paste_image", "Image paste command payload was empty.");

    public static PasteNativeFormatsCommand ParsePasteNativeFormats(ProtocolEnvelope envelope) =>
        Parse<PasteNativeFormatsCommand>(
            envelope,
            "paste_native_formats",
            "Native formats paste command payload was empty.");

    private static T Parse<T>(ProtocolEnvelope envelope, string expected, string emptyMessage)
    {
        var command = GetCommandName(envelope);
        if (command != expected)
        {
            throw new InvalidDataException($"Unsupported platform command '{command}'.");
        }
        var parsed = envelope.Payload.GetProperty("data").Deserialize<T>(JsonDefaults.Options);
        return parsed ?? throw new InvalidDataException(emptyMessage);
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
