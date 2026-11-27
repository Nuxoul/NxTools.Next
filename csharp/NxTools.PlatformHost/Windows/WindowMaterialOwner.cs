using System.Diagnostics;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

// The caller owns bounded STA admission. This owner never creates threads or changes presentation.
internal sealed class WindowMaterialOwner(IPlatformLog? log = null, IWindowMaterialNative? native = null)
{
    private readonly IWindowMaterialNative _native = native ?? new WindowMaterialNative();
    private readonly object _stateGate = new();
    private readonly object _writeGate = new();
    private readonly Dictionary<long, ulong> _generations = [];
    private Guid _session;
    private uint _peer;
    private ulong _revision = 1;
    private bool _hasFacts;
    private MaterialCapabilitySnapshot _facts;

    public Guid BindPeer(uint processId)
    {
        lock (_stateGate)
        {
            _peer = processId;
            _session = processId == 0 ? Guid.Empty : Guid.NewGuid();
            _revision = 1;
            _generations.Clear();
            return _session;
        }
    }

    internal void RetireWindow(long windowHandle)
    {
        lock (_stateGate) _generations.Remove(windowHandle);
    }
    internal int TrackedWindowCount { get { lock (_stateGate) return _generations.Count; } }

    public SystemMaterialCapabilitiesChangedEvent? UpdateCapabilities(MaterialCapabilitySnapshot snapshot)
    {
        SystemMaterialCapabilitiesChangedEvent? result;
        lock (_stateGate)
        {
            if (_hasFacts && _facts == snapshot) return null;
            if (_hasFacts && _session != Guid.Empty) _revision = checked(_revision + 1);
            _facts = snapshot;
            _hasFacts = true;
            result = SnapshotLocked();
        }
        DiagnosticFacts(snapshot, result);
        return result;
    }

    public SystemMaterialCapabilitiesChangedEvent? SnapshotEvent()
    {
        lock (_stateGate) return SnapshotLocked();
    }

    private SystemMaterialCapabilitiesChangedEvent? SnapshotLocked() => !_hasFacts || _session == Guid.Empty ? null :
        new(Guid.NewGuid(), _session, _revision, _facts.Supported, _facts.TransparencyEnabled,
            _facts.HighContrast, _facts.EnergySaver, _facts.RemoteSession, _facts.NativeErrorCode,
            _facts.SystemBackdrop, _facts.RoundedCorners);

    public WindowsCommandResult Apply(ApplyWindowMaterialCommand command, Guid correlation, CancellationToken cancellation)
    {
        var elapsed = Stopwatch.StartNew();
        var trace = new MaterialTrace();
        Diagnostic("accepted", command, correlation, elapsed, null, "none", "none", trace);
        WindowsCommandResult result;
        var effective = "none";
        var fallback = "none";
        lock (_writeGate)
        {
            Diagnostic("start", command, correlation, elapsed, null, effective, fallback, trace);
            result = ApplyLocked(command, correlation, cancellation, trace, out effective, out fallback);
        }
        Diagnostic("terminal", command, correlation, elapsed, result, effective, fallback, trace);
        return result;
    }

