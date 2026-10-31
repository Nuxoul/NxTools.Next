# Audit #2 only — download sortable headers with keyboard equivalence

## Outcome, scope and canonical phase

User directly requested “开始下一步” after the four prior slices were parent-accepted. Implemented
only original audit #2, Downloads stabilization; no canonical phase renumbering or list migration.
Ready for parent independent review, **not self-accepted**. Stop after this fifth single-item delivery;
no sixth item, recursive delegation, commit/push, package/install, product/PlatformHost/service launch,
real clipboard/picker/Shell/user-config/database/registry operation. Existing tests use their original
temporary/in-memory/loopback fixtures, not external network or real downloads.

Per-file current dirty baselines were saved before edits, not git HEAD. Three existing files changed:
`main_window/download_page.rs` only the header function, append-only existing
`main_window/download_page/kit_migration_tests.rs`, append-only download reference. Existing
`main_window/tests.rs` was baseline-recorded but remains byte-identical. No unrelated dirty work claimed.

## Reference and exact component selection

Read AGENTS, current UI selection/release/table/API index, applicable Desktop/shared specs and owning
staged plan; read download reference first, then ABDM before legacy NxTools.
- ABDM `shared/app/src/desktopMain/.../table/customtable/TableUtils.kt::MaybeSortableCell178–203`
  reverses the already-sorted column, otherwise takes the default. `Table.kt` sortable indicator359–375
  feeds the same sort model. Presentation sorting does not change queue admission or stable task ID.
- Legacy `Views/MainWindow.axaml1101–1120` has actual sortable Buttons bound to SortByCommand;
  `ViewModels/MainWindowViewModel.SortBy1817–1831` reverses same key, otherwise Name/Status ascending;
  NotifySortHeaders4616–4635 renders arrows and SortTasks4593–4614 projects ordering only.
- Original Next menu invokes set_download_sort(key,false); headers invoke true. Menu is already
  keyboard-operable but cannot reverse direction, so it was not equivalent to pointer headers.
  Parent's bounded handoff explicitly preserves menu default-direction semantics; menu was not changed.
- Exact Component0.8.0 Button::render obtains its original keyed FocusHandle and Tab binding, reports
  Role::Button and explicit name, and uses the original Base one release-click for Enter/Space.
  Public ghost/xsmall/compact and child content support the dense header. Framework focus_style paints
  a ghost edge line; its `focus-ring` debug selector is only library cfg(test), not app-test capability.
  No vendor/framework/helper patch or second key/focus dispatcher needed.

Chosen layout: keep original `download_table_header` columns, widths, name36px left inset,28px outer
height and original positional debug selectors. The existing slot contains an official XSmall
**ghost/compact Button** with full available width/height and zero additional padding; this is an
explicit adaptation to the existing measured header, not a new density system. No Toolbar parent
transformation involved. Public child div remains left aligned and explicitly retains existing
TEXT_META text size so XSmall's default text token does not silently enlarge the header.

The actual Button ID is (download-sort-header-button, key), within the original header/window scope.
Its readable name is 按{column}排序, or 按{column}排序，当前升序/降序 when active; original visible
column text and ↑/↓ stay. Official Button owns keyboard focus edge/hover/activation. Existing callback
still calls set_download_sort(key,true), original scroll_to_item(0,Top), persist_download_sort and notify.
Same-key reverse/new-key defaults, stable view-only ordering and Queue semantics remain unchanged.
The original menu key,false, algorithm, multi-selection/anchor/search/filter/detail/virtualization,
OLE/native and deferred DownloadApplication persistence are byte-identical outside the header function.
#11 cloning/#12 detail IO, global rem/cursor/wording and other audit tasks were not addressed.
The pointer cursor naturally becomes official Button default only on the replaced header; no #7 sweep.

Existing low-noise download.list.sort_changed and original preference save events remain; no new operation,
IPC/schema/logging/high-frequency probes or content logs were added. No temporary DEBUG markers remain.

## Red-green and fixture convergence

All Rust shells inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256; serial ample timeouts. Twice pre-start guards found unrelated Rust
processes: no own Cargo was launched; waited naturally. Later wait-and-start used one shell. No process
was killed/paused, no other session was asked to yield/reserve/reorder, no duplicate build launched.

`cargo test -p nxtools-desktop sort_keyboard_ -- --test-threads=1`:
1. Valid pre-fix red (`sort-red.log`): compiled, **0 PASS / 3 FAIL**. Actual production MainWindow header
   lacks Role::Button/focus/key node. This is semantic-boundary red, not a claim that release/state
   assertions ran against the old div. Final tests remain red-capable at the same node boundary.
2. After actual Button replacement, role/name/geometry assertions passed, but all3 failed Tab reachability:
   new fixture called blur before Tab, leaving no Root dispatch focus path. Corrected test start to original
   download search InputState; product had no additional change.
3. Then2/3 passed; test incorrectly looked for library-only cfg(test) focus-ring selector. Replaced this
   observer with actual painted quad border/position/size/theme-ring assertions, retaining focus coverage.
4. A later added row-order assertion attempted semantic snapshots for ordinary layout div rows. Switched
   only observer to existing actual row debug bounds (two static-lifetime selectors, existing fixture
   pattern), not a product semantic/list change. No feature assertion removed.
5. Final production/test source: **3/3 PASS** (`sort-final.log`,14.38s). MainWindow tests177/177,
   including unchanged old pointer/persistence/menu tests. Intermediate failures retained in evidence;
   not mislabeled as new product defects or final-test full original-tree replay.

