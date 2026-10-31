# Audit #8 — command naming outcome and verification blockers

## Outcome and status
The bounded requested Clipboard/Downloads/Tools command-label implementation is delivered directly in this session, without delegation. Actual production label reds preceded repairs; final **12/12 new regressions PASS**, zero ignored. Actual original pointer/keyboard/once/cancel ownership and full object names are preserved. **Not a clean global #8 closeout:** broad MainWindow, production/test strict Clippy and workspace-wide fmt retain genuine failures below. Original closure count remains7/13; earlier#6/#7 incomplete statuses and failed full-run history are unchanged. No complete Desktop run occurred.

The exact changed/direct-result/exception mapping is in command-matrix.md. No blanket ellipsis replacement: Clipboard Backup/Compact are immediate worker operations and retain no ellipsis; Import/Export/Migrate use pickers and gain one U+2026. Download menus distinguish current confirmation/file-delete policy from immediate list removal and task cancellation. Tools Configure and image selection open further surfaces and gain ellipsis. Final delete/Cancel, Restore default, direct clear/copy/queue commands remain immediate and unmodified.

## Source changes and behavioral preservation
Own existing Rust files13, new test file1, append-only references3:
- main_window/clipboard_page.rs: only Export/Import/Migrate visible labels; actual Button semantic name derives from the same label.
- main_window/download_page.rs: header Settings/Schedule/New, task-window and task-cancel labels; menu Delete wording projects the SAME current task_removal_deletes_file setting via a read-only application getter. False still calls original remove(task,false); true still opens original confirmation. Separate list-remove topology is unchanged, even when labels coincide. No confirmation/capability/action-policy change.
- downloads.rs: only read-only task_removal_deletes_file getter, no mutation or new business decision.
- Tools link/currency/floating Configure labels and idle image-picker label; no body-toggle/keyed-window/picker callbacks changed.
- downloads/settings_view.rs and new_download_view.rs: exact full directory/FFmpeg purpose names synchronized with Tooltip/a11y and one ellipsis;3browser-export labels reflect the original save dialog. No export/signing/picker/native implementation change.
- ui/components/menu.rs: original custom ElementItem content gets selector-derived ID, Role::Label/aria_label from the SAME existing rendered SharedString and public test observation. Original MenuItem/Label geometry, icon/color/variant/handler/controller/Confirm/dismiss remain. This is a named semantic CONTENT label, not a replacement action node or native reader certification.
- Existing Clipboard test777line prefix unchanged; Download old3952line prefix changes only two full picker-purpose strings with U+2026. Existing Floating assertion changes only the image-picker ellipsis. New main_window/command_naming_tests.rs registered by cfg(test) append only.
- docs/reference-code/{clipboard,download,gpui}.md receive append-only actual command chains, fixed behavior and evidence limits; old bytes remain exact prefixes.

All original callbacks are exact after reversing the presentation-only delta; local Floating rustfmt whitespace is normalized for comparison. #3 object-specific names are not shortened; #4 Archive complete identifiers/producer/ticket/Confirm/defer files are unchanged. No typography role/basefont/rem/layout/palette/bitmap/native client/hit/permissions/vendor/dependency/IO/String ownership or cursor work. Existing diagnostics remain unchanged; no payload/input/frame logging added.

## Reference/API review actually performed
Read AGENTS,mandatory selection,current0.8.0/table/API, original design guide command/result/ellipsis rules651–718, Desktop/shared owner/verification/guides and matching Clipboard/Download/Tools notes. Exact Button semantic fallback, PopupMenu ElementItem(no automatic a11y label)/Label public composition and existing Toolbar parent adaptation were checked. An initially considered PopupMenuItem::label was correctly rejected after source inspection: it is a static noninteractive title, not a chainable semantic-name setter. No such invalid API was implemented.

Actual owner chains were followed before wording: Clipboard original picker pending/result/empty-data cancellation versus direct maintenance worker; Download typed Batch Remove/Cancel versus original confirmation/direct-policy branch and engine final_path conditional; Tools Configure stop-propagation/keyed normal window/Root/close; floating original PickImage; browser export empty destination -> C# adapter -> STA SaveFileDialog -> cancellation before export. C#/engine were read only, not run/edited. Engine list removal cleans owned temporary artifacts; only delete_artifact=true removes final_path, and no new test executes true.
Reference review includes ABDM RemoveDownload.kt explicit --remove-file flag and legacy BuildTaskContextMenu. Initial legacy snippets were read before the later bounded ABDM source follow-up; no strict reference-order compliance is claimed for this naming slice. Legacy ClipboardHistoryService ExportJson/ImportJson/Vacuum and SettingsWindow constructor/WindowAnimationService.Attach/OnOpened/Apply/Cancel/OnClosed were reread. These references preserve behavior; unavailable Compose/Avalonia native internals are not inferred. Tools has no verified matching generic legacy window. No actual original/native picker/product operation occurred.


