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

## Installed update occupancy recheck (2026-10-10)
BrowserHost is launched by browser Native Messaging, not DesktopRuntime child ownership. Current source explicit Exit quits GPUI and drops owned PlatformHost/client supervisor; LocalSystem Indexer remains independent. Live BrowserHost was observed in the ordinary installation directory, not Validation; no process killed or registrar changed. Extension now demand-connects/releases idle transport and no longer auto-reconnects/diagnostic-starts; passive health does not restart desktop. Installer explicitly lists five installed EXEs for CloseApplications/Restart Manager and disables RestartApplications, preserving SameOwnedPath/OwnedIndexerService uninstall guard. This is file-resource registration, not a process-name kill; source guard alone does not prove effective lock release or protected service update. Stopped Indexer restore/real scans remain separately authorized.

## Ordinary shell apply after installed upgrade (2026-10-10)
Rechecked legacy StartupLaunchService.cs:51–87: ordinary HKCU startup uses the actual running desktop, not index-service version approval; SettingsWindow constructor/base/open/apply/cancel boundary unchanged. Current C# shell resolver used IsApprovedProtectedDesktop for both configured current desktop and protected fallback, rejecting a valid new installed package whenever old protected tuple/registry hashes remained. New temporary-file regression runs red with current desktop bytes/hash plus old protected bytes, before product fix. Shell resolution may accept configured NxTools.exe only when it is the exact sibling of the running PlatformHost and its stable opened bytes match embedded current DesktopSha256; outside-package, tampered/missing/invalid-hash paths fail closed. Existing protected fallback remains fully approved and service admission/deployment authorization is unchanged. No service repair, registry mutation, startup/tray side effects in the regression.

## Kit styled-component Foundation authorization (2026-10-10)
User approves option A and Foundation implementation: released Kit0.7.1 control typography/radius/padding/state grammar replaces legacy custom control chrome;neutral palette retained. Re-read legacy MainWindow.axaml.cs OnOpened/OnClosing/HideToTray/EnsureTrayIcon/TitleBar_PointerPressed and SettingsWindow.axaml.cs constructor/OnOpened/ConfigureForCurrentScope/CloseButton_Click/SaveButton_Click/TitleBar_PointerPressed. Main hide remains presentation-only,same tray restore owner,explicit exit bypasses hide. Settings retains its original save/cancel scope; native drag uses left-pressed boundary and double-click maximize. Avalonia native base is unavailable;no claim of source through that implementation or permission to rewrite native lifecycle. Next Foundation changes only theme/control/sidebar/render composition; Rust state/layout,C# native commands and GPUI HWND boundaries stay unchanged. Link tool has no legacy counterpart;its Next ClipboardApplication deferred settings writer and keyed460×320 window remain authority.

## Queue09 Global Settings component prerequisite
Rechecked old SettingsWindow constructor -> WindowAnimationService Attach/Open animation/
Closing cancellation -> delayed public Close; OnOpened ConfigureForCurrentScope, navigation
scroll, ApplySettings/CloseSettings commands and OnClosed cancel. Native Avalonia base
unavailable. Old global startup/tray intent and proxyOff/Auto/Custom host/port reviewed;
ABDM Desktop ProxyConfigurableRenderer/ProxyEditState first for proxy behavior, no PAC/
auth capability imported. Next supports Direct/System/ManualHTTP and visible unsupported
PAC/credentials note; raw host/port drafts/Apply validation remain. Canonical per-app
independent Clipboard/Quick scopes are Next amendment, not old global table equivalent.
Candidates releasedSettings/Page/Group/Item/Field public per-control/item slots, Form/
Field,Checkbox/ToggleGroup/Sidebar. Preserve720x480/native title/footer, original stable
inputs/localrules/editremove/busy/cancel/retry/atomicglobal save/desired-not-applied and
cx.defer consumer publication. field_shell re-enables appearance/border/focus; caller
falseflags are redundant, not runtime-suppressed inputs. No schema/trust/reset/native
or genuine userconfig/system/registry operation; synthetic tempconfig/bridge only.

