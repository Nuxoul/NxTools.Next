# Queue 03 — Extension preview cancellation diagnosis

## Outcome and honest status

**Confirmed and corrected a local test-fixture hazard: the test sampled an entering Dialog's Cancel bounds and later released the pointer at a coordinate the animated Button had left.** The product cancellation, candidate identity and asynchronous publication code were not changed. The business/config test now uses the same public reduced-motion fixture policy as released Component Dialog tests, and an additional motion-enabled regression verifies real pointer cancellation after entrance and across reopening.

**The exact causal attribution of the historical complete-suite failure remains unproven.** The unmodified original test passed official scheduler seeds0–31. The historical1237/1 log contains only the preview-is-none assertion, not pointer/frame/identity observations. The controlled experiment below reproduces that same assertion through a demonstrated moving-target mechanism, but deliberately advances the test clock between down/up; it is not a claim that the original run used that exact timing. No complete Desktop/MainWindow rerun was performed. Do not convert the historical full-suite FAIL into PASS or claim this task resolved every possible import cancellation race.

Strict Desktop Clippy also remains **FAIL** at the recorded verification point because of an independently changed WireInvocation large_enum_variant in nxtools-extensions/src/session.rs326. This task neither edits nor suppresses that outside owner. Related runtime tests, workspace check, scoped format/diff and four source/document gates pass.

## Read and preserved owner chain

Read AGENTS/shared/Desktop/Extensions contracts, current Kit0.8.0/defaultpre0.3.9 selection/table/index, WinShun ZIP-import reference note, exact released Component Button/DialogClose/Root/Dialog animation source and existing production import code. Reference-only/native lifecycle was not changed or executed.

The traced production chain is:

1. Actual preview Button -> deferred ExtensionApplication::preview_path -> original bounded background ZIP validation.
2. Publication checks original generation/pending identity and sets preview only for the current generation.
3. ExtensionManagerView::reconcile_import_dialog captures immutable candidate identity and opens one official Root Dialog on a later app turn.
4. Actual Cancel is official DialogClose. Its public trigger styles a Base-owned Button; on_cancel checks submitted/busy, then defers cancel_import_current(identity).
5. Application cancellation rejects busy/foreign identity, advances the generation, clears preview and notifies. Dialog on_close retires the view's matching identity on a later turn.
6. Explicit confirmation revalidates current candidate/previous package and enters the original checked Core temporary-config commit; cancellation cannot install.

All of that production code remains byte-identical to the captured task-start dirty baseline. No competing click dispatcher, direct cancel shortcut, permission/transport change or vendor patch was introduced.

## Bounded experiments and falsifiable observations

The ranked hypotheses were moving entrance bounds, deferred cancellation not yet applied, and stale/new candidate identity or late publication. Exploration was bounded to the exact named test and targeted related modules, not repeated whole-suite green hunting.

- **Original seed0–7:** PASS across eight official GPUI scheduler seeds.
- **Original seed8–31:** PASS across another24 seeds. These are non-reproductions, not proof of a fix.
- **Controlled pointer/clock red:** temporarily split the original Cancel simulate_click into MouseDown,50ms official test-clock advancement plus one draw, then MouseUp at the original sampled center. No wall-clock sleep or direct app cancellation was added. The same original preview-is-none assertion failed. Test-local probe recorded Cancel bounds x1162/y575,size39x32 before down and x1162/y596 during the held gesture:21px vertical movement. The old center was no longer inside the current Button. Application busy=false, current application/view candidate identities agreed before and after, and preview remained present. This proves the click did not successfully invoke cancellation; it does not show a valid cancellation was later overwritten.
- **One-variable reduced-motion control:** kept the same split real pointer gesture,50ms clock progress and draw, but set App::set_reduce_motion(true) in the test fixture. The Button stayed at x1162/y667,size39x32; preview was cleared and the original full business flow passed. No application/identity/filter/timeout code changed.
- Released Component Dialog source installs SlideDown entrance by default with public ANIMATION_DURATION250ms; Button activation remains release-owned. Released Dialog tests explicitly set App reduced motion for business/geometry fixtures. This supports fixing the fixture rather than changing product animation or cancel semantics.

The artificial50ms progress is a controlled red/green stress variable, not an arbitrary wall-clock wait added to make a test pass. Temporary DEBUG-EXTENSION-CANCEL diagnostics and their source snapshots were retained in evidence, then removed from live source.

## Own test changes

Only the cfg(test) portion of main_window/extensions_page.rs changed:

