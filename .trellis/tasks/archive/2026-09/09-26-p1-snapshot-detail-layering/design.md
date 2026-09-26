# P1 Snapshot/Detail Layering Design

## Decision Boundary

The download engine remains the sole owner of task state, queue state, SQLite reads, and snapshot construction. IPC only transports typed snapshot/detail values. GPUI and CLI consume projections and never open SQLite.

The current `DownloadSnapshot` duplicates a lightweight `DownloadTask` with a full `DownloadEngineTask` projection for every result. The implementation will first measure this shape at 1,000 and 10,000 tasks. Only a material measured reduction authorizes the compact path.

## Data Flow

```text
SQLite / engine task map
  -> command_snapshot(query)
  -> typed snapshot summary + queues + metrics + generation/sequence
  -> IPC
  -> desktop/CLI list projection

selected task ID
  -> command_task_detail(task_id)
  -> one task's probe/path/validator/segments/timeline
  -> desktop/CLI detail projection
```

## Proposed Contract

Introduce a typed `DownloadTaskSummary` containing the fields required by list state and reconciliation: stable task ID, origin, status, created/updated/completed timestamps, downloaded/total bytes, queue eligibility, fault category/retryability, and a bounded display-name/search projection only where measurement and existing list behavior require it.

`DownloadSnapshot` will expose summaries, queues, aggregate metrics, generation, and sequence. Detail-only fields remain in `DownloadTaskDetail`: the full engine task/probe, validator, temporary/final paths, transfer segments, media plan projection, and bounded timeline.

Because the repository keeps protocol v8 compatibility, the transition must use optional/default fields or an explicit legacy projection path. Existing JSON without the compact field must continue to decode. Current desktop and CLI callers must be migrated together so no consumer silently relies on an empty legacy field.

## Compatibility and Invariants

- Summary IDs and status are authoritative for list reconciliation; no row is keyed by index.
- Queue order and admission are unchanged by presentation sorting.
- Snapshot generation/sequence remain unchanged and continue to gate stale-event recovery.
- A sequence/generation gap must be recoverable using only the compact snapshot.
- Detail queries load only the selected task and do not load all plans/timelines.
- No URLs, headers, cookies, credentials, or raw payload secrets are added to diagnostics.
- No second journal/database, Aria2 GID, or sidecar lifecycle is introduced.

## Measurement Method

Use deterministic in-memory or temporary SQLite fixtures at exactly 1,000 and 10,000 tasks. Measure independently:

1. JSON serialization bytes for the current snapshot shape and proposed compact shape.
2. `command_snapshot` wall-clock construction plus SQLite query time using an injected/isolated fixture boundary.
3. Desktop list projection and refresh work from the received snapshot, without rendering screenshots.
4. Selected-task `TaskDetail` query time and serialized bytes for a representative task with a transfer plan and bounded timeline.

Report median and p95 over repeated iterations after warm-up. The benchmark must print stable machine-readable labels and assert only invariants, not machine-specific absolute latency thresholds.

## Rollback Shape

If the compact path regresses list behavior, event-gap recovery, protocol compatibility, or detail access, retain the measured baseline and revert only the compact consumer path. The benchmark and contract fixtures remain useful evidence; do not retain an unproven public-shape change merely to align with Rayburst.
