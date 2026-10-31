using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text.Json;
using System.Threading.Channels;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.ExtensionHost;
using NxTools.PlatformHost.Windows;
using NxTools.PlatformHost.Windows.Extensions;

namespace NxTools.PlatformHost.Protocol;

internal readonly record struct ArchiveCliHostOptions(uint ParentPid, string PipeName, Guid SessionId);

// Demand-only archive transport. The original sessions/runner/Job/native output remain the owners;
// no adapter, window, hooks, Shell, installation, reconnect or alternate worker selection lives here.
internal static class ArchiveCliHost
{
    private const string PipePrefix = "nxtools-archive-cli-";

    internal static bool IsMode(IReadOnlyList<string> args) => args.Contains("--archive-cli");

    internal static bool TryParse(IReadOnlyList<string> args, out ArchiveCliHostOptions options)
    {
        options = default;
        if (args.Count != 5 || args[0] != "--archive-cli" || args[1] != "--parent-pid" ||
            args[3] != "--pipe-name" || !uint.TryParse(args[2], NumberStyles.None, CultureInfo.InvariantCulture, out var parent) ||
            parent == 0 || parent == Environment.ProcessId || !args[4].StartsWith(PipePrefix, StringComparison.Ordinal)) return false;
        var suffix = args[4][PipePrefix.Length..];
        if (!Guid.TryParseExact(suffix, "D", out var session) || session == Guid.Empty || suffix != session.ToString("D")) return false;
        options = new(parent, args[4], session);
        return true;
    }

    // The delegates are the pure dispatch test seam, not a process/helper policy override.
    // Production supplies only the two original per-pipe sessions, whose own identity checks
    // reject controls for foreign/retired operations and whose unchanged bounds own admission.
    internal static ProtocolEnvelope DispatchCommand(ProtocolEnvelope envelope,
        Func<ProtocolEnvelope, WindowsCommandResult> read, Func<ProtocolEnvelope, WindowsCommandResult> job)
    {
        string command;
        ArchiveJobCommand? parsedJob = null;
        var allowed = false;
        try
        {
            envelope.Validate();
            if (envelope.Id == Guid.Empty) throw new InvalidDataException();
            command = PlatformCommandParser.GetCommandName(envelope);
            if (command == "archive_read")
            {
                var parsed = ArchiveReadWire.Parse(envelope);
                allowed = parsed.Action is "open" or "test" or "ack" or "cancel" && parsed.Limits?.BrowserContext != true;
            }
            else if (command == "archive_job")
            {
                parsedJob = ArchiveJobWire.Parse(envelope);
                allowed = parsedJob.Action == "start"
                    ? parsedJob.Request is { Kind: "extract", Readmission: null }
                    : parsedJob.Action is "cancel" or "pause" or "resume" or "reply";
            }
        }
        catch
        {
            // Untrusted shape errors have one fixed public rejection, never raw parser/frame data.
            command = "";
        }
        finally
        {
            // Strict parsing must not retain a second decoded copy of an interaction secret.
            if (parsedJob?.Reply?.Password is { } reply) CryptographicOperations.ZeroMemory(reply);
            if (parsedJob?.Request?.Create?.Password is { } create) CryptographicOperations.ZeroMemory(create);
        }
        var result = allowed
            ? command == "archive_read" ? read(envelope) : job(envelope)
            : new WindowsCommandResult(false, "archive_cli_command_denied");
        return PlatformPipeClient.CreateCommandResultEnvelope(envelope, result);
    }

