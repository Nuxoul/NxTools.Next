using System.Security.Cryptography;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerRunner
{
    // C# only forwards a member-scoped answer. Rust compares it with the immutable admission.
    internal bool RespondRisk(Guid id, ArchivePasswordTicket ticket, ulong? approvedLimit)
    {
        if (ticket.OperationId != id || ticket.PromptId == Guid.Empty || ticket.Generation == 0) return false;
        lock (gate) {
            if (!active.TryGetValue(id, out var run) || run.Request.Operation != "extract" || run.Failure is not null) return false;
            var bytes = Pack(new { version = 1, operation_id = id, op = "risk_reply", risk_reply = new { ticket, approved_limit = approvedLimit } });
            if (run.InputBytes + bytes.Length > 4 * FrameLimit || run.PendingWrites >= 2 || !run.Input.Writer.TryWrite(bytes)) {
                CryptographicOperations.ZeroMemory(bytes); return false;
            }
            run.PendingWrites++; run.InputBytes += bytes.Length; return true;
        }
    }
#if DEBUG
    // No public command/env override; only lowers fixed warning thresholds in the test apphost.
    internal (ulong Bytes, ulong Ratio)? RiskThresholdFixture { get; set; }
#endif
    private void AddRiskFixture(ref byte[] input, ArchiveWorkerRequest request)
    {
#if DEBUG
        if (RiskThresholdFixture is not { } fixture) return;
        if (request.Operation != "extract" || fixture.Bytes is 0 or > (64UL << 20) || fixture.Ratio is 0 or > 1000 ||
            !string.Equals(Path.GetFileName(Environment.ProcessPath), "NxTools.PlatformHost.Tests.exe", StringComparison.OrdinalIgnoreCase))
            throw new ArchiveOutputException("invalid_risk_fixture");
        var value = System.Text.Json.Nodes.JsonNode.Parse(input.AsSpan(4))!;
        value["zip_risk_fixture"] = new System.Text.Json.Nodes.JsonObject { ["bytes"] = fixture.Bytes, ["ratio"] = fixture.Ratio };
        CryptographicOperations.ZeroMemory(input); input = Pack(value);
#endif
    }
}
