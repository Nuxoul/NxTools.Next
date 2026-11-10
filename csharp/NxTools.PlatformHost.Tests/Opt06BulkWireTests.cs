using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Original Runner/owned Job and native handles; peer bytes are independent of the product codec.
internal static partial class Opt06BulkWireTests
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    private sealed class WireLog(bool throws = false) : IPlatformLog
    {
        internal IReadOnlyDictionary<string, object?>? Terminal;
        public void Write(string level, string name, Guid? id = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (name is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed") Terminal = fields;
            if (throws) throw new InvalidOperationException("controlled sink");
        }
    }
    private static ArchiveWorkerRunner Runner(Action<ArchiveWorkerEvent> publish, IPlatformLog log) =>
        (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--opt06-bulk-peer", publish, log, null], null)!;
    private static readonly string[] Malformed = ["version", "reserved", "direction", "operation", "empty-call", "empty-token", "zero-sequence", "future-sequence", "length", "actual", "oversize", "undersize", "truncated", "legacy-write", "repeat-sequence", "old-sequence", "duplicate-call"];
    private static readonly string[] ReadMalformed = ["source-stale", "source-future", "source-zero-length", "source-big-length", "source-empty-token"];
    private static async Task<(ArchiveWorkerEvent Terminal, WireLog Log)> Run(string mode, Action<string, ArchiveWorkerRunner, Guid>? configure = null, bool throwingLog = false)
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "opt06-wire-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, mode + ".bin"); var output = Path.Combine(root, "missing", "output");
        File.WriteAllBytes(source, mode == "source-cancel" ? new byte[65536] : [7, 8, 9]);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new WireLog(throwingLog);
        using var runner = Runner(events.Enqueue, log);
        var id = Guid.NewGuid();
        try
        {
            configure?.Invoke(root, runner, id);
            Check(runner.Start(new(id, mode.StartsWith("source-", StringComparison.Ordinal) ? "create_zip" : "create_gzip", source, 65001,
                new { max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: output, CreateName: "result.gz", CompressionLevel: "fast", TimeoutMs: 5000)), "modern admission");
            Check(await runner.DrainAsync(15000), "actual Run/pumps/Job did not drain");
            var terminal = events.Single(e => e.Kind == "exited");
            Check(terminal.CleanupComplete && runner.ActiveCount == 0, "actual native cleanup required");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            Check(events.All(e => e.Frame is not { } frame || !frame.TryGetProperty("native", out _)), "native payload escaped business frames");
            if (mode is "missing" or "duplicate-cap" or "malformed-cap" or "cap-object" or "duplicate-accepted" or "no-accepted")
            {
                Check(terminal.Category == "worker_capability_mismatch", "required binary capability was accepted: " + terminal.Category);
                Check(!Directory.Exists(Path.Combine(root, "missing")), "capability refusal mutated output namespace");
            }
            else if (Malformed.Contains(mode) || ReadMalformed.Contains(mode))
            {
                Check(terminal.Category == (mode is "oversize" or "undersize" ? "stdout_frame_limit" : "stdout_invalid_frame"), "malformed refusal " + mode + ": " + terminal.Category);
                Check(terminal.FilesCommitted == 0 && !File.Exists(Path.Combine(output, "result.gz")), "malformed data reached publication");
            }
            else if (mode is "wrong-token" or "wrong-offset" or "source-wrong-token")
            {
                Check(terminal.Category == "worker_failure" && terminal.FilesCommitted == 0, "native typed data refusal " + terminal.Category);
            }
            else if (mode == "source-cancel")
            {
                Check(terminal.Category == "cancelled" && terminal.FilesCommitted == 0, "blocked input cancellation " + terminal.Category);
            }
            else if (mode == "write-failure")
            {
                Check(terminal.Category == "write_file" && terminal.NativeErrorCode == 112 && terminal.FilesCommitted == 0, "first native failure lost: " + terminal.Category);
            }
            else if (mode == "commit-failure")
            {
                Check(terminal.Category == "target_denied" && terminal.NativeErrorCode == 5 && terminal.FilesCommitted == 0, "commit failure lost: " + terminal.Category);
            }
            else if (mode == "large-wire")
            {
                Check(terminal.Category == "success" && terminal.FilesCommitted == 1, "binary bytes charged to old JSON limit: " + terminal.Category);
                using var decoded = File.OpenRead(Path.Combine(output, "result.gz")); var buffer = new byte[65536];
                Check(decoded.Length == 129L * 65536, "large bounded output length");
                int count; while ((count = decoded.Read(buffer)) != 0) Check(buffer.AsSpan(0, count).IndexOfAnyExcept((byte)0x5A) < 0, "independent native output bytes");
            }
            else
            {
                Check(terminal.Category is "success" or "cancelled_with_committed_output" && terminal.FilesCommitted == 1,
                    "raw frame rejected by original Runner: " + terminal.Category);
                Check(File.ReadAllBytes(Path.Combine(output, "result.gz")).SequenceEqual(new byte[] { 1, 2, 3 }), "native binary output differs");
                using (File.Open(Path.Combine(output, "result.gz"), FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            }
            Check(!Directory.Exists(output) || !Directory.EnumerateFiles(output).Any(path => Path.GetFileName(path) != "result.gz"), "owned partial not cleaned");
            return (terminal, log);
        }
        finally { runner.Stop(); Check(await runner.DrainAsync(15000), "finally drain"); Directory.Delete(root, true); }
    }
    public static async Task RawRunner()
    {
        var (_, log) = await Run("raw"); var fields = log.Terminal!;
        Check(Convert.ToInt64(fields["transport_stdout_binary_bytes"]) == 83 && Convert.ToInt64(fields["transport_stdout_binary_frames"]) == 1, "actual binary stdout accounting");
        Check(Convert.ToInt64(fields["payload_conversion_input_bytes"]) == 0 && Convert.ToInt64(fields["transport_stdout_json_frames"]) > 0, "zero binary conversion and JSON frames");
        Check(Convert.ToInt64(fields["transport_payload_buffers_high_water"]) == 1, "raw write buffer bound");
    }
    public static async Task Capabilities()
    {
        foreach (var mode in new[] { "missing", "duplicate-cap", "malformed-cap", "cap-object", "duplicate-accepted", "no-accepted" }) await Run(mode);
    }
    public static async Task MalformedRunner()
    {
        foreach (var mode in Malformed.Concat(["wrong-token", "wrong-offset"])) await Run(mode);
    }
    public static async Task NativeReads()
    {
        foreach (var mode in new[] { "partial", "source-short", "source-error", "split-io" })
        {
            var (_, log) = await Run(mode); var fields = log.Terminal!;
            Check(Convert.ToInt64(fields["payload_conversion_input_bytes"]) == 0, "binary path converted payload");
            if (mode is "partial" or "source-short" or "source-error")
            {
                Check(Convert.ToInt64(fields["transport_stdin_binary_frames"]) == 2, "short read/EOF must each produce one binary reply");
                Check(Convert.ToInt64(fields["transport_payload_buffers_high_water"]) == 2 && Convert.ToInt64(fields["transport_payload_buffer_bytes_high_water"]) == 6, "native + queued envelope bound");
            }
            if (mode.StartsWith("source-", StringComparison.Ordinal))
                Check(Convert.ToInt64(fields["source_payload_read_calls"]) == 2 && Convert.ToInt64(fields["source_payload_read_bytes"]) == 3, "original native source counters");
        }
    }
    public static async Task FailureAndPublication()
    {
        await Run("raw", throwingLog: true);
        await Run("write-failure", (_, runner, _) => runner.OutputWriteCheckpointProbe = _ => throw new ArchiveOutputException("write_file", 112));
        await Run("commit-failure", (_, runner, _) => runner.OutputCommitCheckpointProbe = stage => { if (stage == "before_flush") throw new ArchiveOutputException("target_denied", 5); });
        await Run("late-cancel", (_, runner, id) => runner.OutputCommitCheckpointProbe = stage => { if (stage == "native_published") runner.Cancel(id); });
    }
    public static async Task CancelBackpressure()
    {
        Task? cancellation = null; byte[]? ownedEnvelope = null; object? originalRun = null;
        await Run("source-cancel", (_, runner, id) => {
            cancellation = Task.Run(async () => {
                var deadline = DateTime.UtcNow.AddSeconds(8);
                while (DateTime.UtcNow < deadline)
                {
                    var active = (System.Collections.IDictionary)typeof(ArchiveWorkerRunner).GetField("active", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(runner)!;
                    object? run;
                    lock (typeof(ArchiveWorkerRunner).GetField("gate", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(runner)!) run = active[id];
                    var pending = run?.GetType().GetField("BulkPending", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(run);
                    var envelope = (byte[]?)pending?.GetType().GetField("Envelope", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(pending);
                    if (envelope?.Length == 65616)
                    {
                        ownedEnvelope = envelope; originalRun = run;
                        var first = JsonSerializer.SerializeToElement(new { schema = 1, operation_id = id, op = "write", payload = "control-one" });
                        var second = JsonSerializer.SerializeToElement(new { schema = 1, operation_id = id, op = "write", payload = "control-two" });
                        Check(runner.Write(id, first) && runner.Write(id, second), "bounded controls must remain admitted during payload backpressure");
                        Check(runner.Cancel(id), "cancel pending payload"); return;
                    }
                    await Task.Delay(10);
                }
                throw new InvalidOperationException("did not observe original pending binary reply");
            });
        });
        await cancellation!;
        Check(ownedEnvelope is not null && ownedEnvelope.All(value => value == 0), "blocked/queued envelope not zeroed");
        Check(originalRun!.GetType().GetField("BulkPending", BindingFlags.Instance | BindingFlags.NonPublic)!.GetValue(originalRun) is null, "pending binding survived actual cleanup");
    }
    public static Task LiteralContract()
    {
        var op = Guid.Parse("00112233-4455-6677-8899-aabbccddeeff");
        var call = Guid.Parse("10213243-5465-7687-98a9-bacbdcedfe0f");
        var token = Guid.Parse("ffeeddcc-bbaa-9988-7766-554433221100");
        var literal = Convert.FromHexString("4F0000800101000000112233445566778899AABBCCDDEEFF102132435465768798A9BACBDCEDFE0FFFEEDDCCBBAA99887766554433221100010000000000000008070605040302010300000003000000010203");
        Check(literal.SequenceEqual(Literal(op, call, token, 1, 0x0102030405060708, 3, [1, 2, 3])), "independent complete byte literal");
        var parsed = ArchiveBulkFrame.Parse(literal.AsSpan(4), 1);
        Check(parsed == new ArchiveBulkBinding(1, op, call, token, 1, 0x0102030405060708, 3), "network UUID and LE integer parse");
        foreach (var kind in new byte[] { 2, 3 })
        foreach (var bytes in new byte[][] { [], [9], [1, 2, 3] })
        {
            using var reply = new ArchiveBulkReply(parsed with { Kind = kind }, bytes);
            var wire = ArchiveBulkFrame.Pack(reply);
            Check(wire.SequenceEqual(Literal(op, call, token, 1, parsed.Offset, 3, bytes, kind)), "independent read header/short EOF encoding");
            Check(ArchiveBulkFrame.Parse(wire.AsSpan(4), kind) == reply.Binding, "read reply parse");
        }
        Check(!ArchiveBulkFrame.ValidBodyLength(75) && ArchiveBulkFrame.ValidBodyLength(76) && ArchiveBulkFrame.ValidBodyLength(65612) && !ArchiveBulkFrame.ValidBodyLength(65613), "preallocation bounds");
        Check(ArchiveWorkerBudget.BulkTransportReservation <= ArchiveWorkerBudget.Overhead, "fixed IPC reservation inside original overhead");
        return Task.CompletedTask;
    }

    internal static async Task<int> Peer()
    {
        var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
        var start = await Receive(input); var id = start.GetProperty("operation_id").GetGuid();
        var mode = Path.GetFileNameWithoutExtension(start.GetProperty("path").GetString()!);
        Check(start.GetProperty("schema").GetInt32() == 2 && start.GetProperty("bulk_transfer").GetInt32() == 1, "explicit modern bulk start");
        var caps = new List<object> { "source_lease_v1", "budget_reservations_v1", "partial_read_v1", "source_tree_v1", "zip_create_v1" };
        if (mode != "missing") caps.Add("binary_bulk_v1");
        if (mode == "duplicate-cap") caps.Add("binary_bulk_v1");
        if (mode == "malformed-cap") caps.Add(17);
        object capability = mode == "cap-object" ? new { binary_bulk_v1 = true } : caps;
        var accepted = new { schema = 1, operation_id = id, op = "accepted", capabilities = capability };
        if (mode != "no-accepted") await Send(output, accepted);
        if (mode == "duplicate-accepted") await Send(output, accepted);
        if (mode is "missing" or "duplicate-cap" or "malformed-cap" or "cap-object" or "duplicate-accepted" or "no-accepted")
        { await Send(output, new { schema = 1, operation_id = id, op = "complete" }); return 0; }
        async Task<JsonElement> Call(string action, Guid? token = null, ulong? sequence = null, ulong? offset = null, uint? length = null)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = call, action, token, sequence, offset, length, path = "result.gz" } });
            if (mode == "ordered" && action == "commit") File.WriteAllText(Path.Combine(Path.GetDirectoryName(start.GetProperty("path").GetString()!)!, "commit-sent"), "sent");
            return await Receive(input);
        }
        async Task Read(string action, Guid token, ulong sequence, ulong offset, uint requested, byte[] expected)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = call, action, token, sequence, offset, length = requested } });
            var raw = await ReceiveWire(input);
            Check(raw.SequenceEqual(Literal(id, call, token, sequence, offset, requested, expected, action == "source_read" ? (byte)2 : (byte)3)), "exact native read reply binding/short EOF");
        }
        Check((await Call("source_acquire")).GetProperty("success").GetBoolean(), "source admission");
        ulong next = 1;
        if (mode.StartsWith("source-", StringComparison.Ordinal))
        {
            var roots = await Call("source_roots"); using var list = JsonDocument.Parse(roots.GetProperty("read_data").GetString()!);
            var source = list.RootElement[0].GetProperty("id").GetGuid();
            if (ReadMalformed.Contains(mode) || mode == "source-wrong-token")
            {
                var requestedToken = mode == "source-empty-token" ? Guid.Empty : mode == "source-wrong-token" ? Guid.NewGuid() : source;
                var sequence = mode == "source-stale" ? 0UL : mode == "source-future" ? 2UL : 1UL;
                var length = mode == "source-zero-length" ? 0U : mode == "source-big-length" ? 65537U : 1U;
                var refused = await Call("source_read", requestedToken, sequence, 0, length);
                Check(!refused.GetProperty("success").GetBoolean() && refused.GetProperty("token").GetGuid() == requestedToken &&
                    refused.GetProperty("sequence").GetUInt64() == sequence && refused.GetProperty("offset").GetUInt64() == 0 && refused.GetProperty("length").GetUInt32() == length,
                    "read refusal must preserve exact binding before any native read");
                await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = "IoFailure" }); return 0;
            }
            Check((await Call("source_open", source)).GetProperty("success").GetBoolean(), "source open");
            if (mode == "source-cancel")
            {
                await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = Guid.NewGuid(), action = "source_read", token = source, sequence = 1, offset = 0, length = 65536 } });
                await Task.Delay(10000); return 0;
            }
            if (mode == "source-error")
            {
                var error = await Call("source_read", source, next++, 4, 1);
                Check(!error.GetProperty("success").GetBoolean() && error.GetProperty("error_code").GetString() == "read_limit" &&
                    error.GetProperty("token").GetGuid() == source && error.GetProperty("sequence").GetUInt64() == 1 && error.GetProperty("offset").GetUInt64() == 4 && error.GetProperty("length").GetUInt32() == 1,
                    "original typed source error with exact bulk binding");
            }
            await Read("source_read", source, next++, 0, 8, [7, 8, 9]);
            await Read("source_read", source, next++, 3, 8, []);
            Check((await Call("source_close", source)).GetProperty("success").GetBoolean(), "source close");
        }
        var begin = await Call("begin"); var token = begin.GetProperty("token").GetGuid(); var writeCall = Guid.NewGuid();
        if (mode == "large-wire")
        {
            var payload = Enumerable.Repeat((byte)0x5A, 65536).ToArray();
            for (var block = 0; block < 129; block++)
            {
                var call = Guid.NewGuid();
                await output.WriteAsync(Literal(id, call, token, next++, (ulong)block * 65536, 65536, payload)); await output.FlushAsync();
                var result = await Receive(input);
                Check(result.GetProperty("success").GetBoolean() && result.GetProperty("call_id").GetGuid() == call && result.GetProperty("sequence").GetUInt64() == next - 1, "bounded write acknowledgement");
            }
            Check((await Call("commit", token)).GetProperty("success").GetBoolean(), "bounded large native publication");
            await Send(output, new { schema = 1, operation_id = id, op = "complete" }); return 0;
        }
        if (mode == "legacy-write")
        {
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = writeCall, action = "write", token, data = new[] { 1, 2, 3 } } });
            await Task.Delay(2000); return 0;
        }
        var wire = Literal(id, writeCall, token, next++, 0, 3, [1, 2, 3]);
        switch (mode)
        {
            case "version": wire[4] = 2; break;
            case "reserved": wire[6] = 1; break;
            case "direction": wire[5] = 2; break;
            case "operation": wire[8] ^= 1; break;
            case "empty-call": wire.AsSpan(24, 16).Clear(); break;
            case "empty-token": wire.AsSpan(40, 16).Clear(); break;
            case "zero-sequence": wire.AsSpan(56, 8).Clear(); break;
            case "future-sequence": BinaryPrimitives.WriteUInt64LittleEndian(wire.AsSpan(56), 2); break;
            case "length": BinaryPrimitives.WriteUInt32LittleEndian(wire.AsSpan(72), 65537); break;
            case "actual": BinaryPrimitives.WriteUInt32LittleEndian(wire.AsSpan(76), 2); break;
            case "oversize": BinaryPrimitives.WriteUInt32LittleEndian(wire, 0x8001004D); break;
            case "undersize": BinaryPrimitives.WriteUInt32LittleEndian(wire, 0x8000004B); break;
            case "truncated": await output.WriteAsync(wire.AsMemory(0, 82)); return 0;
            case "wrong-token": wire = Literal(id, writeCall, Guid.NewGuid(), 1, 0, 3, [1, 2, 3]); break;
            case "wrong-offset": BinaryPrimitives.WriteUInt64LittleEndian(wire.AsSpan(64), 1); break;
        }
        if (mode == "split-io") { foreach (var value in wire) { await output.WriteAsync(new byte[] { value }); await output.FlushAsync(); } }
        else { await output.WriteAsync(wire); await output.FlushAsync(); }
        if (Malformed.Contains(mode) && mode is not ("repeat-sequence" or "old-sequence" or "duplicate-call")) { await Task.Delay(2000); return 0; }
        var ack = await Receive(input);
        Check(ack.GetProperty("token").GetGuid() == new Guid(wire.AsSpan(40, 16), bigEndian: true), "write failure token echo");
        Check(ack.GetProperty("call_id").GetGuid() == writeCall && ack.GetProperty("sequence").GetUInt64() == next - 1 &&
            ack.GetProperty("offset").GetUInt64() == (mode == "wrong-offset" ? 1UL : 0) && ack.GetProperty("length").GetInt32() == 3, "exact write acknowledgement");
        if (mode is "wrong-token" or "wrong-offset" or "write-failure")
        {
            Check(!ack.GetProperty("success").GetBoolean(), "failed native write acknowledged success");
            await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = mode == "write-failure" ? "Cancelled" : "IoFailure" }); return 0;
        }
        Check(ack.GetProperty("success").GetBoolean() && ack.GetProperty("token").GetGuid() == token, "binary native write ack");
        if (mode is "repeat-sequence" or "old-sequence" or "duplicate-call")
        {
            var repeat = Literal(id, mode == "duplicate-call" ? writeCall : Guid.NewGuid(), token, mode == "duplicate-call" ? next : 1, 3, 3, [1, 2, 3]);
            if (mode == "old-sequence")
            {
                await Read("read", token, next++, 0, 3, [1, 2, 3]);
            }
            await output.WriteAsync(repeat); await output.FlushAsync(); await Task.Delay(2000); return 0;
        }
        if (mode == "partial")
        {
            await Read("read", token, next++, 0, 8, [1, 2, 3]);
            await Read("read", token, next++, 3, 8, []);
            var error = await Call("read", token, next++, 4, 1);
            Check(!error.GetProperty("success").GetBoolean() && error.GetProperty("error_code").GetString() == "read_limit" && error.GetProperty("token").GetGuid() == token && error.GetProperty("sequence").GetUInt64() == next - 1, "typed partial error binding");
        }
        var commit = await Call("commit", token);
        if (mode == "commit-failure")
        {
            Check(!commit.GetProperty("success").GetBoolean() && commit.GetProperty("native_error_code").GetInt32() == 5, "original commit native error");
            await Send(output, new { schema = 1, operation_id = id, op = "failure", error_code = "IoFailure" }); return 0;
        }
        Check(commit.GetProperty("success").GetBoolean(), "native publication");
        await Send(output, new { schema = 1, operation_id = id, op = "complete" }); return 0;
    }
    internal static byte[] Literal(Guid operation, Guid call, Guid token, ulong sequence, ulong offset, uint requested, byte[] payload, byte kind = 1)
    {
        var frame = new byte[80 + payload.Length];
        BinaryPrimitives.WriteUInt32LittleEndian(frame, 0x80000000U | (uint)(76 + payload.Length));
        frame[4] = 1; frame[5] = kind;
        Convert.FromHexString(operation.ToString("N")).CopyTo(frame, 8);
        Convert.FromHexString(call.ToString("N")).CopyTo(frame, 24);
        Convert.FromHexString(token.ToString("N")).CopyTo(frame, 40);
        BinaryPrimitives.WriteUInt64LittleEndian(frame.AsSpan(56), sequence);
        BinaryPrimitives.WriteUInt64LittleEndian(frame.AsSpan(64), offset);
        BinaryPrimitives.WriteUInt32LittleEndian(frame.AsSpan(72), requested);
        BinaryPrimitives.WriteUInt32LittleEndian(frame.AsSpan(76), (uint)payload.Length);
        payload.CopyTo(frame, 80); return frame;
    }
    private static async Task Send(Stream stream, object value)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(value);
        await stream.WriteAsync(BitConverter.GetBytes(bytes.Length)); await stream.WriteAsync(bytes); await stream.FlushAsync();
    }
    private static async Task<byte[]> ReceiveWire(Stream stream)
    {
        var header = new byte[4]; await stream.ReadExactlyAsync(header);
        var size = BinaryPrimitives.ReadUInt32LittleEndian(header) & 0x7FFFFFFFU;
        Check(size is > 0 and <= 1048576, "bounded private reply");
        var wire = new byte[4 + size]; header.CopyTo(wire, 0); await stream.ReadExactlyAsync(wire.AsMemory(4)); return wire;
    }
    private static async Task<JsonElement> Receive(Stream stream)
    {
        var wire = await ReceiveWire(stream); Check((wire[3] & 0x80) == 0, "expected typed JSON reply");
        using var doc = JsonDocument.Parse(wire.AsMemory(4)); return doc.RootElement.Clone();
    }
}
