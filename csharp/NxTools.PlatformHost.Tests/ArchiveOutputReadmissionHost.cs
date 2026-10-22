using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Fixture adapter only: performs each supplied attempt with the ORIGINAL runner. No retry policy.
internal static class ArchiveOutputReadmissionHost
{
    internal static async Task<int> RunAsync(string outputRoot)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) return 2;
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(helper)) return 2;
        File.Copy(worker, helper);
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        var input = Console.OpenStandardInput(); var output = Console.OpenStandardOutput();
        var attempts = 0;
        try {
            while (await Read(input) is { } request) {
                while (events.TryDequeue(out _)) { }
                var id = request.GetProperty("operation_id").GetGuid();
                var source = request.GetProperty("path").GetString()!;
                if (runner.ActiveCount != 0 || ++attempts > 2) return 3;
                var limits = request.GetProperty("limits").Clone();
                var password = request.TryGetProperty("password", out var p) && p.ValueKind != JsonValueKind.Null ? p.GetString() : null;
                var policy = request.TryGetProperty("conflict_policy", out var cp) ? cp.GetString() : null;
                var copyMotw = !request.TryGetProperty("extract_options", out var options) || options.ValueKind == JsonValueKind.Null || options.GetProperty("copy_motw").GetBoolean();
                var privateOptions = request.GetProperty("output_readmission").Clone();
                var command = new ArchiveWorkerRequest(id, "extract", source, request.GetProperty("codepage").GetInt32(), limits,
                    Password: password, OutputRoot: outputRoot, ExtractOptions: new(copyMotw), ConflictPolicy: policy, OutputReadmission: privateOptions);
                if (!runner.Start(command)) return 4;
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited")) {
                    if (clock.ElapsedMilliseconds > 15000) { runner.Cancel(id); return 5; }
                    await Task.Delay(10);
                }
                if (!await runner.DrainAsync()) return 6;
                var terminal = events.Single(e => e.Kind == "exited");
                var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                var shortfall = frames.FirstOrDefault(f => f.GetProperty("op").GetString() == "output_budget_shortfall");
                var failure = frames.FirstOrDefault(f => f.GetProperty("op").GetString() == "failure");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                using (File.Open(helper, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                var result = new { operation_id = id, cleanup_complete = terminal.CleanupComplete,
                    files_committed = terminal.FilesCommitted,
                    error_code = failure.ValueKind == JsonValueKind.Object ? failure.GetProperty("error_code").GetString() : terminal.Category == "success" ? null : terminal.Category,
                    shortfall = shortfall.ValueKind == JsonValueKind.Object ? (JsonElement?)shortfall.GetProperty("output_budget_shortfall") : null,
                    audit = new { attempt = attempts, started = events.Count(e => e.Kind == "started"), active = runner.ActiveCount,
                        source_released = true, helper_released = true, output_exists = Directory.Exists(outputRoot),
                        category = terminal.Category, source_acquired = frames.Any(f => f.GetProperty("op").GetString() == "accepted"),
                        codepage = command.Codepage, conflict_policy = command.ConflictPolicy, copy_motw = copyMotw,
                        memory_budget = ArchiveWorkerBudget.Approve(command).Memory } };
                await Write(output, result);
            }
            return 0;
        } finally { runner.Stop(); await runner.DrainAsync(); File.Delete(helper); }
    }
    private static async Task<JsonElement?> Read(Stream stream) {
        var header = new byte[4]; var first = await stream.ReadAsync(header.AsMemory(0, 1));
        if (first == 0) return null;
        await stream.ReadExactlyAsync(header.AsMemory(1)); var size = BinaryPrimitives.ReadUInt32LittleEndian(header);
        if (size is 0 or > 1048576) throw new InvalidDataException();
        var bytes = new byte[size]; await stream.ReadExactlyAsync(bytes);
        using var doc = JsonDocument.Parse(bytes); return doc.RootElement.Clone();
    }
    private static async Task Write(Stream stream, object value) {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(value); var header = new byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(header, (uint)bytes.Length);
        await stream.WriteAsync(header); await stream.WriteAsync(bytes); await stream.FlushAsync();
    }
}
