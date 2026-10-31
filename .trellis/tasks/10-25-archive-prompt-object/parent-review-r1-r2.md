# Parent closeout — audit #4, R1/R2 verification

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after revision delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`, revision task `b2a1a66f-ac05-4c6e-9f80-e5f116cdf6f4`, report `70b43a1d-9dc2-4cb7-8d01-ba4184f4e8ff`.

## Decision

**R1/R2 satisfied. Original audit #4 is accepted only for the reviewed display-only source, compiled and synthetic producer/transport/Dialog behavior.** The unresolved CreateOverwrite Space activation defect is explicitly not fixed or accepted. No new full Desktop result, native/physical input, reader, DPI, visual, installed-product, Release or user acceptance is claimed.

This closes the concrete corrections from the original directed review. Parent did not restart a broad review, delegate another reviewer or authorize a seventh item. The original `parent-review.md` remains immutable and records the changes-requested decision and the earlier executed checks.

## R1 — verified exact diagnostic relocation

- Read the complete preserved diagnostic and its English reproduction/status README. The saved function retains its original test attribute, real Tab/focused Button assertion, explicit Space KeyDown/KeyUp, expected-one reply assertion, duplicate-release expectations and the entire later fallback/cancel section.
- Diagnostic SHA256 `d961cc97bed5db46c476dc1d10ee5036eb2d0c26b52bc867308854178d1d277c` matches the original saved function. Its complete original compilation context SHA256 is `1723f2428a980d14a06871069c41ae5a92c5746f308e7e37c52f89347a5c639e`.
- Independently verified the current `archive/job_tests.rs` equals that original context with exactly this one complete newly introduced diagnostic removed, byte-for-byte; every other byte remains. No pre-existing test/assertion is removed, no ignore/conditional bypass is added, and six display-scope regressions remain registered.
- The preserved source lives at `evidence/revision-r1-r2/unresolved-diagnostics/create-keyboard-unresolved-probe.rs.txt`, with complete context and required private helper/dependency/command information beside it. It is clearly **UNRESOLVED / NOT ACCEPTED**, not a standalone runnable test or a passing default regression.
- Product `DialogAction`/Button/on_ok, display implementation, worker/protocol/C# and vendor code were not changed in R1/R2. Moving the diagnostic does not fix the underlying Space activation failure. Pointer/fallback and Conflict keyboard success do not imply CreateOverwrite keyboard success.

## R2 — engineering document correction

Read the revised PRD and outcomes. They are now English engineering documents, retaining exact phase/API/test names, original red/green and intermediate failure history, one-way wire compatibility, two concurrency events, Protocol attribution limits and separate parent-run results. Chinese product labels remain unchanged. The original Chinese document bytes remain in versioned baselines and exact deltas; raw evidence/logs were not translated or rewritten.

## Integrity verification performed by parent

- Three current revised file hashes and all 28 revision evidence hashes match the revision manifest.
- Independently reconstructed and replayed all three baseline-to-after deltas in memory, with exact baseline/delivery hashes and live bytes: **3/3 PASS**, no workspace writes.
- All **92 frozen original artifacts** and **28 frozen source/reference/test files** still match their recorded hashes. Original parent review SHA256 remains `a614b00a21d8ec914a5046ddbb17cf037eee9fed768aca2b464ae6025a725637`.
- Original delivery manifest SHA256 remains `bea18e4b467792ba69e46cb825d96b405a63e684634c1638e92ea64d27371924`; original own-deltas SHA256 remains `ce07e982cc393e881976240d5cd20900db633bb21db074b278f60bb5b14d277b`. They describe the original delivery, while the new manifest describes the revision; no historical hash was silently updated.

## Parent-executed targeted checks

Every Rust shell inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. Checks ran serially after natural-idle checks, with sufficient timeouts and no other-session/process interruption or request to reserve/reorder build time.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop audit04_ -- --test-threads=1` | PASS: 6 passed / 0 failed / 0 ignored; 4.20s |
| `cargo test -p nxtools-desktop archive::application::jobs::tests:: -- --test-threads=1 --skip audit04_` | PASS: 11 original job regressions / 0 failed / 0 ignored; 0.31s |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| `rustfmt --check --edition 2024 --config skip_children=true rust/apps/nxtools-desktop/src/archive/job_tests.rs` | PASS; no formatting write |
| Spec / component catalog / GPUI release / Markdown editor gates | PASS: four checks |
| Scoped `git diff --check` | PASS |

Parent logs are in parent scratch: `item04-r1r2-review-standard.log`, `item04-r1r2-review-jobs.log`, `item04-r1r2-review-clippy.log`. No full Desktop, full Protocol, worker or C# rerun was needed for these corrections; their earlier actual results are in the unchanged original parent review and delivery evidence.

## Historical failures and remaining limitations

The initial focused **6/7** and sole complete Desktop **1207/1208** remain failed historical runs, with their complete diagnostic and logs preserved. No later full-suite green is asserted after relocation. The underlying CreateOverwrite Space defect remains outstanding. Initial Protocol **79/80**, Desktop Clippy and workspace-format failures remain historical evidence, separately from the original parent review's later targeted current-tree passes; no unrelated shared-tree change is attributed to audit #4. The additional Quick Launch architecture gate remains the previously recorded unresolved verification boundary.

No actual product/PlatformHost/service/native HWND, picker/clipboard/Shell/OLE, real user configuration/database/archive/registry, physical input/IME, multi-DPI, UIA/reader, screenshots/visual comparison, package/install, commit or push operation was performed in this revision closeout. Existing synthetic C# temporary file/Job evidence is not live product lifecycle acceptance.

The sixth single item is closed at the limited evidence levels above. The implementation conversation must remain stopped. Another backlog item requires a new explicit single-item dispatch and another review-before-next-item gate; this record does not authorize it.
