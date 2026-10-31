using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Actual original runner/Job/worker; <=8448actual bytes per ZIP, no large expansion or user files.
internal static class ArchiveRiskTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
            => Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    private static JsonElement[] Prompts(ConcurrentQueue<ArchiveWorkerEvent> events) => events
        .Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "risk_prompt")
        .Select(e => e.Frame!.Value.GetProperty("risk_prompt")).ToArray();
    private static async Task Until(Func<bool> condition, string detail) {
        var clock = Stopwatch.StartNew(); while (!condition()) { Check(clock.ElapsedMilliseconds < 10000, detail); await Task.Delay(10); }
    }
    // Producer is independent .NET ZIP; header mutations affect only the named synthetic copies.
    private static void Produce(string source, string mode)
    {
        using (var zip = ZipFile.Open(source, ZipArchiveMode.Create)) {
            var names = mode == "cancel" ? new[] { "prior.bin", "payload.bin", "later.bin" } : mode == "decline" ? new[] { "payload.bin", "later.bin" } : mode == "total_limit" ? new[] { "prior.bin", "payload.bin" } : mode == "stale" ? new[] { "payload.bin", "second.bin" } : mode == "collision" ? new[] { "payload.bin", "PAYLOAD.BIN" } : new[] { mode == "unsafe" ? "../escape.bin" : "payload.bin" };
            foreach (var name in names) {
                using var stream = zip.CreateEntry(name, CompressionLevel.Optimal).Open();
                stream.Write(new byte[name == "prior.bin" ? 256 : 4096]);
            }
        }
        var bytes = File.ReadAllBytes(source);
        for (var offset = 0; offset + 46 <= bytes.Length; offset++) {
            if (!bytes.AsSpan(offset, 4).SequenceEqual("PK\u0001\u0002"u8)) continue;
            var rawName = System.Text.Encoding.UTF8.GetString(bytes, offset + 46, BitConverter.ToUInt16(bytes, offset + 28));
            if (rawName != "payload.bin") continue;
            var local = (int)BitConverter.ToUInt32(bytes, offset + 42);
            if (mode is "entry_limit" or "total_limit" or "production_pending") {
                var declared = mode == "production_pending" ? (uint)((64 << 20) + 1) : 2048u;
                BitConverter.GetBytes(declared).CopyTo(bytes, local + 22); BitConverter.GetBytes(declared).CopyTo(bytes, offset + 24);
            }
            if (mode == "corrupt") { bytes[local + 14] ^= 1; bytes[offset + 16] ^= 1; }
        }
        File.WriteAllBytes(source, bytes);
    }
#if !DEBUG
    public static Task ActualZipRiskConfirmation() =>
        throw new TestSkippedException("Tiny risk fixture requires the DEBUG-only stricter threshold seam.");
