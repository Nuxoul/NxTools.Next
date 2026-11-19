# B6 — Connection experiments

Status: **accepted at source, owned local HTTP/TLS/SQLite/writer/pipe and production TestPlatform boundaries**. [Coordinator acceptance](coordinator-acceptance.md) closes B6-01 after37independent behavior selections/842successful executions. B1–B5 retain their recorded authority; B7 is unstarted. UIA/native-memory and package/install delivery remain separately owned.

## Decisions

| REF | Decision | Measured basis | Retained cost / boundary |
| --- | --- | --- | --- |
| REF-13 — Learned connection ceiling | **adopt** | Four-task refusal sequence `[2,2,2,2]`→`[2,0,0,0]`; all TCP attempts32→26, HTTP requests24unchanged.18formal trials and actual expiry/route/recovery/lower-cap/pause/resume/cancel/proxy controls pass. | Primary completion median+4.7208%, slowest+4.5378%, within frozen5%; throughput1.436145→1.371404MiB/s. Benefit is fewer refusals, not throughput. |
| REF-14 — Eligible first-GET reuse | **retain-current** | Existing full fallback GET only; immediate median5.041251→4.422997seconds, requests7→3. Required10second confirmation delay causes incomplete candidate bodies3/3.36formal trials:33PASS/3requiredFAIL. | No extra GET, preconfirmation download or retained-response product holder. Current probe/cancel releases actual unconfirmed sockets without artifact. |
| REF-15 — Slow-connection retirement | **retain-current** | Six scenarios/36formal trials pass. Original released-slot split precedes candidate qualification; candidate retirements0in every formal/warmup run. Persistent median5.797766→5.780608seconds,−0.2959%, below15%gate. | Original durable split/checkpoint and limiter ownership retained; original capped buffered-but-not-durable overlap is recorded, not credited as a win or TCP retransmission. |
| REF-16 — Multi-address connection selection | **retain-current** | Locked stack's original same-family HTTP setup gap is real. Hostname preference primary median8.172699→2.050162seconds, but port-swapped median4.107963→8.173587seconds,+98.9693%; pending connects2→4.48formal records/42authoritative selections. | Original300ms address-family racing and DNS order retained. Port-swapped TCP attempts6→8; proxy slowest+7.1286%also exceeds frozen5%. No winner cache, probe sockets or connector wrapper remains. |

Each REF uses a separately frozen single candidate, identical load/output success criteria, at least three serial AB/BA/AB pairs and complete byte verification outside timing. Raw trials, warmups, median/slowest, requests/retries/sockets/served overlap, memory samples and actual resource exits are preserved in the individual results:

- [REF-13](ref13-outcomes.md), `evidence/ref13/files.txt`.
- [REF-14](ref14-outcomes.md), `evidence/ref14/files.txt`.
- [REF-15](ref15-outcomes.md), `evidence/ref15/files.txt`.
- [REF-16](ref16-outcomes.md), `evidence/ref16/files.txt`.

## Original owners and final implementation

Only REF-13 changes the production algorithm:

- `rust/crates/nxtools-download/src/engine.rs`: original operation-client capture and engine-owned ephemeral observation table.
- `rust/crates/nxtools-download/src/engine/connection_ceiling.rs`:256entries, initial600second maximum within captured final authority/policy/route/TLS/System-client scope; actual first payload progress and two qualified typed pre-header refusals. Lower original caps win. System-client recapture after its original30second reuse window may retire the scope sooner.
- `rust/crates/nxtools-download/src/engine/segmented_runner.rs`: original initial/refill/split/insufficient-tail admission and adaptive limits; every original worker spawn receives the same run Arc, so bounded table eviction cannot separate feedback from admission. Original retry/error priority and stop/join/sync/Waiting-checkpoint/persist-before-respawn remain.
- `rust/crates/nxtools-download/src/http.rs`: original request/redirect/proxy/TLS/client ownership and typed native connect refusal classification. Response observation/resolver seams are test-only; production has no rejected candidate path.

`engine/resolution.rs`, original HTTP connector, adaptive/segment/checkpoint/artifact, queue and completion owners remain authoritative. There is no second controller, scheduler, transport, persisted ceiling setting, feature flag, dependency/vendor upgrade or UI/theme change. Production diagnostics use the original correlation/task plus one bounded learned-state event with fixed reason/width/elapsed/native code; no URL/path/Header/Cookie/credential or per-sample payload log.

Retained tests use original engine and HTTP seams with owned bounded Python fixtures. `tests/resolution_contract.rs` asserts the entire effective request including the exact B4 default User-Agent; product Header policy is unchanged. [Original changed paths](changed-paths.txt), [repair paths](b601-changed-paths.txt) and [repair evidence](b6-01-repair.md) separate source ownership from concurrent work.

## Verification

[Verification](verification.md) records37fresh independent repaired-source selections and842successful executions, all nonzero identical LIST/RUN, exits0, FAIL/SKIP/not_run0. This includes Download lib143, engine105, HTTP26, resolution15, B4 URL11/Header3/coverage1, B5 completion9, owner/caller libraries and all ten Desktop TestPlatform selections. Overlapping selectors are not842distinct tests.

REF-13 retains genuine native-refusal and bounded-cache worker RED→GREEN. The B6-01 combined-sequence regression adds identical-test/fixture RED→GREEN and independent repaired execution: learned width1, terminal10061, three attempts, exact4MiBpeer range and actual joins. Independent nine-package all-target check, three affected-owner strict lints, eleven-file format and five source/lifecycle gates pass. [Independent repair evidence](evidence/coordinator-reacceptance/files.txt) retains99materials; frozen performance records remain unchanged.

## Known issues and evidence limits

Strict Clippy remains nonzero only for three unchanged historical diagnostics: Protocol `archive_browser.rs:563` (`unnecessary_map_or`), Desktop `browser_handoff_impl.rs:436` (`nonminimal_bool`) and `settings_task_rem_tests.rs:614` (`unwrap_used`). Actual logs and B6-start source attribution are retained without suppression or unrelated repair. No new B6 lint remains.

Controlled local HTTP/TLS/SQLite/writer/pipe and TestPlatform proof is not general Internet performance, physical GUI/UIA/PlatformHost memory or installed/Release acceptance. Kernel TCP retransmission remains unobserved/null; application served overlap is reported separately. Rejected prototypes do not certify unimplemented product lifecycles. Untouched pre-existing untracked files have start-inventory/edit ownership, not a complete before-byte snapshot.

B6 is accepted with B6-01 closed. REF-13 adopts fewer-refusal learning; REF-14/15/16 retain current behavior. No B6 blocker remains at the accepted levels. The coordinator changed no product source and performed no package/install/service/Registry change, product/reference GUI, real power/current Desktop exit, commit/push or B7 dispatch.
