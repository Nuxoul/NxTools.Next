# Phase D — End-to-end operations and desktop launch intents: independent acceptance

2026-10-22. **Bounded source/isolated implementation checks PASS; overall Phase D NOT accepted.**
One independent full Desktop run remains973passed/6failed. JOB07/SAFE08 source recycle and real
GUI/Shell/DPI/mixed-monitor/UIA/user acceptance are not closed. Only
[checkpoint](phase-a-checkpoint.md) owns current status. [Implementation outcomes](phase-d-operation-outcomes.md)
and earlier A/B/C reports remain unmodified history.

## Review and bounded findings

Reviewed explicit Phase D dependency amendment, contract/outcomes/reference follow-up, component
selection/locked APIs, Desktop draft validation/forms, FIFO/resources, original readmission adapter,
job event/receipt/prompt/terminal ownership, post-create Test and startup/Quick Find ingress. One
bounded optional read-only review checked native job/source/output/preflight/commit boundaries.
No actionable new in-scope defect was found in this review; this is not exhaustive product proof.

Confirmed source/compiled seams:
- Real official Form/Field and stable Input/Select compose capability-validated drafts. Defaults feed
  requests; unsupported hidden fields are rejected/omitted, stream multi-source requires explicit
  separate jobs, whole batch is validated/admitted before pump. No new execution framework.
- Strict FIFO2running/32queued with finite memory/conflicting path admission; original cleanup owns
  release, not progress close/receipt. Pending receipt identity precedes dispatch. Native attempts,
  logical job/session/generation and current ticket are checked before applying results/replies.
- Extract uses original finite output_readmission/InteractionPort; native source revision binding
  precedes selected ordinal mapping. Missing destination/known free-space and create overwrite
  confirmations precede output effects. Unknown sizes remain unknown; approval does not silently
  raise work/native memory budgets. Known-format representative native tests passed, not a full
  format/policy/input cross-product or unbounded compressed-TAR discovery promise.
- Post-create Test is a fresh native attempt in the same logical slot; created commit facts survive
  failed/cancelled Test. Once-only final notification/taskbar cleanup, current Dialog retirement,
  deferred cross-owner work and background/restore have production-owner tests.
- args_os intent parsing/handoff precedes ordinary single-instance return. Bounded same-user/
  SID/session/PID/executable-authenticated Rust listener/readiness/receipt/replay route is present;
  Quick Find ordinary recognized file Open uses same owner and retains other Shell actions.
- Source deletion remains explicitly off/unavailable. No path/permanent-delete substitute or real
  Recycle Bin acceptance is inferred from safe create/extract transactions.

## Independent execution

All Cargo commands offline/locked, jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, one lane checked
before each command. Logs use one unique directory and append-only command records. Full Desktop
ran exactly once; its failure was preserved, not retried. Current C# test apphost built first;
matching isolated-codecs worker rebuilt after final feature/workspace checks before native LIST/RUN.

| Check | Actual result |
| --- | --- |
| Core / IPC / Protocol library tests |70 /38 /63passed,0failed/ignored |
| Desktop archive:: |68passed,0failed/ignored,911filtered |
| Full Desktop, one run |**973passed/6failed**,0ignored/filtered,111.82s,exit101; NOT PASS |
| Archive representative |**60passed**,0failed/ignored:lib20/plan5/interaction4/job2/readmission3/extraction9/wire17 |
| Touched Desktop/Core/Protocol/IPC strict Clippy --bins --tests |PASS,-Dwarnings |
| Archive isolated-codecs all-targets strict Clippy |PASS,-Dwarnings |
| Workspace check --tests / full fmt --check |both PASS; check is compilation, not workspace tests |
| C# test apphost build |0warnings/0errors,exit0 |
| Exact same13native LIST/RUN/PASS names |**selected=run=pass13,skip=fail=not_run0**,exit0 |
| desktop-shell/font16case/UI consistency/token/PlatformHost selection |all PASS,source-only |
| spec/catalog/task/diff |PASS;107docs/14owners,79pages/78locked entries,82implement/79check; context-size warnings retained |

