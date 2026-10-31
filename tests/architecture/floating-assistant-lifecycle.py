#!/usr/bin/env python3
"""Floating assistant source-boundary guard; not native, visual or release acceptance."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DESKTOP = "rust/apps/nxtools-desktop/src/"
NATIVE = "csharp/NxTools.PlatformHost/"
checks = 0


def read(path):
    return (ROOT / path).read_text(encoding="utf-8-sig")


def require(condition, message):
    global checks
    if not condition:
        raise AssertionError(message)
    checks += 1


def section(source, start, end):
    return source.split(start, 1)[1].split(end, 1)[0]


owner = read(DESKTOP + "tools/floating_assistant/application.rs")
actions = read(DESKTOP + "tools/floating_assistant/actions.rs")
view = read(DESKTOP + "tools/floating_assistant/view.rs")
router = read(DESKTOP + "tools/floating_assistant/mod.rs")
media = read(DESKTOP + "tools/floating_assistant/media.rs")
playback = read(DESKTOP + "tools/floating_assistant/playback.rs")
coordinator = read(NATIVE + "Windows/FloatingAssistantCoordinator.cs")
input_surface = read(NATIVE + "Windows/FloatingAssistantInputSurface.cs")
wire = read("rust/crates/nxtools-protocol/src/floating_assistant.rs")
contract = read(NATIVE + "Protocol/FloatingAssistantContract.cs")
regressions = read(DESKTOP + "tools/floating_assistant/application/tests.rs")

send = section(owner, "    fn send(", "    fn sync(")
require(send.index("self.lifecycle.begin") < send.index("send_command_with_id"),
        "native pending must precede observable dispatch")
require("show: false" in owner and "focus: false" in owner,
        "renderer must be created hidden/passive")
require("window.draw(cx).clear(cx)" in owner and "expected_frame" in view,
        "current-size GPUI frame must precede explicit show")
require("cx.defer" in router and router.index("cx.defer") < router.index("owner.update"),
        "platform event ingress must leave the dispatcher lease")
require("Phase::Retire" in owner and "self.cancel_actions(true)" in owner,
        "retry must retire native and deferred action ownership")
require("event.session_id != self.session" in owner and
        "event.sequence <= self.last_event_sequence" in owner and
        "accepted_drop_ids.contains" in owner,
        "session, sequence and once-only drop checks are required")
require("edge_hidden: self.edge_hidden" in owner and "pub edge_hidden: bool" in wire and
        '"edge_hidden"' in contract and "bool EdgeHidden = false" in contract,
        "explicit edge-hidden intent must cross Rust/C# together")
require('command.Operation != "prepare" && command.EdgeHidden' in coordinator,
        "only explicit subsequent edge hide may leave a partial client")
require("FloatingAssistantEnvironmentSignature" in coordinator and
        "_environment.TryPublish" in coordinator and "_environment.Reset()" in coordinator,
        "changed monitor/work-area/DPI/fullscreen snapshots must notify and reset")
require("if (!publish()) return false;" in coordinator,
        "failed environment delivery must remain retryable")
apply = section(coordinator, "    private WindowsCommandResult Apply(",
                "    private WindowsCommandResult Hide(")
require("0x0010 | 0x0020" in apply and "0x0040" not in apply and "SWP_SHOWWINDOW" not in apply,
        "native preparation must not transiently show via SetWindowPos")
require("SW_SHOWNA" in apply and "SetInteraction(_visible, command.ClickThrough)" in apply,
        "passive renderer and input visibility must be applied together")
require("NOREDIRECTIONBITMAP" in input_surface and "RegisterDragDrop" in input_surface and
        "RevokeDragDrop" in input_surface,
        "one input-only OLE boundary must retain registration cleanup")
require("SetParent(" not in coordinator + input_surface,
        "cross-process reparenting is not an approved native boundary")
require("send_command_with_id_correlated" in actions and
        "pending.follow_up.take()" in actions and "if !self.pending_actions.is_empty()" in actions,
        "actions must remain bounded, correlated and result-gated")
require("paste_after_write: false" in actions and "include_file_drop_fallback: false" in actions,
        "image drop must use copy-only native image data")
require("pending.dispatched" in actions and "pending.cancelled = true" in actions and
        "owner.action_is_current(id)" in actions,
        "cancellation must distinguish preparation from admitted native ownership")
require("open_floating_download" in actions and "show_floating_search" in actions and
        "submit_request" not in actions,
        "reuse existing confirmation/search owners, never silently submit")
require("MAX_SOURCE_BYTES" in media and "MAX_FRAMES" in media and
        "MAX_RETAINED_BYTES" in media and "media::decode_file" in owner,
        "decode must retain explicit source/frame/memory budgets")
require("background_executor().spawn" in owner and "render" not in
        section(view, "    pub(super) fn replace(", "    #[allow"),
        "decoded media must be prepared outside rendering")
require("1_048_576.0" in playback and ".clamp(0.25, 4.0)" in playback and
        "fps.clamp(1, 60)" in playback,
        "approved speed mapping/display cap must remain explicit")
require("self.timer = None" in view and "motion_allowed(cx)" in view and
        "self.progress.filter(|_| self.visible)" in view,
        "hidden/reduced surfaces must stop playback and progress presentation")
for test in (
    "native_pending_precedes_prepare_and_frame_precedes_reveal",
    "disable_during_prepare_cannot_reveal_and_reuses_window",
    "disconnect_and_late_ack_never_reveal",
    "drop_copy_is_once_only_copy_command_and_releases_after_result",
    "cancelled_deferred_routes_release_drop_without_dispatch",
    "disable_keeps_dispatched_copy_alive_until_native_result",
    "main_restore_is_bounded_and_activation_follows_success",
    "environment_reapplies_revealed_bounds_and_fullscreen_recovery",
    "disable_during_image_preparation_never_dispatches_late_copy",
    "disable_invalidates_decode_cache_and_reenable_loads_current_image",
):
    require("fn floating_assistant_" + test in regressions,
            "missing production lifecycle regression: " + test)

print(f"PASS floating assistant lifecycle: {checks} source contracts (not native/visual acceptance)")
