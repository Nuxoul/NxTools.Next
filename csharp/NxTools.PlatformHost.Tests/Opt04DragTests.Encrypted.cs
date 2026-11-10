using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt04DragTests
{
    // Effects: marked GUID-owned scratch copies of the pinned independent solid encrypted RAR4
    // fixture and explicit current worker; original read/preview/whole TEST authorization/drag/
    // runner/output/cache, controlled COM media and native streams, owned process observation.
    // No GUI, external target, clipboard, registry, service or user archive operations.
    private const string Rar4FixtureCommit = "79c571f845dbdd250d69bff9d93ae034f674135e";
    private static readonly string[] Rar4Members = ["a.txt", "b.txt", "c.txt", "d.txt"];

    private sealed class Rar4Prompts
    {
        internal readonly Channel<ArchiveBrowserPasswordPrompt> Queue = Channel.CreateBounded<ArchiveBrowserPasswordPrompt>(4);
        private int count;
        internal int Count => Volatile.Read(ref count);
        internal Rar4Prompts(ArchiveEntrySession entries)
        {
            entries.PasswordPrompt = envelope => {
                var prompt = envelope.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!;
                Interlocked.Increment(ref count);
                Check(Queue.Writer.TryWrite(prompt), "bounded original encrypted preview/authorization prompt channel");
            };
        }
    }

    internal static async Task SolidRar4EncryptedPreviewGrant()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        var prepared = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT04_RAR4_ROOT");
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrWhiteSpace(prepared) || !Path.IsPathFullyQualified(prepared) ||
            string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit prepared NXTOOLS_ARCHIVE_OPT04_RAR4_ROOT and matching current isolated-codecs NXTOOLS_ARCHIVE_WORKER_FIXTURE required");
        Check(!string.IsNullOrWhiteSpace(scratch) && Path.IsPathFullyQualified(scratch), "absolute PI_SCRATCH_DIR required");
        prepared = Path.GetFullPath(prepared).TrimEnd('\\');
        var leaf = Path.GetFileName(prepared);
        const string prefix = "archive-product-opt04-rar4-";
        Check(string.Equals(Path.GetDirectoryName(prepared), Path.GetFullPath(scratch!).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) &&
            leaf.StartsWith(prefix, StringComparison.Ordinal) && Guid.TryParseExact(leaf[prefix.Length..], "D", out _) &&
            File.Exists(Path.Combine(prepared, "owned-opt04-rar4")), "marked prepared GUID-owned RAR4 root in session scratch");
        using var provenance = JsonDocument.Parse(File.ReadAllText(Path.Combine(prepared, "fixture-provenance.json")));
        var fixture = provenance.RootElement;
        Check(fixture.GetProperty("name").GetString() == "rar4-solid-encrypted" && fixture.GetProperty("archive").GetString() == "source.rar" &&
            fixture.GetProperty("source").GetString() == "https://raw.githubusercontent.com/libarchive/libarchive/" + Rar4FixtureCommit +
                "/libarchive/test/test_read_format_rar4_solid_encrypted.rar.uu", "pinned genuine solid RAR4 fixture provenance");
        var declaration = File.ReadAllText(Path.Combine(prepared, "independent-list.txt"));
        Check(declaration.Contains("Solid = +", StringComparison.Ordinal) && declaration.Contains("Type = Rar", StringComparison.Ordinal), "prepared independent solid RAR declaration");
        var root = Path.Combine(scratch!, "archive-product-opt04-encrypted-" + Guid.NewGuid().ToString("D"));
        Check(!Directory.Exists(root), "fresh owned encrypted fixture root");
        Directory.CreateDirectory(root);
        File.WriteAllText(Path.Combine(root, "owned-opt04-rar4"), "OPT04 solid encrypted RAR4 native fixture");
        ArchiveReadHelper.GuardOwnedRoot(root);
        foreach (var file in new[] { "source.rar", "fixture-provenance.json", "independent-list.txt", "libarchive-COPYING", "upstream-test.c" })
            File.Copy(Path.Combine(prepared, file), Path.Combine(root, file));
        var source = Path.Combine(root, "source.rar");
        Check(new FileInfo(source).Length == fixture.GetProperty("archive_bytes").GetInt64(), "independent prepared RAR4 source size");
        var expected = new Dictionary<string, byte[]>(StringComparer.Ordinal);
        Directory.CreateDirectory(Path.Combine(root, "expected"));
        foreach (var file in fixture.GetProperty("files").EnumerateArray()) {
            var name = file.GetProperty("name").GetString()!;
            Check(Rar4Members.Contains(name, StringComparer.Ordinal), "four fixed independent RAR4 member names");
            var copy = Path.Combine(root, "expected", name);
            File.Copy(Path.Combine(prepared, "expected", name), copy);
            var bytes = File.ReadAllBytes(copy);
            Check(bytes.Length == file.GetProperty("size").GetInt32() && bytes.SequenceEqual(Encoding.ASCII.GetBytes("This is from " + name)), "independently decoded exact RAR4 member bytes");
            expected.Add(name, bytes);
        }
        Check(expected.Count == 4, "four independently decoded solid encrypted members");
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        File.Copy(worker, helper);
        Console.WriteLine("ARCHIVE_OPT04_RAR4_PAIR " + JsonSerializer.Serialize(new {
            root, prepared, worker, worker_bytes = new FileInfo(worker).Length, worker_modified_utc = File.GetLastWriteTimeUtc(worker),
            host = typeof(ArchiveVirtualDragSession).Assembly.Location, tests = typeof(Opt04DragTests).Assembly.Location,
            source_bytes = new FileInfo(source).Length, cache_policy = "no_system_cache_flush"
        }));
        var observations = new List<object>();
        var log = new Events();
        await using (var reads = ArchiveReadSession.ForOwnedTestPackage(root)) {
            var (browser, files) = await BrowseEncryptedRar4(reads, source, expected);
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log);
            var prompts = new Rar4Prompts(entries);
            var template = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = "preview", Path = source, Browser = browser, Ordinal = files.Single(file => file.Name == "a.txt").Ordinal,
                CopyMotw = false, PasswordInteraction = true };
            var preview = await Rar4Prompted(reads, entries, prompts, template);
            Check(preview.Path is not null && preview.LeaseId is not null && File.ReadAllBytes(preview.Path).SequenceEqual(expected["a.txt"]), "ordinary encrypted preview is actually sealed");
            Check(entries.AuthorizationCountForFixture == 0, "successful encrypted preview is not a drag grant");
            observations.Add(await Rar4DeniedDrag(root, entries, log, prompts, template, files, expected));
            var authorize = template with { OperationId = Guid.NewGuid(), Action = "authorize", Ordinal = null };
            CheckFormatOutputs(root, files.Where(file => file.Name == "a.txt"), expected);
            var grant = await Rar4Prompted(reads, entries, prompts, authorize);
            Check(grant.LeaseId == authorize.OperationId && grant.Path is null && entries.AuthorizationCountForFixture == 1,
                "independent whole TEST creates the source/session/generation-bound grant after real cleanup");
            var authorizationTerminal = WorkerTerminals(log).Single(line => line.Id == authorize.OperationId);
            Check(log.Named("archive.worker.started").Count(line => line.Id == authorize.OperationId) == 1 &&
                log.Named("archive.worker.source_acquired").Count(line => line.Id == authorize.OperationId) == 1 &&
                authorizationTerminal.Name == "archive.worker.completed" && authorizationTerminal.Fields.GetProperty("cleanup_complete").GetBoolean() &&
                authorizationTerminal.Fields.GetProperty("files_committed").GetUInt64() == 0, "independent original whole TEST worker/source/cleanup with no output publication");
            CheckFormatOutputs(root, files.Where(file => file.Name == "a.txt"), expected);
            observations.Add(await Rar4AdoptedDrag(root, source, helper, entries, log, prompts, template, preview, authorize, files, expected, revoke: false));
            var nextPreview = template with { OperationId = Guid.NewGuid() };
            var nextReady = await Rar4Prompted(reads, entries, prompts, nextPreview);
            Check(nextReady.Path is not null && File.ReadAllBytes(nextReady.Path).SequenceEqual(expected["a.txt"]), "second ordinary encrypted preview, not an inherited password grant");
            observations.Add(await Rar4AdoptedDrag(root, source, helper, entries, log, prompts, nextPreview, nextReady, authorize, files, expected, revoke: true));
            Check(prompts.Count == 3 && !prompts.Queue.Reader.TryPeek(out _) && entries.AuthorizationCountForFixture == 0 && entries.ActiveWorkerCountForFixture == 0,
                "two preview prompts plus one independent whole TEST prompt; no GetData prompt or live grant/worker remains");
            Check(WorkerTerminals(log).Length == log.Named("archive.worker.started").Length &&
                WorkerTerminals(log).All(line => line.Fields.GetProperty("cleanup_complete").GetBoolean()), "every actual worker start has a real drained cleanup terminal");
        }
        using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        Check(File.ReadAllBytes(source).SequenceEqual(File.ReadAllBytes(Path.Combine(prepared, "source.rar"))), "copied source unchanged");
        File.WriteAllText(Path.Combine(root, "encrypted-results.json"), JsonSerializer.Serialize(new {
            scenarios = 3, format = "Rar4", observations, password_prompts = 3,
            limits = new { native_internal_block_counts = "unobserved", first_ready_catalog_count = "terminal work observation only", physical_target = "controlled consumer only" }
        }, new JsonSerializerOptions { WriteIndented = true }));
        File.Delete(helper);
        Console.WriteLine("ARCHIVE_OPT04_RAR4_REPORT " + Path.Combine(root, "encrypted-results.json"));
    }

    private static async Task<(ArchiveBrowserContext Browser, ArchiveReadEntry[] Files)> BrowseEncryptedRar4(
        ArchiveReadSession reads, string source, Dictionary<string, byte[]> expected)
    {
        var request = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(request); var all = new List<ArchiveReadEntry>();
        Check(reads.Dispatch(request).Success, "original RAR4 metadata admission without password interaction");
        while (true) {
            var envelope = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30));
            Check(envelope.Payload.GetProperty("event").GetString() != "archive_password", "RAR4 filenames are not encrypted; browsing must not ask for a password");
            var frame = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (frame.Batch is { } batch) {
                all.AddRange(batch.Entries);
                Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation,
                    sequence: batch.Sequence, version: 2)).Success, "original RAR4 metadata credit");
            }
            if (frame.Terminal is not { } terminal) continue;
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser?.Format == "Rar4", "actual RAR4 browser source binding and cleanup");
            var files = all.Where(file => file.Kind == "File").OrderBy(file => file.Ordinal).ToArray();
            Check(files.Length == 4 && files.Select(file => file.Name).SequenceEqual(Rar4Members) && files.Select(file => file.Ordinal).Distinct().Count() == 4 &&
                files.All(file => file.Encrypted == true && file.Size == (ulong)expected[file.Name].Length), "actual encrypted catalog ordinals and sizes match independent bytes");
            return (terminal.Browser!, files);
        }
    }

    private static async Task<ArchiveEntryResult> Rar4Prompted(ArchiveReadSession reads, ArchiveEntrySession entries, Rar4Prompts prompts, ArchiveEntryCommand command)
    {
        var before = prompts.Count; var grants = entries.AuthorizationCountForFixture;
        Check(!prompts.Queue.Reader.TryPeek(out _), "no stale password prompt before original entry operation");
        var pending = entries.Dispatch(EntryWire(command));
        var waiting = prompts.Queue.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30));
        await Task.WhenAny(pending, waiting).WaitAsync(TimeSpan.FromSeconds(30));
        Check(waiting.IsCompletedSuccessfully, "actual encrypted operation must issue its own password prompt");
        var prompt = await waiting;
        Check(prompt.OperationId == command.OperationId && prompt.SessionId == command.SessionId && prompt.Generation == command.Generation &&
            prompt.ErrorCode == "PasswordRequired" && entries.AuthorizationCountForFixture == grants, "current original ticket; preview or pending TEST cannot mint a grant");
        // Public libarchive fixture password; only the original transient response adapter receives it.
        var bytes = Encoding.UTF8.GetBytes("password");
        try {
            var answer = ProtocolEnvelope.Create("platform_command", new { command = "archive_browser_password", data = new {
                prompt, password = bytes.Select(value => (uint)value).ToArray()
            } }, prompt.OperationId);
            Check(ArchiveBrowserPassword.Respond(answer, reads, entries), "actual admitted password response through original adapter");
        } finally { CryptographicOperations.ZeroMemory(bytes); }
        var reply = await pending.WaitAsync(TimeSpan.FromSeconds(30));
        var result = JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
        Check(reply.Success && result.Success && result.CleanupComplete && result.LeaseId is not null && entries.ActiveWorkerCountForFixture == 0,
            "original encrypted preview/whole TEST succeeds only after actual cleanup");
        Check(prompts.Count == before + 1 && !prompts.Queue.Reader.TryPeek(out _), "one genuine prompt for this operation");
        return result;
    }

    private static ArchiveVirtualDragCommand Rar4DragCommand(ArchiveEntryCommand template, ArchiveReadEntry[] files, Guid? grant)
        => new(1, Guid.NewGuid(), template.SessionId, template.Generation, "start", 42,
            files.Reverse().Select((file, index) => new ArchiveVirtualDragItem($"export-{index}.txt", file.Size,
                template with { OperationId = Guid.NewGuid(), Action = "materialize", Ordinal = file.Ordinal, PasswordInteraction = false, LeaseId = null })).ToArray(), grant);

    private static int Rar4Index(ArchiveVirtualDragCommand command, ArchiveReadEntry[] files, string name)
        => Array.FindIndex(command.Items!, item => item.Entry.Ordinal == files.Single(file => file.Name == name).Ordinal);

    private static void Rar4CleanTerminal(Events log, Guid operation)
        => Check(log.Named("archive.virtual_drag.terminal").Single(line => line.Id == operation).Fields.GetProperty("cleanup_complete").GetBoolean(), "actual encrypted drag cleanup terminal");

    private static async Task<object> Rar4DeniedDrag(string root, ArchiveEntrySession entries, Events log, Rar4Prompts prompts,
        ArchiveEntryCommand template, ArchiveReadEntry[] files, Dictionary<string, byte[]> expected)
    {
        var command = Rar4DragCommand(template, files, null);
        var promptCount = prompts.Count; Exception? consumerFailure = null; var attempts = 0;
        var startsBefore = log.Named("archive.worker.started").Length;
        await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
            prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                try {
                    var get = Slot<NativeGetData>(pointer, 3);
                    var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, Rar4Index(command, files, "a.txt"), TYMED.TYMED_ISTREAM);
                    for (var repeat = 0; repeat < 2; repeat++) {
                        var result = get(pointer, ref format, out var medium);
                        try { Check(result < 0 && medium.unionmember == IntPtr.Zero, "encrypted preview cannot supply FileContents without a separate valid grant"); }
                        finally { if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium); }
                        attempts++;
                    }
                    return (unchecked((int)0x80004005), 0);
                } catch (Exception failure) { consumerFailure = failure; return (unchecked((int)0x80004005), 0); }
                finally { Marshal.Release(pointer); }
            })) {
            var terminal = await drag.Dispatch(Wire(command), 1).WaitAsync(TimeSpan.FromSeconds(120));
            Check(consumerFailure is null && attempts == 2 && !terminal.Success, "actual no-grant consumer rejects without drag success: " + consumerFailure?.Message);
        }
        var starts = log.Named("archive.worker.started").Count(line => line.Id == command.OperationId);
        var terminals = WorkerTerminals(log).Where(line => line.Id == command.OperationId).ToArray();
        Check(starts <= 1 && log.Named("archive.worker.started").Length == startsBefore + starts && terminals.Length == starts &&
            terminals.All(line => line.Fields.GetProperty("cleanup_complete").GetBoolean()) &&
            entries.ActiveWorkerCountForFixture == 0 && entries.AuthorizationCountForFixture == 0, "no hidden replacement worker or leaked slot/grant after encrypted denial");
        Check(!log.Named("archive.entry.drag.preview_adopted").Any(line => line.Id == command.OperationId) &&
            !log.Named("archive.entry.drag.member_ready").Any(line => line.Id == command.OperationId) && prompts.Count == promptCount && !prompts.Queue.Reader.TryPeek(out _),
            "no unauthorized preview adoption, ready or password prompt from GetData");
        Rar4CleanTerminal(log, command.OperationId);
        CheckFormatOutputs(root, files.Where(file => file.Name == "a.txt"), expected);
        using (File.Open(template.Path!, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        return new { scenario = "preview_without_drag_grant", attempts, worker_starts = starts, cleanup_complete = true,
            preview_adopted = false, password_prompt_delta = prompts.Count - promptCount };
    }

    private static async Task<object> Rar4AdoptedDrag(string root, string source, string helper, ArchiveEntrySession entries, Events log,
        Rar4Prompts prompts, ArchiveEntryCommand previewCommand, ArchiveEntryResult preview, ArchiveEntryCommand authorize,
        ArchiveReadEntry[] files, Dictionary<string, byte[]> expected, bool revoke)
    {
        var command = Rar4DragCommand(previewCommand, files, authorize.OperationId);
        var promptCount = prompts.Count; var startsBefore = log.Named("archive.worker.started").Length;
        var runner = (ArchiveWorkerRunner)ArchiveResourceProbe.Field(entries, "runner");
        var cache = (ArchiveEntryCache)ArchiveResourceProbe.Field(entries, "cache");
        await cache.StartupCleanup.WaitAsync(TimeSpan.FromSeconds(30));
        var loopReturned = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        STGMEDIUM heldPreview = default, heldMember = default; IntPtr capability = IntPtr.Zero;
        Process? process = null; string? memberPath = null; Exception? consumerFailure = null;
        bool started = false, ended = false; var cursors = 0; var rejected = 0;
        ArchiveEntryCache.VerificationCounts cacheCounts = default;
        var priorCounts = entries.CacheVerificationForFixture;
        ArchiveEntryCache.VerificationCounts CaptureCounts() {
            var current = entries.CacheVerificationForFixture;
            return new(current.SealHashBytes - priorCounts.SealHashBytes + expected["a.txt"].Length,
                current.CursorHashBytes - priorCounts.CursorHashBytes, current.CursorChecks - priorCounts.CursorChecks,
                current.CursorOpens - priorCounts.CursorOpens);
        }
        var demanded = new HashSet<string>(StringComparer.Ordinal) { "a.txt" };
        var clock = Stopwatch.StartNew(); double endToCleanup = 0;
        try {
            await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
                prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                    var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                    try {
                        var get = Slot<NativeGetData>(pointer, 3);
                        var descriptor = Format(ArchiveVirtualFileDataObject.DescriptorFormat, -1, TYMED.TYMED_HGLOBAL);
                        Check(get(pointer, ref descriptor, out var group) == 0, "actual encrypted RAR4 descriptor GetData");
                        try { var address = GlobalLock(group.unionmember); try { Check(address != IntPtr.Zero && Marshal.ReadInt32(address) == 4, "four frozen descriptors"); }
                            finally { if (address != IntPtr.Zero) GlobalUnlock(group.unionmember); } } finally { ReleaseStgMedium(ref group); }
                        var a = Format(ArchiveVirtualFileDataObject.ContentsFormat, Rar4Index(command, files, "a.txt"), TYMED.TYMED_ISTREAM);
                        Check(data.QueryGetData(ref a) == 0 && log.Named("archive.worker.started").Length == startsBefore, "descriptor/query does not materialize");
                        capability = Capability(pointer);
                        Check(Slot<Start>(capability, 5)(capability, IntPtr.Zero) == 0, "actual async receiver StartOperation"); started = true;
                        Check(get(pointer, ref a, out heldPreview) == 0 && FormatStreamSize(ref heldPreview) == expected["a.txt"].Length &&
                            ReadBytes(ref heldPreview, 1)[0] == expected["a.txt"][0], "authorized sealed encrypted preview reaches native target cursor"); cursors++;
                        Check(log.Named("archive.entry.drag.preview_adopted").Count(line => line.Id == command.OperationId) == 1 &&
                            log.Named("archive.worker.started").Length == startsBefore && runner.ActiveCount == 0, "exact encrypted preview adoption starts zero new workers");
                        CheckFormatOutputs(root, files.Where(file => file.Name == "a.txt"), expected);
                        var release = previewCommand with { Action = "release", Path = null, Browser = null, Ordinal = null,
                            PasswordInteraction = false, CopyMotw = null, LeaseId = preview.LeaseId };
                        var released = entries.Dispatch(EntryWire(release)).WaitAsync(TimeSpan.FromSeconds(30)).GetAwaiter().GetResult();
                        Check(released.Success && JsonSerializer.Deserialize<ArchiveEntryResult>(released.Data!, JsonDefaults.Options)!.Retained && File.Exists(preview.Path),
                            "preview owner release retains the actual borrowed drag consumer");
                        Check(get(pointer, ref a, out var repeat) == 0, "borrowed cursor survives preview retirement");
                        try { Check(ReadBytes(ref repeat, expected["a.txt"].Length).SequenceEqual(expected["a.txt"]), "independent adopted repeat starts at zero"); cursors++; }
                        finally { ReleaseStgMedium(ref repeat); }
                        if (revoke) {
                            var revokeCommand = new ArchiveEntryCommand { Version = 1, OperationId = authorize.OperationId, SessionId = authorize.SessionId,
                                Generation = authorize.Generation, Action = "revoke", LeaseId = authorize.OperationId };
                            Check(entries.Dispatch(EntryWire(revokeCommand)).GetAwaiter().GetResult().Success && entries.AuthorizationCountForFixture == 0, "actual independent grant revoked after cache adoption");
                            foreach (var name in new[] { "a.txt", "d.txt" }) {
                                var refused = Format(ArchiveVirtualFileDataObject.ContentsFormat, Rar4Index(command, files, name), TYMED.TYMED_ISTREAM);
                                var result = get(pointer, ref refused, out var medium);
                                try { Check(result < 0 && medium.unionmember == IntPtr.Zero, "revoked grant rejects cached repeat and new demand"); rejected++; }
                                finally { if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium); }
                            }
                            cacheCounts = CaptureCounts();
                            Check(cacheCounts.SealHashBytes == expected["a.txt"].Length && cacheCounts.CursorOpens == cursors &&
                                cacheCounts.CursorChecks == cursors && cacheCounts.CursorHashBytes == cursors * (long)expected["a.txt"].Length,
                                "revoked cached/new GetData opens no cursor or additional Seal");
                            Check(log.Named("archive.worker.started").Length == startsBefore && runner.ActiveCount == 0, "revoke cannot launch a replacement worker");
                            CheckFormatOutputs(root, files.Where(file => file.Name == "a.txt"), expected);
                            return (0x00040101, 0);
                        }
                        foreach (var name in new[] { "d.txt", "b.txt", "c.txt", "d.txt", "a.txt", "c.txt", "b.txt" }) {
                            var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, Rar4Index(command, files, name), TYMED.TYMED_ISTREAM);
                            Check(get(pointer, ref format, out var medium) == 0, "actual solid RAR4 demand at its original ordinal");
                            try {
                                Check(FormatStreamSize(ref medium) == expected[name].Length, "actual sealed RAR4 native size"); cursors++;
                                if (name == "d.txt" && heldMember.unionmember == IntPtr.Zero) {
                                    heldMember = medium; medium = default;
                                    Check(ReadBytes(ref heldMember, 1)[0] == expected[name][0], "held newly decoded solid cursor prefix");
                                    process = ArchiveResourceProbe.Worker(runner, command.OperationId, helper);
                                    var protectedSource = false;
                                    try { using var exclusive = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None); }
                                    catch (IOException failure) when ((failure.HResult & 0xffff) is 32 or 33) { protectedSource = true; }
                                    Check(protectedSource && !process.HasExited, "source and actual retained worker remain alive at first uncached ready");
                                    Check(get(pointer, ref format, out var second) == 0, "independent repeated solid native cursor");
                                    try { Check(ReadBytes(ref second, expected[name].Length).SequenceEqual(expected[name]), "exact independent solid repeat bytes"); cursors++; }
                                    finally { ReleaseStgMedium(ref second); }
                                    memberPath = FormatOutputs(root).Single(output => output.Name == name).Path;
                                } else Check(ReadBytes(ref medium, expected[name].Length).SequenceEqual(expected[name]), "exact independently decoded RAR4 bytes for forward/backward/repeated demands");
                            } finally { if (medium.unionmember != IntPtr.Zero) ReleaseStgMedium(ref medium); }
                            demanded.Add(name);
                            CheckFormatOutputs(root, files.Where(file => demanded.Contains(file.Name)), expected);
                            Check(runner.ActiveCount == 1 && process is not null && !process.HasExited &&
                                log.Named("archive.worker.started").Length == startsBefore + 1 && log.Named("archive.worker.started").Count(line => line.Id == command.OperationId) == 1 &&
                                log.Named("archive.worker.source_acquired").Count(line => line.Id == command.OperationId) == 1 && !WorkerTerminals(log).Any(line => line.Id == command.OperationId),
                                "one retained Run/source, no worker terminal or released slot between solid member demands");
                        }
                        Check(log.Named("archive.entry.drag.member_ready").Count(line => line.Id == command.OperationId) == 3, "only uncached D/B/C materialize; repeats and preview A do not");
                        cacheCounts = CaptureCounts();
                        Check(cacheCounts.SealHashBytes == expected.Values.Sum(bytes => (long)bytes.Length) && cacheCounts.CursorOpens == cursors &&
                            cacheCounts.CursorChecks == cursors && cacheCounts.CursorHashBytes == cursors * (long)expected["a.txt"].Length,
                            "one actual Seal per preview/materialized member and full native verification per independent cursor");
                        return (0x00040100, 1);
                    } catch (Exception failure) { consumerFailure = failure; return (unchecked((int)0x80004005), 0); }
                    finally { Marshal.Release(pointer); loopReturned.TrySetResult(); }
                })) {
                var terminalTask = drag.Dispatch(Wire(command), 1);
                try {
                    await Task.WhenAny(loopReturned.Task, terminalTask).WaitAsync(TimeSpan.FromSeconds(120));
                    if (!loopReturned.Task.IsCompleted) throw new InvalidOperationException("encrypted drag rejected before controlled native loop: " + (await terminalTask).Error);
                    Check(consumerFailure is null, "controlled encrypted consumer: " + consumerFailure?.Message);
                    var returned = Stopwatch.StartNew();
                    while (!log.Named("archive.virtual_drag.receiver_pending").Any(line => line.Id == command.OperationId) && !terminalTask.IsCompleted) {
                        Check(returned.Elapsed < TimeSpan.FromSeconds(30), "original receiver-pending deadline"); await Task.Delay(10);
                    }
                    Check(started && !terminalTask.IsCompleted && runner.ActiveCount == (revoke ? 0 : 1), "OLE return retains the actual async receiver and any retained worker");
                    var endMs = clock.Elapsed.TotalMilliseconds;
                    Check(Slot<End>(capability, 7)(capability, revoke ? unchecked((int)0x80004004) : 0, IntPtr.Zero, revoke ? 0u : 1u) == 0, "actual receiver EndOperation"); ended = true;
                    var terminal = await terminalTask.WaitAsync(TimeSpan.FromSeconds(30)); endToCleanup = clock.Elapsed.TotalMilliseconds - endMs;
                    Check(revoke ? !terminal.Success : terminal.Success && terminal.Data!.Contains("dropped"), "truthful encrypted drop/revocation terminal");
                    Rar4CleanTerminal(log, command.OperationId);
                    Check(runner.ActiveCount == 0 && (revoke || process!.HasExited), "actual worker exit and original slot zero before target releases streams");
                } finally {
                    if (started && !ended) {
                        Check(Slot<End>(capability, 7)(capability, unchecked((int)0x80004004), IntPtr.Zero, 0) == 0, "failed controlled receiver EndOperation"); ended = true;
                    }
                }
            }
            Check(prompts.Count == promptCount && !prompts.Queue.Reader.TryPeek(out _), "GetData never prompts for an encrypted preview hit, solid miss or revoked grant");
            var workers = WorkerTerminals(log).Where(line => line.Id == command.OperationId).ToArray();
            Check(workers.Length == (revoke ? 0 : 1) && workers.All(line => line.Name == "archive.worker.completed" && line.Fields.GetProperty("cleanup_complete").GetBoolean()), "one actual successful retained worker cleanup, or no worker for adopted/revoked-only demand");
            Check(log.Named("archive.worker.started").Length == startsBefore + (revoke ? 0 : 1), "no hidden or replacement worker during receiver retirement");
            using (File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
            Check(File.Exists(preview.Path) && (revoke || File.Exists(memberPath)), "target-held adopted and materialized files survive source/session retirement");
            Check(ReadBytes(ref heldPreview, expected["a.txt"].Length - 1).SequenceEqual(expected["a.txt"][1..]), "borrowed target cursor survives preview release, drag retirement and grant revoke");
            if (!revoke) Check(ReadBytes(ref heldMember, expected["d.txt"].Length - 1).SequenceEqual(expected["d.txt"][1..]), "new solid target cursor survives worker cleanup at its independent position");
            ReleaseStgMedium(ref heldPreview); heldPreview = default;
            if (heldMember.unionmember != IntPtr.Zero) { ReleaseStgMedium(ref heldMember); heldMember = default; }
            cache.Sweep();
            var sessions = Path.Combine(root, "archive", "sessions");
            Check(!Directory.Exists(sessions) || !Directory.EnumerateFileSystemEntries(sessions).Any(), "original cache sweep after target release removes lease outputs/manifests/retirement records");
            Check(!Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "actual encrypted cleanup leaves no partials");
            var work = log.Named("archive.worker.work").Where(line => line.Id == command.OperationId).ToArray();
            if (!revoke) {
                Check(work.Length == 1, "one actual retained RAR4 work record");
                Check(work[0].Fields.TryGetProperty("catalog_loads", out var catalogs) && catalogs.GetUInt64() == 1 &&
                    work[0].Fields.TryGetProperty("rar_decoder_opens", out var opens) && opens.GetUInt64() == 2 &&
                    work[0].Fields.TryGetProperty("rar_replayed_entries", out var replayed) && replayed.GetUInt64() == 1,
                    "measured retained RAR4 D-to-B-to-C work: catalog1, TEST decoder opens2, one replayed predecessor");
                Check(log.Named("archive.entry.drag.terminal").Single(line => line.Id == command.OperationId).Fields.GetProperty("cleanup_complete").GetBoolean(), "actual original retained entry terminal");
            } else Check(work.Length == 0 && rejected == 2, "revoke has no decoder work and rejects both cached and uncached GetData");
            return new { scenario = revoke ? "revoke_after_encrypted_preview_adoption" : "encrypted_preview_then_solid_d_b_c",
                native_cursors = cursors, rejected_getdata = rejected, distinct_members = demanded.Count,
                worker_starts = log.Named("archive.worker.started").Count(line => line.Id == command.OperationId),
                source_acquisitions = log.Named("archive.worker.source_acquired").Count(line => line.Id == command.OperationId),
                work_records = work.Select(line => line.Fields).ToArray(), password_prompt_delta = prompts.Count - promptCount,
                cache_before_retirement = cacheCounts,
                receiver_end_to_cleanup_ms = endToCleanup, cleanup_complete = true, active_slots = runner.ActiveCount,
                held_streams_survived = true, source_exclusively_reopened = true, cache_removed_after_target_release = true };
        } finally {
            if (heldPreview.unionmember != IntPtr.Zero) ReleaseStgMedium(ref heldPreview);
            if (heldMember.unionmember != IntPtr.Zero) ReleaseStgMedium(ref heldMember);
            if (capability != IntPtr.Zero) Marshal.Release(capability);
            process?.Dispose();
        }
    }
}
