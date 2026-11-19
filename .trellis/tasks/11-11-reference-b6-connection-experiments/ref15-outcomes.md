# REF-15 — Slow-connection retirement

Decision: **retain-current**. Acceptance: **accepted for this decision at the coordinator's stated boundaries**. [Current verdict](coordinator-acceptance.md).

The fixed persistent-slow workload did not establish an independent retirement benefit. Its completion median changed from5.797766s to5.780608s (−0.2959%), below the frozen15% adoption threshold. The existing released-slot tail split acted before the candidate's five-sample qualification. Candidate retirement requests were0 in all36 formal trials and24 warmups. Retain the original adaptive/rebalance implementation and REF-13's adopted learned ceiling; REF-14 remains unchanged.

## Original owners and candidate

Reference order and inspected ABDM → legacy → Mister/Ghost chains are in [reference-review.md](reference-review.md). Reference age/ratio values were experiment inputs, not production constants.

- `rust/crates/nxtools-download/src/engine/segmented_runner.rs`: original public engine task → actual worker JoinSet/progress map → one-second adaptive tick → released-slot tail selection → request_rebalance → physical worker return → synced Waiting checkpoint → persisted plan split → original queue/spawn.
- `rust/crates/nxtools-download/src/http.rs`: original segment response validation, bandwidth acquisition, writer, rebalance stop and sync. `DownloadTransferControl` owns both task and shared global limiters.
- `src/engine.rs`, `src/rate_limit.rs`: manual-global and runtime caps feed the same effective global limiter; task limits use the original task limiter and live update path.
- Default original layout is four4MiB segments; adaptive enabled, maximum32segments, minimum3MiB remaining. REF-13 ADOPT is fixed on both sides; no ceilings are learned in this fixture.

A temporary test-build candidate sampled real cumulative per-worker bytes on the original tick, retaining five stable samples, age≥2s, rate<quarter of best qualified active peer and>2s useful remaining work. Any effective task/global/runtime cap cleared qualification. One original pending rebalance identity signaled the existing worker control; its same-range requeue branch followed joined Rebalanced and Waiting checkpoint, without another controller, timer, transport or limiter. It did not trigger in the fixed workloads.

The candidate hooks were removed. `http.rs` and `segmented_runner.rs` were compared byte-for-byte with their REF-15 before-images and match. No production flag, unused dual path or new setting remains. Exact A/B source snapshots are retained under `evidence/ref15/baseline/` and `candidate/`; the binaries remain in session scratch only.

## Fixed load and measurement

- Owned loopback HTTP process:16MiB deterministic `offset %251` payload, width4,16KiB paced writes. First transfer request128KiB/s; all other/replacement requests1MiB/s. Recovering changes that first stream to1MiB/s after6s; stable uses1MiB/s throughout.
- Task, manual-global and runtime controls each set the original effective256KiB/s cap. No production timeout, payload, rate, width, adaptive policy or success assertion changes between versions.
- Server sends are cancellation-aware and count actual successful socket writes under the same120s total deadline; legal limiter backpressure is not a two-second fixture failure. The earlier unsuitable-send preflight is retained separately and excluded from comparisons.
- Six scenarios, two independent-resource warmups/version/scenario, then three serial pairs in **AB/BA/AB** order. No compilation, packaging, source/fixture edit or competing benchmark during timing.
- Public submit→Completed monotonic duration. Exact complete output is read afterward in64KiB chunks against an independent oracle, without whole-file buffering or hashing. Original in-memory SQLite, artifact writer, plan checkpoints and final publication are used.
- Every measured invocation has nonzero LIST1→identical-filter RUN1/PASS1, failed0/ignored0/not_run0/exit0. Formal36/36, exact576MiB; warmups24/24, excluded from statistics.
- WinAPI client/server memory and system commit sampled every100ms. Driver checks one compiler/package lane and at least8GiB commit headroom before each invocation. Full raw environment/resource samples and actual commands are archived.

## Completion seconds

A=current; B=temporary retirement candidate. All three raw values remain in pair order.

| Scenario | A raw | B raw | Median A→B | Slowest A→B | Median / slowest change |
| --- | --- | --- | --- | --- | --- |
| Persistent slow |5.798850,5.797766,5.794368|5.793605,5.780608,5.777370|5.797766→5.780608|5.798850→5.793605|−0.2959% /−0.0904%|
| Recover at6s |5.784926,5.776080,5.780904|5.790351,5.783077,5.779489|5.780904→5.783077|5.784926→5.790351|+0.0376% /+0.0938%|
| Stable |4.020781,4.025812,4.016978|4.029041,4.021581,4.029503|4.020781→4.029041|4.025812→4.029503|+0.2054% /+0.0917%|
| Task cap |70.955751,71.334213,71.247365|70.932691,71.543326,71.416402|71.247365→71.416402|71.334213→71.543326|+0.2373% /+0.2931%|
| Manual-global cap |68.859258,71.483925,70.703472|70.989730,71.728703,71.274610|70.703472→71.274610|71.483925→71.728703|+0.8078% /+0.3424%|
| Runtime cap |71.462973,71.760586,69.242327|70.659361,71.344736,71.516554|71.462973→71.344736|71.760586→71.516554|−0.1655% /−0.3401%|

