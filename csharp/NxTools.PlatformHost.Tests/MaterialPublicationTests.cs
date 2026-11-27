using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class MaterialPublicationTests
{
    internal static Task LatestOnlySessionAndStop()
    {
        var session = Guid.NewGuid();
        var first = Snapshot(session, 1);
        var latest = Snapshot(session, 3);
        var pending = new PendingMaterialPublication();
        var queue = Channel.CreateBounded<WindowsPlatformEvent>(1);
        Check(queue.Writer.TryWrite(first), "fill original bounded event queue");
        pending.Stage(first);
        Check(pending.TryPublish(session, value => queue.Writer.TryWrite(value)) is null && pending.HasPending,
            "full queue must retain the pending state");
        pending.Stage(latest);
        Check(queue.Reader.TryRead(out _), "drain occupied slot");
        Check(pending.TryPublish(session, value => queue.Writer.TryWrite(value)) == latest,
            "queue retry must deliver only the latest revision");
        Check(queue.Reader.TryRead(out var delivered) && delivered == latest && !pending.HasPending,
            "successful latest publication retires once");
        pending.Stage(first);
        Check(pending.TryPublish(Guid.NewGuid(), _ => throw new InvalidOperationException("stale delivery")) is null && !pending.HasPending,
            "reconnect must retire the previous session without delivery");
        pending.Stage(latest);
        pending.Stop();
        pending.Stage(first);
        Check(!pending.HasPending && pending.TryPublish(session, _ => true) is null,
            "Stop must prevent late capability publication");
        return Task.CompletedTask;
    }

    internal static Task NegotiatedEnvelopeAndSessionFilter()
    {
        var session = Guid.NewGuid();
        var facts = Snapshot(session, 9);
        var accepted = ProtocolEnvelope.Create("hello_ack", new { AcceptedCapabilities = new[] { "window_material_v1" } });
        var legacy = ProtocolEnvelope.Create("hello_ack", new { AcceptedCapabilities = Array.Empty<string>() });
        Check(PlatformCapabilities.Create(false).Contains("window_material_v1"), "host advertises the current capability");
        Check(PlatformPipeClient.AcceptsWindowMaterial(accepted) && !PlatformPipeClient.AcceptsWindowMaterial(legacy),
            "new material events require explicit negotiation");
        Check(PlatformPipeClient.CanForwardMaterialEvent(facts, true, session), "current accepted session forwards");
        Check(!PlatformPipeClient.CanForwardMaterialEvent(facts, false, session)
            && !PlatformPipeClient.CanForwardMaterialEvent(facts, true, Guid.NewGuid())
            && !PlatformPipeClient.CanForwardMaterialEvent(facts, true, Guid.Empty), "foreign/legacy sessions reject");
        Check(PlatformPipeClient.CanForwardMaterialEvent(new SystemThemePreferenceChangedEvent(Guid.NewGuid(), false, 0), false, Guid.Empty),
            "ordinary theme events keep their original route");
        var envelope = PlatformEventEnvelopeFactory.Create(facts);
        using var document = JsonDocument.Parse(JsonSerializer.Serialize(envelope, JsonDefaults.Options));
        var root = document.RootElement;
        var payload = root.GetProperty("payload");
        Check(payload.GetProperty("event").GetString() == "system_material_capabilities_changed", "exact Rust event variant");
        var data = payload.GetProperty("data");
        Check(data.GetProperty("session_id").GetGuid() == session && data.GetProperty("revision").GetUInt64() == 9,
            "typed session/revision survive serialization");
        Check(root.GetProperty("correlation_id").GetGuid() == data.GetProperty("correlation_id").GetGuid(),
            "same request correlation spans envelope and data");
        Check(data.EnumerateObject().Select(field => field.Name).SequenceEqual(new[]
            {
                "correlation_id", "session_id", "revision", "supported", "transparency_enabled", "high_contrast",
                "energy_saver", "remote_session", "native_error_code", "system_backdrop", "rounded_corners"
            }) && data.GetProperty("transparency_enabled").GetBoolean(),
            "facts contain exactly the released wire fields");
        Check(data.GetProperty("system_backdrop").GetBoolean() && !data.GetProperty("rounded_corners").GetBoolean(),
            "DWM backdrop and corner capabilities serialize as independent booleans");
        return Task.CompletedTask;
    }

    internal static async Task DetachedCommandKeepsReaderResponsive()
    {
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var dispatcher = new PlatformCommandSequenceDispatcher();
        try
        {
            await dispatcher.DispatchAsync("apply_window_material", async () => { entered.SetResult(); await release.Task; });
            await entered.Task.WaitAsync(TimeSpan.FromSeconds(5));
            var later = false;
            await dispatcher.DispatchAsync("set_window_visibility", () => { later = true; return Task.CompletedTask; });
            Check(later, "a pending material command cannot block the original pipe reader");
        }
        finally { release.TrySetResult(); await dispatcher.DrainAsync(); }
    }

    internal static async Task ActualPipeRouteUsesBoundOwner()
    {
        var native = new RouteNative();
        var owner = new WindowMaterialOwner(native: native);
        owner.UpdateCapabilities(new(true, true, false, false, false, 0, false, false));
        var session = owner.BindPeer(42);
        var adapter = (WindowsPlatformAdapter)System.Runtime.CompilerServices.RuntimeHelpers.GetUninitializedObject(typeof(WindowsPlatformAdapter));
        typeof(WindowsPlatformAdapter).GetField("_windowMaterials", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)!.SetValue(adapter, owner);
        var client = new PlatformPipeClient("unused-owned-material-route", adapter, NxTools.PlatformHost.Diagnostics.NullPlatformLog.Instance);
        var execute = typeof(PlatformPipeClient).GetMethods(System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance)
            .Single(method => method.Name == "ExecuteCommandAsync" && method.GetParameters().Length == 5 && method.GetParameters()[4].ParameterType == typeof(uint));
        var command = new ApplyWindowMaterialCommand(session, 1, 99, 1, "acrylic", "backdrop", false);
        var outgoing = Channel.CreateBounded<ProtocolEnvelope>(1);
        async Task<ProtocolEnvelope> Send(object data)
        {
            var request = ProtocolEnvelope.Create("platform_command", new { Command = "apply_window_material", Data = data });
            await (Task)execute.Invoke(client, [request, "apply_window_material", outgoing.Writer, CancellationToken.None, 42u])!;
            var response = await outgoing.Reader.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(5));
            Check(response.CorrelationId == request.Id, "actual router reply retains original request identity");
            return response;
        }
        var applied = await Send(command);
        Check(applied.Payload.GetProperty("success").GetBoolean() && native.Writes == 3,
            "actual route reaches the bound owner on the original bounded worker");
        var receipt = JsonSerializer.Deserialize<JsonElement>(applied.Payload.GetProperty("data").GetString()!);
        Check(receipt.GetProperty("session_id").GetGuid() == session && receipt.GetProperty("effective").GetString() == "acrylic"
            && !receipt.GetProperty("dark").GetBoolean(), "actual route returns qualified effective state and explicit Light");
        owner.BindPeer(0);
        var rejected = await Send(command with { Generation = 2 });
        Check(!rejected.Payload.GetProperty("success").GetBoolean() && native.Writes == 3,
            "disconnect rejects an actual routed request before native writes");
        var invalid = await Send(new { private_input = "material-private-fixture" });
        Check(!invalid.Payload.GetProperty("success").GetBoolean()
            && invalid.Payload.GetProperty("error").GetString() == "window_material_failed",
            "malformed route returns a fixed native error category");
    }

    private sealed class RouteNative : IWindowMaterialNative
    {
        internal int Writes;
        public int ValidateWindow(long window, uint processId) => window == 99 && processId == 42 ? 0 : 5;
        public bool SupportsSystemBackdrop => false;
        public bool SupportsRoundedCorners => false;
        public bool SupportsAlphaComposition(long window) => true;
        public int SetDark(long window, bool dark) { Writes++; return 0; }
        public int SetBackdropType(long window, int type) => throw new InvalidOperationException("Win10 does not write DWM38");
        public int SetRoundedCorners(long window) => throw new InvalidOperationException("Win10 does not write DWM33/34");
        public int ExtendFrame(long window, bool fullClient) { Writes++; return fullClient ? 87 : 0; }
        public int SetAccent(long window, int state) { Writes++; return state == 5 ? 0 : 87; }
        public int SetHostBackdropBrush(long window, bool enabled) => throw new InvalidOperationException("Win10 does not write DWM17");
    }

    private static SystemMaterialCapabilitiesChangedEvent Snapshot(Guid session, ulong revision) =>
        new(Guid.NewGuid(), session, revision, true, true, false, false, false, 0, true, false);
    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
