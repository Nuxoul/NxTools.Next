# REF-13 — Learned connection ceiling

Decision: **adopt**. Acceptance: **accepted at the coordinator's stated boundaries**, with B6-01 closed. [Current verdict](coordinator-acceptance.md).

The primary benefit is removal of repeated refused connection attempts after learning, not higher throughput. In the controlled receiving-width workload, post-learning refusals fall100percent. Batch completion median increases4.72percent and slowest increases4.54percent, within the frozen5percent bound. Unrestricted and lower-task-cap controls remain effectively unchanged. Freeze this decision for both REF-15 versions.

## Original ownership and implementation

- `rust/crates/nxtools-download/src/engine.rs`: original `DownloadEngineInner` owns the bounded ephemeral observation table and captures immutable operation clients through `http_for_operation_traced`.
- `rust/crates/nxtools-download/src/engine/connection_ceiling.rs`: maximum256entries, initial600second expiry; key is final resource authority plus original policy generation/route/certificate scope. Rebuilt System clients have distinct scope because the existing `system` route key does not identify the actual proxy endpoint.600seconds is the maximum within that captured scope, not a promise to survive its replacement; the original30second System-client reuse window may cause later tasks to capture a new scope sooner.
- `rust/crates/nxtools-download/src/http.rs`: native refusal requires a pre-header connect error whose typed IO source is `ConnectionRefused`. Authentication, HTTP status, TLS/body/resource validation and stop errors do not supply this evidence. Request construction, redirects, captured routing and error categories remain with the original client.
- `rust/crates/nxtools-download/src/engine/segmented_runner.rs`: actual first payload progress establishes a receiving lease; two qualified refusals with receiving peers learn the largest proven width. Initial, refill, split and insufficient-tail admissions take the minimum of this observation and existing limits. One run captures one observation Arc and passes it to every original segment spawn, so bounded table eviction cannot separate worker feedback from that run's admission. Later runs may relearn after normal eviction. Running workers are not force-stopped and persisted requested width/plan identity is unchanged.
- Existing rebalance still stops and joins, synchronizes the writer, persists Waiting/checkpoint state and only then reassigns. No scheduler, transport, persisted setting, protocol change or production feature flag is added.

Production diagnostics use the original correlation/task identity and one bounded `download.connection_ceiling.learned` event with fixed reason, width,600second maximum lifetime, elapsed time and native error code. No request payload or sample-by-sample log is added.

Reference behavior was inspected in ABDM first, then legacy NxTools and Mister/Ghost; exact chains are in `reference-review.md`. GPL reference code, comments and tests were not copied.

## Fixed experiment

Windows controlled loopback HTTP/1.1, locked reqwest0.13.4/hyper-util0.1.20. One Cargo lane; jobs1/incremental0/codegen-units256 and scratch TEMP/TMP. Binaries were built before timing. No compilation, packaging or competing benchmark was started by this task during the serial batch.

Each measured batch is four successive8MiB tasks, requested width4, original DownloadEngine acceptance/task/shutdown, in-memory SQLite and real segmented artifacts. The server permits2receiving streams, closes its actual listener at that width, then rebinds when a receiving stream exits. A resolver barrier only orders the initial extra connects after actual listener closure; TCP itself returns the native refusal. Segment payload pacing is2.5seconds, fixed before valid A/B timing. Native prerequisite calls observed WinSock10061 after2.0308902/2.0306899seconds while established peers continued receiving.

Each scenario has two untimed warmup batches per version on separately owned random-port resources, followed by three serial pairs in AB/BA/AB order. Exact full output is read in64KiBchunks against independent `offset % 251` values outside the measured interval; no whole-file buffer or output hash is used. Actual engine/server/reader joins follow the measurement. Memory is sampled every100ms during the measurement markers.

## Raw completion times

Seconds per four-task batch. Warmups are excluded.

| Scenario | Version | Pair1 | Pair2 | Pair3 | Median | Slowest | Median change | Slowest change |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Server width2, task width4 | A current |22.281875|21.794647|22.324779|22.281875|22.324779|—|—|
| Server width2, task width4 | B learned |23.333758|23.337825|22.845904|23.333758|23.337825|+4.7208%|+4.5378%|
| Unrestricted server, task width4 | A current |10.081060|10.089230|10.091582|10.089230|10.091582|—|—|
| Unrestricted server, task width4 | B learned |10.106593|10.114545|10.106421|10.106593|10.114545|+0.1721%|+0.2275%|
| Server width2, task cap1 | A current |10.102753|10.085193|10.088789|10.088789|10.102753|—|—|
| Server width2, task cap1 | B learned |10.059499|10.097389|10.077918|10.077918|10.097389|−0.1078%|−0.0531%|

Primary aggregate throughput is1.436145→1.371404MiB/s; unrestricted is3.171699→3.166250MiB/s. These values describe the fixed local fixture, not general network performance.

## Request, connection and resource cost

Every measured batch completes exactly32MiB; every server ends with active0/failure=null, original workers drained and actual server process exited.

| Scenario/version | Refusals by task in every measured round | HTTP requests | Accepted TCP | Total TCP attempts | Extra served bytes | Maximum receiving streams |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Limited A |[2,2,2,2]|24|24|32|4|2|
| Limited B |[2,0,0,0]|24|24|26|4|2|
| Unrestricted A/B |[0,0,0,0]|24|24|24|4|4|
| Task cap1 A/B |[0,0,0,0]|12|12|12|4|1|

Six post-learning refusals per batch become zero; total refused attempts fall8→2 and all connect attempts32→26. Refused pre-header connects do not create HTTP requests. In this single-address complete-success fixture, total TCP attempts are actual accepted sockets plus typed native refusals; no other connect errors occurred. The4served bytes are one-byte probe overlap from four tasks.

