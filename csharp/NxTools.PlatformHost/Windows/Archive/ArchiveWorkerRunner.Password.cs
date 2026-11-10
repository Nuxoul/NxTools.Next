using System.Text.Json;
using System.Text.Json.Serialization;
using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed record ArchivePasswordTicket(
    [property: JsonPropertyName("operation_id")] Guid OperationId,
    [property: JsonPropertyName("prompt_id")] Guid PromptId,
    [property: JsonPropertyName("generation")] ulong Generation);

internal sealed partial class ArchiveWorkerRunner
{
    // Only transport admission here. Rust InteractionPort alone accepts/rejects the ticket.
    // null is the typed user Cancel response, not an empty-password guess.
    internal bool RespondPassword(Guid id, ArchivePasswordTicket ticket, string? password)
    {
        if (ticket.OperationId != id || ticket.PromptId == Guid.Empty || ticket.Generation == 0 ||
            password is not null && (System.Text.Encoding.UTF8.GetByteCount(password) > 4096 || password.Contains('\0'))) return false;
        lock (gate) {
            if (!active.TryGetValue(id, out var run) || !run.Request.PasswordInteraction || run.Failure is not null) return false;
            var bytes = Pack(new { version = 1, operation_id = id, op = "password_reply",
                password_reply = new { ticket, password } });
            if (run.InputBytes + bytes.Length > 4 * FrameLimit || run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(bytes)) {
                CryptographicOperations.ZeroMemory(bytes); return false;
            }
            run.PendingWrites++; run.InputBytes += bytes.Length; return true;
        }
    }
    internal static bool ValidPasswordBytes(ReadOnlySpan<byte> password)
    {
        if (password.Length > 4096 || password.IndexOf((byte)0) >= 0) return false;
        try { _ = new System.Text.UTF8Encoding(false, true).GetCharCount(password); return true; }
        catch (System.Text.DecoderFallbackException) { return false; }
    }
    // UTF8 span -> private JSON directly; authorization never creates a managed secret string.
    internal bool RespondPasswordBytes(Guid id, ArchivePasswordTicket ticket, byte[]? password)
    {
        if (ticket.OperationId != id || ticket.PromptId == Guid.Empty || ticket.Generation == 0 || password is not null && !ValidPasswordBytes(password)) return false;
        lock (gate) {
            if (!active.TryGetValue(id, out var run) || !run.Request.PasswordInteraction || run.Failure is not null) return false;
            var bytes = PackSecret(writer => {
                writer.WriteStartObject(); writer.WriteNumber("version", 1); writer.WriteString("operation_id", id); writer.WriteString("op", "password_reply");
                writer.WriteStartObject("password_reply"); writer.WritePropertyName("ticket"); JsonSerializer.Serialize(writer, ticket);
                if (password is null) writer.WriteNull("password"); else writer.WriteString("password", password.AsSpan());
                writer.WriteEndObject(); writer.WriteEndObject();
            });
            if (run.InputBytes + bytes.Length > 4 * FrameLimit || run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(bytes)) {
                CryptographicOperations.ZeroMemory(bytes); return false;
            }
            run.PendingWrites++; run.InputBytes += bytes.Length; return true;
        }
    }
    private sealed class SecretBuffer : System.Buffers.IBufferWriter<byte>, IDisposable
    {
        private readonly byte[] bytes = new byte[FrameLimit];
        internal int Count { get; private set; }
        internal ReadOnlySpan<byte> Written => bytes.AsSpan(0, Count);
        public void Advance(int count) { if (count < 0 || count > bytes.Length - Count) throw new ArchiveFault("input_frame_limit"); Count += count; }
        public Memory<byte> GetMemory(int sizeHint = 0) { if (sizeHint > bytes.Length - Count) throw new ArchiveFault("input_frame_limit"); return bytes.AsMemory(Count); }
        public Span<byte> GetSpan(int sizeHint = 0) => GetMemory(sizeHint).Span;
        public void Dispose() => CryptographicOperations.ZeroMemory(bytes);
    }
    private static byte[] PackSecret(Action<Utf8JsonWriter> write)
    {
        using var buffer = new SecretBuffer();
        using (var writer = new Utf8JsonWriter(buffer)) { write(writer); writer.Flush(); }
        if (buffer.Count is < 1 or > FrameLimit) throw new ArchiveFault("input_frame_limit");
        var frame = new byte[4 + buffer.Count];
        System.Buffers.Binary.BinaryPrimitives.WriteUInt32LittleEndian(frame, (uint)buffer.Count);
        buffer.Written.CopyTo(frame.AsSpan(4)); return frame;
    }
    private static void AddBrowserPassword(ref byte[] input, byte[] password)
    {
        using var original = JsonDocument.Parse(input.AsMemory(4));
        var replacement = PackSecret(writer => {
            writer.WriteStartObject();
            foreach (var property in original.RootElement.EnumerateObject()) property.WriteTo(writer);
            writer.WriteStartArray("browser_password"); foreach (var value in password) writer.WriteNumberValue(value); writer.WriteEndArray();
            writer.WriteEndObject();
        });
        CryptographicOperations.ZeroMemory(input); input = replacement;
    }
    private void DiagnosticPassword(Run run, JsonElement state)
    {
        var phase = state.GetProperty("state").GetString();
        if (phase is not ("running" or "awaiting_password" or "attempt_finished" or "extracting" or "modifying")) throw new ArchiveFault("stdout_invalid_frame");
        try {
            log?.Write("info", "archive.worker.password." + phase, run.Request.OperationId,
                new Dictionary<string, object?> { ["attempt_id"] = state.GetProperty("attempt_id").GetGuid(),
                    ["attempt"] = state.GetProperty("attempt").GetUInt32(),
                    ["error_code"] = state.TryGetProperty("error_code", out var code) ? code.GetString() : null,
                    ["elapsed_ms"] = run.Clock.ElapsedMilliseconds });
        } catch (Exception) { /* Diagnostics never change the operation. */ }
    }
}
