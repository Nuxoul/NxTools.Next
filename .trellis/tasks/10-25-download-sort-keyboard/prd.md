# Audit #2 only — download sort keyboard equivalence

## Scope and phase
User directly requested the next single item after #1/#3/#9/#10 were parent-accepted.
Only original audit #2: sortable Download column headers lack an equivalent keyboard interaction.
Canonical phase remains Downloads stabilization, not a new phase or list/controller rewrite.
Per-file current dirty baseline saved before edits; preserve unrelated changes and prior accepted work.
Stop after delivery for parent review; no sixth item, delegation, commit/push, package/install/product/
host/service/native clipboard/picker/Shell/user config/database/registry operations. Tests synthetic only.

## Reference and gap
Read download note first, then ABDM table MaybeSortableCell/Table sort callback; same key reverses,
new key uses its default and table sort is presentation only. Legacy MainWindow.axaml sortable Button
commands -> MainWindowViewModel.SortBy1817 -> NotifySortHeaders/RebuildVisibleTasks -> SortTasks4593;
same key reverses, Name/Status start ascending. Keep Next defaults and Queue semantics unchanged.
Exact Component0.8.0 Button public ghost/xsmall/compact, content/label/accessibility_label and original
keyed focus/Tab/single keyboard-release activation supply needed capability, no vendor patch needed.
Actual Next header div has pointer-only on_click -> set_download_sort(key,true); original menu uses
false and cannot reverse current direction. Preserve menu's original default-direction behavior.

## Minimal change and invariants
Change download_sort_header_cell only to a real official ghost/compact XSmall Button; retain original
outer table columns/28px header/widths/name left inset/text alignment and dense display through public
layout/content slots. Keep direction arrow and explicit readable Button sort key/current direction.
No painted replacement, second keyboard dispatcher, new focus owner or helper framework.
Reuse unchanged set_download_sort/persist_download_sort/scroll_to_item(0,Top), original deferred
DownloadApplication config section update and low-noise sort diagnostics. Preserve view-only stable
sort, authoritative task/queue order, multi-selection/anchor/search/filter/detail/OLE/native owners.
Do not address #11 clones, #12 detail IO, rem/cursors/wording or other audit items.
Expected own files: main_window/download_page.rs, direct real production tests (existing kit test
fixture or tests.rs), append download reference and this task evidence; no accepted helper/router edits.

## Red-green acceptance
Actual production MainWindow/Root, real Button role/name/Tab focus/Enter/Space, not helper-only tests.
Cover Name/Progress/Status/Created headers at default1060x650 and min920x480; same-key reverse,
new-key default; held repeat/down/up exactly one activation, focus transfer cancels armed release;
pointer same result; actual sort display/readable state and column geometry/focus-visible paint.
Persist both fields to temporary config and reconstruct production application/window to restore.
Original selection set/anchor/tasks/queue unchanged; menu still chooses original defaults. Keep old
pointer and persist assertions, no feature assertion removal. No actual network/download/native effect.
Run focused red-green, MainWindow/download/UI/picker/key/identity/name regressions, strict DesktopClippy
--tests -Dwarnings/workspacecheck/fmt and related source gates; complete Desktop at most once, retain
Archive Unlock and extra QuickLaunch source-gate failures without broad repair or green hunting.
Every Rust shell jobs1/incremental0/codegen256, serial ample timeout/natural idle only. Deliver exact
dirty-baseline own-delta/manifest/logs parent-readable; separate synthetic/source from native/physical
IME/DPI/reader/visual/user acceptance, then stop pending parent review.
