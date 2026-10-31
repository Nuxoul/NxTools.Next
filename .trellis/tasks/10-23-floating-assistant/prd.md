# Floating window in Other Tools

## Goal
Add one optional desktop floating window to Other Tools: a persistent quick entry displaying the built-in ball or a selected image/GIF, with predictable download/search/clipboard actions and genuine mouse click-through. Reuse existing Next feature owners rather than introducing parallel engines.

## Background and confirmed decisions
- User explicitly selected **basic floating-window behavior + every requested option**. Legacy Time bar, hover shortcut panel, hotkeys, profiles, modifier actions and custom commands are excluded.
- User approved Automatic text routing: downloadable link -> ordinary Add Download prefill; other text -> Quick Launch search. Automatic image routing: network image link -> ordinary Add Download; local image/raw bitmap -> image clipboard write. No OCR and no silently started download.
- User approved speed mapping: aggregate downloading speed1MiB/s=1x; positive factor clamped0.25..4x; a Downloading task with0aggregate speed pauses GIF; no Downloading task restores configured baseline.
- Source evidence: [legacy reference](../../../docs/reference-code/assistant.md) records `D:\Code\NxTools\Models\AppState.cs:118-189`, `Views\FloatingAssistantWindow.axaml:1-84`, `.axaml.cs:55-154,281-343,444-538,1041-1256`, `ViewModels\MainWindowFloatingAssistantViewModel.cs:328-406,608-668`, `Controls\FloatingAssistantGifView.cs:109-347`, and `Views\MainWindow.axaml.cs:2229-2445,2608-2637`. Old GIF has a hidden6FPS cap and no speed synchronization; these are not desired new behavior.
- [Current integration research](research/current-next.md) identifies reusable entry points and the current text/bitmap-drop gap. No product code or native runtime has been changed during planning.

## Requirements
| ID | Observable requirement |
| --- | --- |
| R1 | Add a180x160-style official Other Tools tile. Body enables/disables; separate outlined Configure opens/reuses the460x320 compact settings window without toggling. Default disabled. |
| R2 | Preserve core reference behavior: passive/non-taskbar topmost ball, drag with saved position, monitor-work-area recovery, base/hover opacity, edge snap/auto-hide, fullscreen hiding, position lock, download progress/count. Position lock blocks move, not selected click actions. |
| R3 | Use the default built-in ball or select PNG/JPG/JPEG/BMP/WebP/GIF. Expose size20..300/default72logical pixels and frame-rate1..60/default24FPS; no hidden6FPS limit. Keep GIF source timing under configured display cap. Image selection reset only returns to default appearance; it does not erase other preferences. |
| R4 | Download-speed synchronization Boolean, default off. Apply the confirmed mapping above to GIF playback only; do not alter download state or limits. Static pictures remain static; hidden/disabled surfaces stop animation work. Display never exceeds60FPS. |
| R5 | Left-click dropdown exactly `无 / 打开主窗口 / 添加下载`; proposed default Open Main. Right-click dropdown has the same choices; proposed default Add Download. One gesture yields at most one action and a completed drag yields no click. |
| R6 | Text-drop dropdown exactly `无 / 自动判断 / 添加到下载 / 速启搜索`, proposed default Automatic. None has no side effects. Automatic follows the confirmed rule; explicit Search sends complete text to original Quick Launch input without automatically executing a result. Explicit Add Download without an eligible link explains why it cannot proceed. |
| R7 | Image-drop dropdown exactly `无 / 自动判断 / 添加到下载 / 写入剪切板`, proposed default Automatic. Support browser image URL/metadata, local image and bitmap payload within documented budgets. Automatic follows the confirmed rule. Explicit Add Download requires a valid HTTP(S) source; a local path/bitmap is not fabricated into a download. Copy writes actual supplied image content, never URL text; if no image bytes/local image are available it reports a clear failure, without secretly fetching a URL. |
| R8 | Mouse click-through Boolean, default off. When on, clicks, wheel, drag and drop pass to underlying windows; turning it off from Other Tools restores interaction. Native application failure must be distinguishable from persisted intent. |
| R9 | Settings persist by one dedicated section and survive restart without overwriting unrelated settings. Proposed inherited defaults: base/hover opacity48/96%, auto-hide on, snap distance24, fullscreen hide on, position unlocked. Runtime native errors retain retry/recovery and do not report false success. |
| R10 | Keep one logical floating window/session, one animation owner and one native input boundary. Mere show/hover/move does not steal focus. Explicit Open Main/Add Download/Search activates only its intended existing destination. Close/reopen, disable, disconnect and late results cannot resurrect old windows or repeat actions. |

