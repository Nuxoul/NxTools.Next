# Queue06 — Download Settings/task-window ordinary rem outcome

## Status and scope

The bounded Settings/task-window layout implementation,related compiled regressions,strict Desktop Clippy and workspace check are complete and PASS. **Latest workspace-wide formatting remains an external FAIL**,not a product-layout blocker. Original audit#6 remains OPEN. Work ran directly in this conversation without delegation. No later queued task or complete Desktop suite was started.

Own product edits: `downloads/settings_view.rs` and `downloads/task_window.rs`; one new direct `downloads/settings_task_rem_tests.rs`; append-only `docs/reference-code/download.md`. Nineteen live dirty byte baselines were captured before repair,not reconstructed from HEAD. Sixteen other sources remain hash-identical. No shared helper,vendor,dependency,global theme,basefont,palette,native budget,engine,storage or IO/copy ownership was changed. No #11/#12 refactor,virtualization controller or repeated NewDownload/Schedule/main-row migration.

## References and component selection

Read AGENTS,mandatory selection,current080/table/API index,Desktop/shared specs and Download reference. Inspected ABDM first: DesktopSettingsComponent selected configurable groups; ShowDownloadDialogs keyed task/phase windows and CommonContent/close; ProgressDownloadPage bounded tab/details scrollers; CompletedDownloadPage name/icon/footer actions. Then legacy SettingsWindow constructor->WindowAnimationService.Attach->OnOpened/ConfigureForCurrentScope,SaveButton_Click ApplySettingsCommand and close-only cancellation; DownloadProgressWindow constructor/subscriptions/OnOpened/OnClosed/ShowCompletionState/ResizePreservingCenter. WindowAnimationService guards delayed Close. Unavailable AWT/Avalonia framework-native bases were not inferred or operated.

Retained released0.8.0 Settings/Page/Group/Item,Form/Input/Select/Switch/ListItem/Button,DescriptionList,semantic Table,Progress,Toolbar public content and DialogFooter. Variants and semantic sizes remain. No Toolbar.child adaptation was introduced. `download_settings_input` is a local stateless layout helper accepting Length,so ordinary rails can use existing ordinary_rem while the original Input entity/field_shell owns its appearance/IME. Settings sidebar_width resolves rem at the current Window boundary; DescriptionList label_width accepts rem directly. Original shared aligned_setting_row composition is reused with a relative consumer width; its internal common-control migration remains the separate shared-UI task,not a blanket claim that all shared helpers are now rem-compliant.

## Ordinary vs fixed roles

Changed ordinary roles:
- Settings local88/120/250/286input rails,164sidebar/228category column,40category row minimum,32category heading,workbench13/20+12/18 type,local4/8/12spacing,diagnostic36label/28action rails. Original official controls retain their tiers; category editor has bounded scrolling inside its original management area.
- Task content16/8/4insets/gaps,34success vector badge,18/16vector glyphs,32identity/two-line name,16metric/capability lines,44DescriptionList label rail,24semantic Table rows. Table columns retain original40/88/140/140 ratios inside actual available width. Data vectors,status/speed sampling and segment semantics are unchanged.
- Task content can scroll within the fixed client; original failure Alert has its own bounded error viewport. Long completed/failed paths gain bounded Tooltip/full semantic labels without any new native/Shell action owner. Footer actions remain outside content scrolling.

Fixed/unmodified budgets and exceptions:
- Settings actual984x650 options and native minimum; constrained720x480 is synthetic verification only,not a new native minimum. Existing titlebar36/footer48 chrome,Header Textarea120viewport,category310management area,error96viewport and taskbar preview/native/device/hit geometry remain fixed. No window inflation.
- Task width440,collapsedprogress/completed184,failed196,expanded280,title34/footer44,minimum/phase/transition/retry/reset/close/native placement policy remain unchanged. Failure viewport36..40,chrome inset/radius,hairlines and approved physical/bitmap/hit roles remain.
- Existing official XSmall Progress supplies its own tier; matching local track allocation follows the same rem scale. No replacement progress/animation clock or diagnostic stream.

## Effective reds and preserved failed attempts

