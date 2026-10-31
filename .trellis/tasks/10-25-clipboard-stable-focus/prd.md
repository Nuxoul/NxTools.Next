# Audit #10 only — stable Clipboard row/action/focus identity

## Phase and scope
Canonical phase: Clipboard + Tools, stabilization of current implementation, no phase renumbering. Repair only mutable-list retained identity; original entry.id + action kind and existing panel/view namespace replace index behavior keys across slot/wrapper/ListItem/toolbar/local Buttons/focus lookup. Index remains paging/scroll/position hint. Preserve 4x80 bounded history, ListState splice/measure, selected ID alignment, full-by-ID payload, native menu/preview, shortcuts/IME/pause/type-anywhere and all current visual exceptions. Accepted #1/#9, shared action helper, engine/protocol/native/vendor/theme/dependencies unchanged.

## Reference and owner boundary
Read clipboard note then legacy ClipboardHistoryListView constructor/cache/PointerReleased and popup constructor/Open/Activated/Prepare/Hide -> inherited Window.Hide, original item acceptance. Avalonia native base unavailable. Exact pre0.3.9 List has no element ID or index namespace, render callbacks/layout_as_root/prepaint use the original ancestry; window.use_keyed_state uses current full element stack. Component0.8.0 Button and unchanged project FocusableContent keep one original FocusHandle/release-click. Fix entire retained ancestry, not only innermost IDs. No second global keyboard dispatcher.

Expected own paths: ui/history.rs and ui/mod.rs identity/focus-lifecycle boundary, new ui/history_identity_tests.rs, directly affected existing tests (update real key expectations only), append-only clipboard reference and task evidence. Save per-file current dirty baseline, preserve other sessions. If a target changes concurrently stop editing that path/report; never ask another session to stop or reserve time.

## Invariants and acceptance
- A focused row/action keeps same actual FocusHandle + immutable item after original Prepend/front-trim/reorder while still mounted; Enter/Space dispatch one original-ID action, no paste bubbling from subaction.
- Removed/unmounted record cannot be replaced by occupant of its old index. Armed press release is cancelled or remains exact-ID according to original current-generation policy. Focus retires to explicit valid target when needed, without activating a new selected row as a side effect.
- Original query/generation/selection, search InputState, menu session, full payload/worker and native correlation untouched. Cross-owner work next-turn with original weak/current-ID checks.
- Retain actual ListItem/Button/Toolbar variants, grammar, density, gradients/shadow/Markdown geometry, no visual redesign or shared helper behavior change. Diagnostics use existing action correlation, no payload/per-frame logs.

## Tests and verification
First add deterministic production ClipboardPanel + actual ListState/worker page callback + real Tab/FocusHandle/key down/up regression. Cover row A prepend/trim still mounted, actual available management action, removed row/action and armed release, rerender/reorder/different panel/close-reopen, current selection/search/menu/query/preview rules. Only temp synthetic SQLite/config/bridge, no real clipboard/paste/native/product.
Run focused red/green, relevant Clipboard history/panel/lifecycle, strict Desktop Clippy --tests -D warnings/workspacecheck/fmt and source gates. At most one complete Desktop integrated run; report Archive Unlock/any other failure as FAILED, no unrelated fixes/green hunting. Every Rust shell jobs1/incremental0/codegen256; natural occupied build wait only. No package/install/host/service/userdata/registry/commit/push/delegation. Return exact own-deltas/manifest by session communication, separate evidence levels, stop for parent review before any next item.
