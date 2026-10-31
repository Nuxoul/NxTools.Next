# GPUI guide audit — final engineering review and acceptance handoff

Date: 2026-10-26 · Workspace: `D:\Code\NxTools.Next-main` · Delivery type: documentation only.

## 1. Executive decision

**The remediation programme is not complete and the project is not declared globally green, visually accepted, natively accepted or release-ready.** This report consolidates evidence; it does not perform another remediation, review delegation, build, product launch or acceptance run.

- **Seven original findings have existing limited parent acceptance:** #1, #2, #3, #4, #5, #9 and #10. Their acceptance is source/compiled/synthetic interaction only, at the reviewed delivery, not whole-project or current installed acceptance.
- **Four later findings have bounded implementation/regression evidence:** #8, #11, #12 and #13. They are not assigned new global acceptance merely by this report. #8 retains broad integration failures; #11–#13 retain baseline quality limitations.
- **Two findings remain substantively open:** #6 ordinary-rem consistency is partially implemented; #7 cursor semantics was not repaired. Queue08–11 gaps are not merely missing final paperwork.
- **Final convergence was not admitted.** The preceding [admission decision](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-final-convergence-admission/outcomes.md) found incomplete prerequisites. Final-stage Rust invocations: **0**. Final-stage complete Desktop attempts: **0**. New full-suite passed/failed/ignored/exit: **NOT RUN**, not an invented zero-failure result. The one permitted final integrated attempt remains unconsumed.
- Latest recorded test-target strict Clippy and workspace check passed in the #13 delivery; latest recorded production strict Clippy, workspace formatting and the complete Quick Launch source gate did not all pass. No present syntax/type compiler failure is inferred from those warning/source-test failures.

The **historical accepted count remains 7/13**. The status partition above is not a new 11/13 closeout or an 84.6% completion claim. No physical/native/DPI/reader/visual/user/release acceptance is newly granted.

Chinese user handoff: [中文摘要](D:/Code/NxTools.Next-main/docs/audits/2026-10-26-gpui-guides-summary-zh.md). Exact source/evidence inventory and hashes: [handoff evidence](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/source-index.json).

## 2. Authority, scope and evidence vocabulary

