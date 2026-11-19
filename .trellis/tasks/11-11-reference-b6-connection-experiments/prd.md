# B6 — Connection experiments

## Goal and scope

Independently evaluate REF-13/14/15/16 against the original download engine and adopt only measured benefits. B1–B5 remain accepted; B7 remains unstarted. B6 proceeds independently of PlatformHost/UIA memory acceptance. Formal packaging, UIA source/verification, installers and release materials belong to other tasks and are read-only here. Shared source/build contention and actual measurement resource pressure are the only execution-window constraints.

## Reference authority

Read docs/reference-code/download.md and actual indexed ABDM sources before Mister sources from docs/reference-code/mister-download-manager.md. Check legacy NxTools compatibility where indexed. Mister/Ghost are behavioral GPL references: do not copy code/comments/tests/assets. Inspect the locked HTTP dependency/connector and immutable network route before proposing a multi-address mechanism.

## Requirements and acceptance by REF

### REF-13 — Learned connection ceiling

Evaluate largest actually successful concurrent width under original host/route identity, initial10minute expiry. Only clear extra-connection refusal supplies evidence; authentication and ordinary HTTP failures do not. Learning never bypasses a lower user/task/host limit. Original admission/adaptation owns any adopted cap, not a new scheduler or persisted setting. Controlled servers measure refusals, requests, throughput, expiry and route-change recovery; preserve pause/cancel and durable rebalance.

### REF-14 — Eligible first-GET reuse

Evaluate only an already issued eligible probe GET's complete response; HEAD and one-byte Range are not whole-file streams. Never issue another GET to manufacture reuse. Response lifetime belongs to original resolution generation and must end on cancel/edit/replacement/close/expiry/failed admission. Retain routing/request-context/redirect/validator and bounded ownership. Test single-use links, delayed confirmation, route changes, stale results, body failure, exact request count and final bytes. If current probe shapes have no useful retained response, record the evidence and retain behavior rather than adding traffic.

### REF-15 — Slow-connection retirement

Evaluate only inside original adaptive/rebalance ownership, with stable samples, enough remaining work and no active manual/task/runtime speed ceiling. Reference age/speed ratios are experiment inputs, not automatically product constants. Stop/join/sync/checkpoint must finish before reassignment; retain actual-error priority and cancellation semantics. Compare slow tails, recovering connections, stable and limited transfers; report completion time, retransmitted bytes, requests and connection churn. Preserve pause/reopen/plan/artifact identity. Incorporate the REF-13 decision before this experiment.

### REF-16 — Multi-address connection selection

Inspect locked stack address-family/connection racing first; add a narrow connector adjustment only for a measured gap. Prefer the existing connector over extra probe sockets/controllers. Preserve immutable DNS/proxy/TLS route, cancellation and error reporting; explicit proxies must not trigger direct-origin probes. Cover slow/unreachable address, family fallback, route change, cache expiry and cancellation, measuring setup time and additional connection cost. Equivalent/better existing behavior yields retain-current.

## Shared measurement contract

Each mechanism is compared separately using equivalent loads and at least three serial baseline/candidate pairs in interleaved AB/BA/AB order. Freeze workload, durations, parameters and decision thresholds before timing. Compile binaries first; no compilation/other benchmark in measured windows. Keep raw trials, median, slowest, requests/retries/refusals/extra connections/retransmitted bytes/peak memory. Verify complete exact bytes outside the timed interval. Do not combine candidates into a single unexplained benefit or claim broad Internet speedup from controlled delays.

Temporary experiment parameters belong in test/scratch harnesses, not permanent feature flags or duplicate runtime paths. Preserve original Rust task/plan/segment/queue/SQLite/writer/event/completion owners, B4 request/validator/revision/template/coverage, limits and Server. No replacement manager/transport/settings store, dependency/vendor upgrade or UI redesign. Native Windows work stays in C#.

## Completion

Deliver a separate adopt/retain-current decision and raw evidence for each REF, with precise changed paths/owners, RED/GREEN where applicable, commands/LIST/RUN counts/exits, resources and remaining physical/native/visual/Release levels. Pure first-pass coverage is not invented RED. Run affected Rust/caller and B1–B5 preservation, strict lint/check/scoped format/source gates with historical unrelated lint separately attributed. C# UIA/native verification remains outside this batch. One Cargo lane and required jobs1/incremental0/codegen-units256/scratch TEMP/TMP apply; timing starts only after build/commit checks are quiet.

No real power/current Desktop exit, product/reference GUI/UAC, packaging/install, service/registry mutation, commit/push or B7. Parent final state is ready_for_acceptance / not_accepted pending independent acceptance.
