# B6 verification

Acceptance: **accepted at source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform boundaries**. [Coordinator reacceptance](coordinator-acceptance.md) closes B6-01 after37fresh independent selections/842successful executions. Historical strict-lint and physical/Release limits remain below.

## Final-source behavior selections

All37selections below independently run against the repaired source. Every row has nonzero LIST, identical-filter serial RUN, LIST/RUN exits0, PASS equal to LIST, FAIL0, ignored/SKIP0 and not_run0. Total842successful executions; overlapping Desktop filters are not842distinct tests. Fresh commands, counts, exits, preflights and logs are in `evidence/coordinator-reacceptance/results.json`. Original841execution delivery evidence remains under `evidence/final/`.

| Package / target or Desktop filter | LIST | RUN | PASS |
| --- | ---: | ---: | ---: |
| Download lib, excluding `trial` and `real_expiry_` |143|143|143|
| `domain_contract` |12|12|12|
| `message_contract` |16|16|16|
| `engine_contract` |105|105|105|
| `http_client_contract` |26|26|26|
| `http_filename_contract` |1|1|1|
| `resolution_contract` |15|15|15|
| `artifact_contract` |4|4|4|
| `queue_order_contract` |1|1|1|
| `storage_contract` |7|7|7|
| `storage_capacity_contract` |3|3|3|
| `http_fixture_contract` |7|7|7|
| `server_info_contract` |11|11|11|
| `b4_paused_url_contract` |11|11|11|
| `b4_request_policy_contract` |3|3|3|
| `b4_coverage_contract` |1|1|1|
| `b5_completion_contract` |9|9|9|
| `rate_limit_contract` |2|2|2|
| `runtime_speed_cap_contract` |8|8|8|
| DownloadIPC lib |9|9|9|
| Network lib |3|3|3|
| Core lib |98|98|98|
| IPC lib |62|62|62|
| Protocol lib |116|116|116|
| CLI lib |15|15|15|
| BrowserHost lib |48|48|48|
| Core `b4_download_settings_contract` |2|2|2|
| Desktop `b5_` |26|26|26|
| Desktop `completion_actions::tests::` |5|5|5|
| Desktop `shutdown` |12|12|12|
| Desktop `b4_` |30|30|30|
| Desktop `ref06_` |11|11|11|
| Desktop `server_info_` |7|7|7|
| Desktop `task_window_handoff_` |2|2|2|
| Desktop `task_window_render_never_reads_the_parent_download_application` |1|1|1|
| Desktop `ref01_` |4|4|4|
| Desktop `new_download_link_tests::` |6|6|6|

The original engine105covers scheduler/adaptive/rebalance/stop/checkpoint/error/identity behavior. B4 selections retain exact default Header/reset/merge, URL revision/validator, template and actual coverage ownership; resolution's full effective-request assertion includes the exact default User-Agent. B5 real HTTP/SQLite and Desktop event-pump selections retain authorization, source-round/member invalidation, terminal refusal, stopped-queue retry publication, countdown cancellation and normal Exit join. Desktop uses TestPlatform, not product GUI or current Desktop exit.

Selection commands:

```text
cargo test --locked --offline -p <package> --lib <filter> -- --list <same-exclusions>
cargo test --locked --offline -p <package> --lib <filter> -- --nocapture --test-threads=1 <same-exclusions>
cargo test --locked --offline -p nxtools-download --test <target> -- --list
cargo test --locked --offline -p nxtools-download --test <target> -- --nocapture --test-threads=1
cargo test --locked --offline -p nxtools-desktop --bin nxtools-desktop <filter> -- --list
cargo test --locked --offline -p nxtools-desktop --bin nxtools-desktop <filter> -- --nocapture --test-threads=1
```

The Download library uses `--skip trial --skip real_expiry_` identically in LIST and RUN. Frozen timed/actual-expiry records remain in the per-REF evidence; repair reacceptance does not remeasure them. All independent Rust execution uses one lane,21600second budgets, at least8GiBcommit headroom and jobs1/incremental0/codegen-units256/scratch TEMP/TMP in the originating environment. `evidence/coordinator-reacceptance/b601-coordinator-verify.py` records the fresh37group selection and quality commands.

## Behavior RED and lifecycle assertions

