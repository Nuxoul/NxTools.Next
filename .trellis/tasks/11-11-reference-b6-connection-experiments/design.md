# B6 independent connection experiments

Status: implementation authorized for B6 only. No UIA or packaging prerequisite applies. [Source review](reference-review.md) records inspected ABDM → legacy → Mister/Ghost and locked connector evidence. Each REF remains open until its actual baseline/candidate evidence supports adopt or retain-current.

## Shared experiment contract

One mechanism changes at a time. Freeze the accepted result of earlier mechanisms for both sides of each later comparison; REF-15 includes REF-13's decision. Preserve separate baseline/candidate builds in scratch, not permanent feature flags or duplicate product paths. Use the original engine/network owners; no new scheduler/transport/settings store and no dependency/vendor change.

For every primary and control workload below, run at least three serial pairs in AB/BA/AB order, equivalent request/action/byte inputs and freshly reset fixture state. A trial starts with two untimed warmups on a separate resource, then measures the stated batch using a monotonic clock. Warmup must not train the measured host/route cache. Each fixed request/transfer has a120second failure deadline; expiry checks and outside-timing byte comparisons are recorded separately. No compiler or competing benchmark runs during timing.

Use owned loopback HTTP/proxy/TLS listeners, bounded connections/threads, GUID-owned SQLite/artifacts and deterministic8MiB/16MiB payloads. Compare every final byte against the independent fixture pattern outside timing, without whole-file buffering or output hashing. Measure client acceptance-to-terminal, transfer-only and setup-to-headers separately; do not label request-to-headers as RTT/Ping. Baseline failures have no successful-completion median; report their terminal latency/error and request costs separately rather than substituting zero.

Raw trial records include mechanism/scenario/order/version/seed, exact command/build inputs, warmup and start/end, final status, elapsed, requests/retries/refusals/max-active/extra connections/retransmitted or extra served bytes, peak client/server private memory and system commit. Sample memory every100ms during short transfers. Keep every raw value, per-scenario median/slowest, actual listener/connection/worker/engine/pipe joins and correctness results. Count TCP attempts at the fixture's actual connect/accept boundary, not just HTTP requests. Server bytes written are identified as served bytes, not kernel-level retransmission; do not invent a TCP retransmit counter.

Current limiter, artifact/checkpoint, source URL/context/validator, queue/task identity, durable-before-publication, B4 Header/reset/replacement/template/coverage/Server and B5 action/Exit ownership stay intact. Production diagnostics contain only correlation/fixed state/reason/count/elapsed/native codes. Fixture logs exclude URL/path/Header/Cookie/focus/credentials and high-frequency production events.

## REF-13 — Learned connection ceiling

### Candidate and owner

Use a private ephemeral table in original DownloadEngineInner/admission. Key by final resource authority plus immutable policy generation/route/certificate context, not host text alone. For System routes invalidate on built-client generation because route_key system does not describe the actual proxy endpoint. Initial expiry is the requested10minutes. Bound entries by the existing resolution receipt capacity256; retire oldest/expired entries on the same engine owner. No persistence or user setting.

Evidence requires two clear pre-header ConnectionRefused native IO outcomes from an additional worker while same-key peers actually receive data. Keep largest actually receiving width; later proven higher width can raise it. Ordinary HTTP statuses (including401/403/408/425/429/5xx), TLS/auth/body/validator/stall errors are not evidence. Typed phase/source error classification, not error-text matching or broad retryability, owns this decision.

Effective admission is the minimum of learned width and all existing lower user/task/host limits. Do not rewrite the persisted requested width/plan or abandon queued ranges. Reuse original retry, pause/join/checkpoint and adaptive owner.

### Fixed workloads and decision

- Primary:8MiB, requested width4, server permits2receiving streams. Each2MiB segment is paced over2.5seconds, longer than the workstation's observed2.03second native ConnectionRefused latency. Pacing precedes each payload write, so completed client bytes do not precede the server's final pacing wait. An owned resolver barrier delays only the initial additional connects until two peers start and the owned listener is closed; all connect outcomes still come from real TCP. Listener reopens when receiving width drops. Four successive same-authority/route tasks train on the first and evaluate the next three. This fixture prerequisite is frozen before valid baseline/candidate timing; the earlier120ms preflight produced no native refusal and is not a behavioral baseline.
- Controls: unrestricted width4; task cap1; auth401/403; retryable HTTP429/503; first connection unavailable with no receiving peer; same host changed route; actual learned expiry. Expiry policy uses controlled clock in functional tests plus one real10minute expiry observation outside the timed batch.
- Adopt only if post-learning same-key refusals fall by at least80percent, every lower-cap/route/expiry/auth rule holds, output/pause/reopen remain exact, and completion median/slowest are no more than5percent worse than baseline in primary/unrestricted controls. Report throughput and any time gain separately; refusal reduction is the primary benefit.

