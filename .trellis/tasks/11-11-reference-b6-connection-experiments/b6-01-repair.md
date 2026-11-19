# B6-01 — Terminal refusal observation

Status: **accepted at the coordinator's stated boundaries**. B6-01 is closed after identical-test/fixture RED→GREEN and independent37group/842execution reacceptance, FAIL/SKIP/not_run0. [Coordinator verdict](coordinator-acceptance.md) records the current result and limits.

## Original owner and change

`rust/crates/nxtools-download/src/engine/segmented_runner.rs::spawn_segment_transfer` offers each `Err(error)` exactly once to `connection_ceiling::observe_refusal` before `should_retry_transfer`. A nonretryable/exhausted failure immediately returns the same `Err(error)`. The existing learned transition diagnostic remains at the single observer call.

The production change replaces the retry-qualified match arm with an unconditional error arm and adds the terminal retry-eligibility check after observation. Maximum attempts remains3. Retry delay, error/task precedence, actual stop/join/writer sync/checkpoint ordering, same-run Arc,256entry cache,600second maximum validity, captured route/policy/TLS/System-client isolation and lower caps are unchanged. No controller, setting, protocol, dependency or production log is added.

Reference order: ABDM `PartDownloader.kt::start/canRetry/download` and `HttpDownloadJob.kt::beginDownloadParts` first; legacy `DownloadManagerService.cs::IsConnectionPressureException/SegmentConnectionLimiter` second. ABDM keeps terminal failure and physical connection/writer retirement with the part owner. The reference chains and frozen REF-13 decision remain in `reference-review.md`, `docs/reference-code/download.md` and `ref13-outcomes.md`.

## Regression boundary

- `src/engine/connection_experiments/b601.rs::terminal_refusal_after_http_503_learns_without_an_extra_attempt` uses the original captured Direct HTTP client, real HEAD/Range probe, SQLite queued identity, original segment workers and segmented artifact. Paths here are relative to `rust/crates/nxtools-download/`.
- `tests/support/b601_server.py` owns one listener and exactly one receiving peer. The target worker receives HTTP503on attempt1; the fixture closes only its listener, leaving the accepted peer socket sending. Attempts2and3therefore fail through the original connector with genuine pre-header native ConnectionRefused. No synthetic IO error or manual observer call supplies evidence.
- The original test resolver gates scheduling before each refused connection. After503the effective ceiling query remains4; after one typed refusal it still remains4. This detects status misclassification and duplicate observation before the terminal second refusal is allowed.
- The target returns its original native failure before the peer is released to complete. The test verifies the target range and zero target bytes, three target attempts, exactly two typed native refusals, six total original resolutions/connections (two probes, one peer, three target attempts), four accepted HTTP connections/requests and no fourth target attempt.
- Both actual worker joins, exact peer bytes, released worker observation ownership, absent output publication, engine shutdown, server socket/thread/listener exit, child process wait and reader join precede the final learning assertion. The table and test retain the only two observation Arcs after worker exit.

This is the approved original HTTP/segment-worker seam. The target worker's typed terminal result is asserted directly; public task failure, durable ordering, pause/cancel and completion ownership remain covered by the original engine/B4/B5 preservation selections. No GUI layout, placement, focus or native-window lifecycle changes.

## Genuine RED and identical-assertion GREEN

Both runs use the same test and fixture bytes. RED production bytes match this repair's before-copy. GREEN changes only the production observation/retry ordering.

| Fact | RED | GREEN |
| --- | ---: | ---: |
| Nonzero LIST / actual RUN |1/1|1/1|
| PASS / FAIL / ignored |0/1/0|1/0/0|
| Cargo RUN exit |101|0|
| Target attempts |3|3|
| Native refusals |2|2|
| Terminal WinSock code |10061|10061|
| Effective ceiling queried with4 |4(unlearned)|1(learned)|
| Peer bytes before first refusal |9,216|8,192|
| Peer bytes before terminal attempt |158,720|155,648|
| Peer bytes at terminal refusal |292,864|290,816|
| Exact completed peer bytes |4,194,304|4,194,304|
| Server active after joins |0|0|
| Worker/writer/socket/engine/server/reader exits |Verified|Verified|
| Final output published |No|No|

RED fails only the final assertion: `terminal second native refusal must learn the proven receiving width`, actual4versus expected1. Fixture, error sequence, continued reception, attempt budget, original terminal error and actual exits already pass. GREEN passes the same assertions. The8MiBpreallocated partial artifact is not represented as a completed8MiBdownload.

