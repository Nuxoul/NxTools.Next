# B4 — Task operation and visibility

Status: **accepted at source, production TestPlatform and owned HTTP/SQLite/writer/pipe boundaries**. [Coordinator reacceptance](coordinator-acceptance.md) closes all four B4 items after independent repair checks. [Repair evidence](acceptance-repair.md) and the original tables below retain their executed scopes. B5 prompt preparation does not dispatch implementation.

Baseline: `refactor/ui-gpui-kit`, HEAD `962badcbb1fe9b8b96f05fb982312549e173f8c6`, workspace `D:\Code\NxTools.Next-main`. B1/B2/B3 remain accepted at their recorded boundaries. Canonical Archive **Phase E — Format expansion and ZIP editing / in_progress** is unchanged.

## Delivered behavior and owners

- **Default headers:** one Core `DEFAULT_DOWNLOAD_USER_AGENT` and effective Header helper supply configuration, actual editable Settings Textarea and ordinary probe/transfer. Exact default: `User-Agent: Mozilla/5.0 (Windows NT 6.1; Trident/7.0; rv:11.0) like Gecko`. Original TextareaState and direct official Small outlined 重置 retain draft-only Reset and Apply/Cancel/error behavior. The original locked config transaction merges only changed normalized leaves; concurrent untouched values and exact byte limits survive. Accepted request snapshots and explicit task/browser priority remain authoritative. [Header evidence](headers-outcomes.md).
- **REF-08:** original command → EngineTaskState candidate lease/control → retained-request probe/identity proof → scheduler/task recheck → one original SQLite metadata transaction → TaskChanged. Only explicit user pause with no queue eligibility/active transfer/media may replace URL. One valid validator selection governs admission/If-Range/response validation; legal quoted-empty ETag remains strong. Required expected_revision and bounded task-owned freshness reject exact retired envelopes before HTTP; original TaskDetail supplies the token. Context/plan/checkpoints/artifact/queue survive refusal, cancellation and failed commit. The same keyed production window uses the original deferred official Dialog and stable Input. Native close directly signals the existing host watch; registration retirement defers. Current command11/Hello4 retain the original stable pipe and shared caller constants. Actual held-probe draining-pipe/native-close witnesses are recorded in [scoped repair results](acceptance-repair.md). [Original REF-08 evidence](ref08-outcomes.md).
- **REF-09:** original new-download Input/action → checked local single inclusive ascending u64 range → frozen complete requests/exact official Checkbox preview → selected entries once through original BrowserBatchSubmit. Step1, supplied padding, maximum32 checked before materialization. Zero preview HTTP/tasks; original naming/conflict/duplicate policy and ordered partial outcomes remain authoritative. Stable dialog content focus preserves repeat/Escape after confirmation disables. [REF-09 evidence](ref09-outcomes.md).
- **REF-10:** original actual writer count and `(start,length)` maps → original coalesced segment projection/event pump → same TaskWindowView → passive public no-id Plot/PlotElement. Actual completed spans and written prefixes are merged/clamped; allocation is blank, missing fields unknown and unknown total has no ratio. One original open-time TaskDetail read supplies ranges/final-artifact metadata; status-boundary hydration and known persisted prefixes cover pause/reopen/resume before the first nonempty live range event. Current live layout replaces rebalanced IDs without render IO, another timer or scheduler. Official aggregate Progress,440×184/280 budgets, Server/metrics, original speed editor, scrollable table and actions remain. [REF-10 evidence](ref10-outcomes.md).

Original50handoff paths are in [changed-paths.txt](changed-paths.txt); exact30scoped repair paths are in [repair-changed-paths.txt](repair-changed-paths.txt). Applicable owner contracts and verified reference trace are updated. IPC tests reuse already locked workspace Tokio as a dev dependency; no dependency version, C#, native adapter or vendor changes belong to the repair.

## Behavioral regressions

Genuine boundary RED→GREEN evidence is retained in the slice records: real ordinary UA mismatch; missing official Reset; concurrent Cookie/precision/global-limit transaction failures; compatible paused replacement rejection and missing replacement Button; original template action entering ordinary probing; missing actual live start offsets; missing paused-window coverage; persisted prefix hidden before the first resumed live event.

