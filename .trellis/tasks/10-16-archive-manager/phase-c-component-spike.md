# Phase C — Archive browser and entry lifecycle: Blocking component spike

2026-10-21. **Spike completed: required DataTable multi-selection NOT supported by the
reviewed public seams; narrow fallback awaits user confirmation.** Tree synchronization
is feasible through public APIs. Phase A/B and real-window NOT RUN obligations remain;
only phase-a-checkpoint.md owns current status. No browser or fallback was implemented.

## Boundary / selection record before code
Smallest question: can locked Kit0.7.1 supply archive multi-row selection/Ctrl/Shift/keyboard
and safe Tree synchronization through its public styled components? The existing code has no
archive browser. Do not build one to answer this question.

Changes: one new cfg(test) Desktop module archive_component_spike.rs and one cfg(test) module
entry in main.rs; task authorization/evidence only otherwise. Minimal synthetic rows/folders;
no archive engine, config, persistence, IPC/native adapter, launch route, window manager or jobs.
No dependency/version/features/vendor/private API edits. Component source is inspected read-only.
No native GUI launch or safety bypass; TestWindow evidence is not physical/UIA/user acceptance.

Requested shape is left Tree + right details table. Candidate APIs:
- component::table::{DataTable,TableState,TableDelegate,TableSelection,TableEvent,Column};
  sources vendor/gpui-component-071/src/table/{mod,data_table,state,delegate}.rs. Actual default
  styled table, Small via Sizable; no appearance changes or custom ordinary controls. Public
  render_tr row/semantic callback composition is evaluated, not presumed sufficient.
- component::tree::{Tree,TreeState,TreeItem} with official ListItem public row slot/default
  style. Base TreeState is re-exported by released Component; public new/items/selected_item/
  set_selected_item/focus are used. No internal state is accessed.
- GPUI Context observe/subscribe/defer bridges public immutable folder IDs through one local
  fixture owner, with original entity notifications and next-turn cross-entity updates. No
  synchronous leased-entity writeback or duplicate project multi-selection controller.

Exact released state currently exposes one TableSelection::{None,Row,Column,Cell}; DataTable
owns row/keyboard dispatch and overwrites row selected semantics after the public row renderer.
Tests will determine actual gesture and extension-slot effects, not simply count exported types.
Three focused tests: stock pointer/keyboard/right-click; public external row-selection seam and
row_selectable(false) tradeoff; deferred Tree<->Table pointer/keyboard synchronization. No broad
format/visual/performance matrix or native-window test.

## Executed evidence and capability result

Fixture: `rust/apps/nxtools-desktop/src/archive_component_spike.rs`, compiled only under
`#[cfg(test)] mod archive_component_spike` in main.rs. Uses actual released DataTable,
TableState/TableDelegate, official Tree/ListItem and GPUI TestWindow, six synthetic row IDs
and two folders. No native window was launched and no source archive/config was opened.

| Required behavior / public candidate | Observed compiled result |
| --- | --- |
| Stock click0,Ctrl-click1 | TableSelection::Row(1); exactly1 semantic Row selected, not additive |
| Stock Shift-click3 | selected_row=3, exactly1 semantic selected Row; no range |
| Stock Shift+Down | remains Row(3),1 selected; no keyboard range |
| Stock Ctrl+A |1 selected, not all6 |
| Stock Down/Up |3->4->3; normal single-row keyboard works |
| Stock right-click row1 after selecting3 |right_clicked_row=1, selected_row remains3; context state separate from selection |
| Stock double click |DoubleClickedRow emitted |
| Public render_tr + external set + row_selectable(false) |Ctrl records IDs{0,1},Shift records{1,2,3}; TableSelection::None and0 selected Row semantics despite supplied aria_selected |
| Same public row override, Down/Shift+Down/Ctrl+A |external set remains{1,2,3},internal None; built-in row navigation unavailable |
| Same override,double click |no built-in DoubleClickedRow event |
| Tree pointer on FolderB |selected_item ID -> deferred owner -> table IDs{20,21} |
| Tree public focus+Up/Down |FolderB->A->B and table IDs{10,11}->{20,21}; same notification path |
| Table double folder0 |deferred owner updates Tree to folder-a and table{10,11} |
| Unchanged Tree notify |deduplicated folder ID, no extra sync count or reentry/feedback loop |

The3tests PASS because they assert the actual capability boundary, including negative results.
**They do not pass the archive multi-select requirement or authorize the browser.** Snapshot
Row selected properties are current-frame TestSupport semantics, not an actual Windows UIA
provider/screen-reader result. No native screenshots,physical input,DPI,user visual acceptance.

### Exact public-source evidence (read-only)
- `vendor/gpui-component-071/src/table/state.rs:59–75`: one TableSelection value, with no row-set
  variant. `selection`480 and `set_selected_row`501 overwrite a single row; event521 sends its
  index without Ctrl/Shift modifiers. `on_row_left_click`804–819 ignores modifiers and gates
  both selection and double-click on row_selectable.
