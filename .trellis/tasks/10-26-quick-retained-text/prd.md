# Audit #13: retained Quick Launch presentation text

## Scope and baseline
Direct current-session implementation, no delegation. Live dirty baseline2026-10-26T09:12:44.222655Z:385files/4293historical evidence files, including audit11/12. No full Desktop, real user-file search, Clipboard/Shell/native/product/service, packaging/install or commit. No mechanical domain/protocol String conversion.

## Source/reference review
Read GPUI coding guide retained-text section: keep stable SharedString in owning View/Entity; clone handle per frame; mutable editing/assembly String and temporary borrowed str stay distinct. Reviewed locked StyledText::new(Into<SharedString>) and official component contracts; original Button/Input/row geometry/variants remain, not a visual migration.
Listary first: managed SearchMode.Results/selection and SearchWindowBase constructor/event bindings -> ShowWindow -> base.Show/Activate/Hide/Close/input focus; SearchItemList movement gate and nested action-mode references. Managed boundary only, not proprietary native engine. SwiftList second: current Helpers/QuickSearchWindowInputHandler.cs and QuickSearchWindowLifecycle.cs, original action/IME/Escape/show-hide/focus owners. Indexed old input-handler path is absent; current Helpers path verified. No WPF/native internals inferred.

## Behavior gap and narrow design
Original panel keeps Vec<SearchResult> String payload, render_result clones title/subtitle and entire result for retained hover callback each row/frame. Actions retain String label/section/level title; visible_actions and render/context clone them repeatedly.
- Original panel owns one shared immutable result snapshot plus a stateless per-result display projection prepared at result commit/new/show/recent-mode updates. Full SearchResult unchanged; no second search controller, global/per-row lookup cache, domain or protocol change.
- Result title/subtitle/fallback are full SharedString display values. Elements clone handles, preserving existing highlights/order/geometry. Preview closure retains original immutable result snapshot and index, preserving old captured payload rather than resolving an old index into a newer result list. Full payload is cloned only when actual preview/execution requires ownership.
- ActionItem.label/section_label and ActionLevel.title are view-only SharedString; add stable action-target display title at original action-mode construction. Editable filter/parent query and command/path/session/protocol payloads remain String. Status retained by panel is shared display text; input buffer unchanged.
- Keep original generation/admission/merge/selection/highlight/preview/key-up/pointer/action-stack/hide/close/focus/native and preceding rem/row-gap values. No truncation/content cap, palette, input dispatcher or visual redesign.
Expected files:panel.rs,actions.rs,small mechanical original result mutation/merge fixture adapters as necessary; new retained_text_tests; append reference with verified new source paths. No finder/protocol/vendor/dependency edits.

## Confirmed regression seam
User requests production result update/repeated render. Observe actual SharedString values passed at production title/subtitle/action/context composition, retain them through repeated real Root/Window draw to prevent allocator-address reuse, assert one stable backing allocation per unchanged long text. Run red before changing ownership. Cover real replace_results/reorder/updated text and typed shell-action apply/submenu/filter, complete display text and unchanged full SearchResult payload, visible range/measurement, focus/input identity. Positive gestures use original UI pointer/key paths; no real execution effects. Timings descriptive only, no threshold-only acceptance.

## Verification and evidence
Natural-idle serial Rust jobs1/incremental0/codegen256. Retain failed attempts. Use established command-only Desktop test debug0/_LINK_=/DEBUG:NONE due disk/PDB limits, no project profile edit. Focused new/QuickLaunch render/action/preview/generation/release protections; strict tests/production,workspace check,scoped/workspace fmt and gates. Known QuickLaunch release-chain source-gate defect belongs to queued item16, not silent fix here. English results/deltas/limits and immutable old evidence. No global13closure or native/DPI/reader/user acceptance implied.
