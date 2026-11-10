using System.ComponentModel;
using System.IO.Pipes;
using System.ServiceProcess;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Owned pipe and injected transaction callbacks only; never touches installed SCM or releases.
internal static class TrustedUpdateBrokerLifecycleTests
{
    private static string PipeName() => "NxTools.UpdateLifecycle." + Guid.NewGuid().ToString("N");
    private static Task<Guid?> Serve(string name, CancellationToken token = default,
        Func<string, Guid, CancellationToken, Task>? apply = null, int handshakeMilliseconds = 2_000) =>
        TrustedUpdateBroker.ServeAsync(NullPlatformLog.Instance, token, () => Task.CompletedTask,
            apply, name, TimeSpan.FromMilliseconds(handshakeMilliseconds), TimeSpan.FromMilliseconds(150));

    private static async Task<NamedPipeClientStream> Connect(string name)
    {
        var client = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.Asynchronous);
        try { await client.ConnectAsync(2_000); return client; }
        catch { client.Dispose(); throw; }
    }

    internal static async Task OneShotHandshakesAndTimeouts()
    {
        var idle = PipeName();
        Require(await Serve(idle, handshakeMilliseconds: 150) is null, "unrequested service must finish its handshake deadline");
        var eof = PipeName();
        var eofTask = Serve(eof);
        using (await Connect(eof)) { }
        Require(await eofTask is null, "EOF must finish rather than reopen the pipe");
        var read = PipeName();
        var readTask = Serve(read);
        using (var client = await Connect(read))
        {
            Require(await readTask is null, "partial or silent client must finish its read deadline");
        }
        var invalid = PipeName();
        var invalidTask = Serve(invalid);
        using (var client = await Connect(invalid))
            await TrustedUpdateBroker.WriteFrameAsync(client,
                new { PackageDirectory = "", CorrelationId = Guid.Empty }, CancellationToken.None);
        Require(await invalidTask is null, "invalid frame must reject once and stop");
    }

    internal static async Task StartOnlyStoppedApprovedService()
    {
        var starts = 0;
        foreach (var current in new[] { ServiceControllerStatus.Running, ServiceControllerStatus.StartPending })
            TrustedUpdateBroker.StartIfStopped(() => current, () => starts++);
        Require(starts == 0, "running or start-pending service must not be started twice");
        TrustedUpdateBroker.StartIfStopped(() => ServiceControllerStatus.Stopped, () => starts++);
        Require(starts == 1, "stopped approved service must start exactly once");
        try
        {
            TrustedUpdateBroker.StartIfStopped(() => ServiceControllerStatus.StopPending, () => starts++);
            throw new InvalidOperationException("stop pending service accepted");
        }
        catch (InvalidOperationException failure) when (failure.Message == "Updater service is not available for demand start.") { }
        var status = ServiceControllerStatus.Stopped;
        TrustedUpdateBroker.StartIfStopped(() => status, () => { status = ServiceControllerStatus.StartPending; throw new Win32Exception(1056); });
        Require(starts == 1, "only an observed start-pending race may absorb already-running");
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();
        var outcome = await TrustedUpdateBroker.RequestAsync("", Guid.NewGuid(), cancelled.Token, NullPlatformLog.Instance);
        Require(!outcome.Success && outcome.Data == "cancelled" && outcome.NativeErrorCode == new OperationCanceledException().HResult,
            "cancelled update must return before enrollment, package or service start");
    }

    internal static async Task MaintenanceReadinessStopsAfterProbe()
    {
        var name = PipeName();
        var serving = Serve(name);
        var id = Guid.NewGuid();
        using (var client = await Connect(name))
        {
            await TrustedUpdateBroker.WriteFrameAsync(client,
                new { PackageDirectory = "", CorrelationId = id, MaintenanceProbe = true }, CancellationToken.None);
            var reply = await TrustedUpdateBroker.ReadFrameAsync<JsonElement>(client, CancellationToken.None);
            Require(reply.GetProperty("Success").GetBoolean() && reply.GetProperty("CorrelationId").GetGuid() == id &&
                reply.GetProperty("Stage").GetString() == "maintenance_ready", "probe must acknowledge once");
        }
        Require(await serving == id, "probe must complete the service's only request");
        using var second = new NamedPipeClientStream(".", name, PipeDirection.InOut, PipeOptions.Asynchronous);
        try { await second.ConnectAsync(150); throw new InvalidOperationException("second probe was accepted"); }
        catch (System.TimeoutException) { }
    }

    internal static async Task PreparedWorkIgnoresClientStop()
    {
        var name = PipeName();
        using var stop = new CancellationTokenSource();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var settle = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var id = Guid.NewGuid();
        var serving = Serve(name, stop.Token, async (_, received, token) =>
        {
            Require(received == id && !token.CanBeCanceled, "prepared work must not inherit the client/SCM stop token");
            entered.SetResult();
            await settle.Task;
        });
        using (var client = await Connect(name))
        {
            await TrustedUpdateBroker.WriteFrameAsync(client,
                new { PackageDirectory = Path.GetTempPath(), CorrelationId = id }, CancellationToken.None);
            await entered.Task;
        }
        stop.Cancel();
        Require(!serving.IsCompleted, "client disconnect and explicit stop must not interrupt settlement");
        settle.SetResult();
        Require(await serving == id, "settlement must finish before the one-shot service exits");
    }

    private static void Require(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }
}
