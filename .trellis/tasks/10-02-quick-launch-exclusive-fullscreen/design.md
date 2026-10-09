# Design: Quick Launch exclusive-fullscreen summon guard

## Verified reference and data flow

`docs/reference-code/quick-launch-research.md` documents the Quick Launch owner boundary and the 2026-10-02 Windows SDK check. The current flow is:

```text
QuickLaunchSettings (Rust config/persistence)
 -> QuickLaunchApplication::send_configuration_for
 -> ConfigureQuickLaunchCommand (typed IPC)
 -> PlatformHost ApplyQuickLaunchConfiguration
 -> native trigger registration
 -> WndProc(WM_HOTKEY) or WndProc(WmQuickLaunchGesture)
 -> PublishQuickLaunchPanelRequest -> Rust GPUI window/session
```

The low-level keyboard callback must only post a message. Detection belongs in the existing PlatformHost message loop immediately before publishing the event, so the event is not created, foreground context is not captured, and GPUI cannot begin opening the window while exclusive fullscreen is active.

## State and compatibility

- Add `QuickLaunchSettings::suppress_in_exclusive_fullscreen`, serialized with `#[serde(default = "default_true")]`; `Default` is also `true`.
- Add the matching snake_case field to `ConfigureQuickLaunchCommand`, with serde default `true` for old wire payloads.
- Rust uses the existing `persist_and_sync_quick_launch_behavior` path after the toggle changes; command correlation remains the existing envelope correlation. C# stores the setting with other current Quick Launch configuration.
- The UI goes on the General page beside summon behavior, uses existing `compact_toggle` and stable `debug_selector`/IDs, and explains “仅 Direct3D 独占全屏；无边框全屏不屏蔽”.

## Native detection and dispatch

- Introduce an injectable notification-state query interface/function at the Windows boundary. Production calls `SHQueryUserNotificationState` from `shellapi.h` and compares the successful result to numeric enum value 3 (`QUNS_RUNNING_D3D_FULL_SCREEN`). Do not depend on an API that guesses from `GetWindowRect`, and do not treat `QUNS_BUSY` as exclusive fullscreen.
- Guard both message branches in the Window Procedure: `WM_HOTKEY` calls the Quick Launch path only if not suppressed; the posted double-Ctrl gesture does the same. Other registered hotkeys remain unchanged.
- Query on the Windows message thread, not inside the low-level keyboard callback. On API failure, fail open and keep `Marshal.GetLastWin32Error`/HRESULT in a result value for diagnostics/tests; if Shell returns `E_NOTIMPL`/another failed HRESULT, do not suppress.
- Use an injectable pure policy seam: setting disabled -> allow without query; query Success + state 3 -> suppress; query Success + any other state -> allow; query failure -> allow and expose error code.

## Diagnostics

This introduces a Windows-native decision. Reuse the message-loop event boundary and bounded PlatformHost logger: keep the configuration envelope correlation for configuration-apply events; generate a fresh correlation for a locally suppressed/failing summon decision if logged. Record accepted/suppressed/failure decision, trigger kind (`hotkey`/`double_control`), state/result, native HRESULT and elapsed time. Rate-limit repeated query failures and unchanged suppressed states. Never log window titles, paths, foreground payload or key streams; diagnostics are best-effort and must not change the summon decision.

## Expected files

- `rust/crates/nxtools-core/src/lib.rs` — persisted setting and serde default.
- `rust/crates/nxtools-protocol/src/lib.rs` — typed config contract and compatibility default.
- `rust/apps/nxtools-desktop/src/quick_launch.rs` — setting update, persist and config sync.
- `rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs` — compact toggle and user-facing text.
- Rust tests for settings serialization, control behavior and config command.
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs` — message-loop suppression branch.
- A small internal query/policy helper (new file only if that is the testable seam required by existing project structure).
- `csharp/NxTools.PlatformHost.Tests/Program.cs` — native API mapping, decision and both-trigger behavior tests.

Do not modify download-window layouts/motion, the existing fullscreen geometry heuristic for the floating assistant, the hook's double-tap detector or installed artifacts.
