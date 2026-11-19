# Download manager reference implementation index

Last verified: 2026-08-11.

## Scope and authority

| Source | Role | Reuse rule |
| --- | --- | --- |
| `D:\Code\NxTools` | Existing NxTools user-visible behavior and legacy migration format | Preserve useful behavior; do not carry its architecture into NxTools.Next |
| `D:\Code\ab-download-manager-master` | Primary HTTP, queue, destination and CLI source reference | Apache-2.0; concepts or attributed compatible reuse are allowed |
| `D:\Code\FluxDown-main` | Rust engine, event, persistence and dynamic-segment research | AGPL-3.0; clean-room behavioral/architectural reference only, no code copying |
| `D:\Code\NxTools.Next-main` | Product implementation target | MIT OR Apache-2.0 |

The old NxTools application is the behavior source when the references disagree about existing product behavior.
ABDM is the primary implementation reference for HTTP transfer and CLI interactions. FluxDown is useful for
architecture and failure scenarios, but its AGPL code must not enter NxTools.Next.

## Verified NxTools.Next baseline

- `rust/apps/nxtools-desktop/src/main_window.rs::render_downloads` renders the Phase 1A real task list,
  actions, progress and task details.
- `rust/crates/nxtools-network` owns the product-wide validated network policy and immutable
  per-operation route snapshots. `rust/crates/nxtools-download` consumes that policy and owns the
  download domain/message/storage contracts, deterministic HTTP fixture and transfer execution.
- Desktop and CLI call the same versioned command/snapshot seam. The engine retains one bounded
  primary desktop stream and can attach additional bounded event subscribers for IPC watch clients.
- `rust/crates/nxtools-download-ipc` owns the current-user named-pipe transport, handshake and
  typed command/result/event frames. It never opens download storage.
- `rust/apps/nxtools-cli` owns parsing, human/JSON output, stable exit-code mapping and host
  discovery. Normal commands may start the packaged desktop in background mode; the explicit global
  `--connect-only` policy only attaches to an existing host and never crosses the process-start
  boundary. It never invokes SQLite or constructs a download engine.
- `docs/architecture.md` already assigns download planning, segmentation, scheduling and persistence to Rust.
- C# remains a Windows adapter and GPUI remains the renderer.
- `Windows/TaskbarProgress.cs` requests the Windows 7+ `ITaskbarList4` interface, which inherits the
  progress methods from `ITaskbarList3`. This also supports Windows installations where
  `CLSID_TaskbarList` exposes `ITaskbarList4` but rejects a direct `ITaskbarList3` query.
- Taskbar diagnostics record state changes and 25-percent milestones; identical high-frequency
  progress commands do not generate generic command lifecycle pairs.
- A CDN can advertise byte ranges during probing but later route segments to inconsistent replicas:
  some return `206`, others return a full `200`, and validators may differ. A `200` response to a
  segment is a range-capability inconsistency rather than a transient transfer error, so a fresh task
  may discard only the current temporary segmented artifact and restart as one full stream. The safety
  boundary is whether a durable resume checkpoint existed before the attempt (`If-Range`), not whether
  another concurrent segment reported in-flight progress. Durable bytes must never be combined with
  an unconditioned full response. If a probe still validates the saved resource identity but every
  conditional segment request is answered with a full `200`, the compatible recovery is to explicitly
  discard the old segmented artifact and plan, then retry from byte zero with fresh ordinary Range
  requests. If those fresh requests are also ignored, the existing clean single-stream fallback applies.
- ABDM's `HttpDownloadJob.fetchDownloadInfoAndValidate` validates the resource-level length and ETag
  before creating/resuming parts, while `HttpPartDownloader.connectAndVerify` validates each active
  part by its response length and `Content-Range` start rather than requiring every CDN replica to
  repeat the same ETag. NxTools.Next follows that boundary: fresh concurrent segments require a valid
  `206`, exact range and length; persisted resumes additionally use `If-Range` and strict validator
  protection. Equivalent replicas must not force a completed partial artifact to be discarded and
  downloaded again.
- The taskbar overlay GPUI window can be created before PlatformHost finishes its handshake. Initial
  native placement is therefore best-effort; `PlatformNotification::Connected` must replay the saved
  anchor, offsets and size before later visibility changes are relied upon.
- The taskbar overlay has no legacy implementation to copy, but old NxTools
  `WindowAnimationService.Attach` establishes the product motion language: short opacity plus slight
  vertical-offset transitions on open and close. NxTools.Next notification surfaces use the same
  compact shape with roughly `150 ms` enter and `120 ms` exit timing. The overlay must keep its last
  visible GPUI model and native HWND visible until exit completes, show the native HWND before enter,
  and invalidate a pending hide when activity returns so a stale timer cannot hide a new transfer.

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

Reverified 2026-09-03 against `D:\Code\NxTools` after the user reported completion-window flash and
stale open/reveal behavior (old `Views/DownloadCompletedWindow.axaml.cs` closes the window after the
user confirms a post-completion action):

- Old `DownloadCompletedWindow` treats "打开文件" / "打开文件夹" as terminal user intents: it dispatches
  the Shell action and closes immediately, without observing whether Explorer actually opened. NxTools.Next
  preserves the terminal end state but strengthens the boundary: close only after the correlated
  `OpenShellTarget` result succeeds. Use `DownloadApplication::close_task_window` so the keyed
  `DownloadWindowLifecycle` registration is released with the native window; never call a bare
  `window.remove_window()` that leaves stale registration behind.
- If the Shell command is rejected, fails, times out or is invalidated by disconnect, keep the completed
  window available for retry. Bind the pending result to the originating HWND + window generation so a
  late success cannot close a replacement window. Main-list artifact actions carry no task-window identity
  and must not close the completion window. Diagnostics record both operation correlation ID and IPC
  command ID, never the target path.
- Old `MainWindowViewModel::OpenTaskFolder` selects an existing file in Explorer, opens an existing
  directory directly, and falls back to the save directory when the file is missing. The old owner catches
  Explorer launch errors, so immediate completion-window close does not prove Explorer was shown.
- A native close (title bar X, Alt+F4, taskbar close) must release the keyed lifecycle registration
  through `on_window_should_close` + `handle_task_window_native_close`, matching the
  delete-confirmation window pattern. Without it, the next task event targets the destroyed window,
  fails `update_task_window`, and the recovery path re-creates the window — the visible flash the
  user reported. After a native close of a progress window, a later progress event must NOT
  resurrect it (`task_changed` returns `None` when nothing is registered); only a genuine terminal
  transition may open the passive fallback.

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

### CDN validator and taskbar preview stabilization

Verified on 2026-08-01 after reproducing against `hkg.download.datapacket.com`:

- Different CDN replicas can temporarily return different ETag and Last-Modified values for the same Range URL.
  NxTools.Next must never write bytes from the mismatching response, but one mismatching segment response is a
  bounded retry condition rather than an immediate task-wide terminal failure. Persistent mismatch still fails
  after the existing segment retry budget.
- The taskbar preview observes Rust-owned download state. The GPUI observer must defer reading the observed
  `DownloadApplication` and updating the overlay entity until the next application turn; synchronously reading
  the still-leased application produces `RefCell already borrowed` and leaves preview editing unresponsive.
- PlatformHost temporarily tracks placement before `SetWindowPos` because Windows can synchronously emit taskbar
  location events. If placement, visibility, or a later reapply reports an invalid GPUI HWND, that tracked entry
  must be removed with a compare-and-swap so taskbar/display events cannot keep targeting a destroyed window.

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

### Phase 4 legacy browser-extension findings

Reverified on 2026-08-08 before starting browser takeover, using the old NxTools extension as the
product compatibility baseline rather than designing a replacement from memory:

- `browser_extension/nxtools-interceptor/manifest.json` is a Manifest V3 extension at version
  `0.4.0`. One source tree targets Chrome and Edge directly and is rewritten at export time for
  Firefox. It requests download, cookie, request-observation, content-script, storage, tab and
  navigation permissions and has a fixed Firefox add-on ID.
- `Services/BrowserExtensionPackageService.cs` packages the same source tree as signed CRX3 for
  Chromium browsers and XPI for Firefox. `ViewModels/MainWindowViewModel.cs` and the settings window
  expose open-folder plus Chrome/Edge and Firefox export actions; `NxTools.csproj` also copies and
  packages the extension during publish. NxTools.Next must preserve the existing extension identity,
  stored settings and one-source/multi-browser behavior wherever the browser installation model
  permits it, rather than shipping an unrelated extension with a blank profile.
- `background.js::{ensureDesktopConnection,sendDesktopRequest,sendDownloadPayload}` currently uses
  a configurable unauthenticated loopback WebSocket. `LocalApiService` accepts broad loopback HTTP,
  WebSocket and CORS callers, permits messages up to 64 MiB and stages Blob payloads from base64.
  It also logs URLs from extension diagnostics. These are legacy behaviors to replace, not security
  or diagnostics patterns to reproduce in NxTools.Next.
- The extension already has valuable product behavior to retain and simplify: per-site takeover,
  download/link/page context actions, request Header/Cookie/Referrer capture, a task/resource popup,
  page/content hooks, media resource discovery, a media-adjacent download affordance and a drop
  target. Capability negotiation must keep unsupported media/merge actions honest until phases 4/5
  actually provide their backend rather than displaying a future success message.
- Reverified on 2026-08-10 before the batch/context-action slice: the old and migrated workers both
  register link/media and page context actions, but the handler accepts every non-popup menu ID and
  paints a success badge when `sendUrl` merely resolves, even when the native response explicitly
  rejects the handoff. The user-facing action is therefore normalized to the exact
  `使用 NxTools 下载` wording, handles only its declared IDs, preserves the captured browser
  Referrer/Header boundary, and reports success only after a positive correlated acceptance.
- The migrated resource list already owns checkbox selection, but those selections only feed the
  disabled Phase 5 merge affordance. Phase 4 reuses that explicit selection for a separate
  `批量加入队列` action. One native request carries 1–32 ordinary HTTP(S) items, one parent
  correlation ID and a stable UUID handoff per item; the worker durably stores those IDs before the
  request leaves the extension, and Rust returns an identity-bound result for each
  item, so a retry cannot duplicate accepted tasks and partial failures remain selected for retry.
  Later observations of the same resource cannot erase a pending handoff ID, and acceptance resolves
  the current authoritative resource by ID before clearing only the matching handoff, so an in-flight
  recapture cannot leave an already accepted identity stuck in the cache. A single-resource send
  freezes that identity before awaiting Native Messaging, so an older response also cannot clear a
  newer in-flight handoff. Best-effort extension
  diagnostics use a separate transport request ID and carry the feature correlation as validated
  payload metadata, so a log acknowledgement cannot resolve the pending batch request; transport
  rejection still produces the batch terminal event.
  Blob items retain their proven single-resource data plane and are not smuggled through the native
  control message. Batch intake is queue-only to avoid the explicitly deferred multi-window storm
  policy; starting or confirming many visible task windows remains outside this slice.
- Chromium `interceptBrowserDownload` cancels the browser item before desktop acceptance and only
  attempts a new URL-only browser download when handoff fails. Firefox
  `handlePreemptiveBrowserDownload` cancels at response-header time before the asynchronous handoff;
  its failure path explicitly says the browser download was already cancelled. Both flows can lose
  filename, request context or the original download and therefore violate the required fallback
  invariant.
- The first NxTools.Next slice must reverse that ownership boundary: generate a stable handoff ID,
  send the ordinary HTTP(S) request through a correlated native channel, wait for a positive Rust
  acceptance containing the same ID, and only then cancel/erase the browser item. Missing host,
  timeout, rejection or malformed response leaves the browser download untouched. Rust keeps a
  bounded idempotency window so reconnect/resend returns the existing task instead of creating a
  duplicate.
- Native Messaging is the primary control plane. The extension service worker uses
  `runtime.connectNative`; C# installs/removes the current-user Chrome, Edge and Firefox host
  manifests/registry entries with exact allowed extension IDs; the launched bounded host forwards
  framed messages to the Rust intake adapter, which validates limits, origin, schema, capabilities
  and deduplication before calling the existing download command interface. C# does not create a
  task or own browser policy.
- Official Chrome and Edge Native Messaging documentation requires the `nativeMessaging`
  permission, an exact `allowed_origins` list and length-prefixed UTF-8 JSON over stdio. Chrome caps
  a host-to-browser message at 1 MiB. Firefox uses the same extension API but its host manifest uses
  exact `allowed_extensions`. NxTools.Next therefore applies a smaller product-level bound to all
  control messages and never sends Blob bytes through that channel.
- Reverified on 2026-08-10 before the bounded Blob slice: the old page hook retains the `Blob`
  behind `URL.createObjectURL`, converts up to 48 MiB to a data URL, and sends `blobBase64` through
  the extension WebSocket. `LocalApiService::PrepareBrowserBlobRequest` decodes the entire payload
  with `Convert.FromBase64String`, writes a cache file, replaces the request URL with its broad
  loopback `/api/browser-blobs/...` endpoint, and then lets the existing `openDialog` path prefill
  the ordinary add-download window. This verifies the product behavior to preserve (revoked Blob
  URLs remain downloadable and `openDialog=true` still confirms destination/name) while also
  confirming the legacy memory amplification, 48 MiB ceiling and unauthenticated local-server
  boundary that NxTools.Next must not reproduce.
- The NxTools.Next Blob slice keeps Native Messaging as a metadata/control plane. A prepare request
  creates a short-lived, one-use, high-entropy capability scoped to one declared Blob size; the
  page-owned `Blob` is read as bounded transferable `ArrayBuffer` slices; the isolated content
  script, not the untrusted page realm, POSTs those chunks to a random loopback port and streams
  them directly into the current user's application cache without Base64 or a generic API. The
  listener accepts only the exact capability path, monotonic offset, bounded chunk/total lengths,
  one upload, a short rolling deadline and small connection/active-transfer limits. Capability
  values, page URLs and bytes are never exposed to page scripts or logged. Only a
  completely received staging file may be committed through the typed Rust download command.
  Failed prepare/upload/commit, timeout, size mismatch or host loss leaves the page's browser
  fallback available.
- The Rust engine, not BrowserHost or C#, owns the committed task and final artifact policy. For
  `openDialog=true`, the staged Blob reaches GPUI as a private typed dialog source with prefilled
  name/size and no network probe; confirmation imports it to the selected destination and records a
  completed browser-origin task through a compensated artifact/database commit. A database failure
  restores the artifact to staging; rollback failure is an explicit correlated terminal error and
  is never swallowed. Direct mode imports to the configured default download directory. The engine
  accepts staging paths only from its dedicated application-data staging root and verifies the exact
  file size again before the move/copy boundary.
- Reverified against old `MainWindowViewModel::ConfigureLocalApi` and
  `OpenDownloadDialogFromLocalApi` after the first packaged browser test: `openDialog=true` means
  “prefill and open the add-download confirmation window, then parse”, not “persist a `created`
  task without starting it”. NxTools.Next therefore separates `BrowserOpenDialog` from direct
  `BrowserSubmit`. The former is accepted only while GPUI has attached a bounded private in-process
  receiver and confirms successful window creation within five seconds; a headless DownloadHost,
  timeout or window-creation failure rejects it. If no pipe exists, BrowserHost launches the
  protected `NxTools.exe --browser-dialog --browser-correlation-id <id>` for a window request,
  waits for the pipe and retries only the
  typed, retryable consumer-not-attached startup result while GPUI attaches its receiver. Queue
  delivery and GPUI window-creation failures are non-retryable; it never substitutes a headless host
  for this path. The dedicated launch intent makes the ordinary main window hidden/non-focusing;
  after PlatformHost connects, a typed native command shows it with `SW_SHOWMINNOACTIVE` and accepts
  success only after `IsWindowVisible + IsIconic` readback. The add-download surface independently
  keeps the existing topmost and foreground-activation commands. URL/request context reaches
  `NewDownloadView` without entering the
  serializable event stream, preserving Header/Cookie/Referrer confidentiality. Direct submissions
  remain persistently idempotent and start `NxTools.exe --background` when no engine is available.
- Official primary sources checked: Chrome
  `https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging`, Microsoft Edge
  `https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging`,
  and Mozilla
  `https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging` plus
  `Native_manifests`. Chrome's messaging guidance also treats content-script messages as untrusted;
  every page-derived field must be validated again in the privileged extension worker and Rust.
- Reverified the old and migrated popup/capture implementation before the Phase 4 extension-quality
  pass. The old popup CSS is byte-identical to the migrated popup CSS: a fixed 520 px, card-heavy
  four-tab surface with no capture-health summary. This is a compatibility baseline for available
  actions, not a visual constraint; the user explicitly requested a denser NxTools.Next treatment.
- The migrated content bridge retains the old 3.5-second full-page scan in addition to its
  `MutationObserver` and `PerformanceObserver`; each full scan also calls `getComputedStyle` on up
  to 360 elements for background images. Background request/response capture separately calls
  `chrome.storage.local.get` on every candidate. The quality pass must cache settings in the
  service worker, keep event-driven capture authoritative, and reserve the expensive background
  image scan for an explicit/deferred deep scan while preserving dynamic media discovery.
- Captured resource snapshots arrive from request, response, DOM, page-hook and Performance
  boundaries with different authority. A later weak DOM/Performance name or empty page context
  must not erase a Content-Disposition filename, response MIME/size, Referrer or useful request
  headers. Popup state needs only a bounded presentation projection; Header/Cookie/Referrer remain
  in the privileged worker for the eventual handoff and must not be serialized through each popup
  refresh.
- Post-review hardening assigns separate authority to filename, response metadata and request
  context. Browser `webRequest` snapshots outrank page observations; page-supplied Header objects
  are ignored rather than merely length-checked. The same merge policy is used for direct cache
  updates and same-URL reconciliation. Persistent resource snapshots omit Header, Referrer, page
  URL, page title and URL-derived page key, and startup rewrites legacy stored snapshots through
  that projection.
- Chromium extension popups need deterministic intrinsic root dimensions. A responsive webpage
  pattern such as `width: min(456px, 100vw)` can participate in the browser's initial Popup viewport
  calculation and collapse the surface to a narrow strip. Keep `html/body` at the intended fixed
  Popup width and constrain the internal scrolling workspace with fixed bounds; test viewport-unit
  absence at that root boundary separately from ordinary responsive-page rules.
- The old Avalonia startup used `WindowState.Minimized` before assigning the main window. GPUI 0.2.2
  has no initial-minimized `WindowOptions`; showing first and minimizing later can flash and cannot
  report an observed native result. NxTools.Next therefore creates this one startup main window with
  `show=false` and delegates `SW_SHOWMINNOACTIVE` plus `IsIconic` readback to PlatformHost.

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
- `OkHttpHttpDownloaderClient.newCall` adds the inclusive `Range` header for a part but does not
  attach `If-Range`. `HttpPartDownloader.connectAndVerify` validates the returned range and length
  before the destination writer is used.

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

`DownloadManagerService.CreateProbeRequest` and `DownloadSegmentAsync` likewise send `Range`
without `If-Range`; the segment path requires `206` and validates `Content-Range` before writing.

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
- A fresh transfer plan sends ordinary `Range` requests. It does not send `If-Range`, because some
  otherwise range-capable CDNs select a different representation when that conditional header is
  present. Every fresh partial response still has to match the probed validator before any bytes
  are written.
- A persisted plan with durable bytes uses `If-Range` when recreating its connections. If the
  server returns the full representation or changes validators, those old bytes are not combined
  with the response. Reverified from the 2026-08-08 user reproduction and correlated desktop log:
  some CDN nodes return `200` to all conditional ranges even though the immediately preceding probe
  reports the same length, Range capability and ETag, while fresh unconditioned ranges return exact
  `206` responses. In that narrow case the engine may explicitly reset the durable plan and retry as
  a fresh segmented transfer; it must log the discarded durable byte count and must never reuse the
  old artifact in the unconditioned attempt.
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
- The selected detail row must always belong to the visible result. Following the 2026-08-10
  interaction correction, an empty selection stays empty instead of silently selecting the first
  visible task; the bottom detail dock is rendered only after an explicit row selection. Filtering
  out every selected task clears the detail pane and shows the matching list or filter state.
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

Reverified on 2026-08-13 for the P0 task-table refinement:

- ABDM's optional Speed and Time Left columns are meaningful only when they remain vertically
  stable across every row. NxTools.Next keeps the same invariant in one bounded transfer region:
  downloaded/total, live speed and estimated remaining time use fixed subcolumns above the existing
  3 px progress track. Missing live values render as an em dash instead of changing the row shape.
- The task snapshot remains authoritative for durable downloaded/total values, while the desktop's
  already-bounded `DownloadProgress` cache supplies live bytes, rate and remaining time. The list
  must not invent a second sampler, timer or per-byte diagnostic path.
- The 46 px row remains two-line: 12 px file name and 11 px path/error. A failure replaces only the
  auxiliary line with danger text; the complete bounded message remains available through the
  selected detail dock and a 320×144 px scrollable content viewport inside a tooltip surface bounded
  to 340×170 px, so it cannot compress the primary name or transfer columns.
- At the compact production window width the flexible name column yields before the fixed transfer
  and status columns, but retains enough width for useful file identity. Header and row geometry are
  verified through the production download workspace rather than a component-only fixture.

Reverified on 2026-08-22 after the packaged main-list review:

- AB Download Manager's `TableDownloadItem.kt::NameCell -> FileIconProvider::rememberIcon` places
  the operating-system file-type icon directly beside the name, while `OSFileIconProvider` caches
  it by extension. Its failure row uses the mapped `DownloadErrorReason.title`; the raw throwable
  message remains supporting diagnostic detail rather than becoming the table label.
- Old NxTools `Views/MainWindow.axaml` used one generic `draft` glyph for every task and
  `DownloadTaskViewModel` supplied localized status text. The generic glyph is a compatibility
  baseline only: it does not preserve file identity and is not carried into the corrected Next
  workbench.
- Before this correction, NxTools.Next likewise rendered every task as a tinted Download glyph in
  a raised 26 px tile and copied `DownloadFault.message` directly into the auxiliary line.
  That exposed internal English protocol strings and task UUIDs in the primary scanning surface.
- The corrected row reuses the existing typed `ExtractShellIcon` PlatformHost boundary and its
  bounded PNG cache. It requests one 24 px Windows Shell icon per unfinished file-type resource and
  uses the actual target path for completed tasks, then renders the result in
  the existing transparent icon slot and keeps a neutral file fallback while the request is pending
  or rejected. Rust continues to own the task/icon association; C# only adapts the Windows Shell.
- Failure rows derive one short localized summary from the typed error category and known protocol
  cases. The complete bounded engine message remains in the existing tooltip and detail dock, so
  diagnostics are preserved without letting correlation IDs or raw transport prose dominate the
  task table. Icon completion and failure notify the observed `DownloadApplication` only; they do
  not mutate task state, queue order, selection or download lifecycle.
- Old NxTools also exposed `CreatedAtCompactText` as the final fixed table column. NxTools.Next keeps
  that useful scan field as a directly sortable 68 px `添加时间` column at all supported window widths:
  today's tasks use `HH:mm`, older tasks use `MM-dd`, and the full local timestamp remains in the
  tooltip. The 920 px native minimum and 1060 px default both keep this column visible; the existing
  compact sort menu retains the same Created sort as an alternative input path.

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
- A browser-created add window cannot rely on GPUI `focus: true`: the browser owns the Windows
  foreground lock when Native Messaging completes. Resolve the real GPUI HWND, request legacy
  always-on-top first, then use the existing PlatformHost `ActivateWindow` path. This is the same
  typed Windows boundary already proven by keyed progress-window activation; GPUI remains the
  window owner and C# only performs the native topmost/foreground adaptation.
- The initial window is a compact URL/probe stage. Destination, editable file name and remote
  metadata appear only after a successful probe; the probe result must hydrate the visible file-name
  input immediately. Destination and file-name edits do not invalidate remote metadata, while a URL
  edit does require a new probe.
- A browser handoff already carries a populated download request and opens the complete confirmed
  form height before its automatic probe begins. It must not rely on a later asynchronous native
  resize to make destination, file-name and footer controls reachable; a manual empty form keeps the
  compact URL-only height. This browser-specific correction was verified on 2026-08-10.
- When a successful asynchronous probe changes the add window from its compact URL stage to the
  confirmed form, the probe result and required window size are one generation-scoped transition.
  The real HWND is resized through native `SetWindowBoundsCentered`, which derives the current
  visual center from the native rectangle rather than GPUI's cached origin.
- A user may keep moving the non-resizable add window while the probe is in flight. Probe completion
  must not leave the confirmed body in the old compact client rectangle; stale probe generations
  must not apply an old height.
- Reverified 2026-10-05 at the native GPUI/PlatformHost boundary: the Rust
  `SetWindowBoundsCenteredCommand.width/height` values are physical client/content pixels
  (the same dimensions GPUI receives through `WM_SIZE`), not DWM extended-frame bounds.
  PlatformHost reads the current outer frame plus screen-space client rectangle, converts the
  requested client size to one atomic `SetWindowPos`, then applies a bounded visible-center
  correction outside a move/size modal loop. Recovery reuses the same target client dimensions
  and center correction. Low-noise diagnostics carry the command correlation ID and record only
  HWND, requested/actual client and visible dimensions, recovery state, native error and elapsed
  time; no URL, path, file name or payload is logged.
- A real temporary Win32 HWND regression now asserts both client/content dimensions and the
  moved-window visible center for direct resize and move-end replay. The GPUI production test
  still asserts the confirmed form, inspector and footer fit above the fixed footer. These tests
  do not replace the still-required live drag-during-probe smoke at the user's DPI.
- Native client correction also depends on the actual P/Invoke entry names:
  `WindowsPlatformAdapter.ClientResize.cs::{ApplyClientResizeWithFrameChange,
  TryGetClientResizeScreenBounds}` calls `GetWindowRect`, `GetClientRect`,
  `ClientToScreen`, and `SetWindowPos`. Managed method prefixes are not export
  names; each prefixed declaration must specify its Win32 `EntryPoint`.
  The custom-client-chrome HWND regression exercises this mismatch branch,
  not only the system-dialog path where client dimensions already match.
  Final client readback must occur after visible-center correction, and a
  mismatch must keep generation-scoped recovery pending rather than report
  completion. A presentation timeout must never release a clipped form.
- The bounded confirmed-state inspector has one generation-keyed 120 ms opacity-only enter using the
  existing GPUI motion token and easing. It never animates native bounds or changes footer geometry.
- Download storage startup is defensive at the SQLite boundary. `SqliteDownloadStore::open` avoids
  rewriting an already-WAL database, retries bounded `SQLITE_IOERR_TRUNCATE`/busy/locked startup
  failures, and quarantines a zero-byte database only when no `-wal`/`-shm` sidecars exist. A file
  with sidecars is left untouched so SQLite can recover it; an isolated empty placeholder is kept
  as a uniquely named `.empty-*.bak` artifact before a fresh schema is created.
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
- The compact NxTools.Next `NewDownloadView` exposes optional local date/time scheduling only after
  a successful probe. The fixed footer owns one compact calendar-icon trigger; clicking it opens an
  anchored date-time surface with a compact Chinese calendar (`2026年7月`, Sunday-first grid) plus
  non-editable hour and minute selectors. The surface does not participate in window height or push
  the footer below the viewport. `确定` commits the draft, `清除` returns to immediate download, and
  clicking outside discards only the open surface. It rejects past, invalid and ambiguous local
  times before submission and preserves the committed selection across a probe refresh. A delayed
  `StartNow` submission must not open a progress window early; the existing keyed window opens only
  once the engine admits the task.
- Reverified on 2026-08-13 for the P0 shared-Popover migration: old NxTools exposes the one-time
  schedule as a peer form field (`Views/MainWindow.axaml:3203-3218`) and ABDM keeps scheduling in a
  normal configuration group (`QueueInfoComponent.kt`); neither requires a separate decorative
  card. NxTools.Next therefore keeps its anchored calendar interaction but renders it through the
  same 6 px bounded Popover shell as category selection. The calendar uses Label text for Chinese
  dates/days and Meta only for weekday metadata; its Clear/Confirm actions use the shared 28 px
  action tier in one separated footer, while month navigation uses shared square icon actions with
  visible tooltips. Opening snapshots the committed schedule into a local draft,
  outside/trigger dismissal never commits that draft, and Clear/Confirm update local state
  before the controlled surface closes. No date, time or schedule draft is added to diagnostics.
- Calendar and month-navigation glyphs use locally embedded Google Material Symbols
  (`calendar_month`, `chevron_left`, `chevron_right`). The desktop asset source does not expose the
  full gpui-component icon bundle, so using an unregistered `IconName` produces an empty button in
  the packaged application even when development compilation succeeds.
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
- Reverified on 2026-07-30 for the user-requested capability-driven menu change: ABDM and the old
  NxTools keep a fixed action topology and lower the opacity of unavailable actions. NxTools.Next
  intentionally diverges for its denser editor-style surface: the task context menu is assembled
  from the selected tasks' current capabilities, so unavailable lifecycle and queue groups are
  omitted rather than merely disabled. A completed task with a final artifact shows open, reveal,
  task-window, delete-file and remove actions only; empty groups never leave leading, trailing or
  repeated separators. Mixed-state lifecycle actions target only the selected tasks that support
  that action, avoiding partial batch failures; queue and remove remain all-selection operations.

Reverified on 2026-08-14 before the DL-06 visual remediation slice:

- ABDM still keeps status/category filtering upstream of the stable-ID table and exposes sort
  through the table model; old NxTools still applies status first and search second, while its
  sortable headers change presentation order only. Neither reference turns filter/search/sort
  controls into queue or transfer commands.
- NxTools.Next keeps the six existing status predicates and their counts, but presents them as one
  shared segmented control: one 28 px outer boundary, one selected cell and six equal-width cells.
  Counts are inline metadata rather than a second row of badges, so All, Downloading, Waiting,
  Paused, Completed and Failed cannot change width as counts change.
- Search remains in the module header and uses one 28 px editor-style field with a bounded clear
  action. The result summary moves into the header metadata so the 38 px filter strip contains only
  the status group and one compact sort action.
- The sort menu remains necessary for Queue, Size and Date Added, which have no dedicated visible
  table column at compact width. Name, Progress and Status headers continue to toggle direction;
  both entry points update the same local `DownloadSortState` and never persist queue order.
- Required regression: open the production download workspace at the compact 720 px width, prove
  all six status cells are equal-height/equal-width inside one bounded group, click a real status
  cell, drive the real search input and clear action, and change sorting through both the menu and a
  table header without issuing a download command or changing the authoritative task snapshot.

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

### Phase 3 keyed task-window reopen and foreground findings

Reverified on 2026-07-30 before implementing the sixth Stage 3 slice:

- ABDM `desktop/app/.../AppComponent.kt::openDownloadDialog` first searches
  `openedDownloadDialogs` by download ID. A hit calls
  `DesktopSingleDownloadPageComponent::bringToFront`; only a miss navigates a new keyed component
  into the dialog collection. `ShowDownloadDialogs.kt::CommonContent` handles that effect by
  clearing `WindowState.isMinimized` and calling the native window's `toFront()`. Reopening never
  creates a second component or changes the job lifecycle.
- Old NxTools `Views/MainWindow.axaml.cs::{ShowDownloadProgressWindow,
  BringSecondaryWindowToFront}` uses one task-ID dictionary. A live hit restores visibility,
  pulses topmost and asks `WindowActivationService` to bring that exact window forward; a closed
  entry is removed before a later request creates a replacement. The requested task's current
  status chooses progress or compact completion presentation.
- NxTools.Next already owns one `DownloadTaskWindow` per task ID and routes main-list double-click
  and the single-selection context-menu action through `request_task_window`. GPUI activation alone
  is not the complete Windows boundary: reuse must also send the existing typed
  `ActivateWindow { window_handle }` command so C# can restore an iconic HWND, attach the relevant
  input threads and call `BringWindowToTop`/`SetForegroundWindow`.
- An explicit reopen is activating, including completed and failed tasks. Automatic terminal
  fallback remains passive and must not be changed by this slice. If the cached GPUI/native handle
  is stale, the registry is cleared and exactly one replacement is opened in the phase derived
  from the current Rust task status; it must not fall back unconditionally to the progress phase.