1. `initial-red.log`: actual production Root2/2 failed atbase14. Settings Input was88px wide beside28px control height;task metric line remained16px instead of14. These are effective layout reds,not compile failures.
2. `audit06_aux_-first-green.log` is misleadingly named historical compile-only failure:E0277 opaque Length lacked Clone at `.w`;two new unkeyed Div semantic-label calls failed E0599. Fixed by converting width to concrete Length and adding stable path IDs. No behavior red is claimed from this log.
3. `six-first.log`: new test used load_or_create(AppPaths) with a PathBuf;compile-only E0308. Corrected test to read/deserialize its owned temporary config,not initialize user paths.
4. `six-second.log`3PASS/3FAIL and `six-third.log`4PASS/2FAIL retain fixture defects and their causal evidence:
   - Category debug selector represented960px Scrollable content,not a clipping viewport;original last-row Button wasy1298 outside the984x650window. Test now uses the existing outer master/action/content intersection and actual current-frame Button bounds for real wheel/pointer. It does not fabricate visibility or weaken a geometry assertion.
   - Clipboard.enabled is intentionally forced true by existing Core normalize;expecting false after a section save was invalid. Test uses valid unrelated max_items=3210,verifies independent temporary write,then proves original download Apply merges it. Product normalize/section/save owners were not changed.
   - Released Button prevents pointer default focus. Raw Tab from absent focus had no focused node;test initializes the existing public focus sequence once,then sends actual Tab and complete Space down/up. Current hit bounds are reread after scroll/resize. No production focus workaround or second key dispatcher.
5. `six-fourth.log`6/6 PASS;final collapsed-progress wheel/end extension is verified in `aux-final-resume.log`6/6,and the final equivalent lint cleanup is verified again in `aux-lint-clean-final.log`6/6. Long Settings/error/categorization,Apply/Cancel/section merge,task phase/error/path/details/actions are green.
6. External compile-only blockers retained,not repaired or counted as runtime reds:
   - `aux-final.log`:Core extensions.rs calls Extensions private verify_runtime_selection,E0624. Independent registry source later made it public.
   - `audit05-final.log`:storage_calls calls private storage::validate_value,E0603. Independent work later made it pub(crate).
   - `audit05-resume-final.log`:runtime_storage tokio binding ambiguous E0282. Independent implementation replaced that chain;remaining protections then ran.
7. `desktop-clippy.log`:strict -D warnings FAIL at external `rust/crates/nxtools-extensions/src/runtime.rs:353`,unused async method `still_current`. Observed SHA256 `68cf2c9d6e6b73a8fcd8e767afe51c8c192424fb9bd9dbb9ee25b24add43f99f`;snapshot retained. No suppression or outside edit.
8. Independent runtime repair made still_current test-only and forwarded to existing still_run;`runtime-clippy-external-delta.txt` retains the exact observation. Re-running strict lint then found one own new-test collapsible_if (`desktop-clippy-after-external-repair.log`);an equivalent let-chain fixed it without suppression. Final strict Clippy PASS in `desktop-clippy-clean-final.log`. No external repair is attributed to this slice.
9. Workspace fmt initially PASS in `workspace-fmt-observed.log`;later external test edits introduced formatting differences at nxtools-core/src/extensions.rs1132/1140 (`workspace-fmt-clean-final.log`). Latest fullfmt remains FAIL;no outside formatting was applied.

Temporary DEBUG-AUX diagnostic marker was removed. No ignored tests,fixed sleeps,old assertion weakening,full-suite green hunting or direct business-method positive pointer/key evidence. Historical NewDownload compile-only checkpoints,Queue04/05 failures and original full Desktop1237/1FAIL remain immutable.

## Actual interaction checks

