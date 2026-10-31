using System.Buffers.Binary;
using System.Collections.Concurrent;
using System.IO.Compression;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveVirtualPasswordTests
{
    private static readonly string Scratch = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "phase-f-p208-password-20261023-202345");
    private const string ZipSecret = "synthetic-private-ZIP-secret", WrongSecret = "csharp-password-private-wrong-sentinel";
    private static string Root() => Directory.CreateDirectory(Path.Combine(Scratch, "archive-product-csharp-password-" + Guid.NewGuid())).FullName;
    private static ArchiveEntryCommand Authorize(string path) => new() { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
        Action = "authorize", Path = path, Browser = new(Guid.NewGuid(), "Zip", "", new("v1:fixture", new uint[32], 1, 1)), CopyMotw = false, PasswordInteraction = true };
    private static ProtocolEnvelope Wire(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
    private static ProtocolEnvelope DragWire(ArchiveVirtualDragCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_virtual_drag", data = value }, value.OperationId);
    private static ArchiveEntryCommand Revoke(ArchiveEntryCommand value) => new() { Version = 1, OperationId = value.OperationId, SessionId = value.SessionId, Generation = value.Generation, Action = "revoke", LeaseId = value.OperationId };
    private static ArchiveEntryResult Result(WindowsCommandResult value) => JsonSerializer.Deserialize<ArchiveEntryResult>(value.Data!, JsonDefaults.Options)!;
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Events = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Events.Enqueue(JsonSerializer.Serialize(new { eventName, correlationId, fields }));
    }
    private static async Task Until(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow.AddSeconds(20);
        while (!condition()) { Check(DateTime.UtcNow < deadline, "bounded password fixture deadline"); await Task.Delay(10); }
    }
    private static Channel<ArchiveBrowserPasswordPrompt> Prompts(ArchiveEntrySession entries, ConcurrentQueue<string>? observed = null)
    {
        var queue = Channel.CreateBounded<ArchiveBrowserPasswordPrompt>(4);
        entries.PasswordPrompt = envelope => {
            observed?.Enqueue(envelope.Payload.GetRawText());
            Check(queue.Writer.TryWrite(envelope.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!), "bounded original prompt queue");
        };
        return queue;
    }
    private static bool Answer(ArchiveReadSession reads, ArchiveEntrySession entries, ArchiveBrowserPasswordPrompt prompt, string? password)
    {
        var bytes = password is null ? null : Encoding.UTF8.GetBytes(password);
        try {
            // Exercise the real private-byte public pipe response adapter, not a grant test injector.
            var envelope = ProtocolEnvelope.Create("platform_command", new { command = "archive_browser_password", data = new { prompt, password = bytes?.Select(value => (uint)value).ToArray() } }, prompt.OperationId);
            return ArchiveBrowserPassword.Respond(envelope, reads, entries);
        } finally { if (bytes is not null) CryptographicOperations.ZeroMemory(bytes); }
    }
    private static void CopyWorker(string root)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit matching isolated worker required; preexisting worker is exploratory only");
        File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
    }
    private static string Fixture(string environment, string name)
    {
        var directory = Environment.GetEnvironmentVariable(environment);
        if (string.IsNullOrEmpty(directory) || !File.Exists(Path.Combine(directory, name))) throw new TestSkippedException("Explicit attributed encrypted fixture directory required: " + environment);
        return Path.Combine(directory, name);
    }
    private static async Task<(ArchiveBrowserContext Context, ArchiveReadEntry[] Entries)> Browse(string root, string path, string? password = null, CaptureLog? log = null)
    {
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root, log);
        await using var unusedEntries = ArchiveEntrySession.ForOwnedTestPackage(root, log);
        var wire = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var command = ArchiveReadWire.Parse(wire); var entries = new List<ArchiveReadEntry>();
        Check(reads.Dispatch(wire, browserInteraction: password is not null).Success, "original browser admission");
        while (true) {
            var frame = await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30));
            if (frame.Payload.GetProperty("event").GetString() == "archive_password") {
                Check(Answer(reads, unusedEntries, frame.Payload.GetProperty("data").Deserialize<ArchiveBrowserPasswordPrompt>(JsonDefaults.Options)!, password), "original metadata password route"); continue;
            }
            var item = frame.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch) { entries.AddRange(batch.Entries); Check(reads.Dispatch(ArchiveProductReadTests.Command("ack", command.OperationId, command.SessionId, command.Generation, sequence: batch.Sequence, version: 2)).Success, "original read credit"); }
            if (item.Terminal is { } terminal) {
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "actual original browser source binding: " + terminal.Code);
                return (terminal.Browser!, entries.ToArray());
            }
        }
    }
    private static ArchiveVirtualDragItem[] Items(ArchiveEntryCommand authorization, ArchiveReadEntry[] entries)
    {
        var items = entries.Select(entry => new ArchiveVirtualDragItem(entry.Name.TrimEnd('/'), entry.Kind == "Directory" ? null : entry.Size,
            authorization with { OperationId = Guid.NewGuid(), Action = "materialize", Ordinal = entry.Ordinal, PasswordInteraction = false }, entry.Kind == "Directory")).ToList();
        foreach (var file in items.Where(item => !item.Directory).ToArray()) {
            var slash = file.Name.LastIndexOf('/');
            if (slash > 0) {
                var parent = file.Name[..slash];
                if (!items.Any(item => item.Name == parent)) items.Add(new(parent, null, file.Entry with { OperationId = Guid.NewGuid(), Ordinal = null }, true));
            }
        }
        return items.ToArray();
    }
    private static ArchiveVirtualDragCommand Drag(ArchiveEntryCommand authorization, ArchiveVirtualDragItem[] items, Guid? id) =>
        new(1, Guid.NewGuid(), authorization.SessionId, authorization.Generation, "start", 42, items, id);
    private static void NoOutputs(string root) => Check(!Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any() && !Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "authorize has no cache/output publication");
    // Inspect actual ownership buffers only; never install a synthetic grant or log secret bytes.
    private static byte[] ProtectedBytes(ArchiveEntrySession entries, Guid id)
    {
        const System.Reflection.BindingFlags flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
        var map = (System.Collections.IDictionary)typeof(ArchiveEntrySession).GetField("authorizations", flags)!.GetValue(entries)!;
        var grant = map[id]!; var secret = grant.GetType().GetField("Secret", flags)!.GetValue(grant)!;
        var bytes = (byte[])secret.GetType().GetField("protectedBytes", flags)!.GetValue(secret)!;
        Check(bytes.Length > 0 && bytes.Length % 16 == 0, "same-process protected memory block16"); return bytes;
    }
    private static async Task<ArchiveEntryResult> Unlock(ArchiveReadSession reads, ArchiveEntrySession entries, ArchiveEntryCommand command, string secret, bool wrongFirst = false)
    {
        var prompts = Prompts(entries); var pending = entries.Dispatch(Wire(command));
        var first = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20));
        Check(first.ErrorCode == "PasswordRequired", "whole TEST requires a secret even after metadata open");
        Check(Answer(reads, entries, first, wrongFirst ? WrongSecret : secret), "original admitted UTF8 secret response");
        if (wrongFirst) {
            var second = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20));
            Check(second.ErrorCode == "WrongPassword" && second.Ticket != first.Ticket, "RAR explicit wrong only creates fresh ticket");
            Check(!Answer(reads, entries, first, WrongSecret), "stale ticket cannot replace last admitted secret");
            Check(Answer(reads, entries, second, secret), "current correction");
        }
        var result = Result(await pending.WaitAsync(TimeSpan.FromSeconds(30)));
        Check(result.Success && result.CleanupComplete && result.LeaseId == command.OperationId && result.Path is null, "grant only after actual whole TEST and cleanup: " + result.Code);
        Check(entries.ActiveWorkerCountForFixture == 0 && entries.AuthorizationCountForFixture >= 1, "actual worker cleanup precedes authorization success");
        return result;
    }
    internal static async Task AuthorizeWireAndNoOutputAdmission()
    {
        var root = Root();
        // JSON shape deliberately compiled and failed before authorize/revoke production changes.
        var command = Authorize(Path.Combine(root, "source.zip"));
        Check(ArchiveEntryWire.Parse(Wire(command)).Action == "authorize", "authorize parses on original entry contract");
        foreach (var bad in new[] { command with { PasswordInteraction = false }, command with { Ordinal = 0 }, command with { LeaseId = Guid.NewGuid() }, Revoke(command) with { LeaseId = Guid.NewGuid() } }) {
            try { ArchiveEntryWire.Parse(Wire(bad)); throw new InvalidOperationException("invalid authorization shape accepted"); } catch (InvalidDataException) { }
        }
        Check(ArchiveEntryWire.Parse(Wire(Revoke(command))).Action == "revoke", "opaque ID reuses lease slot only for revoke");
        await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
        var terminal = Result(await entries.Dispatch(Wire(command)).WaitAsync(TimeSpan.FromSeconds(10)));
        Check(!terminal.Success && terminal.Code == "helper_missing" && terminal.CleanupComplete && terminal.LeaseId is null && terminal.Path is null, "authorize reaches original runner, not invalid_request");
        NoOutputs(root);
        var items = new[] { new ArchiveVirtualDragItem("one.txt", 1, command with { OperationId = Guid.NewGuid(), Action = "materialize", Ordinal = 0, PasswordInteraction = false }) };
        var drag = Drag(command, items, Guid.NewGuid());
        Check(ArchiveVirtualDragWire.Parse(DragWire(drag)).AuthorizationId == drag.AuthorizationId, "wire accepts opaque structural ID without pretending lifecycle validity");
        Check(!JsonSerializer.Serialize(Drag(command, items, null), JsonDefaults.Options).Contains("authorization_id", StringComparison.Ordinal), "old plaintext DTO default remains omitted");
        foreach (var bad in new[] { drag with { AuthorizationId = Guid.Empty }, drag with { Action = "cancel", SourceWindowHandle = 0, Items = null } }) {
            try { ArchiveVirtualDragWire.Parse(DragWire(bad)); throw new InvalidOperationException("invalid drag authorization accepted"); } catch (InvalidDataException) { }
        }
        var prepared = 0;
        await using var native = new ArchiveVirtualDragSession(entries, validSource: (_, _) => true, prepare: _ => { prepared++; return null; }, pressed: () => true, loop: (_, _) => throw new InvalidOperationException("OLE must not be reached"));
        Check(!(await native.Dispatch(DragWire(drag), 1)).Success && prepared == 0, "unregistered grant rejected before STA/OLE admission");
        Check(!ArchiveWorkerRunner.ValidPasswordBytes(new byte[] { 0xff }) && !ArchiveWorkerRunner.ValidPasswordBytes(new byte[] { 0 }) && !ArchiveWorkerRunner.ValidPasswordBytes(new byte[4097]), "strict UTF8/NUL/4096-byte validation");
    }
    // Append one ordinary ZIP entry without decoding/re-encoding independent encrypted fixtures.
    // This bounded fixture-only merger copies local records verbatim and relocates central offsets.
    private static void AddPlain(string path)
    {
        var encrypted = File.ReadAllBytes(path); using var memory = new MemoryStream();
        using (var zip = new ZipArchive(memory, ZipArchiveMode.Create, true)) { using var output = zip.CreateEntry("plain.txt").Open(); output.Write("plain"u8); }
        var plain = memory.ToArray();
        static int End(byte[] bytes) { for (var i = bytes.Length - 22; i >= 0; i--) if (BinaryPrimitives.ReadUInt32LittleEndian(bytes.AsSpan(i)) == 0x06054b50) return i; throw new InvalidDataException(); }
        var a = End(encrypted); var b = End(plain); var localA = (int)BinaryPrimitives.ReadUInt32LittleEndian(encrypted.AsSpan(a + 16)); var localB = (int)BinaryPrimitives.ReadUInt32LittleEndian(plain.AsSpan(b + 16));
        var centralB = plain.AsSpan(localB, b - localB).ToArray(); BinaryPrimitives.WriteUInt32LittleEndian(centralB.AsSpan(42), (uint)localA);
        var central = encrypted.AsSpan(localA, a - localA).ToArray().Concat(centralB).ToArray(); var end = encrypted.AsSpan(a).ToArray();
        var count = (ushort)(BinaryPrimitives.ReadUInt16LittleEndian(end.AsSpan(10)) + 1); BinaryPrimitives.WriteUInt16LittleEndian(end.AsSpan(8), count); BinaryPrimitives.WriteUInt16LittleEndian(end.AsSpan(10), count);
        BinaryPrimitives.WriteUInt32LittleEndian(end.AsSpan(12), (uint)central.Length); BinaryPrimitives.WriteUInt32LittleEndian(end.AsSpan(16), (uint)(localA + localB));
        File.WriteAllBytes(path, encrypted.AsSpan(0, localA).ToArray().Concat(plain.AsSpan(0, localB).ToArray()).Concat(central).Concat(end).ToArray());
    }
    private static byte[] ReadBytes(ref STGMEDIUM medium, int count)
    {
        var stream = (IStream)Marshal.GetObjectForIUnknown(medium.unionmember); var read = Marshal.AllocCoTaskMem(4);
        try { var bytes = new byte[count]; stream.Read(bytes, count, read); Check(Marshal.ReadInt32(read) == count, "actual native stream byte count"); return bytes; }
        finally { Marshal.FreeCoTaskMem(read); Marshal.ReleaseComObject(stream); }
    }
    internal static async Task ActualZipGrantLazyMixedTreeAndRetainedStream()
    {
        var root = Root(); CopyWorker(root); var path = Path.Combine(root, "encrypted.zip");
        File.Copy(Fixture("NXTOOLS_ARCHIVE_ZIP_FIXTURES", "independent-ae2-256.zip"), path); AddPlain(path);
        var log = new CaptureLog(); var browse = await Browse(root, path, log: log);
        Check(browse.Entries.Any(e => e.Encrypted == true) && browse.Entries.Any(e => e.Encrypted == false && e.Kind == "File"), "actual mixed plaintext/encrypted archive tree");
        var command = Authorize(path) with { Browser = browse.Context }; STGMEDIUM held = default;
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root, log);
        await using (var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log)) {
        var prompts = Prompts(entries, log.Events); using var cleanupEntered = new ManualResetEventSlim(); using var finishCleanup = new ManualResetEventSlim();
        entries.CleanupCheckpointForFixture(() => { cleanupEntered.Set(); Check(finishCleanup.Wait(TimeSpan.FromSeconds(20)), "bounded cleanup release"); });
        var pending = entries.Dispatch(Wire(command)); var prompt = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20)); NoOutputs(root);
        Check(Answer(reads, entries, prompt, ZipSecret), "original ZIP TEST answer");
        try { await Until(() => cleanupEntered.IsSet); Check(!pending.IsCompleted && entries.AuthorizationCountForFixture == 0, "private complete alone cannot grant before actual cleanup"); }
        finally { finishCleanup.Set(); }
        var grant = Result(await pending.WaitAsync(TimeSpan.FromSeconds(30))); Check(grant.Success && grant.LeaseId == command.OperationId && grant.Path is null && grant.CleanupComplete, "ZIP whole TEST authorization"); NoOutputs(root);
        var items = Items(command, browse.Entries); Check(entries.TryCaptureVirtualAuthorization(command.OperationId, items, out _), "source-bound grant ready");
        Check(ArchiveVirtualDragWire.Parse(DragWire(Drag(command, items, command.OperationId))).Items!.Length == items.Length, "formal encrypted/mixed tree still uses original structural parser");
        var protectedBytes = ProtectedBytes(entries, command.OperationId);
        Check(!Encoding.UTF8.GetString(protectedBytes).Contains(ZipSecret, StringComparison.Ordinal), "grant secret protected at rest, not plaintext bytes");
        var expected = Encoding.UTF8.GetBytes("独立 ZIP crypto fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
        try {
            var loops=0;
            await using (var drag = new ArchiveVirtualDragSession(entries,validSource:(_,_)=>true,prepare:_=>null,pressed:()=>true,escape:()=>false,loop:(data,source)=> {
                Interlocked.Increment(ref loops);
                var descriptor = Format(ArchiveVirtualFileDataObject.DescriptorFormat, -1, TYMED.TYMED_HGLOBAL); data.GetData(ref descriptor, out var group); ReleaseStgMedium(ref group); NoOutputs(root);
                var encryptedIndex = Array.FindIndex(items, item => item.Name.EndsWith("payload.bin", StringComparison.Ordinal)); var plainIndex = Array.FindIndex(items, item => item.Name == "plain.txt");
                var file = Format(ArchiveVirtualFileDataObject.ContentsFormat, encryptedIndex, TYMED.TYMED_ISTREAM); data.GetData(ref file, out held);
                Check(ReadBytes(ref held, 1)[0] == expected[0], "decrypted first byte");
                data.GetData(ref file, out var repeated); try { Check(ReadBytes(ref repeated, expected.Length).SequenceEqual(expected), "actual independent AES bytes and repeat cursor"); } finally { ReleaseStgMedium(ref repeated); }
                var plain = Format(ArchiveVirtualFileDataObject.ContentsFormat, plainIndex, TYMED.TYMED_ISTREAM); data.GetData(ref plain, out var plainStream); try { Check(Read(ref plainStream, 5) == "plain", "mixed plaintext selected bytes"); } finally { ReleaseStgMedium(ref plainStream); }
                Check(entries.Dispatch(Wire(Revoke(command))).GetAwaiter().GetResult().Success, "exact grant revoked");
                Check(protectedBytes.All(value => value == 0), "exact revoke zeroes actual protected-memory array");
                try { data.GetData(ref file,out var late);ReleaseStgMedium(ref late);throw new InvalidOperationException("revoked grant allowed new content"); } catch (ArchiveOutputException) { } catch (OperationCanceledException) { } catch(COMException) { }
                return (0x00040101,0);
            })) {
                var wrong=await drag.Dispatch(DragWire(Drag(command,items,Guid.NewGuid())),1);Check(!wrong.Success&&wrong.Error=="authorization_invalid"&&loops==0,"invalid grant rejected BEFORE STA/OLE loop");
                var terminal=await drag.Dispatch(DragWire(Drag(command,items,command.OperationId)),1).WaitAsync(TimeSpan.FromSeconds(40));Check(terminal.Success&&loops==1,"formal authorized session enters original single dedicated loop");
            }
            Check(ReadBytes(ref held, expected.Length - 1).SequenceEqual(expected.AsSpan(1).ToArray()), "already returned native IStream survives grant revoke and content retirement");
        } finally { if (held.unionmember != IntPtr.Zero) ReleaseStgMedium(ref held); }
        var safe = string.Join("\n", log.Events) + JsonSerializer.Serialize(Drag(command, items, command.OperationId), JsonDefaults.Options);
        Check(!safe.Contains(ZipSecret, StringComparison.Ordinal) && !safe.Contains(WrongSecret, StringComparison.Ordinal) && !safe.Contains("\"password\":", StringComparison.Ordinal), "no secret in safe events/log/public DTO/cache records");
        using (File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        using (File.Open(Path.Combine(root, ArchiveReadHelper.DeploymentName), FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
        Check(entries.ActiveWorkerCountForFixture == 0 && entries.AuthorizationCountForFixture == 0, "actual worker cleanup and revoked map empty");
        }
        var diskRecords=string.Join("\n",Directory.GetFiles(root,"*.json",SearchOption.AllDirectories).Select(File.ReadAllText));
        Check(!diskRecords.Contains(ZipSecret,StringComparison.Ordinal)&&!diskRecords.Contains(WrongSecret,StringComparison.Ordinal),"no secret in disk cache after original owner releases its manifest lock");
    }
    internal static async Task ActualRarTicketsRevokeExpiryReplacementAndSource()
    {
        var root = Root(); CopyWorker(root); var path = Path.Combine(root, "header.rar"); File.Copy(Fixture("NXTOOLS_ARCHIVE_CODEC_FIXTURES", "rar5-header-encrypted.rar"), path);
        var log = new CaptureLog(); var browse = await Browse(root, path, "password", log); var command = Authorize(path) with { Browser = browse.Context };
        var utcTicks = DateTimeOffset.UtcNow.Ticks;
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root, log);
        await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log, () => new DateTimeOffset(Interlocked.Read(ref utcTicks), TimeSpan.Zero));
        Check(entries.AuthorizationCountForFixture == 0, "metadata password must not silently authorize drag");
        await Unlock(reads, entries, command, "password", wrongFirst: true); NoOutputs(root);
        var items = Items(command, browse.Entries); Check(entries.TryCaptureVirtualAuthorization(command.OperationId, items, out var originalToken), "original pipe owns original grant");
        var originalBytes = ProtectedBytes(entries, command.OperationId);
        await using (var replacement = ArchiveEntrySession.ForOwnedTestPackage(root)) Check(!replacement.TryCaptureVirtualAuthorization(command.OperationId, items, out _), "replacement pipe cannot reuse old map");
        foreach (var changed in new[] { items[0].Entry with { SessionId = Guid.NewGuid() }, items[0].Entry with { Generation = 2 }, items[0].Entry with { Path = path + ".other" }, items[0].Entry with { Codepage = 936 }, items[0].Entry with { Depth = 1 }, items[0].Entry with { CopyMotw = true }, items[0].Entry with { Browser = browse.Context with { SnapshotId = Guid.NewGuid() } }, items[0].Entry with { Browser = browse.Context with { Comment = "changed" } }, items[0].Entry with { Browser = browse.Context with { Source = browse.Context.Source with { Digest = new uint[32] } } } })
            Check(!entries.TryCaptureVirtualAuthorization(command.OperationId, [items[0] with { Entry = changed }], out _), "all immutable source/session/context fields bind grant");
        var next = command with { OperationId = Guid.NewGuid() }; await Unlock(reads, entries, next, "password");
        Check(!(await entries.Dispatch(Wire(Revoke(command) with { Generation = 2 }))).Success && entries.AuthorizationCountForFixture == 2, "stale revoke cannot cancel current grants");
        Check((await entries.Dispatch(Wire(Revoke(command)))).Success && originalToken.IsCancellationRequested, "old revoke affects only old grant");
        Check(entries.TryCaptureVirtualAuthorization(next.OperationId, Items(next, browse.Entries), out var nextToken), "old revoke preserves new grant");
        var nextBytes = ProtectedBytes(entries, next.OperationId); Check(originalBytes.All(value => value == 0), "old grant buffer zeroed on revoke");
        Interlocked.Add(ref utcTicks, TimeSpan.FromSeconds(121).Ticks);
        await Until(() => entries.AuthorizationCountForFixture == 0); Check(nextToken.IsCancellationRequested, "background expiry without GetData clears and cancels grant");
        Check(nextBytes.All(value => value == 0), "timer expiry zeroes actual protected-memory array without a GetData request");
        var cancelCommand = command with { OperationId = Guid.NewGuid() }; var prompts = Prompts(entries); var pending = entries.Dispatch(Wire(cancelCommand)); var prompt = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20));
        Check((await entries.Dispatch(Wire(Revoke(cancelCommand)))).Success && !Answer(reads, entries, prompt, "password"), "pending revoke cancels ticket and late secret");
        var cancelled = Result(await pending.WaitAsync(TimeSpan.FromSeconds(30))); Check(!cancelled.Success && cancelled.CleanupComplete && entries.AuthorizationCountForFixture == 0 && entries.ActiveWorkerCountForFixture == 0, "revoked pending authorization actual cleanup/no resurrection");
        var userCancel = command with { OperationId = Guid.NewGuid() }; prompts = Prompts(entries); pending = entries.Dispatch(Wire(userCancel)); prompt = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20)); Check(Answer(reads, entries, prompt, null), "typed user Cancel");
        Check(!Result(await pending.WaitAsync(TimeSpan.FromSeconds(30))).Success && entries.AuthorizationCountForFixture == 0, "cancel answer grants nothing");
        var changedCommand = command with { OperationId = Guid.NewGuid() }; await Unlock(reads, entries, changedCommand, "password");
        // Actual native/source revision mismatch, not merely a changed DTO.
        File.SetLastWriteTimeUtc(path, DateTime.UtcNow.AddMinutes(1));
        await using (var content = new ArchiveVirtualContent(entries, Items(changedCommand, browse.Entries), authorizationId: changedCommand.OperationId)) {
            try { var stream = content.Open(0, CancellationToken.None); Marshal.Release(stream); throw new InvalidOperationException("changed source materialized"); }
            catch (ArchiveOutputException failure) { Check(failure.Stage == "source_changed", "original source lease rejects changed native revision"); }
            catch (OperationCanceledException) { Check(entries.AuthorizationCountForFixture == 0, "source revision invalidation cancels original grant/content"); }
        }
        entries.Stop(); Check(entries.AuthorizationCountForFixture == 0, "disconnect clears all protected grants"); NoOutputs(root);
        var safe = string.Join("\n", log.Events); Check(!safe.Contains(WrongSecret, StringComparison.Ordinal) && !safe.Contains("\"password\":", StringComparison.Ordinal), "wrong supplied secret absent from safe diagnostics");
        using (File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { }
    }
    internal static async Task ActualZipWrongIsTerminalAndPendingCleanupRevoke()
    {
        var root = Root(); CopyWorker(root); var path = Path.Combine(root, "wrong.zip"); File.Copy(Fixture("NXTOOLS_ARCHIVE_ZIP_FIXTURES", "independent-ae2-256.zip"), path);
        var browse = await Browse(root, path); var command = Authorize(path) with { Browser = browse.Context };
        await using var reads = ArchiveReadSession.ForOwnedTestPackage(root);
        await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root); var prompts = Prompts(entries);
        var pending = entries.Dispatch(Wire(command)); var prompt = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20));
        Check(Answer(reads, entries, prompt, WrongSecret), "admit ZIP wrong attempt once");
        var failed = Result(await pending.WaitAsync(TimeSpan.FromSeconds(30))); Check(!failed.Success && failed.CleanupComplete && entries.AuthorizationCountForFixture == 0 && !prompts.Reader.TryRead(out _), "ZIP authentication/data ambiguity is terminal, not guessed WrongPassword"); NoOutputs(root);
        command = command with { OperationId = Guid.NewGuid() }; prompts = Prompts(entries);
        using var cleanupEntered = new ManualResetEventSlim(); using var releaseCleanup = new ManualResetEventSlim();
        entries.CleanupCheckpointForFixture(() => { cleanupEntered.Set(); Check(releaseCleanup.Wait(TimeSpan.FromSeconds(20)), "bounded cleanup release"); });
        pending = entries.Dispatch(Wire(command)); prompt = await prompts.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(20)); Check(Answer(reads, entries, prompt, ZipSecret), "correct TEST before revoke race");
        try {
            await Until(() => cleanupEntered.IsSet); Check(entries.ActiveWorkerCountForFixture == 1, "original worker slot retained through actual cleanup");
            Check((await entries.Dispatch(Wire(Revoke(command)))).Success && !pending.IsCompleted && entries.AuthorizationCountForFixture == 0, "revoke while private TEST complete cannot grant early/free slot");
        } finally { releaseCleanup.Set(); }
        var cancelled = Result(await pending.WaitAsync(TimeSpan.FromSeconds(30))); Check(!cancelled.Success && cancelled.CleanupComplete && entries.AuthorizationCountForFixture == 0 && entries.ActiveWorkerCountForFixture == 0, "late actual cleanup cannot recreate revoked grant");
        var next = command with { OperationId = Guid.NewGuid() }; await Unlock(reads, entries, next, ZipSecret);
        Check(!(await entries.Dispatch(Wire(Revoke(command)))).Success && entries.TryCaptureVirtualAuthorization(next.OperationId, Items(next, browse.Entries), out _), "late old revoke does not cancel new grant");
        var protectedBytes = ProtectedBytes(entries, next.OperationId);
        cleanupEntered.Reset(); releaseCleanup.Reset();
        var activeContent = new ArchiveVirtualContent(entries, Items(next, browse.Entries), authorizationId: next.OperationId);
        var opening = Task.Run(() => { var stream = activeContent.Open(0, CancellationToken.None); Marshal.Release(stream); });
        try {
            await Until(() => cleanupEntered.IsSet); Check(entries.ActiveWorkerCountForFixture == 1, "materialization original worker retains actual cleanup slot");
            Check((await entries.Dispatch(Wire(Revoke(next)))).Success && protectedBytes.All(value => value == 0), "revoke erases secret and signals original active content");
        } finally { releaseCleanup.Set(); }
        try { await opening.WaitAsync(TimeSpan.FromSeconds(30)); throw new InvalidOperationException("revoked pending content returned late medium"); } catch (OperationCanceledException) { } catch (ArchiveOutputException) { }
        await activeContent.DisposeAsync();
        Check(entries.ActiveWorkerCountForFixture == 0 && entries.AuthorizationCountForFixture == 0, "active revoke keeps original cleanup, no late output authority");
        entries.Stop(); Check(entries.AuthorizationCountForFixture == 0, "session Stop erases grant");
        Check(!Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Any(), "original native cancellation leaves no owned partial; published/retained output remains original cache owner's decision");
    }
}
