using System.Runtime.InteropServices;
using System.Text;
using NxTools.PlatformHost.Windows;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: GUID-owned temporary source/destination and a copy of the frozen current worker;
// original Job/worker/native output, no Shell, system clipboard, picker or user files.
internal static class Opt03NativeTests
{
    private static void Check(bool condition, string reason)
    { if (!condition) throw new InvalidOperationException(reason); }

    public static async Task ActualCreateResultLocations()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        Check(worker is not null && Path.IsPathFullyQualified(worker) && File.Exists(worker), "frozen current worker required");
        var root = Path.Combine(Path.GetTempPath(), "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        try
        {
            File.Copy(worker!, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var source = Path.Combine(root, "source.bin");
            // Exceeds the 64-KiB split threshold even for fast ZIP/7z encoding.
            var bytes = new byte[160000]; new Random(83).NextBytes(bytes); File.WriteAllBytes(source, bytes);
            var destination = Path.Combine(root, "destination"); Directory.CreateDirectory(destination);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            foreach (var (format, split, name, primary) in new[] {
                ("zip", false, "ordinary.zip", "ordinary.zip"),
                ("zip", true, "split.zip", "split.zip"),
                ("seven_z", true, "split.7z", "split.7z.001") })
            {
                var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
                    Request = new ArchiveJobRequest { Kind = "create", Paths = [source], Destination = destination,
                        Create = new ArchiveJobCreate { Format = format, Name = name, Level = "fast", Method = format == "zip" ? "deflate" : null,
                            Solid = format == "seven_z" ? false : null, EncryptHeader = format == "seven_z" ? false : null,
                            VolumeBytes = split ? 65536ul : null, StripSingleRoot = false, ExcludePatterns = [], TestAfterCreate = false } } };
                Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId)).Success, "actual Job admission");
                ArchiveJobTerminal? terminal = null;
                while (terminal is null)
                {
                    var frame = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(45))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                    terminal = frame.Terminal;
                }
                var expected = Path.Combine(destination, primary);
                Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete && terminal.OutputPublished == true && terminal.FilesCommitted > 0,
                    "actual publication and cleanup: " + terminal.Code);
                Check(terminal.ResultLocation?.Target == expected && terminal.ResultLocation.Action == "select_in_explorer" && File.Exists(expected),
                    "native committed create location must survive Job terminal: " + name);
                var wire = JsonSerializer.SerializeToElement(terminal, JsonDefaults.Options);
                Check(wire.GetProperty("output_published").GetBoolean() && wire.GetProperty("result_location").GetProperty("target").GetString() == expected,
                    "actual C# terminal wire");
                var export = Environment.GetEnvironmentVariable("NXTOOLS_OPT03_CREATE_WIRE");
                if (export is not null)
                {
                    Check(export == Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") && Path.IsPathFullyQualified(export) &&
                        Directory.Exists(export), "session scratch export directory required");
                    File.WriteAllText(Path.Combine(export, name + ".json"), JsonSerializer.Serialize(terminal, JsonDefaults.Options));
                }
            }
            var late = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = "start", Request = new ArchiveJobRequest { Kind = "create", Paths = [source], Destination = destination,
                    Create = new ArchiveJobCreate { Format = "zip", Name = "late.zip", Level = "fast", Method = "deflate",
                        StripSingleRoot = false, ExcludePatterns = [], TestAfterCreate = false } } };
            ProtocolEnvelope Command(ArchiveJobCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = value }, value.OperationId);
            var cancelledAfterCommit = false;
            jobs.ProbeOutputCommitForFixture(stage => { if (stage == "native_published" && !cancelledAfterCommit) {
                cancelledAfterCommit = true; Check(jobs.Dispatch(Command(late with { Action = "cancel", Request = null })).Success, "late native cancel"); } });
            Check(jobs.Dispatch(Command(late)).Success, "late cancel create admission");
            ArchiveJobTerminal? retained = null;
            while (retained is null)
                retained = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(45))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!.Terminal;
            Check(cancelledAfterCommit && retained.Outcome == "cancelled" && retained.OutputPublished == true && retained.FilesCommitted == 1 &&
                retained.CleanupComplete && retained.ResultLocation?.Target == Path.Combine(destination, "late.zip"), "late cancel retains actual native publication/location");
            jobs.ProbeOutputCommitForFixture(_ => { });
            var missingSource = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = "start", Request = new ArchiveJobRequest { Kind = "create", Paths = [Path.Combine(root, "absent.bin")],
                    Destination = destination, Create = new ArchiveJobCreate { Format = "zip", Name = "never.zip", Level = "fast", Method = "deflate",
                        StripSingleRoot = false, ExcludePatterns = [], TestAfterCreate = false } } };
            Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = missingSource }, missingSource.OperationId)).Success,
                "failed-before-commit Job admission");
            ArchiveJobTerminal? failed = null;
            while (failed is null)
                failed = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(45))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!.Terminal;
            Check(failed.Outcome == "failed" && failed.FilesCommitted == 0 && failed.OutputPublished != true && failed.ResultLocation is null &&
                failed.CleanupComplete && !File.Exists(Path.Combine(destination, "never.zip")), "no guessed location before commit");
            var observed = ArchiveCreateDestination.Observe(new(1, Guid.NewGuid(), Guid.NewGuid(), 1, destination));
            Check(observed.Success && !JsonSerializer.Deserialize<ArchiveCreateDestinationResult>(observed.Data!, JsonDefaults.Options)!
                .ExistingNames.Contains("raced.zip"), "destination observed before late collision");
            var racedTarget = Path.Combine(destination, "raced.zip");
            File.WriteAllText(racedTarget, "late unrelated target");
            var raced = late with { OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(),
                Request = late.Request! with { Create = late.Request!.Create! with { Name = "raced.zip" } } };
            Check(jobs.Dispatch(Command(raced)).Success, "late collision original admission");
            var confirmations = 0;
            ArchiveJobTerminal? refused = null;
            while (refused is null)
            {
                var item = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(45)))
                    .Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                if (item.Prompt is { } prompt)
                {
                    Check(prompt.Kind == "create_overwrite" && ++confirmations == 1 &&
                        File.ReadAllText(racedTarget) == "late unrelated target", "late collision requires original confirmation");
                    Check(jobs.Dispatch(Command(raced with { Action = "reply", Request = null,
                        Reply = new() { Ticket = prompt.Ticket, Kind = "create_overwrite", Decision = null, ApplyToAll = false } })).Success,
                        "cancel original overwrite ticket");
                }
                refused = item.Terminal;
            }
            Check(confirmations == 1 && refused.Outcome == "cancelled" && refused.FilesCommitted == 0 &&
                refused.OutputPublished != true && refused.CleanupComplete && refused.ResultLocation is null &&
                File.ReadAllText(racedTarget) == "late unrelated target" && File.ReadAllBytes(source).SequenceEqual(bytes),
                "advisory plan cannot overwrite a late target or alter its source");
        }
        finally { Directory.Delete(root, true); }
    }
    public static Task CreateDestinationObservation()
    {
        var root = Path.Combine(Path.GetTempPath(), "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        try
        {
            var target = Path.Combine(root, "destination"); Directory.CreateDirectory(target);
            File.WriteAllText(Path.Combine(target, "existing.zip"), "unchanged");
            Directory.CreateDirectory(Path.Combine(target, "folder.zip"));
            var command = new ArchiveCreateDestinationCommand(1, Guid.NewGuid(), Guid.NewGuid(), 3, target);
            var wire = ProtocolEnvelope.Create("platform_command", new { command = "archive_create_destination", data = command }, command.OperationId);
            Check(ArchiveCreateDestinationCommand.Parse(wire) == command, "original typed wire");
            var result = ArchiveCreateDestination.Observe(command);
            Check(result.Success, "held native observation");
            var data = JsonSerializer.Deserialize<ArchiveCreateDestinationResult>(result.Data!, JsonDefaults.Options)!;
            Check(!data.DestinationMissing && data.ExistingNames.OrderBy(x => x).SequenceEqual(new[] { "existing.zip", "folder.zip" }), "file and directory components");
            Check(File.ReadAllText(Path.Combine(target, "existing.zip")) == "unchanged", "no mutation");
            var missing = ArchiveCreateDestination.Observe(command with { Destination = Path.Combine(root, "missing", "child") });
            Check(missing.Success && JsonSerializer.Deserialize<ArchiveCreateDestinationResult>(missing.Data!, JsonDefaults.Options) is { DestinationMissing: true, ExistingNames.Length: 0 } && !Directory.Exists(Path.Combine(root, "missing")), "missing root no create");
            var bad = ArchiveCreateDestination.Observe(command with { Generation = 0 });
            Check(!bad.Success && bad.Error == "invalid_request", "invalid request");
            var malformed = JsonSerializer.SerializeToNode(wire, JsonDefaults.Options)!;
            malformed["payload"]!["data"]!["extra"] = true;
            try { ArchiveCreateDestinationCommand.Parse(malformed.Deserialize<ProtocolEnvelope>(JsonDefaults.Options)!); throw new InvalidOperationException("extra accepted"); }
            catch (JsonException) { }
            var wrongParent = ArchiveCreateDestination.Observe(command with { Destination = Path.Combine(target, "existing.zip", "child") });
            Check(!wrongParent.Success && wrongParent.Error == "destination_unavailable", "file ancestor cannot be missing directory");
            using (var cancelled = new CancellationTokenSource())
            {
                cancelled.Cancel();
                var stopped = ArchiveCreateDestination.Observe(command, cancellation: cancelled.Token);
                Check(!stopped.Success && stopped.Error == "cancelled", "cancel before native read");
            }
            var budgetRoot = Path.Combine(root, "budget"); Directory.CreateDirectory(budgetRoot);
            for (var i = 0; i < 1100; ++i)
                File.WriteAllBytes(Path.Combine(budgetRoot, i.ToString("D4") + new string('x', 236)), []);
            var overBudget = ArchiveCreateDestination.Observe(command with { Destination = budgetRoot });
            Check(!overBudget.Success && overBudget.Error == "destination_limit" &&
                JsonSerializer.Deserialize<ArchiveCreateDestinationResult>(overBudget.Data!, JsonDefaults.Options)!.ExistingNames.Length == 0,
                "aggregate namespace limit rejects whole observation");
            return Task.CompletedTask;
        }
        finally { Directory.Delete(root, true); }
    }
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GlobalAlloc(uint flags, UIntPtr bytes);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GlobalLock(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern bool GlobalUnlock(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern IntPtr GlobalFree(IntPtr handle);
    private static string[] ReadOwnedDrop(byte[] bytes, bool throughClipboard = false)
    {
        var handle = GlobalAlloc(0x0002, (UIntPtr)bytes.Length);
        Check(handle != IntPtr.Zero, "owned HGLOBAL allocation");
        try
        {
            var pointer = GlobalLock(handle); Check(pointer != IntPtr.Zero, "owned HGLOBAL lock");
            try { Marshal.Copy(bytes, 0, pointer, bytes.Length); }
            finally { _ = GlobalUnlock(handle); }
            if (!throughClipboard) return ClipboardFileDropReader.ReadHandle(handle);
            var access = new FakeClipboard(handle);
            var result = ClipboardFileDropReader.ReadClipboard(IntPtr.Zero, access);
            Check(access.Opens == 1 && access.Gets == 1 && access.Closes == 1, "borrowed open/get/close balanced");
            return result;
        }
        finally { Check(GlobalFree(handle) == IntPtr.Zero, "reader must release borrowed lock, never free owned handle"); }
    }
    private sealed class FakeClipboard(IntPtr drop, bool canOpen = true) : IClipboardFileDropAccess
    {
        internal int Opens, Closes, Gets;
        public bool Open(IntPtr owner) { Opens++; return canOpen; }
        public bool HasDrop() => drop != IntPtr.Zero;
        public IntPtr GetDrop() { Gets++; return drop; }
        public void Close() => Closes++;
    }
    public static Task ClipboardOwnedMemory()
    {
        var emptyDrop = new byte[24];
        BitConverter.GetBytes(20).CopyTo(emptyDrop, 0);
        BitConverter.GetBytes(1).CopyTo(emptyDrop, 16);
        Check(ReadOwnedDrop(emptyDrop).Length == 0, "empty CF_HDROP is successful empty batch");
        var command = new ArchiveClipboardCommand(1, Guid.NewGuid(), Guid.NewGuid(), 2);
        var wire = ProtocolEnvelope.Create("platform_command", new { command = "read_clipboard_file_drop", data = command }, command.OperationId);
        Check(PlatformCommandParser.ParseReadClipboardFileDrop(wire) == command &&
            PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardFileDropRead), "typed command and advertised capability");
        var paths = new[] { @"C:\fixture\one.txt", @"D:\fixture\中文.zip" };
        Check(ReadOwnedDrop(ClipboardFileDropPayload.Encode(paths), throughClipboard: true).SequenceEqual(paths), "borrowed Unicode HDROP");
        var absent = new FakeClipboard(IntPtr.Zero);
        Check(ClipboardFileDropReader.ReadClipboard(IntPtr.Zero, absent).Length == 0 && absent.Opens == 1 && absent.Closes == 1 && absent.Gets == 0, "absent format success and close");
        var denied = new FakeClipboard(IntPtr.Zero, false);
        try { ClipboardFileDropReader.ReadClipboard(IntPtr.Zero, denied); throw new InvalidOperationException("failed open accepted"); }
        catch (ClipboardFileDropException failure) { Check(failure.Code == "clipboard_unavailable" && denied.Opens == 1 && denied.Closes == 0, "failed open no close"); }
        var malformed = JsonSerializer.SerializeToNode(wire, JsonDefaults.Options)!;
        malformed["payload"]!["data"]!["unknown"] = true;
        try { PlatformCommandParser.ParseReadClipboardFileDrop(malformed.Deserialize<ProtocolEnvelope>(JsonDefaults.Options)!); throw new InvalidOperationException("unknown authority accepted"); }
        catch (JsonException) { }
        var ansi = new byte[20 + Encoding.ASCII.GetByteCount(@"C:\fixture\ansi.txt") + 2];
        BitConverter.GetBytes(20).CopyTo(ansi, 0);
        Encoding.ASCII.GetBytes(@"C:\fixture\ansi.txt").CopyTo(ansi, 20);
        Check(ReadOwnedDrop(ansi).SequenceEqual(new[] { @"C:\fixture\ansi.txt" }), "borrowed ANSI HDROP");
        var invalid = ClipboardFileDropPayload.Encode(paths);
        BitConverter.GetBytes((uint)(invalid.Length + 1)).CopyTo(invalid, 0);
        try { ReadOwnedDrop(invalid); throw new InvalidOperationException("invalid header accepted"); }
        catch (ClipboardFileDropException failure) { Check(failure.Code == "invalid_clipboard_data", "malformed header whole failure"); }
        var truncated = ClipboardFileDropPayload.Encode(paths)[..^2];
        try { ReadOwnedDrop(truncated); throw new InvalidOperationException("missing terminator accepted"); }
        catch (ClipboardFileDropException failure) { Check(failure.Code == "invalid_clipboard_data", "unterminated whole failure"); }
        try { ReadOwnedDrop(ClipboardFileDropPayload.Encode(Enumerable.Range(0, 33).Select(i => @"C:\fixture\" + i).ToArray())); throw new InvalidOperationException("33 paths accepted"); }
        catch (ClipboardFileDropException failure) { Check(failure.Code == "clipboard_limit", "count budget whole failure"); }
        try { ReadOwnedDrop(ClipboardFileDropPayload.Encode(Enumerable.Range(0, 32).Select(i => @"C:\fixture\" + new string('x', 4200) + i).ToArray())); throw new InvalidOperationException("aggregate accepted"); }
        catch (ClipboardFileDropException failure) { Check(failure.Code == "clipboard_limit", "UTF8 budget whole failure"); }
        return Task.CompletedTask;
    }
}
