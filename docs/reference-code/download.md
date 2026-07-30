# Download manager reference implementation index

Last verified: 2026-07-28.

## Scope and authority

| Source | Role | Reuse rule |
| --- | --- | --- |
| `D:\RiderProject\NxTools` | Existing NxTools user-visible behavior and legacy migration format | Preserve useful behavior; do not carry its architecture into NxTools.Next |
| `D:\Code\ab-download-manager-master` | Primary HTTP, queue, destination and CLI source reference | Apache-2.0; concepts or attributed compatible reuse are allowed |
| `D:\Code\FluxDown-main` | Rust engine, event, persistence and dynamic-segment research | AGPL-3.0; clean-room behavioral/architectural reference only, no code copying |
| `D:\RiderProject\NxTools.Next` | Product implementation target | MIT OR Apache-2.0 |

The old NxTools application is the behavior source when the references disagree about existing product behavior.
ABDM is the primary implementation reference for HTTP transfer and CLI interactions. FluxDown is useful for
architecture and failure scenarios, but its AGPL code must not enter NxTools.Next.

## Verified NxTools.Next baseline

- `rust/apps/nxtools-desktop/src/main_window.rs::render_downloads` renders the Phase 1A real task list,
  actions, progress and task details.
- `rust/crates/nxtools-download` owns the domain/message/storage contracts, deterministic HTTP fixture
  and the Phase 1A single-connection HTTP execution path.
- Desktop and CLI call the same versioned command/snapshot seam. The engine retains one bounded
  primary desktop stream and can attach additional bounded event subscribers for IPC watch clients.
- `rust/crates/nxtools-download-ipc` owns the current-user named-pipe transport, handshake and
  typed command/result/event frames. It never opens download storage.
- `rust/apps/nxtools-cli` owns parsing, human/JSON output, stable exit-code mapping and background
  host discovery. It never invokes SQLite or constructs a download engine.
- `docs/architecture.md` already assigns download planning, segmentation, scheduling and persistence to Rust.
- C# remains a Windows adapter and GPUI remains the renderer.
- `Windows/TaskbarProgress.cs` requests the Windows 7+ `ITaskbarList4` interface, which inherits the
  progress methods from `ITaskbarList3`. This also supports Windows installations where
  `CLSID_TaskbarList` exposes `ITaskbarList4` but rejects a direct `ITaskbarList3` query.
- Taskbar diagnostics record state changes and 25-percent milestones; identical high-frequency
  progress commands do not generate generic command lifecycle pairs.

## Old NxTools behavioral source

### Entry files

| File | Verified responsibility |
| --- | --- |
| `Services/DownloadManagerService.cs` | Queue pumping, probing, HTTP segments, retries, HLS/DASH, merge, verification and finalization |
| `ViewModels/DownloadTaskViewModel.cs` | Task presentation state, progress/speed/remaining values and user actions |
| `ViewModels/MainWindowViewModel.cs` | Add flow, categories, scheduler, state restore/save, filters and command orchestration |
| `ViewModels/AddDownloadDialogViewModel.cs` | Probe-first new-download interaction and media resource selection |
| `Models/DownloadTaskStatus.cs` | Legacy task state enumeration |
| `Models/AppState.cs` | Legacy settings, task, category and segment JSON shape |
| `Services/AppStateStore.cs` | Atomic JSON state save at `%LOCALAPPDATA%\NxTools\downloader-state.json` |
| `Services/LocalApiService.cs` | Loopback HTTP/WebSocket task intake and control |
| `Services/BrowserExtensionPackageService.cs` | Browser extension packaging and installation |
| `browser_extension/nxtools-interceptor/background.js` | Download interception, request context, Blob/media discovery and desktop handoff |
| `Views/MainWindow.axaml` | Legacy task list, filters, categories, details and settings |
| `Views/MainWindow.axaml.cs` | Legacy per-task progress/completion window registry, reuse, placement and foreground behavior |
| `Views/AddDownloadWindow.axaml` | Legacy URL, save path, category and task settings flow |
| `Views/DownloadProgressWindow.axaml` | Legacy detailed progress layout plus the hidden compact completion layout |
| `Views/DownloadProgressWindow.axaml.cs` | Progress-to-completion in-place state change, center-preserving resize and final-file drag |
| `Views/DownloadCompletedWindow.axaml` | Passive completion fallback shown when no progress window remains open |
| `Views/DownloadCompletedWindow.axaml.cs` | Completion open/reveal/drag/close Windows behavior |

### Verified legacy capabilities

- HTTP/HTTPS probing and transfer;
- Range-based segments and resume;
- adaptive segment creation and retry;
- queue ordering and global concurrent-task admission;
- task scheduling and daily quota;
- global/per-task speed limits;
- sparse-file preallocation and disk checks;
- request headers, Cookie, proxy and TLS policy;
- SHA-256 and expected checksum;
- HLS and DASH download/merge;
- browser interception, Blob staging and media discovery;
- categories, completion notifications and post actions;
- task/segment history restore from JSON.

### Verified legacy download UI lifecycle

```text
AddDownloadDialogViewModel
  -> AddOnly: add a paused/manual-start task; do not show progress window
  -> DownloadNow: enqueue task and raise RequestDownloadProgressWindow
       -> MainWindow keeps one DownloadProgressWindow per task ID
       -> repeated open brings the existing window to front
       -> closing the window does not pause or cancel the task
       -> task completion raises RequestDownloadCompletedWindow
            -> progress window still open: ShowCompletionState on the same window
            -> progress window closed: create passive DownloadCompletedWindow when enabled
```

Verified invariants:

- `DownloadProgressWindow.ShowCompletionState` hides the progress tabs/footer, reveals the completion body/footer,
  changes `436 x 314` to `432 x 196`, preserves the previous visual center and locks the completed size.
- An existing progress window becomes the completion window; a second completion window is not stacked on top.
- The completion layout exposes open file, open folder, native file drag and close.
- `DownloadCompletedWindow.DragFileSurface_PointerPressed` and
  `DownloadProgressWindow.StartFileDragAsync` initiate transfer from the still-active Avalonia pointer event.
  `WindowsFileDragService` publishes `CF_HDROP` plus `Preferred DropEffect` and owns the OLE drag loop in the
  same visible UI process.
- The standalone completion fallback is non-activating, topmost and centered; it exists only when completion-dialog
  behavior is enabled and no progress window is available to transform.
- `MainWindow` stores progress and completion windows in task-ID dictionaries, removes closed windows from the
  registry and reuses live windows.
- Closing a progress window is presentation-only. Pause/resume/cancel are explicit actions.
- The progress window contains Info, Speed and On Completion sections plus locate-in-list, folder,
  pause/resume and cancel actions.

### Legacy failure and migration invariants

- Active transient states are restored as paused instead of automatically restarting unknown work.
- A task is complete only after output finalization and any configured verification.
- Old incomplete part files are format-coupled to the old implementation. NxTools.Next imports settings and
  history only; it does not claim those part files are resumable.
- Browser handoff failure must not silently discard the browser's original download.
- Request headers, Cookie and Referrer belong to the request context and must survive pause/resume.

The legacy `DownloadManagerService.cs` is over 4,500 lines and mixes multiple responsibilities. It is a behavior
inventory, not a module structure to reproduce.