    internal static async Task<int> RunAsync(ArchiveCliHostOptions options, IPlatformLog? log = null)
    {
        var clock = Stopwatch.StartNew();
        using var stop = new CancellationTokenSource();
        using var pipe = new NamedPipeClientStream(".", options.PipeName, PipeDirection.InOut, PipeOptions.Asynchronous);
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(new BoundedChannelOptions(32)
        {
            FullMode = BoundedChannelFullMode.Wait, SingleReader = true, SingleWriter = false
        });
        ExtensionKernelHandle? parent = null;
        ArchiveReadSession? reads = null;
        ArchiveJobSession? jobs = null;
        // Exactly one parent monitor, writer and pump per original event/progress stream.
        Task[] tasks = [Task.CompletedTask, Task.CompletedTask, Task.CompletedTask, Task.CompletedTask, Task.CompletedTask];
        string? failureStage = null;
        int? failureNative = null;
        long commandCount = 0;
        var stage = "parent_validation";

        void Diagnostic(string name, string currentStage, int? native = null)
        {
            try
            {
                log?.Write(name == "failure" ? "warning" : "info", "archive.cli_host." + name, options.SessionId,
                    new Dictionary<string, object?>
                    {
                        ["stage"] = currentStage, ["command_count"] = commandCount,
                        ["native_error_code"] = native, ["elapsed_ms"] = clock.ElapsedMilliseconds
                    });
            }
            catch { /* Observational sinks cannot change authorization, execution or cleanup. */ }
        }
        void Fail(string failedStage, int? native = null)
        {
            if (Interlocked.CompareExchange(ref failureStage, failedStage, null) is null) failureNative = native;
            stop.Cancel();
        }
        bool Publish(ProtocolEnvelope envelope)
        {
            if (stop.IsCancellationRequested) return false;
            if (outgoing.Writer.TryWrite(envelope)) return true;
            Fail("outgoing_full"); // Never park the command reader ahead of original cancellation.
            return false;
        }
        async Task PumpAsync(ChannelReader<ProtocolEnvelope> source, string pumpStage)
        {
            try
            {
                await foreach (var envelope in source.ReadAllAsync(stop.Token))
                    if (!Publish(envelope)) return;
                if (!stop.IsCancellationRequested) Fail(pumpStage);
            }
            catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
            catch { Fail(pumpStage); }
        }
        async Task WriteAsync()
        {
            try
            {
                await foreach (var envelope in outgoing.Reader.ReadAllAsync(stop.Token))
                {
                    var bytes = JsonSerializer.SerializeToUtf8Bytes(envelope, JsonDefaults.Options);
                    if (bytes.Length > ProtocolConstants.MaxFrameBytes) { Fail("outgoing_frame_limit"); return; }
                    await pipe.WriteAsync(bytes, stop.Token);
                    await pipe.WriteAsync(new byte[] { 10 }, stop.Token);
                    await pipe.FlushAsync(stop.Token);
                }
            }
            catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
            catch (Exception failure) { Fail("write", NativeCode(failure)); }
        }
        async Task MonitorAsync(ExtensionKernelHandle heldParent)
        {
            try
            {
                while (!stop.IsCancellationRequested)
                {
                    var wait = ExtensionNative.Wait(heldParent, 0);
                    if (wait != 258)
                    {
                        Fail(wait == 0 ? "parent_exit" : "parent_wait", wait == uint.MaxValue ? Marshal.GetLastWin32Error() : null);
                        return;
                    }
                    await Task.Delay(100, stop.Token);
                }
            }
            catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
            catch (Exception failure) { Fail("parent_wait", NativeCode(failure)); }
        }

        Diagnostic("start", stage);
        try
        {
            // Fixed production layout only, including Debug: no arbitrary image/env override.
            parent = ExtensionHostBroker.ValidateParent(options.ParentPid,
                new ExtensionParentPolicy(Path.Combine(AppContext.BaseDirectory, "NxTools.Cli.exe")));
            tasks[0] = MonitorAsync(parent);
            stage = "connect";
            await pipe.ConnectAsync(3000, stop.Token);
            stage = "pipe_peer";
            if (!GetNamedPipeServerProcessId(pipe.SafePipeHandle, out var serverPid))
                throw new Win32Exception(Marshal.GetLastWin32Error());
            if (serverPid != options.ParentPid) throw new InvalidDataException();
            tasks[1] = WriteAsync();
            var reader = new BoundedFrameReader(pipe);
            stage = "handshake";
            var hello = ProtocolEnvelope.Create("hello", new
            {
                ClientName = "platform_host",
                ClientVersion = typeof(ArchiveCliHost).Assembly.GetName().Version?.ToString() ?? "0.1.0",
                Capabilities = new[] { "archive_read_v1", "archive_job_v1", "archive_cli_v1" }
            }, options.SessionId);
            if (!Publish(hello)) return 1;
            using (var handshake = CancellationTokenSource.CreateLinkedTokenSource(stop.Token))
            {
                handshake.CancelAfter(TimeSpan.FromSeconds(3));
                var acknowledgement = await ReadAsync(reader, handshake.Token);
                if (acknowledgement is null || acknowledgement.Type != "hello_ack" || acknowledgement.CorrelationId != hello.Id)
                    throw new InvalidDataException();
            }
            reads = new ArchiveReadSession(log);
            jobs = new ArchiveJobSession(log);
            tasks[2] = PumpAsync(reads.Events, "read_events");
            tasks[3] = PumpAsync(jobs.Events, "job_events");
            tasks[4] = PumpAsync(jobs.Progress, "job_progress");
            Diagnostic("connected", "ready");
            stage = "read";
            while (!stop.IsCancellationRequested && await ReadAsync(reader, stop.Token) is { } envelope)
            {
                if (envelope.Type == "ping")
                {
                    var sentAt = envelope.Payload.GetProperty("sent_at_unix_ms").GetInt64();
                    if (!Publish(ProtocolEnvelope.Create("pong", new { SentAtUnixMs = sentAt }, envelope.Id))) break;
                }
                else if (envelope.Type == "platform_command")
                {
                    commandCount++;
                    if (!Publish(DispatchCommand(envelope, e => reads.Dispatch(e), jobs.Dispatch))) break;
                }
                else throw new InvalidDataException();
            }
        }
        catch (OperationCanceledException) when (stop.IsCancellationRequested) { }
        catch (Exception failure) { Fail(stage, NativeCode(failure)); }
        finally
        {
            // EOF, writer/peer/parent/protocol failure all stop the ORIGINAL runs. A receipt or
            // private complete is never cleanup evidence. DisposeAsync awaits their real drain.
            // Mark transport retirement before completing source channels; otherwise their
            // normal completion can race PumpAsync into a false read_events/job_events failure.
            stop.Cancel();
            reads?.Stop();
            jobs?.Stop();
            pipe.Dispose();
            outgoing.Writer.TryComplete();
            try { await Task.WhenAll(tasks); }
            catch (Exception failure) { Fail("transport_cleanup", NativeCode(failure)); }
            try { if (reads is not null) await reads.DisposeAsync(); }
            catch (Exception failure) { Fail("read_cleanup", NativeCode(failure)); }
            try { if (jobs is not null) await jobs.DisposeAsync(); }
            catch (Exception failure) { Fail("job_cleanup", NativeCode(failure)); }
            parent?.Dispose();
            if (failureStage is not null) Diagnostic("failure", failureStage, failureNative);
            Diagnostic("stopped", failureStage ?? "eof", failureNative);
        }
        return failureStage is null ? 0 : 1;
    }

    private static async Task<ProtocolEnvelope?> ReadAsync(BoundedFrameReader reader, CancellationToken token)
    {
        var frame = await reader.ReadAsync(token);
        if (frame is null) return null;
        var envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(frame, JsonDefaults.Options) ?? throw new InvalidDataException();
        envelope.Validate();
        if (envelope.Id == Guid.Empty) throw new InvalidDataException();
        return envelope;
    }

    private static int? NativeCode(Exception failure) => failure switch
    {
        ExtensionFailure extension => extension.NativeCode,
        Win32Exception windows => windows.NativeErrorCode,
        _ => null
    };

    // Existing pipe-peer declarations are private to their owners. This one narrow query binds
    // the connected server to the already held/authenticated parent; no new native runtime.
    [DllImport("kernel32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetNamedPipeServerProcessId(SafePipeHandle pipe, out uint processId);
}