## Acceptance criteria
| ID / requirements | Observable acceptance |
| --- | --- |
| A1 (R1,R9) | Pointer/Enter/Space toggles once; Configure does not toggle; repeated Configure reuses one window. Durable restart defaults/roundtrip and concurrent unrelated-section saves are verified using temporary configuration. |
| A2 (R2,R10) | Drag persists acknowledged final position with no click; lock retains click but prevents movement. Edge reveal/fullscreen recovery/monitor loss work within native bounds and do not steal foreground. Download count/progress reflects existing live task state. |
| A3 (R3,R4) | Real static/GIF fixtures test decode, transparency/disposal, size/FPS, invalid/oversize rejection and stale loads.0.25/1/4x,0speed pause, no-downloading baseline and sync-off are deterministic; static/hidden states schedule no animation. |
| A4 (R5) | Each of the3left and3right choices dispatches its chosen existing action once; None, drag/capture loss/retired generation yield no action. Add Download is ordinary confirmation, not automatic submission. |
| A5 (R6,R7) | Every text/image choice runs through the actual receiving/classification/router boundary; link/plain text/browser image/local image/bitmap/multiple representations/None/unsupported input are covered. One drop never triggers both Download and Clipboard. |
| A6 (R8,R10) | An owned native fixture verifies pointer pass-through to an underlying owned target and reversibility; no invisible receiver remains after hide/disconnect/disposal. Source, renderer, receiver and operation identities are checked. |
| A7 (R1,R3,R5–R9) | Actual official controls and all fields remain reachable within460x320 by scrolling; default32DIP inputs/selects, focus/keyboard/cancel, busy/error/long-content and activeLight/Dark themes are checked. No ordinary controls are repainted as substitutes. |
| A8 (R9,R10) | Correlated native/controller lifecycle covers hidden preparation -> final native readback -> correct-size frame -> passive reveal, failure/timeout/disconnect/replacement and no synchronous cross-entity reentry. |
| A9 (all) | Affected tests, scoped lint/typecheck/fmt and source gates are reported truthfully. Source/compiled/owned-HWND evidence is separate from physical browser/Shell drop, mixed-DPI, installed and user visual acceptance. |

## Constraints and risk boundary
- Rust owns configuration/state/layout; GPUI renders; C# owns Windows APIs/OLE/clipboard. Preserve current Kit0.8.0/defaultpre0.3.9 graph, existing fonts/themes and unrelated dirty work.
- Existing Windows GPUI drop target supports CF_HDROP only (`vendor/gpui-pre-windows-039/src/window.rs:1144-1285,1516-1527`). First implement a safe owned native receiving-layer feasibility fixture. No extra framework patch, cross-process WndProc injection or broad global hook is authorized. Failure of that prerequisite is reported as a genuine blocker, not a reduced feature claim.
- Frame/input/byte-tick/payload logging is prohibited. User-facing/cross-process actions have bounded correlation/state/terminal/elapsed/native-code diagnostics.
- Final planning-summary approval authorizes local implementation and safe isolated checks only. Packaging, installation, production launches, real clipboard/user-data mutation and service operations require separate permission.

## Out of scope
- Legacy Time bar, hover shortcut panel, global hotkeys, saved profiles, modifier mappings, broad file/video processing, custom commands/templates and radial action menus.
- OCR, silent download submission, treating local images as remote downloads, remote fetching solely to copy URL-only image payloads.
- Download engine/store rewrite, new search or clipboard history controller, theme/framework upgrade, browser-extension/indexer/installer changes, commit/push or release operations.

All user-owned scope/routing/speed decisions are resolved. Proposed defaults/UI composition and the bounded prerequisite are included in the final summary for fresh implementation approval. Technical risks are described in `design.md`, not treated as already verified capabilities.
