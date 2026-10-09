# NxTools.Next UI and Lifecycle Recovery Design

## Scope and decisions

This parent task coordinates four independently verifiable slices:

1. global Windows shell settings failure/compile fix (start first; the checked-in worktree changes do not currently parse/compile);
2. finish download secondary-window position and probe-drag lifecycle, preserving the dynamic connection count, segment dots and expanded details already present;
3. fix link detector recognition and floating-window lifecycle;
4. stage the wider desktop visual redesign across the shared shell, Downloads, Quick Find, Clipboard, Tools, global settings and secondary surfaces.

The first three are correctness prerequisites and should complete before broad visual migration. They retain existing product behavior. The UI redesign must not erase the user's current download/window/clipboard worktree changes; integrate with them rather than overwrite.

## Current visual direction

Use the user's Rust GPUI design brief, reconciled with existing `theme.rs`, `ui/metrics.rs`, `ui/typography.rs`, shared components and `ui-token-boundaries.ps1`:

- Canonical base palette (from the user brief): base `#18181b`; surface 1 `#1f1f23`; surface 2 `#121215`; surface 3 `#27272a`; subtle border `#27272a`; focused border `#0ea5e9`; primary text `#f4f4f5`; secondary text `#a1a1aa`; muted text `#71717a`; primary accent `#0ea5e9`; hover accent `#38bdf8`; semantic success/warning/danger colors.
- Type: retain Microsoft YaHei UI and centralize roles; page titles 16 px, titles 14 px, body 12 px, labels 11 px, metadata 10 px, monospace only for paths/technical values.
- Layout: a compact 4px-grid desktop workbench; shared client titlebar, module rail and one continuous working surface. Settings use stable navigation/content hierarchy rather than repeated decorative cards. Secondary dialogs use compact title/body/footer hierarchy and validated position lifecycle.
- Geometry: derive controls/gaps from 4px metrics; semantic 3/6/8px radii by role; focusable controls expose a visible `#0ea5e9` focus ring; preserve keyboard and pointer parity.
- Motion: short user-triggered transitions only; no decorative gradients, large shadows, idle animation or animation-frame native HWND commands.

The visual direction uses the user's exact palette and geometry instead of inheriting the current Zed-derived colors. Its distinctive device is disciplined, high-density desktop hierarchy through the active module rail and restrained surface ladder; not a field of identical cards or gradient decoration.

## Architecture boundaries

- Rust owns user settings, generation/correlation state, layout and GPUI view composition.
- GPUI owns rendering and window entities.
- C# PlatformHost owns Windows registry/NotifyIcon calls, native HWND bounds/Z-order/visibility and Win32 error codes.
- IPC preserves command ID and correlation ID from Rust to PlatformHost result. Do not report saved settings as applied before the Windows result is successful.
- Do not introduce GPUI Kit as a second in-process GPUI platform/event loop. The workspace uses only `gpui 0.2.2` plus vendored `gpui-component 0.5.1`; there is no GPUI Kit dependency. Keep the floating window in the current GPUI runtime.

## Slice A: global shell apply

Trace `NetworkSettingsApplication::apply` -> `ConfigureDesktopShell` -> `ApplyDesktopShellConfiguration` -> startup registration -> tray configuration -> correlated result. Keep config persistence semantics explicit: persisted desired configuration can remain saved after an OS failure, but the UI must name the exact failed stage and must not update the live `SystemSettings` state as if the operation succeeded. Preserve the fail-closed close-to-tray policy.

Correct both concrete syntax/contract gaps present in the worktree: `global_shell_failure_message` must match `(failure_kind, stage)` for all staged error results; the C# catch must return a `WindowsCommandResult` after deriving the native error code. Add C# tests for each failure stage (missing approved desktop executable, Run-key failure, tray failure, unexpected exception/native error) and Rust tests proving the staged result maps to actionable, payload-free UI messages and stale command IDs are ignored.