## AB Download Manager reference

Root: `D:\Code\ab-download-manager-master`

### HTTP and lifecycle entries

| File or directory | Verified responsibility |
| --- | --- |
| `downloader/core/.../DownloadManager.kt` | Downloader/job registration, lifecycle events and settings |
| `downloader/core/.../downloaditem/DownloadJob.kt` | Boot-once lifecycle, active scope, periodic save and terminal transitions |
| `downloader/core/.../downloaditem/http/HttpDownloadJob.kt` | HTTP prepare, resume, parts, retry, validation and state save |
| `downloader/core/.../part/HttpPartDownloader.kt` | Range connection and response verification |
| `downloader/core/.../part/PartDownloader.kt` | Part start/stop/join, transfer liveness and terminal status |
| `downloader/core/.../part/PartSplitSupport.kt` | Runtime part splitting |
| `downloader/core/.../destination/` | Segmented destination, temporary files and completed output |
| `downloader/core/.../queue/DownloadQueue.kt` | Queue start/stop, concurrency and persistence |
| `downloader/core/.../queue/QueueManager.kt` | Named queue lifecycle and membership |
| `downloader/core/.../utils/speedlimiter/` | Shared throttling |
| `downloader/core/.../db/` | Download, part and queue persistence interfaces |
| `desktop/app/.../AppComponent.kt` | Opens progress on `OnJobStarting`, completion/error notification routing |
| `desktop/app/.../pages/singleDownloadPage/ShowDownloadDialogs.kt` | One keyed task dialog that switches between progress and compact completed windows |
| `desktop/app/.../pages/singleDownloadPage/ProgressDownloadPage.kt` | Info/settings/completion tabs, progress, parts, pause/resume/cancel and error actions |
| `desktop/app/.../pages/singleDownloadPage/CompletedDownloadPage.kt` | Compact completed file, open/reveal/drag/close actions |
| `shared/app/.../singledownloadpage/BaseSingleDownloadComponent.kt` | Monitor-driven progress-to-completed transition and per-task completion preference |

Verified useful behavior:

- download jobs are booted exactly once;
- pause cancels the active execution scope but preserves resumable state;
- state is periodically saved while active;
- destination finalization is a distinct phase before completed;
- Range responses are validated rather than trusted from request intent;
- segments can be split while the task is active;
- queue admission is separate from protocol execution;
- changing server resume support or file identity is treated as validation failure.
- `OnJobStarting` opens a progress dialog when the global setting is enabled.
- a task dialog is unique by task ID; repeated open requests bring the existing window to the front.
- the window title and Windows taskbar reflect percent and transfer state.
- the progress window can expand a resizable part table and persists whether part details are shown.
- completion changes the keyed task dialog from the progress layout to a smaller always-on-top completed layout.
- if the progress component is not already open, completion may create it according to the global completion setting.
- per-task completion preference overrides the global completion-window setting.
- failures use an in-window error action when the progress dialog is open; otherwise they use an error dialog or
  notification instead of silently disappearing.
- `CompletedDownloadPage.Actions` attaches Compose `dragAndDropSource` directly to the completed-file action.
  `DownloadItemTransferable` exposes the operating-system file-list flavor, so Compose owns the complete gesture
  on the visible source window instead of forwarding only a click to another process.

### CLI entries and behavior

| File | Verified responsibility |
| --- | --- |
| `desktop/app/.../cli/Cli.kt` | Root command and `gui`, `download`, `native-messaging` command groups |
| `desktop/app/.../cli/download/Download.kt` | Download command group |
| `desktop/app/.../cli/download/add/` | HTTP/HLS task submission |
| `desktop/app/.../cli/download/show/ShowDownload.kt` | One-shot table and live `--watch` progress |
| `desktop/app/.../cli/download/pause/` | Pause commands |
| `desktop/app/.../cli/download/resume/` | Resume commands |
| `desktop/app/.../cli/download/remove/` | Remove commands |
| `desktop/app/.../utils/singleInstance/` | CLI-to-running-application IPC |

Verified call pattern:

```text
CLI parses command
  -> connects to the already running single application instance
  -> sends an IPC request to application-owned download state
  -> prints a table or watches a progress stream
```

NxTools.Next should preserve this single-engine property. The CLI is a test and automation caller, not another
download process with separate storage.

### NxTools.Next Phase 1B call chain

```text
NxTools.Cli parser
  -> current-user pipe nxtools-next-download-v1
  -> desktop or NxTools.exe --background host
  -> DownloadEngine::handle_command
  -> one downloads.sqlite3 owner
  -> correlated result / bounded event subscriber
  -> human table or stable JSON/NDJSON
```

Verified implementation entries:

- `rust/apps/nxtools-cli/src/lib.rs`: command grammar, safe output projection, exit codes, host
  startup and human/JSON rendering.
- `rust/crates/nxtools-download-ipc/src/lib.rs`: handshake, typed frames, concurrent clients and
  watch-stream delivery.
- `rust/crates/nxtools-ipc/src/local_pipe.rs::accept_current_user_pipe`: local-only pipe with a
  SYSTEM + current-process-user SID ACL.
- `rust/apps/nxtools-desktop/src/runtime.rs::run_background_download_host`: headless single-engine
  owner that remains alive for active transfers or clients and exits after bounded idle.

Phase 1B invariants:

- CLI never opens `downloads.sqlite3`, never creates `DownloadEngine`, and never performs HTTP.
- Normal desktop and background mode use the same `desktop.lock`; two database owners cannot run.
- The background engine drains its primary bounded stream even when no GPUI consumer exists;
  watch clients receive independent bounded subscriptions.
- Machine-readable task output omits Cookie, Authorization and the complete header collection.
- The packaged CLI locates only its sibling `NxTools.exe`; it does not accept a caller-controlled
  host path.

## FluxDown clean-room reference

Root: `D:\Code\FluxDown-main`

| File | Verified responsibility |
| --- | --- |
| `native/engine/src/lib.rs` | Host-neutral `Engine` facade and injected host/event collaborators |
| `native/engine/src/events.rs` | Synchronous non-blocking `EventSink` and typed engine events |
| `native/engine/src/model.rs` | Persistent task, queue and segment DTOs |
| `native/engine/src/download_manager.rs` | Task lifecycle, queue admission, concurrency and progress orchestration |
| `native/engine/src/downloader.rs` | HTTP metadata, deduplication, segments and transfer |
| `native/engine/src/segment_advisor.rs` | Static and measured segment-count advice |
| `native/engine/src/segment_coordinator.rs` | Dynamic split and slow-segment recovery |
| `native/engine/src/speed_limiter.rs` | Global throttling |
| `native/engine/src/db.rs` | SQLite task, segment, queue and validator persistence |
| `native/engine/src/hls_downloader.rs` | HLS-specific transfer |
| `native/engine/src/dash_downloader.rs` | DASH-specific transfer |

Useful clean-room findings:

- the engine does not depend on the UI framework;
- event delivery must return immediately and must not block the engine runtime;
- task snapshots and targeted incremental events coexist;
- dynamic segment counts are caps, not a promise that all connections start immediately;
- segment split/merge and progress persistence need atomic database operations;
- incomplete active tasks are reset to paused on startup;
- protocol-specific behavior stays behind the engine rather than leaking into presentation code.