## Effective reds / failed preparation retained
| Run | Actual observation and disposition |
| --- | --- |
| clipboard-label-red |0/1 runtime red:actual Export label/name 导出 vs 导出… |
| audit08-label-reds |2PASS/3FAIL:Clipboard label/once/cancel green; DownloadSettings name missing ellipsis and Tools Configure missing ellipsis are effective reds. Menu absent semantic node is an observation/preparation gap, not a proven old-label red. One new unused cx warning later removed. |
| menu-label-confirmed-red-2 |4PASS/1FAIL:actual named old menu 删除文件 vs 删除文件…; header/Tools/Clipboard green |
| image-label-red-keyboard |8PASS/1FAIL:actual image-picker 选择图片 vs 选择图片…; menu Enter/Tab+Space cancellation green |
| remaining-download-label-red |9PASS/2FAIL:actual new-form directory and Settings default-directory names missing ellipsis |
| export-label-confirmed-red |0/1:actual Chrome/Edge export label lacks ellipsis, after directory fix exposed actual export control |
| audit08-final-twelve |12/12PASS,0ignored,4.72s;final complete source and test formatting |

Not behavioral reds: new KeyDownEvent missed prefer_character_input(E0063); new menu test_support lacked public trait import(E0599); an unsupported Edit old_string payload was rejected with no file write; test fixture duplicate line was found/fixed before compile; Option<bool> focus assertions corrected during preparation; unrelated independent Cargo/rustc idle checks refused command admission before Cargo/log creation. Intermediate logs with green/red in their filenames are attempts, not results. No old assertion was ignored/weakened, no fixed sleep or direct business-method positive gesture shortcut, no blanket lint suppression or competing input dispatcher.
Formatting/preparation diagnostics retained: scratch prefix-stat read used default GBK after byte-prefix check already passed; corrected explicitUTF8. Final wrapper inline Python with Chinese failed in PowerShell before production formatting/tests; a separate escaped UTF8 Python file passed original-prefix guard and only unexecuted steps resumed. These were tooling failures, not source/layout reds. The opening independent process had prevented earlier queue execution but ended naturally during this task's bounded180second observation; no kill/pause/yield/reservation/lock bypass.


## Actual interaction tests and limits
- Clipboard production Settings search/Buttons: labels/name and minimum-width containment; Export pointer, Import actual Tab+Space down0/up1/duplicate-up0, Migration pointer each produce one original typed fake picker. Success+empty data cancels; no worker import/export/backup/compact/migration. Exact owned config bytes, worker database path and absent backup directory retained.
- Actual MainWindow default1060x650/min920x480 header and3Tool Configure Button labels/bounds. Pointer Configure twice reuses same keyed window; original close removes presentation. Existing tool_card_toggles_and_configure_opens_without_toggling passes actual card Enter/Space and Configure keyboard/pointer/no-bubble behavior.
- Original task context menu text observes current file-delete true/false policy. Pointer or official menu Enter opens exactly one original confirmation; final Delete file has no ellipsis. Pointer or actual Tab+Space Cancel closes without task removal/file writes. Duplicate release is checked at original input owners.
- Original list-only callback removes synthetic in-memory queued record while preserving an owned sentinel; task Cancel retains record and becomes Cancelled. UI snapshot is display-completed but underlying engine remains queued: this proves original command branch/admission/record outcome and non-touch of sentinel, not native completed-file cleanup correctness. Source review establishes delete_artifact conditional; no new test clicks file-delete commitment.
- Directory/FFmpeg/new-form purpose names preserve full object;3extension export Buttons dispatch one fake typed format/destination-empty request and canceled reply retires pending. No actual package/file dialog/export. Long Chinese file labels/settings controls/current footer reachability retain fixed clients; existing relevant long path/rem tests pass.

## Compiled/source checks — actual outcomes
All Rust invocations, including rustfmt/scratch format/direct shells, inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. Own commands were serial and admitted after natural idle, with full-operation timeouts. Independent commands sometimes subsequently started; no duplicate own build, process termination/pause or resource coordination. Full Desktop is prohibited and was not run.

