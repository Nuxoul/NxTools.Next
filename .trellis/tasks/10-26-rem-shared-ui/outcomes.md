# Queue07 — Shared ordinary UI scales: outcomes

## Outcome and status
The authorized shared-ordinary-scale implementation is delivered in this session, without delegation. Actual shared controls/layouts follow production Root font16/14/18 while approved fixed/native/card/tile budgets remain separate. All selected behavioral regressions, test-target strict Clippy, workspace check, scoped/workspace fmt, diff whitespace and11 source/document gates pass.

**One genuine remaining verification blocker:** production-only `cargo clippy -p nxtools-desktop -- -D warnings` fails on `tools/floating_assistant/media.rs:62`, `DecodedMedia.animated` never read. That file is byte-identical to the live task-start snapshot (SHA256 `187ca0f8e4815f97a9e30f6cff78a1595940d21fb9c899d6b4e1fec550f56d7f`). It was not edited or suppressed. This is not a clean production strict-lint closeout.

Original finding #6 remains OPEN for Queue08–10 consumer work and residual-role closure. The original seven findings remain protected; task-count progress is not original-finding closure. No complete Desktop run was executed. The historical1237pass/1fail full run remains failed historical evidence; no isolated green replaces that result.

## Authority, inventory and implementation
The English PRD records the reviewed AGENTS/UI authority/mandatory selection/current0.8.0/table/index, actual consumer/unit inventory, exact released API/variant/size/parent adaptation and fixed-exception ledger. Machine-readable symbol/reference inventories include source coordinates and distinguish reference/import navigation from compiled coverage. No typography role numbers, fonts/basefont, palette, vendor, dependency, zoom setting, cursor, ellipsis/copy or data/thread refactor changed. `ui/metrics.rs`, `ui/typography.rs`, theme and fixed Clipboard/Tools production sources retain task-start bytes.

Production changes:
- Shared path Input composition uses official `h_8/gap_2`, matching its real Medium Input/Button instead of32px outer geometry.
- `aligned_setting_row` accepts actual `Length`, not f32 forced to px, and uses `min_h_8`. Download440, Quick280, Clipboard300 and Global332 consumers explicitly pass ordinary rem; Download removes its previous rem→Pixels→f32 round trip. Existing65% maximum keeps the control lane bounded.
- Shared feature-settings body/footer insets and gaps use official scale; native feature header48 and settings/footer48 remain fixed. Original Switch/Button variants and callback owners remain.
- ModuleHeader ordinary rail48/title16/22/meta11/16 and spacing follow rem. Native client title36 and close-slot36 remain fixed; its ordinary text follows rem.
- Shared Empty/progress/error/interactive-state minimum72 and vector icon18 follow rem. Actual Empty/Label/Alert/Spinner remain official, natural content and original motion/operation ownership preserved.
- Shared icon actions use official Icon Small instead of fixed14px, retaining the actual named styled Small Button and original callback.
- Shared Quick command master224/rows40/headers40+48, workbench type13/20+12/18, insets and action reservations follow rem inside original360px editor viewport. Original Form/five InputState entities/master selection/save/picker/checkbox owners remain.
- Exclusion ordinary type/spacing/icon rail follows rem while40px delete Button/hit target remains exact; row minimum accommodates content without changing that hit contract. Volume ordinary rows/metadata/type/spacing and diagnostics rows/type/action slots follow rem.
- Hotkey ordinary260 width/status-marker/gaps follow rem with max-width/min0 containment; original outlined Button/recording eligibility/controller unchanged.
- Bounded Tooltip11/16 text follows rem while344x136 content/360x160 surface maxima remain bounded. Menu content minimum160 follows rem; menu native/placement/viewport caps remain fixed.

No unnecessary new shared helper/API was added. Existing `ordinary_rem` is reused; official scale/semantic Size is preferred. The single small existing API change (`Length`) represents the stable common control-lane concept, not renamed pixels.

## Effective red→green evidence
All layout reds used actual released production Root; changing ComponentTheme.font_size controlled rem16/14/18, not Windows DPI. Production sources were still unchanged when the initial reds were obtained.

