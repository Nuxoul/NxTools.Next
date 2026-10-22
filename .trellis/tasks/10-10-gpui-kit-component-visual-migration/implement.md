# Execution plan — actual component migration

## Activation gate
Planning only. Require a subsequent explicit approval of the final planning summary before `task.py start`; then load Phase 2.1 and `trellis-before-dev`. Option A resolves visual preference, not the final execution-plan gate. One task owns shared components and ordered family slices. No concurrent same-file writers or automatic commit/push/package/install.

## Before first code change
- [ ] Capture current branch/status, dirty source baseline/diff/untracked list and affected hashes in session scratch. Preserve accumulated fixes; historical checkpoint42f60a8 is not a safe wholesale restore point.
- [ ] Read PRD/design/research/ledger, applicable owner/shared specs and feature-preservation.md. Update the shared UI target/current matrix for A during Foundation implementation; previous519-test/API closeout is baseline, not visual acceptance.
- [ ] Follow indexed legacy references through constructor/base/native show/hide for each changed interaction. Downloads: ABDM first, then NxTools. Record verified invariants before editing.
- [ ] Add red-capable production boundary tests before adapter/consumer replacement. Keep functional assertions; source adoption guards are supporting evidence only.

## 1. Foundation
- [ ] `theme.rs`: retain Kit release typography/radius/state tokens. Map approved neutral semantic colors and compatible installed CJK font family; separate native outer alpha from ordinary controls. Remove old global control/type/radius overrides; no dependency/vendor expansion.
- [ ] `ui/components/{action,field,settings,list,dialog,state,titlebar,menu,popover}.rs`: use actual styled Button/Input/Textarea/Switch/Radio/Checkbox/Select, InputGroup, ToggleGroup and Form/Field where applicable. Eliminate duplicate chrome, forced legacy child typography and unexplained appearance suppression. Keep adapters limited to IDs/official tiers/callbacks/focus contracts.
- [ ] `main_window/shell.rs` and representative Tools settings: Kit Sidebar/MenuItem and compound form/settings presentation with original local state and callbacks. Main1060×650/min920×480, rail176 and Tools460×320 stay fixed.
- [ ] Compare project-adapted controls against direct released components under the same neutral theme: paint/text/bounds, default/hover/pressed/focus/selected/disabled/loading, one callback, Tab/ShiftTab and held/modifier KeyUp. Verify CJK/long paths/help and footer reachability at minimum budgets.
- [ ] Capture and actually inspect owned isolated production-render default/minimum/compact surfaces; verify HWND/PID/title/size. Deliver a representative shell/settings pilot, not only a component showroom. Obtain user Foundation visual approval before broad rollout.

## Approved execution-order amendment — 2026-10-11
User explicitly requests Quick Find before Downloads, with Steward visual reference.
Execute **1. Foundation → 3. Quick Find → 2. Downloads → 4. Clipboard + Tools →
5. Global settings + supporting surfaces**. Preserve original stage names/numbers;
do not silently renumber. Prior "next2.Downloads" handoff is superseded. Current
work is Steward reference/Quick Find planning, not product implementation approval.
Research: docs/reference-code/steward.md; scope/geometry choice remains open.

## 2. Downloads
- [ ] `main_window/download_page.rs`: migrate search/filters/toolbars/detail tabs using Input/InputGroup/Button/TabBar/Badge/Progress and compatible row parts. Preserve six filters/counts, sorting, multi-selection, all columns and five detail tabs.
- [ ] Evaluate ListItem/DataTable against actual Ctrl/Shift anchors, double/right click, persisted sort, batch/queue/context/OLE. Do not create a competing data controller. If incompatible, retain the business grid and adopt leaf components; document the specific exception.
- [ ] `downloads/{new_download_view,settings_view,task_window,schedule_view,shutdown_window,delete_confirmation}.rs`: styled inputs/textarea/select/switch/radio/progress, Form/Field, compatible Popover and compound dialog content. Preserve all destination/category/request/media/default/schedule/FFmpeg/taskbar/diagnostic controls and task actions.
- [ ] Use official compact tiers/composition/scroll inside560×176→278, DASH353/HLS418/error456, task440×184/280/196, settings984×650, schedule430×326 and shutdown450×200. Preserve fixed successful-probe new-HWND handoff, presentation retry without reprobe, same-HWND completion and native OLE/cancel.
- [ ] Red-green real control/label/footer hit bounds plus existing download behavior/window suites; inspect owned visual states. No real user URLs/tasks without authorization.

## 3. Quick Find
- [ ] `main_window/quick_launch_page.rs` and `ui/components/{custom_command_editor,volume_scope,path_list,quick_launch_diagnostics,hotkey}.rs`: Settings/Sidebar/Form/Field, Switch/Checkbox/Radio/InputGroup/Select and NumberInput only when conversion preserves limits. Preserve all five topics, edit-not-execute commands and immediate per-control saves/native status; retain the recorder.
- [ ] `quick_launch/{panel,preview}.rs`: styled search and compatible row/button/label/alert parts on the existing virtual generation controller. No wholesale Command-controller or activating preview substitution.
- [ ] Preserve620×44/410, search42/results40/icon26/trailing48/preview400×410, keyboard/pointer selection, action mode, host integration, focus restoration and hidden/noactivate/final-bounds/show lifecycle. Verify long names/errors/context eligibility.

## 4. Clipboard + Tools
- [ ] `ui/{mod,history,preview}.rs`: styled Input/Textarea/Form/Field, compatible category Tab/Toggle and ordinary buttons. Preserve stable entity identities, IME/query intent, category reorder/drag/wheel, undo, modifiers, paging/cache and300×440/default user resizing.
- [ ] Retain project_menu's single Root session, narrow300px placement, deferred callbacks and frame liveness. Align compatible typography/states with Kit; never restore the overflowing parallel menu path. Ordinary dropdowns remain Kit-owned.
- [ ] `main_window/{clipboard_page,tools_page}.rs` and `tools/link_panel.rs`: compound settings/forms/controls; retain capture/hotkey/modifiers/retention/archive/storage/migration and every Link row/action. Keep Tools460×320, Link420/derived height and original copy-only/browser/six-second hide policies.
- [ ] Preserve actual action/disabled/nontext/category tests and search focus/scroll/resize/passive-preview generation/cancel/timeout/HWND tests, including current8-controller cases. No user Clipboard/history reads or real data operations.

## 5. Global settings + supporting surfaces
- [ ] `network_settings.rs` and `network_settings/blacklist.rs`: Kit Sidebar/Form/Field/Switch/InputGroup/ToggleGroup/Checkbox with all General/Network/application rules. Preserve720×480, validation, cancel/busy/retry, atomic save, deferred consumers and saved≠applied; no trust/schema rewrite.
- [ ] `main_window/notifications.rs` and supporting dialogs/toolbars/state: compatible Notification/Alert/Badge/Progress/Button presentation. Existing dedup/timer/bounded manager remains owner; max3,width336/max112 and action reachability stay fixed. No second notification manager.
- [ ] Audit every ledger entry. Any remaining ordinary-control suppression/manual chrome must have a specific verified capability/native exception, not an entire custom-page exemption.

