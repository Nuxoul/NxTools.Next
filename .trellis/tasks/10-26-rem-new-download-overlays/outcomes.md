# Queue05 — New Download overlays/media rem outcome

## Result

This bounded category/schedule/probe-inspector/HLS/DASH ordinary-layout slice is technically verified. Original audit#6 remains OPEN for later ordinary-role consumers. Work was performed directly in this conversation without delegation. No complete Desktop run or next queued task was started.

Own source changes: `downloads/new_download_view.rs` layout/public component composition and test-only module/geometry selectors; new `downloads/new_download_overlay_rem_tests.rs` production-Root fixtures; append-only `docs/reference-code/download.md`. Eleven live dirty baselines were captured before product repair. Nine protected files, including Queue04 tests,keyboard8,picker14,polish tests,native window support,ordinary_rem and shared field/list helpers, remain byte-identical. The working tree was not reset,external work was not overwritten,and no external repair is credited here.

## Reference/API review and fixed boundaries

Read AGENTS,mandatory component selection,current080/table/index,applicable Desktop/shared specs,Download note and Queue04 outcomes/current sources. Inspected ABDM first: CategorySelect.kt CategorySelect/RenderCategory and AddDownloadPage.kt57–193; then legacy AddDownloadWindow.axaml232–387,constructor->WindowAnimationService.Attach->OnOpened/RequestClose/OnClosed cleanup and guarded delayed Close. Existing Download reference records the product HLS metadata chain and ABDM's absence of a matching complete master-quality/DASH chooser. Unavailable AWT/Avalonia framework-native show/close was not inferred or operated.

Retained actual released0.8.0 Popover/ListItem/Input/Small Select/compact ghost date Buttons/RadioGroup+Radio and their public slots. No replacement Calendar/selection/scroll controller,custom painted control,vendor/dependency/theme/basefont/palette/native policy or new input dispatcher. Outer category320x240,schedule304x200,calendar<=124 and media-list<=120 remain. Native compact560x176/ordinary560x246,caption34/footer44,media base278,HLS one/many338/418,DASH353/433,error reserve38 and all native options/handoff functions remain unchanged. Default16 date40x19 and original geometry tests pass. Larger ordinary content yields to bounded scroll/flex,not larger windows or removed controls.

- Category:34px two-line row12/18+11/16,search gap and available content width follow rem; fixed width subtracts actual released Popover/ListItem padding rather than scaling272px into overflow. Original search/highlight/ScrollHandle/Enter select/focus-return and full name+path Tooltip remain.
- Schedule:header/date/type/gaps/66px Select rails follow rem; existing calendar viewport can shrink below124 to keep original time/actions inside200. Seven ideal40px columns shrink only within available width. Original reset/confirm/clear/cancel/deferred owners remain.
- Media:inspector120x112 and its ordinary text/icon roles now follow rem for media too. HLS/DASH heading/gaps/minimum40px rows follow rem; long Chinese metadata wraps in original Radio children,retaining complete names/details and released official semantic typography.
- Stateless `NewDownloadMediaViewport` resolves natural ordinary height at the existing Window/rem boundary and clamps parent allocation to120px before layout. It hosts the original official Scrollable with per-list stable IDs. No business state,new ScrollHandle or parallel scroll owner. Released RadioGroup12px/rem gap remains official.
- Bitmap/hairline/native caption/footer/hit/placement exceptions remain fixed. Resolution-details Popover and Queue04 basic input/picker/compact/confirmed/footer composition are protected unchanged.

## Effective reds and preserved failures

| Evidence | Observed result and disposition |
| --- | --- |
| `category-red.log` |0/1 effective red:base14 actual ListItem34px,expected29.75; corrected ordinary row/type/width roles |
| `three-layout-first.log` |category PASS; schedule/media effective reds atbase14:date19px vs16.625,media40px vs35; corrected roles within fixed budgets |
| `five-interactions-first.log`,`media-scroll-diagnostic.log` |long-media wheel did not move final row; actual viewport528x120,y222 and final rowy586. Released Scrollable retained max_size on content,preventing usable scroll extent. Moved budget to a plain layout viewport around original Scrollable |
| `audit06_overlay_-after-scroll.log` |HLS end/selection passed; DASH final metadata failed from same-callsite shared scroll identity. Public independent ID correction made end/pointer pass; not a second controller |
| `media-scroll-independent-id.log` |new test failed reaching Radio from absent focus. Source recheck rejected the speculative roving-focus explanation; actual Input pointer focus before Tab made original Radio focus/Space release pass. No production focus compensation |
| `downloads-first.log` |200PASS/1FAIL:old media_submit_error_preserves_all_probe_rows_and_native_target found error y478/footer412,options268px; retained h+max_h still contributed oversized parent flow. Window-bound clamped viewport fixed old geometry without changing assertion/native budget |
| `media_submit_error_preserves_all_probe_rows_and_native_target-bounded-viewport.log` |original regression1/1 PASS; subsequent Downloads201/201 PASS |
| `audit06_overlay_-final.log` |4PASS/1FAIL from a new exact-width assertion:base14 Select rail57.5px vs theoretical57.75. Corrected only the new assertion to0.5px GPUI pixel-alignment tolerance; old geometry untouched |
| `ui-token-boundaries.log` |5new test-only constant-font restoration violations. Corrected tests to capture/restore complete original ComponentTheme,not weaken gate; final token gate PASS |

