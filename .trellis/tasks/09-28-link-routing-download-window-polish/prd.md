# Repair link routing and download window transitions

## Goal

Preserve the existing clipboard link detector while routing existing local files to Windows Explorer, and make the download probe-to-task workflow stable and lightly animated.

## Requirements

- Classify detected targets without changing the existing conservative recognition boundary: HTTP(S) URLs and bare domains remain web targets; existing drive and UNC paths become local file or local directory targets.
- Clicking an existing local file, including the reported `C:\Users\Administrator\.pi-desktop\scratch\...\Throne.exe` target, must copy the value and use the existing `OpenShellTarget` / `SelectInExplorer` action so Explorer opens with the file selected. It must never be prefixed with `https://` or launched through the browser path.
- Clicking an existing local directory must copy the value and use the existing shell `Open` action. Web targets retain the configured browser/copy-only behavior.
- Preserve the link panel's close-before-copy ordering and echo suppression. Do not regress the confirmed Directory Opus behavior or introduce a second shell protocol.
- Keep the download probe window at a stable layout for compact, confirmed, error, and media-inspector states. A single logical layout transition must not emit a visible sequence of competing reposition/resize operations.
- Transition from the probe window to the task window through one task-window lifecycle handoff: preserve the native anchor, avoid a centered-window flash or gap, reveal the replacement after native placement, and use a short GPUI content transition. Existing in-place progress-to-terminal transforms remain in place.
- Define bounded diagnostics for routing and window transitions. Events may include target kind, state, generation, window identity, elapsed time, and native result/error code, but must not log raw paths, URLs, cookies, headers, or clipboard contents.

## Constraints

- Rust owns classification, lifecycle state, layout intent, and animation presentation; C# owns Windows Shell and HWND operations; the existing IPC types remain the boundary unless implementation evidence proves a field is required.
- Reuse the native `SelectInExplorer`, `SetWindowBoundsCentered`, `MoveWindowCenteredOverKey`, and `SetWindowVisibility` contracts.
- Keep native geometry atomic and low-frequency. Animate rendered content, not every native resize frame.
- The implementation must remain generation-scoped and must not synchronously re-enter a leased GPUI entity from a callback.

## Acceptance Criteria

 - [x] A regression for an existing `.exe` path proves the link detector produces a local-file target and the real panel action emits `OpenShellTarget(SelectInExplorer)`; no browser launch command is emitted.
 - [ ] Regressions cover an existing directory, HTTP(S) URL, bare domain, copy-only mode, panel echo suppression, and an ignored nonexistent path.
 - [ ] Probe layout tests prove compact/confirmed/error geometry remains inside the fixed footer contract and stale probe generations cannot resize a newer state.
 - [x] Download handoff tests prove one task-window registration, preserved native anchor, no centered flash, and a visible replacement only after placement; task progress/terminal transforms keep the same window generation.
 - [ ] Motion tests or production visual tests prove a short opacity/offset transition without high-frequency native resize commands or animation timers that outlive the window generation.
 - [ ] Focused Rust and PlatformHost checks pass, followed by the relevant desktop test suite and a manual Windows smoke test using the reported local executable path.
 - [x] The existing DOpus behavior remains unchanged and the final diff contains no temporary `DEBUG-...` probes or payload-secret diagnostics.

## Notes

- This parent task coordinates two independently verifiable child tasks: local executable link routing and download window transition polish.
- Product default for local directories in this plan is shell `Open`; local files use Explorer selection semantics.