- REF-13 refusal baseline: LIST1/RUN1/FAIL1/exit101 with complete exact32MiB and actual joins; refusal sequence `[2,2,2,2]`. Adopted candidate: LIST1/RUN1/PASS1/exit0, `[2,0,0,0]`, same load/output/drain.
- REF-13 bounded-cache boundary: `active_run_workers_keep_the_same_observation_after_cache_eviction` forces256legal authority lookups, proves original-entry eviction, then runs the original HTTP segment worker/artifact. RED fails only at same-run ownership after exact8MiB/no retry/actual joins; GREEN retains and releases the run Arc. Current independent lib143/143includes it. Raw `b6-ref13-ownership-{red,green}*` records remain in `evidence/final/`.
- REF-13 real statuses, lower caps,600000ms expiry, route change/recovery, durable same-plan pause/resume/cancel and manual proxy controls pass at actual HTTP/SQLite/writer boundaries.
- REF-14 required delayed confirmation is a candidate behavior failure:3/3incomplete bodies, each actual exit101. It is not a passing safety test. Retain-current avoids an unimplemented response holder; current probe/cancel proves no artifact and actual stream/socket/server/reader exit.
- REF-15 candidate retirement count is0in every formal and warmup run. Original split and exact persisted Completed ranges/actual drain pass; no activated candidate retirement lifecycle is claimed.
- REF-16 original HTTP same-family pending baseline, address-family fallback, port-swapped regression, real cancellation and31second prototype expiry, strict/explicit-invalid TLS, CONNECT/SNI and no-origin-probe controls are separately recorded. Final retained same-family/cross-port/cancel/TLS selections pass without a winner cache.
- B6-01 `terminal_refusal_after_http_503_learns_without_an_extra_attempt`: identical-test/fixture RED LIST1/RUN1/FAIL1/exit101 only at final width4versus1; delivered GREEN LIST1/RUN1/PASS1/exit0. Independent Download lib143includes the same regression. Both native refusals, continuing peer reception, original terminal10061, no fourth target attempt, exact4MiBpeer range and actual joins are asserted. `evidence/coordinator-b601-result.json` retains the independent result.

Compile/fixture prerequisites are not behavior RED. Supplementary first-pass coverage is not represented as RED→GREEN. Full bytes are checked outside timing in bounded chunks against an independent oracle; application served-byte overlap is not kernel TCP retransmission.

## All-target check and strict lint

Independent nine-package locked/offline `cargo check --keep-going --all-targets`: **exit0**. Independent strict Clippy passes Download, Network and DownloadIPC. The settled-source delivery's nine-package strict results below retain seven passing packages and the three historical diagnostics; raw commands/exits are in `evidence/b6-01-completion/b601c-quality-results.json`.

| Package | Strict exit | Actual result |
| --- | ---: | --- |
| Core |0|PASS|
| Download |0|PASS, including all retained B6 tests|
| DownloadIPC |0|PASS|
| Network |0|PASS|
| IPC |0|PASS|
| Protocol |101|Historical `unnecessary_map_or`, `rust/crates/nxtools-protocol/src/archive_browser.rs:563`|
| Desktop |101|Historical `nonminimal_bool`, `rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs:436`; `unwrap_used`, `rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs:614`|
| CLI |0|PASS|
| BrowserHost |0|PASS|

The strict suite is **not all green**. Settled-source `b601c-lint-attribution.json` and its actual raw logs retain only the three historical diagnostics above, with no additional external diagnostic or suppression. The independent strict rerun covers the three affected owner packages; its exact commands and exits are in `evidence/coordinator-reacceptance/results.json`.

## Final source and lifecycle gates

All six independent final commands exit0, recorded in `evidence/coordinator-reacceptance/results.json`:

- `rustfmt --check --edition 2024 --config skip_children=true` on the eleven explicit B6 Rust files, including `connection_experiments/b601.rs`, without recursive child formatting.
- `python tests/architecture/spec-contracts.py`.
- `python tests/architecture/gpui-component-catalog.py`.
- `python tests/architecture/gpui-release-contracts.py`.
- `python tests/architecture/markdown-editor-contracts.py`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/download-ui-lifecycle.ps1`.

These detect scoped format/spec drift, catalog/API/provenance changes and original entity/window lifecycle violations. They do not certify visual/native/Release acceptance.

## Ownership, protection and resource audit

Original B6 attribution remains the captured194dirty-path baseline and before-images under `evidence/final/`. Repair attribution uses its own before/current source and `b601-changes.diff`; settled-source verification remains separately archived under `evidence/b6-01-completion/`. No whole-HEAD diff is assigned to B6-01, and concurrent Archive/Quick Launch/Desktop/C#/reference changes are preserved.

Independent pre-recording audit compares2177relevant frozen experiment/prior acceptance/source files with their captured bytes; all match. Seven repair source/fixture snapshots match delivery and remain unchanged throughout independent execution. The original combined-regression test/fixture bytes are identical between RED and GREEN. Coordinator record updates are separate from product source.

Final independent resource observation: no owned B6fixture/test/driver/compiler process and no temporary `b6-tls-*` directory. Actual per-test worker/server/reader/socket joins remain lifecycle authority. `evidence/coordinator-reacceptance/resources.json` retains the observation; the verification wrapper exits0.

## Limitations and handoff

- The three historical strict-lint diagnostics remain outstanding outside this batch; they are not reported as passes.
- Kernel TCP retransmission and losing-fallback native codes not observed by a trial remain null. Served overlap, exact final bytes and actual resource exit are separately recorded.
- Rejected REF-14/15/16 prototypes do not certify unimplemented production response-holder, retirement or winner-cache integration lifecycles.
- Evidence establishes source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform boundaries only. External-network speed, physical GUI/UIA/PlatformHost memory, installed service and Release/package acceptance remain separate.

B6 is **accepted**, with B6-01 closed and no remaining blocker at the stated boundaries. Independent repair evidence is in `evidence/coordinator-reacceptance/`; initial verdict and frozen performance evidence remain retained. No B7, product/reference GUI, real power/current Desktop exit, package/install/service/Registry mutation, commit or push.
