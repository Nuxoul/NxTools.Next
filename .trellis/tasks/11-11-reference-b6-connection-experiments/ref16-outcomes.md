# REF-16 — Multi-address connection selection

Decision: **retain-current**. Acceptance: **accepted for this decision at the coordinator's stated boundaries**. [Current verdict](coordinator-acceptance.md).

The locked connector has a measurable repeated same-family setup cost, but the hostname winner candidate doubles setup latency and losing connections when two ports on the same host have different usable addresses. Preserve the existing connector and its300ms cross-family Happy Eyeballs; no winner cache or alternate connector is shipped.

## Original owner and experiment

Reference order and exact locked-stack entries are recorded in [reference-review.md](reference-review.md): ABDM's existing HTTP client first, then legacy NxTools and Mister/Ghost. Mister's independent TCP probes are behavioral comparison only; no reference code or probe framework is copied.

Original owner: `rust/crates/nxtools-download/src/http.rs::build_for_snapshot`, `send`, `prepare_download`, `download`, `commit_prepared`; `nxtools-network` owns the immutable Direct/System/Manual proxy/TLS snapshot. Locked reqwest0.13.4 → hyper1.10.1/hyper-util0.1.20 → hyper-rustls0.27.9/rustls0.23.42 retains serial addresses within a family and300ms fallback between families. Original30second connection/read/stall deadlines are unchanged.

The test seam is `src/http/connection_experiments/ref16.rs`; owned fixtures are `tests/support/b6_address_server.py` and `b6_tls_server.py`. The test-only response observer measures original request-to-headers duration. It does not alter production requests. Existing public resolver injection supplies fixed two-answer DNS order. The transfer path uses original request validation, artifact paths, streamed writer, sync and no-replace commit. Supplied fixture metadata fixes1MiB length without adding probe traffic.

A measured batch contains four serial forced-new1MiB GETs. Every fixture response closes its connection, preventing connection pooling from masking setup. Payload is offset modulo251. Independent64KiB reads verify every output byte after the batch timer stops; no output hash or whole-file buffering. Each transfer has the same120second fixture watchdog. REF-13's adopted production behavior is identical in A and B; no segmented admission is exercised by this single-stream workload.

Candidate B changes only the original resolver's subsequent answer order: successful `Response::remote_addr` becomes a Direct-only hostname winner, at most256names,30second validity, no extra probe sockets. Other answers retain relative order; the experiment supplies at most two candidates. DNS `Name` exposes hostname, not destination port. Each independently constructed captured client has its own observation/cache. Explicit proxy routes neither reorder origin answers nor learn origin preferences. A and B source snapshots differ only in the test candidate constant; the constant and cache implementation are absent from retained source.

## Actual connection prerequisite and baseline

Owned127.0.0.2 listeners with a saturated backlog and same-port127.0.0.1 live listeners require no firewall, route, service or Registry changes. A read-only Windows IP Helper observer filters the exact Rust child PID and owned destination endpoints. `SYN_SENT` is native TCP evidence, not delayed HTTP preparation. Three separate native preflights observed pending connects for0.7653482/0.7507060/0.7618374seconds while alternate-address connect completed in0.0001348/0.0001124/0.0001497seconds; all owned sockets and accept workers exited.

The original HTTP baseline then completed four exact outputs in8.134798seconds. Its four request-to-headers durations were2.023515/2.030819/2.027794/2.040196seconds; four original-process pending connects were observed. The gap therefore exists in the actual locked HTTP path, not only a Python socket model.

Scenarios are fixed before candidate comparison:

- **Primary:** first same-family address pending, second live, one port.
- **Cross-port:** alternate ports A/B/A/B; A has `.2` pending/`.1` live and B has `.1` pending/`.2` live, same hostname and original DNS order.
- **Stable:** first address live.
- **Family6/Family4:** pending IPv6/live IPv4 and pending IPv4/live IPv6.
- **Unreachable:** first loopback address has no listener, second live.
- **Proxy:** original explicit local HTTP proxy, absolute-form requests, no origin DNS resolution.

## Serial paired measurements

A=current, B=hostname preference. Each scenario has three serial AB/BA/AB pairs, with two independent-resource warmups per version excluded. No compilation, source/fixture editing, packaging or competing benchmark occurs during paired timing. Raw environment/commit/private/working-set samples, command lines and actual exits are retained.

