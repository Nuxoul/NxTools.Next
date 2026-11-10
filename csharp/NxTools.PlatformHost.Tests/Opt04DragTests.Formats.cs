using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt04DragTests
{
    // Effects: copies of the prepared independent format fixtures and explicit current worker in
    // marked GUID-owned PI_SCRATCH_DIR roots; original read/drag/entry/runner/output/cache owners,
    // controlled COM GetData/IStream, exact owned-process observation and owned JSON evidence.
    // No physical gesture, external target, clipboard, service, registry or user archive access.
    private sealed record FormatOutput(string Path, string Name);

    internal static async Task RetainedFormatMaterialization()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch), "absolute PI_SCRATCH_DIR required");
        var prepared = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT") ?? throw new InvalidOperationException("explicit prepared NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT required");
        Check(Path.IsPathFullyQualified(prepared), "absolute prepared format root");
        Check(File.Exists(Path.Combine(prepared, "owned-opt-fixtures")), "prepared independent format fixture marker");
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit freshly built matching isolated-codecs NXTOOLS_ARCHIVE_WORKER_FIXTURE required");
        using var provenance = JsonDocument.Parse(File.ReadAllText(Path.Combine(prepared, "fixture-provenance.json")));
        var fixtures = provenance.RootElement.EnumerateArray().ToArray();
        Check(fixtures.Length == 4 && fixtures.Select(f => f.GetProperty("name").GetString()).ToHashSet()
            .SetEquals(new[] { "sevenz-solid", "rar5-solid", "targz", "tarxz" }), "four independently decoded prepared sources");
        var scenarios = 0;
        foreach (var fixture in fixtures) {
            var name = fixture.GetProperty("name").GetString()!;
            var archive = fixture.GetProperty("archive").GetString()!;
            var original = Path.Combine(prepared, name, archive);
            Check(new FileInfo(original).Length == fixture.GetProperty("archive_bytes").GetInt64(), "prepared source size " + name);
            foreach (var order in new[] { "forward", "backward", "nonmonotonic" }) {
                var root = Path.Combine(scratch!, "archive-product-opt04-formats-" + Guid.NewGuid().ToString("D"));
                Check(!Directory.Exists(root), "fresh GUID-owned format root");
                Directory.CreateDirectory(root);
                File.WriteAllText(Path.Combine(root, "owned-opt04-format-fixture"), "OPT04 retained format materialization");
                ArchiveReadHelper.GuardOwnedRoot(root);
                foreach (var evidence in new[] { "fixture-provenance.json", "independent-decoder.txt", "libarchive-COPYING", "upstream-fixture-README.md" })
                    File.Copy(Path.Combine(prepared, evidence), Path.Combine(root, evidence));
                var source = Path.Combine(root, archive);
                File.Copy(original, source);
                var expected = new Dictionary<string, byte[]>(StringComparer.Ordinal);
                foreach (var file in fixture.GetProperty("files").EnumerateArray()) {
                    var member = file.GetProperty("name").GetString()!;
                    var copied = Path.Combine(root, "expected", member.Replace('/', Path.DirectorySeparatorChar));
                    Directory.CreateDirectory(Path.GetDirectoryName(copied)!);
                    File.Copy(Path.Combine(prepared, name, "expected", member.Replace('/', Path.DirectorySeparatorChar)), copied);
                    var bytes = File.ReadAllBytes(copied);
                    Check(bytes.Length == file.GetProperty("size").GetInt32(), "independent expected size " + name);
                    expected.Add(member, bytes);
                }
                Check(expected.Count == (name == "rar5-solid" ? 4 : 2), "multi-member prepared source");
                var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
                File.Copy(worker, helper);
                Console.WriteLine("ARCHIVE_OPT04_FORMAT_PAIR " + JsonSerializer.Serialize(new {
                    root, format = name, order, worker, worker_bytes = new FileInfo(worker).Length,
                    worker_modified_utc = File.GetLastWriteTimeUtc(worker), host = typeof(ArchiveVirtualDragSession).Assembly.Location,
                    tests = typeof(Opt04DragTests).Assembly.Location, source, source_bytes = new FileInfo(source).Length,
                    configuration = typeof(Opt04DragTests).Assembly.Location.Split(Path.DirectorySeparatorChar).Contains("Release", StringComparer.OrdinalIgnoreCase) ? "Release" :
                        typeof(Opt04DragTests).Assembly.Location.Split(Path.DirectorySeparatorChar).Contains("Debug", StringComparer.OrdinalIgnoreCase) ? "Debug" : "unknown",
                    prepared, cache_policy = "no_system_cache_flush"
                }));
                var (browser, files) = await BrowseFormat(root, source, name, expected);
                Check(File.ReadAllBytes(source).SequenceEqual(File.ReadAllBytes(original)), "read operation preserves the copied source");
                await DragFormat(root, source, helper, name, order, browser, files, expected);
                Check(File.ReadAllBytes(source).SequenceEqual(File.ReadAllBytes(original)), "drag operation preserves the copied source");
                // Only successful original-owner retirement permits removal of this copied helper.
                File.Delete(helper);
                Check(!File.Exists(helper), "format helper released and removed");
                scenarios++;
            }
        }
        Check(scenarios == 12, "four sources, three independent demand orders each");
        Console.WriteLine("ARCHIVE_OPT04_FORMAT_COVERAGE " + JsonSerializer.Serialize(new {
            scenarios, sources = 4, solid_rar4 = "separate solid encrypted RAR4 preview/grant lifecycle selection",
            sevenz_extra_shapes = "existing ArchiveVirtualTreeTests selection is separate",
            plain_tar_and_standalone_streams = "not selected"
        }));
    }

    internal static async Task RetainedSplitZipMaterialization()
    {
        var root = Root(false); var helper = InstallWorker(root);
        var expected = new Dictionary<string, byte[]>();
        var sources = new List<string>();
        for (var index = 0; index < 2; index++) {
            var bytes = new byte[150000]; new Random(1937 + index).NextBytes(bytes);
            var name = "split-" + index + ".bin"; var path = Path.Combine(root, name);
            File.WriteAllBytes(path, bytes); expected.Add(name, bytes); sources.Add(path);
        }
        var destination = Directory.CreateDirectory(Path.Combine(root, "split-output")).FullName;
        try {
            await using (var jobs = ArchiveJobSession.ForOwnedTestPackage(root)) {
                var create = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
                    Request = new() { Kind = "create", Paths = sources.ToArray(), Destination = destination, Create = new() {
                        Format = "zip", Level = "fast", Method = "deflate", StripSingleRoot = false, ExcludePatterns = [], TestAfterCreate = false,
                        Name = "split.zip", VolumeBytes = 65536 } } };
                Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = create }, create.OperationId)).Success, "original split creation admission");
                while (true) {
                    var frame = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                    Check(frame.Prompt is null, "owned split creation has no collision prompt");
                    if (frame.Terminal is not { } terminal) continue;
                    Check(terminal.Outcome == "succeeded" && terminal.FilesCommitted >= 2 && terminal.CleanupComplete, "actual original split publication");
                    break;
                }
            }
            var source = Path.Combine(destination, "split.zip");
            var (browser, files) = await BrowseFormat(root, source, "splitzip", expected);
            foreach (var order in new[] { "forward", "backward" }) await DragFormat(root, source, helper, "splitzip", order, browser, files, expected);
        } finally { File.Delete(helper); }
    }

    internal static async Task RetainedBlockIntegrityBeforeReady()
    {
        var prepared = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT") ?? throw new InvalidOperationException("prepared formats required");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!;
        var root = Directory.CreateDirectory(Path.Combine(scratch, "archive-product-opt04-integrity-" + Guid.NewGuid())).FullName;
        File.WriteAllText(Path.Combine(root, "owned-opt04-integrity"), "OPT04 owned corrupted block");
        ArchiveReadHelper.GuardOwnedRoot(root);
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")!;
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName); File.Copy(worker, helper);
        var source = Path.Combine(root, "source.7z");
        var bytes = File.ReadAllBytes(Path.Combine(prepared, "sevenz-solid", "source.7z"));
        var headerOffset = checked((long)System.Buffers.Binary.BinaryPrimitives.ReadUInt64LittleEndian(bytes.AsSpan(12)));
        // Independent 7-Zip listing: this pinned sample has one 2240-byte LZMA2 block at offset32.
        Check(bytes.Length == 2419 && headerOffset + 32 > 2272, "fixed independent solid block precedes encoded header");
        bytes[32 + 2240 - 8] ^= 0x40;
        File.WriteAllBytes(source, bytes);
        try {
            var expected = Directory.GetFiles(Path.Combine(prepared, "sevenz-solid", "expected"), "*", SearchOption.AllDirectories)
                .ToDictionary(path => Path.GetRelativePath(Path.Combine(prepared, "sevenz-solid", "expected"), path).Replace('\\', '/'), File.ReadAllBytes);
            var (browser, files) = await BrowseFormat(root, source, "sevenz-solid", expected);
            var selected = files[0]; var session = Guid.NewGuid(); var log = new Events();
            var command = new ArchiveVirtualDragCommand(1, Guid.NewGuid(), session, 1, "start", 42,
                [new("export.bin", selected.Size, new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session,
                    Generation = 1, Action = "materialize", Path = source, Ordinal = selected.Ordinal, Browser = browser, CopyMotw = false })]);
            var refused = false;
            await using (var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log)) {
                await using var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true, prepare: _ => null,
                    pressed: () => true, escape: () => false, loop: (data, _) => {
                        var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                        try {
                            var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, 0, TYMED.TYMED_ISTREAM);
                            var code = Slot<NativeGetData>(pointer, 3)(pointer, ref format, out var medium);
                            refused = code < 0 && medium.unionmember == IntPtr.Zero;
                            if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium);
                            return (0x00040100, 1);
                        } finally { Marshal.Release(pointer); }
                    });
                var terminal = await drag.Dispatch(Wire(command), 1).WaitAsync(TimeSpan.FromSeconds(40));
                Check(refused && !terminal.Success && log.Named("archive.entry.drag.member_ready").Length == 0,
                    "corrupt required full block cannot supply earlier member: refused=" + refused + ", terminal=" + terminal.Success + "/" + terminal.Error + ", ready=" + log.Named("archive.entry.drag.member_ready").Length);
                Check(entries.ActiveWorkerCountForFixture == 0 && WorkerTerminals(log).Single().Fields.GetProperty("cleanup_complete").GetBoolean(), "failed full-block demand actually drains");
            }
            Check(!FormatOutputs(root).Any() && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any() &&
                !Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "no committed or partial member after integrity failure");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        } finally { File.Delete(helper); }
    }

    private static async Task<(ArchiveBrowserContext Browser, ArchiveReadEntry[] Files)> BrowseFormat(
        string root, string source, string name, Dictionary<string, byte[]> expected)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var request = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(request);
        var entries = new List<ArchiveReadEntry>();
        Check(read.Dispatch(request).Success, "original format read admission " + name);
        while (true) {
            var frame = (await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload
                .GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (frame.Batch is { } batch) {
                entries.AddRange(batch.Entries);
                Check(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId,
                    identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original format metadata credit");
            }
            if (frame.Terminal is not { } terminal) continue;
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null,
                "actual format read cleanup " + name + "/" + terminal.Code);
            var files = entries.Where(entry => entry.Kind == "File").OrderBy(entry => entry.Ordinal).ToArray();
            Check(files.Length == expected.Count && files.Select(file => file.Ordinal).Distinct().Count() == files.Length &&
                files.Select(file => file.Name).ToHashSet(StringComparer.Ordinal).SetEquals(expected.Keys) &&
                files.All(file => file.Size == (ulong)expected[file.Name].Length), "actual ordinal/name/size catalog matches independent bytes");
            Check(terminal.Browser!.Format == (name == "splitzip" ? "Zip" : name == "sevenz-solid" ? "SevenZ" : name == "rar5-solid" ? "Rar5" : name == "targz" ? "TarGz" : "TarXz"),
                "actual format identity, not a relabelled archive");
            return (terminal.Browser!, files);
        }
    }

    private static FormatOutput[] FormatOutputs(string root)
    {
        var cache = Path.Combine(root, "archive", "sessions");
        if (!Directory.Exists(cache)) return [];
        return Directory.GetFiles(cache, "*", SearchOption.AllDirectories).Select(path => {
            var parts = Path.GetRelativePath(cache, path).Replace('\\', '/').Split('/', 3);
            return parts.Length == 3 && Guid.TryParseExact(parts[0], "D", out _) && parts[1] == "files"
                ? new FormatOutput(path, parts[2]) : null;
        }).OfType<FormatOutput>().ToArray();
    }

    private static void CheckFormatOutputs(string root, IEnumerable<ArchiveReadEntry> demanded, Dictionary<string, byte[]> expected)
    {
        var names = demanded.Select(file => file.Name).ToHashSet(StringComparer.Ordinal);
        var actual = FormatOutputs(root);
        Check(actual.Length == names.Count && actual.Select(file => file.Name).ToHashSet(StringComparer.Ordinal).SetEquals(names),
            "exact demanded outputs only, including solid predecessors and compound spool members");
        Check(actual.All(file => File.ReadAllBytes(file.Path).SequenceEqual(expected[file.Name])), "native published output exact independent bytes");
        Check(!Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "no member partial remains at ready");
        Check(Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Length == names.Count, "one original cache lease per distinct demand");
    }

    private static long FormatStreamSize(ref STGMEDIUM medium)
    {
        Check(medium.tymed == TYMED.TYMED_ISTREAM && medium.unionmember != IntPtr.Zero, "actual native stream medium");
        var stream = (IStream)Marshal.GetObjectForIUnknown(medium.unionmember);
        try { stream.Stat(out var stat, 1); return stat.cbSize; }
        finally { Marshal.ReleaseComObject(stream); }
    }

    private static void RequireFormatWork(Events log, string key, ulong expected)
    {
        var actual = Work(log, key);
        Check(actual is not null, "archive.worker.work must expose measured " + key);
        Check(actual == expected, "actual archive.worker.work " + key + ": expected " + expected + ", observed " + actual);
    }

    private static async Task DragFormat(string root, string source, string helper, string name, string order,
        ArchiveBrowserContext browser, ArchiveReadEntry[] files, Dictionary<string, byte[]> expected)
    {
        // Reverse descriptor order and change display names so lindex/name cannot masquerade as ordinal.
        var descriptorFiles = files.Reverse().ToArray();
        var session = Guid.NewGuid();
        var command = new ArchiveVirtualDragCommand(1, Guid.NewGuid(), session, 1, "start", 42,
            descriptorFiles.Select((file, index) => new ArchiveVirtualDragItem($"export-{index}.bin", file.Size,
                new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                    Action = "materialize", Path = source, Ordinal = file.Ordinal, Browser = browser, CopyMotw = false })).ToArray());
        int[] ranks = files.Length == 4 ? order switch {
            "forward" => [0, 1, 2, 3, 0, 3, 1, 2],
            "backward" => [3, 2, 1, 0, 3, 0, 2, 1],
            _ => [1, 3, 2, 1, 3, 2] // Never demand the first member, even when its dictionary is needed.
        } : order switch {
            "forward" => [0, 1, 0, 1, 1, 0],
            "backward" => [1, 0, 1, 0, 0, 1],
            _ => [1, 0, 0, 1, 0, 1]
        };
        var indices = ranks.Select(rank => Array.FindIndex(descriptorFiles, file => file.Ordinal == files[rank].Ordinal)).ToArray();
        var demanded = new HashSet<ulong>();
        var log = new Events(); var clock = Stopwatch.StartNew();
        var loopReturned = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        STGMEDIUM held = default; IntPtr capability = IntPtr.Zero;
        bool receiverStarted = false, receiverEnded = false;
        Exception? consumerFailure = null; Process? process = null;
        string? heldPath = null; double firstReadyMs = 0, receiverEndMs = 0, cleanupMs = 0;
        var readyMs = new List<double>();
        ArchiveEntryCache.VerificationCounts counts = default;
        try {
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log);
            var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(entries, "runner");
            var cache = (ArchiveEntryCache)ArchiveResourceProbe.Field(entries, "cache");
            await cache.StartupCleanup.WaitAsync(TimeSpan.FromSeconds(30));
            await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
                prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                    var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                    try {
                        var get = Slot<NativeGetData>(pointer, 3);
                        var descriptor = Format(ArchiveVirtualFileDataObject.DescriptorFormat, -1, TYMED.TYMED_HGLOBAL);
                        Check(get(pointer, ref descriptor, out var group) == 0, "actual format descriptor GetData");
                        try {
                            var address = GlobalLock(group.unionmember);
                            try { Check(address != IntPtr.Zero && Marshal.ReadInt32(address) == files.Length, "frozen native descriptor count"); }
                            finally { if (address != IntPtr.Zero) GlobalUnlock(group.unionmember); }
                        } finally { ReleaseStgMedium(ref group); }
                        foreach (var index in indices.Distinct()) {
                            var query = Format(ArchiveVirtualFileDataObject.ContentsFormat, index, TYMED.TYMED_ISTREAM);
                            Check(data.QueryGetData(ref query) == 0, "query is metadata only");
                        }
                        Check(runner.ActiveCount == 0 && log.Named("archive.worker.started").Length == 0 &&
                            FormatOutputs(root).Length == 0 && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any(),
                            "descriptors and queries start no worker or cache output");
                        capability = Capability(pointer);
                        Check(Slot<Start>(capability, 5)(capability, IntPtr.Zero) == 0, "actual receiver StartOperation");
                        receiverStarted = true;
                        foreach (var (index, position) in indices.Select((index, position) => (index, position))) {
                            var selected = descriptorFiles[index]; var bytes = expected[selected.Name];
                            var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, index, TYMED.TYMED_ISTREAM);
                            var startMs = clock.Elapsed.TotalMilliseconds;
                            Check(get(pointer, ref format, out var medium) == 0, "controlled COM format demand " + name + "/" + order + "/" + index);
                            try {
                                Check(FormatStreamSize(ref medium) == bytes.Length, "native Stat uses sealed actual size");
                                if (position == 0) {
                                    held = medium; medium = default;
                                    Check(ReadBytes(ref held, 1)[0] == bytes[0], "held target cursor prefix");
                                    firstReadyMs = clock.Elapsed.TotalMilliseconds - startMs;
                                    process = ArchiveResourceProbe.Worker(runner, command.OperationId, helper);
                                    Check(!process.HasExited, "actual original worker process alive at first ready");
                                    var protectedSource = false;
                                    try { using var exclusive = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None); }
                                    catch (IOException failure) when ((failure.HResult & 0xffff) is 32 or 33) { protectedSource = true; }
                                    Check(protectedSource, "actual source remains held at first ready");
                                    Check(get(pointer, ref format, out var repeat) == 0, "same-index independent native cursor");
                                    try {
                                        Check(FormatStreamSize(ref repeat) == bytes.Length && ReadBytes(ref repeat, bytes.Length).SequenceEqual(bytes),
                                            "repeat starts at zero without moving held target cursor");
                                    } finally { ReleaseStgMedium(ref repeat); }
                                } else Check(ReadBytes(ref medium, bytes.Length).SequenceEqual(bytes), "exact original ordinal bytes for every new or repeated demand");
                            } finally { if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium); }
                            readyMs.Add(clock.Elapsed.TotalMilliseconds - startMs);
                            demanded.Add(selected.Ordinal);
                            Check(runner.ActiveCount == 1 && WorkerActive(entries, log) && process is not null && !process.HasExited &&
                                log.Named("archive.worker.started").Length == 1 && log.Named("archive.worker.started")[0].Id == command.OperationId &&
                                log.Named("archive.worker.source_acquired").Length == 1 && log.Named("archive.worker.source_acquired")[0].Id == command.OperationId &&
                                WorkerTerminals(log).Length == 0, "one actual retained Run/source, no terminal or released slot at member ready");
                            CheckFormatOutputs(root, files.Where(file => demanded.Contains(file.Ordinal)), expected);
                            if (position == 0) heldPath = FormatOutputs(root).Single().Path;
                        }
                        Check(log.Named("archive.entry.drag.member_ready").Length == demanded.Count, "repeats never rematerialize a member");
                        counts = entries.CacheVerificationForFixture;
                        var sealedBytes = files.Where(file => demanded.Contains(file.Ordinal)).Sum(file => (long)expected[file.Name].Length);
                        var cursorBytes = ranks.Sum(rank => (long)expected[files[rank].Name].Length) + expected[files[ranks[0]].Name].Length;
                        Check(counts.SealHashBytes == sealedBytes && counts.CursorOpens == indices.Length + 1 &&
                            counts.CursorChecks == counts.CursorOpens && counts.CursorHashBytes == cursorBytes,
                            "one Seal per demanded member, complete validation for each actual independent cursor, no size-only cursor");
                        return (0x00040100, 1);
                    } catch (Exception failure) { consumerFailure = failure; return (unchecked((int)0x80004005), 0); }
                    finally { Marshal.Release(pointer); loopReturned.TrySetResult(); }
                })) {
                var terminalTask = drag.Dispatch(Wire(command), 1);
                try {
                    await Task.WhenAny(loopReturned.Task, terminalTask).WaitAsync(TimeSpan.FromSeconds(120));
                    if (!loopReturned.Task.IsCompleted)
                        throw new InvalidOperationException("original drag rejected before controlled COM loop: " + (await terminalTask).Error);
                    Check(consumerFailure is null, "controlled format consumer failed: " + consumerFailure?.Message);
                    var receiverWait = Stopwatch.StartNew();
                    while (log.Named("archive.virtual_drag.receiver_pending").Length == 0 && !terminalTask.IsCompleted) {
                        Check(receiverWait.Elapsed < TimeSpan.FromSeconds(30), "original OLE loop return deadline");
                        await Task.Delay(10);
                    }
                    Check(log.Named("archive.virtual_drag.receiver_pending").Length == 1, "original loop returned with actual async receiver pending");
                    Check(receiverStarted && !terminalTask.IsCompleted && runner.ActiveCount == 1 && process is not null && !process.HasExited,
                        "OLE loop return cannot retire the async receiver or retained worker");
                    receiverEndMs = clock.Elapsed.TotalMilliseconds;
                    Check(Slot<End>(capability, 7)(capability, 0, IntPtr.Zero, 1) == 0, "actual successful receiver EndOperation");
                    receiverEnded = true;
                    var terminal = await terminalTask.WaitAsync(TimeSpan.FromSeconds(30));
                    cleanupMs = clock.Elapsed.TotalMilliseconds - receiverEndMs;
                    Check(terminal.Success && terminal.Data!.Contains("dropped"), "original format drag terminal " + terminal.Error);
                    var dragTerminal = log.Named("archive.virtual_drag.terminal").Single(e => e.Id == command.OperationId);
                    var workerTerminal = WorkerTerminals(log).Single(e => e.Id == command.OperationId);
                    var entryTerminal = log.Named("archive.entry.drag.terminal").Single(e => e.Id == command.OperationId);
                    Check(dragTerminal.Fields.GetProperty("cleanup_complete").GetBoolean() &&
                        workerTerminal.Name == "archive.worker.completed" && workerTerminal.Fields.GetProperty("cleanup_complete").GetBoolean() &&
                        entryTerminal.Fields.GetProperty("cleanup_complete").GetBoolean() && runner.ActiveCount == 0 && process!.HasExited,
                        "real worker exit, entry/drag cleanup terminals and slot zero");
                } finally {
                    if (receiverStarted && !receiverEnded) {
                        Check(Slot<End>(capability, 7)(capability, unchecked((int)0x80004004), IntPtr.Zero, 0) == 0, "failed controlled receiver EndOperation");
                        receiverEnded = true;
                    }
                }
            }
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            cache.Sweep();
            Check(heldPath is not null && File.Exists(heldPath), "original cache cannot remove target-held output during retirement");
            var heldBytes = expected[files[ranks[0]].Name];
            Check(ReadBytes(ref held, heldBytes.Length - 1).SequenceEqual(heldBytes[1..]), "held native cursor survives receiver end and drag disposal at its own position");
            ReleaseStgMedium(ref held); held = default;
            cache.Sweep();
            var cacheRoot = Path.Combine(root, "archive", "sessions");
            Check(!Directory.Exists(cacheRoot) || !Directory.EnumerateFileSystemEntries(cacheRoot).Any(),
                "original cache cleanup after target release removes outputs, manifests, retirement records and lease directories");
            Check(!Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "actual cleanup leaves no partials");
            var measured = new[] { "catalog_loads", "outer_decodes", "sevenz_block_decodes", "rar_decoder_opens", "rar_replayed_entries" }
                .ToDictionary(key => key, key => Work(log, key));
            var report = Path.Combine(root, name == "splitzip" ? "split-results-" + order + ".json" : "format-results.json");
            File.WriteAllText(report, JsonSerializer.Serialize(new {
                format = name, order, indices, demand_ordinals = ranks.Select(rank => files[rank].Ordinal).ToArray(), distinct_members = demanded.Count,
                worker_starts = log.Named("archive.worker.started").Length, source_acquisitions = log.Named("archive.worker.source_acquired").Length,
                first_ready_ms = firstReadyMs, demand_ready_ms = readyMs, receiver_end_to_cleanup_ms = cleanupMs,
                first_ready_worker_alive = true, first_ready_source_protected = true, first_ready_slots = 1,
                first_ready_catalog_loads = (ulong?)null,
                work = measured, work_records = log.Named("archive.worker.work").Select(e => e.Fields).ToArray(), cache = counts,
                worker_exited = process!.HasExited, active_slots = runner.ActiveCount, cleanup_complete = true,
                source_exclusively_reopened = true, held_stream_survived_retirement = true, cache_removed_after_target_release = true,
                unavailable = new {
                    first_ready_catalog_count = "work observation is terminal-only; final catalog_loads is asserted independently",
                    native_internal_source_reads = "not measured", native_internal_block_decodes = "not measured",
                    physical_gesture_ole_and_external_target = "controlled consumer only"
                }
            }, new JsonSerializerOptions { WriteIndented = true }));
            Console.WriteLine("ARCHIVE_OPT04_FORMAT_REPORT " + report);
            // Split admission reads its original volume directory and the normalized ZIP catalog once each.
            RequireFormatWork(log, "catalog_loads", name == "splitzip" ? 2UL : 1UL);
            RequireFormatWork(log, "outer_decodes", name is "targz" or "tarxz" ? 1UL : 0UL);
            // The consuming public BlockDecoder verifies the full shared block once per distinct demand.
            if (name == "sevenz-solid") RequireFormatWork(log, "sevenz_block_decodes", 2);
            if (name == "rar5-solid") {
                // Count actual retained TEST opens and predecessor replays, not UnRAR's internal blocks.
                RequireFormatWork(log, "rar_decoder_opens", order == "forward" ? 1UL : order == "backward" ? 4UL : 2UL);
                RequireFormatWork(log, "rar_replayed_entries", order == "forward" ? 0UL : order == "backward" ? 3UL : 2UL);
            }
        } finally {
            if (held.unionmember != IntPtr.Zero) ReleaseStgMedium(ref held);
            if (capability != IntPtr.Zero) Marshal.Release(capability);
            process?.Dispose();
        }
    }
}
