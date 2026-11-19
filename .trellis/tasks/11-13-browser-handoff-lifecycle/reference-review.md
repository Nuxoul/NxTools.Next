# Browser ownership and external intake reference

## Behavioral sources

AB Download Manager browser integration1.5.2, commit `bf6811dd8a3ee2b47b94d70e13c2c7e70be7be05`, is the primary browser behavior reference. `src/linkgrabber/DownloadLinkInterceptor.ts` and `src/contextmenus/ContextMenus.ts` separate browser interception from deliberate context-menu intake. The desktop reference is `D:\Code\ab-download-manager-master`: `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/enterurl/EnterNewURLPage.kt:55` and `:218`, `shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/pages/enterurl/BaseEnterNewURLComponent.kt:65`, and `EnterNewDownloadWindow.kt:34` preserve explicit manual admission, window state, focus and close ownership.

Legacy NxTools is the product-specific external intake reference. In `D:\Code\NxTools`, `ViewModels/MainWindowViewModel.cs:591` constructs manual intake without auto-parse; `:2818` constructs external browser intake with `autoParse:true`. `Views/MainWindow.axaml.cs:1932` owns show/reuse, `:2004` defers `ParseUrlAsync` until Loaded, `:2608` passes automatic parsing for floating intake, and `:2844` owns placement/foreground. `ViewModels/AddDownloadDialogViewModel.cs:135` and `:552` own the auto-parse flag and cancellation/disposal; `Views/AddDownloadWindow.axaml.cs:20` and `:85` own constructor/opened/closed/close boundaries.

Mister0.1.0 was inspected only as behavioral evidence in `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main`. `extension/background.js:1312`, `:1334`, `:1354` and `:1547` control browser ownership using the Chromium filename callback and Firefox interception/pause/resume. `crates/ipc/src/lib.rs:426` acknowledges ordinary commands on in-memory channel admission; `crates/app/src/app.rs:6677` and `:8045` perform later confirmation. Its control principle does not establish durable acceptance. NxTools requires a persisted non-transferring Held task and verified browser cancellation before activation. No Mister code, comments, tests or assets are used.

## Current owner chain

Automatic browser download: actual `downloads.onDeterminingFilename`/Firefox `onCreated` → `interceptBrowserDownload` → `runControlledBrowserHandoff` → Native Messaging `prepare_browser_handoff` → BrowserHost → `BrowserPrepareHandoff` → original private desktop dialog receiver → original NewDownloadView probe/fixed confirmation → `BrowserAcceptHandoff` → durable Held → browser cancel → `BrowserCommitHandoff` → original scheduler. Status/abort use the same handoff ID. Manual URL/Blob/batch use their original intake commands.

External link recognition: `tools/link_panel.rs` named Download action closes its own panel before deferring `DownloadApplication::open_link_download` → `NewDownloadInitialSource::Url` → original window helper → deferred `begin_probe`. Floating actions use `tools/floating_assistant/actions.rs` → `downloads/floating_assistant.rs` → external Download source. Manual header/empty Add actions in `downloads/download_page.rs` use `None`; clipboard prefill and Paste do not start parsing.

## Preserved invariants

- Rust owns state/layout, GPUI rendering and C# native placement/show/focus. No replacement native window or control framework.
- One original NewDownloadView generation owns parsing and cancellation; cross-entity application effects run on a later application turn. Compact-to-fixed transition transfers the context rather than aborting it when the old window is removed.
- External input automatically parses but never automatically confirms or creates a task. Eligible pristine blank-window reuse retires old clipboard prefill; populated, edited, external or active drafts remain untouched.
- Browser control is established before credential/native demand. Chromium filename suggestion resolves once at the terminal boundary; no early Save As hand-back while awaiting confirmation.
- Pending has a renewable30second lease. Held is durable until explicit Commit/Abort and cannot transmit through resume, batch, queue, schedule or restart. Held never expires merely because the browser worker is unavailable.
- Cancel/window/probe failure aborts desktop pending before resuming browser. Abort failure cannot activate Held; cleanup retries the same identity. Unknown abort prevents late acceptance.
- Browser cancellation is verified before commit. An unknown cancellation result retains Held and the cancellation phase; a verified cancelled phase survives missing browser history. Commit replay starts no second task, and stale abort cannot cancel a committed task.
- Browser session storage contains only IDs, phases, timestamps and bounded options. No URL, Cookie, Header, body or credentials. Diagnostics are fixed lifecycle transitions, not per-poll events.

## Verification boundary

Production-entry extension fixtures, temporary SQLite/localhost transfer fixtures, owned IPC pipes and GPUI test windows verify behavior at the implemented seams. They do not certify installed Chrome/Firefox, real authenticated downloads, physical input, mixed DPI, accessibility provider or human visual acceptance. No package/install/registration/service/real-site operation is part of this task.
