# GPUI Kit UI queue — consolidated production gallery and acceptance report

**Latest amendment:** [post-gallery feedback](gpui-kit-ui-feedback.md) implements
user-authorized TimeField/ListItem/Clipboard Tabs/24px search and removes Global
duplicate labels;latest625tests/strict/build/gates pass. Browserrestored:30newPNG
actuallyinspected afterDownloads/Globalcontrol-lane,proxyproportion/schedulerhythm
corrections. Historical50PNG below are not currentbuild;user/fullstateacceptance open.

**Queue12 delivery, 2026-10-12: 50 real production PNGs captured and inspected; visual acceptance awaits the user. Coverage remains partial, not whole-migration completion.**

## Authority and scope

- [Confirmed queue](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/ui-queue.md), [cumulative outcomes](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/ui-queue-outcomes.md), [adoption ledger](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/adoption-ledger.md), [execution evidence](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/implement.md).
- [Complete official index](../reference-code/gpui-kit-component-index.md), [locked release contracts](../reference-code/gpui-kit-071.md), [approved UI authority](../../.trellis/spec/shared/ui-design.md).
- Preserve canonical Foundation → Quick Find → Downloads → Clipboard + Tools → Global settings + supporting surfaces order. Approved launcher is not redesigned. Prior narrow decisions are not implemented by this reconciliation.
- One agent; no delegation, commit/reset, dependency/fork, Release/package/install, services/indexer/real-volume or ordinary business startup. Only temporary synthetic Desktop tests and explicitly owned native fixtures ran.
- No product source or architecture gate changed in queue11. A SHA256 manifest froze **313 Rust/C#/vendor/config/gate files**, including Cargo configuration/lockfile. It remained unchanged after checks. Existing broad dirty work is preserved.

## Frozen verification