## Acceptance mapping — real MainWindow/Root/control/IO boundaries

Tests append to existing production workspace tests, using original MainWindow/DownloadApplication,
original two queue-only synthetic tasks/in-memory engine, temporary config file and official Root.
No manually invoked header callback, replacement list/controller or native effect.

| Test | Verified |
| --- | --- |
| sort_keyboard_actual_headers_have_names_tab_focus_and_original_column_bounds | Actual Name/Progress/Status/Created Button role+action name; original Tab path from original search; real focused binding/keyboard modality at1060x650 and920x480;28px header/Created width/containment; actual official theme ring border quad matching focused Button bounds |
| sort_keyboard_enter_space_match_pointer_reverse_once_and_preserve_task_identity | All4 keys: Enter down+held repeat does not dispatch, release chooses new-key default, duplicate release does not toggle, Space reverses same key, pointer reverses through same callback; readable current direction; actual virtual row Y-order matches original view sort; original authoritative tasks/queue order and dual selection+primary+anchor unchanged |
| sort_keyboard_persists_both_fields_and_reopened_main_restores_direction | Enter selects Created default descending then Space reverses; original deferred config owner writes key+false into temporary disk config; focus transfer to search cancels armed Enter release; fresh DownloadApplication initialized from disk-read AppConfig and new MainWindow constructor restore same direction |

Original tests unchanged and passed: clicking_download_table_headers_changes_only_the_view_sort,
download_table_header_clicks_persist_the_sort_preference, original column/header row alignment,
search/filter/menu sort test and other MainWindow lifecycle/keyboard/selection tests. Menu's original
set_download_sort(false) and all six sort keys remain unchanged. No visual/native/reader acceptance is
inferred from these synthetic focus or paint observations.

## Final verification

| Command/check | Actual result |
| --- | --- |
| sort_keyboard_ focused | **3/3 PASS** |
| `cargo test -p nxtools-desktop main_window:: -- --test-threads=1` | **177/177 PASS** |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | **185/185 PASS** |
| `cargo test -p nxtools-desktop ui:: -- --test-threads=1` | **231/231 PASS** |
| strict `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| `cargo check --workspace --tests` | PASS |
| `cargo fmt --all -- --check` | PASS; only own two files formatted, skip_children |
| Scoped `git diff --check` | PASS |
| Exactly one complete `cargo test -p nxtools-desktop -- --test-threads=1` | **1196/1196 PASS**,0ignored/filtered,225.28s |

The current full run includes accepted #1 keyboard8/8, #9 picker14/14, #10 Clipboard identity11/11,
#3 accessible names7/7 and this sort3/3 PASS. Earlier complete Desktop1191/1 Archive Unlock failure
remains historical evidence; this slice has **no Archive edit** and does not claim to have fixed it.
Current shared working tree's actual single run has Archive Unlock PASS and total1196 (other authorized
work can change unrelated test counts). Do not attribute unrelated convergence or release acceptance
to this header change; no second full-suite run or green hunting was performed.

Static **10 PASS / 1 FAIL**: Download UI, Desktop shell, Clipboard UI, Clipboard, UI state, UI token,
Clipboard gate mutation5/5, GPUI release, Markdown editor, spec114documents/14owners PASS. No gate changed.
Extra quick-launch-lifecycle.ps1 remains **FAILED exit1 at1765**, same elevated-navigation compound
source gate as previous slice. Current build.ps1 still delegates publish-release-hosts.ps1 rather
than directly naming write-release-binary-manifest.ps1. This slice does not modify build/release/security
source or weaken this gate, and does not certify that boundary. Full previous bounded diagnosis remains
in accepted #3 outcomes/evidence; current failing log is included here.

## Delta integrity and protected owners

Three existing-file own-deltas relative to this task's dirty baselines (not HEAD):
- main_window/download_page.rs: only download_sort_header_cell,35added/19removed (format-aware diff).
- main_window/download_page/kit_migration_tests.rs:361lines appended, prior924lines unchanged.
- docs/reference-code/download.md:18lines appended, prior reference unchanged.
No MainWindow/tests.rs edit, no existing pointer/feature assertion deleted.

Exact protections **15/15 PASS**: ten whole accepted files unchanged (#1/#9 new/settings/picker/
keyboard and #3 action/name tests, #10 history/panel/identity); original MainWindow/tests.rs unchanged;
production prefix before header and suffix after header unchanged (menu/algorithm/render list/detail/
persistence/modules); original existing kit-tests prefix and download reference prefix unchanged.
This is byte/hash proof, not an assertion that only this session has dirty files in the repository.

Parent-readable `evidence/own-delta.json` supplies exact baseline/delivery text per operation and line
positions. `evidence/manifest.json` records baseline/current hashes, new task docs and evidence hashes.
Logs include valid red, fixture diagnostic failures, final scoped/strict/single-full results and current
source-gate failure. Ad-hoc scripts remain in session scratch. Review artifacts are not patches to apply.

## Remaining limits / stop

No item-specific source/compiled/synthetic blocker remains; parent review and acceptance are pending.
Extra Quick Launch gate remains failing. Current full Desktop is technically green, but this does not
retroactively clear historical failures or certify installed behavior. Actual Windows UIA/provider/
reader, physical keyboard/pointer/IME, real DPI/mixed-monitor, screenshot/visual/user acceptance,
native popup/picker/Shell/clipboard/OLE effects, product/Release/install/service acceptance are **NOT RUN /
NOT ACCEPTED**. Only source/compiled/synthetic/render-paint evidence is delivered. Stop after this item;
no next backlog task without a new single-item user/parent assignment.
