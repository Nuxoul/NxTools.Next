# Download SQLite ownership

Evidence: `repo:rust/crates/nxtools-download/src/storage.rs#SqliteDownloadStore`, `repo:rust/crates/nxtools-download/tests/storage_contract.rs`, `repo:rust/crates/nxtools-download/tests/engine_contract.rs`.

Mutex<Connection> owns schema version15 and parameterized transactions; reject newer schema, set WAL only when needed, bounded open retry, quarantine zero-byte file only without WAL/SHM. Commit before memory/events; writer sync before checkpoint, close/rename before Completed. Redownload marker/queue/transfer operations remain store-atomic. No UI SQL shortcuts. Migration/reopen/crash-window tests use temp store. [Engineering](../../shared/engineering.md), [verification](../../shared/verification.md); retain scenarios below. Controlled browser task/queue/receipt admission is one transaction; [Held/Commit/Abort contract](../../browser-extension/controlled-handoff.md) governs every start route and restart.

Optional `DownloadRequest.source_page` is flattened into existing `download_tasks.request_json` and selected-task detail; no schema change or bulk backfill is needed. Missing legacy metadata stays None and is omitted on serialization so existing receipt fingerprints/replay remain byte-identical. Page provenance does not identify remote bytes: `resolution_resource_fingerprint` clears it, while submitted URL, redirect probe/final URL, frozen Referer/Cookie and user-edited filename/destination retain their separate owners. The framed BrowserHost-to-engine SQLite-reopen test and controlled acceptance identity contract cover this boundary.

## Scenario: Redownload and compact snapshot consistency

### 1. Scope / Trigger
- Trigger: changes to retry/redownload, download snapshot projection, browser task actions, or event sequence recovery.
- Purpose: make replacement tasks crash-recoverable and keep compact UI/browser projections behaviorally equivalent to full state.

### 2. Signatures
- `save_redownload_task_with_transfer_and_queue(old_task_id, replacement, transfer, queue, eligible)` creates the replacement and durable operation marker in one transaction.
- `complete_redownload_operation(old_task_id, replacement_task_id)` removes the marker only after old-task cleanup and state removal succeed.
- `DownloadTaskSummary.final_path` and `DownloadCommandOutcome::Accepted { task_id }` are required compact/browser projections.

### 3. Contracts
- Startup reconciles every pending redownload marker; cleanup failure leaves both marker and retryable ownership intact.
- Owned media cleanup validates both targets and removes the mux temporary artifact before the fragment directory, so a later failure does not destroy the checkpoint first.
- Compact snapshot filtering preserves status/name/source/category/final-path search semantics; summary projection preserves final path for list/search/detail handoff.
- Desktop rejects event sequence values `<= last_event_sequence` before mutating progress; an authoritative snapshot clears the live progress cache.
- Browser `task_action` returns the replacement task ID for redownload and the original ID for same-task actions.

### 4. Validation & Error Matrix
- Existing idempotent/replacement marker -> return the existing replacement ID, never create another task.
- Cleanup failure -> typed retryable storage failure; do not remove the old task or marker.
- Missing replacement during startup reconciliation -> retain marker and log a bounded reconciliation event; do not silently claim completion.
- Stale event sequence -> discard; sequence gap -> refresh authoritative snapshot.

### 5. Good/Base/Bad Cases
- Good: process dies after replacement commit but before old cleanup; next engine open reconciles exactly one replacement.
- Base: compact list carries bounded summary fields and selected task detail is loaded separately.
- Bad: create replacement, delete old task, and rely on in-memory ordering to recover a crash window.

### 6. Tests Required
- Restart in the redownload crash window and assert one replacement task and durable marker reconciliation.
- Obstruct each media cleanup target independently and assert checkpoint ownership is preserved.
- Assert browser redownload response echoes the replacement ID.
- Assert stale progress is discarded and compact final-path search matches the full snapshot contract.

### 7. Wrong vs Correct
#### Wrong
```rust
let replacement = submit_new_task()?;
remove_old_task(old_id)?;
```
#### Correct
```rust
let replacement = store.save_redownload_task_with_transfer_and_queue(old_id, ...)?;
cleanup_old_owned_artifacts(old_id)?;
store.complete_redownload_operation(old_id, replacement)?;
```

## Scenario: Durable average transfer statistics
### 1. Scope / Trigger
Completed task rate and persisted/compact task recovery.
### 2. Signatures
`DownloadTransferStatistics { transferred_bytes: u64, active_download_ms: u64 }`, `bytes_per_second() -> Option<u64>`; `DownloadTask::transfer_statistics()`. Optional summary/wire pair; schema13 adds nullable measured_payload_bytes/active_download_ms, all-or-none nonnegative SQL constraint.
Evidence: `repo:rust/crates/nxtools-download/src/transfer_statistics.rs`, `repo:rust/crates/nxtools-download/tests/average_contract.rs`, `repo:rust/crates/nxtools-download/src/storage.rs`.
### 3. Contracts
Count successful payload writes (including retries/ciphertext), not final output size or fragment count. Monotonic RAII worker interval union counts parallel workers once; excludes probe/queue/pause/zero-worker backoff/mux. Milliseconds round up once per control, resumed quantization <=1ms. Capture pair on original task/transfer/artifact transaction before publishing; compact projection/reconstruction carries it. Missing legacy pair stays unknown, new task known zero; zero duration returnsNone. Abrupt interrupted recovery or uncheckpointed artifact adoption invalidates reliability instead of dividing all bytes by suffix-only time. Orderly pause/resume/reopen retains matched pair; no new byte-tick logging.
### 4. Validation & Error Matrix
Unknown prefix/crash window/overflow -> unknown average, not fabricated. Task transaction error -> no Completed/state/pair leak. Existing best-effort terminal failure invalidates uncommitted pair and logs one bounded correlated native/SQLite code without payload. Newer schema -> existing reject, never downgrade.
### 5. Good/Base/Bad Cases
Good: two concurrent workers active3seconds produce one3second interval; pause day adds0. Base: completed record without timing displaysdash. Bad: created_at wall age, sum worker times, resumed suffix time with whole artifact bytes, publish before durable commit.
### 6. Tests Required
average_contract real localhost HTTP/HLS/DASH/AES/retransmission/pause/restart/store/compact/legacy11; average_ controlled union/RAII/overflow/migration/rollback/crash6; failed_completion_transaction_never_leaks_completed_into_memory; full engine_contract serial and Desktop actual completed rate cell. Installed network timing remains separate.
### 7. Wrong vs Correct
Wrong: `final_size / (completed_at-created_at)` includes pause and omits retransmitted/ciphertext payload.
Correct: checked durable transferred-byte / active-union-time pair; missing reliability staysNone.
