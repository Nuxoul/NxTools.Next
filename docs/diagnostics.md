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

Blob handoff adds `browser_native_host.blob_upload_prepared`, then one
`browser_native_host.blob_upload_completed`, `browser_native_host.blob_upload_failed` or
`browser_native_host.blob_upload_expired` data-plane terminal. Commit ends in
`browser_native_host.blob_import_accepted`, `browser_native_host.blob_dialog_accepted` or
`browser_native_host.blob_import_rejected`, followed by the engine terminal
`download.browser_blob.imported`. Cancellation of an unconfirmed dialog uses
`download.browser_blob.discarded` or the idempotent `download.browser_blob.cleanup_completed`.
Filesystem cleanup failures use `browser_native_host.blob_staging_cleanup_failed` or
`download.browser_blob.discard_failed` with the native error code; a database rejection whose
artifact compensation also fails uses `download.browser_blob.rollback_failed`.
Startup cleanup without a live request uses `browser_native_host.blob_stale_cleanup_failed` with
only the bounded artifact kind and native error code. Data-plane write failures additionally carry
a fixed `failure_kind` such as `staging_open`, `staging_write`, `staging_flush` or `socket_read`.
Prepare, upload, commit and dialog cleanup preserve one correlation ID and handoff ID. These events
retain only correlation/handoff/upload/task IDs, bounded byte count, elapsed time and outcome. The
one-time capability/token, loopback URL, page/blob URL, filename, MIME payload and bytes are never
logged. Rejected chunks are coalesced into the upload's single bounded failure/expiry terminal rather
than per-chunk payload diagnostics.

Selected-resource batches use one parent correlation ID from extension to engine. The extension
emits `batch_handoff_started` and one `batch_handoff_completed`; BrowserHost emits
`browser_native_host.batch_started` followed by `browser_native_host.batch_completed`,
`browser_native_host.batch_rejected` or the generic correlated
`browser_native_host.request_failed`; the engine emits
`download.browser_batch.started` and `download.browser_batch.completed`. Batch events contain only
item/accepted/rejected/duplicate counts and elapsed time. Per-item URLs, names, request headers,
cookies and referrers are never logged. Context-menu handoff uses the existing single-item chain and
adds only the fixed event identities `context_menu_handoff_completed` or
`context_menu_handoff_failed`.
Extension diagnostic messages use their own transport request ID so their acknowledgements cannot
complete a pending feature request; the validated feature correlation travels as a separate UUID
field and is the only correlation recorded for correlated extension events.

When the extension requests its legacy confirmation-window behavior and no Desktop pipe exists,
BrowserHost records `browser_native_host.desktop_start_requested`, starts the protected sibling as
`NxTools.exe --browser-dialog --browser-correlation-id <id>`, and records
`browser_native_host.desktop_started` plus
`browser_native_host.desktop_ready`. Spawn failure uses `browser_native_host.desktop_start_failed`
with the native error code; pipe timeout uses `browser_native_host.desktop_ready_timeout`. Direct
submissions, snapshots and task actions use the same bounded events with `start_mode=background`
and are the only paths allowed to append `--background`.

The browser-dialog startup intent prevents the ordinary main window from claiming focus and keeps
it hidden until PlatformHost can issue `SW_SHOWMINNOACTIVE` and verify `IsIconic`. Desktop records
`download.browser_startup.main_window_minimize_requested`, then the observed
`download.browser_startup.main_window_minimized`,
`download.browser_startup.main_window_minimize_failed` or
`download.browser_startup.main_window_minimize_timeout`, including elapsed time and the correlation
ID forwarded by BrowserHost. PlatformHost records the native observation with the same correlation.
The add-download surface still completes foreground ownership through the separate
`download.browser_dialog.foreground_requested` chain; none of these events records URL, file name
or request context.