Queue09 actual result: releasedSettings3Page3Group7individualItems/publicFieldrender,
twoSmallFormField proxy/ruleeditor,controlledSwitch/independentCheckbox/SmallButtons;
officialSidebar/searchfilterresize/scroll/resetfalse. Original720x480/draft/entities/
proxyhostportvalidation/allscopes/matching/savecancel/busyretry/nativeApply owners remain.
scope_action marks/Baseordinarycontrols removed;sharedfield_shell alreadyreenabled
callerfalseflags,not runtimesuppression. Zero-callworkbenchrowsection removed only.
OriginalstyledButtonGroup retains exclusiveproxy mode;actualToggleGroupdirectrecipe
allowsallfalse/keyboardgroupcallback0,otheradapters unverified,not global rejection.
Originalsave/apply/nativepending/result/timeout/deferredconsumer/add_rule/constructors
and sharedlivehelpers22bodies equal09baseline;Core/consumer/fieldpreview filesbytes equal.
8new productionregressions/full606/606/strictchecks/9sourcegates0;actualTooltip hover
candidateframe requires outside/refreshed then hover/frame/delay,notproductioncompensation.
Exact scoped evidence/options: owningui-queue-outcomes.md09. No realWindowsstartup/tray/
registry/config/network/Clipboard/userhistory/Release/service/fork/delegation/commit.
No09PNG/nativeactualHWND/physical/DPI/reader/installedacceptance;support10/gallery12pending.

## Post-gallery label correction — 2026-10-12
Rechecked old SettingsWindow.axaml and .axaml.cs constructor -> animation Attach ->
OnOpened/base -> ConfigureForCurrentScope; ApplySettingsCommand then Close, ordinary
close uses CloseSettingsCommand. StartupLaunchService retains quoted --silent-start.
Current Global Settings keeps720x480/original atomicApply/Cancel/native owner. Actual
gallery showed duplicate SettingItem and Switch visible labels. Released Switch public
accessibility_label preserves the semantic name without repeating visible text;
red-capable production geometry/name test precedes this presentation-only correction.

## Actual screenshot balance follow-up — 2026-10-12
Browser restored; new Global screenshots show far-edge switches and equal-width
host/port fields. Released SettingItem public field-render slots and Form4-column/
Field.col_span3 allow local composition without replacing Settings selection/search.
Constrain ordinary control lane360px; proxy address3 columns/port1. Preserve720x480,
complete help/accessible names/original entities/ApplyCancel/atomic/deferred/native
owners. Old constructor/animation/baseOpen/ApplyClose chain rechecked before edits.

## Update authorization reference review

See the [Listary / SwiftList / WinShun update comparison](../../.trellis/tasks/10-01-one-time-uac-trusted-updates/research/listary-update-uac-comparison.md)
for the verified managed updater call chain, installation/startup separation and adoption limits.
Listary sets `RunUpdateAsAdmin = false`, but this does not establish that the downloaded
installer never elevates. SwiftList's `SilentInstall` still uses `runas`; its existing-service
start path and original-user post-install launch are distinct. WinShun requests whole-app
`requireAdministrator`, which must not be adopted for NxTools. No reference here certifies
an authenticated silent protected updater or replaces NxTools' atomic release validation.
The NxTools signed-update source mechanism now exists; see the owning
implementation-approved.md and outcomes.md. Production key enrollment and LocalSystem
update acceptance are still pending. Moving deployment into installation alone would
remove a first-launch prompt, not eliminate authorization per update.

## Six-position screen notifications (2026-10-16 user correction)
Read legacy SettingsWindow.axaml.cs constructor/OnOpened and Apply/Cancel/Close boundary through inherited Avalonia Window; source has no equivalent six-anchor feature. Earlier Next documentation incorrectly said in-window and was rejected in installed acceptance. The user's requirement is screen-work-area-relative independent passive notifications. Original SettingItem/draft/atomic save and NotificationCenter remain; prepare_notification_window validates source/popup/pipe-peer identity and applies monitor/DPI/passive/final-client-readback in C#, Rust owns one bounded reusable GPUI popup and original-source actions. Hidden Windows popup ordinary frame delivery does not run: build its correct-size first scene after the host lease with public GPUI draw, then deferred nonactivating reveal; no show-first/timeout-ready/framework patch. Directory feedback uses this same center without page status. Existing signed/installed tray lifecycle remains independent. [Correction and evidence](../../.trellis/tasks/10-16-menu-notification-refinement/screen-notification-outcomes.md). No legacy six-anchor implementation or installed/mixed-DPI acceptance claimed.