- Diagnostics record one bounded reopen request and the activation command's accepted/terminal
  result using task ID, window handle, correlation ID, elapsed time and success only. They do not
  record URL, path, file name, Header, Cookie or other task payloads. Repeated refresh/progress
  frames do not emit activation diagnostics.

### Phase 3 download-settings and interaction-finish findings

Reverified on 2026-07-30 before implementing the remaining user-selected Phase 3 work:

- ABDM `desktop/app/.../pages/settings/DesktopSettingsComponent.kt` groups download-folder behavior
  separately from engine controls. Its engine group exposes the default folder, speed limit,
  connection/part count, concurrent downloads and retry policy; its appearance group owns automatic
  progress and completion-window preferences.
- ABDM `shared/app/.../settings/CommonSettings.kt` validates every numeric setting against an
  explicit range and treats zero speed as unlimited. Folder selection uses a native picker and
  validates write access instead of requiring a manually typed path.
- Old NxTools `Views/SettingsWindow.axaml` presents download behavior in a dedicated settings
  surface. `Models/AppState.cs` and `Services/AppStateStore.cs` persist user policy separately from
  transient task progress.
- NxTools.Next exposes only controls already backed by a real runtime boundary in Phase 3:
  default save directory, default connection limit, default-queue concurrency, global speed limit,
  automatic progress-window behavior and completion-window behavior. Retry-count, proxy, browser
  integration and media settings remain absent until their engine phases provide real mutable
  contracts.
- The settings surface is one keyed, client-decorated GPUI window. It edits a local draft; close or
  cancel discards it. Apply validates every field first, updates the download engine through typed
  commands, persists desktop policy, refreshes the authoritative snapshot and emits low-frequency
  correlated accepted/succeeded/failed diagnostics.
- The native directory picker result is routed to the requesting surface. A directory selected for
  settings must not overwrite an open new-download form, and a new-download selection must not
  mutate the settings draft.
- The task context menu derives queue placement from the authoritative default-queue order. A
  selection already touching the first queue position omits `优先下载` and `上移`; one touching the
  last position omits `下移` and `移到底部`. Empty action groups never leave leading, trailing or
  repeated separators.

Reverified on 2026-08-12 after the expanded Stage 5/6 settings exposed a layout regression:

- Old NxTools `Views/SettingsWindow.axaml` uses a stable `984 x 612` workspace with a `224 px`
  navigation rail and one selected settings page. It does not place every unrelated settings group
  in one content-sized vertical strip.
- ABDM `DesktopSettingsComponent.kt` likewise owns explicit settings pages and changes the bounded
  group list when a page is selected. Its layout hierarchy is page first, configurable group
  second; child controls never determine the width of the entire settings workspace.
- NxTools.Next must retain one definite-width content column beside a compact navigation rail.
  Every selected-page section shares that column width; two-column rows may subdivide it but must
  not be the mechanism that stretches the parent. Configuration, log and database paths stay on
  separate full-width diagnostic rows so long paths cannot collide across half-width columns.
- Page switching is local GPUI presentation state. It resets only that page's scroll position and
  must not mutate the settings draft, start native work, or add diagnostics. Apply/Cancel and the
  existing directory, FFmpeg and taskbar-edit lifecycles remain unchanged.

### Phase 3 taskbar-internal aggregate progress findings

Reverified on 2026-07-30 before implementing the user-selected taskbar overlay slice:

- ABDM
  `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/ShowDownloadDialogs.kt::UpdateTaskBar`
  maps one task window's current percent and lifecycle state to Java `Taskbar`; it does not inject
  into Explorer and does not render a persistent aggregate strip inside the taskbar.
- Old NxTools has no taskbar-internal aggregate progress surface. Its download progress and
  completion windows remain ordinary top-level windows, and `ShowInTaskbar` controls only whether
  those windows receive taskbar buttons. There is therefore no legacy placement or focus behavior
  to preserve for this new surface.
- NxTools.Next already has the typed `SetTaskbarProgress` seam for per-task window buttons and a
  low-noise C# `ITaskbarList4` adapter. The taskbar-internal overlay is a separate interface: Rust
  produces at most three compact presentation lanes (overflow tasks share the last weighted lane),
  C# anchors one external GPUI HWND to the taskbar, and GPUI renders it. It must not alter or replace
  existing per-task taskbar-button progress.
- The visual result is an overlay inside the taskbar's upper edge, but implementation remains
  out-of-process from Explorer. The HWND is no-activate, tool-window, topmost and click-through;
  left/right anchor and X/Y offsets are logical pixels converted with the taskbar DPI and clamped
  to its physical rectangle.
- Placement is event-driven. Display/settings changes and taskbar create/show/location events
  reapply the last configuration; ordinary byte ticks update only the GPUI presentation and never
  trigger native positioning or diagnostic writes. The Windows adapter publishes the newly
  requested placement before calling `SetWindowPos`, because that native call can synchronously
  produce a taskbar-location event; the reapply path must never race in with the previous offsets.

Reverified on 2026-07-31 before extending the overlay editor and adaptive connection policy:

- The first NxTools.Next overlay slice is deliberately minimal: `download_taskbar_overlay.rs`
  renders at most three two-pixel task lanes inside a fixed `132 x 8` logical-pixel window.
  A single active task therefore produces one lane. The model carries only status, downloaded
  bytes and total bytes; speed, remaining time, task count, connection count and byte totals are
  not yet presentation inputs.
- The extended surface remains one GPUI window. Rust owns its persisted width, height, anchor,
  offsets, lane mode and visible-information mask; GPUI owns preview/edit chrome and content;
  C# only applies taskbar DPI, bounds and passive-versus-editable Win32 styles.
- Normal mode is non-activating, tool-window, topmost and click-through. Settings edit mode may
  temporarily remove click-through so the user can drag and resize a deterministic preview even
  when no task is active. Finishing persists the draft; cancelling restores the last saved
  placement. Leaving edit mode restores passive styles.
- Visibility diagnostics on 2026-08-08 measured about 20 ms from queue admission to Rust show and
  54 ms to the native visible result, so the reported delay was perceptual rather than IPC latency.
  The enter motion now completes in 90 ms and starts at 45% opacity with only 1.5 px translation;
  exit remains 120 ms so native hide still waits for the complete fade without abrupt removal.
- Overlay dimensions and offsets are bounded and clamped to the primary taskbar. Pointer movement
  is not logged. Diagnostics cover edit begin, committed/cancelled, native placement accepted and
  terminal placement failure under one correlation ID.
- Optional information is independent of transfer execution. Progress lanes stay available in
  aggregate, per-task and per-segment modes; textual speed, remaining time, active-task count,
  active-connection count and downloaded/total bytes are separately switchable. Unknown totals or
  rates use an honest indeterminate/placeholder state instead of a fabricated value.

### Runtime segment-splitting and adaptive-connection findings

Reverified on 2026-07-31:

- ABDM `HttpDownloadJob.kt::getPartDownloader` fills an available downloader slot from an inactive
  part first. When none exists and `dynamicPartCreationMode` is enabled, it selects splittable
  active parts by descending remaining length, calls `HttpPartDownloader.splitPart`, persists the
  new range in the job's part collection and starts it in the free slot. `PartSplitSupport` protects
  a moving safe zone so the old and new downloader never overlap.
- Current NxTools.Next implements the same safety shape: when a worker slot becomes available it
  chooses the largest remaining active segment, asks that worker to flush/synchronize, atomically
  splits the transfer plan and restarts both non-overlapping ranges. The regression
  `dynamic_rebalance_splits_a_slow_tail_without_overlapping_output` protects this boundary.
- Old NxTools `DownloadManagerService.DownloadSegmentsAdaptiveAsync` starts the initial segment set
  at the configured `ThreadCount` and keeps a separate connection limiter at that same hard cap.
  Its adaptive accelerator grows the number of queued/persisted segments, not the number of active
  connections: after five aggregate-speed samples remain within roughly 15 percent deviation it
  splits up to four useful tails, then waits until the full batch exists before collecting a fresh
  five-second benefit window. It stops automatic splitting after any segment retry or when the
  observed gain is below roughly 80 percent of the proportional segment increase. Defaults are
  enabled, 32 maximum segments and 3 MiB minimum remaining tail.
- NxTools.Next retains the safe split/restart transaction and layers that speed-sampled segment
  policy above it. A new task starts all configured connections immediately. Additional split
  ranges wait in a Rust-owned queue whenever all connection slots are occupied, so the persisted
  segment count can grow without violating the connection cap. Rust owns sampling, batch state,
  benefit evaluation, queueing and persistence; C# and GPUI do not participate in scheduling.
- The old implementation applies its enable switch, maximum segment count and minimum remaining
  tail to both stable-speed acceleration and the split attempted after a segment completes. The
  Next implementation follows that same rule: disabling adaptive segmentation must prevent every
  automatic split path, and a queued segment remains `Waiting` until a connection slot actually
  starts it. Persisted `Active` count therefore never exceeds the request's connection cap.

### Stage 5 HLS master-playlist and quality-selection findings

Reverified on 2026-08-10 before starting the first Stage 5 slice:

- AB Download Manager keeps HLS behind a distinct downloader boundary. Its
  `shared/app/.../hls/HLSLinkChecker.kt` reads the manifest body during link checking and carries
  the parsed manifest into `HLSDownloadJobExtraConfig`; HTTP and HLS are not inferred later from a
  downloaded `.m3u8` file. `downloader/core/.../hls/HLSResponseInfo.kt` bounds suspicious
  non-HLS responses to 2 MiB, requires at least one media segment and currently accepts only
  unencrypted TS media playlists. That implementation does not resolve master-playlist variants,
  so it is not copied as a complete quality-selection model.
- Old NxTools provides the product-specific master-playlist behavior in
  `Services/HlsPlaylistParser.cs::{Parse,SelectBestVariant,ResolveUrl}`. It recognizes
  `#EXT-X-STREAM-INF`, resolves the following URI against the effective playlist URL and preserves
  bandwidth, average bandwidth, resolution, codecs, frame rate and audio/subtitle group identity.
  The fallback choice is the highest advertised bandwidth.
- Old NxTools exposes those variants through
  `DownloadManagerService.BuildHlsResourceOptionsAsync` and
  `AddDownloadDialogViewModel.ApplyHlsResources`. The visible label combines resolution, frame
  rate, bitrate and codecs; an explicit selection changes the proposed output name. Submission is
  rejected when a discovered master list has no selected resource. URLs and request credentials in
  the old diagnostic messages are not compatible with the Next diagnostic contract and must not be
  carried forward.
- NxTools.Next's first vertical slice will keep HLS probing inside the Rust download engine. A
  manifest response is read with a hard 2 MiB cap, UTF-8/BOM handling and a maximum of 32 variants.
  Relative variant URIs are resolved against the final redirected manifest URL, a media playlist
  must contain at least one `EXTINF` segment, and every resolved URI remains HTTP(S). Variant URLs
  and presentation attributes are separately capped, then the complete HLS probe is checked against
  a 512 KiB serialized budget so its result envelope stays below the 1 MiB IPC frame. The probe
  returns only bounded presentation metadata; request headers, cookies,
  manifest text and URLs are never written to diagnostics.
- The add-download GPUI remains compact while the manifest is unconfirmed. A confirmed master list
  expands once, presents a single explicit quality choice sorted from strongest to weakest, and
  keeps ordinary HTTP confirmation unchanged. The selected playlist identity must be carried by a
  typed Rust request before HLS transfer is enabled; it must never fall through to the ordinary HTTP
  downloader and falsely complete after saving only the manifest.
- The Apple Bip Bop master used by the Stage 5 acceptance gate was rechecked after the packaged UI
  exposed five rows all labelled `自动`. That manifest currently advertises only `BANDWIDTH` and
  `CODECS`; it does not contain `RESOLUTION` or frame-rate attributes. The parser is therefore
  correct to preserve an absent resolution, but GPUI must not present the absence as an automatic
  quality decision. It labels the row as a video or audio stream from the bounded codec metadata,
  keeps the advertised bitrate visible, and never invents dimensions by inspecting media payloads.
  Confirmed HLS/DASH windows size themselves from the bounded visible row count so a one-row DASH
  choice does not retain the maximum scroller height as blank space.

Reverified on 2026-08-10 before the first HLS transfer slice:

- AB Download Manager's `HLSDownloadJob.kt::{actualBoot,fetchDownloadInfoAndValidate,updateParts,
  beginDownloadParts,saveParts}` keeps media parts as durable domain state, limits active part
  downloaders to the requested connection count, and revalidates every completed part against its
  artifact before resuming. Part state is flushed before persistence, and a completed part is reused
  by media-sequence identity only when its duration remains compatible.
- Old NxTools' `DownloadManagerService.cs::{DownloadHlsAsync,
  DownloadHlsSegmentsWithBoundedWorkersAsync,DownloadHlsSegmentWithRetryAsync,
  MergeHlsAndVerifyAsync}` writes each media segment to a deterministic task working directory,
  skips only nonempty/expected-length artifacts, uses bounded workers, and concatenates in playlist
  order into a temporary output before the final move. `HlsPlaylistParser.cs::Parse` also tracks
  `EXT-X-ENDLIST`, media sequence, byte ranges, initialization maps and encryption state.
- The first Next transfer slice intentionally accepts only finite `EXT-X-ENDLIST`, unencrypted,
  non-BYTERANGE media playlists without an initialization map. AES-128, fMP4 maps, byte ranges and
  live-window refresh remain explicit typed rejections until their own red/green slices; they must
  never silently download incorrect bytes.
- The selected media-playlist URL becomes typed `DownloadRequest` state and survives SQLite reopen.
  Parsed media segments are persisted as bounded per-task records. A segment is downloaded to a
  staging file, flushed and atomically renamed before its completed checkpoint; recovery resets a
  completed checkpoint when its artifact is missing and adopts a valid completed artifact left by a
  crash before the database checkpoint. URLs, headers, cookies, keys and local paths are not logged.
- Finalization concatenates completed segments in media-sequence order into the task temporary
  artifact, flushes it, and uses the existing atomic final rename. Pause/cancel stops bounded workers;
  pause preserves completed segment artifacts, while cancellation removes task-owned staging.

Implemented and deterministically verified on 2026-08-10:

- `DownloadRequestMedia::Hls` carries the selected playlist across command protocol v5 and task
  persistence; GPUI no longer permits the ordinary HTTP adapter to complete a manifest as media.
- SQLite schema v6 stores one bounded HLS plan plus ordered Segment rows. `Active` returns to
  `Waiting` after pause/failure, a flushed atomically renamed artifact becomes `Completed`, missing
  or mismatched completed artifacts are reset, and a nonempty orphan artifact is adopted.
- The Rust engine uses a bounded `JoinSet` capped by the task connection limit. Aggregate progress
  includes durable completed bytes plus active-worker bytes, while merge order remains playlist
  order regardless of completion order.
- Real loopback HTTP tests cover ordered output, observed parallelism without exceeding the cap,
  and pause -> close engine -> reopen -> resume. Parser tests pin typed rejection of live,
  `SAMPLE-AES`, unsupported key formats, `EXT-X-MAP` and `EXT-X-BYTERANGE` inputs; AES-128 support
  is supplied by the subsequent slice below.
- The 2026-08-11 review pass made resume re-fetch the selected finite playlist. Only identical
  media-sequence/segment identity and duration layout may reuse checkpoints; compatible refreshed
  URLs are persisted without resetting completed state, while incompatible VOD identity fails
  before transfer. The loopback gate counts the first segment request to prove it is not fetched
  twice after reopen.
- HLS bandwidth waits now select the same pause/cancel directive as ordinary transfers, and a final
  stop check closes the last-worker race before merge. Cancellation clears its plan only after both
  segment staging and merge staging are actually removed; cleanup failure is a correlated failed
  terminal that retains ownership. Checkpoint reconciliation uses one aggregate event instead of a
  per-segment success log.
- Paused/failed cancellation and history removal use the same task-ownership cleanup rule. They
  remove both the HLS Segment directory and merge temporary artifact before publishing success or
  deleting the durable plan; a real filesystem obstruction keeps the prior task/plan recoverable.

Reverified on 2026-08-11 before the AES-128 vertical slice:

- RFC 8216 sections 4.3.2.4, 5 and 6.3.6 define whole-segment `AES-128` as AES-128-CBC with PKCS#7
  padding. An `identity` key file is exactly 16 binary octets. `EXT-X-KEY` applies until the next
  tag for the same key format; `METHOD=NONE` returns following segments to plaintext. An explicit
  IV is a 128-bit hexadecimal value; without one, the segment Media Sequence Number—not its array
  index—is encoded big-endian into a zero-left-padded 16-byte IV.
- Old NxTools `HlsPlaylistParser.cs` carries the current `HlsEncryptInfo` onto each subsequent
  Segment and resolves relative key URIs. `DownloadManagerService.cs::{GetHlsKeyAsync,
  DecryptAes128}` caches 16-byte keys by URI and decrypts AES-128-CBC/PKCS#7 before publishing each
  Segment. Its `BuildDefaultHlsIv(segment.Index)` is not retained because it is wrong whenever
  `EXT-X-MEDIA-SEQUENCE` is nonzero. Its per-segment logs also expose key URIs and IVs and therefore
  violate the Next diagnostic privacy contract.
- AB Download Manager `HLSResponseInfo.kt::isMediaPlayListEncrypted` still rejects every encrypted
  media playlist, so it supplies no compatible decryption behavior. The Next slice keeps AES work
  inside Rust, persists only bounded encryption metadata (never key bytes), refetches keys after
  restart, reuses a key at most once per runtime key slot, and accepts only absent/`identity`
  `KEYFORMAT`. `SAMPLE-AES`, unsupported key formats, MAP and BYTERANGE remain typed rejections.
- Recovery compatibility compares the encryption method, explicit-IV value and rotation-slot
  layout while allowing authenticated key URLs to refresh. Completed plaintext Segment artifacts
  remain reusable only under that compatible identity; pending encrypted bytes are never treated as
  durable progress. Diagnostics may record only bounded counts, key-slot/Segment indices, fixed
  error kinds and elapsed time—never key URLs, key bytes, IVs, cookies or request headers.

Implemented and deterministically verified on 2026-08-11:

- `HlsSegmentEncryption` carries plaintext or AES-128 identity metadata on every Segment. SQLite
  schema v7 persists this bounded metadata as JSON with a v6 default of plaintext; key bytes exist
  only in the task runtime and are refetched after engine reopen. Because that JSON repeats the
  active key URL on each Segment, parser and recovery validation charge every repeated copy against
  the 8 MiB aggregate URL budget rather than counting only distinct key tags.
- A per-task key-slot cache serializes only callers sharing the same slot, so concurrent Segments
  fetch one 16-byte key while different rotations may proceed independently. Key requests inherit
  the original bounded request context and the same pause/cancel and bandwidth seams as media bytes.
- Ciphertext is written to a separate task-owned staging file. RustCrypto AES/CBC decrypts it in
  fixed 16-byte blocks to the normal Segment staging file, checks PKCS#7 on the final block, flushes
  and syncs nonempty plaintext, then removes ciphertext before the existing atomic Segment checkpoint.
  Retry, pause and cancel clean both non-durable files; cleanup failure is terminal and correlated.
- Resume compatibility compares plaintext/AES method, key-slot layout and explicit IV while allowing
  current segment/key URLs to refresh. Completed plaintext Segment artifacts are never re-decrypted;
  abandoned ciphertext and plaintext `.part` files are not adopted as durable progress.
- Independent ciphertext vectors plus real loopback Engine tests cover Media Sequence Number 7794,
  an explicit IV, key rotation, same-slot concurrent caching, exact merged plaintext, pause/reopen,
  key-fetch/local-decrypt/retry pause boundaries, all-padding empty plaintext, and invalid key
  length/padding without final publication or retained `.part` files.

### Stage 5 DASH MPD, track-selection and transfer findings

Reverified on 2026-08-11 before starting the DASH vertical slice:

- AB Download Manager was checked first. Its current source tree has no MPD/DASH downloader or
  parser entry, so it provides no compatible DASH behavior to migrate. Its existing HLS boundary
  still supports the general rule that protocol manifests are recognized during probing and are
  not allowed to fall through to the ordinary HTTP artifact path.
- Old NxTools owns the product behavior in
  `Services/DashManifestParser.cs` and
  `Services/DownloadManagerService.cs::{BuildDownloadProbeInfoAsync,DownloadDashAsync,
  DownloadDashInitsAsync,DownloadDashSegmentsWithBoundedWorkersAsync,MergeDashAndVerifyAsync}`.
  It resolves MPD/Period/AdaptationSet/Representation `BaseURL` values, inherits
  `SegmentTemplate`, expands `$RepresentationID$`, `$Bandwidth$`, `$Number$` and `$Time$`, chooses
  the strongest video and audio representations, downloads initialization/media fragments with
  bounded workers, concatenates each track in manifest order and uses FFmpeg stream copy when both
  audio and video are present.
- The old implementation is not a safe parser contract: XML and expanded Segment counts are not
  bounded, a duration template without a timeline is hard-coded to one Segment, negative timeline
  repetition is coerced to zero, only the first Period is used implicitly, and production logs
  expose MPD/Segment URLs and local paths. NxTools.Next must not preserve those failures.
- FluxDown `native/engine/src/dash_downloader.rs` was inspected only as a clean-room behavior and
  failure-boundary reference. Useful independent findings are hierarchical BaseURL resolution,
  representation/adaptation template inheritance, duration-derived Segment counts, initialization
  fragments, separate audio/video track artifacts and FFmpeg mux. Its source, schema, comments and
  tests are not copied, and silent first-Period fallback or URL-bearing diagnostics are not adopted.
- Old NxTools `Services/VideoCompressionService.cs::ResolveFfmpegExecutable`,
  `Services/FfmpegInstallerService.cs`, `MainWindowVideoViewModel::InstallFfmpegAsync` and
  `Views/VideoSettingsWindow.axaml` were rechecked for the user-managed FFmpeg boundary. The old
  product accepts a configured executable, probes `PATH`, also checks application-local/install
  locations, and exposes an explicit “自动下载” action. That action tries Gyan then BtbN-derived
  sources, streams a ZIP with progress and cancellation, extracts to a unique temporary directory,
  finds `ffmpeg.exe`, copies its binary directory under the current user's local tools directory,
  writes the installed path back into settings, and cleans its temporary directory.
- AB Download Manager was checked first for this FFmpeg-management slice, following the download
  reference priority. A repository-wide scan of its Kotlin/Java/XML/Gradle sources found no
  `ffmpeg`/`ffprobe` executable resolver, managed installer, external-tool download flow or related
  settings entry. It therefore supplies no FFmpeg auto-install behavior to reproduce; old NxTools
  is the product-behavior reference for this specific boundary.
- The corrected NxTools.Next product decision is: FFmpeg is not bundled into the release package,
  but a user may explicitly request an in-product download/install. A valid configured executable
  still wins, followed by `FFMPEG_PATH` and process `PATH`; a successful managed install becomes the
  new configured path. GPUI retains the public Gyan.dev and BtbN page links as manual fallbacks.
  The managed installer uses only the publishers' HTTPS artifacts and checksum manifests:
  Gyan's release-essentials ZIP/sidecar and BtbN's floating `win64-gpl` ZIP plus
  `checksums.sha256`. These are separately installed third-party FFmpeg builds under their own
  licenses after an explicit user action; NxTools neither mirrors them nor includes them in its
  package. Proxy mirrors from the old implementation are intentionally not retained.
  NxTools.Next must not copy the old installer's unsafe edges: source URLs/paths in logs, proxy
  mirrors, unbounded archives/entry expansion, path traversal, in-place overwrite, swallowed cleanup
  failures, or accepting an executable without a bounded `-version` validation.
  Diagnostics record only the fixed resolution source (`configured`, `environment`, `path` or
  `missing`) and elapsed time, never the executable path or child output. Settings labels an
  existing path only as a discovered candidate; the bounded `-version` process remains the
  authoritative validation before muxing, and a missing runtime result points back to Media Tools.
- The first NxTools.Next DASH slice accepts only bounded static VOD MPDs with exactly one Period,
  no `ContentProtection`, and HTTP(S) resources described by inherited `SegmentTemplate` plus
  either a finite `SegmentTimeline` or positive duration with a finite Period/MPD duration.
  `SegmentList`, `SegmentBase`, dynamic MPDs, multi-Period presentations, negative timeline
  repetition and unsupported template tokens fail with typed errors rather than producing partial
  media. The parser caps the XML body, track count, Segment count, individual fields/URLs and total
  expanded metadata before the probe can cross IPC.
- The first publication path is deliberately MP4-only (`video/mp4` / `audio/mp4`). WebM and other
  container families are rejected until their final container and mux policy are designed; they
  must never be downloaded and merely renamed `.mp4`. Duration templates derive both `$Number$`
  and `$Time$`; unsupported presentation-time offsets are rejected rather than ignored.
  Presentation duration math uses bounded integer nanoseconds rather than rounded floating-point
  milliseconds, so fractional durations cannot create a spurious final Segment.
- Probe exposes bounded video/audio presentation tracks. GPUI chooses one video and optional audio
  track explicitly and persists only the manifest URL plus stable representation identities in the
  typed request. The engine re-fetches and revalidates the MPD before transfer/recovery; compatible
  refreshed signed URLs may replace stored URLs, but representation, initialization and Segment
  timeline identity, including raw timescale/duration rather than rounded milliseconds, must remain
  compatible before a completed fragment can be reused.
- Rust owns MPD parsing, selection, durable Segment/checkpoint state, bounded transfer and final
  track layout. GPUI only renders and chooses tracks. FFmpeg is a separately bounded child-process
  boundary used for audio/video stream-copy mux; no manifest, request Header, Cookie, Referrer,
  resource URL, local path or child-process output payload may enter production diagnostics.
- DASH fragment workers reuse the proven HLS stop-aware HTTP seam but have their own durable
  fragment identity and checkpoint orchestration. The task connection limit bounds all init/media
  workers, retries clear only non-durable staging, and progress aggregates durable plus active
  bytes. Recovery compares codec, MIME, bandwidth, resolution, language and timeline identity.
  Single-track concatenation and FFmpeg version/mux are pause/cancel aware; dual-track publication
  gives FFmpeg an explicit MP4 muxer (`-f mp4`) because the temporary output suffix is deliberately
  not a media extension.

Reverified on 2026-08-11 before the combined real-media acceptance gate:

- Apple's official HLS Streaming Examples page remains the source for the Bip Bop sample. The
  HTTPS `bipbop_4x3_variant.m3u8` master currently exposes five relative variants; the lower
  `gear1/prog_index.m3u8` media playlist is finite (`EXT-X-ENDLIST`), plaintext MPEG-TS and contains
  no `EXT-X-MAP`, `EXT-X-BYTERANGE` or `EXT-X-KEY`. The remote media is not copied into this
  repository and availability remains an external acceptance dependency.
- DASH-IF's dash.js repository documents the Akamai-hosted reference streams, and the CTA WAVE
  `avc_sets/15_30_60/t1/stream.mpd` fixture was checked directly. It is a 60-second static MPD with
  one Period, finite SegmentTimeline templates, one AVC `video/mp4` track and one AAC `audio/mp4`
  track, without ContentProtection, SegmentList or SegmentBase. The manifest credits the remote
  Tears of Steel derivative under CC BY 3.0; NxTools only downloads it during an explicit runtime
  acceptance and does not redistribute the asset.
- `tests/runtime/download-media-smoke.ps1` keeps remote-source drift separate from product
  acceptance. Source-only mode caps body bytes, HLS segment count/duration and DASH per-track
  timeline expansion/duration, and never launches NxTools.
  Full mode requires exactly one selected package desktop or its byte-identical protected atomic
  desktop/indexer/PlatformHost copy. Every CLI invocation uses the production `--connect-only`
  policy, so a pipe race cannot start a hidden replacement. Remote bodies and native CLI/ffprobe
  children have end-to-end deadlines, bounded capture and whole-process-tree termination. Before
  submit, the script reserves a unique source/directory/name identity; a lost add response is
  reconciled through the durable snapshot, and cooperative failure/interrupt cleanup runs in
  `finally`, requests cancellation for every reconciled task and boundedly verifies its terminal.
  Abrupt termination of the PowerShell host remains an external unknown boundary. Final artifacts
  must match the requested path, remain available for manual playback, and may be checked for
  audio/video plus the fixture-specific duration range with a user-supplied `ffprobe.exe`.

### Stage 6 product-wide proxy findings

Reverified on 2026-08-11 before the first Stage 6 vertical slice:

- AB Download Manager was inspected first at
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/proxy/{Proxy.kt,ProxyManager.kt}`,
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/storage/ProxyDatastoreStorage.kt`,
  `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/{proxy/Proxy.kt,OkHttpHttpDownloaderClient.kt}`,
  `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/utils/proxy/DesktopSystemProxySelectorProvider.kt`
  and `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/ui/configurable/comon/renderer/ProxyConfigurableRenderer.kt`.
  It persists one application proxy policy, exposes Direct/System/Manual/PAC modes, validates a
  nonempty manual host and a port in `1..65535`, applies a strategy for each requested URL, and
  injects desktop system-proxy discovery behind a provider seam. Manual HTTP authentication is
  attached by the HTTP client; system and PAC selection remain desktop-specific infrastructure.
- The useful invariants are one shared policy, explicit direct/system/manual selection, bounded
  validation, per-request strategy resolution and an injected system-proxy boundary. NxTools.Next
  does not adopt ABDM's process-global Java SOCKS authenticator, mutable live-policy lookup during
  an operation, PAC mode, or plaintext persisted credentials in the first slice.
- Old NxTools was then rechecked at `Models/AppState.cs`, `Services/AppStateStore.cs`,
  `ViewModels/MainWindowViewModel.cs` and `Services/DownloadManagerService.cs`. Its global settings
  persist Off/Auto/Custom plus protocol, host, port, username and password; a task snapshots proxy
  values into its runtime configuration and HTTP clients are cached by proxy plus certificate
  policy. This supplies the requested product compatibility, but its URL/Header/proxy diagnostics,
  plaintext secret persistence and silent legacy-TLS fallback are not retained.
- The NxTools.Next decision is one product-level `AppConfig.network` owned by Rust and rendered in
  a global GPUI settings surface, separate from download-only Header/Cookie/certificate controls.
  The independent `nxtools-network::NetworkPolicy` (not the download domain) validates
  configuration and returns an immutable operation snapshot, so clipboard and quick-launch can
  opt in without depending on `nxtools-download`. The desktop runtime owns one shared policy and
  injects it into current outbound consumers. A settings save persists canonical `AppConfig`
  before publishing the new live value; a rejected persistence write therefore cannot leak a
  tentative route into a concurrently starting operation. Setting changes affect only later
  operations or an explicit later resume and never mutate an active transfer. Only real outbound
  HTTP consumers opt in—local IPC, Native Messaging, the MFT indexer and Windows adapter traffic do
  not use the proxy merely because they belong to a product feature.
- The first slice supports Direct, Windows System and Manual HTTP proxy modes. PAC and authenticated
  proxy storage remain unavailable until their Windows discovery/credential-protection boundaries
  are designed; the UI must not fake either capability. Manual proxy configuration is host plus
  `1..65535` port, never logs address/path/credentials, and fails validation before an operation is
  accepted. Diagnostics carry only correlation ID, fixed mode/source, bounded failure kind, native
  or HTTP code where owned, terminal result and elapsed time.
- Reqwest 0.13.4's checked-in crate documentation and `system-proxy` feature contract explicitly
  cover Windows and macOS system proxy settings; the workspace already enables that feature.
  `System` therefore keeps the builder's automatic system resolver, while Direct calls
  `ClientBuilder::no_proxy()` and Manual disables automatic resolution before installing its one
  explicit HTTP proxy. If later Windows acceptance finds a WinINET/WinHTTP mismatch, that becomes a
  typed C# adapter follow-up rather than a silent fallback to Direct.

### Stage 6 download request-context and certificate-policy findings

Reverified on 2026-08-12 before the second Stage 6 vertical slice:

- AB Download Manager was inspected first at
  `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/{DownloadSettings.kt,connection/OkHttpHttpDownloaderClient.kt}`,
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/AppSSLFactoryProvider.kt`,
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/perhostsettings/PerHostSettingsItem.kt`
  and `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/downloaderinui/http/applyHostSettingsToExtraConfig.kt`.
  ABDM keeps custom request values on the download credentials, applies them after client defaults,
  and deliberately refuses a caller-supplied `Host` because redirects must derive their own host.
  Its certificate switch wraps the platform trust manager and is strict unless the user explicitly
  disables checking. ABDM has per-host username/password/User-Agent settings, but no equivalent
  application-wide free-form Header/Cookie editor in the inspected path; that negative finding is
  why NxTools.Next uses the old product only for this compatibility surface.
- Old NxTools was then inspected at `Services/RequestHeaderParser.cs`,
  `ViewModels/AddDownloadDialogViewModel.cs`, `Models/AppState.cs`,
  `Services/DownloadManagerService.cs` and `Views/SettingsWindow.axaml`. It persists default Header
  text, default Cookie text and an ignore-certificate-errors flag; a new-download dialog copies the
  two request defaults into the task, and every probe/playlist/segment request reuses that durable
  task context. Its parser accepts ordinary `Name: value` and curl `-H` lines, lets the dedicated
  Cookie field replace a Cookie embedded in Header text, and excludes transport-owned
  `Accept-Encoding`, `Connection`, `Content-Length`, `Host`, `Range` and `Transfer-Encoding`.
  Certificate policy participates in its HTTP-client cache key, so a task uses one selected policy
  rather than mutating a live handler mid-transfer.
- NxTools.Next retains those useful invariants with stricter boundaries: strict certificate
  validation is the default; accepting invalid certificates is an explicit download-only setting;
  configured Header/Cookie defaults are bounded and validated before persistence; transport-owned
  headers cannot be configured; and explicit task/browser request context outranks defaults. The
  effective context is frozen into a task before its first probe or transfer, persisted with that
  task, and therefore survives redirects, pause/resume and process restart without reading mutable
  settings during an active attempt. Redirects keep safe request context while the HTTP stack owns
  `Host`, `Content-Length`, `Range`, connection framing and content coding.
- Diagnostics may carry only fixed certificate-policy and outcome/failure fields. They never record
  Header names or values, Cookie, Referrer, URL, file name or path. Old NxTools' diagnostic strings
  that include URL, response Header and proxy endpoint are explicit negative examples and are not
  migrated.
- The self-signed TLS regression uses test-only `rcgen 0.14.9` and `tokio-rustls 0.26.4`; both
  declare `MIT OR Apache-2.0`. They generate and serve a bounded ephemeral certificate only during
  tests and add no certificate, private key or TLS binary to the packaged product.

### Stage 6 classification, automatic-directory and target-conflict findings

Reverified on 2026-08-12 before the third Stage 6 vertical slice:

- AB Download Manager was inspected first at
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/{Category.kt,CategoryManager.kt,DefaultCategories.kt}`,
  `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/adddownload/single/BaseAddSingleDownloadComponent.kt`,
  `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/{DownloadManager.kt,utils/FileNameUtil.kt,utils/OnDuplicateStrategy.kt}`
  and `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/NewDownloadChecker.kt`. Categories
  are persisted, reorderable rules containing optional file extensions, URL wildcards and an
  optional save directory. A category with both rule kinds must accept both; URL-constrained and
  then more-specific rules sort ahead of broad extension-only rules. Default categories cover
  archives, programs, video, music, pictures and documents under the default download directory.
  ABDM serializes duplicate admission with a mutex, considers both disk files and destinations
  reserved by existing tasks, and defaults to a numbered name before the extension.
