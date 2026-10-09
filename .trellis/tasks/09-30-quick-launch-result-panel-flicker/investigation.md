# Quick Launch result-panel flicker investigation

Date: 2026-10-02
Status: **visual reproduction blocked; no product behavior changed**

## Environment and exact blocker

- The installed protected desktop process is running at `C:\ProgramData\NxTools\NxTools.Next\Service\NxTools.exe` with the launch intent `--browser-dialog`; its foreground window handle is zero. The paired installed PlatformHost is already running.
- PI-Desktop is the current foreground application. This session does not expose a native desktop screenshot/video capture tool; the available browser-preview capability cannot capture a native GPUI HWND.
- The approved phase is investigation-only. I did not send synthetic Ctrl input, change focus, restart/replace PlatformHost, open a competing desktop instance, or alter the animation/window behavior. Injecting the global gesture into the in-flight browser-dialog session without a synchronized recording would not meet the task's evidence gate.
- Consequently there is **no screen recording/frame sequence and no synchronized live event trace**. A root cause cannot be inferred from code inspection alone.

## Existing automated signal exercised

All commands ran with the workstation Rust environment (`CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`):

- `cargo test -p nxtools-desktop --bin nxtools-desktop popup_reveal_waits_for_first_frame_and_first_native_bounds_are_staged -- --test-threads=1` — passed.
- `cargo test -p nxtools-desktop --bin nxtools-desktop result_appearance_motion_does_not_flash_opacity -- --test-threads=1` — passed.
- `cargo test -p nxtools-desktop --bin nxtools-desktop panel_hide_is_result_tracked_and_self_healing -- --test-threads=1` — passed.
- `cargo test -p nxtools-desktop --bin nxtools-desktop production_popup_keeps_search_only_collapsed_and_expands_one_stable_surface -- --test-threads=1` — passed.
- Final desktop binary — 427 passed, 0 failed; `cargo fmt --check --all`, desktop clippy `-D warnings`, UI token gate and download lifecycle gate passed.
- Existing PlatformHost tests cover released double-Control taps, rejected repeats/intervening keys, both WndProc trigger routes, and fullscreen gating. They do not measure visible result-panel frames.

These prove state/lifecycle contracts, not the user's visible flash.

## Source-verified timeline and observability limits

1. `ModifierDoubleTapDetector.OnKeyDown` arms only after two released modifier presses in its time window; it returns the summon only on the second matching key-up. `OnGlobalKeyDown/Up` only post `WmQuickLaunchGesture`.
2. EventWindow handles the posted message and `QuickLaunchSummonGate.Dispatch` publishes one correlated `QuickLaunchPanelRequestedEvent` (subject to the fullscreen gate).
3. Rust rejects stale requests, then toggles the current popup or opens a session. New windows are created hidden/staged; reveal is scheduled for the next GPUI frame, positions the HWND, shows it, then starts native activation.
4. `prepare_for_show` starts with `results_visible=false` and an empty query. A later query/result/action-mode transition may mount the results surface; its entrance animation is keyed by `results_appearance_generation` and translates by 1 logical pixel without an opacity fade. Hide sets `results_visible=false`, increments that generation, and queues native visibility hide with one bounded retry.
5. Existing production logs expose C# summon decision (including its correlation), popup reveal/window handle, Rust hide request/window handle and correlated hide result. They do **not** currently form one operation-correlated timeline through the Rust session, `results_visible` transition, animation start/end, and native hide confirmation. No query or result payload was logged or inspected.

## Evidence-ranked causes

No cause is ranked: the visible symptom was not captured, so ranking would be speculation. Current code alone does not distinguish among the task's falsifiable candidates (entrance transition overlapping dismissal; reveal/first-frame ordering; stale/restarted results generation; duplicate trigger/dismiss event). No behavior fix is proposed in this investigation-only task.

## Safe user-run capture protocol

When the desktop is in its ordinary visible session and no browser-dialog operation is pending:

1. Start a desktop recorder at 60 fps and include a visible seconds/milliseconds clock if available. Do not expose a private path, URL, clipboard contents, or personal search term; use an empty summon, then a harmless query such as `notepad`.
2. Record at least 10 cycles of: double tap/release left Ctrl; dismiss immediately with a second double tap; reopen, type `notepad`, and dismiss while results are arriving; then reopen empty and type the first character. Leave ~1 second idle before/after each sequence.
3. Save the video locally. Alongside it, copy only payload-free log rows/timestamps for `quick_launch.summon_decision`, `quick_launch.popup.revealed`, `quick_launch.popup.hide_requested`, and `quick_launch.panel.hide_applied` / `hide_rejected` / `hide_failed`. Redact correlation IDs if sharing outside the project; never include window titles, query text, paths, URLs, or result labels in logs.
4. Note cycle number and approximate video timestamp for any flash. The current logs lack result-visible/animation boundary timestamps; if the video still cannot distinguish the candidates, add a separate approved temporary `DEBUG-...` probe at those state transitions, capture again, then remove the probe before any fix.

## Gate

Do not change animation duration, `results_visible`, hotkey detection, native show/hide order, or styling until the synchronized visual evidence is reviewed. Resume this task with the video/timestamp notes or permission to instrument and run an idle normal desktop session. The task remains `in_progress`.