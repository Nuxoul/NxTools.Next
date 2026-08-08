# Diagnostics

NxTools.Next writes daily logs under:

```text
%LOCALAPPDATA%\NxTools\NxTools.Next\data\logs
```

- `desktop.log.YYYY-MM-DD` contains Rust/GPUI/application and IPC events.
- `platform-host.log.YYYY-MM-DD.jsonl` contains structured Windows adapter diagnostics.

## Feature logging contract

Every cross-process user operation carries one UUID correlation ID across Rust, IPC and C#. Search
that UUID to reconstruct one request without relying on timestamps alone. A completed feature slice
must record:

- request acceptance or start at the owning boundary;
- meaningful state transitions rather than UI frames;
- terminal success or failure, elapsed time and actionable native/OS error codes;
- enough stable identity to locate the task or window without logging the user payload.

High-frequency streams are deliberately coalesced. Download byte ticks, pointer movement and
animation frames are not logged individually. They produce state changes, bounded percentage
milestones or rate-limited repeated failures instead. The bounded background sinks ensure that disk
I/O cannot block paste, download or Windows event threads.

Sensitive field names are redacted at the sink. Logs must not contain clipboard contents,
Authorization/Cookie values, full request headers, tokens or passwords.

## Clipboard paste

Every paste request has the same correlation ID in both files. It reconstructs the complete path
from the clicked Rust history item through clipboard write, target activation, modifier recovery,
each `SendInput` key event and the final command result.

The Windows log records window handles, process/thread IDs, clipboard sequence numbers, every
clipboard-write attempt, input structure size and numeric Win32 errors. It records the text length
but never the clipboard text itself; sensitive field names are redacted again at the log sink.
Both desktop and PlatformHost logs are rotated daily and retained for 14 days.

For a failed paste, collect the matching sections beginning with `paste.start` and ending in
`paste.completed` or `paste.failed`, plus the Rust line carrying the same `correlation_id`.

## Completed-file actions

Native file drag uses `file_drag.stage` with one correlation ID. The production lifecycle retains
source validation, input-queue attachment, capture release, OLE entry/callbacks/data exchange,
terminal result and worker exit. A one- and five-second watchdog records a bounded input snapshot
only while OLE remains active. Shell open/reveal commands retain their normal
`command.received`/`command.completed` pair.

## Taskbar progress

Taskbar progress uses `taskbar_progress.applied` for state changes and the
0/25/50/75/100 percent milestones. Identical updates inside one milestone are silent.
`taskbar_progress.failed` records the native error and is rate-limited for identical failures.
The generic per-command lifecycle log is disabled only for this high-frequency command; the typed
taskbar events remain correlated and preserve the feature outcome.

## Segmented download transfer

Each segment request records `download.segment_transfer.started` and exactly one terminal
`download.segment_transfer.completed` or `download.segment_transfer.failed` event. The caller
supplies the download task ID and command correlation ID; the same correlation must be retained
when the engine coordinator and IPC command path adopt the segmented primitive.

Events include only segment index, resumed/completed byte counts, elapsed time and a bounded error
kind. They do not include the source URL, query string, request headers, Cookie or Authorization
values. Network chunks and checkpoint byte ticks are intentionally silent.

The engine adds `download.transfer_plan.created`,
`download.segment_checkpoint.restored` and
`download.segment_checkpoint.persisted` around the durable state boundary. A server that ignores a
validated Range probe produces one `download.segment_transfer.fallback` event only when no segment
byte has been accepted; the fallback retains the original command correlation ID.

For a multi-connection task, filter by the task ID and correlation ID together. One
`download.transfer_plan.created` reports the bounded `segment_count`; each segment then has its own
index under that same correlation. Progress remains a coalesced task event, so logs do not grow
with connection count or network chunk count.

Transient segment failures add one `download.segment_retry.scheduled` event containing the segment
index, failed/next attempt, bounded error kind and delay. A pause or cancel during that delay does
not emit another network attempt. Validation details, URLs, response bodies and credentials remain
excluded.