| Effective red | Observed failure | Repair/evidence |
| --- | --- | --- |
| shared-layout-red.log | Root14 state72 instead of63;path32 instead of28;module48 instead of42;volume40 instead of35 | Original four actual component/Root tests green after ordinary consumption fixes |
| editors-layout-red.log | Root14 hotkey260 instead of227.5 | Real recorder/master/path/Form geometry green;delete40 and editor360 stay fixed |
| consumers-viewport-diagnosis /shared-lines-scroll-check | At actual MainWindow920x480/Root18,Save bottom475 exceeded detail viewport bottom461;Input value correct but pointer Save did not persist | Public official Scrollable nested arbitration;actual wheel/current Button clip/pointer/temporary-config persistence passes |

The command failure was diagnosed rather than hidden by a direct save method. Form measured bounds already included Save. Flex/nonshrinking-content attempt did not change the failure and was removed. Temporary `DEBUG-SHARED-REM` probe showed the naked detail div did not receive wheel; released Settings Page→Base ScrollBounce takes capture ownership. Public `overflow_y_scrollbar()` on the original detail content joins the same released nested arbitration with one stable ID. Original360viewport,Form,draft/entities/actions and outer Settings ListState remain; no competing dispatcher/controller or vendor patch. Probe removed before final checks. Exact chain and reference lifecycle notes were appended to `docs/reference-code/gpui.md`, preserving its original prefix.

Preparation/fixture failures are preserved separately and not claimed as product layout reds:
- First new test used nonexistent simulate_scroll_wheel (E0599); replaced with real ScrollWheelEvent.
- New MainWindow fixture initially missed AppConfig/SharedString imports and required a static selector lifetime; corrected before behavior execution.
- Close fixture tried drawing after window removal; now observes original close and App.windows without updating a removed window.
- New Clipboard fixture initially counted feather on two visible cards; retained one synthetic record. Text160 is a maximum rather than forced natural height (Root14 long text159); only text allows natural<=160/near-cap,while image190/file62 remain exact,plus externalgap8/source24/feather14.
- Read-only inventory initially used unavailable rg/defaultGBK; switched to explicitUTF8 Python/embedded search. An owner-inspector initially mistook a top-level cfg(test) module declaration for the old test-body suffix; corrected to the actual `mod tests {` marker,without weakening callback comparison.
- Two production-unused px imports from this migration were fixed as cfg(test),not suppressed. The unrelated production animated-field strict failure remains.

## Actual behavior and fixed exceptions verified
Nine new direct tests cover real Form/Input/Button/Empty/Alert,main/settings consumers and fixed exceptions:
1. Shared path rail/gap matches actual Medium Input/Button at16/14/18;real pointer once each,original Input edit/entity identity,actual Tab/Space same Button once. Long40-line error wheels to its actual last line;48px footer/apply remains inside380x240.
2. Shared Empty/loading/error actual bounds and ordinary icon/minimum follow Root;distinct actual official states remain.
3. Actual volume ListItem/workbench line and diagnostics log row scale coherently without line clipping.
4. Shared module rail/title and original styled trailing action share ordinary scale.
5. Original hotkey,path exclusion and command master/Form scale;delete40x40/editor360 remain exact.
6. Actual MainWindow920x480 switches via real module Buttons through Downloads/Quick/Clipboard/Tools;ordinary headers scale while native caption36/sidebar176 and Tools180x160 tile remain fixed. Configure opens original keyed460x320 window;real switch/control/close bounds fit;original close removes only that presentation.
7. Actual Quick official Settings navigation→command row click→original retained Input edit→real wheel→current Save Button→original disk+memory save at16/14/18. Actual diagnostic/recorder containment retained at minimum. No program execution,picker or index service action.
8. Actual Global720x480 fixed48footer contains original scaled Small actions;rendering never saves;real Cancel retains exact owned config bytes and closes original window.
9. Actual ClipboardPanel/Root16/14/18 preserves text fixedcap160 (not a forced natural height),image190,file62,gap8,source24 and14DIP overflow feather. Existing whole-payload/gradient/selection/shadow/no-frame-loop regressions also pass. No Clipboard production geometry was modified.

Native clients/footer/caption/coordinates/readback/bitmap/hit/physical-line policies,Tools tile,Clipboard persistedcaps/image190/file62/gap8/gradient-shadow roles remain outside ordinary scaling. Bounds/paint tests do not certify screenshots,user acceptance,physical input,DPI/IME/provider/native or release behavior.

## Verification performed
Every Rust command inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Own commands were serial and started after observed natural Cargo/rustc idle;some other processes subsequently started and Cargo waited on cache/build locks. No process was killed/paused,no resource reservation or coordination/delegation was requested,and no duplicate own build was started. Long operations had full-operation timeouts. Run logs/exit codes are retained individually,including earlier failures.

