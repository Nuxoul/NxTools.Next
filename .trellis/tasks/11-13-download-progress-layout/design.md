# Download progress layout

## Scope

The user rejects the expanded progress window's crowded controls, mismatched speed input and flickering segment coverage. Keep the existing task/window/entity and download behaviors. Collapsed client stays 440×184 DIP; expanded client becomes 440×400 DIP. Failed/completed clients, native centered placement, presentation-only close and same-HWND completion remain unchanged.

## References

ABDM `desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/pages/singleDownloadPage/ProgressDownloadPage.kt::{ProgressDownloadPage,RenderSettings,RenderOnCompletion,RenderPartInfo,RenderParts}` separates Info/Speed/OnCompletion pages, keeps progress/actions outside them, and keys spatially ordered parts. `ShowDownloadDialogs.kt` → `DesktopSingleDownloadPageComponent.kt` → shared `BaseSingleDownloadComponent.kt` → app-utils `CustomWindow.kt` owns keyed presentation and native Compose windows. Legacy `Views/DownloadProgressWindow.axaml` has separate information/speed/completion tabs; its constructor → WindowAnimationService.Attach → OnOpened and MainWindow's task registry own Show/Close and completion transformation.

Mister `crates/app/src/progress.rs::{body,tabs,limiter_page,completion_page,segment_bar,connections}` uses one of three pages, a persistent meter/action row and a separately expanded segment table. Its segment paint uses actual start/cursor/end in file coordinates, not transient connection identities. The app polls active snapshots at roughly 250 ms. GPL source is behavior-only reference; no code, tests or assets are reused. Keep Next's persistent completion intentions and same-window completion instead of Mister's window-owned intentions/separate completion window.

## Composition

- Compact progress retains filename, observed server, official aggregate Progress and stable two-column metrics.
- Expanded secondary area uses released Kit 0.8.0 `TabBar::underline().small()` and `Tab::new().label`, with Connection details, Speed limit and On completion pages. Only one page is mounted. Same local selected-tab state survives progress updates.
- Connection page contains resume/connection facts, static actual-byte coverage and the original official Table with a bounded scroll viewport. No editor inside the table page.
- Speed page uses official `Form::vertical`, `Field` and Medium Input (32 DIP), one original InputState and suffix unit. Original Enter/Blur submission, Escape restore, dirty/pending/error and effective task/global limit remain authoritative.
- Completion page reuses the existing persistent completion command/menu. No duplicate shutdown checkbox. Footer has one Small outlined Options entry and the original Small cancel/pause/resume commands; `Toolbar.content` avoids ghost/compact adaptation. Options opens Speed in the expanded area. The identity disclosure opens/collapses the same secondary area.
- Keep original themes/typography; no dependency/vendor/native changes. All cross-entity effects remain deferred and window/generation checked.

## Stable coverage

The application intentionally clears its transient progress cache on authoritative snapshot refresh. A same-attempt downloading task can therefore arrive at the existing view with no live segment payload while only an older durable checkpoint is available. Retain the last known complete segment projection in that view until a new nonempty projection replaces it. Do not union old plans with new split ranges. Retire retained live ranges on task/attempt/source/status changes; paused/reopened windows use persisted durable ranges. Keep unknown data unknown. Render contains no IO, sampling timer or diagnostic stream.

## Verification

Red-capable production view tests cover snapshot-gap coverage, replacement split ranges, retirement, mounted tabs, 32-DIP input, footer/table viewport, pointer/keyboard and source identity. Check actual captures separately from geometry; do not claim physical/native/DPI/user approval from headless tests. Reuse current speed/intent/window lifecycle tests with the new navigation, preserving command and persistence assertions. Use one Cargo lane with workstation memory settings. No packaging/service or installed application launch.
