using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record ApplyWindowMaterialCommand(
    Guid SessionId, ulong FactsRevision, long WindowHandle, ulong Generation,
    string Style, string Surface, bool Dark);

internal static class WindowMaterialContract
{
    private static readonly string[] Fields =
        ["session_id", "facts_revision", "window_handle", "generation", "style", "surface", "dark"];

    internal static ApplyWindowMaterialCommand Parse(ProtocolEnvelope envelope)
    {
        try
        {
            envelope.Validate();
            if (envelope.Id == Guid.Empty || envelope.Type != "platform_command") throw new InvalidDataException();
            var payload = envelope.Payload;
            RequireFields(payload, ["command", "data"]);
            if (payload.GetProperty("command").GetString() != "apply_window_material") throw new InvalidDataException();
            var data = payload.GetProperty("data");
            RequireFields(data, Fields);
            var command = new ApplyWindowMaterialCommand(
                data.GetProperty("session_id").GetGuid(), data.GetProperty("facts_revision").GetUInt64(),
                data.GetProperty("window_handle").GetInt64(), data.GetProperty("generation").GetUInt64(),
                data.GetProperty("style").GetString()!, data.GetProperty("surface").GetString()!,
                data.GetProperty("dark").GetBoolean());
            if (!IsValid(command)) throw new InvalidDataException();
            return command;
        }
        catch (Exception error) when (error is InvalidOperationException or KeyNotFoundException
            or FormatException or OverflowException or InvalidDataException)
        {
            throw new InvalidDataException("invalid_window_material_request");
        }
    }

    internal static bool IsValid(ApplyWindowMaterialCommand command) =>
        command.SessionId != Guid.Empty && command.FactsRevision != 0 && command.WindowHandle != 0
        && command.Generation != 0 && IsStyle(command.Style) && IsSurface(command.Surface);

    internal static bool IsStyle(string? style) => style is "solid" or "acrylic";
    internal static bool IsSurface(string? surface) => surface is "backdrop" or "transient" or "transparent";
    internal static bool IsFrosted(string style) => style == "acrylic";

    private static void RequireFields(JsonElement value, string[] expected)
    {
        if (value.ValueKind != JsonValueKind.Object) throw new InvalidDataException();
        var remaining = new HashSet<string>(expected, StringComparer.Ordinal);
        foreach (var field in value.EnumerateObject())
            if (!remaining.Remove(field.Name)) throw new InvalidDataException();
        if (remaining.Count != 0) throw new InvalidDataException();
    }

    internal static string Receipt(ApplyWindowMaterialCommand command, string effective, string fallbackReason, int nativeErrorCode) =>
        JsonSerializer.Serialize(new
        {
            command.SessionId, command.FactsRevision, command.WindowHandle, command.Generation,
            command.Style, command.Surface, command.Dark, Effective = effective,
            FallbackReason = fallbackReason, NativeErrorCode = nativeErrorCode
        }, JsonDefaults.Options);
}