The refusal fixture must expose the real native connect error, not manufacture an HTTP status and call it a connection ceiling.

## REF-14 — Eligible first-GET reuse

### Candidate and owner

The only currently identified opportunity is the existing full fallback GET after HEAD and exact one-byte Range both fail. Do not retain either small probe as a whole file, treat a media manifest as file data or issue a new GET to manufacture reuse. Eligible response begins at byte zero, is a successful complete-file response and carries same-response metadata/route/validator facts.

Keep a one-shot response in the original ResolutionState under operation ID, resolver generation, session generation and exact effective request/context values. Default no-op probes have no extension receipt: retain their existing operation identity through confirmation without fabricating provenance. Real extension receipts retain their original validation and approval rules. Compare existing resource/context fields directly; add no hash/fingerprint solely for this experiment.

At most4responses are retained, reusing the original resolution concurrency bound. Metadata receipts remain at their original256bound. Capacity refusal drops the response and preserves current metadata behavior, without extra pre-confirmation traffic. Response expiry is15seconds after headers, or a real resolver receipt's earlier validity. This bounds the experiment while covering10second delayed confirmation; metadata receipt lifetime is not response lifetime. Use the existing owned engine runtime/future set for expiry/cancel wait, not a new scheduler or OS thread. Drop response/expiry work on original engine drain.

Atomically take a response once after original submit identity/admission checks and durable task/transfer acceptance. Carry the accepted probe facts into the original single-stream runner, bypassing only duplicate probe/first GET. Do not open artifacts or write payload before confirmation. Persist serializable metadata only; response/socket is never serialized or stored in SQLite. Queued/scheduled/non-immediate work cannot hold a response indefinitely: expiry/cancel/stop remains authoritative and normal transfer behavior is retained if reuse is ineligible.

Original edit/replacement/cancel/close/stale-result/route-policy update/expiry/failed admission explicitly retire the operation's response. Add one idempotent operation-retirement command through the existing Download command/IPC owner if adopted; update its version/compatibility contract only where required. GPUI callbacks change local generation/state first and schedule retirement through the original next-turn application/runtime path. No synchronous leased-entity reentry or render IO. Default no-op and extension flows both have tests.

### Fixed workloads and decision

- Primary:8MiB at2MiB/s, HEAD405, Range416, eligible fallback full GET200. Immediate confirmation and10second delayed confirmation each compare exact requests/bytes. Repeatable-body baseline is expected to re-probe and open transfer GET; candidate must need exactly one full GET (three initial HTTP requests total).
- Single-use control: only the first full GET is200; every later full GET410. Report baseline terminal failure separately from candidate successful completion, not a fictitious percentage speedup.
- Retirement/control cases: edit/replace/close/cancel during probe and after headers; stale operation/generation; policy route change;15second real expiry outside timing; receipt expiry; four-response capacity; admission/destination conflict; body truncation; unknown content length; normal HEAD/one-byte Range and HLS/DASH ineligibility. Verify server-side connection closure, no late writer/event and actual engine join.
- Adopt only if eligible repeatable and single-use paths use exactly one full GET with exact output, every invalidation releases within1second of its handled retirement (and expiry within1second of deadline), no ineligible path issues a new GET, existing failure/limit/checkpoint semantics hold, noneligible median/slowest regress no more than5percent and peak client memory increases by no more than8MiB. Benefits are scoped to fallback-GET eligibility, not the common HEAD/Range path.

Evaluate response/body feasibility first in the original HTTP test seam, preserving the real probe, request builder, streamed writer and commit path. A failed required body/lifecycle control rejects this candidate before production resolution/IPC/UI integration. A rejected prototype does not certify unimplemented product invalidation paths; record their verification level separately.

## REF-15 — Slow-connection retirement

### Candidate and owner