The native host records `browser_native_host.dialog_accepted` only after the interactive Desktop
has attached its private GPUI intake. A pipe can become available slightly before that subscriber;
`browser_native_host.dialog_consumer_waiting` records this bounded startup state, followed by
`browser_native_host.dialog_consumer_ready` or `browser_native_host.dialog_consumer_timeout`.
Only the engine's “interactive consumer not attached yet” result is retryable; queue delivery,
window creation, malformed and other terminal failures return immediately. The consumer-ready retry
is capped at three seconds so the five-second pipe startup,
three-second subscriber wait and five-second GPUI acknowledgement remain within the extension's
15-second create-task timeout. Rust records `download.browser_dialog.accepted` (or
`download.browser_dialog.duplicate_accepted`), and GPUI completes the correlated chain with
`download.browser_dialog.opened` or `download.browser_dialog.open_failed`. A headless host returns a
retryable unavailable result; BrowserHost coalesces the wait into one start and one terminal event,
so the browser keeps its original download without repeated production warnings. Rust returns
acceptance only after a bounded GPUI completion acknowledgement; a timeout or dropped
acknowledgement records `download.browser_dialog.open_unconfirmed` and is rejected without retry.
After GPUI resolves the real HWND, it records `download.browser_dialog.foreground_requested` with
the same handoff/correlation IDs and the bounded topmost/activation command IDs. If either command
cannot be queued, it records `download.browser_dialog.foreground_request_failed`; it never records
the URL or request context.
The private in-process message is not a serializable `DownloadEvent` because it can carry Header,
Cookie and Referrer values. Direct task state delivery is observable by task ID and event sequence
through `download.desktop.task_event_applied`; snapshot failures use
`download.desktop.snapshot_refresh_failed`.

Page resource discovery uses the rate-limited `browser_extension.event` host event. It retains the
Native Messaging request ID plus a fixed allowlisted extension event identity such as
`resource_cached`; the untrusted extension message, URL, page title, file name, Header, Cookie and
Referrer are not written
to the host log. Repeated capture events remain throttled in the extension and are never emitted per
network chunk or scan candidate.

## HLS probing and transfer

The versioned download command boundary records `download.probe.started` followed by exactly one
`download.probe.completed` or `download.probe.failed` under the command request/correlation ID.
The success terminal carries only the fixed media kind, bounded variant count and elapsed time.
Failure carries a fixed error kind, optional native error code and elapsed time. Manifest text,
playlist or variant URLs, file names, request headers, cookies and referrers are never logged.

The GPUI quality choice is local presentation state and does not create per-click diagnostics. HLS
submission uses a new command correlation rather than the probe correlation. Playlist resolution
records `download.hls_playlist.started` followed by one `download.hls_playlist.completed` or
`download.hls_playlist.failed`; a durable-plan restore uses the same pair with
`source=network_revalidation` because the current finite playlist must remain compatible with the
persisted media sequence, segment count and durations; a changed identity fails before any old/new
segments can mix. Plan creation records `download.hls_transfer_plan.created`. Aggregate artifact
recovery records at most one `download.hls_checkpoints.reconciled` with adopted/reset counts.
Transfer records
`download.hls_transfer.started` and one `download.hls_transfer.completed`,
`download.hls_transfer.paused`, `download.hls_transfer.cancelled` or the existing correlated task
failure. A transient segment retry uses `download.hls_segment_retry.scheduled`; staging cleanup
failure uses `download.hls_cleanup.failed` with a fixed reason, artifact kind, native error code and
elapsed time; cancellation cleanup failure becomes a failed task and retains plan ownership rather
than reporting a false successful cancellation.
Paused/failed cancellation and history removal additionally emit correlated
`download.hls_cleanup.started` and `download.hls_cleanup.completed`; their correlation ID is the
versioned command request ID (or a new operation ID only for a direct in-process API call).
AES-128 playlist activation emits one `download.hls_encryption.configured` event per task attempt
with only encrypted Segment count, distinct key-slot count and explicit-IV count. Key fetch,
ciphertext validation and PKCS#7 failures use fixed error kinds through the existing bounded retry
and task-failure lifecycle. Key URLs, key bytes, IVs, request headers, cookies and encrypted or
decrypted media bytes are never logged.

These events retain only correlation/task identity, fixed source/reason/error kind, bounded segment
and byte counts, connection count and elapsed time. Manifest text, playlist/segment URLs, local
paths, file names, headers, cookies, encryption keys and media bytes are never logged. Successful
segments do not emit per-segment production logs.
