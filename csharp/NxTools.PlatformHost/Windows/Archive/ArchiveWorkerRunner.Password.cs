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
    private void DiagnosticPassword(Run run, JsonElement state)
    {
        var phase = state.GetProperty("state").GetString();
        if (phase is not ("running" or "awaiting_password" or "attempt_finished" or "extracting")) throw new ArchiveFault("stdout_invalid_frame");
        try {
            log?.Write("info", "archive.worker.password." + phase, run.Request.OperationId,
                new Dictionary<string, object?> { ["attempt_id"] = state.GetProperty("attempt_id").GetGuid(),
                    ["attempt"] = state.GetProperty("attempt").GetUInt32(),
                    ["error_code"] = state.TryGetProperty("error_code", out var code) ? code.GetString() : null,
                    ["elapsed_ms"] = run.Clock.ElapsedMilliseconds });
        } catch (Exception) { /* Diagnostics never change the operation. */ }
    }
}
