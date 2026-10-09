# Parent Execution Plan

## Canonical Work Order

1. **Global shell settings apply failure** (child `10-02-global-shell-settings`): fix the current Rust/C# compile defects and staged error result contract. Run Rust/C# focused tests before proceeding.
2. **Download secondary-window lifecycle** (child `10-02-download-window-lifecycle`): re-read `.trellis/tasks/09-28-download-window-transition-polish/implement.md` and `docs/reference-code/download.md`; continue its current canonical phase without renaming/reordering. Keep existing dynamic connection count, dots and expanded details. Add only demonstrated native-position fixes; complete Windows interaction smoke if available.
3. **Link detector floating panel** (child `10-02-link-detector-panel`): exact hostname regression, generation-scoped timer, topmost result on creation/reuse, visible configured-action feedback and transparent GPUI root. Verify the real HWND path.
4. **Whole-app UI redesign** (child `10-02-staged-desktop-ui`): after correctness prerequisites are accepted, run five reviewable stages in child PRD order (foundation/shell; Downloads; Quick Find; Clipboard/Tools; global/supporting surfaces). Each stage requires user visual review before moving to the next stage.

## Parent Integration Checks

- Before each child starts, review that child's PRD/design/implement files and relevant specs; `task.py validate <child>` must pass.
- Rust commands must inherit `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, and `RUSTFLAGS=-C codegen-units=256`; respect release memory preflight and do not start duplicate builds after timeouts.
- After child 1 run `cargo fmt --check`, focused `nxtools-desktop` tests, PlatformHost tests, desktop clippy and C# compile/test.
- After child 2 run focused/full relevant desktop tests and `tests/architecture/download-ui-lifecycle.ps1`; separately record interactive Windows native-drag and transition evidence.
- After child 3 run focused/full desktop tests, `tests/architecture/ui-token-boundaries.ps1`, and source/window lifecycle gates; separately record real native topmost/transparency/input/timeout smoke.
- For each visual child stage run the token boundary test plus the affected module interaction suite and GPUI lifecycle/architecture gates. Capture before/after screenshots where available and obtain user review before the next stage.
- Finish with full relevant desktop and PlatformHost tests, `cargo fmt --check`, clippy, `tests/architecture/download-ui-lifecycle.ps1`, `tests/architecture/ui-token-boundaries.ps1`, and a scoped working-tree/diff review. Preserve user edits outside this task.
- Do not package, push, or shut down as part of this task unless separately requested after implementation and release review.

## Approval Gates

- This parent and the four child task artifacts are planning only. Do not run `task.py start` for a child until the user explicitly approves this final plan.
- Each major visual stage has a user review/acceptance checkpoint; whole-app completion is not inferred from one approved mock/screenshot.
