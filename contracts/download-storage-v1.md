# Download storage contract v1

Status: frozen for download phase 0

Last updated: 2026-07-27

## Location and ownership

The product database will live under the NxTools.Next data directory as `downloads.sqlite3`.
Only the Rust download host opens it. CLI, browser extensions, GPUI and C# use the download command
interface.

SQLite `PRAGMA user_version` is the schema version. Version 1 creates:

- `download_tasks`
- `download_segments`
- `download_queues`
- `download_artifacts`
- `download_settings`

Foreign keys are enabled. A file-backed database uses WAL and a bounded busy timeout.

## Migration rules

1. Migrations are ordered, transactional and advance `user_version` only in the same transaction.
2. A database newer than the running binary is rejected without writes.
3. A migration failure leaves the previous schema usable; startup reports a storage fault.
4. Future destructive migrations require a verified backup before the transaction starts.
5. Old NxTools JSON import is a separate idempotent phase-6 operation, not a schema migration.

## Task recovery

Request context is persisted with the task, including Header, Cookie, Referrer, save destination,
connection limit, positive per-task speed limit and schedule. The optional global speed limit is
stored under the typed `global_speed_limit_bytes_per_second` settings key. A JSON `null` means
unlimited; zero is not a valid persisted limit.

On startup:

- `probing`, `downloading`, `finalizing` and `verifying` become `paused`;
- `waiting`, `paused`, `failed`, `completed` and `cancelled` keep their state;
- recovery updates the task timestamp in the same database operation.

The host never reports a task completed merely because all response bytes arrived. Artifact commit
and configured verification must finish first.

Before a known-length transfer creates or resumes its incomplete artifact, the Rust host checks the
destination volume against the remaining durable bytes plus a bounded reserve. If the destination
volume cannot be identified, the preflight is skipped and the real file operation remains
authoritative. A known shortage is `disk_full`; Windows sharing/lock violations are `file_in_use`.

The task owns one logical incomplete file. Sparse creation is best effort on supported file
systems, and failure falls back to ordinary allocation without changing segment ownership.
Persistent progress continues to live only in `download_segments`; sparse allocation never creates
a second progress format.

## Segment and artifact atomicity

Future phase-2 mutations must transact:

- segment split/merge plus its task transfer-plan generation;
- segment progress checkpoints that make a resume claim;
- artifact state plus final task completion.

A process failure may repeat safe work, but must not create overlapping byte ownership or a
completed task whose final artifact was not committed.