| Check | Result |
| --- | --- |
| Final audit08 new regressions |12/12PASS,0ignored,4.72s |
| Final related Downloads |210/210PASS,0ignored,175.23s; exactoriginalkeyboard8 andpicker14 verified from pass lines |
| UI related |237/237PASS,0ignored,59.58s; menu/identity/gradient/fullpayload/fixedcap protections included |
| MainWindow related |**206PASS/6FAIL**,0ignored,106.50s; all6failure fixtures are original independent Extensions removal/storage resource-limit admission at removal_tests.rs72. Original log remains FAIL, no broad rerun or skip. |
| #3 actual names final |7/7PASS,0ignored,0.93s;two object-purpose expectations synchronize ellipsis only |
| #4 complete object/prompt |6/6PASS,0ignored,4.38s |
| #5 focused confirm/default submit |15/15PASS,0ignored,4.37s |
| Existing #6 rem protections |36/36PASS,0ignored,160.08s;does not close original#6 |
| #10 history identity |11/11PASS,0ignored,22.37s |
| Sort keyboard |3/3PASS,0ignored,15.81s |
| QuickLaunch presentation |11/11PASS,0ignored,0.41s |
| Existing Tools interaction subset within MainWindow |tool-card no-bubble1,Floating3,Currency1 all observed PASS;not whole MainWindow green |
| Final workspace check --workspace --tests |PASS,exit0,29.88s |
| Final scoped rustfmt skip_children --check (14Rust files) |PASS,exit0 |
| Final scoped git diff --check |PASS,exit0 |
| Final strict Desktop --tests -- -D warnings |**FAIL101**:external extensions/storage_management.rs78/196 nonminimal_bool only |
| Final production strict Desktop -- -D warnings |**FAIL101**:sameexternal2 plus task-start-existing main_window.rs75unusedSPACING_12 and tools/floating_assistant/media.rs62animatedneverread. No allow/outside repair. |
| Final workspace cargo fmt --all -- --check |**FAIL1** in unchanged Queue08main_window/page_rem_tests.rs,shell.rs,network_settings/blacklist.rs,page_rem_tests.rs and network_settings.rs. OwnFloatinglabelwrap fixed;finalscopedfmtPASS. |
| Source/document gates |11/11PASS;Download,Desktop shell,ClipboardUI,token,consistency,state,motion,catalog,release,editor,spec. Finalreferenceappend specgate separatelyPASS. |

Groups overlap; counts are not summed into a fabricated unique/fullDesktop count. Strict failures are not inferred from old Queue07 history: both production/test strict were freshly executed on current bytes, before/after final narrow additions, and raw failed logs remain. No claim current project strict is green or all later diagnostics would be absent after external repair.

## Integrity, dirty attribution and history
Baseline captured373 live source/contract files and2676prior task/evidence files at2026-10-26T02:07:18.160673+00:00, never HEAD. Final inspector passed17exact inverse/prefix/history checks over13existing Rust files plus3append-only reference files;1new test is separately new.347unaffected baseline files remain hash-identical, including accepted original #4/#5/gesture/rem/native owners. All2676old records remain unchanged. Precise before/current snapshots and forward/reverse opcode deltas are published separately; no historical manifest was rewritten to pretend its live layout hash still holds.

Ten independently changed files are observed separately, not owned/reset/credited: Desktop database-guidelines.md; extensions.rs; extensions/{service,service_storage_tests,storage_management,ui_host}.rs; main_window/extensions_page/{removal,removal_tests,storage}.rs; main_window/extensions_settings.rs. Their observed hashes are timestamped observations, not a workspace lock/promise. Source ownership verification and artifact hash replay are distinct from compiled/visual/native acceptance.

## Remaining blockers and stop
The bounded naming/interaction implementation is delivered, but **global#8closeout is NOT declared** because broad shared-tree tests/strict/workspacefmt are not all green. Original7/13technical closure count is unchanged. Preserve failed MainWindow/storage,productionlint andformat logs; do not silently repair independent work, exclude tests or borrow future external fixes. No additional audit item or background resumption is started/promised.

Physicalpointer/keyboard,WindowsIME,DPI/mixedmonitor,UIA/reader,screenshots/visual/user,realclipboard/paste/search/delete/import/export/archive/network/picker/Shell/nativeHWND/Host/service/product/runtime/userdata,packaging/install/commit/push andinstalled/releaseacceptance:NOTRUN/NOTAUTHORIZED. Synthetic temporary SQLite/config/IPC/ownedsentinel tests do not imply them. Read-only final verifier checks evidence/source/delta/history,not product acceptance.