The original review concerned the [GPUI design guide](https://gpui-kit.com/docs/design-guides/) and [coding guide](https://gpui-kit.com/docs/coding-guides/). The issue descriptions below are reconstructed from the original task PRDs, parent reviews and frozen outcomes; no standalone verbatim initial thirteen-item transcript was found in the inspected project documents. The guide snapshots used by the work remain recorded with the owning tasks. This report does not claim a new live website audit.

Current framework authority is Kit/Base/Component/Assets **0.8.0**, default **gpui-pre0.3.9**, not the historical 0.7.1 catalogue. [AGENTS](D:/Code/NxTools.Next-main/AGENTS.md), [verification levels](D:/Code/NxTools.Next-main/.trellis/spec/shared/verification.md), [current UI authority](D:/Code/NxTools.Next-main/.trellis/spec/shared/ui-design.md) and [current release/API delta](D:/Code/NxTools.Next-main/docs/reference-code/gpui-kit-080.md) govern interpretation. Rust owns state/layout, C# Windows APIs and GPUI rendering. Reference limits remain explicit: ABDM precedes legacy download compatibility; Listary evidence is its managed boundary, not proprietary native-engine source.

| Mark | Meaning in this report |
| --- | --- |
| S | Source/reference/ownership review or source gate; not proof of runtime behavior |
| C | Actual compilation/lint/check at the recorded time; not automatically current or native |
| Y | Synthetic regression reaches the real production renderer/controller/action boundary, using temporary config/SQLite or memory bridges; not a physical user or installed runtime |
| P | Existing parent limited acceptance; distinct from this documentation-only synthesis |
| OPEN | Missing implementation, meaningful regression or required verification remains |

A test using production views is called **production-boundary synthetic**, never a real product acceptance run. Test counts overlap and are not summed into an invented unique total. Frozen logs control over filenames such as “green”. A source/hash pass does not convert an unexecuted check to PASS.

## 3. Per-finding disposition

Source paths in this section are relative to `rust/apps/nxtools-desktop/src/` unless stated otherwise. The [machine-readable source index](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/source-index.json) supplies the exact task file lists and recorded delta paths; this report identifies the behavior-owning changes and essential fixture adaptations, not a claim that every line in a touched file belongs to one finding.

### #1 — NewDownload default Enter preempted focused controls and schedule Popover

**Original problem.** A window-level Enter interceptor could call `submit(StartNow)` while a child schedule/category layer or official focused Button/Select owned the gesture.

**Actual changes.** [new_download_view.rs](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/downloads/new_download_view.rs) restricts default submission to original URL/directory/file-name inputs and eligible noncomposing, nonheld, unmodified state; child/focused controls remain first. [new_download_keyboard_tests.rs](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/downloads/new_download_keyboard_tests.rs), the existing schedule fixture and Enter source-string assertion inside `downloads/tests.rs`, and the append-only reference were updated. The date fixture scrolls into the actual viewport; production calendar behavior was not changed.

**Real boundary evidence.** Eight actual production GPUI key-event regressions cover child layers, Select, IME/character preference, modifiers, held/repeated input, busy/native gates and once-only original probe/submit dispatch. Parent reran **8/8 keyboard**, **168/168 Downloads**; strict/check/fmt results are separately attributed delivery evidence, not fabricated parent reruns.

**Disposition.** **S/C/Y/P closed for this slice.** No native picker, physical Enter/IME, focus or full-project acceptance. The historical implementation full run **1158 pass / 2 fail** remains failed. Sources: [PRD](D:/Code/NxTools.Next-main/.trellis/tasks/10-24-download-enter-popover/prd.md), [parent acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/10-24-download-enter-popover/parent-review.md).

### #2 — Pointer-only sortable Download headers lacked keyboard equivalence

**Actual changes.** `main_window/download_page.rs::download_sort_header_cell` became an actual official ghost/compact XSmall Button, with public content/name and the same dense header geometry. Original `set_download_sort`, scroll-to-top and deferred section persistence remain the only owners. Tests append to `main_window/download_page/kit_migration_tests.rs`; reference append is isolated. No new sort/selection controller or authoritative queue reorder.

**Real boundary evidence.** Actual MainWindow/Root headers at default/minimum size, Tab, Enter/Space down/up, held/repeated release, focus-transfer cancellation, pointer equivalence, direction/defaults, rendered ordering and temporary-config reconstruction. Parent **3/3 sort**, **177/177 MainWindow**, **185/185 Downloads**, **231/231 UI**, strict Clippy and ten source/document checks passed.

**Disposition.** **S/C/Y/P bounded closed.** Historical implementer full **1196/1196** is a dated prior run, not current final convergence or credit for unrelated shared-tree fixes. The then-extra Quick Launch gate still failed. [PRD](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-download-sort-keyboard/prd.md), [parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-download-sort-keyboard/parent-review.md), [own deltas](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-download-sort-keyboard/evidence/own-delta.json).

### #3 — Actual Buttons lacked action/object-specific accessible names

**Actual changes.** `ui/components/action.rs` requires semantic names on primary/icon helpers; `path_list.rs` names the actual delete Button. Download Settings/new/task/schedule/window-support and Global callsites forward complete object purposes. Helper tests in `ui/components/action_name_tests.rs`, production semantic tests and necessary lifecycle assertions were added/adapted. Arbitrary child content, icons and Tooltips are not treated as the Button name; outer div labels are not substituted for actual control semantics.

**Real boundary evidence.** Rendered actual Button role/name/state under normal/disabled/loading, original click/KeyUp owner, production Apply/picker/path-delete controls and preserved hit/paint. Effective seven-test red was name `None` at actual controls. Parent **7/7 names**, **185/185 Downloads**, **35/35 Global**, **231/231 UI**, strict and source gates passed.

**Disposition.** **S/C/Y/P bounded closed; UIA/reader remains unaccepted.** Historical full **1191 pass / 1 fail** (Archive Unlock) and source-gate failure are not erased. [PRD](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-button-accessible-names/prd.md), [parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-button-accessible-names/parent-review.md), [exact twelve existing-file deltas](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-button-accessible-names/evidence/own-delta.json).

### #4 — Archive conflict/overwrite prompts did not identify the current complete decision object

**Actual changes.** Presentation-only optional `ArchiveJobPrompt.display` spans `rust/crates/nxtools-protocol/src/archive_job.rs`, archive `worker_conflict.rs`/`worker_output.rs`, C# `ArchiveJobContract.cs`/`ArchiveJobSession.cs`, and Desktop `archive/progress.rs`. The original ticket carries incoming member/current target; missing old context has an explicit honest fallback. DescriptionList/Label/body scrolling and fixed footer keep full Unicode identity and apply-all/actions reachable. `archive/readmission.rs`, CLI/session fixtures only receive necessary `display:None` construction adaptations; the new worker contract and original job/C# tests exercise producer/privacy/safety boundaries. Native observation tokens, overwrite decisions, path safety, budgets and publication do not consume the new display fields.

**Real boundary evidence.** Actual typed notification → job state → official Dialog tests, equal-size/different objects/jobs/tickets, missing context, long identifiers/body end and once-only decisions. Parent R1/R2 **display6/6 + original jobs11/11** passed. Producer contract **2 tests across18 format/strategy runs**, direct safety39, Archive Protocol6, IPC42 and exact C# LIST5/RUN5/PASS5 are separate cross-layer fixtures, not one live end-to-end product session. Compatibility is **new receiver reads omitted old fields only**; old strict receivers reject populated new fields, so no bidirectional mixed-version claim.

**Disposition.** **S/C/Y/P closed for display only.** Initial new CreateOverwrite Space diagnostic failed; R1 preserved its complete76lines outside default registration, and #5 later restored/repaired it. Initial focused6/7, full **1207 pass / 1 fail**, Protocol79/80 and failed lint/fmt remain immutable; later targeted passes do not make those historical suites green. [R1/R2 parent acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-archive-prompt-object/parent-review-r1-r2.md), [production/verification history](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-archive-prompt-object/outcomes.md), [own deltas](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-archive-prompt-object/evidence/own-deltas.json), [preserved diagnostic](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-archive-prompt-object/evidence/revision-r1-r2/unresolved-diagnostics/create-keyboard-unresolved-probe.rs.txt).

### #5 — Safe default submission and focused confirmation keyboard activation were incomplete

**Actual changes.** `archive/forms.rs`, `archive/progress.rs` and `downloads/schedule_view.rs`, with direct `forms_tests.rs`, `job_tests.rs`, `downloads/polish_tests.rs` and reference append. Actual focused prompt Button invokes public Confirm → original Dialog `on_ok` → deferred ticket reply; destructive approval is never root/unfocused Enter. Form fallback is limited to allowed editing controls; TimeField requires real enabled focus. Input/IME, masked password/multiline, Select/Popover/Dialog, hold/modifier/busy and retirement gates remain.

**Real boundary evidence.** The complete prior76line diagnostic was restored with only audit-name substitution. Actual pointer/Tab/focused Enter/Space/release, duplicate/old release and new-ticket cancellation; form typed admission; original schedule SQLite SetSchedule/reopen with unchanged AppConfig. Parent **audit05 15/15**, **Archive134/134**, **Downloads188/188**, identity11/names7/sort3 and strict/check/fmt passed.

**Disposition.** **S/C/Y/P bounded closed.** Historical worker full1232/1232 was hash-verified, not a parent rerun or current full acceptance. [Parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-default-submit-keyboard/parent-review.md), [eight dirty-baseline deltas](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-default-submit-keyboard/evidence/own-deltas.json).

### #6 — Ordinary layout/type/spacing used fixed pixels inconsistently with rem

**Original problem and invariant.** Ordinary application roles must scale on the original measure/render axis using `ordinary_rem(design_pixels)=rems(design_pixels/16)`, not a fake relative wrapper or a global font/zoom rewrite. Fixed native/content/bitmap/hit budgets and official mixed-unit internals are separate, named exceptions, not a zero-`px` requirement.

| Delivery / actual source family | Production-boundary evidence | Disposition |
| --- | --- | --- |
| Initial `downloads/schedule_view.rs`, `main_window/download_page.rs`, `ui/metrics.rs`, direct polish/kit tests | Actual Root16/14/18 row/header measure/paint/end-scroll/stable selection and Schedule long validation/Tab/Space/Cancel; parent6/6, Downloads191, page21, strict/check/fmt | **S/C/Y/P limited** to Schedule + Download48row/28header; #6 stayed OPEN |
| Queue04 `new_download_view.rs`, existing/new `new_download_rem_tests.rs` | Effective Root14 guidance red and40line error natural-scroll red; final5/5, Downloads196, keyboard8/picker14 and protected subsets | **S/C/Y bounded** basic body/rails/inspector/error; original native clients unchanged |
| Queue05 same view + `new_download_overlay_rem_tests.rs` | Category/calendar/media red; real wheel end/pointer/Radio/Enter; original media-error footer456 budget retained; final5/5, Downloads201, combinedrem16 | **S/C/Y bounded** overlays/media; one original Scrollable, independent list IDs, fixed outer caps |
| Queue06 `downloads/settings_view.rs`, `task_window.rs`, `settings_task_rem_tests.rs` | Root14 lane/type reds; long category/error/40segments end/actions/Apply-failure-retaineddraft/retry section merge;6/6, Downloads207, combinedrem22 | **S/C/Y bounded** Settings/task; latest outside workspacefmt failure preserved |
| Queue07 shared `ui/components/{action,field,settings,titlebar,state,custom_command_editor,path_list,volume_scope,quick_launch_diagnostics,hotkey,tooltip,menu}.rs`, original consumers and two direct rem test modules |9direct tests, combinedrem31, UI237/Downloads207/Main191/Global36; tight Root18 real command Save after official nested scrollbar arbitration | **S/C/Y bounded**; production-only animated-field strict failure retained; exact23-entry delta index includes tests/reference |
| Queue08 `main_window/shell.rs`, `tools_page.rs`, currency/floating settings, `network_settings.rs`/`blacklist.rs`, two page-rem test files | Effective shell/Tool/Global reds and limited green;24long rules end/edit/Tab/Space/failedApply/draft/latest-section merge | **INCOMPLETE**; Currency/Floating full rem consumers not executed; Extensions28/80/88 ordinary repairs never landed |
| Queue09 Quick/Clipboard consumers | Inventory/reference/fixed-role reconciliation only | **NO IMPLEMENTATION**, no effective Root red or compiled rem closure |
| Queue10 Archive/residual consumers | Corrected lexical/role matrix and source review only | **NO IMPLEMENTATION**, no effective Root red/green; no Archive phase advance |

**Disposition.** **OPEN; no global S/C/Y closure.** The [initial parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-rem-role-consistency/parent-review.md) is expressly partial. Exact continued results: [basic](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-new-download-basic/outcomes.md), [overlays](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-new-download-overlays/outcomes.md), [Settings/task](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-download-settings-task/outcomes.md), [shared](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-shared-ui/outcomes.md), [Queue08](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-pages-global-tools/outcomes.md), [Queue09](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-quick-clipboard/outcomes.md), [Queue10 role matrix](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-archive-final-matrix/role-matrix.md). Residual classifications and exact exceptions are in §4, not silently cleared by later performance tests.

### #7 — Ordinary commands/selections used pointing-hand cursors

**Original problem.** Arrow is expected for ordinary native commands/selections; hand only for actual links/link-like content; Text/Resize/Move/Grab/Prohibited require actual manipulation semantics. A URL in a result payload alone does not turn a command row into a Link.

**Actual modifications.** **None.** Queue11 found12 declarations:11 production across8files and1file-level test helper. Candidate owners include `focused_action`, path/volume/command-master rows, launcher result/action and attached rows, NewDownload category and Clipboard add-category. Nested controls may override cursor requests, so source hits are not proven paint defects.

**Evidence and limit.** Published pre0.3.9 reverse-painted-hit resolution and official Button/Link source were read. A compliant actual final-cursor paint-hit observation was not established/compiled; no fake getter, copied resolver or vendor patch was introduced. Natural-idle blockage prevented effective red/repair/green.

**Disposition.** **S inventory only; C/Y repair NOT RUN, OPEN / NOT REPAIRED.** Genuine links, editor Text, panel/taskbar resize/move and active-drag exceptions stay protected. [Inventory and exact owners](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-cursor-semantics/inventory.md), [outcome](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-cursor-semantics/outcomes.md).

### #8 — Command labels did not accurately communicate further input/confirmation or immediate results

**Actual changes.** Thirteen existing Rust files plus new `main_window/command_naming_tests.rs` and three append-only references. Clipboard Export/Import/Migrate, Download Settings/Schedule/New/task-window/pickers/browser export, Tool Configure and image selection gain ellipsis only where the real owner opens another surface. Backup/Compact/direct Cancel/list removal/final delete remain immediate without a blanket replacement. `downloads.rs` adds a read-only policy getter; Download menu wording projects the same current deletion setting without changing flags or capabilities. `ui/components/menu.rs` exposes the same rendered content through a Label semantic slot, not a new action/native provider. Tool and existing fixture adapters preserve original callbacks.

**Real boundary evidence.** Effective actual-label reds; final **12/12**. Actual pointer/Tab/Space/Enter/KeyUp/duplicate release, fake picker/empty-cancel, config bytes, keyed Tool reuse and original confirmation/Cancel. Full object-purpose labels retained. Queued synthetic record/sentinel checks do **not** prove native completed-file deletion; no new test commits delete-artifact=true. Related Downloads210/UI237 and protected audit04/05/06/identity/name/sort groups passed.

**Disposition.** **S/C/Y bounded implementation, not global closure or new P acceptance.** MainWindow **206 pass / 6 fail** remains a genuine recorded integration failure at independent Extensions storage/removal admission. Then-test strict failed nonminimal_bool; later #11–#13 test strict passes are separately dated, not proof this old log passed. [Command matrix](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-command-naming/command-matrix.md), [outcome and exact attribution](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-command-naming/outcomes.md), [delta index](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-command-naming/evidence/own-deltas.json).

### #9 — Late directory-picker results could overwrite another window or newer draft

**Actual changes.** Original `DownloadApplication` in `downloads.rs`/new private `downloads/directory_picker.rs`, original NewDownload/Settings local draft callbacks, new `directory_picker_tests.rs` and existing picker fixture integration. Same weak view + GPUI window + operation UUID + Change revision/current value replaces Settings/NewTask-type broadcasting. Pending exists before dispatch; claim and final deferred application revalidate separately. One pending slot remains; no new controller/native picker implementation or autosave.

**Real boundary evidence.** Actual production view/Button → original typed fake request/result; normal once success, multiple same-type views, close/reopen, Settings/NewTask separation, A→B→A and queued Change races, duplicate/stale/error/cancel/disconnect/retry/queue-full and exact temporary config. Parent **picker14 + keyboard8 =22/22**, Downloads182, strict and source gates passed.

**Disposition.** **S/C/Y/P bounded closed.** Historical full **1173 pass / 1 fail** Archive Unlock remains failed; real picker/modal/HWND lifetime unaccepted. [PRD](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-download-picker-identity/prd.md), [parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-download-picker-identity/parent-review.md). This early task stores its readable records directly, not under an invented evidence subdirectory.

### #10 — Mutable Clipboard list positions were used as retained row/action/focus identity

**Actual changes.** `ui/history.rs` and `ui/mod.rs` use immutable record ID + action kind under original panel/window ancestry; original paging/position indices remain positional. New `ui/history_identity_tests.rs` and actual history feedback/menu/kit/polish fixtures were adapted; lifecycle gate/sensitivity assertions and reference append follow actual identity. Same-scope real FocusHandle is obtained in the mounted row; orphan fallback qualifies only the current mounted ready search. Full-by-ID payload and one original release-click owner remain.

**Real boundary evidence.** Actual ClipboardPanel/ListState/worker page callbacks, real Tab/focus handles and down/up under prepend/front-trim/reorder, removed row/action, armed release, two panels and new sessions. Effective deletion fallback/index-key mutation reds; parent **identity11/11**, **UI228/228**, strict/source gates and5gate-sensitivity fixtures passed.

**Disposition.** **S/C/Y/P bounded closed.** Historical full **1184 pass / 1 fail** remains failed. No real paste/Clipboard/provider acceptance. [PRD](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-clipboard-stable-focus/prd.md), [parent review](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-clipboard-stable-focus/parent-review.md), [nine existing-file deltas](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-clipboard-stable-focus/evidence/own-delta.json).

### #11 — DownloadPage projection and row composition copied full task/progress data unnecessarily

**Actual changes.** `downloads.rs` owns `Rc<Vec<DownloadEngineTask>>`; snapshot access shares the original allocation. `main_window/download_page.rs` projects source indices, captures snapshot+IDs and guards projected identity before range composition; each real requested row borrows task/progress and only that range reads progress/icons by ID. `main_window.rs`, floating-assistant iteration/test mutation, old average/kit/main fixtures and new `download_page/projection_tests.rs` are necessary adaptations. No global cache, lock, second list/controller, geometry or execution change.

**Real boundary evidence.** Original Root → `render_downloads` → real `uniform_list` requested ranges → row accessors. Five regressions cover1000/10000, bottom scroll, six sorts/filter/ties, replacement after projection and actual pointer/Ctrl/reorder/removal. In observed draw batches, task projection copies **1000/10000→0**, row payload copies **26→0**, progress/icon reads each **2000/20000→26**; row construction was **already26**, not an eager-all-elements claim.26includes legitimate measurement callbacks and is not a universal viewport constant. Remaining O(N) traversal/O(MlogM) sort and necessary payload allocations are disclosed.

**Checks/disposition.** **S/C/Y bounded loop achieved; not global/P/native closure.** New5/5, page26, main27, Downloads210, protection69 and final new/name/identity23 passed; test strict/check/scopedfmt passed. Production strict/workspacefmt baseline failures retained. A stalled new cross-window fixture and a false shell exit0 were preserved; actual stdout failure governed, then the real-boundary seam was corrected. Cargo.lock's flate2 reconciliation is unowned/toolchain observation, not an optimization dependency claim. [Outcome](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-download-virtual-projection/outcomes.md), [nine-entry delta](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-download-virtual-projection/evidence/own-deltas.json).

### #12 — Selected-detail SQLite work was deferred but still ran on the GPUI thread

**Actual changes.** New `downloads/detail_request.rs` remains on original DownloadApplication; existing `nxtools-download` runtime `spawn_blocking` invokes original typed TaskDetail/store chain through `engine/projection.rs`. Original MainWindow/page/shell owns a shared live presentation scope and retires it locally before deferred cross-owner cleanup. New owner/page tests and test-only actual-store lock seam are declared through the two manifests. No schema/query/connection ownership rewrite, second runtime or cache. One physically running worker + one latest pending request; generation/correlation/task/live-scope/session/existence qualify publication. Last-good cache remains but dirty refresh cannot be hidden. Own read-only CommandRejected receipts cannot trigger a requery loop.

**Real boundary evidence.** Original actual SQLite mutex held while request is scheduled: red unrelated UI turn **did not run**, watchdog unlocked at3013ms; green UI turn ran before watchdog, observed0ms rounded (not SLA), then actual result awaited. Second red proves cache refresh rebinding. Nine tests cover ABA/duplicate/cache counts, actual SQLite query failure/restore/retry, stale failure, physical occupancy after retirement, real row/Tab responsiveness, retained entity after actual close and real module switch. No injected fake success or direct completion shortcut.

**Checks/disposition.** **S/C/Y bounded loop achieved; not global/P/native closure.** New9/9, page29, main27, no-network Downloads48, names/identity18, original storage contract1 passed. Desktop-test strict and Download test/production strict/check/scopedfmt passed. Desktop-production strict and four-file workspacefmt failures retained. PDB/disk failures and the temporary cfg(test)-on-shared-helper E0425 mistake remain logged; helper restored exactly. Other synchronous compact snapshots/artifact/task/shutdown detail consumers are explicitly outside this repair. [Outcome](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-download-detail-async/outcomes.md), [twelve-entry delta](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-download-detail-async/evidence/own-deltas.json).

### #13 — Retained Quick Launch presentation text was copied on each render/callback construction

**Actual changes.** New `quick_launch/retained_text.rs` pairs complete original `Vec<SearchResult>` and full SharedString display text in one immutable original-panel snapshot. `panel.rs::render_result` clones handles; hover captures same snapshot/index and clones full record only when the callback actually executes. `actions.rs` view labels/section/level/target titles and panel status are shared; existing InputState shared read value is used. Editable filter/parent query/format buffers, Finder/domain/protocol/session/path/command and complete execution payload stay original types. `quick_launch.rs` only registers modules; old tests/extension-provider fixture assignments adapt container type. No second search controller, global/per-row lookup cache, truncation, rem or cursor repair.

**Real boundary evidence.** Production result replacement/typed action update and repeated actual Root/Window text-element composition; observations retain all handles to exclude allocator-address reuse and require expected slots/full content. Effective red action-target **6handles/6allocations→6/1**, result-subtitle0 **18/18→18/1**; repeated backing-text bytes2740/7922→0. Other green slots/reorder/same-ID update/fallback/status/submenu and original full payload are checked. These are deterministic backing-identity/copy proxies, not total allocator/RSS/GPU/frame-time measurements. Commit construction/reference counts/highlight/layout costs remain.

**Checks/disposition.** **S/C/Y bounded loop achieved; not global/P/native closure.** New5/5 + original121 + polish11 = **137selected tests**, zero failed/ignored. Test strict/check/scopedfmt passed; production strict/four-file workspacefmt and whole QL gate did not all pass. Original API-preparation/disk/format attempts remain separate. #13 does not implement unfinished Queue09 ordinary roles or #7 cursor. [Outcome](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-quick-retained-text/outcomes.md), [eight-entry delta](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-quick-retained-text/evidence/own-deltas.json).

## 4. Residual ordinary-pixel roles and accurately approved exceptions

The authoritative residual source is the frozen [role matrix](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-rem-archive-final-matrix/role-matrix.md), not a new blanket exemption invented here. Its coordinates/counts describe that task's snapshot.247files/2882direct `px` calls include2028test calls; this is lexical navigation, **not2882defects or exhaustive semantic clearance**. Indirect units/macros/aliases remain unfinished unless mapped. Superseded v1 cfg(test)-field parsing incorrectly swallowed later production ranges; only corrected v2+manual Archive cross-check is usable.

Archive's158direct calls comprise104test and54production:16protected native/client,3explicitly approved tree-width calls,23ordinary candidates,4unresolved persisted geometry,2unresolved viewports and6other unfinished decisions. **35unfinished does not mean35proved bugs.** No Queue10 product repair occurred.

| Role class | Accurate retained authority/budget | What is not exempt |
| --- | --- | --- |
| Archive native clients/placement | Browser960×640/min720×480; job560×300/min480×260; forms560/680×600/min480×360; canonical [Archive UI specification](D:/Code/NxTools.Next-main/.trellis/tasks/10-16-archive-manager/ui-spec.md) and original native owners | Ordinary rows/type/toolbars/Tree indent, comment/error viewport and form Textarea heights |
| Archive approved layout | Tree200DIP, range160..280, exact canonical approval | Details minimum160, Settings sidebar160/range160..208 and persisted column policy are not automatically the same exception |
| Download original fixed shells | NewDownload560×176/246,caption34/footer44/media base278; category320×240,schedule304×200/calendar≤124/media≤120; Settings984×650/title36/footer48/header120/category310/error96; Task440×184/196/280,title34/footer44 | Ordinary internal labels/rails/spacing still need rem; application row48/header28 already use shared rem measure axis, bitmap20/selection edge2stay fixed |
| Main/Tools fixed budgets | Main1060×650/min920×480,caption36/rail176/controls36×36/brand bitmap18; Tool settings460×320/header48; catalog tile180×160/vector32/status6 | Tool Form/control/text allocations, Global/Extension ordinary roles are not whole-surface fixed |
| Quick Launch native/hit families | Main620×44/410,header42; attached500/36/264; passivepreview400×410,header72/footer30; interactivecontent40,slots40/44,gap0/4outsidehit | Result/action/recent/status/preview ordinary content in Queue09 remains unfinished; #13 text sharing is not rem clearance |
| Clipboard approved content finish | Persisted text cap120..320/default160 (supersedes210); images190/media144,files62,gap8,source24,overflow-only feather14; black8%shadow offset1/sigma1; source-icon-derived gradient/full payload preserved | Ordinary menu/lanes/status/header/category/history/preview type and measurement are not all fixed; historical16feather deferral/210cap is not current authority |
| Official mixed units | Released Form/Field internal4/8px, Switch Medium36×20, Progress XSmall4 and Dialog viewport/footer grammar retained by official-component/no-vendor-patch rule | Caller widths/Textarea heights/application spacing are not official internals; cannot repaint or patch vendor just to obtain rem |
| Native/bitmap/hairline/hit coordinates | Only named owner/source-backed physical/client/bitmap/hit/drag boundaries | A small literal or old passing geometry test alone is not approval |

Remaining concrete roles include Extensions vector28 and confirmation summary/technical widths80/88; Quick/Clipboard ordinary candidates; Archive row32/glyph16/toolbar64,min56/glyph24/separator48/Tree32,indent16; row/PageUpDown `(viewport.height−192)/ROW_HEIGHT`; persisted columns, comments/errors and Textarea48/160. Unknowns remain **UNFINISHED_REVIEW**, not fabricated fixed exceptions. Font14/16/18 tests are not Windows100/125/150% DPI tests. See [current Clipboard amendments](D:/Code/NxTools.Next-main/.trellis/spec/shared/ui-design.md:293) and [cursor manipulation exceptions](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-cursor-semantics/inventory.md:42).

## 5. Compile, unstable-test and source-gate follow-ups

| Follow-up | Actual bounded correction/evidence | Attribution/limit |
| --- | --- | --- |
| Queue01 Extensions compilation | Existing UnsupportedTransport→Unavailable mapping/public method_minimum_transport were independently repaired before own edit. Own56line committed-event regression:1new,4committed,API3+transport3; Desktop/Extensions strict and workspace check passed | External production repair not credited to this session; old E0004/uuid replay/missing SharedString compilation logs retained. [Record](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-extensions-compile-closeout/outcomes.md) |
| Queue02 Archive Settings search | Seed3reproduced wrong first TextInput: NumberInput value10 selected instead of sidebar search. Actual semantic path/exactly-one locator + focused full query assertion; seeds0–7 and Settings19passed | **Test-only locator repair**, not product search/save redesign; old185/1stays FAIL. [Record](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-archive-settings-search-diagnosis/outcomes.md) |
| Queue03 Extension Cancel | Original seeds0–31did not reproduce. Controlled down→50ms test-clock/draw→up showed Cancel moving21px; reduced-motion business fixture plus separate250ms-settled normal-motion/reopen test; manager6/import5/picker1passed | **Test-fixture hazard**, not proof the historical full failure had that exact timing or product cancellation was fixed. [Record](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-extension-preview-cancel-diagnosis/outcomes.md) |
| Queue16 QL elevated-navigation source gate | Actual74predicate matrix:73hold, only obsolete direct build→writer filename assumption fails. True chain is build→fixed publisher→writer→generated manifests consumed by Host. AST structure/arguments/order/failures checked; **100SourceOnly assertions**:1real chain+26broken chains+73original security mutations | Only gate/tests changed, no product security implementation. Fixed helper/auth/hash/atomic tuple/ACL/timeout/fail-close predicates retained. Not a demonstrated compromise. Whole gate still stops earlier at QL-11line458. [Record](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-quick-source-release-chain/outcomes.md) |
| Final convergence | Read latest records; Queue08partial,09/10unimplemented,#7unrepaired prevented admission.632sources matched Queue16;5451historical files/15raw result records protected | **Zero Rust / zero complete-suite attempts.** Documentation checks only; no final convergence green. [Decision](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-final-convergence-admission/outcomes.md) |

The Queue16 minimal existing-file edit adds publisher-source loading/import and replaces one obsolete condition. [Checker](D:/Code/NxTools.Next-main/tests/architecture/quick-launch-release-chain.ps1), [SourceOnly regression](D:/Code/NxTools.Next-main/tests/architecture/quick-launch-release-chain-tests.ps1), [exact delta](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-quick-source-release-chain/evidence/own-deltas.json). It rejects comments/dead branches/dormant functions/wrong or duplicate calls, altered argument/output paths/order, swallowed publish failures, SHA1/missing-input/literal-hash mutations. This is a strict current source-structure contract, not arbitrary control-flow or deployed security proof. PSScriptAnalyzer was unavailable; parser checks were not called a lint pass.

## 6. Check chronology, historical failures and present blockers

### Latest relevant recorded quality state — not rerun by this report

| Check | Last recorded actual result | Interpretation |
| --- | --- | --- |
| Desktop `clippy --tests -- -D warnings` | #13 **PASS**,44.16s | Test-target strict, not production-target strict; no current compile/type failure inferred |
| `cargo check --workspace` | #13 **PASS**,13.94s with two warnings; #12 workspace-tests check also passed | Prior compiled evidence, not this report's new run |
| Desktop production strict | **FAIL** SPACING_12 at main_window.rs75 and unread DecodedMedia.animated at media.rs62 | Baseline-identical owners, no suppression; not a test-target failure |
| Workspace fmt | **FAIL** four files:main_window/page_rem_tests.rs,network_settings.rs,network_settings/blacklist.rs,network_settings/page_rem_tests.rs | Last #13log; no broad unrelated format was applied |
| Whole Quick Launch source gate | **FAIL** QL-11line458,missing CONTROL_HEIGHT_INPUT source literal | Earlier failure masks later whole-script execution; targeted release-chain100PASS is separate |
| #8 broad MainWindow | **206 pass / 6 fail / 0 ignored**,106.50s | Independent Extensions storage/removal resource-limit fixture failures remain recorded; not freshly reproduced or declared fixed |
| Final-stage full Desktop | **NOT RUN**,attempts0,passed/failed/ignored/exit null | Admission denied, not canceled mid-run or hidden failure; allowance unconsumed |

### Frozen full-run/failure ledger

Early full-suite results belong to the historical implementation deliveries, not this current report or a current parent full rerun: #1 **1158/2FAIL**,#9 **1173/1FAIL**,#10 **1184/1FAIL**,#3 **1191/1FAIL**;#2 **1196/0PASS**;#4 **1207/1FAIL**;#5 **1232/0PASS**;initial#6 **1237/1FAIL**. Counts changed with shared-tree additions/repairs; they cannot be combined or credited entirely to one finding. Earlier password/DefaultApps/keyboard failures retain their own source/time/causal limits even where later scoped success exists.

The last historical original#6 full log is UTF-16 and actually reports **1237passed/1failed/0ignored/0filtered,281.18s**,command **exit101**,493.42s total. Failure:`main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config`,extensions_page.rs445. The earlier MainWindow **185/1FAIL** and #8 **206/6FAIL** remain failures. No isolated pass or #13group converts them into past green. [Raw full log](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-rem-role-consistency/evidence/desktop-full-once.log), [exit record](D:/Code/NxTools.Next-main/.trellis/tasks/10-25-rem-role-consistency/evidence/desktop-full-once.json), [precise15log hashes](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-final-convergence-admission/evidence/failure-log-hashes.json).

Canceled initial #6 deliveries and NewDownload compile-only checkpoints remain canceled/preparation, not effective layout reds. Queue08–11 natural-idle blocked observations did not diagnose external deadlock/memory and did not authorize killing/pausing others. Later idle state does not implement missing work. #11's interrupted fixture has no terminal pass; its diagnostic wrapper's false exit0 is overridden by actual0/1failure. #12PDB/disk/scheduler/API errors and #13disk/API/format errors remain failed attempts; only explicitly identified own failed outputs were cleaned with manifests. One read-only `cargo rustc --help` lacked explicit environment variables but built/ran nothing; that policy omission remains disclosed. No whole-suite green-hunting loop was performed in the final stage.

The early #1implementer improperly requested other conversations to reserve/defer Cargo time; the user rejected it and the request was explicitly withdrawn. Its outcome records that no process/session was actually paused/cancelled/terminated. This historical process-policy deviation is retained, not hidden by later wait-only compliance. Subsequent direct-queue work used natural-idle waiting without resource reservations.

### Blocking completion now

1. Substantive Queue08–10ordinary-rem completion/coverage and compliant #7paint-hit repair/exception verification are absent.
2. Whole QL source gate QL-11 and four-file workspacefmt remain unpassed; production strict has two baseline warnings-as-errors. Latest test strict/check success must not be misrepresented as all quality gates green.
3. #8broad integration failure lacks a later converged full result. Whether those six fixtures still reproduce is not determined by this report.
4. Final synthetic convergence was not admitted. Any later implementation or integrated test run requires a new scope/admission decision; this report starts no queue or follow-up task.
5. Physical/native/DPI/provider/visual/user/installed acceptance below remains separately pending even if all synthetic tests later pass.

## 7. Dirty-state protection, ownership and provenance

All remediation deltas use **live dirty bytes, not HEAD**, with original source snapshots, exact forward/inverse operations or preserved prefixes. A later authorized task baselines previous accepted changes rather than rewriting their manifests. Old verifiers can truthfully report live-layout movement after a later authorized edit; that is not permission to alter the frozen old manifest. Source preservation is not compiled behavior acceptance.

Attribution is not flattened:

- Earlier seven individual deliveries and first bounded#6work were implemented in the historical implementation session, then this parent session reviewed/reran their named subsets. Their own worker full runs are not called parent reruns.
- Queue01onward was executed directly here; new final report/summary/evidence are this turn's **only own modifications**. No new code repair or delegation occurs in this handoff.
- Queue01production compile mapping/re-export/import repairs were external. Queue03's external enum lint repair, subsequent independent Extensions storage/settings work and #8ten external files remain separately observed, not reset or credited as own fixes.
- #4observed external formatter segments and the archive-reference original770+own30+external34line provenance remain separated. Original unresolved76line source stays frozen even though #5restored/repaired its current activation boundary.
- #11Cargo.lock flate2 reconciliation matched a previously live manifest; initiating process is unproven. #12did not change that lock. No dependency credit or reset is invented.
- Historical failed/canceled/blocked tasks are not overwritten. The handoff captures **8,045Git-visible regular files**,including original dirty/untracked files,about153MB, before writing new reports. Final read-only hash comparison checks these same bytes. Git-ignored build outputs/external repositories are outside that coverage; no claim is made about unobserved files.

Exact task deltas, current/source/log paths and SHA256 values are indexed in [source-index.json](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/source-index.json). Final report-only commands and outcomes are recorded in [checks-final.json](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/checks-final.json); the [initial check attempt](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/checks.json) remains immutable. No old manifest is updated, no reset/clean/commit/push/package/install occurs.

## 8. Pending acceptance — concrete steps and prior authorization

**Every step below is a proposed manual/owned-fixture acceptance procedure, NOT performed by this report.** Obtain separate explicit authorization for the exact binary/process/path/monitor/device/data/OS effect. Use an isolated Windows account or disposable VM, owned synthetic files/catalog/config/database and a recorded rollback/cleanup plan; no real user Clipboard, secrets, archives or search roots by default. Freeze binary/source/manifest identity before recording a result. Missing prerequisites are BLOCKED/SKIP, never PASS.

### A. Physical mouse and keyboard

**Authorization needed:** launch the exact approved Desktop fixture/product binary, target only its owned windows, use physical keyboard/mouse and only synthetic action targets. Real download/archive/Clipboard/Shell effects require separate per-operation approval.

1. Record binary SHA256, configuration, theme, OS build, input device/layout and all window/client sizes. Start with owned synthetic rows and disconnected/fake effect endpoints where supported.
2. For #1/#5,physically Tab/ShiftTab to each permitted input and Button; verify plain Enter and Space activate only on eligible release once. Exercise held repeat,duplicate release,Ctrl/Alt/Shift,focus transfer between down/up,Escape,disabled/busy and child Select/Popover/Dialog. Confirm root/unfocused destructive Enter never approves. Schedule TimeField edits must remain local and one real Save persists only the schedule.
3. For #2,physically sort each header via pointer/Tab+Enter/Space; compare direction/default/order,selected IDs/anchor and temporary-config reopening. Preserve fixed header/row bounds.
4. For #9,open one owned directory picker,edit A→B→A,close/reopen/replace source window,then complete/cancel the old dialog. Confirm no new draft/window overwrite,duplicate dispatch or implicit save. Actual native picker interaction is separately authorized.
5. For #10,focus row/action A,prepend/reorder/trim/remove A while a gesture is armed; release cannot execute the old index's new occupant. Confirm correct valid search focus fallback,one original-ID action and no subaction bubbling. Any real paste uses only a separately approved disposable target.
6. For #8/#13,use complete long CJK names/paths/actions,keyboard and pointer selection/submenus/Escape; check original full payload against synthetic sentinel. Observe cursor semantics as **known pending#7**,not accepted until its implemented fix passes.

Capture actual action/IPC counts/current IDs and physical observations/video without sensitive payload logging. Any unexpected action/focus theft fails the case; do not compensate by repeated clicks or altered assertions.

### B. Windows IME and composition

**Authorization needed:** physical Windows IME use in exact owned windows; choose Microsoft Pinyin and an agreed second layout if required. Synthetic GPUI marked-text events are not this acceptance.

1. Enter long Chinese query/name/path text,compose/convert/cancel candidates and press Enter while preedit/candidate UI is active. The confirmation Enter must not probe/start/approve/paste/open a result.
2. After commit/cancel,press a fresh ordinary Enter; it must execute exactly once rather than swallowing the next gesture. Test Ctrl/Shift combinations and Space candidate selection without action leakage.
3. Move physical focus among search,TimeField,password/multiline/form,Select,action mode and another owned window during composition; close/reopen a view. Preserve complete text/undo/InputState ownership, no stray newline,stale action or forced focus reset.
4. Run popup/result/preview cases with both primary and auxiliary HWNDs visible. IME/candidate windows must not be mistaken for an external-dismiss gesture. Record IME mode/steps/outcomes,not secret text.

### C. Native focus, HWND identity and show/hide/reuse lifecycle

**Authorization needed:** exact owned HWND fixture launch/C#native filters or product lifecycle; explicitly permit activation,style/bounds/query/message operations and cleanup of only created exact-path processes. Do not apply blanket native run-all authority.

1. Build/identify exact frozen Desktop and harness under separately approved build policy. Read [native acceptance wrapper](D:/Code/NxTools.Next-main/tests/native/gpui-kit-acceptance.ps1) and [harness effect-selection rules](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/README.md); list exact intended filters before execution. The wrapper needs explicit `-AuthorizeNative`; it does not build/package. Do not invoke it now.
2. For Quick and Clipboard,record primary/passive PID/HWND/executable/session. Create preview hidden,verify C#NOACTIVATE,final device-pixel bounds/readback,draw/frame/reveal ordering; reused preview retains one HWND and repositions/updates in place rather than opening duplicates.
3. Physically click/hover preview while typing in primary search; native primary foreground and GPUI input focus must remain/restored correctly. Primary panel remains activating; passive preview/notifications must not gain keyboard activation. Message return3versus1is only a subcase,not proof of a physical click outcome.
4. Show→hide→show,Escape,outside focus/pointer,query/selection changes,action mode,scroll,close/reopen/replace HWND; inject only reviewed fake IPC late/duplicate/disconnect/failure/timeout replies. Old generations cannot reveal/hide/refocus replacement windows; failures stay truthful and retryable.
5. For #12retain UI entity across actual window close,module switch and repeated selection while an owned storage fixture is busy; publication remains qualified and the UI remains usable. Do not claim native cancellation physically interrupts SQLite or uncooperative Windows APIs.

### D. 100%,125%,150% and mixed-monitor DPI

**Authorization needed:** configure or use agreed test monitors/scales,launch/move only owned windows and capture their synthetic content; OS display changes require explicit permission and restoration plan.

1. At each scale100/125/150%(nominal96/120/144DPI),record actual monitor work area/current HWND DPI,binary hash and theme. Test initial open and default/minimum clients; do not infer scale from Root font or screenshot dimensions.
2. Verify logical→device-client mapping/readback once,original title/footer/bitmap/hit budgets,all actual text/control containment and long CJK/path/error last-line/footer/action reachability. Check row/header virtual measure/paint/scroll stride and Page/Home/End selection; ordinary font14/16/18is an additional axis,not a substitute.
3. Move primary and active/reused preview across100↔125,125↔150and100↔150monitors,including negative origins/taskbars/work-area edges. Repeat while query/selection/probe is pending and after close/reopen. No cumulative scaling,hidden CW_USEDEFAULT flash,offscreen preview,unwanted resize or focus theft.
4. Inspect source icon/bitmap crispness,hairlines,gradient/feather/shadow and actual hit targets,not only bounding-box math. Record actual screenshots plus pointer/keyboard lifecycle outcomes per cell; unsupported hardware is BLOCKED,not accepted.

### E. UIA/provider and real screen reader

**Authorization needed:** run agreed Inspect/Accessibility Insights/Narrator or NVDA against only the exact owned process/window,with synthetic content and privacy consent for captured trees/audio. No cross-process provider injection.

1. Inspect actual provider tree at normal/disabled/loading/selected/error states. Actual #3Button nodes expose action+object names,roles,enabled/focusable/value/state; Tooltips/parent labels alone cannot satisfy control naming. Verify label/value/error associations where supported.
2. Physically Tab/ShiftTab through real UI and compare provider focus/events to actual focus,including retained #10row after prepend/reorder and correct fallback after removal. Test release activation once; do not use UIA Invoke as sole evidence of physical keyboard equivalence.
3. Read full #4incoming/target identifiers and long #8/#13labels without truncating the underlying accessible value. Check current ticket,missing-context fallback,apply-all and footer actions. Do not expose password/Clipboard secrets through provider/log capture.
4. With Narrator/NVDA,verify name/role/state/value announcements,logical navigation,modal/Escape return and no unsolicited focus/announcement flood when a passive preview appears. Record user/operator observations and actual provider tree. Headless semantic snapshots/owned fixture UIA checks are prerequisites,not reader approval.

### F. Visual and user acceptance

**Authorization needed:** agreed visual matrix and a named human/user reviewer; screenshot capture of only synthetic owned content. A test screenshot or AI inspection is not sign-off.

1. Compare current official0.8.0grammar and approved target/exception matrix at default/minimum,Light/Dark/System and agreed DPI/font axes. Cover normal,hover,pressed,keyboard focus,selected,disabled,busy,empty,error,success,long-content states.
2. Review hierarchy/alignment/typography/contrast/discoverability,recognizable outlined command boundaries,complete object/action text,viewport/end/footer reachability and fixed client/card/tile budgets. Keep source-icon hue/Clipboard finish and existing user-approved geometry; do not recolor/pad away failures without separate implementation authorization.
3. User performs representative creation/schedule/sort/picker/cancel/list/action/submenu/preview flows with agreed synthetic endpoints. Record per-surface ACCEPT/REJECT/BLOCKED,reviewer,date,binary hash and observations. Do not turn one accepted screenshot into all-state or native acceptance.

### G. Release,installed lifecycle and LocalSystem service

**Authorization needed:** separate explicit Release build/package/signing/install/UAC/Registry/SCM/process-restart/rollback and owned test-volume/search approval,preferably a disposable VM. This report grants none. Resolve implementation and quality/admission blockers first.

1. Freeze source/dependency/vendor provenance and run admitted scoped quality checks,then exactly the allowed integrated synthetic attempt with real counts/exits. Every Rust/release shell must set jobs1/incremental0/codegen256,naturally wait for idle,run serially with full timeout and inspect survivors after failure. No competing-process kill/reservation or repeat whole-suite green hunting.
2. Only after packaging approval,use committed-memory headroom preflight and publisher-owned signing key/monotonic sequence. Verify actual build→publisher→writer→Host manifest consumption and signed full-package artifacts; source AST tests alone do not prove generated/deployed hashes. Never copy private keys into reports or hand-edit generated approval.
3. Install/upgrade only the exact authorized directory. Stop only the running Desktop with the exact package executable path; service stop/restart only for the registered exact matching package. Verify protected Desktop/PlatformHost/Indexer **atomic tuple**,signed all-file/archive helper identities,approved/runtime hash equality,ACL/reparse/stable-handle/fixed-helper/auth/session/timeout/fail-close policy. Tamper/downgrade/partial-package/wrong-peer tests belong to separately approved isolated security fixtures,not the user's live installation. No runas fallback on authentication failure.
4. After approved installation,exercise ordinary-user first launch/reconnect/hide/close/reopen/upgrade rollback with actual package identity and truthful failure results. `Ready`,successful compile or source gate is not release acceptance.
5. For indexer acceptance,with explicit service-restart/owned-volume authorization,use the existing [idle CPU gate](D:/Code/NxTools.Next-main/tests/performance/indexer-idle-cpu.ps1) with `-RestartService -AssertRegression`,then perform a separately approved real-service search against a disposable indexed fixture through the authenticated service boundary. No independently parameterized service-search runner was found in the inspected tests; an owned-volume/query/expected-result plan must be established before execution rather than pretending an existing script supports it. Verify versioned mmap base+persisted delta restoration,no reconnect-triggered full rebuild,matching approved/package/protected/runtime identities,bounded idle CPU/IO and exact results. Listary native-engine source is not claimed available.
   The existing [Quick Launch UI smoke](D:/Code/NxTools.Next-main/tests/runtime/quick-launch-search-smoke.ps1) is **not a substitute or a safe read-only service query**: it fixes query `q`,injects global keyboard/pointer/window messages,has no roots/query parameter,and by default stops exact-path Desktop/PlatformHost processes. Its launch/reuse/UseRealPlatformHost/cleanup effects require their own exact native/product authorization and controlled installation. Do not invoke it on the user's running product merely because this report links it.
6. Record RUN/PASS/SKIP/FAIL/not_run,actual exit,counters,time,artifact/log hashes and cleanup/rollback evidence; missing environment or skipped installed/native case cannot be green. Any additional integrated rerun or expanded effect needs a new explicit decision.

## 9. Report-only verification and handoff stop

This delivery's checks are limited to file/link existence,`python tests/architecture/spec-contracts.py`,document-scoped diff/whitespace and read-only baseline/history/hash verification. Actual final command exits and exact log SHA256 are in [checks-final.json](D:/Code/NxTools.Next-main/.trellis/tasks/10-26-gpui-guides-final-handoff/evidence/checks-final.json); they certify the report,not product behavior. English report and requested Chinese user summary are new files;no old outcome/manifest is revised.
The first report-check preparation used Windows-default GBK for one UTF-8 read and stopped before spec/diff commands; it was corrected to explicit UTF-8. The initial aggregate then incorrectly expected0from clean new-file `git diff --no-index --check`. Controlled scratch fixtures proved this Git returns1/no diagnostics for a clean intentional difference and3/explicit diagnostics for trailing whitespace; both reports had empty diagnostics. That initial aggregate FAIL and exact commands remain frozen, with final checks published separately. Neither event is a product failure,an erased check,nor a reason to run product/native tests.

No new remediation,queue/subagent/worker,build/full Desktop/native/UI/product/service/Registry/user-data/network/Clipboard/Shell/package/install/commit/push is started. **The user now owns the decision on remaining remediation and separately authorized acceptance.** This session waits for that decision; no automatic follow-up or background-resumption promise is made.
