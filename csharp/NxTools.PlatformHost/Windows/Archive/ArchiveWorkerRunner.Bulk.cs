using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerRunner
{
    private sealed class PendingBulk(ArchiveBulkBinding binding)
    {
        internal readonly ArchiveBulkBinding Binding = binding;
        internal readonly TaskCompletionSource<bool> Sent = new(TaskCreationOptions.RunContinuationsAsynchronously);
        internal byte[]? Envelope;
    }
    private sealed partial class Run
    {
        internal bool BulkRequested;
        internal ulong NextBulkSequence = 1;
        internal Guid LastBulkCall;
        internal readonly object BulkGate = new();
        internal PendingBulk? BulkPending;
        internal long StdinJsonFrames, StdoutJsonFrames, StdinBinaryBytes, StdoutBinaryBytes, StdinBinaryFrames, StdoutBinaryFrames;
        internal long OutputWriteAckFrames;
        internal ArchiveNativeOutputFailure? OutputWriteFailure;
        internal long PayloadConversionInputBytes, PayloadEnvelopeCopyBytes;
        internal int BulkBufferHighWater, BulkBufferBytesHighWater;
        internal void ObserveBulkBuffers(int count, int bytes)
        {
            BulkBufferHighWater = Math.Max(BulkBufferHighWater, count);
            BulkBufferBytesHighWater = Math.Max(BulkBufferBytesHighWater, bytes);
        }
        internal void CompleteBulkInput(byte[] data, bool success)
        {
            lock (BulkGate) {
                if (BulkPending is { } pending && ReferenceEquals(pending.Envelope, data)) pending.Sent.TrySetResult(success);
            }
        }
    }
    private static bool BulkOperation(ArchiveWorkerRequest request) => request.ExtractOptions?.Reclamation is null &&
        (request.Operation == "extract" || IsCreation(request.Operation) || IsModification(request.Operation));
    private static bool HasBulkCapability(JsonElement capabilities) => capabilities.ValueKind == JsonValueKind.Array &&
        capabilities.EnumerateArray().All(value => value.ValueKind == JsonValueKind.String) &&
        capabilities.EnumerateArray().Select(value => value.GetString()).Distinct(StringComparer.Ordinal).Count() == capabilities.GetArrayLength() &&
        capabilities.EnumerateArray().Count(value => value.GetString() == "binary_bulk_v1") == 1;

    private static PendingBulk BeginBulk(Run run, ArchiveBulkBinding binding)
    {
        if (!run.BulkRequested || !run.Negotiated || run.SawTerminal || run.Output is null || run.Failure is not null ||
            run.Source is null && run.CreationSources is null || !binding.Valid || binding.OperationId != run.Request.OperationId)
            throw new ArchiveFault("stdout_invalid_frame");
        lock (run.BulkGate) {
            if (run.BulkPending is not null || binding.Sequence != run.NextBulkSequence || binding.CallId == run.LastBulkCall)
                throw new ArchiveFault("stdout_invalid_frame");
            var pending = new PendingBulk(binding);
            run.BulkPending = pending; run.NextBulkSequence = checked(binding.Sequence + 1); run.LastBulkCall = binding.CallId;
            return pending;
        }
    }
    private static object BindBulkResult(object result, ArchiveBulkBinding binding)
    {
        // Keep the original typed success/error/native code and add only the exact transfer binding.
        var reply = JsonSerializer.SerializeToNode(result)!;
        reply["token"] = binding.Token; reply["sequence"] = binding.Sequence;
        reply["offset"] = binding.Offset; reply["length"] = binding.Length;
        return reply;
    }
    private static async Task RejectBulk(Run run, ArchiveBulkBinding binding)
    {
        var pending = new PendingBulk(binding);
        lock (run.BulkGate) {
            if (run.BulkPending is not null) throw new ArchiveFault("stdout_invalid_frame");
            run.BulkPending = pending;
        }
        await SendBulkResult(run, pending, new { version = 1, operation_id = run.Request.OperationId,
            op = "native_result", call_id = binding.CallId, success = false, error_code = "invalid_native_call" });
    }
    private static async Task SendBulkResult(Run run, PendingBulk pending, object result)
    {
        byte[]? envelope = null;
        try {
            if (result is ArchiveBulkReply bulk) {
                using (bulk) {
                    if (bulk.Binding != pending.Binding) throw new ArchiveFault("stdout_invalid_frame");
                    envelope = ArchiveBulkFrame.Pack(bulk);
                    run.PayloadEnvelopeCopyBytes += bulk.Payload.Length;
                    run.ObserveBulkBuffers(bulk.Payload.Length == 0 ? 0 : 2, bulk.Payload.Length * 2);
                }
            } else envelope = Pack(BindBulkResult(result, pending.Binding));
            lock (run.BulkGate) pending.Envelope = envelope;
            await run.Input.Writer.WriteAsync(envelope, run.InputStop.Token);
            envelope = null; // The original unique input writer now owns and zeros the envelope.
            if (!await pending.Sent.Task.WaitAsync(run.InputStop.Token)) throw new ArchiveFault("stdin_io_failure");
            if (pending.Binding.Kind == 1) Interlocked.Increment(ref run.OutputWriteAckFrames);
        }
        finally {
            if (envelope is not null) CryptographicOperations.ZeroMemory(envelope);
            lock (run.BulkGate) { if (ReferenceEquals(run.BulkPending, pending)) run.BulkPending = null; }
        }
    }
    private static async Task ReadBulkWrite(Run run, Stream stream, uint size)
    {
        if (!ArchiveBulkFrame.ValidBodyLength(size)) throw new ArchiveFault("stdout_frame_limit");
        if (!run.BulkRequested || !run.Negotiated || run.SawTerminal) throw new ArchiveFault("stdout_invalid_frame");
        var body = new byte[size];
        try {
            stream.ReadExactly(body);
            Interlocked.Add(ref run.StdoutBinaryBytes, size + 4L); Interlocked.Increment(ref run.StdoutBinaryFrames);
            var binding = ArchiveBulkFrame.HeaderBinding(body);
            if (run.Request.ParallelExtraction && run.ClosingTokens.Contains(binding.Token)) throw new ArchiveFault("stdout_invalid_frame");
            PendingBulk pending;
            try { ArchiveBulkFrame.Parse(body, 1); pending = BeginBulk(run, binding); }
            catch (Exception failure) when (failure is InvalidDataException or ArchiveFault) {
                await RejectBulk(run, binding);
                throw new ArchiveFault("stdout_invalid_frame");
            }
            run.ObserveBulkBuffers(1, body.Length - ArchiveBulkFrame.HeaderLength);
            var reply = run.Output!.Execute(run.Request.OperationId, binding.NativeCall("write"), out var nativeFailure,
                body.AsMemory(ArchiveBulkFrame.HeaderLength), binding);
            if (run.Output.PipelinedExtraction) {
                if (run.Request.ParallelExtraction && nativeFailure is not null) run.FailNative(nativeFailure);
                if (nativeFailure is not null) Interlocked.CompareExchange(ref run.OutputWriteFailure, nativeFailure, null);
                // The OS pipe is the bounded producer/consumer queue. This single pump
                // finishes each native write before reading a later control/close record.
                lock (run.BulkGate) { if (ReferenceEquals(run.BulkPending, pending)) run.BulkPending = null; }
            } else {
                if (nativeFailure is not null) run.FailNative(nativeFailure);
                await SendBulkResult(run, pending, reply);
            }
        }
        finally { CryptographicOperations.ZeroMemory(body); }
    }
    private static async Task ReadBulkRequest(Run run, JsonElement call, bool source)
    {
        var binding = ArchiveBulkFrame.ReadRequest(run.Request.OperationId, call, source ? (byte)2 : (byte)3);
        PendingBulk pending;
        try { pending = BeginBulk(run, binding); }
        catch (ArchiveFault) {
            await RejectBulk(run, binding);
            throw new ArchiveFault("stdout_invalid_frame");
        }
        object result;
        if (source) result = run.CreationSources!.Execute(run.Request.OperationId, call, binding);
        else {
            result = run.Output!.Execute(run.Request.OperationId, call, out var nativeFailure, bulk: binding);
            if (nativeFailure is not null) run.FailNative(nativeFailure);
        }
        await SendBulkResult(run, pending, result);
    }
    private static void ObserveInputWire(Run run, byte[] data)
    {
        if ((BinaryPrimitives.ReadUInt32LittleEndian(data) & ArchiveBulkFrame.Discriminator) != 0) {
            Interlocked.Add(ref run.StdinBinaryBytes, data.Length); Interlocked.Increment(ref run.StdinBinaryFrames);
        } else {
            Interlocked.Add(ref run.TransportStdinBytes, data.Length); Interlocked.Increment(ref run.StdinJsonFrames);
        }
    }
}
