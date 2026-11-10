using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;
using static NxTools.PlatformHost.Tests.ArchiveVirtualDragTests;

namespace NxTools.PlatformHost.Tests;

internal static partial class Opt04DragTests
{
    private static ProtocolEnvelope EntryWire(ArchiveEntryCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = command }, command.OperationId);
    internal static async Task PreviewDragLeaseAdoption()
    {
        foreach (var variation in new[] { "hit", "hit_then_miss", "source", "session", "generation", "snapshot", "ordinal", "retired" }) {
            var root = Root(false); var helper = InstallWorker(root);
            try {
                var browser = await Browse(root);
                var command = Command(browser, root);
                var preview = command.Items![0].Entry with { Action = "preview", PasswordInteraction = true };
                var log = new Events();
                await using (var entries = ArchiveEntrySession.ForOwnedTestPackage(root, log)) {
                    var outer = await entries.Dispatch(EntryWire(preview)).WaitAsync(TimeSpan.FromSeconds(30));
                    var ready = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
                    Check(outer.Success && ready.Success && ready.CleanupComplete && ready.LeaseId is not null && ready.Path is not null, "actual sealed preview " + ready.Code);
                    Check(File.ReadAllBytes(ready.Path!).SequenceEqual(Payload(0)), "preview bytes before adoption");
                    var starts = log.Named("archive.worker.started").Length;
                    var release = preview with { Action = "release", Path = null, Browser = null, Ordinal = null, PasswordInteraction = false, CopyMotw = null, LeaseId = ready.LeaseId };
                    if (variation == "retired") Check((await entries.Dispatch(EntryWire(release))).Success, "preview owner retires before drag");
                    var session = variation == "session" ? Guid.NewGuid() : command.SessionId;
                    var generation = variation == "generation" ? 2ul : command.Generation;
                    var source = Path.Combine(root, "sixteen.zip");
                    var expected = browser;
                    if (variation == "source") {
                        source = Path.Combine(root, "another.zip"); File.Copy(Path.Combine(root, "sixteen.zip"), source);
                        expected = await Browse(root, source);
                    } else if (variation == "snapshot") expected = browser with { SnapshotId = Guid.NewGuid() };
                    var ordinal = variation == "ordinal" ? 1 : 0;
                    command = command with { SessionId = session, Generation = generation, Items = [new(Name(ordinal), MemberSize,
                        preview with { OperationId = Guid.NewGuid(), Action = "materialize", PasswordInteraction = false, Path = source, SessionId = session,
                            Generation = generation, Browser = expected, Ordinal = (ulong)ordinal })] };
                    if (variation == "hit_then_miss") command = command with { Items = [command.Items![0], new(Name(1), MemberSize,
                        command.Items[0].Entry with { OperationId = Guid.NewGuid(), Ordinal = 1 })] };
                    var hit = variation is "hit" or "hit_then_miss"; STGMEDIUM held = default; Exception? consumerFailure = null;
                    await using (var drag = new ArchiveVirtualDragSession(entries, log, validSource: (_, _) => true,
                        prepare: _ => null, pressed: () => true, escape: () => false, loop: (data, _) => {
                            var pointer = Marshal.GetComInterfaceForObject(data, typeof(System.Runtime.InteropServices.ComTypes.IDataObject));
                            try {
                                var get = Slot<NativeGetData>(pointer, 3);
                                var format = Format(ArchiveVirtualFileDataObject.ContentsFormat, 0, TYMED.TYMED_ISTREAM);
                                Check(get(pointer, ref format, out held) == 0, "preview/miss native GetData " + variation);
                                Check(ReadBytes(ref held, 1)[0] == Payload(ordinal)[0], "transferred cursor prefix");
                                Check(log.Named("archive.worker.started").Length == starts + (hit ? 0 : 1), "exact preview adoption or one retained miss " + variation);
                                Check(WorkerActive(entries, log) == !hit, "cache hit has zero Run; miss remains actually active");
                                if (variation != "retired") Check(entries.Dispatch(EntryWire(release)).GetAwaiter().GetResult().Success, "preview consumer share released");
                                if (variation == "hit_then_miss") {
                                    var secondFormat = Format(ArchiveVirtualFileDataObject.ContentsFormat, 1, TYMED.TYMED_ISTREAM);
                                    Check(get(pointer, ref secondFormat, out var second) == 0, "second member creates original cache lease and runs sweep");
                                    try { Check(ReadBytes(ref second, MemberSize).SequenceEqual(Payload(1)), "actual uncached second member"); }
                                    finally { ReleaseStgMedium(ref second); }
                                    Check(log.Named("archive.worker.started").Length == starts + 1, "one worker for mixed preview hit and uncached member");
                                }
                                Check(get(pointer, ref format, out var repeat) == 0, "drag consumer survives preview retirement");
                                try { Check(ReadBytes(ref repeat, MemberSize).SequenceEqual(Payload(ordinal)), "independently verified adopted/repeated cursor"); }
                                finally { ReleaseStgMedium(ref repeat); }
                                return (0x00040100, 1);
                            } catch (Exception failure) { consumerFailure = failure; throw; }
                            finally { Marshal.Release(pointer); }
                        })) {
                        try {
                            var terminal = await drag.Dispatch(Wire(command), 1).WaitAsync(TimeSpan.FromSeconds(60));
                            Check(terminal.Success, "original synchronous drag terminal " + variation + "/" + terminal.Error + "/" + consumerFailure?.Message);
                            Check(ReadBytes(ref held, MemberSize - 1).SequenceEqual(Payload(ordinal)[1..]), "stream survives source retirement");
                        } finally { if (held.unionmember != IntPtr.Zero) ReleaseStgMedium(ref held); }
                    }
                    Check(log.Named("archive.entry.drag.preview_adopted").Length == (hit ? 1 : 0), "no mismatched or retired preview adoption");
                    Check(log.Named("archive.worker.started").Length == starts + (variation == "hit" ? 0 : 1), "repeat never materializes again");
                }
                Check(!Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any() && !Outputs(root).Any(), "preview and drag shares retired after native stream release");
                using var sourceRead = File.Open(Path.Combine(root, "sixteen.zip"), FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            } finally { if (File.Exists(helper) && !Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any()) File.Delete(helper); }
        }
    }
}