## Slice B: download secondary windows

Behavioral authority: `docs/reference-code/download.md` (legacy NxTools per-task progress/completion reuse and compact terminal transformation) and `.trellis/tasks/09-28-download-window-transition-polish/{prd,design,implement}.md` (canonical current phase/acceptance). Preserve the canonical task phase and order; do not reclassify this as a completed feature from static UI alone.

Current source already contains a live connection/segment count, an expand/collapse details control, bounded dot-style segment progress, and position-anchor handoff. Do not recreate these. Remaining acceptance is the real Windows path: drag the new/probing dialog while probing, then verify resizing/confirmed transition keeps the HWND at its actual current position; verify probe-to-task handoff places before reveal, and progress-to-failed/completed transition preserves actual native center with no stale GPUI-origin recenter. Make only the minimal native geometry/lifecycle correction demonstrated by a red-capable test. Run the canonical download window lifecycle architecture check and focused GPUI tests; report that interactive Windows smoke is still needed if unavailable.

## Slice C: link detector panel

- Pin the exact input `mage.routerapi.net` at the detector test boundary. It should be recognized as a bare web host; prepend `https://` only when invoking the browser, not when displaying/copying.
- Preserve current default click contract: clicking a URL row copies the URL and closes the panel; browser launch remains controlled by the existing “自动打开浏览器” setting. Add a visible, accessible copied/failed action result instead of silently discarding click feedback. Do not change default behavior to open the browser without a separate user decision.
- Keep one reused popup HWND. Reapply and await/handle topmost result on both creation and reuse, and set final bounds before showing/revealing.
- Make auto-hide capture-generation scoped/cancellable so an earlier timer cannot hide newer copied content; each accepted capture owns one 6-second (or configured) dwell. Do not alter the timeout based on pointer hover absent evidence/user request.
- Remove the opaque gpui-component Root background from the transparent floating root through a purpose-built transparent GPUI root/renderer (no broad vendor theme change). First prove rounded alpha survives the native Windows composition path; do not use a rounded GPUI child inside an opaque full-window Root. Keep hit testing on the visible rounded panel and verify corner click-through/behavior in Windows.
- Carry one correlation ID for the cross-process presentation operation, bounded transitions and native errors; never log URL/path contents.

## Slice D: staged whole-app UI redesign

Review/approval checkpoints by surface:

1. Foundation + representative surface: complete semantic token/focus/spacing system and shared shell. Add token architecture gates; compare representative shell screenshot and interaction tests.
2. Downloads: list/filters/detail/task windows/settings and smaller schedule/delete/completion surfaces. Preserve download lifecycle and task data.
3. Quick Find: result panel, settings pages, preview and context menu. Preserve search/selection/preview generation contracts.
4. Clipboard + Tools: history, settings, link panel and passive preview. Preserve no synchronous entity re-entry, single-window preview ownership and native bounds/show order.
5. Global network settings and notifications; final accessibility/keyboard/focus and cross-surface consistency review.

Each stage has its own visual review and interaction regression gate. Keep app/package compatibility and window lifecycles stable. No single mass rewrite; no change to state ownership or cross-process protocol solely for styling.

## Diagnostic contract

`ConfigureDesktopShell` already has an operation correlation ID; retain it through all Rust/C#/IPC stages. Record accepted/start, startup registration result, tray result, terminal applied/failed, elapsed time and Win32/HRESULT code. For link-panel operations, correlate presentation generation through topmost/bounds/visibility results. Use non-blocking bounded logging and never record clipboard text, URL/path contents, cookies, request headers or trust material.

## Rollback

Each child task is independently revertible. Keep global settings result handling, download geometry, link panel lifecycle/root rendering, and visual-token/module migration in separate commits. If a stage fails its behavior gate, roll back its presentation/geometry change without reverting unrelated user changes or altering persisted download/clipboard data.
