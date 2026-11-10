using System.Reflection;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt06BulkWireTests
{
    private const BindingFlags HiddenInstance = BindingFlags.Instance | BindingFlags.NonPublic;
    private static object? Field(object owner, string name) => owner.GetType().GetField(name, HiddenInstance)!.GetValue(owner);
    public static async Task BinaryBudgetSeparation()
    {
        var (_, log) = await Run("large-wire"); var fields = log.Terminal!;
        Check(Convert.ToInt64(fields["transport_stdout_binary_bytes"]) == 129L * (65536 + 80) &&
            Convert.ToInt64(fields["transport_stdout_binary_frames"]) == 129 && Convert.ToInt64(fields["transport_stdout_json_bytes"]) < 1048576,
            "complete binary wire bytes/frames must remain independent of JSON business budget");
        Check(Convert.ToInt64(fields["transport_payload_buffers_high_water"]) == 1 && Convert.ToInt64(fields["transport_payload_buffer_bytes_high_water"]) == 65536,
            "large transfer must not grow the transport window");
    }
    public static async Task ReadRequestRefusals()
    {
        foreach (var mode in ReadMalformed.Concat(["source-wrong-token"]))
        {
            var (_, log) = await Run(mode);
            Check(Convert.ToInt64(log.Terminal!["source_payload_read_calls"]) == 0 && Convert.ToInt64(log.Terminal["transport_stdin_binary_frames"]) == 0,
                "refused read consumed source payload or returned binary success");
        }
    }
    public static async Task OrderedAcknowledgement()
    {
        using var release = new ManualResetEventSlim();
        var entered = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        Task? inspection = null; var written = 0; var held = 0;
        try
        {
            await Run("ordered", (root, runner, id) => {
                runner.OutputWriteCheckpointProbe = _ => Volatile.Write(ref written, 1);
                runner.ControlInputCheckpointProbe = op => {
                    if (op == "native_result" && Volatile.Read(ref written) == 1 && Interlocked.Exchange(ref held, 1) == 0)
                    { entered.TrySetResult(true); Check(release.Wait(8000), "ordered acknowledgement release deadline"); }
                };
                inspection = Task.Run(async () => {
                    try
                    {
                        await entered.Task.WaitAsync(TimeSpan.FromSeconds(8));
                        var marker = Path.Combine(root, "commit-sent");
                        var deadline = DateTime.UtcNow.AddSeconds(5);
                        while (!File.Exists(marker) && DateTime.UtcNow < deadline) await Task.Delay(10);
                        Check(File.Exists(marker), "peer did not send commit behind data acknowledgement");
                        Check(runner.InspectOutputCommit(id).FilesCommitted == 0, "commit overtook unconfirmed data");
                        var active = (System.Collections.IDictionary)Field(runner, "active")!;
                        object run; lock (Field(runner, "gate")!) run = active[id]!;
                        Check(Field(run, "BulkPending") is not null && (ulong)Field(run, "NextBulkSequence")! == 2,
                            "one exact operation-wide rendezvous must remain pending");
                    }
                    finally { release.Set(); }
                });
            });
            await inspection!;
        }
        finally { release.Set(); }
    }
    public static async Task QueueCancellation()
    {
        // Fill the ORIGINAL capacity-four channel without a writer, then cancel its blocked
        // payload WriteAsync and run the ORIGINAL cancelled input pump to observe final drain.
        var runType = typeof(ArchiveWorkerRunner).GetNestedType("Run", BindingFlags.NonPublic)!;
        var pendingType = typeof(ArchiveWorkerRunner).GetNestedType("PendingBulk", BindingFlags.NonPublic)!;
        var request = new ArchiveWorkerRequest(Guid.NewGuid(), "create_gzip", "controlled", 65001, new { });
        var json = JsonSerializer.SerializeToUtf8Bytes(new { bulk_transfer = 1 });
        var initial = BitConverter.GetBytes(json.Length).Concat(json).ToArray();
        var run = Activator.CreateInstance(runType, HiddenInstance, null, [request, ArchiveWorkerBudget.DefaultProcess, initial], null)!;
        var channel = (Channel<byte[]>)Field(run, "Input")!;
        var queued = new List<byte[]> { initial };
        for (var index = 0; index < 3; index++)
        {
            var control = Enumerable.Repeat((byte)0xA5, 17).ToArray(); queued.Add(control);
            Check(channel.Writer.TryWrite(control), "original input capacity changed");
        }
        Check(!channel.Writer.TryWrite([1]), "original input capacity exceeds four");
        var binding = new ArchiveBulkBinding(2, request.OperationId, Guid.NewGuid(), Guid.NewGuid(), 1, 0, 65536);
        var pending = Activator.CreateInstance(pendingType, HiddenInstance | BindingFlags.Public, null, [binding], null)!;
        runType.GetField("BulkPending", HiddenInstance)!.SetValue(run, pending);
        var native = Enumerable.Repeat((byte)0xB6, 65536).ToArray();
        using var reply = new ArchiveBulkReply(binding, native);
        var send = (Task)typeof(ArchiveWorkerRunner).GetMethod("SendBulkResult", BindingFlags.Static | BindingFlags.NonPublic)!.Invoke(null, [run, pending, reply])!;
        var envelope = (byte[])Field(pending, "Envelope")!;
        var stop = (CancellationTokenSource)Field(run, "InputStop")!;
        try
        {
            Check(!send.IsCompleted && envelope.Length == 65616 && native.All(value => value == 0), "one queued copy/native retirement before backpressure");
            Check((int)Field(run, "BulkBufferHighWater")! == 2 && (int)Field(run, "BulkBufferBytesHighWater")! == 131072, "two maximum payload buffers only");
            stop.Cancel();
            var cancelled = false;
            try { await send.WaitAsync(TimeSpan.FromSeconds(3)); } catch (OperationCanceledException) { cancelled = true; }
            Check(cancelled && envelope.All(value => value == 0) && Field(run, "BulkPending") is null, "InputStop must retire blocked channel payload and binding");
            using var runner = Runner(_ => { }, new WireLog()); using var output = new MemoryStream();
            var pump = (Task)typeof(ArchiveWorkerRunner).GetMethod("SendInput", HiddenInstance)!.Invoke(runner, [run, output])!;
            await pump.WaitAsync(TimeSpan.FromSeconds(3));
            Check(queued.All(bytes => bytes.All(value => value == 0)) && !channel.Reader.TryRead(out _), "original finally must zero and drain every control");
        }
        finally
        {
            stop.Cancel(); stop.Dispose();
            ((CancellationTokenSource)Field(run, "MetadataStop")!).Dispose();
            ((CancellationTokenSource)Field(run, "RecycleCancellation")!).Dispose();
        }
    }
}
