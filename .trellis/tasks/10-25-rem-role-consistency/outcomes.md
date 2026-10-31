# Audit #6 — bounded ordinary rem-role outcome

## Delivery status and limits
Implemented only the authorized bounded ordinary-role slice: the daily Download Schedule form and Download workspace task rows/matching table header. This is not a claim that every ordinary px consumer in the repository is compliant. No later audit item was started. Await parent review/retest/acceptance; no commit, push, package, install or product/native launch was performed.

The complete Desktop suite is **not green**: its single permitted run finished **1237 passed / 1 failed / 0 ignored**, with `main_window::extensions_page::tests::w6_manager_pointer_preview_confirm_and_toggle_commit_real_config` failing at `extensions_page.rs:445` (`observer.preview.is_none()`). That file was not edited. No causal attribution beyond the observed failure is asserted, no out-of-scope fix was attempted, and the full suite was not repeated.

A preceding broad MainWindow run was **185 passed / 1 failed**, `archive_settings_official_search_never_saves` at `archive_page_tests.rs:205` (missing `archive-test-after-create` bounds). This test passed in the later complete run, but the earlier failure remains a failure in its original log; this slice does not claim to have fixed it. The known additional QuickLaunch source gate still exits1 in its elevated-navigation compound check; no build/security/gate workaround was made.

## Implemented roles
- `ui/metrics.rs::ordinary_rem(design_pixels)` returns real GPUI Rems against a fixed16px design base. It never reads/divides by the current rem and introduces no state, controller or zoom configuration. Existing constants, typography role values, theme/palette and font families remain unchanged.
- Actual product theme plus production Root confirms the default window rem is16px. Root applies controlled14/16/18 Component Theme.font_size variations itself; the fixtures do not set window rem directly.
- Schedule ordinary4/8/12 spacing uses existing GPUI scale builders. Custom body/label type and line heights,32px control slots,50px Switch rail,114px time rail and label widths use relative roles. Original horizontal Small Form public label-width adapts to available fixed-client width while reserving controls. Weekday layout may wrap; Small outlined Toggles and default TimeFields remain the original official controls.
- Schedule validation retains its original two-line viewport budget, but full long Chinese/path text can now scroll within that same bounded region rather than being irretrievably clipped. Tooltip/full payload, validation, Apply/Cancel and error ownership are retained. The original native430x326 client,36px titlebar and48px footer are unchanged; the real Cancel hit target remains reachable after scrolling.
- Download rows and matching header use relative48px row/28px header, ordinary padding/gaps,26px icon rail,15px vector fallback,13/20 primary type,12/18 supporting type and matching metadata/transfer/status/date column roles. Default16px density and all prior default geometry assertions remain.
- GPUI uniform_list measures the same actual `download_task_row` callback on each layout; no parallel height calculator/cache/controller was added. Thirty queue-only synthetic tasks verify the real last row is fully reachable after current-rem remeasurement and pointer selection still selects its immutable ID.
- Test-only passive canvas probes observe inherited text paint style under the real primary/supporting nodes. They paint nothing, dispatch nothing, and do not alter ordinary production builds. A debug selector identifies the existing list viewport. These observations supplement actual bounds and selected-row quad paint, not a substitute text renderer.

## Preserved exceptions and uncovered consumers
Verified unchanged in the affected production render: shell bitmap20x20, fixed2px selection edge, Schedule native client/titlebar/footer and default/minimum main-window clients. Original hairlines, tooltip popup caps, detail/filter/native budgets, persisted geometry, placement/hit coordinates/OLE and all native adapters remain outside this delta. Tools180x160 and Clipboard card/cap/image190/files62/gap8/gradient/shadow roles were not edited; their approved values are not reinterpreted by this slice.

Exact released Component0.8.0 Switch track remains the official fixed36x20px track; XSmall Progress retains its released4px internal track. Their public semantic sizing is mixed-unit, and this slice neither patches vendor internals nor paints a substitute. The ordinary surrounding rails/rows scale; the internal fixed tracks are **not** claimed rem-relative. These are explicit remaining framework limitations, not newly granted whole-surface exemptions.

Unmodified ordinary consumers remain in New Download, task-window/supporting surfaces, Settings, other MainWindow content, shared UI/history and other modules. Investigation found many px token uses, but the search included approved fixed roles and tests and was not a replacement checklist. No global conversion or full-project closure is claimed. No cursor, copy/ellipsis, performance projection, IO scheduling or String ownership audit work was included.

## Reference and behavior preservation
Read mandatory component selection, current0.8 release, table/index, exact sizing/Form/Field/Button/TimeField/Switch/Progress/Root/uniform_list/scroll source, Desktop/shared contracts and original guide. Download reference review inspected ABDM QueueInfoComponent/Table/OptionsDialog before legacy SettingsWindow.axaml(.cs)/WindowAnimationService. The reference note records the call chains and uninspected AWT/Avalonia native boundary explicitly.

