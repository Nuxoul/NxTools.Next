# B5 repair verification

Status: **accepted at the [coordinator's stated boundaries](coordinator-acceptance.md)**. B5-01/B5-02/B5-03 are closed. Original repair runs below retain their scope; independent33selection reacceptance passes with B5 UI26. B1–B5 acceptance and canonical Archive **Phase E — Format expansion and ZIP editing / in_progress** retain their boundaries.

## Outcome and original ownership

- B5-01: the original scheduler gate retains one destructive source-membership cutoff per queue, with session-local task accepted_revision. The original snapshot and QueueRound reject coalesced accept/remove and out/back without a mutation journal. Genuine later acceptance remains usable. Initial nonterminal membership establishes the coordinator baseline; first dispatch advances its existing attempt 0→1 only with unchanged queue/cutoff, never a new acceptance revision.
- B5-02: the original task command/domain owner rejects Completed/Failed/Cancelled intention changes before persistence, memory or events. An already committed success cannot be armed after the fact. Nonterminal intention persistence, rollback and close/reopen remain usable.
- B5-03: durable newly accepted retry publishes the original TaskChanged even without a worker. The authoritative completion snapshot projects eligible Failed retry as Waiting, preserving raw scheduler state and stopped admission/checkpoint/artifact/idempotency. The original pump cancels the old countdown promptly; original queue start and real success authorize the same new attempt once. Old execute/cancel callbacks cannot alter the new state.

The owner chain remains DownloadEngine/task/queue → original SQLite commit → original bounded DownloadEvent/pump → CompletionActionCoordinator/countdown → typed platform boundary. Rust owns domain and coordination, GPUI presentation, C# Windows effects. No second coordinator/countdown/scheduler, persistence store, sampler, transport, schema, protocol, dependency, vendor or layout change was added in this repair.

## Behavioral evidence

The four required regressions use the original persisted DownloadApplication, public commands, actual HTTP/writer publication and SQLite. TestApp observation is held while independent workers continue; last_event_sequence remains unchanged through the target mutation interval. Resuming the original production pump is the observation boundary, not per-mutation coordinator calls or a manual refresh.

The original four-case RED was LIST4/RUN4/PASS0/FAIL4; the final four-case run is LIST4/RUN4/PASS4/FAIL0, both SKIP0/not_run0. B5-03 separately exposed Failed instead of Waiting after its missing-event fix. Exact stage counts, assertions and test names are in [acceptance-repair.md](acceptance-repair.md); raw LIST/RUN evidence is retained in [repair-verification.json](repair-verification.json).

Later legitimate rounds execute one typed fake Sleep request, retain correlation, consume authority before dispatch, preserve controlled native failure50 and reject duplicate/late callbacks. Supplementary positive coverage is not represented as an invented RED. Existing execute-time recheck already prevented old power dispatch; B5-03 repairs notification and future-attempt eligibility.

## Final preservation selections

All **33 selections** have nonzero LIST followed by the identical package/target/filter serial RUN. For each count below, LIST = RUN = PASS, FAIL/SKIP/not_run = 0 and LIST/RUN exits = 0/0. Selections overlap; counts are not a unique cumulative total.

| Original boundary | Count |
| --- | --- |
| Desktop B5, including four new pump regressions |26|
| Original completion coordinator / Shutdown |5 /12|
| B4 production UI / REF06 / Server / theme |30 /11 /7 /10|
| Window handoff / render owner / phase size / cached detail |2 /1 /1 /1|
| Settings rollback / save failure |1 /1|
| Download completion / library / domain / messages / engine |9 /128 /12 /16 /105|
| Paused URL / HTTP client / Header policy / coverage |11 /26 /3 /1|
| Rate limit / runtime speed cap |2 /8|
| Download-IPC / IPC / Protocol |9 /62 /116|
| Core library / settings |98 /2|
| CLI / BrowserHost |15 /48|
| Archive draft / Job |12 /129|

The shared HTTP fixture's recovery control stays inside its existing HTTP handler; default failure behavior remains503. Its importing Desktop B5/B4, Download paused and Download-IPC selections were rerun, together with the completion contract. The final records use their latest passing runs. Product code was unchanged during that fixture-only refresh.

Archive Job129 reads the retained genuine B3 progress wire and ordinary.zip/split.zip/split.7z exports under `../11-10-reference-b3-split-file-usability/evidence/reaccept-job-exports`. No new native export or archive worker was started. Original delivery's122/7 record remains unchanged historical evidence.

## Format, compile and source gates

- Scoped rustfmt check: exit0.
- Eight-package all-target cargo check: exit0.
- Strict all-target Clippy: Core, Download, Download-IPC, IPC, CLI and BrowserHost exit0.
- Protocol/Desktop strict Clippy: exit101 for the three historical diagnostics listed below. Quality wrapper exit2 reflects these retained strict failures, not a globally green lint result.
- `spec-contracts.py`: pass,129 documents/14 owners.
- `gpui-component-catalog.py`: pass.
- `gpui-release-contracts.py`: pass, Kit0.8.0/default pre0.3.9 and exactly two approved narrow patches.
- `markdown-editor-contracts.py`: pass.
- `download-ui-lifecycle.ps1`: pass.

Exact command arrays, logs and exits are in [repair-verification.json](repair-verification.json). Every Rust execution used one global lane, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, and session-scratch TEMP/TMP in the same execution. No unrelated process was stopped.

## Resource retirement

Successful repair fixtures await actual engine workers, HTTP connection drain and listener close. Current B5 tests retain the original normal runtime cleanup order: engine futures → Download IPC stop/client/caller/host joins, zero clients/Stopped and post-stop refusal. A quit request is not physical process exit.

The read-only scan at **2026-11-11T16:37:48+08:00** found zero global compiler/linker, workspace test or repair-wrapper processes, archive workers, test-owned TCP connections and B5 completion pipes. The exact scan is retained in [resource evidence](repair-evidence/b5-repair-final-resources.json).

## Repair paths and attribution

Product/test paths changed relative to the repair-start bytes:

```text
rust/crates/nxtools-download/src/engine.rs
rust/crates/nxtools-download/src/engine/queue.rs
rust/crates/nxtools-download/src/engine/browser.rs
rust/crates/nxtools-download/src/storage.rs
rust/crates/nxtools-download/tests/support/b4_url.rs
rust/apps/nxtools-desktop/src/completion_actions.rs
rust/apps/nxtools-desktop/src/completion_actions_b5_tests.rs
rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs
```

Source-backed contracts are updated in the existing Download quality guideline, Desktop UI experience contract and `docs/reference-code/download.md`. Original implementation's62-path manifest and outcomes remain history. The exact repair-only inventory, including records and136 retained evidence files, is [repair-changed-paths.txt](repair-changed-paths.txt). Before-relative attribution, untouched tracked differences, protected coordinator bytes, JSON/JSONL, diff and branch/HEAD results are in [repair-audit.json](repair-audit.json).

## Limitations and independent acceptance

- Strict lint is not globally green: `rust/crates/nxtools-protocol/src/archive_browser.rs:563` unnecessary_map_or; `rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs:436` nonminimal_bool; `rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:614` unwrap_used. All three were re-observed; the settings expression is byte-equivalent to the repair-start source, and the other owner paths were not changed by this repair. No suppression or unrelated repair was applied.
- The initial LIST memory prerequisite failed with Windows1455 and RUN0; it is not behavioral RED. User released memory before the real regressions ran.
- C# source and native adapters were unchanged; no C# fixture was rerun in this repair. Current Rust selections cover typed action dispatch, native pending slot, suspend wait/disconnect-unconfirmed and actual Download joins.
- Physical Shutdown/Sleep/Hibernate/current-user Desktop Exit, OS-close/IME/DPI/Shell, human visual, installed/Release and full-workspace acceptance remain separate. No real power request, current Desktop termination, GUI launch, package/install/system/service/registry change, commit or push was performed.

Independent B5 reacceptance is complete with verdict accepted at the stated boundaries. Next-task prompt preparation includes user-added PlatformHost UIA memory root repair before B6 experiments; this handoff does not dispatch implementation.