Production Root16/14/18:
- Settings984x650 and constrained720x480:ordinary input width/height scale coherently;fixed footer and viewport budgets remain. Twenty-four long Chinese category records wheel to the real final row;actual original Button selects it. Pointer Up then actual Tab/Space Up invokes existing action counter exactly once per gesture (1 then2),preserving original category owner.
- Thirty-five-line synthetic Chinese validation/error scrolls to the true original Alert end;Apply stays inside fixed footer. Original Header120frame,stable entities,request/FFmpeg/picker behavior are retained by existing related tests.
- Actual Input edit7 -> original Apply pointer -> original temporary config/engine change;newer unrelated Clipboard max_items3210 survives section merge. Reopen/edit9 -> actual Tab/Space Cancel leaves exact config bytes and live limit7 unchanged. No real user-data write.
- Task actual pointer disclosure -> expanded280 -> original semantic Table wheel to40th segment,inside width. Current pointer collapse then actual Tab/Space re-expands once. Original cancel Button transitions only queued synthetic task to Cancelled;one keyed task window remains.
- Failed196/completed184 synthetic display states retain34title/44footer. Forty-line failure Alert scrolls to full end. Long completed filename stays within two relative lines;full path Tooltip is reachable,all original completed action bounds remain in footer without invoking Shell/OLE. Actual caption close removes only the window,retains task. Collapsed184progress wheel reaches capabilities at larger rem without moving footer.
- Original retry/details-reset/native phase behavior is unchanged and existing related regression passes. No new retry/resume/real transfer was initiated by new tests.

## Verification performed

All Rust commands inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256,natural Cargo/rustc idle,serial execution and full-operation timeouts. No kill/pause/resource reservation or duplicate build. Source/static gates do not certify product behavior/native/visual acceptance.

| Check | Actual result |
| --- | --- |
| Final direct production Root tests |6/6 PASS,0ignored,71.07s;includes collapsed-content end and final equivalent lint cleanup |
| Related Downloads |207/207 PASS,0ignored,167.83s |
| Original keyboard/picker within Downloads |exact8/8 and14/14 PASS;source bytes unchanged |
| Combined audit06 |22/22 PASS;preserves all prior16 plus new6 |
| audit05 |15/15 PASS after independent compile repair |
| Clipboard stable identity |11/11 PASS |
| Actual Button accessible names |7/7 PASS |
| Download sort keyboard |3/3 PASS |
| Archive object/display |6/6 PASS |
| Exact download_page::kit_migration_tests |18/18 PASS,not the historical wider21 selection |
| Strict Desktop Clippy --tests -- -D warnings |PASS in desktop-clippy-clean-final.log after independent external repair and own equivalent test lint cleanup;both earlier failures retained |
| Workspace check --workspace --tests |PASS without warning in workspace-check-clean-final.log;earlier warning-bearing pass retained |
| Scoped rustfmt skip_children --check |PASS for all3owned Rust files |
| Workspace cargo fmt --all -- --check |Latest FAIL outside scope:Core src/extensions.rs1132/1140;prior PASS in workspace-fmt-observed.log retained;no external formatting repair |
| Scoped diff --check |PASS |
| Relevant source/document gates |9/9 PASS:download lifecycle,token,consistency,state,motion,catalog,release,editor,spec |

No complete Desktop suite was run. Compiled tests were run against live shared sources at their documented times;independent Extensions changes are observations,not attributed to this task.

## Integrity and remaining blockers

`evidence/inspect.py` independently checks39source sections/original callbacks,16live protected files,161frozen Queue04+Queue05 evidence hashes and480older history files. It protects settings draft/parser/constructor/subscription/picker/save/notification suffix and all callbacks;task native size/shell/snapshot/state/sampler/constants/segment/capability semantics and all callbacks. Reference starting bytes are exact prefix. Original shared settings/field helpers,window_support/task_windows_impl,new/schedule/row/test sources are unchanged. Forward/reverse deltas against live dirty baselines and complete snapshots are published;new test snapshot is separately marked new. `evidence/verify.py` is read-only and verifies evidence/current source hashes plus exact delta replay.

**Remaining current check failure:**workspace-wide formatting in external Core tests at extensions.rs1132/1140. Bounded product implementation/tests/strict lint/check/scopedfmt/source gates are green. No external code was repaired or suppressed,and old evidence hashes were not updated to imply it was unchanged. Original#6 overall remains OPEN for later ordinary-role/shared surfaces and closure matrix.

Physical pointer/keyboard,WindowsIME,DPI/mixed-monitor,UIA/reader,screenshots/visual/user/native/installed/release acceptance NOT RUN. Controlled base-font variation is not Windows DPI. No real external network/download/picker/Shell/Host/native/service/user data,packaging/install,commit/push was performed. Stop after this delivery;no later queued work started.
