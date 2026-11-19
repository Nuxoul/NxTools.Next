# REF-14 — Eligible first-GET reuse

Decision: **retain-current**. Acceptance: **accepted for this decision at the coordinator's stated boundaries**. [Current verdict](coordinator-acceptance.md).

Reusing the already-issued complete fallback GET removes four requests in the immediate-confirmation workload and completes the single-use link. It fails the required10second delayed-confirmation control in all three measured rounds. The unpolled response encounters real server write backpressure, the server times out after2seconds without write progress, and the original body reader rejects the incomplete response. No final artifact is published. Keep the current probe/drop/reprobe/download behavior.

## Original owners and candidate

- `rust/crates/nxtools-download/src/http.rs`: `probe_timed` starts HEAD and exact `bytes=0-0` requests on the original immutable routed client. Only when both fail does its existing full fallback GET provide a potentially reusable response. HEAD, partial Range and media-manifest responses are not whole-file candidates.
- Current confirmation behavior reprobes, then `download` → `prepare_download` sends the transfer GET, streams through the original bandwidth/progress/writer loop, synchronizes and uses `commit_prepared` for no-replace publication.
- `rust/crates/nxtools-download/src/engine/resolution.rs` remains the original operation/generation/session/receipt/cancellation owner. Default no-op resolution has no extension receipt or provenance. No receipt was fabricated.
- The measured candidate used a test-only tap of that existing fallback response and a single owned response slot, not an extra GET or second transport. Confirmation consumed it once through the original `prepare_download` body loop and `commit_prepared`. No artifact/body consumption occurred before confirmation.
- The proposed product bound is four responses and15seconds from headers, or earlier actual resolver validity. The feasibility experiment needs only one response and confirms at0or10seconds, inside that bound. It rejects the body mechanism before introducing resolution/IPC/UI ownership changes. Product code retains no response holder, injection path, feature flag or second scheduler.

ABDM was inspected first; its metadata GET is disposed and transfer uses another GET. Mister/Ghost comparison and exact entry chains are in `reference-review.md`. No reference implementation code, comments or tests were copied.

## Fixed workload and measurement

Owned Windows loopback HTTP/1.1,8MiB deterministic payload,2MiB/s pacing in64KiBwrites. Every request has200ms server preparation before response headers; this is service/header delay, not a simulated TCP-connect delay. Socket write timeout is2seconds. HEAD405 / Range416 / full GET200 creates the existing eligible fallback. Single-use permits only the first full GET; later full GET returns410. The normal/ineligible control supplies HEAD200 and exact one-byte206. Truncation sends4MiB while declaring8MiB; unknown-length closes after the full8MiB.

The real current implementation ran all six scenarios before candidate changes. Separate A/B test binaries were then used without compilation during timing. Each scenario ran two independent-resource warmups per version and three serial pairs in AB/BA/AB order. Original REF-13 adopted behavior is unchanged. No task-started packaging, source/fixture edit or competing benchmark ran during the serial comparison.

Timing includes initial probe, any required10second confirmation delay and confirmation-to-terminal. Exact full output is checked afterwards in64KiBchunks against independent `offset % 251` values. Actual server socket/thread/reader/process joins follow timing.100ms memory samples, pressure snapshots, exact LIST/RUN commands and raw exits are retained.

## Raw terminal times

Seconds. Completion statistics exist only when all three transfers completed with exact output. Failed terminal latency is not download throughput or a speedup.

| Scenario | Version | Pair1 | Pair2 | Pair3 | Completion median | Completion slowest | Terminal result |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Immediate | A current |5.054342|5.024190|5.041251|5.041251|5.054342|3 exact completions|
| Immediate | B reuse |4.420200|4.435499|4.422997|4.422997|4.435499|3 exact completions|
|10second delay | A current |15.079371|15.035555|15.063073|15.063073|15.079371|3 exact completions|
|10second delay | B reuse |10.432965|10.442849|10.423907|—|—|3 body failures, test exit101|
| Single-use | A current |0.816043|0.833711|0.822007|—|—|3 HTTP410 terminals|
| Single-use | B reuse |4.434624|4.443583|4.420944|4.434624|4.443583|3 exact completions|
| Normal/ineligible | A current |4.649377|4.624555|4.636317|4.636317|4.649377|3 exact completions|
| Normal/ineligible | B reuse |4.616638|4.655839|4.649693|4.649693|4.655839|3 exact completions|
| Truncated | A current |3.026279|3.043844|3.042126|—|—|3 body errors, no final file|
| Truncated | B reuse |2.431285|2.435878|2.430409|—|—|3 body errors, no final file|
| Unknown length | A current |5.057448|5.026962|5.043355|5.043355|5.057448|3 exact completions|
| Unknown length | B reuse |4.424617|4.433722|4.431251|4.431251|4.433722|3 exact completions|

Immediate median falls12.2639percent. Normal/ineligible median rises0.2885percent and slowest rises0.1390percent. These local benefits do not override the failed delayed-body requirement. Single-use A has no successful completion median and no percentage speedup is assigned to it.

