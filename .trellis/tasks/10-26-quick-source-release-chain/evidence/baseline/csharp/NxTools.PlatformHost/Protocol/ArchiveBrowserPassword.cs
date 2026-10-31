using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

internal sealed record ArchiveBrowserPasswordPrompt(Guid OperationId, Guid SessionId, ulong Generation,
    Guid PromptId, ulong TicketGeneration, string ErrorCode, uint Attempt)
{
    internal ArchivePasswordTicket Ticket => new(OperationId, PromptId, TicketGeneration);
    internal ProtocolEnvelope Envelope() => ProtocolEnvelope.Create("platform_event", new { @event = "archive_password", data = this }, OperationId);
    internal static ArchiveBrowserPasswordPrompt? From(JsonElement frame, Guid operation, Guid session, ulong generation)
    {
        var state = frame.GetProperty("password_state");
        if (state.GetProperty("state").GetString() != "awaiting_password") return null;
        var ticket = state.GetProperty("ticket").Deserialize<ArchivePasswordTicket>() ?? throw new InvalidDataException();
        if (ticket.OperationId != operation || ticket.PromptId == Guid.Empty || ticket.Generation == 0) throw new InvalidDataException();
        var code = state.GetProperty("error_code").GetString();
        if (code is not ("PasswordRequired" or "WrongPassword")) throw new InvalidDataException();
        return new(operation, session, generation, ticket.PromptId, ticket.Generation, code, state.GetProperty("attempt").GetUInt32());
    }
}
internal static class ArchiveBrowserPassword
{
    internal static bool Respond(ProtocolEnvelope envelope, ArchiveReadSession reads, ArchiveEntrySession entries)
    {
        byte[]? secret = null;
        try {
            var data = envelope.Payload.GetProperty("data");
            var prompt = data.GetProperty("prompt").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options) ?? throw new InvalidDataException();
            if (envelope.CorrelationId != prompt.OperationId || prompt.Attempt is < 1 or > 3) return false;
            var password = data.GetProperty("password");
            if (password.ValueKind != JsonValueKind.Null) {
                if (password.ValueKind != JsonValueKind.Array || password.GetArrayLength() > 4096) return false;
                secret = new byte[password.GetArrayLength()]; var index = 0;
                foreach (var item in password.EnumerateArray()) secret[index++] = item.GetByte();
                if (!ArchiveWorkerRunner.ValidPasswordBytes(secret)) return false;
            }
            if (entries.IsAuthorizationPasswordOperation(prompt.OperationId))
                return entries.RespondAuthorizationPassword(prompt.OperationId, prompt.SessionId, prompt.Generation, prompt.Ticket, secret);
            // Legacy reads and selected-entry opens retain their existing short-lived string route.
            var text = secret is null ? null : new UTF8Encoding(false, true).GetString(secret);
            return reads.RespondPassword(prompt, text) || entries.RespondPassword(prompt.OperationId, prompt.SessionId, prompt.Generation, prompt.Ticket, text);
        } catch (Exception failure) when (failure is JsonException or InvalidDataException or InvalidOperationException or FormatException or DecoderFallbackException or KeyNotFoundException) { return false; }
        finally { if (secret is not null) CryptographicOperations.ZeroMemory(secret); }
    }
    internal static ProtocolEnvelope NormalizeRead(ProtocolEnvelope envelope) => envelope with {
        Payload = JsonSerializer.SerializeToElement(new { command = "archive_read", data = envelope.Payload.GetProperty("data") }, JsonDefaults.Options)
    };
}
