using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Root drag destination location: one real wrapper folder is offered as CF_HDROP; the receiver's
// copy of its unique marker identifies the destination folder for the ordinary extraction job.
internal static class ArchiveDropLocateTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string Root() => Directory.CreateDirectory(Path.Combine(Path.GetTempPath(), "archive-product-" + Guid.NewGuid())).FullName;
    private static ArchiveBrowserContext Synthetic() => new(Guid.NewGuid(), "Zip", "", new("v1:fixture", Enumerable.Repeat(1u, 32).ToArray(), 1, 1));
    private static ProtocolEnvelope Wire(ArchiveVirtualDragCommand c) => ProtocolEnvelope.Create("platform_command", new { command = "archive_virtual_drag", data = c }, c.OperationId);
    private static ArchiveVirtualDragItem Item(Guid session, string name, bool directory, ArchiveBrowserContext browser) =>
        new(name, directory ? null : 3, new() { Version = 1, OperationId = Guid.NewGuid(), SessionId = session, Generation = 1, Action = "materialize",
            Path = @"C:\owned\source.zip", Ordinal = directory ? null : 0, Browser = browser, CopyMotw = false }, directory);
    private static ArchiveVirtualDragCommand Locate(string wrapper)
    {
        var session = Guid.NewGuid();
        return new(1, Guid.NewGuid(), session, 1, "locate", 42, [Item(session, wrapper, true, Synthetic())]);
    }
    private static ArchiveVirtualDragCommand LocateFile(string name)
    {
        var session = Guid.NewGuid();
        return new(1, Guid.NewGuid(), session, 1, "locate", 42, [Item(session, name, false, Synthetic())]);
    }

    internal static Task LocateWireAdmitsOnlyOneWrapper()
    {
        var command = Locate("Photos");
        var parsed = ArchiveVirtualDragWire.Parse(Wire(command));
        Check(parsed.Action == "locate" && parsed.Items!.Single() is { Name: "Photos", Directory: true } && parsed.Plan is not null, "single wrapper admitted");
        var browser = command.Items![0].Entry.Browser!;
        var direct = command with { Items = [Item(command.SessionId, "selected-files", false, browser)] };
        Check(!ArchiveVirtualDragWire.Parse(Wire(direct)).Items![0].Directory, "direct marker file admitted without a wrapper");
        foreach (var invalid in new[] {
            command with { Items = [command.Items[0], Item(command.SessionId, "Photos/a.txt", false, browser)] },
            command with { Items = [Item(command.SessionId, "Photos/inner", true, browser)] },
            command with { AuthorizationId = Guid.NewGuid() },
            command with { SourceWindowHandle = 0 },
            command with { Items = [] } }) {
            try { ArchiveVirtualDragWire.Parse(Wire(invalid)); throw new InvalidOperationException("invalid locate shape admitted"); }
            catch (InvalidDataException) { }
        }
        return Task.CompletedTask;
    }

    // A receiver that performs an ordinary folder copy like Explorer: read CF_HDROP, create the
    // destination folder (optionally renamed on collision), then copy every file inside it.
    private static (int, int) CopyReceiver(System.Runtime.InteropServices.ComTypes.IDataObject data, string destinationRoot, string? rename = null)
    {
        var format = new FORMATETC { cfFormat = 15, dwAspect = DVASPECT.DVASPECT_CONTENT, lindex = -1, tymed = TYMED.TYMED_HGLOBAL };
        data.GetData(ref format, out var medium);
        string source;
        try {
            Check(DragQueryFileW(medium.unionmember, 0xFFFFFFFF, null, 0) == 1, "exactly one dragged folder");
            var buffer = new StringBuilder(32768);
            DragQueryFileW(medium.unionmember, 0, buffer, (uint)buffer.Capacity);
            source = buffer.ToString();
        } finally { ReleaseStgMedium(ref medium); }
        Check(Directory.Exists(source), "dragged path is a real folder");
        var target = Path.Combine(destinationRoot, rename ?? Path.GetFileName(source));
        Directory.CreateDirectory(target);
        foreach (var file in Directory.GetFiles(source)) File.Copy(file, Path.Combine(target, Path.GetFileName(file)));
        return (0x00040100, 1);
    }
    private static (int, int) CopyMarkerReceiver(System.Runtime.InteropServices.ComTypes.IDataObject data, string destination)
    {
        var format = new FORMATETC { cfFormat = 15, dwAspect = DVASPECT.DVASPECT_CONTENT, lindex = -1, tymed = TYMED.TYMED_HGLOBAL };
        data.GetData(ref format, out var medium);
        string source;
        try {
            Check(DragQueryFileW(medium.unionmember, 0xFFFFFFFF, null, 0) == 1, "exactly one dragged marker");
            var buffer = new StringBuilder(32768);
            DragQueryFileW(medium.unionmember, 0, buffer, (uint)buffer.Capacity);
            source = buffer.ToString();
        } finally { ReleaseStgMedium(ref medium); }
        Check(File.Exists(source), "dragged path is a real marker file");
        File.Copy(source, Path.Combine(destination, Path.GetFileName(source)));
        return (0x00040100, 1);
    }

    private static ArchiveVirtualDragSession Session(ArchiveEntrySession entries, string root, string watched, ArchiveOleLoop loop,
        IReadOnlyList<string>? probes = null, bool watch = true, TimeSpan? window = null) =>
        new(entries, validSource: (_, _) => true, prepare: _ => null, pressed: () => true, loop: loop, escape: () => false) {
        CreateLocator = (operation, wrapper, direct) => direct
            ? ArchiveDropLocator.CreateDirect(operation, wrapper, Path.Combine(root, "temp"), watch ? [watched] : [])
            : ArchiveDropLocator.Create(operation, wrapper, Path.Combine(root, "temp"), watch ? [watched] : []),
            LocateProbeFolders = () => probes ?? [],
            LocateWindow = window ?? TimeSpan.FromSeconds(10),
            UnconfirmedLocateWindow = TimeSpan.FromMilliseconds(300)
        };

    private static string? Destination(WindowsCommandResult result)
    {
        if (!result.Success || result.Data is null) return null;
        using var document = JsonDocument.Parse(result.Data);
        return document.RootElement.GetProperty("status").GetString() == "located" ? document.RootElement.GetProperty("destination").GetString() : null;
    }

    private static void AssertClean(string root, string folder)
    {
        Check(!Directory.EnumerateFiles(folder, ArchiveDropLocator.MarkerPrefix + "*", new EnumerationOptions { AttributesToSkip = 0 }).Any(), "copied marker removed from destination");
        Check(!Directory.Exists(Path.Combine(root, "temp", "NxTools")) || !Directory.EnumerateDirectories(Path.Combine(root, "temp", "NxTools")).Any(), "owned staging folder removed");
    }

    internal static async Task ReceiverCopyLocatesCreatedFolder()
    {
        var root = Root();
        try {
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var watched = Directory.CreateDirectory(Path.Combine(root, "explorer")).FullName;
            var drop = Directory.CreateDirectory(Path.Combine(watched, "Downloads")).FullName;
            var oleThreads = new List<ApartmentState>();
            await using (var drag = Session(entries, root, watched, (data, _) => { oleThreads.Add(Thread.CurrentThread.GetApartmentState()); return CopyReceiver(data, drop); })) {
                var result = await drag.Dispatch(Wire(Locate("Photos")), 1).WaitAsync(TimeSpan.FromSeconds(20));
                var expected = Path.Combine(drop, "Photos");
                Check(Destination(result) == expected, "receiver-created folder reported: " + result.Error + " " + result.Data);
                Check(oleThreads.SequenceEqual([ApartmentState.STA]), "one loop on a dedicated STA");
                AssertClean(root, expected);
                Check(!Directory.EnumerateFileSystemEntries(expected).Any(), "destination stays empty for the extraction job");
            }
            // Receiver resolves a name collision itself; the renamed folder is the destination.
            Directory.CreateDirectory(Path.Combine(drop, "Music"));
            await using (var renamed = Session(entries, root, watched, (data, _) => CopyReceiver(data, drop, "Music (2)"))) {
                var result = await renamed.Dispatch(Wire(Locate("Music")), 1).WaitAsync(TimeSpan.FromSeconds(20));
                Check(Destination(result) == Path.Combine(drop, "Music (2)"), "receiver-renamed folder reported");
                AssertClean(root, Path.Combine(drop, "Music (2)"));
            }
            // Change notifications unavailable: the folder probe still finds the copied marker.
            await using (var probed = Session(entries, root, watched, (data, _) => CopyReceiver(data, drop), probes: [drop], watch: false)) {
                var result = await probed.Dispatch(Wire(Locate("Video")), 1).WaitAsync(TimeSpan.FromSeconds(20));
                Check(Destination(result) == Path.Combine(drop, "Video"), "probe fallback reported folder");
                AssertClean(root, Path.Combine(drop, "Video"));
            }
        } finally { Directory.Delete(root, true); }
    }

    internal static async Task DirectMarkerLocatesParentWithoutWrapper()
    {
        var root = Root();
        try {
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var watched = Directory.CreateDirectory(Path.Combine(root, "explorer")).FullName;
            var drop = Directory.CreateDirectory(Path.Combine(watched, "Downloads")).FullName;
            await using var drag = Session(entries, root, watched, (data, _) => CopyMarkerReceiver(data, drop));
            var result = await drag.Dispatch(Wire(LocateFile("selected-files")), 1).WaitAsync(TimeSpan.FromSeconds(20));
            Check(Destination(result) == drop, "direct marker reports the receiver parent");
            AssertClean(root, drop);
            Check(!Directory.EnumerateDirectories(drop).Any(), "direct marker never creates a wrapper folder");
        } finally { Directory.Delete(root, true); }
    }

    internal static async Task UncopiedOrCancelledDropReportsNoDestination()
    {
        var root = Root();
        try {
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var watched = Directory.CreateDirectory(Path.Combine(root, "explorer")).FullName;
            await using (var ignored = Session(entries, root, watched, (_, _) => (0x00040100, 1), window: TimeSpan.FromMilliseconds(600))) {
                var result = await ignored.Dispatch(Wire(Locate("Photos")), 1).WaitAsync(TimeSpan.FromSeconds(20));
                Check(!result.Success && result.Error == "drop_location_unknown", "non-filesystem receiver: " + result.Error);
                AssertClean(root, watched);
            }
            var loops = 0;
            await using (var cancelled = Session(entries, root, watched, (_, _) => { loops++; return (0x00040101, 0); })) {
                var clock = System.Diagnostics.Stopwatch.StartNew();
                var result = await cancelled.Dispatch(Wire(Locate("Photos")), 1).WaitAsync(TimeSpan.FromSeconds(20));
                Check(result.Success && result.Data!.Contains("\"cancelled\"") && loops == 1 && clock.Elapsed < TimeSpan.FromSeconds(5), "cancelled drop ends without waiting");
                AssertClean(root, watched);
            }
            await using (var busy = Session(entries, root, watched, (data, source) => {
                while (source.QueryContinueDrag(false, 1) == 0) Thread.Sleep(5);
                return (0x00040101, 0);
            })) {
                var command = Locate("Photos");
                var running = busy.Dispatch(Wire(command), 1);
                await Task.Delay(100);
                var second = await busy.Dispatch(Wire(Locate("Other")), 1);
                Check(!second.Success && second.Error == "virtual_drag_busy", "locate shares the single native drag slot");
                Check((await busy.Dispatch(Wire(command with { Action = "cancel", SourceWindowHandle = 0, Items = null }), 1)).Success, "exact cancel");
                var result = await running.WaitAsync(TimeSpan.FromSeconds(10));
                Check(result.Success && result.Data!.Contains("\"cancelled\""), "exact cancel ends locate");
            }
        } finally { Directory.Delete(root, true); }
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern uint DragQueryFileW(IntPtr drop, uint index, StringBuilder? file, uint count);
    [DllImport("ole32.dll")]
    private static extern void ReleaseStgMedium(ref STGMEDIUM medium);
}