Do not copy FluxDown source, comments, tests or schema. Reproduce only independently specified behavior through
NxTools.Next's own interfaces and tests.

## NxTools.Next target call chains

### GPUI submission

```text
GPUI new-download interaction
  -> Rust DownloadApplication probe/confirm state
  -> nxtools-download probe/submit interface
  -> SQLite task + scheduler
  -> HTTP implementation
  -> bounded engine events
  -> DownloadApplication snapshot
  -> GPUI main-list render
  -> per-task window controller keyed by task ID
       -> progress layout while non-terminal
       -> same window shrinks to completed layout when still open
       -> passive completion fallback when the progress window was closed
```

### CLI submission

```text
NxTools CLI
  -> versioned local IPC
  -> desktop/download host
  -> same nxtools-download command interface
  -> correlated result or progress event stream
```

### Browser submission

```text
browser extension
  -> authenticated bounded local channel
  -> Rust intake adapter
  -> same nxtools-download submit interface
  -> correlated acceptance/failure response
```

### Windows actions

```text
Rust task/user intent
  -> typed PlatformHost command
  -> C# Windows adapter
  -> correlated result
  -> Rust state/notification update
```

## Cross-phase required scenario catalog

Phase 0 provides the network fixture routes. Destination and crash-window scenarios become executable only when
phase 1A introduces an artifact writer; segment checkpoint scenarios belong to phase 2. They remain release
requirements even though pulling their implementation into phase 0 would create fake transfer code.

- Phase 0: HEAD works and returns useful metadata;
- Phase 0: HEAD is rejected but ranged GET works;
- Phase 0: server supports valid `206` ranges;
- Phase 0: server advertises Range but ignores the requested range;
- Phase 0: malformed or mismatched `Content-Range`;
- Phase 0: unknown length/chunked response;
- Phase 0: redirects and final URL;
- Phase 0: ETag or Last-Modified changes during resume;
- Phase 0: connection drops after a deterministic byte count;
- Phase 0: connection stalls without closing;
- Phase 0: transient 429/5xx and permanent 4xx;
- Phase 1A: no disk space, permission failure and destination occupied;
- Phase 1A: process termination after all bytes arrive but before final rename;
- Phase 1A: immediately started task opens one progress window; add-only does not;
- Phase 1A: closing progress window does not alter task state and reopening reuses one task window;
- Phase 1A: completion with progress window open transforms and shrinks the same window without stacking another;
- Phase 1A: completion with progress window closed follows the completion-window setting and emits one feedback;
- Phase 1A: completed actions open/reveal/drag the final artifact and deletion requires explicit confirmation;
- Phase 2: process termination after segment progress but before database flush.

## Phase 1A verified transfer lifecycle

Verified on 2026-07-27 before starting the HTTP/HTTPS single-task slice.

### Old NxTools call chain

```text
DownloadManagerService.RunWithRetryAsync
  -> DownloadAsync / EnsureHttpProbeAndWorkspaceAsync
  -> ProbeAsync (HEAD Range -> GET Range -> HEAD -> GET fallback)
  -> single streaming response with ResponseHeadersRead
  -> write a task-owned temporary file
  -> flush the file
  -> VerifyAndFinalizeHttpTargetAsync
  -> move the temporary file to the final path
```

Additional verified entries:

- `DownloadManagerService.Pause`, `Resume`, `Cancel`, `HandleInterruptedTask`: pause preserves
  resumable work, cancel removes task-owned temporary work, and interruption never becomes
  completed.
- `DownloadManagerService.BuildProbeResult`: server `Content-Disposition` wins over URL-derived
  file names; final response metadata supplies size, content type and validators.
- `DownloadManagerService.VerifyAndFinalizeHttpTargetAsync`: completion requires an existing
  temporary file and a final filesystem move.

### AB Download Manager call chain

```text
HttpDownloadJob.resume
  -> fetchDownloadInfoAndValidate
  -> createPartsIfNotCreated
  -> prepareDestination
  -> beginDownloadParts
  -> periodic saveState (destination flush before part checkpoint)
  -> DownloadJob.onDownloadFinished
  -> SimpleDownloadDestination.onAllPartsCompleted
  -> atomic incomplete-file move to final output
```

Additional verified entries:

- `HttpDownloadJob.fetchDownloadInfoAndValidate`: a resume must reject changed length, ETag or
  resume support instead of appending untrusted bytes.
- `HttpDownloadJob.pause`: cancels the active scope, joins part writers and only then publishes the
  paused state.
- `SimpleDownloadDestination`: writes through a task-owned incomplete path, closes writers before
  finalization, flushes before persisted progress and keeps the incomplete artifact if the final
  move fails.
- `HttpPartDownloader.connectAndVerify`: validates the response before writing it to the
  destination.

### Phase 1A invariants for NxTools.Next

- Probing follows redirects and records the final URL; it prefers HEAD but falls back to a bounded
  ranged GET when HEAD is rejected or unhelpful.
- `Content-Disposition` has priority over URL path when choosing a safe Windows file name.
- The body is consumed chunk-by-chunk into a task-owned temporary file; no API in this slice
  exposes a whole-response byte buffer.
- Pause/cancel waits for the transfer writer to stop before changing persistent state.
- Resume in phase 1A may restart a single connection from byte zero; byte-range continuation is
  phase 2 and must not be implied.
- Completion order is: finish response -> flush -> durable sync -> close -> atomic same-directory
  rename -> persist completed.
- If the process stops before the rename, the final path is not published and startup restores the
  task as paused.
- Destination conflicts, permission errors, disk-full errors and protocol failures are typed and
  remain visible in task details/notifications rather than being appended to button labels.
- Queue-only submission persists `Created` without a transfer control or network worker; resume is
  the only transition that starts it.
- One task ID owns at most one GPUI progress/completion window. Completion transforms that window
  in place; a stale or closed window may be replaced by one passive terminal fallback only.
- Completed and failed tasks restored from SQLite establish the startup baseline. They must not be
  replayed as fresh terminal transitions or reopen historical task windows; only a terminal
  transition observed during the current process lifetime may create the passive fallback.
- Until the richer information/speed/completion tabs are implemented, task windows use compact
  content-driven phase sizes (`440 x 176` progress, `440 x 196` failed and `440 x 180` completed)
  instead of preserving empty space reserved by the old full-detail layout.
- Rust owns taskbar progress values and the final artifact selected for dragging. C# is limited to
  the inherited taskbar progress methods exposed through `ITaskbarList4`, Shell open/reveal and the
  native Windows file-drag loop.

## Phase 2 verified segmentation and resume lifecycle

Reverified on 2026-07-28 before starting the segmented-transfer slice.

### AB Download Manager call chain

```text
HttpDownloadJob.resume
  -> fetchDownloadInfoAndValidate
  -> createPartsIfNotCreated / loadPartState
  -> SimpleDownloadDestination.prepareFile
  -> beginDownloadParts
  -> HttpPartDownloader.connectAndVerify
  -> DestWriter at the part's current byte offset
  -> destination flush
  -> HttpDownloadJob.saveState
```

Verified entries and invariants:

- `HttpDownloadJob.fetchDownloadInfoAndValidate` rejects a change in resume support, content
  length or ETag before existing part progress is trusted.
