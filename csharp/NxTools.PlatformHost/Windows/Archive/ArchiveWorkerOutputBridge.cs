using System.Text.Json;
using System.Security.Cryptography;
using System.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed record ArchiveNativeOutputFailure(Guid OperationId, Guid CallId, string Action, Guid? Token,
    string Category, int? NativeCode, ArchiveCommitState State);

// One worker stdout pump owns this bridge. Native-call bodies never reach business events/logs.
internal sealed partial class ArchiveWorkerOutputBridge : IDisposable
{
    private readonly ArchiveOutputSession session;
    private readonly ulong maxTotal;
    private readonly bool allowPartialRead;
    private readonly string? createName;
    private readonly HashSet<Guid> active = [];
    private ulong bytes;
    private int files;
    private readonly ulong maximumFiles;
    private bool outputPrepared;
    private int directoryCalls;
    private readonly IReadOnlyDictionary<string, string> overwrites;
    private readonly bool browserEntry;
    private readonly bool conflicts;
    private readonly bool ordinaryExtraction;
    internal bool PipelinedExtraction => ordinaryExtraction;
    internal bool ParallelOrdinary { get; set; }
    private readonly object parallelGate = new();
    private ArchiveNativeOutputFailure? writeFailure;
    internal Action<string>? CommitCheckpointProbe { get; set; }
    internal Action<string>? BeginCheckpointProbe { get => session.BeginCheckpointProbe; set => session.BeginCheckpointProbe = value; }
    private Action<int>? writeCheckpointProbe;
    internal Action<int>? WriteCheckpointProbe {
        get => writeCheckpointProbe;
        set { writeCheckpointProbe = value; if (ordinaryExtraction) session.ExtractionWriteCheckpointProbe = value; }
    }
    internal long PayloadWriteCalls { get; private set; }
    private long commitTicks;
    internal double NativeCommitMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref commitTicks)).TotalMilliseconds;
    private long beginTicks, directoryTicks, directoryMetadataTicks;
    internal double NativeBeginMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref beginTicks)).TotalMilliseconds;
    internal double NativeDirectoryMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref directoryTicks)).TotalMilliseconds;
    internal double NativeDirectoryMetadataMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref directoryMetadataTicks)).TotalMilliseconds;
    internal ulong PayloadWriteBytes => bytes;
    internal long PayloadConversionInputBytes { get; private set; }
    private int filesCommitted;
    private readonly Dictionary<Guid,ulong> tokenBytes=[];
    private long committedBytes;
    internal ulong BytesCommitted => (ulong)Interlocked.Read(ref committedBytes);
    private Guid? committingToken;
    internal int FilesCommitted => Volatile.Read(ref filesCommitted);
    internal ArchiveCommitState? LastCommitState { get; private set; }
    internal int? LastCommitNativeCode { get; private set; }
    // Current commit facts only; terminal failure ownership is returned for the exact call.
    private readonly CancellationTokenSource cancellation = new();
    internal CancellationToken OperationCancellation => cancellation.Token;
    internal void RequestCancellation() { try { cancellation.Cancel(); } catch (ObjectDisposedException) { } }
    internal ArchiveOutputSession CreationOutput => session;
    internal void BindZoneIdentifier(ReadOnlyMemory<byte>? value) => session.BindZoneIdentifier(value);
    internal ArchiveWorkerOutputBridge(ArchiveWorkerRequest request, NxTools.PlatformHost.Diagnostics.IPlatformLog? log, ArchiveOutputSession? prepared = null, bool bulkTransfer = false)
    {
        if (request.Operation != "extract" && !ArchiveWorkerRunner.IsModification(request.Operation) && !ArchiveWorkerRunner.IsCreation(request.Operation) || string.IsNullOrEmpty(request.OutputRoot)) throw new ArchiveOutputException("invalid_output_request");
        reclamationRequested = request.ExtractOptions?.Reclamation is not null; reclamationLog = log;
        outputPrepared = !request.JobExecution && !bulkTransfer;
        editing = ArchiveWorkerRunner.IsModification(request.Operation);
        allowPartialRead = ArchiveWorkerRunner.IsCreation(request.Operation) || editing;
        createName = editing ? Path.GetFileName(request.Path) : allowPartialRead ? request.CreateName : null;
        conflicts = request.ConflictPolicy is not null;
        browserEntry = request.BrowserEntry is not null;
        ordinaryExtraction = request.Operation == "extract" && !browserEntry && !reclamationRequested && request.DragMaterialization is null;
        var limits = JsonSerializer.SerializeToElement(request.Limits);
        maxTotal = limits.TryGetProperty("max_output_bytes", out var maximum) ? maximum.GetUInt64() : 100UL << 30;
        if (maxTotal is 0 or > (1UL << 50)) throw new ArchiveOutputException("write_limit");
        if (request.ConfirmedOverwrites?.Count > 4096) throw new ArchiveOutputException("session_entry_limit");
        overwrites = request.ConfirmedOverwrites is null ? new Dictionary<string, string>() : new Dictionary<string, string>(request.ConfirmedOverwrites, StringComparer.Ordinal);
        session = prepared ?? (bulkTransfer || ArchiveWorkerRunner.IsTreeCreation(request.Operation) || request.OutputReadmission is not null || request.JobExecution
            ? ArchiveOutputSession.Prepare(request.OutputRoot, request.OperationId, log)
            : ArchiveOutputSession.Open(request.OutputRoot, request.OperationId, log));
        maximumFiles = 4096;
        if (request.JobLargeCount) {
            var admitted = (ArchiveAdmittedLimits)request.Limits;
            maximumFiles = Math.Min(admitted.max_entries, admitted.max_metadata_bytes / 2048);
            session.ConfigureJournal(admitted);
        }
        session.CommitCheckpointProbe = stage => {
            if (stage == "commit_in_flight") LastCommitState = ArchiveCommitState.CommitInFlight;
            if (stage == "native_published") {
                LastCommitState = ArchiveCommitState.Committed;
                if (committingToken is { } published) MarkPublished(published);
            }
            CommitCheckpointProbe?.Invoke(stage);
        };
    }
    internal object Execute(Guid operationId, JsonElement call, out ArchiveNativeOutputFailure? nativeFailure,
        ReadOnlyMemory<byte>? writeData = null, ArchiveBulkBinding? bulk = null)
    {
        var callId = call.GetProperty("call_id").GetGuid();
        if (callId == Guid.Empty) throw new ArchiveOutputException("invalid_native_call");
        nativeFailure = null;
        var action = call.GetProperty("action").GetString()!;
        if (ParallelOrdinary && action == "write")
            return ExecuteParallelWrite(operationId, callId, call, writeData, bulk, out nativeFailure);
        Guid? token = null;
        var callStarted = Stopwatch.GetTimestamp();
        try
        {
            if (writeFailure is { } firstWrite && action != "abort") {
                if (action == "commit" && call.TryGetProperty("token", out var failedCurrent) && active.Contains(failedCurrent.GetGuid())) {
                    session.Cancel(failedCurrent.GetGuid()); active.Remove(failedCurrent.GetGuid());
                }
                throw new ArchiveOutputException(firstWrite.Category, firstWrite.NativeCode, firstWrite.State);
            }
            if (action.StartsWith("hz5_", StringComparison.Ordinal)) return ExecuteReclamation(operationId, callId, action, call);
            if (reclamationRequested) {
                if (reclamation is null) throw new ArchiveOutputException("reclamation_source_required");
                reclamation.ValidateOutputAction(action);
            }
            if (!outputPrepared && action is ("directory" or "directory_new" or "begin" or "begin_overwrite" or "observe_target")) { session.CreatePreparedRoot();outputPrepared=true; }
            if (HandleExtendedCall(action,call,operationId,callId,out var extended)) return extended!;
            switch (action)
            {
                // General jobs defer directory effects until the original validated plan actually
                // requests output; an empty archive must not create a spurious destination.
                case "directory_new":
                    if (createName is not null || browserEntry || ++directoryCalls>65536) throw new ArchiveOutputException("invalid_output_request");
                    session.CreateExclusiveDirectory(call.GetProperty("path").GetString()!);break;
                case "directory":
                    if (++directoryCalls > 65536) throw new ArchiveOutputException("session_entry_limit");
                    if (createName is not null) throw new ArchiveOutputException("invalid_output_request");
                    session.EnsureDirectory(call.GetProperty("path").GetString()!); break;
                case "begin_overwrite":
                    if (!ordinaryExtraction || !conflicts) throw new ArchiveOutputException("invalid_native_call");
                    AdmitFile(accepted: false);
                    var approvedPath = call.GetProperty("path").GetString()!;
                    token = session.BeginExtractedFile(approvedPath, overwrite: true, cancellation.Token);
                    files++; active.Add(token.Value); tokenBytes[token.Value] = 0; outputNames[token.Value] = approvedPath;
                    break;
                case "begin":
                    AdmitFile(accepted: !ordinaryExtraction);
                    if (browserEntry && files != 1) throw new ArchiveOutputException("invalid_output_request");
                    var relative = call.GetProperty("path").GetString()!;
                    if (browserEntry && relative.Split('/').Any(part => part.Equals("manifest.json", StringComparison.OrdinalIgnoreCase))) throw new ArchiveOutputException("invalid_output_request");
                    if (createName is not null && !AllowedOutputName(relative)) throw new ArchiveOutputException("invalid_output_request");
                    var overwrite = editing || !conflicts && overwrites.TryGetValue(relative, out _);
                    var expected = editing ? editTargetIdentity ?? throw new ArchiveOutputException("source_lease_required") : overwrite ? overwrites[relative] : null;
                    if (ordinaryExtraction) {
                        try { token = session.BeginExtractedFile(relative, overwrite, cancellation.Token); }
                        catch (ArchiveOutputException exists) when (exists.Stage == "target_exists" && !overwrite) {
                            var existingBytes = session.ExistingExtractionSize(relative);
                            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId,
                                success = false, error_code = "target_exists", existing_bytes = existingBytes };
                        }
                        files++;
                    } else token = session.BeginFile(relative, expected, overwrite, cancellation.Token);
                    active.Add(token.Value); tokenBytes[token.Value]=0; outputNames[token.Value]=relative;
                    if(volumeRequest is not null && files==1)session.ExcludeEncoderSpool(token.Value);
                    break;
                case "write":
                    var writeToken = call.GetProperty("token").GetGuid(); RequireToken(writeToken);
                    if (writeData is { } raw) {
                        if (bulk is not { Kind: 1 } writeBinding || writeBinding.Token != writeToken ||
                            writeBinding.Offset != tokenBytes.GetValueOrDefault(writeToken) || raw.Length != writeBinding.Length)
                            throw new ArchiveOutputException("invalid_native_call");
                        WritePayload(writeToken, raw);
                    } else {
                        var data = call.GetProperty("data");
                        if (data.ValueKind != JsonValueKind.Array || data.GetArrayLength() is < 1 or > 65536) throw new ArchiveOutputException("write_limit");
                        var buffer = new byte[data.GetArrayLength()];
                        try {
                            var index = 0; foreach (var item in data.EnumerateArray()) buffer[index++] = item.GetByte();
                            WritePayload(writeToken, buffer);
                        }
                        finally { CryptographicOperations.ZeroMemory(buffer); }
                    }
                    break;
                case "read":
                    if (!allowPartialRead) throw new ArchiveOutputException("read_not_supported");
                    var readToken = call.GetProperty("token").GetGuid(); RequireToken(readToken);
                    if (!call.GetProperty("offset").TryGetInt64(out var readOffset) || !call.GetProperty("length").TryGetInt32(out var readLength))
                        throw new ArchiveOutputException("read_limit");
                    var readBytes = session.ReadPartial(readToken, readOffset, readLength);
                    if (bulk is { Kind: 3 } readBinding) return new ArchiveBulkReply(readBinding, readBytes);
                    try { PayloadConversionInputBytes += readBytes.Length; return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId,
                        success = true, read_data = Convert.ToBase64String(readBytes) }; }
                    finally { CryptographicOperations.ZeroMemory(readBytes); }
                case "directory_metadata":
                    if (allowPartialRead) throw new ArchiveOutputException("metadata_not_supported");
                    session.FinalizeDirectoryMetadata(call.GetProperty("path").GetString()!, ArchiveOutputMetadata.Parse(call.GetProperty("metadata")), cancellation.Token);
                    break;
                case "file_metadata":
                    if (allowPartialRead) throw new ArchiveOutputException("metadata_not_supported");
                    var metadataToken = call.GetProperty("token").GetGuid(); RequireToken(metadataToken);
                    session.SetMetadata(metadataToken, ArchiveOutputMetadata.Parse(call.GetProperty("metadata")));
                    break;
                case "commit":
                    var commitToken = call.GetProperty("token").GetGuid(); RequireToken(commitToken);
                    if (ordinaryExtraction && call.TryGetProperty("metadata", out var closingMetadata) && closingMetadata.ValueKind != JsonValueKind.Null)
                        session.SetMetadata(commitToken, ArchiveOutputMetadata.Parse(closingMetadata));
                    committingToken = commitToken;
                    ArchiveOutputResult result;
                    try {
                        if (volumeRequest is not null && (!volumeVerified || !verifiedVolumeTokens.Contains(commitToken))) throw new ArchiveOutputException("volumes_not_verified");
                        var started = Stopwatch.GetTimestamp();
                        try { result = editing ? session.CommitVerifiedEdit(commitToken,editSource ?? throw new ArchiveOutputException("source_lease_required"),editCompletion ?? throw new ArchiveOutputException("edit_reads_incomplete"),cancellation.Token) : session.Commit(commitToken, cancellation.Token); }
                        finally { Interlocked.Add(ref commitTicks, Stopwatch.GetTimestamp() - started); }
                    }
                    finally { committingToken = null; }
                    LastCommitState = result.State;
                    LastCommitNativeCode = result.NativeCode;
                    // Publication facts survive worker cancellation or a lost acknowledgement.
                    if (result.State is ArchiveCommitState.Committed or ArchiveCommitState.CommittedButVerificationFailed or ArchiveCommitState.TargetChangedAfterCommit) {
                        MarkPublished(commitToken);
                    }
                    if (result.Category != "committed") throw new ArchiveOutputException(result.Category, result.NativeCode, result.State);
                    if (ordinaryExtraction) CommitCheckpointProbe?.Invoke("native_closed");
                    break;
                case "abort":
                    var abortToken = call.GetProperty("token").GetGuid(); RequireToken(abortToken);
                    var aborted = session.Cancel(abortToken);
                    if (aborted.Category != "partial_removed" && !(reclamationRequested && aborted.Category == "reclamation_retained")) throw new ArchiveOutputException(aborted.Category, aborted.NativeCode);
                    active.Remove(abortToken); break;
                default: throw new ArchiveOutputException("invalid_native_call");
            }
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = true, token, error_code = (string?)null };
        }
        catch (OperationCanceledException) when (action == "directory_metadata" ||
            cancellation.IsCancellationRequested && action is ("edit_reads_complete" or "begin" or "begin_overwrite"))
        {
            var state = FilesCommitted > 0 ? ArchiveCommitState.Committed : ArchiveCommitState.FailedBeforeCommit;
            nativeFailure = new(operationId, callId, action, null, "cancelled", null, state);
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = false, error_code = "cancelled" };
        }
        catch (ArchiveOutputException failure)
        {
            var category = reclamationRequested && !failure.Stage.StartsWith("reclamation_", StringComparison.Ordinal) && failure.Stage != "cancelled" ? "reclamation_" + failure.Stage : failure.Stage;
            reclamation?.ReportFailure(category, failure.NativeCode);
            if (action == "commit") { LastCommitState = failure.State; LastCommitNativeCode = failure.NativeCode; }
            var state = action == "directory_metadata" && FilesCommitted > 0 ? ArchiveCommitState.Committed : failure.State;
            nativeFailure = new(operationId, callId, action,
                call.TryGetProperty("token", out var failedToken) ? failedToken.GetGuid() : token,
                category, failure.NativeCode, state);
            if (ordinaryExtraction && action == "write") { lock (parallelGate) writeFailure ??= nativeFailure; }
            if (writeFailure is not null) nativeFailure = writeFailure;
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = false, token = (Guid?)null, error_code = category, native_error_code = failure.NativeCode };
        }
        finally {
            var elapsed = Stopwatch.GetTimestamp() - callStarted;
            if (action is "begin" or "begin_overwrite") Interlocked.Add(ref beginTicks, elapsed);
            else if (action is "directory" or "directory_new") Interlocked.Add(ref directoryTicks, elapsed);
            else if (action == "directory_metadata") Interlocked.Add(ref directoryMetadataTicks, elapsed);
        }
    }
    private void WritePayload(Guid token, ReadOnlyMemory<byte> data)
    {
        if (data.Length is < 1 or > 65536) throw new ArchiveOutputException("write_limit");
        var maximum = volumeRequest is null ? maxTotal : maxTotal > ulong.MaxValue / 2 ? ulong.MaxValue : maxTotal * 2;
        if ((ulong)data.Length > maximum - bytes) throw new ArchiveOutputException(browserEntry ? "selected_output_limit" : "write_limit");
        if (!session.Write(token, data)) throw new ArchiveOutputException("write_file");
        bytes += (ulong)data.Length;
        PayloadWriteCalls++;
        tokenBytes[token] = tokenBytes.GetValueOrDefault(token) + (ulong)data.Length;
        if (!ordinaryExtraction) WriteCheckpointProbe?.Invoke(data.Length);
    }
    private void MarkPublished(Guid token) {
        if (active.Remove(token)) {
            Interlocked.Increment(ref filesCommitted);
            Interlocked.Add(ref committedBytes,checked((long)tokenBytes.GetValueOrDefault(token)));
            tokenBytes.Remove(token);
            if (volumeRequest is null) outputNames.Remove(token);
        }
    }
    private void AdmitFile(bool accepted = true) {
        if ((ulong)files >= maximumFiles || active.Count >= 4096) throw new ArchiveOutputException("session_entry_limit");
        if (accepted) files++;
    }
    private void RequireToken(Guid token) { if (!active.Contains(token)) throw new ArchiveOutputException("unknown_token"); }
    public void Dispose() { try { session.Dispose(); } finally { reclamation?.ReportEnd(cancellation.IsCancellationRequested); reclamation?.Dispose(); cancellation.Dispose(); } }
}
