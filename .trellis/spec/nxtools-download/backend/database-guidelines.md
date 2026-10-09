# Download SQLite ownership

Evidence: `repo:rust/crates/nxtools-download/src/storage.rs#SqliteDownloadStore`, `repo:rust/crates/nxtools-download/tests/storage_contract.rs`, `repo:rust/crates/nxtools-download/tests/engine_contract.rs`.

Mutex<Connection> owns schema version12 and parameterized transactions; reject newer schema, set WAL only when needed, bounded open retry, quarantine zero-byte file only without WAL/SHM. Commit before memory/events; writer sync before checkpoint, close/rename before Completed. Redownload marker/queue/transfer operations remain store-atomic. No UI SQL shortcuts. Migration/reopen/crash-window tests use temp store. [Engineering](../../shared/engineering.md), [verification](../../shared/verification.md); retain scenario below.

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
