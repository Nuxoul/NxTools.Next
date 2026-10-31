# Window / progress / preferences continuation — integration evidence

Status: source implemented; settled scoped verification and explicit remaining global/native limits are recorded below and in window-tabs-icons-drag-outcomes.md. This supplements, never replaces, phase-a-checkpoint.md or retained history. No packaging/install/product launch/Registry/association/service/commit/push authority. Installation error1 remains OPEN.

## Reference and component/native boundary

Read AGENTS and component selection -> current Kit0.8.0 -> table/index -> UI authority, Desktop/Core/IPC/Protocol/PlatformHost topics, archive.md/archive-virtual-drag.md and canonical task plan/current checkpoint. Legacy tracked archive search identifies packaging/FFmpeg/image/video owners, not a general archive operation UI. Licensed Archiver AppWindow115 -> callbacks -> ui.run158, selected extraction before open697; public Bandizip language resources describe progress/form decisions but provide no native lifecycle proof. Current original forms/progress constructors and WindowOptions are the actual Next boundary. No version/vendor/native API change.

Approved caption reference is current browser released Component0.8.0 TitleBar:34DIP caption/control width,12DIP left inset,16DIP Small glyphs, theme secondary hover/pressed and danger close, live maximize/restore and native Drag/Min/Max regions. Existing official TitleBar has private controls and Windows on_close_window ignores custom callbacks, so a single shared public official Button/WindowControlArea composition preserves original presentation-close/draft/background owners. No per-window copied paint/control implementation. The changed shared TITLEBAR_HEIGHT is34; global theme unchanged. ButtonCustomVariant owns hover/active; adding Button.hover again caused actual GPUI panic and was removed, not patched upstream. Main window keeps original C# move/min/max/tray-close commands; other captions retain GPUI native event regions and original callbacks.

caption_window_options explicitly sets transparent native caption + Client decorations, preventing native server caption plus inner caption. Archive host, forms and progress use it. Existing other windows already use Client decorations/titlebarNone; their visibility/focus/bounds are retained. Rust owns layout/state, GPUI rendering, C# Windows adapters. All cross-entity effects defer.

## Desktop surface inventory

| Surface | Caption/buttons and existing lifecycle |
| --- | --- |
| Main settings/workspace window | shared caption_shell/caption_action, min/max-or-restore/close; original C# commands and close-to-tray policy preserved |
| Archive browser host | one shared controlled caption above official TabBar; resizable min/max/close; original application retires all tabs before HWND removal |
| Extract/create/ZIP edit forms | shared controlled caption; resizable min/max/close; native close retires secrets/pickers, Cancel never submits or persists draft |
| Archive task progress | shared controlled caption; resizable min/max/close; Close backgrounds presentation, never sends cancel; explicit Stop retains original safe committed-output semantics |
| Download progress/failed/completed task | shared fixed-client caption, min/close only, original keyed background/restore/close; no fake maximize |
| Download new/picker/confirmation, delete, schedule, shutdown | shared client caption close only, existing fixed budgets, original cancel/save/handoff/once-only owners |
| Download settings and global settings | shared caption close only; native fixed/nonminimizable; original draft Apply/Cancel/overlay-edit cleanup |
| Tool feature settings (currency/link/floating assistant etc.) | shared controlled caption min/close only, original460x320 window and activation row/scroll; native is_minimizable alreadytrue |
| Clipboard primary popup / Quick Launch search / link panel / floating assistant | existing specialized popup/gesture chrome, no ordinary caption or three-button addition |
| Clipboard/Quick Launch preview, notification popup, menu popup, taskbar overlay | passive/tool/borderless lifecycle untouched; never add activating captions |
| Clipboard text/category editor, command editor | in-window edit layers, not independent native windows; keep original editor/dialog cancel/focus, no duplicate window chrome |
| System pickers/UAC/other applications | unchanged, not owned by Desktop caption component |

Real native caption dragging/double-click/maximize/restore/100/125/150/200%DPI/keyboard/provider/user visual acceptance NOT RUN. Synthetic hit bounds/actions do not establish those results.

## Progress statistics and layout

Original worker_job::ProgressWriter.flush publishes latest OperationControl bytes/files (processed codec work) at>=100ms with nullable total and files_totalNone. These are not final native publications. C# ArchiveJobSession derives terminal files/bytes committed and skipped from actual runner/output journal, and verified counts separately from worker terminal. UI must not equate these metrics.

Original JobProgressView now renders known progress only from honest totals, readable shared binary units and existing shared duration. Running shows processed bytes/files with explicit unknown/processed!=committed; terminal primary result shows actual committed/skipped or actual verified for Test, elapsed, native/error/uncertain-cleanup facts and partial-output warning. No invented percentage/count, business-stat mutation or whole-task rollback claim. One natural compact scroll body, fixed accessible action/footer and shared34caption; default560x300/min480x260. Form min480x360 retains scrolling body, target Input/browse and fixed Cancel/Submit, errors scroll within bounded footer rather than clipping primary actions.

## Persistence field inventory and compatibility

Only explicit accepted form submission with original Remember selected persists operation defaults. Extract fields: default_destination, conflict_policy, smart_single_file_folder, copy_motw, open_after_extract, keep_extract_progress_open, optional extract_custom_destination. Custom target has separate opt-in and existing bounded local absolute-path validation; Here continues source-parent behavior and does not replace saved custom target. Old missing or explicit-null optional target meansNone; malformed JSON/section or invalid non-null path is preserved/rejected rather than default-overwritten. Form still defaults Rememberfalse; Cancel/source picker/render never writes.

Create remembers original supported defaults create_format/compression_level/strip_single_root/exclude_patterns/test_after_create/keep_create_progress_open, no password/selection/source. Runtime password/show/grants/permissions/source snapshot/ordinals/request All-vs-Selected/jobs/delete-source are not ordinary preferences.

