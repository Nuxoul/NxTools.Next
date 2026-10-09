# Finish Download Secondary-Window Lifecycle

## Goal

Finish and verify the download secondary-window position/drag lifecycle without reimplementing work already present in the current branch.

## Confirmed Context and Reference Authority

- Follow `docs/reference-code/download.md` as the legacy behavioral source and the existing canonical task `.trellis/tasks/09-28-download-window-transition-polish/{prd,design,implement}.md` for phase names and order.
- The earlier request included: retain position through probe/progress/completion, dragging during probe must not corrupt layout, dynamic connection count must replace stale “8”, dot-style per-segment progress, and an expand/collapse detail control.
- Current source already has live active-connection/segment count, segment dots, expanded details and native position-anchor handoff. Do not recreate these features.
- The canonical task records automated checks complete but Windows interactive drag/resize and visual smoke outstanding.

## Requirements

1. Preserve task-window per-task identity and the in-place progress/failure/completion lifecycle.
2. Reproduce the remaining position defect at the native boundary: drag a probing/new-download HWND, allow probe completion/layout resize, then observe its actual native location and layout.
3. Verify actual HWND-center preservation on progress-to-failed/completed size transitions. Do not use stale GPUI cached origin as native geometry evidence.
4. Verify the hidden replacement handoff places the target before revealing it and removes the source exactly once.
5. Retain current dynamic connection/segment count, dot visualization and details disclosure; add only regressions if their current behavior is not adequately protected.
6. Use a red-capable test at the production window/native geometry boundary before any correction.

## Out of Scope

- Download-engine, task persistence, queue, request, IPC protocol or product download setting changes.
- Rebuilding the already-present thread count, segment-dot view, or details panel.
- Reordering or renaming the existing canonical download transition phase/step.
- Visual redesign of unrelated Download Manager surfaces; that belongs to the staged UI child after this correctness gate.

## Acceptance Criteria

 - [ ] A regression reproduces dragging while probing and verifies probe result/resize leaves the HWND at its actual user-moved bounds with stable body/footer layout.
 - [x] A regression verifies actual center and one keyed window identity through progress -> failure/completion transformations.
 - [x] Handoff command order proves target registration/final placement precede visibility and source retirement.
 - [x] `tests/architecture/download-ui-lifecycle.ps1`, focused GPUI download tests and the applicable desktop suite pass.
 - [ ] Windows interactive smoke verifies probe-drag, resize, handoff, retry and completion; if unavailable, report these interaction checks as pending rather than claiming completion.
 - [x] No changes to download engine semantics or unrelated worktree files.