- Old NxTools was then inspected at `Models/{AppState.cs,DuplicateDownloadStrategy.cs}`,
  `ViewModels/{DownloadCategoryViewModel.cs,AddDownloadDialogViewModel.cs,MainWindowViewModel.cs}`
  and `Services/FileTypeCategoryResolver.cs`. It adds a fallback category, supports user categories,
  extension and URL wildcard rules, optional category directories, explicit dialog category
  selection and numbered names of the form `name (1).ext`. Its useful compatibility behavior is
  that a manual category wins over automatic classification and active task destinations count as
  occupied. Its `OverrideExistingFile` path deletes the old final file before the replacement is
  durable; that crash window is an explicit negative reference and is not migrated.
- NxTools.Next keeps category identity and the resolved category name on the accepted task, but
  evaluates mutable rules only once at the central task-acceptance boundary. Explicit category or
  directory choices win; otherwise automatic classification selects the most specific enabled rule
  and creates its directory before persisting the task. Rules and strings are bounded. Changing
  settings affects only later accepted tasks; restart and pause/resume use the persisted resolved
  destination.
- An ordinary request that intentionally omits its name keeps that absence until the bounded HTTP
  probe resolves the authoritative `Content-Disposition`/final-URL name. Under the same serialized
  admission gate, the engine then classifies that real name, resolves disk/nonterminal-task
  conflicts and persists the final identity before transfer starts. It never lets a weak URL
  basename or `download.bin` placeholder suppress a server-provided name.
- Without the user-removed SHA/hash feature, “duplicate” means only that the case-insensitive target
  path is occupied on disk or reserved by a nonterminal task; it never claims content equality. The
  safe strategies in this slice are `add_numbered` (default) and `reject`. Admission, disk checks,
  directory creation, final name selection and task persistence share one engine serialization
  boundary, so concurrent ordinary, browser, CLI and batch submissions cannot reserve the same
  artifact. No strategy deletes or replaces an existing final artifact or an active task artifact.
- Diagnostics carry the acceptance correlation ID, fixed classification/conflict fields, bounded
  category ID and elapsed time. They do not record URL, file name, category directory or any other
  path. Organization resolution happens before task identity is created, so these events do not
  invent or preallocate a task ID.

Reverified on 2026-08-12 after the first Release classification-layout inspection:

- Old NxTools `Views/AddDownloadWindow.axaml:303-338` gives category selection a normal full-width
  form row instead of a narrow auxiliary button; `AddDownloadDialogViewModel.ApplySelectedCategoryDirectory`
  resolves the chosen category's directory. NxTools.Next keeps its safer central acceptance rule,
  but the visible category control must still read as a peer of the file-name field and expose the
  category directory in its menu without clipping.
- The persisted `DownloadCategorySettings.save_directory` was already wired into the Next settings
  draft, but the first GPUI editor rendered it as an unlabeled 180 px input beside the category
  name. A per-category directory is therefore not discoverable even though the domain supports it.
  The settings UI must give every selected category an explicit labeled save-directory row, explain
  that an empty value uses the default directory and that relative values resolve below it, and use
  the available page width for name, directory, extension and URL-rule editing.
- Category navigation is an editor list, not a tag cloud: each item must have a readable hit target
  and show category name plus directory state. Enabled state, ordering and destructive actions stay
  in the selected-category editor so list selection does not become a row of tiny adjacent controls.
- The add-download category popover is bounded by the compact native confirmation window rather
  than only by a nominal popup maximum. It exposes name and directory as two lines, preserves the
  complete directory in a tooltip, and pages the bounded 32-category domain so the last item remains
  inside the real client hit-test area.

Reverified on 2026-08-12 after comparing the packaged download surfaces with the accepted clipboard
workspace and Zed's control scale:

- The current clipboard workspace is the in-product visual baseline: its ordinary actions use a
  consistent 26 px control height, 10.5 px labels, a subtle border and a raised surface. Controls
  remain compact, but they do not collapse into unframed 9 px text. Download settings had mixed
  `xsmall`, `small`, ghost and icon-only defaults in the same row, so equally important operations
  appeared to have unrelated hit targets.
- Zed `crates/ui/src/components/button/button_like.rs::ButtonSize::rems` defines an explicit scale
  (18 compact, 22 default, 28 medium and 32 large) instead of allowing every caller to invent a
  height. NxTools.Next uses the clipboard-compatible 28 px action tier for download forms and keeps
  30–32 px inputs; compactness comes from predictable rows and spacing, not from illegible labels.
- A bounded category menu must also be information-dense. Rendering only two 44 px categories per
  page creates 16 pages for the supported 32-category limit. The confirmation window can fit four
  34–36 px two-line rows plus one compact pager; the full directory remains available through the
  existing tooltip, and the last category must still pass the native hit-test regression.
- New-download and settings footers keep one stable bar, but every visible action inside the bar
  shares the same 28 px rhythm. Icon-only pick/refresh actions are square; text actions have explicit
  horizontal padding and visible secondary/primary hierarchy.

Reverified on 2026-08-13 for the P0 Popover/ListBox migration:

- The earlier four-item pager was a bounded-window safety fix, not an old-product behavioral
  requirement. With the current 560 px confirmation window it forces seven page transitions at the
  supported 32-category limit and conflicts with the approved `DL-21` remediation.
- Zed `crates/ui/src/components/popover.rs` keeps the elevated surface attached to its trigger, while
  `crates/ui/src/components/list/list.rs` and `list_item.rs` use one continuous full-width item stack
  with neutral selection. NxTools.Next therefore uses one bounded, scrollable ListBox rather than
  page controls.
- The ListBox owns one visible search field and one highlighted choice. Typing filters bounded
  category name/directory/extension metadata; Up/Down moves the highlight, Enter selects, and
  Escape closes through the existing Popover lifecycle. Mouse selection and keyboard confirmation
  update the same local `selected_category_id` before the controlled surface closes; close returns
  keyboard focus to the adjacent file-name field rather than leaving focus on a hidden search box.
- The committed category and the single keyboard highlight use distinct neutral surfaces. Every
  close path, including overlay dismissal and trigger toggle, returns focus only after the local
  selection/controlled-open transition has completed.
- The automatic-classification choice remains first only when the search is empty. Every category
  continues to show name plus directory state, with the full bounded directory in a tooltip; no
  query, directory or category text enters diagnostics.

Reverified on 2026-08-13 for the P0 task-context-menu migration:

- ABDM first: its download list treats the secondary gesture as a selection-aware operation; an
  unselected row becomes the target, while an existing multi-selection remains the batch target.
  Old NxTools next: `MainWindow.axaml.cs::TaskRow_ContextRequested` applies the same selection rule
  before `BuildTaskContextMenu`, then groups artifact, queue, transfer and removal actions with
  separators. NxTools.Next retains the safer capability plan: unavailable actions are omitted and
  queue-edge moves are calculated from the authoritative order.
- The menu is one bounded 176–240 px elevated surface. Each selectable action uses the shared 28 px
  action rhythm, a fixed 14 px icon slot and 12 px body text; only file deletion and list removal
  use the danger tone. Separators describe capability groups rather than decorating every row.
- The pinned `gpui-component` PopupMenu remains responsible for Up/Down/Enter/Escape, outside-click
  dismissal and focus return. NxTools.Next supplies only presentation and domain callbacks; every
  callback defers cross-entity DownloadApplication work to the next application turn.
- The production task-row regression opens the real menu and verifies its geometry, capability
  omissions and Escape lifecycle. Diagnostics continue to record only bounded selected count,
  fixed action kind and correlation ID—never URL, task name, artifact path, Header or Cookie.

Reverified on 2026-08-14 for the P0 detail-dock and multi-selection-toolbar migration:

- ABDM first: `HomePage.kt` keeps `selectionList` as the stable list-level selection authority and
  passes it into `DownloadList`; header actions remain attached to the continuous download surface
  instead of becoming controls inside every row. Old NxTools next: `MainWindow.axaml` exposes a
  selected-task detail inspector and `MainWindowViewModel` applies pause/resume/cancel to
  `GetSelectedTasksOrCurrent()`. NxTools.Next keeps its safer capability-derived target subsets and
  queue-edge rules rather than copying the old first-selected-only queue behavior.
- The selected-task inspector stays docked below the virtual table. It is absent when selection is
  empty, uses one fixed-height tab strip and one persistent scroll viewport, and presents ordinary
  fields with stable label/value columns rather than miniature metric cards or arbitrary font
  sizes. Long URL/path/error content remains inside the inspector scroll boundary and never moves
  the task-table columns.
- Multi-selection adds one transient toolbar between the filter strip and table header only when at
  least two visible tasks are selected. It reuses the exact context capability calculation:
  lifecycle actions target only capable members, queue moves appear only when the entire selection
  is movable at that edge, and removal appears only when every selected task is removable. Single
  selection and empty selection allocate no toolbar height.
- Toolbar actions keep the stable selected-ID set and defer every cross-entity
  `DownloadApplication` call to the next application turn. Clearing the selection is local view
  state. Diagnostics remain owned by the existing typed batch/queue commands and never include
  file names, paths, URLs, Header or Cookie values.

Reverified on 2026-08-13 for the final Stage 6 completion-action slice:

- ABDM first: `shared/app/.../ondownloadcompletion/OnDownloadCompletionActionRunner.kt` observes
  completed jobs and delegates actions through `OnDownloadCompletionActionProvider` instead of
  embedding OS work in the downloader. Desktop power actions are selected in
  `desktop/app/.../actions/onevennts/getOnDownloadCompletionAction.kt`; the alert in
  `pages/poweractionalert/PowerActionComponent.kt` owns a 30-second transient countdown with
  **Shut down now** and **Cancel**, and calls the platform power adapter only after the countdown.
  `PowerActionWindows.kt` ultimately delegates shutdown to Windows. Its useful invariant is the
  cancellable, non-durable intent; its direct command execution and unbounded repeated per-job
  callbacks are not copied.
- ABDM notification sound is a separate completion presentation concern
  (`util/notification/BaseNotificationSound*.kt`). Desktop falls back to the system beep when no
  custom source exists. NxTools.Next intentionally narrows this: it never imports or executes an
  arbitrary audio path and asks PlatformHost to play the current Windows system notification sound.
- Old NxTools next: `MainWindowViewModel.HandleDownloadCompleted` shows the completion surface,
  optionally opens the destination directory and evaluates shutdown only after the active queue has
  drained. `TryShutdownWhenAllRequestedDownloadsComplete` arms shutdown per task, waits while any
  created/probing/scheduled/queued/downloading/merging/paused/verifying task remains, then launches
  `shutdown.exe` immediately. `DownloadProgressWindow.axaml` exposes the task-local arm switch.
  Useful compatibility behavior is that an armed task must complete successfully and the whole
  queue must be idle; the unsafe immediate, non-cancellable process launch is a negative reference.
- NxTools.Next keeps persistent presentation preferences only for completion sound and automatic
  directory reveal. A task-local **完成后关机** arm is process memory only: it is never stored in
  config, SQLite or a download command, and therefore cannot resume after restart. When an armed
  task completes and no nonterminal download remains, Rust opens one always-on-top 30-second GPUI
  countdown. Cancel/close/new work disarms it; **立即关机** or expiry sends one typed Windows command.
  C# owns the Windows sound and shutdown adapter but never decides when shutdown is allowed.
- Completion feedback is burst-coalesced. One bounded batch produces one in-app notification, at
  most one Windows notification sound, and reveals at most three distinct destination directories.
  Additional directories are reported only as a count. Initial terminal tasks restored at startup
  are marked seen, so restart emits neither stale feedback nor a stale power action.
- Diagnostics use one new correlation ID per coalesced feedback batch or shutdown countdown. They
  carry only bounded task/result/directory counts, fixed action/failure kinds, elapsed time and the
  native error code. They never record URLs, file names, paths, Header/Cookie values or custom sound
  data.

Reverified on2026-11-11 for B5 completion intentions:

- ABDM `AppComponent.kt:1197–1234` owns one transient SlotNavigation/childSlot,
  serializer=null, and stops download work before the power adapter. The interface is in
  that same file at1300–1308. `PowerActionWindows.kt` implements Shutdown only;
  Sleep/Hibernate are TODO and are not evidence of Windows support.
- Legacy `DownloadProgressWindow.axaml.cs:26–103` binds/unsubscribes presentation and
  transforms the same window on completion; `WindowAnimationService.cs:13–49` wraps
  actual Close. Closing presentation is not task cancellation or intention retirement.
- Current original owners: `rust/crates/nxtools-download/src/lib.rs::DownloadTask` and
  `DownloadQueue`; `storage.rs::StoredResolvedRequest/save_task_row/load_queues`;
  `engine.rs` acceptance under scheduler→tasks→store and `commit_completed_state`
  persist before TaskChanged. `engine/queue.rs::schedule_queue_locked` persists before
  spawning the original worker. Queue membership remains download_queue_items.
- `rust/apps/nxtools-desktop/src/downloads.rs::new` observes existing terminal records;
  original event pump/refresh reaches `completion_shutdown_impl.rs`. The sole
  `completion_actions.rs::CompletionActionCoordinator` owns400ms feedback, max3 distinct
  directories and one30s countdown; task_windows_impl.rs owns keyed close/reopen.
- Native entry is original PlatformBridge→PlatformPipeClient/parser→WindowsPlatformAdapter
  →ICompletionSystemActions. `CompletionSystemActions.cs::RequestShutdown/RequestSuspend`
  own the actual OS requests. Shutdown keeps its bounded ACK deadline; Sleep/Hibernate
  retain the original pending action slot until the real correlated terminal. Disconnect
  retires executing presentation with an unconfirmed outcome, not native rejection.
- Normal Exit uses cx.quit→Application.run return→DesktopRuntime::shutdown_downloads_and_wait
  →original engine JoinSet retirement→Download IPC stop/client/host joins→extension/clipboard
  drains→runtime/supervisor release. A close request is not resource drain or physical exit.
- B5 keeps durable intention separate from current successful round and ephemeral one-shot
  authority. Old terminal reload cannot trigger; cancellation/removal cannot turn an
  unsuccessful queue into success. Source-backed design and acceptance boundaries are in
  `.trellis/tasks/11-11-reference-b5-completion-intentions/design.md`.
- `engine.rs::completion_snapshot` carries session-local queue invalidation cutoffs and
  accepted task revisions from the original scheduler gate. `remove_correlated` and
  `engine/queue.rs::AssignTasks` retain destructive source-membership facts only after the
  original SQLite commit; `storage.rs::queue_id_for_task` reads download_queue_items.
  `completion_actions.rs::QueueRound` aborts the old round across coalesced observation,
  while genuinely later acceptance can establish a new round. Dispatch/pause/resume do
  not revive the aborted acceptance.
- `engine.rs::set_task_completion_intention` rejects Completed/Failed/Cancelled before
  persistence. `resume_eligible_correlated` emits the original TaskChanged after durable
  retry admission even with a stopped queue; completion_snapshot projects eligible Failed
  retry as Waiting without changing raw scheduler state, checkpoint or artifact. Original
  daily schedule start still owns worker admission. Real HTTP/writer/SQLite regressions
  in `downloads/settings_task_rem_tests.rs::b5_repair_01_*`, `b5_repair_02_*` and
  `b5_repair_03_*` hold the production event pump across mutations and retain once-only
  future success plus stale-callback refusal.

Reverified on 2026-08-20 for the DL-09/DL-10 download-settings shell migration:

- ABDM first: `DesktopSettingsComponent.kt` keeps navigation and the selected settings page inside
  one stable desktop settings surface. Old NxTools `Views/SettingsWindow.axaml` likewise keeps one
  modal settings lifetime; neither reference rebuilds the window or commits the draft when the user
  changes a settings category.
- NxTools.Next therefore reuses the shared `SettingsShell`: one titlebar, stable left navigation,
  one scrollable content viewport and one fixed 48 px footer. Five-page navigation changes only the
  locally rendered page and scroll offset; it does not mutate the draft, start native work, resize
  the window or move Apply/Cancel.
- The shared visual contract is Body 12/18 and Label 11/16 for Chinese settings copy, 32 px text
  inputs and 28 px ordinary actions. Directory and FFmpeg pickers are square icon actions with
  tooltips; category, FFmpeg, taskbar-preview and footer actions use the same action tier instead of
  mixing `xsmall`, `small`, `compact` and caller-defined heights.
- Existing ownership remains authoritative: the GPUI view owns the draft, Cancel/native close
  discard it, and Apply keeps the existing validation, persistence, cross-entity defer and correlated
  failure lifecycle. The migration adds no download command, protocol field or diagnostic payload.

Reverified on 2026-08-20 before the DL-11/DL-12 category master-detail migration:

- ABDM category ownership remains in the previously indexed `Category`, `CategoryManager` and
  `DefaultCategories` path: order, enabled state, matching rules and optional save directory are one
  persisted category identity. Presentation must not split those values into a second state owner.
- Old NxTools `Views/CategoryWindow.axaml` and `ViewModels/CategoryDialogViewModel.cs` were followed
  through the editable name, save directory, extension and URL-rule fields to `BuildState()`. The
  useful compatibility invariant is one coherent form with aligned labels and one save result; its
  separate dialog and 34 px local control dialect are not migrated into the shared settings shell.
- NxTools.Next keeps the existing local settings draft and Apply/Cancel boundary. The left side is a
  continuous master list with Body category name, Label directory state, neutral hover/selection and
  a non-shifting 2 px selected edge. Disabled categories remain readable but visually demoted.
- The selected category editor uses the shared 32 px `Field` shell for name, save directory,
  extensions and URL rules. Name/directory and extension/URL form two equal-width rows with aligned
  label baselines; switching the master selection only changes which existing draft entity is shown.
- Default-directory, category-name and category-directory input changes explicitly invalidate the
  owning settings view, so the master identity and resolved destination cannot lag behind the
  selected editor. Relative destinations resolve against the current draft default directory;
  truncated rows expose the complete resolved path through one stable production Tooltip boundary.

Reverified on 2026-08-20 before the DL-13 category-action hierarchy migration:

- ABDM first: `shared/app/.../pages/home/CategoryActions.kt` keeps category deletion as a distinct
  destructive action and routes it through a delete request; `BaseHomeComponent.reorderCategory`
  expresses ordering as a bounded index delta, while desktop `Categories.kt` shows reorder affordance
  only in the category interaction zone. The useful invariant is separation between state, ordering
  and destructive intent; its drag-only Compose presentation is not copied into the settings form.
- Old NxTools `CategoryWindow.axaml` keeps enable/path state with the category fields and reserves the
  footer for Save/Cancel. It has no category-list ordering or deletion control, so it provides no
  basis for making every editor action equally prominent or permanently red.
- NxTools.Next keeps all four operations inside the existing local settings draft. Enable belongs in
  the selected-category heading, Up/Down form one neutral ordering group with edge-disabled states,
  and Delete is spatially separated. Delete is neutral at rest and gains danger semantics only on
  hover/press; only Apply persists the resulting enabled/order/removal changes.

Reverified on 2026-08-20 before the DL-14 request-context form migration:

- ABDM first: `CommonSettings.ignoreSSLCertificates` presents certificate trust as a dedicated
  boolean setting, while `PerHostSettingsItem` keeps bounded host-specific identity fields rather
  than an application-wide free-form request editor. The useful presentation invariant is that the
  security exception remains explicit and separate from ordinary request metadata; the missing
  global Header/Cookie surface remains a negative finding rather than a feature to invent.
- Old NxTools `Views/SettingsWindow.axaml::HeadersSection` gives Header and Cookie named multiline
  fields inside one request-metadata section, while its SSL exception is a separate setting row.
  `Services/RequestHeaderParser.cs` confirms that the dedicated Cookie value overrides a Cookie in
  Header text and that transport-owned fields stay excluded. Its equal two-column text areas are not
  copied because a long Header is the dominant editing task and a Cookie is normally one bounded
  line in NxTools.Next.
- NxTools.Next keeps one request-context form in the existing settings draft: Header owns a fixed,
  internally scrollable full-width editor; Cookie is a full-width 32 px field below it; certificate
  validation is a separate security section with strict validation as the quiet default. Long input
  never grows the page or moves the fixed settings footer, and navigation, Cancel and Apply retain
  the existing draft/persistence lifecycle. No Header or Cookie value enters diagnostics or UI test
  failure text.

Reverified on 2026-08-20 before the DL-15 transfer-settings matrix migration:

- ABDM first: `DesktopSettingsComponent.kt::DownloadEngine` keeps speed limit, connection/thread
  count, concurrent downloads and dynamic part download in one `ConfigurableGroup`. The settings
  component owns their presentation while the existing repository flows remain the value authority;
  it does not promote every transfer parameter into an independent card or commit on navigation.
- Old NxTools `Views/SettingsWindow.axaml::DownloadEngineSection` likewise keeps automatic scaling,
  its numeric bounds, global speed and thread/queue policy as continuous setting rows inside one
  download-engine surface. Its larger spacing, steppers and unrelated retry field are not migrated.
- NxTools.Next therefore presents the six existing transfer fields as three aligned two-cell rows in
  one bounded section: connection capacity beside queue concurrency, adaptive splitting beside its
  segment ceiling, and the minimum split tail beside the global speed ceiling. The matrix has one
  outer boundary and one quiet center seam; cells do not gain card surfaces, radii or shadows.
- This is presentation-only. The same local draft, validation ranges, toggle/input entities and
  Apply/Cancel persistence lifecycle remain authoritative. The matrix only subdivides the current
  settings content width and must not impose a child minimum width that stretches the parent.

Reverified on 2026-08-20 before the DL-16 FFmpeg settings-flow migration:

- ABDM was checked first through the previously recorded repository-wide negative finding: it still
  has no FFmpeg resolver, installer or settings entry, so it supplies no presentation or lifecycle
  contract for this slice. Old NxTools remains the product-behavior reference at
  `Views/VideoSettingsWindow.axaml`, `MainWindowVideoViewModel::{InstallFfmpegAsync,
  CancelFfmpegInstall}` and `Services/FfmpegInstallerService`.
- The useful old-product order is explicit: show the configured executable, let the user browse for
  one, expose one primary automatic-download action with visible progress/cancellation, and keep
  manual acquisition as the fallback. The old separate video window, card surfaces, large button
  pair, GPU refresh and unsafe installer edges are not migrated.
- NxTools.Next keeps one continuous Media Tools section with three full-width sequential rows. The
  executable field and picker lead; automatic install is the sole filled action; Gyan.dev and BtbN
  remain quiet public-source fallbacks. All three action groups begin on one left control rail so a
  short button is not isolated at the far right of a mostly empty settings row.
- Existing ownership is unchanged: the settings view owns the configured-path draft, the application
  owns the bounded install/cancel and shell-open operations, every cross-entity callback remains
  deferred, successful managed install keeps its existing immediate configuration transaction, and
  ordinary Apply/Cancel still governs manual path edits. Install state may replace copy or reveal
  Cancel without moving the fixed settings footer.

Reverified on 2026-08-20 before the DL-17 window/progress settings migration:

- ABDM first: `DesktopSettingsComponent.kt::Appearance` keeps automatic progress-window and
  completion-window preferences in one bounded configurable group, while
  `ShowDownloadDialogs.kt::UpdateTaskBar` maps task state to the native taskbar without mixing those
  policies into transfer-engine controls. It has no equivalent of the NxTools.Next aggregate
  taskbar overlay, so it supplies grouping—not invented overlay behavior.
- Old NxTools `Views/SettingsWindow.axaml` likewise renders progress/completion and post-completion
  behavior as continuous title/description/control rows. It has no taskbar-internal aggregate
  overlay; its larger spacing and unrelated completed-item actions are not migrated.
- NxTools.Next therefore keeps two stable, continuous matrices on the existing Window page. The
  task-window section pairs visibility and completion feedback in two rows. The taskbar-progress
  section pairs enable/edit, anchor/mode, X/Y, width/height and three information rows. Both use one
  outer boundary, quiet center seams, equal flexible cells and aligned trailing control rails.
- This remains presentation-only. Toggle, numeric and segmented controls retain the same draft
  entities and validation. Entering, updating, applying, cancelling or closing taskbar layout edit
  mode keeps the existing deferred GPUI and typed PlatformHost lifecycle.

Reverified on 2026-08-20 before the DL-18 diagnostics-path migration:

- ABDM first: `DesktopSettingsComponent.kt` has Appearance, Download Engine and Browser Integration
  settings but no diagnostics-path page or path-copy controls, so it supplies no UI to imitate.
  `DefinedPaths.kt` and `AppInfo.kt` do supply the useful ownership invariant: config, data, system,
  log and download-database locations are centralized runtime facts rather than editable settings.
  `WindowsFileUtils.kt` routes folder open/reveal through the platform adapter after validating the
  target; its direct presentation and fallback details are not copied into GPUI.
- Old NxTools next: `Views/SettingsWindow.axaml` keeps a **日志输出** row with an adjacent
  `OpenLogFolderCommand`, and `MainWindowViewModel.OpenLogFolder` opens the AppLogService-owned log
  directory. Its clipboard storage surface exposes the complete `ClipboardDatabasePathText` rather
  than hiding the database identity. The old direct `Process.Start` call and any diagnostics that
  print paths are negative references, not architecture to migrate.
- NxTools.Next keeps the three existing runtime-owned values from `AppPaths -> DesktopRuntime ->
  DownloadRuntimeContext -> DownloadApplication -> DownloadSettingsView`: config file, log directory
  and download database file. The diagnostics page presents them as three continuous full-width
  rows. Each path owns a stable text region plus adjacent 28 px Copy and Open/Reveal icon actions;
  truncation never creates a half-width column, and the complete bounded value remains available in
  a Tooltip and through Copy.
- GPUI continues to own rendering and defers every cross-entity callback. Copy reuses the typed
  copy-only `PasteText` platform command; directory open and file reveal reuse `OpenShellTarget`.
  C# remains the only Windows Shell/clipboard adapter. One bounded operation identity follows each
  request to a terminal success, failure or timeout, while diagnostics record only fixed path kind,
  action kind, elapsed time and bounded native/error metadata—never the path or clipboard payload.

Reverified on 2026-08-22 before the DL-19 new-download form-grid migration:

- ABDM was checked first at
  `desktop/app/.../pages/addDownload/single/AddDownloadPage.kt` and
  `shared/app/.../pages/adddownload/single/BaseAddSingleDownloadComponent.kt`. Its URL remains the
  full-width dominant field, while category, location and name are one continuous form hierarchy;
  category selection may update the location, but the component remains the single value owner.
- Old NxTools `Views/AddDownloadWindow.axaml` keeps the confirmed destination, category and file-name
  controls in one compact sequence and binds them to the same `AddDownloadDialogViewModel`. Its
  placeholder-only labels, 34 px local control dialect and separate checkbox are not migrated.
- NxTools.Next `NewDownloadView::render` still used a private `download_input_field` with 9.5 px
  labels and 30 px shells, while category rebuilt the same label/control shape independently. The
  existing production category test compared only the two outer containers, so it could pass when
  the visible labels or actual controls no longer shared a baseline.
- DL-19 keeps the two-stage probe, browser-prefilled confirmed height, directory picker, category
  Popover/focus return, submit modes and fixed footer unchanged. URL is one full-width shared Field;
  directory is one full-width shared labeled path row; file name and category form an equal-width
  two-column row. All four use Label 11/16 and 32 px control shells, with file-name/category labels
  and controls sharing their y coordinate and height in the production confirmed window.
- The red-capable gate opens the real browser-prefilled `DownloadApplication -> NewDownloadView`
  window and asserts the full-width URL/directory boundaries plus visible label and control bounds,
  not just wrapper equality. It continues through the real category Popover and Cancel lifecycle so
  the grid migration cannot replace the existing interaction owner.

Reverified on 2026-08-22 before the DL-20 probe-summary migration:

- ABDM `AddDownloadPage.kt::{RenderFileTypeAndSize,RenderResumeSupport}` keeps file type/size and
  resume capability in an intrinsic-width right rail beside the category/location/name column. It
  uses a 24 dp gap and does not introduce a second full-width metadata card below the URL.
- Old NxTools `Views/AddDownloadWindow.axaml` keeps its confirmed size/resume text at the trailing
  edge of the resolved URL row. Its read-only URL and separate resource panels are legacy workflow
  choices, not a reason to duplicate probe metadata in NxTools.Next.
- Before DL-20, NxTools.Next already placed three probe strings in a 118 x 82 px bordered-left
  fragment, but it used private 9/9.5/11 px typography, did not bound long MIME/status text, and had
  no production boundary proving that the fragment stayed a narrow inspector or disappeared before
  confirmation.
- DL-20 keeps the compact two-stage geometry: the initial window is 560 x 176 and confirmation adds
  only the destination-form height, producing 560 x 278 without changing width. The initial hint is
  a shared Label field-tail status; the confirmed summary is a clipped 120 x 112 px right inspector
  spanning the destination rows, with shared Label, Body and Meta typography. HLS/DASH row-driven
  expansion remains owned by `new_download_window_size_for_probe`.
