# Restore Desktop UI and Window Reliability

## Goal

Resolve the user's concrete NxTools.Next reliability/UI issues first, then redesign all desktop surfaces using the supplied Rust GPUI design specification in reviewable stages. Preserve existing feature behavior unless a behavior change is explicitly requested, and integrate with—not overwrite—the extensive existing worktree changes.

## User-Reported Outcomes

1. **Global settings:** saving global settings reports “设置已保存，但 Windows 启动项或托盘设置应用失败”. The user expects Windows startup/tray settings to apply correctly or a precise actionable failure.
2. **Download secondary windows:** the user asks whether the previously requested download secondary-window UI was actually completed. Original requests covered stable position through probe/downloading/completed states, dragging while probing without layout corruption, live dynamic connection count, old-NxTools dot-style segment progress, and an expandable segment-details view.
3. **Link detector panel:** copied hosts such as `mage.routerapi.net` should be recognized; the popup should stay topmost, disappear on its configured timer, respond visibly to clicks, and have real transparent rounded corners. Current appearance is unacceptable.
4. **Whole desktop:** redesign the complete application according to the user-provided GPUI design specification—not just the link panel.
5. **GPUI Kit:** answer whether this one auxiliary panel can use GPUI Kit without splitting the app's rendering/runtime architecture.

## Confirmed Repository Facts

### Worktree and task constraints

- The user explicitly approved creating a Trellis planning task and asked to continue the current task. No product-code edits have been made in this planning session.
- The worktree already contains extensive uncommitted Rust/C#/IPC/UI/reference changes. Preserve all unrelated and unrecognized changes; do not revert, format wholesale, or commit/push/package/shutdown as part of this task.
- The existing download transition task `.trellis/tasks/09-28-download-window-transition-polish/` is still marked `in_progress`. Its exact phase/step names and order remain canonical; new download work must be limited to unfinished acceptance rather than renumbering or rewriting its completed implementation.
- Old NxTools has no verified equivalent link-detector popup, so the link detector is new Next behavior; `docs/reference-code/clipboard.md` records its current ownership/native constraints.

### Global Windows shell settings

- `NetworkSettingsApplication::apply` persists the desired `AppConfig`, sends a correlated `ConfigureDesktopShell` command, and only publishes new live `SystemSettings` after a successful Windows result (`rust/apps/nxtools-desktop/src/network_settings.rs`). A failed native result deliberately fails close-to-tray policy and surfaces an error.
- C# `ApplyDesktopShellConfiguration` applies approved protected executable lookup, current-user Run-key registration, then tray configuration (`csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`). C# returns stage tags such as `desktop_executable_unavailable`, `startup_registration_failed`, and `tray_icon_failed`; Rust should map those to the actionable error text now added by the in-progress change.
- The current uncommitted diff contains a concrete build blocker in both languages: `global_shell_failure_message` has a missing match pattern for the `desktop_executable_unavailable` stage, and `ApplyDesktopShellConfiguration`'s catch block computes `nativeErrorCode` but has no `return new WindowsCommandResult(...)`. Fix those in the global-shell child first, with regression tests. Do not mislabel persisted-but-unapplied values as applied; keep config-persistence semantics explicit.

### Download secondary windows

- Original user requirements in the previous session were: preserve native position across probe/progress/completion transitions; dragging during probing must not corrupt layout; show dynamic active connection/thread count rather than stale “8”; use old NxTools-style dots for segments; provide a disclosure to inspect detailed segment progress.
- Current source already contains a dynamic `active_connections`/segment count (`task_window.rs:527-655`), an expand/collapse details action (`:702-738`), and bounded dot-style segment progress plus summary (`:791-835,892-981`). Those three must be preserved and verified, not reimplemented.
- Current source also has native position-anchor handoff and phase lifecycle changes; `.trellis/tasks/09-28-download-window-transition-polish/implement.md` records extensive automated checks and specifically leaves Windows interactive drag/resize, probe/handoff and visual smoke outstanding. Do not declare the download window done based only on source or compilation.
- `docs/reference-code/download.md` is the behavior source for reused per-task progress/completion windows and compact in-place terminal transformation. The child must retain that lifecycle and run `tests/architecture/download-ui-lifecycle.ps1`.