Intermediate uncompiled line-edit mistakes were corrected by immediate live diff review; original Close Button was restored before any build. All original state/methods/callbacks remain independently checked. Temporary DEBUG-OVERLAY diagnostic marker was removed after real interaction passed. No ignored tests,fixed sleeps,blanket suppression,direct business-method positive activation or old-test weakening.

Earlier Queue04 failures,compile-only continuation checkpoints and the historical complete Desktop1237/1FAIL remain immutable. This task does not relabel compile failure as a runtime red or use focused green to erase the old full-suite failure.

## Compiled and source verification

Every Rust invocation inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; natural Cargo/rustc idle,serial commands and full-operation timeouts. No process kill/pause,reservation,duplicate build or peer coordination.

| Check | Actual result |
| --- | --- |
| Final five production Root overlay/media tests |5/5 PASS,0ignored,30.85s; final complete-theme restoration included |
| Related Downloads |201/201 PASS,0ignored,93.44s; unchanged oldkeyboard8/8,picker14/14 included |
| audit06 combined |16/16 PASS:Queue04five,accepted Schedule/row/header6,new5 |
| audit05 |15/15 PASS |
| Exact download_page::kit_migration_tests |18/18 PASS; not historical wider21-test selection |
| Original media error/native target regression |1/1 PASS;456px budget/error/footer assertion unchanged |
| Strict Desktop Clippy --tests -- -D warnings |PASS including final test-theme restoration |
| Workspace check --workspace --tests |PASS before final test-only theme-restoration cleanup; final Desktop test compile/strict Clippy cover cleanup |
| Scoped rustfmt skip_children --check / scoped diff --check |PASS |
| Workspace cargo fmt --all -- --check |FAIL outside scope:nxtools-core/src/extensions.rs15,933,940,949,965;nxtools-extensions/src/registry.rs529. No outside formatting repair |
| Source/document gates |9/9 PASS after test-theme restoration:download lifecycle,token,consistency,state,motion,catalog,release,editor,spec |

No complete Desktop test suite was run.

## Actual interaction evidence and limits

Production Root16/14/18:category wheel reaches32nd item,original hover/test-clock exposes complete-path Tooltip,semantic ListItem retains full name/path,real search filters and Enter selects original category without outer validation. Schedule actual wheel reaches last date,real date Button changes draft,Small Select pointer+Down/Enter changes only time draft,real Tab reaches Confirm/Clear,Enter confirms,Space clears,Escape cancels while preserving committed selection. Long HLS/DASH metadata wraps and wheel reaches full final detail;pointer selects original final item;after actual filename Input focus,real Tab reaches selected audio Radio and Space release clears once. Original outer error/probing/submitting/tasks remain unchanged in child-layer tests. Original keyboard8 covers IME/held/modified/busy/childSelect/Popover guards;picker14 remains unchanged.

Synthetic pointer/key events and controlled font14/16/18 are not physical keyboard,WindowsIME,DPI/mixed-monitor,UIA/reader,visual/user or native/installed/release acceptance. No real external network/download,picker/Shell/Host/service/user-data,extension execution/install,packaging/install,commit or push was performed by this slice. Test fixtures use isolated synthetic data,memory engine and disconnected bridge; existing unrelated fixture behavior is not claimed as product acceptance.

## Evidence integrity and remaining work

`evidence/inspect.py` checks26 source sections/owners including18 original callbacks,9live protected files,88Queue04 evidence hashes and480older frozen-history files. Native size/typed-media helpers,state/constructors/probe/TLS/Enter/IME/picker/submit/defer,Queue04 basic composition,HLS label classification,DASH nullable selection and resolution Popover/motion are normalized-identical. Reference starting bytes are exact prefix. New source snapshots and forward/reverse deltas are against live dirty baselines,not HEAD. `evidence/verify.py` verifies frozen evidence,live sources and exact reversible deltas read-only.

Bounded slice has no remaining test/compile/strict-lint/source-gate blocker. Workspace formatting remains an explicit external FAIL. Original audit#6 stays OPEN for later ordinary surfaces/role matrix. Physical/native/DPI/reader/visual/user/installed acceptance remains NOT RUN. Stop here; no later queued work was started.