- `table/data_table.rs:17–29,152–165`: fixed stock key bindings/actions. No Shift-range/Ctrl+A
  binding. Public builders only style/size/scrollbar options; TableState public selection has
  no multi-row policy or controlled row-selected getter.
- `table/delegate.rs:91–98`: public render_tr returns Stateful<Div>, so callback/ARIA can be
  supplied. `table/state.rs:2020–2025` then unconditionally applies Role::Row and
  aria_selected(self.selected_row()==row);2242–2272 applies internal selected style and appends
  the internal right-click/left-click handlers. Row override is real but does not own final
  selection semantics. Retaining row_selectable keeps the competing single-row owner;
  disabling it loses the builtin row keyboard/double-click path and still overwrites ARIA.
- `vendor/gpui-component-071/src/tree.rs:15,82–92` reexports BaseTreeState and adapts official
  styled ListItem. Exact resolved gpui-base0.7.1/src/tree.rs:221–256 exposes items/selection
  getters/setters with notify;287 public focus;417–444 own keyboard/pointer state changes.
  TreeEvent93–96 has only Expanded/Collapsed, but observing entity notify + selected_item ID
  and deferring cross-owner work is sufficient in this small fixture.

This is a concrete state/dispatch/semantic limit under the reviewed released APIs, not a
claim that custom callbacks cannot record a set or that all possible compositions were tested.
An inner second selectable row/grid/controller would replace—not fix—the DataTable owner and
misrepresent its outer row semantics; it is not accepted as component compliance.

### Narrowest proposed fallback — NOT implemented; approval required
Permit only the **archive browser details rows** to use official styled ListItem in the
existing public GPUI uniform/virtual-list composition, with a small aligned header/cell layout.
Keep official Tree,splitter,Toolbar,menus,Inputs,Buttons,dialogs and default Kit theme/tiers.
Do not change Downloads or any other DataTable consumer; no dependency/vendor/private API edit.

One browser-view state owns immutable session/revision+entry IDs, selected-ID set, focused ID
and range anchor; visible sort/filter positions are only projections. The same owner processes
Ctrl toggle,Shift range,Shift+arrow,Ctrl+A,ordinary navigation,right-click selection retention,
Enter/double-open exactly once. Official ListItem.selected and public row accessibility
properties project that one state; no nested DataTable selection owner and no painted control
replacement. Explicitly preserve five default columns,horizontal overflow and required resize/
sort behavior; no silent loss because the grid changed.

Before later browser delivery, this fallback still needs its actual production row semantics,
sort/filter/scroll immutable identity,focus/right-click/range keyboard and deferred Tree lifecycle
regressions. Actual native UIA/DPI/pointer/visual acceptance remains separate. No such fallback
proof or implementation is claimed by this spike. Wait for user confirmation before proceeding.

## Actual checks / preservation / stop
- `cargo test -p nxtools-desktop archive_component_spike:: --offline -- --test-threads=1 --nocapture`:
  **3pass,0fail/ignored,871filtered**,exit0. All3actual component tests executed.
- `cargo clippy -p nxtools-desktop --tests --offline -- -D warnings`:exit0.
- `cargo fmt --all -- --check`:exit0. Source/format checks are not component/native acceptance.
- Each Cargo command checked no competing cargo/rustc and inherited jobs1/incremental0/
  RUSTFLAGS=-C codegen-units=256. Two earlier fixture compilation attempts failed due only to
  retained Rc/selector lifetime and misplaced test statement; they executed0tests and are not
  Kit failure evidence. Corrected before the executed3test run.
- Minimal blocking evidence is sufficient: sort/filter/scroll multi-select identity and full
  browser keyboard maps are NOT RUN here, not silently accepted. No Phase B/fullDesktop871
  rerun,codec/large/performance matrix,packaging,install,system changes or GUI bypass.
- Raw logs: current session scratch `phase-c-component-spike-{first,second,third}.log`,
  `phase-c-spike-clippy.log`, `phase-c-spike-fmt.log`; source/dependency/vendor/task hashes in
  `phase-c-spike-baseline/preservation.json`. Final document/preservation results are retained
  alongside these logs. Only authorized task/checkpoint/evidence and test-only module changes.
- Final spec-contracts/component-catalog/task validation all exit0; only the two pre-existing
  oversized-context document warnings remain. Scoped diff/whitespace checks pass. Baseline403:
  402unchanged, only approved ui-spec evidence addition differs; dependency/manifests/lock/vendor
  hashes match. main.rs exact delta is its2line cfg(test) module entry. No cargo/rustc remains.
  Exact preservation result: `phase-c-spike-preservation-results.json`; document exits:
  `phase-c-spike-document-results.json`.

**Disposition:** keep official Tree; request the narrow archive-details fallback above.
No Phase A/B/C or A–F completion claim. Stop; no automatic browser/fallback implementation.
