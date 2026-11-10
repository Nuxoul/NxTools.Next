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
- HEAD+Range start concurrently; after a usable successful response (length, valid range or recognized media), collect complementary headers for350ms. Empty200headers must still await the other attempt; failed-first retains other/fallback. SkippedRange never proves resume. Safe routed/certificate-policy client reuse invalidates on network-policy generation, has30sTTL, and probe attempts retain10sdeadline. Strip reqwest URL payloads from errors.

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

## Windows junction fixture rejection (2026-10-12)
`category_directory_cannot_escape_through_a_real_junction` must exercise the real isolated junction boundary and assert no admitted task/no outside Nested directory. On this workstation canonicalization can fail earlier with native448 (ERROR_UNTRUSTED_MOUNT_POINT), a safe OS rejection rather than application InvalidCategoryDirectory. Accept only that exact Io.raw_os_error()448 alongside the original typed rejection, not arbitrary IO failure. Production traversal policy is unchanged; full engine_contract103/103 rerun passes. Evidence: `repo:rust/crates/nxtools-download/tests/engine_contract.rs`.

Foreground policy uses `DownloadEngine::set_runtime_speed_cap(Option<u64>, Uuid)` on existing shared limiter; desired manual value/persisted SQLite and task caps remain unchanged. Actual ceiling is minimum enabled manual/runtime;None removes only runtime and restores latest manual state.0is rejected, identical calls log nothing, reopen discards runtime only. Required `repo:rust/crates/nxtools-download/tests/runtime_speed_cap_contract.rs`8tests cover persistence/concurrency/failure/no-diagnostic boundaries; Desktop owns foreground/config, not engine window polling.

## Scenario: Interactive probe completion without unusable first-response wins
### 1. Scope / Trigger
HEAD/Range/manifest latency, cancellation and routed client reuse changes. Evidence: `repo:rust/crates/nxtools-download/src/http.rs`, `repo:rust/crates/nxtools-download/tests/http_client_contract.rs`, `repo:rust/crates/nxtools-download/tests/resolution_contract.rs`.
### 2. Signatures
`HttpDownloadClient::probe_timed(&DownloadRequest) -> Result<(DownloadProbe,ProbeTimings),HttpDownloadError>`; `usable_probe_response(&Response)`; ProbeTimings adds head_companion_skipped/range_companion_skipped. Existing DownloadResolutionOperation check owns cancellation.
### 3. Contracts
Both attempts share immutable routed/certificate client. A successful length/validRange/recognizedmedia response may collect the companion for350ms; empty200 must still await necessary metadata. Firstfailure awaitsother; bothfailure retains plainGETfallback. TimelyHEAD-onlyfilename retained; overduefuture dropped, no detached latefilename mutation. SkippedRange cannot claim resume.10sprobeattempt/30stransferstall unchanged; manifests keep boundedbody/readtimeouts. Clientcache generation/route/certificate/30sTTL unchanged. Existing correlated accepted/completed/failed carries fixedstages/ms/skipflags, noURL/header/cookie/endpoint.
### 4. Validation & Error Matrix
SlowHEAD + usableRange -> boundedcompletion. SlowRange + usableHEAD -> conservative noresume. EmptyHEAD -> awaitactualRange length/validator. SlowHLS/DASHbody -> typedfailure, no incompletepublication. CancelafterHTTPstart -> operationretired promptly, no task/artifact. Explicitproxyfailure never fallsbackDirect.
### 5. Good/Base/Bad Cases
Good: delayed200msHEADfilename merged;1sslowHEAD no longer holds usableRange path. Base: redirect/requestcontext/Rangevalidation unchanged. Bad: first200winswithoutmetadata, lowerglobaltimeouts or lateheader overwrites acceptedfilename.
### 6. Tests Required
HTTP26 localcontracts includinglatency/emptyHEAD/slowmanifest; resolution15 inclHTTPstagecancel; Downloadlib117 clientreuse/generation/certificate tests and serialengine103. NetworkrealDirect/Systeminjected/Manual3tests; injectedSystem doesnotproveOSregistry/PAC. Actualexternalresource unspecified, reportremainingreproductionneeded; no browser/resolverrewritewithoutcausalrepro.
### 7. Wrong vs Correct
Wrong: joinbothfullrequestdeadlines or blindlyacceptfirstsuccess. Correct: usablemetadata -> boundedcompaniongrace; otherwise preserveother/fallback, originalimmutablecontext/cancel/publication.

