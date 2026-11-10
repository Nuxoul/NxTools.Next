using System.Collections.Concurrent;
using System.Formats.Tar;
using System.Text;
using System.IO.Compression;
using System.Text.Json;
using System.Text.Json.Nodes;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Owned temporary ZIP/cache/worker only: no GUI, Shell, Registry, service or integration settings.
internal static class ArchivePreviewTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static ProtocolEnvelope Wire(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
    private static ProtocolEnvelope WireOptions(ArchiveEntryCommand value, JsonNode? options)
    {
        var data = JsonSerializer.SerializeToNode(value, JsonDefaults.Options)!.AsObject();
        data["preview_options"] = options;
        return ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data }, value.OperationId);
    }
    private static ProtocolEnvelope RawOptions(ArchiveEntryCommand value, string options)
    {
        var data = JsonSerializer.SerializeToNode(value, JsonDefaults.Options)!.AsObject();
        var json = data.ToJsonString();
        using var document = JsonDocument.Parse(json[..^1] + ",\"preview_options\":" + options + "}");
        return ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = document.RootElement.Clone() }, value.OperationId);
    }
    private static JsonNode Options(ulong maxBytes, uint timeoutMs) => JsonSerializer.SerializeToNode(new { max_bytes = maxBytes, timeout_ms = timeoutMs })!;
    private static void Reject(ProtocolEnvelope envelope, string detail)
    {
        try { ArchiveEntryWire.Parse(envelope); }
        catch (Exception failure) when (failure is InvalidDataException or JsonException) { return; }
        throw new InvalidOperationException(detail);
    }
    public static Task PurposeWire()
    {
        foreach (var action in new[] { "preview", "preview_text" }) {
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = action, Path = @"C:\owned\source.zip", Ordinal = 0, TempPolicy = ArchiveTempPolicy.Default,
                Browser = new(Guid.NewGuid(), "Zip", "", new("v1:fixture", new uint[32], 0, 0)) };
            Check(ArchiveEntryWire.Parse(Wire(command)).Action == action, "strict preview purpose");
            var maximum = action == "preview_text" ? 8ul << 20 : 128ul << 20;
            foreach (var boundary in new[] { (1ul, 200u), (maximum, 120000u) }) {
                var parsed = ArchiveEntryWire.Parse(WireOptions(command, Options(boundary.Item1, boundary.Item2)));
                Check(parsed.Action == action && parsed.PreviewOptions == new ArchivePreviewOptions(boundary.Item1, boundary.Item2), "configured preview boundary");
            }
            Check(ArchiveEntryWire.Parse(WireOptions(command, null)).PreviewOptions is null, "explicit null keeps legacy preview");
            var typed = command with { PreviewOptions = new(4096, 5000) };
            var typedJson = JsonSerializer.SerializeToElement(typed, JsonDefaults.Options);
            Check(typedJson.GetProperty("preview_options").GetProperty("max_bytes").GetUInt64() == 4096 &&
                ArchiveEntryWire.Parse(Wire(typed)).PreviewOptions == typed.PreviewOptions, "typed symmetric preview options");
            var workerCommand = typed with { Action = "materialize", PreviewOptions = null };
            Check(!JsonSerializer.SerializeToElement(workerCommand, JsonDefaults.Options).TryGetProperty("preview_options", out _),
                "private materialize command omits preview options");
            foreach (var invalid in new[] { command with { Ordinal = null }, command with { LeaseId = Guid.NewGuid() }, command with { Action = "open" } })
                Reject(Wire(invalid), "preview cannot grant cache/Shell authority");
            foreach (var invalid in new[] {
                "{}", "{\"max_bytes\":1}", "{\"timeout_ms\":200}",
                "{\"max_bytes\":1,\"timeout_ms\":200,\"extra\":1}",
                "{\"max_bytes\":1,\"max_bytes\":2,\"timeout_ms\":200}",
                "{\"max_bytes\":\"1\",\"timeout_ms\":200}",
                "{\"max_bytes\":1,\"timeout_ms\":200.0}",
                "{\"max_bytes\":0,\"timeout_ms\":200}",
                "{\"max_bytes\":" + (maximum + 1) + ",\"timeout_ms\":200}",
                "{\"max_bytes\":1,\"timeout_ms\":199}",
                "{\"max_bytes\":1,\"timeout_ms\":120001}" })
                Reject(RawOptions(command, invalid), "malformed preview options accepted: " + invalid);
        }
        var browser = new ArchiveBrowserContext(Guid.NewGuid(), "Zip", "", new("v1:fixture", new uint[32], 0, 0));
        var session = Guid.NewGuid();
        var revokeId = Guid.NewGuid();
        var nonPreview = new[] {
            new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                Action = "materialize", Path = @"C:\owned\source.zip", Ordinal = 0, Browser = browser },
            new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                Action = "authorize", Path = @"C:\owned\source.zip", Browser = browser, PasswordInteraction = true },
            new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1, Action = "cancel" },
            new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1, Action = "release", LeaseId = Guid.NewGuid() },
            new ArchiveEntryCommand { Version = 1, OperationId = revokeId, SessionId = session, Generation = 1, Action = "revoke", LeaseId = revokeId },
            new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1, Action = "open", LeaseId = Guid.NewGuid() }
        };
        foreach (var command in nonPreview) {
            Check(ArchiveEntryWire.Parse(WireOptions(command, null)).Action == command.Action &&
                ArchiveEntryWire.Parse(WireOptions(command, null)).PreviewOptions is null,
                "explicit null must equal absent for " + command.Action);
            Reject(WireOptions(command, Options(1, 200)), "non-preview options accepted for " + command.Action);
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
            var gzipPath = Path.Combine(root, "large.txt.gz");
            var gzipPayload = Enumerable.Range(0, 65537).Select(index => (byte)(index * 31)).ToArray();
            using (var archive = File.Create(gzipPath))
            using (var gzip = new GZipStream(archive, CompressionLevel.Fastest)) gzip.Write(gzipPayload);
            await using var gzipReads = ArchiveReadSession.ForOwnedTestPackage(root);
            var gzipOpen = ArchiveProductReadTests.Command(path: gzipPath, version: 2, limits: new() { BrowserContext = true });
            var gzipIdentity = ArchiveReadWire.Parse(gzipOpen);
            Check(gzipReads.Dispatch(gzipOpen).Success, "unknown-size Gzip source accepted");
            ArchiveReadTerminal? gzipTerminal = null;
            ArchiveReadEntry? gzipEntry = null;
            while (gzipTerminal is null) {
                var frame = await gzipReads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) {
                    gzipEntry ??= batch.Entries.SingleOrDefault();
                    Check(gzipReads.Dispatch(ArchiveProductReadTests.Command("ack", gzipIdentity.OperationId, gzipIdentity.SessionId,
                        gzipIdentity.Generation, sequence: batch.Sequence, version: 2)).Success, "Gzip metadata credit");
                }
                gzipTerminal = item.Terminal;
            }
            Check(gzipEntry is { Size: null } && gzipTerminal.Status == "succeeded" && gzipTerminal.Browser is not null && gzipTerminal.CleanupComplete,
                "standalone Gzip selected member remains unknown-size until actual decode");
            var previewLog = new CaptureLog();
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root, previewLog);
            var gzipCommand = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = gzipIdentity.SessionId, Generation = 1,
                Action = "preview_text", Path = gzipPath, Ordinal = gzipEntry!.Ordinal, Browser = gzipTerminal!.Browser, CopyMotw = false };
            var gzipOuter = await entries.Dispatch(WireOptions(gzipCommand, Options(65536, 30000))).WaitAsync(TimeSpan.FromSeconds(40));
            var gzipResult = JsonSerializer.Deserialize<ArchiveEntryResult>(gzipOuter.Data!, JsonDefaults.Options)!;
            var gzipWorker = previewLog.Rows.Single(row => row.Event == "archive.worker.failed" && row.Id == gzipCommand.OperationId);
            Check(!gzipOuter.Success && !gzipResult.Success && gzipResult.Code == "selected_output_limit" && gzipResult.Path is null &&
                gzipResult.CleanupComplete && Equals(gzipWorker.Fields["category"], "selected_output_limit") && entries.ActiveWorkerCountForFixture == 0,
                "unknown-size Gzip selected output limit must stay distinct: " + gzipResult.Code);
            Check(!Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any(), "failed Gzip preview output/lease cleanup");
            var gzipRetry = gzipCommand with { OperationId = Guid.NewGuid() };
            var gzipRetryOuter = await entries.Dispatch(WireOptions(gzipRetry, Options(131072, 30000))).WaitAsync(TimeSpan.FromSeconds(40));
            var gzipRetryResult = JsonSerializer.Deserialize<ArchiveEntryResult>(gzipRetryOuter.Data!, JsonDefaults.Options)!;
            Check(gzipRetryOuter.Success && gzipRetryResult.Success && gzipRetryResult.Path is not null &&
                File.ReadAllBytes(gzipRetryResult.Path!).SequenceEqual(gzipPayload), "Gzip preview readmits after failed selected quota");
            var gzipRelease = gzipRetry with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = gzipRetryResult.LeaseId };
            Check((await entries.Dispatch(Wire(gzipRelease))).Success && !File.Exists(gzipRetryResult.Path), "Gzip preview retry lease cleanup");
            var forbidden = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = 1,
                Action = "materialize", Path = path, Ordinal = 4, Browser = terminal.Browser, CopyMotw = false };
            var forbiddenResult = await entries.Dispatch(WireOptions(forbidden, Options(80, 200)));
            Check(!forbiddenResult.Success && forbiddenResult.Error == "invalid_request" && entries.ActiveWorkerCountForFixture == 0 &&
                !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any(), "non-preview options must reject before native/cache effects");
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
                } else Check(result.Path is null && result.Code == "selected_output_limit", "typed selected output failure");
            }
            var customText = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = 1,
                Action = "preview_text", Path = path, Ordinal = 1, Browser = terminal.Browser, CopyMotw = false };
            var customTextOuter = await entries.Dispatch(WireOptions(customText, Options(65537, 30000))).WaitAsync(TimeSpan.FromSeconds(40));
            var customTextResult = JsonSerializer.Deserialize<ArchiveEntryResult>(customTextOuter.Data!, JsonDefaults.Options)!;
            Check(customTextOuter.Success && customTextResult.Success && customTextResult.CleanupComplete && customTextResult.Path is not null &&
                new FileInfo(customTextResult.Path!).Length == 65537, "configured text preview above legacy limit: " + customTextResult.Code);
            var customTextRelease = customText with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = customTextResult.LeaseId };
            Check((await entries.Dispatch(Wire(customTextRelease))).Success && !File.Exists(customTextResult.Path), "configured text preview cleanup");

            foreach (var narrow in new[] { (Max: 65536ul, Internal: ArchiveEntrySession.MaterializeOutputLimit), (Max: 65537ul, Internal: 65536ul) }) {
                var command = customText with { OperationId = Guid.NewGuid() };
                var outer = await entries.Dispatch(WireOptions(command, Options(narrow.Max, 30000)), narrow.Internal).WaitAsync(TimeSpan.FromSeconds(40));
                var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
                Check(!outer.Success && result.Code == "selected_output_limit" && result.Path is null && result.CleanupComplete,
                    "configured/internal selected output limits remain minimum: " + narrow);
            }

            var customImage = customText with { OperationId = Guid.NewGuid(), Action = "preview", Ordinal = 3 };
            var customImageOuter = await entries.Dispatch(WireOptions(customImage, Options(8388609, 30000))).WaitAsync(TimeSpan.FromSeconds(40));
            var customImageResult = JsonSerializer.Deserialize<ArchiveEntryResult>(customImageOuter.Data!, JsonDefaults.Options)!;
            Check(customImageOuter.Success && customImageResult.Success && customImageResult.Path is not null && new FileInfo(customImageResult.Path!).Length == 8388609,
                "configured image preview above legacy limit: " + customImageResult.Code);
            var customImageRelease = customImage with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = customImageResult.LeaseId };
            Check((await entries.Dispatch(Wire(customImageRelease))).Success && !File.Exists(customImageResult.Path), "configured image preview cleanup");
            var tinyImage = customText with { OperationId = Guid.NewGuid(), Action = "preview", Ordinal = 4 };
            var tinyImageOuter = await entries.Dispatch(WireOptions(tinyImage, Options(1, 30000))).WaitAsync(TimeSpan.FromSeconds(40));
            var tinyImageResult = JsonSerializer.Deserialize<ArchiveEntryResult>(tinyImageOuter.Data!, JsonDefaults.Options)!;
            Check(!tinyImageOuter.Success && tinyImageResult.Code == "selected_output_limit" && tinyImageResult.CleanupComplete,
                "configured image selected output limit can narrow legacy authority");
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
            Check(!largeOuter.Success && !largeResult.Success && largeResult.Code == "selected_output_limit" && largeResult.Path is null && largeResult.CleanupComplete,
                "actual oversized compound selected member keeps output limit");
            Check((await entries.Dispatch(Wire(tarRelease))).Success, "compound lease release");
            var timeoutLog = new CaptureLog();
            await using (var timed = ArchiveEntrySession.ForOwnedTestPackage(root, timeoutLog)) {
                timed.ProbeWriteForFixture(_ => Thread.Sleep(700));
                var timeoutCommand = customText with { OperationId = Guid.NewGuid() };
                var elapsed = System.Diagnostics.Stopwatch.StartNew();
                var timeoutOuter = await timed.Dispatch(WireOptions(timeoutCommand, Options(65537, 200))).WaitAsync(TimeSpan.FromSeconds(10));
                var timeoutResult = JsonSerializer.Deserialize<ArchiveEntryResult>(timeoutOuter.Data!, JsonDefaults.Options)!;
                var workerTerminal = timeoutLog.Rows.Single(row => row.Event == "archive.worker.failed" && row.Id == timeoutCommand.OperationId);
                Check(!timeoutOuter.Success && !timeoutResult.Success && timeoutResult.Path is null && timeoutResult.CleanupComplete &&
                    Equals(workerTerminal.Fields["category"], "timeout") && elapsed.ElapsedMilliseconds is >= 200 and < 5000 && timed.ActiveWorkerCountForFixture == 0,
                    "configured 200ms timeout must reach original runner and actual cleanup: " + timeoutResult.Code);
            }
            Check(!Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any(), "all preview leases and failed outputs cleaned");
        } finally { Directory.Delete(root, true); }
    }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Event, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Rows = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Rows.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
}
