# Stabilize download window transitions

## Goal

Remove visible download-window jitter and make probe confirmation and probe-to-task handoff feel continuous without weakening the existing lifecycle guarantees.

## Requirements

- Treat compact, confirmed, media-inspector, and error sizes as generation-scoped layout states with one authoritative native resize request per state transition.
- Keep the native C# centered-resize boundary as the authority for real HWND geometry and its drag/modal-loop retry. Remove redundant Rust-side timer resends that can repeatedly recenter the same window.
- Route all post-creation new-download size changes through the same native-aware helper; keep the footer and form geometry stable when errors appear.
- On accepted submission, stash the probe window's native anchor, create/register the task window without exposing its default centered rectangle, place it over the saved anchor, then reveal it and retire the source window exactly once.
- Add a short, generation-keyed GPUI opacity/offset transition for the confirmed probe content and incoming task surface. Do not animate native bounds frame by frame.
- Preserve one task window per task ID, in-place progress/failed/completed transforms, native-close cleanup, terminal action cleanup, and focus/activation policy.
- Emit bounded lifecycle diagnostics for layout request, handoff placement/reveal, transition completion/failure, and stale-generation suppression.

## Acceptance Criteria

 - [ ] Probe success, probe failure, submit validation failure, and media-probe layout tests show stable footer/body bounds and no duplicate native resize stream for one generation.
 - [ ] A stale probe result cannot resize or animate a newer dialog generation.
 - [x] A submit-to-progress test proves the source position is stashed before retirement, the task window is placed before visibility, and the task window is not briefly shown at a default center.
 - [ ] The incoming task view has a bounded short fade/offset transition; no repeating idle animation or high-frequency native command loop is introduced.
 - [x] Progress-to-failure/completed and retry transitions reuse the same task-window identity and preserve existing action behavior.
 - [ ] Existing browser handoff, Blob import, scheduled task, taskbar overlay, DOpus, and completion/reveal behavior remain green.
 - [ ] Focused desktop tests, relevant Rust tests, and a Windows manual smoke test show no jitter during probe confirmation, submit, retry, or native window drag.

## Notes

- Behavioral references: `docs/reference-code/download.md` for task-window reuse/center-preserving transforms and `docs/reference-code/gpui.md` for restrained opacity/offset motion.
- The current likely jitter source is the Rust five-step resize retry loop in `new_download_view.rs` combined with a destroy-then-deferred-create handoff.
