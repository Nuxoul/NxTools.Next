using System.Text.Json;
using System.Threading.Channels;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Injected samples and bounded queues only: never read or mutate the user's registry/theme.
internal static class SystemThemePreferenceTests
{
    private sealed class Reader : ISystemThemeReader
    {
        public ThemePreferenceSnapshot Next { get; set; }
        public bool ThrowOnRead { get; set; }
        public int Calls { get; private set; }
        public ThemePreferenceSnapshot Read()
        {
            Calls++;
            if (ThrowOnRead) throw new IOException("synthetic read failure");
            return Next;
        }
    }

    public static Task ReaderFallbackAndDedup()
    {
        var reader = new Reader { Next = new(true, 0) };
        var preference = new SystemThemePreference(reader);
        Assert(preference.Snapshot is { Dark: true, NativeErrorCode: -1 }, "unknown snapshot uses safe dark fallback");
        Assert(preference.Refresh(out var first) && first == reader.Next, "first sample must be observed even when dark");
        Assert(!preference.Refresh(out _), "identical samples are deduplicated");
        reader.Next = new(false, 0);
        Assert(preference.Refresh(out var light) && !light.Dark, "light transition is sampled");
        reader.Next = new(false, 5);
        Assert(preference.Refresh(out var failed) && failed is { Dark: true, NativeErrorCode: 5 }, "failed native read retains code and uses dark fallback");
        Assert(!preference.Refresh(out _), "repeated native error is deduplicated");
        reader.ThrowOnRead = true;
        Assert(preference.Refresh(out var exception) && exception is { Dark: true, NativeErrorCode: -2 }, "reader exception remains a typed snapshot");
        Assert(!preference.Refresh(out _), "repeated reader exception is deduplicated");
        reader.ThrowOnRead = false;
        reader.Next = new(false, 0);
        Assert(preference.Refresh(out var recovered) && !recovered.Dark, "recovery can return to light");
        Assert(reader.Calls == 8, "refresh reads exactly once; snapshot access never polls");
        return Task.CompletedTask;
    }

    public static Task FullQueueRetainsLatestAndStops()
    {
        var events = Channel.CreateBounded<WindowsPlatformEvent>(new BoundedChannelOptions(1)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = true
        });
        var pending = new PendingThemePublication();
        events.Writer.TryWrite(new SystemThemePreferenceChangedEvent(Guid.NewGuid(), true, 0));
        pending.Stage(new(false, 0));
        var firstCorrelation = pending.PendingCorrelationId;
        Assert(firstCorrelation is not null, "pending transition has correlation before admission");
        Assert(pending.TryPublish(value => events.Writer.TryWrite(value)) is null && pending.HasPending,
            "full queue cannot retire latest state");
        Assert(pending.PendingCorrelationId == firstCorrelation, "queue rejection preserves correlation");
        pending.Stage(new(true, 5));
        var latestCorrelation = pending.PendingCorrelationId;
        Assert(latestCorrelation is not null && latestCorrelation != firstCorrelation, "replacement has its own transition identity");
        events.Reader.TryRead(out _);
        var delivered = pending.TryPublish(value => events.Writer.TryWrite(value));
        Assert(delivered is { Dark: true, NativeErrorCode: 5 } && delivered.CorrelationId == latestCorrelation && !pending.HasPending,
            "retry publishes only latest coalesced snapshot");
        Assert(events.Reader.TryRead(out var queued) && queued == delivered, "bounded queue contains actual latest event");
        Assert(pending.TryPublish(_ => throw new InvalidOperationException("duplicate publication")) is null,
            "successful event is retired once");
        pending.Stage(new(false, 0));
        pending.Stop();
        pending.Stage(new(true, 0));
        Assert(!pending.HasPending && pending.PendingCorrelationId is null && pending.TryPublish(_ => true) is null,
            "stop clears state/correlation and rejects late stages");
        return Task.CompletedTask;
    }

    public static Task EventAndHandshakeWireRoundTrip()
    {
        foreach (var snapshot in new[] { new ThemePreferenceSnapshot(false, 0), new(true, 0), new(true, 5) })
        {
            var correlation = Guid.NewGuid();
            var transition = PlatformEventEnvelopeFactory.Create(new SystemThemePreferenceChangedEvent(
                correlation, snapshot.Dark, snapshot.NativeErrorCode));
            var handshake = PlatformPipeClient.CreateSystemThemeSnapshotEnvelope(snapshot, correlation);
            foreach (var envelope in new[] { transition, handshake })
            {
                var encoded = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
                var decoded = JsonSerializer.Deserialize<ProtocolEnvelope>(encoded, JsonDefaults.Options)
                    ?? throw new InvalidOperationException("missing decoded theme event");
                decoded.Validate();
                Assert(decoded.Type == "platform_event" && decoded.CorrelationId == correlation, "theme envelope correlation/type");
                Assert(decoded.Payload.GetProperty("event").GetString() == "system_theme_preference_changed", "existing protocol discriminator");
                var data = decoded.Payload.GetProperty("data");
                Assert(data.EnumerateObject().Count() == 3 && data.GetProperty("correlation_id").GetGuid() == correlation &&
                    data.GetProperty("dark").GetBoolean() == snapshot.Dark &&
                    data.GetProperty("native_error_code").GetInt32() == snapshot.NativeErrorCode, "exact Rust theme wire shape");
            }
        }
        return Task.CompletedTask;
    }

    public static Task OpenWithTargetValidationAndCancelledWire()
    {
        var root = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath();
        var directory = Path.Combine(root, $"nxtools-open-with-validation-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "synthetic.bin");
        File.WriteAllBytes(file, []);
        try
        {
            Assert(ShellTarget.TryCreate(file, "open_with", out var target, out _) && target.Kind == ShellTargetKind.OpenWith && target.Target == file,
                "typed Open With accepts exact absolute existing file");
            foreach (var invalid in new[] { directory, file + ".missing", "synthetic.bin", "https://example.invalid/file", @"C:synthetic.bin", file + "\nother" })
                Assert(!ShellTarget.TryCreate(invalid, "open_with", out _, out _), "Open With rejects non-file/non-absolute/multiline targets");
            var request = ProtocolEnvelope.Create("platform_command", new { Command = "open_shell_target", Data = new { Target = file, Action = "open_with" } });
            var result = PlatformPipeClient.CreateCommandResultEnvelope(request, new WindowsCommandResult(false, Data: "cancelled"));
            Assert(result.CorrelationId == request.Id && !result.Payload.GetProperty("success").GetBoolean() &&
                result.Payload.GetProperty("error").ValueKind == JsonValueKind.Null &&
                result.Payload.GetProperty("data").GetString() == "cancelled", "cancelled result remains non-error/non-success for Rust window retention");
        }
        finally { Directory.Delete(directory, true); }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
