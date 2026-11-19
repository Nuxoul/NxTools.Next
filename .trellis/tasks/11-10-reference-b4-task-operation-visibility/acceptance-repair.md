# B4 acceptance repair

Status: **accepted at source, production TestPlatform and owned HTTP/SQLite/writer/pipe boundaries**. [Coordinator reacceptance](coordinator-acceptance.md) closes B4-01/B4-02/B4-E1/B4-E2; implementation commands and results below retain their original scope.

Scope: **B4-01, B4-02, B4-E1 and B4-E2 only**. [Coordinator acceptance](coordinator-acceptance.md) remains authoritative for independent acceptance. The coordinator report and `docs/reference-improvement-coordination.md` are read-only. B1/B2/B3, reviewed Header/reset, REF-09, REF-10 and canonical Archive **Phase E — Format expansion and ZIP editing / in_progress** remain unchanged. B5–B7 remain paused.

Baseline: `D:\Code\NxTools.Next-main`, `refactor/ui-gpui-kit`, HEAD `962badcbb1fe9b8b96f05fb982312549e173f8c6`.

## Product boundaries

### B4-01 — One effective resume validator

`ResourceValidator::if_range` is the shared selection boundary for replacement admission, actual If-Range and response validation. It selects a valid quoted strong ETag, including legal `""`, otherwise a valid Last-Modified. Missing, blank, whitespace-only and weak ETag cannot override the valid date. Exact original validator equality, known equal total and Range compatibility still gate replacement. The original segmented-runner automatic recovery strategy and unrelated Headers are unchanged.

The real HTTP/SQLite/writer regression covers blank/date, whitespace/date, weak/date and quoted-empty strong ETag. It writes a partial artifact, explicitly pauses, accepts replacement and resumes. The candidate server honors the retained suffix only for the expected actual If-Range; wrong values return200. Assertions require the original plan/checkpoints/prefix after replacement, exactly one payload transfer from `paused.downloaded_bytes` rather than zero, original task ID, Completed and exact16-byte output. Original strong/missing/weak/changed/Last-Modified-only/refusal cases remain.

### B4-02 — Bounded task-owned freshness

The existing `PausedUrlReplacement` owns one checked u64 revision. Current revision is0 before any acquired attempt. Required `expected_revision` must match at admission; only an acquired candidate consumes the next revision before HTTP. Terminal retirement retains it, and exhaustion rejects rather than wraps. Existing active/last-ID/source/generation/status checks and cancellation ownership remain. No UUID history, persistent deduplication database, new manager or schema is introduced.

Original `DownloadTaskDetail.paused_url_revision` and read-only `DownloadEngine::paused_url_replacement_revision` supply the token. Desktop reads it at each explicit confirmation outside render, retaining the same Input/error/retry transaction. The original command/pipe use **command11 / Hello4** and shared caller constants; previous Hello is explicitly rejected on the same stable pipe. Request freshness is required, not a serde-default bypass. Ordinary full/compact task snapshots are not widened. Two existing TaskDetail fixture literals receive revision0. IPC tests reuse the already locked workspace Tokio as a dev dependency; no dependency version or production dependency is upgraded.

The public-command regression freezes the complete A envelope, cancels it while its actual probe is held and waits for terminal/socket drain. B fails. Replaying exact A must reject before HTTP, with request count and original task/probe/URL/plan/checkpoints/artifact/queue unchanged. Fresh C accepts, retains original segments and resumes exact bytes. Immediate/active duplicates, concurrent resume, stale generation, rollback and cancellation owners remain covered.

## Lifecycle witnesses

### B4-E1 — Original draining host and pipe

`b4_e1_draining_pipe_cancels_held_candidate_and_preserves_written_task` uses actual SQLite/partial writer and the original `run_download_host`, typed frames, stable correlations and concurrent original clients. TaskDetail supplies freshness through that pipe. One candidate remains physically held at HTTP while the host enters Draining. A new replacement is Unavailable without HTTP. Exact Cancel is Accepted with its own correlation; the separate candidate result is Cancelled with the original correlation.

The test then joins the candidate caller, waits for held sockets to reach zero and sends exact Cancel again: InvalidState proves the original lease retired, not merely that ACK arrived. Full original task/probe/downloaded count/temp path, durable task/full plan, artifact bytes and explicit-user-paused/not-eligible queue facts remain. Clients are dropped and `connected_clients==0` is observed. The HTTP listener/connection task join completes and a new TCP connection is refused.

The original perpetual accept thread is scoped to its test executable's lifetime, not explicitly joined. The final audit observes that owning Download-IPC executable and its original pipe-client owners exited.

### B4-E2 — Original production native-close callback

`b4_e2_native_close_retires_held_probe_without_changing_reopened_task` uses the original production Root/task window, official Button/Input/Confirm and actual held HTTP probe. Public `Window::dispatch_event` supplies the pointer events, and the public TestDispatcher moves the sole original background task to an owned thread so the TestPlatform UI can close while synchronous probe I/O is held. No direct business submission or vendor change substitutes for the production callback.

