# Boundary and design

## Source-backed differences
ClipboardPanel currently restricts all history shortcuts to search focus and number release requires Ctrl still held. Reproduce both through real key events before selecting the input fix. Current text rows flatten source to a Label; preview Auto uses a conservative classifier rather than default Markdown.

## Components
Preserve compact300x440/min280x280 history popup and existing paging, selection, full-by-ID actions. Exact Kit0.7.1 styled Button::ghost/small/icon/accessibility_label/tooltip, directly composed at the header's far-right without Toolbar adaptation. Official IconName pause/play assets. No menu/duration controls. Durable failed state stays visible, not only tooltip. Text uses component::text::{TextView,TextViewState}, selectable(false), scrollable(false) in bounded rows, stream_fade(false), inert links. Preview retains its passive HWND and optional source/code controls; normal prose/Markdown is rendered by default, confidently detected code may retain code highlighting.

## Invariants
One active input owner; no text insertion or duplicate effects on shortcut gesture. IME/editor/dialog/context-menu guards, immutable ID/query generation, held-repeat suppression, correlated native success-only paste close remain. Local state first, cross-entity application updates next-turn. One passive preview window reused/updated/repositioned/hidden; no activation or native lifecycle rewrite. Summary400characters stays distinct from full payload. Safe Markdown removes resource-bearing HTML/image nodes and enforces existing parser budgets; cache is bounded by the retained list window and invalidated when summary changes. No frame/key/pointer telemetry; existing action/pause/preview correlation events remain.

## Expected files
ui/efficiency.rs + real-panel regressions; ui/capture_controls.rs + ui/mod.rs header placement/state; ui/preview_rich.rs and preview tests; ui/history.rs and bounded row Markdown preparation/cache plus relevant tests. Owning reference/spec/outcomes record the superseded interactions. No C#/Protocol/storage changes expected.

## Verified input and row details
Regression red proves Ctrl-first key release and missing legacy unmodified-number path; initial focus repro used an invalid keyed-state lookup and was corrected to actual focus traversal (not counted as product-red evidence). Final input permits Page/Ctrl-number beyond search but keeps other controls' Enter/Space owner. Key release uses capture phase. No screenshot/installed/native acceptance is inferred.
List TextView parses only the400character summary, mounted keyed state reuses the same official state on repaint and recreates only when source changes. Row content stays bounded (ordinary24, heading40, multiline96DIP maximum) and four rendered lines; outer ListState handles scrolling. Safe image/HTML projection is shared with preview. A real red showed official TextView intercepts link clicks before the row: public on_link_click must invoke the original row action with current modifier policy, never open the URL or leave a dead region. No new action controller.

## Header press boundary
Real production header regression initially observed one header drag-handler call when pressing the new Pause icon (expected0). Official Button avoids pointer focus but does not stop this MouseDown bubbling. A noninteractive layout slot around the official Button now stops only left MouseDown before the native move owner; button click/keyboard and blank-header dragging remain intact. The retained cfg(test) counter is a boundary regression probe, not production logging. TextView on_link_click requires Send+Sync: a weak keyed-row entity retrieves/clones the original Rc action before invocation, without retaining a self-cycle or holding a lease across action dispatch.
