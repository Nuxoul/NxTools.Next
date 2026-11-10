using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveExtractionPipelineTests
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal IReadOnlyDictionary<string, object?>? Terminal;
        public void Write(string level, string name, Guid? id = null, IReadOnlyDictionary<string, object?>? fields = null)
        { if (name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled") Terminal = fields; }
    }
    public static async Task OrderedWritesAndClose()
    {
        foreach (var mode in new[] { "success", "fail", "fail_eof" })
        {
            var fail = mode != "success";
            var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-direct-wire-" + Guid.NewGuid());
            Directory.CreateDirectory(root);
            var source = Path.Combine(root, mode + ".zip");
            using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) { using var stream = zip.CreateEntry("source.txt").Open(); stream.WriteByte(1); }
            var output = Path.Combine(root, "output");
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
            using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
                null, [Environment.ProcessPath!, "--archive-direct-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, log, null], null)!;
            if (fail) {
                var writes = 0;
                runner.OutputWriteCheckpointProbe = _ => {
                    if (++writes == 4)
                        throw new IOException("owned disk-full fixture", unchecked((int)0x80070070));
                };
            }
            try
            {
                Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001, new { max_entries = 100 }, OutputRoot: output, TimeoutMs: 10000)), "direct pipeline admission");
                Check(await runner.DrainAsync(15000), "original process/pipe/Job drain");
                var terminal = events.Single(item => item.Kind == "exited");
                Check(terminal.CleanupComplete, "cleanup must finish");
                Check(terminal.FilesCommitted == (fail ? 1 : 3), $"only successfully closed files count: {mode}/{terminal.FilesCommitted}/{terminal.Category}/{terminal.ExitCode}");
                Check(terminal.Category == (fail ? "write_file" : "success"), "first native write failure: " + terminal.Category);
                if (fail) Check(terminal.NativeErrorCode == 112, "first native disk-full code survives close");
                if (mode == "fail") Check(events.Any(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure"), "peer confirmed close returned original native error");
                var completed = Path.Combine(output, "file0.bin");
                Check(File.ReadAllBytes(completed).SequenceEqual(Enumerable.Repeat((byte)0x51, 65536).Concat(new byte[] { 7, 8, 9 })), "independent complete bytes");
                Check(!Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "ordinary extraction never creates partial siblings");
                if (fail) Check(!File.Exists(Path.Combine(output, "file1.bin")), "failed current file is deleted; completed file remains");
                else
                {
                    Check(Convert.ToInt64(log.Terminal!["transport_stdin_json_frames"]) == 8, "start + source + two synchronous controls per file, zero write ACKs");
                    Check(Convert.ToInt64(log.Terminal["transport_stdout_binary_frames"]) == 9, "three ordered writes per file");
                    Check(Convert.ToInt64(log.Terminal["output_write_ack_frames"]) == 0, "pipelined writes have no acknowledgement frames");
                }
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            }
            finally { runner.Stop(); Check(await runner.DrainAsync(15000), "finally drain"); Directory.Delete(root, true); }
        }
    }
    internal static async Task<int> Peer()
    {
        var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
        var start = await Receive(input); var id = start.GetProperty("operation_id").GetGuid();
        var mode = Path.GetFileNameWithoutExtension(start.GetProperty("path").GetString());
        var fail = mode != "success";
        await Send(output, new { schema = 1, operation_id = id, op = "accepted", capabilities = new[] {
            "source_lease_v1", "budget_reservations_v1", "extraction_metadata_v1", "motw_v1", "zip_risk_v1", "binary_bulk_v1", "extraction_output_v1" } });
        async Task<JsonElement> Call(string action, string? path = null, Guid? token = null, object? metadata = null)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = call, action, path, token, metadata } });
            var reply = await Receive(input);
            Check(reply.GetProperty("call_id").GetGuid() == call, "received a per-write ACK instead of the close result");
            return reply;
        }
        Check((await Call("source_acquire")).GetProperty("success").GetBoolean(), "held source");
        ulong sequence = 0;
        for (var index = 0; index < 3; index++)
        {
            var token = (await Call("begin", "file" + index + ".bin")).GetProperty("token").GetGuid();
            ulong offset = 0;
            foreach (var data in new[] { Enumerable.Repeat((byte)0x51, 32768).ToArray(), Enumerable.Repeat((byte)0x51, 32768).ToArray(), new byte[] { 7, 8, 9 } })
            {
                await output.WriteAsync(Binary(id, token, ++sequence, offset, data));
                offset += (ulong)data.Length;
            }
            if (mode == "fail_eof" && index == 1) return 0;
            var closed = await Call("commit", token: token, metadata: new { modified = (object?)null, attributes = (object?)null });
            if (fail && index == 1)
            {
                Check(!closed.GetProperty("success").GetBoolean() && closed.GetProperty("native_error_code").GetInt32() == 112, "close aggregates first native failure");
                await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = "IoFailure" });
                return 0;
            }
            Check(closed.GetProperty("success").GetBoolean(), "successful drained close");
        }
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 3, bytes_verified = 196617 });
        return 0;
    }
    private static byte[] Binary(Guid operation, Guid token, ulong sequence, ulong offset, byte[] payload)
    {
        var result = new byte[80 + payload.Length]; var body = result.AsSpan(4);
        BinaryPrimitives.WriteUInt32LittleEndian(result, 0x80000000 | (uint)(76 + payload.Length));
        body[0] = 1; body[1] = 1;
        operation.TryWriteBytes(body[4..20], bigEndian: true, out _);
        Guid.NewGuid().TryWriteBytes(body[20..36], bigEndian: true, out _);
        token.TryWriteBytes(body[36..52], bigEndian: true, out _);
        BinaryPrimitives.WriteUInt64LittleEndian(body[52..60], sequence);
        BinaryPrimitives.WriteUInt64LittleEndian(body[60..68], offset);
        BinaryPrimitives.WriteUInt32LittleEndian(body[68..72], (uint)payload.Length);
        BinaryPrimitives.WriteUInt32LittleEndian(body[72..76], (uint)payload.Length);
        payload.CopyTo(body[76..]); return result;
    }
    private static async Task Send(Stream stream, object value)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(value);
        await stream.WriteAsync(BitConverter.GetBytes(bytes.Length)); await stream.WriteAsync(bytes); await stream.FlushAsync();
    }
    private static async Task<JsonElement> Receive(Stream stream)
    {
        var prefix = new byte[4]; await stream.ReadExactlyAsync(prefix);
        var bytes = new byte[BinaryPrimitives.ReadUInt32LittleEndian(prefix)]; await stream.ReadExactlyAsync(bytes);
        using var document = JsonDocument.Parse(bytes); return document.RootElement.Clone();
    }
}
