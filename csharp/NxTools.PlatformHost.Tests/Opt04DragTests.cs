using System.Collections;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

// Effects: explicit current worker, GUID-owned session-scratch ZIP, original read/drag/entry/runner/output/cache,
// controlled native COM GetData/IStream and owned JSON. No physical gesture, Shell target or user source.
internal static partial class Opt04DragTests
{
    private static readonly Guid AsyncId = new("3D8B0590-F691-11d2-8EA9-006097DF5BD4");
    private const int Selected = 16, MemberSize = 256 * 1024;
    private sealed record Event(string Name, Guid? Id, JsonElement Fields, double AtMs);
    private sealed class Events : IPlatformLog
    {
        internal readonly ConcurrentQueue<Event> Lines = new();
        private readonly Stopwatch clock = Stopwatch.StartNew();
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Lines.Enqueue(new(name, correlationId, JsonSerializer.SerializeToElement(fields), clock.Elapsed.TotalMilliseconds));
        internal Event[] Named(string name) => Lines.Where(e => e.Name == name).ToArray();
    }
    private sealed record Observation(string Scenario, int Trial, double DescriptorMs, double QueryMs,
        double? FirstReadyMs, double? AllReadyMs, double? ReceiverEndToCleanupMs,
        int WorkerStarts, int SourceAcquisitions, ulong? CatalogLoads, ulong? OuterDecodes,
        int WorkerTerminals, int? WorkerTerminalsAtFirstReady, bool? FirstReadyWorkerActive,
        int OutputFilesAtReady, long OutputBytesAtReady, JsonElement[] WorkRecords,
        ArchiveEntryCache.VerificationCounts Cache, bool CleanupComplete, bool SourceExclusivelyReopened);

