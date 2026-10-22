# Production migration map

## Current authorization
The original design-only delivery remains historical evidence. After pushed checkpoint42f60a8, the user explicitly authorized local production UI migration with all original functionality governing presentation. [Feature preservation ledger](feature-preservation.md) is mandatory: prototype omissions never authorize deletion of fields/actions/states. The first bounded product slice is implemented and checked; exact scope/limits are recorded in [the audit](../../audits/2026-10-09-feature-first-ui-migration.md). No package/install/service/user-data or automatic commit/push authorization is implied. Further native/user visual acceptance remains separate.

[Inventory](surface-inventory.md) · [Design system](design-system.md) · [Blueprints](page-blueprints.md) · [Acceptance](acceptance-matrix.md)

## Canonical stages
Names/order below retain the existing staged task; no competing new numbering or advancement from historical source-complete reports. Stabilization remains in its actual current stage.

| Stage | Expected production seams after approval | Freeze before changing | Relevant regression / visual evidence |
| --- | --- | --- | --- |
| Foundation and representative shell | `ui/{metrics,typography}.rs`, `ui/components/{action,field,list,settings,titlebar,menu,state}.rs`, `main_window/shell.rs` | Candidate type/density roles; main1060/920 budgets; local focus+deferred application ownership; component defaults; preserve theme colors | Actual bounds, fonts/line-height, pointer/KeyUp/Tab single dispatch, min size/footer/long state; token/consistency/state gates; native representative screenshots |
| Downloads | `main_window/download_page.rs`, `downloads/{new_download_view,task_window,settings_view,schedule_view,delete_confirmation,shutdown_window}.rs`, `download_taskbar_overlay.rs` | ABDM then legacy source; accepted fixed handoff, native sole-owner placement, task same-size completion, category/schedule/media/durable semantics | `downloads/{tests,polish_tests}.rs`, download-ui-lifecycle, actual fixed/expanded/media/error content and footer; native interaction only when authorized |
| Quick Find | `main_window/quick_launch_page.rs`, `quick_launch/{panel,actions,preview,file_dialog_search}.rs`, relevant command/path/scope/hotkey components | Matching query generation, summon host/focus, one passive preview/controller, in-window actions/lazy Shell, existing setting apply model | `quick_launch/{tests,polish_tests}.rs`, quick-launch-lifecycle,file-dialog-search-visual/window-follow; real search/result/action/preview/focus render matrices |
| Clipboard and Tools | `main_window/{clipboard_page,tools_page}.rs`, `ui/{mod,history,preview}.rs`, `tools/link_panel.rs` | Bounded worker/history; no real data access for fixtures; click copy/paste policy; passive preview/session; category editor focus; retention/storage owner; existing registered tools only | `ui/{tests,polish_tests}.rs`,tools/polish_tests,clipboard-ui/clipboard-lifecycle; normal/long/empty/error/editor/busy panels and passive preview |
| Global settings and supporting surfaces | `network_settings.rs`, `network_settings/blacklist.rs`,notifications/dialog/popover/supporting views; companion extension UI last | Per-app scopes, canonical-empty migration, one atomic save+deferred no-save consumers; saved≠Windows-applied; keyed windows/cancel; no global mega-transaction | network_settings/main_window tests,desktop-shell-lifecycle; field/grid/focus/state/actual native render; browser extension DOM/CSP/reduced-motion checks independently |

All filenames above are under `rust/apps/nxtools-desktop/src/` unless otherwise named. Render extraction is allowed only for a concrete ownership/reuse need; no broad controller rewrites for styling. Do not modify sibling dirty changes, reset branch or auto-commit/archive.