    private WindowsCommandResult ApplyLocked(ApplyWindowMaterialCommand command, Guid correlation,
        CancellationToken cancellation, MaterialTrace trace, out string effective, out string fallback)
    {
        effective = "none";
        fallback = "none";
        if (correlation == Guid.Empty || !WindowMaterialContract.IsValid(command)) return Failure("invalid_window_material_request");
        var invalid = CheckCurrent(command, cancellation, false);
        if (invalid is not null) return invalid;
        var validation = Invoke(() => _native.ValidateWindow(command.WindowHandle, PeerFor(command)));
        if (validation != 0) return Failure("invalid_material_window", validation);
        MaterialCapabilitySnapshot facts;
        lock (_stateGate)
        {
            invalid = CheckStateLocked(command, cancellation, false);
            if (invalid is not null) return invalid;
            _generations[command.WindowHandle] = command.Generation;
            facts = _facts;
        }
        var framed = command.Surface != "transparent";
        var frosted = framed && WindowMaterialContract.IsFrosted(command.Style);
        fallback = frosted ? Fallback(facts) : "none";
        if (frosted && fallback == "none")
        {
            try
            {
                if (!_native.SupportsAlphaComposition(command.WindowHandle)) fallback = "unsupported";
            }
            catch (Exception error)
            {
                return Failure("material_composition_read_failed", error is System.ComponentModel.Win32Exception nativeError
                    ? nativeError.NativeErrorCode : ExceptionCode(error));
            }
        }
        effective = !framed ? "transparent" : !frosted ? "solid" : fallback == "none" ? "acrylic" : "acrylic_opaque";
        var receiptCode = frosted ? facts.NativeErrorCode : 0;
        bool backdrop, rounded;
        try
        {
            backdrop = _native.SupportsSystemBackdrop;
            rounded = _native.SupportsRoundedCorners;
        }
        catch (Exception error) { return Failure("material_version_failed", ExceptionCode(error)); }

        var plan = new List<NativeStep> { new(StepKind.Dark, command.Dark ? 1 : 0) };
        if (effective == "acrylic" && backdrop)
        {
            plan.Add(new(StepKind.Accent, WindowMaterialNative.AccentDisabled));
            plan.Add(new(StepKind.Frame, 1));
            plan.Add(new(StepKind.Backdrop, SystemAcrylic));
        }
        else if (effective == "acrylic")
        {
            plan.Add(new(StepKind.Accent, WindowMaterialNative.AccentHostBackdrop));
            plan.Add(new(StepKind.Frame, 0));
            if (rounded) plan.Add(new(StepKind.HostBrush, 1));
        }
        else plan.AddRange(ResetPlan(backdrop, rounded, framed));
        if (framed && rounded) plan.Add(new(StepKind.Corners, 1));

        var failed = default(NativeStep?);
        var applyCode = 0;
        foreach (var step in plan)
        {
            // Check session, revision, generation, cancellation and live HWND identity before each write.
            invalid = Write(command, cancellation, () => Run(step, command.WindowHandle), out var nativeCode);
            if (invalid is not null) return invalid;
            if (nativeCode != 0) { failed = step; applyCode = nativeCode; break; }
            trace.Record(step);
        }
        if (failed is { } failedStep)
        {
            fallback = "native_apply_failed";
            var resetCode = 0;
            foreach (var step in ResetPlan(backdrop, rounded, framed))
            {
                invalid = Write(command, cancellation, () => Run(step, command.WindowHandle), out var nativeCode);
                if (invalid is not null) return invalid;
                if (nativeCode == 0) trace.Record(step);
                else if (resetCode == 0) resetCode = nativeCode;
            }
            if (resetCode != 0) return Failure("material_reset_failed", resetCode);
            if (!frosted) return Failure("material_apply_failed", applyCode);
            if (rounded && failedStep.Kind != StepKind.Corners)
            {
                var corners = new NativeStep(StepKind.Corners, 1);
                invalid = Write(command, cancellation, () => Run(corners, command.WindowHandle), out var cornerCode);
                if (invalid is not null) return invalid;
                if (cornerCode != 0) return Failure("material_reset_failed", cornerCode);
                trace.Record(corners);
            }
            effective = "acrylic_opaque";
            receiptCode = applyCode;
        }
        invalid = CheckCurrent(command, cancellation, true);
        if (invalid is not null) return invalid;
        return new(true, Data: WindowMaterialContract.Receipt(command, effective, fallback, receiptCode), NativeErrorCode: receiptCode);
    }

    // DWMSBT_TRANSIENTWINDOW: the system acrylic for every framed frosted window.
    internal const int SystemAcrylic = 3;

    private static IEnumerable<NativeStep> ResetPlan(bool backdrop, bool rounded, bool framed)
    {
        if (backdrop) yield return new(StepKind.Backdrop, 1);
        yield return new(StepKind.Accent, WindowMaterialNative.AccentDisabled);
        if (framed) yield return new(StepKind.Frame, 0);
        // Windows 11 before 22621 frosts through the host backdrop brush.
        if (framed && rounded && !backdrop) yield return new(StepKind.HostBrush, 0);
    }

    private int Run(NativeStep step, long window) => step.Kind switch
    {
        StepKind.Dark => _native.SetDark(window, step.Value != 0),
        StepKind.Backdrop => _native.SetBackdropType(window, step.Value),
        StepKind.Accent => _native.SetAccent(window, step.Value),
        StepKind.Frame => _native.ExtendFrame(window, step.Value != 0),
        StepKind.Corners => _native.SetRoundedCorners(window),
        StepKind.HostBrush => _native.SetHostBackdropBrush(window, step.Value != 0),
        _ => throw new ArgumentOutOfRangeException(nameof(step))
    };

    private enum StepKind { Dark, Backdrop, Accent, Frame, Corners, HostBrush }
    private readonly record struct NativeStep(StepKind Kind, int Value);

    // Last accepted native values for the lifecycle diagnostics; scalars only.
    private sealed class MaterialTrace
    {
        internal int? BackdropType;
        internal int? AccentState;
        internal bool Round;
        internal void Record(NativeStep step)
        {
            if (step.Kind == StepKind.Backdrop) BackdropType = step.Value;
            if (step.Kind == StepKind.Accent) AccentState = step.Value;
            if (step.Kind == StepKind.Corners) Round = true;
        }
    }

    private uint PeerFor(ApplyWindowMaterialCommand command)
    {
        lock (_stateGate) return command.SessionId == _session ? _peer : 0;
    }

