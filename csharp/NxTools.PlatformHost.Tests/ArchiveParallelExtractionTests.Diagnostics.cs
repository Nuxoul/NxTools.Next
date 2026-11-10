using System.Collections.Concurrent;
using System.IO.Compression;
using System.Reflection;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveParallelExtractionTests
{
    public static async Task PendingObservationExcludesCompletedCalls()
    {
        var owned = Directory.CreateTempSubdirectory("archive-parallel-pending-");
        var source = Path.Combine(owned.FullName, "sequential.zip");
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
            for (var index = 0; index < 16; index++) zip.CreateEntry($"file{index}.bin");
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new TerminalLog();
        using var runner = (ArchiveWorkerRunner)Activator.CreateInstance(typeof(ArchiveWorkerRunner), BindingFlags.Instance | BindingFlags.NonPublic,
            null, [Environment.ProcessPath!, "--archive-parallel-begin-peer", (Action<ArchiveWorkerEvent>)events.Enqueue, log, null], null)!;
        try
        {
            Check(runner.Start(new(Guid.NewGuid(), "extract", source, 65001,
                new { max_entries = 100, max_working_bytes = 64UL << 20, max_metadata_bytes = 4UL << 20 },
                OutputRoot: Path.Combine(owned.FullName, "output"), TimeoutMs: 20000, ParallelExtraction: true)), "sequential control fixture admission");
            var terminal = await ActualTerminal(runner, events);
            Check(terminal.Category == "success" && terminal.FilesCommitted == 16, "sequential control fixture prerequisites");
            Check(Convert.ToInt32(log.Fields!["native_pending_high_water"]) is >= 1 and < 8,
                "completed native calls must not accumulate into concurrency high-water: " + log.Fields["native_pending_high_water"]);
            ActualSourceReleased(source);
        }
        finally { runner.Stop(); Check(await runner.DrainAsync(20000), "pending observation final drain"); owned.Delete(true); }
    }

    private static async Task<int> PeerSequential(Stream input, Stream output, Guid id)
    {
        async Task<JsonElement> Call(string action, string? path = null, Guid? token = null)
        {
            var call = Guid.NewGuid();
            await Send(output, new { schema = 1, operation_id = id, op = "native_call", native = new {
                call_id = call, action, path, token, metadata = action == "commit" ? new { modified = (object?)null, attributes = (object?)null } : null } });
            var reply = await Receive(input);
            Check(reply.GetProperty("call_id").GetGuid() == call && reply.GetProperty("success").GetBoolean(), "sequential bound native reply");
            return reply;
        }
        for (var index = 0; index < 16; index++)
        {
            var token = (await Call("begin", $"file{index}.bin")).GetProperty("token").GetGuid();
            await Call("commit", token: token);
        }
        await Send(output, new { schema = 1, operation_id = id, op = "complete", files_verified = 16, bytes_verified = 0 });
        return 0;
    }
}