## Official notifications and preserved native settings boundary (2026-10-17)
After isolated locked0.7.1 capability proof the user accepts official fixed5seconds, whole-list remaining hover/focus pause and hover-only close. This supersedes prior custom-card/default6/configurable1..60 timing, not legacy settings Apply/Cancel or native screen lifecycle. NotificationCenter now owns bounded business admission/generations/actions only; one official NotificationList/ToastStack owns card/display/clock/exit. Different invocations have distinct IDs; same progress/result invocation reuses its own key. Position-only changes update official global placement without repush/countdown reset. Settings shows read-only5seconds, preserving ignored legacy duration on unrelated save. Public content measurement/focus and pointer bounds allocate enough passive HWND space for expansion then release it on collapse/retirement/empty, with visible keep_visible adjustment. Hidden draw/readback/deferred reveal, style-free whole/client equality, original-source once-only deferred actions and screen monitor/DPI remain unchanged. Native motion/error/disconnect reaches GPUI policy. No legacy equivalent is invented, no dependency/native protocol change or automatic package/install. [Current exact evidence](../../.trellis/tasks/10-16-menu-notification-refinement/official-notification-integration-outcomes.md); sequence15 remains prior custom implementation.

## Runtime feedback investigation (2026-10-24)
Rechecked legacy App.axaml.cs silent-start minimized/non-taskbar plus MainWindow hide/tray restore chain (framework native base unavailable). Next `main.rs::main_window_placement_for_display` requests logical1060x650 and explicit display. Published Windowspre0.3.9 `window.rs::new` creates CW_USEDEFAULT, computes requested placement, but with showfalse stores initial_placement; `activate -> set_window_placement` applies it, while original PlatformEventDispatcher tray route calls C# SetWindowVisibility then ActivateWindow directly. NativeMethods.SetWindowVisibility uses SW_SHOWNOACTIVATE and does not apply GPUI's cached placement. This is a concrete integration concern, NOT proof of the user's rare oversized boot scenario; no causal native replay was obtained and no main/startup/vendor code was changed in this batch. Do not add a per-show default-size reset. Follow-up needs controlled hidden-create/first-show/nativeclient/DPI trace under authorized native fixture or a redacted user capture.
System-proxy display is an explicit new request: ABDM DesktopSystemProxySelectorProvider delegates desktop default ProxySelector rather than pretending PAC has one host/port. Old proxyOff/Auto/Custom settings supply save/draft compatibility only. Next new snapshot reads configuration without PAC/network evaluation and never changes System route or manual draft. Blacklist editor moves above existing rows, still one original draft/Apply/Cancel owner.

## R5 approved first-show repair boundary (2026-10-24)
Live userHWND evidence now captured in task10-24-runtime-interaction-feedback/r5-live-boot-evidence.md: silentstartup/notmaximized/96DPI/1904x1021client, firsttrayvisibleDWMframe equals current. Rechecked old App.axaml.cs29–35 and MainWindow.axaml.cs2696–2751; silent nofocus and samewindow restore preserveusergeometry. Current main.rs creates hiddenmain with cachedGPUIplacement, native restore entrypoints neverconsume it. Repair belongs to a once-only hiddenmain preparation before publishing mainHWND/enablingentrypoints, authenticated C#nativeDPI/clientreadback and originalRustdispatcherACK barrier. No per-show defaults, focusworkaround or newvendorpatch. New regression must replay actualCW_USEDEFAULThidden -> originalnativeShow -> clientdimensions, then verifyprepare/hide/userresize/restore. This section records approvedinvariants, not yet a passing repair.

## R5 repair result (2026-10-24, later than investigation)
The original ownedCW_USEDEFAULThidden -> nativeShowNoActivate regression ran red with1904x990clientat96DPI vsintended1060x650. Production `PrepareMainWindow` now stageshiddenprimary/readsDPI/appliesexactclientwithliveinsets/verifieshiddenclient before returning typedreadback; samefixture green, later1200x700userresize surviveshide/show. Original PlatformEventDispatcher optionalstartupbarrier recordspendingbeforeenqueue, buffers<=128notifications, exactACK+readback then nextturncallback outsidelease publisheshint/notificationhost/floatingentrypoint and replaysoriginalnotifications. Deadline8s/native5s, disconnect/invalid/stale/queuefailure failhidden; normalshownstartup unchanged; originaltray/summon/floating show/activate remains afteronceprepare. No per-showreset, GPUIactivationworkaround, framepolicychange or vendorpatch. Native3/3/Ruststartup6/6/Protocol77/77/strictlint/workspacepass; finalDesktop1123pass/1unchangeddateassertionfailure, notfullgreen. Exactsource/evidence limits in taskr5-repair-outcomes.md. Installedreboot/mixedDPI/useracceptance notperformed; nodeployment.