    private WindowsCommandResult? Write(ApplyWindowMaterialCommand command, CancellationToken cancellation, Func<int> write, out int nativeCode)
    {
        nativeCode = 0;
        var invalid = CheckCurrent(command, cancellation, true);
        if (invalid is not null) return invalid;
        nativeCode = Invoke(() => _native.ValidateWindow(command.WindowHandle, PeerFor(command)));
        if (nativeCode != 0) return Failure("invalid_material_window", nativeCode);
        invalid = CheckCurrent(command, cancellation, true);
        if (invalid is not null) return invalid;
        // Native calls are synchronous on the caller's bounded worker. An already entered call
        // cannot be interrupted; retirement remains nonblocking and prevents the next write/receipt.
        nativeCode = Invoke(write);
        return null;
    }

    private WindowsCommandResult? CheckCurrent(ApplyWindowMaterialCommand command, CancellationToken cancellation, bool reserved)
    {
        lock (_stateGate) return CheckStateLocked(command, cancellation, reserved);
    }

    private WindowsCommandResult? CheckStateLocked(ApplyWindowMaterialCommand command, CancellationToken cancellation, bool reserved)
    {
        if (cancellation.IsCancellationRequested) return Failure("material_cancelled", 1223);
        if (_session == Guid.Empty || command.SessionId != _session || _peer == 0) return Failure("stale_material_session");
        if (!_hasFacts) return Failure("material_facts_not_ready");
        if (command.FactsRevision != _revision) return Failure("stale_material_facts");
        if (_generations.TryGetValue(command.WindowHandle, out var current)
            ? reserved ? current != command.Generation : command.Generation <= current
            : reserved) return Failure("stale_material_generation");
        return null;
    }

    private static string Fallback(MaterialCapabilitySnapshot facts) =>
        facts.NativeErrorCode != 0 ? "capability_read_failed" : !facts.Supported ? "unsupported"
        : facts.HighContrast ? "high_contrast" : !facts.TransparencyEnabled ? "transparency_disabled"
        : facts.EnergySaver ? "energy_saver" : facts.RemoteSession ? "remote_session" : "none";

    private static WindowsCommandResult Failure(string error, int nativeCode = 0) => new(false, error, NativeErrorCode: nativeCode);
    private static int ExceptionCode(Exception error) => error.HResult == 0 ? -1 : error.HResult;
    private static int Invoke(Func<int> operation)
    {
        try { return operation(); }
        catch (Exception error) { return ExceptionCode(error); }
    }

    private void Diagnostic(string stage, ApplyWindowMaterialCommand command, Guid correlation,
        Stopwatch elapsed, WindowsCommandResult? result, string effective, string fallback, MaterialTrace trace)
    {
        try
        {
            log?.Write("info", "window.material." + stage, correlation, new Dictionary<string, object?>
            {
                ["session_id"] = command.SessionId, ["facts_revision"] = command.FactsRevision,
                ["window_handle"] = command.WindowHandle, ["generation"] = command.Generation,
                ["style"] = WindowMaterialContract.IsStyle(command.Style) ? command.Style : "invalid",
                ["surface"] = WindowMaterialContract.IsSurface(command.Surface) ? command.Surface : "invalid", ["dark"] = command.Dark,
                ["effective"] = result is { Success: false } ? "none" : effective, ["fallback_reason"] = fallback,
                ["backdrop_type"] = trace.BackdropType, ["accent_state"] = trace.AccentState,
                ["corners"] = trace.Round ? "round" : "none",
                ["success"] = result?.Success, ["error"] = result?.Error,
                ["native_error_code"] = result?.NativeErrorCode ?? 0, ["elapsed_ms"] = elapsed.ElapsedMilliseconds
            });
        }
        catch (Exception) { /* Diagnostics cannot affect native operation results. */ }
    }

    private void DiagnosticFacts(MaterialCapabilitySnapshot snapshot, SystemMaterialCapabilitiesChangedEvent? change)
    {
        try
        {
            log?.Write("info", "system.material.capabilities_changed", change?.CorrelationId, new Dictionary<string, object?>
            {
                ["session_id"] = change?.SessionId, ["revision"] = change?.Revision,
                ["supported"] = snapshot.Supported, ["transparency_enabled"] = snapshot.TransparencyEnabled,
                ["high_contrast"] = snapshot.HighContrast, ["energy_saver"] = snapshot.EnergySaver,
                ["remote_session"] = snapshot.RemoteSession, ["native_error_code"] = snapshot.NativeErrorCode,
                ["system_backdrop"] = snapshot.SystemBackdrop, ["rounded_corners"] = snapshot.RoundedCorners
            });
        }
        catch (Exception) { /* Diagnostics cannot affect fact publication. */ }
    }
}
