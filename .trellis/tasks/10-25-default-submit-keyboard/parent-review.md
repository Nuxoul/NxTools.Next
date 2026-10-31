# Parent review — audit #5, seventh single-item delivery

## Decision and scope

**Accepted with source/compiled/synthetic-interaction limits on 2026-10-25.** This is the parent's independent review of task `7c30bc5e-547e-40e9-925b-463e4e5bc522`, delivered by session `da6a2ca3-52f0-41f1-a333-3b5548154f0b` (delivery message `2f88c230-5b44-4ff4-be10-230c1854fd5a`). No confirmed in-scope defect requiring a revision was found. This does not grant product, native, visual, release or user acceptance. Item eight remains unauthorized and unstarted.

Only original audit #5 is accepted here: safe focused default submission for OperationForm and DownloadScheduleView, and actual focused archive prompt Button activation through the original Dialog/reply owner. The previous six limited acceptances remain historical records; this review does not rewrite them or advance the archive plan/checkpoint.

## Independent source review

Reviewed the eight task-start dirty-baseline deltas, PRD/outcomes, three production entry points and directly appended tests. Consulted current component-selection/UI/verification contracts, locked 0.8.0 reference table/index and exact Component Button/Dialog source. Independently inspected published gpui-pre 0.3.9 `elements/div.rs` keyboard release/focus-generation logic and `keymap.rs` NoAction precedence from the parent's previously verified source archive. The implementer's external reference investigation is recorded in its reference append; this parent review does not claim a second visit to unavailable native framework internals or proprietary implementation.

- **OperationForm:** unconsumed raw Enter is a local fallback only for actual name/destination/volume focus. It checks character preference, modifiers, held/repeated press, marked IME text, original busy/picker/format/retired/submitted state and active Dialog before calling original submit. KeyUp and window activation reset the latch but do not submit. Password/confirmation, multiline, Select and nested format Dialog retain their own owners. Typed Create admission and original owner-close are tested; extract/modify retain observable original validation/admission behavior.
- **Daily schedule:** the fallback requires enabled, actually focused TimeField. Original save/deferred application update is preserved, with no root/unfocused approval. Original segmented editing and ordinary Cancel Buttons retain ownership. Persistence is correctly the existing SetSchedule/queue SQLite path, not a newly invented AppConfig write. Temporary SQLite reopen and byte-identical AppConfig are asserted.
- **Archive prompt:** actual official Button on_click dispatches public Confirm through its own non-Tab child anchor to original Dialog on_ok, then original deferred reply_job. No empty callback, pointer-bubbling workaround, vendor change or second reply dispatcher. NoAction is bound only in ArchivePromptButton context to remove ancestor Enter interception; it does not generate an action. CreateOverwrite additionally requires explicit Button activation, preventing root/body Enter approval. Local per-button filters reject invalid input and stale release; they do not emit clicks. Original GPUI owns release activation/focus-generation cancellation. Prompt-ID keys and view activation clearing prevent armed state crossing replacement/deactivation. Cancel remains DialogClose; Conflict retains its original decision callback. Shared Password/Risk/Destination and real OutputBudget readmission tests preserve typed reply and budget authority.
- Original submit, save and #4 object identifier helpers are independently whitespace-normalized equal to their task-start versions. The original prompt reply, ticket, privacy and deferred cross-entity boundaries remain. No new diagnostic payload or frame/input logging was added.

Production windows, native budgets, palettes, visible labels/variants, shared action helpers, protocol/worker/C#/engine implementations are not changed by this slice. Headless geometry/name regressions do not constitute visual or native acceptance.

## Integrity independently verified

Parent verifier and results are in `parent-evidence/integrity.py` and `parent-evidence/integrity-result.json`.

