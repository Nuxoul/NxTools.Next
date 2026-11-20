# Multi-monitor window lifecycle and feature controls

## Goal
Make Quick Launch, Clipboard and auxiliary task windows appear on the intended monitor and remain usable, restore automatic download progress presentation, and remove manual archive-integration setup from installed use.

## Background
Source inspection identifies mismatched C# display ordinals and GPUI monitor IDs, a primary-display-only link panel, missing confirmed native activation on archive job surfaces, a controlled-browser post-Commit progress-admission gap and incomplete native Clipboard enablement. Installation currently has no automatic archive Apply entry. The existing default-choice restoration/uninstall paths can retire restoration evidence prematurely. Evidence anchors and inspected external/native lifecycle boundaries are in [research](research/boundaries.md); these are source findings, not runtime reproductions.

## Requirements
- R1: Quick Launch, Clipboard and affected auxiliary windows respect multiple monitors, negative origins, work areas and mixed DPI. Use the inspected Listary decompiled managed window lifecycle as the primary Quick Launch placement reference. Summoned panels use the cursor monitor captured at summon; owned previews/task surfaces follow their owner. Reopening selects the current monitor before the first visible frame.
- R2: Default link-detector placement is top-right; default notification placement is top-center. Preserve explicitly configured placement.
- R3: Archive creation/extraction progress and visible completion surfaces reliably enter the foreground at presentation transitions. Passive notifications/previews never steal focus; explicit background/close remains authoritative.
- R4: Starting a download automatically presents its existing keyed progress window when automatic progress is enabled. Controlled browser downloads wait for real post-Commit start, not Held acceptance. Preserve QueueOnly/delay/background/close and same-window completion behavior.
- R5: Set the default parallel connection limit for each new download task to 16, preserving explicit configuration and existing tasks. The simultaneous-task queue limit remains unchanged.
- R6: Installation automatically applies archive integration through the existing owner, including silent installation and launch unchecked. Fresh installs use the existing recommended policy; upgrades preserve selected extensions/menu commands/opt-outs. Report genuine Apply failure or Windows association warning. Revert/uninstall restores recorded prior choices when still owned; restoration failure retains retry evidence instead of reporting complete success.
- R7: Add a persisted feature enable/disable Switch at the top of the main Quick Launch and Clipboard pages. It controls actual feature availability, closes feature-owned transient surfaces, retires pending work and stops native triggers/capture where applicable. Keep history, index data and individual preferences; re-enabling restores those preferences. Preserve independent temporary Clipboard pause intent.

## Acceptance criteria
- AC1 (R1): Secondary/left/above monitors and 100/125/150% mixed DPI keep panels/previews/task surfaces in the selected work area on first show and reopen/reuse, with no stale-monitor reveal.
- AC2 (R2): New/missing placement settings produce top-right links and top-center notifications; explicit saved values survive section updates and reload.
- AC3 (R3): Archive progress/reuse/visible completion requests confirmed native activation for the current surface; stale replies do not affect replacement windows. Background work and passive surfaces stay nonactivating.
- AC4 (R4): Browser Held has no progress activation; actual StartNow Commit produces one keyed progress window. Duplicate events, abort/removal, disabled automatic progress, QueueOnly and delayed work obey their lifecycle. Closing does not cancel or reopen on ordinary progress events.
- AC5 (R5): New/missing download connection settings produce default_connection_limit=16 and the actual new-task request receives it; explicit old/custom values and existing tasks survive section updates/reload. Simultaneous-task queue concurrency remains unchanged.
- AC6 (R6): Successful file finalization invokes integration as original user without a manual action or optional launch. Result is acknowledged across protected redirect/existing-instance forwarding. Failed Apply remains visible; uninstall/default restoration preserves later user changes and incomplete recovery facts.
- AC7 (R7): Pointer/keyboard produce one requested-value callback. Off prevents native/local/queued summons and capture, hides owned panels/previews and prevents late reveal. Save/apply errors and reconnect remain truthful; the switch is usable while off and re-enable preserves history/settings and temporary pause semantics.

## Constraints
Rust owns state/layout, GPUI rendering and C# Windows APIs. Preserve original generation/task/session/transaction owners and section-owned durable writes. Use official Kit0.8.0 Medium Switch and retain existing Settings composition, outside native captions. Diagnostics carry one correlation, accepted/start/terminal/elapsed/native code and no payload secrets. Preserve preexisting installer/runtime/PlatformHost changes.

## Out of scope
Dependency upgrades, theme/window redesigns, new download/archive schedulers, data clearing, changing explicit user settings, indexer engine/service redesign, package creation, installation, real Registry writes and live Shell/service acceptance without separate authorization. Compiled/synthetic checks and physical/native/user acceptance are separate evidence levels.
