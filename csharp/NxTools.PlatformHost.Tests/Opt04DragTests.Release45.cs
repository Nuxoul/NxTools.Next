using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt04DragTests
{
    // Effects: read the supplied ZIP without changing it; copy only the explicit worker to a
    // fresh GUID-owned PI_SCRATCH_DIR package. Original Read/Job/Entry/Drag/Run/native output,
    // controlled COM receiver and owned output files. No physical GUI/OLE drop or member execution.
    private const string R45Source = @"D:\Tencent\xiugou-tv-windows-x64-v1.0.3-portable.zip";
    private const long R45Bytes = 206_572_163;
    private const int R45Files = 208;
    private const int R45Buffer = 64 * 1024;
    private sealed record R45Catalog(ArchiveBrowserContext Browser, ArchiveReadEntry[] Files, ArchiveReadEntry[] Directories);

    private static async Task<R45Catalog> R45Browse(string root)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: R45Source, version: 2, limits: new() { BrowserContext = true });
        var id = ArchiveReadWire.Parse(wire);
        var entries = new List<ArchiveReadEntry>();
        Check(read.Dispatch(wire).Success, "original full-archive browser admission");
        while (true)
        {
            var frame = (await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromMinutes(3))).Payload
                .GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            Check(frame.OperationId == id.OperationId, "matching browser operation");
            if (frame.Batch is { } batch)
            {
                entries.AddRange(batch.Entries.Select(entry => entry with { Name = entry.Name.Replace('\\', '/') }));
                Check(read.Dispatch(ArchiveProductReadTests.Command("ack", id.OperationId, id.SessionId, id.Generation,
                    sequence: batch.Sequence, version: 2)).Success, "original metadata credit");
            }
            if (frame.Terminal is not { } end) continue;
            Check(end.Status == "succeeded" && end.CleanupComplete && end.Browser?.Format == "Zip", "real full ZIP read cleanup");
            var files = entries.Where(entry => entry.Kind == "File").OrderBy(entry => entry.Ordinal).ToArray();
            Check(files.Length == R45Files && files.Sum(file => checked((long)(file.Size ?? 0))) == R45Bytes &&
                files.All(file => file.Size is not null) && files.Select(file => file.Ordinal).Distinct().Count() == R45Files,
                "complete 208-file/206572163-byte selection and original ordinals");
            return new(end.Browser!, files, entries.Where(entry => entry.Kind == "Directory").ToArray());
        }
    }

    private static ArchiveVirtualDragCommand R45Command(R45Catalog catalog)
    {
        var session = Guid.NewGuid();
        var wrapper = Path.GetFileNameWithoutExtension(R45Source);
        var actual = catalog.Directories.ToDictionary(directory => directory.Name.TrimEnd('/'), StringComparer.OrdinalIgnoreCase);
        var paths = new HashSet<string>(StringComparer.OrdinalIgnoreCase) { wrapper };
        foreach (var entry in catalog.Files.Concat(catalog.Directories))
        {
            var parts = entry.Name.TrimEnd('/').Split('/');
            for (var i = 1; i < parts.Length; i++) paths.Add(wrapper + "/" + string.Join('/', parts.Take(i)));
            if (entry.Kind == "Directory") paths.Add(wrapper + "/" + entry.Name.TrimEnd('/'));
        }
        var items = new List<ArchiveVirtualDragItem>();
        foreach (var name in paths.OrderBy(path => path.Count(ch => ch == '/')).ThenBy(path => path, StringComparer.Ordinal))
        {
            var relative = name == wrapper ? "" : name[(wrapper.Length + 1)..];
            actual.TryGetValue(relative, out var directory);
            items.Add(new(name, null, new ArchiveEntryCommand {
                Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                Action = "materialize", Path = R45Source, Ordinal = directory?.Ordinal,
                Browser = catalog.Browser, CopyMotw = false
            }, Directory: true));
        }
        foreach (var file in catalog.Files)
            items.Add(new(wrapper + "/" + file.Name, file.Size, new ArchiveEntryCommand {
                Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1,
                Action = "materialize", Path = R45Source, Ordinal = file.Ordinal,
                Browser = catalog.Browser, CopyMotw = false
            }));
        Check(items.Count(item => !item.Directory) == R45Files && items[0].Name == wrapper && items[0].Directory,
            "wrapper-first complete native descriptor plan");
        var command = new ArchiveVirtualDragCommand(1, Guid.NewGuid(), session, 1, "start", 42, items.ToArray());
        Check(ArchiveVirtualDragWire.Parse(ProtocolEnvelope.Create("platform_command",
            new { command = "archive_virtual_drag", data = command }, command.OperationId)).Plan?.Count == items.Count,
            "actual serialized command admits complete Root plan");
        return command;
    }

    private static void R45Compare(Stream actual, Stream expected, long size)
    {
        var left = new byte[R45Buffer]; var right = new byte[R45Buffer];
        long read = 0;
        while (read < size)
        {
            var count = (int)Math.Min(left.Length, size - read);
            var n = actual.Read(left, 0, count);
            Check(n > 0, "truncated output");
            expected.ReadExactly(right.AsSpan(0, n));
            Check(left.AsSpan(0, n).SequenceEqual(right.AsSpan(0, n)), "independent ZIP payload bytes differ");
            read += n;
        }
        Check(actual.ReadByte() == -1 && expected.ReadByte() == -1, "actual and independent ZIP stream end");
    }

    private static void R45CompareOutputs(string root, R45Catalog catalog)
    {
        using var zip = ZipFile.OpenRead(R45Source);
        var expected = zip.Entries.Where(entry => !string.IsNullOrEmpty(entry.Name)).ToDictionary(entry => entry.FullName.Replace('\\', '/'), StringComparer.Ordinal);
        Check(expected.Count == R45Files, "independent ZIP contains the same 208 files");
        var written = 0L;
        foreach (var file in catalog.Files)
        {
            Check(expected.TryGetValue(file.Name, out var member) && member.Length == checked((long)file.Size!.Value), "independent ZIP member identity/size");
            var path = Path.Combine(root, file.Name.Replace('/', Path.DirectorySeparatorChar));
            using var actual = File.OpenRead(path);
            using var decoded = member!.Open();
            R45Compare(actual, decoded, member.Length);
            written += actual.Length;
        }
        Check(written == R45Bytes && Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories).Count() == R45Files,
            "exactly 208 independently verified output files");
    }

    private static object R45Counters(Events log, Guid operation, bool retained)
    {
        var terminal = WorkerTerminals(log).Single(line => line.Id == operation);
        Check(terminal.Name == "archive.worker.completed" && terminal.Fields.GetProperty("cleanup_complete").GetBoolean(),
            "original worker drained with successful terminal");
        var fields = terminal.Fields;
        var work = log.Named("archive.worker.work").Single(line => line.Id == operation).Fields;
        foreach (var key in new[] { "zip_member_decodes", "catalog_loads", "zip_inflate_calls", "zip_inflate_us", "zip_payload_read_us",
            "output_ipc_ack_us", "output_write_calls", "output_write_bytes" })
            Check(work.TryGetProperty(key, out _), "missing original work phase: " + key);
        foreach (var key in new[] { "output_payload_write_calls", "output_payload_write_bytes", "native_write_ms", "native_commit_ms",
            "native_fingerprint_ms", "native_postpublication_ms", "transport_stdout_binary_frames", "transport_payload_buffers_high_water" })
            Check(fields.TryGetProperty(key, out _), "missing native terminal counter: " + key);
        var catalogLoads = work.GetProperty("catalog_loads").GetUInt64();
        Check(work.GetProperty("zip_member_decodes").GetUInt64() == (retained ? 208UL : 215UL) &&
            (retained ? catalogLoads == 1 : catalogLoads is 1 or 2) &&
            work.GetProperty("output_write_bytes").GetUInt64() == R45Bytes &&
            fields.GetProperty("output_payload_write_bytes").GetUInt64() == R45Bytes &&
            work.GetProperty("output_write_calls").GetUInt64() == fields.GetProperty("output_payload_write_calls").GetUInt64() &&
            fields.GetProperty("transport_stdout_binary_frames").GetInt64() == fields.GetProperty("output_payload_write_calls").GetInt64() &&
            fields.GetProperty("transport_payload_buffers_high_water").GetInt32() <= 2,
            "same complete output, route-specific catalog/record work and bounded accounting: " + JsonSerializer.Serialize(new {
                decodes=work.GetProperty("zip_member_decodes"), catalogs=work.GetProperty("catalog_loads"),
                acknowledged_bytes=work.GetProperty("output_write_bytes"), acknowledged_calls=work.GetProperty("output_write_calls"),
                native_bytes=fields.GetProperty("output_payload_write_bytes"), native_calls=fields.GetProperty("output_payload_write_calls"),
                binary_records=fields.GetProperty("transport_stdout_binary_frames"), buffers=fields.GetProperty("transport_payload_buffers_high_water") }));
        return new { work, terminal = fields, archive_output = log.Lines.Where(line => line.Id == operation && line.Name.StartsWith("archive.output.", StringComparison.Ordinal))
            .Select(line => new { line.Name, line.Fields, line.AtMs }).ToArray() };
    }

    private static async Task<object> R45Extract(string root, int trial, R45Catalog catalog)
    {
        var destination = Directory.CreateDirectory(Path.Combine(root, "extract-" + trial)).FullName;
        var log = new Events();
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root, log);
        var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(jobs, "runner");
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
            Action = "start", Request = new ArchiveJobRequest { Kind = "extract", Paths = [R45Source], Destination = destination,
                Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "skip", CopyMotw = false } } };
        var clock = Stopwatch.StartNew();
        Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId)).Success,
            "original ordinary extract Job admitted");
        ArchiveJobTerminal? terminal = null;
        while (terminal is null)
        {
            var frame = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromMinutes(3))).Payload.GetProperty("data")
                .Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Check(frame.OperationId == command.OperationId && frame.Prompt is null, "matching prompt-free original Extract");
            terminal = frame.Terminal;
        }
        Check(await runner.DrainAsync() && runner.ActiveCount == 0, "actual ordinary worker cleanup");
        clock.Stop();
        Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete && terminal.OutputPublished == true &&
            terminal.FilesCommitted == R45Files && terminal.BytesCommitted == R45Bytes && terminal.Journal?.Count == R45Files &&
            terminal.Journal?.All(item => item.Published == true && item.State == "Committed") == true, "all original extract publications");
        R45CompareOutputs(destination, catalog); // Not included in operation wall time.
        var counters = R45Counters(log, command.OperationId, retained: false);
        Directory.Delete(destination, recursive: true); // Fresh fixture-owned target, after completed cleanup and independent validation.
        return new { route = "ordinary_job_extract", trial, operation_id = command.OperationId,
            operation_wall_ms = clock.Elapsed.TotalMilliseconds, terminal.FilesCommitted, terminal.BytesCommitted, terminal.CleanupComplete, counters };
    }

    private static async Task<object> R45Drag(string root, int trial, R45Catalog catalog)
    {
        var destination = Directory.CreateDirectory(Path.Combine(root, "receiver-" + trial)).FullName;
        var command = R45Command(catalog); var log = new Events(); var clock = Stopwatch.StartNew();
        var loopReturned = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        Exception? consumerFailure = null; IntPtr capability = IntPtr.Zero;
        bool started = false, ended = false;
        double firstDemand = 0, firstReady = 0, lastReady = 0, receiverCopy = 0, endToCleanup = 0;
        double receiverCopyTotal = 0, operationWall = 0;
        var contentCount = 0; long contentBytes = 0;
        ArchiveEntryCache.VerificationCounts cache = default;
        await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log);
        var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(entries, "runner");
        await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
            prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                try
                {
                    var get = Slot<NativeGetData>(pointer, 3);
                    var descriptor = Format(ArchiveVirtualFileDataObject.DescriptorFormat, -1, TYMED.TYMED_HGLOBAL);
                    Check(get(pointer, ref descriptor, out var group) == 0, "original native FILEGROUPDESCRIPTORW");
                    try {
                        var address = GlobalLock(group.unionmember);
                        try {
                            Check(address != IntPtr.Zero && Marshal.ReadInt32(address) == command.Items!.Length, "whole Root descriptor count");
                            for (var i = 0; i < command.Items!.Length; i++)
                                Check(Marshal.PtrToStringUni(address + 4 + 592 * i + 72)?.Replace('\\', '/') == command.Items[i].Name,
                                    "wrapper-first original descriptor path/ordinal slot");
                        } finally { if (address != IntPtr.Zero) GlobalUnlock(group.unionmember); }
                    } finally { ReleaseStgMedium(ref group); }
                    Check(log.Named("archive.worker.started").Length == 0, "descriptors do not materialize");
                    capability = Capability(pointer);
                    Check(Slot<Start>(capability, 5)(capability, IntPtr.Zero) == 0, "controlled receiver StartOperation");
                    started = true;
                    foreach (var directory in command.Items!.Where(item => item.Directory))
                        Directory.CreateDirectory(Path.Combine(destination, directory.Name.Replace('/', Path.DirectorySeparatorChar)));
                    var buffer = new byte[R45Buffer];
                    foreach (var (item, index) in command.Items!.Select((item, index) => (item, index)).Where(pair => !pair.item.Directory))
                    {
                        var requested = Format(ArchiveVirtualFileDataObject.ContentsFormat, index, TYMED.TYMED_ISTREAM);
                        var before = clock.Elapsed.TotalMilliseconds;
                        if (contentCount == 0) firstDemand = before;
                        Check(get(pointer, ref requested, out var medium) == 0, "original indexed FileContents demand");
                        try {
                            var ready = clock.Elapsed.TotalMilliseconds;
                            if (contentCount == 0) {
                                firstReady = ready - before;
                                Check(runner.ActiveCount == 1 && WorkerTerminals(log).Length == 0,
                                    "first verified stream precedes actual worker cleanup");
                            }
                            lastReady = ready;
                            var copyClock = Stopwatch.StartNew();
                            Check(medium.tymed == TYMED.TYMED_ISTREAM && medium.unionmember != IntPtr.Zero, "native IStream medium");
                            var native = (IStream)Marshal.GetObjectForIUnknown(medium.unionmember);
                            try {
                                var target = Path.Combine(destination, item.Name.Replace('/', Path.DirectorySeparatorChar));
                                Directory.CreateDirectory(Path.GetDirectoryName(target)!);
                                using var output = new FileStream(target, FileMode.CreateNew, FileAccess.Write, FileShare.None, R45Buffer);
                                var length = checked((long)item.Size!.Value);
                                native.Stat(out var streamStat, 1);
                                Check(streamStat.cbSize == length, "sealed native stream size matches original member identity");
                                var countPointer = Marshal.AllocHGlobal(sizeof(int));
                                try {
                                    long received = 0;
                                    while (received < length) {
                                        var take = (int)Math.Min(buffer.Length, length - received);
                                        native.Read(buffer, take, countPointer);
                                        var count = Marshal.ReadInt32(countPointer);
                                        Check(count > 0 && count <= take, "native content cursor returned bytes");
                                        output.Write(buffer, 0, count);
                                        received += count;
                                    }
                                    native.Read(buffer, 1, countPointer);
                                    Check(Marshal.ReadInt32(countPointer) == 0, "actual native cursor end");
                                    contentBytes += received; contentCount++;
                                } finally { Marshal.FreeHGlobal(countPointer); }
                            } finally { Marshal.ReleaseComObject(native); }
                            receiverCopyTotal += copyClock.Elapsed.TotalMilliseconds;
                        } finally { ReleaseStgMedium(ref medium); }
                    }
                    receiverCopy = clock.Elapsed.TotalMilliseconds - lastReady;
                    Check(contentCount == R45Files && contentBytes == R45Bytes && runner.ActiveCount == 1,
                        "all Root contents copied while one worker still owns Run");
                    cache = entries.CacheVerificationForFixture;
                    return (0x00040100, 1);
                }
                catch (Exception failure) { consumerFailure = failure; return (unchecked((int)0x80004005), 0); }
                finally { Marshal.Release(pointer); loopReturned.TrySetResult(); }
            }))
        {
            var terminalTask = drag.Dispatch(Wire(command), 1);
            try {
                await Task.WhenAny(loopReturned.Task, terminalTask).WaitAsync(TimeSpan.FromMinutes(3));
                if (!loopReturned.Task.IsCompleted)
                    throw new InvalidOperationException("Root admission failed before controlled COM loop: " + (await terminalTask).Error);
                Check(consumerFailure is null, "controlled Root receiver: " + consumerFailure?.Message);
                var receiverWait = Stopwatch.StartNew();
                while (log.Named("archive.virtual_drag.receiver_pending").Length == 0 && !terminalTask.IsCompleted) {
                    Check(receiverWait.Elapsed < TimeSpan.FromSeconds(30), "controlled OLE return must enter receiver-pending phase");
                    await Task.Delay(10);
                }
                Check(started && !terminalTask.IsCompleted && log.Named("archive.virtual_drag.receiver_pending").Length == 1,
                    "async receiver retains original Run beyond OLE return");
                var receiverEnd = clock.Elapsed.TotalMilliseconds;
                Check(Slot<End>(capability, 7)(capability, 0, IntPtr.Zero, 1) == 0, "receiver EndOperation COPY");
                ended = true;
                var terminal = await terminalTask.WaitAsync(TimeSpan.FromMinutes(3));
                endToCleanup = clock.Elapsed.TotalMilliseconds - receiverEnd;
                Check(terminal.Success && terminal.Data!.Contains("dropped") && runner.ActiveCount == 0,
                    "original Root receiver end and worker/entry cleanup: " + terminal.Error);
                Check(log.Named("archive.virtual_drag.terminal").Single(line => line.Id == command.OperationId)
                    .Fields.GetProperty("cleanup_complete").GetBoolean(), "actual drag terminal cleanup");
            }
            finally {
                if (started && !ended)
                    Check(Slot<End>(capability, 7)(capability, unchecked((int)0x80004004), IntPtr.Zero, 0) == 0,
                        "failed controlled receiver retired");
                if (capability != IntPtr.Zero) Marshal.Release(capability);
            }
        }
        operationWall = clock.Elapsed.TotalMilliseconds;
        Check(log.Named("archive.entry.drag.member_ready").Count(line => line.Id == command.OperationId) == R45Files &&
            WorkerTerminals(log).Count(line => line.Id == command.OperationId) == 1 && runner.ActiveCount == 0,
            "208 unique ready members and one actually drained worker");
        R45CompareOutputs(Path.Combine(destination, Path.GetFileNameWithoutExtension(R45Source)), catalog);
        var counters = R45Counters(log, command.OperationId, retained: true);
        Check(cache.CursorOpens == R45Files && cache.SealHashBytes == R45Bytes && cache.CursorHashBytes == R45Bytes,
            "every genuine IStream cursor and Seal validated once");
        Directory.Delete(destination, recursive: true); // Fixture-owned receiver copy only.
        return new { route = "root_virtual_drag_controlled_receiver", trial, operation_id = command.OperationId,
            descriptor_count = command.Items!.Length, first_demand_to_ready_ms = firstReady,
            operation_wall_ms = operationWall, receiver_copy_total_ms = receiverCopyTotal,
            first_demand_to_last_ready_with_interleaved_receiver_copy_ms = lastReady - firstDemand,
            receiver_copy_after_last_ready_ms = receiverCopy, receiver_end_to_cleanup_ms = endToCleanup,
            files = contentCount, bytes = contentBytes, cache, counters,
            cache_lifecycle = log.Lines.Where(line => line.Id == command.OperationId && line.Name.StartsWith("archive.entry.drag.", StringComparison.Ordinal)).Select(line => new { line.Name, line.Fields, line.AtMs }).ToArray(),
            receiver_lifecycle = log.Lines.Where(line => line.Id == command.OperationId && line.Name.StartsWith("archive.virtual_drag.", StringComparison.Ordinal)).Select(line => new { line.Name, line.Fields, line.AtMs }).ToArray() };
    }

    internal static async Task Release45SameSourceMeasurements()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch) && Directory.Exists(scratch),
            "existing absolute PI_SCRATCH_DIR required");
        // GuardOwnedRoot accepts temporary storage. Keep this process's temporary root inside
        // the explicit session scratch for all three original session constructors, then restore it.
        var originalTemp = Environment.GetEnvironmentVariable("TEMP");
        var originalTmp = Environment.GetEnvironmentVariable("TMP");
        Environment.SetEnvironmentVariable("TEMP", scratch);
        Environment.SetEnvironmentVariable("TMP", scratch);
        try
        {
            Check(string.Equals(Path.GetTempPath().TrimEnd('\\'), Path.GetFullPath(scratch!).TrimEnd('\\'),
                StringComparison.OrdinalIgnoreCase), "owned package guard must resolve the session scratch as temporary storage");
        var label = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_R45_LABEL") ?? "baseline";
        Check(label is "baseline" or "32k", "measurement label is baseline or 32k only; not an output policy");
        var trialsText = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_R45_TRIALS");
        var trials = trialsText is null ? 3 : int.TryParse(trialsText, out var selected) && selected is >= 1 and <= 3
            ? selected : throw new InvalidOperationException("R45 trial count must be 1..3");
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        Check(!string.IsNullOrWhiteSpace(worker) && Path.IsPathFullyQualified(worker) && File.Exists(worker),
            "explicit current matching isolated-codecs worker required");
        var configuration = typeof(Opt04DragTests).Assembly.Location.Split(Path.DirectorySeparatorChar).Contains("Release", StringComparer.OrdinalIgnoreCase)
            ? "Release" : "Debug";
        Check(string.Equals(Path.GetFileName(Path.GetDirectoryName(Path.GetFullPath(worker!))), configuration,
            StringComparison.OrdinalIgnoreCase), "worker and test apphost must use the same Debug/Release configuration");
        var source = new FileInfo(R45Source);
        Check(source.Exists && source.Length == 137_737_166, "exact read-only supplied sample size");
        var sourceModified = source.LastWriteTimeUtc;
        var root = Path.Combine(scratch!, "archive-product-r45-" + Guid.NewGuid().ToString("D"));
        Check(!Directory.Exists(root), "fresh owned worker package");
        Directory.CreateDirectory(root);
        ArchiveReadHelper.GuardOwnedRoot(root);
        var image = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        File.Copy(worker!, image);
        var observations = new List<object>();
        var started = DateTimeOffset.UtcNow;
        var catalog = await R45Browse(root);
        _ = R45Command(catalog);
        for (var trial = 1; trial <= trials; trial++) {
            observations.Add(await R45Extract(root, trial, catalog));
            observations.Add(await R45Drag(root, trial, catalog));
        }
        Check(new FileInfo(R45Source).Length == source.Length && File.GetLastWriteTimeUtc(R45Source) == sourceModified,
            "original supplied ZIP remains unchanged");
        Check(!Directory.EnumerateFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "no owned unpublished partial");
        // Failed/incomplete sessions keep their owned helper rooted for actual cleanup.
        File.Delete(image);
        var report = Path.Combine(scratch!, "r45-measurements-" + label + "-" + Guid.NewGuid().ToString("D") + ".json");
        using (var output = new FileStream(report, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            JsonSerializer.Serialize(output, new { schema = 1, label, trials, started_utc = started,
                finished_utc = DateTimeOffset.UtcNow, source_bytes = source.Length, source_modified_utc = sourceModified,
                expanded_files = R45Files, expanded_bytes = R45Bytes,
                configuration,
                worker_path = worker, worker_bytes = new FileInfo(worker!).Length,
                host = typeof(ArchiveVirtualDragSession).Assembly.Location, tests = typeof(Opt04DragTests).Assembly.Location,
                package_root = root, cache_policy = "no_system_cache_flush", observations,
                timing_semantics = "independent ordinary Job wall and controlled Root GetData/receiver clocks; inclusive phase counters not additive; independent ZIP comparisons excluded from measured route",
                physical_target = "not_run" }, new JsonSerializerOptions { WriteIndented = true });
        Console.WriteLine("ARCHIVE_R45_MEASUREMENTS " + report);
        }
        catch (Exception failure) { Console.Error.WriteLine(failure.StackTrace); throw; }
        finally
        {
            Environment.SetEnvironmentVariable("TEMP", originalTemp);
            Environment.SetEnvironmentVariable("TMP", originalTmp);
        }
    }
}
