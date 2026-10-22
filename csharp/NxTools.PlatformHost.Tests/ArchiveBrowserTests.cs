using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only strict DTOs and GUID-owned cache files; no Shell, hooks, service or GUI.
internal static class ArchiveBrowserTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    public static Task WireAndBackwardDefaults()
    {
        var picker = ProtocolEnvelope.Create("platform_command", new { command = "pick_archive_path", data = new { source_hwnd = 42L } });
        Check(PlatformCommandParser.ParsePickArchivePath(picker).SourceHwnd == 42, "picker source identity");
        using var cancelledPicker = new CancellationTokenSource(); cancelledPicker.Cancel();
        var cancelledReply = NxTools.PlatformHost.Windows.ArchivePathPicker.Pick(new(0), cancelledPicker.Token);
        Check(cancelledReply.Success && JsonDocument.Parse(cancelledReply.Data!).RootElement.GetProperty("status").GetString() == "cancelled", "picker pre-cancel must not show GUI");
        var entry = JsonSerializer.Deserialize<ArchiveReadEntry>("{\"ordinal\":0,\"name\":\"x\",\"kind\":\"File\",\"size\":0,\"packed_size\":0,\"encrypted\":false,\"warning_codes\":[],\"provenance\":\"Utf8Flag\",\"codepage\":65001}", JsonDefaults.Options)!;
        Check(entry.Modified is null && entry.Method is null && entry.Crc32 is null && entry.Attributes is null, "legacy optional display defaults");
        var id = Guid.NewGuid(); var session = Guid.NewGuid();
        ProtocolEnvelope Envelope(object data) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data }, id);
        var valid = new { version = 1, operation_id = id, session_id = session, generation = 1, action = "cancel" };
        Check(ArchiveEntryWire.Parse(Envelope(valid)).Action == "cancel", "typed cancel");
        foreach (var extra in new[] { "output_root", "worker_path", "password", "executable" }) {
            var data = JsonSerializer.SerializeToNode(valid)!.AsObject(); data[extra] = "forbidden";
            try { ArchiveEntryWire.Parse(Envelope(data)); throw new InvalidOperationException("accepted authority " + extra); } catch (JsonException) { } catch (InvalidDataException) { }
        }
        Check(!new ArchiveReadLimits { BrowserContext = true }.ValidFor(1) && new ArchiveReadLimits { BrowserContext = true }.ValidFor(2), "explicit v2 opt-in");
        Check(!JsonSerializer.SerializeToElement(new ArchiveReadLimits(), JsonDefaults.Options).TryGetProperty("browser_context", out _), "default v2/v1 serialized opt-in");
        return Task.CompletedTask;
    }
    public static async Task SessionRejectsStaleAndMissingHelper()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            await using var session = ArchiveEntrySession.ForOwnedTestPackage(root);
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = "materialize", Path = Path.Combine(root, "not-opened.zip"), Ordinal = 0, CopyMotw = false,
                Browser = new(Guid.NewGuid(), "Zip", "", new("v1:1:1:1:1", new uint[32], 1, 1)) };
            ProtocolEnvelope Envelope(ArchiveEntryCommand item) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = item }, item.OperationId);
            var reply = await session.Dispatch(Envelope(command)).WaitAsync(TimeSpan.FromSeconds(10));
            var terminal = JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
            Check(!reply.Success && terminal.Code == "helper_missing" && terminal.Path is null && terminal.CleanupComplete, "missing helper terminal");
            Check(!(await session.Dispatch(Envelope(command))).Success, "retired duplicate replaced operation");
            var cancel = command with { Action = "cancel", Path = null, Browser = null, Ordinal = null, CopyMotw = null };
            Check(!(await session.Dispatch(Envelope(cancel))).Success && !(await session.Dispatch(Envelope(cancel with { Generation = 2 }))).Success, "stale control accepted");
            for (var attempt = 0; attempt < 34; attempt++) {
                var repeated = await session.Dispatch(Envelope(command with { OperationId = Guid.NewGuid() })).WaitAsync(TimeSpan.FromSeconds(10));
                Check(JsonSerializer.Deserialize<ArchiveEntryResult>(repeated.Data!, JsonDefaults.Options)!.Code == "helper_missing", "failed leases exhausted cache slots");
            }
        } finally { Directory.Delete(root, true); }
    }
    public static Task ReadBrowserBudgetAndEmpty()
    {
        var context = new ArchiveBrowserContext(Guid.NewGuid(), "Zip", "plain comment", new("v1:1:00000000000000000000000000000001:0:0", new uint[32], 0, 0));
        var frame = JsonSerializer.SerializeToElement(new { browser = context }, JsonDefaults.Options);
        var metadata = new ArchiveReadMetadata(new() { BrowserContext = true }, 0, 2); metadata.ReadBrowser(frame);
        Check(metadata.Entries == 0 && metadata.Batches == 0 && metadata.Browser?.SnapshotId == context.SnapshotId && metadata.Bytes > 0, "empty archive context omitted/uncharged");
        try { metadata.ReadBrowser(frame); throw new InvalidOperationException("duplicate context accepted"); } catch (InvalidDataException) { }
        try { new ArchiveReadMetadata(new() { MaxRetainedBytes = 1, BrowserContext = true }, 0, 2).ReadBrowser(frame); throw new InvalidOperationException("context bypassed budget"); } catch (ArchiveReadBudgetException) { }
        return Task.CompletedTask;
    }
    public static async Task OwnedLeaseRetention()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            using var cache = ArchiveEntryCache.ForOwnedTestCache(root);
            var lease = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            Check(Directory.Exists(lease.Root) && File.Exists(Path.Combine(lease.Root, "manifest.json")), "exclusive owned manifest");
            var output = Path.Combine(lease.Root, "output.txt"); File.WriteAllText(output, "baseline");
            lease.Seal(output);
            File.WriteAllText(output, "user edit");
            Check(!cache.Release(lease.Id), "changed output was deleted");
            Check(File.ReadAllText(output) == "user edit", "edited data retained");
            Check(!cache.Release(Guid.NewGuid()), "unknown lease accepted");
            var clean = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            var cleanPath = Path.Combine(clean.Root, "clean.txt"); File.WriteAllText(cleanPath, "safe"); clean.Seal(cleanPath);
            Check(cache.Release(clean.Id) && !File.Exists(cleanPath), "unchanged owned file not removed");
            Check(!Directory.Exists(clean.Root), "successful release left manifest/session root");
            var locked = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            var lockedPath = Path.Combine(locked.Root, "locked.txt"); File.WriteAllText(lockedPath, "safe"); locked.Seal(lockedPath);
            using (var external = File.Open(lockedPath, FileMode.Open, FileAccess.Read, FileShare.Read)) Check(!cache.Release(locked.Id) && File.Exists(lockedPath), "external lock not retained");
            var sweeperTrigger = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            Check(!File.Exists(lockedPath) && File.Exists(output), "bounded sweep lost edited file or retained unlocked file");
            string abandoned;
            using (var previous = ArchiveEntryCache.ForOwnedTestCache(root)) {
                var oldLease = previous.Create(Guid.NewGuid(), Guid.NewGuid(), 1); abandoned = Path.Combine(oldLease.Root, "abandoned.txt");
                File.WriteAllText(abandoned, "retained"); oldLease.Seal(abandoned);
                using var external = File.Open(abandoned, FileMode.Open, FileAccess.Read, FileShare.Read);
                Check(!previous.Release(oldLease.Id), "locked explicit retirement retained"); previous.Dispose();
                Check(File.Exists(abandoned), "dispose ignored external lock");
            }
            cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
            Check(!File.Exists(abandoned) && File.Exists(output), "persisted owned manifest sweep");
            await Task.CompletedTask;
        } finally { Directory.Delete(root, true); }
    }
    public static async Task ActualSelectedEntryAndRevision()
    {
        var root = ArchiveProductReadTests.Root();
        try {
            var worker = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../target/debug/nxtools-archive-worker.exe"));
            Check(File.Exists(worker), "parent-built isolated worker required");
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var path = Path.Combine(root, "source.zip");
            using (var zip = System.IO.Compression.ZipFile.Open(path, System.IO.Compression.ZipArchiveMode.Create)) {
                foreach (var name in new[] { "first.txt", "selected.txt", "last.txt" }) {
                    var item = zip.CreateEntry(name); using var writer = new StreamWriter(item.Open()); writer.Write(name);
                }
            }
            await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
            var open = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
            var identity = ArchiveReadWire.Parse(open); Check(read.Dispatch(open).Success, "original read admission");
            ArchiveReadTerminal? terminal = null;
            while (terminal is null) {
                var frame = await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) Check(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original credit");
                terminal = item.Terminal;
            }
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "actual source-bound browser terminal: " + terminal.Code);
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = identity.SessionId, Generation = 1,
                Action = "materialize", Path = path, Ordinal = 1, Browser = terminal.Browser, CopyMotw = false };
            ProtocolEnvelope Envelope(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
            var result = await entries.Dispatch(Envelope(command)).WaitAsync(TimeSpan.FromSeconds(40));
            var output = JsonSerializer.Deserialize<ArchiveEntryResult>(result.Data!, JsonDefaults.Options)!;
            Check(result.Success && output.CleanupComplete && output.Path is not null && File.ReadAllText(output.Path) == "selected.txt", "selected entry terminal: " + output.Code);
            Check(Directory.EnumerateFiles(Path.GetDirectoryName(output.Path!)!).Select(Path.GetFileName).Order().SequenceEqual(new[] { "selected.txt" }), "unselected outputs created");
            Check(!(await entries.Dispatch(Envelope(command))).Success, "duplicate materialize accepted");
            var release = command with { Action = "release", Path = null, Ordinal = null, Browser = null, CopyMotw = null, LeaseId = output.LeaseId };
            Check((await entries.Dispatch(Envelope(release))).Success && !File.Exists(output.Path), "owned release");
            var cancelledCommand = command with { OperationId = Guid.NewGuid() };
            var pending = entries.Dispatch(Envelope(cancelledCommand));
            var cancel = cancelledCommand with { Action = "cancel", Path = null, Ordinal = null, Browser = null, CopyMotw = null };
            Check(!(await entries.Dispatch(Envelope(cancel with { Generation = 2 }))).Success, "stale cancel accepted");
            Check((await entries.Dispatch(Envelope(cancel))).Success, "exact cancellation refused");
            var cancelledResult = JsonSerializer.Deserialize<ArchiveEntryResult>((await pending.WaitAsync(TimeSpan.FromSeconds(40))).Data!, JsonDefaults.Options)!;
            Check(!cancelledResult.Success && cancelledResult.Path is null && cancelledResult.CleanupComplete && cancelledResult.Code == "cancelled", "cancel published Shell authority");
            using (var zip = System.IO.Compression.ZipFile.Open(path, System.IO.Compression.ZipArchiveMode.Update)) zip.CreateEntry("changed.txt");
            var changed = await entries.Dispatch(Envelope(command with { OperationId = Guid.NewGuid() }));
            Check(!changed.Success && JsonSerializer.Deserialize<ArchiveEntryResult>(changed.Data!, JsonDefaults.Options)!.Code == "source_changed", "replacement revision allowed");
        } finally { Directory.Delete(root, true); }
    }
}
