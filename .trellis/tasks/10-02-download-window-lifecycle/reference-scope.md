# Download Window Reference Scope

Before changing migrated behavior, read `docs/reference-code/download.md` and inspect the old NxTools source entries recorded there: `D:\Code\NxTools\Views\MainWindow.axaml.cs`, `Views\DownloadProgressWindow.axaml.cs`, `Views\DownloadProgressWindow.axaml`, `Views\DownloadCompletedWindow.axaml.cs`, plus constructor/base/native show-hide paths. Update `docs/reference-code/download.md` with any newly verified call chain/invariant before code changes.

Behavior contract: one keyed task window; progress -> terminal state transforms in place; compact terminal bounds preserve actual visual center; closing progress is presentation-only; task retry/action/focus/native-close cleanup must remain intact.

Current branch already has dynamic active connections/segments, an expanded details disclosure and bounded segment-dot rendering. Do not rebuild these. Current task phase is governed by `.trellis/tasks/09-28-download-window-transition-polish/implement.md`; reread it before implementation and preserve exact phase name/order. Remaining user acceptance is actual HWND position after dragging during probe and resizing/transitions, plus interactive Windows smoke.

Relevant current boundaries: `rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`, `task_window.rs`, `task_windows_impl.rs`; C# `WindowsPlatformAdapter.NativeMethods.cs` owns actual geometry/retry. GPUI cached origin is not evidence of current HWND position.
