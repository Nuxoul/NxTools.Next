# B5 Download backend outcomes

Acceptance: `not_accepted`.

## Original owner chain

- `nxtools-download/src/lib.rs::DownloadTask` owns task intention and execution attempt. The original custom Deserialize/restore and `storage.rs::StoredResolvedRequest` retain them in request metadata. Missing intention/attempt default to empty/0. Queue-only work stays attempt0; first admission is1, genuine retry increments, pause/resume retains the attempt, committed browser Blob is1.
- `SqliteDownloadStore` schema14 adds `download_queues.completion_intention_json TEXT NOT NULL DEFAULT '{}'`. Queue intention stays with `DownloadQueue`, independent of task-window visibility.
- `set_queue_completion_intention(queue_id,intention,baseline)` merges only changed action/directory leaves inside the original transaction. Missing baseline is an explicit full setter. Failed persistence does not publish the draft. `assign_queue_tasks` changes original task/membership rows atomically and preserves eligibility/user-pause; active transfer assignment is rejected. Move remains order-only.
- Original Task/Queue `SetCompletionIntention` and Queue `AssignTasks` use command12; Download IPC Hello5 retains the original pipe. CLI/BrowserHost share the released constants. Missing fields remain default-compatible.
- `DownloadEngine::completion_snapshot()` locks scheduler → tasks → original store and projects generation, accepted-work revision, tasks and queues. Task facts contain actual stored membership, status, attempt and intention. Directory is present only for Completed with a committed final artifact.
- Accepted admission, genuine retry, membership/cancel/remove and actual worker admission update the original accepted-work revision only after durable acceptance. Drafts, metadata edits, probes, duplicate receipts and failed saves do not. Worker success/failure is separate from admission.
- Successful queue transactions publish the existing `DownloadEvent::SnapshotRequired`; the original Desktop event pump reevaluates authoritative completion facts for both UI and IPC changes.

## Worker retirement

`DownloadEngine::shutdown_and_wait()` delegates to `shutdown_and_wait_correlated(Uuid)`. The original scheduler closes admission, invalidates schedule wakes, requests resumable transfer pause and cancels candidates/probes/resolution. Transfer workers, HTTP probes, filename probes, resolver futures and the existing managed-tool operation share the original retained Tokio JoinSet and stop gate. No second manager is introduced.

Drain takes and awaits actual owned futures outside scheduler/task locks. Completed results reaped during normal admission retain their failure state. All remaining workers retire before a JoinError is returned; a panic is not successful drain. Tasks and artifacts remain stored. Existing managed-tool cancellation uses its original cleanup; no download/install operation is part of this verification.

## Evidence

The focused `b5_completion_contract` contains nine cases: durable task/queue intentions, assignment/reopen, absent-field defaults, failed save, changed-leaf merge, duplicate admission, actual HTTP final publication/directory, worker drain/post-drain rejection, and original queue-event invalidation.

- `b5-backend-pass2`: LIST9/RUN9/PASS9/FAIL0/SKIP0/not_run0, exits0/0.
- Queue invalidation regression: LIST1/RUN1/FAIL1 → GREEN in the nine-case contract.
- Worker JoinError regression: LIST1/RUN1/FAIL1 → `b5-worker-drain-green` LIST1/RUN1/PASS1, exits0/0.
- Production held-probe retirement and real SQLite/HTTP retry/schedule/membership/restart are covered by Desktop lifecycle tests.

Exact commands, final current-source preservation results and log paths are recorded in `verification.md` and `verification.json`. Overlapping selections are not additive coverage totals.

## Limitations

Drain is the normal post-run boundary, not a self-join from a Download worker. Schedule wakes are invalidated; this is not a claim that every process-lifetime runtime thread has been joined. Physical power, current-user Desktop exit, installed/Release and independent product acceptance remain separate.