## Reference checklist before each product slice
1. Read matching `docs/reference-code/` note, then recorded old source and supplied reference.
2. Follow constructors/base and real native show/hide/placement boundaries; leaf render files are not lifecycle evidence. Missing Avalonia native base/Listary native engine source stays explicit.
3. Update note with newly verified entry/call/invariant before implementation.
4. Freeze layout/lifecycle/cancel/focus/failure invariants; add a red-capable actual production-boundary test before correction.
5. Install pending IDs/generations before dispatch; defer cross-entity work. Layout migration does not license synchronous cache-hit re-entry.
6. Verify geometry/state and inspect actual render. No guessed WM_SIZE, dimension poll loop, additional size owner or unbounded worker.

## Implementation batching
After direction approval, start with shared Foundation and a representative workspace/settings pair; compare native screenshots to the prototype. If type/metrics fail native small-window budgets, resolve before mass migration. Then migrate owning module surfaces in canonical order, keeping small windows on explicit compact tokens instead of sweeping every `.small()` to a larger global value.

Per-surface change ledger: old screenshot/state, source ownership, approved target, regression red, local changes, focused green, full affected tests, native render, user/DPI acceptance. Maintain one shared-component write owner when parallelizing independent surfaces; do not compile while writers are active.

## Verification commands (future product phase)
Root Cargo.toml; one Cargo; workstation environment on every direct/background command:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```
Use focused GPUI owner tests, desktop full suite, scoped production/tests Clippy `-D warnings`, workspace `--tests`,fmt and affected architecture gates. Use full-operation timeout; inspect surviving processes before retry, never duplicate Cargo. Current design round does **not** run these Rust commands.

C# verification only if a native seam changes; exact safe name filters and actual RUN-set check. No unfiltered native/DOpus suite. Packaging and protected Indexer release/search/CPU are separately authorized operations, not required to demonstrate a browser layout.

## Risks and deliberate deferrals
- Browser text rendering/zoom/ARIA differs from pinned GPUI/DirectWrite. Follow design with native comparison, not a screenshot transplant.
- 13px body and176px main rail are candidates; smallest pages/content may need compact role or different measured rail. User visual review freezes them.
- Existing two-level navigation is retained in representative config pages; content width at920 must be tested, not hidden by prototype fit scaling.
- Fixed276/278-era bugs must not be reopened by layout changes; current accepted new confirmation is278, ordinary task184.
- Pinned GPUI0.2.2 lacks verified rendered screen-reader provider seam. HTML semantic labeling does not implement native accessibility. Dependency/provider upgrade is out of scope.
- Current user-installed19:30 package and newer Debug global layout are distinct baselines. Do not claim prototype is installed or previous package contains it.

## First local production slice status
Foundation:176px/40px named rail and opt-in13/20 workbench roles, compact roles unchanged; representative global settings uses readable rows. Downloads:48px task row/name/support and local settings typography, all six filters/full columns/five details retained; fixed new/task HWND owners untouched. Quick Find:readable five-topic main config/stable control rails plus compact leaf alignment. Clipboard + Tools:continuous complete settings groups and stable history/link-panel leaf slots, old immediate/worker/native owners retained. Global:representative type/row projection only; same per-app save/apply logic. Integrated488/488 plus strict lint/workspace/fmt and11 gates pass. Native owned100% main/minimum/module switching and Clipboard scroll-to-storage inspected. Full auxiliary/state/mixed-DPI/installed visual acceptance not complete; do not mark stages fully accepted from this slice.

## Remaining local production slice status
The user requested continuation of all remaining source work. Downloads auxiliary 32px sibling controls/12/18 copy retain all fixed budgets/actions and accepted handoff/completion lifecycle. Quick Find advanced master/detail,scope,exclusion,diagnostics use workbench roles and bounded32px form actions. Clipboard compact editors retain300×440/244px budgets with genuinely constrained source scroll and stable footer. Notification full copy/details scroll inside336×112 with28px action; browser popup/shadow styling retains controller/IDs/palette. Final494 desktop tests, strict checks/build,21 extension tests/38 DOM checks and static preservation pass. Five owned native captures have budget/hash evidence; new PNG eye-inspection and installed/full-state/DPI/U/provider acceptance remain deferred. [Remaining audit](../../audits/2026-10-09-remaining-ui-layout-migration.md).