The authoritative table contains42selections. Cross-port uses port-qualified slow-endpoint observation in `b6-ref16-crossport-pairs.json`; the other six scenarios use `b6-ref16-pairs.json`. All48recorded formal selections, including six additional cross-port observations, are retained: LIST1→RUN1/PASS1/exit0 each,0skip/not_run,192MiB exact formal output. All32warmup selections also pass and are excluded from adoption statistics.

Seconds, raw values in pair order:

| Scenario | A raw | B raw | Median A→B | Slowest A→B | Median / slowest change |
|---|---|---|---|---|---|
| Primary |8.172699,8.190464,8.159895|2.050162,2.064789,2.033046|8.172699→2.050162|8.190464→2.064789|−74.9145% /−74.7903%|
| Cross-port |4.107963,4.137180,4.095731|8.157313,8.180215,8.173587|4.107963→8.173587|4.137180→8.180215|+98.9693% /+97.7244%|
| Stable |0.016457,0.016251,0.018048|0.016964,0.017170,0.016430|0.016457→0.016964|0.018048→0.017170|+3.0808% /−4.8648%|
| Pending IPv6/live IPv4 |1.244355,1.251500,1.253732|0.319141,0.317686,0.322226|1.251500→0.319141|1.253732→0.322226|−74.4993% /−74.2987%|
| Pending IPv4/live IPv6 |1.309879,1.248869,1.245245|0.337925,0.331004,0.321988|1.248869→0.331004|1.309879→0.337925|−73.4957% /−74.2018%|
| Unreachable first |8.188676,8.155814,8.176380|2.063711,2.028663,2.058425|8.176380→2.058425|8.188676→2.063711|−74.8247% /−74.7980%|
| Explicit HTTP proxy |0.017093,0.017521,0.015518|0.015650,0.016312,0.018770|0.017093→0.016312|0.017521→0.018770|−4.5691% /+7.1286%|

Primary repeated request-to-headers median is2.041994→0.000567seconds, including DNS/connection/header work rather than RTT. The first request still pays the original setup cost. Cross-port preference repeatedly selects the address that worked on the other port and is pending on the requested port. The proxy slowest also exceeds the frozen5% control allowance, by1.249ms. Neither threshold is changed after measurement.

## Requests, connections and bytes

Every formal batch has4HTTP requests,4accepted live sockets,4MiB served and4MiB exact committed output. There is no extra application payload, HTTP retry or speculative probe. Fixture backlog-fill sockets are fixed infrastructure, not client attempts. The original request-level error counters are `[0,0]`: successful connector fallback does not surface its losing attempt as a failed HTTP request.

| Scenario | A pending attempts/batch | B pending attempts/batch | Total client TCP attempts A→B | B cache hits |
|---|---:|---:|---:|---:|
| Primary |4|1|8→5|3|
| Cross-port |2|4|6→8|3|
| Stable |0|0|4→4|3|
| Family6/Family4 |4|1|8→5|3|
| Unreachable |4|1|8→5|3|
| Explicit proxy |0|0|4→4|0|

Here total attempts are accepted live sockets plus distinct observed losing pending endpoints. Cross-port counts include the destination port in the slow-endpoint identity. Cache-hit primary requests add no losing socket; cross-port hits each incur an additional losing connection. Losing-connect native failure codes are not exposed by successful reqwest fallback and remain unobserved. `tcp_retransmitted_bytes=null`; pending retries or served bytes are not kernel TCP retransmission measurements.

All formal terminal records have active0/open_sockets0/pending_now0/failure=null. Rust waits for the server child and reader thread; the fixture joins actual accept/connection/observer workers and closes listeners/fillers. The observer confirms process exit with `GetExitCodeProcess`, not memory-query success.

Peak bytes over authoritative formal trials:

| Scenario/version | Client private | Server private | System committed |
|---|---:|---:|---:|
| Primary A |3825664|11563008|35162300416|
| Primary B |3035136|11759616|34932359168|
| Cross-port A |2453504|11603968|35184599040|
| Cross-port B |3272704|11665408|34986844160|
| Stable A |1597440|11608064|34976157696|
| Stable B |1601536|11800576|34977165312|
| Family6 A |3833856|11735040|34896175104|
| Family6 B |3833856|11771904|34891874304|
| Family4 A |3821568|11706368|34884984832|
| Family4 B |4259840|11603968|34884804608|
| Unreachable A |3895296|11550720|34990174208|
| Unreachable B |4247552|11706368|34969272320|
| Proxy A |1605632|11743232|34846883840|
| Proxy B |1593344|11558912|34843906048|

