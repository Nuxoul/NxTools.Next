# Phase C — Archive browser and entry lifecycle: implementation boundary

2026-10-21 user approves archive-details-only official ListItem/public virtual-list fallback and
complete Phase C ahead of unfinished A/B acceptance. implement.md and sole checkpoint record it.
Only additional permission: existing internal nxtools-archive path dependency for product client.
No third-party versions/features/Kit/vendor edits, Phase D forms/FIFO/E edits/F Shell registration.

## Reference review / retained behavior
Read archive note before Next production changes. Rechecked tracked legacy archive keywords;
only browser-package/FFmpeg ZIP and image/video features found, no archive browser native lifecycle.
Archiver GUI main.rs AppWindow::new115 -> wire_callbacks -> ui.run158 -> process shared temp cleanup159;
view.rs archive_children derives implicit folders, parent_prefix, directories-first sorting.
main.rs activate669 -> folder prefix or view_item697 -> background selected extract711 ->
Msg::Done success/After::OpenFile -> poll300 -> open_path967 (cmd start on Windows). It opens only
after extract success, but shared PID temp root/recursive exit cleanup/no lease/source binding and
raw error/password matching must NOT be copied. SupaZip main.rs App default controller -> eframe
run_native682 -> UI drains events, lib.rs AppController::open_archive460/state projection; egui/Slint
renderers are not reused. Bandizip resources retain tree/details/comment/status and executable
warning vocabulary, not runtime lease/window proof. New Next requirements in design govern safety.

## Invariants and smallest owner gaps
Current Next has saveable settings and real bounded headless ArchiveReadClient but no browser.
Reuse product client/pipe/Job runner/credit/cancel; no direct UI codecs or private worker JSON.
Rust browser owns snapshot, folder/filter/sort/columns and ONE selection/focus/range state.
Metadata projection contains snapshot+ordinal IDs; implicit directories have snapshot+path IDs.
Sorting/filtering cannot retarget a selected file. Navigation clears irrelevant selection.
View callbacks update local state then defer other entities, including Tree and immediate results.
Keyed canonical paths reuse/focus activating windows,960x640/min720x480. Close retires generation,
cancels view reads, stale replies cannot resurrect; source changes retain lastgood read-only warning.
No synchronous IO in render. Metadata budgets/backpressure remain original, terminal requires cleanup.

Entry-open must be bounded single selected file, active-content consent before execution,512MiB
budget and depth<=3; original native protected extraction and source revision binding precede Shell.
Temp roots are GUID-owned cache/archive sessions, held lease, no arbitrary config cleanup, child
nested lease independent. Shell open receipt is not external-process lifetime; retire only own
inactive session, retain locked files for later cleanup. No source deletion/recycle.

## Component ledger / parent adaptations
Tree::new/TreeState public IDs+observe/defer, official ListItem; archive details ListItem.selected
and public role/selected semantics on one state in GPUI uniform_list; no DataTable state/custom ordinary
controls. Five default columns+optional methods/encryption/CRC/attributes/path; public resize grip
and shared horizontal viewport, stable IDs. Other DataTable users untouched.
Toolbar::content hosts actual Button::outline default Medium32 (not Toolbar.child ghost adaptation).
Per-window DropdownMenu/PopupMenu; global AppMenuBar not changed. Official h_resizable/resizable_panel,
StatusBar, Input/InputState, Dialog/Alert, Empty, Tooltip and KitDefault themes. Only real current
actions shown; D/E operations absent or explicitly unavailable, never fake success.

## Expected source seams / checks
New Desktop archive/{model,browser,application,...} own feature; main composes owner; settings page
opens via owner; platform_events forwards picker/Shell/native terminals; narrow protocol/C# picker/
entry lease commands only if absent. Existing product metadata extended only for real display/source
fields with compatible defaults and bounded serializers; no secret paths in diagnostic fields.
Red-capable tests first at model,view,worker/native boundaries; then scoped tests/strict lint/fmt,
original lifecycle gates and one final Desktop suite. C# safe tests LIST then same RUN. Native GUI
input/screenshots/DPI/UIA/user acceptance remain NOT RUN under earlier self-app denial; no bypass.