- The red-capable production gate opens the real compact `DownloadApplication -> NewDownloadView`
  window, verifies that no inspector space exists before confirmation, injects a bounded probe at
  the view seam, resizes through the same confirmed size helper, and asserts the right rail plus all
  three visible rows remain inside the form and above the fixed footer.

Reverified on 2026-08-22 before the DL-22 category-information migration:

- ABDM `desktop/app/.../addDownload/shared/CategorySelect.kt::RenderCategory` shows a compact icon
  plus category name in its bounded dropdown. Old NxTools `AddDownloadWindow.axaml` likewise shows
  only the category name. NxTools.Next keeps its intentional product extension: the destination is
  necessary secondary information when several categories have similar names.
- The existing NxTools.Next row already contained name and directory strings and attached a generic
  directory tooltip, but the name used a private 16 px line height and neither the name nor tooltip
  exposed a stable production boundary. A regression could collapse the row or silently remove the
  only complete-path surface while the earlier directory-height assertion remained green.
- DL-22 keeps each option as one dense two-line row: category name uses shared Body 12/18 and the
  destination uses Label 11/16. Both lines own the available row width and truncate only inside the
  ListBox; hovering the row opens a stable, wrapped, monospace tooltip containing the complete
  destination. Selection, highlight, search, scroll and focus-return ownership remain unchanged.
- The production gate opens the real 32-category Popover, asserts both visible lines are ordered and
  contained by the same option, then advances the tooltip clock after hovering the long-path row and
  requires the complete-directory tooltip boundary before continuing through the existing overlay,
  keyboard-selection and native Cancel lifecycle.

Reverified on 2026-08-22 before the DL-23 HLS/DASH track-list migration:

- AB Download Manager's HLS boundary remains
  `shared/app/.../hls/HLSLinkChecker.kt -> HLSDownloadJobExtraConfig ->
  downloader/core/.../hls/HLSResponseInfo.kt`; it separates HLS from ordinary HTTP but still does
  not expose a complete master-quality chooser. Its reusable queue selection surface uses native
  radio controls, confirming the single-choice interaction grammar without supplying NxTools's
  product-specific quality metadata.
- Old NxTools's product chain remains
  `Services/HlsPlaylistParser.cs::{Parse,SelectBestVariant,ResolveUrl} ->
  DownloadManagerService.BuildHlsResourceOptionsAsync ->
  AddDownloadDialogViewModel::{ApplyHlsResources,HlsResource_PropertyChanged} ->
  Views/AddDownloadWindow.axaml`. It preserves resolution, frame rate, bitrate and codecs and rejects
  submission when a discovered media list has no selection. Its legacy multi-resource checkbox
  table is not copied into Next's already typed single-playlist request.
- Before DL-23, NxTools.Next's `NewDownloadView::render -> hls_quality_options/dash_track_options ->
  dash_track_option_row` rebuilt two card-like selectors with 8–10 px private typography. HLS placed
  frame rate in the primary label and bitrate in a separate trailing column; DASH used the same
  visual card language independently. Missing HLS resolution was already classified from codec, and
  that “视频流/音频流/未标注分辨率” behavior must survive so absence never becomes “自动”.
- DL-23 keeps probe, request and submission ownership unchanged. Both media surfaces use one shared
  42 px radio-list row: the primary line is the concrete resolution or track identity, while codec,
  bitrate and optional frame rate occupy the secondary line. The selected row uses a neutral surface
  plus a 14 px ring and 6 px accent dot; unselected rows have no dot, and ordinary rows do not become
  separate rounded cards. DASH video remains required single-select, while its existing optional
  audio track can return to no selection on a second click.
- `new_download_window_size_for_probe -> bounded_media_rows_height` remains the native size owner and
  now uses the shared 42 px row height with a three-row cap. The red-capable GPUI gates render HLS and
  DASH choices, assert row/radio/title/detail geometry and both radio states, then exercise selection,
  optional-audio cancellation and typed-media submission through the existing view boundary.

Reverified on 2026-08-22 before the DL-24 new-download footer migration:

- AB Download Manager `desktop/app/.../pages/addDownload/single/AddDownloadPage.kt` anchors Add,
  Download and Cancel at the bottom of the single-download window and keeps Download as the primary
  completion path. Old NxTools `Views/AddDownloadWindow.axaml` preserves the same confirmed ordering:
  `AddOnlyCommand`, primary `DownloadNowCommand`, then `CloseCommand`; its initial stage exposes only
  primary Continue and secondary Cancel. Neither reference promotes queue-only or cancel alongside
  the immediate download action.
- NxTools.Next's current chain is `NewDownloadView::render -> footer_leading_actions /
  footer_submission_actions -> NewDownloadView::submit`. The confirmed footer adds the product's
  existing delayed-start trigger and refresh utility without changing the two typed submission modes:
  QueueOnly closes after durable queue admission, while StartNow closes before scheduling the keyed
  task window. Cancel remains a native window close and schedule remains a Popover-owned draft.
- Before DL-24, schedule, queue-only, start-now and cancel each rebuilt `Button::small` with private
  28 px height, 10/11 px padding and 4 px radius; the production gate accepted 28–30 px and therefore
  could not enforce the shared action tier. The immediate Download button was visually primary, but
  no real-window boundary identified it as the sole primary action or kept the other three secondary.
- DL-24 must reuse the shared 28 px action constructors. Delayed download and queue-only are outlined
  secondary actions, Cancel is ghost secondary, and Download is the only accent-filled primary action.
  Refresh stays a square 28 px utility. The footer remains fixed, the left/right grouping and existing
  click owners do not move, opening the schedule Popover does not resize the window, and submit/close
  lifecycle behavior remains unchanged.

Reverified on 2026-08-22 before the DL-25 task-window skeleton migration:

- Old NxTools keeps one `DownloadProgressWindow` per task in `Views/MainWindow.axaml.cs`. Its
  `DownloadProgressWindow.axaml.cs::ShowCompletionState` hides the progress layout, reveals the
  completion layout, resizes around the existing center and keeps the same registered native window;
  `DownloadCompletedWindow` is only the passive fallback when that keyed progress window was closed.
- AB Download Manager follows the same owner chain:
  `AppComponent.kt -> ShowDownloadDialogs.kt -> BaseSingleDownloadComponent.kt ->
  ProgressDownloadPage.kt/CompletedDownloadPage.kt`. One task-keyed dialog changes from progress to
  failed/completed presentation; repeated opens focus the existing dialog and failure remains an
  in-window state when that dialog is alive.
- NxTools.Next already preserves this lifecycle in
  `DownloadWindowLifecycle::task_changed -> DownloadApplication::apply_task_window_directive ->
  update_task_window -> DownloadTaskWindowView::replace_snapshot`. A transform updates the existing
  view entity, calculates a center-preserving native `SetWindowBounds`, refreshes that same window and
  never opens a second keyed entry. Terminal fallback remains separately gated by completion settings.
- Before DL-25, `DownloadTaskWindowView::render` rebuilt the entire root, titlebar, content and footer
  independently for Completed, Failed and Progress. Progress used a 42 px footer while both terminal
  states used 44 px, and no production selector proved that all three phases shared one skeleton.
  Consequently visual drift could coexist with a correct lifecycle state machine.
- DL-25 must introduce one shared task-window root/titlebar/body/footer skeleton with a 34 px titlebar
  and 44 px footer, while leaving phase-specific content and actions in their current owners. A real
  production keyed window must expose Progress, Failed and Completed through the same skeleton and
  view generation. Center-preserving resize math and the one-entry registry remain explicit gates;
  completion action hierarchy and drag behavior belong to DL-26 and are not redesigned here.

Reverified on 2026-08-22 before the DL-26 completed-window action migration:

- Old NxTools exposes the same completed action row in both
  `Views/DownloadProgressWindow.axaml::{CompletedFooter,CompletedContent}` and
  `Views/DownloadCompletedWindow.axaml`: Open, Open Folder, an icon-only drag source and Close.
  `CompletedOpenButton_Click` and `CompletedOpenFolderButton_Click` execute the final-artifact action
  and close the terminal window; neither completed footer contains Delete. Both drag surfaces start
  from `PointerPressed`, require the left button to remain pressed and keep the native/file-transfer
  gesture attached to the visible completed window.
- AB Download Manager's
  `desktop/app/.../singleDownloadPage/CompletedDownloadPage.kt::Actions` uses the same order and
  hierarchy: Open and Open Folder are labelled actions, drag is an icon action with a tooltip and
  Close is separated at the far edge. Open/folder dismiss through
  `BaseSingleDownloadComponent::{openFile,openFolder}`. The Compose `dragAndDropSource` stays on the
  icon's visible hit target and supplies the operating-system file-list transferable directly.
- NxTools.Next already keeps the correct typed owners:
  `DownloadTaskWindowView::render -> DownloadApplication::{open_task_artifact,
  begin_task_file_drag} -> PlatformCommand::{OpenShellTarget,BeginFileDrag}`. Rust selects the final
  artifact; C# owns Shell/OLE adaptation. The drag begins synchronously from the still-pressed GPUI
  `on_mouse_down`, carries the source HWND and must not move behind `cx.defer` or a click-only action.
- Before DL-26, Open File was visually primary, but the three actions had no stable completed-footer
  role contract. Drag was a private 28 px labelled `div`, Reveal was ghost and Open rebuilt a private
  small button, so the two secondary actions did not share the standard action tier and tests could
  not prove that Open remained the only primary action or that Delete stayed outside the completed
  path.
- DL-26 must use the shared 28 px action primitives: Open File is the sole accent-filled primary;
  Open Directory is an outlined labelled secondary; drag is an outlined square icon secondary with
  a tooltip. The role selector must live on each real hit target. Existing typed Shell actions,
  pointer-down OLE handoff, source HWND, final path, window lifecycle and the absence of Delete remain
  explicit production gates.

Reverified on 2026-08-22 before the DL-27 auxiliary-dialog shell migration:

- AB Download Manager first: `desktop/app/.../ui/template/DialogUi.kt` keeps dialog content and a
  separated footer in one stable column; `ConfirmDialog.kt` maps native close to Cancel and keeps
  confirmation distinct from cancellation. `PowerActionAlertWindow.kt` uses the same fixed content
  plus footer topology, maps native close to `performCancel`, disables actions while shutdown is in
  flight and closes only after the platform action succeeds. `QueueInfoComponent.kt` keeps schedule
  edits as configuration state with independent start/stop controls and weekday validation.
- Old NxTools `Views/DeleteConfirmationWindow.axaml{,.cs}` keeps deletion in a dedicated compact
  window. Cancel and native close return no confirmation; Delete returns the explicit local-file
  choice before closing. `SettingsWindow.axaml::SchedulerSection` keeps schedule controls together,
  while `DownloadProgressWindow.axaml::完成后` treats shutdown as a task-local, non-durable intent.
  The old immediate shutdown execution remains a negative reference; Next's cancellable typed
  PlatformHost boundary stays authoritative.
- Before DL-27, Next's schedule, delete and shutdown windows each rebuild their own 34 px titlebar,
  44/46/48 px footer and private button sizing. Schedule Apply is accent-primary, but Delete uses a
  permanently dangerous button and Shutdown Now uses an accent primary, so two destructive actions
  do not share the neutral-at-rest/danger-on-interaction contract. Schedule/Delete programmatic
  closes also leave stale keyed registrations until the next open attempt.
- DL-27 must render all three through one shared `DialogShell`: shared 36 px `ClientTitlebar`, one
  bounded body and one 48 px `DialogFooter`, with 28 px shared actions. Schedule Apply is the only
  accent primary. Delete File and Shutdown Now use the shared danger-hover variant; Cancel remains
  an outlined secondary action. Native close and Cancel discard local work, Delete closes only after
  removal succeeds, Shutdown actions remain disabled while execution is pending, and schedule/delete
  keyed registrations are cleared on every accepted close path. All cross-entity action callbacks
  remain deferred outside the leased GPUI view.

Reverified on 2026-08-22 before the DL-28 taskbar-overlay legibility migration:

- AB Download Manager's `ShowDownloadDialogs.kt::UpdateTaskBar` confirms that taskbar progress is a
  passive glance surface driven by current percent and lifecycle state. It does not provide an
  Explorer-internal aggregate layout or an editor, so it supplies state honesty rather than visual
  geometry for this Next-only surface.
- Old NxTools has no taskbar-internal aggregate overlay. `DownloadProgressWindow.axaml` does retain
  the useful hierarchy invariant that transfer numbers remain readable secondary information and
  progress owns the accent, while `WindowAnimationService.Attach` remains the motion reference.
  Ordinary progress/completion windows do not supply placement or edit-mode behavior to copy.
- NxTools.Next's current overlay is the behavioral source for the new surface: Rust aggregates the
  lanes and telemetry, GPUI renders one passive window, and C# owns taskbar-DPI placement plus
  passive/editable Win32 styles. Normal mode stays no-activate, topmost and click-through; edit mode
  alone accepts the existing typed move/resize gestures and keeps the same correlation lifecycle.
- Before DL-28, edit mode subtracts 10 px from lane width, reserves different vertical capacity and
  adds 5/1 px content padding, so entering the editor moves every information boundary and may drop
  a lane. Its telemetry also uses an arbitrary 9 px proportional row with a 10 px box, below the
  product typography scale. DL-28 must use the shared readable numeric role and keep one identical
  content frame and lane capacity in both modes; editing may add only an overlay border and resize
  handle, without changing telemetry/progress coordinates or transfer semantics.

Reverified on 2026-10-03 while planning the download-dialog visual / Explorer-reveal follow-up:

- AB Download Manager was inspected first at the Desktop `AddDownloadPage.kt`,
  `ProgressDownloadPage.kt`, `CompletedDownloadPage.kt`, `ShowDownloadDialogs.kt` and shared
  `BaseSingleDownloadComponent.kt` entries. It keeps one task-keyed window, aligns add-form
  configuration/metadata/actions, separates progress detail from the action/progress region, and
  gives completion one compact file-identity region plus a separated action footer. These are
  layout references, not copied Compose implementation or a requirement to restore a full tabbed
  page to NxTools.Next.
- Old NxTools `Views/AddDownloadWindow.axaml` keeps URL probe and confirmed form stages with their
  action row anchored after the form. `Views/DownloadProgressWindow.axaml(.cs)` changes the same
  per-task progress HWND into compact completion and offers Open, Open Folder, native file drag and
  Close. `ViewModels/MainWindowViewModel.cs::OpenTaskFolder` selects an existing final file,
  opens an existing directory directly, and falls back to the save directory when the file is
  missing. Legacy `OpenPathInExplorer` catches launch errors but does not confirm the selected
  Explorer HWND became foreground.
Reverified 2026-10-03 during implementation of the download-dialog / Explorer-Reveal follow-up:

- Current Reveal chain is `DownloadTaskWindowView::render -> DownloadApplication::open_task_artifact_from_task_window -> OpenShellTarget(SelectInExplorer) -> PlatformHost::OpenShellTargetAsync -> ShellTarget::Launch -> ExplorerShellActivation::LaunchAndActivate -> correlated result/identity-guarded close`. Rust carries one correlation ID and closes only the originating HWND + generation after foreground-confirmed success; failed, timed-out, disconnected, late and duplicate actions leave the completed window available for retry.
- C# always requests `/select,<file>` for an existing file, even when a matching folder window already exists. Existing directories are opened directly. Lookup includes minimized Explorer top-level windows, which are restored before activation. Success requires a post-activation foreground readback whose still-valid HWND belongs to the intended Explorer root. The lookup/activation loop is bounded; process launch or `SetForegroundWindow` alone is never success.
- Rust resolves Reveal targets by kind: select an existing final file, open an existing directory directly, and fall back to the saved directory only when the final file is missing. Open-file and drag requests do not fall back to a folder.
- One correlation is carried through Rust and PlatformHost diagnostics. Logs record bounded fixed stages, terminal outcome, elapsed time and native errors, and omit target paths, URL, file name and window title. User-visible Reveal failures are fixed/retryable and do not surface native path-bearing details.
- The production schedule Popover keeps draft/commit/dismiss semantics. Time selection and its Clear/Confirm footer are separate aligned rows; regression coverage measures real visible button bounds, tier, baseline, inset, overlap, shell containment and unchanged parent bounds.
- Automated verification passed: 428/428 `nxtools-desktop` tests, `cargo fmt --all -- --check`, the download UI lifecycle architecture gate, five PlatformHost Explorer activation cases, the separate non-DOpus shell-command responsiveness test, and PlatformHost Debug build with zero warnings/errors.
- User-provided screenshots could not be previewed in this environment. Native Windows DPI visual comparison and live Explorer foreground/file-selection smoke remain manual acceptance gates; the unrelated DOpus navigation harness was not run.
## Resolved phase-0 decisions

- CLI connects to a current-user local download pipe and starts the packaged desktop in background mode when
  the download host is absent. It never opens SQLite directly.
- Download command, storage and CLI v1 contracts are recorded in `contracts/`.

## Open research items

- The final authenticated browser transport: Native Messaging only, or Native Messaging plus a bounded loopback
  channel for large Blob payloads.
- HLS/DASH shared protocol seam must wait until the HTTP implementation and one media implementation both exist.

## 2026-10-08 SOL remediation: verified boundary contracts

Reference entries reverified before implementation: ABDM `PartSplitSupport.kt` (locked safe part boundary) and `HttpDownloadJob.getPartDownloader` (idle ranges before split, bounded connections); legacy `Services\DownloadManagerService.cs::{DownloadSegmentsAdaptiveAsync,DownloadDashSegmentsWithBoundedWorkersAsync}`; `Views\AddDownloadWindow.axaml.cs::{constructor,OnOpened,OnClosed,RefreshContentSize,SizeChanged}`; `DownloadProgressWindow.axaml.cs` and `Services\WindowAnimationService.cs` (same-window stages and deferred final close). The legacy checkout does not include Avalonia's native/base show/hide source. Next verification reaches real HWND client readback, not merely the leaf view.

- `CenteredResizeCoordinator.TrySchedule` bounds native work to two workers/64 HWND slots, one in-flight plus one coalesced next turn per HWND. IPC/EventWindow accept/enqueue without waiting on SetWindowPos. Accepted (`applied=false`) is not settled; a stuck native call keeps its slot and never triggers unlimited replacement threads.
- `WindowsPlatformAdapter.NativeMethods` pending work is an immutable generation transaction. Replay reads one complete snapshot; compare-and-update/compare-and-remove protect replacement, even with repeated Guid.Empty IDs. Final client correction/readback occurs after center correction, preserves the user's live moved center, and never changes activation or Z-order. Stale completion has no terminal authority.
- `NewDownloadWindowView::begin_probe` retires old generation/target before validation. Compact errors replace guidance until actual viewport reserve exists. Ordinary heights remain 176/278; media rows and errors use the same measured content budget. Timeout never releases clipped confirmation. Task details share centered native ownership; GPUI learns native pixels via WM_SIZE. Cross-entity export/settings work defers beyond the view lease.
- `segmented_runner` preserves shrink identity until joined writer sync and Waiting checkpoint, then durably merges an adjacent Waiting range. `TransferPlan::merge_waiting_segment` preserves the written prefix and rejects prepending unwritten bytes to a progressed right range. One sample is appended per tick; stable shrink precedes growth/low-benefit evaluation; saved grown plans use the request's initial floor and an attempt-local learned baseline.
- `dash_runner` re-fetches Completed fragments whose nonempty file length disagrees with the checkpoint; it still adopts legitimate uncheckpointed crash-window bytes. No equal-length cryptographic integrity claim is made.
- `blob_upload` does not idle-reap Receiving. The owning socket bounds each body to two minutes, per-read timeout at most 30 seconds/remaining deadline, rejects a duplicate Receiving claim, and owns cleanup/capacity release.
- `http` keeps safe routed/certificate-policy client reuse, policy generation invalidation and per-request ten-second probe deadline. HEAD and Range both complete via join; the abandoned 150-ms HEAD shortcut is not the contract. Diagnostics remove reqwest URL payloads and report phase durations only.

Regression evidence and limits: [SOL remediation report](../audits/2026-10-08-sol-review-remediation.md). The full localhost grow/shrink test uses production HTTP writers/SQLite/checkpoints/pause/reopen/output and only injects speed samples. It does not prove external jitter or the real one-second sampler. Native/GPUI tests do not replace live pointer/probe/DPI acceptance. Full engine-contract runs still have unresolved failures; no package/install was performed.

### 2026-10-09 engine-contract fixture closeout evidence

ABDM `HttpDownloadJob.kt::fetchDownloadInfoAndValidate` and `pause/stopAllParts` were reverified (length/ETag/resume validation, stop/join before durable pause). Legacy `DownloadManagerService.cs::MergeDashAndVerifyAsync:3595-3682` concatenates selected tracks then uses cancellation-aware FFmpeg stream copy before final publication; it provides no socket-fixture behavior to migrate. No window/layout/focus or product semantics change belongs to this closeout.

The real Next chain is `DownloadEngine::resolve_missing_file_name_before_acceptance -> HttpDownloadClient::probe`, followed by `segmented_runner::run_task -> probe` and durable pause/reopen/resume; DASH reaches `dash_runner -> media::{detect_ffmpeg_version,mux_dash_tracks}`. Both failing contracts share `tests/support/mod.rs::serve_connection`. On Windows the accepted stream inherits the nonblocking listener mode: an early zero-byte read returns WSAEWOULDBLOCK (10035), and the fixture discarded it. Explicitly restore blocking mode on accepted streams before the existing bounded read, rather than changing production retry/probe policies.

Two causal reproductions: FFmpeg mux-marker wait actually failed earlier with Network/error-sending-request after 10035; crash-window initial HEAD/Range were both dropped, acceptance fallback GET consumed `serve_resume_required` zero-offset eligibility, then worker fallback GET returned HTTP409. A delayed-first-byte raw socket regression is red before correction (client 10053/server10035); run for both serial and concurrent fixture modes. HEAD shows the same listener/accepted-stream defect, not a new SOL production regression; compiled HEAD baseline was not run. Tests must surface a task terminal failure before a missing marker timeout while retaining cancellation/output/staging assertions. Diagnostics/probes contain native code/state only, no user payloads, and temporary DEBUG-ENGINE-CLOSEOUT probes must be removed after verification.

### 2026-10-09 user screenshot: post-readback retirement and queued size mismatch

The installed fixed Validation five-binary hashes matched the latest packaged build; no old-package explanation. User screenshot still showed confirmed directory/inspector inside a176px client without footer. Safe dimension-only runtime projection recorded requested278 then final176 with success/client_size_settled. Product pointers/DPI acceptance therefore remains red, regardless of earlier native/GPUI tests.

References rechecked before correction: ABDM AddDownloadPage and legacy AddDownloadWindow constructor/Open/Closed/RefreshContentSize/SizeChanged retain content measurement/live center; GPUI0.2.2 Windows events.rs::handle_size_msg170–213 trusts WM_SIZE lParam for logical_size/render/resize callback, move modal loop216–247 pumps app work; Windows window.rs::resize schedules native SetWindowPos. Avalonia native/base source remains unavailable.

Actual regressions added at production native seams: a real HWND changed to compact176 after replay readback but before commit returned success=true,pending=false; old cached clientMatches was used at retirement. A custom-client HWND grew278 then shrank176 before pumping messages; one queued synthetic WM_SIZE still announced278 against GetClientRect176. Fix retirement readback at commit and avoid payload-stale posted WM_SIZE; use actual native frame/client recomputation rather than a second logical-only resize owner. A same-size refresh regression must prove current native WM_SIZE is delivered, not merely delete synchronization and make no-message tests pass. No extra retry/worker budget, no native bounds animation, no focus/Z-order changes; final dimensions and generation remain authoritative.

Operational incident: a cmd-quoted C# test filter was parsed incorrectly and selected a broader harness including forbidden real DOpus navigation; it is unrelated to download behavior. Agent stopped/reported it, user authorized only continued download repair. Subsequent checks preinspect exact names and direct PowerShell argument arrays, reject unexpected RUN names and exclude both real DOpus cases. No further DOpus/product/service/package operation is authorized by this follow-up.

Directed native review found that SendMessageTimeoutW(ref WINDOWPOS) can return while a delayed receiver still owns the marshalled parameter. A hidden HWND regression held the receiver beyond2s and failed with `client refresh returned while the receiver still owned its WINDOWPOS parameter`; it deliberately skipped delayed pointer reads after early return, rather than provoking undefined memory access. The repair uses SendMessageW inside the existing fixed resize workers. The receiver reads the retained parameter and derives current WM_SIZE before return; placement/client/foreground remain unchanged. A permanent native hang retains a fixed slot; no claim of bounded Windows execution or forced cancellation. Microsoft SendMessage/SendMessageTimeout documentation was checked directly. Real GPUI pointer/DPI acceptance remains pending; the installed09:19 Validation package lacks this follow-up.

### 2026-10-09 staged Downloads presentation recheck (before UI edits)

- ABDM first: `desktop/app/.../pages/addDownload/single/AddDownloadPage.kt::{AddDownloadPage,ConfigActionsButtons,MainActionButtons}` keeps URL/configuration and footer decisions in one compact window; `desktop/app/.../pages/singleDownloadPage/ShowDownloadDialogs.kt::{ShowDownloadDialog,CommonContent,CompletedWindow,ProgressWindow}` switches one keyed component to its completed presentation, with native `CustomWindow` handling actual visibility and `onCloseRequest` calling the component close. `CompletedDownloadPage.kt::Actions` keeps the pressed-gesture drag source and open/reveal actions in a separated footer; `desktop/app/.../ui/template/DialogUi.kt::{DialogMainContent,DialogFooter}` separates dialog body and actions. No ABDM queue or transfer behavior is changed by this presentation pass.
- Legacy next: `Views/AddDownloadWindow.axaml.cs::{AddDownloadWindow,OnOpened,OnClosed,RefreshContentSize}` attaches `Services/WindowAnimationService.cs::Attach` in the constructor; the service subscribes to `Window.Opened` and `Window.Closing`, animates content/opacity and calls `window.Close()` after the closing animation. Its `Window` base/native show/hide implementation is not checked into the legacy tree, so leaf callbacks alone are not proof of HWND visibility. `Views/MainWindow.axaml.cs::{ShowDownloadProgressWindow,ShowDownloadCompletedWindow}` owns keyed reuse, then calls `Show`; `Views/DownloadProgressWindow.axaml.cs::{DownloadProgressWindow,OnOpened,OnClosed,ShowCompletionState}` attaches the same base animation and switches its already-open window to completed content with center-preserving resize. Closing remains presentation-only; one-window completion and passive fallback remain distinct.
- Next presentation boundary: `DownloadWindowLifecycle::task_changed -> DownloadApplication::apply_task_window_directive -> DownloadTaskWindowView::replace_snapshot` owns phase/key identity. `NewDownloadView::begin_probe` and the C# native centered bounds/WM_SIZE path own staged 176/278 client budgets and media row expansion, not a GPUI style pass. `DownloadTaskWindowView::render` and `main_window/download_page.rs` project state only; callbacks defer application mutations while the view is leased. Keep source HWND and synchronous pressed-pointer entry for OLE file drag; keep typed failure, bounded inspector/error, selected-ID details and schedule/delete draft lifecycles. Focus/action/pressed styling may change without moving those boundaries. Actual live show/hide, pointer, DPI and native drag acceptance remain separately pending.

### 2026-10-09 staged keyboard/four-grid follow-up (verified before product edits)

- ABDM first: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/AddDownloadPage.kt::{AddDownloadPage,ConfigActionsButtons,MainActionButtons}` exposes a bounded add flow with ordinary Compose clickable/button controls, while `pages/queue/QueueInfoComponent.kt::createConfigurableList` keeps weekday/start/stop configuration as a queue-owned draft. Neither supplies NxTools.Next's one-time date-grid keyboard policy. `pages/singleDownloadPage/CompletedDownloadPage.kt::Actions` remains a native pressed-gesture drag source, not a click substitute.
- Legacy next: `Views/MainWindow.axaml` plan-start row and `ViewModels/MainWindowViewModel.cs` scheduled-start fields provide the one-time scheduling behavior, not a GPUI calendar implementation. `Views/AddDownloadWindow.axaml.cs::{AddDownloadWindow,OnOpened,OnClosed}` attaches `Services/WindowAnimationService.cs::Attach`, then Avalonia's `Window` base handles the actual Show/Close HWND boundary; base source is not checked in. Reuse Next's existing anchored date popover and preserve its draft/commit/clear/outside-dismiss focus return, rather than copying native date UI or another window.
- Next gap: `new_download_view.rs::delayed_start_calendar` currently builds each day as a pointer-only `div` with an `on_click` installed only for non-past dates. Promote eligible dates to one keyed GPUI Button/KeyUp click path; past dates must be truly disabled, not just dimmed. Keep 40×19 day hit rectangles, month navigation's existing Buttons, generation-scoped view update via `window.defer`, and fixed popover/176/278 client budgets. `settings_view.rs::download_settings_navigation_item` and `schedule_view.rs` weekdays already use keyed focusable_content/Button from the earlier integration; don't add duplicate keyboard dispatch. `task_window.rs` native OLE file drag remains mouse-down-only with the source HWND. Other custom actions need a separate scoped boundary test before altering their behavior.

## Repeated real transition failure — fixed-stage investigation
User's new Validation screenshot shows the compact176 client and explicit resize-timeout error after probing, not a confirmed278 surface. Rechecked ABDM AddDownloadPage/BaseOptionDialog and legacy AddDownloadWindow constructor→WindowAnimationService.Attach→OnOpened/SizeChanged plus Stage/SizeToContent refresh; legacy uses one staged Window and its Avalonia native base remains unavailable. Current Next probe completion stages metadata, sends a cross-process centered resize, waits viewport/timeout and clears a successful probe on presentation expiry. Existing GPUI tests compile `native_window_handle` to None and thus do not execute this native gate; direct278 screenshot likewise bypasses transition. Add an owned Debug GPUI HWND/PID fixture (real176→C# production resize→real client/viewport278), separate from ordinary runtime/service/clipboard/network, before claiming a new fix. Non-resizable GPUI minmax sets only minimum track size, not a fixed maximum; actual message path must be measured rather than blaming old code. Ordinary176/278 remain fixed; placement/focus/cancellation/generation and media budgets must survive any stage-replacement simplification.

### Approved fixed-window stage handoff (2026-10-09, before Rust implementation)

ABDM `AddDownloadPage.kt::AddDownloadPage` retains its own dialog and closes through `onCloseRequest`; legacy `AddDownloadWindow.axaml.cs::{AddDownloadWindow,OnOpened,OnClosed,RefreshContentSize}` attaches `WindowAnimationService.Attach` in its constructor, then changes `SizeToContent` on the same Avalonia `Window`. Its base/native Show/Close implementation is not checked in. The user explicitly changes the Next presentation behavior: successful probe replaces the compact GPUI HWND with a new hidden final-size confirmation HWND, rather than waiting for an in-place Win32 resize/WM_SIZE on the still-moving compact HWND. The production fixture `downloads/polish_tests.rs::successful_probe_replaces_compact_window_with_fixed_confirmation` was red because the old registered window remained the same. Preserve one registered add window, retained successful probe/draft on failed native placement, source HWND center from the native adapter, generation/identity checks before closing the source, and deferred cross-entity callbacks. `PresentFixedWindowOverAnchor { window_handle, anchor_window_handle, width, height }` carries final device-pixel client dimensions: C# applies those bounds centered over the source while hidden, shows nonactivating, then reads back once; no resize of the source or retry loop. Direct synthetic forms and failed probes remain independently bounded; native success is not inferred from a timer or admission.

Rust fixed-stage slice (compiled/native verification deferred to parent): `NewDownloadView::begin_probe` stages successful data and defers `DownloadApplication::create_fixed_confirmation`. The owner validates registered source/window liveness plus weak-view probe generation before creation and completion. Fresh InputState/SelectState entities are constructed for the target; copied state includes external/confirmed request, local destination/name/category/schedule, idempotency, probe/media selection and defaults. One pending slot is installed before caller-ID IPC dispatch; failure/disconnect/one ten-second IPC timeout closes only the target and preserves the successful source draft for presentation-only retry. A deferred window-closed observer retires orphan targets; stale/late results cannot register or close a newer owner.