Evidence root for this session: `C:\Users\Administrator\.pi-desktop\scratch\f32f826f-f318-49e4-a4bf-509b7d001dcd\queue11-frozen\`.
`rust-checks.json` records exact commands, start/end, exits and memory environment; logs are complete, not a stdout sample. All Cargo operations were serial, with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, and full-operation timeout21600s.

| Executed command | Actual result | Log |
| --- | --- | --- |
| `cargo test -p nxtools-desktop -- --test-threads=1` | **615 passed,0failed,0ignored**, exit0 | `full.log` |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | exit0 | `clippy-tests.log` |
| `cargo clippy -p nxtools-desktop -- -D warnings` | exit0 | `clippy-production.log` |
| `cargo check --workspace --tests` | exit0; compile check, not workspace test execution | `workspace.log` |
| `cargo fmt --all -- --check` | exit0; no formatting rewrite | `fmt.log` |
| `cargo build -p nxtools-desktop` | Debug executable build exit0, not Release | `debug-build.log` |

Fourteen affected **source-only** architecture gates passed, each exit0: ui-state-lifecycle, ui-consistency, ui-token-boundaries, ui-preview-isolation, ui-motion-policy, desktop-shell-lifecycle, clipboard-ui-lifecycle, clipboard-lifecycle, quick-launch-lifecycle, download-ui-lifecycle, file-dialog-search-visual, file-dialog-search-lifecycle, file-dialog-window-follow, taskbar-overlay-lifecycle. `gates.json` and individual `gate-*.log` retain exits. No package/service/indexer gate was executed as a product operation.

`quick-launch-search-outline.ps1` also passed on the **three historical PNGs** at620×44/410. This checks retained approved images, not newly captured frozen-build screenshots or agent visual inspection. Official-index verification matched captured Component78/Base47/Shell15 name+URL sets and173 exact-release source paths; no refetch/latest substitution. Queue10 owner comparison remains40 original bodies plus4 whole files equal to its captured dirty baseline; this is scoped preservation evidence, not equality of every changed presentation file.

## Safe native evidence

[Native fixture contract](../../tests/native/README.md) and [exact-filter wrapper](../../tests/native/gpui-kit-acceptance.ps1) were inspected with the Rust synthetic entry and C# owned-HWND tests before execution. Harness Debug build succeeded with0warnings/0errors. The preview entry intercepts before product startup, uses synthetic text and invalid application owners, and does not open real history/config or connect services.

| Owned mode | Exact RUN/PASS | Actual native evidence |
| --- | --- | --- |
| Quick, `-Check All` | **2/2**, both exit0, no skip | PID102044;96DPI,400×410 client; passive WM_MOUSEACTIVATE3, primary/policy-disabled1; show/hide/reapply sameHWND; foreground unchanged; synthetic UIA Edit/Button/focus capability |
| Clipboard, `-Check All` | **2/2**, both exit0, no skip | PID100464;96DPI,460×406 preview client; same message/policy/reuse/UIA checks |

The only C# RUN names were `owned GPUI passive preview rejects mouse activation` and `owned GPUI native client and UIA acceptance`. The wrapper rejects zero/expanded RUN sets and checks PID/executable/title before native operations. It cleaned only owned fixture processes. No unfiltered DOpus, real OLE drag, browser/Shell/shutdown, registry/service or installed operation ran. This evidence does **not** certify physical input, mixed/high DPI, whole-product screen reader, full native controller/IPC flow or installed acceptance. No PNG was captured by queue11.

## Queue01–10 reconciliation

Paths below are relative to `rust/apps/nxtools-desktop/src/`; exact release prefix `C:` is gpui-component0.7.1 and `B:` is gpui-base0.7.1. Public API use is backed by production composition and current full tests, not imports alone. Historical per-slice test counts remain in outcomes, not relabeled as current runs.

| Item / current status | Production constructors and public seam | Preserved function / boundary | Retained or incomplete scope |
| --- | --- | --- | --- |
| **01: requested slice verified** | `quick_launch/file_dialog_search.rs::render` InputGroup/InputGroupAddon; `ui/components/{volume_scope,path_list,custom_command_editor}.rs` ListItem selected/children; `quick_launch_diagnostics.rs::diagnostics_metric` DescriptionList::vertical/item. C:input/group.rs,list/list_item.rs,description_list.rs | Original InputState/IME/pending,36px attached rail/host clamp, volume auto/manual and exclusion keys, command edit-not-execute/picker/save, detached diagnostics; sole focus/action/deferred owners | Delete40×40 uses tested public custom Size; business rows40/48 and metadata rails remain. Launcher unchanged; no whole Quick Find visual certification |
| **02: requested hierarchy/form slice verified, category-master content not adopted** | `downloads/settings_view.rs::render` Settings/5Page/10Group/39Item, individual SettingField::render/SettingItem::render; category Form4Fields/request Form2Fields/original Textarea. C:setting/*,form/*,input/textarea.rs |984×650/footer48, Apply/Cancel/localdraft/validation/busy, all settings/request secret roundtrip, FFmpeg manual versus immediate managed save, taskbar/export/diagnostics; resetfalse | Category master still `compact_list_row`, explicitly excluded in original02. This is an adoption gap, not a verified API exception or entire Settings rejection; see remaining coverage below |
| **03: partial, grid deferred** | `main_window/download_page.rs::render_downloads` InputGroup, Toolbar/ToolbarGroup::content, `download_task_detail` Small underline TabBar/all5Tabs; `download_aggregate_progress` XSmallProgress; `download_detail_field` DescriptionList::horizontal | Six filters/counts/all columns/search/sort/stable-ID CtrlShift/context/batch/queue/detail original owners,1060×650/min920×480/36batch/30tab/176dock; domain segment strip retained | DataTable external selected semantics overwrite in tested public row recipe; original uniform_list retained pending decision03. Callback availability is not denied |
| **04: partial, date grid deferred** | `downloads/new_download_view.rs::render` Form/Field/original Inputs, actual styled Popover, stateless categoryListItem; SmallRadio/RadioGroup, sole public Radio on_click for nullable audio. C:form/*,popover.rs,list/list_item.rs,radio.rs |560×176→fresh278/media353/418/error456, inspector120/footer44; probe/cancel/retry/hiddenfixedhandoff, HeaderCookie carried data, category search/draft/confirm/focus and optionalaudio repeated-clear | Existing156px compact date region inside NEW304×240 styledPopover; direct SmallCalendar conflict, alternate navigation unverified, decision04 |
| **05: partial, raw-time policy deferred** | `downloads/schedule_view.rs` Switch/7independentToggle/SmallFormField/StatusBar/DialogContentFooter; task Progress/DescriptionList/Toolbar/Checkbox; delete/shutdown DialogContent/Alert/Footer/SmallButtons | Original schedule draft/Apply validation,430×326; task440×184/280/196/sameHWND/generation, live380×178 delete Cancel/native-close/success distinction,450×200 once-only shutdown, C#STA/OLE sourceHWND ownership | HH:MM styledInput keeps rawinvalid draft rather than valid-only TimeField; decision05. Native caption budget remains; task removed/failure inner states supplemented by10 |
| **06: requested slice verified** | `main_window/clipboard_page.rs::render_clipboard` Settings/3Page/3Group/13Items, individual fields/2FormField items; default recorder/modifierButtons, Small step/managementButtons; C:setting/*,form/*,button/* | Immediate section-owned integer +/- saves/ranges, recorder native suspend/Escape/failure, archive/storage paths/busy/cancel/risk, officialsearch/scroll/resetfalse, worker/controller/notification owners | Noneditable +/- is not editablef64NumberInput policy; tested styled composition, not universal NumberInput rejection. No new native/data feature |
| **07: partial, navigation deferred** | `ui/mod.rs::render` InputGroup/Search/scopeInputGroupButton, verticalSmallFormField/defaultTextarea and horizontalcategoryFormField/Input, SmallsourceCheckbox/SaveCancel/undo |300×440/userresize/min240×380/244categoryeditor, stableentities/IME/querygeneration, realtempworker save and no-saveCancel, occlusion/Escape/searchfocus/source selection/undo/passivepreview | category_tab/strip/add-target/order/drag/drop/wheel/context retain original oneowner; direct Tab/focus/ToggleGroup recipes limited, decision07 |
| **08: requested ordinary-presentation slice verified** | `ui/history.rs::history_row` statelessListItem/Toolbar/XSmallButtons/literalLabel; popupEmpty/Alert; tools_page ListItem/Label/SmallTag, link_panel ListItem/Label/SmallButton/StatusBar | Variable text/files64/image≤192/cache96/paging/selection/menu/modifier routes/nontext eligibility; Tools settings/keyedreuse; Link420/28row/24expander/22footer/60-84-116 derived heights, close-before-defer/copyonly/browseropt-in/timer | One project Root menu retained by previously tested300px overflow/private-placement boundary; not a new independent styledPopupMenu. No competing ListState or invented key/value labels |
| **09: requested slice verified** | `network_settings.rs::render` Settings/3Page/3Group/7Items/publicrender; twoSmallFormField, Switch, `blacklist.rs` independentCheckbox/SmallButtons; C:setting/*,form/*,checkbox.rs |720×480/input≥200/table≤202/right≤700/160scroll40rows, localdraft/atomicCoreSave/durable-before-deferredconsumers/rawproxyvalidation, failure/retry/busy/Cancel/saved≠Windowsapplied | Actual styled ButtonGroup preserves exclusive non-null proxy mode; tested directToggleGroup can clear all/group-key callback0, no need to force replacement or add new mode policy |
| **10: partial, directNotification adapter deferred; independent substitutions verified** | state.rs Empty/EmptyContent/Label/XSmallAlert/loadingSmallSpinner; fieldinlineAlert; notifications.rs actualAlert/Label/SmallButton/XSmallclose, menu publicicon/Label, hotkey defaultButton/Label; taskremovedEmpty/failedAlert, back-to-topSmallButton, clienttitlebarSmallclose | Empty/loading/error distinct; no loadingpercentage; originalmotionpolicy; max3/336/max112/timers/dedup/generation/action manager; original native Dialog/titlebar/OLE/passive owners and fixed budgets | DirectNotification dismiss/privatecompletion recipe limited, decision10. Native/custom content labels/chips not globally rewritten or certified; category-master debt remains explicit |

All ten items have current production functions and regression coverage. None is currently blocked by a compiled test failure. **Five item substitutions remain deliberately deferred; an additional unimplemented ordinary-row adoption gap remains.** There is no basis to claim all controls, all phases or all78 component families migrated.

## Suppression, wrappers and custom-content classification

1. **Not a runtime suppression finding:** `ui/components/field.rs::field_shell` explicitly sets appearance(true),bordered(true),focus_bordered(true). Previous false caller flags were redundant; removed flags do not prove a previously invisible Input. Current production scan has no appearance(false)/focus_bordered(false). Assertions/document strings are excluded from this conclusion.
2. Four `.bordered(false)` calls are DescriptionLists: download overview, task metrics, two Quick diagnostic helpers. They are official unbordered metadata presentation, not suppressed Input/Button chrome.
3. Passive28px divs around real Small24px Buttons in download batch/path diagnostics/sort are composition/observation slots, not Button size overrides. Do not promote the slot bounds into actual Button hit geometry. The full suite exercises actual descendants/actions/containment. `path_picker_shell`32px is a composition rail around defaultInput, not another painted frame.
4. **Verified narrow adoption adaptation:** path_list delete40×40 uses one actual styledButton with public Size::Size. Released Medium/Large icon Buttons both32; the original40px hit-boundary regression failed directLarge. No extra dispatcher or square legacy radius.
5. **Previously justified behavior seams:** main Sidebar public keyedfocus adapter, variable-history statelessListItem with original controller, project menu's oneRoot/session/clamped submenus, native caption drag/capture/OLE/passiveHWND ownership. These do not exempt unrelated inner ordinary controls. Domain segment dots/grids are not ordinary aggregateProgress.
6. **Existing scope-specific content:** volume/path/command outer frames, final-file/path/source summaries, preview/media panels, nativeheaders and Clipboard editor occlusion shells are not automatically ordinary Button/Input frames. Their remaining legacy typography is not certified as whole-content Kit adoption or an unsupported capability.
7. **Confirmed adoption gap:** `downloads/settings_view.rs::render_setting_control` category branch calls `ui/components/list.rs::compact_list_row`. That helper draws selected background/hover/2pxmarker and old12/11px title/detail under Base focusable_content, rather than released ListItem. Only the outer228px category-master frame is business layout; the selectable row is not explained by that classification. Candidate C:list/list_item.rs publicchildren/selected plus originalfocus/selection callback. No tested API conflict is recorded. Preserve current draft/enable/order/delete/selection/budgets; require a separately authorized production red-green replacement or explicit retention decision. Queue11 does not redesign this slice or hide it as completed.
8. **Narrow retained native caption:** `downloads/window_support.rs::download_window_close_button` is an actual styledButton with40×32 native task-title hit budget inside34px header. Unlike passive wrappers it is a real size refinement; no direct official-tier alternative is certified. Keep named native-close/pointer-consumption/windowphase tests, but final acceptance must explicitly retain or separately verify this caption adaptation. Sharedclienttitlebar close was migrated toSmall24 in10; do not conflate the two helpers.
9. Clipboard category add action remains Base/custom28px with drag-over/drop-at-end: it belongs to the full navigation/drag owner retained under07, not evidence that ordinary SaveCancel/Input remain custom. Replacing it independently without its navigation gates is unverified.
10. Optional help is real Tooltip with original full copy; severity/error/risk/runtime paths remain visible. Label and Alert String remain literal, not implicit Markdown/HTML. Natural metadata height/focus/current-frame observations take precedence over unsupported debug selectors or inactive native a11y assumptions.

## Concrete deferred decisions

| Decision | Exact source / verified evidence | Retained behavior and recommendation | Alternative requiring separate verification / user decision |
| --- | --- | --- | --- |
| **03 DataTable grid** | `main_window/download_page.rs::{render_downloads,download_task_row}`; C:table/state.rs::render_table_row applies aria_selected after delegate row. Actual `queue03_datatable_ctrl_click_exposes_single_index_selection` plus public sort/context tests | Approve narrow original stable-ID virtual grid; external multiselect semantic projection is not preserved in tested render_tr/row_selectablefalse recipe | Authorize public child/external-selection adapter investigation or upstream hook with CtrlShift/key/sort/context/OLE/semantic gates. Sort/context callbacks work; untested alternatives not globally impossible |
| **04 inline Calendar** | new_download_view.rs::delayed_start_calendar; C:time/calendar.rs Smallday28px/header/padding; actual `queue04_official_small_calendar_exceeds_existing_inline_date_budget` exceeds156px | Approve original compactdate grid inside actualstyled304×240Popover, preserving outerdraft/ClearConfirm/cancel | Authorize scrollableCalendar or nestedDatePicker and explicit changednavigation with disabledpastdate/Chinese/focus/keyboard/draft/240px tests. DirectSmall failure only |
| **05 TimeField raw draft** | schedule_view.rs::schedule_trigger_row/save; B:time_field.rs::SegmentEditor validNaiveTime/input_digit restart; actual `queue05_timefield_keeps_a_valid_time_instead_of_raw_invalid_draft` | Approve actualstyledInput retaining rawinvalid99:00/intermediate/Applyerror and disabled-ignore policy | Authorize valid-only segmented policy change or publicraw-draft adapter gates. invalid(bool) is visual, not arbitraryraw state; no mirroredcontroller/fork |
| **07 category navigation** | ui/mod.rs::category_tab/strip/adddrop; actual directTab, publictrack_focus/tab_stop and focusedToggleGroup groupcallback tests in ui/kit_migration_tests.rs | Approve narrow original navigation/order/drag/wheel/context/selection/focus owner | Authorize one publicfocus/action adapter with full pointer/keyheld/modifier/release/order/drop/wheel/context/300px tests. Specific recipes limited, no universal Tab impossibility |
| **10 Notification** | main_window/notifications.rs; C:notification.rs privateDismissRequest/publicdismiss, librarycompletion; mounted `queue10_direct_notification_dismiss_does_not_complete_original_manager_callback` reports on_close0 | Approve original boundedNotificationCenter with verifiedAlert/Label/Button copy/action presentation, no secondtoast manager | Authorize publicexternally-controlled completion adapter with max3/336×112/timer/dedup/generation/action/lease/focus gates. Directrecipe gap only; upstream/dependency change not authorized |

Separate from these five proven recipe conflicts, **category-master row adoption is unimplemented, not unsupported**. User decision: authorize the bounded original-owner ListItem migration, or explicitly accept a documented narrow retained customrow. Nativecaption/content adaptations also remain named, not visually auto-approved. No decision is silently made in this draft.

## Evidence gaps and final acceptance

- Current build and tests do not certify aesthetic quality, every control's palette/transientstate, whole-product accessibility, physical input,125/150/mixedDPI, genuine OLE drag or installed lifecycle.
- Historical Foundation/approvedlauncher/QuickSettings captures and user approvals remain scoped evidence; they are not screenshots of this frozenbuild. Queue01–11 acquired no new consolidated gallery PNGs.
- Queue12 captured actual startup-isolated production rendering and inspected50PNG images through BrowserPreview/browser screenshots. Default/minimum/fixed and key long/error/empty/disabled/loading/scroll states are represented; missing states are explicitly listed below, never replaced with HTML controls.
- [Single consolidated gallery](../../artifacts/ui-kit-final-acceptance/index.html) and [capture evidence](../../artifacts/ui-kit-final-acceptance/capture-evidence.json) are delivered. Final user acceptance and all outstanding adoption decisions remain open.
- Frozen native/domain/worker/save behavior is verified at the listed boundaries, not by an unrestricted system harness. Avalonia nativebase and proprietaryListary engine sources are unavailable; only indexed public/managed source evidence is claimed.

## Queue11 closeout status

Frozen integration and adoption reconciliation are complete **as a verification task**, with no product fixes required. Product migration remains partial. Local document/link/task/diff checks and freeze/process cleanup are recorded in the queue11 outcome after execution; no automatic packaging, installation, source commit or queue12 execution is implied.

Closeout checks: specs87documents/12owners, task context8implement/6check entries,29 local links across8documents with0missing, official78/47/15 sets/173 releasepaths, diff--check and313-file frozen-hash verification all exit0. Build/native-fixture survivors0; exactnative4RUN/4PASS/noSKIP. Debug executable SHA256 is recorded in scratch. Existing unrelated CRLF normalization warnings are not whitespace errors. Only task/ledger/index/report documents changed during queue11; no Rust/C#/dependency/gate modification or source assertion deletion.

## Queue12 production screenshot delivery

The latest validated Debug executable is `target/debug/nxtools-desktop.exe`, SHA256
`FA591A03984A6977AA9774F832DDBD7CFBF231F6008EE1131D1F679A5B1B69F5`.
Every final PNG record names this same executable/hash, ownedPID/HWND, actualclient
dimensions,96DPI, capturetime and imageSHA256. PNG pixel dimensions match nativeclient
readback. [Full per-image evidence](../../artifacts/ui-kit-final-acceptance/capture-evidence.json)
and group `capture-*.json` accompany the [single gallery](../../artifacts/ui-kit-final-acceptance/index.html).
Images are raw client PrintWindow captures, not HTML-mock product controls or repainted
compositions. The HTML page only organizes actual images, provenance and missing coverage.

### Isolation and fixture changes
- First matched queue11 source/executable hashes. Missing visual fixture coverage required
  new Debug-only `NXTOOLS_PREVIEW_FINAL_GALLERY` branches and production-view projections.
  Seven expected source/gate changes: main_window/visual_preview.rs and its final_gallery.rs,
  downloads/visual_preview.rs, network_settings/visual_preview.rs, ui/final_gallery.rs,
  ui/mod.rs Debug module registration, ui-preview-isolation gate. Original businessview
  render/save/native/controller functions, dependencies and approvedlauncher unchanged.
- Explicit `--ui-preview foundation` intercept remains before ordinary startup; Release
  rejects it. UUID session scratch SQLite worker/in-memoryDownloadEngine, syntheticitems,
  empty-rootQuickcatalog and disconnectedplatform/indexer bridges only. No realhistory/
  config/Clipboard/URL/volume/service access. Download tasks were enqueue-only, not run.
  Native task identities use0 HWND for business commands; no liveOLE/Windows actions.
- Settings constructors may discover FFmpeg. Capture child-process PATH/FFMPEG_PATH was
  restricted to session scratch, then restored; no realtool/PATH discovery or installation.
  NoSave/Apply/Download/Delete/Archive/Compact/Migrate/Browser/Shutdown action was clicked.
  UIA supplement clicked only exactnamed ownedSettings pages/Configure and captured the
  resulting ownedTool window. Nativecapture/resize/wheel targeted only the createdPID/HWND.
- Windows defaultstack overflowed in the first DebugDownloads fixture before capture.
  Same capture loop turned green with a32MiB **isolated finalgallery thread only**; ordinary
  startup and existingpreview threads are untouched. This is Debugfixture evidence, not
  proof the Release product has the same fault or a blanket fix of nativeinteraction.
  Earlier crash, incomplete marker capture and filenamecollision are retained in scratch;
  final exactmarker waits/client+PNG checks and uniqueIDs exclude incomplete/overwritten images.
- All capture-created previewprocesses and exactnativefixtures were cleaned by exactpath/
  ownedPID; no unrelated process was terminated. No package/install/services/indexer,
  systemDPI change, delegation or commit. Debugstdout contains only readiness metadata,
  not secrets/userpayloads. Synthetic rawvalues are intentionally visible in screenshots.

### Latest verification after fixture additions
New queue12 isolationtest1/1; final fullDesktop **616/616**,0failed/0ignored; strict
tests/productionClippy, workspace--tests, fmt--check,Debugbuild all exit0. Complete logs
`queue12-final-{full,clippy-tests,clippy-production,workspace,fmt,build}.log` and `.exit`
files are in session scratch. Every Rust invocation used serialjobs1/incremental0/
codegen-units256 and full-operationtimeout21600s. Fourteen affectedarchitecture gates
all exit0, including expandedDebug/scratch/disconnected isolation contract.
Latest exactsafe nativeAll Quick2/2 andClipboard2/2 RUN/PASS/noSKIP/exit0; same96DPI
400×410/460×406, passiveMA3/primarypolicyoff1/clientreuse/UIAsynthetic checks.
[Quick native evidence](../../artifacts/ui-kit-final-acceptance/native-quick-evidence.json),
[Clipboard native evidence](../../artifacts/ui-kit-final-acceptance/native-clipboard-evidence.json).
This is still not physical/highmixedDPI/fullreader/installed or genuineOLEdrag certification.

### Screenshot inventory and actual inspection
All **50/50** final images were actually viewed through BrowserPreview and browser
screenshot attachments in eightmodule groups plus the three supplementalimages. Group
screenshots stayed in scratch; `Inspected=true` means agent actually saw thePNG content,
**not** that the user approved it or every transient state was certified.

| Group | PNG count | Actualclient budgets / captured states | Inspection result / limit |
| --- | --- | --- | --- |
| QuickFindSettings |7|1060×650 fivepages;920×480commands default+scrolled|All viewed; originalentities/forms/diagnostics visible; minimumSave appears after actualscroll, noSave clicked|
| Approvedlauncher/content |4|620×44collapsed;620×410results/actions;400×410content|All viewed; originalapprovedeffect retained, no redesign or actionexecution|
| Downloadsworkspace/settings/support |14|1060×650/920×480main;984×650fiveSettings/error/busy/window-scrolled;430×326schedule;380×178delete;440×184queued/removed|All viewed; existingcategorymaster debt still visible; footer/risk/longHeader/error projections effective; no transfer/delete/settingscommit|
| ClipboardSettings/popup/editors |12|1060×650threeSettingspages;920×480Settingsminimum;300×440history/empty/loading/error/text/category;240×380minimum+scrolled|All viewed; distinctempty/loading/error, longtextSaveCancel/occlusion, originalstep/status/storagepath; image/filemixed/menu not captured|
| Tools/Link/notification |5|1060×650/920×480Tools+threeoriginalnotifications;460×320ToolSettings;420×84/116Linkcollapsed/expanded|All viewed; max3 boundednotifications andoriginalaction remain; no browser/copy/nativecommand/settingssave|
| GlobalSettings |6|720×480General/Network/rules/error/busy/outer-scrolled|All viewed; errors/disabled/footer present; General duplicateSwitchlabels noted; scrolled is outerpage only, not innerlistlastrow proof|
| Newdownloadordinaryconfirmation |1|560×278originalconfirmedform|Viewed; fields/inspector/footer present; no networkprobe/submit/download|
| Once-onlyshutdownwarning |1|450×200countdown/risk/footer|Viewed; fullonce-onlywording; no realshutdown/timeraction|

### Visible observations and missing screenshots
- GlobalGeneral shows both aSettingItem title andSwitchlabel for the same setting. This
  is recorded as a **user visualfeedback candidate**, not silently redesigned or called
  a functionalfailure. User may accept it or request a boundedlabel-only correction.
- Quickcommands920×480 default requires originalscroll to reachSave; the scrolledPNG
  actually showsSave fully. Downloadswindow-scrolled shows finalsettings controls and
  fixedfooter. Globalrules-scrolled shows outerpage movement only; ruleinternal lastrow
  reachability is not certified by that image. Clipboardminimum-scrolled shows history
  moving andback-to-top, not everyhistoryrow/terminalpage acceptance.
- Longerror images show boundedscroll viewports; onePNG cannot prove everyline was read.
  Fulltests cover completecopy/state/scroll owner, but palette/readability/aesthetic
  acceptance remains for the user. No captured image was replaced by a blank placeholder.
- **Explicitly missing:** attachedfile-dialog-search productionPNG; newdownloadHLS/DASH,
  category/schedulePopover anderror/disabled states; taskfailed/completed/expanded;
  Clipboardimage/filemixedhistory, longcategorynavigation andcontext/submenu; every
  surface'scompletehover/pressed/keyboardfocus matrix; innerGlobalrulesbottom state.
  These are screenshotcoverage gaps, not declarations the features are broken or
  unsupported. No new fixture/realworkflow was invented to force them into this delivery.
- EarlierfiveAPIrecipe decisions andcategorymaster unimplementedadoption remain exactly
  the decisions above. Nativecaption40×32/customcontent typography remain namedbounded
  retained roles, not wholepage exemptions. All finaluser decisions stay open.

### Acceptance request
Open the [single final gallery](../../artifacts/ui-kit-final-acceptance/index.html), review
the capturedmodule/state groups and provide one consolidated list of acceptedgroups,
visibledefects or requestedcorrections. Please also choose retention versus separately
authorizedinvestigation for03/04/05/07/10, and boundedListItemmigration versusexplicit
customrowretention for theDownloadSettingscategorymaster. Missingstates may be requested
as a subsequentisolatedcapture slice. No response yet means no uservisualapproval.
Queue12 delivers a traceable **partial coverage gallery**, not complete migration,
physical/highmixedDPI/reader/installed/releaseacceptance. Packaging/install/service actions
remain separately authorized and were not performed.

Queue12 final closeout:50uniquePNG/50actualinspections/50gallerycards/102localresources,
clientpixel96DPI/currentexe+imagehash checksPASS;only5existingDebug/module/gatepaths and
2newDebugfixtures differfrom11baseline,ui/mod productionremainder identical. Specs87docs/
12owners,task8implement6check,42locallinks8docs/0missing,diff--check all exit0. Owned
build/preview/nativefixture survivors0. NoRustedit afterfinal616/strict/build/native/
PNG acquisition;report/gallery metadata do not claim completecoverage or userapproval.