- `HttpDownloadJob.createPartsIfNotCreated` creates inclusive, contiguous ranges only after the
  remote length and concurrent-transfer support are known.
- `HttpPartDownloader.connectAndVerify` snapshots the mutable part boundary before connecting and
  validates the returned length/range before acquiring a destination writer.
- `PartSplitSupport` changes the original part boundary and creates the new upper-half part while
  holding the same boundary lock used by the active reader. Bytes already inside the reader's safe
  zone are never reassigned to the new part.
- `SimpleDownloadDestination` owns one incomplete file and returns writers positioned at each
  part's current byte offset. It flushes the shared file before part checkpoints are persisted.
- A persisted part is a domain `Segment`; an HTTP request is a transient `Connection`. Restart
  recreates connections from persisted segment progress instead of persisting sockets or workers.

### Old NxTools compatibility chain

```text
DownloadManagerService.DownloadSegmentWithRetryAsync
  -> DownloadSegmentAsync
  -> GET with identity encoding and an inclusive Range
  -> require 206
  -> ValidateContentRange
  -> CopyToTargetAtOffsetAsync
  -> sparse progress checkpoint
```

The old implementation also falls back to one segment when Range cannot be trusted. NxTools.Next
keeps that product behavior but tightens the validation boundary: a segmented response must match
the requested start and end, declared body length and known total; a present validator must match
the probed resource. An ignored or malformed Range response is rejected before any segment byte is
written. Safe single-stream fallback may only restart from byte zero after segmented progress has
been explicitly discarded.

### NxTools.Next Phase 2 first-slice invariants

- `download_segments` is the normalized persistent source for segment progress; connections remain
  runtime-only.
- A transfer-plan save and all of its segment rows commit atomically. Startup recovery changes
  persisted `active` segments back to `waiting` in the same transaction that pauses interrupted
  tasks.
- The task owns one pre-sized incomplete file. Segment writes are bounded by their inclusive byte
  ranges and may arrive out of order without sharing a seek cursor.
- Range validation happens before opening the destination for a response. A `200`, missing or
  mismatched `Content-Range`, inconsistent `Content-Length`, changed known total or changed
  validator cannot alter the artifact.
- Checkpoints describe bytes already written and durably flushed. Diagnostic events are emitted for
  state transitions and bounded milestones, never for every received chunk.

### NxTools.Next Phase 2 engine integration

Verified on 2026-07-28 through the public `DownloadEngine` task seam:

```text
DownloadEngine submit/resume + command correlation ID
  -> probe and validate persisted TransferPlan
  -> create/open one task-owned pre-sized artifact
  -> restore Segment progress as a transient active Connection
  -> HttpDownloadClient.download_segment
  -> flush segment writer
  -> persist Segment checkpoint
  -> transition task to paused/finalizing
```

- `resume` derives visible downloaded bytes from persisted segments instead of resetting progress.
- A newly observed server that ignores the real Range request may fall back to single stream only
  while aggregate persisted segment progress is zero. The plan and pre-sized artifact are removed
  before the byte-zero restart.
- A length or validator change after a nonzero checkpoint is a terminal protocol failure for that
  attempt. The old incomplete artifact and checkpoint remain available; no new resource bytes are
  appended.
- The submit or resume command correlation ID is carried into segment start/terminal, transfer-plan,
  checkpoint and fallback diagnostic events.

### Phase 2 fixed multi-connection call chain

Verified on 2026-07-28 against the real `DownloadEngine` and deterministic HTTP server:

```text
DownloadEngine::run_task
  -> probe + artifact paths
  -> run_segmented_task
  -> create/load one durable TransferPlan
  -> run one HttpDownloadClient::download_segment future per unfinished segment
  -> independent SegmentWriter handles seek to disjoint offsets in one preallocated artifact
  -> aggregate progress and active connection count
  -> durable per-segment checkpoint
  -> one atomic final artifact commit
```

- The initial segment count is bounded by the request connection limit and useful 64 KiB ranges;
  small resources do not create empty or one-byte connection storms.
- Persisted completed segments are never requested again. Persisted waiting/active segments resume
  from their own byte count after engine restart, and all callbacks publish one task-level byte sum.
- Every segment uses the same task and command correlation IDs but emits only one start and one
  terminal event; HTTP chunks remain silent.
- A clean single-stream fallback is allowed only when the range workers have accepted zero bytes.
  Once any segment has durable progress, failures retain the plan and temporary artifact.
- When a worker completes and releases a connection slot, the coordinator selects the active
  segment with the largest observed unwritten tail. It never edits that active range directly:
  the selected worker first acknowledges a local rebalance after syncing its writer.
- The coordinator checkpoints that acknowledgement as `waiting`, atomically replaces the durable
  plan with a reduced original range plus a new tail, then launches both ranges. The number of
  active workers never exceeds the initial bounded connection target.

### Phase 2 retry and dynamic-split reference findings

- ABDM `PartDownloader` retries a part independently and stops retrying critical validation errors.
  `HttpDownloadJob` separately bounds whole-job retries and resets the failure count after real
  progress. NxTools.Next keeps the useful separation but uses typed Rust error classification:
  transient transport/status failures retry; protocol, validator and disk failures remain terminal.
- NxTools.Next currently performs at most three segment attempts with bounded exponential delay.
  Pause/cancel interrupts that delay and the next worker entry observes the directive before opening
  another HTTP request. Each scheduled retry emits one correlated event; chunks remain silent.
- ABDM `PartSplitSupport` does not blindly bisect an active request. It mutates the old part end under
  `partEndLock`, while a `safeZone` limits how far the old writer may consume. NxTools.Next must
  preserve the stronger durable-file invariant: request an internal worker stop, wait for its writer
  sync acknowledgement, atomically replace the affected segment records, then start the two
  non-overlapping ranges. Splitting the in-memory range before that acknowledgement is forbidden.
- Reqwest 0.13's per-read timeout resets after each successful read; NxTools.Next combines that
  transport guard with an explicit Tokio progress timer. This covers connect/headers/body stalls
  while keeping pause/cancel as the biased first branch. The default production stall boundary is
  30 seconds; tests inject a shorter client boundary without changing task-domain state.
- NxTools.Next now models ABDM's part-end lock boundary explicitly rather than sharing the user's
  pause flag. `DownloadSegmentTransferControl` carries task-wide pause/cancel plus one worker-local
  rebalance signal. A worker returns `Rebalanced` only after its writer has flushed and synced.
- `TransferPlan::split_segment_after_checkpoint` accepts only that resulting `waiting` checkpoint.
  It preserves the downloaded prefix on the reduced segment, creates a new zero-progress tail, and
  reconstructs the whole plan through the normal overlap/index/bounds validator before mutation.
- `DownloadEngine::run_segmented_task` now owns the coordinator: completed workers release a slot,
  the largest eligible observed tail receives `request_rebalance`, and the synced acknowledgement
  is persisted before both replacement workers start. If progress made the tail too small before
  acknowledgement, the original range resumes unchanged instead of failing the task.
- `tests/support::serve_imbalanced_range` makes the first half fast and the second half slow.
  `dynamic_rebalance_splits_a_slow_tail_without_overlapping_output` requires at least four real
  Range transfers and compares the final preallocated artifact byte-for-byte.

