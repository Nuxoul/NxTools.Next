using System.ComponentModel;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record ProcessPickViewport(int Left, int Top, uint Width, uint Height);
internal sealed record ProcessPickWindow(long WindowHandle, uint ProcessId, int Left, int Top, uint Width, uint Height);
internal sealed record ProcessPickCandidate(long WindowHandle, uint ProcessId, string ProcessName, int Left, int Top, uint Width, uint Height);
internal sealed record ProcessWindowPickChangedEvent(Guid SessionId, string State,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ProcessPickViewport? Viewport = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ProcessPickCandidate? Candidate = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? NativeErrorCode = null) : WindowsPlatformEvent;
internal enum ProcessPickInput { LeftDown, LeftUp, EscapeDown, EscapeUp }

internal interface IProcessWindowPickNative
{
    long ElapsedMilliseconds { get; }
    bool ValidateOwners(long source, long overlay, uint peer);
    ProcessPickViewport GetViewport();
    void PrepareOverlay(long overlay, ProcessPickViewport viewport);
    void ShowOverlay(long overlay);
    void HideOverlay(long overlay);
    void InstallHooks(Func<ProcessPickInput, long, bool> input, long source, long overlay);
    void HideSourceWindows(long source, long overlay, uint peer);
    void RestoreSourceWindows();
    void RemoveHooks();
    bool IsPressed(ProcessPickInput input);
    ProcessPickWindow? HitTest(long source, long overlay);
    ProcessPickWindow? ReadWindow(long handle, long source, long overlay);
    string? ResolveProcess(uint pid, out int error);
}