The original view stays strongly owned, so cancellation is not inferred from view destruction. `simulate_close()` invokes original `on_window_should_close`; native removal is modelled separately because TestPlatform does not remove the GPUI window itself. The original candidate worker is joined while its foreground result remains queued. Same-task reopen creates a different window/view before that result is dispatched; processing it cannot change the current owner or install an old draft. Actual held sockets drain. A fresh official confirmation reaches the actual failing endpoint with revision2, proving the old lease retired, and retains its own Input/error. An old-window close notification cannot retire that current owner. Original task/probe/URL/bytes/temp path, durable task/full plan, artifact and queue facts remain. Owned HTTP listener/connection tasks exit and a new connection is refused.

This is controlled TestPlatform/HTTP lifecycle evidence, not physical Windows OS-close, IME/DPI or human visual acceptance.

## RED/GREEN and supplemental evidence

All evidence is retained under `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`. Each prefix below has `-list.log`, `.log` and `-results.json` unless classified as a prerequisite. Selections overlap and are not additive test totals.

| Selection / log prefix | LIST / RUN / PASS / FAIL | SKIP / not_run | Classification |
|---|---:|---:|---|
| `b4-repair01-red` |1 /1 /0 /1|0 /0|Actual If-Range was empty instead of accepted Last-Modified|
| `b4-repair01-green` |1 /1 /1 /0|0 /0|Identical behavioral selection; all four validator combinations|
| `b4-repair01-preserve` |9 /9 /9 /0|0 /0|Original REF-08 selection|
| `b4-repair02-red` |1 /1 /0 /1|0 /0|Exact retired A accepted after B|
| `b4-repair02-green` |1 /1 /1 /0|0 /0|Identical public-command selection; no replay HTTP, fresh C exact output|
| `b4-repair-e1` |1 /1 /1 /0|0 /0|First-pass actual draining/pipe lifecycle coverage|
| `b4-repair-e2-native` |1 /1 /1 /0|0 /0|Actual held production callback lifecycle coverage|
| `b4-repair-e2-late` |1 /1 /1 /0|0 /0|New window exists before queued old result is dispatched|
| `b4-repair-final-ui-late` |30 /30 /30 /0|0 /0|Final B4 UI preservation with that result-order assertion|

E2's earlier `b4-repair-e2` run did not establish a held probe (LIST1/RUN1/PASS0/FAIL1), and `b4-repair-e2-witness-list.log` records a downstream-private API compile prerequisite (no behavior RUN). They are not product REDs or additional lifecycle evidence. The valid witness uses public dispatch/scheduler APIs. No product cancellation change was needed for E1/E2.

Targeted driver: `b4-run-rust.ps1`. Exact test filters are `b4_repair01_validator_selected_for_replacement_is_used_by_actual_resume`, `b4_repair02_retired_attempt_cannot_replay_after_another_failure`, `b4_e1_draining_pipe_cancels_held_candidate_and_preserves_written_task` and `b4_e2_native_close_retires_held_probe_without_changing_reopened_task`. Download regressions use `--test b4_paused_url_contract`, E1 uses Download-IPC `--lib`, E2 uses Desktop `--bin nxtools-desktop`.

## Final behavioral verification

Driver: `b4-repair-final-checks.ps1 -Phase tests`; exact selection arrays, logs, exits and counts: `b4-repair-final-tests.json`; driver log: `b4-repair-final-tests-driver.log`. Final E2/UI evidence uses `b4-repair-final-e2.ps1`, `b4-repair-e2-late-results.json` and `b4-repair-final-ui-late-results.json` with matching LIST/RUN logs. These repeat the same filters with the explicit queued-old-result assertion and are not additive totals. Every row first ran a nonzero LIST and then the identical RUN:

```text
cargo test --locked --offline -p PACKAGE TARGET FILTER -- --list
cargo test --locked --offline -p PACKAGE TARGET FILTER -- --nocapture --test-threads=1
```

All30selections have LIST exit0/RUN exit0, matching LIST/RUN/PASS and FAIL0/SKIP0/not_run0. Log prefix is `b4-repair-final-NAME` with `-list.log` and `.log`. Empty filter means the named target's whole test set.

