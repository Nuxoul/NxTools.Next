using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.IO.Pipes;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using System.Collections.Concurrent;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

internal sealed class PlatformPipeClient(
    string pipeName,
    WindowsPlatformAdapter platformAdapter,
    IPlatformLog log)
{
    private static readonly TimeSpan ReconnectDelay = TimeSpan.FromSeconds(1);
    private static readonly TimeSpan ReconnectLogInterval = TimeSpan.FromSeconds(30);
    private long consecutiveDisconnects;
    private DateTimeOffset lastDisconnectLogAt = DateTimeOffset.MinValue;

    public async Task RunAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await RunSessionAsync(cancellationToken);
                Interlocked.Exchange(ref consecutiveDisconnects, 0);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception error)
            {
                var level = DisconnectLogLevel(error);
                var consecutive = Interlocked.Increment(ref consecutiveDisconnects);
                // A dead desktop turns this loop into a retry storm; logging
                // every attempt buried the diagnostic sink in 700+ identical
                // error records within an hour. Log the first failure and
                // rate-limit the repeats to one bounded record per interval.
                var sinceLastLog = DateTimeOffset.UtcNow - lastDisconnectLogAt;
                if (ShouldLogDisconnect(consecutive, sinceLastLog))
                {
                    lastDisconnectLogAt = DateTimeOffset.UtcNow;
                    if (level == "error")
                    {
                        Console.Error.WriteLine($"PlatformHost disconnected: {error.Message}");
                    }
                    log.Write(level, "ipc.disconnected", fields: new Dictionary<string, object?>
                    {
                        ["exception_type"] = error.GetType().FullName,
                        ["message"] = error.Message,
                        ["stack_trace"] = error.StackTrace,
                        ["consecutive_failures"] = consecutive
                    });
                }
                await Task.Delay(ReconnectDelay, cancellationToken);
            }
        }
    }

    internal static bool ShouldLogDisconnect(long consecutiveFailures, TimeSpan sinceLastLog) =>
        consecutiveFailures == 1 || sinceLastLog >= ReconnectLogInterval;

    internal static string DisconnectLogLevel(Exception error) =>
        error is EndOfStreamException ? "info" : "error";

    private static bool IsPickerCommand(string name) => name.StartsWith("pick_", StringComparison.Ordinal);
    internal static bool ShouldLogCommandLifecycle(string commandName) =>
        !string.Equals(
            commandName,
            "set_taskbar_progress",
            StringComparison.Ordinal);

    internal static ProtocolEnvelope CreateCommandResultEnvelope(
        ProtocolEnvelope request,
        WindowsCommandResult result) =>
        ProtocolEnvelope.Create(
            "platform_command_result",
            new
            {
                result.Success,
                result.Error,
                result.NativeErrorCode,
                result.Data
            },
            request.Id);

    internal static ProtocolEnvelope CreateSystemThemeSnapshotEnvelope(
        ThemePreferenceSnapshot snapshot, Guid correlationId) =>
        PlatformEventEnvelopeFactory.Create(new SystemThemePreferenceChangedEvent(
            correlationId, snapshot.Dark, snapshot.NativeErrorCode));

    private async Task RunSessionAsync(CancellationToken cancellationToken)
    {
        await using var pipe = new NamedPipeClientStream(
            ".",
            pipeName,
            PipeDirection.InOut,
            PipeOptions.Asynchronous);
        await pipe.ConnectAsync(3000, cancellationToken);
        // Bind menu sources to this connected ordinary Desktop peer, not a caller-supplied PID.
        if (!GetNamedPipeServerProcessId(pipe.SafePipeHandle, out var desktopProcessId) || desktopProcessId == 0)
            throw new InvalidDataException("Desktop pipe peer identity is unavailable.");

        var reader = new BoundedFrameReader(pipe);
        await using var writer = new StreamWriter(
            pipe,
            new UTF8Encoding(false),
            leaveOpen: true)
        {
            AutoFlush = true,
            NewLine = "\n"
        };

        var hello = ProtocolEnvelope.Create("hello", new
        {
            ClientName = "platform_host",
            ClientVersion = typeof(PlatformPipeClient).Assembly.GetName().Version?.ToString() ?? "0.1.0",
            Capabilities = PlatformCapabilities.Create(
                platformAdapter.ClipboardHotKeyAvailable,
                platformAdapter.QuickLaunchHotKeyAvailable)
        });
        await WriteAsync(writer, hello, cancellationToken);

        var acknowledgement = await ReadAsync(reader, cancellationToken);
        if (acknowledgement.Type != "hello_ack" || acknowledgement.CorrelationId != hello.Id)
        {
            throw new InvalidDataException("Desktop returned an invalid handshake acknowledgement.");
        }

        var keyboardHealthAccepted = AcceptsKeyboardHealth(acknowledgement);
        platformAdapter.DiscardQueuedEventsForNewSession();
        var keyboardHealthSnapshot = platformAdapter.QuickLaunchKeyboardHealthSnapshot;
        if (CreateKeyboardHealthSnapshotEnvelope(keyboardHealthSnapshot, keyboardHealthAccepted) is { } keyboardSnapshot)
            await WriteAsync(writer, keyboardSnapshot, cancellationToken);
        // The bounded event queue above was discarded: every handshake gets a fresh state
        // snapshot before normal event forwarding, even when no user setting changed.
        var motionSnapshot = platformAdapter.SystemMotionSnapshot;
        var motionCorrelationId = Guid.NewGuid();
        var motionStarted = Stopwatch.StartNew();
        await WriteAsync(writer, PlatformEventEnvelopeFactory.Create(
            new SystemMotionPreferenceChangedEvent(motionCorrelationId,
                motionSnapshot.ReducedMotion, motionSnapshot.NativeErrorCode)), cancellationToken);
        try
        {
            log.Write(motionSnapshot.NativeErrorCode == 0 ? "info" : "warning",
                "system.motion.session_snapshot_sent", motionCorrelationId,
                new Dictionary<string, object?>
                {
                    ["reduced_motion"] = motionSnapshot.ReducedMotion,
                    ["native_error_code"] = motionSnapshot.NativeErrorCode,
                    ["elapsed_ms"] = motionStarted.ElapsedMilliseconds
                });
        }
        catch { /* Diagnostic delivery cannot affect the handshake. */ }
        var themeSnapshot = platformAdapter.SystemThemeSnapshot;
        var themeCorrelationId = Guid.NewGuid();
        var themeStarted = Stopwatch.GetTimestamp();
        await WriteAsync(writer, CreateSystemThemeSnapshotEnvelope(themeSnapshot, themeCorrelationId), cancellationToken);
        try
        {
            log.Write(themeSnapshot.NativeErrorCode == 0 ? "info" : "warning",
                "system.theme.session_snapshot_sent", themeCorrelationId,
                new Dictionary<string, object?>
                {
                    ["dark"] = themeSnapshot.Dark,
                    ["native_error_code"] = themeSnapshot.NativeErrorCode,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(themeStarted).TotalMilliseconds
                });
        }
        catch { /* Diagnostic delivery cannot affect the handshake. */ }
        Console.WriteLine("PlatformHost connected to NxTools desktop protocol v1.");
        log.Write("info", "ipc.connected", fields: new Dictionary<string, object?>
        {
            ["pipe_name"] = pipeName,
            ["capabilities"] = PlatformCapabilities.Create(
                platformAdapter.ClipboardHotKeyAvailable,
                platformAdapter.QuickLaunchHotKeyAvailable)
        });
        using var session = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var commandDispatcher = new PlatformCommandSequenceDispatcher(error =>
            log.Write("error", "command.detached_failed", fields: new Dictionary<string, object?>
            {
                ["exception_type"] = error.GetType().FullName,
                ["message"] = error.Message,
                ["stack_trace"] = error.StackTrace
            }));
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(new BoundedChannelOptions(256)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });
        await using var archiveRead = new ArchiveReadSession(log);
        using var floatingAssistant = new FloatingAssistantCoordinator(desktopProcessId,
            envelope => outgoing.Writer.TryWrite(envelope), log);
        await using var archiveEntry = new ArchiveEntrySession(log);
        await using var rawJoin = new RawNumberedJoinSession(log);
        await using var archiveVirtualDrag = new NxTools.PlatformHost.Windows.Archive.ArchiveVirtualDragSession(archiveEntry,log);
        // One original drag session callback, no queue/extractor manager. Running snapshots are
        // expendable; terminal delivery uses a bounded forwarder, never blocks the OLE apartment.
        var dragTerminals=Channel.CreateBounded<ProtocolEnvelope>(new BoundedChannelOptions(2){SingleReader=true,SingleWriter=false,FullMode=BoundedChannelFullMode.Wait});
        archiveVirtualDrag.Progress=envelope=>{
            var phase=envelope.Payload.GetProperty("data").GetProperty("phase").GetString();
            if(phase is "finished" or "cancelled" or "failed") {
                if(!dragTerminals.Writer.TryWrite(envelope))session.Cancel(); // Transport failure, not silently lost success.
            }else outgoing.Writer.TryWrite(envelope);
        };
        var archiveDragEvents=Task.Run(async()=>{
            await foreach(var item in dragTerminals.Reader.ReadAllAsync(session.Token))
                await outgoing.Writer.WriteAsync(item,session.Token);
        });
        await using var archiveJob = new ArchiveJobSession(log, largeCount:AcceptsArchiveJobLargeCount(acknowledgement));
        var archiveJobProgress = Task.Run(async () => {
            await foreach (var item in archiveJob.Progress.ReadAllAsync(session.Token))
                outgoing.Writer.TryWrite(item); // Latest-only progress never backpressures prompt/terminal.
        });
        var archiveJobEvents = Task.Run(async () => {
            await foreach (var item in archiveJob.Events.ReadAllAsync(session.Token))
                await outgoing.Writer.WriteAsync(item,session.Token);
        });
        archiveEntry.PasswordPrompt = envelope => { if (!outgoing.Writer.TryWrite(envelope)) archiveEntry.Stop(); };
        var archiveEvents = Task.Run(async () => {
            await foreach (var item in archiveRead.Events.ReadAllAsync(session.Token))
                await outgoing.Writer.WriteAsync(item, session.Token);
        });
        var pendingClipboardEvents = new ConcurrentDictionary<Guid, ClipboardCapturedEvent>();
        var writeTask = WriteLoopAsync(
            writer,
            outgoing.Reader,
            pendingClipboardEvents,
            platformAdapter,
            session.Token);
        var eventTask = ForwardEventsAsync(
            outgoing.Writer,
            pendingClipboardEvents,
            keyboardHealthAccepted,
            keyboardHealthSnapshot,
            session.Token);
        var readTask = ReadLoopAsync(
            reader,
            outgoing.Writer,
            session.Token,
            commandDispatcher,
            desktopProcessId,
            archiveRead, archiveEntry, archiveJob, archiveVirtualDrag, floatingAssistant, rawJoin);
        var tasks = new[] { writeTask, eventTask, readTask, archiveEvents, archiveJobEvents, archiveJobProgress, archiveDragEvents };

        var completed = await Task.WhenAny(tasks);
        await session.CancelAsync();
        archiveRead.Stop();
        archiveVirtualDrag.Stop();
        floatingAssistant.Stop();
        archiveEntry.Stop();
        archiveJob.Stop();
        rawJoin.Stop();
        platformAdapter.CancelCurrentShellContextWork();
        commandDispatcher.BeginDrain();
        try
        {
            await completed;
        }
        finally
        {
            try
            {
                await Task.WhenAll(tasks.Select(IgnoreCancellationAsync));
            }
            finally
            {
                CleanupPendingClipboardEvents(pendingClipboardEvents);
            }
        }
    }

    private async Task ReadLoopAsync(
        BoundedFrameReader reader,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken,
        PlatformCommandSequenceDispatcher commandDispatcher,
        uint desktopProcessId,
        ArchiveReadSession archiveRead, ArchiveEntrySession archiveEntry, ArchiveJobSession archiveJob,
        NxTools.PlatformHost.Windows.Archive.ArchiveVirtualDragSession archiveVirtualDrag,
        FloatingAssistantCoordinator floatingAssistant, RawNumberedJoinSession rawJoin)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            var envelope = await ReadAsync(reader, cancellationToken);
            if (envelope.Type == "ping")
            {
                var sentAt = envelope.Payload.GetProperty("sent_at_unix_ms").GetInt64();
                await outgoing.WriteAsync(
                    ProtocolEnvelope.Create(
                        "pong",
                        new { SentAtUnixMs = sentAt },
                        envelope.Id),
                    cancellationToken);
            }
            else if (envelope.Type == "platform_command")
            {
                string commandName;
                try
                {
                    commandName = PlatformCommandParser.GetCommandName(envelope);
                }
                catch (Exception error)
                {
                    await ExecuteCommandAsync(
                        envelope,
                        "<invalid>",
                        outgoing,
                        cancellationToken,
                        error);
                    continue;
                }

                if (commandName == "raw_numbered_join")
                {
                    var result = rawJoin.Dispatch(envelope);
                    await commandDispatcher.DispatchAsync("raw_numbered_join", async commandToken =>
                    {
                        var terminal = await result;
                        if (!commandToken.IsCancellationRequested)
                            await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, terminal), commandToken);
                    }, cancellationToken);
                    continue;
                }
                if (commandName == "floating_assistant")
                {
                    // Admission installs the exact peer lease before any later retire/update arrives.
                    var result = floatingAssistant.Dispatch(envelope);
                    await commandDispatcher.DispatchAsync("archive_entry", async commandToken =>
                    {
                        var applied = await result;
                        if (!commandToken.IsCancellationRequested)
                            await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, applied), commandToken);
                    }, cancellationToken);
                    continue;
                }
                if (commandName == "archive_job") {
                    var jobReceipt=CreateCommandResultEnvelope(envelope,archiveJob.Dispatch(envelope));
                    // Never let a full ordinary receipt queue park the serial reader ahead of
                    // reserved job cancellation. EOF retires/cancels originals if transport stalls.
                    if(!outgoing.TryWrite(jobReceipt))throw new InvalidDataException("Archive job receipt transport full.");
                    continue;
                }
                if (commandName == "read_clipboard_file_drop") {
                    ArchiveClipboardCommand request;
                    try { request = PlatformCommandParser.ParseReadClipboardFileDrop(envelope); }
                    catch { await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, new WindowsCommandResult(false, "invalid_clipboard_data")), cancellationToken); continue; }
                    await commandDispatcher.DispatchAsync("read_clipboard_file_drop", async token => {
                        var result = await platformAdapter.ReadClipboardFileDropAsync(request, token);
                        if (!token.IsCancellationRequested) await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, result), token);
                    }, cancellationToken);
                    continue;
                }
                if(commandName=="archive_space") {
                    ArchiveSpaceCommand space;
                    try { space=ArchiveSpaceCommand.Parse(envelope); }
                    catch { await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,new WindowsCommandResult(false,"invalid_request")),cancellationToken);continue; }
                    await Windows.Archive.ArchiveSpacePreflight.DispatchAsync(commandDispatcher,space,log,cancellationToken,
                        (result,commandToken)=>outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,result),commandToken).AsTask());
                    continue;
                }
                if (commandName == "archive_create_destination") {
                    ArchiveCreateDestinationCommand request;
                    try { request = ArchiveCreateDestinationCommand.Parse(envelope); }
                    catch { await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, new WindowsCommandResult(false, "invalid_request")), cancellationToken); continue; }
                    await Windows.Archive.ArchiveCreateDestination.DispatchAsync(commandDispatcher, request, log, cancellationToken,
                        (result, token) => outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, result), token).AsTask());
                    continue;
                }
                if(commandName=="archive_recovery_inspect") {
                    ArchiveRecoveryInspectCommand inspect;
                    try { inspect=ArchiveRecoveryInspectCommand.Parse(envelope); }
                    catch { await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,new WindowsCommandResult(false,"invalid_request")),cancellationToken);continue; }
                    await Windows.Archive.ArchiveRecoveryInspection.DispatchAsync(commandDispatcher,inspect,log,cancellationToken,
                        (result,commandToken)=>outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,result),commandToken).AsTask());
                    continue;
                }
                if (commandName == "archive_picker") {
                    ArchivePickerCommand picker;
                    try { picker=ArchivePickerCommand.Parse(envelope); }
                    catch { await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,new WindowsCommandResult(false,"invalid_request")),cancellationToken);continue; }
                    await commandDispatcher.DispatchAsync("pick_archive_path",async commandToken => {
                        var picked=ArchiveJobPicker.Pick(picker,desktopProcessId,commandToken,log:log);
                        await outgoing.WriteAsync(picked.Envelope(),commandToken);
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,new WindowsCommandResult(picked.Code is null,picked.Code,NativeErrorCode:picked.NativeErrorCode)),commandToken);
                    },cancellationToken);
                    continue;
                }
                if (commandName == "archive_password_reply") {
                    var accepted = ArchiveBrowserPassword.Respond(envelope, archiveRead, archiveEntry);
                    await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, new WindowsCommandResult(accepted, accepted ? null : "stale_password_prompt")), cancellationToken);
                    continue;
                }
                if (commandName == "archive_browser_read") {
                    await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, archiveRead.Dispatch(ArchiveBrowserPassword.NormalizeRead(envelope), browserInteraction: true)), cancellationToken);
                    continue;
                }
                if(commandName is "archive_virtual_drag" or "archive_virtual_drag_spike") {
                    var result=archiveVirtualDrag.Dispatch(envelope,desktopProcessId); // bounded admission before next cancel
                    await commandDispatcher.DispatchAsync("archive_entry",async commandToken=>
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,await result),commandToken),cancellationToken);
                    continue;
                }
                if (commandName == "archive_entry")
                {
                    // Synchronous bounded admission installs identity before exact cancel can follow.
                    var result = archiveEntry.Dispatch(envelope);
                    await commandDispatcher.DispatchAsync("archive_entry", async commandToken =>
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, await result), commandToken), cancellationToken);
                    continue;
                }
                if (commandName == "archive_read")
                {
                    await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, archiveRead.Dispatch(envelope)), cancellationToken);
                    continue;
                }
                if (commandName is "pick_extension_setting_path" or "pick_directory" or "pick_program" or
                    "pick_extension_manifest" or "pick_extension_runtime" or "pick_clipboard_archive_path" or
                    "pick_clipboard_storage_directory" or "pick_browser_extension_export_path")
                {
                    try { ValidatePickerPeer(envelope, desktopProcessId); }
                    catch (Exception error) when (error is InvalidDataException or JsonException or InvalidOperationException or KeyNotFoundException or FormatException)
                    {
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, new WindowsCommandResult(false, "picker_invalid_peer", NativeErrorCode: 87)), cancellationToken);
                        continue;
                    }
                }
                if (commandName == "prepare_main_window") {
                    try {
                        ValidateMainWindowPeer(envelope, desktopProcessId);
                    } catch (InvalidDataException) {
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope, new WindowsCommandResult(false, "main_window_peer_mismatch")), cancellationToken);
                        continue;
                    }
                }
                if (commandName == "prepare_notification_window")
                {
                    try { ValidateNotificationWindowPeer(envelope, desktopProcessId); }
                    catch (InvalidDataException)
                    {
                        WriteCommandDiagnostic("warning", "ui.notification.surface.terminal", ResolveDiagnosticCorrelationId(envelope),
                            new Dictionary<string, object?> { ["success"] = false, ["elapsed_ms"] = 0,
                                ["native_error_code"] = null });
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,
                            new WindowsCommandResult(false, "Invalid notification payload or source identity.")), cancellationToken);
                        continue;
                    }
                }
                if (commandName == "show_context_menu")
                {
                    try
                    {
                        var menu = PlatformCommandParser.ParseShowContextMenu(envelope);
                        if (menu.SourceProcessId != desktopProcessId)
                            throw new InvalidDataException("Invalid native context menu source identity.");
                    }
                    catch (Exception menuError)
                    {
                        WriteCommandDiagnostic("warning", "context_menu.terminal", envelope.Id,
                            new Dictionary<string, object?> { ["success"] = false,
                                ["exception_type"] = menuError.GetType().FullName, ["elapsed_ms"] = 0 });
                        await outgoing.WriteAsync(CreateCommandResultEnvelope(envelope,
                            new WindowsCommandResult(false, "Invalid native context menu payload or source identity.")), cancellationToken);
                        continue;
                    }
                }
                try
                {
                    await commandDispatcher.DispatchAsync(
                        commandName,
                        commandToken => ExecuteCommandAsync(
                            envelope,
                            commandName,
                            outgoing,
                            commandToken, desktopProcessId),
                        cancellationToken);
                }
                catch (Exception error)
                {
                    await ExecuteCommandAsync(
                        envelope,
                        commandName,
                        outgoing,
                        cancellationToken,
                        error);
                }
            }
        }
    }

    internal static void ValidatePickerPeer(ProtocolEnvelope envelope, uint desktopProcessId)
    {
        var name = PlatformCommandParser.GetCommandName(envelope);
        var owner = name switch
        {
            "pick_extension_setting_path" => PlatformCommandParser.ParsePickExtensionSettingPath(envelope).SourceProcessId,
            "pick_directory" => PlatformCommandParser.ParsePickDirectory(envelope).SourceProcessId,
            "pick_program" => PlatformCommandParser.ParsePickProgram(envelope).SourceProcessId,
            "pick_extension_manifest" => PlatformCommandParser.ParsePickExtensionManifest(envelope).SourceProcessId,
            "pick_extension_runtime" => PlatformCommandParser.ParsePickExtensionRuntime(envelope).SourceProcessId,
            "pick_clipboard_archive_path" => PlatformCommandParser.ParsePickClipboardArchivePath(envelope).SourceProcessId,
            "pick_clipboard_storage_directory" => PlatformCommandParser.ParsePickClipboardStorageDirectory(envelope).SourceProcessId,
            "pick_browser_extension_export_path" => PlatformCommandParser.ParsePickBrowserExtensionExportPath(envelope).SourceProcessId,
            _ => throw new InvalidDataException("Not a picker command.")
        };
        if (desktopProcessId == 0 || owner != desktopProcessId)
            throw new InvalidDataException("Picker source does not match authenticated peer.");
    }

    internal static void ValidateMainWindowPeer(ProtocolEnvelope envelope, uint desktopProcessId)
    {
        var command = PlatformCommandParser.ParsePrepareMainWindow(envelope);
        if (desktopProcessId == 0 || command.SourceProcessId != desktopProcessId)
            throw new InvalidDataException("Invalid main-window source identity.");
    }

    internal static void ValidateNotificationWindowPeer(ProtocolEnvelope envelope, uint desktopProcessId)
    {
        var command = PlatformCommandParser.ParsePrepareNotificationWindow(envelope);
        if (desktopProcessId == 0 || command.SourceProcessId != desktopProcessId)
            throw new InvalidDataException("Invalid notification source identity.");
    }

    private async Task ExecuteCommandAsync(
        ProtocolEnvelope envelope,
        string commandName,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken, uint desktopProcessId)
    {
        var started = Stopwatch.GetTimestamp();
        var logCommandLifecycle = ShouldLogCommandLifecycle(commandName);
        var diagnosticCorrelationId = ResolveDiagnosticCorrelationId(envelope);
        if (cancellationToken.IsCancellationRequested)
        {
            WriteCommandResultDiscarded(
                commandName,
                diagnosticCorrelationId,
                "session_cancelled",
                null);
            return;
        }
        if (logCommandLifecycle)
        {
            var receivedFields = new Dictionary<string, object?>
            {
                ["command"] = commandName
            };
            AddCommandTargetDiagnosticFields(
                envelope,
                commandName,
                receivedFields);
            WriteCommandDiagnostic(
                "info",
                "command.received",
                diagnosticCorrelationId,
                receivedFields);

        }
        WindowsCommandResult result;
        try
        {
            result = await ExecutePlatformCommandAsync(
                envelope,
                commandName,
                diagnosticCorrelationId,
                cancellationToken, desktopProcessId);
        }
        catch (OperationCanceledException error) when (cancellationToken.IsCancellationRequested)
        {
            WriteCommandResultDiscarded(
                commandName,
                diagnosticCorrelationId,
                "operation_cancelled",
                error);
            return;
        }
        catch (Exception error)
        {
            var safeNativeCommand = IsPickerCommand(commandName) || commandName is "show_context_menu" or "cancel_context_menu" or "prepare_notification_window";
            result = safeNativeCommand
                ? new WindowsCommandResult(false, IsPickerCommand(commandName) ? "picker_failed" :
                    commandName == "prepare_notification_window" ? "Notification native command failed." : "Native context menu command failed.", NativeErrorCode:
                    error is Win32Exception nativeError ? nativeError.NativeErrorCode : error.HResult)
                : CreateCommandFailureResult(error);
            WriteCommandDiagnostic("error", "command.failed", diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["command"] = commandName,
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = safeNativeCommand ? null : error.Message,
                    ["native_error_code"] = result.NativeErrorCode
                });
        }

        if (!await TryWriteCommandResultAsync(
                envelope,
                commandName,
                diagnosticCorrelationId,
                outgoing,
                result,
                cancellationToken))
        {
            return;
        }
        if (logCommandLifecycle)
        {
            var completedFields = new Dictionary<string, object?>
            {
                ["command"] = commandName,
                ["success"] = result.Success,
                ["error_present"] = !string.IsNullOrEmpty(result.Error),
                ["native_error_code"] = result.NativeErrorCode,
                ["has_data"] = !string.IsNullOrEmpty(result.Data),
                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
            };
            AddCommandTargetDiagnosticFields(
                envelope,
                commandName,
                completedFields);
            AddResultNavigationDiagnosticFields(
                commandName,
                result,
                completedFields);
            WriteCommandDiagnostic(
                result.Success ? "info" : "error",
                "command.completed",
                diagnosticCorrelationId,
                completedFields);
        }
        if (!result.Success && logCommandLifecycle)
        {
            TryWriteCommandFailureToConsole(commandName);
        }
    }

    private async Task ExecuteCommandAsync(
        ProtocolEnvelope envelope,
        string commandName,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken,
        Exception error)
    {
        var started = Stopwatch.GetTimestamp();
        var logCommandLifecycle = ShouldLogCommandLifecycle(commandName);
        var diagnosticCorrelationId = ResolveDiagnosticCorrelationId(envelope);
        if (cancellationToken.IsCancellationRequested)
        {
            WriteCommandResultDiscarded(
                commandName,
                diagnosticCorrelationId,
                "session_cancelled",
                error);
            return;
        }
        if (logCommandLifecycle)
        {
            WriteCommandDiagnostic("info", "command.received", diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["command"] = commandName
                });
        }
        var safeNativeCommand = IsPickerCommand(commandName) || commandName is "show_context_menu" or "cancel_context_menu" or "prepare_notification_window";
        var result = safeNativeCommand
            ? new WindowsCommandResult(false, IsPickerCommand(commandName) ? "picker_failed" :
                commandName == "prepare_notification_window" ? "Notification native command failed." : "Native context menu command failed.", NativeErrorCode:
                error is Win32Exception nativeError ? nativeError.NativeErrorCode : error.HResult)
            : CreateCommandFailureResult(error);
        WriteCommandDiagnostic("error", "command.failed", diagnosticCorrelationId,
            new Dictionary<string, object?>
            {
                ["command"] = commandName,
                ["exception_type"] = error.GetType().FullName,
                ["message"] = safeNativeCommand ? null : error.Message,
                ["native_error_code"] = result.NativeErrorCode
            });
        if (!await TryWriteCommandResultAsync(
                envelope,
                commandName,
                diagnosticCorrelationId,
                outgoing,
                result,
                cancellationToken))
        {
            return;
        }
        if (logCommandLifecycle)
        {
            WriteCommandDiagnostic("error", "command.completed", diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["command"] = commandName,
                    ["success"] = false,
                    ["error_present"] = true,
                    ["native_error_code"] = result.NativeErrorCode,
                    ["has_data"] = false,
                    ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                });
            TryWriteCommandFailureToConsole(commandName);
        }
    }

    private async Task<bool> TryWriteCommandResultAsync(
        ProtocolEnvelope envelope,
        string commandName,
        Guid diagnosticCorrelationId,
        ChannelWriter<ProtocolEnvelope> outgoing,
        WindowsCommandResult result,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            WriteCommandResultDiscarded(
                commandName,
                diagnosticCorrelationId,
                "session_cancelled",
                null);
            return false;
        }

        try
        {
            await outgoing.WriteAsync(
                CreateCommandResultEnvelope(envelope, result),
                cancellationToken);
            return true;
        }
        catch (Exception error) when (IsResultWriteFailure(error, cancellationToken))
        {
            WriteCommandResultDiscarded(
                commandName,
                diagnosticCorrelationId,
                cancellationToken.IsCancellationRequested
                    ? "session_cancelled"
                    : "old_pipe_write_failed",
                error);
            return false;
        }
    }

    private static bool IsResultWriteFailure(
        Exception error,
        CancellationToken cancellationToken) =>
        cancellationToken.IsCancellationRequested ||
        error is OperationCanceledException or
            IOException or
            ObjectDisposedException or
            InvalidOperationException or
            ChannelClosedException;

    private void WriteCommandResultDiscarded(
        string commandName,
        Guid diagnosticCorrelationId,
        string reason,
        Exception? error)
    {
        WriteCommandDiagnostic("info", "command.result_discarded", diagnosticCorrelationId,
            new Dictionary<string, object?>
            {
                ["command"] = commandName,
                ["reason"] = reason,
                ["exception_type"] = error?.GetType().FullName
            });
    }

    private static void AddCommandTargetDiagnosticFields(
        ProtocolEnvelope envelope,
        string commandName,
        Dictionary<string, object?> fields)
    {
        var propertyName = commandName.Equals(
                "navigate_directory",
                StringComparison.OrdinalIgnoreCase)
            ? "target_directory"
            : commandName.Equals(
                "query_shell_actions",
                StringComparison.OrdinalIgnoreCase)
                ? "target"
                : null;
        if (propertyName is null)
        {
            return;
        }
        try
        {
            var target = envelope.Payload
                .GetProperty("data")
                .GetProperty(propertyName)
                .GetString();
            foreach (var pair in DirectoryNavigationTarget.DiagnosticFields(target))
            {
                fields[pair.Key] = pair.Value;
            }
        }
        catch (JsonException)
        {
            // The command parser remains the owner of payload validation.
        }
        catch (InvalidOperationException)
        {
            // Diagnostics must not alter command handling for malformed payloads.
        }
    }

    private static void AddResultNavigationDiagnosticFields(
        string commandName,
        WindowsCommandResult result,
        Dictionary<string, object?> fields)
    {
        if (!commandName.Equals(
                "query_shell_actions",
                StringComparison.OrdinalIgnoreCase) ||
            string.IsNullOrWhiteSpace(result.Data))
        {
            return;
        }
        try
        {
            using var document = JsonDocument.Parse(result.Data);
            if (!document.RootElement.TryGetProperty(
                    "navigation_directory",
                    out var navigation) ||
                navigation.ValueKind != JsonValueKind.String)
            {
                return;
            }
            foreach (var pair in DirectoryNavigationTarget.DiagnosticFields(
                         navigation.GetString()))
            {
                fields[$"navigation_{pair.Key}"] = pair.Value;
            }
        }
        catch (JsonException)
        {
            // The command result remains authoritative for malformed data.
        }
    }

    private async Task<WindowsCommandResult> ExecutePlatformCommandAsync(
        ProtocolEnvelope envelope,
        string commandName,
        Guid diagnosticCorrelationId,
        CancellationToken cancellationToken, uint desktopProcessId) =>
        commandName switch
        {
            "prepare_main_window" => await platformAdapter.PrepareMainWindowAsync(PlatformCommandParser.ParsePrepareMainWindow(envelope), diagnosticCorrelationId, cancellationToken),
            "prepare_notification_window" => await platformAdapter.PrepareNotificationWindowAsync(
                PlatformCommandParser.ParsePrepareNotificationWindow(envelope), diagnosticCorrelationId, cancellationToken),
            "show_context_menu" => await platformAdapter.ShowContextMenuAsync(
                PlatformCommandParser.ParseShowContextMenu(envelope), envelope.Id, cancellationToken),
            "cancel_context_menu" => platformAdapter.CancelContextMenu(
                PlatformCommandParser.ParseCancelContextMenu(envelope)),
            "read_clipboard_text" => await platformAdapter.ReadClipboardTextAsync(cancellationToken),
            "paste_text" => await platformAdapter.PasteTextAsync(
                PlatformCommandParser.ParsePasteText(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "paste_files" => await platformAdapter.PasteFilesAsync(
                PlatformCommandParser.ParsePasteFiles(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "paste_image" => await platformAdapter.PasteImageAsync(
                PlatformCommandParser.ParsePasteImage(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "paste_native_formats" => await platformAdapter.PasteNativeFormatsAsync(
                PlatformCommandParser.ParsePasteNativeFormats(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "configure_clipboard" => await platformAdapter.ConfigureClipboardAsync(
                PlatformCommandParser.ParseConfigureClipboard(envelope),
                cancellationToken),
            "set_clipboard_capture_paused" => await platformAdapter.SetClipboardCapturePausedAsync(
                PlatformCommandParser.ParseSetClipboardCapturePaused(envelope),
                diagnosticCorrelationId, cancellationToken),
            "set_clipboard_hot_key_recording" => await platformAdapter.SetClipboardHotKeyRecordingAsync(
                PlatformCommandParser.ParseSetClipboardHotKeyRecording(envelope),
                cancellationToken),
            "configure_quick_launch" => await platformAdapter.ConfigureQuickLaunchAsync(
                PlatformCommandParser.ParseConfigureQuickLaunch(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "set_quick_launch_hot_key_recording" => await platformAdapter.SetQuickLaunchHotKeyRecordingAsync(
                PlatformCommandParser.ParseSetQuickLaunchHotKeyRecording(envelope),
                cancellationToken),
            "configure_desktop_shell" => await platformAdapter.ConfigureDesktopShellAsync(
                PlatformCommandParser.ParseConfigureDesktopShell(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "set_window_topmost" => platformAdapter.SetWindowTopmost(
                PlatformCommandParser.ParseSetWindowTopmost(envelope)),
            "set_window_shadow" => platformAdapter.SetWindowShadow(
                PlatformCommandParser.ParseSetWindowShadow(envelope)),
            "set_window_no_activate" => platformAdapter.SetWindowNoActivate(
                PlatformCommandParser.ParseSetWindowNoActivate(envelope)),
            "set_window_bounds" => platformAdapter.SetWindowBounds(
                PlatformCommandParser.ParseSetWindowBounds(envelope)),
            "set_window_bounds_centered" => platformAdapter.SetWindowBoundsCentered(
                PlatformCommandParser.ParseSetWindowBoundsCentered(envelope),
                diagnosticCorrelationId),
            "present_fixed_window_over_anchor" => await platformAdapter.PresentFixedWindowOverAnchorAsync(
                PlatformCommandParser.ParsePresentFixedWindowOverAnchor(envelope), diagnosticCorrelationId, cancellationToken),
            "move_window_centered_over" => platformAdapter.MoveWindowCenteredOver(
                PlatformCommandParser.ParseMoveWindowCenteredOver(envelope)),
            "stash_window_bounds" => platformAdapter.StashWindowBounds(
                PlatformCommandParser.ParseStashWindowBounds(envelope)),
            "move_window_centered_over_key" => platformAdapter.MoveWindowCenteredOverKey(
                PlatformCommandParser.ParseMoveWindowCenteredOverKey(envelope)),
            "set_taskbar_progress" => platformAdapter.SetTaskbarProgress(
                PlatformCommandParser.ParseSetTaskbarProgress(envelope),
                envelope.Id),
            "place_taskbar_progress_overlay" => platformAdapter.PlaceTaskbarProgressOverlay(
                PlatformCommandParser.ParsePlaceTaskbarProgressOverlay(envelope),
                diagnosticCorrelationId),
            "set_taskbar_progress_overlay_visibility" => platformAdapter.SetTaskbarProgressOverlayVisibility(
                PlatformCommandParser.ParseSetTaskbarProgressOverlayVisibility(envelope),
                diagnosticCorrelationId),
            "place_file_dialog_search_window" => platformAdapter.PlaceFileDialogSearchWindow(
                PlatformCommandParser.ParsePlaceFileDialogSearchWindow(envelope)),
            "set_window_owner" => platformAdapter.SetWindowOwner(
                PlatformCommandParser.ParseSetWindowOwner(envelope)),
            "set_window_minimized" => platformAdapter.SetWindowMinimized(
                PlatformCommandParser.ParseSetWindowMinimized(envelope),
                diagnosticCorrelationId),
            "toggle_window_maximized" => platformAdapter.ToggleWindowMaximized(
                PlatformCommandParser.ParseToggleWindowMaximized(envelope)),
            "set_window_visibility" => platformAdapter.SetWindowVisibility(
                PlatformCommandParser.ParseSetWindowVisibility(envelope),
                diagnosticCorrelationId),
            "set_window_position" => platformAdapter.SetWindowPosition(
                PlatformCommandParser.ParseSetWindowPosition(envelope)),
            "activate_window" => platformAdapter.ActivateWindow(
                PlatformCommandParser.ParseActivateWindow(envelope)),
            "begin_window_resize" => platformAdapter.BeginWindowResize(
                PlatformCommandParser.ParseBeginWindowResize(envelope)),
            "begin_window_move" => platformAdapter.BeginWindowMove(
                PlatformCommandParser.ParseBeginWindowMove(envelope)),
            "begin_file_drag" => await platformAdapter.BeginFileDragAsync(
                PlatformCommandParser.ParseBeginFileDrag(envelope),
                envelope.Id,
                cancellationToken),
            "open_shell_target" => await platformAdapter.OpenShellTargetAsync(
                PlatformCommandParser.ParseOpenShellTarget(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "open_default_apps_settings" => platformAdapter.OpenDefaultAppsSettings(
                PlatformCommandParser.ParseOpenDefaultAppsSettings(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "play_system_notification_sound" => platformAdapter.PlaySystemNotificationSound(),
            "read_system_proxy" => SystemProxySnapshotReader.Read(diagnosticCorrelationId, log),
            "shutdown_system" => platformAdapter.ShutdownSystem(
                PlatformCommandParser.ParseShutdownSystem(envelope)),
            "pick_browser_extension_export_path" => platformAdapter.PickBrowserExtensionExportPath(
                PlatformCommandParser.ParsePickBrowserExtensionExportPath(envelope), diagnosticCorrelationId, cancellationToken),
            "export_browser_extension" => platformAdapter.ExportBrowserExtension(
                PlatformCommandParser.ParseExportBrowserExtension(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "launch_process" => platformAdapter.LaunchProcess(
                PlatformCommandParser.ParseLaunchProcess(envelope)),
            "navigate_directory" => platformAdapter.NavigateDirectory(
                PlatformCommandParser.ParseNavigateDirectory(envelope)),
            "open_file_in_dialog" => platformAdapter.OpenFileInDialog(
                PlatformCommandParser.ParseOpenFileInDialog(envelope)),
            "query_shell_actions" => platformAdapter.QueryShellActions(
                PlatformCommandParser.ParseQueryShellActions(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "query_shell_submenu" => platformAdapter.QueryShellSubmenu(
                PlatformCommandParser.ParseQueryShellSubmenu(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "invoke_shell_action" => platformAdapter.InvokeShellAction(
                PlatformCommandParser.ParseInvokeShellAction(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "extract_shell_icon" => platformAdapter.ExtractShellIcon(
                PlatformCommandParser.ParseExtractShellIcon(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "pick_archive_path" => ArchivePathPicker.Pick(PlatformCommandParser.ParsePickArchivePath(envelope), desktopProcessId, cancellationToken),
            "pick_clipboard_archive_path" => platformAdapter.PickClipboardArchivePath(
                PlatformCommandParser.ParsePickClipboardArchivePath(envelope),
                cancellationToken),
            "pick_clipboard_storage_directory" => platformAdapter.PickClipboardStorageDirectory(
                PlatformCommandParser.ParsePickClipboardStorageDirectory(envelope),
                cancellationToken),
            "pick_directory" => platformAdapter.PickDirectory(
                PlatformCommandParser.ParsePickDirectory(envelope),
                cancellationToken),
            "pick_program" => platformAdapter.PickProgram(
                PlatformCommandParser.ParsePickProgram(envelope),
                cancellationToken),
            "pick_extension_manifest" => platformAdapter.PickExtensionManifest(
                PlatformCommandParser.ParsePickExtensionManifest(envelope),
                cancellationToken,
                diagnosticCorrelationId),
            "pick_extension_runtime" => platformAdapter.PickExtensionRuntime(
                PlatformCommandParser.ParsePickExtensionRuntime(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "pick_extension_setting_path" => ExtensionSettingPathPicker.Pick(PlatformCommandParser.ParsePickExtensionSettingPath(envelope), diagnosticCorrelationId, log, cancellationToken),
            "inspect_extension_runtime" => await platformAdapter.InspectExtensionRuntimeAsync(
                PlatformCommandParser.ParseInspectExtensionRuntime(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "ensure_indexer_service" => await platformAdapter.EnsureIndexerServiceAsync(
                PlatformCommandParser.ParseEnsureIndexerService(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            "ensure_browser_native_messaging" => platformAdapter.EnsureBrowserNativeMessaging(
                PlatformCommandParser.ParseEnsureBrowserNativeMessaging(envelope),
                diagnosticCorrelationId),
            "archive_shell_integration" => platformAdapter.ArchiveShellIntegration(
                PlatformCommandParser.ParseArchiveShellIntegration(envelope),
                diagnosticCorrelationId,
                cancellationToken),
            var command => new WindowsCommandResult(
                false,
                $"Unsupported platform command '{command}'.")
        };

    internal static WindowsCommandResult CreateCommandFailureResult(Exception error)
    {
        int? nativeErrorCode = error switch
        {
            Win32Exception win32 => win32.NativeErrorCode,
            ExternalException external => external.ErrorCode,
            _ => null
        };
        return new WindowsCommandResult(
            false,
            error.Message,
            NativeErrorCode: nativeErrorCode);
    }

    internal static ProtocolEnvelope CreateCommandFailureEnvelope(
        ProtocolEnvelope request,
        Exception error) =>
        CreateCommandResultEnvelope(request, CreateCommandFailureResult(error));
    private static void TryWriteCommandFailureToConsole(string commandName)
    {
        try
        {
            Console.Error.WriteLine($"Platform command '{commandName}' failed.");
        }
        catch
        {
            // Console diagnostics must not change the terminal command result.
        }
    }

    private void WriteCommandDiagnostic(
        string level,
        string eventName,
        Guid correlationId,
        IReadOnlyDictionary<string, object?> fields)
    {
        try
        {
            log.Write(level, eventName, correlationId, fields);
        }
        catch
        {
            // Diagnostics must not change command completion behavior.
        }
    }

    internal static Guid ResolveDiagnosticCorrelationId(ProtocolEnvelope envelope) =>
        envelope.Payload.ValueKind == JsonValueKind.Object &&
        envelope.Payload.TryGetProperty("command", out var command) && command.ValueKind == JsonValueKind.String &&
        command.GetString() == "show_context_menu" ? envelope.Id : envelope.CorrelationId ?? envelope.Id;


    internal static ProtocolEnvelope? CreateKeyboardHealthSnapshotEnvelope(
        QuickLaunchKeyboardHealthChangedEvent? snapshot, bool keyboardHealthAccepted) =>
        keyboardHealthAccepted && snapshot is not null ? PlatformEventEnvelopeFactory.Create(snapshot) : null;

    internal static bool AcceptsKeyboardHealth(ProtocolEnvelope acknowledgement) =>
        acknowledgement.Payload.TryGetProperty("accepted_capabilities", out var capabilities) &&
        capabilities.ValueKind == JsonValueKind.Array &&
        capabilities.EnumerateArray().Any(capability =>
            capability.ValueKind == JsonValueKind.String &&
            capability.GetString() == PlatformCapabilities.QuickLaunchKeyboardHealth);

    internal static bool AcceptsArchiveJobLargeCount(ProtocolEnvelope acknowledgement) =>
        acknowledgement.Payload.TryGetProperty("accepted_capabilities", out var capabilities) &&
        capabilities.ValueKind == JsonValueKind.Array &&
        capabilities.EnumerateArray().Any(capability =>
            capability.ValueKind == JsonValueKind.String &&
            capability.GetString() == PlatformCapabilities.ArchiveJobLargeCount);

    internal static bool CanForwardEvent(WindowsPlatformEvent platformEvent, bool keyboardHealthAccepted) =>
        platformEvent is not QuickLaunchKeyboardHealthChangedEvent || keyboardHealthAccepted;

    internal static async IAsyncEnumerable<WindowsPlatformEvent> ReadEventsWithKeyboardHealthSnapshots(
        ChannelReader<WindowsPlatformEvent> events,
        Func<QuickLaunchKeyboardHealthChangedEvent?> keyboardHealthSnapshot,
        bool keyboardHealthAccepted,
        QuickLaunchKeyboardHealthChangedEvent? lastSent,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in events.ReadAllAsync(cancellationToken))
        {
            if (keyboardHealthAccepted && keyboardHealthSnapshot() is { } current && current != lastSent)
            {
                lastSent = current;
                yield return current;
            }
            // Queued health is a wake-up; the retained snapshot owns the latest state.
            if (platformEvent is not QuickLaunchKeyboardHealthChangedEvent)
                yield return platformEvent;
        }
    }

    private async Task ForwardEventsAsync(
        ChannelWriter<ProtocolEnvelope> outgoing,
        ConcurrentDictionary<Guid, ClipboardCapturedEvent> pendingClipboardEvents,
        bool keyboardHealthAccepted,
        QuickLaunchKeyboardHealthChangedEvent? keyboardHealthSnapshot,
        CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in ReadEventsWithKeyboardHealthSnapshots(
            platformAdapter.Events, () => platformAdapter.QuickLaunchKeyboardHealthSnapshot,
            keyboardHealthAccepted, keyboardHealthSnapshot, cancellationToken))
        {
            ProtocolEnvelope envelope;
            int frameBytes;
            try
            {
                envelope = PlatformEventEnvelopeFactory.Create(platformEvent);
                frameBytes = JsonSerializer.SerializeToUtf8Bytes(
                    envelope,
                    JsonDefaults.Options).Length;
            }
            catch (Exception error)
            {
                if (platformEvent is ClipboardCapturedEvent captured)
                {
                    platformAdapter.HandleClipboardCapturedEventForwardingFailure(
                        captured,
                        "serialization_failed",
                        null,
                        error);
                }
                else
                {
                    TryWriteEventForwardingDiagnostic("serialization_failed", error);
                }
                continue;
            }

            if (frameBytes > ProtocolConstants.MaxFrameBytes)
            {
                if (platformEvent is ClipboardCapturedEvent captured)
                {
                    platformAdapter.HandleClipboardCapturedEventForwardingFailure(
                        captured,
                        "frame_too_large",
                        frameBytes);
                }
                else
                {
                    TryWriteEventForwardingDiagnostic("frame_too_large", null);
                }
                continue;
            }

            if (platformEvent is ClipboardCapturedEvent capturedEvent)
            {
                pendingClipboardEvents[envelope.Id] = capturedEvent;
            }
            try
            {
                await outgoing.WriteAsync(envelope, cancellationToken);
            }
            catch (Exception error)
            {
                if (pendingClipboardEvents.TryRemove(envelope.Id, out var captured))
                {
                    platformAdapter.HandleClipboardCapturedEventForwardingFailure(
                        captured,
                        "forward_failed",
                        frameBytes,
                        error);
                }
                throw;
            }
        }
    }

    private void CleanupPendingClipboardEvents(
        ConcurrentDictionary<Guid, ClipboardCapturedEvent> pendingClipboardEvents)
    {
        foreach (var entry in pendingClipboardEvents)
        {
            if (pendingClipboardEvents.TryRemove(entry.Key, out var captured))
            {
                platformAdapter.HandleClipboardCapturedEventForwardingFailure(
                    captured,
                    "session_ended",
                    null);
            }
        }
    }

    private void TryWriteEventForwardingDiagnostic(string reason, Exception? error)
    {
        try
        {
            log.Write("warning", "platform.event_dropped", fields: new Dictionary<string, object?>
            {
                ["reason"] = reason,
                ["exception_type"] = error?.GetType().FullName
            });
        }
        catch
        {
            // Event diagnostics must not change session behavior.
        }
    }

    private static async Task WriteLoopAsync(
        StreamWriter writer,
        ChannelReader<ProtocolEnvelope> outgoing,
        ConcurrentDictionary<Guid, ClipboardCapturedEvent> pendingClipboardEvents,
        WindowsPlatformAdapter platformAdapter,
        CancellationToken cancellationToken)
    {
        await foreach (var envelope in outgoing.ReadAllAsync(cancellationToken))
        {
            try
            {
                await WriteAsync(writer, envelope, cancellationToken);
            }
            catch (Exception error)
            {
                if (pendingClipboardEvents.TryRemove(envelope.Id, out var captured))
                {
                    platformAdapter.HandleClipboardCapturedEventForwardingFailure(
                        captured,
                        "write_failed",
                        null,
                        error);
                }
                throw;
            }
            pendingClipboardEvents.TryRemove(envelope.Id, out _);
        }
    }

    private static async Task IgnoreCancellationAsync(Task task)
    {
        try
        {
            await task;
        }
        catch (OperationCanceledException)
        {
        }
    }

    private static async Task<ProtocolEnvelope> ReadAsync(
        BoundedFrameReader reader,
        CancellationToken cancellationToken)
    {
        var line = await reader.ReadAsync(cancellationToken);
        if (line is null)
        {
            throw new EndOfStreamException("Desktop closed the platform pipe.");
        }

        var envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(line, JsonDefaults.Options)
            ?? throw new InvalidDataException("Desktop sent an empty protocol frame.");
        envelope.Validate();
        return envelope;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool GetNamedPipeServerProcessId(Microsoft.Win32.SafeHandles.SafePipeHandle pipe, out uint processId);

    private static async Task WriteAsync(
        StreamWriter writer,
        ProtocolEnvelope envelope,
        CancellationToken cancellationToken)
    {
        var line = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
        if (Encoding.UTF8.GetByteCount(line) > ProtocolConstants.MaxFrameBytes)
        {
            throw new InvalidDataException("PlatformHost attempted to send an oversized frame.");
        }
        await writer.WriteLineAsync(line.AsMemory(), cancellationToken);
    }
}