Supplemental first-pass tests are coverage, not additional claimed REDs. API/compiler/fixture/surface prerequisites are not product REDs. The full real parallel-event→production-window chain uses the original event pump, not forged progress, and passes true current ranges, partial written sum, real rebalance, stable window/Input entities, actual bounds, exact final8MiB bytes and worker/listener drain.

## Commands and evidence

All logs/results below are in:

`D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`

Each behavior row first ran a nonzero selection with `-- --list`, then the **identical** selection with `-- --nocapture --test-threads=1`:

```text
cargo test --locked --offline -p PACKAGE TARGET FILTER -- --list
cargo test --locked --offline -p PACKAGE TARGET FILTER -- --nocapture --test-threads=1
```

`TARGET` is `--lib`, `--bin nxtools-desktop`, or `--test NAME` as shown. Empty filter means that named library/target's whole test set. Every row below has LIST exit0/RUN exit0, FAIL0, ignored/SKIP0 and not_run0. LIST/RUN/PASS counts match. Selections overlap and are **not additive distinct-test totals**.

### Final source selections

Driver: `b4-final-checks.ps1`; result: `b4-final-results.json`; driver output: `b4-final-driver.log`. Each log prefix has `-list.log` and `.log`.

| Package / target | Filter | LIST / RUN / PASS | Log prefix |
|---|---|---:|---|
| Desktop / bin | `b4_` |29 /29 /29|`b4-final-ui`|
| Desktop / bin | `audit06_aux_task_details_last_segment_and_real_toggle_cancel_keep_budget` |1 /1 /1|`b4-final-task-scroll`|
| Desktop / bin | `ref06_` |11 /11 /11|`b4-final-ref06`|
| Desktop / bin | `server_info_` |7 /7 /7|`b4-final-server`|
| Desktop / bin | `auxiliary_theme_tests::` |10 /10 /10|`b4-final-theme`|
| Desktop / bin | `task_window_handoff_` |2 /2 /2|`b4-final-handoff`|
| Desktop / bin | `task_window_render_never_reads_the_parent_download_application` |1 /1 /1|`b4-final-render-owner`|
| Desktop / bin | `task_windows_use_compact_content_driven_phase_sizes` |1 /1 /1|`b4-final-phase-size`|
| Desktop / bin | `live_progress_updates_the_cached_selected_segment_without_requerying_sqlite` |1 /1 /1|`b4-final-detail`|
| Download / lib | all |127 /127 /127|`b4-final-download-lib`|
| Download / b4_coverage_contract | `b4_ref10_` |1 /1 /1|`b4-final-coverage`|

The final29 B4 Desktop tests comprise Header8, REF-08 6, REF-09 8 and REF-10 7. They include the new live integration and first-event fallback. The same standalone live test also passes LIST1/RUN1/PASS1 in `b4-ref10-live-window-green`; the fallback passes identical1/1/1 in `b4-ref10-pre-event-green`.

### Related preservation and transport selections

Driver/result: `b4-related-checks.ps1`, `b4-related-results.json`. Log prefix is `b4-related-NAME`.

| Package / target | Filter | LIST / RUN / PASS | NAME |
|---|---|---:|---|
| Desktop / bin | `ref01_` |4 /4 /4|ref01|
| Desktop / bin | `ref04_` |6 /6 /6|ref04|
| Desktop / bin | `ref17_` |8 /8 /8|ref17|
| Desktop / bin | `ref18_` |11 /11 /11|ref18|
| Desktop / bin | `settings_persistence_failure_rolls_back_engine_values` |1 /1 /1|settings-rollback|
| Desktop / bin | `download_settings_save_failure_uses_center_and_keeps_original_draft` |1 /1 /1|settings-failure|
| Core / lib | all |98 /98 /98|core-lib|
| Core / b4_download_settings_contract | `b4_` |2 /2 /2|core-b4|
| Download / b4_request_policy_contract | `b4_` |3 /3 /3|http-b4|
| Download / b4_paused_url_contract | `b4_ref08_` |9 /9 /9|paused-b4|
| Download / message_contract | all |15 /15 /15|message|
| Download / engine_contract | `browser_batch_` |3 /3 /3|batch-owner|
| Download / engine_contract | `dynamic` |3 /3 /3|rebalance-owner|
| Download / engine_contract | `concurrent_ranges_resume_all_persisted_segments_after_engine_restart` |1 /1 /1|parallel-reopen|
| Download / engine_contract | `ranged_task_resumes_persisted_bytes_after_the_engine_is_reopened` |1 /1 /1|range-reopen|
| Download / engine_contract | `download_defaults_merge_at_acceptance_and_explicit_request_context_wins` |1 /1 /1|request-context|
| Download / engine_contract | `changed_resource_after_a_checkpoint_fails_without_publishing_or_discarding_progress` |1 /1 /1|changed-resource|
| Download-IPC / lib | all |7 /7 /7|download-ipc|
| IPC / lib | all |62 /62 /62|ipc|
| Protocol / lib | all |115 /115 /115|protocol|
| CLI / lib | all |15 /15 /15|cli|
| BrowserHost / lib | all |48 /48 /48|browser-host|