## Scenario: Nonblocking events and no-replace destination recovery
### 1. Scope / Trigger
Primary event dispatch, batch commands, final publication, destination admission and failed-task recovery. Evidence: `repo:rust/crates/nxtools-download/src/engine.rs`, `repo:rust/crates/nxtools-download/src/http.rs`, `repo:rust/crates/nxtools-download/src/engine/browser.rs`.
### 2. Signatures
`DownloadEngine::request_batch_background(Vec<DownloadTaskId>, DownloadBatchAction, Uuid) -> Result<async_channel::Receiver<DownloadCommandOutcome>, DownloadFault>`; command protocol9 adds `DownloadTaskAction::RetryWithAutoRename` and `DownloadFaultCode::DestinationExists`.
### 3. Contracts
Primary256/progress192 streams stay bounded. State send uses try_send; a lost-state flag causes the next emission with capacity to attempt generation/sequence-aware SnapshotRequired before its ordinary event. No send_blocking or spawned backlog; secondary subscribers retain bounded disconnect-on-full behavior. Release task locks before verified state/progress publication. Background Batch uses the original correlated command on the existing blocking executor with a one-result channel; UI owns one-operation admission. Batch diagnostics are start/aggregate terminal, not one log per target.
Final HTTP/media/Blob publication uses the same no-replace move. Windows MoveFileExW has flags0 in a narrowly allowed unsafe function; async finalization awaits spawn_blocking. Existing target returns DestinationExists and preserves both destination and source. Only uncommitted artifacts reserve names, including Failed/Cancelled because they can be retried or redownloaded. Numbered retry uses the existing organization policy, keeps task/checkpoint/temporary bytes and changes only the final filename/path before normal queue admission. Final HTTP/segmented/HLS/DASH output records prepared_size_bytes durably as the existing temporary artifact's prepared state before publication. A restored prepared output is published without probing/refetching its remote source, through the original no-replace completion/verification/cleanup boundary. An incomplete/preallocated file never gains readiness from its length; without the prepared marker, preserve the original checkpoint-to-temporary-path binding.
Numbered destination, cleared scheduled start, transfer state and queue admission are one save_task_with_transfer_and_admission transaction under scheduler/task locks. Rejection preserves live and stored state, checkpoint/prepared identity, and existing name reservations; emit only the normal CommandRejected event. Update memory and publish TaskChanged only after commit. Queue refill failure after committed admission retains accepted intent.
### 4. Validation & Error Matrix
Full primary queue -> bounded state loss plus later snapshot recovery, never caller wait. Full secondary -> original disconnect. Existing destination -> distinct nonretryable conflict; ordinary Retry does not authorize replacement. Wrong state/committed task -> numbered action rejected. AddNumbered picks another reserved/disk-free target; Reject refuses admission. Move failure -> retain temporary bytes.
### 5. Good/Base/Bad Cases
Good:300 removals without a consumer return; after draining, next state is preceded by SnapshotRequired. Base: ordinary successful final move publishes once. Bad: block the only GPUI consumer, transfer backlog to detached tasks, or allow Failed to release a still-resumable target name.
### 6. Tests Required
`batch_remove_without_event_consumer_returns_and_requests_snapshot` verifies deadline, engine emptiness and recovery ordering. Final-commit/Blob tests assert exact destination/source bytes; failed/cancelled AddNumbered/Reject and same-task numbered retry preserve checkpoint identity. `completed_transfer_conflict_auto_renames_without_refetching_offline_source` verifies durable prepared recovery with an unavailable loopback source; `preallocated_incomplete_conflict_never_publishes_as_prepared` rejects length-only readiness. Full lib and serial engine_contract plus IPC handshake/client checks remain required. Desktop lifecycle assertions belong to the owning UI contract.
`auto_rename_retry_admission_failure_rolls_back_name_transfer_and_events` injects a real SQLite admission-update failure for ordinary/scheduled tasks, verifies two rejected attempts preserve memory/artifacts/admission and emit only CommandRejected, then removes the fault and verifies durable first-numbered acceptance.
### 7. Wrong vs Correct
Wrong: `send_blocking(state)` or replacing rename after a one-time existence check. Correct: bounded try_send plus explicit snapshot recovery; one no-replace final move and typed numbered recovery.

## Scenario: Engine-owned Blob confirmation staging
### 1. Scope / Trigger
BrowserOpenBlobDialog, BrowserImportBlob, BrowserDiscardBlob and engine initialization cleanup.
### 2. Signatures
`open_browser_blob_dialog_correlated`; `browser_blob_staging_path`; `cleanup_pending_browser_blobs(root,correlation_id)`. Existing public commands, BrowserBlob metadata and private GPUI source remain unchanged.
### 3. Contracts
Validate exact size/ID at the original staging root, require the private consumer, then no-replace move the completed file into `browser-blob-staging/pending/<id>.blob` before queue delivery/acknowledgement. Import/discard prefer that claimed location, otherwise use the original direct-import path. BrowserHost only expires top-level blob/uploading files; it cannot sweep pending confirmation ownership. A new engine startup reclaims only pending orphans because confirmation windows do not survive the owner process, retaining unclaimed/receiving files. Newly claimed queue rejection/explicit window failure cleans through original discard; consumer-not-ready keeps original upload for retry. Timeout retains the possibly queued window's claim; late confirm/cancel/close or next startup releases it. Reused existing claims are not deleted by a failed repeat admission. Existing no-replace final publication/database compensation returns bytes to the same claimed staging path.
### 4. Validation & Error Matrix
Missing/size mismatch -> original rejection. No consumer -> retryable rejection, no ownership move. Queue/window rejection -> terminal unavailable, bounded cleanup diagnostics. Confirmation older than host1hour threshold -> file remains and imports successfully. Cancel/close -> original discard removes pending file. Cleanup failure -> original native-code event and retained file for startup retry; startup logs coalesced counts/correlation/elapsed, never paths/names/bytes.
### 5. Good/Base/Bad Cases
Good:aged pending confirmation survives subsequent host startup, imports exact bytes; cancelled claim disappears. Base: direct Blob import still uses top-level upload. Bad: host age-deletes accepted confirmation, timeout deletes a late queued owner's input, or startup sweeps in-progress uploads.
### 6. Tests Required
BrowserHost `blob_upload/ownership_tests.rs` crosses actual engine commands/confirmation channel and host startup sweep: aged import, claimed discard, startup orphan-only recovery, consumer-not-ready retry, explicit rejection and timeout→late cancellation. Retain download-lib rollback/no-replace tests, browser engine contracts and actual Desktop confirmation/cancel tests. Synthetic temporary artifacts do not certify installed/native user acceptance.
### 7. Wrong vs Correct
Wrong: confirmation holds only a top-level blob ID under BrowserHost TTL. Correct: engine claims a separate pending artifact before acknowledgement; original import/discard and startup own release.