Pinned GPUI0.2.2 `src/platform/windows/window.rs:471–477` stores hidden `initial_placement` rather than applying requested HWND bounds, and `activate:690–705 -> set_window_placement:302–324` takes/replays it through SetWindowPlacement before focus. Therefore Rust never calls GPUI activate_window on the replacement or when reusing the registered add window; final placement/show and activation stay native-owned. Local target input focus/notify are deferred without native activation. Actual user probe/move/DPI lifecycle is still unverified; the earlier Debug176→C#resize278 fixture is not this replacement boundary.

## Progress-to-completion position acceptance follow-up
User accepted the15:38 new-download fixed handoff but reports slight downward movement on task completion. Rechecked ABDM ShowDownloadDialogs (progress/completed owner-specific state/windows), then legacy DownloadProgressWindow constructor→WindowAnimationService.Attach→OnOpened and ShowCompletionState→ResizePreservingCenter: final432x196 preserves native center, clamps work area, locks size and retains same task window. Next progress184/completion180 share one keyed HWND; current creation min size equals initial phase, potentially constraining a smaller terminal phase. A center-only native control fixture with min176 passes repeated184/280/180 transitions; that does not cover the real task min184. Add Debug real task-view/window-options fixture and exact native HWND center/minimum regression before correcting constraints. Preserve keyed task/window generation, presentation-only close, final-file/OLE and passive terminal fallback; no new size recovery loop or independent GPUI resize owner.

2026-10-09 correction evidence: the owned real task HWND/min184 fixture rejected target440x180 with actual440x184; the earlier new-download/min176 center control was not equivalent. Two production regressions were behavior-red: collapsed progress184 differed from completed180, and the task lifecycle emitted a native centered resize on ordinary completion. Progress/completed now both use440x184; update_task_window compares previous phase/details logical budget with next budget and emits resize only when different. Minimum size is the collapsed184 budget for all initial phases, so failed196 can return to progress184. Same task/window generation, content/footer containment and no resize command pass. The exact owned native test covers184/280/184/196/184 repeated three times with15 zero-center-drift observations; it is not an installed physical-pointer/mixed-DPI acceptance claim.

## Feature-first Downloads workbench migration
Before styling rechecked ABDM `pages/home/sections/DownloadList.kt::DownloadList`:table keyed by task, Ctrl/Shift selection, double-click open, right-click options and selected transfer; legacy MainWindow.axaml TaskListBox multiple selection, row PointerPressed/ContextRequested, name/source/size/status/progress/speed/remaining/created and complete selected-task metadata. Existing Next six state filters/counts, full sort-key menu/search-clear, Windows selection/context capability, batch/queue actions, virtual rows and five detail tabs remain source-of-truth, despite reduced browser fixture. Preserve actual MainWindow→deferred DownloadApplication request/result and existing new/task HWND owners traced above. Scope is workbench row/header readability/alignment only; no change to accepted probe fixed handoff, completion geometry, durable download, native/OLE, fields or controls. Before code adjust actual production text/row geometry tests without removing functional/containment assertions.

## Remaining browser companion presentation migration
Rechecked legacy browser_extension/nxtools-interceptor/popup.html/css four task/capture/tool/settings tabs, current Next popup IDs and event bindings, deterministic456px root and fixed420/445 scrolling workspace; media-entry shadow root and drag target retain original handlers,positioning,source/privacy policies. Typography/control/rhythm edits stay in HTML copy/CSS/shadow-styles only, no background/content-script/popup routing changes. Keep every task/resource/media/settings control, counts, disabled merge limitation and native-handoff/cookie/header policy. Browser fixture mounts synthetic controls only without real Chrome API/native messaging/network; DOM geometry is not real installed-extension acceptance.

### Remaining auxiliary presentation reference ledger (2026-10-09)
Pre-edit verification was recorded in session scratch `downloads-auxiliary-reference.txt`; this entry synchronizes that evidence, not a new inferred lifecycle. ABDM inspected first: AddDownloadPage.kt:91–181; CompletedDownloadPage.kt:38–145; ShowDownloadDialogs.kt:51–79,108–195; shared BaseSingleDownloadComponent.kt:107–138,233–285; DialogUi.kt:12–21,42–62; ConfirmDialog.kt:43–103; PowerActionAlertWindow.kt:50–70,74–191. Newly resolved scheduler entry is `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/queue/QueueInfoComponent.kt`:148–155,247–301. `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/CustomWindow.kt`:325–373 delegates close/state/resizable/topmost to Compose Window; AWT/Compose internals are not inspected Windows-native implementation.

Legacy inspected after ABDM: AddDownloadWindow.axaml:54–105 and .axaml.cs:20–39,73–109; DownloadProgressWindow.axaml.cs:26–59,84–135; DeleteConfirmationWindow.axaml.cs:13–37,47–60; SettingsWindow.axaml:900–927; WindowAnimationService.cs:13–49. Constructor → animation attach → base Opened/Closed and explicit cancellation/guarded deferred Close were followed; Avalonia native base source remains unavailable. Invariants: original URL/context/default/media/category/schedule/idempotency controls, fixed560×176/new278, media353/418/456, task184/280/196, schedule430×326/shutdown450×200, deferred callbacks and sole native/OLE owners survive typography/alignment changes. Current tests and capture limits: [remaining audit](../audits/2026-10-09-remaining-ui-layout-migration.md).

### Browser-owned host idle lifetime and passive health correction (2026-10-10)
User reports BrowserHost surviving desktop Exit and blocking updates. Before changes, actual extracted background connection functions with fake Chrome ports reproduce engine rejection retaining transport and automatic disconnect→reconnect; new4-test loop is red on idle release/no unsolicited reconnect/startup/Blob lease. ABDM AppComponent:995–1010 stops its download work then exits; legacy MainWindow.axaml.cs:196–212,2795–2803 explicit Exit bypasses tray hide; desktop dispose:188–190→MainWindowViewModel:2551→LocalApiService:54–76 closes its own WebSocket/listener. Legacy background:1046–1120 reconnects WebSocket. Native Messaging changes ownership: browser starts host, stdio EOF exits host lib.run→main; desktop owns PlatformHost child and service client, not browser-owned host or LocalSystem service. Current source run:251–288 waits stdin; hello393–400 only confirms transport; snapshot959–962 currently starts a background desktop, and extension worker startup4213–4222 eagerly connects. Installer overwrites exact app BrowserHost; default Restart Manager behavior is not proof of reliable closure. Current live host path differs from Validation; installed binary/registry identity not read.

Correction scope: demand-only native transport, bounded idle release when no pending/connecting/request owner; no autonomous reconnect or diagnostic-only spawn; Blob capability→upload→commit retains one lease through finally. Current-port identity guards prevent old callbacks mutating replacements. get_snapshot becomes passive existing-pipe-only and must not launch desktop; explicit download requests retain original startup. Preserve protocol2/256KiB, request/handoff IDs, per-item batch/fallback, Blob2GiB/Receiving exclusive lease, bounded timeout/reject once and low-noise lifecycle diagnostics. No service stop/kill/registry/install operation authorized; real EOF/file-lock release and installer native acceptance separate.

## Queue02 download Settings component prerequisite — 2026-10-11
ABDM inspected first: desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/
pages/settings/{DesktopSettingsComponent,SettingsPage}.kt -> ConfigurableGroup/
RenderConfigurableGroup; shared/app/src/commonMain/kotlin/com/abdownloadmanager/
shared/settings/CommonSettings.kt numeric ranges/zero-speed, folder validation and
explicit ignoreSSLCertificates. Current-page change only selects configurables;
ABDM repository-backed writes are not transplanted into Next's explicit Apply draft.
Legacy next: Views/MainWindow.axaml.cs::ShowSettingsWindow -> SettingsWindow
constructor -> WindowAnimationService.Attach -> ShowIndependentSecondaryWindow::Show;
PresentExistingSettingsWindow reuses/activates; Closed releases registry. SettingsWindow
OnOpened configures scope; SaveButton_Click -> ApplySettings -> ConfigureDownloadManager/
MarkStateDirty/SaveStateIfDirty -> AppStateStore::Save temporary-file move; close uses
CloseSettingsCommand and OnClosed without Apply. Avalonia native base source is not
available; observed public Show/Close is the reference boundary, not a native-source claim.
HeadersSection/RequestHeaderParser confirms named Header/Cookie and dedicated Cookie
precedence; CategoryDialogViewModel::BuildState keeps all category fields; FFmpeg
InstallFfmpegAsync/CancelFfmpegInstall retains cancellation and configured-path policy.
Next keeps984x650 client, five General/Request/Transfer/Window/Diagnostics topics,
all original entities/draft ranges/picker targets/FFmpeg generation/install/overlay
edit/native close/deferred Apply/Cancel. Official Settings owns search/selection/scroll
only, resettable(false); no reset or automatic persistence. Request Textarea owns
its single120px frame/internal scroll; secrets never enter diagnostics. Shared
field_shell already re-enables Input appearance and is not broadly rewritten.

## Queue03 main download component prerequisite — 2026-10-11
ABDM first: desktop pages/home/sections/DownloadList.kt::DownloadList stable task key,
Ctrl toggle/Shift range/secondary gesture/native dragAndDropSource ->
HomeComponent::onRequestOpenDownloadItemOption stable selection and tableState sort
-> BaseHomeComponent selectionList/downloadList filter reconciliation. Legacy next:
MainWindow.axaml TaskListBox Multiple/selected inspector/full columns ->
MainWindow.axaml.cs TaskRow_PointerPressed Ctrl/Shift anchor, ContextRequested ->
SelectTaskForContextMenu/BuildTaskContextMenu -> MainWindowViewModel SortBy/
RebuildVisibleTasks/GetSelectedTasksOrCurrent and typed action compatibility.
Keep Next six filters/counts, stable IDs/anchor/sort persistence/view-only queue order,
context capability subsets, double-click, detail five tabs/reset-scroll and fixed
main1060x650/min920x480. Ordinary progress is aggregate per task; segment geometry
remains domain content. Shell/native OLE pressed-pointer/source HWND/unique C# STA
and all auxiliary lifecycle boundaries are unchanged. Table/DataTable public row/
cell/sort/context slots reviewed, actual capability tests required before grid
decision; a business owner alone does not prove incompatibility.

### Queue03 main presentation result and public grid boundary
Production uses real0.7.1 InputGroup/Search/InlineEnd clear on original entity,
Toolbar/ToolbarGroup header+batch public slots,Small underline TabBar/fiveTabs,
XSmall ordinary aggregate Progress and overview horizontal DescriptionList values.
Original six counts/all columns/stable-ID CtrlShift anchor/view sort/context/queue/
batch/deferred/keyed-native/OLE source-HWND/sole C# STA owners preserved; domain
segmented visualization unchanged.20key production bodies equal dirty03baseline.
Reference order remains ABDM DownloadList/HomeComponent/BaseHomeComponent before
legacy MainWindow TaskListBox/row pointer/context/SortBy; no unavailable Avalonia
native-base or proprietary engine-source claim.

Exact DataTable public experiment: Ctrl-click built-in one semantic row index;
row_selectable(false)+delegate.render_tr callback retains external IDs but release
state.rs::render_table_row overwrites row aria_selected with internal single-index
selection. Actual sort-icon pointer invokes perform_sort; secondary gesture invokes
context_menu. These callbacks are supported, not automatically incompatible. Static
Table has no virtualization. Retain only original virtual main grid pending user
exception/upstream-public-hook decision; untested child/Shift/OLE adapters not claimed
impossible or certified. No library fork/dependency/native change.

564/564,strict tests/prodClippy/workspace/fmt/6source gates0;production default/minimum/
long-input/entity/clear/focus/five-tabs/scroll/DescriptionList/fill/batch-rail checks.
Toolbar/Progress no automatic TestSupport;real descendants/arrow focus/painted quads
used. TestWindow native a11y tree unavailable; provider/reader remains unverified.
No03PNG acquired/inspected or native drag/runtime/Release/service/user-data operation.
Full adoption/defer/check detail: owning ui-queue-outcomes.md03;final visuals queue12.

## Queue04 new-download Form/Radio/Popover review and result
ABDM reviewed first: desktop/app/pages/addDownload/single/AddDownloadPage.kt URL ->
category/location/name ->metadata/config ->MainActionButtons; public extra config
retains credentials. Constructor/window boundary: desktop/app-utils/window/custom/
OptionsDialog.kt::BaseOptionDialog ->Compose DialogWindow visible/modeless/undecorated,
focus-loss dismissal/topmost and listener disposal. Old NxTools after ABDM:
Views/AddDownloadWindow.axaml.cs constructor ->WindowAnimationService.Attach ->
DataContextChanged/RequestClose ->OnOpened dark titlebar/size ->OnClosed detach/dispose;
BrowseFolderButton uses native storage picker; AddDownloadDialogViewModel parse
generation/category-directory/AddOnly/DownloadNow/Submit carries requestHeader/Cookie.
Avalonia native base unavailable; no claim that native source was inspected.

Actual Next presentation: released0.7.1 Form/Field per original control, horizontal
URL/directory and two-column filename/category on stable styled InputState32px;
default styled paste/picker buttons. Popover sole styled frame schedule304x240/public
compact inset/category320x240/default inset,manual surface/appearance(false) removed.
Category stateless ListItem two-line data/default12px inset on original search/
highlight/scroll/keyboard/selection/close/focus owner. HLS/DASH actual RadioGroup/
SmallRadio40px,one direct public StatefulInteractiveElement::on_click for nullable
DASH audio preserves repeat-to-clear. Checked on_change itself inert; direct click
and true focus/explicit KeyDown+KeyUp once tested. No duplicate row dispatcher.
No new visible HeaderCookie editor: original carried request/default data retained.
Synthetic typedHeader/Cookie hydration/edit regression verifies carry-through.

Official date candidate: calendar.rs::Calendar Small actual fixture exceeds156px
existing date region; date_picker.rs public CalendarEvent/select_date/Change/open
and nested Calendar reviewed. DirectCalendar can't retain full date+time+actions
inside304x240. Scrollable/nested public alternatives unverified,not globally rejected.
Only delayed_start_calendar remains narrow pending exception/navigation decision,
insideNEWofficialstyledPopover; original date/month/outerClearConfirm draft unchanged.

Fifteen key bodies whitespace-normalizedequal04dirtybaseline:request/probe/submit/
fixeddraft/hydrate/dategrid/media/size/inspector. Hidden target/pending-before-native
fixed apply/presentation-only retry/source-HWND/C#ownership unchanged.560x176->fresh
560x278/media353/418/error456 retained; localhostproductionprobe/newtarget/source
retirement/fieldfooter/media/audio-cancel/category32scroll tests pass.
568/568,stricttests/prodClippy/workspace/fmt/6sourcegates0. No04PNG/standaloneDebugbuild/
start/actualHWND/nativehandoff/physical/DPI/reader/installed/Release/service/userdata.
Exact adoption/deferreddecision/check logs: owning ui-queue-outcomes.md04.

## Queue05 supporting-content prerequisite and native invariants
Reviewed ABDM Desktop first (not the Android same-name files):
desktop/app/pages/queue/QueueInfoComponent.kt::createConfigurableList independent
start/end flags and nonempty weekdays; pages/singleDownloadPage/ProgressDownloadPage.kt
::RenderProgressBar/RenderActions/RenderPartInfo and ShowDownloadDialogs.kt keyed
downloadId -> CustomWindow; pages/poweractionalert/PowerActionAlertWindow.kt ->
PowerActionComponent::performCancel/performPowerAction, ui/template/DialogUi.kt and
ui/widget/ConfirmDialog.kt. desktop/app-utils/window/custom/CustomWindow.kt delegates
to Compose Window; AWT/Compose Windows-native internals were not inspected.
Then old NxTools Views/DownloadProgressWindow.axaml.cs constructor -> animation
attach -> OnOpened/OnClosed, same-window ShowCompletionState, pause/resume/cancel,
Shell actions and pressed-pointer native drag; Views/DeleteConfirmationWindow.axaml
and .cs constructor -> OnOpened -> explicit Delete/Cancel result -> OnClosed(null);
SettingsWindow.axaml::SchedulerSection raw HH:mm text; Services/WindowAnimationService.cs
Opened/Closing -> guarded deferred Close. Avalonia native base unavailable.
Next production deletion is live: main_window/download_page.rs context action ->
DownloadApplication::request_delete_confirmation -> keyed independent380x178 Root/
DeleteDownloadView. Disabled task_removal_deletes_file means list-only removal;
Cancel/native close discard presentation; successful explicit confirm removes file
and record. Stale retained/dead_code annotations are not reachability evidence.
Preserve schedule430x326/task440x184,expanded280,failed196/shutdown450x200, one native
resize owner, task HWND/generation and same-size completion, C# sole STA/OLE source
HWND/still-pressed pointer/cancel boundaries. Kit DialogContent/Footer are inner
content only; no Root modal, DialogAction/Close dispatcher or native-host replacement.
Exact release TimeField/TimeFieldState stores NaiveTime; invalid digit pairs restart
with a valid digit. Original schedule InputState keeps raw invalid/intermediate text
and validates only at Apply. This policy conflict is narrow, not a blanket API rejection;
actual focused digit compatibility test and user alternatives belong to outcomes05.

Queue05 result: actual schedule Switch/independentSmallToggle/per-fieldFormField/
StatusBar/originalstyledInput;taskXSmallProgress/SmallDescriptionList/ToolbarCheckbox/
DialogFooter;shutdown/delete inner DialogContent/XSmallAlert/Footer/styledButtons.
Original rawHH:MM draft/Apply-error retained pending TimeField policy decision.
22production function bodies whitespace-normalized equal05dirtybaseline;task window
and completion-shutdown controllers bytes equal. Original native sizes/keyed lifecycle/
same-HWND content/single resize/OLE/cancel unchanged.571/571,strict tests/prodClippy/
workspace/fmt/sixsourcegates0;default/long/error/footer/action/release lifecycle tests.
No05PNG/actualHWND/physical/DPI/reader/installed/Release/service/user-data operations.
Exact evidence/alternatives/limits: owning ui-queue-outcomes.md05,consolidatedgallery12.

## Post-gallery user decisions — 2026-10-12
User temporarily retains original Download virtual grid for later separately authorized
package review; no packaging now. User explicitly authorizes official TimeField with
application validation, superseding raw-invalid-text retention for daily schedule only.
Rechecked ABDM Desktop QueueInfoComponent TimeConfigurable/nonemptyday/startstop first,
then old SettingsWindow constructor/animation/baseOpen/ApplyClose/Scheduler HHmm input;
native Avalonia base unavailable. Preserve independent430x326/nativehost/saveCancel,
enable/day/crossmidnight/busy/failure rules. Category master now authorized ListItem
on original selection/draft/order/enable/delete owner, not a new ListState/controller.
Exact0.7.1 TimeFieldState time/set_time, H23/minute and publicTimeField are reviewed.
Calendar supports outerStyled/officialsize but itemcells Small28/default32/Large40;
customSize is fallback32, not arbitraryitemshrink. Outercompactpadding/scroll adaptation
not yet certified; previous directSmall conflict is not universal inability to resize.

Public naturalgeometry measurement in post-gallery feedback:Small padding12 220x224;
Small padding0 220x200;Medium/customSize20 padding0 both248x228. Removing outerpadding
works but does not shrink28pxdatecells below original156px inlinebudget. Scroll/DatePicker
alternatives remainunverified. ActualTimeField/Applyvalidation and categoryListItem now
implemented;623tests/strict/build/gates pass. See ../audits/gpui-kit-ui-feedback.md for
new30PNG inspectionblock and exact remaining acceptance,not historical05deferredstatus.

Actual post-gallery PNG inspection after Browser restore found schedule TimeFields
32px inside44px Field slots, producing20px between controls. Adopt32px slots with
original8px bodygap and intact32px TimeField/430x326/footer/error/native budgets.
ABDM Desktop OptionsDialog native modeless focus/dispose and old SettingsWindow
constructor/WindowAnimationService/baseOpen/ApplyClose chain rechecked; no native
policy transplant. Production gap regression red before local slot correction.
NewPNGinspection also confirms Downloads ordinarySwitch atx924 in984pxclient. Only
nonwideSettingFieldrender receives440pxlane;its projectedcontrol uses flex natural
width. Widecategory/request/FFmpeg/pathdiagnostic slots remain unchanged. Actual
Switch position red precedes change;fullcategory/footer/action/nativegates required.

## Authorized entire browser UI redesign — 2026-10-12
User authorizes new popup/injected UX. Rechecked legacy browser popup.html/js and
Next popup render/setView/refresh/collectSettings, content ensureOverlay -> adopted
Shadow stylesheet/showResourceChoices/closeOverlayPanel and configured drop bounds.
Original four views, task8/resource24/batch32 limits, auto-save plus explicit retry,
media index/frame/action messages, queue-only batch, pointer choice/Escape/focus and
original28px media entry placement remain. Background/page-hook/handoff/capture/Blob/
manifest permissions/CSP/extensionID unchanged. Popup state is passive/noDesktopstart.
ProductionDOM mockedChrome/networkblocked red test shows missing filter/feature ARIA,
keyboard preview and uncaught async UI failures/busy duplication. Fix local callback
feedback only. User authorizes independent neutral webworkbench presentation; no new
dependencies/controller/native lifetime. Synthetic DOM is not installedacceptance.

### Resource cache alias removal correction (2026-10-13)

ABDM Desktop NativeMessaging `BrowserMessageRequest.kt:7-25,49-50` was inspected first for request identity; no equivalent inspected extension cache is claimed. Legacy `D:\Code\NxTools\browser_extension\nxtools-interceptor\background.js::{cacheResource:1665-1741,pruneResources:1743-1756,loadPersistentResources:1783-1820,clearTabResources:3887-3896}` stores canonical IDs and normalized URL aliases in the same per-tab/global maps. Its capacity loop sorts duplicate values and removes only resource.id, leaving URL aliases. Current Next has the same capacity/removal shape; its same-URL merge additionally overwrites the cached stable ID through incoming metadata.

Verified Next chain: request/response/page observations -> cacheResource -> canonical+URL alias indexes -> bounded capacity; pruneResources handles TTL/unusable URLs; loadPersistentResources rebuilds indexes and rewrites sanitized storage; clearTabResources retires tab entries. sendResource/sendResourcesBatch resolve IDs, persist pending identity before dispatch, and resolve the current resource again on matching acceptance. No UI/window/focus/native-show lifecycle changes are part of this cache slice.

Required invariants: capacity counts distinct resource objects, not index keys; removal clears canonical/alias references to that object in both indexes without removing an alias now owned by another object. Same-URL metadata shares original source-priority policy but never transfers stable ID/tab/URL/timestamp/pending/sent identity. Recapture retains the authoritative object and pending ID; late acceptance cannot clear a newer handoff. Restore enforces the same bounded index rules and storage projection still excludes privileged context. Isolated Node production-entry regressions reproduced premature eviction, orphan alias and overwritten ID before the fix; real browser acceptance remains separate.

### Isolated CRX signing identity (2026-10-13)
ABDM Desktop BrowserMessageRequest.kt was inspected first for NativeMessaging request identity; no equivalent CRX signer claimed. Legacy D:\Code\NxTools\Services\BrowserExtensionPackageService.cs::CreateCrx3Bytes303-326,LoadOrCreateSigningKey365-395,ResolveSigningKeyPath398-404 retains CRX3 SHA256/SPKI first16bytes, RSA2048 PKCS1/SHA256, per-user LocalAppData/NxTools/browser_extension_key.pem and existing-key reuse. Verified Next Export -> source resolution -> ZIP -> CRX signer -> PEM, and registrar EnsureTransaction -> command/pinned-manifest/exported-key origin union. Export's working-directory fallback is intentionally absent from registrar source resolution. Firefox manifest ID and registration transaction remain original.

Correction: old temp-source/output test still called production-key Export; a pure reflection red guard failed before introducing explicit ExportWithSigningKey. Tests now pass their own temporary PEM; internal BrowserExtensionSigningIdentity owns only legacy path, SPKI-to-ID and read-existing identity. Export remains sole key creator, registration read-only, missing key contributes no origin, malformed Base64/IO fallback and cryptographic/JSON failure categories unchanged. Fixed public-key vector yields haohhnpjfggpdadhcbjakjpkmkljkpif. Actual temp CRX signature verifies and maps to registration origin; no production PEM/registry/browser/install operations. Source guards protect explicit-key test use and independent source fallbacks. No UI/layout/focus/native-window lifecycle change.

## Installed completion readback / monitor-footer correction (2026-10-13)
ABDM Desktop CompletedDownloadPage/ProgressDownloadPage/ShowDownloadDialogs/CustomWindow and legacy DownloadProgressWindow/DownloadCompletedWindow/MainWindow/WindowAnimationService were rechecked before repair; exact paths/native-boundary availability and stronger success-only Next policy are in [download outcomes](../../.trellis/tasks/10-13-installed-lifecycle-feedback/download-outcomes.md).
Rust artifact_actions already closes matching-success/current HWND/generation only. C# ExplorerShellActivation had a wrong duplicated IDispatch GUID/property-ID/type contract, now removed in favor of existing ExplorerWindowAdapter IExplorerBrowserDispatch (IWebBrowser2,HWND-515/int,LocationURL211/string). Actual reflection metadata red and4safe fake-native fixtures pass; real Explorer open/foreground remains untested, never changed to unconditional click-close.
Released Small Toolbar stays32px, now centered in original44px rail; copy separates engine active_connections from total segment vector, nontransfer0/unknownexplicit. Actual render red catches top-biased controls; final Desktop658 passes. No native window budget inflation or packaged/visual/installed acceptance claim.

## Round-2 Explorer target observation (R5, 2026-10-13)
ABDM was inspected first: desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/CompletedDownloadPage.kt:87-104 delegates open/reveal; shared/utils/src/desktopMain/kotlin/ir/amirab/util/osfileutil/WindowsFileUtils.kt:24-39,57-96 uses native selection/open then one executable fallback. It checks API acceptance, not foreground or selection readback. Legacy D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs:17-35,56-75 derives from Avalonia Window, initializes animation, dispatches owner action then closes; ViewModels/MainWindowViewModel.cs:1583-1616,1636-1658 selects existing files, opens directories and catches launch failures. Approved R5 deliberately strengthens completion to observed target, not acceptance, while making activation best effort.
Verified Next native chain: PlatformPipeClient.cs:688-691 -> WindowsPlatformAdapter.OpenShellTargetAsync:931-1025 (detached STA, correlated stages/terminal/elapsed) -> ClipboardShellTarget.cs:123-133 -> ExplorerShellActivation -> ExplorerNativeFolder.Open/Select -> ShellExecuteW or SHOpenFolderAndSelectItems; relative child PIDL stays borrowed until call return. Existing folder-only FindExplorerWindow and root foreground equality cannot prove requested selection and cannot distinguish Windows11 tabs sharing one root.
Required invariants: observe the active visible Shell view's actual folder and selected exact filesystem item; a matching folder alone never confirms a file. Active-view background dispatch comes from IShellBrowser.QueryActiveShellView -> IShellView.GetItemObject(SVGIO_BACKGROUND,IID_IDispatch), not an arbitrary same-root browser Document/LocationURL. Preserve STA/COM ownership and bounded existing eight-attempt loop/one fallback; no second worker. Target confirmed succeeds independently of foreground denial; missing/wrong/unobserved target remains failure. Correlation/stages remain payload-free; Rust owns originating window identity/close and cancellation. No placement/layout/focus-policy change or live Shell acceptance is implied.

## Explorer reader recurrence (2026-10-13)
Rechecked the same ABDM/legacy action/constructor sources indexed above. Windows SDK ShlDisp.h establishes that active.Folder returns base Folder, while Self belongs to Folder2. Current native chain now uses ExplorerShellActivation.ReadFolderSelf -> explicit IID_Folder2 -> get_Self native slot17 -> actual directory plus selected item paths -> ObserveTarget -> correlated result -> Rust current originating-window close. Owned COM red reproduced missing Self; exact Explorer13 fixtures and real GPUI completion action/result lifecycle pass. Launch acceptance remains insufficient; no unconditional click-close, live Explorer enumeration or installed/UNC/Windows11-tab acceptance was performed. Exact evidence: .trellis/tasks/10-13-explorer-theme-recurrence/native-outcomes.md.

## Installed Reveal observation follow-up (2026-10-14)
ABDM WindowsFileUtils.kt:24-39,57-96 was inspected first, then legacy DownloadCompletedWindow.axaml.cs:10-35,56-75, MainWindow.axaml.cs:2193-2226 and WindowAnimationService.cs:13-44, followed by MainWindowViewModel.cs:1583-1616,1636-1658. The constructor attaches Avalonia Opened/Closing animation; keyed completion presentation calls ShowIndependentSecondaryWindow; owner action launches selection/directory before animated Close. Avalonia's actual native Window implementation is not checked into the legacy tree. Next retains stronger exact-observed-target/current-HWND/generation success-only close, detached STA and independent best-effort foreground.
Newly reviewed omission seam: ExplorerShellActivation.NativeTargetObservationPlatform.ReadActiveViews consumes IWebBrowser2.HWND, tests its class as a top-level Explorer root, then queries IServiceProvider -> IShellBrowser -> active IShellView -> GetWindow/visible-root check -> background automation. A browser child HWND must not be confused with its CabinetWClass root; hidden/inactive views still cannot count. Passive existing-window enumeration in the available session returned ShellWindows count0 and Explorer/tab class counts0; this is not evidence about the user's installed Explorer window. Owned COM/native-chain regression and limits are recorded in .trellis/tasks/10-14-installed-feedback-actions/explorer-outcomes.md; no launch acceptance or unconditional close is permitted.
Native gate regression reproduced the omission with owned IWebBrowser2 IDispatch HWND readback, IServiceProvider/IShellBrowser/IShellView COM vtables and actual self-owned CabinetWClass -> ShellTabWindowClass -> visible view HWNDs. Top-level input reached GetItemObject; child input stopped before QueryService with no HRESULT. Reader now normalizes IWebBrowser2.HWND via GetAncestor(GA_ROOT) before the class gate and preserves active-view visibility/root and exact directory/selected-item checks. Both inputs now reach the deliberately failing GetItemObject boundary with preserved E_ACCESSDENIED and balanced references; hidden/wrong-root views still fail closed. Fixed observation-stage vocabulary is emitted once per command per stage, without payloads. This proves the native child-handle gate defect, not that the unavailable installed user window returned that handle. Further verified legacy show boundary: MainWindow.axaml.cs:2844-2884 -> window.Show():2867, topmost-only passive retention:2663-2666; Avalonia native implementation remains unavailable. Exact safe Explorer13 subset passed, no packaging/live Reveal.

