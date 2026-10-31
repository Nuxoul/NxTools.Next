# Parent review — audit #6 first bounded slice

## Decision

**Limited acceptance of the implemented Schedule and Downloads-row/header slice on 2026-10-25. Original audit #6 remains OPEN.** This is not item8/13 global completion. Ordinary px consumers explicitly left in New Download, supporting/task windows, Settings and other modules are not made compliant by representative tests of these two surfaces. Continue the same finding in separately bounded deliveries before advancing to another original audit item.

Delivery reviewed: resumed task `4eb9b4ec-b546-49dc-a479-3a56372365b7`, existing implementation session `da6a2ca3-52f0-41f1-a333-3b5548154f0b`, report `c2bbabbe-2bce-41c8-84ae-e40b2ab2cc89`. Earlier task and read-only delivery cancellations remain cancellations; the direct user continuation authorized the resumed work, not a retrospective success claim.

No concrete defect requiring revision was found in the reviewed production delta. Acceptance is source/compiled/synthetic interaction only. The full Desktop suite remains **FAIL**, and neither native/visual/user nor release acceptance is granted.

## Independent source review

The parent performed one directed review while implementation converged, then checked the exact delivered deltas without restarting a broad review. Read mandatory component-selection/current080/table/index, Desktop quality/shared engineering/verification and affected source. Consulted exact Component Root.prepare (theme font_size -> rem), public Form/Field label-width API, released fixed Field internal gap and TimeField sizing. External ABDM/legacy reference visits belong to the implementer's evidence; parent does not claim to have repeated unavailable native framework inspection.

- The metrics helper returns actual Rems from a fixed verified16 design base; it does not divide by current rem or add another scale/controller/configuration owner.
- Schedule uses official scale builders and original styled Form/Toggle/Switch/TimeField. Relative rails and available-width label adaptation retain the fixed430x326 client,36 titlebar and48 footer. Complete long validation text now scrolls inside the bounded two-line viewport. Existing constructor/activation/default Enter/save/failure/defer/validation and original callbacks remain unchanged.
- Download rows/header share their real relative dimensions, columns, gaps and text roles. Original uniform_list measures the same rendered row; no independent row-height cache/calculator, data projection change, selection controller or native gesture change was added. Bitmap20 and logical2 selection edges remain fixed. Official Switch/Progress internal fixed tracks are disclosed, not patched or represented as rem-scaled.
- Tests change Component Theme base font and let actual Root apply16/14/18. They observe real row bounds, virtual stride/end/current stable-ID selection, inherited paint-time text style and selected-row quads. Passive test-only canvas probes do not replace text rendering or paint content. Schedule uses actual wheel events and actual Cancel; additional14/18 Tab/Space down-up/pointer tests preserve original weekday action ownership. Fixtures restore the original theme before final assertions.
- Default-density assertions and approved native/bitmap/Tools/Clipboard exceptions were preserved. Controlled base-font variation is not Windows DPI, physical input or visual acceptance.

## Independent integrity verification

Parent-owned `parent-evidence/integrity.py` replays all six deltas both ways, validates exact before/after spans and live sources, and recomputes behavior sections rather than trusting stored Boolean claims.

- Manifest SHA256 `0bb653bc732a72047a8227ad9e9c48b3316c198ee7789abf0dfc18ce94f5fb4f`.
- Own-deltas SHA256 `bce01154fec5970e8464b41f51e7deddd6c61cd8ad705c6dbbb74f14ad5efc17`.
- **6** reversible task-start dirty deltas; **8** captured live source snapshots (including two unchanged candidates); two new English documents match.
- **94** delivery-evidence hashes, **17** protected files and **345** historical files match. Earlier accepted/failing manifests, parent evidence and logs are unchanged.
- **16** original behavior sections independently recomputed: schedule state/construction/activation/Enter/save/validation and callbacks; original download data/filter/sort/list/projection/pointer/context/menu/detail/action ownership and original actual sort Button callback/persistence boundary.
- Two old test files and old download reference remain exact byte prefixes. Fourteen captured reference/failure-source hashes match.
- Reviewed effective original runtime reds: fixed weekday gap at14, unscrollable long validation, and actual48px virtual row vs42px expected at14. Compile/fixture/pixel-alignment/retained-scroll/formatting failures remain separately identified, not substituted for behavioral reds.

