# Download engine contract v1

Status: frozen for download phase 0

Last updated: 2026-07-29

## Ownership

The Rust download host is the only owner of download task state and the download SQLite database.
GPUI, CLI, browser and C# submit versioned commands and consume results/events. They never open the
download database or execute protocol transfers.

The Rust contract types live in `nxtools-download`. JSON uses snake-case tags and
`protocol_version = 1`.

## Commands

Every command is wrapped in:

```text
DownloadCommandEnvelope {
  protocol_version,
  request_id,
  origin: ui | cli | browser | internal,
  command
}
```

Supported command families:

- `probe { request }`
- `submit { request, mode: start_now | queue_only }`
- `task { task_id, action }`
- `batch { task_ids, action }`
- `queue { action }`
- `set_global_speed_limit { bytes_per_second? }`
- `snapshot { query }`
- `subscribe { after_sequence? }`

Task actions in v1 are `pause`, `resume`, `cancel`, `set_speed_limit { bytes_per_second? }` and
`remove { delete_artifact }`. Protocol clients, segment writers and database details must never be
added to this interface.

Batch actions are `pause`, `resume`, `cancel`, `remove` and `remove_with_artifact`. Duplicate task
IDs are processed once and the typed result contains one success or fault for every distinct target
in request order. Valid targets are not rolled back because another target was rejected.

Queue actions move a stable multi-selection to `top`, `up`, `down` or `bottom`, change the
positive maximum task concurrency, or replace a queue's daily start/stop schedule. Moving selected
tasks preserves their relative order. Priority does not preempt active transfers; lowering
concurrency applies to later admission instead of forcibly pausing an active task.

A one-time request schedule is an absolute UTC Unix-millisecond admission time. A future task is
durable but ineligible until the Rust engine reaches that time; a restart retains the due time.
Manual resume clears the delay and manual pause cancels it. A daily queue schedule uses local
wall-clock minutes, selected weekdays and independent start/stop enablement. Its window is
start-inclusive and stop-exclusive, and supports crossing midnight; automatic stop therefore
applies exactly at its configured local minute. It pauses active work without marking it as
manually paused, so the next scheduled start can resume it.

For both speed-limit commands, a positive byte-per-second value enables the limit and `null`
restores unlimited transfer. Zero is rejected rather than being given two meanings. The request ID
is retained as the correlation ID for persistence, live limiter update and diagnostics.

Each result repeats the request ID and is either accepted, a per-target batch result, a changed
queue, a probe result, a snapshot, or a machine-readable rejection. Transport delivery is not proof
that a command was accepted.

A probe result returns final URL, suggested file name, optional size, range support, content type
and resource validators. Probing does not create a task ID.

`start_now` persists an eligible task and starts its transfer worker when its queue has a free slot.
While waiting for admission it remains `Created`. `queue_only` persists an ineligible task in
`Created` without network work; a later `resume` makes it eligible. Missing `mode` on an older v1
payload defaults to `start_now`.

A snapshot contains `generation`, last `sequence`, tasks, queues and `aggregate_metrics`.

## Request contract

Phase 1 accepts HTTP and HTTPS only. A request contains:

- source URL;
- save directory and optional single-component file name;
- ordered request headers;
- optional Cookie and Referrer;
- optional connection limit from 1 through 64;
- optional positive per-task speed limit in bytes per second;
- optional absolute UTC Unix-millisecond schedule timestamp.

Header names/values, Cookie and Referrer reject CR/LF injection. The request is an intent; it does
not become a task until submission is accepted.

## Task state

```text
Created -> Probing -> Waiting -> Downloading -> Finalizing -> Verifying -> Completed
                         ^            |
                         |            v
                       Paused <-------+

execution state -> Failed / Cancelled
```

Normative rules are enforced by `DownloadTask::transition_to`:

- completed and cancelled tasks are terminal;
- created tasks enter transfer through probing; an eligible created task may become paused before
  admission, while failed and paused tasks re-enter through waiting rather than directly through a
  network connection;
- queue and schedule are admission facts, not transfer states;
- finalizing may go directly to completed when no verification is requested;
- probing, downloading, finalizing and verifying recover as paused after interruption;
- waiting remains waiting across restart.

## Events and recovery

Every event published by `DownloadEventSequencer` has a strictly increasing `sequence`. A subscriber
may request events after a known sequence. If it observes a gap, it must request a full snapshot; it
must not apply later deltas to an unknown base.

Progress events may be coalesced by the host. State changes, completion, failure and command
rejection are not droppable progress samples.

The in-process Phase 1A host has one owning event subscriber. `DownloadSnapshot.tasks` remains the
stable persisted task list; `task_details` is an additive runtime projection containing progress,
fault, owned-artifact paths and `queue_eligible` for first-party renderers. Callers must obtain both
through the versioned `Snapshot` command rather than inferring admission from transfer status or
reading engine storage directly.

Restored transfer plans pass through the same constructor as newly probed plans. Segment indices
must be unique, inclusive byte ranges must not overlap, and a known resource length is an exclusive
upper bound for every segment. Invalid persisted plans are rejected instead of entering the engine.

## Fault codes

Stable v1 fault codes:

- `invalid_request`
- `not_found`
- `invalid_state`
- `storage`
- `network`
- `protocol`
- `permission`
- `disk_full`
- `file_in_use`
- `cancelled`
- `unavailable`
- `internal`

The display message may improve without changing the code. `retryable` expresses whether repeating
the same intent can plausibly succeed without user correction.
