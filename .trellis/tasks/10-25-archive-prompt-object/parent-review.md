# Parent review — audit #4, changes requested

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after single-item delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`, task `38ad34a0-7b05-4a5f-b85b-e1e9c983b276`.

## Decision

**Changes requested; not accepted.** The reviewed display-only producer/transport/UI changes have no confirmed production correctness blocker from this one directed review. Two concrete delivery corrections are required before the slice can be accepted. No seventh item is authorized, and no unrelated product repair is requested.

### R1 — preserve the unresolved diagnostic without making this slice's default suite permanently fail

`archive/job_tests.rs::audit04_create_keyboard_release_and_missing_object_cancel_are_once_only` is new in this slice and is included in default Desktop tests. Parent independently reproduced 0 replies versus the expected 1 after actual focused CreateOverwrite Button Space down/up. Thus this delivery adds a known failing default regression even though the product keyboard activation owner is unchanged by the display repair.

Do not fix the original DialogAction/Button keyboard behavior in this item, add a no-op callback, change vendor code, weaken the expected reply, add ignore/conditional bypass, or delete any pre-existing regression. Preserve the complete current diagnostic source, original failing assertion, command, actual focus evidence and failed logs as a clearly labeled unresolved task diagnostic outside the default product regression set. The independently passing pointer/fallback and Conflict keyboard regressions remain standard tests. This is a change to the placement of a newly introduced out-of-scope diagnostic, not a claim that keyboard activation works. Keep all historical full-suite failures and the unresolved keyboard limitation explicit.

### R2 — follow engineering document language conventions

The new task PRD and outcomes are written primarily in Chinese. `.trellis/spec/index.md:39` requires engineering documents to be English. Translate those two task documents while retaining exact phase/API/test names, complete failed-run/concurrency/compatibility history and all acceptance limits. Preserve Chinese user-facing labels and messages in product code. Do not translate or rewrite raw evidence/logs or alter other task documentation.

## Directed review completed

- Read the complete task PRD/outcomes, all new Protocol display fields/validation/privacy tests, worker conflict and PrivateOutput producer code, the worker matrix, C# typed display/session forwarding and reviewed fixture bodies, actual Dialog content/footer and all seven new Desktop tests.
- Verified 13 existing delivery hashes, three new source/document hashes, two observed-unmodified hashes and 81 evidence hashes against the delivery manifest. Independently reconstructed and forward-replayed all 13 task-start dirty own-deltas in memory: exact hashes and bytes passed. No workspace product mutation.
- Independently checked all 13 protected prior-item production hashes. Reviewed the three precise job_tests concurrency segments and reconstructed baseline -> own initial -> external formatting -> resumed final. Reviewed Protocol observed-live -> resumed final; the unavailable intermediate formatter bytes are correctly disclosed, not fabricated as an isolated exact delta.
- New display is optional, defaulted/None-omitted and typed for Conflict/CreateOverwrite only. Rust Debug and C# ToString redact it. Existing strict unknown-field rules and version remain; new readers accept old omitted payloads, but old strict readers reject populated new fields. No bidirectional mixed-version compatibility is claimed.
- Worker captures exact current member/target at the original Ask ticket. The new display_prefix is separate from native plan_prefix; original native observation/token/decision/output paths do not consume display. C# builds the UI target from the same immutable command, not a filesystem authority or another job's latest state. UI shows this prompt's snapshot, with honest missing-data fallback.
- Original reply/ticket/equality/once-only/defer/cancel/apply-all authority remains. The same official Checkbox is moved into the fixed footer for long-body reachability; official DescriptionList/Label content retains full identifiers and original window budgets.
- Exact released DialogAction and Button sources support the documented keyboard-owner limitation. The old owner block is byte-equivalent in the own-delta. This is source attribution, not a separately run complete old-UI runtime proof; do not overstate it.
- No further broad review or reviewer subagent was started. The two requested corrections are the closeout scope for this pass.

## Parent-executed checks

Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, serially after natural-idle checks. No process/session was paused, killed or asked to yield/reserve/reorder work.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop audit04_ -- --test-threads=1` | FAILED: 6 passed / 1 failed; actual CreateOverwrite Space 0 replies vs 1, `job_tests.rs:1102` |
| Worker prompt_display_contract with isolated-codecs | PASS: 2/2, matrix covers 18 format/strategy runs |
| Worker interaction/job/readmission/cancel/extraction/wire regressions | PASS: 37/37; with display matrix total 39/39 |
| `cargo test -p nxtools-protocol archive_job -- --test-threads=1` | PASS: 6/6 |
| `cargo test -p nxtools-ipc -- --test-threads=1` | PASS: 42/42, no failed doctests |
| Previously failed Default Apps test, one targeted current-tree run | PASS: 1/1; not a full Protocol rerun or attribution to this slice |
| Strict Desktop / isolated-codecs Archive / Protocol Clippy | PASS on the current shared tree |
| `cargo fmt --all -- --check` | PASS on the current shared tree |
| C# Tests build, no restore, serial build | PASS: 0 warnings / 0 errors |
| Exact five reviewed C# fixtures | LIST5/RUN5/PASS5/SKIP0/FAIL0, GUID-owned synthetic files/helper copies/original sessions, no live UI/Shell/user state |
| Catalog/release/editor/spec/archiveCLI/PhaseF shell/spike/ClipboardUI/DownloadUI gates | PASS: nine checks |
| Archive release-chain `-SourceOnly` | PASS; no packaging was invoked |
| Protected hashes / precise own-deltas / concurrency segments / scoped diff check | PASS |

Parent logs in its scratch are prefixed `item04-review-`: ui, worker, protocol, ipc, worker-safety, defaultapps-failure, desktop-clippy, archive-clippy, protocol-clippy, fmt, csharp-build/list/run. The initial multi-command result aggregator had a PowerShell scope/array error after the native commands; each native result and log was captured explicitly, and later checks used a list-based accumulator. It is not a product failure and did not trigger repeated UI/worker/protocol/IPC runs.

## Failed historical runs and shared-tree changes

Implementer reports one complete Desktop run **1207 passed / 1 failed / 0 ignored**, the newly introduced keyboard diagnostic; it remains a failed historical run. Parent did not repeat the full suite. Implementer Protocol **79/80**, Desktop Clippy needless_update and workspace fmt failures remain in raw evidence. During parent review, the Default Apps single test and current Desktop Clippy/workspace fmt passed, and shell_integration.rs no longer contains the reported redundant update. Those independent shared-tree changes are not attributed to this item, and a targeted pass does not rewrite the historical failed suite.

The extra Quick Launch source gate remains the previously reported separate failed boundary; no build/security/gate repair or acceptance is part of this review.

## Re-delivery requirements and limits

Return only R1/R2 changes plus updated precise own-delta/manifest and fresh relevant scoped verification. Preserve immutable original failed evidence, including a readable copy of the complete unresolved diagnostic. Do not rerun a complete Desktop suite merely to replace a failed record; run the corrected standard audit04 selection and affected original job/Archive regressions, source/doc checks and required scoped lint/format checks. Any new failure must be reported, not fixed outside scope.

No native picker/clipboard/Shell/HWND/product/PlatformHost/service, user config/database/archive/registry, package/install, commit/push, physical input/IME/DPI/reader/screenshots/visual/user acceptance occurred. C# temporary file/Job fixtures are isolated compiled evidence, not live product lifecycle acceptance.

Worker must stop after re-delivery. Parent will inspect these concrete corrections and retest them; no new broad review or next backlog item is authorized by this document.
