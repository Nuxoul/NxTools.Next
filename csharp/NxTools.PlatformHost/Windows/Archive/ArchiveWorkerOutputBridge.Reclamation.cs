using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerOutputBridge
{
    private ArchiveReclamationSession? reclamation;
    private readonly bool reclamationRequested;
    private readonly NxTools.PlatformHost.Diagnostics.IPlatformLog? reclamationLog;
    internal bool HasReclamation => reclamation is not null;
    internal void AcquireReclamation(ArchiveWorkerRequest request, string? expectedIdentity = null)
    {
        if (!reclamationRequested || reclamation is not null) throw new ArchiveOutputException("reclamation_not_admitted");
        // The original worker admission freezes options; direct fixture callers use the same validator.
        var admitted = ArchiveWorkerAdmission.Capture(request);
        try { reclamation = new ArchiveReclamationSession(admitted, session, reclamationLog, expectedIdentity); }
        catch (ArchiveOutputException failure) when (!failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal))
        { throw new ArchiveOutputException(failure.NativeCode is 32 or 33 ? "reclamation_source_busy" : "reclamation_" + failure.Stage, failure.NativeCode, failure.State); }
#if DEBUG
        reclamation.Checkpoint = stage => CommitCheckpointProbe?.Invoke(stage);
#endif
    }
    internal void VerifyReclamationSource() => (reclamation ?? throw new ArchiveOutputException("reclamation_source_required")).VerifySource();
    private object ExecuteReclamation(Guid operationId, Guid callId, string action, JsonElement call)
    {
        if (reclamation is null) throw new ArchiveOutputException("reclamation_not_admitted");
        try
        {
            var result = reclamation.Execute(action, call, cancellation.Token);
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = true,
                read_data = action == "hz5_read" ? (string?)result : JsonSerializer.Serialize(result) };
        }
        catch (OperationCanceledException) { throw new ArchiveOutputException("reclamation_cancelled"); }
        catch (IOException failure) { throw new ArchiveOutputException("reclamation_io_failed", failure.HResult & 0xffff); }
        catch (ArchiveOutputException failure) when (!failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal))
        { throw new ArchiveOutputException("reclamation_" + failure.Stage, failure.NativeCode, failure.State); }
        catch (Exception failure) when (failure is JsonException or InvalidOperationException or KeyNotFoundException or FormatException or OverflowException or ArgumentException)
        { throw new ArchiveOutputException("reclamation_record_invalid"); }
    }
}
