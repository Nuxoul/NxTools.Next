using System.Diagnostics;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerOutputBridge
{
    private int parallelBegins;
    // Only independently owned ordinary files use this route. The stdout pump alone prepares
    // directories and consumes payloads; the short gate reserves shared credits and journals.
    internal void PrepareParallelRoot()
    {
        if (!ParallelOrdinary) throw new ArchiveOutputException("invalid_state");
        if (!outputPrepared) { session.CreatePreparedRoot(); outputPrepared = true; }
    }

    internal object ExecuteParallel(Guid operationId, JsonElement call, out ArchiveNativeOutputFailure? nativeFailure)
    {
        var callId = call.GetProperty("call_id").GetGuid();
        var action = call.GetProperty("action").GetString()!;
        if (callId == Guid.Empty || action is not ("begin" or "begin_overwrite" or "commit"))
            throw new ArchiveOutputException("invalid_native_call");
        var started = Stopwatch.GetTimestamp();
        nativeFailure = null;
        Guid? token = null;
        try
        {
            if (action is "begin" or "begin_overwrite")
            {
                var relative = call.GetProperty("path").GetString()!;
                bool overwrite;
                lock (parallelGate)
                {
                    if (writeFailure is not null) throw new ArchiveOutputException(writeFailure.Category, writeFailure.NativeCode);
                    if (action == "begin_overwrite" && !conflicts) throw new ArchiveOutputException("invalid_native_call");
                    if ((ulong)(files + parallelBegins) >= maximumFiles || active.Count + parallelBegins >= 4096)
                        throw new ArchiveOutputException("session_entry_limit");
                    parallelBegins++;
                    overwrite = action == "begin_overwrite" || !conflicts && overwrites.ContainsKey(relative);
                }
                try { token = session.BeginExtractedFile(relative, overwrite, cancellation.Token); }
                catch (ArchiveOutputException exists) when (exists.Stage == "target_exists" && !overwrite)
                {
                    return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId,
                        success = false, error_code = "target_exists", existing_bytes = session.ExistingExtractionSize(relative) };
                }
                finally { lock (parallelGate) {
                    parallelBegins--;
                    if (token is { } opened) { files++; active.Add(opened); tokenBytes[opened] = 0; outputNames[opened] = relative; }
                } }
            }
            else
            {
                token = call.GetProperty("token").GetGuid();
                ArchiveNativeOutputFailure? failed;
                bool owned = false;
                lock (parallelGate) {
                    failed = writeFailure;
                    if (failed is null) RequireToken(token.Value);
                    else if (active.Remove(token.Value)) { owned = true; tokenBytes.Remove(token.Value); outputNames.Remove(token.Value); }
                }
                if (failed is not null) {
                    if (owned) session.Cancel(token.Value);
                    throw new ArchiveOutputException(failed.Category, failed.NativeCode, failed.State);
                }
                if (call.TryGetProperty("metadata", out var metadata) && metadata.ValueKind != JsonValueKind.Null)
                    session.SetMetadata(token.Value, ArchiveOutputMetadata.Parse(metadata));
                ArchiveOutputResult result;
                var commitStarted = Stopwatch.GetTimestamp();
                try { result = session.Commit(token.Value, cancellation.Token); }
                finally { Interlocked.Add(ref commitTicks, Stopwatch.GetTimestamp() - commitStarted); }
                lock (parallelGate)
                {
                    LastCommitState = result.State; LastCommitNativeCode = result.NativeCode;
                    if (result.Category == "committed") MarkPublished(token.Value);
                }
                if (result.Category != "committed") throw new ArchiveOutputException(result.Category, result.NativeCode, result.State);
                CommitCheckpointProbe?.Invoke("native_closed");
            }
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId,
                success = true, token, error_code = (string?)null };
        }
        catch (OperationCanceledException)
        {
            nativeFailure = new(operationId, callId, action, token, "cancelled", null, ArchiveCommitState.FailedBeforeCommit);
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = false, error_code = "cancelled" };
        }
        catch (ArchiveOutputException failure)
        {
            lock (parallelGate) {
                if (action == "commit") { LastCommitState = failure.State; LastCommitNativeCode = failure.NativeCode; }
                nativeFailure = writeFailure ?? new(operationId, callId, action, token, failure.Stage, failure.NativeCode, failure.State);
            }
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = false,
                error_code = failure.Stage, native_error_code = failure.NativeCode };
        }
        finally { if (action is "begin" or "begin_overwrite") Interlocked.Add(ref beginTicks, Stopwatch.GetTimestamp() - started); }
    }

    private object ExecuteParallelWrite(Guid operationId, Guid callId, JsonElement call, ReadOnlyMemory<byte>? data,
        ArchiveBulkBinding? bulk, out ArchiveNativeOutputFailure? nativeFailure)
    {
        nativeFailure = null;
        var token = call.GetProperty("token").GetGuid();
        try
        {
            if (data is not { } payload) throw new ArchiveOutputException("invalid_native_call");
            lock (parallelGate) {
                if (writeFailure is { } failed) throw new ArchiveOutputException(failed.Category, failed.NativeCode, failed.State);
                RequireToken(token);
                if (bulk is not { Kind: 1 } binding || binding.Token != token ||
                    binding.Offset != tokenBytes.GetValueOrDefault(token) || binding.Length != payload.Length)
                    throw new ArchiveOutputException("invalid_native_call");
                if (payload.Length is < 1 or > 65536 || (ulong)payload.Length > maxTotal - bytes)
                    throw new ArchiveOutputException("write_limit");
                bytes += (ulong)payload.Length;
            }
            session.Write(token, payload);
            lock (parallelGate) {
                tokenBytes[token] += (ulong)payload.Length;
                PayloadWriteCalls++;
            }
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = true };
        }
        catch (ArchiveOutputException failure)
        {
            lock (parallelGate) {
                writeFailure ??= new(operationId, callId, "write", token, failure.Stage, failure.NativeCode, failure.State);
                nativeFailure = writeFailure;
            }
            return new { version = 1, operation_id = operationId, op = "native_result", call_id = callId, success = false,
                error_code = failure.Stage, native_error_code = failure.NativeCode };
        }
    }
}
