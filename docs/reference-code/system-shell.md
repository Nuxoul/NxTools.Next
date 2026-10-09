# Product shell, startup and tray reference index

Last verified: 2026-08-14.

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

Root: `D:\Code\NxTools`

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
| `Views\MainWindow.axaml.cs::EnsureTrayIcon` / `LoadTrayIcon` | Uses the product `Assets/Icon.ico`, tooltip `NxTools`, and native menu labels `打开 NxTools` / `退出`. |

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
- The desktop executable, taskbar and notification area share one formal NxTools brand identity. The
  Rust desktop build embeds a multi-size Windows icon assembled from the same checked-in 16/32/48/128
  brand PNGs used by the browser extension and GPUI title bar; PlatformHost extracts that associated
  executable icon and must not invent a second visual identity.
- The tray remains a native `NotifyIcon` plus native `ContextMenuStrip`. Its bounded presentation
  contract is tooltip `NxTools`, then `打开 NxTools`, one separator and `退出`; SH-08 must not replace
  it with a GPUI or owner-drawn menu.
- All system-shell commands and tray events are versioned, bounded and correlated. Diagnostics carry
  fixed operation/result/native error fields, never arbitrary registry values or executable paths.

## Required regressions

- GPUI click from the title bar opens/reuses product settings; the obsolete sidebar entry is absent.
- Explicit title-bar controls dispatch minimize/maximize/close behavior, and title-bar pointer down
  reaches the typed native move command with the real HWND.
- Registry adapter verifies exact current-user Run value enable/disable with a real isolated key seam.
- Tray adapter creates one icon, maps left click/Open/Exit to typed events and disposes it at shutdown.
- The native tray boundary verifies the exact tooltip/menu order and that the icon is the executable's
  associated product icon. PlatformHost first verifies that the PE owns an `RT_GROUP_ICON` resource,
  so an iconless executable cannot be mistaken for the Shell generic fallback; Release packaging then
  compares the embedded icon pixels with the checked-in brand asset.
- Close-to-tray keeps the GPUI window entity alive; restore reuses it; explicit Exit terminates.
- Release subsystem is `windows`, while tests/debug builds retain ordinary diagnostic behavior.

## 2026-08-13 P0 visual-foundation verification

- The product font boundary is now centralized: normal UI uses the single installed Windows CJK
  family and diagnostic values use the single monospace family. Migrated shell surfaces do not
  repeat font-family literals.
- The title-bar density contract is 36 px high; ordinary form actions are 28 px and switches are
  40×22 px. These are presentation tokens only and do not change the typed native window-command
  lifecycle above.
- The main title bar renders the existing NxTools extension icon as an embedded PNG asset. It no
  longer constructs a brand mark from styled text.
- `MaterialIcon` has an enum-driven asset gate. Every enum member, including Settings and Terminal,
  must resolve through `NxToolsAssets`.
- The product-global settings surface consumes shared setting section, row, toggle and action
  primitives rather than importing helpers named for the download module. Its Apply path and
  PlatformHost confirmation lifecycle remain unchanged.

### Open questions carried from user reports (2026-09-08)