## Parent-executed verification

Each Rust shell used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. Runs were serial after natural Cargo/rustc idle checks; no duplicate build or killed/paused process. No complete Desktop rerun was performed by parent.

| Parent check | Actual result |
| --- | --- |
| audit06_ | **6/6 PASS**, zero ignored,5.59s |
| downloads:: | **191/191 PASS**, zero ignored,51.88s; exact keyboard module8 and picker module14 verified from passing lines |
| main_window::download_page:: | **21/21 PASS**,27.58s |
| audit05_ | **15/15 PASS**,4.97s |
| history_identity_ | **11/11 PASS**,22.43s |
| accessible_name_ | **7/7 PASS**,1.14s |
| sort_keyboard_ | **3/3 PASS**,16.42s |
| audit04_ display | **6/6 PASS**,5.00s |
| strict Desktop Clippy --tests -- -D warnings | **PASS** |
| cargo check --workspace --tests | **PASS** |
| cargo fmt --all -- --check and six-path scoped diff --check | **PASS** |

Twelve parent-rerun source gates passed: token boundaries, consistency, UI state lifecycle, preview isolation, Download UI, Clipboard UI, Clipboard lifecycle, Archive release chain -SourceOnly, component catalog, release contracts, editor provenance and spec contracts. They establish only their documented source/document boundary.

Parent evidence and its separate hash manifest are in `parent-evidence/`; production code and worker delivery evidence were not changed by parent.

## Failed integrated runs — not erased by focused passes

Hash-verified implementer `desktop-full-once.log` reports the only complete run: **1237 passed / 1 failed / 0 ignored**,281.18s test time,exit101. Failure: `main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config`,extensions_page.rs445,preview remained Some after actual cancel click. Its unmodified source alone is not causal proof of independence.

Earlier broad MainWindow run remains **185/1 FAIL** at archive_page_tests.rs205 (`archive_settings_official_search_never_saves`,missing bounds). Its subsequent full-run pass does not establish a fix.

Parent inspected both real test paths and performed **one bounded focused rerun of each**: Archive search1/1 PASS0.42s,Extension manager1/1 PASS0.51s. This shows the symptoms were not reproduced in those isolated runs; it does not explain the cause, certify non-flakiness or make the integrated suite green. No owner/fixture/assertion was changed and no full-suite green hunting occurred. Both original failed logs stay frozen. These are explicit unresolved integrated-verification boundaries, not scope expansion into Archive/Extensions.

Parent reran extra QuickLaunch lifecycle: **FAIL exit1** at the known elevated-navigation compound boundary. The raw log is separate; no build/security/gate repair or elevated-navigation/release acceptance is implied. The PowerShell wrapper propagated that recorded native nonzero status even though the parent-evidence copy/hash steps completed; copied evidence hashes are checked separately.

## Remaining work and evidence limits

Original #6 remains open for explicit uncovered ordinary layout/type/spacing consumers. Accepted work is not reopened unless a concrete regression appears. Next bounded continuation is New Download ordinary content roles with original keyboard/popover/picker/native budgets protected; no later audit finding is authorized merely by this review.

Physical keyboard/pointer/Windows IME, real native focus/HWND, mixed-monitor/DPI, UIA/reader, screenshots/visual comparison, product/user/installed/release acceptance: **NOT RUN**. No product/PlatformHost/service/Shell/registry/real network/download/archive/picker/clipboard/user-data operation, packaging/install, commit or push was performed. User authorization is to continue the serial remediation workflow, not to bypass per-delivery independent review or expand native permissions.