Peak bytes across the three measured rounds, with all individual100ms samples retained:

| Scenario/version | Client private | Server private | System committed |
| --- | ---: | ---: | ---: |
| Limited A |6606848|16871424|34884182016|
| Limited B |6971392|16953344|34831798272|
| Unrestricted A |8093696|16973824|34731126784|
| Unrestricted B |7942144|16977920|34775498752|
| Task cap1 A |4775936|16805888|34739265536|
| Task cap1 B |4587520|16842752|34632728576|

## Correctness and lifecycle evidence

| Selection | LIST→RUN | Result | Boundary |
| --- | --- | --- | --- |
| `ref13_successful_width_prevents_repeated_extra_connection_refusals`, current |1→1|FAIL1, exit101|Behavioral RED: four complete/exact/drained tasks still repeat native refusals after training.|
| Same exact selection, learned candidate |1→1|PASS1, exit0|GREEN: refusal sequence[2,0,0,0], same exact output and actual joins.|
| `engine::connection_ceiling::tests::` |3→3|PASS3, exit0|Two-refusal/no-peer rule, clock boundary, actual progress/lower caps, authority/policy/certificate/System scope and256entry bound.|
| `http_status_resource_and_stop_errors_never_prove_a_connection_ceiling` |1→1|PASS1, exit0|Actual401/403/408/425/429/500/503,21requests; typed resource/stall/stop exclusions; real native refusal with no receiving peer does not learn.|
| `real_expiry_route_generation_and_recovery_release_the_learned_width` with explicit `--ignored` |1→1|PASS1, exit0|Actual600000ms wait, route generation trains independently, recovered server receiving width2→4 after expiry;32MiB exact and all workers/server joined.|
| `learned_ceiling_preserves_pause_resume_plan_and_actual_cancel_drain` |1→1|PASS1, exit0|Learn first, pause after real bytes, query durable SQLite ranges, resume same task/ranges to exact output, then cancel another live task. Four server streams actually retire; final active0, no unexpected server error.|
| `learned_direct_width_does_not_cross_actual_manual_proxy_route` |1→1|PASS1, exit0|Direct learned width2 does not constrain new actual ManualHttp route; six absolute-form proxy requests, receiving width4,16MiB exact, real drain.|
| `active_run_workers_keep_the_same_observation_after_cache_eviction`, original spawn lookup |1→1|FAIL1, exit101|Behavioral RED at original worker boundary:256other authorities evict the current record; real8MiB output and worker/server/reader drain succeed, but the worker does not retain the admission observation.|
| Same selection, same-run Arc |1→1|PASS1, exit0|GREEN: same exact range/output/drain, worker holds and releases the original run observation after eviction; no retry or extra network request.|
| Three scenarios × three pairs × two versions |18→18|PASS18, exit0|All formal paired measurements selected and ran, no ignored/skipped tests.|
| Untimed warmups |12→12|PASS12, exit0|Separate random-port resources, exact output and actual exit for each warmup.|

New control tests are first-pass coverage, not fabricated RED. The explicit `--ignored` option selects the long/measurement entries for execution; their recorded ignored count is0.

Independent repaired-source37affected-package/B1–B5 selections pass842executions, including terminal refusal after503and the same-run cache-eviction boundary. Nine-package all-target check, Download/Network/DownloadIPC strict lint, eleven-file format and five source/lifecycle gates exit0. Historical Protocol/Desktop strict diagnostics remain separately attributed. See `verification.md`, `b6-01-repair.md` and `evidence/coordinator-reacceptance/`; original trial records remain unchanged.

## Reproduction and evidence

Source fixtures:

- `rust/crates/nxtools-download/src/engine/connection_experiments.rs`
- `rust/crates/nxtools-download/src/engine/connection_experiments/ref13_controls.rs`
- `rust/crates/nxtools-download/tests/support/b6_server.py`

`evidence/ref13/files.txt` inventories the181unchanged evidence files. `b6-ref13-pairs.json` contains raw records/median/slowest/costs; each named trial has exact LIST command, raw RUN log, memory samples and exit/result JSON. `b6-ref13-pairs.py` and `b6-measure.py` are the serial timing drivers. Baseline/candidate test executables remain separately preserved in the session scratch under `b6-ref13-baseline/` and `b6-ref13-candidate/`; no binary or production toggle is shipped in the task sources.

Build/functional selection shape:

```text
cargo test --locked --offline -p nxtools-download --lib <exact-filter> -- --list
cargo test --locked --offline -p nxtools-download --lib <same-filter> -- --nocapture --test-threads=1
```

Timing uses the already-built binary with exact filter `engine::connection_experiments::ref13_trial --exact --ignored`, `B6_SCENARIO=limited|unrestricted|lower_cap`, and the same serial test-thread option. The real-expiry selection additionally uses `--ignored` for both LIST and RUN.

The bounded-cache worker regression is archived with final verification under `evidence/final/`. Frozen REF-13 timing loads do not evict the table: their original admission and worker lookups already return the same Arc. The final ownership boundary does not replace those measurements or add a performance claim.

## Limitations

Kernel TCP retransmission bytes were not observed and remain null. Served-byte overlap is not TCP retransmission. Resource liveness is established by actual child wait/GetExitCodeProcess and owned joins, not by the ability to query an exited process object's memory counters. The RED resource record includes both the memory-only observer field and the successful actual child wait; formal pairs use actual process-state sampling.

This decision is supported at source and owned local HTTP/SQLite/writer boundaries. It does not establish external-network performance, PlatformHost physical memory behavior, product GUI acceptance or installed/Release acceptance. Those levels are not taken over by B6.