### Phase 2 speed-limit reference findings

Verified on 2026-07-28 before implementing the NxTools.Next limiter:

- ABDM `utils/speedlimiter/OkioCustomizedThrottler.kt` is a shared allocator, not a delay copied
  independently into every connection. Multiple sources attached to one limiter share one
  aggregate budget; `0` means unlimited, and changing the limit resets the allocation clock and
  wakes current waiters so the new value takes effect immediately.
- ABDM `DownloadManager.speedLimiter` is shared by every task. Each `HttpDownloadJob` also owns one
  `jobThrottler`; every HTTP part is wrapped by both, so global and per-task limits constrain the
  aggregate of all parts rather than dividing a stale number by the configured thread count.
- Old NxTools `DownloadManagerService.ThrottleAsync` sampled the global and per-task values after
  each read, divided them by current task/thread counts and delayed that individual stream. This
  made live changes visible but could exceed or underuse the intended aggregate as concurrency
  changed. The migration preserves `0 = unlimited` and live updates, but follows ABDM's shared
  budget model.
- Rust owns both limiter state and persistence. HTTP workers consume permits only at the response
  body boundary; GPUI will later edit typed values, while C# has no role. Throttle waits must be
  interruptible by pause/cancel and worker-local rebalance, must not count as network stalls, and
  must not emit per-chunk logs.
- Diagnostics are limited to correlated global/task configuration changes and terminal validation
  failures. Byte permits, wakeups and wait durations are intentionally silent.

### Phase 2 storage-allocation reference findings

Verified on 2026-07-28 before implementing the NxTools.Next storage guard:

- ABDM `utils/EmptyFileCreator.kt` checks the destination file store before changing the logical
  file length. For an existing artifact it requires only `target length - current length`; a
  shortage raises the critical `NoSpaceInStorageException` with available and required byte
  counts. Unknown-length transfers cannot be preflighted and still rely on the real write result.
- ABDM `SparseFile.desktop.kt` enables sparse creation only when the nearest existing parent reports
  a known sparse-capable file-system type. Sparse creation is best effort: failure falls back to
  ordinary allocation rather than changing the download result.
- Old NxTools `DownloadManagerService.EnsureEnoughDiskSpace` derives remaining bytes from durable
  segment progress and keeps a reserve of `min(512 MiB, max(64 MiB, total / 20))`. Its Windows
  preparation path marks the single task-owned temporary file sparse before assigning its logical
  length. Capacity-probe permission/path failures are logged and skipped; an observed shortage is
  terminal.
- NxTools.Next preserves the old reserve policy and ABDM's remaining-byte rule. A resume never
  requires the full logical file length again, and SQLite segment checkpoints remain the only
  source of durable progress; no sparse-progress sidecars are introduced.
- Capacity preflight is advisory when the operating system cannot identify the destination volume,
  while a real create, resize, write, flush or rename error is authoritative. Wrapped artifact I/O
  errors must retain `disk_full`, `permission` or `file_in_use` instead of collapsing to `protocol`.
- Sparse allocation is a creation optimization, not a correctness requirement. Unsupported file
  systems or a failed sparse hint fall back to the normal file path and emit one bounded diagnostic
  event. No allocation or write loop may produce per-chunk logging.
- Windows sharing violations (`ERROR_SHARING_VIOLATION` and `ERROR_LOCK_VIOLATION`) are reported as
  `file_in_use`. The final destination-exists rule remains separate and never overwrites an
  unrelated user file.
- The implemented Windows artifact path calls `FSCTL_SET_SPARSE` at the Rust storage boundary,
  verifies `FILE_ATTRIBUTE_SPARSE_FILE` from file metadata, and uses a normal allocation when the
  file system rejects the hint. Merely passing the sparse attribute to `CreateFile` was tested and
  rejected because NTFS did not report the resulting file as sparse.
- Capacity probing is injected behind one Rust storage interface. Production selects the longest
  matching `sysinfo` mount point; engine tests inject a zero-capacity volume and prove the task
  reaches `disk_full` after HEAD metadata but before the artifact or GET request exists.

### Phase 2 forced-interruption recovery findings

- A forced-process test launches a separate download-engine test process against the parent-owned
  deterministic HTTP server, waits until bytes have reached the task artifact, then exits with
  `std::process::exit` before Rust destructors run. The parent reopens the same SQLite database,
  observes the interrupted task as paused, resumes it and verifies the final file byte-for-byte.
- A narrower crash-window test writes bytes beyond the last SQLite segment checkpoint and marks the
  task/segment active, matching a process death after file I/O but before checkpoint commit. Startup
  trusts the older durable checkpoint and the resumed Range writer safely overwrites the
  uncheckpointed tail.
- The deterministic fixture can now drop an active ranged connection and reject all retry
  connections while offline. Exhausting retries leaves no final artifact; after the fixture comes
  back, explicit task resume reuses the durable plan and completes with exact bytes.
- The existing mutable-resource test remains the server-change boundary: after a durable
  checkpoint, changed validators fail without publishing a file or deleting the checkpoint and
  incomplete artifact.

### Phase 3 status-filter and main-list findings

Verified on 2026-07-28 before implementing the first Phase 3 desktop slice:

- ABDM `shared/app/.../pages/home/category/DefinedStatusCategories.kt` models every status category
  as a predicate over the shared download monitor rather than maintaining a second task list.
  `HomePage.kt` keeps the chosen predicate as presentation state, and `DownloadList.kt` renders the
  resulting monitor snapshot. Filtering never mutates queue membership or engine state.
- ABDM's built-in desktop categories are All, Finished and Unfinished. NxTools.Next follows the
  product plan's more useful six-way split instead: All, Downloading, Waiting, Paused, Completed and
  Failed. This is an intentional product extension, not a claim that ABDM exposes the same labels.
- Old NxTools `MainWindowViewModel.FilterTasks` applies the current status/category predicate first
  and search second, while `RebuildVisibleTasks` reconciles the visible selection. Its sidebar shows
  live counts, and an empty filtered result is distinct from an empty database.
- NxTools.Next keeps one authoritative `DownloadApplication::tasks` snapshot. Rust owns the filter
  predicate and selected filter, while GPUI only renders it; C# has no responsibility.
- Status membership is explicit: Probing, Downloading, Finalizing and Verifying are Downloading;
  Created and Waiting are Waiting; Paused, Completed and Failed are exact matches. Cancelled remains
  visible under All but is not misreported in another status category.
- Filter changes are local presentation transitions. They must not issue engine commands, disturb
  active transfers, recreate task windows or change queue order.
- The selected detail row must always belong to the visible result. If the previous selection is
  filtered out, select the first visible task; if the result is empty, clear the detail pane and
  show a filter-specific empty state.
- Counts are computed from the same snapshot as the rows. Task events update counts and visible rows
  together on the next GPUI render, without per-byte diagnostic logging.
- A user filter change emits one low-noise diagnostic containing the filter name, visible count and
  total count. It never records URLs, file names, headers or other payload data.

### Phase 3 download-main-layout findings

Verified on 2026-07-28 before restructuring the NxTools.Next download workspace:

- ABDM `HomeComponent.tableState` defines one real task table with Name forced visible and optional
  Size, Status, Speed, Time Left and Date Added columns. `DownloadList.kt` and
  `TableDownloadItem.kt` use flat table rows, one semantic selection state and contextual actions;
  they do not wrap every task in a large independent card.
