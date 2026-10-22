# Phase D — six Desktop fixture Root mounts: scoped repair outcomes

2026-10-22. **The six-fixture integrated-test blocker is closed in this implementation run.**
This is not overall Phase D acceptance. JOB07/SAFE08 source recycling and real GUI/Shell/DPI/
mixed-monitor/UIA/user acceptance remain unclosed/NOT RUN. Stop at independent acceptance;
no E/F or new archive implementation.

## Minimal change

The prior [independent acceptance](phase-d-operation-acceptance.md) and
[implementation outcomes](phase-d-operation-outcomes.md) retain the973PASS/6FAIL red evidence.
Global Kit init already existed; the failing direct `add_window_view(ClipboardPanel)` lacked
per-window WindowState/Root. No repeat full-suite red run was needed.

Inspected production `clipboard.rs` popup construction (`cx.new(ClipboardPanel::new)` then
`gpui_kit::base::Root::new`) and existing correct UI/efficiency fixtures. Three mount points changed:
- `rust/apps/nxtools-desktop/src/clipboard_resize_tests.rs::panel`: captures the same production
  ClipboardPanel entity, wraps it once in official Base Root, then retains that entity for the
  original resize observers/application popup lifecycle. This shared mount covers the first4tests.
- `rust/apps/nxtools-desktop/src/ui/extension_feedback.rs::tests`: same one-Root/entity-capture
  pattern in the explicit-result-location test; production code above the test module unchanged.
- `rust/apps/nxtools-desktop/src/ui/tests.rs`: same pattern in active-preview foreground test.

No replacement business view, nested Root, extra init, fixed-size workaround, panic suppression,
ignore, weaker assertion, retry/framework or vendor edit. Original source parameters, resize,
configuration persistence/error, cancellation/disconnect, extension location and foreground
assertions remain. No production archive/queue/protocol/native lifecycle behavior changed.

## Verification, in requested order

Every Cargo command checked the shared Cargo/rustc lane and used jobs1/incremental0/
RUSTFLAGS=-C codegen-units=256 plus offline/locked where applicable. No other build was terminated.
Each command has a distinct raw log and command record. No failures occurred in this repair run.

### Six exact targeted invocations

`cargo test --offline --locked -p nxtools-desktop <FULL_NAME> -- --exact --test-threads=1`
ran each of the following separately. Every invocation actually ran **1PASS/0FAIL/0ignored**,
978filtered; selected name and actual output were compared, not inferred from exit0.

| Full test name | Actual result |
| --- | --- |
| clipboard::resize_tests::capture_pause_ack_failure_reconnect_and_inflight_capture_cleanup |1/1PASS |
| clipboard::resize_tests::clipboard_resize_ignores_hidden_startup_and_retired_panel_sessions |1/1PASS |
| clipboard::resize_tests::clipboard_resize_observer_persists_latest_only_and_reopens_logically_at_125_150 |1/1PASS |
| clipboard::resize_tests::clipboard_resize_save_failure_keeps_session_size_and_visible_error |1/1PASS |
| ui::extension_feedback::tests::explicit_result_location_uses_real_worker_page_and_ignores_closed_origin |1/1PASS |
| ui::tests::round2_active_clipboard_preview_does_not_close_on_conflicting_foreground |1/1PASS |

No new business assertion failed after correcting the mount. Source then remained unchanged.

### One full Desktop run and gates

- ONE `cargo test --offline --locked -p nxtools-desktop -- --test-threads=1`:
  **979PASS/0FAIL/0ignored/0filtered**,exit0; test time114.89s, command elapsed116.37s.
  This is the only full-suite run in this repair turn, not a rewrite/retry of historical973/6.
- `cargo clippy --offline --locked -p nxtools-desktop --tests -- -D warnings`:exit0.
- `cargo fmt --all -- --check`:exit0; initial rustfmt touched only the3targetfiles.
- Source-only lifecycle gates: clipboard-lifecycle, clipboard-ui-lifecycle, ui-state-lifecycle,
  desktop-shell-lifecycle all exit0. They are not live GUI/native acceptance.
- Final document gates and baseline-relative scoped diff are recorded in the final addendum.

No Core/IPC/Protocol full-library rerun, Archive representative/codec matrix, C#13fixtures or
worker build. No cache/font independent acceptance reopened. No GUI/Shell/product launch,
system mutation, package/install, source recycle, dependency upgrade, commit or push.

## Evidence and preservation

Local evidence directory:
`C:\Users\Administrator\.pi-desktop\scratch\163f435b-1ee5-4c07-8102-31a2c7441b9b\phase-d-fixture-root-20261022-085603-f78cd184`.
- `00-status-before.txt`, `00-task-baseline.json`,3`before-*.rs` snapshots.
- `02-targeted-1..6.log`, `02-targeted-results.json`: exact six commands/counts.
- `03-diff-*.rs.txt`: this turn's changes relative to opening snapshots, not the heavily dirty HEAD.
- `04-desktop-full.log`/result.json, `05-desktop-strict-tests.*`, `06-fmt-check.*`.
- `07-*-lifecycle*.log`/results, `commands.jsonl`, `gate-commands.jsonl`.
- Before document edits: **157/157opening task files unchanged**. Historical973/6 logs/reports,
  independent acceptance and all Phase A/B/C evidence remain untouched. Only this new record and
  the sole current checkpoint are task-document changes; baseline-scoped checks follow below.

**Remaining blockers:** none for this six-fixture mount repair. Independent confirmation of this
repair is separate. Overall Phase D is not newly passed; source recycling and all previously
unrun real/native/user rows retain their original status. No later phase starts automatically.

### Final scope/document checks
`09-scope-results.json`: normalized before/after source equals exactly the3expected mount
transformations, constructor arguments unchanged; assertions27/8/183 remain27/8/183. Production
extension_feedback prefix and all unrelated UI tests are byte-identical to this turn's snapshots.
`10-document-results.json`: spec-contracts, component-catalog and task-context all exit0; known
native-contracts/archive reference context-size warnings remain, not new failures.
`11-history-preservation.json`:156/157opening task files byte-identical, only current checkpoint
changed. Prior outcomes/independent acceptance remain unchanged. Scoped diff-check exit0.
No source edits occurred after targeted/full-suite verification. This round's logs are unique
and unoverwritten; no optional review or additional full-suite run was started.
