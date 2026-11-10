using System.Text.Json;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveWorkerRunner
{
    private sealed class PendingNativeReply
    {
        internal readonly TaskCompletionSource<bool> Sent = new(TaskCreationOptions.RunContinuationsAsynchronously);
    }
    private sealed partial class Run
    {
        // Pump-owned; no independent listener, channel, pipe, or unbounded task backlog.
        internal readonly List<Task> NativePending = [];
        internal readonly HashSet<Guid> NativeCalls = [];
        internal readonly HashSet<Guid> ClosingTokens = [];
        internal int NativePendingHighWater;
        private readonly object nativeReplyGate = new();
        private readonly Dictionary<byte[], PendingNativeReply> nativeReplies = [];
        internal void CompleteNativeInput(byte[] envelope, bool sent)
        {
            lock (nativeReplyGate) if (nativeReplies.Remove(envelope, out var pending)) pending.Sent.TrySetResult(sent);
        }
        internal async Task SendNativeReply(object reply)
        {
            byte[]? envelope = Pack(reply);
            var pending = new PendingNativeReply();
            lock (nativeReplyGate) nativeReplies.Add(envelope, pending);
            try {
                await Input.Writer.WriteAsync(envelope, InputStop.Token);
                envelope = null; // Unique input pump now owns and zeros these bytes.
                if (!await pending.Sent.Task.WaitAsync(InputStop.Token)) throw new ArchiveFault("stdin_io_failure");
            }
            finally {
                if (envelope is not null) {
                    lock (nativeReplyGate) nativeReplies.Remove(envelope);
                    System.Security.Cryptography.CryptographicOperations.ZeroMemory(envelope);
                }
            }
        }
    }

    private static async Task JoinNative(Run run)
    {
        if (run.NativePending.Count == 0) return;
        await Task.WhenAll(run.NativePending);
        run.NativePending.Clear();
    }

    private static async Task DispatchNative(Run run, JsonElement native)
    {
        var action = native.GetProperty("action").GetString()!;
        var callId = native.GetProperty("call_id").GetGuid();
        if (callId == Guid.Empty || !run.NativeCalls.Add(callId)) throw new ArchiveFault("stdout_invalid_frame");
        if (action == "commit")
        {
            var token = native.GetProperty("token").GetGuid();
            if (token == Guid.Empty || !run.ClosingTokens.Add(token)) throw new ArchiveFault("stdout_invalid_frame");
        }
        run.NativePending.RemoveAll(task => task.IsCompleted);
        while (run.NativePending.Count >= 32)
        {
            await Task.WhenAny(run.NativePending);
            run.NativePending.RemoveAll(task => task.IsCompleted);
            if (run.Failure is not null) throw run.Failure;
        }
        if (run.Failure is not null) throw run.Failure;
        // The cloned control survives disposal of the pump's JSON document and zeroed buffer.
        var owned = native.Clone();
        run.NativePending.Add(Task.Run(async () => {
            try
            {
                var reply = run.Output!.ExecuteParallel(run.Request.OperationId, owned, out var failure);
                if (failure is not null) run.FailNative(failure);
                await run.SendNativeReply(reply);
            }
            catch (OperationCanceledException) when (run.InputStop.IsCancellationRequested) { }
            catch (ArchiveOutputException failure) { run.Fail(failure.Stage, failure.NativeCode); }
            catch (Exception) { run.Fail("runner_failure"); }
        }));
        run.NativePendingHighWater = Math.Max(run.NativePendingHighWater, run.NativePending.Count);
    }
}
