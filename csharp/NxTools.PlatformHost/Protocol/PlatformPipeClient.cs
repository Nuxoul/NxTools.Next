using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

internal sealed class PlatformPipeClient(
    string pipeName,
    WindowsPlatformAdapter platformAdapter)
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
                Console.Error.WriteLine($"PlatformHost disconnected: {error.Message}");
                await Task.Delay(ReconnectDelay, cancellationToken);
            }
        }
    }

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
            Capabilities = PlatformCapabilities.Create(platformAdapter.ClipboardHotKeyAvailable)
        });
        await WriteAsync(writer, hello, cancellationToken);

        var acknowledgement = await ReadAsync(reader, cancellationToken);
        if (acknowledgement.Type != "hello_ack" || acknowledgement.CorrelationId != hello.Id)
        {
            throw new InvalidDataException("Desktop returned an invalid handshake acknowledgement.");
        }

        Console.WriteLine("PlatformHost connected to NxTools desktop protocol v1.");
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
                var command = PlatformCommandParser.ParsePasteText(envelope);
                var result = await platformAdapter.PasteTextAsync(command, cancellationToken);
                await outgoing.WriteAsync(
                    ProtocolEnvelope.Create(
                        "platform_command_result",
                        new
                        {
                            result.Success,
                            result.Error
                        },
                        envelope.Id),
                    cancellationToken);
                if (!result.Success)
                {
                    Console.Error.WriteLine($"Paste command failed: {result.Error}");
                }
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