### Link detector

- `clipboard.rs::handle_link_detector_capture` processes only allowed, non-empty text clipboard captures. `tools/link_detector.rs` recognizes explicit HTTP(S), existing local drive/UNC paths and bare domains with an allowlisted TLD including `.net`; its existing regression uses `image.routerapi.net`, not the exact user text `mage.routerapi.net`. Neither exact host has been verified by live packaged interaction in this session.
- Current settings default is detection enabled, 6-second auto-hide, browser auto-open disabled, copy-only enabled (`nxtools-core/src/lib.rs:651-671`). The tools UI calls these “仅复制” and “自动打开浏览器”. A URL row click closes the panel, queues the configured route and writes the text to the clipboard; default copy-only currently has no prominent feedback, explaining why it can appear inert. Preserve this configured behavior and add visible success/failure feedback; do not silently change the default to open a browser.
- Creation requests topmost but discards the native result; reuse moves/resizes without reapplying topmost (`clipboard.rs:960-1014,1023-1075`). C# `SetWindowTopmost` can verify actual HWND Z-order and reports success/failure.
- `LinkPanelView::arm_auto_hide` spawns a non-cancelled timer for a window handle; reuse arms a new timer without invalidating the previous capture's timer (`tools/link_panel.rs:115-147`). This can hide newly restaged content too early.
- WindowOptions requests a transparent native background, but `gpui_component::Root` paints `window_border()` and an opaque full-size themed background (`vendor/gpui-component/src/root.rs:396-413`). Rounded child clipping alone therefore cannot produce genuine transparent HWND corners. The clipboard reference explicitly warns about rectangular native shells/hit testing (`docs/reference-code/clipboard.md:253-260`).
- The workspace uses `gpui 0.2.2` and `gpui-component 0.5.1`, with a single GPUI app event loop; no `gpui-kit` or `gpui-pre` is present in workspace manifests/lockfile. Do not add a second in-process platform/event-loop runtime for this panel. Keep it on existing GPUI; a separate process would add IPC, packaging and lifecycle complexity without solving a demonstrated requirement.

### Whole-app UI coverage and supplied design system

The desktop contains Downloads, Quick Find, Clipboard and Tools modules, a shared titlebar/sidebar, global network settings, tool settings windows, notifications, popups, preview surfaces and download task/dialog windows.

The supplied user specification is canonical for redesigned surfaces:
- semantic colors: `bg-base #18181b`, `bg-surface-1 #1f1f23`, `bg-surface-2 #121215`, `bg-surface-3 #27272a`, `border-subtle #27272a`, `border-focused #0ea5e9`, `text-primary #f4f4f5`, `text-secondary #a1a1aa`, `text-muted #71717a`, `accent-primary #0ea5e9`, `accent-hover #38bdf8`, and status success/warning/danger tokens;
- strict 4px spacing grid; compact controls (24/28/32 px); radius 3/6/8 px/pill by role;
- high-density desktop layout; no decorative oversized card/gradient treatment; semantic focus rings and keyboard-equivalent Enter/Space; restrained user-triggered motion, no idle loops;
- reusable semantic theme/tokens/components rather than scattered hex values.
Existing theme/metrics/typography/components are partial and must be reconciled to this explicit user palette, not treated as overriding it.

## Requirements

### R1 — Global settings correctness (first implementation child)
- Repair the current Rust/C# compile errors in the staged failure-result mapping/return path.
- Preserve correlated result routing; distinguish missing protected executable, startup registration, tray icon, enqueue, disconnect and timeout.
- Keep saved-config versus applied-runtime status honest and preserve fail-closed close-to-tray behavior.
- Add Rust and C# tests for every staged result and stale/late result handling. User-facing errors are actionable and never expose sensitive payloads.