| Check | Actual result |
| --- | --- |
| New direct tests after import cleanup |9/9 PASS,0ignored,4.54s;final tightened fixed-exception assertion separately1/1 PASS,0.56s |
| Combined audit06 |31/31 PASS,0ignored,126.82s;previous22 accepted tests +9new |
| Shared/UI related |237/237 PASS,0ignored,56.41s |
| Downloads related |207/207 PASS,0ignored,164.77s |
| MainWindow related |191/191 PASS,0ignored,94.08s |
| Global settings related |36/36 PASS,0ignored,7.66s |
| Quick Launch presentation |11/11 PASS,0ignored,0.41s |
| Protected first-seven subsets |NewDownload keyboard8/8,picker14/14,Clipboard identity11/11,originalactualButton names7/7,sortkeyboard3/3,Archive object/display6/6,audit05 15/15;exactdownload-page module18/18 |
| Strict Desktop Clippy --tests -- -D warnings |PASS;latest strict-tests-delivery.log |
| Production-only strict Desktop Clippy -- -D warnings |FAIL;latest strict-production-import-clean.log has only unchangedDecodedMedia.animated dead-code;ownunusedimports fixed,no suppression |
| Workspace check --workspace --tests |PASS;latest workspace-check-delivery.log |
| Scoped rustfmt skip_children --check |PASS for22 task-related Rust sources |
| Workspace cargo fmt --all -- --check |PASS;latest workspace-fmt-delivery.log;older Queue06 externalfmt failure remains immutable historical evidence |
| git diff --check |PASS;line-ending warnings on unrelated dirty files do not mutate them |
| Source/document gates |11/11 PASS twice:Download,Desktop shell,Clipboard UI,token,consistency,state,motion,catalog,release,editor,spec |

Original protected subset membership is extracted from complete successful logs,not inferred from broad counts. The seven original Button-name tests are distinguished from an eighth older Switch-name test. No totals are summed into a fabricated unique/full-suite count. Audit06 and affected groups overlap by design.

## Independent work and integrity
Task-start snapshot captured238 existing Desktop Rust files,not HEAD. This delivery has20 changed existing Rust files,two new test files,and an append-only reference note:21 reversible existing-file dirty deltas +2new snapshots. No existing user dirty work was reset/replaced.61 original callback bodies across16production files compare equal ignoring formatting;old test bodies/prefixes remain.212 unaffected baseline Desktop sources,including metrics/typography/theme/Clipboard fixed geometry/Tools tile,remain byte-identical.779 earlier task/evidence records remain byte-identical;old manifests/outcomes/failures were not updated to pretend their live protected-layout hashes still hold after new authorized shared changes.

Independent changes were observed in extensions.rs,quick_launch.rs,runtime.rs,extensions/service.rs,extensions/ui_host.rs and quick_launch/extension_providers.rs,with new service_storage.rs/service_storage_tests.rs. They are observation snapshots/diffs only,not Queue07 production edits. Early UI/Downloads related runs failed before test execution because independent service tests referenced then-missing new_with_storage/refresh_storage_metadata. Later observed independent repair restored those entries;only the two unexecuted groups were rerun and passed. No independent repair is credited here.

First write-once evidence publication intentionally aborted when service.rs changed again. Its208 already-written files were preserved;publication resumed using new final observation paths without overwriting old evidence. External observation hashes describe that observation,not a promise to monitor/freeze external work after the turn. Own source/delta/history verification remains strict.

Evidence: `evidence/own-deltas.json`,current/baseline snapshots,diffs,consumer inventories,compiled-results.json,protected-source/history lists,all48 run logs,source-gate results,production-lint-blocker.json and immutable external observations. Run `python .trellis/tasks/10-26-rem-shared-ui/evidence/verify.py` for read-only hashes,forward/reverse replay,callback/old-test/history/reference-prefix checks. This checks source/evidence integrity,not native/user acceptance.

## Explicitly not performed
No complete Desktop suite,real network/download/picker/Shell/Host/service/user-data operation,product launch,real extension execution/install,packaging/install,commit/push or delegation. No Windows DPI/mixed-monitor/physical keyboard-pointer/WindowsIME/UIA-reader/screenshots/user visual/installed-release acceptance. Controlled Root14/16/18 is not Windows DPI. Queue08–18 are not started by this task.
