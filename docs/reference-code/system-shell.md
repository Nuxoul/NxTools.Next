# Product shell, startup and tray reference index

Last verified: 2026-08-12.

## User-visible contract

- Product-global settings open from the main title bar, separate from module-specific settings.
- The main client title bar supports native drag, minimize, maximize/restore and close.
- Closing either exits or hides the main window to the notification area according to one persisted
  Rust setting. Hiding must keep background download, clipboard and quick-launch services alive.
- A notification-area left click restores the same main window. Its right-click menu provides
  `打开 NxTools` and `退出`; explicit exit bypasses the hide-on-close policy.
- Optional current-user startup launches `NxTools.exe --silent-start`; silent startup creates the
  product runtime without showing or activating the main window.
- Release desktop is a Windows GUI executable and must not create a debug console.

## Old NxTools: behavioral source of truth

Root: `D:\RiderProject\NxTools`

| File | Verified responsibility |
| --- | --- |
| `Program.cs` | Parses `--silent-start`, initializes logs and enforces one desktop instance. |
| `App.axaml.cs` | Creates the main window minimized and out of the taskbar for silent startup. |
| `Services\StartupLaunchService.cs` | Owns the HKCU `Software\Microsoft\Windows\CurrentVersion\Run` value and quotes the current executable with `--silent-start`. |
| `Models\AppState.cs` | Persists `MinimizeToTrayOnClose` and startup behavior. |
| `ViewModels\MainWindowViewModel.cs` | Loads/saves both settings and exposes their user-facing summaries. |
| `Views\SettingsWindow.axaml` | Places close behavior and silent startup together in product settings. |
| `Views\MainWindow.axaml.cs` | Cancels ordinary close when hide-to-tray is enabled; creates one tray icon, restores on left click, and exposes Open/Exit menu actions. |
| `Views\MainWindow.axaml.cs::TitleBar_PointerPressed` | Starts native move on left press and toggles maximize on double click. |

Verified old lifecycle:

```text
ordinary close + hide enabled
  -> cancel native close
  -> ensure one tray icon
  -> remove taskbar presence + hide main window

tray left click / Open
  -> restore normal state
  -> show in taskbar
  -> show + foreground the same main window

tray Exit
  -> mark explicit exit
  -> hide tray icon
  -> close without applying hide policy
```

## Zed / GPUI rendering findings

- Zed's Windows title bar selects one actual installed icon font (`Segoe Fluent Icons` on Windows
  11, otherwise `Segoe MDL2 Assets`); it does not pass a comma-separated CSS fallback expression to
  GPUI's `font_family` API.
- GPUI 0.2.2 treats `font_family("A, B")` as one literal family name. NxTools.Next runtime logs proved
  that both `Inter, Microsoft YaHei UI, Segoe UI` and `Segoe Fluent Icons, Segoe MDL2 Assets` failed
  lookup and fell back to Microsoft YaHei UI. The latter rendered caption glyphs as mojibake.
- Product UI must use one real Windows CJK UI family and SVG assets for caption controls, avoiding
  icon-font availability and fallback ambiguity. Diagnostic monospace uses one real family rather
  than a CSS-style list.

## NxTools.Next responsibility boundary

- Rust owns persisted system settings, close/restore/exit policy and lifecycle state.
- GPUI owns product-settings and title-bar presentation and converts clicks into typed operations.
- C# PlatformHost owns HKCU startup registration, notification-area icon/menu presentation, native HWND
  show/hide/foreground and `WM_SYSCOMMAND` move. It does not decide policy.
- All system-shell commands and tray events are versioned, bounded and correlated. Diagnostics carry
  fixed operation/result/native error fields, never arbitrary registry values or executable paths.

## Required regressions

- GPUI click from the title bar opens/reuses product settings; the obsolete sidebar entry is absent.
- Explicit title-bar controls dispatch minimize/maximize/close behavior, and title-bar pointer down
  reaches the typed native move command with the real HWND.
- Registry adapter verifies exact current-user Run value enable/disable with a real isolated key seam.
- Tray adapter creates one icon, maps left click/Open/Exit to typed events and disposes it at shutdown.
- Close-to-tray keeps the GPUI window entity alive; restore reuses it; explicit Exit terminates.
- Release subsystem is `windows`, while tests/debug builds retain ordinary diagnostic behavior.
