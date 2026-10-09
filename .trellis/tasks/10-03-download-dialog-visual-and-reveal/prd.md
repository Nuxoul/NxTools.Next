# Improve Download Dialogs and Explorer Reveal

## Goal

Make the small download windows clearer and visually coherent, correct the misaligned controls reported after choosing `稍后下载`, and make `打开所在文件夹` mean that the matching Explorer window is actually in the foreground before NxTools reports success and dismisses the completed window.

## Confirmed Context and Evidence

The user supplied three screenshots under `%APPDATA%\NxTools\NxTools.Next\data\clipboard-images\`: `97b71bc415004f25038e0c400d6b7cb38690fd2c0b7db116198858c670a8a464.png`, `31fe5a078ae075fcd605ee044ef9b9f9b307214361030d519281a214646a50a0.png`, and `0b0c09a7a2d153edb5cd44c362b9c59930a6dbc5fb026fb295b641cc8ad3780f.png`. They identify the scheduling action alignment and inconsistent Explorer foreground as visible symptoms. A local screenshot preview was attempted but the browser guest is unavailable, so source plus the user's description are the available evidence; interactive screenshot comparison remains required.
- UI owner is `rust/apps/nxtools-desktop` (`nxtools-desktop`). Windows Shell ownership is `csharp/NxTools.PlatformHost` through the existing typed `open_shell_target` command.
- The legacy product behavior index `docs/reference-code/download.md` is the behavior authority. Direct checks in `D:\Code\NxTools` verified:
  - `Views/AddDownloadWindow.axaml` keeps a compact URL/probe stage and a confirmed form with stable bottom actions.
  - `Views/DownloadProgressWindow.axaml(.cs)` keeps one progress window per task, changes that same window to compact completion, and offers open/reveal/drag/close actions; closing a progress window does not itself pause/cancel the transfer.
  - `ViewModels/MainWindowViewModel.cs::OpenTaskFolder` selects a file in Explorer, opens an existing directory directly, and falls back to the save directory if the file is missing. Legacy `OpenPathInExplorer` launches Explorer and catches failures; it does not prove the matching Explorer HWND reached the foreground.
- AB Download Manager source was checked as a visual/interaction reference:
  - `desktop/app/.../pages/addDownload/single/AddDownloadPage.kt` keeps URL/file configuration aligned and separates resource metadata from actions.
  - `desktop/app/.../pages/singleDownloadPage/ProgressDownloadPage.kt` groups tabbed detail content apart from a stable progress/action area.
  - `CompletedDownloadPage.kt` uses one compact file-identity area and a divided action footer for open, folder, drag, and close.
  - `ShowDownloadDialogs.kt` and `BaseSingleDownloadComponent.kt` keep one task-keyed window and transform it when completion is allowed. Implementation must re-read the source material in the project's required sequence: behavior index, AB Download Manager reference, legacy NxTools behavior entries, then current migration call chain.
- NxTools.Next already has a shared task-window shell and stable progress/failure/completion lifecycle. Preserve it and retain the existing file/drag/task actions; redesign presentation, not task semantics.
 - Production schedule coverage now measures the Clear/Confirm hit targets, action row and Popover containment; the footer baseline regression separately checks the add-form controls. These are bounds tests, not native-DPI screenshot proof.
 - The completed Reveal call chain remains `DownloadTaskWindowView -> DownloadApplication -> typed OpenShellTarget -> PlatformHost STA -> ShellTarget -> ExplorerShellActivation -> correlated identity-guarded close`. The injected activation seam now verifies the matching Explorer root is still the foreground window before success.
 - Screenshot paths remain available but were not visually previewed here; manual screenshot/DPI comparison is still required.

## Requirements

1. **Download-window visual hierarchy:** Restyle the small add/probe, progress, failed and completed surfaces with one cohesive hierarchy based on existing NxTools.Next Zed-dark theme and shared UI tokens. Keep the file identity and current actionable state prominent; use quieter text/path metadata and stable row/action baselines instead of competing cards. Preserve client titlebar behavior and client-drawn close controls.
2. **Delayed-start / `稍后下载` alignment:** Keep the current in-window GPUI Popover and draft/commit/outside-dismiss behavior. Arrange its calendar, time selection, `清除`, and `确定` as a deliberate compact hierarchy. `清除` and `确定` must share an action row with a common vertical center/baseline, standard 28 px action tier, stable horizontal inset, non-overlapping hit targets, and no clipping outside the current bounded overlay unless a measured regression proves a height adjustment is necessary.
3. **Foregrounded Explorer reveal:** For the completed task window's `打开所在文件夹` action, report success only after readback confirms the matching Explorer top-level HWND (including a root-owned foreground child if appropriate). Distinguish an existing matching directory window, an existing unrelated Explorer window, and a newly opened/reused Shell path. When the final file exists, select that file even in an already-open matching directory; when the target is a directory, open/focus that directory instead of selecting it in its parent; when the final file is missing, preserve the old fallback and open/focus the task save directory. If activation is denied or the expected HWND never appears, return bounded failure with native error/stage, show retryable feedback, and do not dismiss the completed window. Do not log target path, URL, filename, title, credentials, or clipboard content.
4. **Preserve interactions:** Retain one keyed task window and in-place Progress -> Failed/Completed transitions; current open, reveal, file-drag, retry, cancel, pause/resume, details/segments, close and notification behavior. No download-engine, queue, transfer, persistence, settings, IPC wire-schema, packaging, service or deployment changes.
5. **Evidence and visual verification:** Add/red-capable tests at the production GPUI popover/action bounds and the Explorer foreground-readback boundary before any correction. Record that automated bounds/Win32 tests do not replace a manual Windows visual and Explorer smoke.

## Out of Scope

- Redesign of the main Downloads workbench/list/detail inspector.
- Changing the download engine, queue policy, saved task data, start/schedule meanings, completion preferences or legacy task-window dismissal/focus policy beyond making explicit Reveal success mean a foreground-confirmed Explorer window.
- Reimplementing the existing native OLE file-drag boundary or adding a second drag framework.
- Installing or launching a validation build, overwriting official installers, or stopping/restarting PlatformHost or Indexer services.

## Acceptance Criteria

 - [x] The small add/probe, progress, failed, and completed windows use a shared file/state hierarchy with quieter destination text and retain their existing state-specific actions. Manual visual acceptance remains separate below.
 - [x] A production `NewDownloadView` GPUI regression measures the schedule Popover shell, time row, visible Clear/Confirm hit targets, and action row; it checks tier, baseline, insets, non-overlap, and containment. The existing draft/commit/clear/outside-dismiss regression passes, and opening does not move the parent form.
 - [x] Existing production tests still prove schedule commit, clear, reopen, outside-dismiss, and noncommit draft behavior.
 - [x] Injected Explorer activation tests cover an existing matching window, minimized restore, Shell reuse/delayed HWND publication, destroyed HWND, foreground denial/wrong window, file selection, direct directory open, and missing-file save-directory fallback.
 - [x] The completed-window GPUI test covers failed result, duplicate request suppression, timeout, disconnect, late success and generation-safe close. Only the correlated successful native foreground readback closes the original completed window.
 - [x] Focused and full Rust desktop tests pass (428/428), `cargo fmt --all -- --check`, the download UI lifecycle architecture gate, five PlatformHost Explorer activation cases, the non-DOpus Shell responsiveness test, and PlatformHost Debug build (0 warnings/errors). Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`.
 - [ ] Pending manual Windows smoke: compare the supplied screenshots at actual DPI, inspect the visible download stages and schedule hit targets, and exercise foregrounding/selection with existing and newly opened Explorer windows. Screenshot files were not previewed here, and this manual interactive smoke was not run.
 - [x] No download-engine behavior, IPC wire schema, installer, release artifact, protected service or unrelated working-tree changes were modified by this slice.