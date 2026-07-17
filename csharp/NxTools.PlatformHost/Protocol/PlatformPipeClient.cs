using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

internal sealed class PlatformPipeClient(
    string pipeName,
    ForegroundContextProvider foregroundContextProvider)
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
            Capabilities = new[] { "foreground_context" }
        });
        await WriteAsync(writer, hello, cancellationToken);

        var acknowledgement = await ReadAsync(reader, cancellationToken);
        if (acknowledgement.Type != "hello_ack" || acknowledgement.CorrelationId != hello.Id)
        {
            throw new InvalidDataException("Desktop returned an invalid handshake acknowledgement.");
        }

        Console.WriteLine("PlatformHost connected to NxTools desktop protocol v1.");
        await SendForegroundContextAsync(writer, cancellationToken);

        while (!cancellationToken.IsCancellationRequested)
        {
            var envelope = await ReadAsync(reader, cancellationToken);
            if (envelope.Type == "ping")
            {
                var sentAt = envelope.Payload.GetProperty("sent_at_unix_ms").GetInt64();
                await WriteAsync(
                    writer,
                    ProtocolEnvelope.Create(
                        "pong",
                        new { SentAtUnixMs = sentAt },
                        envelope.Id),
                    cancellationToken);
            }
        }
    }

    private async Task SendForegroundContextAsync(
        StreamWriter writer,
        CancellationToken cancellationToken)
    {
        var context = foregroundContextProvider.Capture();
        var envelope = ProtocolEnvelope.Create("platform_event", new
        {
            Event = "foreground_changed",
            Data = context
        });
        await WriteAsync(writer, envelope, cancellationToken);
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