- Original w6 manager business/config test explicitly sets reduced motion, covers scheduler seeds0–7 and preserves every original cancellation/confirmation/install/toggle/alias/remove assertion.
- Its Cancel action is real MouseDown/MouseUp with a50ms test-clock/frame boundary and an added exact stable-bounds assertion, so removing fixture stabilization exposes the proven moving-target hazard rather than silently restoring a flaky immediate click.
- Added exact temporary configuration-byte equality on preview cancellation; existing config package-empty assertion remains.
- New motion-enabled test covers seeds0–3. It opens the real import form, invokes the actual preview Button and original async validation, uses the released entrance duration to advance the test clock (no wall-clock sleep), then re-reads the actual settled Cancel bounds. It asserts an orphan release does not cancel a new candidate, real down/up cancels, reopening changes the candidate generation, configuration stays byte-identical and no temporary installed-package directory is created. Two preview/cancel generations are exercised per seed.

No original assertion was ignored/removed/weakened and no positive cancel result calls the app cancellation method directly. Synthetic paths are hydrated using the original test input setup; the actual Preview/Cancel/Confirm pointer controls own operations. The long original test's explicit confirm still stages only its temporary declarative fixture and reopens the real temporary config; no real installed extension is executed.

## Performed verification and retained failures

Every Rust shell used CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Commands ran serially after natural Cargo/rustc idle checks with adequate timeout. Another active build was allowed to finish without process termination,pause,resource reservation or sequencing requests. SEED/ITERATIONS were restored after controlled runs. No complete Desktop/MainWindow suite was run.

| Check | Actual result |
| --- | --- |
| Controlled animation-on split gesture | FAIL at original preview-is-none assertion;0/1,0.15s |
| Same controlled gesture with reduced-motion fixture only | PASS1/1,0.64s; original full configuration flow retained |
| New motion-enabled settled/reopened identity test | PASS one test across seeds0–3,1.09s |
| Updated original business/config test | PASS one test across seeds0–7,4.43s |
| main_window::extensions_page::tests:: | PASS6/6,zero ignored,5.66s; includes both multi-seed tests |
| main_window::extensions_page::import_confirmation:: | PASS5/5,zero ignored,1.08s; stale candidate,late validation,failure/retry and competing commit/cancel preserved |
| extensions::tests::w6_picker_cancel_disconnect_and_late_result_never_import | PASS1/1,zero ignored |
| cargo clippy -p nxtools-desktop --tests -- -D warnings | FAIL exit101: external session.rs326 WireInvocation large_enum_variant; no allow/boxing/product change attempted |
| cargo check --workspace --tests | PASS exit0 |
| Scoped rustfmt --edition2024 --config skip_children=true --check extensions_page.rs | PASS exit0; only new/edited test rows formatted, original product/tests outside scope retained |
| Scoped git diff --check | PASS |
| GPUI component catalog,release contracts,Markdown editor provenance,spec contracts | All PASS; source/document boundaries only |

Two intermediate new-fixture errors are retained separately: a missing WindowExt trait import prevented compilation, then a directly hydrated import_open flag without view notification left the preview Button unmounted. The corrected new fixture uses the original actual Import command and publishes input hydration before looking up Preview. Neither failure was counted as a product/behavior red, no assertion was weakened and no product callback was repaired for fixture convenience. A temporarily misplaced trait import was corrected before compilation and is not a runtime result.

## Dirty attribution and evidence

Eight task-start dirty baselines were captured before source editing. The production prefix of extensions_page.rs through cfg(test) and all pre-existing tests outside the single business-test region remain unchanged. The new regression is inserted before the original unchanged wait_for_idle helper. Seven other captured files (application,import confirmation,workspace,theme and three released component files) remain hash-identical. They are not inferred unchanged merely from git status; own-delta and protection verification use raw byte baselines.

The original complete-suite1237/1 log and prior parent review remain immutable historical references. Exact task own-delta replays forward/reverse, raw controlled red/green logs,fixture failures,source snapshots and external lint-source observation are published under evidence with hashes. Temporary DEBUG markers exist only in retained diagnostic evidence,not live source.

## Remaining limitations and stop

Confirmed test-fixture moving-coordinate reliance is addressed and directly regression-protected. Normal animated settled cancellation,close/reopen identity isolation,original asynchronous validation retirement and temporary-config confirmation boundaries pass in scoped tests. **Historical exact full-run timing attribution remains unresolved, and strict Clippy remains blocked by the independently changed session enum.** No product cancellation defect is claimed fixed. If a later integrated run reproduces preview retention with a valid stable hit, it requires its own causal capture rather than assuming this fixture correction covers it.

No subagent/independent session,real extension install/execution,user configuration,product/PlatformHost/service,Shell/Registry/native,network/download/archive/clipboard/picker operation,packaging/install,commit or push occurred. Physical mouse/keyboard/WindowsIME,real HWND/DPI/UIA/reader,visual/user/installed/release acceptance: NOT RUN. Stop after this queue task; do not start later tasks manually.
