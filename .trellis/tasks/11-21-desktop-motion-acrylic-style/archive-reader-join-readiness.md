# Archive, Markdown Reader and raw-join material readiness

Canonical phase: **阶段三：原生材质与展示生命周期**. Dependency authority remains Kit 0.10.0 / pre 0.3.12.

## Reference and native lifecycle

- `docs/reference-code/archive.md:18,101–109` records no legacy NxTools general archive browser/job window. The retained Bandizip operation trace connects constructor/coordinator to `DialogBoxParamW` / `CreateDialogParamW`, then original HWND terminal controls and `EndDialog` / `DestroyWindow`; this is static artifact evidence, not proprietary source or a live native run.
- `docs/reference-code/markdown-reader.md:16,48,89–93` records no independent legacy reader. Next retains one ordinary Reader window, tabs, document revisions, close/draft confirmation and the separate Explorer foreground receipt owner.
- `docs/reference-code/archive-volumes.md:32,36,44–48` records no old raw-join window; Next's existing PopupMenu → deferred raw_join::open → one Root/JoinWindow → native should-close interception → original cancel/actual-terminal owner remains authoritative.
- Rechecked legacy `D:\Code\NxTools\Services\AppAppearanceService.cs:76–118`: theme/palette/material/titlebar are separate operations; material changes the background and transparency hints without owning show/hide. No Avalonia native backend is inferred from those hints.
- Rechecked current native chain: `vendor/gpui-pre-0312/src/window.rs:6917–6918` → `vendor/gpui-pre-windows-0312/src/window.rs:819–835`. Creation at 580–592 retains the requested placement while hidden; `activate` first applies `initial_placement` through `set_window_placement:369–391`, preserving constructor bounds and maximize state. This is the ordinary reveal boundary, not the passive drag-progress native visibility boundary.
- Native verified foreground requests remain `ArchiveApplication::activate_browser`, `jobs/job_activation.rs::activate_job` and `markdown_reader/foreground.rs::request_foreground`. C# `WindowsPlatformAdapter.NativeMethods.cs:280–325` restores an iconic window and performs foreground/focus readback; it does not replace GPUI's hidden creation placement.

## Preserved presentation contracts

Requested layout shape, visible variants/sizes and parent adaptations are unchanged. Archive retains its BrowserHost/TabBar, original OperationForm and JobProgressView; Reader retains its Folder/Small tabs, outlined document actions and original source/preview controls; raw join retains the original Form/inputs/Medium confirmation Dialog. No ordinary control, toolbar adaptation, shared material role or framework source is replaced.

Production constructors are hidden and non-focusing. Each owner defers `runtime::when_ready` outside the target window/entity lease, supplies an eight-second ceiling and uses Backdrop. Qualified success consumes GPUI's original creation placement before the existing activation owner; passive drag progress instead joins material after its original NOACTIVATE/bounds receipts and before its original visibility command. The runtime owns apply/receipt retention only.

- Browser pending reuse records the latest requested tab without native activation. Completion qualifies host UUID and exact retained window; failure uses close_host and its original read/preview/source retirement.
- Job pending reuse refreshes the same selected batch surface without activation. Success reads the current selected member and lets original activation capture its current attempt/generation. Failure retires display/window only, never cancels or changes the job result. Original background/close clears pending ownership; prompt restoration remains in the original background path.
- Forms qualify original form UUID, slot generation and exact window. Failure retires the existing form and its application-owned requests; success preserves the same draft/entity/placement.
- Reader preserves the latest ordinary/Explorer presentation request while material is pending. The exact global window/root/Reader must still exist; failure enters the original close guard, success retains document/tabs and the original Explorer foreground result owner.
- Raw join qualifies the original global weak target, exact window and session. Pending reuse cannot activate. Failure calls its original close/cancel owner; no logical cancellation shortcut is added.
- Drag progress qualifies operation/session/generation/window and hidden/pending state. Background or disconnect during the material wait cannot release a late reveal. Its passive native placement and visibility owners remain unchanged.
- PlatformNone Reader/raw-join debug/test creation retains the original visible path; no product switch or native-success bypass is introduced.

## Regression handoff

New tests exercise real constructors/Root owners, original router submission/result routing and shared readiness completion. The parent's test-only `runtime::set_test_window_handle` maps individual TestWindows; production still reads the native handle. Synthetic receipts are not native Acrylic, placement, foreground or visual acceptance.

Filters for the parent's single Cargo lane:

- `archive::application::ordinary_material_tests::material_` — missing HWND, pending reuse, stale host completion, current facts/latest tab/runtime handoff, original form entity.
- `archive::application::jobs::ordinary_material::tests::material_` — pending reuse, original job/window after receipt, failure without cancellation, background-before-readiness.
- `archive::application::drag_progress::material_tests::material_` — bounds ACK insufficient, material receipt continues original visibility command, hidden cancellation blocks late reveal.
- `tools::markdown_reader::material_tests::material_` — missing HWND, isolated PlatformNone, retained Reader/reuse/runtime update.
- `tools::raw_join::material_tests::material_` — missing HWND, isolated PlatformNone, retained weak target/reuse/runtime update.

Verification: scoped rustfmt with `skip_children=true` parsed/formatted only these owners and new modules/tests; scoped tracked-file `git diff --check` passed. Cargo tests/check/Clippy, packaging, native fixtures and product launch were not run.

## Integration limits

The older successful-presentation fixture `application_tests.rs::setup` installs `ordinary_presentation_fixture.rs`: each BrowserHost/OperationForm/JobProgressView Root gets an explicit synthetic HWND and a coherent `Preparation::confirmed` registration. The original deferred readiness callback still validates and releases presentation. Existing assertions, including keyed browser retirement/reopen, remain unchanged. `setup_unprepared` retains the original bridge/client construction for the ten Archive material tests; the six Reader/raw-join material tests keep their independent routed fixtures.

Type checking of the new test modules and production callbacks remains for the parent lane. The owner tests depend on the live cfg(test)-only `runtime::set_test_window_handle` seam and the read-only `runtime::is_waiting` query; neither bypasses production receipt validation. Actual hidden-placement/maximize restoration, foreground ordering, DPI and compositor output remain native acceptance work.

## Existing presentation fixture coverage

- Shared Archive `setup` covers its direct tests and the existing preview, job, form and batch fixtures using that entry.
- `failed_read_admission_stops_loading_on_the_new_generation` explicitly installs the confirmed fixture while retaining its disconnected bridge and absent read client.
- `entry_tests.rs::ready_browser` and `job_tests.rs::fb_ac02_actual_queue_full_recovers_pause_and_keeps_cancel_reserved` explicitly install the same fixture without changing their original client or queue capacity.
- `tools/raw_join/tests.rs::ref18_archive_menu_opens_reuses_and_retires_real_window` explicitly installs a JoinWindow-only Root observer from its owner material-test module, preserving menu/cache-hit/close/reopen assertions and the original bridge.
- Reader `tests.rs::open_fixture` constructs Reader directly with PlatformNone; its external-open regression also uses PlatformNone. Raw-join `tests.rs::mount` constructs JoinWindow directly instead of the gated open constructor. These isolated view fixtures require no material preparation and remain unchanged.

No additional unadapted successful-presentation fixture was identified in these bounded owner searches. The old files received only fixture declarations/setup calls; their existing 0.10 compatibility changes and assertions remain intact. Follow-up validation parsed/formatted only the new fixture modules and checked touched tracked-file whitespace; no Cargo/build or native execution occurred.
