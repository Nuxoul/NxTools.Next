using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned fixture tree and exclusively copied helper; real owned Job/Rust/native chain.
internal static class ArchiveRemediationTests
{
    private static void Require(bool value, string detail)
    { if (!value) throw new InvalidOperationException(detail); }
    private static async Task WaitForExit(ConcurrentQueue<ArchiveWorkerEvent> events)
    {
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited"))
        { Require(clock.ElapsedMilliseconds < 20000, "remediation fixture deadline"); await Task.Delay(20); }
    }
    public static async Task SourceRaceBarriers()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and independent codec fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "fixture refuses existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-source-race-");
        var copied = false;
        var escaped = new List<string>();
        try
        {
            File.Copy(worker, helper, false); copied = true;
            foreach (var fixture in new[] { "lzma.7z", "rar4.rar", "rar5.rar" })
            {
                var scenario = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var input = Directory.CreateDirectory(Path.Combine(scenario.FullName, "input"));
                var path = Path.Combine(input.FullName, fixture);
                File.Copy(Path.Combine(fixtures, fixture), path);
                var original = File.ReadAllBytes(path);
                var target = Directory.CreateDirectory(Path.Combine(scenario.FullName, "output"));
                var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                var stages = new HashSet<string>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                runner.SourceCheckpointProbe = stage =>
                {
                    if (!stages.Add(stage)) return;
                    // A -> B -> A: keep every successful mutation reversible and in this tree.
                    var backup = path + ".original";
                    try
                    {
                        File.Move(path, backup);
                        try { File.WriteAllBytes(path, original); escaped.Add(stage + "/replace"); }
                        finally { if (File.Exists(path)) File.Delete(path); File.Move(backup, path); }
                    }
                    catch (IOException) { }
                    try
                    {
                        using var writer = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete);
                        writer.WriteByte((byte)(original[0] ^ 0x80)); writer.Flush(true);
                        writer.Position = 0; writer.WriteByte(original[0]); writer.Flush(true);
                        escaped.Add(stage + "/write");
                    }
                    catch (IOException) { }
                    try
                    {
                        Directory.Move(input.FullName, input.FullName + ".moved");
                        Directory.Move(input.FullName + ".moved", input.FullName);
                        escaped.Add(stage + "/parent");
                    }
                    catch (IOException) { }
                };
                var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", path, 0,
                    new { max_entries = 100 }, OutputRoot: target.FullName);
                Require(runner.Start(request), "source fixture admission");
                await WaitForExit(events);
                Require(await runner.DrainAsync(), "source fixture drainage");
                Require(events.Last().Category == "success" && events.Last().CleanupComplete, "source fixture terminal");
                Require(stages.SetEquals(new[] { "source_validated", "codec_opened", "source_decoding", "source_precommit" }), "source barriers not traversed");
                var files = Directory.GetFiles(target.FullName, "*", SearchOption.AllDirectories);
                var expected = fixture == "lzma.7z" ? "this is a file\n" : fixture == "rar4.rar" ? "unrar-0.4.0" : "foobar\n";
                Require(files.Length == 1 && File.ReadAllBytes(files[0]).SequenceEqual(Encoding.UTF8.GetBytes(expected)), "source fixture exact bytes");
                // Protection lifetime ends only after actual worker exit and cleanup.
                using var released = new FileStream(path, FileMode.Open, FileAccess.Write, FileShare.None);
            }
            Require(escaped.Count == 0, "unprotected codec source at barriers: " + string.Join(",", escaped));
        }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    public static async Task BudgetAdmission()
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForFixture(Environment.ProcessPath!, events.Enqueue);
        var low = new ArchiveWorkerRequest(Guid.NewGuid(), "test", "job", 0, new { max_working_bytes = 1 });
        Require(!runner.Start(low), "accepted unfulfillable one-byte codec budget");
        Require(events.Last().Category == "resource_limit", "low budget typed rejection");
        var mismatch = new ArchiveWorkerRequest(Guid.NewGuid(), "test", "job", 0,
            new { max_working_bytes = 64UL << 20, max_metadata_bytes = 16UL << 20 }, MemoryBytes: 512 << 20);
        Require(!runner.Start(mismatch), "Limits and MemoryBytes mismatch admitted");
        Require(events.Last().Category == "budget_mismatch", "mismatch typed rejection");
        var invalid = low with { OperationId = Guid.NewGuid(), Limits = new { max_working_bytes = 0 } };
        Require(!runner.Start(invalid) && events.Last().Category == "invalid_budget", "invalid budget typed rejection");
        var malformed = low with { OperationId = Guid.NewGuid(), Limits = new { max_working_bytes = "64MiB" } };
        Require(!runner.Start(malformed) && events.Last().Category == "invalid_budget", "nonnumeric budget typed rejection");
        var defaults = new ArchiveWorkerRequest(Guid.NewGuid(), "open", "job", 0, new { max_entries = 100 });
        Require(runner.Start(defaults), "default legal budget rejected");
        await WaitForExit(events);
        Require(await runner.DrainAsync(), "budget fixture drain");
        var frame = events.Single(e => e.Kind == "frame" && e.Frame!.Value.GetProperty("op").GetString() == "accepted").Frame!.Value;
        Require(frame.GetProperty("data").GetProperty("memory").GetUInt64() == (832UL << 20), "Job not bound to work + metadata + overhead");
    }

    public static async Task SevenzWorkingBudget()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and independent codec fixtures required.");
        // Direct private-worker seam: the parent explicitly acknowledges source protection;
        // only tiny trusted fixtures are decoded. This tests Rust admission, not a fake Job.
        using var child = Process.Start(new ProcessStartInfo(worker, "--archive-worker") {
            RedirectStandardInput = true, RedirectStandardOutput = true, RedirectStandardError = true,
            UseShellExecute = false, CreateNoWindow = true })!;
        try {
            var id = Guid.NewGuid();
            await Send(child.StandardInput.BaseStream, new { schema = 2, operation_id = id, op = "start", operation = "test",
                path = Path.Combine(fixtures, "lzma.7z"), limits = new { max_working_bytes = 1 } });
            string? code = null;
            while (true) {
                using var frame = await Receive(child.StandardOutput.BaseStream);
                var root = frame.RootElement;
                if (root.GetProperty("op").GetString() == "native_call")
                    await Send(child.StandardInput.BaseStream, new { version = 1, operation_id = id, op = "native_result",
                        call_id = root.GetProperty("native").GetProperty("call_id").GetGuid(), success = true });
                if (root.GetProperty("op").GetString() == "failure") { code = root.GetProperty("error_code").GetString(); break; }
                if (root.GetProperty("op").GetString() == "complete") break;
            }
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Require(code == "ResourceLimit", "one-byte work budget decoded trusted LZMA payload");
        }
        finally { if (!child.HasExited) { child.Kill(); await child.WaitForExitAsync(); } }
    }
    public static async Task MappedSourceRace()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and independent codec fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "fixture refuses existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-mapped-source-"); var copied = false;
        try {
            File.Copy(worker, helper, false); copied = true;
            var path = Path.Combine(owned.FullName, "source.7z"); File.Copy(Path.Combine(fixtures, "lzma.7z"), path);
            var original = File.ReadAllBytes(path);
            using var mappedFile = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.ReadWrite | FileShare.Delete);
            using var mapping = System.IO.MemoryMappedFiles.MemoryMappedFile.CreateFromFile(mappedFile, null, 0, System.IO.MemoryMappedFiles.MemoryMappedFileAccess.ReadWrite,
                HandleInheritability.None, leaveOpen: true);
            mappedFile.Dispose();
            using var view = mapping.CreateViewAccessor();
            var reached = false;
            var target = Directory.CreateDirectory(Path.Combine(owned.FullName, "output"));
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            runner.SourceCheckpointProbe = stage => {
                if (stage == "source_validated") { reached = true; view.Write(40, (byte)(original[40] ^ 0x80)); view.Flush(); }
                if (stage == "source_precommit") { view.Write(40, original[40]); view.Flush(); }
            };
            Require(runner.Start(new(Guid.NewGuid(), "extract", path, 0, new { max_entries = 100 }, OutputRoot: target.FullName)), "mapped source admission");
            await WaitForExit(events); Require(await runner.DrainAsync(), "mapped source drain");
            var failure = events.FirstOrDefault(e => e.Frame is {} f && f.GetProperty("op").GetString() == "failure")?.Frame;
            var code = failure?.GetProperty("error_code").GetString();
            Require(events.Last().CleanupComplete, "mapped source cleanup incomplete");
            if (!reached) {
                Require(code == "SourceChanged" && Directory.GetFiles(target.FullName).Length == 0,
                    "mapped source must be refused with SourceChanged before codec: " + code);
            } else {
                Require(events.Last().Category == "success", "writable mapping reached actual codec source");
                Require(File.ReadAllText(Path.Combine(target.FullName, "file.txt")) == "this is a file\n", "mapped source payload differs");
            }
        }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    public static async Task SevenzHeaderClassification()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and independent codec fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "fixture refuses existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-header-classification-");
        var copied = false; var failures = new List<string>();
        try {
            File.Copy(worker, helper, false); copied = true;
            var original = File.ReadAllBytes(Path.Combine(fixtures, "lzma.7z"));
            foreach (var scenario in new[] { "past_eof", "valid_crc_bad_extent", "overflow", "truncated", "metadata", "normal", "dictionary", "encoded_dictionary", "encoded_metadata" }) {
                var bytes = (byte[])original.Clone();
                if (scenario == "past_eof") BitConverter.GetBytes((ulong)bytes.Length + 1).CopyTo(bytes, 12);
                if (scenario == "valid_crc_bad_extent") BitConverter.GetBytes((ulong)bytes.Length).CopyTo(bytes, 20);
                if (scenario == "overflow") { BitConverter.GetBytes(ulong.MaxValue - 1).CopyTo(bytes, 12); BitConverter.GetBytes(8UL).CopyTo(bytes, 20); }
                if (scenario is "past_eof" or "valid_crc_bad_extent" or "overflow") BitConverter.GetBytes(Crc32(bytes.AsSpan(12,20))).CopyTo(bytes, 8);
                if (scenario == "truncated") bytes = bytes[..20];
                if (scenario is "dictionary" or "encoded_dictionary" or "encoded_metadata") {
                    if (scenario.StartsWith("encoded_", StringComparison.Ordinal)) bytes = File.ReadAllBytes(Path.Combine(fixtures, "solid.7z"));
                    var next = checked(32 + (int)BitConverter.ToUInt64(bytes, 12));
                    if (scenario == "encoded_metadata") {
                        // Known independent encoded header declares170bytes as0x80aa.
                        var marker = new byte[] { 0x0c, 0x80, 0xaa };
                        var at = bytes.AsSpan(next).IndexOf(marker); Require(at >= 0, "encoded metadata marker");
                        bytes[next + at + 1] = 0xbf; bytes[next + at + 2] = 0xff;
                    } else {
                        var marker = new byte[] { 0x23, 3, 1, 1, 5, 0x5d };
                        var at = bytes.AsSpan(next).IndexOf(marker); Require(at >= 0, "dictionary marker");
                        BitConverter.GetBytes(128u << 20).CopyTo(bytes, next + at + marker.Length);
                    }
                    BitConverter.GetBytes(Crc32(bytes.AsSpan(next))).CopyTo(bytes, 28);
                    BitConverter.GetBytes(Crc32(bytes.AsSpan(12, 20))).CopyTo(bytes, 8);
                }
                var path = Path.Combine(owned.FullName, scenario + ".7z"); File.WriteAllBytes(path, bytes);
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                object limits = new { max_metadata_bytes = scenario == "metadata" ? 1UL : scenario == "encoded_metadata" ? 4096UL : 256UL << 20,
                    max_working_bytes = 64UL << 20 };
                Require(runner.Start(new(Guid.NewGuid(), "test", path, 0, limits)), "header fixture admission");
                await WaitForExit(events); Require(await runner.DrainAsync(), "header fixture drain");
                var failure = events.FirstOrDefault(e => e.Frame is {} f && f.GetProperty("op").GetString() == "failure")?.Frame;
                var actual = failure?.GetProperty("error_code").GetString() ?? events.Last().Category;
                var expected = scenario == "normal" ? "success" : scenario is "metadata" or "dictionary" or "encoded_dictionary" or "encoded_metadata" ? "ResourceLimit" : "CorruptArchive";
                if (actual != expected || !events.Last().CleanupComplete) failures.Add(scenario + ":" + actual);
            }
            Require(failures.Count == 0, "7z classification mismatch: " + string.Join(",", failures));
        }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    public static async Task NativeJobLimitAndCleanup()
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using (var runner = ArchiveWorkerRunner.ForFixture(Environment.ProcessPath!, events.Enqueue)) {
            var request = new ArchiveWorkerRequest(Guid.NewGuid(), "test", "memorylimit", 0,
                new { max_working_bytes = 64UL << 20, max_metadata_bytes = 16UL << 20 }, MemoryBytes: 144 << 20);
            Require(runner.Start(request), "controlled allocation Job admission");
            await WaitForExit(events); Require(await runner.DrainAsync(), "limited Job cleanup");
            Require(events.Last().Category == "resource_limit" && events.Last().NativeErrorCode is 8 or 1455 or 1816 &&
                events.Last().CleanupComplete && runner.ActiveCount == 0, "native Job limit result/cleanup: " + events.Last().Category);
        }
        events = new(); using var release = new ManualResetEventSlim(); var cleaned = 0;
        using var slots = ArchiveWorkerRunner.ForFixture(Environment.ProcessPath!, events.Enqueue);
        slots.CleanupCheckpointProbe = () => { Interlocked.Increment(ref cleaned); release.Wait(); };
        var first = new ArchiveWorkerRequest(Guid.NewGuid(), "open", "sleep", 0, new { max_entries = 100 });
        var second = first with { OperationId = Guid.NewGuid() };
        try {
            Require(slots.Start(first) && slots.Start(second), "cleanup slots admission");
            var clock = Stopwatch.StartNew();
            while (events.Count(e => e.Kind == "started") < 2) { Require(clock.ElapsedMilliseconds < 10000, "started deadline"); await Task.Delay(20); }
            Require(slots.Cancel(first.OperationId) && slots.Cancel(second.OperationId), "running cancellation");
            clock.Restart(); while (Volatile.Read(ref cleaned) < 2) { Require(clock.ElapsedMilliseconds < 10000, "cleanup barrier deadline"); await Task.Delay(20); }
            Require(slots.ActiveCount == 2 && !slots.Start(first with { OperationId = Guid.NewGuid(), Path = "job" }) &&
                !events.Any(e => e.Kind == "exited"), "terminated process released slot before cleanup");
        }
        finally { release.Set(); slots.Stop(); Require(await slots.DrainAsync(), "cancelled Job drainage"); }
        Require(events.Count(e => e.Kind == "exited" && e.Category == "cancelled" && e.CleanupComplete) == 2, "cancelled terminal cleanup");
    }
    private static uint Crc32(ReadOnlySpan<byte> bytes)
    {
        uint crc = uint.MaxValue;
        foreach (var value in bytes) { crc ^= value; for (var bit = 0; bit < 8; bit++) crc = (crc >> 1) ^ ((crc & 1) == 0 ? 0u : 0xedb88320u); }
        return ~crc;
    }
    private static async Task Send(Stream stream, object frame)
    {
        var bytes = System.Text.Json.JsonSerializer.SerializeToUtf8Bytes(frame);
        await stream.WriteAsync(BitConverter.GetBytes(bytes.Length)); await stream.WriteAsync(bytes); await stream.FlushAsync();
    }
    private static async Task<System.Text.Json.JsonDocument> Receive(Stream stream)
    {
        var header = new byte[4]; await stream.ReadExactlyAsync(header).AsTask().WaitAsync(TimeSpan.FromSeconds(10));
        var size = BitConverter.ToInt32(header); Require(size is > 0 and <= 1048576, "bounded worker frame");
        var bytes = new byte[size]; await stream.ReadExactlyAsync(bytes).AsTask().WaitAsync(TimeSpan.FromSeconds(10));
        return System.Text.Json.JsonDocument.Parse(bytes);
    }
}