Actual command shape for both versions:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
cargo test --locked --offline -p nxtools-download --lib terminal_refusal_after_http_503_learns_without_an_extra_attempt -- --list
cargo test --locked --offline -p nxtools-download --lib terminal_refusal_after_http_503_learns_without_an_extra_attempt -- --nocapture --test-threads=1
```

`evidence/b6-01/b601-red-{list.log,results.json}`, `b601-red.log` and corresponding `b601-green*` files retain raw commands/counts/exits. `red-inputs/` and `current/` retain the actual source inputs.

## Repair-round verification

Every completed group has nonzero LIST followed by the identical-filter RUN, exit0and no failed/ignored/not_run tests. No original B6/coordinator pass was substituted for a repair-round execution.

| Selection | Groups | Successful executions | Result |
| --- | ---: | ---: | --- |
| Download lib and18original integration targets |19|385|Pass; lib143, engine105, HTTP26, resolution15and original storage/artifact/B4/B5/rate boundaries retained.|
| DownloadIPC, Network, Core, IPC, Protocol, CLI, BrowserHost and Core B4settings |8|353|Pass.|
| Original Desktop TestPlatform selections |10|104|Pass; all original intake/state/callback/countdown/Exit selections.|
| Total completed |37|842|All required groups pass; overlapping selections are not distinct tests.|

Download lib identically uses `--skip trial --skip real_expiry_` for LIST and RUN. Every group was newly listed and run on the settled source. Exact commands and per-group counts are in `evidence/b6-01-completion/b601c-final-preservation-results.json` and `b601c-*` logs. Desktop counts in original selection order are26/5/12/30/11/7/2/1/4/6.

Nine-package locked/offline `cargo check --keep-going --all-targets` exits0on the settled source. Strict all-target Clippy passes Core, Download, DownloadIPC, Network, IPC, CLI and BrowserHost. Protocol and Desktop record exit101only for the three unchanged historical diagnostics below. `evidence/b6-01-completion/b601c-quality-results.json` retains all ten actual commands, exits and durations.

Eleven-file scoped nonrecursive rustfmt and all five required source/lifecycle gates exit0: spec contracts, component catalog, locked GPUI release, Markdown editor provenance and download UI lifecycle. `evidence/b6-01-completion/b601c-gates.json` contains exact command inputs. Source gates do not certify physical/native/Release behavior.

Rust executions use one Cargo lane,21600second operation budgets, jobs1/incremental0/codegen-units256and scratch TEMP/TMP in the originating PowerShell environment. Cargo invocations require an idle shared build lane and at least8GiBavailable commit. No unrelated process is terminated.

## Attribution and evidence

The repair baseline was captured2026-11-12T09:15:55.917735+08:00 on `refactor/ui-gpui-kit` at `962badcbb1fe9b8b96f05fb982312549e173f8c6`:1,015pre-existing dirty paths and2,506protected files. Current-round before-images determine ownership, not the earlier B6 baseline or whole-HEAD diff.

The settled-source verification baseline was captured2026-11-12T10:24:34.228128+08:00 with1,162dirty paths and2,185protected files. Seven download source/fixture entries are byte-identical to the original repair evidence. This verification round changes only task records and its separate evidence archive; no product source is edited.

`b601-changed-paths.txt` inventories the implementation; `b601c-changed-paths.txt` inventories settled-source verification records and evidence. `evidence/b6-01/files.txt` retains the original RED/GREEN, source before/current copies and implementation attribution. `evidence/b6-01-completion/files.txt` inventories the fresh37group results, quality/gate/resource records, current-round before-copies and attribution audit. Coordinator report/evidence and frozen experiment evidence remain unchanged.

REF-13 remains ADOPT; REF-14/15/16 remain RETAIN-CURRENT. The frozen performance loads did not cover this combination; their timing data are unchanged and no new performance benefit is claimed.

## Known issues and reacceptance limits

The three historical strict diagnostics are unchanged against the settled-source baseline: Protocol `archive_browser.rs:563` unnecessary_map_or; Desktop `browser_handoff_impl.rs:436` nonminimal_bool and `settings_task_rem_tests.rs:614` unwrap_used. `evidence/b6-01-completion/b601c-lint-attribution.json` records the actual diagnostics and unchanged source attribution. No additional lint diagnostic, rule suppression or unrelated repair is present in this round.

Acceptance is **accepted** at source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform boundaries. The independent37groups/842executions include the terminal-refusal regression and all10Desktop groups. Physical UIA/native memory, Internet performance and installed/Release acceptance remain separate. No next-batch implementation was dispatched.