## Contextual extension submenu correction (2026-10-15)
Reread ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/DownloadList.kt` selection/context owners and old NxTools `Views/MainWindow.axaml.cs:1335–1345` SelectTaskForContextMenu -> BuildTaskContextMenu.Open, `:1452–1488` normal file/queue/transfer/copy groups and real submenus. User explicitly requests extension actions after normal commands and cascading rather than a modal chooser; no such legacy plugin menu is claimed. Next keeps selected task IDs/capability plan/original engine/native callbacks, adds official KitPopupMenu child entity after normal operations/before destructive group, actual leaf -> current ExtensionApplication::start_manual, no chooser/input guessing. Actual GPUI regression checks order, right-expand/click, no chooser and fixed service-unavailable failure without running a fixture executable. No download engine/resolution/receipt/network policy changed; [UX outcomes](../../.trellis/tasks/10-15-extension-action-feedback/ux-outcomes.md).

## Selected-download toolbar alignment / shared outcomes (2026-10-16)
Reread ABDM DownloadList.kt selection/context ownership first, then legacy MainWindow.axaml.cs1335–1345 SelectTaskForContextMenu/BuildTaskContextMenu and existing normal command topology. User explicitly requests centered multi-selection content; no legacy centered-toolbar equivalent is claimed. Next retains selected stable IDs/capability subset/original engine and queue callbacks; official Toolbar.content/ToolbarGroup centers intrinsic summary+eligible actions across the full rail with clear independently right. Actual Small24 buttons require24px wrappers: historical28px wrappers left4px trailing slack even when the group container was centered. Real production geometry red/green and original batch/clear dispatch protect the correction. Extension launch/terminal reporting now uses the shared notification system, not automatic failure/report Dialogs; service-unavailable fixture verifies error notification plus0runs. [Outcomes](../../.trellis/tasks/10-16-menu-notification-refinement/outcomes.md).

## 2026-10-22 user-authorized probe latency diagnosis (before repair)
This slice explicitly excludes re-reading old NxTools downloads; ABDM HttpDownloaderClient::test -> head(credentials,0,255) -> OkHttpHttpDownloaderClient::actualHead actually uses GET (head() commented) then bounded nonrange fallback. All requests use applyProxy; ProxyManager strategy/SystemProxySelector provider owns direct/system/manual. FluxDown meta_prober bounded8s HEAD preserves requestcontext; auto_proxy is separate direct-first sampling/hotroute behavior, NOT adopted and noAGPLcodecopied.
Next chain DownloadApplication/NewDownloadView -> DownloadEngine probe/resolveroperation -> sharedNetworkPolicy immutable snapshot/clientcachegeneration+30sTTL -> HttpDownloadClient::probe_timed HEAD+Range -> metadata/manifest -> correlatedUI/nativeconfirmation. Local headslow fixture returns usableRange immediately/HEAD1s; added timing assertion red measured1.003s because tokio::join awaits both. Existing stage timings distinguish head/range/fallback/manifest, no payload logs. Approved optimization: race both, then bounded companion metadata grace after first usableheaders, preserve timely HEAD-onlyfilename and validRange evidence; failed first response must still await second/fullfallback. Late cancelled companion cannot mutate accepted request/name or prove resume. Actualexternalresource/systemWindowsproxy symptom not yet reproduced; no TLSbypass/directfallback/PAC/autoswitch. Child10-22-download-probe-latency records exactmeasurements/limits.

##2026-10-22 probe latency boundary closeout
Verified local1sslowHEAD red1.0030359s; usableRange/HEAD path now350mscompaniongrace, with delayed200msHEAD-onlyfilename retained and skippedRange never claimingresume. Empty200HEAD regression initiallylost16bytelength; `usable_probe_response` now requireslength/validRange/recognizedmedia before boundedcompanionwait, otherwiseawaitother. No blindfirstsuccesswinner/latebackgroundmetadata oruserfilenameoverwrite. SlowHLS/DASHbodydeadline andactualHTTPstagecancellation use deterministiclocalhostfixtures. HTTP26,resolution15,Downloadlib117,serialengine103 allpass; generation/route/certificate/30sclientcachepolicy unchanged/tested.
Network3 actualrequesttests verifyDirectoriginform, injectedSystemabsoluteform andManualproxyreceipt, plusimmutablesnapshot. Systemtestinjectsclientresolver/proxyonly, notWindowsregistry/PAC; no systemsettingsread/write. ABDMwasinspectedfirst; FluxDownbehavioronly/noAGPLimplementationcopy; oldNxToolsdownloadsourcewasnotrevisitedinthisslice. NoexternalURLprovided, soexternalresource-discovery/proxycause remainsunreproduced. Existingcorrelatedstageelapsed/route/skipflagsnarrowdiagnostics remainpayloadfree; nobrowser/resolver/globaltimeoutrewrite.

## Filename follow-up prerequisite (2026-10-22)
User now explicitly requests oldNxToolsfilenamecomparison, superseding priorno-old-download-reread constraint forthisnarrowtopic. ABDMHttpResponseInfo61–76/fileNameExtractor7–29 inspectedfirst(header->URL,plainpercent/MIMEdecode), thenoldDownloadManagerServiceBuildProbeResult1139–1172,ResolveTaskFileName3972–4022,EnsureFileNameExtension4058–4072. Oldservername canreplaceURLauto/nonrealextensionlabel butnotgenuinecustomname; currentNewDownloadView784–793mostlykeepsnonemptyrequest andRustHTTP2250–2260plainparameterraw. Browserold/currentcentralnamefunctionsequivalent; no browserrewritewithoutred. RegressionsmustuseactuallocalHTTP/confirmation andprotectlateuseredit/generation/context/cancel; existing350msmetadata graceretained. Externalusercaseunprovenwithoutsample.

Current follow-up naming repair: bounded realHTTP plainfilename red->green uses existing strict percentUTF8 decoder once, malformed rawfallback/unsafe Windowsname validation retained;8header cases pass. Actual NewDownloadView realprobe/fixedhandoff protects usefulserver-vsURLauto/label/custom policy. New `file_name_edit_generation` increments on original InputEvent::Change, captured at probe start and checked before suggestion: current-value comparison alone failed editA->B->A regression (server-package.zip overwrote url.bin). Actual5probe cases now pass, including lateedit and ABA. Official set_value is silent hydration; replace_all/useredit emits Change. No new input/probe/controller, context/TLS/route/350msgrace or browser production change; diagnostics remain fixed source/correlation/generation only.

## Recurrent download Reveal and compact chooser prerequisite (2026-10-22)
ABDM CompletedDownloadPage.kt::Actions87–104 and WindowsFileUtils.kt24–39,57–96 reread first; legacy DownloadCompletedWindow constructor17–35/OpenButton/OpenFolder56–75 and MainWindowViewModel.OpenTaskFolder/OpenPathInExplorer1583–1658 reread second. Existing stronger Next observed-target/success-only keyed close remains authoritative. New user request changes chooser presentation only: icon-only official Small outline Button immediately after OpenFile, Tooltip/accessibility retains purpose; existing DialogFooter has no Toolbar.child adaptation. Keep440x184 client and all actions.
Current common chain: task_window.rs Reveal and main_window/download_page.rs context action -> artifact_actions.rs pending-before-dispatch OpenShellTarget SelectInExplorer -> detached C# ShellTarget -> ExplorerShellActivation -> NativeTargetObservationPlatform.ReadActiveView -> original correlated result/current-window close. Directory Reveal matcher now requires selected directory in parent, but native reader still returns early with empty selection for isDirectory. Add real successful reader regression before repair; prior owned COM test deliberately stops at GetItemObject failure and did not exercise successful selection. File requests need their own success/equivalent-path controls, not inference from a directory-only defect. No process-launch-only success, global timeout increase, new worker, native liveShell launch, framework or download-engine changes.
Executed recurrence reds before fixes: exact nativeCOM fixture fails selected-path retention for directoryReveal; matcher fixture fails extendedDOS/ordinaryShellpath equivalence. UI actualgeometry red after adding observation-onlyselector is60x24 versus24x24, not merely missingselector. Reader now collects selectedpaths for bothkinds; comparison canonicalizes only explicit extendedDOS/UNC aliases on target/folder/selection, notdevicenames orbasenamefuzzy. C#10exactreviewedfixtures pass at firstrepaircheckpoint; UNC/wrongname negative controls added for finalcheck. Rust has transient peerIPC serde_json compilefailure recorded, now sourcechangedbyitsowner; nopeerdependencyeditbythisslice. Installedusercase remains separate from these syntheticcausalrepairs.

## 2026-10-24 shared Reveal replacement-host recurrence
ABDM/legacy references and exact-observed-target/current-window-close invariants remain as indexed above; no download-engine/caller semantics change. Initial .NET8probe found service-only ShellWindows objects, but QueryService failed and no Explorer folder root existed: this is not sufficient evidence of an Explorer-version defect. Separately authorized synthetic-file Reveal actually opened Directory Opus13.22. Explorer-only observation explains accepted native request followed by target_not_confirmed despite visible file-manager success. Rechecked current DirectoryOpusFileManagerOperations and indexed SwiftList DOpus sources; official DOpusRT /info paths/listsel API and actual exported HWND/tab/path/selection structure are recorded in quick-launch-research.md#2026-10-24-Reveal-replacement-host-recurrence. New shared native observer validates same-session/PID/starttime dopus.lister, visible current source tab, exact parent/selected filesystem item and fresh path revalidation. No raw payload diagnostics, caller toast masking, mutation commands or increased8attempt/75ms/onefallback. Provider-only Explorer COM test is separate hardening, not actual DOpus acceptance. Native product-entry test now confirms owned scratch file in228ms; separate actual observer accepts exact selection and rejects unselected same-directory synthetic file. Full runtime checks and remaining visual/native limits belong to task10-24-runtime-feedback-reveal-ui.

## Chrome Web Store upload/identity prerequisite (2026-10-23)
User explicitly limits repair to key-free uploadZIP and exactstoreID NativeMessaging matching; initialuploadfailsbeforestoreIDexists. Reread ABDM BrowserMessageRequest.kt(identity only,noequivalentCRXsigner), thenlegacy BrowserExtensionPackageService.cs ExportCrx/ExportFirefox/CreateCrx3Bytes303–326/LoadOrCreateSigningKey365–404. Preservelocalsigning/sourceidentity andFirefox. Next ExportBrowserExtension stringformat -> C#adapter/dialog -> ExportWithSigningKey -> ZIP/CRX currentlytreatsnonfirefoxasCRX. Add additivechrome_store ZIP path that removesonlymanifestkey in copy,neverreads/writesPEM ormutatessource. Existing Settings Page/Group customcontrol32DIPInput/Smalloutlinedexportaction, originalSaveCancel/fieldmerge; noToolbar.childadaptation.
Registrationchain: CoreoptionalstoreID -> settingsdraftvalidation/durableDownloadSettingsmerge -> existingEnsureBrowserNativeMessaging configuredID-orlocal -> BrowserNativeMessagingRegistrar command/pinnedmanifest/existingexportedPEM exactunion -> persistedchromiumhostmanifest -> BrowserHost registered_allowed_origins/validate_browser_origin. No IPCnewfield,wildcard,trustpageautoID orsecondregistrationowner. Saved!=nativeapplied; nativefailure/retryguidance required. Existing correlated registration/export events remain fixed/payloadfree. Tests useexplicittempkey/fakeIPC/tempconfigonly,noregistry/browser/productionPEM. ActualstoreID unavailableuntilfirstupload; optionalemptyisvalid anddoesnotblockZIP.

### Published Chrome store identity amendment (2026-10-28)
User supplied the published Chrome entry `eldfplghejhopcmgfkacimanhaojakph` and explicitly approved removing the manual ID input. Rechecked ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/BrowserMessageRequest.kt` (request correlation only, not CRX signing), then legacy `D:\Code\NxTools\Services\BrowserExtensionPackageService.cs::{CreateCrx3Bytes,LoadOrCreateSigningKey,ResolveSigningKeyPath}`. Local per-user PEM reuse, source manifest key and Firefox identity remain unchanged. Next `DownloadSettingsView::new/current_draft/render_setting_control` removes only the ID Input/entity, preserves the legacy serialized value as non-editable draft data, and adds a direct official Small outline Chrome Button using the same Settings/Page/Group/Item slots as Firefox (no Toolbar.child transformation).
Existing `request_browser_registration` sends the published identity when legacy configuration is empty; `BrowserNativeMessagingRegistrar::ChromiumAllowedOrigins` always unions the exact published origin with command/pinned-source/existing-exported-key origins, deduplicating without wildcard or PEM creation. This also keeps store installs working when a valid old custom value remains. Browser-store actions share the original pending command/result/disconnect/deadline owner and correlated fixed diagnostics; no extra controller, URL/ID logs, new IPC or native registry execution in tests. Original settings Save/Cancel, deferred cross-entity callbacks, native window placement/focus/show/hide and registration failure guidance are unchanged. Tests cover production reconnect for missing/empty/normalized-invalid config, actual settings no-ID render and hidden-value Save/Cancel, original Shell dispatch and isolated exact-origin union; live store/browser/native acceptance remains separate.

## Audit #1 default Enter / schedule child-layer prerequisite (2026-10-24)
ABDM inspected first: desktop AddDownloadPage.kt::AddDownloadPage/MainActionButtons
(URL/category/location/name, separate Add/Download/Cancel callbacks, nested queue/extra
configuration); desktop app-utils OptionsDialog.kt::BaseOptionDialog -> Compose
DialogWindow visible/modeless/onCloseRequest, focus-loss dismissal and listener disposal.
Then legacy Views/AddDownloadWindow.axaml URL ParseUrlCommand and confirmed
AddOnlyCommand/DownloadNowCommand/CloseCommand, .axaml.cs constructor ->
WindowAnimationService.Attach -> OnOpened/OnClosed -> deferred guarded Window.Close.
No window-wide Enter handler was found in those legacy entry files. Avalonia/AWT
native base implementations are unavailable, not inspected native show/hide evidence.
Existing one-time schedule source remains MainWindowViewModel::TryResolveSchedule,
as indexed above; the Next anchored draft/commit interaction is retained, not redesigned.

Exact current released boundary: gpui-pre0.3.9 window.rs::dispatch_key_event calls
window keystroke interceptors before focused actions; KeystrokeEvent has no held flag.
Base0.8.0 input/base/state.rs::enter propagates single-line Enter; div::capture_key_down
receives actual KeyDownEvent with held/character preference after unconsumed actions.
Component0.8.0 Popover delegates to BasePopover; Select::escape/toggle_menu and
Button's Base focus/activation retain their own single keyboard owner.
Before this repair NewDownloadView::new intercepted Enter without schedule/focus/IME guards,
while request reads only committed_schedule and schedule Confirm alone commits the draft.
Required repair: default Enter only on original outer URL/directory/file-name InputState,
no active category/schedule child layer or IME/character preference/modifier/repeat/busy/
native-confirmation gate. Official controls get first refusal; no global Enter swallowing.
Keep original begin_probe/submit/deferred application effects, fixed client/hidden target/
source generation/native show and cancel boundaries. No schedule/text/path payload logging.
Production keyboard red/green and separate acceptance limits belong to task
`.trellis/tasks/10-24-download-enter-popover/`; no other audit item is authorized here.
Repair evidence: production keyboard red1/6 -> green8/8; root default Enter now accepts
only original outer Inputs after official actions, with IME/modifier/held/busy/native
and child-layer guards. Category-confirm Enter is marked owned before focus return,
preventing same-key fallthrough. Original request/submit/probe/handoff bodies unchanged.
Existing schedule lifecycle fixture now actually scrolls clipped late-week dates into
its124px viewport before pointer clicks; no production calendar/layout change. Downloads
168/168/strictClippy/workspacecheck/fmt pass. FullDesktop1158pass/2fail in Archive password
and Extension settings owners; no unrelated repair. No physicalIME/native/DPI/useracceptance.
Exact own-delta, red/green/failed-run evidence and stop condition are in task outcomes.md.

## Audit #9 directory-picker ownership prerequisite (2026-10-25)
ABDM inspected first: desktop pages/addDownload/shared/LocationTextField.kt46–83
binds native picker callback to that field's setText; desktop ui/util/FilePickerUtils.kt
rememberMyDirectoryPickerLauncher -> FileKit rememberDirectoryPickerLauncher with
FileKitDialogParent.awt(LocalWindow.current). FileKit/AWT native internals unavailable.
Then legacy Views/AddDownloadWindow.axaml.cs constructor -> WindowAnimationService.Attach
-> OnOpened/OnClosed/DetachViewModel; BrowseFolderButton_Click captures the current
AddDownloadDialogViewModel before awaiting StorageProvider.OpenFolderPickerAsync and
updates that captured owner, not all windows of a type. SettingsWindow constructor /
OnOpened / explicit Apply or Cancel / OnClosed and MainWindow.ShowSettingsWindow's
keyed ShowIndependentSecondaryWindow preserve presentation/draft lifetime. Avalonia
native base source is unavailable, not claimed as native show/hide evidence.

Next read-only native boundary: WindowsPlatformAdapter.PickDirectory -> DirectoryPicker.Pick
-> StaDialogRunner.Run -> FolderBrowserDialog.ShowDialog(NativeForegroundWindow), result
SelectedPath or empty cancellation. Existing Windows adapter/STA/native modal ownership
and cancellation capability remain unchanged; Rust logical retirement never kills the host.
Before this repair DownloadApplication.pending_directory_picker stores only ID+target type;
handle_directory_result broadcasts DirectoryPicked to every same-type subscriber. Type
separation is insufficient for close/reopen, fixed-stage replacement and late draft edits.
Exact Base0.8.0 InputState.set_value is silent hydration and clears history; replace_all/
real edits emit Change. pre0.3.9 App.flush_effects processes queued Emit/Defer in FIFO,
so final application must allow already-queued Change events to settle and recheck the
original weak view/window, operation and edit revision (including A->B->A), not just text.
Preserve one existing pending policy, original outlined Button/Input/Settings/Form geometry,
save_directory_explicit, validation clearing and explicit Apply/Cancel persistence. No
protocol/native/engine/vendor/shared-control change; accepted audit#1 keyboard handler frozen.
Required regression/evidence and final stop gate belong only to task
`.trellis/tasks/10-25-download-picker-identity/`; no subsequent audit item is authorized.
Repair delivery: original view+window+UUID+directory Change revision now replace type
broadcast; one application pending is installed before caller-ID/correlation dispatch.
Initial and final result application validate the original owner independently; two
deferred turns allow same-update A->B->A Change effects to settle before the last check.
Close/removal/fixed-stage replacement/disconnect retire only logical ownership; original
C#picker, accepted#1keyboard,submit/probe/hydrate/handoff bodies and styled geometry remain.
RealButton/Input/typedResult/tempconfig regressions: before repair1pass/4fail; final
14picker+8keyboard22/22, Downloads182/182, strictClippy/workspacecheck/fmt/source gates pass.
Single fullDesktop1173pass/1fail: originalArchiveUnlockfixture remains unresolved; no
unrelated owner repair/full-green claim. Synthetic only, no native/physical/DPI/useracceptance.
Exact own-deltas, failed attempts and acceptance map: task outcomes.md; stop for parent review.

## Audit #2 sortable header keyboard equivalence prerequisite (2026-10-25)
ABDM checked first: shared/app/src/desktopMain/.../table/customtable/TableUtils.kt::
MaybeSortableCell178–203 and Table.kt sortable indicator359–375 reverse the active key,
otherwise initialize the default; original stable-ID table/selection and presentation-only
queue boundary remain. Legacy next: Views/MainWindow.axaml1101–1120 uses actual Buttons
bound to SortByCommand; MainWindowViewModel.SortBy1817–1831 reverses same key or chooses
Name/Status ascending, NotifySortHeaders4616–4635 updates arrow, SortTasks4593–4614 only
projects order. No queue/transfer/native show/hide behavior change is part of this repair.
Next download_sort_header_cell pointer path -> set_download_sort(key,true) -> scroll/persist
is original owner; its ordinary div lacks Tab/keyboard equivalence. Keep menu key,false
default-direction semantics. Exact Component0.8.0 Button ghost/xsmall/compact, public child
content/accessibility_label, keyed focus/Tab and original release-click provide the missing
header capability. Keep original column geometry/inset/text/arrow and readable current sort
state; no second dispatcher, table replacement, engine/queue/selection/detail/OLE/native work.
Task .trellis/tasks/10-25-download-sort-keyboard/ records actual MainWindow red-green,
temporary-config restart and interaction checks; compiled snapshots are not native/reader/
physical/DPI/visual/user acceptance.

## Audit #5 daily-schedule default-submit prerequisite (2026-10-25)

ABDM first: desktop QueueInfoComponent.kt::createConfigurableList148–155/247–301 owns
independent start/end schedule flags and nonempty weekdays. app-utils OptionsDialog.kt::
BaseOptionDialog delegates to Compose DialogWindow visible/modeless/close request, disposes
its focus-loss listener; AWT/Compose native internals are not inspected. Legacy next:
SettingsWindow.axaml::SchedulerSection900 and .axaml.cs constructor15 -> animation attach18
-> OnOpened21/ConfigureForCurrentScope -> SaveButton_Click68/ApplySettingsCommand -> Close;
OnClosed185 cancels through the original command. WindowAnimationService.Attach13–49 owns
guarded delayed close; Avalonia native base is unavailable. These are schedule/close owner
references, not proof of a window-wide default Enter implementation.

Current Next DownloadScheduleView::save92 -> deferred DownloadApplication::
set_default_queue_schedule -> original DownloadQueueAction::SetSchedule/engine store -> refresh
and original close_schedule_window; it is not a new AppConfig write. TimeField/TimeFieldState
are current production controls, not the old raw-HH:MM Input noted in historical migration.
Exact Base0.8.0 time_field.rs keybindings23–30 own digit/arrow/segment/Tab editing and have no
Enter action; on_key_down418 handles only unmodified one-character edits. Default Apply may
therefore handle an otherwise unconsumed plain Enter only on an enabled focused TimeField,
with held/modifier/character/busy/Dialog guards and the original save/defer owner. Other
Buttons and child layers keep their own action/release semantics; no shared keyboard-helper,
engine/persistence/native/geometry change. Preserve 430x326 fixed client, original input state,
validation/cancel/close and prior audit#1/#9 behavior. New red/green evidence belongs only to
`.trellis/tasks/10-25-default-submit-keyboard/`, not native/physical/reader/user acceptance.

## Audit #6 ordinary rem-role prerequisite (2026-10-25)

ABDM first: QueueInfoComponent.kt::createConfigurableList148–155/247–331 preserves
weekdays followed by independent start/stop configuration and enabled-only time editing.
Table.kt::Table159–183 uses the same keyed LazyColumn render for layout/scroll and fixed
cell-width alignment with its header. OptionsDialog.kt::BaseOptionDialog delegates native
visible/modeless/close and UiScaledContent; AWT/Compose native internals were not inspected.
Legacy next: SettingsWindow.axaml::SchedulerSection900–927 and footer999–1006 retain the
policy/explanation and explicit Cancel/Save. Its .axaml.cs constructor/OnOpened/OnClosed and
WindowAnimationService.Attach close chain remain as recorded above; Avalonia native base
is unavailable. This reference check does not authorize a new native/window lifecycle.

Next open_schedule_window623–699 retains keyed activation,430x326 default/minimum,
normal client-decorated show, production Root and deferred close-identity cleanup.
Component0.8.0 root.rs::WindowState::prepare applies Theme.font_size to window rem; actual
product-theme/Root regression confirms the default16px design base. Form::label_width takes
Pixels, so resolve the ordinary relative label rail only at this existing layout boundary;
its controls remain actual Small outlined Toggle/default Switch/default H23 minute TimeField.
GPUI uniform_list.rs::measure_item invokes the same task-row callback on every measurement;
row rendering and actual virtual stride must use one relative role, not a second calculator.
Current ordinary row/header layout ratios can use GPUI scale builders or metrics::ordinary_rem
against the fixed16 design base. Preserve bitmap20/native fixed clients/titlebar/footer/
physical edges and all original stable-ID selection/context/OLE/sort callbacks. Exact current
Switch and XSmall Progress internals remain released fixed-pixel tracks, not custom substitutes.
Schedule validation keeps its original two-line viewport budget with full text scrollable;
Tooltip/copy and original submit/save/cancel/failure ownership remain. Evidence belongs only to
`.trellis/tasks/10-25-rem-role-consistency/`; controlled14/16/18 base-font geometry/paint is not
native/DPI/reader/visual/user acceptance or a claim that all ordinary px consumers are repaired.

## Audit #6 continuation: New Download ordinary rem prerequisite (2026-10-25)

ABDM first: AddDownloadPage.kt::AddDownloadPage57–193 keeps dominant URL, weighted left
category/location/name and intrinsic metadata/right actions, with MainActionButtons after
the flexible spacer. LocationTextField.kt137–209 bounds long location choices with vertical
scrolling; its original component owns values. Legacy AddDownloadWindow.axaml232–387 uses
one ScrollViewer around URL/parsed stages and original ViewModel commands. Its .axaml.cs
constructor20 attaches WindowAnimationService, OnOpened27 adapts native chrome, DataContext
subscribes RequestClose/PropertyChanged, CloseFromViewModel173 calls Close, and OnClosed34
detaches/disposes. WindowAnimationService.Attach13 guards delayed Close; AWT/Avalonia native
framework implementation was not inspected and is not inferred from these managed sources.

Next browser_handoff_impl.rs::open_new_download_window_with_request437 -> original options/
NewDownloadView/Root keeps native show and focus; create_fixed_confirmation hydrates fresh
window-bound inputs in a hidden target, then original native apply/commit/old-close owner.
Current new_download_window_size(true) computes246px from34+44+16+32+8+112, not historical278.
All size/error/media functions and native placement/readback/generation policy remain fixed.
Ordinary body/label/guidance/inspector/category/calendar/media ratios may follow verified16px
Root base through existing GPUI scale or metrics::ordinary_rem. Exact Component0.8.0 Form
label_width accepts Pixels resolved at Window boundary; Select.menu_width accepts Length;
Popover p_3 is rem-relative before caller style. Preserve outer schedule304x200/calendar124
and category320x240 budgets,34caption/44footer, original semantic controls and all callbacks.
Shared path_picker_shell returns erased AnyElement with fixed32px layout; adapt only the
original New Download public Input/Button layout locally, not shared unrelated consumers.
No second keyboard/action/scroll controller or vendor patch is authorized. Test compact and
confirmed production Root at14/16/18, true scroll-end/footer/hit bounds and child Select/
Popover keyboard ownership; base-font fixtures do not establish native/DPI/visual acceptance.
This continuation has separate evidence in `.trellis/tasks/10-25-rem-new-download/`; the prior
accepted bounded slice and failed complete Desktop run remain immutable history.

## Audit #6 Queue04: basic New Download relative roles (2026-10-26)

The bounded basic slice retains Component0.8.0 Small Form/Field, Medium/default Input and
outlined paste/directory Buttons, plus original footer variants and callbacks. Native compact
560x176 and ordinary confirmed560x246 remain fixed (34caption/44footer); the separate278media
base and all error/media sizing and hidden-target native handoff functions are unchanged.
Ordinary16/8body insets,72label rail,32control composition,12/18guidance and inspector text,
13/14vector sizes,120x112ordinary inspector and command gaps now follow the accepted16px
design base through metrics::ordinary_rem or GPUI scale builders. Form.label_width resolves
Rems to Pixels at the Window boundary. Released Small Form/Field internal6/4pixel gaps and
inherited label line metrics remain official; no vendor/theme/font override is added.

One bounded body scrollbar retains intrinsic form/error height. Compact fallback error wraps
instead of permanently truncating one line; expanded/confirmed errors retain the original
official Alert through a nonshrinking local layout slot. Actual40-line synthetic error wheel
scroll reaches its full end while the fixed footer remains available. Long original Inputs
retain their complete values/entities and accept actual pointer focus,End and tail editing.
Ordinary path-picker rails are local composition only; media uses the original shared helper,
and media inspector pixel roles remain for the next slice. Category/calendar/Popover/media
option internals and all original input/probe/picker/submit/deferred callbacks are preserved.

Evidence lives in `.trellis/tasks/10-26-rem-new-download-basic/`: actual production Root16/14/18,
paint-time inherited type,default density,scroll-end,public focus sequence and real Cancel
pointer checks. Controlled base-font variation is not Windows DPI; source/compiled fixtures
do not establish physical/native/IME/reader/visual/user or installed-release acceptance.
The earlier compile-only checkpoints and full Desktop failure remain immutable history;
this bounded slice does not close original audit#6 or authorize a full Desktop/package run.

## Audit #6 Queue05: New Download overlays and media rem roles (2026-10-26)

ABDM first recheck: CategorySelect.kt::CategorySelect/RenderCategory keeps one bounded
dropdown,original close/selection and an intrinsic icon/name row; AddDownloadPage.kt keeps
the category/location/name column beside intrinsic metadata and the original footer. Legacy
AddDownloadWindow.axaml232–387 retains bounded HLS resource lists and complete metadata in
Tooltip,original commands and ScrollViewer. Its constructor/Attach/OnOpened/RequestClose/
OnClosed cleanup and WindowAnimationService guarded delayed Close remain the lifecycle
reference. Neither managed source establishes its unavailable framework-native show/close.

The current bounded slice retains official0.8.0 Popover,ListItem,Input,Small Select,compact
ghost date Button and RadioGroup/Radio with original callbacks/owners. Category34px two-line
roles12/18+11/16 and search gap now follow ordinary_rem; fixed320x240 minus actual released
Popover/ListItem rem padding determines content width. Existing search/highlight/ScrollHandle/
Enter confirmation/focus-return and full name+path Tooltip remain. Schedule304x200 retains
the same calendar scrollowner,up to124px; rem-relative header/date/type and66px time rails
yield within that budget to nonshrinking original actions. Seven ideal40px date columns may
shrink within available width; default16 remains40x19. Confirmation,clear and overlay cancel
remain local/deferred,never outer submit. No Calendar/Select controller was replaced.

Media inspector and ordinary HLS/DASH headings,gaps and minimum40px Radio rows follow rem.
Long Chinese track metadata wraps inside the original official Radio content instead of
being permanently single-line clipped. Native budgets are still compact176,ordinary246,
media base278,HLS one/many338/418,DASH353/433 and original38px error reserve. List viewports
remain at most120px. Exact released scroll/scrollable.rs::render/root_style_from retains a
caller's max_size on content while copying it to the outer wrapper; using that cap directly
on a long RadioGroup can leave the actual wheel range unusable. Public Scrollable::id is
required when the same callsite creates two independent scrollables. One stateless
NewDownloadMediaViewport resolves ordinary natural height at the existing Window/rem boundary,
caps its outer layout before parent flow,and hosts the original Scrollable with a stable
per-list ID. No new state/ScrollHandle/scroll controller or vendor patch. Original RadioGroup
gap remains released12px/rem; required HLS/video and nullable repeat-to-clear DASH audio
callbacks/typed request owners are unchanged.

Production Root16/14/18 tests prove actual category wheel-to-last/full-path Tooltip/search/
Enter,calendar wheel-to-last-date and pointer selection,real Tab to original actions,time
Select confirmation,Enter confirm/Space clear/Escape cancel and child-no-outer-submit.
Long HLS/DASH metadata scrolls to its true end and pointer selects the original final item;
after real Input focus,Tab reaches the selected audio Radio and Space release clears once.
The original media error geometry remains visible at456px after a red-capable regression
caught an intermediate oversized parent flow. No prior geometry assertion was weakened.
Source/compiled fixtures are not physical keyboard,WindowsIME,DPI/reader/visual/user/release
acceptance. Exact reds,intermediate failures,dirty deltas and protection evidence live in
`.trellis/tasks/10-26-rem-new-download-overlays/`; Queue04 and earlier full-suite failure
remain immutable. Original audit#6 overall remains open for later ordinary-role consumers.

## Audit #6 Queue06: Settings/task ordinary-role boundary (2026-10-26)

Reference order rechecked: ABDM DesktopSettingsComponent.kt/SettingsPage selected groups,
ShowDownloadDialogs keyed task/phase windows and CommonContent/close,ProgressDownloadPage
bounded tab/detail scrollers,CompletedDownloadPage name/icon/footer actions; then legacy
SettingsWindow constructor->WindowAnimationService.Attach->OnOpened/ConfigureForCurrentScope,
SaveButton_Click ApplySettingsCommand and close-only cancellation. DownloadProgressWindow
constructor subscribes task/completion options,OnOpened syncs,OnClosed detaches; completion
switches content/footer and calls ResizePreservingCenter. WindowAnimationService guards
delayed Close. AWT/Avalonia native bases remain unavailable,not inferred or operated.

Current ordinary Settings input/label/control rails,category row minimum/heading,spacing and
workbench type use accepted ordinary_rem plus public semantic control tiers. Local
download_settings_input accepts Length,retaining original Input entity and field_shell.
The original Settings/Page/Group/Item search/navigation/scroll owns the page; category editor
may scroll inside its existing310px area. Header Textarea120px,error96px,984x650 native
options/minimum,title/footer,taskbar preview/native/device/hit geometry remain fixed.
Actual constrained720x480 tests do not change native minimum. Original draft parsing,picker
identity,Apply/Cancel,deferred section update/rollback/error Notification owner are unchanged.