Archive68, startup8 and QuickFind1 passed within the full run and overlap its totals. Native13 are
reviewed GUID-owned small actual workers/Jobs/transactions, not installed application, real picker
input/Shell/source recycling or performance evidence. Same worker before/after native:
`6552BAFBF393448718A52A60B506444F2253BC998CF9ECBEA071580D7E5D34FC`.

## Integrated regression blocker — same6 failures independently reproduced

All fail at vendor/gpui-component-071/src/root.rs:126, missing component window state during paint:
1. clipboard::resize_tests::capture_pause_ack_failure_reconnect_and_inflight_capture_cleanup
2. clipboard::resize_tests::clipboard_resize_ignores_hidden_startup_and_retired_panel_sessions
3. clipboard::resize_tests::clipboard_resize_observer_persists_latest_only_and_reopens_logically_at_125_150
4. clipboard::resize_tests::clipboard_resize_save_failure_keeps_session_size_and_visible_error
5. ui::extension_feedback::tests::explicit_result_location_uses_real_worker_page_and_ignores_closed_origin
6. ui::tests::round2_active_clipboard_preview_does_not_close_on_conflicting_foreground

Inspected clipboard_resize_tests setup/panel: Kit global init is present, but add_window_view mounts
ClipboardPanel directly without a per-window Kit Root. The common panic is real; global init alone
is not an adequate repair. Implementation has a separate prior isolated backtrace; this acceptance
did not rerun any of the6alone or edit unrelated tests/vendor. No baseline-green or archive product
causal regression is established merely by the shared failure. The owning fixture mount needs a
small correct Root/WindowState setup, retained business assertions, and integrated verification
before claiming a full-suite PASS. Do not suppress the panic, skip tests or weaken vendor contracts.

## Acceptance rows and limits

- JOB01–06 and IPC01–05/DIAG01: accept the documented bounded source/isolated evidence only. Scope
  is actual tested representative flow, not every format×policy or physical/native/user acceptance.
- JOB07 remains PARTIAL, SAFE08 unaccepted: original-object recycle is not implemented/accepted in
  this scope; deletion stays unavailable. Recycle failure must never rewrite archive commit truth.
- Real form/picker/password/focus/IME/theme/DPI, cold/protected/silent actual process-window behavior,
  taskbar/notifications/Shell/UIA/user acceptance remain NOT RUN under binding self-app denial.
- Earlier A/B unfinished conditions and C live requirements remain. No E/F, release/package/install,
  registry/service mutation, source deletion, GUI launcher workaround, commit/push occurred.

The implementation's overwritten first Archive19/1 log is an already disclosed historical evidence
limitation; this review neither reconstructs it nor calls it intact. This review's current60PASS
raw log is separately retained. All new command logs remain unique/unoverwritten.

## Preservation and stop

Parent scratch session ceebe351-d540-4ca3-a460-dadc71f3a7df:
`phase-d-independent.ps1`, `phase-d-independent-20261022-082840/{commands.jsonl,per-command logs,
baseline.json,preservation.json,worker-before.json,worker-after.json}`. Actual LIST/RUN/PASS names
and raw suite summaries were compared, not inferred from wrapper exit0.

Before record edits **310/310current task/archive/touched acceptance-baseline files unchanged**.
Implementation's older152historical count is not reasserted as a freshly checked opening baseline.
Acceptance adds this report and updates current checkpoint only; no product/tests/history edits.

**Disposition:** bounded Phase D source/isolated delivery is substantiated, not overall accepted.
Next concrete integration repair is the6direct ClipboardPanel fixture mounts, not another broad
archive review or codec matrix. Preserve source-recycle and live acceptance gaps; do not advance
E/F or declare first-release completion from these results.