- Delivery manifest SHA256: `52cb1160a4536aae54adb064ab2cdb0436ae77c6e44ec3a6502931f4ae4e2ab4`.
- Own-deltas SHA256: `f6a8d678e638d21da6bf4c33f1261a4e2cc54e201ad25f844ec6fd9f0f7aed8c`.
- All eight current existing-file hashes, two new delivery documents, observed unchanged downloads/tests.rs and **85 delivery evidence hashes** match.
- **Eight own-deltas independently replayed forward and reverse**, with exact before/after spans. Three original direct-test file prefixes remain byte-identical.
- archive.md attribution is separated: original 770-line baseline -> own 30-line append -> independently observed external 34-line append. Both steps replay in both directions. External content is preserved and is not credited to this task. Current whole-file SHA256: `d27675547b011db78524f02a47ed76d3206e51c9233ca2c9014841900238d1f4`.
- **13 protected production files** and **124 frozen prior #4 task files** remain hash-identical, including failed logs, unresolved-at-that-time diagnostic, manifests and parent reviews.
- Complete original 76-line CreateOverwrite probe is restored as an executable standard regression, with only audit04 -> audit05 test-name substitution. Original diagnostic SHA256 `d961cc97bed5db46c476dc1d10ee5036eb2d0c26b52bc867308854178d1d277c` remains frozen. This task's new green closes that defect now; it does not convert the old failure into a past success.

Reviewed the preserved runtime reds for missing Space, form Enter and TimeField/ancestor Enter. Historical intermediate fixture/compiler/shared-tree failures remain reported separately in outcomes, not counted as behavioral reds or attributed as fixes by this task.

## Parent-executed verification

All Rust invocations used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. Commands ran serially after a natural Cargo/rustc idle check; none was killed or duplicated. No Cargo/rustc remained at final check.

| Parent command | Actual result |
| --- | --- |
| cargo test -p nxtools-desktop audit05_ -- --test-threads=1 | **15/15 PASS**, zero ignored, 4.26s test time |
| cargo test -p nxtools-desktop archive:: -- --test-threads=1 | **134/134 PASS**, zero ignored, 31.48s; includes #4 display6 |
| cargo test -p nxtools-desktop downloads:: -- --test-threads=1 | **188/188 PASS**, zero ignored, 44.57s; accepted keyboard/picker regressions included |
| cargo test -p nxtools-desktop history_identity_ -- --test-threads=1 | **11/11 PASS**, 19.03s |
| cargo test -p nxtools-desktop accessible_name_ -- --test-threads=1 | **7/7 PASS**, 0.86s |
| cargo test -p nxtools-desktop sort_keyboard_ -- --test-threads=1 | **3/3 PASS**, 14.34s |
| cargo clippy -p nxtools-desktop --tests -- -D warnings | **PASS**, exit0 |
| cargo check --workspace --tests | **PASS**, exit0 |
| cargo fmt --all -- --check | **PASS**, exit0 |
| Scoped git diff --check, eight existing paths | **PASS**, exit0 |

Ten parent-rerun source/document gates passed: spec-contracts.py, gpui-component-catalog.py, gpui-release-contracts.py, markdown-editor-contracts.py, archive-cli.py, archive-phase-f-shell.py, archive-phase-f-spike.py, download-ui-lifecycle.ps1, clipboard-ui-lifecycle.ps1 and archive-release-chain.ps1 -SourceOnly. They certify their stated source/document boundaries only, not native/release behavior.

The sole complete Desktop run belongs to the **implementer**, not this parent: hash-verified desktop-full-once.log reports **1232/1232 PASS, zero failed/ignored, 258.96s**. Parent did not repeat the whole suite. The current test executable SHA256 also matches the delivery artifact: `58ab9c4f4c2f9266d8c025b7f264a3504ebd76626560f7b3ccc109d78a5dd0ed`. Unrelated shared-tree additions/repairs contribute to the full-suite state and are not credited to #5.

Parent logs/results and their separate hash manifest are under `parent-evidence/`. The delivery evidence directory is unchanged. Parent created only review evidence and this document; it made no production correction.

## Unresolved boundaries and stop

Parent reran the **extra** quick-launch-lifecycle.ps1 gate: still **FAIL, exit1 at line1765**, in the existing elevated-navigation compound. The raw parent log is retained separately. It is not repaired, weakened, or represented as green. Prior bounded diagnosis concerns the release-manifest writer source assertion/delegation; this task does not accept elevated-navigation security or release packaging.

Physical keyboard/Windows IME, real native focus/activation/HWND, mixed-monitor/DPI, UIA/reader, screenshots/visual comparison and user acceptance were **NOT RUN**. No product/PlatformHost/service, Shell/registry, real network/download/archive/picker/clipboard, Release/package/install, commit or push was performed. Temporary synthetic GPUI/bridge/config/SQLite tests are the accepted evidence level only.

The parent stops after registering this single-item limited acceptance. No item eight/backlog dispatch is authorized by this review or by an implementation-session completion notice.