## Requests, sockets and served bytes

Values are identical across all three rounds of each row. Every terminal ends with open_sockets0, active0, no unexpected fixture failure and actual child/reader join. Retry logic was not added; the additional baseline full GETs are existing probe/transfer requests, not application retries.

| Scenario/version | HTTP requests | Full GETs | Accepted TCP sockets | Served bytes | Outcome |
| --- | ---: | ---: | ---: | ---: | --- |
| Immediate / delayed / unknown A |7|3|7|8519680|Exact8MiB|
| Immediate / single-use / unknown B |3|1|3|8388608|Exact8MiB|
| Delayed B |3|1|3|2359296|One real write timeout; incomplete body|
| Single-use A |6|2|6|65536|HTTP410|
| Normal/ineligible A/B |5|1|5|8388610|Exact8MiB plus two one-byte probes|
| Truncated A |7|3|7|4325376|No final publication|
| Truncated B |3|1|3|4194304|No final publication|

Baseline completed fallback workloads serve131072additional bytes on discarded probe streams. The delayed candidate serves2359296bytes before confirmation into socket/HTTP buffers, not an artifact. Not consuming the application body does not suspend the remote sender or ensure its response remains viable.

Peak bytes across the three measured rounds:

| Scenario/version | Client private | Server private | System committed |
| --- | ---: | ---: | ---: |
| Immediate A |2347008|8736768|35940962304|
| Immediate B |2383872|8622080|35956682752|
| Delayed A |2359296|8818688|37893931008|
| Delayed B |2412544|8777728|37856313344|
| Single-use A |1957888|8634368|36536696832|
| Single-use B |2342912|8597504|38356324352|
| Ineligible A |2342912|8777728|37115404288|
| Ineligible B |2383872|8634368|37305073664|
| Truncated A |2359296|8736768|37377925120|
| Truncated B |2347008|8749056|37377187840|
| Unknown A |2371584|8732672|37243895808|
| Unknown B |2379776|8790016|37295710208|

## Correctness and lifecycle checks

- Formal pairs:36 nonzero LIST1→same-filter RUN1 selections;33tests pass and3required delayed-candidate tests fail with exit101,0skips. The process runner's successful evidence collection does not convert these failures to passes. Baseline single-use and both truncated cases pass their negative-result oracle but are not successful downloads.24formal transfers complete with exact8MiB output.
- Warmups:24nonzero selections,22pass and2required delayed-candidate failures,0skips. Warmups are excluded from formal statistics.
- All completed outputs use the original streamed writer/sync/commit path and match every byte outside timing. Every body failure leaves the final artifact absent.
- `http::connection_experiments::ref14_current_probe_and_probe_cancellation_release_unconfirmed_sockets`: LIST1→RUN1/PASS1, exit0. Actual normal fallback return and cancellation while both header probes are pending release sockets and join the owned server in99440/176399microseconds respectively, no artifacts. This is first-pass coverage, not a fabricated RED.
- The candidate's delayed control is a real mechanism failure after a valid baseline and successful immediate reuse, not a compilation or fixture-prerequisite failure.

## Paths and reproduction

Retained tests:

- `rust/crates/nxtools-download/src/http/connection_experiments.rs`
- `rust/crates/nxtools-download/tests/support/b6_reuse_server.py`
- `rust/crates/nxtools-download/src/http.rs` declares the test module; its product request/body paths remain unchanged by REF-14.

`evidence/ref14/files.txt` inventories raw evidence. `b6-ref14-pairs.json` retains all36formal records, summaries, memory peaks, request/socket/served costs and failed exits. Each measured/warmup trial retains raw LIST, RUN,100ms samples and JSON, plus the serial `b6-ref14-pairs.py` / `b6-measure.py` drivers and exact A/B source snapshots. Separate executables remain in session scratch `b6-ref14-baseline/` and `b6-ref14-candidate/`; no binary is shipped in the source task.

```text
cargo test --locked --offline -p nxtools-download --lib http::connection_experiments::ref14_trial -- --list --ignored
cargo test --locked --offline -p nxtools-download --lib http::connection_experiments::ref14_trial -- --nocapture --test-threads=1 --ignored
```

Timing runs the already-built A/B executable with exact filter `http::connection_experiments::ref14_trial --exact --ignored`, same RUN selection and `B6_SCENARIO=immediate|delayed|single_use|ineligible|truncated|unknown`. The retained source reproduces current behavior; rejected candidate source snapshots reproduce the experiment.

## Limitations

The candidate was rejected at the original HTTP/body/writer boundary before product resolution/IPC/UI integration. Product edit/replacement/close/route-generation/receipt-expiry/capacity/admission invalidation for a retained response was not implemented or certified. Existing ownership remains unchanged. The experiment demonstrates a failure within the proposed15second retention window, not a universal server timeout policy.

Kernel TCP retransmitted bytes remain null; served-byte overlap is not TCP retransmission. The200ms setup is server header preparation, not a measured slow TCP handshake. No external-network, product GUI, PlatformHost physical-memory or installed/Release acceptance follows from these owned local results.
