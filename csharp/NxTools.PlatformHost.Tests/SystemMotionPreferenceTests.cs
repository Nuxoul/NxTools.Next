using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class SystemMotionPreferenceTests
{
    private sealed class Reader : ISystemMotionReader
    {
        public MotionPreferenceSnapshot Next { get; set; }
        public int Calls { get; private set; }
        public MotionPreferenceSnapshot Read() { Calls++; return Next; }
    }

    public static Task SyntheticReaderChangeDedupAndWireRoundTrip()
    {
        var reader = new Reader { Next = new MotionPreferenceSnapshot(false, 0) };
        var preference = new SystemMotionPreference(reader);
        Assert(preference.Snapshot.ReducedMotion, "Unknown must reduce motion");
        Assert(preference.Refresh(out var first) && !first.ReducedMotion, "Native enabled allows motion");
        Assert(!preference.Refresh(out _), "Identical native sample is a no-op");
        reader.Next = new MotionPreferenceSnapshot(true, 0);
        Assert(preference.Refresh(out var disabled) && disabled.ReducedMotion, "Native disabled reduces motion");
        reader.Next = new MotionPreferenceSnapshot(false, 5);
        Assert(preference.Refresh(out var failed) && failed.ReducedMotion && failed.NativeErrorCode == 5,
            "Failed native read must fail closed and retain native code");
        Assert(!preference.Refresh(out _), "Repeated failure is deduplicated");
        Assert(reader.Calls == 5, "Each synthetic native change must sample, not read persisted state");
        foreach (var snapshot in new[] { first, disabled, failed })
        {
            var correlationId = Guid.NewGuid();
            var envelope = PlatformEventEnvelopeFactory.Create(new SystemMotionPreferenceChangedEvent(
                correlationId, snapshot.ReducedMotion, snapshot.NativeErrorCode));
            var encoded = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
            var decoded = JsonSerializer.Deserialize<ProtocolEnvelope>(encoded, JsonDefaults.Options)
                ?? throw new InvalidOperationException("Missing decoded motion event");
            decoded.Validate();
            Assert(decoded.Type == "platform_event" && decoded.CorrelationId == correlationId,
                "Motion event must preserve protocol type and correlation");
            Assert(decoded.Payload.GetProperty("event").GetString() == "system_motion_preference_changed",
                "Motion discriminator mismatch");
            var data = decoded.Payload.GetProperty("data");
            Assert(data.GetProperty("correlation_id").GetGuid() == correlationId &&
                data.GetProperty("reduced_motion").GetBoolean() == snapshot.ReducedMotion &&
                data.GetProperty("native_error_code").GetInt32() == snapshot.NativeErrorCode,
                "Motion state/native code changed on the wire");
        }
        var pending = new PendingMotionPublication();
        pending.Stage(first);
        var pendingCorrelation = pending.PendingCorrelationId;
        Assert(pendingCorrelation is not null, "pending snapshot keeps its diagnostic correlation");
        Assert(pending.TryPublish(_ => false) is null && pending.HasPending, "full queue retains pending state");
        Assert(pending.PendingCorrelationId == pendingCorrelation, "rejected delivery preserves correlation");
        pending.Stage(disabled);
        var delivered = pending.TryPublish(_ => true);
        Assert(delivered is { ReducedMotion: true, NativeErrorCode: 0 } && !pending.HasPending,
            "available queue publishes only latest coalesced snapshot");
        Assert(pending.TryPublish(_ => throw new InvalidOperationException("duplicate delivery")) is null,
            "published snapshot is retired exactly once");
        pending.Stage(failed);
        pending.Stop();
        pending.Stage(first);
        Assert(!pending.HasPending && pending.TryPublish(_ => true) is null, "stop clears and rejects late stage");
        Assert(pending.PendingCorrelationId is null, "stop clears diagnostic correlation with the slot");
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string reason)
    {
        if (!condition) throw new InvalidOperationException(reason);
    }
}
