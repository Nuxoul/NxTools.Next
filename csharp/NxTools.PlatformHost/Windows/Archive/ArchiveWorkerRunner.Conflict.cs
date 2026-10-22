using System.Text.Json;
using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerRunner
{
    // Native approval identity is never supplied by the caller. Rust validates the original ticket.
    internal bool RespondConflict(Guid id, ArchivePasswordTicket ticket, string? decision, bool applyToAll = false)
    {
        if (ticket.OperationId != id || ticket.PromptId == Guid.Empty || ticket.Generation == 0 || decision is not (null or "overwrite" or "skip" or "auto_rename")) return false;
        lock (gate) {
            if (!active.TryGetValue(id, out var run) || run.Request.ConflictPolicy is null || run.Failure is not null) return false;
            var bytes = Pack(new { version = 1, operation_id = id, op = "conflict_reply", conflict_reply = new { ticket, decision, apply_to_all = applyToAll } });
            if (run.InputBytes + bytes.Length > 4 * FrameLimit || run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(bytes)) {
                CryptographicOperations.ZeroMemory(bytes); return false;
            }
            run.PendingWrites++; run.InputBytes += bytes.Length; return true;
        }
    }
}
