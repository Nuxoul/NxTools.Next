# Archive workflow presentation

## Scope
Choose sources, choose destination/options, start, inspect actual progress/result. Existing ArchiveApplication, OperationForm, browser, job queue, native source/output and HZ5 journal owners remain. No engine, protocol, format, budget, persistence or native mutation changes.

## Layout and components
- Extraction: source identity and file scope, destination and folder arrangement, name conflicts, concise advisory space state, optional source-space reclamation, then completion preferences. One edge-aligned scroll body; fixed Cancel/Extract footer.
- Creation: bounded source list, name/destination, format and compression level; password and advanced format/source options use explicit progressive disclosure. Existing options and InputState/Select identities survive disclosure.
- Released Component 0.8.0 `form/{form,field}.rs`: Form::vertical, Field label/description/public child composition, normal Medium controls. Form layout is not a state owner.
- `collapsible.rs::Collapsible::{open,child,content}` with real Small outline Button trigger; no reveal motion. Disclosure does not own submission or validation.
- InputGroup::input and actual outlined browse Button share one destination line. Buttons remain outside Toolbar.child, so outline/Medium is preserved.
- Space: no data table before valid input. Pending, available estimate, unknown estimate, shortage, failure and expired observations have distinct presentation. Only actual known capacities appear. Unknown does not imply zero or block ordinary extraction. Existing seven-second native request/deferred receipt and epoch guards remain.
- Reclamation: default-off Checkbox associated with selected archive, current incompatibility reason rather than a permanent format warning. Potential ZIP is not confirmed eligible. Explicit enablement discloses Stored/single-volume/unencrypted/same-volume local NTFS and irreversible source consumption; original separate confirmation and native qualification remain. Fresh continuation has distinct title, recovery ID and remaining-work wording.
- Colors, font and spacing use existing Kit theme/rem helpers; no palette or vendor changes.

## Reference and lifecycle
Official design guide read in full: https://gpui-kit.com/zh-CN/docs/design-guides/. Read archive, archive-editing and virtual-drag reference notes and HZ5 design/wire/independent acceptance. Rechecked legacy tracked ZIP search: package-only API, no general archive UI. Read Archiver main.rs constructor/AppWindow::new -> callbacks -> ui.run, spawn/poll completion and do_extract selection/destination flow; Bandizip SimpChinese.lang IDD_SELECT_FOLDER/IDD_ARCHIVE/IDD_EXTRACT and low-space prompt are resource evidence only. No proprietary implementation was inspected.

OperationForm callbacks keep local changes local, cross-owner work deferred; selection/source/target changes invalidate old space epochs. Automatic refresh coalesces edits through the existing request seam; manual refresh remains for external capacity changes and retry. Browser/preview retirement never grants stale source authority. Ordinary extraction preserves source. HZ5 remains optional/not persisted and native-qualified with unchanged output/journal/release order.

## Verification plan
Production GPUI regressions must cover empty/unknown/shortage/failure/expired states, automatic refresh after edits, source and target invalidation, deferred stale receipt rejection, disclosure and retained field identity, initial/source-specific reclamation states, separate confirmation, minimum/footer bounds and rem/theme layouts. Existing extraction limits, no source deletion, selection, picker, cancellation and HZ5 intent assertions remain required. Isolated Debug native windows use production renderers and synthetic data only; no product service or destructive operation. Actual executed results and outstanding checks are recorded separately.
