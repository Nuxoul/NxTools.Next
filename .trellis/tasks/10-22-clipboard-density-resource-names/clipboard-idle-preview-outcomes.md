# Clipboard stationary-open preview repair

## Outcome and scope
The user reports a passive image preview appearing on summon without any movement. Source repair is implemented in the existing Clipboard trigger chain. No package/install/product launch, real clipboard/history/config mutation, service operation, delegation/contact, commit/push or archival. Preserve concurrent Archive/editor/browser/release work. Physical/installed reproduction after deployment remains unverified.

## Actual red and root cause
`cargo test -p nxtools-desktop idle_preview_ -- --test-threads=1 --nocapture` ran before production edits:1 failed/0 passed,0 ignored. Real panel starts with no rows, pointer parks at(45,155), rows are restored and painted three times without another mouse event. Assertion failed: `painting a row underneath a stationary cursor must not arm the preview timer`.

Exact released GPUI pre0.3.9 elements/div.rs3133–3184 invokes on_hover from paint as well as mouse events; window.rs3740 refreshes hit testing before paint. Existing history.rs forwarded on_hover(true) directly to the original preview scheduler. Thus initial hit presence incorrectly established preview intent. A fresh fixture reproduces it without an old popup session or native activation event. Legacy D:/Code/NxTools/Controls/ClipboardHistoryListView.cs434–460 establishes the hovered preview item from PointerMoved. Legacy popup constructor/Open/Activated/Prepare/Hide and one passive preview chain were reread; Avalonia native base is unavailable, not invented evidence.

## Minimal implementation
- ui/history.rs: keep released Component0.8.0 ListItem visual/selection/size/parent adaptations. Public on_mouse_enter forwards actual movement, including inside an already-hovered row. Reject pressed/active-drag movement. Ignore on_hover(true) from paint, retain on_hover(false) cancellation.
- ui/mod.rs: one panel-retained screen-position Cell, seeded at creation, updated by existing bounds observation and parent actual move. Reject unchanged-coordinate events across redraws. Existing deferred callback now also validates captured session/query and current row ID plus current open/editor/armed/pending/fullread/query readiness.
- clipboard.rs: same hovered item admission is idempotent at original PreviewSession, preserving the original800ms timer rather than restarting on every move. A test-only generation/item observer supports real-boundary assertions.
- No changes to PREVIEW_HOVER_DELAY, native presentation, one passive HWND, scroll suppression/resume, full-ID hydration, original payload, styling, C#/protocol or framework sources. No new polling/global hook/controller/log stream.

## Regression and diagnosis ledger
| Execution | Result |
| --- | --- |
| Original stationary-paint test before fix |0 PASS/1 FAIL, actual candidate incorrectly armed |
| First/second four-case checks |2 PASS/2 FAIL; active-motion controls had unsettled fixture initialization; probe observed admitted entry followed by leave, and draining initial callbacks resolves the controls |
| Test-only DEBUG-idle-preview fixed-stage probe |Confirmed real entry/current=true followed by late leave; no pointer/payload values logged |
| Drain initial window callbacks before active controls |4/4 PASS; original failing-capable stationary test retained |
| Final new regressions including image |5/5 PASS,0 ignored |
| input_followup_ |4/4 PASS |
| input_repair_ |8/8 PASS |
| Initial wrong presentation filter |0 tests; not validation |
| Exact clipboard::preview_presentation module |14/14 PASS incl800ms/cancel, passive ACK ordering/failure/timeout/stale/reuse/measurement |

Five new tests exercise stationary text mount/no candidate after1200ms, stationary image mount/same-coordinate events then real movement, same-row generation/leave/reentry, same-position/pressed events, and old session/query before deferred admission. All DEBUG-idle-preview instrumentation was removed. No tests changed unrelated Archive behavior.

## Final integrated verification
All Rust commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. Busy preflights exit75 started no tests; no other process was terminated/contacted.

- Strict `cargo clippy -p nxtools-desktop --tests -- -D warnings`: PASS, including dependencies.
- `cargo check --workspace --tests`: PASS.
- One final full Desktop: **1040 PASS/1 FAIL/0 ignored**,1041 total,133.56s. All five new regressions and original Clipboard input/presentation tests pass. Failure is `archive::application::tests::production_row_drag_read_hydration_and_result_route_share_original_snapshot`, archive/application_tests.rs689, assertion `cross-entity result writeback is deferred`. Read-only inspection shows the Archive test calls app.notification then asserts view.drag_matches before run_until_parked. Its implementation/tests are concurrent work outside this repair. No causal root-fix, isolation rerun or full-suite rerun was performed; integrated result remains NOT GREEN.
- Static16 gates: **15 PASS/1 FAIL**. Six Python provenance/docs gates and nine PowerShell lifecycle/motion/isolation/token/consistency gates pass. quick-launch-lifecycle.ps1 still fails its broad elevated-navigation condition; the known literal check at1762 requires write-release-binary-manifest.ps1 in scripts/build.ps1, which currently delegates publish-release-hosts.ps1. Neither source nor gate is changed by this repair. Do not claim this literal is the sole possible failing clause without enumerating the whole condition.
- Scoped rustfmt four source files and git diff --check: PASS.
- Saved idle-preview baseline comparison: native clipboard_preview_presentation.rs SHA256 unchanged; direct three-file production diff review contains only the stated admission/identity/idempotence changes. No broad format/restore.

## Evidence and remaining acceptance
Scratch logs: clipboard-idle-preview-red.log; focused.log/focused2.log/probe.log/settled.log; idle_preview_-verified.log/input_followup_-verified.log/input_repair_-verified.log (all prefixed clipboard-idle-preview-); final-presentation.log/final-clippy.log/final-workspace.log/final-desktop.log; static-results.json and per-gate logs (same prefix). Original clipboard-idle-preview-baseline/ retained.

This is a source/compiled synthetic regression repair, not evidence that the user's current installed executable has changed. No physical Windows summon/hover/IME/mixed-DPI/reader/user screenshot acceptance was performed. Separate deployment authority and installed interaction acceptance remain necessary. Notification hovered Ctrl+C remains cancelled, not a blocker. Do not resume Archive/release fixes as part of this Clipboard task.
