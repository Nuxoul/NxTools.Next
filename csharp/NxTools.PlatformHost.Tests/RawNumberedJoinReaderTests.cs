using System.Collections.Concurrent;
using System.IO.Pipes;
using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class RawNumberedJoinReaderTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Lines = new();
        public void Write(string level, string name, Guid? id = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Lines.Enqueue(JsonSerializer.Serialize(new { name, id, fields }));
    }
    private static ProtocolEnvelope Wire(RawNumberedJoinCommand command) =>
        ProtocolEnvelope.Create("platform_command", new { command = "raw_numbered_join", data = command }, command.OperationId);
    private static RawNumberedJoinReply Reply(ProtocolEnvelope frame) =>
        JsonSerializer.Deserialize<RawNumberedJoinReply>(frame.Payload.GetProperty("data").GetString()!, JsonDefaults.Options)!;

    internal static async Task SerialReaderResponsiveAndPhysicalDrain()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "raw-join-reader-" + Guid.NewGuid()); Directory.CreateDirectory(root);
        var source = Path.Combine(root, "reader.bin.001"); var bytes = new byte[200123]; new Random(1819).NextBytes(bytes); File.WriteAllBytes(source, bytes);
        var passed = false;
        var log = new CaptureLog();
        await using var join = new RawNumberedJoinSession(log);
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        using var release = new ManualResetEventSlim();
        using var sessionCancellation = new CancellationTokenSource();
        var name = "NxTools.RawJoin.Fixture." + Guid.NewGuid();
        using var server = new NamedPipeServerStream(name, PipeDirection.In, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        using var client = new NamedPipeClientStream(".", name, PipeDirection.Out, PipeOptions.Asynchronous);
        var connecting = server.WaitForConnectionAsync(sessionCancellation.Token); await client.ConnectAsync(sessionCancellation.Token); await connecting;
        using var writer = new StreamWriter(client, new UTF8Encoding(false), 4096, leaveOpen: true) { AutoFlush = true };
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(32);
        var dispatcher = new PlatformCommandSequenceDispatcher();
        // Only raw-join, ping and invalid clipboard metadata are delivered: no Windows adapter/API is invoked.
        var pipeClient = new PlatformPipeClient(name, null!, log);
        var read = typeof(PlatformPipeClient).GetMethod("ReadLoopAsync", BindingFlags.Instance | BindingFlags.NonPublic)!;
        var readTask = (Task)read.Invoke(pipeClient, [new BoundedFrameReader(server), outgoing.Writer, sessionCancellation.Token,
            dispatcher, (uint)Environment.ProcessId, null!, null!, null!, null!, null!, join])!;
        async Task Send(ProtocolEnvelope envelope) => await writer.WriteLineAsync(JsonSerializer.Serialize(envelope, JsonDefaults.Options));
        async Task<ProtocolEnvelope> Receive(Guid correlation)
        {
            var frame = await outgoing.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(10));
            Check(frame.CorrelationId == correlation, "exact original reader receipt ownership"); return frame;
        }
        try
        {
            var command = new RawNumberedJoinCommand(1, Guid.NewGuid(), Guid.NewGuid(), 1, "inspect", SelectedPath: source);
            var inspect = Wire(command); await Send(inspect);
            var previewFrame = await Receive(inspect.Id); Check(previewFrame.Payload.GetProperty("success").GetBoolean(), "reader inspect success");
            var preview = Reply(previewFrame).Preview!;
            join.TransferObservation = new RawJoinObservation { Checkpoint = phase => {
                if (phase == "write") { entered.TrySetResult(); if (!release.Wait(TimeSpan.FromSeconds(15))) throw new IOException("owned fixture wait"); }
            } };
            var startCommand = command with { Action = "start", SelectedPath = null, PreviewId = preview.PreviewId, OutputPath = preview.SuggestedOutput };
            var start = Wire(startCommand); await Send(start); await entered.Task.WaitAsync(TimeSpan.FromSeconds(10));
            var ping = ProtocolEnvelope.Create("ping", new { sent_at_unix_ms = 18 }); await Send(ping);
            Check((await Receive(ping.Id)).Type == "pong", "real serial reader ping while native transfer held");
            var ordinary = ProtocolEnvelope.Create("platform_command", new { command = "read_clipboard_file_drop", data = new { } }); await Send(ordinary);
            Check(!(await Receive(ordinary.Id)).Payload.GetProperty("success").GetBoolean(), "original clipboard parse failure responsive without touching clipboard");
            var cancel = Wire(command with { Action = "cancel", SelectedPath = null }); await Send(cancel);
            Check((await Receive(cancel.Id)).Payload.GetProperty("success").GetBoolean(), "exact cancel acknowledged while held");
            Check(!outgoing.Reader.TryRead(out _), "cancel ACK cannot publish terminal before physical completion");
            var busy = await join.Dispatch(Wire(command with { OperationId = Guid.NewGuid() })); Check(!busy.Success && busy.Error == "busy", "no replacement operation while cancelled worker is still finishing");
            var retiring = join.DisposeAsync().AsTask(); Check(!retiring.IsCompleted, "disconnect/close waits for actual worker and handles");
            release.Set();
            var terminalFrame = await Receive(start.Id); var terminal = Reply(terminalFrame).Terminal!;
            Check(terminal.Outcome == "cancelled" && terminal.CleanupComplete && !File.Exists(preview.SuggestedOutput), "original Start terminal after owned cleanup");
            await retiring.WaitAsync(TimeSpan.FromSeconds(10));
            using (var exclusive = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) Check(exclusive.Length == bytes.Length, "source handles actually released");
            var rawLogs = log.Lines.Where(line => line.Contains("tools.raw_numbered_join.", StringComparison.Ordinal)).ToArray();
            Check(rawLogs.Any(line => line.Contains(".accepted")) && rawLogs.Any(line => line.Contains(".started")) && rawLogs.Any(line => line.Contains(".cancelling")) && rawLogs.Any(line => line.Contains(".terminal") && line.Contains("cleanup_complete")), "correlated native lifecycle diagnostics");
            Check(rawLogs.All(line => !line.Contains(root, StringComparison.OrdinalIgnoreCase) && !line.Contains("reader.bin") && line.Contains(command.OperationId.ToString())), "no names/paths/content in correlated diagnostics");
            passed = true;
        }
        finally
        {
            release.Set(); join.Stop(); sessionCancellation.Cancel();
            try { await readTask.WaitAsync(TimeSpan.FromSeconds(10)); } catch (OperationCanceledException) { }
            await join.DisposeAsync(); dispatcher.BeginDrain(); await dispatcher.DrainAsync(TimeSpan.FromSeconds(10));
            if (passed) Directory.Delete(root, true); else Console.Error.WriteLine("PRESERVED FIXTURE " + root);
        }
    }
}