// All methods, including Input, run on the same dedicated message-loop owner.
// Input is the hook boundary: only fixed-size state changes, never publication/process IO/logging.
internal sealed class ProcessWindowPickController(
    IProcessWindowPickNative native, Func<ProcessWindowPickChangedEvent, bool> publish, IPlatformLog log)
{
    private ProcessWindowPickCommand? _session;
    private CancellationToken _cancellation;
    private uint _peer;
    private long _started, _terminalAt;
    private bool _terminal, _stopped, _initialLeft, _leftUp, _escapeUp, _escapePending, _clickPending;
    private bool _leftObservedPressed, _escapeObservedPressed;
    private long _clicked;
    private long _mouseDowns, _mouseUps, _escapeDowns, _escapeUps, _mouseSwallowed, _escapeSwallowed;
    private ProcessPickWindow? _lastHit;
    private ProcessPickViewport? _viewport;
    private bool _hoverSent;
    internal bool Occupied => _session is not null;

    internal WindowsCommandResult Execute(ProcessWindowPickCommand command, uint peer, CancellationToken cancellation)
    {
        if (command.Action == "cancel")
        {
            if (_session?.SessionId == command.SessionId) Finish("cancelled", "cancel");
            return new(true);
        }
        if (_stopped || cancellation.IsCancellationRequested) return new(false, "window_pick_cancelled");
        if (command.Action == "show_overlay")
        {
            if (_session?.SessionId != command.SessionId || _terminal || _cancellation.IsCancellationRequested ||
                _clickPending || _escapePending) return new(false, "window_pick_retired");
            if (native.ElapsedMilliseconds - _started >= 120_000)
            { Finish("cancelled", "deadline"); return new(false, "window_pick_retired"); }
            try
            {
                if (!native.ValidateOwners(_session.SourceWindowHandle, _session.OverlayWindowHandle, _peer))
                { Finish("cancelled", "owner_gone"); return new(false, "window_pick_owner_gone"); }
                native.ShowOverlay(_session.OverlayWindowHandle);
                return new(true);
            }
            catch (Exception error) { var code = Code(error); Finish("failed", "show_failed", code); return new(false, "window_pick_show_failed", NativeErrorCode: code); }
        }
        if (command.Action != "start" || command.SessionId == Guid.Empty || command.SourceWindowHandle == 0 ||
            command.OverlayWindowHandle == 0 || command.SourceWindowHandle == command.OverlayWindowHandle)
            return new(false, "invalid_process_window_pick_request");
        if (Occupied) return new(false, "window_pick_busy");
        _session = command; _peer = peer; _cancellation = cancellation; _started = native.ElapsedMilliseconds;
        _terminal = _hoverSent = _leftUp = _escapeUp = _escapePending = _clickPending = false; _clicked = 0; _lastHit = null;
        _leftObservedPressed = _escapeObservedPressed = false;
        _mouseDowns = _mouseUps = _escapeDowns = _escapeUps = _mouseSwallowed = _escapeSwallowed = 0;
        Diagnostic("accepted", "accepted", "start", null);
        var prepared = false;
        try
        {
            if (!native.ValidateOwners(command.SourceWindowHandle, command.OverlayWindowHandle, peer))
                throw new Win32Exception(1400);
            _viewport = native.GetViewport();
            prepared = true;
            native.PrepareOverlay(command.OverlayWindowHandle, _viewport);
            _initialLeft = native.IsPressed(ProcessPickInput.LeftDown);
            native.InstallHooks(Input, command.SourceWindowHandle, command.OverlayWindowHandle);
            native.HideSourceWindows(command.SourceWindowHandle, command.OverlayWindowHandle, peer);
            Diagnostic("started", "active", "start", null);
            return new(true);
        }
        catch (Exception error)
        {
            var code = Code(error);
            var restoreCode = TryRestoreSource();
            if (restoreCode is not null) code = restoreCode.Value;
            Diagnostic("failed", "failed", restoreCode is null ? "startup_failed" : "restore_failed", code);
            // Installation can fail after the first hook succeeds.
            native.RemoveHooks();
            if (prepared) TryHide();
            _session = null;
            return new(false, "window_pick_start_failed", NativeErrorCode: code);
        }
    }

    internal bool Input(ProcessPickInput input, long root)
    {
        var session = _session;
        if (session is null) return false;
        switch (input)
        {
            case ProcessPickInput.LeftDown: _mouseDowns++; break;
            case ProcessPickInput.LeftUp: _mouseUps++; break;
            case ProcessPickInput.EscapeDown: _escapeDowns++; break;
            case ProcessPickInput.EscapeUp: _escapeUps++; break;
        }
        var swallowed = DecideInput(session, input, root);
        if (swallowed)
        {
            if (input is ProcessPickInput.LeftDown or ProcessPickInput.LeftUp) _mouseSwallowed++;
            else _escapeSwallowed++;
        }
        return swallowed;
    }

    private bool DecideInput(ProcessWindowPickCommand session, ProcessPickInput input, long root)
    {
        if (input == ProcessPickInput.LeftUp)
        {
            _initialLeft = false;
            if (!_leftUp) return false;
            _leftUp = _leftObservedPressed = false; return true;
        }
        if (input == ProcessPickInput.EscapeUp)
        {
            if (!_escapeUp) return false;
            _escapeUp = _escapeObservedPressed = false; return true;
        }
        if (input == ProcessPickInput.EscapeDown)
        {
            if (_terminal && !_escapeUp) return false;
            if (!_escapeUp) _escapeObservedPressed = false;
            _escapeUp = true; _escapePending = true; return true;
        }
        if (_terminal) return _leftUp;
        if (_initialLeft || root == session.SourceWindowHandle) return false;
        if (!_leftUp) _leftObservedPressed = false;
        _leftUp = true;
        if (!_clickPending && !_escapePending)
        {
            _clicked = root == session.OverlayWindowHandle ? 0 : root;
            _clickPending = true;
        }
        return true;
    }

    internal void Tick()
    {
        if (_session is null) return;
        try
        {
            var age = native.ElapsedMilliseconds - _started;
            if (!_terminal)
            {
                if (_initialLeft && !native.IsPressed(ProcessPickInput.LeftDown)) _initialLeft = false;
                if (_stopped || _cancellation.IsCancellationRequested) Finish("cancelled", _stopped ? "disposed" : "disconnected");
                else if (age >= 120_000) Finish("cancelled", "deadline");
                else if (!native.ValidateOwners(_session.SourceWindowHandle, _session.OverlayWindowHandle, _peer)) Finish("cancelled", "owner_gone");
                else if (_escapePending) Finish("cancelled", "escape");
                else if (_clickPending)
                {
                    var hit = native.ReadWindow(_clicked, _session.SourceWindowHandle, _session.OverlayWindowHandle);
                    var candidate = Candidate(hit, out var code);
                    Finish(candidate is null ? "failed" : "selected", candidate is null ? "target_unavailable" : "selected", code, candidate);
                }
                else Hover();
            }
            if (_session is not null && _terminal)
            {
                // Suppressed downs need not enter async key state. A false sample alone
                // cannot retire their release owner; fallback requires an observed down->up.
                var releaseRecoveryDue = native.ElapsedMilliseconds - _terminalAt >= 2000;
                if (_leftUp)
                {
                    var pressed = native.IsPressed(ProcessPickInput.LeftDown);
                    _leftObservedPressed |= pressed;
                    if (releaseRecoveryDue && _leftObservedPressed && !pressed) _leftUp = false;
                }
                if (_escapeUp)
                {
                    var pressed = native.IsPressed(ProcessPickInput.EscapeDown);
                    _escapeObservedPressed |= pressed;
                    if (releaseRecoveryDue && _escapeObservedPressed && !pressed) _escapeUp = false;
                }
                if (!_leftUp && !_escapeUp) Cleanup();
            }
        }
        catch (Exception error) { Finish("failed", "native_failed", Code(error)); }
    }

    private void Hover()
    {
        var session = _session!;
        var viewport = native.GetViewport();
        if (viewport != _viewport) { Finish("cancelled", "display_changed"); return; }
        var hit = native.HitTest(session.SourceWindowHandle, session.OverlayWindowHandle);
        if (_hoverSent && hit == _lastHit) return;
        _lastHit = hit; _hoverSent = true;
        var candidate = Candidate(hit, out var code);
        if (!publish(new(session.SessionId, "hover", viewport, candidate, code))) Finish("failed", "event_queue_full");
    }

    private ProcessPickCandidate? Candidate(ProcessPickWindow? hit, out int? code)
    {
        code = null;
        if (hit is null) return null;
        var name = native.ResolveProcess(hit.ProcessId, out var nativeCode);
        if (string.IsNullOrWhiteSpace(name)) { code = nativeCode == 0 ? 1168 : nativeCode; return null; }
        if (!name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) name += ".exe";
        return new(hit.WindowHandle, hit.ProcessId, name, hit.Left, hit.Top, hit.Width, hit.Height);
    }

    internal void Stop() { _stopped = true; if (_session is not null) Finish("cancelled", "disposed"); }

    private void Finish(string state, string reason, int? code = null, ProcessPickCandidate? candidate = null)
    {
        if (_session is null || _terminal) return;
        _terminal = true; _terminalAt = native.ElapsedMilliseconds;
        var hideCode = TryHide();
        var restoreCode = TryRestoreSource();
        if (hideCode is not null) { state = "failed"; reason = "hide_failed"; code = hideCode; candidate = null; }
        if (restoreCode is not null) { state = "failed"; reason = "restore_failed"; code = restoreCode; candidate = null; }
        Diagnostic(state == "failed" ? "failed" : "completed", state, reason, code);
        publish(new(_session.SessionId, state, Candidate: candidate, NativeErrorCode: code));
        if (!_leftUp && !_escapeUp) Cleanup();
    }

    private int? TryHide()
    {
        try { native.HideOverlay(_session!.OverlayWindowHandle); return null; }
        catch (Exception error) { return Code(error); }
    }

    private int? TryRestoreSource()
    {
        try { native.RestoreSourceWindows(); return null; }
        catch (Exception error) { return Code(error); }
    }

    private void Cleanup() { native.RemoveHooks(); _session = null; }
    private static int Code(Exception error) => error is Win32Exception win32 ? win32.NativeErrorCode : error.HResult;
    private void Diagnostic(string phase, string state, string reason, int? code)
    {
        try { log.Write(phase == "failed" ? "warning" : "info", "blacklist.window_pick." + phase, _session!.SessionId,
            new Dictionary<string, object?> { ["state"] = state, ["reason"] = reason,
                ["elapsed_ms"] = native.ElapsedMilliseconds - _started, ["native_error_code"] = code,
                ["mouse_down_received"] = _mouseDowns, ["mouse_up_received"] = _mouseUps,
                ["escape_down_received"] = _escapeDowns, ["escape_up_received"] = _escapeUps,
                ["mouse_events_swallowed"] = _mouseSwallowed, ["escape_events_swallowed"] = _escapeSwallowed,
                ["pending_mouse_release"] = _leftUp, ["pending_escape_release"] = _escapeUp }); }
        catch { /* Diagnostics cannot change native input ownership. */ }
    }
}
