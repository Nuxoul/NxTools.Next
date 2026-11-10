using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    private static void ObservePeak(ref int peak, int current)
    {
        int observed;
        while ((observed = Volatile.Read(ref peak)) < current &&
            Interlocked.CompareExchange(ref peak, current, observed) != observed) { }
    }

    public static async Task NativeBeginsOverlap()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-parallel-begin-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var source = Path.Combine(root, "source.zip");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
        {
            foreach (var name in new[] { "first.bin", "second.bin" })
                using (var entry = zip.CreateEntry(name).Open()) entry.WriteByte(1);
        }
        var destination = Path.Combine(root, "output");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, null, null], null)!;
        using var secondEntered = new ManualResetEventSlim();
        var active = 0; var highWater = 0; var firstTimedOut = 0; var began = 0;
        runner.OutputBeginCheckpointProbe = relative =>
        {
            var current = Interlocked.Increment(ref active);
            ObservePeak(ref highWater, current);
            Interlocked.Increment(ref began);
            try
            {
                if (relative == "first.bin")
                {
                    if (!secondEntered.Wait(TimeSpan.FromSeconds(5))) Interlocked.Exchange(ref firstTimedOut, 1);
                }
                else if (relative == "second.bin") secondEntered.Set();
            }
            finally { Interlocked.Decrement(ref active); }
        };
        try
        {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: destination, TimeoutMs: 12000, ParallelExtraction: true)), "ordinary ZIP worker admission");
            Check(await runner.DrainAsync(20000), "original process/pipe/Job did not drain");
            var terminal = events.Single(item => item.Kind == "exited");
            Check(terminal.CleanupComplete && runner.ActiveCount == 0 && terminal.Category == "success" && terminal.FilesCommitted == 2,
                $"ordinary source/output prerequisites failed: {terminal.Category}/{terminal.FilesCommitted}/{terminal.CleanupComplete}");
            Check(File.ReadAllBytes(Path.Combine(destination, "first.bin")).SequenceEqual(new byte[] { 1, 2, 3 }) &&
                File.ReadAllBytes(Path.Combine(destination, "second.bin")).SequenceEqual(new byte[] { 4, 5, 6 }), "independent final-name output bytes");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            Check(!Directory.EnumerateFiles(destination, ".nxpart-*", SearchOption.AllDirectories).Any(), "ordinary final names have no partial siblings");
            Check(began == 2, $"expected two native begin entries, observed {began}");
            Check(firstTimedOut == 0 && highWater == 2,
                $"ordinary native begin overlap missing: first_wait_timed_out={firstTimedOut}, native_begin_high_water={highWater}");
        }
        finally
        {
            runner.Stop();
            Check(await runner.DrainAsync(20000), "finally original Job cleanup");
            Directory.Delete(root, true);
        }
    }

    internal static async Task<int> Peer()
    {
        var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
        var start = await Receive(input); var id = start.GetProperty("operation_id").GetGuid();
        Check(start.GetProperty("schema").GetInt32() == 2 && start.GetProperty("bulk_transfer").GetInt32() == 1 &&
            start.GetProperty("parallel_extraction").GetBoolean(), "ordinary parallel bulk start");
        var capabilities = new List<string> { "source_lease_v1", "budget_reservations_v1", "extraction_metadata_v1", "motw_v1",
            "zip_risk_v1", "binary_bulk_v1", "extraction_output_v1" };
        var mode = Path.GetFileNameWithoutExtension(start.GetProperty("path").GetString());
        if (mode != "missing-cap") capabilities.Add("parallel_extraction_v1");
        await Send(output, new { schema = 1, operation_id = id, op = "accepted", capabilities });
        if (mode == "missing-cap") return 0;
        async Task<JsonElement> Call(string action, string? path = null, Guid? token = null)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = call, action, path, token, metadata = action == "commit" ? new { modified = (object?)null, attributes = (object?)null } : null } });
            var reply = await Receive(input);
            Check(reply.GetProperty("call_id").GetGuid() == call, "native reply call binding");
            return reply;
        }
        Check((await Call("source_acquire")).GetProperty("success").GetBoolean(), "held original source");
        if (mode == "cap") return await PeerCap(input, output, id);
        if (mode == "sequential") return await PeerSequential(input, output, id);
        if (mode is "cancel" or "first-error" or "cancel-first-error" or "cancel-worker-error") return await PeerLifecycle(input, output, id, mode);
        if (mode == "close-overlap") return await PeerClose(input, output, id);
        if (mode == "late-payload") return await PeerLatePayload(input, output, id);
        var firstCall = Guid.NewGuid(); var secondCall = Guid.NewGuid();
        foreach (var (call, path) in new[] { (firstCall, "first.bin"), (secondCall, "second.bin") })
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new { call_id = call, action = "begin", path } });
        var tokens = new Dictionary<Guid, Guid>();
        for (var index = 0; index < 2; index++)
        {
            var reply = await Receive(input);
            Check(reply.GetProperty("success").GetBoolean(), "native begin reply");
            var call = reply.GetProperty("call_id").GetGuid();
            Check((call == firstCall || call == secondCall) && !tokens.ContainsKey(call), "distinct begin reply routing");
            tokens.Add(call, reply.GetProperty("token").GetGuid());
        }
        ulong sequence = 0;
        foreach (var (call, payload) in new[] { (firstCall, new byte[] { 1, 2, 3 }), (secondCall, new byte[] { 4, 5, 6 }) })
        {
            var token = tokens[call];
            await output.WriteAsync(Binary(id, token, ++sequence, payload));
            Check((await Call("commit", token: token)).GetProperty("success").GetBoolean(), "native close after payload drain");
        }
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 2, bytes_verified = 6 });
        return 0;
    }

    private static byte[] Binary(Guid operation, Guid token, ulong sequence, byte[] payload, ulong offset = 0)
    {
        var bytes = new byte[80 + payload.Length]; var body = bytes.AsSpan(4);
        BinaryPrimitives.WriteUInt32LittleEndian(bytes, 0x80000000 | (uint)(76 + payload.Length));
        body[0] = 1; body[1] = 1;
        operation.TryWriteBytes(body[4..20], bigEndian: true, out _);
        Guid.NewGuid().TryWriteBytes(body[20..36], bigEndian: true, out _);
        token.TryWriteBytes(body[36..52], bigEndian: true, out _);
        BinaryPrimitives.WriteUInt64LittleEndian(body[52..60], sequence);
        BinaryPrimitives.WriteUInt64LittleEndian(body[60..68], offset);
        BinaryPrimitives.WriteUInt32LittleEndian(body[68..72], (uint)payload.Length);
        BinaryPrimitives.WriteUInt32LittleEndian(body[72..76], (uint)payload.Length);
        payload.CopyTo(body[76..]); return bytes;
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
