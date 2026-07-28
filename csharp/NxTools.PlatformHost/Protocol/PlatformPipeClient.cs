using System.IO.Pipes;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
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

    public async Task RunAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await RunSessionAsync(cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception error)
            {
                var level = DisconnectLogLevel(error);
                if (level == "error")
                {
                    Console.Error.WriteLine($"PlatformHost disconnected: {error.Message}");
                }
                log.Write(level, "ipc.disconnected", fields: new Dictionary<string, object?>
                {
                    ["exception_type"] = error.GetType().FullName,
                    ["message"] = error.Message,
                    ["stack_trace"] = error.StackTrace
                });
                await Task.Delay(ReconnectDelay, cancellationToken);
            }
        }
    }

    internal static string DisconnectLogLevel(Exception error) =>
        error is EndOfStreamException ? "info" : "error";

    internal static bool ShouldLogCommandLifecycle(string commandName) =>
        !string.Equals(
            commandName,
            "set_taskbar_progress",
            StringComparison.Ordinal);

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

        Console.WriteLine("PlatformHost connected to NxTools desktop protocol v1.");
        log.Write("info", "ipc.connected", fields: new Dictionary<string, object?>
        {
            ["pipe_name"] = pipeName,
            ["capabilities"] = PlatformCapabilities.Create(
                platformAdapter.ClipboardHotKeyAvailable,
                platformAdapter.QuickLaunchHotKeyAvailable)
        });
        using var session = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(new BoundedChannelOptions(256)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });
        var writeTask = WriteLoopAsync(writer, outgoing.Reader, session.Token);
        var eventTask = ForwardEventsAsync(outgoing.Writer, session.Token);
        var readTask = ReadLoopAsync(reader, outgoing.Writer, session.Token);
        var tasks = new[] { writeTask, eventTask, readTask };

        var completed = await Task.WhenAny(tasks);
        await session.CancelAsync();
        try
        {
            await completed;
        }
        finally
        {
            await Task.WhenAll(tasks.Select(IgnoreCancellationAsync));
        }
    }

    private async Task ReadLoopAsync(
        BoundedFrameReader reader,
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken)
    {
        var commandDispatcher = new PlatformCommandSequenceDispatcher(error =>
            log.Write("error", "command.detached_failed", fields: new Dictionary<string, object?>
            {
                ["exception_type"] = error.GetType().FullName,
                ["message"] = error.Message,
                ["stack_trace"] = error.StackTrace
            }));
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
                var commandName = PlatformCommandParser.GetCommandName(envelope);
                await commandDispatcher.DispatchAsync(commandName, async () =>
                {
                    var started = Stopwatch.GetTimestamp();
                    var logCommandLifecycle = ShouldLogCommandLifecycle(commandName);
                    if (logCommandLifecycle)
                    {
                        log.Write("info", "command.received", envelope.Id,
                            new Dictionary<string, object?>
                            {
                                ["command"] = commandName
                            });
                    }
                    var result = commandName switch
                    {
                        "paste_text" => await platformAdapter.PasteTextAsync(
                            PlatformCommandParser.ParsePasteText(envelope),
                            envelope.Id,
                            cancellationToken),
                        "paste_files" => await platformAdapter.PasteFilesAsync(
                            PlatformCommandParser.ParsePasteFiles(envelope),
                            envelope.Id,
                            cancellationToken),
                        "paste_image" => await platformAdapter.PasteImageAsync(
                            PlatformCommandParser.ParsePasteImage(envelope),
                            envelope.Id,
                            cancellationToken),
                        "paste_native_formats" => await platformAdapter.PasteNativeFormatsAsync(
                            PlatformCommandParser.ParsePasteNativeFormats(envelope),
                            envelope.Id,
                            cancellationToken),
                        "configure_clipboard" => await platformAdapter.ConfigureClipboardAsync(
                            PlatformCommandParser.ParseConfigureClipboard(envelope),
                            cancellationToken),
                        "set_clipboard_hot_key_recording" =>
                            await platformAdapter.SetClipboardHotKeyRecordingAsync(
                                PlatformCommandParser.ParseSetClipboardHotKeyRecording(envelope),
                                cancellationToken),
                        "configure_quick_launch" => await platformAdapter.ConfigureQuickLaunchAsync(
                            PlatformCommandParser.ParseConfigureQuickLaunch(envelope),
                            cancellationToken),
                        "set_quick_launch_hot_key_recording" =>
                            await platformAdapter.SetQuickLaunchHotKeyRecordingAsync(
                                PlatformCommandParser.ParseSetQuickLaunchHotKeyRecording(envelope),
                                cancellationToken),
                        "set_window_topmost" => platformAdapter.SetWindowTopmost(
                            PlatformCommandParser.ParseSetWindowTopmost(envelope)),
                        "set_window_no_activate" => platformAdapter.SetWindowNoActivate(
                            PlatformCommandParser.ParseSetWindowNoActivate(envelope)),
                        "set_window_bounds" => platformAdapter.SetWindowBounds(
                            PlatformCommandParser.ParseSetWindowBounds(envelope)),
                        "set_taskbar_progress" => platformAdapter.SetTaskbarProgress(
                            PlatformCommandParser.ParseSetTaskbarProgress(envelope),
                            envelope.Id),
                        "place_file_dialog_search_window" =>
                            platformAdapter.PlaceFileDialogSearchWindow(
                                PlatformCommandParser.ParsePlaceFileDialogSearchWindow(envelope)),
                        "set_window_owner" => platformAdapter.SetWindowOwner(
                            PlatformCommandParser.ParseSetWindowOwner(envelope)),
                        "set_window_visibility" => platformAdapter.SetWindowVisibility(
                            PlatformCommandParser.ParseSetWindowVisibility(envelope)),
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
                        "open_shell_target" => platformAdapter.OpenShellTarget(
                            PlatformCommandParser.ParseOpenShellTarget(envelope)),
                        "launch_process" => platformAdapter.LaunchProcess(
                            PlatformCommandParser.ParseLaunchProcess(envelope)),
                        "navigate_directory" => platformAdapter.NavigateDirectory(
                            PlatformCommandParser.ParseNavigateDirectory(envelope)),
                        "open_file_in_dialog" => platformAdapter.OpenFileInDialog(
                            PlatformCommandParser.ParseOpenFileInDialog(envelope)),
                        "query_shell_actions" => platformAdapter.QueryShellActions(
                            PlatformCommandParser.ParseQueryShellActions(envelope)),
                        "query_shell_submenu" => platformAdapter.QueryShellSubmenu(
                            PlatformCommandParser.ParseQueryShellSubmenu(envelope)),
                        "invoke_shell_action" => platformAdapter.InvokeShellAction(
                            PlatformCommandParser.ParseInvokeShellAction(envelope)),
                        "extract_shell_icon" => platformAdapter.ExtractShellIcon(
                            PlatformCommandParser.ParseExtractShellIcon(envelope)),
                        "pick_clipboard_archive_path" => platformAdapter.PickClipboardArchivePath(
                            PlatformCommandParser.ParsePickClipboardArchivePath(envelope)),
                        "pick_clipboard_storage_directory" => platformAdapter.PickClipboardStorageDirectory(
                            PlatformCommandParser.ParsePickClipboardStorageDirectory(envelope)),
                        "pick_directory" => platformAdapter.PickDirectory(
                            PlatformCommandParser.ParsePickDirectory(envelope)),
                        "pick_program" => platformAdapter.PickProgram(
                            PlatformCommandParser.ParsePickProgram(envelope)),
                        "ensure_indexer_service" => await platformAdapter.EnsureIndexerServiceAsync(
                            PlatformCommandParser.ParseEnsureIndexerService(envelope),
                            cancellationToken),
                        var command => new WindowsCommandResult(false, $"Unsupported platform command '{command}'.")
                    };
                    await outgoing.WriteAsync(
                        ProtocolEnvelope.Create(
                            "platform_command_result",
                            new
                            {
                                result.Success,
                                result.Error,
                                result.Data
                            },
                            envelope.Id),
                        cancellationToken);
                    if (logCommandLifecycle)
                    {
                        log.Write(result.Success ? "info" : "error", "command.completed", envelope.Id,
                            new Dictionary<string, object?>
                            {
                                ["command"] = commandName,
                                ["success"] = result.Success,
                                ["error"] = result.Error,
                                ["has_data"] = !string.IsNullOrEmpty(result.Data),
                                ["elapsed_ms"] = Stopwatch.GetElapsedTime(started).TotalMilliseconds
                            });
                    }
                    if (!result.Success && logCommandLifecycle)
                    {
                        Console.Error.WriteLine($"Platform command '{commandName}' failed: {result.Error}");
                    }
                });
            }
        }
    }

    private async Task ForwardEventsAsync(
        ChannelWriter<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in platformAdapter.Events.ReadAllAsync(cancellationToken))
        {
            await outgoing.WriteAsync(
                PlatformEventEnvelopeFactory.Create(platformEvent),
                cancellationToken);
        }
    }

    private static async Task WriteLoopAsync(
        StreamWriter writer,
        ChannelReader<ProtocolEnvelope> outgoing,
        CancellationToken cancellationToken)
    {
        await foreach (var envelope in outgoing.ReadAllAsync(cancellationToken))
        {
            await WriteAsync(writer, envelope, cancellationToken);
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
