using System.Runtime.InteropServices;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class WindowMaterialTests
{
    private static readonly MaterialCapabilitySnapshot Enabled = new(true, true, false, false, false, 0, true, true);
    private static readonly string[] Styles = ["solid", "acrylic"];
    private static readonly string[] Surfaces = ["backdrop", "transient", "transparent"];
    private static void Check(bool value, string reason) { if (!value) throw new InvalidOperationException(reason); }
    private static ProtocolEnvelope Wire(string data) => ProtocolEnvelope.Create("platform_command", new
    { command = "apply_window_material", data = JsonSerializer.Deserialize<JsonElement>(data) });
    private static ApplyWindowMaterialCommand Command(Guid session, ulong revision = 1, ulong generation = 1) =>
        new(session, revision, -10, generation, "acrylic", "backdrop", true);
    private static JsonElement Receipt(WindowsCommandResult result)
    {
        Check(result.Success && result.Data is not null, "successful JSON receipt required");
        return JsonSerializer.Deserialize<JsonElement>(result.Data!);
    }
    private static string Field(WindowsCommandResult result, string field) => Receipt(result).GetProperty(field).GetString()!;

    // Exact native order per outcome; "{type}" is the DWMWA_SYSTEMBACKDROP_TYPE value.
    private static string[] Win11Frosted(int type) => ["dark:True", "accent:0", "frame:full", $"backdrop:{type}", "corners"];
    private static readonly string[] Win11Opaque = ["dark:True", "backdrop:1", "accent:0", "frame:none", "corners"];
    private static readonly string[] Win11Transparent = ["dark:True", "backdrop:1", "accent:0"];
    private static readonly string[] Win10Frosted = ["dark:True", "accent:5", "frame:none"];
    private static readonly string[] Win10Opaque = ["dark:True", "accent:0", "frame:none"];
    private static readonly string[] Win10Transparent = ["dark:True", "accent:0"];
    private static readonly string[] Reset = ["backdrop:1", "accent:0", "frame:none"];

    internal static Task StrictWireAndReceipt()
    {
        var command = Command(Guid.NewGuid()) with { FactsRevision = ulong.MaxValue, Generation = ulong.MaxValue };
        var data = JsonSerializer.Serialize(command, JsonDefaults.Options);
        Check(WindowMaterialContract.Parse(Wire(data)) == command, "full u64 and negative HWND survive parsing");
        foreach (var style in Styles)
        foreach (var surface in Surfaces)
        {
            var variant = command with { Style = style, Surface = surface };
            Check(WindowMaterialContract.Parse(Wire(JsonSerializer.Serialize(variant, JsonDefaults.Options))) == variant,
                "every released style and surface parses");
        }
        foreach (var bad in new[]
        {
            "{}", "[]", "null", data.Replace("\"acrylic\"", "\"Acrylic\""), data.Replace("\"acrylic\"", "\"frosted\""),
            data.Replace("\"acrylic\"", "\"mica\""), data.Replace("\"backdrop\"", "\"glass\""),
            data.Replace("\"backdrop\"", "\"Transient\""), data.Replace("\"dark\":true", "\"dark\":1"),
            data.Replace("\"window_handle\":-10", "\"window_handle\":0"),
            data.Replace(ulong.MaxValue.ToString(), "0"), data.Replace(ulong.MaxValue.ToString(), "-1"),
            data.Replace(ulong.MaxValue.ToString(), "18446744073709551616"),
            data.Replace(command.SessionId.ToString(), Guid.Empty.ToString()),
            data[..^1] + ",\"dark\":true}", data[..^1] + ",\"extra\":false}",
            data.Replace("\"surface\":\"backdrop\",", ""), data.Replace("\"generation\"", "\"Generation\"")
        })
        {
            try { WindowMaterialContract.Parse(Wire(bad)); throw new Exception("invalid material shape accepted"); }
            catch (InvalidDataException) { }
        }
        foreach (var badEnvelope in new[] { Wire(data) with { Type = "event" }, Wire(data) with { Id = Guid.Empty },
            Wire(data) with { ProtocolVersion = 2 },
            ProtocolEnvelope.Create("platform_command", new { command = "other", data = JsonSerializer.Deserialize<JsonElement>(data) }) })
        {
            try { WindowMaterialContract.Parse(badEnvelope); throw new Exception("invalid envelope accepted"); }
            catch (InvalidDataException) { }
        }
        var receipt = JsonSerializer.Deserialize<JsonElement>(WindowMaterialContract.Receipt(command, "acrylic_opaque", "energy_saver", 5));
        Check(receipt.EnumerateObject().Select(p => p.Name).Order().SequenceEqual(new[]
        { "session_id", "facts_revision", "window_handle", "generation", "style", "surface", "dark", "effective", "fallback_reason", "native_error_code" }.Order()), "receipt exact snake_case fields");
        Check(receipt.GetProperty("session_id").GetGuid() == command.SessionId
            && receipt.GetProperty("facts_revision").GetUInt64() == ulong.MaxValue
            && receipt.GetProperty("window_handle").GetInt64() == -10
            && receipt.GetProperty("generation").GetUInt64() == ulong.MaxValue
            && receipt.GetProperty("native_error_code").GetInt32() == 5, "receipt scalar types and qualification");
        return Task.CompletedTask;
    }

    internal static Task PreferenceAndSessionFacts()
    {
        var reader = new Reader { Value = Enabled };
        var preference = new SystemMaterialPreference(reader);
        Check(preference.Refresh(out var initial) && initial == Enabled, "first sample publishes");
        Check(!preference.Refresh(out _), "equal facts coalesce");
        reader.Value = Enabled with { EnergySaver = true };
        Check(preference.Refresh(out var changed) && changed.EnergySaver, "power transition sampled");
        reader.Throw = true;
        Check(preference.Refresh(out var failed) && failed.NativeErrorCode != 0 && !failed.TransparencyEnabled
            && !failed.SystemBackdrop && !failed.RoundedCorners, "reader failure forces conservative snapshot");
        Check(preference.Snapshot == failed, "snapshot is last sampled facts");
        var live = new WindowsSystemMaterialReader().Read();
        Check(WindowMaterialNative.ReadVersion(out var major, out var build) != 0
            || live.SystemBackdrop == WindowMaterialNative.HasSystemBackdrop(major, build)
            && live.RoundedCorners == WindowMaterialNative.HasRoundedCorners(major, build), "reader derives DWM capabilities from the OS build");
        var owner = new WindowMaterialOwner(native: new Native());
        Check(owner.BindPeer(42) != Guid.Empty && owner.SnapshotEvent() is null, "connection alone is not facts readiness");
        Check(owner.UpdateCapabilities(Enabled) is { Revision: 1, SystemBackdrop: true, RoundedCorners: true }, "first sample revision one");
        Check(owner.UpdateCapabilities(Enabled) is null, "duplicate fact does not increment");
        Check(owner.UpdateCapabilities(changed) is { Revision: 2 }, "fact transition increments");
        Check(owner.UpdateCapabilities(changed with { SystemBackdrop = false }) is { Revision: 3, SystemBackdrop: false, RoundedCorners: true },
            "DWM capability transition is a fact transition");
        var first = owner.SnapshotEvent()!; var second = owner.SnapshotEvent()!;
        Check(first.SessionId == second.SessionId && first.Revision == second.Revision
            && first.CorrelationId != Guid.Empty && first.CorrelationId != second.CorrelationId, "replay fresh correlation, same facts revision");
        Check(owner.BindPeer(0) == Guid.Empty && owner.SnapshotEvent() is null, "disconnect revokes publication");
        Check(owner.UpdateCapabilities(Enabled) is null, "facts retained without peer");
        var next = owner.BindPeer(42);
        Check(next != first.SessionId && owner.SnapshotEvent() is { Revision: 1, EnergySaver: false, SystemBackdrop: true },
            "rebind retains facts and resets revision");
        return Task.CompletedTask;
    }

    internal static Task OrderedFallbackAndExemptSurfaces()
    {
        var cases = new (MaterialCapabilitySnapshot Facts, string Reason)[]
        {
            (new(false, false, true, true, true, 123, true, true), "capability_read_failed"),
            (new(false, false, true, true, true, 0, true, true), "unsupported"),
            (new(true, false, true, true, true, 0, true, true), "high_contrast"),
            (new(true, false, false, true, true, 0, true, true), "transparency_disabled"),
            (new(true, true, false, true, true, 0, true, true), "energy_saver"),
            (new(true, true, false, false, true, 0, true, true), "remote_session"),
            (Enabled, "none")
        };
        foreach (var (facts, reason) in cases)
        foreach (var surface in Surfaces)
        foreach (var style in Styles)
        {
            var native = new Native(); var owner = new WindowMaterialOwner(native: native);
            owner.UpdateCapabilities(facts); var command = Command(owner.BindPeer(42)) with { Style = style, Surface = surface };
            var result = owner.Apply(command, Guid.NewGuid(), CancellationToken.None);
            var frosted = style != "solid" && surface != "transparent";
            var effective = surface == "transparent" ? "transparent" : style == "solid" ? "solid" : reason == "none" ? "acrylic" : "acrylic_opaque";
            Check(Field(result, "effective") == effective && Field(result, "fallback_reason") == (frosted ? reason : "none"), "ordered facts and explicit identities");
            var expected = effective == "transparent" ? Win11Transparent : effective == "acrylic"
                ? Win11Frosted(3) : Win11Opaque;
            Check(native.Calls.SequenceEqual(expected), $"Windows 11 {style}/{surface}/{effective} native order");
            Check(Receipt(result).GetProperty("native_error_code").GetInt32() == (frosted ? facts.NativeErrorCode : 0), "capability error only belongs to capability fallback");
        }
        foreach (var style in Styles)
        foreach (var surface in Surfaces)
        {
            var win10 = new Native { SupportsSystemBackdrop = false, SupportsRoundedCorners = false };
            var win10Owner = new WindowMaterialOwner(native: win10); win10Owner.UpdateCapabilities(Enabled with { SystemBackdrop = false, RoundedCorners = false });
            var result = win10Owner.Apply(Command(win10Owner.BindPeer(42)) with { Style = style, Surface = surface }, Guid.NewGuid(), default);
            var effective = surface == "transparent" ? "transparent" : style == "solid" ? "solid" : "acrylic";
            Check(Field(result, "effective") == effective && Field(result, "fallback_reason") == "none",
                "Windows 10 accent blur is effective acrylic for every frosted style");
            Check(win10.Calls.SequenceEqual(effective == "transparent" ? Win10Transparent : effective == "acrylic" ? Win10Frosted : Win10Opaque),
                $"Windows 10 {style}/{surface} writes accent only, never DWM38/33/34");
        }
        var rounded = new Native { SupportsSystemBackdrop = false };
        var roundedOwner = new WindowMaterialOwner(native: rounded); roundedOwner.UpdateCapabilities(Enabled with { SystemBackdrop = false });
        Check(Field(roundedOwner.Apply(Command(roundedOwner.BindPeer(42)) with { Style = "acrylic" }, Guid.NewGuid(), default), "effective") == "acrylic"
            && rounded.Calls.SequenceEqual(Win10Frosted.Append("host:1").Append("corners")), "Windows 11 before 22621 frosts through the host backdrop brush and still rounds");
        foreach (var surface in Surfaces)
        foreach (var style in Styles)
        {
            var native = new Native { AlphaComposition = false };
            var owner = new WindowMaterialOwner(native: native); owner.UpdateCapabilities(Enabled);
            var command = Command(owner.BindPeer(42)) with { Style = style, Surface = surface };
            var result = owner.Apply(command, Guid.NewGuid(), default);
            var frosted = style != "solid" && surface != "transparent";
            Check(Field(result, "effective") == (frosted ? "acrylic_opaque" : surface == "transparent" ? "transparent" : "solid")
                && Field(result, "fallback_reason") == (frosted ? "unsupported" : "none"),
                "redirected GPUI swapchain cannot report Acrylic acceptance");
            Check(native.Calls.SequenceEqual(surface == "transparent" ? Win11Transparent : Win11Opaque),
                "non-composition window confirms the opaque state without a system backdrop");
        }
        var unreadable = new Native { ThrowAlphaRead = true };
        var unreadableOwner = new WindowMaterialOwner(native: unreadable); unreadableOwner.UpdateCapabilities(Enabled);
        var failedRead = unreadableOwner.Apply(Command(unreadableOwner.BindPeer(42)) with { Surface = "transient" }, Guid.NewGuid(), default);
        Check(!failedRead.Success && failedRead.NativeErrorCode == 1400 && unreadable.Calls.Count == 0,
            "composition read failure cannot write or publish a successful fallback");
        var versionless = new Native { ThrowVersionRead = true };
        var versionOwner = new WindowMaterialOwner(native: versionless); versionOwner.UpdateCapabilities(Enabled);
        var failedVersion = versionOwner.Apply(Command(versionOwner.BindPeer(42)), Guid.NewGuid(), default);
        Check(!failedVersion.Success && failedVersion.Error == "material_version_failed" && versionless.Calls.Count == 0,
            "OS version read failure performs no writes");
        return Task.CompletedTask;
    }

    internal static Task RejectBeforeWritesAndRetireGenerations()
    {
        foreach (var code in new[] { 1400, 5, 87 })
        {
            var native = new Native { ValidationError = code }; var owner = new WindowMaterialOwner(native: native);
            owner.UpdateCapabilities(Enabled); var result = owner.Apply(Command(owner.BindPeer(42)), Guid.NewGuid(), default);
            Check(!result.Success && result.NativeErrorCode == code && native.Calls.Count == 0, "foreign/root/session native rejection before writes");
        }
        var n = new Native(); var o = new WindowMaterialOwner(native: n); var session = o.BindPeer(42); var c = Command(session);
        Check(!o.Apply(c, Guid.NewGuid(), default).Success && n.Calls.Count == 0, "unsampled facts cannot apply");
        o.UpdateCapabilities(Enabled);
        foreach (var bad in new[] { c with { SessionId = Guid.NewGuid() }, c with { FactsRevision = 2 }, c with { Generation = 0 },
            c with { WindowHandle = 0 }, c with { Style = "frosted" }, c with { Surface = "popup" } })
            Check(!o.Apply(bad, Guid.NewGuid(), default).Success && n.Calls.Count == 0, "invalid qualifications perform no writes");
        using var cancel = new CancellationTokenSource(); cancel.Cancel();
        Check(!o.Apply(c, Guid.NewGuid(), cancel.Token).Success && n.Calls.Count == 0, "pre-cancelled operation cannot write");
        Check(o.Apply(c with { Generation = 2 }, Guid.NewGuid(), default).Success, "new generation succeeds");
        var count = n.Calls.Count;
        foreach (var stale in new[] { c, c with { Generation = 2 } })
            Check(!o.Apply(stale, Guid.NewGuid(), default).Success && n.Calls.Count == count, "equal and older generations cannot replay");
        o.UpdateCapabilities(Enabled with { RemoteSession = true });
        Check(!o.Apply(c with { Generation = 3 }, Guid.NewGuid(), default).Success && n.Calls.Count == count, "old revision rejected");
        o.BindPeer(0);
        Check(!o.Apply(c, Guid.NewGuid(), default).Success && n.Calls.Count == count, "disconnect invalidates old session");
        var replacement = o.BindPeer(42);
        Check(o.Apply(Command(replacement), Guid.NewGuid(), default).Success, "new session clears window generation");
        return Task.CompletedTask;
    }

    internal static Task NativeFailureRequiresSuccessfulReset()
    {
        var plan = Win11Frosted(3);
        foreach (var surface in new[] { "backdrop", "transient" })
        foreach (var stage in plan)
        foreach (var resetFails in new[] { false, true })
        {
            var native = new Native { FailureStage = stage };
            if (resetFails) native.Errors["frame:none"] = 6;
            var owner = new WindowMaterialOwner(native: native); owner.UpdateCapabilities(Enabled);
            var result = owner.Apply(Command(owner.BindPeer(42)) with { Surface = surface }, Guid.NewGuid(), default);
            var expected = plan.Take(Array.IndexOf(plan, stage) + 1).Concat(Reset)
                .Concat(resetFails || stage == "corners" ? [] : new[] { "corners" });
            Check(native.Calls.SequenceEqual(expected), $"{stage} failure resets 38=1, Accent0 and frame before rounding");
            if (resetFails) Check(!result.Success && result.Data is null && result.Error == "material_reset_failed"
                && result.NativeErrorCode == 6, "failed reset never authorizes opaque receipt");
            else Check(Field(result, "effective") == "acrylic_opaque" && Field(result, "fallback_reason") == "native_apply_failed"
                && result.NativeErrorCode == 5 && Receipt(result).GetProperty("native_error_code").GetInt32() == 5, "confirmed reset fallback preserves original apply code");
        }
        var win10 = new Native { SupportsSystemBackdrop = false, SupportsRoundedCorners = false, FailureStage = "accent:5" };
        var win10Owner = new WindowMaterialOwner(native: win10); win10Owner.UpdateCapabilities(Enabled);
        var win10Result = win10Owner.Apply(Command(win10Owner.BindPeer(42)), Guid.NewGuid(), default);
        Check(Field(win10Result, "effective") == "acrylic_opaque" && Field(win10Result, "fallback_reason") == "native_apply_failed"
            && win10.Calls.SequenceEqual(new[] { "dark:True", "accent:5", "accent:0", "frame:none" }), "Windows 10 host backdrop failure resets Accent without DWM38/33/34");
        foreach (var surface in Surfaces)
        {
            var native = new Native(); native.Errors["accent:0"] = 6;
            var owner = new WindowMaterialOwner(native: native); owner.UpdateCapabilities(Enabled);
            var result = owner.Apply(Command(owner.BindPeer(42)) with { Style = "solid", Surface = surface }, Guid.NewGuid(), default);
            Check(!result.Success && result.Data is null && result.Error == "material_reset_failed" && result.NativeErrorCode == 6,
                "solid/transparent also require confirmed reset");
            foreach (var failure in new[] { "dark:True", "backdrop:1" })
            {
                var failingNative = new Native { FailureStage = failure };
                var failingOwner = new WindowMaterialOwner(native: failingNative); failingOwner.UpdateCapabilities(Enabled);
                var failed = failingOwner.Apply(Command(failingOwner.BindPeer(42)) with { Style = "solid", Surface = surface }, Guid.NewGuid(), default);
                Check(!failed.Success && failed.Data is null && failed.Error == "material_apply_failed" && failed.NativeErrorCode == 5,
                    "Solid/transparent failure cannot produce an Acrylic fallback receipt");
                Check(!failingNative.Calls.Contains("frame:full") && (surface != "transparent" || !failingNative.Calls.Any(c => c.StartsWith("frame") || c == "corners")),
                    "transparent reset never extends the frame or rounds");
            }
        }
        var failedBackdrop = new Native { FailureStage = "backdrop:3" }; failedBackdrop.Errors["backdrop:1"] = 7;
        var backdropOwner = new WindowMaterialOwner(native: failedBackdrop); backdropOwner.UpdateCapabilities(Enabled);
        var backdropResult = backdropOwner.Apply(Command(backdropOwner.BindPeer(42)), Guid.NewGuid(), default);
        Check(!backdropResult.Success && backdropResult.Data is null && backdropResult.NativeErrorCode == 7
            && failedBackdrop.Calls.TakeLast(3).SequenceEqual(Reset), "DWM38 reset failure cannot be masked by successful Accent0");
        var throwing = new Native { AfterDark = () => throw new ExternalException("private-native-payload", 123) };
        var safeLog = new Log(); var exceptionOwner = new WindowMaterialOwner(safeLog, throwing); exceptionOwner.UpdateCapabilities(Enabled);
        var exceptionResult = exceptionOwner.Apply(Command(exceptionOwner.BindPeer(42)), Guid.NewGuid(), default);
        Check(exceptionResult.Success && exceptionResult.NativeErrorCode == 123 && Field(exceptionResult, "fallback_reason") == "native_apply_failed"
            && !JsonSerializer.Serialize(safeLog.Events.Select(e => e.Fields)).Contains("private-native-payload", StringComparison.Ordinal),
            "native exceptions retain only scalar error and confirmed-reset receipt, never exception payload");
        var terminal = safeLog.Events.Single(e => e.Name == "window.material.terminal").Fields;
        Check(Equals(terminal["backdrop_type"], 1) && Equals(terminal["accent_state"], 0) && Equals(terminal["corners"], "round"),
            "fallback diagnostics report the confirmed reset state");
        return Task.CompletedTask;
    }

    internal static async Task SerializedWritesAndMidFlightRetirement()
    {
        foreach (var retirement in new[] { "cancel", "facts", "peer", "window" })
        {
            var native = new Native(); var owner = new WindowMaterialOwner(native: native); owner.UpdateCapabilities(Enabled);
            var command = Command(owner.BindPeer(42)); using var cancel = new CancellationTokenSource();
            native.AfterDark = () =>
            {
                if (retirement == "cancel") cancel.Cancel();
                if (retirement == "facts") owner.UpdateCapabilities(Enabled with { EnergySaver = true });
                if (retirement == "peer") owner.BindPeer(0);
                if (retirement == "window") native.ValidationError = 1400;
            };
            var result = owner.Apply(command, Guid.NewGuid(), cancel.Token);
            Check(!result.Success && native.Calls.SequenceEqual(new[] { "dark:True" }), "retirement stops every subsequent write, including reset");
        }
        var n = new Native(); var o = new WindowMaterialOwner(native: n); o.UpdateCapabilities(Enabled); var c = Command(o.BindPeer(42));
        using var entered = new ManualResetEventSlim(); using var release = new ManualResetEventSlim();
        n.AfterDark = () => { entered.Set(); Check(release.Wait(TimeSpan.FromSeconds(5)), "test release deadline"); };
        var first = Task.Run(() => o.Apply(c, Guid.NewGuid(), default));
        Check(entered.Wait(TimeSpan.FromSeconds(5)), "native entered");
        var second = Task.Run(() => o.Apply(c with { Generation = 2 }, Guid.NewGuid(), default));
        try { await Task.Delay(40); Check(n.Calls.Count == 1, "second caller cannot interleave native writes"); }
        finally { release.Set(); }
        Check((await first).Success && (await second).Success, "both serialized generations complete");
        Check(n.Calls.SequenceEqual(Win11Frosted(3).Concat(Win11Frosted(3))), "whole native sequences serialized");
        foreach (var retirement in new[] { "facts", "peer", "cancel" })
        {
            var blockedNative = new Native(); var owner = new WindowMaterialOwner(native: blockedNative);
            owner.UpdateCapabilities(Enabled); var command = Command(owner.BindPeer(42));
            using var blocked = new ManualResetEventSlim(); using var unblock = new ManualResetEventSlim();
            using var cancel = new CancellationTokenSource();
            blockedNative.AfterDark = () => { blocked.Set(); Check(unblock.Wait(TimeSpan.FromSeconds(5)), "native release deadline"); };
            var applying = Task.Run(() => owner.Apply(command, Guid.NewGuid(), cancel.Token));
            Check(blocked.Wait(TimeSpan.FromSeconds(5)), "native write blocked");
            try
            {
                await Task.Run(() =>
                {
                    if (retirement == "facts") owner.UpdateCapabilities(Enabled with { EnergySaver = true });
                    if (retirement == "peer") owner.BindPeer(0);
                    if (retirement == "cancel") cancel.Cancel();
                }).WaitAsync(TimeSpan.FromSeconds(2));
                Check(!applying.IsCompleted, "retirement does not wait for uncooperative native call");
            }
            finally { unblock.Set(); }
            Check(!(await applying).Success && blockedNative.Calls.SequenceEqual(new[] { "dark:True" }),
                "cross-thread retirement prevents late writes and success receipt");
        }
    }

    internal static Task BoundedWindowHistoryAndDiagnostics()
    {
        var native = new Native(); var log = new Log(); var owner = new WindowMaterialOwner(log, native);
        owner.UpdateCapabilities(Enabled); var c = Command(owner.BindPeer(42)) with { Style = "acrylic" }; var correlation = Guid.NewGuid();
        Check(owner.Apply(c, correlation, default).Success, "first apply");
        var lifecycle = log.Events.Where(e => e.Name.StartsWith("window.material.")).ToArray();
        Check(lifecycle.Select(e => e.Name).SequenceEqual(new[]
            { "window.material.accepted", "window.material.start", "window.material.terminal" }), "bounded lifecycle diagnostic stages");
        Check(lifecycle.All(e => e.Correlation == correlation
            && e.Fields.ContainsKey("elapsed_ms") && e.Fields.ContainsKey("native_error_code")), "correlation elapsed and native code carried");
        Check(lifecycle[0].Fields["backdrop_type"] is null && lifecycle[0].Fields["accent_state"] is null
            && Equals(lifecycle[0].Fields["corners"], "none"), "accepted stage carries no native values yet");
        var terminal = lifecycle[2].Fields;
        Check(Equals(terminal["style"], "acrylic") && Equals(terminal["surface"], "backdrop") && Equals(terminal["effective"], "acrylic")
            && Equals(terminal["backdrop_type"], 3) && Equals(terminal["accent_state"], 0) && Equals(terminal["corners"], "round"),
            "terminal diagnostics carry scalar backdrop, accent and corner state");
        log.Throw = true;
        for (var i = 1; i <= 128; i++)
        {
            var window = 1000 + i;
            Check(owner.Apply(c with { WindowHandle = window }, Guid.NewGuid(), default).Success,
                "successive window lifetimes remain supported despite throwing diagnostics");
            owner.RetireWindow(window);
        }
        Check(owner.TrackedWindowCount == 1, "destroyed windows retire their native generation history");
        var count = native.Calls.Count;
        Check(!owner.Apply(c, Guid.NewGuid(), default).Success && native.Calls.Count == count,
            "retirement of other windows cannot forget a live window generation");
        native.AfterDark = () => owner.RetireWindow(c.WindowHandle);
        Check(!owner.Apply(c with { Generation = 2 }, Guid.NewGuid(), default).Success && owner.TrackedWindowCount == 0,
            "destroying the current HWND invalidates the remaining writes and receipt");
        return Task.CompletedTask;
    }

    internal static Task NativePolicyAbi()
    {
        var host = WindowMaterialNative.Policy(5); var reset = WindowMaterialNative.Policy(0);
        Check(host.AccentState == 5 && host.AccentFlags == 0 && host.GradientColor == 0 && host.AnimationId == 0, "host backdrop exact policy");
        Check(reset.AccentState == 0 && reset.AccentFlags == 2 && reset.GradientColor == 0 && reset.AnimationId == 0, "disabled Accent policy");
        foreach (var state in new[] { 3, 4 })
        {
            try { _ = WindowMaterialNative.Policy(state); throw new Exception($"Accent{state} must not be reachable"); }
            catch (ArgumentOutOfRangeException) { }
        }
        var full = WindowMaterialNative.FrameMargins(true); var none = WindowMaterialNative.FrameMargins(false);
        Check(full.Left == 0 && full.Right == 0 && full.Top == 0 && full.Bottom == 32767, "frame extends from the bottom only");
        Check(none.Left == 0 && none.Right == 0 && none.Top == 0 && none.Bottom == 0, "frame reset is zero margins");
        Check(Marshal.SizeOf<WindowMaterialNative.Margins>() == 16, "native MARGINS ABI");
        Check(Marshal.SizeOf<WindowMaterialNative.AccentPolicy>() == 16
            && Marshal.SizeOf<WindowMaterialNative.CompositionData>() == (IntPtr.Size == 8 ? 24 : 12)
            && Marshal.OffsetOf<WindowMaterialNative.CompositionData>(nameof(WindowMaterialNative.CompositionData.Data)).ToInt32() == IntPtr.Size
            && Marshal.OffsetOf<WindowMaterialNative.CompositionData>(nameof(WindowMaterialNative.CompositionData.Size)).ToInt32() == IntPtr.Size * 2, "native WCA19 ABI");
        Check(!WindowMaterialNative.HasSystemBackdrop(10, 19045) && !WindowMaterialNative.HasSystemBackdrop(10, 22000)
            && WindowMaterialNative.HasSystemBackdrop(10, 22621), "DWM38 exact Windows11 build floor");
        Check(!WindowMaterialNative.HasRoundedCorners(10, 19045) && !WindowMaterialNative.HasRoundedCorners(10, 21999)
            && WindowMaterialNative.HasRoundedCorners(10, 22000), "DWM33/34 exact Windows11 build floor");
        Check(WindowMaterialNative.IdentityError(-10, -10, false, 42, 42, 1, 1) == 0, "same-session root eligible");
        foreach (var code in new[]
        {
            WindowMaterialNative.IdentityError(-10, -11, false, 42, 42, 1, 1),
            WindowMaterialNative.IdentityError(-10, -10, true, 42, 42, 1, 1),
            WindowMaterialNative.IdentityError(-10, -10, false, 42, 43, 1, 1),
            WindowMaterialNative.IdentityError(-10, -10, false, 42, 42, 2, 1),
            WindowMaterialNative.IdentityError(-10, -10, false, 42, 42, 0, 0)
        })
        {
            Check(code != 0, "real identity policy rejects child, nonroot, foreign PID/session and session zero");
            var native = new Native { ValidationError = code }; var owner = new WindowMaterialOwner(native: native);
            owner.UpdateCapabilities(Enabled);
            Check(!owner.Apply(Command(owner.BindPeer(42)), Guid.NewGuid(), default).Success && native.Calls.Count == 0,
                "identity rejection reaches owner before any write");
        }
        var previousError = Marshal.GetLastPInvokeError();
        try
        {
            Marshal.SetLastPInvokeError(0); Check(WindowMaterialNative.LastError() != 0, "failed BOOL cannot masquerade as success with zero LastError");
            Marshal.SetLastPInvokeError(5); Check(WindowMaterialNative.LastError() == 5, "BOOL native error retained");
        }
        finally { Marshal.SetLastPInvokeError(previousError); }
        return Task.CompletedTask;
    }

    // Creates and mutates only hidden, self-owned STATIC HWNDs. No global settings or input changes.
    internal static Task OwnedHiddenNativeWindow()
    {
        var window = CreateWindowExW(0x08000000, "STATIC", "", 0x80000000, 0, 0, 120, 80, 0, 0, 0, 0);
        Check(window != 0, "owned native window created");
        nint child = 0, composition = 0;
        try
        {
            var native = new WindowMaterialNative(); var pid = (uint)Environment.ProcessId;
            Check(native.ValidateWindow(window.ToInt64(), pid) == 0, "actual owned hidden root accepted");
            Check(!native.SupportsAlphaComposition(window.ToInt64()), "redirected HWND has no composition alpha");
            composition = CreateWindowExW(0x08200000, "STATIC", "", 0x80000000, 0, 0, 120, 80, 0, 0, 0, 0);
            Check(composition != 0 && native.SupportsAlphaComposition(composition.ToInt64()), "owned no-redirection HWND selects composition alpha");
            Check(native.ValidateWindow(window.ToInt64(), pid + 1) != 0, "actual foreign PID rejected");
            child = CreateWindowExW(0, "STATIC", "", 0x40000000, 0, 0, 20, 20, window, 0, 0, 0);
            Check(child != 0 && native.ValidateWindow(child.ToInt64(), pid) != 0, "actual WS_CHILD rejected");
            var facts = new WindowsSystemMaterialReader().Read();
            if (!facts.Supported) throw new TestSkippedException("owned material fixture requires Windows10 1809+ with DWM composition");
            var owner = new WindowMaterialOwner(native: native); owner.UpdateCapabilities(facts);
            var command = Command(owner.BindPeer(pid)) with { WindowHandle = window.ToInt64(), Style = "solid" };
            Check(GetWindowRect(window, out var before), "owned initial bounds");
            var style = GetWindowLongPtrW(window, -16); var extended = GetWindowLongPtrW(window, -20);
            var foreground = GetForegroundWindow();
            Check(Field(owner.Apply(command, Guid.NewGuid(), default), "effective") == "solid", "actual owned solid acceptance");
            var acrylic = owner.Apply(command with { Style = "acrylic", Generation = 2 }, Guid.NewGuid(), default);
            Check(acrylic.Success && Field(acrylic, "effective") == "acrylic_opaque", "redirected owned window confirms opaque fallback");
            Check(Field(owner.Apply(command with { Surface = "transparent", Generation = 3 }, Guid.NewGuid(), default), "effective") == "transparent", "owned transparent host reset");
            Check(GetWindowRect(window, out var after) && before.Equals(after) && GetWindowLongPtrW(window, -16) == style
                && GetWindowLongPtrW(window, -20) == extended && GetForegroundWindow() == foreground && !IsWindowVisible(window),
                "material preserves bounds, styles, foreground and hidden state");
            var eligible = facts.NativeErrorCode == 0 && facts.TransparencyEnabled && !facts.HighContrast
                && !facts.EnergySaver && !facts.RemoteSession;
            foreach (var (frostedStyle, surface, generation) in new[] { ("acrylic", "backdrop", 1UL), ("acrylic", "transient", 2UL), ("solid", "backdrop", 3UL) })
            {
                var applied = owner.Apply(command with { WindowHandle = composition.ToInt64(), Style = frostedStyle, Surface = surface, Generation = generation },
                    Guid.NewGuid(), default);
                var expected = frostedStyle == "solid" ? "solid" : eligible ? "acrylic" : "acrylic_opaque";
                Check(applied.Success && Field(applied, "effective") == expected, $"actual owned {frostedStyle}/{surface} native sequence accepted");
            }
            Check(!IsWindowVisible(composition) && GetForegroundWindow() == foreground, "composition fixture stays hidden and passive");
        }
        finally { if (child != 0) DestroyWindow(child); if (composition != 0) DestroyWindow(composition); DestroyWindow(window); }
        return Task.CompletedTask;
    }

    private sealed class Reader : ISystemMaterialReader
    {
        internal MaterialCapabilitySnapshot Value; internal bool Throw;
        public MaterialCapabilitySnapshot Read() => Throw ? throw new InvalidOperationException() : Value;
    }
    // Calls record the exact native write; FailureStage fails its first occurrence, Errors fail every occurrence.
    private sealed class Native : IWindowMaterialNative
    {
        internal bool ThrowVersionRead;
        private bool _systemBackdrop = true, _roundedCorners = true;
        public bool SupportsSystemBackdrop
        {
            get => ThrowVersionRead ? throw new ExternalException("version", 87) : _systemBackdrop;
            set => _systemBackdrop = value;
        }
        public bool SupportsRoundedCorners
        {
            get => ThrowVersionRead ? throw new ExternalException("version", 87) : _roundedCorners;
            set => _roundedCorners = value;
        }
        internal bool AlphaComposition = true;
        internal bool ThrowAlphaRead;
        public bool SupportsAlphaComposition(long window) => ThrowAlphaRead
            ? throw new System.ComponentModel.Win32Exception(1400) : AlphaComposition;
        internal readonly List<string> Calls = [];
        internal readonly Dictionary<string, int> Errors = [];
        internal int ValidationError;
        internal string? FailureStage;
        private bool _stageFailed;
        internal Action? AfterDark;
        public int ValidateWindow(long window, uint processId) => ValidationError;
        private int Result(string call)
        {
            if (!_stageFailed && FailureStage == call) { _stageFailed = true; return 5; }
            return Errors.GetValueOrDefault(call);
        }
        private int Record(string call) { Calls.Add(call); return Result(call); }
        public int SetDark(long window, bool dark) { Calls.Add($"dark:{dark}"); AfterDark?.Invoke(); return Result($"dark:{dark}"); }
        public int SetBackdropType(long window, int type) => Record($"backdrop:{type}");
        public int ExtendFrame(long window, bool fullClient) => Record(fullClient ? "frame:full" : "frame:none");
        public int SetRoundedCorners(long window) => Record("corners");
        public int SetAccent(long window, int state) => Record($"accent:{state}");
        public int SetHostBackdropBrush(long window, bool enabled) => Record(enabled ? "host:1" : "host:0");
    }
    private sealed class Log : IPlatformLog
    {
        internal bool Throw;
        internal readonly List<(string Name, Guid? Correlation, IReadOnlyDictionary<string, object?> Fields)> Events = [];
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (Throw) throw new InvalidOperationException();
            Events.Add((eventName, correlationId, fields!));
        }
    }
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern nint CreateWindowExW(uint exStyle, string className, string title, uint style,
        int x, int y, int width, int height, nint parent, nint menu, nint instance, nint parameter);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool DestroyWindow(nint window);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool IsWindowVisible(nint window);
    [StructLayout(LayoutKind.Sequential)] private struct Rect { internal int Left, Top, Right, Bottom; }
    [DllImport("user32.dll", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool GetWindowRect(nint window, out Rect rect);
    [DllImport("user32.dll", ExactSpelling = true)] private static extern nint GetWindowLongPtrW(nint window, int index);
    [DllImport("user32.dll", ExactSpelling = true)] private static extern nint GetForegroundWindow();
}
