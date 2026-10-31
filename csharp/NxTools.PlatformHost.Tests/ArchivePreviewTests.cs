using System.Formats.Tar;
using System.Text;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Owned temporary ZIP/cache/worker only: no GUI, Shell, Registry, service or integration settings.
internal static class ArchivePreviewTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static ProtocolEnvelope Wire(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
    public static Task PurposeWire()
    {
        foreach (var action in new[] { "preview", "preview_text" }) {
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = action, Path = @"C:\owned\source.zip", Ordinal = 0, TempPolicy = ArchiveTempPolicy.Default,
                Browser = new(Guid.NewGuid(), "Zip", "", new("v1:fixture", new uint[32], 0, 0)) };
            Check(ArchiveEntryWire.Parse(Wire(command)).Action == action, "strict preview purpose");
            foreach (var invalid in new[] { command with { Ordinal = null }, command with { LeaseId = Guid.NewGuid() }, command with { Action = "open" } }) {
                var rejected = false;
                try { ArchiveEntryWire.Parse(Wire(invalid)); } catch (InvalidDataException) { rejected = true; }
                Check(rejected, "preview cannot grant cache/Shell authority");
            }
        }
        return Task.CompletedTask;
    }
    public static async Task ActualOutputBudgets()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            var worker = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../target/debug/nxtools-archive-worker.exe"));
            Check(File.Exists(worker), "parent-built isolated worker required");
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var path = Path.Combine(root, "source.zip");
            using (var zip = ZipFile.Open(path, ZipArchiveMode.Create)) {
                foreach (var size in new[] { 65536, 65537, 8388608, 8388609 }) {
                    using var output = zip.CreateEntry("member-" + size + ".txt", CompressionLevel.NoCompression).Open();
                    var buffer = new byte[8192]; var remaining = size;
                    while (remaining > 0) { var count = Math.Min(remaining, buffer.Length); output.Write(buffer, 0, count); remaining -= count; }
                }
                using var small = new StreamWriter(zip.CreateEntry("small.txt", CompressionLevel.NoCompression).Open());
                small.Write(new string('p', 80));
            }
            // The original source contains other members far beyond the preview output cap.
            // Its selected 80-byte member still has to cross source acquisition and the worker.
            using (var zip = ZipFile.OpenRead(path)) {
                Check(zip.Entries[4].Length == 80 && new FileInfo(path).Length > (8L << 20), "independent source and selected-member sizes");
            }
            await using var reads = ArchiveReadSession.ForOwnedTestPackage(root);
            var open = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
            var identity = ArchiveReadWire.Parse(open); Check(reads.Dispatch(open).Success, "source read accepted");
            ArchiveReadTerminal? terminal = null;
            while (terminal is null) {
                var frame = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original credit");
                terminal = item.Terminal;
            }
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "source binding");
            await using (var sourceReads = ArchiveReadSession.ForOwnedTestPackage(root)) {
                var limited = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true, MaxOutputBytes = 1 });
                Check(sourceReads.Dispatch(limited).Success, "finite source-read request accepted");
                ArchiveReadTerminal? sourceTerminal = null;
                while (sourceTerminal is null) {
                    var sourceFrame = await sourceReads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                    sourceTerminal = sourceFrame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!.Terminal;
                }
                Check(sourceTerminal.Status == "failed" && sourceTerminal.Code == "resource_limit" && sourceTerminal.CleanupComplete,
                    "source admission limit is not source-unavailable or selected-member size: " + sourceTerminal.Code);
            }
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            foreach (var pair in new[] { ("preview_text", 4ul, true), ("preview_text", 0ul, true), ("preview_text", 1ul, false), ("preview", 2ul, true), ("preview", 3ul, false), ("materialize", 3ul, true) }) {
                var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = 1,
                    Action = pair.Item1, Path = path, Ordinal = pair.Item2, Browser = terminal.Browser, CopyMotw = false, PasswordInteraction = pair.Item2 == 4 };
                var outer = await entries.Dispatch(Wire(command)).WaitAsync(TimeSpan.FromSeconds(40));
                var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
                Check(outer.Success == pair.Item3 && result.Success == pair.Item3 && result.CleanupComplete, "actual purpose ceiling: " + pair.Item1 + "/" + pair.Item2 + "/" + result.Code);
                if (pair.Item3) {
                    Check(result.Path is not null && File.Exists(result.Path), "same owned cache output");
                    if (pair.Item2 == 4) Check(File.ReadAllText(result.Path!) == new string('p', 80), "actual 80-byte preview output");
                    if (pair.Item1 != "materialize") {
                        var openLease = command with { Action = "open", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = result.LeaseId, PasswordInteraction = false };
                        var refused = await entries.Dispatch(Wire(openLease));
                        Check(!refused.Success && refused.Error == "unsupported_operation", "preview never grants Shell authority");
                    }
                    var release = command with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = result.LeaseId, PasswordInteraction = false };
                    Check((await entries.Dispatch(Wire(release))).Success && !File.Exists(result.Path), "original lease release");
                } else Check(result.Path is null && result.Code == "resource_limit", "typed resource failure");
            }
            var tarPath = Path.Combine(root, "nested.tar.gz");
            using (var archive = File.Create(tarPath))
            using (var gzip = new GZipStream(archive, CompressionLevel.Fastest))
            using (var tar = new TarWriter(gzip)) {
                tar.WriteEntry(new PaxTarEntry(TarEntryType.RegularFile, "small.txt") { DataStream = new MemoryStream(Encoding.ASCII.GetBytes(new string('p', 80))) });
                tar.WriteEntry(new PaxTarEntry(TarEntryType.RegularFile, "other.bin") { DataStream = new MemoryStream(new byte[128 * 1024]) });
            }
            // The expanded TAR exceeds the 64 KiB output cap, but is within metadata credit.
            Check(new FileInfo(tarPath).Length < (64L << 10), "small encoded compound source");
            await using var tarReads = ArchiveReadSession.ForOwnedTestPackage(root);
            var tarOpen = ArchiveProductReadTests.Command(path: tarPath, version: 2, limits: new() { BrowserContext = true });
            var tarIdentity = ArchiveReadWire.Parse(tarOpen);
            Check(tarReads.Dispatch(tarOpen).Success, "compound source accepted");
            ArchiveReadTerminal? tarTerminal = null;
            ArchiveReadEntry? tinyEntry = null;
            while (tarTerminal is null) {
                var frame = await tarReads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) {
                    tinyEntry ??= batch.Entries.FirstOrDefault(entry => entry.Name == "small.txt");
                    Check(tarReads.Dispatch(ArchiveProductReadTests.Command("ack", tarIdentity.OperationId, tarIdentity.SessionId, tarIdentity.Generation, sequence: batch.Sequence, version: 2)).Success, "compound credit");
                }
                tarTerminal = item.Terminal;
            }
            Check(tinyEntry is { Ordinal: 0, Size: 80 }, "actual member size survives metadata and read transport");
            Check(tarTerminal.Status == "succeeded" && tarTerminal.Browser is not null && tarTerminal.CleanupComplete, "compound source binding");
            var tarCommand = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = tarIdentity.SessionId, Generation = 1,
                Action = "preview_text", Path = tarPath, Ordinal = 0, Browser = tarTerminal.Browser, CopyMotw = false, PasswordInteraction = true };
            var tarOuter = await entries.Dispatch(Wire(tarCommand)).WaitAsync(TimeSpan.FromSeconds(40));
            var tarResult = JsonSerializer.Deserialize<ArchiveEntryResult>(tarOuter.Data!, JsonDefaults.Options)!;
            Check(tarOuter.Success && tarResult.Success && tarResult.CleanupComplete && tarResult.Path is not null, "80-byte compound preview: " + tarResult.Code);
            Check(File.ReadAllText(tarResult.Path!) == new string('p', 80), "actual compound 80-byte output");
            var tarRelease = tarCommand with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = tarResult.LeaseId, PasswordInteraction = false };
            var largeCommand = tarCommand with { OperationId = Guid.NewGuid(), Ordinal = 1 };
            var largeOuter = await entries.Dispatch(Wire(largeCommand)).WaitAsync(TimeSpan.FromSeconds(40));
            var largeResult = JsonSerializer.Deserialize<ArchiveEntryResult>(largeOuter.Data!, JsonDefaults.Options)!;
            Check(!largeOuter.Success && !largeResult.Success && largeResult.Code == "resource_limit" && largeResult.Path is null && largeResult.CleanupComplete,
                "actual oversized compound member keeps output limit");
            Check((await entries.Dispatch(Wire(tarRelease))).Success, "compound lease release");
        } finally { Directory.Delete(root, true); }
    }
}