## Verification
All direct Cargo commands use the root manifest, one process, no concurrent source writers:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-desktop <focused-filter> -- --test-threads=1
cargo test -p nxtools-desktop -- --test-threads=1
cargo clippy -p nxtools-desktop --tests -- -D warnings
cargo clippy -p nxtools-desktop -- -D warnings
cargo check --workspace --tests
cargo fmt --all -- --check
cargo build -p nxtools-desktop
```
Use `cmd /d /c` and complete scratch logs, actual exits/counts and full-operation timeouts. Run focused checks per slice, then an integrated full suite when fixes converge. Fix/retest concrete defects without broad review loops. If building is declined for memory pressure, defer Cargo until separately authorized.

Affected architecture gates: clipboard-lifecycle, clipboard-ui-lifecycle, quick-launch-lifecycle, download-ui-lifecycle, desktop-shell-lifecycle, ui-consistency, ui-token-boundaries, ui-state-lifecycle, ui-preview-isolation, file-dialog-search-visual; ui-motion-policy when animations are affected. Replace obsolete appearance expectations with measured released-component assertions, never weaken functional or native-budget gates. Add a source adoption guard alongside actual production render comparisons; imports are not acceptance.

Planning/document checks: `python tests/architecture/spec-contracts.py`, `python .trellis/scripts/task.py validate <task-dir>`, `git diff --check`. Native checks remain exact-filtered isolated C# fixtures only; never unrestricted DOpus/native harness. Re-run both modes of existing passive native All gate when preview focus/rendering changes; report actual RUN/PASS and message-policy versus physical scope. No service/indexer operation automatically authorized.

## Evidence closeout
- [ ] Each ledger row distinguishes adoption, functional checks, inspected rendering, user approval and native/physical/DPI/reader limitations.
- [ ] All ordinary controls retain Kit visual grammar and complete feature reachability at default/minimum/fixed clients, including long text/error/disabled/loading/keyboard states.
- [ ] Full Desktop, strict lint, workspace/fmt and affected gates pass. At most one optional bounded review if useful, not repeated debate.
- [ ] Foundation approval recorded; final whole-app visual acceptance presented. If screenshots cannot be inspected, state the visibility limitation rather than substituting HTML-only proof.
- [ ] Update references/spec/audit at the actual evidence level. Component source migration is not installed/user acceptance. Packaging/install requires new explicit instruction.

## Rollback and capability gates
Shared theme/action/field code has the highest blast radius. Capture per-family baseline and test fixed windows before propagating changes. Preserve callback/focus/entity identities and existing save owners. Roll back only task-owned hunks after live Read; never reset dirty files. Compound Settings, DataTable and modal compatibility are measured before rollout. Do not recreate old UI merely to make tests green. Report an actual unresolved native-budget/capability conflict with evidence, not a speculative blocker.

## Foundation pilot checkpoint — 2026-10-10
Final planning summary explicitly approved; task started in_progress on refactor/ui-gpui-kit (degraded session pointer). Baseline archive/diff/untracked list saved in scratch. Adopted Kit default theme type/radius, styled shared ordinary buttons/Input/Switch/ButtonGroup, real Sidebar+focused styled Buttons, and production Tools Form/Field at460×320. Nine new adoption/production tests are included; actual first red5/5 andGroup source red retained. Integrated528/528, strict production/tests lint/workspace/fmt/Debug build and10 source gates pass;actual owned96DPI screenshots inspected for main1060×650/min920×480/tool460×320. Both passive native All modes2/2 pass. Foundation user visual approval pending; do not start Downloads yet.

Complete radio/list row, multiline field, download-specific Popover and legacy compact label/workbench composition migration remains in owning stages, not marked adopted. A global Form adapter attempt caused real fixed-download layout regression andwas withdrawn while retaining styled Kit inputs; actual Tools Form remains. Shared adapters use official tiers (Small actions24px/Switch track28×16, Medium input), no forced legacy border/radius. Exact old appearance assertions were replaced only after actual Kit comparison; functional/native containment assertions retained. Narrow disabled label color pairing retained for readability. Detailed outcomes/failures/limits: docs/audits/2026-10-10-gpui-kit-component-foundation.md. No package/install/service/commit/push.

## Foundation alignment correction — first pilot rejected
User rejected the first pilot;do not treatpriorimageinspectionaspilotapproval. Title/actionsmeasured10pxcenterlinedifferenceinredtest. Reworkedtwo-rowheader:24pxfirstrowfortitle/status/Switch/close,16pxsecondrowfordescriptionwithfulltooltip;centerswithin1pxacrossenabled/longcases. Navigationfixed16pxicon/8pxgap/elasticleftlabelcolumns,KitButtonstate/focuspreserved. Full530/530andstrictlint/workspace/fmt/buildpass,6affectedstaticgatespass. Actual96DPIclients/capturesunchangedbudgetsandprocesscleaned. TwoBrowser screenshotreadtimeouts:latestPNGnotvisuallyinspectedbyagent,userreviewpending. ContinueFoundationonly,noDownloadsstageadvance,nopackage/install/service.

## Foundation form normalization continuation
Loaded frontend-design andapplied a single information/control composition instead of per-field patches. ToolForm fourFields nowshare compact_tool_field: fixed264px informationlane,title24px/help16px/gap4,help fullTooltip,control alignedto firstline. Kit styles/geometry retained;window460×320 unchanged. Decorative Configure arrow replacedbyactualKitButton,butparentrowremainssoleactivationowner/nochildcallback/tabstop. Addedproductioncolumn/spacing/containmenttest(firstfailedabsentnamedlanes)andpointerConfigurekeyedwindowreuseassertion;originalduration/mutualexclusion/menu/keyboardtestsretained. Full531/531,strictproduction/testsClippy,workspace/fmt/Debugbuildand6affectedgatespass. Latestowned96DPIcapturessavedandprocesscleaned. Browsernavigate reports 'Browser view did not become visible for the calling session. Switch to that session and retry.';visualinspectionBLOCKEDpendingusersessionswitch. Noaestheticapproval,nostageadvance/package/install/service/commit/push.

## Foundation unified layout pass — 2026-10-10 (single agent, no delegation)
Aborted Opus delegate left no source changes after 22:25 (mtime audit). Inspected actual owned 96DPI captures before/after in the Browser panel. Defects seen: tool entry icon/status/action centred on two-line block; sidebar icon column (16px) off the Kit group label edge (20px) and rail darker than canvas; stepper used text "-"/"+" with value floating; position select lacked caret; controls in a 264px lane ended at different right edges; muted (#71717a) secondary copy too faint.
Rules applied: information left, actions on title line (24px lane); compact settings grid 12|300 info|12 gutter|124 control slot|12, every control right-aligned to slot edge; Kit SidebarGroup label + compact ghost Button rows share the 20px column; Kit sidebar tokens mapped to surface/selection/text; Kit Small outline icon steppers (icons/minus,plus) with disabled edges at 2/60 and accessibility labels; Kit dropdown_caret on position; secondary copy uses text_secondary.
New red-capable tests: control slot/right edge, stepper geometry+disabled bounds, sidebar column inset, tool-entry title-line alignment (initial run 1 failed on value gutter/line-height, fixed). Full 534/534, strict Clippy prod/tests, workspace check, fmt, Debug build, 6 UI gates, spec-contracts, task validate, diff --check pass. Captures re-taken (main1060×650, min920×480, tool460×320); tool and main screenshots inspected. Physical input/DPI>96/screen reader not verified. Foundation user approval still pending; no Downloads stage, package, install, commit.

## Foundation recovery — numeric Blur and Select regression (2026-10-11)

Resumed session b54c35cc-1a3b-45d3-ba44-64d6f226efc4 after agent heap exhaustion;
used supplied recent messages, this task and bounded recent scratch logs, not a
whole-session export. `trellis mem context` did not find the Desktop session.
Git branch remains refactor/ui-gpui-kit; accumulated dirty changes preserved.
No Cargo/rustc process was active at recovery. Todo plugin had no active entries;
task.json remains in_progress. Canonical current stage is **1. Foundation**.

Verified old evidence: dedicated numeric test failed with missing Blur; latest
three field/stepper/header tests each passed one test. Source hashes matched the
last cited reads. The actual failing fixture/probe was in
`main_window/kit_foundation_tests.rs`, not main_window/tests.rs.

Recovery changes and causal checks:
- Original numeric command reproduced red (0 passed/1 failed, exit101).
- Adding only explicit synthetic window activation/draining/active assertion
  made the same test pass with production unchanged (1/1, exit0). Released
  GPUI inactive frame filtering and retained Kit Input blur subscriptions were
  rechecked; see docs/reference-code/gpui-kit-071.md. First-render initialization
  is not this failure's cause: no constructor rewrite or new focus listener.
- Removed the DEBUG-dedicated-blur count monitor. Permanent behavior assertion
  checks invalid99 is restored to last-valid12 on Input-to-Select pointer blur
  **before Escape**, retaining invalid/intermediate/no-commit and Select cancel.
- Foundation filter passed17/17. First full suite was535/536: the existing real
  tool/config regression still searched the retired dropdown option selector.
  Updated that fixture to an active window and actual Select Down/Enter confirm;
  retained temporary SQLite/config and disk TopRight assertion. Focused1/1.
- Strict lint exposed existing nested range validation; combined conditions
  equivalently, without suppression or behavior/persistence changes.

Final frozen-source evidence in session scratch
`f32f826f-f318-49e4-a4bf-509b7d001dcd/`:
- `recovery-final-full3.log`:536/536,0 ignored/failed, exit0.
- `recovery-final-clippy-tests2.log`, `recovery-final-clippy-production.log`,
  `recovery-final-workspace.log`, `recovery-final-fmt2.log`: all exit0.
- Seven source-only architecture gates passed: desktop-shell-lifecycle,
  ui-state-lifecycle, ui-consistency, ui-token-boundaries, ui-preview-isolation,
  clipboard-ui-lifecycle and quick-launch-lifecycle. Logs recovery-gate-*.log.
- DEBUG-dedicated/blur_count/blur_counter search returned zero source matches.
  Earlier red/full/lint failures are retained, not reclassified as passes.
- Every Cargo invocation used jobs1/incremental0/codegen-units256, serially.

Remaining acceptance: this continuation did not launch/capture/inspect the latest
NumberInput/Select pilot, run physical Windows input, mixed/high DPI, reader or
installed acceptance, or rebuild the standalone Debug executable. Foundation
visual user approval is still pending; do not start Downloads. No Release build,
package/install/service operation, commit/push or real Clipboard/history access.
The original Quick Launch flicker/Clipboard crash release outcome is not newly
certified by this focused synthetic/source recovery.

## Foundation role/tier implementation candidate — 2026-10-11

User corrected recovery focus: the real current task is proper Kit component
adoption, size-tier choice and page roles, not only numeric Blur. Explicitly
authorized execution with "开始". No delegation; preserve all accumulated changes.

Reference/production boundary inspected: released0.7.1 NumberInput/Select/Switch/
Form/Field, fetched official NumberInput example HTML, supplied ShellRS title
group/no-drag and host Form/Field source (precedent only, no copied GPL code).
Official Browser page/screenshot inspection unavailable for the calling session;
HTML/source read is not visual evidence. Added reference entries before edits.

Two new production-boundary regressions first failed: default NumberInput32px
versus current Small24px; feature status/Switch and close both had8px spacing.
Initial E0499 fixture compile error was corrected and is not product-red proof.
Candidate retains true dedicated components, changes NumberInput/Select to
released defaults and separates feature-state group from close16px versus8px.
Tools uses264px full information width and remaining control width,32px minimum
lanes without fixed124x24 control box. Field's unused secondary gap is zeroed;
actual first containment failure was4px beyond308, then original boundary passed.
No functional assertion weakened or window inflated. Existing fixed124/Small24
appearance expectations were replaced by actual direct-tier/role/containment
evidence; limits/steps/mutual exclusion/cancel/config/reuse tests retained.

Source changes only main_window/tools_page.rs, kit_foundation_tests.rs and shared
ui/components/settings.rs; metrics unchanged. No save owner, focus initialization,
schema, native command, diagnostics or callback rewrite. Snapshot plus task delta
diffs retained in current scratch foundation-composition-before/composition-delta-*.

Actual checks (serial Cargo jobs1/incremental0/codegen-units256):
- Foundation19/19 (composition-foundation2.log); full Desktop538/538,0 ignored/
  failed (composition-final-full.log).
- Strict tests/production Clippy, workspace --tests check, fmt: exit0; Debug
  executable build exit0. Logs composition-final-* and composition-build.log.
- Seven source gates exit0: desktop-shell, ui-state, ui-consistency, ui-token,
  ui-preview-isolation, clipboard-ui and quick-launch lifecycle. Diff check0.
- Isolated real GPUI Debug preview96DPI:1060x650 main,460x320 Tool captures;
  same HWND reuse, UIA close Invoke retires window, reopen succeeds, owned preview
  process cleaned. composition-native-final.log exit0 and capture-evidence.json.
  UIA observes36x20 switches,32x32 close,168x32 ComboBox and native numeric buttons.
  First native script attempts failed due ineffective synthetic Escape/fixed
  coordinate path and then PowerShell UTF8中文 name matching; production unchanged.
  Corrected harness uses pointer-outside cancel and ASCII Unicode name match/UIA.

Six latest production PNGs: artifacts/ui-kit-foundation-composition/ with gallery.
They are NOT visually inspected: Browser reports calling-session view not visible,
even after BrowserPreview. Need user switch to this session before resuming visual
review. Do not call the design accepted from geometry or538 tests. Physical input,
mixed/high DPI, whole product reader/installed acceptance unexecuted. Current
phase stays **1. Foundation**, awaiting inspection/user approval; no Downloads,
Release package/install/service, real Clipboard/history operation or commit/push.

## Shared template + Tooltip explanatory-copy migration — 2026-10-11

Latest user rejects titlebar feature-state weight and supplies explicit title/close
header, divider, body explanation/enable/settings structure, asking shared template
change. Implemented FeatureSettingsWindowSpec.enabled_label and moved activation
into body with original callback. Structure regression first red, then Foundation19
green/full538green; intermediate strict lint issues corrected, not counted green.
User then requests no permanent per-feature explanations, real Kit Tooltip across
Downloads/Quick Find/Clipboard/global settings. This explicit narrow cross-family
amendment recorded in PRD; full phase order/approval unchanged.

Read release Tooltip/Info Button/accessibility/Root and official Tooltip examples;
ABDM before legacy Download controls, indexed legacy Clipboard/shell behavior.
Added entries before migration; no vendor/upstream edits or GPL code copying.
Tooltip is hover/long-press, not focus auto-show. Shared description_help retains
full text with official Kit Tooltip/XSmall ghost Info, keyboard accessible name,
one owner and no business callback. Sources shared field/settings/tooltip, Tools,
download settings/schedule, quick-find page/diagnostics/volume, Clipboard. Global
settings consumes changed shared rows. Dynamic registration/scan/index/storage/
FFmpeg state remains inline through explicit status rendering; no persistence or
IPC/native contract changes. Tooltip-aware tests update old appearance assertions,
not functionality. Dormant test-only compact_setting_row is historical, not product.

Actual causal verification: Tools help permanent264x16 red→compact trigger green.
Full first536pass/4fail: two retired inline explanation selectors, one real category
info overflow (20px button in18px fixed label), and a synthetic initial-focus gap.
Category label now min-height, original cell containment green. Tooltip content
registered via official TestSupportExt for current-frame show/hide, not cached debug
bounds. Explicit fixture initial FocusHandle→real Tab passes; no production focus
listener, no invented focus-popup. All concrete failures fixed and retested.

Final logs in scratch f32f826f-f318-49e4-a4bf-509b7d001dcd:
- tooltip-final-full.log540/540,0ignored/failed; strict tests/prod Clippy,workspace
  --tests,fmt,Debug build logs tooltip-final-*: exit0, serial jobs1/incremental0/
  codegen-units256. Original failures remain in tooltip-full-first and focus logs.
- Eight source gates tooltip-gate-* exit0: desktop-shell,ui-state,ui-consistency,
  ui-token,ui-preview-isolation,clipboard-ui,quick-launch,download-ui lifecycle.
- tooltip-native-final.log0: self-owned isolated Debug96DPI client460x320,
  enabled centerY76 below titlebar, Tooltip hover/Select snapshots,same HWND reuse,
  UIA close/reopen; preview cleaned.7PNG/gallery artifacts/ui-kit-feature-tooltips/;
  capture-evidence.json and tooltip-uia-controls.json include exact fixture scope.
- Debug probe search0,diff check0. No unrestricted native/service or user data.

Latest screenshot tool reports calling-session Browser not visible even after
BrowserPreview. Images captured NOT inspected; no visual acceptance claim.
Physical input/high-mixedDPI/reader/installed and cross-family live screenshots
remain unexecuted. Current full phase remains **1. Foundation** awaiting visual
review/user approval; this explanatory slice does not advance Downloads. No Release
build/package/install/service, delegation, commit/push or real Clipboard/history.

## User visual approval and next-stage handoff — 2026-10-11

User says "效果可以，接下来该什么UI了" after the shared feature-template and
Tooltip explanation slice. Record approval of this latest presented UI approach;
do not continue pixel/layout correction without concrete feedback. The prior
Foundation pilot approval hold is released for ordered rollout. Next canonical
stage is **2. Downloads**, not Quick Find or another Foundation catch-up phase.
Latest source/native fixture evidence remains540/540 and scoped96DPI checks above.
Agent-side latest screenshot inspection remains unavailable; user approval is
separate evidence, not a claim the agent inspected those images. Physical input,
mixed/high DPI, reader, installed and whole-app acceptance remain open.
This turn only records approval and explains scope; no next-stage implementation,
Cargo, preview, Release package/install/service operation or commit/push.

## Steward Quick Find research checkpoint — 2026-10-11
Reviewed supplied MIT source, constructor/Root/window/native show-hide boundary,
result/control/search/IME/ranking and benchmark/test sources; static asset screenshot
visible through Browser and attachment. No reference or Next executable/test/bench
run. No product code edited; dirty Quick Find changes preserved. User-authorized
stage reorder above; retain original **3. Quick Find** title. User then clarifies:
reference UI layout and components only, preserve all existing sizes/functions/
visible behavior. Unsolicited compact-vs-wide choice is withdrawn. Features and
performance standards are informational reference review, not implementation.
No product code/build changed in this clarification; proceed only within this scope.

## Quick Find layout/component-only first slice — 2026-10-11
User explicitly approves corrected scope ("理解就好，开始吧"). Stage remains
**3. Quick Find**, executed before2.Downloads by user amendment. No size/function/
behavior changes. Read Steward custom input/result composition and released Kit
InputGroup ownership before implementation; old SwiftList recorded input path no
longer exists, managed Listary native-show base rechecked, no engine-source claim.

Smallest production change: panel.rs search Icon+appearance-suppressed Input
replaced by real Kit InputGroup/default32px frame+Search addon, retaining the
original InputState, text roles,42px header and recent-directory button/callback.
Current two-line results/uniform_list/40px row/26icon/48hint, selected/hover/focus,
action mode,400x410 passive preview and settings remain unchanged: already match
compatible continuous-layout principles, not replaced just to increase diff.
No query/IME/cancel/native/search/cache/ranking/feature change or benchmark run.

New production-render gate first red on absent real search group, green after
adoption; group actual Kit snapshot/default height/containment observed. New addon
click focus and original input entity/text identity through expand/collapse passes.
Quick-launch focused129/129 before added identity case; final Desktop541/541,
0 ignored/failed (quick-layout-full-final.log). Strict production/tests Clippy,
workspace --tests/fmt/Debug build exit0 (quick-layout-final-* and subsequent logs).
Six static gates0:ui-consistency/state/token/preview-isolation/quick-launch and
file-dialog-search-visual. All Cargo serial jobs1/incremental0/codegen-units256.

Added explicit Debug-only NXTOOLS_PREVIEW_QUICK_LAYOUT synthetic production
fixture and isolation guard. Invalid application handles; no real data/runtime/
IPC/service/store. Owned96DPI PNGs+exact client readbacks620x44 collapsed,620x410
results/actions,400x410 content in artifacts/ui-quick-find-steward/. This content
window is visual-only, not a substitute for passive native policy. Capture process
cleaned; helper stdout removed from evidence array,4 owned window records retained.
Gallery screenshot actually visible through tool attachment and inspected: title/
path/icon/hint remain bounded, continuous result/action lists and no overlap; Kit
input border now visible. No user aesthetic approval implied from supplied image.

Existing passive native All rerun: quick2/2 + clipboard2/2, actual exact RUN/PASS/
exit0,96DPI clients400x410/460x406 and mouse policy3 vs ordinary1/foreground unchanged.
This is synthetic UIA/message evidence, not physical input/high/mixedDPI/reader/
installed acceptance. No Release/package/install/service/delegation/commit/push.
Stage not complete: broader Quick Find settings/adoption/state review remains under
original scope; current slice awaits user feedback, do not repeat pixel changes
without concrete defect.

## Quick Find collapsed duplicate-outline correction — 2026-10-11
User rejects the square outer outline surrounding rounded Kit input in the supplied
collapsed screenshot. Prior agent visual review missed this defect; do not treat
that earlier review as proof of aesthetic acceptance. Confirmed original native
PNG edge27272a vs canvas18181b, root1px border/header bottom line, plus gallery CSS
image border. Real PNG check first failed at0,0; corrected native screenshot passes
620x44 size and canvas-color edge samples. Collapsed shell/divider colors now canvas,
retaining1px layout reservation and original input position; expanded shell still
uses window_border. InputGroup/shape/dimensions/functions/native policies unchanged.
Gallery adds no border. This removes duplicate painted outline, NOT native HWND
rounding/transparency; no claim of rounded native corners.

Final focused quick131/131/full Desktop542/542,0ignored/failed; strict tests Clippy,
workspace/fmt/Debug build exit0. Logs quick-outline-* in current scratch; actual
capture4owned windows unchanged620x44/410 and400x410. Pixel regression script
check-quick-outline.ps1 retained in scratch. Source regression records collapsed
paint policy. Existing lifecycle gate initially rejected obsolete unconditional
border token; updated to assert expanded-window_border AND collapsed-canvas rather
than removing boundary checks. Latest actual screenshot retry is blocked by calling
session Browser visibility; new pixels verified but latest image NOT eye-inspected.
Current phase3.Quick Find remains in progress; no user visual acceptance, physical/
mixedDPI/reader/installed certification, package/install/service/commit/push.

## Unified collapsed/results/actions search outline correction — 2026-10-11
User identifies expanded results/action screenshots still have the rejected
enclosing outline. Previous collapsed-only fix incomplete; expanded boundary
exception withdrawn. All modes now use canvas shell/divider paint with original1px
layout insets; Kit InputGroup owns the only visible search outline/focus treatment.
Removed unused shell search_focused variable, not actual input focus or motion.
No dimensions/input/controller/results/action/preview/native-policy changes.

Added checked-in PNG gate tests/architecture/quick-launch-search-outline.ps1:
old collapsed passes,results fails at0,0FF27272A (PowerShell fixture expression
error fixed before actual red). After source rebuild/owned synthetic capture,
all3 PNG modes PASS with original620x44/410 and actual Kit input boundary retained.
Source regression/lifecycle gate now guards common policy in all modes, not the
superseded expanded-border rule. Full542/542,0ignored/failed; strict tests lint,
workspace/fmt/Debug build and lifecycle/isolation/consistency gates exit0. Logs
quick-outline-all-* in session scratch. Captures updated in existing gallery;
owned HWND client readbacks unchanged,preview process cleaned. Browser screenshot
retry still blocked by calling-session visibility; latest images NOT eye-inspected.
User aesthetic approval pending,phase3.Quick Find remains ongoing. No native
rounding/transparency,physical/highDPI/installed claim,Release/package/service/commit.

## User-approved running effect / reduced search inset — 2026-10-11
User runs isolated Debug preview, accepts result/action effect and asks input frame
to extend outward toward outer corners. Shared header horizontal padding8->4px;
input grows8px total, no Kit radius/height/client/result/action/behavior change.
Real production geometry regression red8px vs4px, then full542/542 green; strict
tests lint/fmt/Debug build exit0 (quick-inset-* logs). Same Debug exe updated; no
running unrelated process stopped. All3 native PNG recaptured at original sizes;
single-outline pixel gate/lifecycle verification retained. User approval applies
to prior running effect, not automatic approval of this final4px refinement.
No Release/package/service/native-shape/feature change. Phase3.Quick Find ongoing.

## Quick Find settings Switch / command Input slice — 2026-10-11

User accepted the search inset ("可以了"), then explicitly authorized the audited
narrow settings slice and continuation ("开始吧"). Stage remains **3. Quick Find**;
approved launcher rendering is untouched. No delegation, package/install/service,
ordinary runtime launch, real history/index/Clipboard access or commit/push.

Reference review and baseline precede replacement: quick-launch-research.md records
managed CustomCommandViewModel compatibility and released0.7.1 Switch/Input; no
legacy NxTools custom-command surface or Listary native-engine-source claim.
Scratch quick-settings-before preserves both production sources. Genuine adoption
red: simulated toggle35x32 vs direct released wrapper36x32; manual input frame300px
vs actual inset Input282px. Initial fixture ElementId compile error is not red proof.

Production: three controlled Kit Switches have complete accessible names, one
on_change handler and copied requested bool passed through original cx.defer owner
updates. Preserve32px layout slot; default36x20 track is centered, not stretched.
Existing layout regression caught a20px slot after direct adoption; fixed wrapper
composition, not assertion weakening. Five command fields use styled default Input
and their original entities, removing duplicate div border/padding and appearance
suppression. Master selection, picker, flags, save and native lifecycle unchanged.

Final frozen checks (serial jobs1/incremental0/codegen-units256):
- Focused7/7 across6 filters; actual Switch role/checked, pointer once, Space release
  once, same controlled value, real temporary section disk save and exact typed
  ConfigureQuickLaunch payload/one command for all3 production controls.
- Original InputState Chinese edit/focus/repaint identity and single full-width32px
  frame; command master edit-not-execute/picker/cancel/stale/save and minimum/long
  workbench regressions pass without removed functional assertions.
- quick-settings-final-full2.log:545/545,0failed/ignored; strict tests/production
  Clippy, workspace --tests, fmt and Debug build all exit0. Earlier failed source
  edit payload and nested preview-if lint are retained in logs, corrected/retested.
- Seven source gates exit0: quick-launch, preview-isolation, UI consistency/state/
  token, desktop-shell lifecycle and file-dialog-search-visual. Diff check exit0.

Debug-only NXTOOLS_PREVIEW_QUICK_SETTINGS reuses production MainWindow with synthetic
commands, scratch-only UUID database/config paths, empty roots, disconnected bridge
endpoints and invalid unrelated owners. Isolation gate includes this fixture.
Owned PID1692/4HWND captures verified96DPI clients1060x650 (general/context/commands)
and920x480 (commands minimum), then owned process cleaned. Capture log exit0 and
capture-evidence.json in artifacts/ui-quick-find-settings-kit/. Browser full gallery
screenshot actually visible and inspected: default Kit tracks/one input frame,
bounded labels/actions and long-value clipping; no newly observed overlap.

User visual approval of these NEW settings/editor images remains pending. Physical
input, mixed/highDPI, full reader and installed acceptance unexecuted; synthetic
GPUI behavior/screenshot evidence is not native product certification. Prior passive
native fixtures are unchanged, not newly rerun. Broader Quick Find adoption gaps
remain outside this slice (file-dialog search input, navigation, buttons/checkboxes).
Do not declare the whole stage complete or advance Downloads from this slice.

## Official component discovery correction / index — 2026-10-11
User rejects relying on familiar leaf controls/old layout and having to name missing
official components themselves. Settings was already in the approved plan but not
implemented at page/group/item level; previous545-test leaf slice is not compound
adoption. User requests a full official index BEFORE further UI implementation.

Created docs/reference-code/gpui-kit-component-index.md: all78 official Component
documentation families (each page fetched; Input reset retried),47 Base navigation
entries and15 Shell documentation entries; published0.7.1 source/public-export map,
compound subparts, purpose/selection map, extra public modules absent from sidebar,
website/release API discrepancies and explicit current adoption gaps. No claim of
running/visually inspecting every example or fully reviewing every method. Raw
official pages/navigation/public-symbol scan stay in current scratch kit-official-index.

Mandatory discovery now linked by AGENTS.md, specs entry, Desktop pre-development
checklist, reference README and release note. Shared UI contract requires a surface/
compound-first candidate record, exact locked public API, tested compatibility and
only narrow source-backed exceptions. Missing review is not unsupported capability;
green tests/imports/one old page embedded in a custom slot are not page adoption.

Important corrected feasibility detail: SettingPage::render is pub(super) and
SettingGroup::render is pub(crate), not standalone public RenderOnce components.
Top-level Settings owns internal keyed selection/search/resizable panels; selected
public slots can carry existing editors/recorders, but direct custom Sidebar+Page
composition must not be promised without an actual public API. This is a pending
compatibility evaluation, not permission to reject Settings or change behavior.

This slice edits docs/guidance only. Phase remains3.Quick Find; approved search/UI,
window budgets, native/domain/runtime/dependencies untouched. No Cargo/build,
package/install/service/data access/delegation/commit/push. Validate exact catalog
name/URL coverage, recorded release paths, local links/spec-contracts/task/diff.

Index closeout checks: official name+URL sets Component78/78,Base47/47,Shell15/15
match with0missing/extra;173 recorded published source paths exist after correcting
5initial file/directory-name mistakes. spec-contracts87documents/12owners, task
context validation and git diff --check all exit0. These are documentation/catalog
checks only; no new product behavior/native or adoption certification.

## Previously modified UI source reinspection — 2026-10-11
User requests self-review of earlier modified UI against the complete official
component index. Single-agent/read-only product review; scope current dirty tree
and historical layout/Foundation/Quick/Tooltip claims, not a commit-only diff.
Report: docs/audits/2026-10-11-gpui-kit-adoption-reinspection.md. Twelve confirmed
adoption/coverage groups, explicit surface status, retained genuine adopted areas,
pending List/DataTable/Command compatibility and narrow native/menu/focus exceptions.
Cross-family Tooltip missed path_list exclusion-header explanation. Several false
appearance(false) hits are re-enabled by shared field_shell and excluded from
suppressed-chrome findings. Historical545tests are not rerun/compound acceptance.
Ledger now leads with current report instead of relying on stale initial planned
rows or historical implemented-layout wording. Canonical3.Quick Find remains current;
recommend coherent Settings/Form migration without touching approved launcher pixels.
No production source edit, build/preview/runtime/native/service/package/delegation/
commit/push. Document/link/spec/task/diff verification only; implementation awaits
separate instruction, not silently started during review.

## Quick Find Settings capability gate after approval — 2026-10-11
User authorizes current3.Quick Find implementation after reinspection. Preserved
dirty source baseline in scratch quick-compound-before; product rendering unchanged.
Reviewed canonical task/spec/index plus managed OptionsWindowViewModel/CommandOptions
and current Next select_quick_launch_settings_page keyboard/scroll/unmount ownership.
Added test-only settings_compatibility_tests using actual released Settings with
all5pages/Group/Item, resettable(false),160px sidebar and original neutral theme.
Actual2/2PASS,0failed/ignored (quick-compound-settings-capability.log,exit0;
jobs1/incremental0/codegen-units256): Settings always renders one visible search
Input; rebuilding the same keyed Settings with default page1 still renders page0.
Published API confirms private keyed selection, no public selection-change/control
callback and nonpublic Page/Group render. Search has no explicit disable builder.
Unconditional resizable host remains source fact, not independently interaction-tested.

This is a real public-API/product-preservation conflict, not completed adoption or
product-red evidence. Do not hide the search via cosmetic clipping or synchronize
domain page state through speculative render-time side effects. User decision is
required before a narrow local0.7.1 extension or an official Sidebar/Form public
composition exception; dependency/upstream changes were not previously authorized.
No production replacement, launcher change, service/native/Release or data access.
Only the focused capability tests have run; full Desktop/lint/build/native/render
verification is not yet run for this continuation. Form/InputState/picker/save and
Tooltip catch-up remain planned pending the coherent integration choice.

## Official Settings / command Form adoption — 2026-10-11
User clarifies that preserving existing functions does NOT forbid official built-in
search/presentation features; explicitly requests actual official Settings because
old settings UI is rejected. This supersedes the capability-gate product-behavior
hold above. No library extension/fork/dependency change: adopt released0.7.1 directly.
Canonical phase remains3.Quick Find; launcher search/native preview unchanged.

Production adoption:
- render_quick_find builds real Settings/5SettingPages/SettingGroups/SettingItems/
  SettingFields. Released search/filter/sidebar resize/selected-page scrolling own
  presentation. Fill group surface maps through group_box theme token to existing
  neutral surface; no hand-painted substitute or geometry override. resettable(false)
  prevents inventing reset/save behavior. Old navigation/header/row/group/scroll
  animation production path removed; historical scroll helper only test-compiled.
- Ordinary titles are official SettingItem, existing controls use public field
  render slots. Volume/path/command/diagnostic are distinct custom-content items,
  not the entire old page embedded in one item. Per-item renderer projects existing
  application state/callbacks; diagnostics constructs only for its requested page.
- Official private selection is NOT mirrored by render-time state mutation. Existing
  quick_launch_settings_page is the initial-page seed for fixtures, not active
  selection. Index Up/Down/Escape/Delete/Backspace belongs to mounted exclusion
  content's own FocusHandle/listener; search/edit/other pages cannot invoke it.
- Original section saves/native hotkey/apply/pickers/command drafts remain; five
  original InputState entities now live in actual2column Form/Fields with program
  picker/full-row flags/footer. Removed old action/Checkbox type/height overrides.
  Detail scroll remains bounded; minimum920x480 actual wheel reaches Save fully.
- Missed path-exclusion description now real description_help Tooltip. No Downloads,
  Clipboard/global setting migration pulled forward; no native/service/user-data op.

Actual causal evidence and checks:
- Initial exact-release capability2/2; production adoption first2red (no search,
  missing Form),then4/4green. Capability tests are not product-red substitutes.
- First integrated537pass/12fail: retired selectors/type/source-boundary assertions,
  wrong exclusion focus scope and test counting all owner notifications as module
  transitions. Fixed scope at same mounted focus/listener node; observer counts
  actual module changes. Original callbacks/picker/cancel/stale/disk checks retained.
- Official navigation uses actual released0-index observation nodes; current-page
  Group registrations avoid stale debug_bounds. Five pages, long20path scrolling,
  diagnostic detached actions, hotkey lifecycle, master/edit/picker/save, minimum/
  long slots, three Switch pointer/Space/disk/typed one-command gates pass.
- Production search: keyword selects Commands, unmatched unmounts all content,
  log keyword selects Diagnostics, clear retains that page. No private API access.
- Group color regression genuinely red before token mapping; initial Rgba->token
  compile mistake is not red proof. Minimum Save red at495.5bottom vs480 viewport
  came from wheel outside hit region; actual form-area wheel passes unchanged
  containment. Screenshot minimum-scrolled confirms Save visible; no client inflation.
- Final frozen full549/549,0failed/ignored (quick-official-final-full3.log),strict
  tests/prodClippy/workspace --tests/fmt/Debug build exit0. Logs quick-official-final-*.
  All Cargo serial jobs1/incremental0/codegen-units256; no warning suppressions.
- Seven source gates0 after exact obsolete settings anchors replaced with official
  hierarchy/focus and actual callback boundaries; native/worker/persistence gates
  retained. Final2-gate logs; preview isolation/diff checks0.

Final isolated96DPI capture:6owned windows PID42484,7PNGs (all5topics plus920x480
command default/scrolled);1060x650/920x480 client readbacks confirmed. Synthetic
commands/exclusions, empty-root scratch catalog and disconnected bridge endpoints;
no ordinary runtime. Actual Browser gallery screenshot visible and inspected:
official search/navigation/groups, neutral Fill surface, bounded command fields,
minimum scrolled Save fully visible. artifacts/ui-quick-find-settings-kit/index.html
and capture-evidence.json; owned preview cleaned,0build/preview processes remain.

New settings aesthetic user approval pending; physical input/high-mixedDPI/reader/
installed acceptance not run. Passive native fixtures not rerun because controllers/
preview rendering/focus unchanged. Broader Quick Find business row/diagnostic/volume/
path chrome and attached-file-dialog search remain audit items, NOT blanket completion.
No Release/package/install/service/commit/push/delegation or user history/index reads.

## Ordered AI Task Queue / consolidated acceptance — 2026-10-11
User requests pi-auto queue for remaining UI replacements, with genuinely ambiguous
items retained and explained in final report, and one consolidated screenshot
acceptance at the end. Recorded shared execution/defer/verification constraints and
ordered12task brief in ui-queue.md. Queue entries added to this same session using
the registered task tool; all pending/draft, requiresConfirmation=true, no task
started by this queue-preparation turn. User confirmation in AI Task Queue panel
authorizes sequential plugin-dispatched tasks, not automatic Release/install/service.
After queue confirmation no per-surface aesthetic pause is required; retain scoped
adoption/functional evidence and consolidate user visual acceptance in task12.
Canonical stage titles/order remain unchanged. Preserve unclear narrow boundaries,
record exact API/behavior evidence/alternatives/decision needed in ui-queue-outcomes.md,
continue independent clear work. Actual compile/behavior regressions are not
ambiguity and must be fixed or reported as a real blocker; plugin failures pause.
Final outputs: docs/audits/gpui-kit-ui-queue-final.md and
artifacts/ui-kit-final-acceptance/index.html with actual isolated production PNGs.
Already implemented official Quick Settings/Form and approved launcher are not redone.
This preparation changed task documentation only; no Cargo/product/preview run.

## Queue01 — remaining 3. Quick Find presentation
Single-agent/no-delegation scope from ui-queue.md, complete index/latest ledger/
reinspection/reference and specs reviewed first. New scratch quick-queue01-before
preserves current dirty sources/status/diff/hashes, not pre-Settings rollback.
Created ui-queue-outcomes.md as cumulative record; item01 has exact release/public
selection, real changes/checks, bounded40px deletion adaptation and acceptance limits.
Actual production InputGroup/addons, stateless ListItem3business families and six
DescriptionList metrics; original native/input/IME/generation/keyboard/picker/save/
hotkey owners, official Settings/Form and approved launcher preserved. No ListState/
CommandState or second dispatcher. Removed old row selection stripes and button
geometry/type/paint overrides; domain fixed-density copy rails remain explicit.

Checks in scratch f32f826f-f318-49e4-a4bf-509b7d001dcd:
- quick-queue01-red.log0/4fail exit101: real absent compound/row and old metric layout.
  Green1 invalid debug_selector APIs were compile mistakes, not red evidence.
  Direct published DescriptionList comparison replaces guessed20px label offset.
- full-first550pass/3fail retired presentation assertions, replaced by actual Kit
  semantic selected nodes/Small24px/group snapshots, original functions retained.
- full-third549pass/4fail caught real default26px metric overflow and icon Large32
  not40px. Kit text_sm natural metadata lines fit48px; public Size40 preserves
  original delete hit zone, Kit state/radius/content unchanged. No weakened bounds.
- Two-path divider fixture red0/1 (quick-queue01-divider.log): bottom border consumed
 1px interior and clipped40px deletion. Absolute passive divider preserves40px
 ListItem/Button containment with no new dispatcher; final unchanged bounds green.
- quick-queue01-frozen-full.log553/553,0failed/ignored exit0. Narrow/default attached
 288/500x36/300, pending addon current frame, stable entity/edit/focus; main/minimum/
 long, all original Quick lifecycle/picker/keyboard/persistence/native source tests.
- Final strict tests/prod Clippy,workspace --tests,fmt serial resource variables;
 quick-queue01-frozen-{clippy-tests,clippy-production,workspace,fmt}.log and seven
 final gate logs exit0. Documentation spec/task/diff gates0 with no truncation warning.
 Gates quick-launch-lifecycle,ui-preview-isolation,ui-consistency,ui-state-lifecycle,
 ui-token-boundaries,desktop-shell-lifecycle,file-dialog-search-visual. Adoption
 guards supplement behavioral/native ownership checks, never replace them.

No genuine unresolved task01 interface/behavior/budget decision. Named40px delete
and retained domain density/native/recorder boundaries carried into final report.
Shared empty/loading/error/support remains queue10. Requested slice done,phase3 and
whole product visual acceptance not declared complete. No task01 screenshot acquired/
inspected, standalone executable build/start, physical/high-mixedDPI/reader/installed,
Release/package/install/service, user data, dependency change, commit or push.
Final gallery/user review remains queue12; no per-surface approval requested.

## Queue02 — 2. Downloads settings migration
Read queue/index/latest01ledger/results/PRD/design/specs and ABDM first then legacy
full SettingsWindow constructor/animation/native Show/Close/registry/Apply/store chain.
New downloads-queue02-before dirty source/status/diff/hash baseline preserved.
Actual released Settings5Page/10Group/39Item with public per-field/content slots;
category Form4Fields, request Form2Fields and original styled120pxTextarea. No
reset/automatic save/second controller/shared label rewrite. Old nav/section/matrix/
scroll presentation removed. zero-call old multiline helper/import removed only;
single-line field_shell still enables Kit appearance, no false source-hit claim.
Original984x650native shell/48footer/all fields/actions remain;96pxbounded visible
error scroll, original draft/save/picker/FFmpeg/overlay/native boundaries preserved.
Detailed item02 and final remaining boundaries in ui-queue-outcomes.md.

Evidence in session scratch:
- downloads-queue02-red2.log0/2fail exit101 actual search absent/duplicate textarea
  geometry; initial red compile mutability error not causal evidence. green1 2/2.
- full-first547/8fail retired selectors; tests updated to real public menu/current
  frame/scroll/bounds, all persistence/functional assertions retained. GroupBox
  itself not observed; business content under actual group path is test boundary.
- settings3/4/5 causal failures: asynchronous fixture viewport, offscreen category
  ordering/Tooltip action, exclusive contains right edge, corrected without native
  size/control/function changes. Test Button disabled flag unavailable; actual busy
  no-save/no-close pointer checks pass. Official no-match query prefix changes page;
  direct log→clear preserves diagnostics, no-match→clear remounts current content.
- error-red3 actual missing bounded96pxerror viewport; final containment green.
- downloads-queue02-frozen-full.log **556/556**,0failed/ignored exit0. Frozen
  clippy-tests/clippy-production/workspace/fmt logs all0; serial jobs1/incremental0/
  codegen-units256. Full other fixed surfaces unchanged. No warning suppression.
- Six downloads-queue02-final-gate-*.log exit0: download-ui-lifecycle,desktop-shell,
  ui-consistency,ui-state,ui-token,ui-preview-isolation; source gates supplement real
  tests, not native certification. Spec/task/diff documentation validation0.
- current_draft/save/toggle_taskbar_overlay_edit/sync_taskbar_overlay_edit bodies
  compare equal to02baseline; settings WindowOptions/native close/engine/C# unchanged.

Requested02slice complete;2.Downloads continues,not entire phase acceptance. No
genuine unresolved02user decision. Category master/custom business/state content
not blanket migrated from Settings; remaining consumer audit stays10/11. No PNG
acquired/inspected or standalone Debug exe build/start,physical/high-mixedDPI/reader/
installed,Release/package/install/service/user data/delegation/commit/push. Final
gallery/user review stays12, no per-page aesthetic confirmation pause.

## Queue03 — 2. Downloads main workspace
Explicit InputGroup/addon/clear,header+batch Toolbar/Group,all5TabBar/Tabs,ordinary
aggregate Progress and overview DescriptionList substitutions verified. Original
1060x650/min920x480/30tab/176dock/36batch budgets and data/selection/search/sort/
queue/context/deferred/native owners remain. Domain segmented strip/dots unchanged.
Read queue/fullindex/latest02/R03R08R09/specs; ABDM first then oldNxTools source chain.
Baseline downloads-queue03-before retains dirty source/status/diff/hashes.

Actual DataTable capability experiments: built-in Ctrl click selects one row index;
public render_tr with row_selectable(false) retains external ID set but release
overwrites row aria_selected. Actual sort-icon invokes perform_sort,secondary gesture
invokes context_menu. Static Table lacks virtualization. Only production virtual
grid substitution deferred; proposed narrow exception/upstream public-hook alternative
and user decision recorded in ui-queue-outcomes.md03/final pending list. No claim
that business ownership itself prevents adoption; untested child/Shift/OLE adaptations
are not certified. Requested03outcome **partial**, whole Downloads continues.

True red missing InputGroup (red.log1pass/1fail); metadata-red2.log3pass/1fail old16px
label vs22.5natural DescriptionList. Intermediate API/syntax compile errors are not
red. Toolbar/Progress lack autoTestSupport: actual descendants/focus wrap/painted
quads replace unsupported role observations. Attempts at direct RenderOnce semantics
failed; TestWindow a11y treeNone; ineffective test probes removed. Native provider
remains unverified, no fake semantic snapshot. Actual scaled painted fill0/37.5/
clamped100percent passes. Old button selectors/28input/29tab/5aggregate expectations
updated to true Kit IDs/32input/SmallTab<=30/4aggregate; functions/bounds retained.

Frozen logs in session f32f826f-f318-49e4-a4bf-509b7d001dcd:
- downloads-queue03-frozen-full.log **564/564**,0failed/ignored exit0;8new real tests.
- frozen-clippy-tests/clippy-production/workspace/fmt logs all0; serial Rust jobs1,
  incremental0,codegen-units256. No standalone executable build or launch.
- final2-gate-{download-ui-lifecycle,desktop-shell-lifecycle,ui-consistency,
  ui-state-lifecycle,ui-token-boundaries,ui-preview-isolation}.log all0. Added actual
  component guards alongside original native/worker guards, not release evidence.
-20production bodies equal03dirty baseline:search/sort/filter/selection/context,
  five content renderers/domain segments/header masking/persisted sort. diff check0,
  no Cargo/rustc survivors. Documentation spec/task checks recorded at closeout.

No PNG acquired/inspected,physical/highmixedDPI/reader/installed/native drag/service/
Release/package/user-data/delegation/commit/push. Isolated temp/memory/disconnected/
queue-only synthetic.invalid data only. Final consolidated production gallery12.
Closeout docs: downloads-queue03-final-spec.log87documents/12owners exit0;
final-task.log all context entries valid exit0;git diff --check0,temporary DEBUG/
ProgressProbe matches0 and Cargo/rustc count0. No final visual acceptance implied.

## Queue04 — 2. Downloads new-download Form/Radio/Popover
Read queue/fullindex/latest03/specs;ABDM AddDownloadPage/BaseOptionDialog before old
AddDownloadWindow constructor/animation/show-close/picker/viewmodel parse/category/
Submit/HeaderCookie;native Avalonia base unavailable. Baseline downloads-queue04-before.
Actual per-field Form/Field on original32InputStates;horizontalURL/directory/two-column
filename/category,original120inspector/44footer/fixed176->fresh278/media353/418/error456.
KitstyledPopover sole frame,schedule304x240/publicp_1/category320x240defaultpadding;
no appearance(false)/manualshell. Category statelessListItem/original search/keyboard/
scroll/selection/focus. HLS/DASH RadioGroup/SmallRadio40px;publicStatefulon_click sole
callback for optionalaudio repeat-clear,explicitkeydown/up once verified. No second
focus/action/controller/save/native owner. Zero-call old labels/popover/radio/dot/color
helpers removed,live field/picker/error/compactlist unchanged. Exact evidence outcomes04.

Only date-grid delayed_start_calendar deferred:actualreleasedSmallCalendar exceeds
156pxinlinebudget;datepicker publicchange/date/open chain reviewed,scroll/nestedpublic
integration unverified. Preserve existing compact date grid insideNEWofficialPopover;
recommend narrowexception or separately test changednavigation/outerdraft/focus/240px
gates.User decision in finalpendinglist,not whole04completion or globalAPIrejection.

Realred2.log1pass/2fail old16pxlabel/noRadio;initialAPIcompilemistakes NOTred. First
compiledfull562/5fail fourretiredselectors+realscheduleoverflow;actualForm/Radio snapshots
andKitdefaultinsets replace appearance-only assertions while typedmedia/audio cancel/
allfields/footerbounds remain. Publiccompactp_1/Small24actions repairedschedulewithin240.
Keyboardtrial failed because missingKeyUp;truefocus+explicitdown/up passes unchanged
production,notunsupportedcapability. Strictprod lint zero-callhelpers removed without
warning suppression;newHeaderfixture tuple corrected totypedDownloadRequestHeader.

Final logs session f32f826f-f318-49e4-a4bf-509b7d001dcd:
-downloads-queue04-final2-frozen-full.log **568/568**,0failed/ignored exit0.
-final2-frozen-clippy-tests/clippy-production/workspace/fmt logs all0;serialjobs1,
 incremental0,codegen-units256. Fournewproduction/APIregressions.
-final2-gate-download-ui-lifecycle/desktop-shell-lifecycle/ui-consistency/ui-state-
 lifecycle/ui-token-boundaries/ui-preview-isolation logs all0. Isolation nowtraces
 helperpending->actualInput+pasteButtondisabled,keepshiddendisplay/nativeguards.
-15keyproductionbodies whitespace-normalizedequal04baseline:probe/submit/request/
 draft/hydrate/media/sizes/dategrid/inspector. Browserhandoff/C#/engine/optionsunchanged.
-Realboundedlocalhostbegin_probe ->freshfixed278/sourceclosed/formfooterpasses;
 failure/disconnect/timeout/presentationretrywithoutreprobe/stale/pendinglocks/media/
 FFmpeg/queue/cancel/category32scroll/Tooltip/focus/Chineseentities/typedHeaderCookie
 remaincovered. TestWindow is not actualHWND certification.
No04PNG/standaloneDebugbuild/start/nativehandoff/physical/highmixedDPI/reader/installed/
Release/service/userdata/delegation/commit/push.Finalconsolidatedgallery12.
Closeout: downloads-queue04-final-spec.log87docs/12owners exit0;final-task.log context
validexit0;diffcheck0;temporaryprobes0;Cargo/rustc0. No visual/native acceptance implied.

## Queue05 — 2. Downloads supporting native-window content
Read queue/fullindex/current04ledger/results/specs and ABDM Desktop chains before old
NxTools constructors/baseOpenedClosed/animation/result/ShowCompletionState references;
Avalonia native source unavailable. Dirty downloads-queue05-before preserved.
Actual schedule Switch/independentToggle/per-fieldFormField/original32Input/StatusBar/
DialogContentFooter; task XSmall4Progress/SmallDescriptionList/ToolbarGroupCheckbox/
completedfailedDialogFooter/XSmalldisclosure; shutdowndelete inner DialogContent/
XSmallAlert/Footer/styledButton and full risk wording. Deletion production menu->
request_delete_confirmation->keyed380x178 is LIVE; removed stale dead_code annotations.
No modal host/library/controller/Windows/persistence change.22key bodies equal baseline,
two controller files bytes equal;native sizes/owners/OLE/domain dots unchanged.

Only TimeField replacement deferred: exact0.7.1 NaiveTime/segmented invalid-digit
restart cannot directly retain raw invalid/intermediateHH:MM for original Apply error.
Real focused9,9edit confirms;keep actualstyledInput,record options/recommendation in
ui-queue-outcomes.md05/finalpendinglist. Explicit substitutions verified,05partial,
not whole phase or visual/native acceptance. Shared custom state catch-up remains10.

True red3.log1pass/2fail (Switch0/2,Progress5/4),risk-red2Alert layout red,form-red.log
old18vs22.5Formlabel red. Intermediate APIcompilemistakes not red. First compiledfull
566/5obsolete presentation sizes; updated real official tiers,functions/bounds retained.
Checkbox pointer no-focus source confirmed;failed unfocusedSpace fixture repaired with
actualTab/currentframe,production unchanged. Focused control/budget tests green.
downloads-queue05-frozen-full.log **571/571**,0failed/ignored exit0;frozen strict tests/
prodClippy/workspace/fmt all0,serialjobs1/incremental0/codegen-units256. Six finalgate
logs0:download-ui/desktop-shell/ui-consistency/ui-state/ui-token/ui-preview-isolation.
final-compare.log22bodies whitespace-normalizedequal/two controller bytes equal.
No PNG/standalonebuild/start/nativeactualHWND/physical/DPI/reader/installed/Release/
service/userdata/delegation/commit/push. Final gallery12,not per-window aesthetic pause.
Closeout: downloads-queue05-final-spec.log87docs/12owners exit0;final-task.log context
validexit0;final-diff.log0;temporaryprobes0;Cargo/rustc0. Pending03/04/05 retained.

## Queue06 — 4. Clipboard + Tools / Clipboard Settings
Read complete queue/index/latest05/spec/reference/task;old settings/viewmodel/section
save/history/repository/hotkey/popupconstructor/show-hide/singlepassive native chain.
Avalonianativebase unavailable;legacy CreateBackup notfreshlylocated,Nextexisting
archive/migration preserved independently. Baseline clipboard-queue06-before.
ActualSettings3Page3Group13Item/11Field render+2individualFormFieldItems,official
search/filter/sidebarresize/selection/scroll/resettablefalse. Removed old workbench
hierarchy/scroll/manualordinarychrome;styleddefaultrecorder/menuButton/Smallstepand
managementButtons on originalcallback/savepolicy. NumberInputeditablef64Change would
add policy over existingnoneditablesteps;reviewed/publicstyledcomposition retained.
Inlinevalidation/risk/Windowsstatus/path/fulloptionalTooltip;worker/recorder/native/
notification/dataowners unchanged. Requested06settingssliceverified,whole4ongoing.
No newlyunresolved06decision,earlier03/04/05pending kept,07/08/10/12notpulledforward.

True red2.log0/2officialsearchmissing;initialAPIcompile/errorlineedits notred. First
full576/2retiredsurface/groupselectors;updatedrealpublic0-indexnavigation/Root/current
controlbounds,originalToolassertions kept. Menuinitialselection/Tooltipnoautoregistration
anddelay/unfocusedSpacefixtureissues fixed withoutproductioncontrollercompensation.
Final-frozen-focused9/9/full **580/580**,0failedignoredexit0;strict tests/prodClippy/
workspace/fmtall0,serialjobs1incremental0codegenunits256. Sevenfinalsourcegates0.
11keyhelper/recording/projectionbodies equal06baseline;ClipboardApplication/worker/
MainWindowbytes equal. Newproductioncomponenttests use tempSQLite/config and fakepicker
cancel/error only,no newarchive/backup/compact/migrationrun or realuserconfiguration/
Clipboard/history. Minimumscrolltomigrate/longpath/error/lastaction/Space release/disk/
root/typedConfigure/recorderCancel/Tooltip/originalnotificationprogress pass.
NoPNG/standalonebuildstart/nativeactualHWND/physical/DPI/reader/installed/Release/
service/delegation/commit/push.Finalconsolidatedgallery12,notwholefamilyacceptance.
Closeout: finalspec87docs12owners/taskcontext/diff0;Cargo/rustc0. Additional shared
recorder quick-launch-lifecycle finalgate0,total8sourcegates. No whole4/visualacceptance.

## Queue07 — 4. Clipboard + Tools / search, editors and navigation
Read fullqueue/officialindex/latest06/task/spec/old Clipboard search/category/text/
constructor/show-hide/passive-native chain. Avalonia native base unavailable; Next
customcategories/sourceassignment/extendedsearch existing behavior preserved. Dirty
clipboard-queue07-before retained. ui/mod.rs actual InputGroup/Search/InlineEnd scope
InputGroupButton on originalInputState;verticalSmallFormField/defaultstyledTextarea,
horizontalSmallFormField/defaultstyledcategoryInput;sourceSmallCheckbox and styled
SmallSaveCancel/undoButtons. Removed ordinary duplicate frames/appearance suppression.
Original300x440/defaultuserresize/244categoryeditor/sourcebounded/fullhelp/occlusion/
SaveCancelEscape/undo/focus and passivepreview/workerpaging/cache/controllers unchanged.

Navigation only deferred: real XSmallunderlineTab pointeronce/<=28px works, directTab
and tested publictrack_focus+tab_stop recipe keyboard target/action absent. Focused
checkedToggleGroup explicitSpace groupcallback0. Other public adapters/full drag/drop/
context/wheel combination unverified, not globally impossible. Original category_tab/
order/drag/drop/wheel/rightclick/selection/ScrollHandle/one focusable_content retained.
Recommendation/options/finaluserdecision recorded outcomes07;no per-surface pause.
Explicit substitutions verified,07partial,notwhole4orofficialnavigation certification.

True red2.log1pass/2fail missingInputGroup/category238vs256duplicateinset;initialAPI/
delimitercompilemistakes notred. Initial actual tempworker save ItemNotFound traced
to fixture seed before startup legacy reset;seedafterinitialization/bounded200x10ms
terminalwait repairs fixture only,notproductionreset/save. Tennew production/APItests
include Chineseedits/queryscopeclear/entitygenerations,temptext/categorysourcesave,
occlusion/focus,80line/default/minimumresize/Cancelidentity and longnavwheel/rightmenu.
clipboard-queue07-frozen-focused.log10/10/full.log **590/590**,0failedignoredexit0;
frozen-clippy-tests/clippy-production/workspace/fmt all0;serial jobs1/incremental0/
codegenunits256. No standaloneexe build/start. Eight final-gate logs all0: Clipboard
ui/lifecycle,desktop-shell,ui-consistency/state/token/preview-isolation,quick-launch.
Finalcompare27original bodies+ClipboardPanelconstructor whitespace-normalizedequal;
Application/worker/oldUItests bytes equal. Comparison initially had two new symbols,
scopedconstructor toactualowner,no productchange. Presentation helper changed intentionally.
NoPNG/nativeactualHWND/physical/DPI/reader/installed/Release/service/userdata/delegation/
commit/push. Earlier03/04/05decisions retained;historyTools08/sharedstates10/gallery12
notpulledforward. Documentation/process/probe closeout recorded after checks.
Closeout: clipboard-queue07-final-spec.log87docs12owners/taskcontext8implement6check
validexit0/final-diff0(existing unrelatedCRLF warnings only);temporaryprobes0;
Cargo/rustc0. No Rust edit after frozen verification;no visual/native acceptance.

## Queue08 — 4. Clipboard + Tools / history, Tools workspace and Link
Read fullqueue/index/latest07/task/spec/references and legacy historyliteralrow/menu/
popupconstructor/show-hide/passive native chain. Link is Next existing behavior;
native bounds/show/reuse/close-before-route/timer inspected, not legacy equivalent.
Baseline clipboard-queue08-before retains15dirtysources/diff/status/hashes.
Actual history statelessListItem/Toolbarcontent/XSmallButtons/literalLabel metadata;
popupEmptyHeaderMediaTitleDescription/XSmallAlert-boundederror,originalstateconditions.
Removedzero-call compact_result_row/constants/imports,livecompact_list_row unchanged.
ToolsworkspaceListItemLabelSmallTag/originalfocusable dispatcher,completedFormNumberInput
Select settings unchanged. Link statelessListItemLabel/Small expander/StatusBar slots;
420width/28row/24expander/22footer/60single84collapsed116expanded unchanged. Original
projectmenu/passivepreview/pagingcache/ID/action/modifier/close-defer/timer/native owners.
List same-height/fullstate APIs and DescriptionList/Tag publicslots reviewed; no
competingcontroller or inventedhistorylabels/classifications. No new08decision;
earlier03/04/05/07 retained, sharedstates10/gallery12 not pulled forward.

True red.log0/2 actualListItem missing;state-red.log0/1 Emptymissing;metadata-red2.log
0/1old18vs20. Metadata firstfixturestretch1080/importsyntax/APIclockborrow errors not
red. AltdefaultPlainText/ControlCopyOnly corrected fixture only. Link refresh not
removalproof: actualAppwindowcount plus TestPlatformcopy and registeredtimerclock used,
productionunchanged. Fullfirst597/1retiredEmpty selectors/geometry;updatedrealcomponent
content observations, kept focus/viewport/loading/error/category and native budgets.
Frozen focused8/8/full **598/598**,0failedignored exit0;strict tests/prodClippy/workspace/
fmt all0 under serialjobs1incremental0codegenunits256. Eight finalsourcegates0.
46keyoriginal bodies equalbaseline/Applicationworker/projectmenu/preview bytes equal;
originalLinkrow+expander callbacks/menuactionprojection equal,Toolsettingstailexactequal.
NoPNG/actualHWND/physical/DPI/reader/installed/standalonebuildstart/Release/service/
userdata/fork/delegation/commitpush. Requested08slice done,notwhole4/finalacceptance.
Closeout: clipboard-queue08-final-spec87docs12owners/taskcontext8implement6check valid;
diff/compare0/probes0/Cargo-rustc0. Newqualitysummary links detailed08evidence within
32KiB injection budget;originalguidelines preserved. No Rust edit after frozenchecks.

## Queue09 — 5. Global settings + supporting surfaces / Global Settings/rules
Completequeue/index/latest08/task/spec/oldglobal constructor/animation/OpenClose/
ApplyCancel sourcechain read;ABDMDesktopproxy first then oldNxTools,noPAC/auth import.
Dirty global-queue09-before10files/statusdiffhashes retained. ActualSettings3Page3Group
7individualItems/3ordinaryFieldrender+4distinctrenders,twoSmallFormField proxy/rules;
officialSidebar/searchfilterresize/scroll/resetfalse,no activepage mirror. RealSwitch/
Checkbox requestedbool once/SmallruleButtons replace ordinaryBase+div/handpaintedmark;
actualTooltip optionalhelp/visiblepolicynote/error80scroll/busyoccluder. Original720x480/
input>=200/right<=700/table<=202 budgets,local12inset actualoverflowrepair/no inflation.
OriginalatomicSave/desired!=Windowsapplied/nextturnconsumers/casepathEXE/localdraft
policies unchanged. Zero-callworkbenchrowsection removed,livehelpers equal. Shared
field_shell alreadyreenables appearance,falsecallerflags redundant not runtimefalse.
Proxy originalactualstyledButtonGroup retained:directreleasedToggleGroup3mode recipe
allowsallfalse/focusedgroupkeycallback0;otherpublicadapters unverified,notimpossible.
VerifiedBGpointer/Space/repeat/busyexclusivepolicy;no new09decision or page exception.

True red2missingSettings0/1;initialRoleAPIcompileerror notred. Firstnetwork12/5oldnav/
section/mark/32footer observations;updatedactualofficialnodes/Checkbox/24Buttons while
keepingall domain/savecancel/native/consumer gates. Subsequent16/1realrightoverflow
fixedpubliccomposition. Searchmulti-stringkeystroke/ownedID/DivID/anchor errors fixture/
API corrected. Tooltipinitialhovercandidateframe diagnosed:outside+refresh/hover+frame/
700msactualcontent green withoutproductioncompensation. Focused8/8/finalfull **606/606**,
0failedignoredexit0;finalstrict testsprodClippy/workspace/fmt all0,serialjobs1incremental0
codegenunits256. Ninefinalsourcegates0;22originalbodies equal/6ownerfiles bytes equal.
NoPNG/nativeactualHWND/physical/DPI/reader/installed/buildstart/Release/service/userdata/
fork/delegation/commitpush. Requested09done,whole5/sharedsupport10/gallery12notcomplete.
Closeout: global-queue09-final-spec87docs12owners/taskcontext8implement6check valid,
diff/compare0/probes0/Cargo-rustc0. Globalrulespec now matches actualKitordinarycontrols,
not retiredBase/legacytype requirements;no Rust edit after frozen606/strictchecks.

## Queue10 — 5. Global settings + supporting surfaces / shared support catch-up
Fullqueue/index/outcomes01-09/task/spec/reference reviewed; dirtysupport-queue10-before
preserved. ActualEmpty/EmptyContent/Label/XSmallAlert; loadingSmallSpinner under original
motionallowed/static reducedicon, never percentage. fieldinlineAlert; original field/picker
bodies equal. Notification actualAlert ORIGINAL copy/Labeldetails/boundedscroll/Small24action/
XSmall20close/tested28pxclose lane under original max3/336/max112 manager. DirectNotification
render/publicdismiss/on_close0 privatecompletion seam tested; only standalone recipe deferred,
not global incompatibility. Recommend originalmanager+actualpresentation, finaldecision10.
TaskremovedEmpty/failure40pxscrollAlert,back-to-topSmallButton/originalListState,FFmpeg
redundantflags/menuLabel+officialicon/recorderdefaultButton/nativeheaderSmallclose adopted.
Capture/native5states/DialogHWND/title36/OLE/passive/oneowner remain; no broadadapterrewrite.
Businesscontainer/nativecontent classification and remaining unverified custom typography
explicit in outcomes10; prior03/04/05/07 decisions preserved. Not whole5/final acceptance.

True red logs retained: sharedstates/action/task/menu/recorderaccessible/close/notification
overlap; fixture/API/path-onlyBaseButton observation excluded. Initialfull604/609,612/613,
614/615 obsoleteobservations fixed while lifecycle/budgets preserved; actual overlap repaired.
final-frozen-focused9/9; support-queue10-final2-frozen-full.log **615/615**,0failedignoredexit0;
final2-frozen-clippy-tests/clippy-production/workspace/fmt all0. Rustserialjobs1incremental0
codegenunits256. Zero-production-useLINE_HEIGHT_META now test-only, no lint suppression.
11final-gate logs all0; sourceguards supplement native/domain conditions. final-compare.log
40original bodies+4wholefiles equal10dirtybaseline; changedpresentation/latecopies excluded.
NoPNG/actualHWND/physical/DPI/reader/installed/standalonebuildstart/Release/service/realdata/
dependencyfork/delegation/commitpush. Finalgallery12/frozenreconciliation11notstartedhere.
Spec/task/diff/probe/process closeout recorded after running, not visual/native certification.
Closeout: finalspec87docs12owners/taskcontext8implement6check/diffcompare exit0;
quality32666bytes/no injectiontruncation;probes0/Cargo-rustc0. FrozenRust615/strict
checks unchanged during docscloseout. Requested10 settled with explicitnarrowdefer.

## Queue11 — frozen integration/adoption reconciliation — 2026-10-12
Read fullqueue/outcomes01-10/index/ledger/plan/specs/publicAPI and nativefixture safety.
Source/config/gates313SHA256 unchanged; no product/gatesource defects found or edited,
no assertions deleted/ignored, no deferred/aesthetic redesign. Draft finalreport at
docs/audits/gpui-kit-ui-queue-final.md records every itemconstructor/publicslot/function/
status/exception. Partial03/04/05/07/10 retained;02hierarchy/formverified does NOT certify
categorymaster compact_list_row adoption. That genuine rowgap has no testedListItem
incompatibility;separateauthorizedmigration/explicitretention decision,not nativeexemption.
Passive28pxslots/Small24Buttons andfield_shell reenabled flags exclude textualfalsehits.

queue11-frozen full615/615/0ignored,strict testsprodClippy/workspace--tests/fmt--check/
Debugbuild exit0;complete logs/rust-checks.json commands/start-end/exits/resources.
Serialjobs1incremental0codegenunits256/21600timeout;no duplicateCargo.14affectedsource
gates0,old3PNGoutline0(not newcapture),official78/47/15sets+173pathsPASS,queue10owner
40bodies4filescomparePASS. HarnessDebug0warnings0errors;exactfiltered ownednative All
Quick2/2+Clipboard2/2 RUN/PASS/noSKIP exit0,96DPI400x410/460x406/passive3primary1/UIA/
reuseforeground. Ownedprocesscleanup;no unfilteredDOpus/realdrag/service/registryop.
No newPNG/physical/highmixedDPI/fullreader/installed or wholemigrationacceptance.
Onlysafe syntheticpreview ran;noordinaryruntime/Release/package/install/realuserdata/
dependencyfork/delegation/commit. Verification11settled,productpartial/gallery12pending.
Document/task/link/diff/freeze/process closeout recorded after running checks.
Closeout: finalspec87docs12owners/taskcontext8implement6check/local-links29in8docs
0missing/index78/47/15+173paths/diff/freeze313unchanged all0. DebugexeSHA256saved;
exactnative4RUN4PASS/noSKIP/buildfixture survivors0. Productsources frozen;onlydocsedit.

## Queue12 — unified production gallery/report — 2026-10-12
Fulltask/results/ledger/frozen11 readfirst;initialsource/exehashesmatched. Missingvisual
fixtures added onlyDebugstartupintercept/finalgallery/DownloadsGlobalClipboardsynthetic
projection/moduleisolationguard,7expectedpaths;ordinarybusiness/native/deps unchanged.
ScratchUUIDworkerSQLite/memoryenqueueonlyengine/droppedbridges/invalidbusinessHWND;
capturechildPATHFFMPEG limitedscratch/no realtools. NoSave/probe/submit/nativebusiness op.
DebugDownloadsfault defaultthreadstackoverflow:onlyisolatedfinalgallery32MiBthreadgreen,
notReleasebugclaim. Oldincomplete/collision/UTF8script errors preserved/notproductreds.
Finalfixtureisolation1/1/full616/616/0ignored/strict testsprodClippy/workspace/fmt/Debug
build logsqueue12-final all0/serialjobs1incremental0codegenunits256/21600s.14sourcegates0;
latestexactsafeownednativeAllQuick2/2Clipboard2/2 RUNPASS/noSKIP/0,96DPImessages/UIA only.
50finalPNG/pixelclientDPI/exeimagehash verified,onebuildSHA256FA591A03984A6977AA9774F832DDBD7CFBF231F6008EE1131D1F679A5B1B69F5.
BrowserPreview/browserattachments actuallyviewedall8groups+3extraimages;50/50inspection,
NOT userapproval. Singlegallery artifacts/ui-kit-final-acceptance/index.html;report
docs/audits/gpui-kit-ui-queue-final.md updated;outcomes12/ledgeraligned. Missingstates/
partialcoverage/GlobalduplicateSwitchlabel/innerlistscrolllimit explicit. FiveAPIdecisions/
categorymasterrowgapremain. Awaitconsolidateduservisualfeedback,notwholeacceptance.
Allownedpreviews/nativefixturescleaned;noDPIchange/physical/readerinstalled/package/
service/realuserdata/delegationcommit. Docs/task/link/diff/evidence closeout follows.
Closeout:evidence50uniquePNG/50inspected/102resources/clientDPIhashPASS;5existingDebug+
2newfixturepaths only/uiMODoriginalequal. Spec87docs12owners/task8+6/42links0missing/
diff0/buildpreview survivors0. NoRustsourcechangesafterfinal616/strict/build/native.

## Post-gallery feedback implementation — 2026-10-12
Latest user amendment is implementation authorization,not queue13/stage renumber.
See docs/audits/gpui-kit-ui-feedback.md for exact changes/reds/checks/limitations.
TimeFieldH23minute/originalApply validation, categorymasterListItem andactualClipboard
Tab/onefocusclick+drag/Small24pxsearch implemented. Originalnativeclients/controllers/
save owners retained;user supersedes daily raw-invalidtextonly. Taskgrid retained.
Calendar tiers/padding measured;Notificationon_close requirement corrected to intent.
GlobaloldPNG actuallyinspected:duplicateSwitchlabel corrected with semanticname.
Final623/623/0ignored,strictprodtests/workspace/fmt/Debugbuild/14sourcegates0;
exact safeownednativeQuick2/2Clipboard2/2 noSKIP. Fullinitial622/623 failures resolved;
compile/API/observer mistakes excluded from behaviorred. Logs ui-feedback-final4-*.
30newowned96DPIproductionPNG/sameexe6CAE729F.../pixels verified in ui-kit-feedback;
old50gallery kept unchanged. NewPNGinspection blocked by Browser calling-session
visibility;remaining targetedspacing/balance corrections andcompletekeyboarddrag/time
state matrix are not claimed complete. User must switch to this session's Browser
to continue actual visual review. No ordinaryruntime/Release/package/install/services/
user-data/delegation/commit/push;ownedpreview/native/build processescleaned.

## Browser-restored targeted visual balance — 2026-10-12
User switchedback;Browser30/30images loaded andlatest30PNG actuallyviewed in8bounded
groups. Observedreds:GlobalSwitchx668,DownloadSwitchx924,time-rowgap20px. Local
SettingFieldlanes360/440px (not widecontent), naturalwidthcontrols,Form4columns hostspan3
andportspan1,scheduleField32pxslots corrected. Intermediate332pxSwitchstretch fixed
without weakening36pxcontrol test. No type/palette/radius/nativeclient/controlowner/
businessmethod changes. ClipboardSmall24searchactualfocusring is Kitowned,not duplicate.
Final625/625/0ignored,strictprodtests/workspace/fmt/Debugbuild/14gates0;logsui-balance-final2-*.
Latest30PNG allsame7A40B129...exe/clientDPIpixelhashverified andactuallyinspected;
earlier30baseline inscratch,old50gallery unchanged. No ordinaryruntime/dataactions/
package/install/services/delegationcommit. Fullstate/physical/DPI/reader/userapproval
remainopen;Browserblock resolved. Taskstaysin_progress forwholemigrationacceptance.
