using System.Text.Json;

namespace NxTools.PlatformHost.Protocol;

internal sealed record ProcessWindowPickCommand(string Action, Guid SessionId,
    long SourceWindowHandle = 0, long OverlayWindowHandle = 0);

internal static class ProcessWindowPickContract
{
    internal static ProcessWindowPickCommand Parse(ProtocolEnvelope envelope)
    {
        try
        {
            envelope.Validate();
            if (envelope.Type != "platform_command" || PlatformCommandParser.GetCommandName(envelope) != "process_window_pick")
                throw new InvalidDataException();
            var data = envelope.Payload.GetProperty("data");
            var action = data.GetProperty("action").GetString();
            if (action is not ("start" or "show_overlay" or "cancel")) throw new InvalidDataException();
            var expected = action == "start"
                ? new[] { "action", "session_id", "source_window_handle", "overlay_window_handle" }
                : new[] { "action", "session_id" };
            var names = data.EnumerateObject().Select(p => p.Name).ToArray();
            if (names.Length != expected.Length || !names.Order().SequenceEqual(expected.Order())) throw new InvalidDataException();
            var session = data.GetProperty("session_id").GetGuid();
            if (session == Guid.Empty) throw new InvalidDataException();
            var source = action == "start" ? data.GetProperty("source_window_handle").GetInt64() : 0;
            var overlay = action == "start" ? data.GetProperty("overlay_window_handle").GetInt64() : 0;
            if (action == "start" && (source == 0 || overlay == 0 || source == overlay)) throw new InvalidDataException();
            return new(action, session, source, overlay);
        }
        catch (Exception error) when (error is InvalidOperationException or KeyNotFoundException or FormatException or OverflowException or InvalidDataException)
        {
            throw new InvalidDataException("invalid_process_window_pick_request");
        }
    }
}