### R2 — Download secondary-window completion
- Retain the existing dynamic connection/segment count, dot-style progress and disclosure/expanded detail; add no duplicate UI.
- Follow the exact phase order in the existing download task plan.
- Verify native geometry after user drag during probe: probe result resize, confirmed-layout resize, task handoff, and progress-to-failure/completion center preservation must use actual HWND bounds rather than stale GPUI origin.
- Repair only a failing demonstrated lifecycle boundary; preserve task-window identity, completion transform, actions, and native close behavior.
- Run focused tests and architecture gate; perform Release/Windows interactive drag/resize/handoff smoke before declaring the interaction done, or explicitly leave it as a user verification blocker.

### R3 — Link detector panel correctness
- Add exact `mage.routerapi.net` regression plus case/punctuation variants; recognized bare host remains displayed/copied without scheme; browser route adds `https://` only at launch boundary.
- Keep current default action/configuration: copy-only by default, optional auto-open remains opt-in. Add obvious copied/failed feedback to make clicks observable.
- Keep one reusable panel HWND, set final physical bounds before reveal, and confirm/reapply topmost on both initial show and restage; handle failure results.
- Make timer ownership generation-scoped/cancellable: restaging a capture restarts the configured default 6-second timer and a stale timer never hides the new capture.
- Render on a transparent GPUI root that does not paint an opaque full-window background; validate native transparency, corner hit-testing and mouse clicks on Windows. Keep Rust state/layout, GPUI rendering and C# native calls in their current ownership layers.
- Keep diagnostics bounded, correlated across Rust/IPC/C#, and payload-free (no URL/path/clipboard content).

### R4 — Complete application UI redesign in reviewable stages
- Use the exact color, density, spacing, radius, typography, keyboard/focus and motion rules above.
- First complete design tokens/shared controls and the shared application shell; then migrate Downloads, Quick Find, Clipboard + Tools, and finally global settings/notifications/secondary surfaces in that order.
- Every stage must preserve existing interactions/domain behavior and pass its unit, GPUI lifecycle, architecture and visual review gates before the next stage.
- Provide before/after screenshots for the shared shell and each stage when the environment supports capture; document any surface that cannot be visually reviewed rather than calling it finished.
- Do not perform one mass rewrite or re-platform the app.

## Out of Scope

- Any unrelated worktree cleanup, broad revert, commit, push, release packaging or automatic shutdown.
- Changing download engine/protocol/domain semantics, clipboard persistence/capture policy, or the link click default without explicit need and approval.
- Introducing a second GPUI runtime, separate GPUI Kit process, webview renderer, global UAC/service/registry/ACL changes.
- Declaring success solely from compilation, static source inspection, or a screenshot where interaction lifecycle is required.

## Acceptance Criteria

 - [x] **R1:** `nxtools-desktop` and PlatformHost compile; staged failure tests prove exact actionable Rust message per Windows failure result, valid correlation/command routing, no false applied state, and fail-closed live tray policy.
 - [x] **R2:** canonical download lifecycle phase/step names and order are retained; focused GPUI tests and `tests/architecture/download-ui-lifecycle.ps1` pass; live drag-during-probe, post-probe resize and phase-transition position are verified on Windows or explicitly marked awaiting user smoke.
- [ ] **R3:** exact `mage.routerapi.net` detection, expected clipboard feedback/configured route, generation-safe 6-second hide, one-window reuse, topmost success/failure and final-bounds-before-show gates pass; actual transparent corner rendering and pointer hit testing are verified on Windows.
- [ ] **R4:** each of the five UI stages passes token-boundary checks and its module interaction regressions; documented visual acceptance covers all major windows/surfaces and no product workflow changes without a specific requirement.
- [ ] No unrelated pre-existing worktree edit is lost; no sensitive clipboard/link/request data enters diagnostics.

## Decisions and Deferred Items

- Click behavior is resolved from the existing persisted defaults and settings labels: URL rows continue copy-only by default, optional browser launch stays opt-in; add visible feedback. This is a correction of silent feedback, not an unapproved behavior change.
- Auto-hide remains the existing configurable dwell (default 6 seconds); every new capture restarts it. Hover must not extend it unless the user later asks for that behavior.
- GPUI Kit is not introduced for one panel; use the existing GPUI runtime. The user requested the capability answer, and current workspace evidence does not support mixing a second GPUI platform runtime.
- No unresolved planning decision blocks the first implementation slice. User review/approval of this final plan is still required before starting any child task.
