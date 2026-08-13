# Download manager reference implementation index

Last verified: 2026-08-11.

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

## Resolved phase-0 decisions

- CLI connects to a current-user local download pipe and starts the packaged desktop in background mode when
  the download host is absent. It never opens SQLite directly.
- Download command, storage and CLI v1 contracts are recorded in `contracts/`.

## Open research items

- The final authenticated browser transport: Native Messaging only, or Native Messaging plus a bounded loopback
  channel for large Blob payloads.
- HLS/DASH shared protocol seam must wait until the HTTP implementation and one media implementation both exist.
