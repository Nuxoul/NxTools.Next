# Download quality and boundary regressions

Evidence: `repo:rust/crates/nxtools-download/tests/engine_contract.rs`, `repo:rust/crates/nxtools-download/tests/domain_contract.rs`, `repo:rust/crates/nxtools-download/tests/http_fixture_contract.rs`.

Preserve request/task/plan/artifact identity, synced checkpoints and cancellation/publication ordering. Use actual HTTP/SQLite/writer/child-process seams with narrow injections, not predicate-only proof. Full suite failures remain recorded until causal repair and integrated rerun. The dated audit includes the 2026-10-09 socket correction; live FFmpeg/network timing is separate. Follow [verification](../../shared/verification.md) and [engineering](../../shared/engineering.md); retain scenarios below.

## Scenario: Durable adaptive shrink and DASH reconciliation

### 1. Scope / Trigger
- Changes to adaptive sampling, segmented writer completion/rebalance, saved plans or DASH fragment adoption.

### 2. Signatures
- `TransferPlan::merge_waiting_segment(&mut self, cancelled_segment_index: u32, absorbing_segment_index: u32) -> Result<(), TransferPlanError>`.
- `segmented_runner` joins/syncs `Rebalanced` workers and persists Waiting before plan merge; `dash_runner` reconciles Completed checkpoint bytes with actual fragment length.

### 3. Contracts
- One sample append per tick; stable shrink evaluated before growth/low-benefit consumption. Saved grown plan uses request-derived initial floor and attempt-local baseline.
- Retain shrink identity through pending clear/join; no growth while shrink is pending. Persist Waiting before durable adjacent merge; preserve prefix and active connection cap.
- Both ranges Waiting, cancelled bytes zero, adjacent. Never prepend an unwritten left range to a progressed right range.
- Completed DASH fragment must be nonempty with checkpoint-matching length; mismatches re-fetch. Valid uncheckpointed crash-window adoption remains, without claiming equal-length hash integrity.
- HEAD+Range are joined; safe routed/certificate-policy client reuse invalidates on network-policy generation, has bounded TTL, and probe requests use ten-second deadline. Strip reqwest URL payloads from errors.

### 4. Validation & Error Matrix
- Missing/non-Waiting/nonadjacent merge or invalid prefix -> typed TransferPlanError, unchanged plan.
- Pause/cancel/retry dominates adaptive changes; no output publication before all ranges complete.
- Wrong-size Completed fragment -> reset/re-fetch, never adopt as completed.

### 5. Good/Base/Bad Cases
- Good: real worker sync -> Waiting checkpoint -> 3→2 merge -> pause/reopen/resume exact bytes.
- Base: valid completed DASH checkpoint resumes without another fetch.
- Bad: clear shrink identity before matching join; append twice/sample; use saved grown count as minimum; trust nonempty corrupt Completed file.

### 6. Tests Required
- `sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact`: real localhost HTTP/writers/SQLite/5 MiB output; 2→3→2 and ceiling 2→1; four slow samples do not shrink, five do; prefix/cap/timeline/resume assertions.
- Domain merge tests reject progressed-right prepend and preserve left prefix; corrupt DASH reopen re-fetches while valid resume remains one GET.
- Winsock fixtures explicitly make accepted sockets blocking; bounded request headers/write_all. Do not relax production retry policy to repair fixtures.
- Download-lib and relevant HTTP/storage/engine contracts; report full-suite failures. Injected samples do not validate external jitter/real sampler.

### 7. Wrong vs Correct
```text
Wrong: pending shrink cleared -> joined worker classified as growth/completion.
Correct: retain identity -> join/sync -> persist Waiting -> durable adjacent merge.
```

Evidence and dated engine-contract closeout: `docs/audits/2026-10-08-sol-review-remediation.md`.

## Scenario: Accepted Windows fixture sockets and native-stage waits

### 1. Scope / Trigger
- Changes to shared DeterministicHttpServer accepted connections or marker-based FFmpeg engine tests. This is test infrastructure, not product retry/probe policy.

### 2. Signatures
- `tests/support/mod.rs::serve_connection(TcpStream, ...)`; `fixture_preserves_accepted_connections_until_delayed_request_bytes_arrive`; original FFmpeg mux-cancel and crash-window resume engine contracts.

### 3. Contracts
- Listener remains nonblocking for bounded shutdown polling; each accepted stream explicitly sets nonblocking(false) before bounded request read. Two-second read/write timeouts prevent blocked fixture IO from hanging a test.
- Delayed first request bytes are not a disconnect. Preserve existing single-transfer409 and If-Range checks; don't weaken them or production retry/probe policies to make tests pass.
- A marker wait also checks task terminal state and reports actual error before timeout. Keep cancel response and staging/output cleanup assertions.

### 4. Validation & Error Matrix
- Accepted socket without initial bytes -> bounded wait, not early WouldBlock10035/drop.
- Failed task before marker -> immediate stage-specific test failure; never a misleading fifteen-second process-start timeout.
- True repeated zero-offset transfer -> fixture409 remains; missing resume validator -> existing428 remains.

### 5. Good/Base/Bad Cases
- Good: delayed accepted socket receives206/exact body and one request for serial/concurrent modes.
- Base: crash-window uncheckpointed tail resumes from durable offset and exact bytes; FFmpeg cancel removes staging/output.
- Bad: inherited listener mode drops HEAD/Range, fallback GET consumes resume eligibility; change product retry or extend deadlines instead of repairing the fixture.

### 6. Tests Required
- Raw delayed-first-byte regression at real TcpStream boundary, both fixture modes, assert response/body/request count; run red before correction.
- Original FFmpeg and crash-window scenarios, bounded focused repeat, then one serial engine suite plus shared HTTP/storage fixture contracts. Do not repeat full suite until accidentally green.
- Remove unique temporary DEBUG probes; distinguish HEAD source comparison from compiled baseline and synthetic media process from real user FFmpeg acceptance.

### 7. Wrong vs Correct
```rust
// Wrong: accepted sockets inherit nonblocking mode; WouldBlock is treated as EOF.
let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
// Correct: restore per-connection blocking semantics before bounded IO.
stream.set_nonblocking(false).expect("accepted fixture blocking mode");
stream.set_read_timeout(Some(Duration::from_secs(2))).expect("bounded fixture read");
stream.set_write_timeout(Some(Duration::from_secs(2))).expect("bounded fixture write");
```
