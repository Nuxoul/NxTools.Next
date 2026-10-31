# Runtime interaction feedback — proposed plan (2026-10-24)

## Status and authority

- Planning only. This document is not implementation approval.
- Workspace: `D:\Code\NxTools.Next-main`; branch: `refactor/ui-gpui-kit`.
- No active Trellis task was reported by `task.py current --source` during this review.
- Task creation and implementation of this new batch await the user's review of the final summary. Do not reuse the historical scope-6 approval in `10-22-clipboard-density-resource-names`.
- The user explicitly confirmed that successful paste keeps a pinned Clipboard panel visible, and that pinning applies only to the current opening. Explicit hide followed by reopen resets pinning; do not persist it.
- Preserve concurrent Archive, floating-assistant, notification, browser, editor, native observation and release changes. No delegation/contacting peer sessions.

## Requirements and acceptance (PRD)

### Goal

Correct the reported Clipboard interaction defects, diagnose two intermittent window/dialog failures, and make the requested narrow UI/settings changes without replacing existing feature owners.

### Confirmed evidence

- `repo:rust/apps/nxtools-desktop/src/ui/mod.rs#hover_preview_ready` does not include menu admission. Existing foreground and pointer handlers already protect an active native context menu; do not claim menu protection is entirely absent.
- `repo:rust/apps/nxtools-desktop/src/ui/components/native_context_menu.rs#handle_notification` currently treats any non-menu HWND pointer press as outside; `finish` routes it to `repo:rust/apps/nxtools-desktop/src/clipboard.rs#context_menu_outside`, which closes the panel. Inside-source and external clicks need distinct outcomes.
- `repo:rust/apps/nxtools-desktop/src/ui/mod.rs#handle_command_result` closes ordinary successful paste with `PopupCloseReason::PasteSuccess`; CopyOnly deliberately restores panel focus. Pinned paste must not reuse CopyOnly focus restoration.
- `repo:rust/apps/nxtools-desktop/src/ui/capture_controls.rs#render_capture_controls` owns the existing upper-right recording control. The new panel pin belongs immediately to its left, not in an item menu.
- `repo:rust/apps/nxtools-desktop/src/tools/floating_assistant/application.rs#pick_image` has a 600-second logical wait. `repo:csharp/NxTools.PlatformHost/Windows/FloatingAssistantCoordinator.cs#PickImage` uses a two-minute bounded STA wait and merges several failures. Native dialog Cancel currently returns success with no selected path. These facts do not prove the reported intermittent root cause.
- `repo:rust/apps/nxtools-desktop/src/main.rs#main_window_placement_for_display` requests logical 1060x650 bounds on an explicit primary display. This does not prove the native DPI/show lifecycle is correct under startup display changes.
- `repo:rust/apps/nxtools-desktop/src/network_settings.rs#render_field` disables the manual address/port controls in System mode but does not display a detected system snapshot.
- `repo:rust/apps/nxtools-desktop/src/main_window/tools_page.rs#render_tools` draws a dot for Link Recognition; Currency and Floating Assistant must use their real enabled state consistently.
- `repo:rust/apps/nxtools-desktop/src/network_settings/blacklist.rs` and the settings render order place the add editor after the list.
- The supplied Firefox listing is published according to the user: `https://addons.mozilla.org/zh-CN/firefox/addon/nxtools-download-interceptor/`. Chrome remains under review; no Chrome listing URL is available.
- The download alignment image was downloaded (146x70 PNG) but has not been visually inspected. Built-in browser visibility failed; local PowerShell-backed viewer observation was refused as a blocked terminal target. Do not bypass that tool restriction or claim to have identified the pictured button.

### Requirement / acceptance matrix

