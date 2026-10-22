using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Tests;

// No WindowsPlatformAdapter, event window, global hook, registry or Shell construction.
internal static class ArchiveProductHost
{
    internal static async Task<int> RunAsync(string pipeName, string ownedRoot)
    {
        await using var session = ArchiveReadSession.ForOwnedTestPackage(ownedRoot);
        await using var pipe = new NamedPipeClientStream(".", pipeName, PipeDirection.InOut, PipeOptions.Asynchronous);
        using var stop = new CancellationTokenSource();
        await pipe.ConnectAsync(10000, stop.Token);
        var reader = new BoundedFrameReader(pipe);
        await using var writer = new StreamWriter(pipe, new UTF8Encoding(false), leaveOpen: true) { AutoFlush = true, NewLine = "\n" };
        var hello = ProtocolEnvelope.Create("hello", new { client_name = "platform_host", client_version = "0.1.0", capabilities = new[] { "archive_read_v1" } });
        await Write(writer, hello, stop.Token);
        var ack = await Read(reader, stop.Token);
        if (ack?.Type != "hello_ack" || ack.CorrelationId != hello.Id) return 2;
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(8);
        var writes = Task.Run(async () => {
            try { await foreach (var item in outgoing.Reader.ReadAllAsync(stop.Token)) await Write(writer, item, stop.Token); }
            finally { session.Stop(); await stop.CancelAsync(); }
        });
        var events = Task.Run(async () => {
            try { await foreach (var item in session.Events.ReadAllAsync(stop.Token)) {
                CaptureTerminal(ownedRoot, item);
                await outgoing.Writer.WriteAsync(item, stop.Token);
            } }
            finally { session.Stop(); await stop.CancelAsync(); }
        });
        try
        {
            while (await Read(reader, stop.Token) is { } request)
            {
                if (request.Type == "ping") await outgoing.Writer.WriteAsync(ProtocolEnvelope.Create("pong", request.Payload, request.Id), stop.Token);
                else if (request.Type == "platform_command")
                    await outgoing.Writer.WriteAsync(PlatformPipeClient.CreateCommandResultEnvelope(request, session.Dispatch(request)), stop.Token);
            }
            return 0;
        }
        catch (OperationCanceledException) when (stop.IsCancellationRequested) { return 0; }
        finally
        {
            session.Stop();
            await stop.CancelAsync();
            try { await Task.WhenAll(writes, events); } catch (OperationCanceledException) { }
        }
    }
    // Fixture-only observation after the SAME production session has observed actual runner cleanup.
    internal static void CaptureTerminal(string ownedRoot, ProtocolEnvelope envelope)
    {
        if (!File.Exists(Path.Combine(ownedRoot, "capture-terminal"))) return;
        if (envelope.Type != "platform_event" || !envelope.Payload.TryGetProperty("data", out var data) ||
            !data.TryGetProperty("terminal", out var terminal) || terminal.ValueKind != JsonValueKind.Object) return;
        var id = data.GetProperty("operation_id").GetGuid();
        var final = Path.Combine(ownedRoot, $"terminal-{id:D}.json");
        var temporary = final + ".tmp";
        File.WriteAllText(temporary, terminal.GetRawText(), new UTF8Encoding(false));
        File.Move(temporary, final);
    }
    private static async Task<ProtocolEnvelope?> Read(BoundedFrameReader reader, CancellationToken token)
    {
        var line = await reader.ReadAsync(token);
        if (line is null) return null;
        var envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(line, JsonDefaults.Options) ?? throw new InvalidDataException();
        envelope.Validate(); return envelope;
    }
    private static async Task Write(StreamWriter writer, ProtocolEnvelope envelope, CancellationToken token)
    {
        var line = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
        if (Encoding.UTF8.GetByteCount(line) > ProtocolConstants.MaxFrameBytes) throw new InvalidDataException();
        await writer.WriteLineAsync(line.AsMemory(), token);
    }
}
