# R5 live oversized main window — 2026-10-24

## Authorization and observation boundary
The user rebooted and explicitly asked to inspect the currently oversized window before any manual interaction. This follow-up performed read-only native/window/process/configuration/log observations. No move/resize/show/hide/activate/minimize/restart, synthetic input, live native replay, product build, code patch, deployment or real clipboard operation was performed. A single screenshot-only Computer Use observation returned dimensions/metadata; no visual acceptance is claimed. Native reads temporarily set only the observer thread to PerMonitorV2, restoring it in finally; target DPI context was not changed.

## Confirmed live facts
- Target Desktop process9080, main HWND132698/thread16044; native process start2026-10-24T18:49:17.227958+08:00. Command flags include `--silent-start`.
- Current visible ordinary window: IsZoomed=false, IsIconic=false, WINDOWPLACEMENT.showCmd=1. It is NOT maximized.
- GetDpiForWindow=96, awareness=2(per-monitor aware). One primary monitor2560x1440, workarea2560x1400, effectiveDPI96. These observations do not prove the entire startup DPI history, but reject current150/200percent scaling.
- GetWindowRect=(208,208)-(2128,1237), size1920x1029devicepx.
- GetClientRect=1904x1021devicepx, equal1904x1021DIP at96DPI. Production main requested1060x650DIP. Width ratio1.7962, height ratio1.5708; not a single uniform DPI multiplier.
- DWM visible frame=(215,208)-(2121,1230), size1906x1022. This differs from Win32 outer bounds because of invisible resize borders. Do not compare these coordinate kinds as equal.
- Two snapshots at22:00:18.466665 and22:05:35.650575 show identical HWND/client/outer bounds and96DPI.
- Running Desktop SHA256 `CE331D8EEC1E0A71BF830A2E7884977B123B43040581DC3AA804186B4763295C`,86929920bytes, exactly equals packaged `artifacts/win-x64/NxTools.exe`. Package manifest sequence28/releaseID `b8ec8615-03dd-4f1b-9c75-45b0a0e50b04`. No stale-copy assumption.
- Read-only global settings confirm silent-start and close-to-tray enabled. No main-window size/bounds top-level persisted setting found; irrelevant settings/payloads were not retained.

## Correlated first-show timeline
Existing desktop and PlatformHost logs were read with FileShare.ReadWrite|Delete, without stopping writers. Only fixed events, times, correlationIDs, window dimensions and safe result fields were projected; business payloads/raw log lines were not persisted.

| Local time | Observation |
|---|---|
|18:49:17.756222|product.shell.silent_startup_started|
|18:49:18.067112|silent_startup_completed,310ms|
|21:52:15.681584|product.shell.tray_action_started action=open, correlation8eeb1b85-a027-47ad-a94a-21da73928fd7|
|21:52:15.681944|native command.received set_window_visibility, same correlation|
|21:52:15.728788|quick_launch.window_visibility (generic adapter event name), exact mainHWND132698, before_visible=false/after_visible=true/success=true; logged bounds215,208,2121,1230|
|21:52:15.728885|set_window_visibility completed success,46.94ms|
|21:52:15.729475|activate_window received, same correlation|
|21:52:15.769128|activate_window completed success,39.65ms|
|21:52:15.780747|tray_action_completed,99ms|

**The first-show logged DWM frame is byte-for-byte equal to the current DWM frame.** This proves the oversized frame was present on that tray visibility transition, not merely a later resize. The log retains AFTER bounds, not before bounds/client/DPI, so it does not independently prove creation-time dimensions. A later21:52:19 native minimize failure exists with a different correlation; it follows the observed oversized first show and does not explain its origin. Later unrelated window commands must not be attributed to the main HWND without target identity.

## Source-backed diagnosis
References: `docs/reference-code/system-shell.md` and current source chain:
1. `rust/apps/nxtools-desktop/src/main.rs:75` sets logical1060x650, `main_window_placement` binds primarydisplay, WindowOptions uses hidden showfalse for SilentStart.
2. `vendor/gpui-pre-windows-039/src/window.rs:549` CreateWindowExW uses CW_USEDEFAULT. `:575` computes requestedplacement; when showfalse `:584` stores it in initial_placement instead of applying SetWindowPlacement.
3. `window.rs:367` set_window_placement consumes that pending placement. `:815` platform activate invokes it.
4. `rust/apps/nxtools-desktop/src/platform_events.rs:509` enqueue_tray_restore sends original C# SetWindowVisibility then ActivateWindow; it never invokes the GPUI platform activate path or explicitly prepares initial bounds.
5. `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs:544` ShowWindow(SW_SHOWNOACTIVATE) checks only visible state. `:258` native activation restores only if iconic and otherwise foreground/focus APIs; no GPUI pendingplacement or targetsize apply. Generic adapter visibility logs use DWM visiblebounds (`WindowsPlatformAdapter.cs:882`, NativeMethods:599), not GetWindowRect.

### Confidence and limitations
- Symptom/launch route/first-visible oversized dimensions are now **observed live**, superseding the old task's unavailable-live-scene statement.
- Current maximized-state, current uniformDPI and wrongpackagedDesktop candidates are rejected by native/hash evidence.
- The hidden-create/pendingplacement bypass is the strongest source-and-timeline explanation and a concrete integration gap. No controlled intervention has yet demonstrated that consuming/preparing that pending placement alone fixes the user's exact scene. Do not label this a completed causal repair or claim a reproducible reboot harness exists.
- Startup display/DPI transition history and exact pre-show nativeclient remain unavailable. No beforeplacement log may be invented.

## Next safe implementation boundary
Construct an owned hidden-main/first-show regression at the real application/native boundary (including tray and other primary restore entrypoints). Prove intendedclient/device placement BEFORE first visible show, with existing native API ownership/readback and no activation during hiddenprepare. Apply initialplacement once, preserving subsequent user resizing/maximize/close-to-tray and browsercoldstart focus behavior; do not reset defaults at everyshow or expand vendor patches speculatively. This follow-up has not started that implementation or modified the live window.

## Reusable evidence artifacts
Session scratch: `inspect-boot-window.ps1`, `boot-window-live-first.json`, `boot-window-live-evidence.json`(secondread), `boot-window-sanitized-timeline.json`, `boot-window-first-show-confirmed.json`, `boot-window-native-window-commands.json`. Scripts read only the observedHWND and bounded/log allowlists; no raw userlog/payload artifact. Some earlier comparison files compare Win32outer againstDWM and therefore show false; the confirmed file and this note use the correct commonDWM coordinate.