- ABDM keeps category/status navigation adjacent to the table but lets the task table remain the
  visual center. Actions are grouped in the header or an item option surface instead of repeating
  several large text controls as the dominant content of every row.
- Old NxTools `Views/MainWindow.axaml` combines a module rail, category rail, sortable task table and
  a rich detail pane. That capability inventory remains useful, but retaining all four fixed areas
  at the current compact window width leaves too little room for the task name and transfer state.
- NxTools.Next already has a persistent product-level module rail, so the initial six status filters
  move into one horizontal strip. A second full-height status rail is unnecessary until user
  categories require a dedicated hierarchy.
- The download page uses one continuous editor-style surface: a 48 px page header, 38 px horizontal
  filter strip, 28 px table header, compact 58 px flat rows and a bottom detail dock separated by one
  hairline. These are density targets rather than additional Windows surfaces.
- Selection uses a quiet neutral row fill plus a narrow accent marker, not a bright border around a
  card. Hover, selected and progress colors reuse the shared Zed-derived semantic theme.
- Name/path remain the dominant first column. Progress, status and actions occupy stable columns so
  repeated values align vertically. Long names and paths truncate instead of pushing actions away.
- The detail area is attached to the list as a docked inspector. It does not float as a rounded card
  beneath a large empty list viewport, and it omits meaningless values such as “无错误”.
- No search box, queue toolbar, sort control or batch action is rendered until its corresponding
  Phase 3 behavior exists. The layout must not advertise inert controls.
- Layout changes remain GPUI-only. Filter selection continues to mutate only local Rust presentation
  state; transfer state, queue order and C# Windows integration are unchanged.

### Phase 3 download-row density correction

Reverified on 2026-07-28 after inspecting the first packaged flat-table layout:

- ABDM keeps item operations in its option popup and selection workflow; `DownloadList.kt` does not
  repeat pause, remove, delete and open text buttons across every table row. NxTools.Next therefore
  removes the provisional per-row operation column. Double-click still opens the existing task
  window, while the complete right-click action surface remains a later Phase 3 behavior.
- Old NxTools used a 52 px two-line row. The first NxTools.Next table pass used 58 px and looked too
  loose at the compact main-window size. The corrected target is 46 px: one file-name line, one
  secondary path/error line, a 26–28 px icon well and a 3 px progress track.
- Name and path must consume the reclaimed width before progress and status. The first packaged
  correction proved that a nested `w_full() + truncate()` text node can resolve to an ellipsis-only
  layout even while its outer flex column has room. Keep `min-width: 0` and clipping on the flexible
  outer column, but let the inner text use natural width plus no-wrap; the parent owns clipping.
- The table header exposes only columns backed by visible row data. After removing inline actions,
  it contains Name, Progress and Status; no empty or inert operation header remains.

### Phase 1A new-download presentation invariants

Reverified on 2026-07-27 after the first packaged interaction pass:

- Old NxTools `Views/AddDownloadWindow.axaml` is a compact two-stage flow. The initial state contains
  only the URL, paste action, Continue and Cancel. The confirmed state places the resolved URL and
  file metadata first, then destination and file name, with Add, Download and Cancel in one footer.
- AB Download Manager
  `desktop/app/.../pages/addDownload/single/AddDownloadPage.kt` keeps the URL full-width, groups the
  category/location/name fields into one left column, reserves a narrow right metadata/action rail,
  and anchors Add, Download and Cancel at the bottom. Its single-download window starts at
  `500 x 265` and is always on top.
- NxTools.Next keeps the old product's explicit probe/confirm behavior while adopting ABDM's compact
  hierarchy: URL is the dominant row; destination and file name align; probe metadata is a narrow
  summary instead of a full-width empty card; primary/secondary actions remain in a stable footer.
- The initial window is a compact URL/probe stage. Destination, editable file name and remote
  metadata appear only after a successful probe; the probe result must hydrate the visible file-name
  input immediately. Destination and file-name edits do not invalidate remote metadata, while a URL
  edit does require a new probe.
- Client-drawn close controls bind an explicit GPUI close action. Windows control-area hit testing is
  not accepted as the sole close path for the add, progress, failed or completed windows.
- Submitting from the add-window entity must not synchronously open another GPUI window. The accepted
  transition is: persist/refresh the task on a deferred application turn, close the source add
  window, then schedule the keyed task window for a later application turn. Queue-only closes the
  source without opening a task window.
- A keyed task window renders only from a task/progress snapshot pushed into its own entity. Its
  render path must never call `DownloadApplication::read`: GPUI may synchronously render a newly
  opened or refreshed window while the parent application entity is still being updated, and that
  re-entry aborts the Windows callback with `0xc0000409`. Task-window actions likewise defer their
  application mutations until the current view callback has returned.

### Phase 1A progress/completion interaction invariants

Reverified on 2026-07-27 against the first packaged progress-window acceptance pass:

- Old NxTools `DownloadTaskViewModel.AddDownloadedBytes` calculates transfer speed from byte and
  monotonic-time deltas and publishes at a bounded interval instead of displaying a protocol
  placeholder. NxTools.Next performs the same calculation in the Rust engine so every UI/CLI
  consumer receives one authoritative rate.
- Old NxTools `DownloadProgressWindow.ShowCompletionState` transforms the existing progress window,
  marks it topmost and brings it forward. NxTools.Next preserves the one-window transform but uses
  `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)` through the Windows adapter: completion becomes
  visible over the user's current work without taking keyboard focus.
- Old NxTools `DownloadCompletedWindow` begins file dragging from a pressed pointer gesture and its
  native `IDropSource` ends the OLE loop when the left button is released. AB Download Manager's
  `CompletedDownloadPage` likewise exposes the artifact as a drag source rather than treating the
  control as an ordinary click button. NxTools.Next must not enter GPUI's internal drag loop and
  then start a second native loop. It dispatches the C# OLE request synchronously from GPUI's
  still-pressed mouse-down callback and carries both the final path and visible source HWND.
  Because PlatformHost is a separate process, C# first sends `WM_CANCELMODE` to GPUI's actual
  capture window, verifies that the left button remains pressed, and only then calls the standard
  OLE `DoDragDrop` entry with a `CF_HDROP`/`Preferred DropEffect` data object.
  Without that capture handoff, the PlatformHost OLE loop does not receive the release transition
  and the command never completes.

### Phase 1A/1B packaged interaction follow-up

Reverified on 2026-07-28 after the combined desktop/CLI acceptance pass:

- AB Download Manager's `CompletedDownloadPage.dragAndDropSource` and
  `DownloadItemTransferable` keep the pressed gesture and the `javaFileListFlavor` payload in one
  drag operation. Old NxTools likewise enters its native/Avalonia drag path directly from
  `DragFileSurface_PointerPressed`; neither implementation converts the interaction into a normal
  click.
- Old NxTools' working native path calls Ole32 `DoDragDrop`; its visible source HWND is not passed
  into the OLE API. In the split NxTools.Next process model, the GPUI HWND is likewise used only to
  identify and release GPUI's pointer capture. PlatformHost must not ask a foreign-process GPUI
  window to provide Shell drag imagery.