The related driver also passed the task-scroll/ref06/server/theme/detail/coverage selections later repeated in the final table. Current real pipe/CLI request correlation, previous Hello rejection, draining replacement refusal and candidate-cleanup availability pass. Cancellation ACK is not physical drain. These checks preserve the accepted owners without reopening B1–B3 implementation.

## Static and compiled checks

Required environment is initialized in the **same PowerShell execution** before every Rust invocation, following a fresh global cargo/rustc/clippy-driver/link/lld-link/rust-lld scan:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
```

- Scoped changed-Rust `rustfmt --check --edition 2024 --config skip_children=true`: exit0, `b4-final-format.log`.
- `cargo clippy --locked --offline --keep-going -p PACKAGE --all-targets -- -D warnings`: Core/Download/Download-IPC exit0; Desktop exit101 solely for the two existing findings below. Logs: `b4-final-clippy-PACKAGE.log`. Strict rules were not weakened.
- `cargo check --locked --offline --keep-going -p nxtools-core -p nxtools-download -p nxtools-download-ipc -p nxtools-ipc -p nxtools-protocol -p nxtools-desktop -p nxtools-cli -p nxtools-browser-host --all-targets`: exit0, `b4-final-check.log`.
- Quality result: `b4-final-quality-results.json`. The final driver exit2 reflects the attributed Desktop lint status, not failed behavior or compilation.
- `git diff --check`: exit0 at the pre-handoff observation.
- Four source/provenance gates use `python tests/architecture/{spec-contracts,gpui-component-catalog,gpui-release-contracts,markdown-editor-contracts}.py`. Final individual exits and full output are recorded in `b4-final-source-results.json` and `b4-final-source-NAME.log`.

## Resource exit

Read-only audit `b4-final-resources.json`, observed **2026-11-10T23:54:47.2542056+08:00**:0compiler/linker processes,0workspace test processes,0B4 wrapper processes and0test-owned TCP connections. Both `nxtools-archive-worker.exe` and `NxTools.ArchiveWorker.exe` were checked and absent. No process was terminated by the audit.

HTTP fixtures assert actual candidate socket/connection-task drain; the new parallel-window test waits for all original fixture connection workers to reach zero, drops/joins the listener, verifies connection refusal and drops the owned Tokio runtime. Temporary config/SQLite/artifact/output fixtures and TestPlatform windows stay under their original owners. No product/reference GUI or installed package was launched.

## Limitations

- **Desktop strict lint is not globally green:** existing `downloads/browser_handoff_impl.rs:436` nonminimal_bool and `downloads/settings_task_rem_tests.rs:612` unwrap_used remain. Both exact expressions exist in baseline HEAD; `b4-baseline-lint-source.txt` records that attribution. They were neither suppressed nor repaired as unrelated work.
- Source, production TestPlatform and owned HTTP/SQLite/writer results do not certify physical Windows IME/DPI/OS-close/Shell interaction, human visual acceptance, installed/Release or whole-workspace behavior. The native-close test covers TestPlatform keyed lifetime, not a held-probe physical OS-close event.
- Independent coordinator reacceptance is complete with verdict **accepted at the stated boundaries**; see [final acceptance](coordinator-acceptance.md).

B4 implementation stops here. B5–B7 remain paused. No commit, push, packaging, installation or system registration change is part of this delivery.
