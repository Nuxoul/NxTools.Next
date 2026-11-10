using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

// Read-only, source-free held namespace observation. Rust alone assigns batch names.
internal static class ArchiveCreateDestination
{
    internal static Task DispatchAsync(PlatformCommandSequenceDispatcher dispatcher, ArchiveCreateDestinationCommand command,
        IPlatformLog? log, CancellationToken cancellation, Func<WindowsCommandResult, CancellationToken, Task> publish) =>
        dispatcher.DispatchAsync("archive_entry", async token => {
            var result = Run(command, log, token);
            if (!token.IsCancellationRequested) await publish(result, token);
        }, cancellation);

    internal static WindowsCommandResult Run(ArchiveCreateDestinationCommand command, IPlatformLog? log = null, CancellationToken cancellation = default)
    {
        if (BoundedStaOperationRunner.TryRun("NxTools.ArchiveCreateDestination", TimeSpan.FromSeconds(5), 2,
            token => Observe(command, log, token), out WindowsCommandResult? result, out _, cancellation))
            return result ?? Failure(command, "destination_unavailable");
        return Failure(command, cancellation.IsCancellationRequested ? "cancelled" : "destination_unavailable");
    }

    internal static WindowsCommandResult Observe(ArchiveCreateDestinationCommand command, IPlatformLog? log = null, CancellationToken cancellation = default)
    {
        var clock = Stopwatch.StartNew();
        void Diagnostic(string phase, string? code = null, int? native = null)
        {
            try { log?.Write("info", "archive.create_destination." + phase, command.OperationId,
                new Dictionary<string, object?> { ["session_id"] = command.SessionId, ["generation"] = command.Generation,
                    ["code"] = code, ["native_error_code"] = native, ["elapsed_ms"] = clock.ElapsedMilliseconds }); }
            catch { /* Diagnostics do not affect observation. */ }
        }
        Diagnostic("accepted");
        Diagnostic("started");
        WindowsCommandResult Fail(string code, int? native = null)
        { Diagnostic("failed", code, native); return Failure(command, code, native); }
        try
        {
            if (!command.Valid) return Fail("invalid_request");
            cancellation.ThrowIfCancellationRequested();
            using var held = ArchiveOutputSession.Prepare(command.Destination, command.OperationId);
            var (missing, names) = held.ObserveCreateNames(cancellation);
            cancellation.ThrowIfCancellationRequested();
            Diagnostic("finished");
            return new(true, Data: JsonSerializer.Serialize(new ArchiveCreateDestinationResult(1, command.OperationId,
                command.SessionId, command.Generation, missing, names), JsonDefaults.Options));
        }
        catch (OperationCanceledException) { return Fail("cancelled"); }
        catch (ArchiveOutputException ex) { return Fail(ex.Stage == "destination_limit" ? "destination_limit" : "destination_unavailable", ex.NativeCode); }
        catch (UnauthorizedAccessException ex) { return Fail("destination_unavailable", ex.HResult & 0xffff); }
        catch (IOException ex) { return Fail("destination_unavailable", ex.HResult & 0xffff); }
        catch { return Fail("destination_unavailable"); }
    }
    private static WindowsCommandResult Failure(ArchiveCreateDestinationCommand command, string code, int? native = null) =>
        new(false, code, NativeErrorCode: native, Data: JsonSerializer.Serialize(new ArchiveCreateDestinationResult(1,
            command.OperationId, command.SessionId, command.Generation, false, [], code, native), JsonDefaults.Options));
}