Samples are100ms, beginning at measurement start and continuing through process exit, including outside-timing validation/cleanup. Short stable/proxy batches may have only one sample. These are process-window peaks, not allocation totals or physical PlatformHost memory evidence. System commit records concurrent workstation pressure rather than candidate allocation attribution.

## Lifecycle, route and TLS controls

- Original pending-connect cancellation: LIST1/RUN1/PASS1 for A and B. After250ms real pending TCP, original transfer cancel returns Cancelled/0bytes, no task artifact/final file/HTTP request; actual pending socket exits. Cleanup including fixture finish is37.379ms A/38.683ms B; retained implementation37.041ms. No detached probe or late request.
- Actual expiry/new client: explicitly selected ignored test LIST1/RUN1/PASS1 per version,0ignored at execution. Wait31seconds after real successful learning; third request again pays>1second setup. Fresh captured client also begins with no winner. Four exact outputs each, original30second timeout unchanged. Candidate total37.33seconds versus current39.37seconds includes the fixed outside-pair31second wait.
- Explicit HTTP proxy: all paired trials produce4absolute-form proxy requests and0origin DNS resolutions/cache hits. Original Network/engine snapshot contracts provide the retained policy-update boundary; the prototype does not install a second resolver/route owner.
- TLS/CONNECT/SNI: one nonzero LIST/RUN selector covers Direct and explicit proxy. Each route uses a temporary owned self-signed certificate: Strict rejects before HTTP/final publication; explicit AcceptInvalid completes an exact1MiB file. Each has2TLS accepts,2matching SNI observations,1TLS rejection and1HTTP body. Proxy has2CONNECT requests,0origin resolver calls and0origin-canary connections. Server/reader/workers join and private PEM temporary directory is removed. No trust-store or system-proxy mutation.
- Retained source final selector `http::connection_experiments::ref16:: --skip trial`: LIST4/RUN4/PASS4/exit0,0skip/not_run. It covers same-family fallback/exact output, unchanged cross-port ordering, pending cancellation and TLS/CONNECT/SNI/proxy. First-pass coverage is not labeled behavioral RED.

## Decision and retained boundary

The primary gain exceeds the frozen50%/250ms gate, but cross-port cost is a material regression and proxy slowest exceeds its control allowance. **Retain-current**. Preserve original DNS order and existing address-family racing. No extra port-aware cache, socket probes, connector wrapper, dependency upgrade or production feature flag is added. Rejected cache/expiry prototype source and both binary inputs remain in evidence/scratch; only original-connector regression coverage remains in product tests.

## Reproduction and evidence

[Evidence inventory](evidence/ref16/files.txt):493unchanged raw/source/fixture/driver files. `b6-ref16-pairs.json`, `b6-ref16-crossport-pairs.json`, raw `*-list.log`, `*.log`, `*.json` and `*.memory.json` preserve commands, all values, environment pressure and actual exits. Baseline/candidate executable copies remain scratch-only, not release artifacts.

```text
cargo test --locked --offline -p nxtools-download --lib ref16_current_same_family -- --list
cargo test --locked --offline -p nxtools-download --lib ref16_current_same_family -- --nocapture --test-threads=1
cargo test --locked --offline -p nxtools-download --lib http::connection_experiments::ref16:: -- --list --skip trial
cargo test --locked --offline -p nxtools-download --lib http::connection_experiments::ref16:: -- --nocapture --test-threads=1 --skip trial
```

Paired runs invoke each frozen executable with `http::connection_experiments::ref16::ref16_trial --exact --ignored --nocapture --test-threads=1` and `B6_SCENARIO=primary|cross_port|stable|family6|family4|unreachable|proxy`. `b6-ref16-pairs.py` and `b6-ref16-crossport-pairs.py` retain the exact serial orders; `b6-measure.py` provides nonzero identical LIST/RUN/resource observation. Cargo setup is jobs1/incremental0/codegen-units256/scratch TEMP/TMP, with lane/commit preflight.

## Limitations

Controlled loopback HTTP/TLS/writer evidence is not an Internet-speed, physical GUI/native-memory or installed/Release acceptance result. Request-to-headers includes more than TCP establishment. The prototype tests explicit per-client scope and actual expiry; production policy-generation/cache integration was not installed or certified. Retained Network/engine caller regressions cover the unchanged policy snapshot behavior. Kernel retransmission and losing-fallback native error codes are unobserved. The coordinator accepts the retain-current decision at the recorded source/owned-fixture/TestPlatform boundaries.