#else
    public static async Task ActualZipRiskConfirmation()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit matching isolated worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Check(!File.Exists(helper), "refuse existing helper");
        var root = Directory.CreateTempSubdirectory("archive-risk-"); var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            ArchivePasswordTicket? previousTask = null;
            foreach (var mode in new[] { "ordinary", "production_pending", "confirm", "decline", "cancel", "stale", "larger_answer", "entry_limit", "total_limit", "declared_limit", "corrupt", "unsafe", "collision" }) {
                var owned = Directory.CreateDirectory(Path.Combine(root.FullName, mode));
                var source = Path.Combine(owned.FullName, "tiny.zip"); Produce(source, mode);
                var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
                var old = Path.Combine(output.FullName, "payload.bin"); File.WriteAllText(old, "original-target");
                var foreign = Path.Combine(output.FullName, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
                if (mode is not ("ordinary" or "production_pending")) runner.RiskThresholdFixture = (1024, 2);
                ulong entryLimit = mode is "entry_limit" or "declared_limit" ? 3072UL : mode == "production_pending" ? 128UL << 20 : 8192UL;
                ulong totalLimit = mode == "total_limit" ? 3072UL : mode == "production_pending" ? 128UL << 20 : 16384UL;
                var id = Guid.NewGuid(); var writes = 0;
                runner.OutputWriteCheckpointProbe = _ => Interlocked.Increment(ref writes);
                var request = new ArchiveWorkerRequest(id, "extract", source, 936, new { max_entries = 100, max_entry_bytes = entryLimit, max_output_bytes = totalLimit }, OutputRoot: output.FullName, ConflictPolicy: "overwrite");
                Check(runner.Start(request), "original ZIP risk admission: " + mode);
                var shouldPrompt = mode is not ("ordinary" or "declared_limit" or "unsafe" or "collision");
                if (shouldPrompt) {
                    await Until(() => Prompts(events).Length == 1 || events.Any(e => e.Kind == "exited"), "pending deadline: " + mode);
                    Check(Prompts(events).Length == 1 && runner.ActiveCount == 1 && !events.Any(e => e.Kind == "exited"), "real AwaitingRisk: " + mode);
                    Check(File.ReadAllText(old) == "original-target" && Directory.GetFiles(output.FullName, ".nxpart-*").Length == 1, "no current-member partial or publish before approval");
                    Check(writes == (mode is "total_limit" or "cancel" ? 1 : 0), "risk precedes actual current decode/write");
                    try { using var denied = File.Open(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source released during risk wait"); } catch (IOException) { }
                    var first = Prompts(events)[0]; var ticket = first.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Check(ticket.OperationId == id && first.GetProperty("admitted_entry_bytes").GetUInt64() == entryLimit && first.GetProperty("admitted_output_bytes").GetUInt64() == totalLimit, "original finite admission in prompt");
                    if (previousTask is not null) Check(!runner.RespondRisk(id, previousTask, entryLimit), "no cross-task approval");
                    if (mode is "cancel" or "production_pending") Check(runner.Cancel(id), "original wait cancel");
                    else if (mode == "decline") Check(runner.RespondRisk(id, ticket, null), "typed risk rejection");
                    else {
                        if (mode == "larger_answer") Check(runner.RespondRisk(id, ticket, entryLimit + 1), "larger request transported, not accepted by Rust");
                        Check(runner.RespondRisk(id, ticket, entryLimit), "explicit finite confirmation");
                        previousTask = ticket;
                        if (mode == "stale") {
                            await Until(() => Prompts(events).Length == 2 || events.Any(e => e.Kind == "exited"), "second member pending");
                            Check(Prompts(events).Length == 2, "first confirmation must not approve later member");
                            var second = Prompts(events)[1]; var next = second.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                            Check(next != ticket && second.GetProperty("ordinal").GetUInt64() == 1, "fresh member-scoped ticket");
                            Check(runner.RespondRisk(id, ticket, entryLimit), "stale reply forwarded for Rust refusal");
                            Check(runner.RespondRisk(id, next, entryLimit), "current second confirmation");
                        }
                    }
                }
                await Until(() => events.Any(e => e.Kind == "exited"), "original terminal: " + mode);
                Check(await runner.DrainAsync(), "actual cleanup");
                var terminal = events.Single(e => e.Kind == "exited");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0 && events.Count(e => e.Kind == "started") == 1, "original worker/Job cleanup with no respawn");
                var success = mode is "ordinary" or "confirm" or "stale" or "larger_answer";
                if (success) {
                    Check(terminal.Category == "success" && terminal.FilesCommitted == (mode == "stale" ? 2 : 1), "risk success exact commit count: " + mode);
                    Check(File.ReadAllBytes(old).SequenceEqual(new byte[4096]), "actual native content exact");
                    if (mode == "stale") Check(File.ReadAllBytes(Path.Combine(output.FullName, "second.bin")).SequenceEqual(new byte[4096]), "second approved content exact");
                } else {
                    var frame = events.Single(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure").Frame!.Value;
                    var expected = mode is "decline" or "cancel" or "production_pending" ? "Cancelled" : mode is "entry_limit" or "total_limit" or "declared_limit" ? "ResourceLimit" : mode == "unsafe" ? "UnsafePath" : mode == "collision" ? "OutputCollision" : "CorruptArchive";
                    Check(frame.GetProperty("error_code").GetString() == expected, "unchanged safety/budget category: " + mode);
                    Check(terminal.FilesCommitted == (mode is "total_limit" or "cancel" ? 1 : 0) && File.ReadAllText(old) == "original-target", "current output refused, old bytes retained");
                    if (expected == "Cancelled") Check(terminal.Category == (mode == "cancel" ? "cancelled_with_committed_output" : "cancelled"), "wait cancellation truth");
                    if (mode is "total_limit" or "cancel") Check(File.ReadAllBytes(Path.Combine(output.FullName, "prior.bin")).SequenceEqual(new byte[256]), "prior commit retained, not rolled back");
                    if (mode is "decline" or "cancel") Check(!File.Exists(Path.Combine(output.FullName, "later.bin")), "no later member after rejected risk wait");
                }
                var confirms = events.Count(e => e.Frame is { } f && f.GetProperty("op").GetString() == "risk_confirmed");
                Check(confirms == (mode == "stale" ? 2 : mode is "confirm" or "larger_answer" or "entry_limit" or "total_limit" or "corrupt" ? 1 : 0), "only current exact-limit answers confirmed");
                if (!shouldPrompt) Check(Prompts(events).Length == 0, "ordinary/hard-refusal path must not prompt");
                Check(Directory.GetFiles(output.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 1 && File.ReadAllText(foreign) == "foreign", "owned partial cleanup, external file retained");
                Check(!File.Exists(Path.Combine(owned.FullName, "escape.bin")), "unsafe path no escape");
                Check(!runner.RespondRisk(id, new(id, Guid.NewGuid(), 1), entryLimit), "retired task cannot approve");
                using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
                var diagnostic = JsonSerializer.Serialize(log.Records.Select(r => new { r.Name, r.Id, r.Fields }));
                Check(!diagnostic.Contains(root.FullName) && !diagnostic.Contains("original-target") && !diagnostic.Contains("payload.bin"), "risk diagnostics fixed-category privacy");
                if (shouldPrompt) Check(log.Records.Any(r => r.Name == "archive.worker.awaiting_risk" && r.Id == id), "correlated observable risk state");
            }
        } finally { if (copied) File.Delete(helper); root.Delete(true); }
    }
#endif
}