| ID | Requirement | Observable acceptance |
| --- | --- | --- |
| R1 | Suppress Clipboard hover preview during a context menu | Opening a menu cancels pending and visible preview. Delayed/cache-hit results cannot reveal it while the menu is active. After menu dismissal, only the original deliberate movement/800ms hover path can schedule another preview; no first-paint reveal. |
| R2 | A click inside Clipboard but outside its menu closes only the menu | The panel stays visible, the dismiss click does not accidentally paste/select/activate a background control, and no stale foreground/pointer event subsequently closes the current session. Truly external dismissal preserves unpinned behavior and respects R3 when pinned. |
| R3 | Add a session-only fixed-display control immediately left of Recording | Default is unpinned. Enabled pin blocks automatic loss-of-focus/external-click/paste-success panel hide, but does not force activation. Successful paste keeps the panel visible and returns focus to the valid target. Explicit hide remains available using existing Esc/menu priority and summon-shortcut behavior. Every new opening resets pin to false; no config write. |
| R4 | Diagnose and repair intermittent image picker failure | Distinguish admission/source identity, dialog cancellation, timeout, retirement, busy and actual native errors. Cancel is not an error. Matching current request alone can commit a selection; disconnect/retirement/old responses cannot change settings or emit stale errors. A fix needs a failing regression for its demonstrated cause; timeout inflation or error suppression alone is insufficient. |
| R5 | Diagnose rare oversized startup main window | Trace logical placement through native creation, display/DPI selection, silent start and later show. Reproduce the causal transition with owned/synthetic tests before a scoped fix. Preserve normal user resizing and all launch-mode focus/show contracts; do not reset every show to the default size. |
| R6 | Align the download-page button pictured by the user | Visually identify the exact button before editing. Compare its final bounds/alignment with the left-hand control at default/minimum window size and Light/Dark. Fix only the responsible public component/parent composition; do not redesign the download page. |
| R7 | Consistent Other Tools enabled indicators | All actual enableable tiles use the same dot+text convention and live enabled state; failed saves do not display false success. Preserve existing tile layout and distinct configure/enable actions. |
| R8 | Show detected system proxy address/port below the selector | Selecting System starts a bounded read-only snapshot and shows actual static endpoint(s) in read-only fields. Preserve System routing and the user's separate manual draft. PAC/auto-detect, multiple protocol endpoints, no proxy and read failure are represented truthfully; never fabricate a host/port or silently convert to Manual. Stale responses cannot overwrite a newer mode/view/request. |
| R9 | Put Add application above the blacklist | The add editor/action appears before the list and is reachable without scrolling through existing entries. Preserve existing single-save, Cancel, validation, row actions and list scrolling. |
| R10 | Add a Firefox store button in Browser Extension settings | One official button opens exactly the supplied HTTPS listing through the existing Shell/open owner. Preserve existing export and ID controls; do not add a speculative Chrome URL or change extension privacy/permission behavior. |

### Out of scope

- Framework upgrades, new vendor patches, palette/theme redesign, replacement menu/preview/clipboard/download controllers.
- Browser privacy/header/cookie findings, permissions or YouTube changes; prior store ZIP repair remains separate.
- Archive, indexer, release/security, unrelated failing tests, notification changes or editor changes.
- Package/install/service operations, formal product launch, physical keyboard/IME/mixed-DPI acceptance, real clipboard/history/config changes, live proxy inspection, registry writes, commit/push/archive. These need separate authorization.

## Proposed technical design

### Owners and lifecycle

- Rust owns session pin state, preview admission, current menu source identity, settings drafts and request generations. GPUI renders using existing public components. C# owns native Windows dialog/window/proxy API access.
- Keep one existing Clipboard preview HWND and native presentation ACK chain. Cancel old preview state before opening the menu; validate menu/session/query/generation again at dispatch and apply time, including synchronous/cache-hit paths.
- Classify source-panel clicks separately from external clicks at the existing context-menu boundary. Retire the exact menu before deferred source updates. Keep the dismiss gesture from falling through to item activation.
- Pin policy is per opening, not HWND topmost alone and not durable preference. Keep existing native paste target restoration. An inactive pinned panel must not invoke the CopyOnly reactivation path. Test target routing after the user switches external applications.
- Picker investigation follows existing Rust command registration, native admission/source HWND validation, bounded STA lifetime, dialog result, terminal response and settings commit. Do not claim logical retirement cancels an already executing Windows API. Preserve responsiveness of the serial command reader and event thread.
- Startup investigation follows constructors/framework/native create/show/DPI notifications through actual client bounds. No speculative framework patch; any unsupported vendor change would require renewed approval.
- Add only a narrowly typed proxy-snapshot command/result if no reusable native read boundary exists. Bound request lifetime; install pending correlation/generation before dispatch. Native reads must not perform network PAC evaluation or probe external endpoints just to populate the UI.
- Existing immutable network policy snapshots and section-save merges remain authoritative.

### Component selection

Current release is Kit/Base/Component/Assets0.8.0 with default pre0.3.9. Read the selection rules, current release delta, table and API index, then exact released source before edits.

