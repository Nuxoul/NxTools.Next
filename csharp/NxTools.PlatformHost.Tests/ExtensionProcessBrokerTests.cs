using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Extensions;
using static NxTools.PlatformHost.Tests.ExtensionProcessTests;

namespace NxTools.PlatformHost.Tests;

internal static class ExtensionProcessBrokerTests
{
    public static async Task ControlResponsivenessAndParentEof()
    {
        using var broker = Broker();
        try
        {
            var run = Request("duplex"); await Send(broker, run);
            await Event(broker, e => e.Op == "stdout" && e.Data == "host-call");
            var invalid = Guid.NewGuid();
            await Send(broker, new ExtensionControlRequest(1, "cancel", invalid));
            var rejected = await Event(broker, e => e.RunId == invalid);
            Assert(rejected.Reason == "unknown_run", "control reader must respond while child waits for host call");
            await Send(broker, new ExtensionControlRequest(1, "write", run.RunId, "reply"));
            await Event(broker, e => e.Op == "stdout" && e.Data == "reply");
            await Event(broker, e => e.Op == "exited");
            var sleeping = Request("sleep"); await Send(broker, sleeping);
            var started = await Event(broker, e => e.Op == "started");
            using var child = Process.GetProcessById((int)started.ProcessId!);
            broker.StandardInput.Close(); await broker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(child.HasExited, "control EOF must terminate owned job before broker exit");
        }
        finally { KillOwned(broker); }
    }

    public static async Task BrokerCrashAndActualParentDeath()
    {
        using (var broker = Broker())
        {
            try
            {
                await Send(broker, Request("sleep"));
                var started = await Event(broker, e => e.Op == "started");
                using var child = Process.GetProcessById((int)started.ProcessId!);
                broker.Kill(); await broker.WaitForExitAsync();
                await Until(() => child.HasExited);
            }
            finally { KillOwned(broker); }
        }
        var root = Path.Combine(Path.GetTempPath(), "NxTools-extension-" + Guid.NewGuid()); Directory.CreateDirectory(root);
        Process? ownedBroker = null;
        using var parent = ExtensionProcessFixture.Start("--extension-fixture-parent", Path.Combine(root, "broker"),
            JsonSerializer.Serialize(Request("tree", Path.Combine(root, "child")), ExtensionProcessProtocol.Json));
        try
        {
            await Until(() => File.Exists(Path.Combine(root, "child")));
            ownedBroker = Process.GetProcessById(int.Parse(File.ReadAllText(Path.Combine(root, "broker"))));
            var descendantPid = int.Parse(File.ReadAllText(Path.Combine(root, "child")).Trim());
            parent.Kill(); await parent.WaitForExitAsync();
            await ownedBroker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(Gone(descendantPid), "authenticated parent death must kill tree even without control EOF");
        }
        finally { KillOwned(parent); if (ownedBroker is not null) { KillOwned(ownedBroker); ownedBroker.Dispose(); } Directory.Delete(root, true); }
    }

    public static async Task ParentIdentityAndMalformedControl()
    {
        using (var denied = ExtensionProcessFixture.Start("--extension-fixture-broker", Environment.ProcessId.ToString(),
            Path.Combine(AppContext.BaseDirectory, "not-the-parent.exe")))
        {
            try
            {
                var rejected = await Event(denied, e => e.Op == "rejected");
                Assert(rejected.Reason == "parent_image_mismatch", "real executable policy must reject before reading start");
                await denied.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
            }
            finally { KillOwned(denied); }
        }
        using var broker = Broker();
        try
        {
            var id = Guid.NewGuid();
            await broker.StandardInput.WriteLineAsync($"{{\"version\":1,\"op\":\"cancel\",\"run_id\":\"{id}\",\"op\":\"write\"}}");
            await broker.StandardInput.FlushAsync();
            Assert((await Event(broker, e => e.Op == "rejected")).Reason == "duplicate_control_key", "duplicate JSON keys must not select another op");
            await broker.StandardInput.BaseStream.WriteAsync(new byte[] { 0xc3, 0x28, 10 });
            await broker.StandardInput.BaseStream.FlushAsync();
            Assert((await Event(broker, e => e.Op == "rejected")).Reason == "control_invalid_utf8", "real broker control is strict UTF8");
            await broker.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(10));
        }
        finally { KillOwned(broker); }
    }

    private static Process Broker() => ExtensionProcessFixture.Start("--extension-fixture-broker", Environment.ProcessId.ToString(), Environment.ProcessPath!);
    private static async Task Send(Process broker, object value)
    { await broker.StandardInput.WriteLineAsync(JsonSerializer.Serialize(value, ExtensionProcessProtocol.Json)); await broker.StandardInput.FlushAsync(); }
    private static async Task<ExtensionProcessEvent> Event(Process broker, Func<ExtensionProcessEvent, bool> match)
    {
        while (true)
        {
            var line = await broker.StandardOutput.ReadLineAsync().WaitAsync(TimeSpan.FromSeconds(10));
            Assert(line is not null, "broker protocol unexpectedly closed");
            var value = JsonSerializer.Deserialize<ExtensionProcessEvent>(line!, ExtensionProcessProtocol.Json)!;
            if (match(value)) return value;
        }
    }
    private static void KillOwned(Process process)
    { if (!process.HasExited) { process.Kill(entireProcessTree: true); process.WaitForExit(5000); } }
}
