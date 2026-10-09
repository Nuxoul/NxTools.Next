# Quick Launch Result-Panel Flicker Investigation

## Goal

Reproduce and explain the brief visible flash of the Quick Launch search-results surface when the panel is summoned or dismissed with the double-Control gesture. This child is investigation-only; do not change animations or window lifecycle before findings are reviewed.

## Confirmed Facts

- Quick Launch is rendered by Rust GPUI 0.2.2; Windows hotkey/native window ownership is in C# PlatformHost.
- `rust/apps/nxtools-desktop/src/quick_launch/panel.rs` conditionally mounts the results surface when `results_visible` and runs a 120ms `with_animation` keyed by `results_appearance_generation` (`:1370-1382`).
- `rust/apps/nxtools-desktop/src/quick_launch.rs` sets `RESULTS_APPEAR_DURATION` to the shared 120ms entry duration (`:96`); the double-Control gesture is detected by PlatformHost and published as a Quick Launch request.
- The user can see a brief flash while opening/closing, but no synchronized recording, exact reproduction frequency, or generation/window trace is attached yet.
- The current worktree contains unrelated uncommitted UI changes including recent typography/motion edits. Preserve them and do not assume they caused the reported symptom without a before/after reproduction.

## Requirements

- Exercise the real double-Control summon path and dismiss through the real GPUI/native window lifecycle.
- Capture a short screen recording or frame sequence synchronized with payload-free timestamps for gesture accepted, summon request/session generation, native show/activation confirmation, results visible/generation, entry-animation start/end, dismissal reason, and native hide confirmation.
- Vary only one trigger at a time: immediate dismiss after summon, fast result arrival while dismissing, empty query then first keystroke, and repeated open/dismiss cycles.
- Do not log query text, result labels, paths, or other search payloads.
- Rank falsifiable causes: stale/restarted result generation, entrance animation overlapping hide, native show before first committed GPUI frame, duplicate summon/dismiss event, or another observed transition.
- Provide a red-capable regression gate that detects the actual lifecycle invariant. If no automated test can assert the visible flash, explicitly separate automated state/lifecycle coverage from the manual visual acceptance protocol.
- Propose the smallest fix and stop for user approval before changing behavior.

## Out of Scope

- Changing animation duration, `results_visible`, show/activation ordering, hotkey detector behavior, theme, or panel layout during investigation.
- Rewriting Quick Launch or introducing another animation/window framework.
- Modifying clipboard/PlatformHost code except for reading the existing gesture/event contract.

## Acceptance Criteria

 - [x] A repeatable protocol reproduces the flash and produces a synchronized visual artifact plus lifecycle trace, or records an exact environmental blocker and user-run capture steps.
 - [ ] The timeline shows whether the results surface is mounted/animated before, during, or after native visibility and dismissal.
 - [ ] At least 3 falsifiable causes are ranked with evidence and a confidence statement; no root cause is inferred from source inspection alone.
 - [x] A correct lifecycle regression seam is exercised; any limitation of automated visual detection is explicit.
 - [ ] No product behavior changed; the smallest evidence-backed fix proposal is presented for approval.

## Current Probe Plan

1. Read the full gesture, request dispatch, session generation, panel visibility, and dismissal paths plus existing lifecycle tests.
2. Run repeated double-Control cycles with a synchronized capture; include a fast result-arrival/dismiss race.
3. Compare visual frames to generation/show/hide timestamps and process/window identity.
4. Add only bounded, payload-free temporary tracing if existing events cannot correlate the transition; remove probes after evidence collection.
5. Report findings and stop before implementation.