- A native drag command must log the capture inspection, capture release, OLE entry and OLE exit
  under the command correlation ID. A `command.received` entry without one of those terminal
  stages is treated as a stuck native drag loop, not as a successful dispatch.
- The 2026-07-28 packaged trace reached `ole_entered` but never produced `ole_exited` or
  `command.completed`. Because `PlatformPipeClient` awaited every command in one serial read loop,
  that one stuck drag also prevented later `open_shell_target` commands from being read. Native
  dragging now runs on one dedicated STA worker, and only `begin_file_drag` is detached from the
  command-read sequence while retaining its correlated eventual result. Shell open/reveal and the
  shared Windows event thread therefore remain responsive even if Windows keeps a drag active.
- ABDM's CLI IPC feeds the same application-owned download system rather than opening a second
  database or engine. NxTools.Next adds one product-specific presentation rule: a previously
  unseen CLI task observed in an active `StartNow` state opens the same keyed progress window as a
  desktop `StartNow` submission. `QueueOnly` remains silent, restored tasks remain silent, and a
  repeated task event must refresh rather than stack windows.
- Removing a task is one event-owned presentation transition. The initiating action must not
  synchronously rebuild the list and then rebuild it again when `TaskRemoved` arrives. Action
  controls inside a row consume their pointer/click event so the now-removed row cannot briefly
  become selected during bubbling.

### Phase 3 queue, concurrency and batch-command findings

Reverified on 2026-07-29 before implementing the second Stage 3 slice:

- AB Download Manager models queue order independently from transfer state in
  `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/queue/DownloadQueue.kt`,
  `QueueManager.kt` and `ManualDownloadQueue.kt`. `maxConcurrent` admits work from an explicit
  ordered pending list; a terminal or paused worker releases a slot and the first eligible pending
  item fills it. Queue order is therefore durable domain state, not an incidental sort by creation
  time or a visual-only row order.
- ABDM's multi-selection move logic preserves the selected items' relative order. Moving upward
  processes source indices from low to high, while moving downward processes them from high to low;
  top and bottom moves are stable partitions. NxTools.Next adopts the same stable semantics.
- ABDM can stop active items when concurrency is reduced. NxTools.Next intentionally applies a
  reduced limit to future admissions and lets already-active transfers finish. This avoids turning
  an administrative concurrency change into a user-visible pause and still converges to the new
  limit as slots are released.
- Old NxTools exposes `MoveQueueToTop`, `MoveQueueToBottom`, `MoveQueueUp` and `MoveQueueDown` in
  `Services/DownloadManagerService.cs`; `MainWindowViewModel.cs` resolves the current multi-selection
  before `PauseSelected`, `ResumeSelected`, `PrioritizeSelected`, queue movement, cancellation and
  deletion. The useful invariant is retained, but all queue ownership and mutation now live in
  Rust rather than the C# Windows adapter.
- Priority is a stable move-to-front of eligible pending items. It never preempts an active
  transfer. Queue-only tasks remain dormant until a resume/start command makes them eligible.
- Batch commands are best-effort and deterministic: targets are processed once in request order,
  valid targets are committed even if another target is rejected, and the typed result reports
  every target outcome. CLI exits non-zero when any target failed while retaining successful
  per-task results in JSON.
- The default queue and its maximum concurrency are persisted in SQLite. Task membership and
  integer queue position are updated transactionally; restoring the engine must reproduce the
  same pending order before admitting new work.
- A pending eligible task can be paused before it consumes a transfer slot. Submission atomically
  commits task state, queue membership and eligibility; startup refills every persisted queue, not
  only the default queue.
- Queue diagnostics use correlation IDs and record only task IDs, queue IDs, action, position and
  slot counts. URLs, headers, cookies, file names and file contents remain excluded from logs.

### Phase 3 daily scheduling and delayed-admission findings

Reverified on 2026-07-29 before implementing the third Stage 3 slice:

- AB Download Manager keeps daily scheduling as durable queue policy in
  `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/queue/ScheduleTimes.kt` and
  `DownloadQueue.kt`. The policy has selected weekdays, independent enabled start/stop controls and
  local wall-clock times. Each trigger computes the nearest eligible local weekday/time, runs once,
  then arms the next day's trigger; it does not busy-poll from the UI.
- ABDM's `DownloadQueue.setUpAutoStartJob` / `setUpAutoStopJob` cancel a superseded timer before
  arming the replacement. Start records the pre-start queue activity and starts the queue; stop
  stops active queue work. Queue scheduling is therefore an engine-owned lifecycle, not an
  application-window timer.
- Old NxTools accepts a one-time absolute `ScheduledStartAt` during task creation in
  `ViewModels/MainWindowViewModel.cs::TryResolveSchedule`; it creates a scheduled task instead of
  starting it early. `ReleaseDueScheduledTasks` later enqueues only tasks whose scheduled time is
  due, and `PauseTasksOutsideSchedulerWindow` pauses active downloads outside the configured daily
  window.
- The old daily window accepts both normal and cross-midnight intervals:
  `start <= stop` means `start <= now <= stop`; `start > stop` means `now >= start || now <= stop`.
  Invalid time input leaves the window permissive instead of unexpectedly pausing work.
- NxTools.Next follows ABDM's explicit stop trigger at the configured minute rather than carrying
  the old inclusive stop minute into the next minute. Its effective recurring window is therefore
  start-inclusive and stop-exclusive. For a cross-midnight window, the stop trigger belongs to the
  calendar day after the selected start weekday.
- NxTools.Next will keep one-time delayed admission as an absolute UTC Unix-millisecond request
  field (`scheduled_at_unix_ms`) and model recurring queue windows separately. A future delayed
  request is durable but ineligible; when due, the Rust engine atomically admits it to its queue.
  Manual resume overrides the delay, while manual pause cancels it. The UI never owns the due timer.
- Scheduling diagnostics carry the command correlation ID and only log task/queue IDs, action,
  due/window timestamps and state transitions. URLs, paths, request headers, cookies and file names
  are excluded.
- The compact NxTools.Next `NewDownloadView` exposes one optional local-time `稍后下载` field only
  after a successful probe, alongside destination and file name. It accepts `YYYY-MM-DD HH:MM`,
  rejects past, invalid and ambiguous local times before submission, preserves the field across a
  probe refresh, and closes the add window after acceptance. A delayed `StartNow` submission must
  not open a progress window early; the existing keyed window opens only once the engine admits the
  task. The field follows the same input focus order, close control and inline validation surface as
  URL/destination/file name, and it adds no separate popup or focus-stealing window.
- ABDM `desktop/app/.../pages/queue/QueueInfoComponent.kt` presents daily scheduling as one queue
  configuration group: active weekdays first, followed by independent auto-start and auto-stop
  toggles whose time fields are visible only while enabled. Old NxTools
  `Views/SettingsWindow.axaml` keeps the same policy in a dedicated settings section and describes
  cross-midnight windows explicitly.
- NxTools.Next keeps the download task table dense by placing one compact `计划` action beside
  `新建下载`; it opens one keyed client-decorated settings window instead of expanding controls
  inside the task list. Repeated clicks activate the existing window. The window owns local draft
  state, does not mutate the engine until `应用`, keeps independent start/stop toggles, requires at
  least one weekday when either trigger is enabled, validates `HH:MM`, and reports rejection inline.
  `取消` and the title-bar close control discard the draft. Successful application refreshes the
  authoritative queue snapshot, closes the settings window, and never changes task selection or
  focus in the main download table.