    private static byte[] Payload(int index) => Enumerable.Range(0, index == Selected ? 16 << 20 : MemberSize)
        .Select(i => (byte)((i * 31 + index * 17) % 251)).ToArray();
    private static string Name(int index) => $"member-{index:D2}.bin";
    private static string Root(bool retained)
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("PI_SCRATCH_DIR required");
        var configured = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT04_ROOT");
        if (retained && string.IsNullOrWhiteSpace(configured)) throw new InvalidOperationException("Set NXTOOLS_ARCHIVE_OPT04_ROOT to the completed baseline root");
        var root = string.IsNullOrWhiteSpace(configured)
            ? Path.Combine(scratch, "archive-product-opt04-" + Guid.NewGuid().ToString("D")) : Path.GetFullPath(configured);
        Check(string.Equals(Path.GetDirectoryName(root), Path.GetFullPath(scratch).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) &&
            Path.GetFileName(root).StartsWith("archive-product-opt04-", StringComparison.Ordinal) &&
            Guid.TryParseExact(Path.GetFileName(root)["archive-product-opt04-".Length..], "D", out _), "GUID-owned PI_SCRATCH_DIR root");
        if (retained) Check(File.Exists(Path.Combine(root, "owned-opt04-fixture")) && File.Exists(Path.Combine(root, "measurements-before.json")), "completed baseline required");
        else {
            Check(!File.Exists(Path.Combine(root, "measurements-before.json")), "completed baseline evidence must not be overwritten");
            if (Directory.Exists(root)) Check(File.Exists(Path.Combine(root, "owned-opt04-fixture")), "existing root must be fixture-owned");
            Directory.CreateDirectory(root); File.WriteAllText(Path.Combine(root, "owned-opt04-fixture"), "OPT04 retained ZIP fixture");
        }
        ArchiveReadHelper.GuardOwnedRoot(root);
        var source = Path.Combine(root, "sixteen.zip");
        if (!File.Exists(source)) {
            Check(!retained, "retained comparison cannot regenerate source");
            using var zip = ZipFile.Open(source, ZipArchiveMode.Create);
            for (var i = 0; i <= Selected; i++) {
                var entry = zip.CreateEntry(Name(i), CompressionLevel.NoCompression);
                entry.LastWriteTime = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero);
                using var stream = entry.Open(); stream.Write(Payload(i));
            }
        }
        using (var zip = ZipFile.OpenRead(source)) {
            Check(zip.Entries.Count == Selected + 1, "16 selected + 16MiB unselected source");
            for (var i = 0; i <= Selected; i++) {
                Check(zip.Entries[i].FullName == Name(i), "original ordinal and name");
                using var input = zip.Entries[i].Open(); using var output = new MemoryStream(); input.CopyTo(output);
                Check(output.ToArray().SequenceEqual(Payload(i)), "source fixture byte identity");
            }
        }
        return root;
    }
    private static string InstallWorker(string root)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit freshly built matching NXTOOLS_ARCHIVE_WORKER_FIXTURE required");
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        Check(!File.Exists(helper), "previous owned helper not retired");
        File.Copy(worker, helper);
        Console.WriteLine("ARCHIVE_OPT04_PAIR " + JsonSerializer.Serialize(new {
            root, configuration = "Debug", host = typeof(ArchiveVirtualDragSession).Assembly.Location,
            tests = typeof(Opt04DragTests).Assembly.Location, worker, worker_bytes = new FileInfo(worker).Length,
            worker_modified_utc = File.GetLastWriteTimeUtc(worker), source = Path.Combine(root, "sixteen.zip"),
            source_bytes = new FileInfo(Path.Combine(root, "sixteen.zip")).Length, cache_policy = "no_system_cache_flush"
        }));
        return helper;
    }
    private static async Task<ArchiveBrowserContext> Browse(string root, string? source = null)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var request = ArchiveProductReadTests.Command(path: source ?? Path.Combine(root, "sixteen.zip"), version: 2, limits: new() { BrowserContext = true });
        var id = ArchiveReadWire.Parse(request);
        Check(read.Dispatch(request).Success, "original read admission");
        while (true) {
            var frame = (await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (frame.Batch is { } batch) Check(read.Dispatch(ArchiveProductReadTests.Command("ack", id.OperationId, id.SessionId, id.Generation, sequence: batch.Sequence, version: 2)).Success, "read credit");
            if (frame.Terminal is { } end) { Check(end.Status == "succeeded" && end.CleanupComplete && end.Browser is not null, "real read cleanup"); return end.Browser!; }
        }
    }
    private static ArchiveVirtualDragCommand Command(ArchiveBrowserContext browser, string root)
    {
        var session = Guid.NewGuid(); var source = Path.Combine(root, "sixteen.zip");
        return new(1, Guid.NewGuid(), session, 1, "start", 42,
            Enumerable.Range(0, Selected).Select(i => new ArchiveVirtualDragItem(Name(i), (ulong)MemberSize,
                new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                    Action = "materialize", Path = source, Ordinal = (ulong)i, Browser = browser, CopyMotw = false })).ToArray());
    }
    private static ProtocolEnvelope Wire(ArchiveVirtualDragCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_virtual_drag", data = value }, value.OperationId);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int NativeGetData(IntPtr self, ref FORMATETC format, out STGMEDIUM medium);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int Start(IntPtr self, IntPtr reserved);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)] private delegate int End(IntPtr self, int result, IntPtr reserved, uint effect);
    private static T Slot<T>(IntPtr pointer, int number) where T : Delegate => Marshal.GetDelegateForFunctionPointer<T>(Marshal.ReadIntPtr(Marshal.ReadIntPtr(pointer), number * IntPtr.Size));
    [DllImport("kernel32.dll", ExactSpelling = true)] private static extern IntPtr GlobalLock(IntPtr memory);
    [DllImport("kernel32.dll", ExactSpelling = true)] private static extern bool GlobalUnlock(IntPtr memory);
    private static IntPtr Capability(IntPtr data)
    {
        var id = AsyncId; Check(Marshal.QueryInterface(data, ref id, out var result) == 0, "native IDataObjectAsyncCapability QI"); return result;
    }
    private static byte[] ReadBytes(ref STGMEDIUM medium, int length)
    {
        var stream = (IStream)Marshal.GetObjectForIUnknown(medium.unionmember);
        try {
            var result = new byte[length]; var actual = Marshal.AllocHGlobal(4);
            try { stream.Read(result, length, actual); Check(Marshal.ReadInt32(actual) == length, "complete native stream read"); return result; }
            finally { Marshal.FreeHGlobal(actual); }
        } finally { Marshal.ReleaseComObject(stream); }
    }
    private static string[] Outputs(string root)
    {
        var sessions = Path.Combine(root, "archive", "sessions");
        return Directory.Exists(sessions) ? Directory.GetFiles(sessions, "member-*.bin", SearchOption.AllDirectories) : [];
    }
    private static bool WorkerActive(ArchiveEntrySession entries, Events log)
    {
        var runner = ArchiveResourceProbe.Field(entries, "runner");
        lock (ArchiveResourceProbe.Field(runner, "gate")) {
            var active = (IDictionary)ArchiveResourceProbe.Field(runner, "active");
            return log.Named("archive.worker.started").Any(e => e.Id is { } id && active.Contains(id));
        }
    }
    private static ulong? Work(Events log, string key)
    {
        var work = log.Named("archive.worker.work");
        if (work.Length == 0 || work.Any(e => !e.Fields.TryGetProperty(key, out _))) return null;
        return work.Aggregate(0UL, (sum, e) => sum + e.Fields.GetProperty(key).GetUInt64());
    }
    private static Event[] WorkerTerminals(Events log) => log.Lines.Where(e => e.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled").ToArray();
    private static async Task<Observation> Drag(string root, ArchiveBrowserContext browser, string scenario, int trial)
    {
        var log = new Events(); var command = Command(browser, root); int[] demanded = scenario == "descriptor_cancel" ? [] :
            scenario == "1" ? [15] : scenario == "2" ? [15, 0] :
            Enumerable.Range(0, Selected / 2).SelectMany(i => new[] { Selected - 1 - i, i }).ToArray();
        STGMEDIUM held = default; IntPtr capability = IntPtr.Zero;
        using var loopReturned = new ManualResetEventSlim();
        var clock = Stopwatch.StartNew(); double descriptorMs = 0, queryMs = 0; double? first = null, all = null, endToCleanup = null;
        bool? firstActive = null; int? firstTerminals = null; int peakFiles = 0; long peakBytes = 0; var receiverEnd = 0.0;
        ArchiveEntryCache.VerificationCounts counts = default;
        bool cleanup = false, reopened = false;
        await using (var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log)) {
            await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
                prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, source) => {
                    var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                    try {
                        var get = Slot<NativeGetData>(pointer, 3);
                        var descriptor = Format(ArchiveVirtualFileDataObject.DescriptorFormat, -1, TYMED.TYMED_HGLOBAL);
                        var start = clock.Elapsed.TotalMilliseconds;
                        Check(get(pointer, ref descriptor, out var group) == 0, "native descriptor GetData");
                        try { var address = GlobalLock(group.unionmember); try { Check(Marshal.ReadInt32(address) == Selected, "frozen descriptor count"); }
                            finally { GlobalUnlock(group.unionmember); } } finally { ReleaseStgMedium(ref group); }
                        descriptorMs = clock.Elapsed.TotalMilliseconds - start;
                        start = clock.Elapsed.TotalMilliseconds;
                        var query = Format(ArchiveVirtualFileDataObject.ContentsFormat, 15, TYMED.TYMED_ISTREAM);
                        Check(data.QueryGetData(ref query) == 0, "native format query without demand");
                        queryMs = clock.Elapsed.TotalMilliseconds - start;
                        Check(!log.Named("archive.worker.started").Any() && Outputs(root).Length == 0, "descriptors and query do not start materialization");
                        if (demanded.Length == 0) return (source.QueryContinueDrag(true, 1), 0);
                        capability = Capability(pointer);
                        Check(Slot<Start>(capability, 5)(capability, IntPtr.Zero) == 0, "receiver StartOperation before OLE return");
                        var demandStart = clock.Elapsed.TotalMilliseconds;
                        foreach (var (index, position) in demanded.Select((index, position) => (index, position))) {
                            var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, index, TYMED.TYMED_ISTREAM);
                            start = clock.Elapsed.TotalMilliseconds;
                            Check(get(pointer, ref format, out var medium) == 0, "native indexed GetData " + index);
                            try {
                                if (position == 0) {
                                    held = medium; medium = default;
                                    Check(ReadBytes(ref held, 1)[0] == Payload(index)[0], "first transferred cursor prefix");
                                    first = clock.Elapsed.TotalMilliseconds - start;
                                    firstActive = WorkerActive(entries, log);
                                    firstTerminals = WorkerTerminals(log).Length;
                                    // A repeat must not move the transferred cursor or decode another member.
                                    Check(get(pointer, ref format, out var repeat) == 0, "repeated native GetData");
                                    try { Check(ReadBytes(ref repeat, MemberSize).SequenceEqual(Payload(index)), "independent repeat bytes"); }
                                    finally { ReleaseStgMedium(ref repeat); }
                                } else Check(ReadBytes(ref medium, MemberSize).SequenceEqual(Payload(index)), "out-of-order ordinal bytes " + index);
                            } finally { if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium); }
                        }
                        all = clock.Elapsed.TotalMilliseconds - demandStart;
                        var files = Outputs(root);
                        peakFiles = files.Length; peakBytes = files.Sum(file => new FileInfo(file).Length);
                        Check(files.Length == demanded.Length && files.All(file => demanded.Contains(int.Parse(Path.GetFileNameWithoutExtension(file)["member-".Length..]))), "no undemanded native cache output");
                        Check(files.All(file => File.ReadAllBytes(file).SequenceEqual(Payload(int.Parse(Path.GetFileNameWithoutExtension(file)["member-".Length..])))), "published native output bytes");
                        counts = entries.CacheVerificationForFixture;
                        return (0x00040100, 1);
                    } finally { Marshal.Release(pointer); loopReturned.Set(); }
                })) {
                var terminalTask = drag.Dispatch(Wire(command), 1);
                try {
                    var arrived = Task.Run(() => loopReturned.Wait(TimeSpan.FromSeconds(120)));
                    if (await Task.WhenAny(arrived, terminalTask) == terminalTask && !loopReturned.IsSet)
                        throw new InvalidOperationException("native drag rejected before controlled OLE loop: " + (await terminalTask).Error);
                    Check(await arrived, "original OLE loop deadline");
                    if (demanded.Length != 0) {
                        Check(!terminalTask.IsCompleted, "receiver lease remains after OLE return");
                        Check(capability != IntPtr.Zero && Slot<End>(capability, 7)(capability, 0, IntPtr.Zero, 1) == 0, "real receiver EndOperation");
                        receiverEnd = clock.Elapsed.TotalMilliseconds;
                    }
                    var terminal = await terminalTask.WaitAsync(TimeSpan.FromSeconds(30));
                    endToCleanup = demanded.Length == 0 ? null : clock.Elapsed.TotalMilliseconds - receiverEnd;
                    Check(terminal.Success && terminal.Data!.Contains(demanded.Length == 0 ? "cancelled" : "dropped"), "original drag terminal " + terminal.Error);
                    var terminalLog = log.Named("archive.virtual_drag.terminal").Single(e => e.Id == command.OperationId);
                    cleanup = terminalLog.Fields.GetProperty("cleanup_complete").GetBoolean();
                    Check(cleanup, "actual drag cleanup terminal");
                    if (demanded.Length != 0) Check(ReadBytes(ref held, MemberSize - 1).SequenceEqual(Payload(demanded[0])[1..]), "transferred IStream survives receiver End and session retirement");
                } finally {
                    if (capability != IntPtr.Zero) Marshal.Release(capability);
                    if (held.unionmember != IntPtr.Zero) ReleaseStgMedium(ref held);
                }
            }
        }
        Check(!Outputs(root).Any() && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any() &&
            !Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "original owner released outputs and partials");
        using (File.Open(Path.Combine(root, "sixteen.zip"), FileMode.Open, FileAccess.ReadWrite, FileShare.None)) reopened = true;
        var work = Work(log, "catalog_loads");
        var workers = log.Named("archive.worker.started"); var terminals = WorkerTerminals(log);
        Check(terminals.Length == workers.Length && terminals.All(e => e.Fields.GetProperty("cleanup_complete").GetBoolean()), "actual worker exit and cleanup for each start");
        if (demanded.Length > 0) Check(work is not null && workers.Length > 0, "observed native catalog and worker work");
        else Check(workers.Length == 0 && work is null, "descriptor-only must not start worker");
        return new(scenario, trial, descriptorMs, queryMs, first, all, endToCleanup,
            workers.Length, log.Named("archive.worker.source_acquired").Length,
            work, Work(log, "outer_decodes"), terminals.Length, firstTerminals, firstActive,
            peakFiles, peakBytes, log.Named("archive.worker.work").Select(e => e.Fields).ToArray(), counts, cleanup, reopened);
    }
    internal static Task CompletedBaseline() => Measure(false);
    internal static Task RetainedContextLifecycle() => Measure(true);
    private static async Task Measure(bool retained)
    {
        var root = Root(retained); var observations = new List<Observation>();
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit freshly built matching NXTOOLS_ARCHIVE_WORKER_FIXTURE required");
        var workerPath = Path.GetFullPath(worker);
        var testImage = typeof(Opt04DragTests).Assembly.Location;
        var configuration = testImage.Split(Path.DirectorySeparatorChar).Contains("Debug", StringComparer.OrdinalIgnoreCase) ? "Debug" :
            testImage.Split(Path.DirectorySeparatorChar).Contains("Release", StringComparer.OrdinalIgnoreCase) ? "Release" : "unknown";
        Check(configuration != "unknown" && string.Equals(Path.GetFileName(Path.GetDirectoryName(workerPath)), configuration, StringComparison.OrdinalIgnoreCase), "same build mode for host/test and frozen worker");
        if (retained) {
            using var prior = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "measurements-before.json")));
            var pair = prior.RootElement.GetProperty("pair");
            Check(pair.GetProperty("configuration").GetString() == configuration &&
                string.Equals(pair.GetProperty("worker").GetString(), workerPath, StringComparison.OrdinalIgnoreCase) &&
                string.Equals(pair.GetProperty("tests").GetString(), testImage, StringComparison.OrdinalIgnoreCase), "same configuration and worker/test paths as before");
        }
        var helper = InstallWorker(root);
        try {
            var browser = await Browse(root);
            observations.Add(await Drag(root, browser, "descriptor_cancel", 1));
            foreach (var scenario in new[] { "1", "2", "16" })
                for (var trial = 1; trial <= 3; trial++) observations.Add(await Drag(root, browser, scenario, trial));
            var document = new {
                mode = retained ? "retained_context_lifecycle" : "completed_before_baseline", root,
                pair = new { configuration, worker = workerPath, tests = testImage, host = typeof(ArchiveVirtualDragSession).Assembly.Location,
                    worker_bytes = new FileInfo(workerPath).Length, worker_modified_utc = File.GetLastWriteTimeUtc(workerPath),
                    source_bytes = new FileInfo(Path.Combine(root, "sixteen.zip")).Length },
                observations, unavailable = new {
                    gesture_to_ole_ms = (double?)null, gesture_to_ole_reason = "controlled COM loop has no physical GPUI gesture or real DoDragDrop entry",
                    dragover_ms = (double?)null, dragover_reason = "controlled QueryGetData is measured, but no external DragOver callback occurs",
                    source_read_bytes = (long?)null, source_read_reason = "worker work reports revision bytes only, not complete source IO",
                    decoder_resets = (long?)null, decoder_resets_reason = "native worker does not report decoder reset count",
                    target_disk_complete_ms = (double?)null, target_disk_complete_reason = "receiver End is not external target disk publication"
                }
            };
            var report = Path.Combine(root, retained ? "measurements-retained.json" : "measurements-before.json");
            Check(!File.Exists(report), "measurement evidence is immutable per mode");
            File.WriteAllText(report, JsonSerializer.Serialize(document, new JsonSerializerOptions { WriteIndented = true }));
            Console.WriteLine("ARCHIVE_OPT04_REPORT " + report);
            if (retained) foreach (var item in observations.Where(item => item.Scenario != "descriptor_cancel"))
                Check(item.WorkerStarts == 1 && item.SourceAcquisitions == 1 && item.CatalogLoads == 1 &&
                    item.WorkerTerminalsAtFirstReady == 0 && item.FirstReadyWorkerActive == true && item.WorkerTerminals == 1,
                    "one drag-scoped worker/source/catalog with first member ready before real worker cleanup: " + item.Scenario);
        } finally {
            // Keep input and bounded evidence for after comparison. Only this fixture's copied helper is removed.
            if (File.Exists(helper) && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any()) File.Delete(helper);
        }
    }
}