The6s recovery is preempted by the original approximately4s released-slot split in both versions. These trials therefore do not establish behavior of a candidate that interrupts a still-live recovery stream.

## Requests, sockets and served-byte cost

Each accepted socket serves one HTTP request. The two probe requests include one payload byte. Maximum active transfer streams is4 in every formal trial. Candidate retirements are0 throughout.

| Scenario/version | HTTP requests = accepted sockets, pairs1–3 | Transfer requests | Served bytes, pairs1–3 |
| --- | --- | --- | --- |
| Persistent A/B |8,8,8|6,6,6|16793601,16793601,16793601|
| Recovering A/B |8,8,8|6,6,6|16793601,16793601,16793601|
| Stable A/B |6,6,6|4,4,4|16777217,16777217,16777217|
| Task A |12,12,12|10,10,10|27754497,27705345,27353089|
| Task B |12,12,12|10,10,10|27623425,27705345,27459585|
| Manual A |8,12,12|6,10,10|20029441,27770881,27951105|
| Manual B |12,12,12|10,10,10|28000257,26517505,27738113|
| Runtime A |12,12,8|10,10,6|27754497,26935297,20029441|
| Runtime B |12,12,12|10,10,10|27336705,27754497,27770881|

Persistent/recovering originals each physically retire one slow server stream; replacement requests consume the two durable split tails. The extra served payload is16384bytes, plus one probe byte. Stable retires none. Under limits, the original adaptive splitting can re-request bytes already sent into socket buffers but not yet durably written. This existing cost occurs with the candidate disabled and is visible in the table; it is not a retirement improvement. Server reset counts alone undercount client retirement when the entire response was already buffered.

Every formal trial has transfer_requests=`4 +2×(final_plan_segments−4)`: all transfer requests are accounted for by initial workers and persisted original splits, with0 additional retry requests in this fixed fixture. Native connection errors/refusals are0. Kernel `tcp_retransmitted_bytes=null`: served-byte overlap is not a measured TCP retransmission counter.

## Peak bytes across three formal trials

| Scenario/version | Client private | Server private | System committed |
| --- | ---: | ---: | ---: |
| Persistent A |4841472|25313280|35187462144|
| Persistent B |4681728|25395200|34942529536|
| Recovering A |4947968|25403392|34966147072|
| Recovering B |4562944|25387008|35249909760|
| Stable A |4857856|25518080|35235307520|
| Stable B |5038080|25407488|34970910720|
| Task A |10227712|25382912|35248234496|
| Task B |11214848|25337856|35280822272|
| Manual A |10743808|25382912|35228237824|
| Manual B |10743808|25407488|35232313344|
| Runtime A |9977856|25399296|35233779712|
| Runtime B |10436608|25333760|35374313472|

## Correctness, lifecycle and reproduction

`engine::connection_experiments::ref15::ref15_current_tail_split_keeps_exact_output_and_durable_plan` passed LIST1→RUN1/PASS1 on current and candidate builds. It verifies actual original tail retirement, exact16MiB, completed persisted segment checkpoints and real engine shutdown. This is first-pass coverage, not a fabricated RED/GREEN.

Every formal trial verifies the same task's SQLite plan is fully completed and sums to16MiB, exact artifact bytes, no unexpected HTTP connect errors, server active0/open_sockets0/failure=null, actual engine workers drained, server child waited successfully and stdout reader joined. Server records retain each actual request range/start/end/served count. Remote closure detection may follow client writer sync; it is not the client ownership boundary.

```powershell
cargo test --locked --offline -p nxtools-download --lib ref15_current_tail_split_keeps_exact_output_and_durable_plan -- --list
cargo test --locked --offline -p nxtools-download --lib ref15_current_tail_split_keeps_exact_output_and_durable_plan -- --nocapture --test-threads=1
```

Timed binaries use exact `engine::connection_experiments::ref15::ref15_trial --exact --ignored`, first with `--list`, then `--nocapture --test-threads=1`; `B6_SCENARIO=persistent|recovering|stable|limited|manual|runtime`. Set jobs1/incremental0/codegen-units256 and scratch TEMP/TMP in the same execution. [Evidence inventory](evidence/ref15/files.txt) includes371unchanged raw files:36formal and24warmup LIST/RUN/JSON/100ms samples, baseline preflights, A/B sources, frozen fixture, runner and pairing/collector scripts. `b6-ref15-pairs.json` is the aggregate record, not a substitute for raw logs.

## Limitations

No candidate retirement fired, so pending-candidate pause/error/cancel/reopen behavior was not dynamically certified. Original pause/resume/error/cancel and durable rebalance contracts remain the required final preservation checks; no activated-candidate lifecycle claim follows from these trials. The fixed workload supplies no independent adoption benefit, and broader thresholds or a changed workload are not shipped. Results are controlled HTTP/SQLite/writer evidence, not Internet throughput, physical PlatformHost/UIA memory, product GUI or installed/Release acceptance.
