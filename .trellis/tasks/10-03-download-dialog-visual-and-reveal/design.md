# Technical Design: Download Dialog Visuals and Explorer Reveal

## Reference authority and verified chains

- Before product-code edits, re-read the reference index and apply the required evidence order: AB Download Manager source first, the recorded legacy NxTools behavior entries next, then the current migration call chain. In this planning turn, the legacy source was opened before the direct AB Download Manager source and that order error was corrected before fixing the visual direction; the implementation pass must follow the required order afresh.
- AB Download Manager Desktop `AddDownloadPage.kt`, `ProgressDownloadPage.kt`, `CompletedDownloadPage.kt`, `ShowDownloadDialogs.kt`, and shared `BaseSingleDownloadComponent.kt` were inspected as visual/interaction references. Reusable findings: one task-keyed window, state-derived progress/failure/completion content, compact completed identity plus separated actions, and a stable body/footer hierarchy. Do not copy its Compose code or the entire tabbed page into NxTools.Next.
- Old NxTools entries directly inspected in `D:\Code\NxTools`: `Views\AddDownloadWindow.axaml`, `Views\DownloadProgressWindow.axaml(.cs)`, `Views\DownloadCompletedWindow.axaml(.cs)`, `ViewModels\MainWindowViewModel.cs`, `Services\WindowActivationService.cs`. Preserve one task window, compact URL/probe -> confirmed-add flow, presentation-only close, in-place terminal transform, open/reveal/drag/close, Explorer file selection and directory fallback.
- Current GPUI path: `NewDownloadView::render -> Popover::content -> delayed_start_calendar + time/action controls`; focused production seam is the mounted `NewDownloadView` `VisualTestContext` and `debug_bounds` selectors.
- Current task content path: `DownloadTaskWindowView::render -> download_task_window_shell -> phase body/footer`; `DownloadWindowLifecycle` owns identity and phase transforms. Do not introduce a second task-window owner or move native resize/center policy into GPUI styling.
- Current Reveal path: `DownloadTaskWindowView` click -> deferred `DownloadApplication::open_task_artifact_from_task_window` -> correlated `OpenShellTarget(SelectInExplorer)` -> PlatformHost reader -> STA `ShellTarget::Launch` -> `ExplorerShellActivation.LaunchAndActivate` -> foreground native result -> Rust `PendingDownloadArtifactAction`. Only a correlated success for the same HWND + generation closes; failure, timeout, disconnect and stale success keep/suppress close.

## Visual direction

Use an editor-like compact file handoff, not a SaaS card stack or imitation of Avalonia chrome:

- Palette: existing Zed-dark tokens (`canvas #1E1F22`, `surface #242529`, `raised #2A2B2F`, `accent #4791BF`, `success #6FBF8F`, `danger #FF6B78`). Borders stay subordinate; reserve semantic color for active progress/success/error and primary action.
- Typography: existing Microsoft YaHei UI / Consolas roles and shared scale (`16/14/12/11/10 px`); filenames are the dominant scan line, path/state are secondary, measured values use compact regular body/label rather than extra tiny text.
- Controls: shared 28 px action tier / 4 px control radius, stable horizontal and vertical alignment, one accent primary per action row, actual visible button as the debug/hit-test target.
- One visual focal point: a small file identity glyph paired with the current task state (success for completed; transfer accent for progress); avoid decorative gradient/card repetition.

Progress/task window wireframe:

```text
┌ titlebar: current state ────────────────────────── × ┐
│ [file/status] filename                    percent    │
│ ━━━━━━━━━━━━━━━ transfer progress ━━━━━━━━━━━━━━━━━ │
│ downloaded / total   speed   remaining   connections │
│ destination path / bounded error detail              │
├───────────────────────────────────────────────────────┤
│ details / locate        cancel   pause-or-resume      │
└───────────────────────────────────────────────────────┘
```

Completed window wireframe:

```text
┌ 下载完成 ───────────────────────────────────────── × ┐
│ [file glyph]  ✓ 下载完成                              │
│               filename                                │
│               size · destination                     │
├───────────────────────────────────────────────────────┤
│ file drag       打开所在目录     [打开文件]           │
└───────────────────────────────────────────────────────┘
```

Add/probe remains a two-stage form: titlebar, full-width URL/paste, one aligned destination/name/category area and anchored actions; after probing, status/size/capability occupy one quiet metadata rail. Failure remains actionable in-window. No new decorative card for each group.

Schedule Popover wireframe:

```text
        ‹          2026年10月          ›
        日    一    二    三    四    五    六
        [bounded Sunday-first date grid]
        时间    [hour] : [minute]
        ───────────────────────────────
        [清除]                    [确定]
```

Keep it as the current controlled/draft GPUI Popover. Measure the rendered `304 x 240` shell at the real production seam; if separating time and actions cannot fit without clipping, adjust the smallest bounded overlay height using the failing bounds test. Do not move/resize the parent form for opening, selecting, clearing, committing or outside dismissal.

## Explorer behavior and Windows boundary

- Rust owns the completed action, operation correlation ID, originating task/window generation, retryable UI outcome and close policy. C# owns the filesystem/Shell HWND discovery, Explorer target selection, restore/foreground calls and native result/error. Keep the existing typed `open_shell_target` wire contract unless a red boundary proves an origin HWND is necessary; any such field must carry only HWND (never path/title) and all Rust callers/protocol tests must be updated in that same slice.
- Define a small injectable Explorer activation/Shell seam so PlatformHost tests can distinguish: matching visible directory window; matching minimized window; an unrelated Explorer window; Shell process reuses an existing process instead of yielding a durable child process; delayed matching HWND; SetForegroundWindow denial; wrong/stale window foreground; valid final foreground readback; file selection in a pre-existing directory; and a directory target when no matching window exists. Verify `GetForegroundWindow()` against the intended root HWND after bounded retry; a process-start result or `SetForegroundWindow` BOOL alone is not success.
- When the final file exists, retain legacy `explorer.exe /select,"<file>"` semantics even if its parent directory window already exists. When the target is an existing directory, open/focus that directory whether reused or newly created; do not select the directory in its parent. When the final file is missing, Rust resolves the task's save directory before issuing the command. No target paths/titles/file names in diagnostics.
- Do not fight the Windows foreground lock with unbounded retries, high-frequency polling, or focus-stealing topmost toggles. Retry only within a small measured/bounded window and return a typed failure with native/stage metadata if Windows does not grant foreground. Only claim success after readback.
- Prefer existing task-window success/failure behavior. A failed/timeout/disconnected Reveal remains retryable, and a late success after expiry cannot close the old or replacement task window. Repeated reveal requests remain bounded and correlated.

## Diagnostics

Use the existing operation correlation ID through Rust, IPC, and C#. Record accepted/start, coarse stage (`existing_window_found`, `shell_launch_requested`, `selection_requested`, `foreground_confirmed` or fixed failure stage), terminal result, elapsed time and native error code. Do not log raw target path, URL, filename, window title, credentials, headers, cookies, or per-attempt/high-frequency retries. Diagnostics are best-effort and cannot change Shell result.

## Rollback and compatibility

- UI layout can roll back independently from shell activation behavior; neither changes task state/queue or data schema.
- Preserve native OLE drag on the still-pressed GPUI pointer boundary, all current role selectors/close behavior, and per-task window generations.
- No installer, release artifact, protected service, PlatformHost/Indexer process, or user task data is part of this work.
- A screenshot/automated GPUI test is not Windows visual proof; manually inspect actual displayed scaling, taskbar/foreground behavior, and target selection before acceptance.
