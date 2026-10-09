# Suppress Quick Launch only during exclusive fullscreen

## Goal

Add a Quick Launch setting so pressing the configured summon gesture (default double Ctrl) does not open the panel while a Direct3D application is in **exclusive fullscreen**. Borderless fullscreen and ordinary maximized windows must keep opening Quick Launch.

## Evidence and constraints

- Quick Launch configuration is persisted by `nxtools_core::QuickLaunchSettings`, translated in `QuickLaunchApplication::send_configuration_for`, and delivered to PlatformHost as `ConfigureQuickLaunchCommand`.
- Native summon sources converge in `WindowsPlatformAdapter.EventWindow`: `WM_HOTKEY` and the deferred `WmQuickLaunchGesture` posted by the low-level double-Ctrl hook. The hook callback itself must stay fast and must not call Shell/COM APIs.
- Windows SDK `shellapi.h` defines `QUNS_RUNNING_D3D_FULL_SCREEN = 3` as an exclusive-mode Direct3D application. `QUNS_BUSY` is only a heuristic that can also mean Presentation Settings; rectangle-equals-display tests misclassify borderless fullscreen.
- Do not claim detection for every legacy/non-Direct3D exclusive API. A query error is fail-open so a broken probe cannot leave Quick Launch permanently unavailable.
- Preserve all existing uncommitted user work. The staged whole-app UI task is still planning; download-secondary UI and motion work are not in this change's scope.

## Requirements

- Add a persisted setting, default enabled for newly created settings and for old config files with the field absent.
- Present a keyboard/pointer-accessible toggle in Quick Launch > 常规, labeled plainly (for example “独占全屏游戏时不呼出速启”) with a description explicitly stating that borderless fullscreen is not suppressed.
- Synchronize this setting through the existing typed `ConfigureQuickLaunchCommand`. Do not add a parallel protocol or a second event loop.
- Check the Windows exclusive-fullscreen state at the PlatformHost message-loop boundary, immediately before publishing the summon request, for both registered `WM_HOTKEY` and posted double-Ctrl gesture. Do not hide the panel after Rust has already received/opened the request.
- Suppress only when the setting is enabled and `SHQueryUserNotificationState` succeeds with exactly `QUNS_RUNNING_D3D_FULL_SCREEN`. `QUNS_BUSY`, borderless/maximized geometry, presentation mode and unknown states do not suppress.
- If the Windows query fails, allow the summon (fail-open), preserve the HRESULT for bounded diagnostics, and do not let diagnostics affect hook/message processing.
- Add regression coverage for defaults/backward-compatible deserialization, IPC round-trip, UI toggle persistence/synchronization, exclusive D3D suppressing both summon sources, borderless/windowed remaining allowed, disabled setting allowing summon, and query failure being fail-open.
- Keep diagnostics low-noise: one correlation ID per logged decision, semantic state transitions only, no per-key or raw title/path logging. Rate-limit repeated API failures.

## Acceptance criteria

 1. [x] A pure decision test proves only the exact successful `QUNS_RUNNING_D3D_FULL_SCREEN` result suppresses when enabled.
 2. [x] C# message-loop seam tests both double-Ctrl's posted message and registered `WM_HOTKEY`; suppressed events publish no `QuickLaunchPanelRequestedEvent`.
 3. [x] Borderless fullscreen (including full-display rectangle), maximized/windowed, `QUNS_BUSY`, disabled setting and API error all preserve normal summon behavior.
 4. [x] The toggle survives save/load and an absent field defaults to enabled; Rust UI and typed configuration message reflect the same value.
 5. [ ] Relevant `nxtools-desktop` tests, PlatformHost build/full test harness, `cargo fmt --check`, and `tests/architecture/ui-token-boundaries.ps1` pass when memory permits.
 6. [x] No download/other secondary-window visual changes are included; no package, install, restart or process termination is performed.

## Out of scope

- Suppression for borderless fullscreen, Presentation Mode, arbitrary window-size/display-rectangle heuristics or every legacy/non-D3D exclusive API.
- Hiding/minimizing the desktop, changing the selected hotkey, or changing double-Ctrl timing semantics.
- Redesigning download secondary windows or their transitions; those remain in the separate staged UI task.
- Shipping/packaging/restarting the currently installed PlatformHost.