- **DOpus multi-tab directory** (RESOLVED 2026-09-08): the user reported the
  captured directory not following the active tab. Corrections: (1) Listary
  source IS available — `D:\Tool\dnSpy-net-win64\dnSpy.Console.exe` decompiles
  `D:\Program Files\Listary\Listary.exe` (obfuscated class names, 1146 files)
  — but an exhaustive byte scan shows this install contains NO dopus/lister
  strings at all, so Listary's DOpus support on this machine is not in the
  managed layer; (2) the behavioral source of truth is SwiftList
  (`D:\Code\SwiftList-main\Plugins\DirectoryOpus\`), which handles dual-pane
  via lister-title/geometry matching and remembers the last active side.
  Ported to DirectoryOpusFileManagerOperations: when OS focus sits on the
  lister frame (tab switches), fall back to the per-lister remembered pane;
  the container caption — which DOpus updates to the active tab's path —
  keeps the directory current through GetContainerPath's caption-first
  read.
- **hydra (D:\Code\hydra-main)** is a new GPL/MIT dual-licensed Rust
  multi-source download engine reference (crates: hydra-core/net/stream/
  host/gui). Behaviorally interesting for NxTools.Next: multi-source
  (mirror) fetching, per-file segment scheduling and its updater design.
  License: GPL-3.0-or-later OR MIT — inspect per-file before copying ANY
  code; treat as clean-room behavioral reference like FluxDown.

## DOpus quick-switch revalidation (2026-09-27)

The current PlatformHost path was rechecked against the Listary and SwiftList notes before the next fix:

- `WindowsPlatformAdapter.EventWindow::ProcessForeground` captures a supported directory source before publishing the file-dialog host; `DirectoryNavigationAdapter` dispatches `dopus` listers to `DirectoryOpusFileManagerOperations`.
- `DirectoryOpusFileManagerOperations` stores an opaque session through `ThirdPartyFileManagerSessionRegistry`; the Rust side forwards that context unchanged in `NavigateDirectory` and does not know DOpus pane details.
- `Win32DirectoryOpusWindowOperations::TryCaptureActivePane` must select the active `dopus.filedisplaycontainer` from the summon-time focus, lister caption/path evidence, geometry, or a per-lister remembered pane. A lister-frame focus is not a valid pane-child focus and must be represented as no restorable child focus, not as a pane focus that later fails validation.
- `IsPaneAvailable` and `TryActivate` must validate the captured lister and container without requiring a stale frame focus to be a descendant of the selected container. Directory navigation remains bounded by `WM_COPYDATA` timeout and post-command path readback.
- Directory-source recognition exhaustion must not leave a previous source dirty for a later file dialog. A failed DOpus capture is not a new source and must not trigger automatic navigation from stale state.
- Pending Directory Opus source recognition is bound to the saved lister HWND rather than the current foreground HWND. If the user returns to a known file dialog during the bounded settle window, PlatformHost passively probes the saved lister and re-consumes the dialog when capture succeeds; the probe does not activate a window, navigate a pane, or send IPC.
- SwiftList's `DirectoryOpusPathCollector`/`ShellPathHelper` shows that DOpus path-bar segments are Shell localized display names. NxTools must resolve both `SHGetFileInfo(SHGFI_DISPLAYNAME)` names and the parent of the current user profile (`C:\Users` on this machine) before `Directory.Exists`, rather than treating `C:\用户` as a parsing path.
- SwiftList sends DOpus `Go` through `DOpus.ParentWindow` without reapplying the captured pane focus afterward. NxTools' DOpus adapter owns activation around the command; the session registry must not run a stale post-navigation focus restore that can return the lister to its pre-command directory.
- Quick Launch directory actions may carry `lib://Pictures`-style library identifiers. The Windows boundary resolves known library names to their real known-folder paths before `Directory.Exists`; Rust action eligibility must accept and preserve these URIs rather than dropping them as non-absolute paths.
- Navigation diagnostics compare only bounded target metadata (kind, root, segment count, length and resolved metadata). Raw filesystem paths and URI payloads are intentionally absent from production logs.

The interactive DOpus 13 and real file-dialog soak remains an environment-dependent gate; the PlatformHost regression seam must cover frame-focus capture, same-pane navigation, and stale-source clearing without logging paths or window titles.

### Same-lister DOpus refresh when a known dialog returns — 2026-10-04

The user reports that selecting an already-existing DOpus tab changes the directory, but returning to the already-open file dialog does not switch until the active tab is navigated or a new tab is created. The code-backed gap had two guards: the dialog-return refresh ran only while `_sourceDirty` was true, and `ProcessForeground` returned early for a visible/reused file-dialog search host before the Auto Quick Switch consumer ran.

- Listary's reference contract is to retain a live source controller and call `GetCurrentFolder()` when the same known file dialog returns. A cached path from an earlier tab is not authoritative.
- On return to a known dialog, PlatformHost identifies the source by its opaque `HostContextId` and registered Directory Opus adapter, passively re-captures the same lister/session, refreshes pane state and path, then lets `AutoQuickSwitchTracker.ObserveFileDialog` consume the result.
- The visible-search-host reuse branch now still reprocesses the exact same foreground dialog for Auto Quick Switch when Auto Quick Switch is enabled and the retained source is a valid DOpus session. Other sources and unrelated HWNDs retain the existing reuse fast path.
- Only a changed canonical path marks one new sync. An unchanged path does not navigate. A failed/superseded session refresh discards pending state, so the old path is never used. The first observation of a new dialog does not refresh or navigate from retained history.
- Refresh never activates, focuses or navigates DOpus and sends no IPC. Diagnostics retain one correlation per passive read and contain only HWND, fixed stage/result and elapsed time, not paths, titles or payload text.
- Regression uses the real `ThirdPartyFileManagerSessionRegistry` seam for same-lister/session recapture, changed/unchanged paths, one-shot candidate, non-DOpus rejection, lister/session mismatch and failed refresh without stale navigation. Separate foreground-reuse predicate coverage protects the early-return path. These tests do not reproduce the user's DOpus UI gesture; manual DOpus acceptance remains required.

SwiftList is a secondary behavior reference: `ExplorerTracker` listens for `EVENT_OBJECT_FOCUS` and re-reads active paths, while `ExplorerActivePathPoller` coalesces location-change events. NxTools also listens for `EVENT_OBJECT_SELECTION` on the tracked DOpus lister, plus name/focus fallbacks, and coalesces passive source refreshes without activation or navigation. No native event trace was captured for the user's exact DOpus click; the selection event is a bounded compatibility path rather than a claim about every DOpus build. No DOpus adapter was found in the searched old `D:\Code\NxTools` checkout; SwiftList is not DOpus engine source.

## 2026-10-08 SOL remediation: native mutation and cancellation

Reference behavior reverified through ABDM `CompletedDownloadPage.kt:103-143` and legacy `Services\WindowsFileDragService.cs:29-61`: one native OLE boundary; normal cancellation is not an operation error. C# owns the dedicated STA OLE loop, source HWND/capture release and initiating-button checks. No competing gesture framework or payload logging was added.

- `NativeFileDragService` preserves normal Escape cancellation as a non-error and distinguishes cancelled from dropped in terminal result data/diagnostics. Real Shell drag/drop remains untested.
- `BrowserNativeMessagingRegistrar` delegates all manifest/browser/legacy-key changes to `BrowserRegistrationTransaction.Apply`. Capture all prior bytes/registry values, kinds and children before the first mutation. Record each attempted write before calling it; it may mutate then throw. Compensate each attempted step in reverse order, even if another compensation fails; report incomplete compensation as AggregateException. Tests use synthetic stores only, not real registry injection.
- `installer/nxtools-next.iss` stops Indexer only after exact ownership validation: quoted ImagePath with --service and exact installed path, or approved protected tuple/hash and matching registered desktop. A service name alone never authorizes stop. This slice was source/ownership-tested, not installer-compiled/executed.
- Centered native resize is fixed-budget asynchronous work, not work on the serial command reader/EventWindow. Generation/CAS owns replacement/replay/retirement; acceptance is distinct from final applied client readback. See the download reference for layout/lifecycle constraints.
- Architecture gates follow real Rust extracted modules and C# partial files. SHA256.Create/ComputeHash and ordinal lookup preserve PowerShell 5 checks rather than removing assertions to make gates green.

No package/install, service operation, real DOpus navigation or product launch occurred. See [SOL remediation evidence](../audits/2026-10-08-sol-review-remediation.md); source/native regressions are not a live Windows certification.

## Global per-app settings ownership (2026-10-10)

Reverified old `Views\SettingsWindow.axaml.cs:11-24,56-78,118-175,185-194`: the
Avalonia Window constructor attaches animation, OnOpened configures the scope, navigation changes
the local visible section, Apply calls the owner command then closes, and unhandled close calls the
cancel command. These base Window show/close boundaries do not imply a new native adapter.
Next keeps one registered fixed global window, local General/Network/Application blacklist tabs,
scrollable continuous rows and a contained footer. Constructor opening reloads latest durable sections
outside render; pending apply blocks close and duplicate dispatch. Saved shell intent remains distinct
from confirmed applied shell state; a rules-only save must not mutate startup/tray registration when
desired and live shell settings agree. Weak Clipboard/Quick Launch bindings publish rules next-turn
after one durable transaction, without consumer disk writes. No screenshot visual acceptance is claimed.