Task ordinary content insets,gaps,vector badge/icons,identity/name/metric/capability lines and
DescriptionList label rail follow rem. Original official Progress XSmall owns its tier;
semantic Table rows follow the same ordinary role while columns retain original40/88/140/140
proportions inside actual available width. No task data/virtualization/IO/copy ownership
change. Width440,collapsedprogress/completed184,failed196,expanded280,title34/footer44 and
phase/retry/reset/close/native handoff functions remain fixed. Content scrolls within those
clients; failure's original36..40px Alert viewport remains; complete failure text and long
path/name remain reachable through original scrolling or bounded Tooltip. Required footer
actions stay outside content scroll. Added path Tooltip/semantic labels retain full values,
not a new Shell or native action owner. Task snapshot/display-sampler/segment-state semantics
and every original pointer/key/deferred callback remain unchanged.

Queue06 evidence is `.trellis/tasks/10-26-rem-download-settings-task/`: effective Root14 red
showed88px input rail beside28px control and fixed16px metric line; new Root16/14/18 fixtures
exercise long category/error/default+constrained Settings,actual category pointer/Tab/Space,
original Apply/Cancel with temp config section merge,and task details last segment/phase
long text/close/cancel. Official Scrollable debug selectors can describe content rather than
its clipping viewport; real hit assertions use existing outer budget/action bounds. A
pointer click on released Button may not initialize keyboard focus; fixture initializes the
public focus sequence before testing actual Tab/KeyUp,not a production focus workaround.
Core's existing Clipboard normalize forces enabled=true; section-merge evidence uses a
valid unrelated max_items value,not a forbidden legacy flag. Native/DPI/physical/WindowsIME/
reader/visual/user/release acceptance is separate; earlier failures/acceptances are immutable.

## Audit #8 command-result naming follow-up (2026-10-26)
Actual MainWindow header Buttons defer to open_settings_window/open_schedule_window/
open_new_download_window; task-menu request_task_window opens its original keyed surface.
These follow-up commands now use one U+2026, with icon Tooltip/a11y sharing the same full purpose.
Settings directory/FFmpeg and NewDownload directory Buttons retain their original picker/ID/
focus/deferred owners and complete object names, synchronizing only the ellipsis. Settings
browser export Buttons -> request_browser_extension_export -> ExportBrowserExtension with
empty destination -> WindowsPlatformAdapter.ExportBrowserExtension ->
BrowserExtensionExportDialog.PickDestination -> StaDialogRunner -> SaveFileDialog.ShowDialog.
Only native cancellation returns success+empty data before package export. The original
Rust pending/result owner and C# export/signing/picker code are unchanged; no native dialog ran.

Deletion wording follows the real existing policy, not the old static label:
request_delete_confirmation checks task_removal_deletes_file. False calls remove(task,false)
directly; true opens original380x178 DeleteDownloadView. Confirm calls remove(task,true),
Cancel/close only retire that presentation. DownloadEngine::remove_correlated always cleans
task-owned temporary artifacts and removes the record; it removes final_path only when
delete_artifact is true. Original Batch Remove passes false; Cancel retains the task record
and changes task state, not list removal. A read-only application getter projects the SAME
setting for the menu label (Delete file ellipsis versus Remove from list); no policy or
capability change. Final confirmation remains the immediate Delete file label without ellipsis.
Source UI menu content keeps the official ElementItem/Label layout and original handler while
exposing the same current text as a semantic Label; no second menu or key dispatcher.

ABDM remove/dialog vocabulary and legacy MainWindow BuildTaskContextMenu/SettingsWindow
constructor->WindowAnimationService.Attach->OnOpened/ApplyCancel/OnClosed were reread.
Unavailable Compose/Avalonia native internals are not inferred. Actual production label reds,
pointer/official-menu Enter/Tab+Space cancellation, synthetic record removal retaining a sentinel,
and original full-object/gesture protections are in .trellis/tasks/10-26-command-naming.
No actual final-file deletion, external download/export, native/product/release operation;
strict/broad shared-tree failures and native/physical/reader/visual/user limits remain explicit.

### Audit #11 projection review (2026-10-26)

- Re-read ABDM first: desktop `pages/home/sections/DownloadList.kt::DownloadList`
  passes task-ID keys and the original LazyListState to shared desktop
  `ui/widget/table/customtable/Table.kt::Table`; its sortedList feeds LazyColumn
  itemsIndexed. Shared `pages/home/BaseHomeComponent.kt::downloadList` combines
  upstream status/category/search, distinctBy ID and original selection reconciliation.
- Then re-read legacy `MainWindowViewModel::{FilterTasks,SortTasks,SortBy,
  RebuildVisibleTasks}` and `MainWindow.axaml.cs` constructor/OnOpened/OnClosed,
  TaskRow_ContextRequested -> SelectTaskForContextMenu -> BuildTaskContextMenu.
  Native Avalonia/AWT implementation is not inferred from these application sources.
- Next's original DownloadApplication replaces its entire task Vec at constructor /
  refresh_snapshot, while progress/icon maps are updated independently by stable ID.
  The list projection can share that immutable authoritative snapshot and sort source
  indices, without deep-copying all task requests/progress/icon paths each repaint.
  A retained index must only address its captured snapshot, never a later refreshed Vec.
- Published gpui-pre0.3.9 `elements/uniform_list.rs::measure_item` requests one real
  measurement row; prepaint additionally requests the visible range. Both are legitimate
  construction work. Keep the existing scroll owner, rem row measurement, ID callbacks,
  selection/context/OLE/detail lifecycle and deferred cross-entity effects. This is a
  data-projection optimization, not a DataTable migration or detail-threading change.
- Regression/evidence owner: `.trellis/tasks/10-26-download-virtual-projection/`.
  Synthetic construction/access/identity evidence is not native, DPI or visual acceptance.

### Audit #12 selected-detail IO/lifecycle review (2026-10-26)

- ABDM first: ProgressDownloadPage.kt::ProgressDownloadPage/RenderPartInfo consume the
  original monitor item and scoped BaseSingleDownloadComponent.extraDownloadProgressInfo;
  scoped close/onDismiss does not imply cancelling the download. Legacy then:
  DownloadTaskViewModel Header/Cookie/Segment summary projections and AddLog retain300;
  MainWindow OnOpened/OnClosing/OnClosed and HideToTray retain presentation ownership.
  Neither application source proves unavailable AWT/Avalonia native implementation.
- Next's render_downloads -> cx.defer -> DownloadApplication::request_task_detail ->
  window_support::load_download_task_detail -> DownloadEngine::handle_command(TaskDetail)
  -> engine/projection.rs::command_task_detail -> task + original store load_transfer_plan
  and load_task_timeline(300) is synchronous. Both storage methods take Mutex<Connection>.
  refresh_snapshot also reloads selected detail synchronously. Deferral is not background IO.
- Preserve that typed command/store/schema and original download runtime. Background blocking
  work must retain no GPUI lease; install pending before dispatch, scope publication to current
  task/request generation and retire selection/closed presentation before accepting completion.
  Cache hits, failures and late replies obey the same boundary. Cancelled IO may physically
  finish, but must not overwrite newer details or spawn unbounded replacement workers.
- Evidence owner: `.trellis/tasks/10-26-download-detail-async/`. Real synthetic SQLite mutex
  contention tests must prove UI progress while the store is still blocked; source defer tokens
  or helper-only tests do not prove responsiveness. No native/network acceptance is implied.

## User-approved 2026-10-28 row/feedback refinement
After read-only audit user explicitly changes historical secondary path/error target: filename second line is file size/unknown-size, actual file icon20->40DIP (source Shell extraction24->48). Keep original48DIP virtual rows/selection/transfer/status/sort/filter, no source-domain duplication. Rechecked ABDM desktop/pages/home/sections/DownloadList.kt170-234 Ctrl/single/double/secondary callbacks and TableDownloadItem.kt80-95, then legacy Views/MainWindow.axaml.cs1217-1243 selection and1335-1345 context selection->BuildTaskContextMenu.Open. Selection alone must no longer create detail IO or show dock; explicit originalPopupMenu ViewDetails creates current original detail scope; changed selection/navigation retires it. Original configurable doubleclick/task HWND and C#Shell APIs unchanged. NewDownloadView local submit validation now belongs to existing NotificationCenter like async operation failures; preserve draft/probe/generation/zero-effect and critical/persistent Alerts, not duplicate transient inline banner/window growth. Source/compiled/native evidence in10-28-ui-experience-consistency-repair, not implied by this note.

## Release38 download context-menu order (2026-10-30)

Rechecked ABDM desktop HomeComponent::onRequestOpenDownloadItemOption272-279 and legacy
NxTools Views/MainWindow.axaml.cs::TaskRow_ContextRequested1335 -> BuildTaskContextMenu1452;
legacy properties ends the original menu1485. Current Next render_download_task_row builds
the same original official PopupMenu and deferred open_download_detail callback. User requests
View Details LAST, keeping its single-selection availability and all existing action groups,
capability predicates, extension submenu and action-context/focus owner. Append after
download_task_context_menu; remove the former initial item/separator, not change menu commands.
Production secondary-click/detail-scope regression asserts the resulting order and same action.

## Batch feedback and final destination conflicts (2026-10-31)

Reference roots: `D:\Code\ab-download-manager-master` (ABDM), then `D:\Code\NxTools` (legacy NxTools).

- ABDM `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/AbstractDownloadActions.kt::{pausableSelections,resumableSelections,deleteAction}` snapshots eligible selections; pause/resume run sequentially in a coroutine. `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/home/BaseHomeComponent.kt::{requestDelete,confirmDelete}` counts finished/unfinished items, emits one confirmation effect, then removes sequentially. `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/HomePage.kt::{ShowDeleteDialog,DeleteDialog}` offers completed-file deletion, dismisses before dispatch and initially focuses Delete. Those paths do not aggregate partial failures or expose batch progress.
- ABDM `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/DownloadManager.kt::addDownload` serializes admission with dbAddSync, checks disk/task names and supports AddNumbered, OverrideDownload and Abort. `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/addDownload/single/AddDownloadPage.kt::DuplicateDownloadSolutions` visibly offers overwrite. `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/destination/SimpleDownloadDestination.kt::onAllPartsCompleted` replaces the final target; it is not a no-replace or finalization-time rename-retry reference.
- Legacy `ViewModels/MainWindowViewModel.cs::{PauseSelected,ResumeSelected,CancelSelected,DeleteSelected}` snapshots visible checked items (fallback SelectedTask), dispatches each and reports the original count, not completed per-target results. `DeleteSelected` awaits `Views/DeleteConfirmationWindow`, requests cancellation and removes records without waiting for writers. Local cleanup errors are logged rather than reflected in the count.
- Legacy `FindDownloadConflictAsync`, `ApplyDownloadConflictResolutionAsync` and `GenerateUniqueFileName` in `MainWindowViewModel.cs`, plus `AddDownloadDialogViewModel.cs` and `Views/AddDownloadWindow.axaml`, expose numbered names, explicit overwrite and reveal-existing-file at admission. Numbering uses `name (1).ext` and disk/nonterminal task reservations. HTTP/HLS finalization uses replacing File.Move; DASH deletes then moves. No finalization-time conflict-retry choice exists.
- Legacy `Views/DownloadProgressWindow.axaml.cs` derives directly from Avalonia Window: constructors attach WindowAnimationService, bind the task and subscribe; `MainWindow.ShowDownloadProgressWindow` keys by task ID and reactivates live windows; `ShowIndependentSecondaryWindow` positions, sets topmost, calls Window.Show and activates. Closed removes the registry entry. Title close is presentation-only; explicit Cancel calls the task action then closes. Failure remains in the same window and does not automatically reopen a closed one. `WindowAnimationService.Attach` owns delayed close; `WindowChromeService` and `WindowActivationService` consume the already-created platform handle. External Avalonia native source was not inspected.

Slice invariants: retain bounded engine streams and generation/sequence snapshot recovery; batch targets are processed once and valid targets survive partial rejection. Existing GPUI confirmation/task-window owners retain placement, activation, close cleanup and deferred callbacks. No list-only-removal confirmation. Quantity-labelled confirmation uses the existing delete surface for destructive batches at least20. Existing failed440x196 client remains fixed, with a bounded filename and reachable official actions. Final publication must not replace another file; failed uncommitted output remains owned and available for numbered recovery. The requested automatic-numbered retry and accurate partial-result notification are product behavior beyond these references, not claims about legacy behavior.

## Download list icon failure lifetime

ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/TableDownloadItem.kt::NameCell`
and `desktop/utils/OSFileIconProvider.kt::getIconOfFileExtension/getIcon` use extension-keyed
OS icons retain a generic file fallback when extraction fails. Legacy NxTools
`Views/MainWindow.axaml` task ListBox uses the generic `draft` glyph; it has no native icon retry loop.

The owner chain is `DownloadApplication::refresh_snapshot/request_task_icons` ->
typed `ExtractShellIcon` -> C# `ShellIconExtractor::ExtractOnSta/Extract` ->
`ResolveIconTarget/SHGetFileInfo` -> bounded PNG cache -> correlated application result.
Requests use 48-pixel icons, shared resources and a neutral fallback in the existing list layout.
Icon state does not change window show/hide, placement or focus.

Failures belong to a live icon resource, not a download task's transfer state. Each resource has at
most three automatic attempts, with five- then ten-second backoff within one PlatformHost connection.
Snapshot refresh and duplicate Connected notifications retain exhausted resources. A real Disconnected
transition retires pending work and failure budgets; reconnect can retry still-live resources while
successful cached resources remain reusable. Removal or a changed resource identity retires failure
state. A deferred retry checks its original resource token before dispatch; success clears that
resource's failures. Offline waits do not consume attempts. Queue rejection uses the same finite
backoff. Late results cannot publish an old task/resource assignment. Tests exercise the actual GPUI
application and typed endpoint.
Accepted/completed/failed diagnostics retain command correlation, elapsed time and native code;
failure events add attempts/retry_scheduled metadata, never file targets.

## 2026-11-02 IPC/browser handoff lifecycle

AB Download Manager was inspected first. `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/nativemessaging/host/DefinedHandlers.kt:8` forwards AddDownload through application IPC; `integration/server/src/main/kotlin/com/abdownloadmanager/integration/model/DownloadCredentialsFromIntegration.kt:16` carries one generic header map; `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/integration/IntegrationHandlerImp.kt:29` enters the ordinary HTTP/HLS add-download owner. Its browser extension is a separate repository (`README.md:119`) and is not locally available. No ABDM header preference, popup lease or Blob-staging counterpart is claimed. `NativeMessagingHost.kt:36` and `stdio/StdioProtocol.kt:87` establish browser stdio EOF as host termination.

Legacy NxTools `browser_extension/nxtools-interceptor/popup.html:192`, `README.md:3` and `background.js:460,1170,2564` establish Header as the master request-context preference and Cookie as its subordinate authenticated-resource preference. Disabled Header sends neither headers nor Cookie; no separately serialized Referrer exception is established. The legacy redundant headers/headerText/cookie shape is not a compatibility invariant. Current protocol-v2 BrowserHost has consumed only headers, dedicated cookie and referrer since foundation commit4de8b1b; missing fields default and unknown fields are tolerated. Old extension→new host tolerates ignored headerText; new extension→old protocol-v2 host may omit it, keep existing field names, and send Cookie only once in its dedicated field. Native protocol2 and256KiB remain unchanged.

Legacy `page-hook.js:770`→`content-script.js:895`→`Services/LocalApiService.cs:413,738,885` stages a retained Blob via Base64 and broad loopback URL; `ViewModels/MainWindowViewModel.cs:2652,2741` opens ordinary confirmation. No matching legacy browser-Blob cancel/close/orphan cleanup owner was found. Preserve confirmation behavior, not that cache lifecycle. Next's actual chain is `BlobUploadService::commit_candidate`→BrowserHost `commit_blob_upload`→`BrowserOpenBlobDialog`→engine `open_browser_blob_dialog_correlated`→private GPUI context→`BrowserImportBlob` or `BrowserDiscardBlob`. Engine exact-size validation, no-replace publication, database compensation, cancel and window-close cleanup remain authoritative. Complete files held by confirmation must outlive BrowserHost's one-hour startup sweep; engine initialization owns orphan recovery because confirmation windows do not survive that owner process.

Legacy `background.js:928,1046,3906` and `popup.js:689` eagerly connect/reconnect and periodically refresh. That transport is explicitly superseded. Browser startup/diagnostics never connectNative; passive snapshots never launch Desktop; pending requests and the entire Blob operation retain their existing demand owners, with1s idle release and no autonomous reconnect. Popup-open adds one explicit owner for its lifetime, released on popup-port disconnect, so1.5s refreshes reuse one host. A disconnected popup does not restart the host on periodic refresh.

M7/L5 are Next-specific transport repairs, not migrated framing algorithms. Old `Services/SingleInstanceService.cs:45` recreates an asynchronous one-line activation server per loop and continues after failures; it has no equivalent shared Download/Indexer/Platform handshake or admission contract. WinShun `src/plugin/Runner.cpp:118,203`, `src/mcp/LineChannel.h:3,35` and `LineChannel.cpp:25,59` verify overlapped duplex handles, event-driven I/O and CancelIoEx followed by completion draining; `src/plugin/runner/main.cpp:58,112` has bounded40ms missing/busy-pipe retries. Its numeric limits/admission policy are not Next contracts. Next retains absolute initial-frame deadline/newline disarm,1MiB frames, native identity admission, four Indexer slots,500ms retiring writes and lease/generation isolation.

L9 changes the Next broker control lifetime; old NxTools has no extension broker. WinShun `LineChannel.cpp:59` and `src/plugin/runner/main.cpp:560` bound the current unterminated line and process complete frames indefinitely without a cumulative stream budget. Next retains256KiB lines and finite per-run stdin/stdout/stderr limits; only the broker's lifetime control stream explicitly has no cumulative budget.

## Compact URL intake and clipboard prefill (2026-11-05)

User requests a single compact URL row, removes idle instructional copy, moves Continue beside Cancel at the right, and authorizes a shorter initial client. ABDM first: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/EnterNewURLPage.kt:55` focuses the link then calls `onPageOpen` once through LaunchedEffect. `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/enterurl/BaseEnterNewURLComponent.kt:65` gates first open, reads clipboard and fills only exactly one extracted link; it does not submit. `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/extractors/linkextractor/StringUrlExtractor.kt` checks whole trimmed lines before regex extraction. `shared/app/src/desktopMain/kotlin/com/abdownloadmanager/shared/util/ClipboardUtil.desktop.kt` catches AWT clipboard-read failure as no result. `desktop/app-utils/src/main/kotlin/com/abdownloadmanager/desktop/window/custom/CustomWindow.kt:343` forwards state/onCloseRequest to Compose Window; its underlying AWT native implementation is not inspected.

Legacy `Views/AddDownloadWindow.axaml{,.cs}` and `ViewModels/AddDownloadDialogViewModel.cs` preserve URL-only explicit Continue/probe, manual Paste, later confirmation and original RequestClose/dispose. Constructor attaches WindowAnimationService; OnOpened applies chrome, OnClosed detaches/disposes, and RequestClose calls Close through the existing animation guard. Avalonia native Show/Close is unavailable. This task intentionally adds the ABDM-style first-open prefill to Next; it is not inferred from legacy Paste.

Next retains `DownloadApplication::open_new_download_window_with_request -> NewDownloadView/Root`, original topmost/activation/deferred focus and hidden fixed confirmation placement/commit. Initial560x136 supersedes only compact560x176 for this approved slice; ordinary confirmation/media and error reserves keep their existing owners. Compact input uses exact official0.8.0 InputGroup/input/addon and one existing InputState, with link/paste slots; confirmed Form/Field and original footer action variants/callbacks remain. C# owns one-shot native text read; Rust owns single valid HTTP/HTTPS selection and generation/revision/closed-window refusal. No automatic probe/task/download, capture-history dependency, clipboard watcher, native bounds animation or new UI controller. Task: `.trellis/tasks/11-05-download-intake-compact/`.

## Download-category extension tags (2026-11-06)

User supplies a wrapping removable-pill reference for extension rules. ABDM was inspected first at `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/util/category/Category.kt` and `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/category/NewCategoryPage.kt`: acceptedFileTypes is a case-insensitive suffix collection; category submission remains explicit. Legacy `ViewModels/CategoryDialogViewModel.cs::SplitTokens/BuildState` strips leading dots/deduplicates and keeps URL rules independent; `Views/CategoryWindow.axaml.cs` attaches the existing window lifecycle, submits BuildState only on Save and resolves null on Cancel/Close. Avalonia native implementation remains unavailable. The approved pill presentation supersedes comma-text editing only, not category selection, directory resolution, matching priority or explicit Save/Cancel.

Next `downloads/settings_view.rs::DownloadCategoryEditor` retains the original unsaved extension InputState and adds a stable pending InputState. `downloads/category_extensions.rs` composes public Kit0.8.0 Tag secondary/outline/rounded_full with actual XSmall ghost removal Buttons, a Small outlined Add pill and a default32DIP Input. Values retain compound suffixes, normalize leading dots/ASCII case and deduplicate; removal is value/current-category bound. Input Enter, button actions and Escape cancellation defer input mutations beyond the current GPUI entity lease. Pending text joins the original settings draft on explicit Apply; no tag edit writes disk or changes accepted tasks. Category form and tag content use official bounded scrolling while bottom category actions remain outside. Original engine/API/IPC/native window owners are unchanged. Task and verification: `.trellis/tasks/11-06-download-extension-tags/`.

## Download-list icon resolution — 2026-11-09

User supplies Ghost Downloader3 for icon clarity and feature comparison. [Verified reference and native boundary](ghost-downloader.md) records its48px QFileIconProvider list path, separate Fluent operation/status icons, GPL-only reuse limit and feature entry files. ABDM OSFileIconProvider.kt and old MainWindow.axaml1137–1138 were reread before implementation.

Current Next requests48px in downloads.rs and paints40DIP in download_page.rs. Original ExtractShellIcon uses the bounded native STA/cache; SHGetFileInfo small/large selection does not itself honor arbitrary requested pixel sizes. Preserve the official existing image/Contain slot, task association, attribute-only type queries, cancellation, bounded cache and handle release. High-resolution lookup belongs to C#, not a new UI icon engine, copied Qt resources or changed download state.