Add per-worker cumulative progress/age to the original one-second adaptive tick. Initial experiment inputs are five stable samples, age at least2seconds, speed below one quarter of best proven receiving peer and more than2seconds useful remaining at that peer rate. These inputs are not automatically shipped constants.

Any active manual/task/global/runtime speed limit disables retirement. REF-13's effective width is already fixed for both sides. Allow only one pending retirement identity in original adaptive/rebalance state. Signal existing request_rebalance; only actual joined Rebalanced permits writer sync, durable Waiting checkpoint and requeue of that same remaining range. Do not reassign before stop/join/sync/checkpoint. Completion, real error, pause/cancel and attempt identity retain original priority. No second timer, manager or transport.

### Fixed workloads and decision

- Primary:16MiB, width4, one request-ordinal connection128KiB/s and other/replacement connections1MiB/s;16KiBpaced writes so extra served-byte accounting is bounded and observable.
- Controls: the initially slow connection recovers at6seconds; all four stable1MiB/s; task cap256KiB/s; manual/global/runtime limits256KiB/s; insufficient remaining work; pause/cancel/error while retirement is pending; reopen/resume same artifact/ranges.
- Adopt only if persistent slow-tail median improves at least15percent, stable/recovering/limited median/slowest regress no more than5percent, stable/limited retirements are zero, each retirement adds at most one transfer request and total extra served bytes are at most1percent of payload. All identity/durable-checkpoint/cancel/error and exact-byte tests must pass.

If only the persistent-slow case wins while recovery/churn/cost fails, retain current behavior. Do not weaken controls to justify retirement.

## REF-16 — Multi-address connection selection

### Existing behavior and narrow candidate

Retain locked hyper-util's existing300ms cross-family Happy Eyeballs. No separate TCP probe or competing racing controller. Only a demonstrated same-family gap permits evaluating Direct-route OS-order DNS preference learned from a successful remote_addr. DNS Resolve receives a hostname, not URL port: cache keys are DNS hostname within the original immutable client/policy/TLS scope, maximum256names and30second validity. All other answers retain relative order. Port-change/failure controls quantify this scope's cost. Use public locked resolver/client APIs only.

Do not apply origin preference to Manual/System proxy routes. Their connector still selects the proxy endpoint and preserves CONNECT/TLS/SNI/error/cancel behavior; explicit proxy never probes the origin directly. System fixtures use only scoped owned-child environment/injected existing resolver inputs, no real Registry/PAC/proxy-secret inspection.

### Fixed workloads and decision

- Primary: four sequential forced-new-connection1MiB requests under one captured original operation client, first same-family address connect-stalled and second live. The first request trains only through real success; repeated requests measure preference and its fixed30second expiry. Record the actual batch duration rather than assume all baseline requests finish within one cache period. Close only fixture responses to avoid pool reuse masking setup cost.
- Controls: dead IPv6/live IPv4, dead IPv4/live IPv6, stable first address, changed answer/route, client/cache expiry, cancellation during pending connection, explicit local HTTP proxy and HTTPS CONNECT with zero direct-origin attempts. Preserve TLS accept/reject behavior with owned test certificates and no trust-store changes.
- A loopback alias/listener with bounded backlog saturation supplies the slow-connect preflight without firewall/route/service changes. Prove an actual pending TCP connect in the locked connector, not HTTP headers/body delay. If this prerequisite cannot be established, record an unavailable native fixture, not a successful address-race result; do not replace it with external network traffic or a mocked speed claim.
- Adopt only if repeated same-family setup median improves at least50percent or250ms, stable/cross-family median/slowest regress no more than5percent, cache-hit requests add zero extra sockets beyond the chosen connection, route/expiry/cancel/TLS/error assertions hold and proxy cases have zero direct-origin connects. First-request cost and all failed/losing attempts remain in the record.

Existing behavior without a demonstrated gap yields retain-current, backed by measured controls. An unavailable required workload is an evidence limitation, not measured equivalence.

## Final decisions and retained levels

Each REF outcome records adopted or retain-current, exact original owner/paths, fixed inputs, raw AB/BA/AB trials, median/slowest/cost/peak memory, correctness/resource exits, actual RED/GREEN where applicable and limits. Delete rejected candidate product code rather than ship unused flags/frameworks; retain useful red-capable tests and evidence. No general Internet-speed claim, physical/native GUI acceptance or Release acceptance follows from controlled HTTP timing. Parent handoff remains ready_for_acceptance / not_accepted for independent review, no B7.
