# Installed feedback: neutral focus and BrowserHost lifetime — 2026-10-10

## User symptoms and authority
User rejects cyan navigation fill/actions and the persistent rectangle on Global settings Network and Download settings Transfer; then reports Quick Find indexer OFFLINE and BrowserHost/Indexer surviving desktop exit and blocking installer replacement. This supersedes the historical cyan approval, not original keyboard/native/service ownership. Changes stay local and uncommitted.04:31 Validation installer remains unchanged and does not include this correction.

## Observed facts, not inferred product failure
- Read-only process inspection initially found BrowserHost under `D:\Users\Administrator\AppData\Local\Programs\NxTools`, parent59896 with Chrome extension argument, **not** the Validation executable path. Installed binary/registry identity was not read; current source conclusions are not proof of that older executable's exact implementation.
- `NxToolsIndexerV1` is Auto/Stopped, PID0, at its existing ProgramData service path. It was intentionally left stopped during previous UI validation. OFFLINE is consistent with this; no indexer-engine regression established.
- BrowserHost is browser-owned Native Messaging stdio, not a DesktopRuntime child. Explicit tray Exit quits GPUI and drops owned PlatformHost/client supervisor; titlebar Close may hide to tray. LocalSystem Indexer intentionally outlives Desktop; this is not a leaked desktop child and must not be killed by name.
- Later read-only inspection found no BrowserHost/build/preview process. Agent did not terminate it, and this is **not** live verification of the new policy.

## Reproduction and fixes
### Neutral roles and focus-visible
Read GPUI/Zed reference first, then Zed ButtonLike/ListItem, ABDM add-download before legacy settings constructor/base/nav. Reference note records unavailable Avalonia native base and fixed GPUI lacking focus_visible. Added two tests before product changes: neutral ordinary roles, actual focused_action render style after real simulated mouse/Tab/KeyUp. Both ran behavior-red: rgba38bdf8 and pointer-retained ring. Initial attempted scene() API caused compiler failure, not a behavior red; replaced with final production element style observation plus actual input/hit bounds.

Theme ordinary action/hover becomes#bfc0c4/#d5d6da, neutral selected#353539/soft#303034, pressed#414145, independent keyboard focus#9d9ea3; existing canvas/text/status colors, fonts/layout/native budgets unchanged. Shared focus presentation retains pointer focus and original tracked-focus KeyUp activation, disabled/repeat guards and Tab navigation. WindowId-keyed weak modality state with frame-owned canvas input observers changes presentation only; input mode changes refresh, no polling/frame logs/second activation. Existing consumed Tab path records keyboard input before original processing. Reused overlays/results use the same visible-focus predicate. No vendor/dependency/native provider rewrite.

Tests caught the initial Tab observation gap and fixed it at the existing handler. Final production-style/geometry regression proves pointer no ring, keyboard visible ring, unchanged bounds and one activation; not complete native installed focus acceptance.

### Demand-only BrowserHost
Read download/system-shell reference and actual ABDM/legacy native lifecycle, then BrowserHost run/stdio, extension connect/reconnect, runtime drop and installer boundary. Safe extracted background functions with fake Chrome ports reproduced rejection retaining port and disconnect starting a replacement. New four lifecycle tests were red before changes; a fifth failure/timeout test added during closeout.

- Browser startup and best-effort logs no longer connectNative.
- A real request owns transport before awaiting connection, installs pending before postMessage and releases in finally. Send/timeout/disconnect failures clear once.
- No connecting/pending/request owner: idle1s disconnects current port, with no autonomous reconnect. Future explicit requests still connect.
- Full Blob prepare→content upload→commit/retry holds an outer finally lease so release cannot prematurely destroy the capability server; all original fallback/IDs/limits remain.
- Old-port callbacks cannot invalidate replacement state.
- Host `get_snapshot` no longer calls connect_or_start: it connects the existing engine only, so passive Popup refresh cannot resurrect desktop. Explicit task requests preserve original startup policy. Snapshot regression is a compiled **source-bound guard**, not an actual pipe/process launch test.

### Installation coordination
Main Inno script explicitly declares CloseApplications=yes, RestartApplications=no and five exact installed-file names in CloseApplicationsFilter. No taskkill/name-wide kill, new protected deployment owner or service operation. Existing SameOwnedPath/OwnedIndexerService uninstall guard retained. Python source gate was red then green. Inno defaults already had some Restart Manager behavior; this is explicit scope/no restart, **not demonstrated native lock release**. Browser demand/idle strategy removes the confirmed permanent port/reconnect cause, but old extensions and protected service updates still require actual authorized installer acceptance. Validation installer in scratch is separate and not changed by this source edit.

## Verification performed
- Desktop **496/496**, BrowserHost library **30/30**, extension **26/26** (5 native-lifetime cases plus21 existing); no ignored/skipped in these runs.
- Strict production/test Clippy for Desktop+BrowserHost, workspace --tests, fmt and Debug build pass after fixing one bool-assert lint. No broad native harness/service/real-volume test.
- Twelve related architecture gates pass: token/consistency/UI-state/motion/preview-isolation/desktop-shell/download/QuickFind/ClipboardUI/attached-search visual/window-follow/browser-native-messaging. Installer-update Python gate passes, source only.
- Owned isolated Debug HWND main1060×650/min920×480, direct confirmation560×278; message-level mouse Downloads→Tab/Enter QuickFind→Clipboard and wheel changed viewport. New gallery `artifacts/ui-layout-quality/neutral-feedback/index.html`. PNGs acquired, not newly eye-inspected; fixed/installed/global-download-settings native acceptance remains separate.
- Format and diff checks pass; Git CRLF normalization warning is not a failing diff check. Session scratch logs `neutral-focus-red/green`, `native-lifecycle-red`, `browser-status-red`, `acceptance-fix-*` retain failures and scope.

## Remaining blockers and operation limits
Indexer remains Stopped. Starting/reinstalling/repairing it enables real system/service/volume operations and needs separate user authorization. No service/process kills, registry changes, normal runtime/user data, installation, repack, commit or push performed here. New code requires a fresh authorized package plus extension update/reload to affect the old installed native port. Actual EOF host process exit/file-lock release, full installed mixed-DPI/physical gesture visual acceptance and protected-service update/release CPU/search gates remain open. Native reader/provider limitation unchanged.

One optional bounded read-only code-review delegation exceeded10 minutes without a final report and was cancelled; it is not claimed as a passed review. Parent regression/lint/gates remain the executed evidence. Spec navigation87 documents/12 owners and checker10 tests pass; no runtime/service/native installer operation.