| Surface | Public component / composition | Variant, size and parent rule |
| --- | --- | --- |
| Clipboard fixed display | `gpui_kit::component::button::Toggle` with `checked`, `on_click`, `icon`, `tooltip` | Small ghost, official checked state; immediately left of the existing Small ghost Recording button; no Toolbar child transformation. Verify accessible pressed state and keyboard owner before finalizing. |
| Proxy snapshot | Existing Settings/Page/Group/Item render slots and Form/Field/Input | Existing control scale/read-only field composition; keep manual InputState separate. No painted substitute or new route selector. |
| Tool state | Existing ListItem tile/status composition with Label and shared status indicator | Keep 180x160 tile shape and existing theme success/muted tokens; no Card/Grid invention. Verify official dot/status API before extracting shared presentation. |
| Blacklist add placement | Existing Settings item order and Form/Input/Button | Reorder existing owner, retain visible controls and single-save model. |
| Firefox link | Existing browser extension SettingGroup with official Button | Small outlined, direct/public content composition; opening uses existing deferred Shell command. |
| Download alignment | Exact pictured control and parent must first be identified | Measure final actual bounds. `Toolbar.child(Button)` still forces ghost+compact; sizing alone is not a complete fix. No invented offsets without layout evidence. |

### Diagnostics

Define fixed, low-noise events before completing each slice:

- Clipboard session/pin state transition and menu terminal classification (inside source vs external), correlation/session/generation only.
- Picker accepted/start and terminal reason (cancel/selected/timeout/busy/source-retired/native-failure), elapsed time and native code at its owner.
- Main-window initial placement/show/DPI transition with bounded dimensions/scale/display classification; no per-frame logging.
- Proxy snapshot accepted/start/terminal, request generation, result kind/count, elapsed/native code; never log endpoints, credentials, PAC URLs or payloads.
- Existing Shell command correlation handles the Firefox link; do not log the URL as diagnostic payload.

No clipboard text, query/keystrokes, selected image path, filename, user window title, raw exception, headers or cookies in logs. Deferred/cancelled work cannot write stale UI results.

## Execution and verification plan

1. After explicit approval, create the new scoped Trellis task/artifacts (or parent/independently verifiable children), preserving peer tasks. Set approval only for this batch; curate real source/spec context before `task.py start`.
2. Refresh live git status, references and all touched owner files. Save a scoped scratch baseline; never restore over concurrent edits. Fill reference index gaps and native lifecycle invariants before code changes.
3. Implement R1-R3 together at the existing Clipboard/menu/paste boundaries, with real failing regressions first for pending/visible preview, source click consumption, external click behavior, pin paste focus and reopen reset.
4. Investigate R4 and R5 separately with isolated dialog/native display fixtures and causal failure records. Fix only demonstrated in-scope defects; retain explicit evidence gaps if the reported scenario cannot be reproduced. No fake success from merely compiling or adjusting deadlines.
5. Complete R6-R10 using existing compounds and native/Shell owners. R6 requires the screenshot identification gate before layout edits. R8 tests use synthetic proxy configurations only.
6. Verify focused UI behavior and current-generation/stale/cancel/failure paths, actual geometry, keyboard/IME owner guards and existing lifecycle contracts. New C# fixture registrations require exact `TestEffects` audit and no live user-state effects.
7. Before every compiled verification, check for competing Cargo/rustc/clippy/MSBuild/relevant tests/non-Unity dotnet (exclude persistent VBCSCompiler). Do not kill or contact peers; exit75 means nothing launched. Build serially with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in every wrapper.
8. Run focused tests, scoped strict lint/workspace check, applicable architecture gates, and one final Desktop suite at the finished state if resources permit. Zero selected tests are not a pass. Preserve concrete failures; do not modify unrelated product/tests for green counts.
9. Use scoped `rustfmt --edition 2024 --config skip_children=true` only on owned Rust files. Run spec-contract checks for changed specs plus component/release/editor gates required by AGENTS. No broad vendor formatting.
10. Record outcomes by evidence level: source/static, compiled/synthetic fixture, image inspected, owned native, physical/mixed-DPI/user visual and installed acceptance. Leave deployment and live user-state checks explicitly unperformed unless separately authorized.

## Remaining technical evidence / risk

- Root causes of the intermittent picker failure and oversized startup are not established. Their acceptance requires causal regression evidence, not speculation.
- The download screenshot needs legitimate visual inspection before its control-specific change. Do not circumvent a blocked Computer Use target.
- PAC and protocol-specific system proxies may have no single universal address/port; display that truthfully without changing routing.
- Existing unrelated test/static failures are historical evidence, not current results. Recheck only as appropriate after implementation; do not claim the current tree is green.

## Review gate

The final summary must cover R1-R10, session-only pin lifetime, scope exclusions, diagnostic prerequisites and verification limitations. Wait for a subsequent explicit user approval before creating/starting implementation work. This plan file itself changes no product behavior.
