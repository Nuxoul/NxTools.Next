using System.Text.Json;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

internal sealed record FloatingAssistantCommand(ushort Version, Guid SessionId, ulong Generation,
    long WindowHandle, long SourceWindowHandle, uint SourceProcessId, string Operation,
    int X, int Y, uint Size, byte Opacity, bool ClickThrough, bool PositionLocked, Guid? DropId,
    bool EdgeHidden = false);

internal static class FloatingAssistantContract
{
    private static readonly HashSet<string> Operations = new(StringComparer.Ordinal)
    { "prepare", "update", "show", "hide", "retire", "release_drop", "pick_image" };

    internal static FloatingAssistantCommand Parse(ProtocolEnvelope envelope)
    {
        try
        {
            if (envelope.Type != "platform_command" || envelope.ProtocolVersion != 1 ||
                envelope.Payload.GetProperty("command").GetString() != "floating_assistant")
                throw new InvalidDataException();
            var data = envelope.Payload.GetProperty("data");
            if (data.ValueKind != JsonValueKind.Object) throw new InvalidDataException();
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in data.EnumerateObject())
            {
                if (!names.Add(property.Name) || property.Name is not
                    ("version" or "session_id" or "generation" or "window_handle" or "source_window_handle" or
                     "source_process_id" or "operation" or "x" or "y" or "size" or "opacity" or
                     "click_through" or "position_locked" or "drop_id" or "edge_hidden")) throw new InvalidDataException();
            }
            var command = new FloatingAssistantCommand(
                data.GetProperty("version").GetUInt16(), data.GetProperty("session_id").GetGuid(),
                data.GetProperty("generation").GetUInt64(), data.GetProperty("window_handle").GetInt64(),
                data.GetProperty("source_window_handle").GetInt64(), data.GetProperty("source_process_id").GetUInt32(),
                data.GetProperty("operation").GetString() ?? "", data.GetProperty("x").GetInt32(),
                data.GetProperty("y").GetInt32(), data.GetProperty("size").GetUInt32(),
                data.GetProperty("opacity").GetByte(), data.GetProperty("click_through").GetBoolean(),
                data.GetProperty("position_locked").GetBoolean(),
                data.TryGetProperty("drop_id", out var drop) && drop.ValueKind != JsonValueKind.Null ? drop.GetGuid() : null,
                data.TryGetProperty("edge_hidden", out var edge) && edge.GetBoolean());
            if (command.Version != 1 || command.SessionId == Guid.Empty || command.Generation == 0 ||
                (command.Operation != "pick_image" && (command.WindowHandle == 0 || command.WindowHandle == command.SourceWindowHandle)) ||
                command.SourceWindowHandle == 0 || command.SourceProcessId == 0 ||
                !Operations.Contains(command.Operation) || command.Size is < 20 or > 300 ||
                command.Opacity is < 10 or > 100 ||
                (command.DropId.HasValue != (command.Operation == "release_drop")) || command.DropId == Guid.Empty)
                throw new InvalidDataException();
            return command;
        }
        catch (Exception error) when (error is JsonException or KeyNotFoundException or FormatException or OverflowException or InvalidOperationException or InvalidDataException)
        { throw new InvalidDataException("Invalid floating assistant command."); }
    }

    internal static ProtocolEnvelope Event(FloatingAssistantCommand command, Guid correlationId, ulong sequence,
        string kind, int x = 0, int y = 0, int delta = 0, bool fullscreen = false,
        FloatingAssistantDropData? drop = null, string? imagePath = null, Guid? dropId = null,
        CursorMonitorSnapshot? cursorMonitor = null) =>
        ProtocolEnvelope.Create("platform_event", new { @event = "floating_assistant", data = new
        {
            version = 1, session_id = command.SessionId, generation = command.Generation,
            window_handle = command.WindowHandle, correlation_id = correlationId, sequence, kind, x, y, delta,
            cursor_monitor = cursorMonitor, fullscreen, text = drop?.Text, url = drop?.Url, html = drop?.Html,
            image_path = imagePath, image_format = drop?.ImageFormat,
            files = drop?.Files ?? Array.Empty<string>(), drop_id = dropId
        } }, correlationId);
}