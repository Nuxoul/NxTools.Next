# Download quality and boundary regressions

Evidence: `repo:rust/crates/nxtools-download/tests/engine_contract.rs`, `repo:rust/crates/nxtools-download/tests/domain_contract.rs`, `repo:rust/crates/nxtools-download/tests/http_fixture_contract.rs`.

Preserve request/task/plan/artifact identity, synced checkpoints and cancellation/publication ordering. Use actual HTTP/SQLite/writer/child-process seams with narrow injections, not predicate-only proof. Full suite failures remain recorded until causal repair and integrated rerun. The dated audit includes the 2026-10-09 socket correction; live FFmpeg/network timing is separate. Follow [verification](../../shared/verification.md) and [engineering](../../shared/engineering.md); retain scenarios below.

## Scenario: HTTP per-connection segment planner and durable recovery

### 1. Scope / Trigger
HTTP segmented transfer scheduling, live tail splitting, interrupted bodies, retries, pause/resume and final publication. Evidence: `repo:rust/crates/nxtools-download/src/engine/segment_planner.rs`, `repo:rust/crates/nxtools-download/src/engine/segment_workers.rs`, `repo:rust/crates/nxtools-download/src/engine/segmented_runner.rs`, `repo:rust/crates/nxtools-download/src/http/segmented.rs`. [Approved behavior](../../../../.trellis/tasks/11-12-download-fault-recovery/solution.md) supersedes the old HTTP adaptive grow/shrink and stop/join-before-split rules. Requested adaptive settings stay serialized unchanged but do not control HTTP segmentation; no UI, IPC, setting, or schema migration is introduced (current schema14 remains).
### 2. Signatures
`SegmentPlanner::{new,assign,claim,commit,release,room,mark_synced,segments,active_workers,receiving_workers,is_complete,checkpoints,progress_segments}`; `Claim::{Granted,Lost}`; `Assignment { index,from,to,written,preempted,split }`. The pure per-run planner owns half-open ranges; `segmented_runner` supervises connection workers, `segment_workers` owns each connection loop, and `http/segmented.rs` owns HTTP body/overlap handling.
### 3. Contracts
- Start from the persisted plan: `cursor = start + downloaded_bytes`, `reserved = synced_cursor = cursor`, `end = end_byte + 1`; caller supplies contiguous ranges. `cursor` is written, `reserved` is claimed but not necessarily written, and `synced_cursor` is acknowledged durable. Assign first an unowned waiting range (prefer `start == after`); next, after `preempt_after`, take over an adjacent assigned range that has received no data (previous worker's next claim returns `Lost`); otherwise split the largest unclaimed tail, first on ties. Split only if half of `end - reserved` is at least12KiB, align `reserved + half` down to4096 if doing so leaves at least12KiB on the left; new index is max+1. Original worker continues. A worker claims from exactly `reserved` before any asynchronous disk write; limit the claim to `end`, mark receiving on positive claim, commit only actually written positions clamped to `[cursor,reserved]`, release abandoned claims by setting `reserved = cursor`, and release ownership at `cursor == end`. No split touches claimed bytes.
- Within the short planner lock, shorten the original range and add its tail; outside it, serialize plan persistence separately from claims. Persist the shortened original range with its current durable checkpoint and the new segment before the new worker sends its request; persistence failure is fatal. Do not hold the planner or progress/range locks across network, disk or SQLite IO; SQLite takes its own lock for the transaction. Keep progress and range maps paired under progress→range lock order; publish actual written prefixes, not reserved bytes. A worker reaching a shortened end abandons the excess response body without stopping the original connection to coordinate the split.
- Every2s, capture committed positions, release the planner lock, perform one unified artifact sync, then mark only those captured positions synced and save one checkpoint from `synced_cursor - start`. Do not checkpoint merely reserved or unsynced written bytes, including commits occurring during the sync. On pause, cancel, fatal error or completion, drain all worker writes and sync actual writers before the final checkpoint and task state transition. On resume reconstruct the planner from durable segments and keep strict If-Range/resource identity; retain original artifact, queue and final no-replace publication owners.
- Every resumed range request starts up to64 already-written bytes earlier; compare that overlap against disk and treat `FileChanged` as fatal. Flush received bytes through claim→write→commit before returning on body error, short body, slow retirement or cancellation. A new attempt resumes from the committed written cursor, with overlap, rather than the beginning of the run.
- Release the assignment before error handling. `Throttled` is HTTP429/503 with optional Retry-After; other non-2xx responses are `Status`, and only a 2xx non-206 segmented response is `RangeNotHonored`. 401/403/404/410/416 mark a dead link; throttling,403 and connection-stage errors are refusals; storage/IO, range/length/resource/overlap failures and cancellation are fatal. Slow retirement is not a failed attempt. For other errors, reset the per-connection failure budget after written progress; allow5 retries after the initial attempt. Retry immediately on first failure, then wait1/2/4/8/16seconds (cap30s). Honor Retry-After seconds or HTTP-date up to300s with cancellable wake. Retire a refused excess worker at a learned cap while others work; dead-link or exhausted nonfatal workers may retire while peers continue. Fatal errors cancel and drain everyone. The supervisor starts `min(learned cap, requested connections, ceil(size / 64KiB))` workers (at least one), stops retries on completion, and when all workers exit incomplete restarts one if no terminal error remains, otherwise returns the last error. A single excess worker's nonfatal exhaustion must not fail a viable task.
- Check slow responses no more than once per500ms: after2s, if the response speed is less than one-quarter of the best response speed observed from samples of at least256KiB, and the remaining range would take over2s at that speed, flush and reconnect without counting a failure. Task, global or runtime limiter activity disables this path. Preserve correlation/task, bounded lifecycle/terminal diagnostics and native codes at their owning boundaries; never log URL, headers, cookies or byte ticks.
- Measure retirement speed using response elapsed minus actual writer and limiter awaits. Keep the best-response yardstick on total elapsed, so buffered bursts never inflate peer expectations. A writer await of at least50ms gives all workers in that receive state a2s retirement grace; true slow reads remain eligible after the grace. Body stall deadlines exclude actual local waits only. Empty periodic flushes do not refresh a dead network. Preserve the256KiB batch and existing sync/checkpoint cadence; no deferred durability or larger buffer.
### 4. Validation & Error Matrix
Lost assignment -> no further claim/write by the old worker. Claim beyond shortened end -> only available bytes, then stop reading that response. Plan-persist failure -> no new tail request and fatal drain. Overlap mismatch or unsynced checkpoint -> never publish complete. Body drop with peers still working -> resume from written bytes rather than fail immediately. Retry exhaustion with no viable worker -> terminal last error, not an infinite respawn.
### 5. Good/Base/Bad Cases
Good: one slow connection is retired while other connections split its unclaimed tail; pause drains writes and saves only synced bytes, resume produces the exact file. Base: a failed response preserves the written prefix and retries with bounded overlap. Bad: cut through an outstanding claim, sync/checkpoint under the planner lock, issue the new range before plan persistence, or count a retired excess worker as global failure.
### 6. Tests Required
`planner_*` tests cover assignment priority, timing, min split/alignment, protected claims, Lost, release, clamp, synced checkpoint and tiling. `repo:rust/crates/nxtools-download/src/engine/fault_recovery.rs` owns `body_drop_resumes_from_written_offset`, `throttle_learns_cap_and_retires_extra_connections`, `retry_after_is_honoured_and_cancellable`, `exhausted_connection_retires_while_others_work`, `slow_connection_is_shed_and_tail_is_split`, `split_while_writing_never_overlaps_and_survives_pause`, and `overlap_mismatch_is_fatal`. Retain actual HTTP/SQLite/writer and pause/cancel assertions plus `b4_coverage_contract`, `engine_contract`, `http_client_contract`, `b5_completion_contract`, `resolution_contract`, `storage_contract` and Download lib coverage. These are required gates, not claims of passing runs.
- `repo:rust/crates/nxtools-download/src/http/disk_pressure_tests.rs` drives the production planned-segment loop over localhost HTTP with per-artifact test-only writer delay. It covers disk-induced stall/slow-retirement rejection, exact bytes, pause during write with a synced owned prefix, and genuine body timeout despite periodic flushes. `segmented_tests.rs` verifies shared pressure expiry and task/global/runtime-limit exclusions.
`repo:rust/crates/nxtools-download/src/http/segmented_tests.rs#partial_write_failure_never_replays_the_batch_into_an_adjacent_owner` injects a real writer prefix followed by IO failure, then executes exit cleanup: the failed batch is consumed once, adjacent bytes remain intact, and written/synced checkpoints stay at the original position.
### 7. Wrong vs Correct
Wrong: stop and join an active writer to split its small tail, checkpoint its unsynced cursor, or classify every non-206 response as RangeNotHonored. Correct: claim-bounded live split; persist before the new request; sync and checkpoint only captured committed positions; classify and retire errors at the owning connection boundary.

## Scenario: DASH fragment reconciliation and interactive probe
### 1. Scope / Trigger
Changes to DASH fragment adoption or HEAD/Range probe cooperation; neither uses the HTTP segmented planner. Evidence: `repo:rust/crates/nxtools-download/src/engine/dash_runner.rs`, `repo:rust/crates/nxtools-download/src/http.rs`.
### 2. Signatures
The existing DASH fragment checkpoint/adoption and `HttpDownloadClient::probe_timed` boundaries retain their separate owners; HTTP planner assignments do not replace them.
### 3. Contracts
Completed DASH fragments must be nonempty and match their checkpoint length; mismatches re-fetch. Valid uncheckpointed crash-window adoption remains, without claiming equal-length hash integrity. HEAD+Range start concurrently; after a usable successful response (length, valid range or recognized media), collect complementary headers for350ms. Empty200headers must still await the other attempt; failed-first retains other/fallback. SkippedRange never proves resume. Safe routed/certificate-policy client reuse invalidates on network-policy generation, has30sTTL, and probe attempts retain10sdeadline. Strip reqwest URL payloads from errors.
### 4. Validation & Error Matrix
Wrong-size Completed fragment -> reset/re-fetch. Empty first probe -> await the companion or fallback; skipped Range -> no inferred resumability. Probe cancellation -> no task or artifact publication.
### 5. Good/Base/Bad Cases
Good: corrupted Completed DASH fragment re-fetches and a usable Range probe merges timely HEAD metadata. Base: valid checkpoint resumes in one GET. Bad: trust nonempty wrong-size fragment or accept an empty first probe as complete metadata.
### 6. Tests Required
Corrupt DASH reopen re-fetches, valid checkpoint resumes with one GET, and original HEAD/Range cancellation and fallback contracts remain required. Winsock fixtures explicitly make accepted sockets blocking and bound request headers/write_all; do not relax production retry policy to repair fixtures. The dated engine-contract closeout remains historical evidence: `docs/audits/2026-10-08-sol-review-remediation.md`.
### 7. Wrong vs Correct
Wrong: treat an incorrect fragment length or an empty first probe as completed. Correct: re-fetch a bad fragment and wait for usable complementary probe metadata within the bounded grace period.

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

## Scenario: Explicitly paused source replacement
### 1. Scope / Trigger
Ordinary download source changes that reuse existing bytes. Evidence: `repo:rust/crates/nxtools-download/src/engine/paused_url.rs`, `repo:rust/crates/nxtools-download/src/storage.rs`.
### 2. Signatures
`DownloadPausedUrlReplacement { task_id, operation_id: Uuid, generation: u64, expected_revision: u64, expected_source_url: String, source_url: String }`; `DownloadCommand::ReplacePausedUrl`; `CancelPausedUrlReplacement { task_id, operation_id }`. `DownloadTaskDetail.paused_url_revision` and `DownloadEngine::paused_url_replacement_revision(task_id)` expose the original task's current token. `replace_paused_url_with_control(replacement, correlation_id, DownloadResolutionControl) -> Result<(), DownloadEngineError>` uses the original probe and task-owned operation lease. `SqliteDownloadStore::save_paused_url_replacement(task, transfer, plan)` commits existing rows without a schema change.
### 3. Contracts
Require explicit Paused, original user_paused admission, no queue eligibility/transfer control/media, matching engine generation/source, current expected_revision and fresh nonnil attempt ID. Only acquired admission consumes a checked task-owned u64 revision before HTTP; retirement retains it. Reject exhaustion rather than wrap. Engine generation bounds nonpersisted freshness across reopen. Candidate request changes URL only; original frozen headers, Cookie, Referer, certificate/route policy and artifact remain. Network runs outside scheduler/task/store locks. Compatibility requires known equal total, Range support and exact original validator equality. `ResourceValidator::if_range` selects a valid quoted strong ETag, including `""`, otherwise a valid Last-Modified; blank/missing/weak ETag does not override the date. Admission, actual If-Range and response validation share this selection. Recheck request/lease/admission under original scheduler→task→store ordering. Commit task URL, transfer probe and plan final URL in one transaction; segment rows/checkpoints/queue/artifact are untouched. Publish memory and TaskChanged only after commit. Only the acquired lease owner retires it after physical probe completion. Resume cannot admit during an active candidate. Task cancel/remove cancel the candidate; candidate cancel never cancels the task. Diagnostics carry correlation/operation/task/generation/elapsed/fault/native codes, not payloads.
### 4. Validation & Error Matrix
Wrong state/admission/generation/source/revision/replayed ID -> original InvalidState without HTTP or mutation. Equal size alone, missing/blank/weak-only identity, changed validator or no Range -> ResourceChanged with old recoverable state. Probe/deadline/cancel/SQLite failure -> original typed fault and unchanged request/plan/artifact. Cancel ACK -> request observed, not drained. Original explicit Redownload is the fresh-byte route.
### 5. Good/Base/Bad Cases
Good: same strong resource at a new URL, same plan/checkpoints and final exact bytes after reopen/resume. Base: refusal leaves original source resumable. Bad: clear artifact on failed validation, reuse same-sized different bytes, publish before SQLite, or let a rejected duplicate retire another attempt's lease.
### 6. Tests Required
`b4_paused_url_contract` crosses real HTTP, original writer and SQLite: compatible/reopen, changed/same-size resources, missing/weak/Last-Modified validators, Range/probe failure, transaction rollback/no false publication, cancellation/socket drain, duplicate/concurrent resume, stale generation/deadline, queue admission and task cancel/remove. Actual resumed If-Range must preserve the written suffix for blank/whitespace/weak ETag plus date and legal quoted-empty ETag. Exact saved A→B→A rejects before HTTP and preserves all old facts; fresh C resumes exact bytes. Message tests require token roundtrip and reject omitted freshness. Original pipe draining and production native-close tests hold actual probes, distinguish ACK from terminal/socket/lease drain and retain original bytes. LIST/RUN use identical filters; TestPlatform is not physical/native visual acceptance.
### 7. Wrong vs Correct
Wrong: authorize reuse from equal length/Range and overwrite task URL before validation. Correct: reserve the original task's candidate, prove validator compatibility, atomically persist metadata and only then publish the unchanged-byte plan.

## Scenario: Actual written-range progress projection
### 1. Scope / Trigger
Ordinary segmented writer progress, live unclaimed-tail splits and paused/reopened task detail. Evidence: `repo:rust/crates/nxtools-download/src/message.rs`, `repo:rust/crates/nxtools-download/src/engine/segmented_runner.rs`, `repo:rust/crates/nxtools-download/src/engine/segment_planner.rs`, `repo:rust/crates/nxtools-download/tests/b4_coverage_contract.rs`.
### 2. Signatures
`DownloadSegmentProgress.start_byte: Option<u64>` is serde-default/omitted when absent; existing `total_bytes` is segment length and `downloaded_bytes` is actual written count. Original `segment_lengths: HashMap<u32,(u64,u64)>` carries `(start,length)` alongside original writer count map. `command_task_detail` returns original persisted plan segments.
### 3. Contracts
Initial and split offsets come from the actual plan, never index or aggregate bytes. Writer callbacks publish actual committed counts through original event coalescing. Live splits update the old and new ranges and both range/count maps together under progress→range lock order before the new tail request, while the original worker continues; no network, disk or persistence under these map locks. Checkpoint only synced prefixes after writer sync; active prefix ends at `start + min(written,length)` and allocation or an outstanding claim contributes no written span. Unknown/missing range metadata does not prove zero/completed. UI reads TaskDetail outside render on open/status boundaries, then consumes cached persisted facts or original live events.
### 4. Validation & Error Matrix
Missing start/length -> unknown location. Unknown total -> no overall coverage fraction. Writer count beyond length -> clamp presentation, not extra bytes. Live split -> current nonoverlapping plan bounds; persisted pause/reopen -> same synced prefixes. Read failure -> original detail diagnostic/error, no invented range.
### 5. Good/Base/Bad Cases
Good: parallel writers continue through a live tail split with true start/length and exact final output. Base: paused SQLite plan hydrates without any render query. Bad: segment_index multiplied by guessed chunk length, reserved bytes painted as completed, or a new tail count paired with an old range.
### 6. Tests Required
Real `b4_coverage_contract` requires completed exact fixture bytes, multiple workers, a live tail split and partial progress before asserting serialized true offsets, bounded written counts and nonoverlapping current ranges. Desktop `b4_ref10_` covers persisted pause/reopen, original cached-detail replacement, sparse/overlap/missing/unknown/long/viewport behavior and retained controls. Keep restart/checkpoint, request-context and task-detail suites; HTTP adaptive growth/shrink-only tests are superseded by the fault-recovery planner and worker tests above.
### 7. Wrong vs Correct
Wrong: publish a new count with an old range or restart the original writer for a split. Correct: update both maps atomically under the established order and publish only written prefixes while the original connection continues.

## Scenario: Persistent completion intentions and accepted rounds (B5)
### 1. Scope / Trigger
Task/queue completion intentions, real success coordination and normal engine drain. Evidence: `repo:rust/crates/nxtools-download/src/engine.rs`, `repo:rust/crates/nxtools-download/tests/b5_completion_contract.rs`.
### 2. Signatures
`DownloadCompletionIntention { action:Option<DownloadCompletionAction>, open_directory:bool }`; actions Shutdown/Sleep/Hibernate/ExitApplication. Task command SetCompletionIntention; queue command SetCompletionIntention { queue_id,intention,baseline:Option<_> } and AssignTasks { queue_id,task_ids }. Command12/Hello5. Schema14 adds download_queues.completion_intention_json DEFAULT '{}'; task intention/attempt remain original request metadata. `completion_snapshot() -> Result<DownloadCompletionSnapshot,DownloadEngineError>` exposes generation/work_revision/tasks/queues; each member carries task_id/status/attempt/actual queue_id/intention/committed directory. `shutdown_and_wait()` and `shutdown_and_wait_correlated(Uuid)` return `Result<(),DownloadEngineError>` after original owned futures retire.
### 3. Contracts
Missing intention/attempt fields default empty/0. Store before memory/event; queue baseline merges only changed leaves in the original transaction, then publishes the existing SnapshotRequired state invalidation. Snapshot holds original scheduler→tasks→store order and uses stored membership, not compact UI metadata. Genuine admission/retry/Blob/assignment/remove/cancel advances checked accepted-work revision; duplicate receipts/probes/drafts do not. Attempt advances on first admission/retry, not presentation or pause/resume. AssignTasks preserves eligibility/user_pause and rejects active transfers; Move remains ordering only. The original worker JoinSet registers transfer/probe/resolver/managed-tool futures under the stop gate. Drain closes admission, invalidates wakes, requests original pause/cancel controls and awaits actual futures outside locks. Reaped/current join failures return failure only after all handles retire; retained tasks/artifacts are not deleted.
The original scheduler gate retains one session-local destructive membership revision per queue; snapshot queue_invalidations and each task's accepted_revision preserve this authority across event coalescing. Successful removal or assignment out of a queue invalidates its original round after the SQLite commit. Same-queue assignment and failed transactions do not. Initial dispatch/pause/resume cannot restamp acceptance; a genuinely later accepted task/retry can establish a new round. These facts are not persisted or replayed after engine restart.
The coordinator's initial snapshot seeds current nonterminal membership and remembers its cutoff as the observation baseline; existing terminal rows remain history. First dispatch advances an already seeded member's attempt 0→1 only while its queue/cutoff remain unchanged, without restamping acceptance. A subsequent destructive cutoff change aborts that live round independently of intermediate UI observations.
SetCompletionIntention rejects Completed/Failed/Cancelled at the task boundary before store, memory or events. A durable retry acceptance emits the original TaskChanged even if its queue is stopped. completion_snapshot projects raw Failed plus stored eligible admission as Waiting for the newly accepted attempt; original scheduler status, queue stop, checkpoint/artifact and idempotent admission remain unchanged.
### 4. Validation & Error Matrix
Storage failure -> original typed fault/previous state. Overflow -> CompletionOverflow without ambiguous authority. Draining -> no new admission/refill. Missing membership/failed/cancelled/waiting/scheduled -> never success. Completed plus final_committed -> actual final-directory fact; old Completed on reopen -> observed history, not execution authority.
### 5. Good/Base/Bad Cases
Good: accepted nonempty all-success round coalesces once. Base: close/reopen restores intent without changing execution. Bad: empty all(), exhausted failure, removed member or old terminal reload authorizes power.
### 6. Tests Required
Original SQLite defaults/persistence/rollback and concurrent-leaf merge, duplicate receipt revision, actual multiqueue assignment/reload, real HTTP writer success and synchronous drain/postdrain rejection. Coordinator/UI/typed-native and original pipe-stop coverage remain separately required. Keep B4 request context/validator/revision/coverage and original scheduler/media/rate tests.
Hold the production Desktop event pump across accept→remove and move-out→move-back, then real writer/SQLite success: neither aborted round authorizes. A later accepted all-success round authorizes once. Hold terminal observation across a public intention command: terminal rejection cannot arm old success. Stopped-queue retry cancels the old countdown through its accepted event, remains Waiting without HTTP, then real queue start/success authorizes the same new attempt once; duplicate retry and stale callbacks do not change authority.
### 7. Wrong vs Correct
Wrong: progress-window bool owns intent; close ACK proves resource exit. Correct: original durable task/queue metadata, generation-scoped successful rounds, one consumed countdown and separately joined owners.

## Scenario: Host-scoped learned connection ceiling for HTTP segments
### 1. Scope / Trigger
Segmented HTTP extra-connection refusals and ephemeral host/route observations. Evidence: `repo:rust/crates/nxtools-download/src/engine/connection_ceiling.rs`, `repo:rust/crates/nxtools-download/src/engine/segment_workers.rs`, `repo:rust/crates/nxtools-download/src/engine/segmented_runner.rs`, `repo:rust/crates/nxtools-download/src/http.rs`. The approved [fault-recovery solution](../../../../.trellis/tasks/11-12-download-fault-recovery/solution.md) supersedes the native-only B6 REF-13 classification; its historical experiment outcomes remain immutable.
### 2. Signatures
`DownloadEngineInner::connection_ceiling(&HttpDownloadClient, final_url: &str)` captures one run observation; `HttpDownloadError::Throttled { status, retry_after }` distinguishes429/503 and `Status(403)` plus connection-stage errors count as refusal. The existing per-run Arc and ephemeral256-entry owner are retained. No command, setting, or schema change (schema14 remains).
### 3. Contracts
Use the final URL's **host only** as the server key, not scheme or port, plus captured policy generation, route, certificate policy and unique rebuilt System-client identity; System route text alone cannot identify its proxy. Expire an observation600s after its last qualifying refusal; retain the original30s System-client reuse expiry. Every worker in one run shares the captured Arc even after bounded-table eviction. Positive payload claims establish receiving workers, not admission or headers. On each429/503/403/connect refusal with another assigned worker, observe max(current receiving workers,1) and retain the maximum proven width. Two refusals enable the cap; higher actual receiving width can raise it. Observe each eligible failure once, including exhaustion. Initial admission uses min(cap,requested connections,ceil(size/64KiB)), at least one; extra refused workers retire when working peers meet the cap. Changed route/policy/certificate/System-client scope or expiry restores normal admission. Requested limits remain lower authorities; serialized HTTP adaptive settings do not drive the planner. Runtime learning never rewrites persisted requested width or plan.
Receiving width comes from the planner's currently owned positive-payload assignments, not surviving response leases: a completed response syncing its file must not inflate the learned cap. `repo:rust/crates/nxtools-download/src/engine/connection_ceiling.rs#synced_response_lease_is_not_an_extra_receiving_connection` protects that boundary.
`download.connection_ceiling.learned` records original correlation/task, proven width, `ttl_seconds=600`, `reason=refused_extra_connection_with_working_peers`, elapsed and `status` (HTTP status or native code); no URL, headers, cookies or per-byte logs. Bounded diagnostics do not change scheduling or errors.
### 4. Validation & Error Matrix
One refusal or no other assigned worker -> no learned cap. Throttled429/503, Status403 and Client errors classified by is_connect() qualify with working peers; ordinary401/404/408/425/500, body/resource-change/stall/pause/cancel failures alone do not train a cap. Learning never changes route or certificate validation. A lower explicit cap still wins; changed/expired scope restores normal admission. Before first payload, width1 is allowed only when another worker remains assigned. Terminal exhausted evidence may train a cap without another attempt.
### 5. Good/Base/Bad Cases
Good: a server allowing two receiving connections sends503 to excess workers; two observations teach host width2 and a later task on another port uses two initial workers. Base: unrestricted and lower-cap tasks retain exact output and original lifecycle. Bad: learn from every5xx, include port in the host key, infer receiving from headers, permanently lower settings, or fail the whole task when only an excess worker retires.
### 6. Tests Required
`engine::connection_ceiling::tests` covers refusal classification, two observations, max width,600s expiry, scoped host identity and Arc eviction. `repo:rust/crates/nxtools-download/src/engine/fault_recovery.rs` contains `throttle_learns_cap_and_retires_extra_connections` for real503, exact output and same-host/new-port reuse; `retry_after_is_honoured_and_cancellable` and `exhausted_connection_retires_while_others_work` cover cancellation and worker lifecycle. Update `connection_experiments::b601` and `ref13_*` to the new classification and host key; retain their real listener/SQLite/worker joins and the Download lib, engine/HTTP/resolution/storage/artifact/rate/B4/B5 gates. The earlier B6 experiment outcomes record their own historical contract, not the current acceptance rule.
### 7. Wrong vs Correct
Wrong: learn only from native10061, refuse to learn from503, or key the learned ceiling by scheme/port. Correct: repeated typed extra refusals with another assigned worker update a short-lived host-and-scope observation; capped admission and nonfatal worker retirement preserve the original durable task owners.

## Scenario: Unexpected web pages at HTTP body admission

### 1. Scope / Trigger
An expected binary download receives a login, waiting or challenge page. Evidence: `repo:rust/crates/nxtools-download/src/http/web_page.rs`, `repo:rust/crates/nxtools-download/src/http.rs`, `repo:rust/crates/nxtools-download/src/http/segmented.rs`.

### 2. Signatures
`ContentGuard::new(&DownloadRequest, &DownloadProbe, &Response, bool) -> Result<ContentGuard, HttpDownloadError>`; `accept(&mut self, &[u8], bool) -> Result<(Vec<u8>, usize), HttpDownloadError>`; `HttpDownloadError::UnexpectedWebPage` maps to `DownloadFaultCode::UnexpectedWebPage`.

### 3. Contracts
The original response/write boundary rejects declared HTML and a bounded byte-zero HTML prefix before writing it. Known binary filename/MIME expectations drive admission; intentional HTML, legacy Office HTML exports and unknown generic data remain valid. No extra request or whole-response buffering. A full200 page returned to a resumed Range must preserve all saved bytes. Other responses retain original status/Range/validator/length checks. `UnexpectedWebPage` is typed, nonretryable and survives SQLite reopen; command negotiation carries the added fault variant. Original task, writer drain, synced checkpoints and no-replace publication remain authoritative.

### 4. Validation & Error Matrix
Expected binary plus declared/prefix HTML -> nonretryable UnexpectedWebPage without final artifact. Intentional HTML/Office or generic unknown data -> original writer. Other status/Range/length/validator failure -> original typed error. Browser queue/native/timeout/disconnect failure -> original notification owner and retained failed window.

### 5. Good/Base/Bad Cases
Good: exact ZIP bytes, including segmented output. Base: intentional HTML and old Office HTML exports. Bad: full200 waiting page on a resumed binary Range; no saved byte may change.

### 6. Tests Required
`repo:rust/crates/nxtools-download/tests/web_page_contract.rs` crosses actual local HTTP, engine, artifacts and SQLite reopen: truthful/false/missing MIME, fragmented prefix, segmented rejection, resumed full200 preserving a synchronized prefix, intentional HTML/Office exports, short binary and exact normal segmented bytes. The production failed-window regression `web_page_failure_opens_browser_once_and_retains_the_failed_window` requires an official Small outlined browser button, deferred original Shell dispatch, pending-click coalescing, terminal result/timeout/disconnect handling and unchanged failed task/window identity. No automatic browser launch or task restart.

### 7. Wrong vs Correct
Wrong: publish a login page under a binary filename, retry it automatically, or close the failed window after launching a browser. Correct: reject before writes with a typed fault and retain the original task/window while an explicit deferred Shell action opens the source.

## Scenario: Media finishing progress and stops

### 1. Scope / Trigger
HLS/DASH fragment concatenation, dual-track FFmpeg muxing and final publication. Evidence: `repo:rust/crates/nxtools-download/src/engine/hls_runner.rs`, `repo:rust/crates/nxtools-download/src/engine/dash_runner.rs`, `repo:rust/crates/nxtools-download/src/media.rs`.

### 2. Signatures
`DownloadProgress::finishing: Option<DownloadFinishingProgress>` is additive event-only data; stages are Concatenating, Muxing and Publishing, with completed units and an optional total. `DownloadEngine::{pause,cancel}` forwards stops to the original active HLS/DASH control during Finalizing.

### 3. Contracts
Concatenating uses actual written bytes and fragment lengths. Muxing uses FFmpeg out_time_us and known selected-track duration, never transfer bytes. Publishing and unknown totals are indeterminate; presentation stays below100% until Completed. Preserve the100ms event gate with forced stage/terminal samples. FFmpeg uses -nostats/-progress pipe:1, bounded incremental line storage, original bounded stderr, stop polling, timeout, kill/wait and both reader drains. HLS/DASH copy in bounded1MiB chunks with stop checks; close writers before owned-artifact cleanup. Pause retains durable fragments, cancellation removes them, and original no-replace publication remains authoritative.

### 4. Validation & Error Matrix
Known positive total -> phase ratio. Unknown/zero total -> indeterminate. Child nonzero exit/timeout -> original typed failure with native/exit diagnostics. Pause/cancel -> no final publication or partial merged output. No additional checksum, persistent schema or notification owner.

### 5. Good/Base/Bad Cases
Good: advancing media clock before Completed and exact merged bytes. Base: ordinary download byte counters stay unchanged. Bad: present received bytes as completed mux work or reject an HLS stop before its original merge control sees it.

### 6. Tests Required
`repo:rust/crates/nxtools-download/tests/engine_contract.rs` covers child-to-engine progress, exact mux output, unknown event totals and child stop paths; `repo:rust/crates/nxtools-download/tests/finishing_contract.rs` crosses real HTTP, HLS finishing, pause/resume/cancel and owned artifacts. `media::tests` covers real-child timeout and bounded-output failures. Production GPUI finishing/taskbar tests preserve transfer bytes, named stages, retained media controls and indeterminate unknown totals.

### 7. Wrong vs Correct
Wrong: estimate finishing progress from network bytes or publish after a stop. Correct: report the current stage's real units and settle its original writer/process owner before cleanup or publication.
