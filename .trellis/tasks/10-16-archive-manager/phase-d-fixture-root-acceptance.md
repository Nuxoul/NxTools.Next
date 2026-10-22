# Phase D — six Desktop fixture Root mounts: independent acceptance

2026-10-22. **Six-fixture integrated-test blocker independently CLOSED.** This does not grant
whole Phase D/A–F, source-recycle or real GUI/native/user acceptance. Only
[checkpoint](phase-a-checkpoint.md) owns current status. Prior [Phase D acceptance](phase-d-operation-acceptance.md)
and [repair outcomes](phase-d-fixture-root-outcomes.md) remain unchanged history.

## Scope review

Compared the actual current three files against the implementation's saved opening snapshots,
not the cumulative dirty repository diff. Ignoring indentation, each delta is exactly one mount
adaptation: create the original ClipboardPanel as an entity, retain that SAME handle through a
local capture, mount one official Kit Base Root around it, then continue original test operations.
Constructor arguments, resize calls, business assertions and production code are unchanged.
The shared resize mount covers four cases; extension-result and foreground-preview mounts cover
one case each. No duplicate Root, replacement business view, vendor patch, ignore, panic catch,
extra initialization or weakened assertion was found. No new in-scope defect or further review.

## Independent execution

Offline/locked serial Cargo with CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256; lane checked before every command. Unique per-command logs and
append-only records, no full-suite rerun or log overwrite.

Six full names were each executed with --exact --test-threads=1 and independently checked for
actual named test output plus1passed/0failed/0ignored,978filtered:
1. clipboard::resize_tests::capture_pause_ack_failure_reconnect_and_inflight_capture_cleanup
2. clipboard::resize_tests::clipboard_resize_ignores_hidden_startup_and_retired_panel_sessions
3. clipboard::resize_tests::clipboard_resize_observer_persists_latest_only_and_reopens_logically_at_125_150
4. clipboard::resize_tests::clipboard_resize_save_failure_keeps_session_size_and_visible_error
5. ui::extension_feedback::tests::explicit_result_location_uses_real_worker_page_and_ignores_closed_origin
6. ui::tests::round2_active_clipboard_preview_does_not_close_on_conflicting_foreground

| Check | Actual independent result |
| --- | --- |
| Six exact invocations |6/6PASS, each actual1test, no failure/ignore |
| ONE full Desktop suite |**979passed/0failed/0ignored/0filtered**,114.16s,exit0 |
| Desktop strict Clippy --tests -Dwarnings |PASS |
| Full fmt --check |PASS |
| clipboard-lifecycle / clipboard-ui-lifecycle / ui-state-lifecycle / desktop-shell-lifecycle |all PASS,source-only |
| spec-contracts / component catalog / task validate / scoped diff |all PASS;107docs/14owners,79pages/78locked entries,82implement/79check |

Six targeted cases overlap979 and are not added to that total. Historical973/6 remains a genuine
prior failed observation; current979/979 is a distinct successful run after the mount correction.
No Core/IPC/Protocol full library, Archive representative, native13, codec worker or performance
matrix was rerun. Existing native-contracts35497/archive-reference41319byte context warnings remain.

## Preservation / evidence / disposition

Parent scratch ceebe351-d540-4ca3-a460-dadc71f3a7df:
fixture-root-independent.ps1 and fixture-root-independent-20261022-091516/{commands.jsonl,
exact-1..6.log,desktop-full.log,other gate logs,baseline.json,preservation.json}.
Before records, **161/161current task/three-source acceptance-baseline files unchanged**.
Implementation's older157historical count is not claimed as a fresh older-baseline check.
This acceptance only adds this report and updates current checkpoint; implementation/tests/history
were not edited. No GUI/Shell/product launch, package/install/system mutation/source recycle,
dependency/vendor edits, commit/push or E/F work occurred.

**Disposition:** accept this minimal fixture repair and close the six-test integrated blocker.
Do not reopen cache/font/archive source review or invent another cleanup task. JOB07 remains
partial and SAFE08/source recycling unaccepted; source deletion remains unavailable. Actual
GUI/Shell/DPI/mixed-monitor/UIA/user acceptance remains NOT RUN under binding self-app denial.
Earlier A/B/C outstanding obligations are unchanged. No whole-phase PASS or automatic E/F advance.
