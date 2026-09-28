using System.ComponentModel;
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

    private async Task RunSessionAsync(CancellationToken cancellationToken)
    {
        await using var pipe = new NamedPipeClientStream(
            ".",
            pipeName,
            PipeDirection.InOut,
            PipeOptions.Asynchronous);
        await pipe.ConnectAsync(3000, cancellationToken);

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

        platformAdapter.DiscardQueuedEventsForNewSession();
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
            session.Token);
        var readTask = ReadLoopAsync(
            reader,
            outgoing.Writer,
            session.Token,
            commandDispatcher);
        var tasks = new[] { writeTask, eventTask, readTask };

        var completed = await Task.WhenAny(tasks);
        await session.CancelAsync();
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
        PlatformCommandSequenceDispatcher commandDispatcher)
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
                    await WriteCommandFailureAsync(
                        envelope,
                        "<invalid>",
                        outgoing,
                        cancellationToken,
                        error);
                    continue;
                }

                try
                {
                    await commandDispatcher.DispatchAsync(
                        commandName,
                        commandToken => ExecuteCommandAsync(
                            envelope,
                            commandName,
                            outgoing,
                            commandToken),
                        cancellationToken);
                }
                catch (Exception error)
                {
                    await WriteCommandFailureAsync(
                        envelope,
                        commandName,
                        outgoing,
                        cancellationToken,
                        error);
                }
            }
        }
    }

    private async Task ExecuteCommandAsync(
        ProtocolEnvelope envelope,
        string commandName,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken)
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
                cancellationToken);
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
            result = CreateCommandFailureResult(error);
            WriteCommandDiagnostic("error", "command.failed", diagnosticCorrelationId,
                new Dictionary<string, object?>
                {
                    ["command"] = commandName,
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = error.Message,
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

    private Task WriteCommandFailureAsync(
        ProtocolEnvelope envelope,
        string commandName,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken,
        Exception error) =>
        ExecuteCommandAsync(
            envelope,
            commandName,
            outgoing,
            cancellationToken,
            error);

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
        var result = CreateCommandFailureResult(error);
        WriteCommandDiagnostic("error", "command.failed", diagnosticCorrelationId,
            new Dictionary<string, object?>
            {
                ["command"] = commandName,
                ["exception_type"] = error.GetType().FullName,
                ["message"] = error.Message,
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
        CancellationToken cancellationToken) =>
        commandName switch
        {
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
            "set_clipboard_hot_key_recording" => await platformAdapter.SetClipboardHotKeyRecordingAsync(
                PlatformCommandParser.ParseSetClipboardHotKeyRecording(envelope),
                cancellationToken),
            "configure_quick_launch" => await platformAdapter.ConfigureQuickLaunchAsync(
                PlatformCommandParser.ParseConfigureQuickLaunch(envelope),
                cancellationToken),
            "set_quick_launch_hot_key_recording" => await platformAdapter.SetQuickLaunchHotKeyRecordingAsync(
                PlatformCommandParser.ParseSetQuickLaunchHotKeyRecording(envelope),
                cancellationToken),
            "configure_desktop_shell" => await platformAdapter.ConfigureDesktopShellAsync(
                PlatformCommandParser.ParseConfigureDesktopShell(envelope),
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
                PlatformCommandParser.ParseSetWindowBoundsCentered(envelope)),
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
            "play_system_notification_sound" => platformAdapter.PlaySystemNotificationSound(),
            "shutdown_system" => platformAdapter.ShutdownSystem(
                PlatformCommandParser.ParseShutdownSystem(envelope)),
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
            "ensure_indexer_service" => await platformAdapter.EnsureIndexerServiceAsync(
                PlatformCommandParser.ParseEnsureIndexerService(envelope),
                cancellationToken),
            "ensure_browser_native_messaging" => platformAdapter.EnsureBrowserNativeMessaging(
                PlatformCommandParser.ParseEnsureBrowserNativeMessaging(envelope),
                diagnosticCorrelationId),
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
        envelope.CorrelationId ?? envelope.Id;


    private async Task ForwardEventsAsync(
        ChannelWriter<ProtocolEnvelope> outgoing,
        ConcurrentDictionary<Guid, ClipboardCapturedEvent> pendingClipboardEvents,
        CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in platformAdapter.Events.ReadAllAsync(cancellationToken))
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
