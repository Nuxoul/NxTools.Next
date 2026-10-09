# Download Window Lifecycle Remainder

## Authority and phase

Use `docs/reference-code/download.md` for legacy task window reuse/terminal transformation and `.trellis/tasks/09-28-download-window-transition-polish/implement.md` for the exact canonical phase/order. Existing task remains in progress. This child only targets unverified native position/drag behavior.

## Current behavior to preserve

- `DownloadWindowLifecycle` retains one task window per task ID and transforms progress/failure/completed in place.
- New download probe completion uses `apply_new_download_window_resize` and C# `SetWindowBoundsCentered`, which must preserve the actual native center and C# retry through move/size modal loops.
- Submission records a source HWND anchor and task window handoff uses placement/visibility operations.
- Task view already renders actual active connections/segments, a disclosure, bounded segment dots and detail content. Preserve it.

## Investigation boundary

Trace real HWND origin updates versus cached GPUI bounds during the exact user's sequence: drag compact new-download dialog while probe runs -> probe result changes layout -> submit -> progress window -> complete/fail. Do not alter geometry based on the historical GPUI bug unless current vendored GPUI/native adapter still exhibits it. The C# adapter's native bounds are authoritative after drag.

Add the narrowest red-capable test at production-bound geometry/handoff seam. Prefer existing `StashWindowBounds`, `SetWindowBoundsCentered`, `MoveWindowCenteredOverKey`, and `SetWindowVisibility`; no frame-by-frame native commands. Do not rename/reorder canonical task phase or move later work into it.

## Validation

Focused GPUI test for drag-aware probe resize/handoff and actual center; tests covering existing count/dots/details; full relevant desktop tests; `tests/architecture/download-ui-lifecycle.ps1`; PlatformHost geometry tests. Manual Windows smoke remains an explicit acceptance gate.