The icon failure path was rechecked against ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/home/sections/TableDownloadItem.kt::NameCell` -> `desktop/utils/OSFileIconProvider.kt::rememberIcon/getIcon/getIconOfFileExtension` first: extension-keyed OS lookup returns the generic file icon on extraction failure, including names without an extension. Legacy `D:\Code\NxTools\Views\MainWindow.axaml` task row1131–1139 always paints its `draft` glyph, independent of file existence. Next's `DownloadApplication::request_task_icons` assigns a completed artifact path resource, then `handle_platform_notification` consumes the correlated result; resource failure has three finite attempts and a generation-bound retry. The original `download_page.rs` 40DIP icon well renders `img` at40DIP or an official File icon at30DIP. A failed image load must use GPUI pre0.3.9 `img.with_fallback` without changing the 48DIP row, task/window focus, placement or synchronous entity-update boundary. Missing paths fall back to a type resource; an extensionless file uses the generic reserved file type. STA admission pressure retries briefly without consuming extraction-failure attempts. Old task-window close remains presentation-only, unrelated to icon failures.

## Mister progress and integration reference (2026-11-09)

[Verified Mister source index](mister-download-manager.md) records server observation/estimated RTT, progress lifecycle, limiter/completion semantics, browser handoff, automatic clipboard popup and comparison with Ghost. Both references are GPL behavioral references; existing ABDM/legacy authority and Nx transport/identity/durable lifecycle remain unchanged. The bounded implementation plan is `.trellis/tasks/11-09-download-server-information/`: reuse the selected HTTP probe observation, label response timing honestly, preserve one progress-window owner and add no Ping/network polling.

The Add Download prefix uses locked0.8.0 `InputGroup::input/addon` with `IconName::Link`. Kit `assets::Assets` embeds only the default component subset; the public complete `IconName` catalog does not imply that its SVG is embedded. Selected application icons must be composed through the released `icon_assets!` source alongside existing default/product assets. Keep the official leading slot, frame, original InputState and paste/focus lifecycle; do not hide a missing asset by deleting its padding.

## B1 originating browser context and handoff boundary (2026-11-09)

Behavioral review for REF-02/03: Mister `extension/background.js::cookiesFor/cookieHeader/handOff` queries the download's named store and separates private context; its enqueue acknowledgement is not durable acceptance. ABDM `DefinedHandlers.kt::AddDownloadNativeRequestHandler.handle` -> application IPC -> `IntegrationHandlerImp.kt::addDownloadByGui/convertToDownloadSystemCredentials` forwards credentials to the ordinary add owner. Its independent extension source is unavailable locally. Legacy NxTools `background.js::cookiePairsForUrl/enrichHeadersForDownload` uses URL-only cookies; `interceptBrowserDownload` and preemptive Firefox handling do not provide Next's durable ownership guarantee. These are behavioral references, not code transplanted into Next.

Next's existing chain is `bg/request-context.js::rememberRequestHeaders/rememberRedirectHint/resolveHeaderSnapshot` -> `bg/capture.js` -> privileged resource context -> `bg/handoff.js::sendDownloadItem/sendDownloadPayload/sendResourcesBatch` -> `bg/native-transport.js::sendDesktopRequest` -> BrowserHost `create_task` -> current-user download pipe -> engine/browser durable receipt -> matching handoff response -> `handoff-policy.js::completeAcceptedHandoff`. Same-URL observations must not move headers or Cookie-store identity between tabs. Origin identity is consumed before Cookie query; Rust receives the original dedicated Cookie field, not an unused store field. Disabled Header/Cookie gates perform no Cookie query. Missing/rejected source-store APIs do not authorize another store. Redirects retain request identity; closing/replacing a tab cannot select a replacement tab's credentials.

REF-03 preserves browser ownership until durable matching acceptance and successful browser cancellation, original cancellation-failure task compensation, stable handoff IDs and persisted idempotency. Popup reasons are bounded transient fixed categories, not request payload or another task store. Enqueued/persisted/browser-transferred/download-completed remain distinct facts. Product/reference launch and installed/native acceptance require separate authorization.

Chromium request events may supply a real tab ID without a privacy/store field. `rememberRequestHeaders` retains the observed tab's native metadata in that same bounded snapshot; a later download's explicit private flag cannot match an unknown or normal context solely by URL. Source metadata survives tab closure, while retired request snapshots cannot be republished by a late tab lookup. This uses the existing tabs permission and request owner, not another credential cache.

`resolveHeaderSnapshot` uses privacy only as a filter, not as a tab/container identity. A DownloadItem with only `incognito` and `referrer` must not select the latest same-URL request across distinct source contexts. `sendDownloadItem` feeds a uniquely resolved referrer tab back into snapshot selection and Cookie-source resolution; `resolveTabIdForDownload` returns no source when several matching referrer tabs remain. The native DownloadItem-shape regressions inspect outbound Authorization/Cookie and exact Cookie API arguments.

BrowserHost `submit_download_with_client` preserves the original production connector and dialog retry, checks the engine result request ID, and returns fixed correlated persisted/pending/rejected metadata while retaining parsed handoff identity. `BrowserDialogAccepted` reports confirmation admission with `accepted=false`, not task persistence. `browser_wire_tests.rs` and `tests/harness/native-host-roundtrip.cjs` cover native framing -> production submission -> isolated current-user pipe -> engine confirmation/SQLite, plus the original extension ownership policy's cancellation-failure compensation through a real Cancel command, stable replay and refusal. No product/default-pipe/native-window execution is used by these fixtures.

## Compact progress and category-rule header (2026-11-09)

The approved interaction polish uses ABDM ProgressDownloadPage.kt/RenderInfo and
BaseSingleDownloadComponent.kt first, then legacy DownloadProgressWindow.axaml,
DownloadTaskViewModel.cs and SettingsWindow.axaml. Main transfer information stays primary;
optional part/capability information uses the existing disclosure. The original task-ID window,
same-window completion, close/cancel, placement/focus and settings Apply/Cancel owners remain.

Next task_window.rs retains440×184 collapsed/440×280 expanded, fixed44DIP footer, official
XSmall Progress and Small borderless DescriptionList. Progress filename14/20 sits above12/16
metrics. Host plus short route and colored HTTP request-to-headers duration replace the earlier
IP/verbose-duration presentation:0–100ms success,101–300ms warning,301+ danger; absent observation
has no time. No peer IP or extra Ping/probe is rendered or logged. Resume/limit/connections/segments
remain inside the original expansion above its existing Table.

settings_view.rs uses real Kit0.8.0 Select/SelectState/SelectEvent over stable category IDs, with
default Medium32DIP trigger and a260×220DIP-bounded popup. Category Rules/help/Select/Small outlined
Add share one wrapping header. selected_category and the original draft/Apply remain authoritative;
input-driven Select mirroring is deferred beyond the current GPUI entity lease. No second saver.

The file fallback uses official IconName::File, not a download-action glyph. Shell extraction owns
one independent native IImageList reference per query; persisted invalid PNGs regenerate inside the
original per-key generation. Bounded cache/STA limits and safe category/native-code diagnostics remain.
See [Ghost icon boundary](ghost-downloader.md) and the task's download-layout-slice.md/icon-slice.md.

## B1 acceptance contracts (2026-11-09)

Observed request privacy/store identity survives a closed Chromium tab; later tabs.get metadata only supplements compatible source facts. Chromium's normal/private store mapping uses an existing enumerated store in the onDeterminingFilename-capable path, not Firefox containers or an unresolved tab ID. `handoff-policy.js::handoffOutcome` admits pending only for original ok=true, accepted=false, matching nonempty handoffId, no taskId and awaiting_confirmation/confirmation_pending without queue admission. `sendDownloadPayload` and Blob commit preserve that operation; context-menu/router/popup/content feedback shows 确认窗口已打开，尚未创建任务 without claiming persistence. `runSingleResourceHandoff` retains pending identity until durable acceptance. Batch openDialog=false and original automatic browser receipt/cancel compensation/current-user pipe/engine owners remain. These distinctions supersede the historical accepted-only manual feedback contract.

`bg/capture.js::captureNetworkResource/captureRequestResource` derives browser referrer/page metadata and Cookie context from its resolved native tab. Apply the same observed-source compatibility before using that tab: an incompatible live tab may not supply page/referrer or privacy/store facts to the captured request/resource. Existing initiator, observed headers and originating store remain authoritative.

## B2 intake and control reference entries

ABDM was inspected before legacy for explicit intake: `EnterNewURLPage.kt:55` -> `BaseEnterNewURLComponent.kt:65` owns first-open focus/single-link prefill without submission; `window/custom/CustomWindow.kt:343` passes original state/close/placement to Compose Window. Legacy `Views/MainWindow.axaml.cs::ShowAddDownloadWindow:1932` reuses one secondary window or constructs `AddDownloadWindow`, attaches a current-window Closed guard, then calls `ShowIndependentSecondaryWindow`; `RunAddDownloadAutoParseWhenNeeded:2004` is separately conditional. `AddDownloadWindow.axaml.cs` attaches/detaches the original animation lifecycle and disposes the view model; `AddDownloadDialogViewModel.cs:19,151,180,552` retains owner defaults, explicit external auto-parse choice, cancellable Parse and disposal. Underlying Compose/Avalonia native internals are not available in these sources.

Next `main.rs:315,332` composes the existing ClipboardApplication and DownloadApplication. `tools/link_panel.rs` owns compact420DIP/28DIP ListItem rows, named official XSmall ghost actions and local close before deferred application work. `DownloadApplication::open_new_download_window_with_request` constructs the original NewDownloadView/Root, topmost/activation and deferred URL focus. Its Download source auto-probes; REF-04 requires a distinct URL-only source without probe or clipboard read. Released Base0.8.0 `Root::view()` exposes the original view for guarded reuse, without a second production view/controller owner. `NewDownloadView::clipboard_prefill_eligible/prefill_clipboard_url` protects blank, zero-edit/probe, nonexternal and inactive drafts, including edit-then-clear. Preserve one original window, dirty draft, fixed confirmation, focus/IME/paste, cancellation and original native placement. Panel replacement before deferred dispatch must suppress the retired action.

For speed limits, ABDM `ToggleableSpeedLimitConfigurableRenderer.kt` preserves explicit units/enablement while its current Compose updates are immediate; Mister `crates/app/src/progress.rs:556` supplies the deliberate Enter/lost-focus interaction. ABDM `OkioCustomizedThrottler.kt:1` retains one aggregate budget, zero/unlimited and waiter signalling after changes. B2 changes submission interaction, not NxTools's existing limiter or persistence owner.

Mister `extension/background.js:1209` separates manual authorization from automatic rules; pause and first/final-address exclusions precede type/known-size matching. Its response discovery at393–683 uses bounded candidates and coalesced publication. Its credentialed manifest sniff is not adopted. NxTools retains original request-store identity, durable handoff/cancellation, DOM/page-hook/PerformanceObserver/Blob and batch owners; any periodic-work reduction requires equivalent pre-change/candidate coverage and browser-time evidence. [B2 design](../../.trellis/tasks/11-10-reference-b2-intake-control/design.md).

Legacy native-facing intake boundary: `MainWindow.axaml.cs::ShowIndependentSecondaryWindow:2844` measures and places the original window, sets Topmost, calls `window.Show():2867`, then uses `WindowActivationService.BringToForegroundWhenReady`; loaded placement/activation remains on the original dispatcher. `AddDownloadWindow`'s existing close animation terminates at its original `Close`, not another confirmation owner.

REF-05 current Next admission: `background.js:143` registers Chromium onDeterminingFilename or Firefox onCreated -> `bg/handoff.js::interceptBrowserDownload` -> `sendDownloadItem/sendDownloadPayload` -> existing native receipt/cancellation policy. `bg/router.js::content_direct_download_request/content_blob_download_request` are the automatic clicked-download substitutes; explicit drop/context-menu/popup submissions keep manual authority. `bg/settings.js::{siteHostKey,firstSiteHostKey,isSiteEnabled,sanitizeSettings,getSettings}` owns existing exact-host normalization and local preferences; `bg/naming.js::{getFileNameFromPath,getFileNameFromUrl,getFileExtension,extensionFromMime}` and restored `dispositionFileNameForUrl` own filename/type facts. Popup `collectSettings -> saveSettings -> popup_save_settings -> storage.local` retains durable-before-publication. `storage.session` is already used for worker-lifetime browser state; REF-05 adds only its session pause field, with no new store or permission. Rule syntax is a suffix-extension allowlist (empty means all), nonnegative minimum KiB for known positive lengths, exact excluded hosts and case-insensitive address substrings against original/final/page URLs. Pause/exclusions precede type/size and original B1 handoff; unknown size does not authorize extension network probing.

REF-05 pause diagnostics retain the existing `sendExtensionLog -> buildExtensionLogRequest -> BrowserExtensionLogPayload -> validate_extension_log_payload -> handle_request(extension_log)` path. Only capture_pause_started/applied/failed are added, with UUID correlation, fixed paused/resumed/session_storage_failed and elapsedMs. The host does not log arbitrary reason/message/URL/Cookie fields; best-effort logs never connect an idle native host or alter pause storage/admission. Native framing and existing browser handoff events remain unchanged.

### REF-06 original submission and rate owners

The original global input is `downloads/settings_view.rs::speed_limit_kib`, read by `build_draft` and the existing `DownloadApplication::apply_settings` transaction. The existing standalone `DownloadApplication::set_global_speed_limit` sends typed `DownloadCommand::SetGlobalSpeedLimit` with one correlation ID; `nxtools-download/src/engine.rs::set_global_speed_limit_correlated` saves SQLite intent before publishing the shared limiter. The task equivalent is `DownloadTaskAction::SetSpeedLimit` → `set_task_speed_limit_correlated`: persist a cloned task, update its existing transfer control, then publish `TaskChanged`. Neither needs another limiter or scheduler.

`http.rs::DownloadTransferControl::acquire_bandwidth` awaits the original global and task limiters together; HTTP and segmented runners retain this control. `rate_limit.rs::set_bytes_per_second` updates the same budget and signals its watch revision to wake existing waiters. Runtime background policy stays a separate minimum ceiling. Reopen reads original persisted intent, not a new settings store.

Official locked Kit0.8.0 `Input::new(&Entity<InputState>)` uses Base0.8.0 `InputEvent::{Change, PressEnter, Blur, Focus}`. Escape is the public `input::Escape` action, not an InputEvent; the editor cancels its own active overlays before propagating an otherwise unhandled action. Keep the stable InputState, subscribe to Enter/Blur, defer cross-owner command/result updates and handle Escape at the owning input wrapper. Existing `field_shell` forces official Medium bordered Input with focus border; the expanded task details arrange that96DIP input and explicit KiB/s/0-unlimited labels in one32DIP row without changing native collapsed/expanded sizes, titlebar, footer or segment owner. Label uses Styled typography, not a Sizable variant. No Toolbar child adaptation applies to this row.

The task window chain remains `request_task_window` → `task_windows_impl.rs::open_task_window` → hidden original Root and keyed view → original native bounds/visibility commands → `update_task_window`. Close releases the keyed window/generation; delayed speed submissions must check that same view/generation before changing a task. Local draft/error state never creates a new window or steals focus. Enter followed by Blur is one submission; failed persistence leaves the applied value unchanged. Escape restores that value and retires the local draft before later blur.

`engine/projection.rs::DownloadTaskSummary::into_engine_task` deliberately omits request-only limits. Task-window creation and `task_windows_impl.rs::update_task_window` obtain current applied task limits through the original full `DownloadEngine::task` projection, retaining its queue-eligibility lookup. Updates keep the original compact status/progress/artifact snapshot as the phase/lifecycle owner and supplement only applied limit/probe facts from full intent. A summary's absent limit is unknown, not persisted unlimited intent; an unavailable full read preserves the last applied rate. The original window update synchronizes clean stable input and computes the effective task/global label from current applied authority, including external typed commands; dirty/pending/error text stays local. No render-time storage query is needed. The original capabilities container exposes its existing accessibility label through Kit's public test-support observation.

The supported command/config rate is exact positive bytes, while the original editor accepts KiB/s. `SpeedLimitEdit::value` uses positive ceil-KiB display; this is presentation only. `settings_view.rs::current_draft` must retain exact `SpeedLimitEdit.applied` for an untouched field, including sub-KiB and fractional-KiB values, when an unrelated option is applied. Only explicit edits use checked KiB-to-byte conversion. Preserve Enter/Blur once-only submission, Escape, save failure and SQLite-before-limiter publication.

### REF-12 existing discovery boundaries

Legacy `content-script.js::scanPage:1069` scans media/CSS/Performance; its buffered PerformanceObserver:2302 and explicit content_scan_now remain independent fallback observations. Current Next `content-script.js::scanPage:353` keeps media/Performance, with CSS only for explicit deep demand; MutationObserver:1280 handles added/src/href elements, loadedmetadata:1300 and page-hook candidates:1307 retain their original source. The buffered observer:1383, explicit deep content_scan_now:1414, navigation resets and settings reenabling scans are separate from periodic visible-page scan7s, context1s and overlay1s jobs. `page-hook.js` retains original fetch/XHR/Blob and5s YouTube-response observation. `bg/capture.js` uses response/request/page facts, then original cache identity/rank/privacy owners; `resource-cache.js::schedulePersistResources` coalesces storage600ms. `tab-bridge.js::scanTabForResources` keeps original3.5s demand throttle/frame injection; background committed/history/removal events clear original tab ownership. These entries are coverage evidence, not browser CPU or latency measurements. [REF-12 audit](../../.trellis/tasks/11-10-reference-b2-intake-control/ref12-audit.md).

### REF-12 evidence-driven selection candidate

Mister `extension/background.js::mediaKind:496` recognizes manifests before segments;503 rejects segment MIME/extensions,514 rejects every native-media response below64KiB,524 rejects non-media ranges/small files, and `noteMedia:540` suppresses same-folder resources after a manifest. Its `mediaList:838` probes/sniffs manifests and performs DOM matching on explicit demand. These thresholds/folder heuristics and credentialed sniffing are not NxTools contracts. NxTools preserves supported small complete and opaque media, normal full-source byte ranges, manual intake and its original Rust network authority.

Legacy `content-script.js::scanPage:1069` retains media/CSS/Performance scans, and buffered PerformanceObserver:2302 plus explicit content_scan_now:2330 are independent fallback sources. Next `responseMeta` derives actual MIME, Content-Disposition and Content-Range before naming enrichment; `captureNetworkResource` and `capturePageResource` record normalized discovery/DOM facts. Original `cacheResource` retains120/TTL/aliases and source-ranked metadata, without transferring frame/pageKey/source/discovery/DOM identity. `resourcesForPopup`, `buildContentResourceChoices` and bounded local `excludedUrls` use the shared selector. Content `observedMediaElement` and direct fact comparison let loadedmetadata/controls/role changes bypass URL/MIME/size/geometry dedup while identical facts coalesce. `scanTabForResources` retains3.5s explicit deep demand and wrapped-response thumbnail publication. Original bucket/pageKey identity retires delayed captures/scans/chooser replies on navigation/history/context/close/replacement. Range fragments require actual audio/video MIME; ordinary ZIP/unknown-MIME range responses remain. The candidate design, deterministic evidence and supplied-Page workload are recorded in [REF-12 delivery](../../.trellis/tasks/11-10-reference-b2-intake-control/ref12-outcomes.md); browser equivalence and measured adoption are not established.

## B4 task operation and visibility reference review (2026-11-10)

ABDM was inspected first: `HttpDownloadJob.kt:631` validates known length/ETag and loss of resume support before parts; `pause:716` cancels scopes and `stopAllParts:705` joins writers. `connection/OkHttpHttpDownloaderClient.kt:25` applies defaults before explicit headers/UA and excludes caller Host. `shared/.../util/UserAgentProviderFromSettings.kt:9` treats blank custom UA as default. `shared/.../downloaderinui/http/edit/HttpEditDownloadChecker.kt:49` only warns on changed length; it is not validator proof. `shared/.../pages/editdownload/BaseEditDownloadComponent.kt:55,96` retains an edited draft and explicit acceptance callback. `shared/.../pages/batchdownload/BaseBatchDownloadComponent.kt:68,114` locally derives inclusive padded integer URLs before import; its1000limit is not Next's admission limit.

Legacy NxTools was then inspected: `AddDownloadDialogViewModel.cs:19` captures default Header/Cookie into the task draft; `RequestHeaderParser.cs:19,52` filters transport headers, gives dedicated Cookie precedence and replaces UA. `DownloadManagerService.cs:787,1128,2149,2222,3397,3428` applies retained task context for probe/transfer. `MainWindow.axaml.cs:2145` reuses one progress window by task ID; `DownloadProgressWindow.axaml.cs:26,34,43,50` chains constructor/base and removes subscriptions on close. `WindowAnimationService.cs:13,37` defers the actual Window.Close until its close animation completes. Actual presentation reaches MainWindow's ShowIndependentSecondaryWindow/Window.Show; closing presentation does not cancel the task. The checkout does not contain Avalonia's native/base show/hide source.

Ghost comparison: `features/http_pack/task.py:65` relink requires Range and equal size but lacks the identity proof required here. `app/view/dialogs/batch_url.py:22` expands broader combinations/letters/steps; B4 supports only one ascending inclusive integer range, fixed step1. `features/http_pack/cards.py:14,66,78` merges written spans, but its16msanimation and guessed aggregate-prefix fallback are not adopted. Reference implementations supply behavioral evidence only, not copied code/comments/tests/assets.

Next's `DownloadHttpPolicy::apply_to_request` preserves explicit names and dedicated Cookie/Referer before immutable request acceptance. One Core User-Agent definition supplies effective default Header text, actual Settings Textarea content and ordinary client fallback. Settings carry the open-time baseline into a changed-leaf merge under the original Core config lock/reread/atomic writer. The merged saved configuration governs runtime publication. Original BrowserBatchSubmit limit is32. [B4 design](../../.trellis/tasks/11-10-reference-b4-task-operation-visibility/design.md).

### Original Next boundaries for task operations and coverage

- `engine.rs::pause` serializes through the scheduler gate, records explicit user-paused admission, and asks the existing transfer control to stop. `segmented_runner.rs` drains writers/checkpoints before `transition_transfer_to_terminal` persists Paused and clears the control. `resume_eligible_correlated` may persist queue eligibility while status is still Paused; status alone cannot authorize replacement.
- `EngineTaskState` owns the original task, transfer snapshot and transfer control. `probe_effective_blocking` runs the retained request through the original routed/certificate HTTP client and accepts `DownloadResolutionOperation`; cancellation drops the actual probe future. `validate_resumed_plan` compares HTTP/range/total/validator facts. `send_range` uses a strong ETag or Last-Modified for If-Range. Empty-validator equality is not identity proof for another URL.
- `SqliteDownloadStore::save_task_with_transfer` uses one transaction for task/request and transfer facts. Persisted plan metadata and segments are separate original rows; a source replacement must commit the request/probe/plan URL consistently while retaining segment identities and checkpoints. No artifact cleanup is part of candidate cancellation.
- `task_windows_impl.rs::request_task_window` → keyed lifecycle directive → `open_task_window/update_task_window`; native close validates the original window identity, retires the keyed registration and stashes placement. `task_window.rs` retains original phase sizes, Server observation, aggregate official Progress, exact speed-limit editor and footer actions. A replacement form belongs to this window and uses deferred work; closing presentation is not ordinary task cancel.
- `engine/browser.rs::submit_browser_batch_correlated` validates all requests and the32item bound before any admission, then returns ordered per-item results through the original QueueOnly handoff/reservation owner. Existing Desktop `batch.rs` performs actions on existing tasks, not intake. Template input/preview/selection belongs to the original new-download presentation and cannot probe or admit before confirmation.
- `engine/segmented_runner.rs::spawn_segment_transfer` updates written counters only from original HTTP writer callbacks; checkpoints are persisted after durable writes. Actual segment `(start,length)` and count maps are initialized from the plan and updated together at growth/shrink boundaries. `DownloadSegmentProgress.start_byte` carries the actual origin; index is identity rather than position.
- `engine/projection.rs::command_task_detail` reads the original task plus persisted plan segments/timeline. `window_support.rs::apply_progress_to_task_detail` adopts the current complete known live layout after rebalance and retains the original count-only fallback when bounds are missing. Coverage carries actual offsets through the original live projection, preserves persisted facts across pause/reopen and bounds painting by viewport width without render-owned network/timers.

Legacy NxTools source search found no paused-source replacement command; its retained request/probe/transfer and keyed progress-window lifecycle remain the relevant compatibility evidence. Ghost relink is comparison evidence only. Ordinary redirect hops retain reqwest's cross-origin sensitive-header removal; the existing final-URL range request separately uses the accepted task context. This context boundary is unchanged by the default User-Agent.

REF-08 storage trace: `save_task_row` preserves resolution provenance and existing probe JSON; `save_transfer_state_row` updates probe metadata and original artifact rows inside its caller's transaction. A source commit therefore calls both plus updates only `transfer_plan_json`, leaving `download_segments`, queue admission and timeline ownership intact. `resume_eligible_correlated` holds the task lock through queue admission, so the same task-owned replacement lease can exclude resume before it becomes eligible. `queue.rs::start_daily_queue_locked` skips explicitly user-paused rows. Download-IPC owns only typed routing; candidate cancellation remains permitted during host draining. New command enum values require a matched command/Hello version, with the stable pipe name retained for explicit prior-version rejection.

REF-09 original intake boundary verified: `downloads/new_download_view.rs::NewDownloadView::request` retains the original external request, directory/name/category/schedule and connection/adaptive intent. `begin_probe` owns the compact next action; `apply_new_download_window_resize_with_correlation` owns the native centered bounds command with TestPlatform fallback. `engine/browser.rs::submit_browser_batch_correlated` validates nonempty/at-most32, unique handoff IDs and every request before ordered `submit_browser_correlated(...QueueOnly...)`. That original path owns request resolution, naming/destination reservations, SQLite handoff acceptance and per-item fault results. Local preview must not enter it; confirmation uses it once and never retries a partially accepted batch.

REF-10 original data boundaries verified: `segmented_runner.rs::spawn_segment_transfer` updates written counts only from original download_segment writer callbacks; initial maps derive from actual DownloadSegment plan entries. Synced shrink merge updates shed/surviving IDs after `persist_transfer_plan_change`; slow-tail split persists reduced/added ranges before workers restart. `engine.rs::download_segment_progress_snapshot` carries optional actual `start_byte`, original segment length and written count. Split/merge update the same range/count maps under progress→range locks before restarted workers publish; index never implies position. `engine/projection.rs::command_task_detail` returns persisted DownloadSegment entries; Desktop `window_support.rs::load_download_task_detail` is the query boundary. `task_windows_impl.rs::open_task_window` reuses one detail read for ranges and final-artifact metadata; `update_task_window` hydrates persisted ranges on status transition outside render. `DownloadTaskWindowView::range_segments` selects original live or paused cached facts and retains known persisted prefixes before the first nonempty resumed live projection. `apply_progress_to_task_detail` adopts current known live bounds after rebalance. `range_coverage.rs` merges actual written prefixes/completed spans and bounds marks to viewport width through public no-id Plot/PlotElement. Render performs no storage/network query or sampling; official aggregate Progress and original task controls remain.

### B4 scoped acceptance repair — validator and attempt freshness

The original REF-08 owners remain `ResourceValidator` → `http.rs::send_range` / response validation and `engine/paused_url.rs` → controlled probe → `SqliteDownloadStore::save_paused_url_replacement`. One selected valid strong ETag (including `""`), otherwise valid Last-Modified, must govern both replacement eligibility and actual If-Range. Ordinary runner recovery is unchanged.

Attempt freshness belongs to the original task's `PausedUrlReplacement`: a checked monotonic revision survives terminal retirement within the engine generation. The original TaskDetail returns `paused_url_revision`; each new ReplacePausedUrl carries required `expected_revision`. Exact retired envelopes reject before HTTP or mutation, while a fresh confirmation reads the current revision. Active ownership, source/generation checks and SQLite-before-publication remain unchanged. Command11 / Hello4 share the original stable pipe and original callers; no persistent UUID history or new transport is introduced.

Lifecycle witnesses use actual held HTTP probes through `run_download_host` with concurrent original clients and the production task window's `on_window_should_close` callback. Cancel ACK, candidate terminal, socket drain and lease retirement are separate observations. TestPlatform close is not physical Windows OS-close acceptance.

## B6 connection source review — 2026-11-11

The [B6 source review](../../.trellis/tasks/11-11-reference-b6-connection-experiments/reference-review.md) records actual ABDM HTTP/job/part entries, then legacy DownloadManagerService, then Mister hosts/download/dns. ABDM stops/joins before idle and flushes before checkpoints; its metadata GET response is closed. Legacy task-local permanent pressure mode is broader than permitted host/route refusal evidence.

Current `http.rs::{probe_timed,prepare_download}` uses HEAD plus exact one-byte Range, with a full fallback GET only after both fail, followed by original transfer GET. `engine/resolution.rs::{ResolutionState,PreparedResolution}` owns generation/operations/receipts; default no-op publishes no extension receipt. The REF-14 feasibility experiment retains current response lifetime after required delayed-confirmation body failures.

`engine/segmented_runner.rs` remains adaptive/rebalance/join/checkpoint authority. `nxtools-network/src/lib.rs` owns immutable route/client construction. Locked hyper-util0.1.20 connect/http.rs226/747/951 already races address families after300ms but tries same-family addresses serially. B6 is independent of UIA/native-memory acceptance and packaging. The [fixed experiment design](../../.trellis/tasks/11-11-reference-b6-connection-experiments/design.md) and separate [REF-13](../../.trellis/tasks/11-11-reference-b6-connection-experiments/ref13-outcomes.md), [REF-14](../../.trellis/tasks/11-11-reference-b6-connection-experiments/ref14-outcomes.md), [REF-15](../../.trellis/tasks/11-11-reference-b6-connection-experiments/ref15-outcomes.md), [REF-16](../../.trellis/tasks/11-11-reference-b6-connection-experiments/ref16-outcomes.md) records retain the frozen experiment evidence. [Coordinator acceptance](../../.trellis/tasks/11-11-reference-b6-connection-experiments/coordinator-acceptance.md) closes B6-01 after independent37group/842execution reacceptance: REF-13 adopt, REF-14/15/16 retain-current, at source/owned-fixture/TestPlatform boundaries.

REF-13 adopts the original-engine `connection_ceiling.rs` observation: final authority plus captured route/policy/TLS/System-client identity,256entries,600second maximum validity within the captured scope, actual receiving payload and repeated typed pre-header native refusal. Original30second System-client reuse expiry may capture a new scope sooner. Every segment worker receives the same run observation Arc as admission, including after bounded table eviction. Original lower caps win; original adaptive stop/join/sync/checkpoint ordering remains. Formal refused connections8→2 and total TCP attempts32→26, with primary median+4.7208% inside the frozen5% allowance; this is fewer refusals, not throughput improvement.

REF-14 retains current probing after all three10second delayed-confirmation retained-body trials fail. REF-15 retains original released-slot tail splitting: the five-sample retirement candidate never activates and persistent median improves only0.2959%, below15%. REF-16 retains original DNS order/Happy Eyeballs: the hostname preference improves the controlled same-port batch but regresses the port-swapped batch98.9693% and adds two losing connects. Rejected product hooks/caches are absent.

Original-owner test entries: `src/engine/connection_experiments.rs` and `connection_experiments/{ref13_controls,ref15}.rs` cross actual engine/SQLite/segmented writer/drain; `src/http/connection_experiments.rs` and `connection_experiments/ref16.rs` cross actual probe/connector/streamed writer/no-replace commit. Owned `tests/support/b6_{server,reuse_server,tail_server,address_server,tls_server}.py` listeners join real sockets/workers. These paths are relative to `rust/crates/nxtools-download/`. TCP retransmission remains unobserved; served-byte overlap is separate. Controlled local and TestPlatform evidence does not establish physical UIA/native memory, Internet performance or installed/Release acceptance.

B6-01 retains that original ownership and separates failure observation from retry eligibility: every failed segment attempt reaches the typed observer once, including terminal attempt3. `src/engine/connection_experiments/b601.rs` and `tests/support/b601_server.py` cover one receiving peer plus `503 -> native refusal -> terminal native refusal`, with intermediate nonlearning, unchanged terminal error/three-attempt budget and actual joins before the final learning assertion. [Repair evidence](../../.trellis/tasks/11-11-reference-b6-connection-experiments/b6-01-repair.md) is separate from the frozen performance trials and unchanged REF-14/15/16 decisions.

## HTTP segmented fault-recovery scheduler

The approved [fault-recovery solution](../../.trellis/tasks/11-12-download-fault-recovery/solution.md)
supersedes the historical Phase2 stop-sync-split and B6 native-only refusal policy for HTTP segments.
Reference source order and verified planner/worker/host entries are recorded in
[Mister fault recovery](mister-download-manager.md#http-segmented-fault-recovery). Per-connection
workers resume synced progress, verify a64byte written overlap, claim before writing, and release
uncommitted claims on failure. Idle workers split unclaimed tails without stopping their original
connection; split plans are durable before new requests.429/503 are throttling, not ignored Range.
Repeated refused extra connections learn host-scoped width within the original network-policy scope;
exhausted extra workers retire while peers continue. Slow-response replacement is disabled by any
active limiter. Existing single-stream/media/probe/routing/queue/publication/native UI owners remain.

Desktop Transfer settings expose effective connection, concurrency, and speed controls. ABDM `DesktopSettingsComponent.kt:155–166` groups these separately from its effective dynamic-part settings; legacy `DownloadManagerService.cs:63,1359–1384` likewise distinguishes queue concurrency from per-task connections. Next’s current HTTP planner owns splitting, so its three obsolete adaptive fields are not editable or form-validated. `DownloadSettingsView::current_draft` → `DownloadApplication::apply_settings` → `AppConfig::merge_download_settings` retains the original save, rollback, and native window lifecycle. Serialized fields and Core normalization remain for existing request compatibility; Apply does not treat them as edited leaves.

## Single-stream response length

Reverified against ABDM `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/OkHttpHttpDownloaderClient.kt::{newCall,actualConnect}` and `part/HttpPartDownloader.kt::connectAndVerify`, then legacy `Services/DownloadManagerService.cs::{CreateProbeRequest,DownloadSingleAsync,CreateHttpClient}`. ABDM exposes the current connection body's length and separately validates strict ranged parts; its job treats webpage downloads as non-strict/unknown-length. Legacy requests identity encoding and disables automatic decompression, but retains a probed-length assumption in its single-stream copy/completion path. That assumption is not suitable for dynamic responses.

NxTools.Next `HttpDownloadClient::{probe_timed,prepare_download}` probes with separate HEAD and Range GET requests, then starts a new ordinary GET from byte zero. A dynamic endpoint can return different complete representations to those requests, including different gzip lengths. The current GET's body length is authoritative for that single stream; a missing length remains unknown rather than inheriting an earlier response's length. Reqwest's body framing still rejects declared-length truncation and incomplete chunked bodies. Preserve exact encoded output, progress based on the current response, pause/cancel writer retirement, sync-before-publication and no-replace final move. Segmented/If-Range resource identity remains strict and independent. Regression: `rust/crates/nxtools-download/tests/single_stream_length_contract.rs`.

Completed ordinary-file full and compact task projections use the existing durable downloaded-byte count when `final_committed` is true, including after SQLite reopen. Media retains its original fragment/manifest size semantics. The original probe and its server observation remain unchanged.

## Failed task window and unified notification

ABDM `desktop/app/.../AppComponent.kt` routes terminal failure to its error dialog/notification while `ProgressDownloadPage.kt` offers an error action. Legacy `ViewModels/MainWindowViewModel.cs` records the failed task and `Views/MainWindow.axaml.cs` reuses its task-keyed window registry. NxTools.Next uses its existing `DownloadApplication::publish_terminal_notifications -> flush_completion_feedback -> show_completion_batch_notification -> NotificationCenter` for error details; startup seeds restored terminal states and repeated snapshots do not republish the same terminal feedback. The user-approved compact failed-window presentation keeps filename, destination, short failure/conflict state and existing recovery actions, without repeating raw error text inline. Preserve the same keyed HWND, fixed size, native-close behavior and deferred action callbacks. Production render/once-only/history tests live in `downloads/conflict_tests.rs`; no additional notification owner or timer is introduced.

`ProgressDownloadPage.kt::RenderActions` receives `downloadErrorReason` and opens the existing error page through the single-download component. Legacy `DownloadProgressWindow.axaml.cs:26–59,260–269` binds the task through construction/open/close and dispatches Resume to the existing owner. Next's failed footer chooses one Kit0.8.0 Small primary Button from the current fault: browser for unexpected HTML, fresh download for resource/range incompatibility, numbered retry for a destination conflict, and in-place retry when eligible. The same DialogFooter/client budget, original task commands, native window lifecycle and deferred callbacks remain. A fresh download is explicit; no automatic reset, extra native command, or new diagnostics owner.

## Progress window information hierarchy

User-requested layout correction: keep compact progress separate from secondary configuration. Reverified ABDM `ProgressDownloadPage.kt::{ProgressDownloadPage,RenderSettings,RenderOnCompletion,RenderPartInfo,RenderParts}`: three mutually exclusive Info/Speed/OnCompletion pages, persistent aggregate progress/actions, separate expandable part table with stable part IDs. Legacy `Views/DownloadProgressWindow.axaml` likewise has information/speed/completion tabs; constructor/base attachment, OnOpened and MainWindow's task-ID registry preserve actual Show/Close and same-window completion.

Mister `crates/app/src/progress.rs::{tabs,limiter_page,completion_page,segment_bar,connections}` has separate configuration pages and a spatial start/cursor/end segment bar, not connection-identity colors. Its GPL implementation is behavior-only reference. Next retains original persistent intentions, limiter semantics, one keyed task HWND and deferred generation-bound actions. The current layout uses Kit0.8.0 TabBar/Form/Input, one Options entry instead of duplicate footer shutdown controls, and a 440×400 expanded client while keeping 440×184 compact. Last-known same-attempt live range facts survive a snapshot gap; a new plan replaces them, and pause/retry/source changes retire them. Exact design: `.trellis/tasks/11-13-download-progress-layout/design.md`.

## Human-driven Shell and file-drag wait

Rechecked ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/CompletedDownloadPage.kt::Actions` first: long-press Open calls `openFile(useChooser=true)`, while the drag control owns `dragAndDropSource` and the completed artifact. `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/singledownloadpage/BaseSingleDownloadComponent.kt::openFile` routes the chooser through `downloadItemOpener.openDownloadItemWith` without a human inactivity deadline. Legacy `Views/DownloadCompletedWindow.axaml.cs` constructor -> Avalonia Window -> OnOpened, `DragFileSurface_PointerPressed -> StartFileDragAsync`, and `Services/WindowsFileDragService.cs::DoDragDrop/QueryContinueDrag` retain the gesture until release or Escape. Legacy Open immediately closes its completed window; Next deliberately retains its existing correlated-success-only close policy. No deeper Avalonia native Show/Close implementation was available.

Next Open With follows `DownloadApplication::dispatch_task_shell_action -> PlatformBridge::send_command_with_id -> CommandRouter::enqueue -> WindowsPlatformAdapter::OpenShellTargetAsync -> ClipboardShellTarget::LaunchCore -> OpenWithShellDialog::Show -> SHOpenWithDialog`. Native file drag follows the pressed-pointer callback -> `BeginFileDrag` -> original dedicated `NativeFileDragCoordinator` STA -> `NativeFileDragService::DoDragDrop`, with one active native operation until actual exit. Both requests must retain their original caller ID through human wait instead of using a machine deadline. The DownloadApplication must not independently expire Open With. Selection success closes only the still-current originating task-window generation; chooser cancellation is quiet and retains that window; disconnect retires original pending requests and late results cannot close replacements. Native focus, placement, capture handoff, clipboard payloads, worker admission and ordinary Open/Reveal machine deadlines remain unchanged. Existing correlated native start/state/terminal events remain the diagnostic owners; no user paths or payloads are added.

## HTTP/1.1 request header presentation

Reverified ABDM `downloader/core/src/commonMain/kotlin/ir/amirab/downloader/connection/OkHttpHttpDownloaderClient.kt::newCall` first: its original OkHttp request builder retains the accepted header values, supplies `User-Agent`, and owns `Range` and redirected `Host`. Legacy `Services/DownloadManagerService.cs::CreateHttpClient` uses `SocketsHttpHandler` with exact HTTP/1.1 and keeps certificate/proxy policy separate from request credentials.

NxTools.Next `DownloadEngine::probe_effective_blocking -> HttpDownloadClient::probe_timed -> probe_attempt` and the ordinary/segmented transfer paths share `HttpDownloadClient::build_for_snapshot`. Its reqwest client uses the official `http1_title_case_headers()` option so HTTP/1.1 sends `User-Agent`, `Accept`, `Host`, `Range`, and other header names in title case. The NodeSeek avatar endpoint rejects the otherwise identical lowercase HTTP/1.1 request with a Cloudflare403 challenge; the same reqwest/rustls client with title-case headers returns200 PNG,200 HEAD and206 Range. Header names remain case-insensitive at the protocol level; this is wire-presentation compatibility, not a change to accepted header values or credential precedence.

Keep immutable routing, strict certificate validation, redirect-sensitive-header handling, timeouts, probe concurrency/fallback, Range/If-Range validation, cancellation and artifact publication unchanged. Existing correlated probe/transfer lifecycle events remain the diagnostic owners; do not log request headers or credentials. `tests/http_client_contract.rs::http1_title_case_headers_preserve_context_across_probe_and_transfers` uses a local403-on-incompatible-presentation endpoint and verifies probe metadata plus exact single-stream and segmented bytes.

## Extensionless task icon identity

Rechecked `D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\utils\OSFileIconProvider.kt::getIcon/getIconOfFileExtension`: names without a dot use the empty extension and still query/cache the OS file-type icon. Legacy `D:\Code\NxTools\Views\MainWindow.axaml` uses its generic draft glyph; it does not own Windows icon extraction.

Next unfinished extensionless names previously produced no resource key, permanently retaining the line-art placeholder, while completed targets received Windows' generic file artwork. They now use the existing `nxtools-file-type:generic` resource and bounded correlated Shell icon request/cache. Known file types and completed actual-target icons retain their existing precedence; pending/image-load failure retains the original neutral File placeholder. No task state, row geometry, selection, native window lifecycle or file association changes. The production DownloadApplication regression asserts an actual generic extraction command, not source tokens.