Twenty-one source/prefix/history checks pass in `evidence/owner-invariants.json`: original schedule struct/constructor/activation/default Enter/save/failure/defer and parse/validation; weekday/start/stop/close/Cancel/Apply callbacks; original Download filter/sort/selection functions, uniform_list callback, immutable row projection and pointer/context/OLE boundary; progress/context/menu/batch/detail/format functions; actual sort Button naming/variant/size and single click/scroll/persist/deferred chain. Where rustfmt changed whitespace, normalized hashes are recorded rather than claiming whole-file equality. Both old test files and the old download reference remain exact byte prefixes. All17 captured protected files and345 prior-task/history files remain hash-identical; no historical manifest, failure or parent review was rewritten.

No new cross-process operation or action authority was introduced. Existing diagnostic events remain; no per-render/scale/pointer logging or temporary DEBUG probes were added.

## Effective red / green and non-behavior failures
| Seam | Effective original red | Green evidence |
| --- | --- | --- |
| Actual Schedule Root spacing vs official Toggle | `schedule-red.log`: base14 has fixed4px weekday gap while Toggle already21px high,0/1 | `schedule-green-2.log`:1/1; final six/full suite retains it |
| Long validation within fixed client | `schedule-scroll-red-2.log`: actual full text558px, wheel before/after unchanged,0/1 | `schedule-final-green.log`:2/2; final six/full suite retains true-end/footer checks |
| Actual Download uniform row/header | `rows-red-resumed.log`: Root base14 but actual row48px instead of42px,0/1 | `rows-green.log`:3/3; extended5/5 and final6/6 |

Not behavioral reds: initial Root Result/Option fixture compile error (`schedule-scroll-red.log`), initial ScaledPixels API fixture compile error (`rows-red.log`), quarter-pixel alignment expectation (`schedule-green-1.log`, actual99.5 vs expected99.75), retained-scroll fixture reset (`schedule-scroll-green.log`,17/18), and final new assert_ne formatting failure (`workspace-fmt-final.log`). Geometry tolerance only covers bounded GPUI pixel alignment; the effective fixed4px spacing red retains0.02px tolerance, fixed budgets/default density remain exact, and no old assertion was removed/weakened/ignored.

The row paint assertion now additionally verifies actual13/20 and12/18 inherited paint-time type at all three bases. Additional passing tests cover bitmap/edge exceptions and actual nondefault14/18 Tab traversal, Space down/up once, pointer same-owner restoration and no accidental Apply. These are supplemental checks, not falsely labelled pre-fix behavioral reds.

## Commands and observed verification
All Rust shells used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256, naturally waited for other Cargo/rustc, and ran serially with full-operation timeouts. No process was killed/paused/reserved and no duplicate build was started.

- Final `cargo test -p nxtools-desktop audit06_ -- --test-threads=1`: **6/6**.
- Related Downloads before the last added keyboard regression: **190/190**; complete-suite coverage after it: **191/191**.
- `main_window::download_page::`: **21/21**; broad `main_window::` **185/1** as preserved above.
- Protected focused runs: audit05 **15/15**, history identity **11/11**, actual names **7/7**, sort keyboard **3/3**, audit04 display **6/6**.
- Complete-suite exact protected coverage also includes new-download keyboard **8/8**, directory_picker_tests module **14/14** (a broader substring also matches one other test; do not confuse it with the module count), and the counts above.
- Strict `cargo clippy -p nxtools-desktop --tests -- -D warnings`: **PASS**. `cargo check --workspace --tests`: **PASS**. Final `cargo fmt --all -- --check`: **PASS** after only the reported new assert_ne formatting correction. Earlier successful strict/check output predates that whitespace-only correction; complete tests compile the final bytes.
- Source gates **12 PASS / 1 known extra FAIL**: token boundaries, consistency, state lifecycle, preview isolation, download UI lifecycle, Clipboard UI lifecycle, Clipboard lifecycle, archive release chain, component catalog, release contracts, Markdown editor contracts and spec contracts PASS. Extra QuickLaunch lifecycle exits1; raw log is retained. No gate source was changed.
- Scoped dirty diff check PASS. Format preview and exact formatter-output hashes show only this slice's touched fragments/new test tails changed, skip_children=true; old test prefixes were not formatted.
- **Only complete Desktop run:**1237/1/0ignored; test runtime281.18s, whole command493.42s including compilation. It remains FAIL and was not rerun.

## Cancellation / explicit resume
The first row-red retry returned TOOL_ABORTED and its log contains only Compiling, no result. Parent observed cancelled/result=null; the cause was unknown. This session saw no direct user stop text and did not infer that none existed. It honored the parent's read-only stop, inspected no live Cargo/rustc and unchanged source hashes, and did not resume from that status message. The later explicit parent task `4eb9b4ec-b546-49dc-a479-3a56372365b7` reported the user's “继续” and authorized resuming original#6. Baselines/logs were reused and verified, not recaptured as a false task start; the resumed red uses a new log filename. Neither cancelled delivery is reported successful.

## Evidence level / NOT RUN
Source inspection, compiled synthetic production GPUI Root/render/bounds/inherited-paint-style/quad tests and temporary/in-memory data only. Native actual window/HWND/Shell/service/PlatformHost/registry/picker/clipboard/network/download/archive operations, real user config/database, screenshots/visual inspection, physical keyboard/pointer/Windows IME, Windows/mixed DPI, UIA/reader, installed/release and user acceptance: **NOT RUN**. Base-font headless variations are not DPI evidence. The package graph/vendor/source remained locked; no packaging/install/commit/push occurred.
