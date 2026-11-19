# Archive browser navigation and Find

## Reference boundary

No indexed old NxTools general archive browser owner is available. The saved Bandizip 7.32.0.1 records under `.trellis/tasks/10-31-archive-research/` are static shipped-artifact evidence, not runtime acceptance or reusable source.

- `bandizip-browser-navigation.md` N8/N9: saved `FUN_1401574b0` asks for the current tree item's parent; null parent is a no-op. After parent selection rebuilds the list, `FUN_1401a73c0` resolves the departed child pointer, sets selected/focused state and calls `LVM_ENSUREVISIBLE`. It restores one child, not historical arbitrary multi-selection.
- `bandizip-browser-integration.md` E1–E8/E24: `CFindDlg` constructor `0x140110610` → main `0x14013b940` → `DialogBoxParamW` template `0xcd`; `0x140111260` scans the loaded catalog, supports empty/all, path substring and basename wildcard and abandons changed queries. Selected result `lParam` resolves through original catalog indices at `0x140112c50`; Extract/Delete use shared original operation owners. Modal accept navigates the originating tree/list; cancel does not change browser selection. The saved assembly/callee record `bandizip-browser-analysis-0x140112eb0.json` binds deletion to the originating dialog and edit owner.
- Encoding menu → `0x14013a8d0` → `0x1401579a0` captures internal location, reopens with the selected code pages and restores the location after successful catalog replacement. Navigation resolves the nearest surviving ancestor when the full path is gone.

## Next entry and lifecycle

`ArchiveApplication` owns the source/session/connection and complete read; `ArchiveBrowser::install` replaces immutable `BrowserModel` metadata. `RowId::Entry(snapshot, ordinal)` and `RowId::Folder(snapshot, path)` remain the only row identities. Noncontiguous ordinals are not vector offsets. `member_path` normalizes separators only for presentation and matching.

`ArchiveBrowser::keyboard`/menu → `Command::Up` → parent projection → one departed-child selection/focus → snapshot/epoch-guarded nearest reveal. Root Up does not modify filter, focus or selection. Details and name-list keep their original public virtual viewport.

Find uses the original browser Root modal and a browser-owned, snapshot-bound query surface. Query events read only the loaded model; they neither reopen a source nor start a Job/decoder. Query selection is independent until an explicit Navigate/Extract/Edit action. Each action rechecks the originating browser generation, snapshot, query generation and retirement before installing original IDs; extraction/editing then uses the existing capability and final-submit checks. Refresh, encoding, source failure, tab suspension and close invalidate the modal's authority. Closing it does not cancel source operations or mutate unrelated browser selection.

`install` restores the prior internal folder or nearest surviving ancestor for replacement snapshots, clears old row IDs and preserves initial-open-only single-root entry. Tree work belongs to catalog installation; projections belong to state transitions, not render. Preview and edit readiness are presentation facts cached at those transitions; actual commands retain authoritative selection expansion.

## Component selection

Find: original `WindowExt::open_dialog` / `Dialog` width 720, title and public child slot; `Form::vertical().child(Field::new().label(...).child(...))`; stable `InputState` inside `InputGroup::new(...).input(Input::new(...))`; official unchecked `Checkbox::on_click` for wildcard mode; `ListItem::new(...).selected(...).accessibility_label(...).on_click(...)` inside public `uniform_list`, 32px rows and a bounded viewport. Direct Medium outlined Navigate/Extract/Delete/Close `Button`s compose the official fixed `DialogFooter`, outside the scroll body and `Toolbar.child`; theme and typography remain unchanged. Parent layout arranges official controls.

Verified OPT-07 boundary: both deferred Tree publications read the latest same-snapshot model folder/focus. Result element identity and callback both bind query generation plus original RowId; action additionally revalidates Find epoch/browser generation/Ready snapshot. Selected Extract final admission checks the registered original WindowSlot and loaded read connection; Modify retains EditFormOrigin. `MemberQuery` reuses locked globset in nxtools-archive with1024-byte/control/malformed-glob refusal. Transition-owned edit readiness/preview visibility remove repeated-draw catalog scans while final commands keep original validation. See [executable contract](../../.trellis/spec/nxtools-desktop/backend/archive-browser-workflows.md).

`find_action` resolves original entry/folder identity, safe path and shared `model::entry_warning` policy before closing Find or entering the target parent. Warning Extract/Delete therefore preserves the existing query/input/result and all browser/projection facts with no form/Job/command; Navigate keeps the original warning-row selection path. The two original-footer regressions live in `application_tests.rs`; no catalog clone, new controller or archive I/O is involved.

## Invariants and pending acceptance

Callbacks update their local entity first and defer cross-entity actions to the next application turn, including cached/immediate paths. No query, member name, source path or payload is logged. Low-frequency query lifecycle diagnostics carry only correlation, work counts, status and elapsed time.

The existing preview generation/snapshot/epoch/selection fingerprint, exact lease adoption, independent encrypted drag grant and transferred-consumer lifetime remain authoritative. Internal preview never executes a member; explicit Open/OpenWith/nested-open retain their original confirmation and lease owners.

Source tests and controlled GPUI events do not establish physical focus/keyboard/DPI or user visual acceptance. G-18 remains pending; no product GUI or Explorer operation is implied by these records.

## Fixed-height Tree label

`archive/browser.rs::folder_tree` publishes the source-derived `flutter_assets` name as one `TreeItem` label. Released Component0.8.0 `tree.rs:80-133` wraps the supplied ListItem in Base Tree's virtual scroll row; `list/list_item.rs:198-255` centers its public child slot but does not impose a single text line. The ArchiveBrowser Tree render item fixes each ListItem at32DIP and constrains the left pane to160–280DIP; `.text_ellipsis()` alone does not stop its child text from wrapping. The resulting second line is outside the row. A single-line shrinking label with its existing full-name Tooltip and accessible name keeps the same tree/selection/scroll, disclosure, pressed gesture and fixed-pane ownership. No archive I/O or native window geometry changes.