| Package / target | Filter | LIST / RUN / PASS | NAME |
|---|---|---:|---|
| Download / b4_paused_url_contract | all |11 /11 /11|paused|
| Download / message_contract | all |16 /16 /16|messages|
| Download-IPC / lib | all |8 /8 /8|download-ipc|
| Desktop / bin | `b4_` |30 /30 /30|ui|
| Download / lib | all |127 /127 /127|download-lib|
| Download / http_client_contract | all |26 /26 /26|http-client|
| Download / b4_request_policy_contract | `b4_` |3 /3 /3|headers-http|
| Download / b4_coverage_contract | `b4_` |1 /1 /1|coverage-http|
| Core / b4_download_settings_contract | `b4_` |2 /2 /2|core-settings|
| Core / lib | all |98 /98 /98|core-lib|
| Desktop / bin | `ref06_` |11 /11 /11|ref06|
| Desktop / bin | `server_info_` |7 /7 /7|server|
| Desktop / bin | `auxiliary_theme_tests::` |10 /10 /10|theme|
| Desktop / bin | `audit06_aux_task_details_last_segment_and_real_toggle_cancel_keep_budget` |1 /1 /1|task-scroll|
| Desktop / bin | `task_window_handoff_` |2 /2 /2|handoff|
| Desktop / bin | `task_window_render_never_reads_the_parent_download_application` |1 /1 /1|render-owner|
| Desktop / bin | `task_windows_use_compact_content_driven_phase_sizes` |1 /1 /1|phase-size|
| Desktop / bin | `live_progress_updates_the_cached_selected_segment_without_requerying_sqlite` |1 /1 /1|detail|
| Desktop / bin | `settings_persistence_failure_rolls_back_engine_values` |1 /1 /1|settings-rollback|
| Desktop / bin | `download_settings_save_failure_uses_center_and_keeps_original_draft` |1 /1 /1|settings-failure|
| Download / engine_contract | `browser_batch_` |3 /3 /3|batch|
| Download / engine_contract | `dynamic` |3 /3 /3|rebalance|
| Download / engine_contract | `concurrent_ranges_resume_all_persisted_segments_after_engine_restart` |1 /1 /1|parallel-reopen|
| Download / engine_contract | `ranged_task_resumes_persisted_bytes_after_the_engine_is_reopened` |1 /1 /1|range-reopen|
| Download / engine_contract | `download_defaults_merge_at_acceptance_and_explicit_request_context_wins` |1 /1 /1|request-context|
| Download / engine_contract | `changed_resource_after_a_checkpoint_fails_without_publishing_or_discarding_progress` |1 /1 /1|changed-resource|
| IPC / lib | all |62 /62 /62|ipc|
| Protocol / lib | all |115 /115 /115|protocol|
| CLI / lib | all |15 /15 /15|cli|
| BrowserHost / lib | all |48 /48 /48|browser-host|

The30B4 UI tests retain Header8, REF-08 6, REF-09 8 and REF-10 7 plus E2. The11backend replacement tests retain the original9 plus the two product regressions. The8IPC tests retain the original7 plus E1. Default Trident UA text, same-Textarea draft Reset, original Header transaction, templates, actual range coverage, exact non-KiB rates, Server, controls and original window geometry remain covered.

## Compiled/static verification

Every Rust invocation scans the global compiler/linker lane and initializes in the same PowerShell process:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
```

Driver: `b4-repair-final-checks.ps1 -Phase quality`; result: `b4-repair-final-quality.json`. Scoped changed-Rust `rustfmt --check --edition 2024 --config skip_children=true` passed. `cargo clippy --locked --offline --keep-going -p PACKAGE --all-targets -- -D warnings` passed Core/Download/Download-IPC. Desktop exit101 contains only the two HEAD baseline findings: `downloads/browser_handoff_impl.rs:436 nonminimal_bool` and `downloads/settings_task_rem_tests.rs:612 unwrap_used`; no suppression or unrelated fix. Logs use `b4-repair-final-clippy-PACKAGE.log`; original source attribution remains in `b4-baseline-lint-source.txt`. The quality driver exit2 reflects these baselines, not behavioral or compile failure.

`cargo check --locked --offline --keep-going -p nxtools-core -p nxtools-download -p nxtools-download-ipc -p nxtools-ipc -p nxtools-protocol -p nxtools-desktop -p nxtools-cli -p nxtools-browser-host --all-targets` passed. After the E2 order assertion, its scoped format, Desktop strict lint and the same eight-package check were repeated: format0/check0, exactly the same two Desktop lint baselines. Final logs: `b4-repair-final-e2-{format,clippy,check}.log`; result: `b4-repair-final-e2-quality.json`; owned driver exit0.

Four source gates passed, each exit0: `python tests/architecture/spec-contracts.py` (129documents/14owners), `gpui-component-catalog.py`, `gpui-release-contracts.py` (Kit0.8/pre0.3.9/exactly two approved vendor deltas) and `markdown-editor-contracts.py`. Logs: `b4-repair-source-NAME.log`; result: `b4-repair-source-results.json`.

## Paths, resources and remaining acceptance

Exact30repair paths: [repair-changed-paths.txt](repair-changed-paths.txt). Original50path handoff remains in [changed-paths.txt](changed-paths.txt); coordinator additions are not implementation-owned paths. Direct byte comparison, task/context JSON validation, exact path union and diff-check results are recorded in `b4-repair-record-check.json`.

Actual exit audit: `b4-repair-final-resources.json`, observed **2026-11-11T09:22:14.2895887+08:00**, exit0. Compiler/linker, workspace test executables, repair wrappers, test-owned TCP and both `nxtools-archive-worker.exe` / `NxTools.ArchiveWorker.exe` counts are0. E1/E2 runtime assertions separately prove candidate/socket/lease/listener/caller-worker cleanup. No unrelated process was terminated.

Independent coordinator reacceptance is complete with verdict **accepted at the stated boundaries**. Physical Windows OS-close/IME/DPI/Shell, human visual, installed/Release and whole-workspace acceptance remain separate. No product/reference GUI launch, package/install/system mutation, commit or push is included. B4 implementation is complete; providing a B5 prompt does not start that batch.