Extract save uses baseline-vs-desired changed-field merge inside original AppConfig lock/disk reread/normalize/sibling sync/rename. Other sections, shell desired revision and concurrently changed untouched archive fields survive. Original ordered/deferred saved-settings publication remains. Explicit progress Keep Open checkbox now saves ONLY the corresponding create/extract keep field using the same save_settings/background writer; failure keeps current job policy and reports durable failure, never claims success. A red actual checkbox->disk-reopen test proved the previous memory-only gap.

## Actual verification / retained failures

- Initial integration compile failures (in-flight form chain, Button ID/import/API, IconName Debug/PartialEq and test mutability) retained in session logs. They are not behavior passes.
- archive-targeted6:201PASS/3FAIL,0skip: terminal fixture did not wait original readmission attempt, launch fixture dropped native endpoint, tab-close fixture chose ambiguous same-label target. Fixed actual-boundary fixtures, no safety assertion deletion. archive-progress-red8:31PASS/3FAIL incl real Keep Open not durable, terminal coordination fixture, affected download fixed-Tab-count assumption.
- Real titlebar duplicate-hover panic reproduced with3actual host tests, fixed through official variant only. No vendor edit.
- archive-targeted-green9:205PASS/0FAIL/0ignored. Covers Tree real down/move, original rows/marquee/preview/password/cancel, actual form min/scroll and disk saves plus progress commit semantics. Later tabs-final5/5 adds original entity/selection/preview retention and host all-close/stale-close. Shared caption1/1; affected download disclosure and toolbar keyboard regressions pass after actual semantic target focus (not fixed Tab count).
- Core93/93,IPC43/43,Protocol83/83 PASS (archive-owners-check10).
- Strict four-owner Clippy --tests -Dwarnings PASS and workspace --tests check PASS. Global fmt initially FAIL for own last fragments and unrelated network settings files; own fragments formatted, final global status reported by main outcomes.
- C# test-host build PASS0warnings/0errors. Exact reviewed virtual Tree5 LIST/RUN/PASS5,SKIP0/FAIL0; matching isolated-codecs worker rebuilt. Additional exact pure icon ProgID plan LIST/RUN/PASS1. No real Registry/OLE target/Explorer/product execution.
- Full Desktop first attempt was interrupted after recording an obsolete fixed-Tab-count failure; no summary. Resumed run completed1393PASS/4FAIL, retained below; allfourfailedcases subsequentlyPASSindividually. No full-unit rerun or fullPASSclaim.
- Source/icon/spec/catalog/release/editor gates PASS as recorded in main and independent icon/Tree reports. Actual rebuilt Desktop PE resource verification and final lifecycle checks recorded by main after source convergence.

## Remaining acceptance

No real product/window screenshots, physical gestures, Explorer/foreign target, mixed-DPI/IME/UIA/user visual/installed acceptance. Original install error1 and all older canonical-phase obligations remain separate OPEN. Current package/artifacts and protected installation have not been updated. No user data/system association/service or installation diagnostic chain changed.

## Settled integration results (supersedes in-progress status, retains failures above)

Resumed full Desktop completed1393PASS/4FAIL/0ignored of1397tests,436.93seconds,exit101. Failed tests: download shutdown fixedTab3, Tools header old36DIP, globalsettings close old24DIP, archiveexclusion scroll over childTextarea/saveoutsideviewport. All four now individuallyPASS in settled logs; old assertions were updated to approved34caption/currentsemanticfocus/actualpage-margin scroll and fulltargetvisibility, original action/disk checks retained. No second full-unit rerun or false whole-suitePASS claim. Independent Desktop extension_http10/10PASS.

Additional actual new red: oldsourcepassword Dialog covered newlyopenedtab. Hostedconstructor now explicitly suppresses initialfocus; old Dialog cleanup precedes currentcontentfocus and active Dialog blocks contentfocus. A first attempt queried WindowExt before Root and produced143PASS/65FAIL; actual common ComponentWindowState-missing panic was corrected via one originalconstruction helper with explicitfocuspolicy (no preRootDialogquery). Final archive208/208PASS,0ignored includes sevenhosttests plus retainedTree/rightrow/marquee/preview/password/cancel. Allnegative logs retained.

Maintitlebar doubleclick actualMouseDown count2 originally sent BeginWindowMove (newtestRED0/1); minimal originalcallback now sends existing C#ToggleWindowMaximized fordoubleclick and preserves BeginWindowMove forsingle. Matching original titlebartransport test GREEN recorded by main. No newWindowsAPI or vendorpatch.

Final four-owner `cargo clippy --tests -- -D warnings` and workspace`check --tests` PASS. Production-only strict remains FAIL at original SPACING_12 import/main_window.rs75 and floatingassistant animated/media.rs62; no allow/suppression or unrelated implementation. Newtest-onlyconstructor/retiredradius warnings fixed with exactcfg(test). Globalfmt remains FAIL only preexistingnetwork_settings.rs1135/1184/1201,network_settings/blacklist.rs,page_rem_tests.rs; ownfragments/scopedfmt pass. All final command names/exitcodes live in mainoutcomes.

Actual rebuilt Desktop PE PASSgroup1blue/group2orange/18exactdecodedframes withoutLoadLibrary/Shell/execution. C#LIST/RUN/PASS5virtualtree+1pureiconplan,0skip. Source gatesDesktopShell/DownloadUI/ArchiveFspike/shell/spec/catalog/release/editor/tree/iconPASS. No liveproduct/native/DPI/visualacceptance or packagegeneratedidentitychange. Optionalreadonlyreview was interrupted by usermessage and yielded no settled report; no reviewcompletion or secondreview claimed.