### Phase 3 virtual task table, search, sorting and context-action findings

Reverified on 2026-07-29 before implementing the fourth Stage 3 slice:

- AB Download Manager renders the task surface through
  `desktop/app/.../pages/home/sections/DownloadList.kt::DownloadList`, which passes stable task IDs
  and a `LazyListState` into its reusable `Table`. Only the visible rows are composed; selection is
  stored by task ID rather than row index, so filtering and sorting cannot redirect an action to a
  different task.
- ABDM `DownloadListCells` makes Name, Status, Size, Speed, Time Left and Date Added sortable.
  `HomeComponent.tableState` defaults to Date Added descending and persists table state. A column
  click changes presentation order only; queue order and engine admission order remain unchanged.
- ABDM `BaseHomeComponent.downloadList` combines search, category/status/queue filtering and the
  active/completed streams before the table. Search is case-insensitive and matches the task name.
  Duplicate task IDs are removed before they reach the keyed lazy list.
- ABDM `DownloadList` opens the context menu from the secondary pointer gesture. Before opening,
  `HomeComponent.onRequestOpenDownloadItemOption` replaces the selection with the clicked task when
  that task was not already selected; right-clicking an existing multi-selection preserves that
  selection. Menu enablement is derived from the selected task set and disabled actions remain
  visible at lower opacity.
- Old NxTools keeps the compatible product-specific extensions in
  `ViewModels/MainWindowViewModel.cs::{FilterTasks,SortTasks,SortBy}` and
  `Views/MainWindow.axaml(.cs)::{TaskRow_ContextRequested,BuildTaskContextMenu}`. It searches file
  name, URL and source domain; sorts by name, size, status, progress, speed, remaining time or
  creation time; and exposes open file/folder, queue movement, resume, pause, cancel/delete,
  redownload, copy, speed limit, checksum and properties according to capability.
- NxTools.Next will keep the existing dense 46px row and bottom detail strip, replace eager child
  construction with GPUI `uniform_list` plus one persistent `UniformListScrollHandle`, and preserve
  the same stable-ID Ctrl/Shift selection model. Search matches resolved file name, URL and final
  path without sending engine commands. Sorting is a view-only stable order with an explicit
  queue-order default and never writes `download_queue_items.position`.
- Search, filter and sort changes reconcile primary/multi-selection against the resulting visible
  stable IDs. An empty search result is distinct from an empty status filter. Secondary click first
  selects an unselected row, then builds one context menu from that exact selection. File open,
  reveal and delete-file actions are enabled only for a single task with a final artifact; batch
  lifecycle and queue actions continue through typed Rust engine commands.
- The feature logs only bounded search/sort/context action state transitions: query length and
  result count, sort key/direction, selected count, action name and correlation IDs already owned by
  the engine/platform boundary. It never logs the query, URL, path, file name, headers or cookies,
  and virtual row creation/scroll frames are never logged.

### Phase 3 task detail, segment, Header, verification and log findings

Reverified on 2026-07-29 before implementing the fifth Stage 3 slice:

- ABDM `desktop/app/.../singleDownloadPage/ProgressDownloadPage.kt::{ProgressDownloadPage,
  RenderInfo,RenderPartInfo,RenderParts}` keeps one compact tab strip above the progress surface.
  Information rows use a fixed label/value hierarchy. Part details are optional, preserve their
  expanded state, use stable part IDs, render one proportional aggregate strip and a table with
  number, state, downloaded bytes and total bytes. Opening details never changes transfer state.
- ABDM `shared/app/.../singledownloadpage/BaseSingleDownloadComponent.kt::
  extraDownloadProgressInfo` derives name, state, total, downloaded, speed, remaining time and
  resume support from the monitor snapshot. Error presentation is separate from part state, and
  the UI does not read downloader internals directly.
- Old NxTools `Views/MainWindow.axaml` download-detail surface adds the product-specific URL/final
  path, request/response Header, resource validator/checksum, segment and task-log sections.
  `ViewModels/DownloadTaskViewModel.cs::{HeaderSummaryText,RequestHeaderSummaryText,
  CookieSummaryText,SegmentSummaryText,AddLog}` bounds each task timeline to 300 newest entries;
  `Models/AppState.cs::DownloadTaskState` persists those entries and segments.
- NxTools.Next will keep the existing bottom-docked inspector instead of restoring the old wide
  right sidebar. It uses compact `概览 / 分段 / Header / 校验 / 日志` tabs, one persistent detail
  scroll handle and no decorative empty cards. Selecting a task may query its detail snapshot but
  must never send a lifecycle command, alter queue order or synchronously re-enter the leased GPUI
  entity.
- The normal task-list snapshot stays lightweight. A typed Rust `TaskDetail` query loads only the
  selected task's persisted transfer plan, ordered segments and bounded timeline. GPUI renders that
  immutable snapshot; C# has no role. Detail refresh follows already-coalesced task events and does
  not poll or load every task's segments on each frame. Live byte events update the selected
  in-memory segment snapshot only after the engine's 80 ms publication gate; the engine constructs
  and sorts the segment payload only for an event that will actually be published. The published
  aggregate byte count is recomputed from that same immutable segment payload so parallel callbacks
  cannot produce an internally inconsistent event. A dynamic split persists the new plan and emits
  one coarse task refresh so a newly introduced stable segment ID cannot remain absent from the
  inspector cache.
- Request Header and Cookie values may contain credentials. The inspector shows non-sensitive
  request values, but masks Authorization, Proxy-Authorization, Cookie, Set-Cookie and API-key
  values; diagnostics never record any Header name/value, Cookie, URL, path or file name. Response
  details are limited to already persisted probe metadata: content type/length, final URL, range
  support, ETag and Last-Modified.
- This slice reports current resource identity and transfer-integrity verification only. User
  supplied expected hashes and SHA-256 computation remain the explicit Stage 6 item and must not be
  pulled forward or represented as already implemented.
- The per-task timeline is durable, newest-first and bounded to 300 entries. It records coarse
  lifecycle states and transfer-plan changes only; byte ticks, connection frames and raw fault
  payloads are excluded. The `TaskDetail` diagnostic records correlation ID, task ID, counts,
  elapsed time and terminal success/failure without copying timeline or request payload content.
  Credential masking normalizes separator variants such as `X-API-Key`, `X-ApiKey` and
  `X_API_KEY`; the proportional aggregate strip represents both each segment's byte share and its
  downloaded fraction rather than assigning every segment the same width.

## Resolved phase-0 decisions

- CLI connects to a current-user local download pipe and starts the packaged desktop in background mode when
  the download host is absent. It never opens SQLite directly.
- Download command, storage and CLI v1 contracts are recorded in `contracts/`.

## Open research items

- Whether Windows system proxy resolution can remain wholly in Rust or needs one C# discovery command.
- The final authenticated browser transport: Native Messaging only, or Native Messaging plus a bounded loopback
  channel for large Blob payloads.
- HLS/DASH shared protocol seam must wait until the HTTP implementation and one media implementation both exist.