Transient ordinary or range-fallback single-stream failures use the equivalent bounded
`download.single_stream_retry.scheduled` event. It contains the transfer reason, failed/next
attempt, bounded error kind and delay under the same task/correlation ID, and never records URL,
path, file name, headers or credentials. The resulting single-stream completed, paused or cancelled
event includes total elapsed milliseconds; terminal `download.task.failed` records the worker's
elapsed milliseconds under the same correlation ID.

A no-progress connection terminates with error kind `stalled`; the message includes only the
configured timeout in milliseconds. It is eligible for the same bounded retry lifecycle and never
logs the URL or response body. Pause/cancel winning the wait produces the normal segment terminal
event rather than a misleading stall failure.

An internal dynamic-split stop terminates the worker with outcome `rebalanced` under the existing
`download.segment_transfer.completed` event. This means the writer has durably synchronized its
reported prefix; it does not mean the task or segment is complete.

When a completed worker releases a connection slot, `download.segment_rebalance.requested` records
the selected segment index, bounded remaining-byte count, current worker count and desired worker
count. After the selected writer acknowledges and its checkpoint is durable,
`download.segment_split.persisted` records only the old/new segment indices and their byte ranges.
Both events retain the task command correlation ID. Selection scans, progress samples and chunks
remain silent.

## Download speed limits

`download.speed_limit.global_changed` and `download.speed_limit.task_changed` are emitted only after
the new value has been validated, durably persisted and applied to the live shared limiter. Each
event records the command correlation ID, whether the limit is enabled and the positive
byte-per-second value (zero only when reporting disabled). Task changes also record the task ID.

Permit acquisition, token refill, waiter wakeups and throttle durations are intentionally silent.
No source URL, headers, cookies or transferred payload bytes are logged. A command arriving through
IPC keeps its envelope request ID through validation, persistence, live update and the terminal
event; direct in-process calls create one correlation ID for the same lifecycle.

## Download storage allocation

`download.storage_preflight.accepted`, `download.storage_preflight.skipped` and
`download.storage_preflight.failed` describe the one capacity decision made for a known-length task
attempt. They retain the task and command correlation IDs and include only remaining, reserve,
required and available byte counts plus a bounded reason/native error code. They never include the
source URL, headers or credentials.

`download.artifact.sparse_enabled` is emitted once when the temporary artifact accepts sparse
creation. `download.artifact.sparse_fallback` is emitted once when the file system or platform
cannot honor it and ordinary allocation is used. File create/resize/write/flush/rename failures use
bounded `disk_full`, `file_in_use`, `permission` or `storage` kinds. Allocation calls and write
chunks remain silent.

## Download interruption recovery

At startup, each persisted active task changed to paused emits one
`download.recovery.interrupted_task_paused` event with its task ID and a recovery correlation ID.
One `download.recovery.completed` event under the same correlation records the bounded recovered
task count. Resume then uses the new command correlation ID for checkpoint restoration, network
attempts and its terminal result.

Disconnects use the existing bounded `download.segment_retry.scheduled` events. Exhausting retries
produces one terminal `download.task.failed`; reconnecting and explicitly resuming creates a new
correlated attempt. Server validator changes remain protocol failures and never discard the last
durable segment checkpoint or incomplete artifact.

## Browser download handoff

`download.browser_native_messaging.registration_requested` begins registration after PlatformHost
connects. C# completes it with `browser_native_messaging.registration_completed` or
`browser_native_messaging.registration_failed`; an explicit disable ends with
`browser_native_messaging.registration_removed`. These events include only browser count, elapsed
time, exception type and native error code.

The native host records `browser_native_host.started`, one bounded request outcome, and
`browser_native_host.stopped`. A successful new submission ends with
`download.browser_handoff.accepted`; an idempotent retry ends with
`download.browser_handoff.duplicate_accepted`. Both keep the handoff ID, task ID and correlation
ID but never record the URL, file name, Referrer, headers, Cookie or Authorization value.
