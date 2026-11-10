using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Effects: self-owned offscreen STATIC HWNDs, original bounded STA and controlled modal returns.
// No actual system dialog, Shell, clipboard, files, registry, service or user data.
internal static class Opt07PickerTests
{
    private static void Check(bool value, string detail)
    {
        if (!value) throw new InvalidOperationException(detail);
    }

    private static ArchivePickerCommand Command(IntPtr owner, string purpose = "folders") => new()
    {
        Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 19,
        OwnerWindowHandle = owner.ToInt64(), Purpose = purpose
    };

    private static ArchivePickerCommand Parsed(ArchivePickerCommand command) => ArchivePickerCommand.Parse(
        ProtocolEnvelope.Create("platform_command", new { command = "archive_picker", data = command }, command.OperationId));

    private static ArchiveOwnedPickerResult Pick(ArchivePickerCommand command, ArchiveJobFolderPicker presentation,
        CancellationToken token = default) => ArchiveJobPicker.Pick(Parsed(command), (uint)Environment.ProcessId, token, presentation);

    public static Task MultipleFolders()
    {
        using var owner = new OwnedWindow();
        foreach (var count in new[] { 2, 16, 32 })
        {
            var command = Command(owner.Handle) with { CurrentDirectory = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")! };
            var paths = Enumerable.Range(0, count).Select(index => @"C:\owned\folder-" + index).ToArray();
            var calls = 0;
            var multiple = false;
            var sameOwner = false;
            var sameInitial = false;
            var sta = false;
            var folderOptions = false;
            var adapter = new ArchiveJobFolderPicker((dialog, hwnd) =>
            {
                calls++;
                multiple = dialog.Multiselect;
                folderOptions = (dialog.Options & 0x868u) == 0x868u;
                sameOwner = hwnd == owner.Handle;
                sameInitial = dialog.InitialDirectory == command.CurrentDirectory;
                sta = Thread.CurrentThread.GetApartmentState() == ApartmentState.STA;
                return dialog.Multiselect ? paths : [paths[0]];
            });
            var result = Pick(command, adapter);
            Check(calls == 1 && sameOwner && sameInitial && sta && folderOptions, "original owner/initial/STA and filesystem-folder native options");
            Check(multiple && result.Code is null && !result.Cancelled && result.Paths.SequenceEqual(paths),
                $"one native multiselect must return all {count} folders; actual={result.Paths.Length}, multiselect={multiple}");
            var wire = result.Envelope();
            var decoded = wire.Payload.GetProperty("data").Deserialize<ArchiveOwnedPickerResult>(JsonDefaults.Options)!;
            Check(wire.CorrelationId == command.OperationId && decoded.OperationId == command.OperationId &&
                decoded.SessionId == command.SessionId && decoded.Generation == command.Generation && decoded.Paths.SequenceEqual(paths),
                "original typed result and correlation preserve complete vector");
        }
        return Task.CompletedTask;
    }

    public static Task LimitsAreAtomic()
    {
        using var owner = new OwnedWindow();
        var command = Command(owner.Handle);
        var longPath = @"C:\" + new string('x', 32764);
        var exact = new[] { longPath, longPath, longPath, longPath, @"C:\x" };
        Check(exact.Sum(Encoding.UTF8.GetByteCount) == 131072, "exact UTF-8 fixture bound");
        var valid = Pick(command, new((_, _) => exact));
        Check(valid.Code is null && valid.Paths.SequenceEqual(exact), "entire exact byte-budget vector admitted");
        var unicode = @"C:\" + new string('界', 10000);
        foreach (var paths in new[]
        {
            Enumerable.Range(0, 33).Select(index => @"C:\owned\" + index).ToArray(),
            new[] { longPath, longPath, longPath, longPath, @"C:\xx" },
            new[] { unicode, unicode, unicode, unicode, unicode },
            new[] { @"C:\owned\valid", "relative-folder" }
        })
        {
            var result = Pick(command, new((_, _) => paths));
            Check(result.Code == "picker_limit" && !result.Cancelled && result.Paths.Length == 0,
                $"whole-vector refusal must retain picker_limit, actual={result.Code}");
        }
        var nativeOverflow = Pick(command, new((_, _) => throw new ArchivePickerLimitException()));
        Check(nativeOverflow.Code == "picker_limit" && nativeOverflow.Paths.Length == 0, "adapter limit failure retains picker_limit");
        return Task.CompletedTask;
    }

    public static Task CancelAndNativeFailure()
    {
        using var owner = new OwnedWindow();
        var command = Command(owner.Handle);
        var cancelled = Pick(command, new((_, _) => null));
        Check(cancelled.Cancelled && cancelled.Code is null && cancelled.Paths.Length == 0, "native Cancel is no selection");
        using var stop = new CancellationTokenSource();
        stop.Cancel();
        var calls = 0;
        var adapter = new ArchiveJobFolderPicker((_, _) => { calls++; return [@"C:\owned\not-selected"]; });
        var before = Pick(command, adapter, stop.Token);
        Check(before.Cancelled && calls == 0, "pre-cancel never presents");
        foreach (var failure in new Exception[] { new Win32Exception(5), new COMException("private-path-sentinel", unchecked((int)0x80070005)) })
        {
            var failed = Pick(command, new((_, _) => throw failure));
            var native = failure is Win32Exception win32 ? win32.NativeErrorCode : failure.HResult;
            Check(failed.Code == "picker_failed" && failed.NativeErrorCode == native && failed.Paths.Length == 0 && !failed.Cancelled,
                "native presentation failure retains category and native code");
        }
        return Task.CompletedTask;
    }

    public static Task RetiredOwner()
    {
        using var owner = new OwnedWindow();
        var command = Command(owner.Handle);
        using var presented = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var operation = Task.Run(() => Pick(command, new((_, _) =>
        {
            presented.Set();
            Check(release.Wait(TimeSpan.FromSeconds(10)), "retirement release");
            return [@"C:\owned\late-selection"];
        })));
        try
        {
            Check(presented.Wait(TimeSpan.FromSeconds(10)), "retirement reached original modal seam");
            owner.Dispose();
        }
        finally { release.Set(); }
        var result = operation.GetAwaiter().GetResult();
        Check(result.Paths.Length == 0 && (result.Cancelled || result.Code == "invalid_owner"),
            "retired native owner must not publish a selected vector");
        return Task.CompletedTask;
    }

    public static Task CancelRetainsActualReservation()
    {
        using var owner = new OwnedWindow();
        using var stop = new CancellationTokenSource();
        using var presented = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        Thread? modalThread = null;
        var command = Command(owner.Handle);
        var operation = Task.Run(() => Pick(command, new((_, _) =>
        {
            modalThread = Thread.CurrentThread;
            presented.Set();
            Check(release.Wait(TimeSpan.FromSeconds(10)), "cancel release");
            return [@"C:\owned\late-selection"];
        }), stop.Token));
        try
        {
            Check(presented.Wait(TimeSpan.FromSeconds(10)), "cancel reached original modal seam");
            stop.Cancel();
            Check(operation.Wait(TimeSpan.FromSeconds(5)), "logical cancel returns boundedly");
            var cancelled = operation.GetAwaiter().GetResult();
            Check(cancelled.Cancelled && cancelled.Paths.Length == 0, "cancel cannot publish late vector");
            var overtakingCalls = 0;
            var blocked = Pick(command, new((_, _) => { overtakingCalls++; return [@"C:\owned\overtake"]; }));
            Check(overtakingCalls == 0 && blocked.Code == "picker_failed" && blocked.Paths.Length == 0,
                "actual modal retains original single STA reservation");
        }
        finally
        {
            release.Set();
            Check(modalThread is not null && modalThread.Join(TimeSpan.FromSeconds(5)), "actual STA returned and cleaned up");
        }
        var next = Pick(command, new((_, _) => [@"C:\owned\next"]));
        Check(next.Code is null && next.Paths.SequenceEqual(new[] { @"C:\owned\next" }), "slot available only after actual return");
        return Task.CompletedTask;
    }

    public static Task DestinationAndOwnerAdmission()
    {
        using var owner = new OwnedWindow();
        var multiple = true;
        var destination = Pick(Command(owner.Handle, "destination") with { CurrentDirectory = @"C:\nonexistent-opt07-" + Guid.NewGuid() },
            new((dialog, _) => { multiple = dialog.Multiselect; Check(dialog.InitialDirectory.Length == 0, "missing initial folder omitted"); return [@"C:\owned\destination"]; }));
        Check(!multiple && destination.Code is null && destination.Paths.Length == 1, "destination stays single");
        var calls = 0;
        var adapter = new ArchiveJobFolderPicker((_, _) => { calls++; return [@"C:\owned\unexpected"]; });
        var foreign = ArchiveJobPicker.Pick(Command(owner.Handle), (uint)Environment.ProcessId + 1, default, adapter);
        var absent = ArchiveJobPicker.Pick(Command(IntPtr.Zero), (uint)Environment.ProcessId, default, adapter);
        Check(calls == 0 && foreign.Code == "invalid_owner" && absent.Code == "invalid_owner" &&
            foreign.Paths.Length == 0 && absent.Paths.Length == 0, "native HWND and exact pipe peer required before presenting");
        return Task.CompletedTask;
    }

    public static Task CapabilityAndDiagnostics()
    {
        var capabilities = PlatformCapabilities.Create(false);
        Check(capabilities.Count(value => value == "archive_multi_folder_picker_v1") == 1, "production Hello advertises exactly one multi-folder capability");
        using var owner = new OwnedWindow();
        foreach (var fail in new[] { false, true })
        {
            var command = Parsed(Command(owner.Handle));
            var log = new RecordingLog();
            var result = ArchiveJobPicker.Pick(command, (uint)Environment.ProcessId, default,
                new((_, _) => fail ? throw new Win32Exception(5, "private-path-sentinel") : [@"C:\owned\private-path-sentinel"]), log);
            Check(log.Rows.Select(row => row.Event).SequenceEqual(new[] { "archive.picker.accepted", "archive.picker.start", "archive.picker.terminal" }), "low-frequency original boundary lifecycle");
            Check(log.Rows.All(row => row.Correlation == command.OperationId && row.Fields.ContainsKey("elapsed_ms")), "original correlation and elapsed on every event");
            var terminal = log.Rows[^1].Fields;
            Check((string?)terminal["category"] == (fail ? "picker_failed" : "selected") &&
                (int)terminal["count"]! == result.Paths.Length && (int?)terminal["native_error_code"] == result.NativeErrorCode,
                "terminal category/count/native code match typed result");
            Check(!JsonSerializer.Serialize(log.Rows.Select(row => new { row.Event, row.Correlation, row.Fields })).Contains("sentinel", StringComparison.Ordinal), "diagnostics never retain paths or exception messages");
        }
        var unaffected = ArchiveJobPicker.Pick(Command(owner.Handle), (uint)Environment.ProcessId, default,
            new((_, _) => [@"C:\owned\selected"]), new RecordingLog { ThrowOnWrite = true });
        Check(unaffected.Code is null && unaffected.Paths.Length == 1, "throwing diagnostic sink cannot change selection");
        using var source = new OwnedWindow();
        using var alwaysOnTop = new OwnedWindow(0x8, 0x10000000);
        var pid = (uint)Environment.ProcessId;
        var envelope = ProtocolEnvelope.Create("platform_command", new { command = "pick_browser_extension_export_path",
            data = new { format = "firefox", source_window_handle = source.Handle.ToInt64(), source_process_id = pid } });
        PlatformPipeClient.ValidatePickerPeer(envelope, pid);
        var wireCases = new (string Name, object Data)[]
        {
            ("pick_directory", new { current_directory = "", title = "folder", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_program", new { current_path = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_extension_manifest", new { current_path = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_extension_runtime", new { program = "node", current_path = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_extension_setting_path", new { kind = "folder", current_path = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_clipboard_storage_directory", new { current_directory = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
            ("pick_clipboard_archive_path", new { mode = "import", suggested_file_name = "", source_window_handle = source.Handle.ToInt64(), source_process_id = pid }),
        };
        foreach (var (name, data) in wireCases)
        {
            var request = ProtocolEnvelope.Create("platform_command", new { command = name, data });
            PlatformPipeClient.ValidatePickerPeer(request, pid);
            try { PlatformPipeClient.ValidatePickerPeer(request, pid + 1); throw new InvalidOperationException("accepted foreign " + name); }
            catch (InvalidDataException) { }
        }
        Check(PlatformCommandParser.ParsePickBrowserExtensionExportPath(envelope).SourceWindowHandle == source.Handle.ToInt64(), "parser preserves original owner");
        try { PlatformPipeClient.ValidatePickerPeer(envelope, pid + 1); throw new InvalidOperationException("accepted foreign peer"); }
        catch (InvalidDataException) { }
        var shows = 0;
        var selected = PickerOwner.Run(source.Handle.ToInt64(), pid, default, original =>
        {
            shows++;
            Check(original.Handle == source.Handle && (GetWindowLong(alwaysOnTop.Handle, -20) & 0x8) == 0,
                "original owner presented above suspended desktop Topmost window");
            return new WindowsCommandResult(true, Data: @"C:\Synthetic\fixture.xpi");
        });
        Check(selected.Success && shows == 1 && (GetWindowLong(alwaysOnTop.Handle, -20) & 0x8) != 0,
            "same live Topmost state restored after selection");
        Check(!PickerOwner.Run(source.Handle.ToInt64(), pid + 1, default,
            _ => throw new InvalidOperationException("foreign presentation")).Success, "foreign owner never presents");
        using var cancelledPicker = new CancellationTokenSource();
        using (var first = PickerOwner.SuspendTopmost(pid))
        using (var second = PickerOwner.SuspendTopmost(pid))
        {
            first.Dispose();
            Check((GetWindowLong(alwaysOnTop.Handle, -20) & 0x8) == 0, "overlapping picker retains first presentation suspension");
            second.Dispose();
        }
        Check((GetWindowLong(alwaysOnTop.Handle, -20) & 0x8) != 0, "last picker restores original Topmost state");
        var retired = PickerOwner.Run(source.Handle.ToInt64(), pid, cancelledPicker.Token, _ =>
        {
            cancelledPicker.Cancel();
            return new WindowsCommandResult(true, Data: "late-selection");
        });
        Check(!retired.Success && retired.Data is null && retired.NativeErrorCode == 995 &&
            (GetWindowLong(alwaysOnTop.Handle, -20) & 0x8) != 0, "retirement discards selection and restores state");
        alwaysOnTop.Dispose();
        Check(BoundedStaOperationRunner.TryRun("owned picker presentation fixture", Timeout.InfiniteTimeSpan,
            _ => PickerOwner.Run(source.Handle.ToInt64(), pid, default, original =>
            {
                Check(original.Handle == source.Handle && Thread.CurrentThread.GetApartmentState() == ApartmentState.STA,
                    "original owner presented on bounded STA");
                return new WindowsCommandResult(true, Data: string.Empty);
            }), out WindowsCommandResult? staResult, out _, CancellationToken.None) && staResult!.Success,
            "owned picker STA admission and cleanup");
        var missing = ProtocolEnvelope.Create("platform_command", new { command = "pick_browser_extension_export_path", data = new { format = "firefox" } });
        try { PlatformCommandParser.ParsePickBrowserExtensionExportPath(missing); throw new InvalidOperationException("missing owner admitted"); }
        catch (InvalidDataException) { }
        var exportWithoutDestination = ProtocolEnvelope.Create("platform_command", new { command = "export_browser_extension",
            data = new { format = "firefox", destination = "" } });
        try { PlatformCommandParser.ParseExportBrowserExtension(exportWithoutDestination); throw new InvalidOperationException("machine export entered human picker"); }
        catch (InvalidDataException) { }
        return Task.CompletedTask;
    }

    private sealed class RecordingLog : IPlatformLog
    {
        internal bool ThrowOnWrite { get; init; }
        internal List<(string Event, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Rows { get; } = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (ThrowOnWrite) throw new InvalidOperationException("private-log-sentinel");
            Rows.Add((eventName, correlationId, fields!));
        }
    }

    private sealed class OwnedWindow : IDisposable
    {
        internal IntPtr Handle { get; private set; }
        internal OwnedWindow(uint exStyle = 0, uint style = 0)
        {
            Handle = CreateWindowExW(exStyle, "STATIC", "", style, -30000, -30000, 1, 1, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
            Check(Handle != IntPtr.Zero, "create self-owned HWND");
        }
        public void Dispose()
        {
            if (Handle == IntPtr.Zero) return;
            Check(DestroyWindow(Handle), "destroy self-owned HWND on its creating thread");
            Handle = IntPtr.Zero;
        }
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowExW(uint extendedStyle, string className, string name, uint style,
        int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    private static extern int GetWindowLong(IntPtr window, int index);
}
