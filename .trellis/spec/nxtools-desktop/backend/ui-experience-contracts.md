# Unified document and experience contracts

Authority: approved2026-10-22 six-item snapshot; framework remains Kit0.8.0/defaultpre0.3.9. This supersedes historical no-editor/built-in-default-only presentation restrictions, not existing action/native ownership. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/markdown_document.rs`, `repo:rust/apps/nxtools-desktop/src/ui/markdown_edit.rs`, `repo:rust/apps/nxtools-desktop/src/ui/preview_image.rs`, `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/crates/nxtools-download/src/http.rs`.

## Scenario: Unified passive documents and explicit save
### 1. Scope / Trigger
Clipboard list/full preview/edit and Quick Launch text/Markdown presentation changes.
### 2. Signatures
`MarkdownDocument`, `DocumentMode::{Markdown,Literal(language)}`, `document`, `cached_view`; pinned editor `BlockEditor::passive(bool)` defaults false. `ClipboardPanel::{prepare_markdown_edit,markdown_edit_value,save_edit}`; `edit_generation`, `edit_save_pending: Option<Uuid>` guard async ownership.
### 3. Contracts
Same editor projection/Kit renderer across four surfaces. Passive skips focus/input/action/caret/grip editing ownership; links propagate to original list action, never browser/network. ImagePolicy::Placeholder, existing64KiB/1200line/depth32/node8192 safety budgets, mounted400char list summary/210DIP whole text card/8gap preserved. Overbudget falls back literal original text, never old TextView. Original action payload independent from summary/canonical editor source. No-edit/cancel preserve exact raw bytes; source save preserves CRLF, changed rich save may canonicalize with visible notice. Same content/mode cache avoids per-frame parse; source_snapshot is revision-cached. Cross-owner effects next-turn including cache hits. Save pending installed before original worker dispatch; duplicate pending save ignored. Completion must match edit generation/pending ID/item. Cancel/reopen retires UI ownership but does not undo an accepted durable worker write. Newer typing remains open after submitted value succeeds. Failure retains draft and retry; no payload/error Display in diagnostics.
### 4. Validation & Error Matrix
Unsafe/deep/oversize -> bounded literal fallback, no resource loader. IME composition -> no save. No changes -> close without write. Duplicate -> one operation. Old completion -> no new editor close/focus. Worker failure -> pending cleared/draft retained. Rich-mode format normalization is explicit, not applied to readonly data.
### 5. Good/Base/Bad Cases
Good: saveA, cancel/reopen draftB, A completes and B remains. Base: readonly Markdown/table/code displays and original full paste unchanged. Bad: readonly flag treated as passive, summary pasted, canonical text silently overwrites CRLF, or old save closes new draft.
### 6. Tests Required
Editor passive focus/default editing and Fenwick regression; four real host seams/full payload/link action, source/no-edit/cancel/save/failure/duplicate/stale, actual focus/IME/undo, long last-block scroll and resource privacy. Final editor234/Desktop1002 source tests; exact native Quick/Clipboard2each at96DPI separate from physical WindowsIME/mixedDPI/reader/user acceptance. Provenance57files allows only element.rs and exactly3cull.rs lowbit substitutions; no nightly/bootstrap/vendor expansion.
### 7. Wrong vs Correct
Wrong: `set_readonly(true)` in a result row; worker success unconditionally clears editing_item_id. Correct: explicit passive renderer plus original owner; current edit_generation/pending/item/draft gate before terminal UI publication.

## Scenario: Durable image modes and external launcher gaps
### 1. Scope / Trigger
Core presentation settings, Clipboard image layout and Quick Launch result/action rows.
### 2. Signatures
`ClipboardImageViewMode::{Auto,Fit,Panorama,Light,Mini,Compact}`, serde-defaultAuto; `QuickLaunchRowStyle::{Continuous,Separated}`, serde-defaultSeparated/gap0or4. `image_plan(source_pixels,available_DIP,dpi_scale,mode) -> ImagePlan`; `set_image_view_mode`, `set_row_style`.
### 3. Contracts
Official Settings/Page/Group/Item/dropdown field32DIP, original deferred app setter. Core locked section-field merge preserves latest storage/other owners; failed commit never publishes live mode. Auto uses workarea and aspect-aware overflow, Fit full/no scroll/no unnecessary upscale, Panorama sourcepx=devicepx, light/mini/compact whole-window shortedge<=800/500/300devicepx. Viewport/content separate; reset scroll on item/mode; bounded decode unchanged. `visible_bounds` uses rcWork, original adjacent right/left/clamp. One passive HWND through original hide/offscreen/generation/boundsACK/frame/reveal. Image list190/files62/sourceiconhue unchanged. Launcher interactive content40DIP, slots40/44; gap outside click/focus/hover, ordinary/recent/action rows share owner and scroll stride. No indexing/native service configuration for either presentation setting.
### 4. Validation & Error Matrix
Legacy missing fields -> Auto/Separated. Corrupt/unwritable config -> priorlive, retry notice. Tiny workarea -> existing bounded placement behavior, not claimed mixed-DPI acceptance. Overflow axis only scrolls; Fit never retains offset. Gap click -> no action/selection/preview. Old result/ACK -> original generation rejection.
### 5. Good/Base/Bad Cases
Good:125%DPI Panorama2400sourcepx produces2400devicepx content. Base:4DIP default can return to0. Bad: innerpadding as gap, caplongedge instead ofshortedge, full cached config overwrite or index restart for presentation.
### 6. Tests Required
Sixmodes×aspect/DPI math, actual GPUI longimage scroll/Fitreset/settingsdurablefail, Coredefaults/merge;0/4actualgap/noaction, persistfail/noIPC, fullQL153. Preserve preview lifecycle/native readback tests. Physical mixedDPI/actual wheel/native image-mode matrix remains separate.
### 7. Wrong vs Correct
Wrong: fixed440x340/fullscreen bounds or uniform40stride with4padding. Correct: sourcepixel/DIP/workarea plan plus original staged HWND;40content+external0/4slot.

## Scenario: Passive notification cadence and coherent themes
### 1. Scope / Trigger
Notification auxiliary WindowOptions and default theme selection.
### 2. Signatures
`notification_window_options(display_id)` sets inactive_frame_intervalNone. `tests/native/notification-cadence.ps1 -AuthorizeNative` requires existing Debugbinaries and scratch; Debug `preview_cadence` captures<=512paints/8milestones. ThemeRegistry::load_themes_from_str + Theme::update/change; project GraphiteLight/Dark config names.
### 3. Contracts
Official Toast remains sole animation/clock/stack/close owner. No new production frame clock/log; idle stops. Native hiddenprepare/readback/finaldraw/reveal and exit-beforehide retained. Debug legacycap only when explicit isolated Cadence global+env enabled; Release lacks sampler. Native production preference calls App::set_reduce_motion; fixture must do same, not only businessglobal. Theme registration does not replace Registry defaults; app explicitly chooses project configs for Light/Dark/System. Optional official themes unchanged; current Kitcolors -> renderabletokens/Base/businessprojection. Keep fonts/radius/geometry/sourceiconRGB.
### 4. Validation & Error Matrix
Default33333us -> source/config regression. Invalid native readback/stalegeneration -> no reveal. Missing effective refresh/sample -> report unavailable, not smoothnessPASS. Unknown keys -> schema gate fail. RGB/HSL roundtrip -> channel tolerance1e-5; original Kitrole/token equality exact.
### 5. Good/Base/Bad Cases
Good:180Hz currentpaint~5.5ms vslegacy~34ms, idle0; reducedentrance1frame. Base: same optionaltheme retains wholepalette. Bad: recursive frame sampling drivesidle, hardcode60FPS, recolorcustomcanvas while KitInput/Menu staysold, or screenshots treated as userapproval.
### 6. Tests Required
Notification35 plus actual ownednative boundedcadence/reduced/foreground/readback. Theme11+optional1,112schema/statecontrastroles each, actual96DPI dark/light0/4GPUIcaptures. Source/compiled/native/AIvisual/user/installed evidence reported separately. Upstream editor serialize.rs Clippy nonminimal_bool remains unmodified/outsideapproval; strict projectpackages use --no-deps and disclose unrestrictedlintfailure.
### 7. Wrong vs Correct
Wrong: configtest provesperceivedsmoothness; is_defaultJSON replacesregistrydefault. Correct: productionoptions + boundednativepaintcomparison; explicitregisteredconfigselection via publicThemeupdate/change.

## Integration closeout amendment — 2026-10-22
User explicitly approves exactly one equivalent serialize.rs list_item Boolean simplification plus
rustfmt whitespace in that push_str fragment and element.rs passive-link if block. This supersedes
the two-file/serializer-outside-approval statements above only for these exact deltas; no allow,
dependency upgrade or other vendor formatting. markdown-editor-contracts.py reverses serializer to
originalSHA256, checks16Boolean cases and reverses element whitespace to its approved passive hash.
Full dependency strict Clippy remains required, not --no-deps substitution. Clipboard UI lifecycle
gate checks real original rich/source mounts, pending-before-dispatch/item/draft gates, cancel/focus
and passive readonly document ownership; scratch positive/negative source-mutation tests retain
gate sensitivity without changing product behavior.

## Scenario: Adjustable Clipboard density and official visual theme draft
### 1. Scope / Trigger
Approved density/theme follow-up supersedes wholetext210DIP and Graphite default aliases only; earlier document/action/native contracts remain.
### 2. Signatures
`ClipboardSettings.text_card_max_height: u32`, missingdefault160, normalize120..320; `ClipboardApplication::set_text_card_max_height`, `ClipboardPanel::set_text_card_max_height`. `NetworkSettingsView::render_theme_chooser`, official `Radio/RadioGroup::children`, `ScrollableElement::overflow_y_scrollbar`; one original `ThemePreference` and Apply/Cancel.
Evidence: `repo:rust/crates/nxtools-core/src/lib.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs`, `repo:rust/apps/nxtools-desktop/src/ui/history.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings/theme_chooser.rs`.
### 3. Contracts
Wholecard includes header/body/border/feedback, externalgap8 independent. Short natural; summary400/full actionpayload independent; images190/files62 unchanged. Field-only durablemerge before deferred live ListState remeasure preserving logical scroll/query/selection; failure priorlive/no captureIPC. Official NumberInput outer32DIP, finiteinteger guard. Optional feather deferred because resolved ListItem hoverbackground unavailable without added ownership/patch. Light=Latte/Dark=Frappe/Systemsamepair; all explicitofficialthemes/fonts/geometry retained, originalGraphiteassets remain historical. Clean-room NxTools miniatures, reserved marker/border, Light4/Dark6/System1, no globalpreview/autosave/newpair. Nested Settings ScrollBounce requires official overflow_y_scrollbar arbitration; bare inner overflow_y_scroll has valid extent but wheeloffset0. Do not invent custom wheelforwarding. RadioGroup in released0.8.0 is semantic grouping, not arrow-key controller; original Tab/Space remain supported.
### 4. Validation & Error Matrix
Missing height ->160; outofrange config normalizes; fractional/negative/string JSON rejected; invalid setting/noop -> no write. Commitfailure -> priorlive and original notification. Draftselection -> no globalchange/diskwrite; Cancel discards; failedSave retains draft/activepalette/retry. Official Latte supportingcontrast below historicalGraphite4.5 and Frappe equalprimaryhover/active are retained published facts, not recolored to satisfy oldtests.
### 5. Good/Base/Bad Cases
Good:120DIP longtext reduces summary before keeping feedbackactions inside. Base:160default and selectable210. Bad: clipbuttons/savewholecachedconfig/copy400summary or claim wholewindow is notificationcard. Themegood: scrolltolastSolarizedDark and Tab/Space chooseGruvboxDark with footerreachable; bad: exposeoffscreenchoices/imports as usabilityproof.
### 6. Tests Required
Core legacy/integer/clamp/merge75full; realcards4,setting/lifecycle2,feedbackfourcaps; chooser4focused plus originalSaveCancel/optionaltheme tests. Production wheel regression red offset0/max431.5 -> officialscrollwrapper green. Historical natural-layout tests explicitly320 while default160clipping tests remain. Primary paintedforeground compares exactKitrole, not RGB/HSLreverse equality. Full Desktop evidence/failures in taskoutcomes; no physicalIME/mixedDPI/reader/uservisual/installed acceptance.
### 7. Wrong vs Correct
Wrong: implicit210constant, bare nested scroll, immediate appwide theme preview. Correct: ownedheightfield->durablemerge->nextturnpresentation; publicKitselection+scrollcompound on originaldraft/save owner.

## Scenario: Probe naming protects asynchronous user edits
### 1. Scope / Trigger
Existing NewDownloadView probeconfirmation or HTTP Content-Disposition name; not browserdiscovery/alwaysprobe policy.
### 2. Signatures
`content_disposition_file_name`, existing `percent_decode_utf8`; `should_replace_suggested_file_name`; `file_name_edit_generation` captures original InputEvent::Change. Evidence: `repo:rust/crates/nxtools-download/src/http.rs`, `repo:rust/crates/nxtools-download/tests/http_filename_contract.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`.
### 3. Contracts
Read ABDM first,oldNxTools second. Plainheader strictUTF8percentdecode once,plusliteral,malformedrawfallback,extendedpriority,Windows-safe validation. Usefulprobe mayreplace URLderived/nonfilenameversionlabel, not genuinecustomextension/genericprobe. Compare editgeneration AND currentvalue before applying oldprobe; editA->B->A remains userintent. Silentset_value hydration doesnotemitChange; originalreplace_all/useredit does. Existing probe generation/cancel/receipt/context/TLS/route/350msgrace/media/fixedhandoff remain; no additionalremoteprobe. Diagnostic source/correlation/generation only, no filename/URL/context payload.
### 4. Validation & Error Matrix
InvalidUTF8/escape ->rawliteral then existingvalidation; decodedtraversal ->URLfallback. Genericdownload.zip ->retainusefulcurrentname. Superseded/cancel -> originalretirement, no newhandoff. Edited/cleareddraft ->currentname regardless probe. Externalreportedresource remains unproven without sample.
### 5. Good/Base/Bad Cases
Good: encodedURLauto matches once-decoded name and adopts usefulservercandidate; custommy-custom.zip stays. Base: optionalname adoptsserver. Bad: allnonemptybrowsernamesassumedcustom, oldprobeoverwriteslateedit, or percentdecode twice.
### 6. Tests Required
Actualloopback8headercases and7realprobe/fixedconfirmationcases; retainexistingcontext/cancel/media/receipt tests, fullDownloadlib117/HTTP26/resolution15/serialengine103. Taskledgerrecords red andgreen; tests not externalserveracceptance.
### 7. Wrong vs Correct
Wrong: currentvalue==startvalue means unedited. Correct: original Change revision plus currentvalue and probegeneration guard.

## Historical hovered notification Ctrl+C — cancelled
The hovered-card Ctrl+C prerequisite proved only settled geometry and the official exit-start business-record gap. User explicitly cancelled that global-hook scheme; do not resume it. The2026-10-23explicit header Copy button below is a separately approved replacement, not permission to consume global Ctrl+C.

## Scenario: Primary Clipboard activation after native reveal
### 1. Scope / Trigger
The user reports Ctrl+digits and type-anywhere still failing after summon, with an unrelated hover preview. This amendment preserves the existing input/action owners and adds the missing primary activation step.
### 2. Signatures
`ClipboardApplication::{request_popup_activation,finish_popup_activation,retire_popup_activation}`; `PendingPopupActivation { id, popup, started }`; existing `PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle })`; `ClipboardPanel::{focus_search_for_session,hover_preview_ready}`.
Evidence: `repo:rust/apps/nxtools-desktop/src/clipboard/popup_activation.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs`, `repo:rust/apps/nxtools-desktop/src/ui/mod.rs`, `repo:csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs`.
### 3. Contracts
Hidden GPUI creation with focus:true does not activate its HWND. The existing native visibility adapter uses SW_SHOWNOACTIVATE. After a matching reveal ACK, register pending activation before dispatching the existing native ActivateWindow command. Match command ID, session, window and HWND; after successful native focus/foreground result, defer original search focus and revalidate current/open/editor/overlay state on the next turn. Never call GPUI activate_window on the prepared hidden HWND, which may replay initial placement. Passive preview remains nonactivating. Hide, replacement and disconnect retire pending activation; a three-second logical deadline reports failure without retry loops. Failure retains the visible panel and original notification error owner. New hover entry is checked against CURRENT panel state in the deferred callback; armed number, pending paste/full-item read, query and editor states cannot requeue preview. Leave and idle hover retain the original owner. Diagnostic events carry correlation/session/current/success/fixed reason/elapsed/native code only.
### 4. Validation & Error Matrix
Reveal success -> one primary activation command, not inferred keyboard readiness. Duplicate/old reveal or activation -> no new focus effect. Hide/replacement/disconnect -> retired pending; late success cannot refocus. Native failure/timeout/dispatch failure -> clear pending, fixed retry guidance, no false success. Ordinary platform text from a control -> original search InputState; IME/editor/menu protections and original full-by-ID paste remain.
### 5. Good/Base/Bad Cases
Good: native reveal -> ActivateWindow ACK -> next-turn search focus. Base: original active-panel Ctrl-first release and Unicode input still work. Bad: show:false/focus:true or GPUI focus alone treated as OS activation; a new hover callback reopens preview during a paste.
### 6. Tests Required
`input_followup_` exercises actual application reveal/result queues and panel events: missing native activation and hover requeue were two real reds; four new cases pass, including five retirement/failure modes. Retain eight `input_repair_` regressions, exact typed PasteText/full tail/target and ordinary control ownership. Strict lint, workspace and full Desktop1029/1029 passed at this checkpoint. Synthetic IPC does not certify physical Windows keyboard/IME or installed acceptance.
### 7. Wrong vs Correct
Wrong: visibility ACK -> GPUI search focus -> assume keyboard works. Correct: visibility ACK -> pending-before-native-activation -> current ACK -> deferred original search focus. Wrong: cancel old preview once. Correct: also reject new deferred hover while the original operation owns input.

## Scenario: Overflow-only text feather and small static card shadow
### 1. Scope / Trigger
User approves a light bottom fade instead of hard clipping and a subtle history-card shadow. Earlier optional-feather deferral is superseded by this public-API composition; notification hovered Ctrl+C is explicitly cancelled by the user, not an outstanding blocker.
### 2. Signatures
`HistoryText { id, source, maximum_height, selected, group }`; `MarkdownDocument::bounds() -> Bounds<Pixels>` reads the original canvas cache; `body_feather(document,selected,group,cx)`; public ListItem Styled/`group_hover`, GPUI canvas/fill/linear_gradient and BoxShadow.
Evidence: `repo:rust/apps/nxtools-desktop/src/ui/history.rs`, `repo:rust/apps/nxtools-desktop/src/ui/markdown_document.rs`, `repo:rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs`.
### 3. Contracts
One cached passive document and its existing natural bounds; no reparse or second layout. Cap its visible wrapper using the already-computed body budget. At prepaint compare the current document bottom against that viewport with0.5DIP tolerance; paint at most one14DIP vertical transparent-to-opaque gradient per mounted overflowing body. Short/fitting text has none. Only the prose column fades, never count/source/feedback/actions. The endpoint is the opaque result of the current official card fill over the canvas: normal surface, selected official list_active/accent, hover tokens.list_hover through the existing group_hover. Do not store parallel hover state. No click/action/focus/occlusion callback on the fade; original full paste remains. One card shadow uses black8% alpha, offset(0,1DIP), GPUI blur sigma1DIP, zero spread; no animation, offscreen mask or whole-window shadow. Whole-card cap120..320/default160, image190/file62, gap8 and hit bounds remain unchanged. Test-only finish switch is constant true outside tests, not persisted configuration.
### 4. Validation & Error Matrix
Actual overflow -> one bottom quad. Fitting/short body -> no quad. Theme/hover/selection -> endpoint matches actual filled-card composition, including alpha. Missing prepared document -> existing literal fallback, not extra parsing. Fade/gap clicks -> original action/no action respectively. No added idle frame/timer. CPU fixture timing is observational, not a brittle latency threshold or GPU claim.
### 5. Good/Base/Bad Cases
Good: clipped prose fades over14DIP while count and buttons remain crisp; a click on the fade pastes the full original. Base: short text and image/file geometry are unchanged. Bad: blur the whole card, repaint a substitute ListItem, fade by character count, duplicate hover state, hide short-content layout defects or introduce a nested scroll owner.
### 6. Tests Required
Three `feather_followup_` cases: actual long-body red0quads->green1; short-none, Light/Dark normal/selected/hover endpoint matching and full-original fade click; alternating12x8 forced-draw CPU cost with unchanged document identity/heights and bounded quad count. Framework Style paints fill and border separately: observe the nontransparent fill at exact card coordinates. Hsla JSON serialization quantizes to8bit colors: compare the typed Background directly, not a reconstructed hex color. Drain initial control-mount callbacks before off/on idle comparison; both states then schedule0frames. Measured Debug synthetic median11624.06usoff/11803.74uson (~1.55%,179.68us),2extra quads for2clipped bodies; not GPU/display/installed acceptance. Preserve all height, action, feedback and pointer regressions plus strict/workspace/full Desktop checks.
### 7. Wrong vs Correct
Wrong: any non-solid quad proves feather; serialized color inequality means a product color bug. Correct: actual14DIP body-local gradient, exact typed background equality to independently observed card fill, unchanged interaction/geometry and measured static cost.

## Scenario: Clipboard preview requires pointer intent
### 1. Scope / Trigger
Summon, history repopulation or repaint beneath a stationary cursor must not open preview. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/history.rs`, `repo:rust/apps/nxtools-desktop/src/ui/mod.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs`, `repo:rust/apps/nxtools-desktop/src/ui/efficiency_tests.rs`.
### 2. Signatures
Released Component0.8.0 `ListItem::on_mouse_enter` forwards actual `MouseMoveEvent`; `on_hover` also runs from GPUI pre0.3.9 paint. `ClipboardPanel.preview_pointer: Rc<Cell<Point<Pixels>>>` retains one screen-position baseline. Original `ClipboardApplication::schedule_preview(entered,item,panel_bounds,pointer_position,panel_scale_factor,cx)` and PreviewSession remain sole scheduling owners.
### 3. Contracts
Only actual unpressed/non-drag movement enters preview; ignore on_hover(true), retain on_hover(false) leave cancellation. One panel-retained screen position rejects unchanged-coordinate activation/hit refresh across redraws, seeded from window position at construction, updated by bounds observation and normal parent move after row dispatch. Do not initialize a per-render Cell from window.mouse_position: a fresh closure is not cross-render intent memory. Admission remains deferred; validate captured session/query/current row ID and current open/editor/armed/pending/fullread/query readiness before touching the application. Same hovered item is idempotent in original PreviewSession so motion does not restart800ms or allocate another candidate clock. Full-ID hydration, scroll suppression/resume, generations, one passive HWND and C# nativeACK/frame/reveal remain unchanged. No new polling, global input, delay increase, diagnostics stream or styles.
### 4. Validation & Error Matrix
Paint/repopulate with no move -> no candidate even after1200ms. Same-coordinate events after redraw -> no candidate. Real movement within an already-hovered text/image row -> original candidate without requiring leave/reentry. Pressed/drag -> no admission. Same item motion -> same generation; leave clears; reentry admits. Deferred old session/query/row -> discard without mutating newer preview.
### 5. Good/Base/Bad Cases
Good: panel appears underneath a stationary pointer and remains quiet, then2DIP deliberate motion admits. Base: existing800ms/full-original/native presentation. Bad: on_hover(true) schedules from first paint, require users to leave/reenter, or reset the timer on every mouse event.
### 6. Tests Required
Five `idle_preview_` real-panel regressions cover stationary text/image mount, same-position/pressed events, same-row generation/leave/reentry and deferred old session/query. Original stationary mount was executed red before production edits. Drain initial fixture window bounds/focus callbacks before active-motion controls; a late test initialization callback can otherwise reset the test mouse and generate a false leave. Keep input_followup_/input_repair_ and actual preview presentation/800ms tests, strict lint/workspace/fullDesktop and lifecycle/source gates. No physical/installed acceptance is implied.
### 7. Wrong vs Correct
Wrong: `.on_hover(|entered,...| schedule_preview(*entered,...))`. Correct: public ListItem movement -> unchanged-position filter -> next-turn current identity/readiness -> original idempotent preview owner; hover exit retains cancellation.

## Scenario: Completion Open With is a compact adjacent action
### 1. Scope / Trigger
User requests no visible Open With text, only a button icon to the right of Open File. Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/task_window.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/tests.rs`.
### 2. Signatures
Existing `icon_action(id,MaterialIcon::ExpandMore,"选择应用打开下载文件").outline()` uses official Small Button/public icon, accessibility_label and Tooltip. Existing OpenShellTarget OpenWith callback/current task-window identity unchanged.
### 3. Contracts
DialogFooter order Drag,Reveal,Open primary,OpenWith24x24outlineicon; no visiblelabel/no extra menu/controller. Direct footer composition avoids Toolbar.child ghost adaptation;440x184client unchanged. Tooltip and accessiblepurpose retained, official pointer/keyboard owner only. Deferred existingapplication command keeps pending-before-dispatch/cancel/retry/currentHWND+generation close. Main-list Reveal never closes unrelated completion windows.
### 4. Validation & Error Matrix
Icon button -> one chooser command. Cancel -> keep window/no error notice. Matching confirmed Reveal/Open -> originalwindow removed and keyedregistration released. Failure/timeout/disconnect/stale/duplicate -> original retry/identity rules; not unconditional clickclose.
### 5. Good/Base/Bad Cases
Good: OpenFile then24x24caret with accessiblepurpose. Base: chooser Windows API behavior unchanged. Bad:60x24textbutton beforeOpen or custom-painted clickableicon.
### 6. Tests Required
Actualproduction geometry red60x24versus24x24, order/bounds/accessibilityname, actualiconclick->typedOpenWith/cancel; originalrealwindow lifecycle/result/isolation regression and main-list context menu. Report screenshots/user/installed acceptance separately.
### 7. Wrong vs Correct
Wrong: remove all labels including accessiblepurpose or close immediately afterclick. Correct: omit visible Button.label, retain Tooltip/accessibility and original correlated success-only close.

## Scenario: Explicit notification header copy
### 1. Scope / Trigger
User2026-10-23approves Copy immediately left of original Close and fork preservation. Evidence: `repo:rust/apps/nxtools-desktop/src/notifications/copy.rs`, `repo:rust/apps/nxtools-desktop/src/notifications/presentation.rs`, `repo:vendor/gpui-component-080/src/notification.rs`.
### 2. Signatures
`Notification::header_action(builder -> AnyElement) -> Self`; `NotificationCenter::{bind_copy_platform,request_copy,handle_copy}`; existing `PasteTextCommand { text,paste_after_write:false,target_window_handle:0,target_focus_window_handle:0 }`. Public slot is defaultNone, official XSmall ghost Button/Copy or Check; no new wire/hook/clipboard engine.
### 3. Contracts
Same official top-right toolbar and original Close/list/ToastManager/native noactivate/hiddenframe owners. Header focus scope tab_stopfalse keeps toolbar visible while either control is focused: GPUI KeyDown switches to keyboard modality and suppresses group_hover; hover-only visibility otherwise removes keyboard listeners before Space/Enter release. Optional header omitted during Ending; no custom key dispatcher. Title reserves32DIP at currentrem for two20DIPcontrols+gap beyond official padding; longbody remains originalscroll. Callback stops bubbling and defers to current ID/generation; one pending installed before correlated caller-ID dispatch. Native success alone sets Check; fixed retryTooltip on enqueue/nativefailure/15sdeadline/disconnect; no recursive notice, lifetime reset, action claim or feedbacktimer. <=3retained statuses, oneactivecopy. Snapshot title/message/first8displayeddetails only, <=256KiBUTF8 checked before allocation; overbudget rejects, never truncates. No payload diagnostics. C# WindowsPasteCoordinator copy-only returns before activation/paste branch; pendingWindowswrite cannot be retracted, late result never marks replacement.
### 4. Validation & Error Matrix
Old/closed/exitingbusinessgeneration -> no enqueue; pending -> no duplicate; unmatchedresult -> unchanged; replacement/retirement -> oldstatepruned; timeout/disconnect/failure -> retryablecurrentcard, lateackignored. OfficialEnding -> optionalbuttonabsent. Oversize/empty -> failedfeedback/no command. Copy success does not close/remove/repush/change notificationcount, sourceaction or timer.
### 5. Good/Base/Bad Cases
Good: focusedCopy+SpaceDown0/SpaceUp1PasteText(fulltext,copyonly), matchedresultCheck; original2snotice expires on originaldeadline. Base: nonheader callers retain5sdefault/originalClose. Bad: activatepopup to receiveCtrlC, group_hover-onlytoolbar loses focusedbutton onKeyDown, queueadmission=>success, copyingactionlabel/internalID, or feedbacknotice recursivelycopies.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/notifications/copy_tests.rs`: actualcopy/closeequalheight+leftorder, exactpayload/fulltail/targets0/correlation/pending/failure/retry/oldresult/timeout/disconnect/closed/oversize/keyboardonce/longtitle/originalexpiry. Existing allpositions/LightDark/stackpause/nativescreen suite retained. Final46notificationrelated/fullDesktop1102/strict/workspace/fmtpass; sourcegates/provenance pass. Realclipboard/physicalpointer/DPI/reader/uservisual not performed. One optional review cancelled/no report, not passed. Fork normalpush/diffidentity separately recorded in taskoutcomes.
### 7. Wrong vs Correct
Wrong: `.invisible().group_hover(...)` is sufficient for a keyboard-reachable header action. Correct: originalhover OR currentheaderfocus keeps the same official toolbar visible, so native KeyUp reaches official Button's existing listener. Never add a duplicate manual Enter/Space handler.

## Runtime feedback: bounded Select and nonredundant focus (2026-10-24)
Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/tools_page/currency_settings.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/currency_tests.rs`, `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/apps/nxtools-desktop/src/theme_graphite_tests.rs`. Same keyed460x320feature shell, Form/Field, official default32DIP Select/SearchableVec and original entity/actions. Released Select::w_full is constrained by parent w_full/min_w_0/max_w164DIP; dropdown menu remains320x160. Actual input snapshot with full Chinese Yuan Renminbi choice previously extendedx280..600; now inside right content edge448. Do not claim outer-wrapper bounds alone prove actual trigger containment.
Global released Theme.focus_ring=false omits the redundant outside halo only. Preserve original ring/selection palette, official thin tinted edge/internalfocus, focusability and keyboard selection. No vendor/API upgrade, fake Select, enlarged native window, transparent selection or keyboard suppression. Real GPUI currency selection/save/reuse and theme-preference regressions pass; pixel/aesthetic/reader/mixed-DPI/physical acceptance remains unverified. Current fullDesktop result1102pass/1unrelatedschedule assertion failure is recorded in task10-24-runtime-feedback-reveal-ui, not a full-green claim.

## Scenario: Session-only Clipboard fixed display and menu exclusion (2026-10-24)
### 1. Scope / Trigger
Approved runtime feedback changes panel auto-dismiss/menu/preview and narrow settings presentation. Evidence: `repo:rust/apps/nxtools-desktop/src/ui/capture_controls.rs`, `repo:rust/apps/nxtools-desktop/src/ui/components/native_context_menu.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings/system_proxy.rs`.
### 2. Signatures
`ClipboardPanel::fixed_display` defaults false per construction and clears on explicit close. Official Small ghost Toggle sits left of Recording. Menu session records source device-pixel bounds, pointer origin and current source session; `ReadSystemProxy` returns bounded `SystemProxySnapshot` through the existing command result.
### 3. Contracts
Pinned paste stays visible without CopyOnly reactivation; native target focus owner remains original. External foreground can update target except same-process/preview/transientIME. Explicit close/reopen resets pin, never config. Menu open retires pending/visible preview and new/delayed admission stays blocked. Source HWND and overlay click inside source both dismiss menu only; overlay click does not swallow the following independent click. Source dismiss gesture cannot trigger an item. One preview HWND/800ms/generation/nativeACK remains. Proxy read-only InputStates are separate from manual draft; requestID+view generation+window gate, pending-before-dispatch, deferred application. System routing/save model unchanged, PAC/auto/no endpoint/failure truthful. Blacklist editor precedes bounded120DIP row viewport at720x480; footer remains reachable. Tools share official Badge.dot+text from real enabled state. Firefox Small outline action uses exact supplied HTTPS listing and correlated original Shell owner.
### 4. Validation & Error Matrix
Menu pending/visible/lateACK -> cancelled/hidden; old source/session -> ignored. Pinned paste -> visible/no activation; unpinned unchanged. Proxy old ID/modeABA/closedview -> discarded; disconnect/5sdeadline -> current read failure, manual untouched. Native admission is not browser-open success.
### 5. Good/Base/Bad Cases
Good: menu-overlay click within Clipboard closes only menu, next click acts normally. Base: unpinned original auto-dismiss. Bad: pin is only TOPMOST, saved durable pin, CopyOnly focus branch for pinned paste, system snapshot overwrites manual draft or fabricates PAC address.
### 6. Tests Required
Actual panel/menu/preview lateACK/pressedToggle/targetIME/command terminal, proxy synthetic modes/stale/manual, blacklist many/empty/busy/footer, tool keyboard/persistence, Firefox exact command and actual sort button centerline. Source/synthetic evidence is not physical/installed acceptance. Current outcomes and unreproduced startup-size issue: task10-24-runtime-interaction-feedback.
### 7. Wrong vs Correct
Wrong: compare only sort wrapper bounds or menu HWND. Correct: actual Button centerline (pre-fix100.5vs102.5DIP) and native-menu overlay screen-point classification; original component/focus owners retained.

## Scenario: Hidden primary first-show preparation (R5,2026-10-24)
### 1. Scope / Trigger
Silent/browser/archive-origin primary hidden startup. Live96DPI/notmaximized1904x1021client firstshow and exactDWMlog correlation are in task10-24-runtime-interaction-feedback/r5-live-boot-evidence.md. Evidence: `repo:rust/apps/nxtools-desktop/src/platform_events/main_preparation.rs`, `repo:rust/apps/nxtools-desktop/src/main.rs`, `repo:csharp/NxTools.PlatformHost/Windows/MainWindowPreparation.cs`.
### 2. Signatures
`PrepareMainWindowCommand { window_handle,source_process_id,width,height }` carries logicalDIP; `PreparedMainWindow { x,y,width,height,dpi }` returns actualdeviceclient. Originaldispatcher owns optionalpreparation/pendingID/bounded128notificationbuffer/8sdeadline; native5sboundedSTA.
### 3. Contracts
HiddenGPUIWindow retainsCW_USEDEFAULT bounds until publicactivate consumes initialplacement; nativeShow/Activate bypasses that. Before publishingHWND/enablingentrypoints, C# verifies authenticatedpipepeer+ownedhiddenroot/samesession, stagesprimarymonitorhidden, readsactualtargetDPI, appliesexact1060x650DIP scaledonce with liveclientinsets, verifiesclient/hidden/DPI. NoSHOW/activate/TOPMOST/stylechange or vendorpatch. Rustpending installedbeforeenqueue; exactcurrentACK+boundedtypedreadback then deferredreadycallback outside dispatcherlease, then originalnotificationreplay. Source/queue/nativefailure/disconnect/deadline/budget -> hiddenfail/quit, nooldACKrevival. Oncenativeprepare only; latertray/summon/floatingrestore keepsoriginalsize/focusowners and userresize. Browsercoldstart mainfocus/show flags unchanged; its minimization occursafterreadiness.
### 4. Validation & Error Matrix
Wrongpeer/type/extraDTO/visible/destroyedroot -> reject beforefinalapply; DPIoutside48..960 orclientmismatch -> noreadiness; wrongID/duplicate/retiredACK -> nopublication; disconnectbetweenACK/deferredcommit -> nocallback; overflow128 -> failhidden. Nativecancel islogical, cannot claimuncooperativeAPI stopped; boundedslot retaineduntilactualreturn.
### 5. Good/Base/Bad Cases
Good: hiddenprepare/readback thenoriginaltrayreveal, later1200x700userresize surviveshide/show. Base: normalshownstartup retainsoldconstruction. Bad: per-showdefaultreset, GPUIactivate tofixsize stealsfocus or replaysstalebounds, fakeWM_SIZE, SHOWthenresize or nativeBOOL=>clientready.
### 6. Tests Required
OwnedCW_USEDEFAULToriginalShow red1904x990vs1060x650, productionprepare+actualclientgreen; noshow/foregroundchange and lateruserresize. C#DPI96/120/144/192/readback/cancel/peer/malformed, Rustactualdispatcher6cases/Protocolwire+capturedscene rejection. FullDesktop1123pass/1unchangedschedulefailure; strictlint/workspacepass. Physicalreboot/mixedDPI/installedacceptance separate; nodeployment.
### 7. Wrong vs Correct
Wrong: onlyassertWindowOptionslogicalsize. Correct: actualnativeclientbeforefirstshow plus pendingACK barrier beforeeveryentrypointcanobtainmainHWND.

## Scenario: Archive recent history and leased temporary-file settings
### 1. Scope / Trigger
Remaining reference UI continuation2026-10-25. Original UIAC01/02 and association/menu source
acceptance stay CLOSED. History/temp controls are real consumers, not overall visual acceptance.
### 2. Signatures
`ArchiveSettings::{remember_recent,recent_limit,temp:ArchiveTempSettings}`; temp fields
`base_directory:String,retention_hours:u32,cleanup_on_child_exit:bool`.
`UiStateStore::{update_recent,flush,changes}` extends its original one bounded writer/thread/file.
`ArchiveEntryCommand.temp_policy:Option<ArchiveTempPolicy>` is materialize-only; new `Open` action
requires existing operation/session/generation/lease_id and forbids path/source/output authority.
Evidence: `repo:rust/apps/nxtools-desktop/src/archive/ui_state.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/archive_temp_settings.rs`,
`repo:csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.Open.cs`.
### 3. Contracts
Recent defaulttrue/10,0..30; path data stays in original separate local UI-state file, never logs.
Only successful current top-level reads with actual cleanup and native browser identity enter MRU;
nested leased archives and accepted/failed reads do not. Dedup case-insensitive, bound30/128KiB raw
paths/1MiB serialized file. Geometry save preserves latest recent field; clear awaits original writer
durability receipt and propagates failure, not queue admission as success. Original bounded change
signal notifies existing views, no polling/new history runtime. Corrupt state blocks overwrite.
Temp legacy defaultempty/24h/false; retention choices1/6/24/72/168h. Local drive-qualified paths only,
finite components/reserved names/traversal/device/UNC rejected; C# rechecks real no-reparse ancestry.
Original Settings/Normal groups/typed NumberInput/dropdown/Switch/render slots/ListItem and actual
Small outlined Buttons compose controls; temp Input keeps draft and per-input revision. Browse only
fills unchanged current draft; cancel/late result never saves. Explicit directory save uses original
locked changed-field merge, detects dirty baseline conflict; retention/exit fields merge separately.
Successful results keep original ordered/deferred publication; active jobs/leases are not moved.
Policy snapshots each new materialization into original Cache; chosen base gains only owned namespace.
Same cache retains bounded old-root discovery; manifest/fileID/hash/MOTW/lease/empty-ancestry deletion
remain mandatory. Retention is abandoned-file cleanup eligibility, not a deletion deadline.
Lease-only Open resolves sealed path natively, retains known external-process hold, returns promptly
and observes only captured process asynchronously. Optionon tries original safe retirement on exit;
optionoff retires only after browser request. Null handle uses original close/expiry fallback. Edited,
locked/replaced/foreign files survive. No forced process termination, recursive cleanup or write-back.
### 4. Validation & Error Matrix
Invalid config/policy/control fields -> reject without silent defaults. Missing/invalid recent store
-> visible unavailable, old bytes retained. Failed clear -> error, current archive remains valid.
Changed directory baseline -> conflict+draft retained; unedited save adopts latest disk. Picker input
revision/value mismatch -> ignore selected path. Native lease identity mismatch/unsealed -> no launch.
Disconnect stops observation, never labels it child exit or waits indefinitely for an external app.
### 5. Good / Base / Bad Cases
Good: top-level success records MRU, actual list opens original read, clear survives late window close.
Base: legacy config/default cache and original expiry; default external open has no early cleanup.
Bad: nested temp path persisted, whole stale UI snapshot resurrects history, Process.Start return
treated as document exit, arbitrary custom-base recursion or active lease deletion.
### 6. Tests Required
Core default/save/reopen/invalid; Protocol materialize-policy/open lease shape. Production Settings
list/open/clear/draft/save/conflict/late picker/cancel, Light/Dark minimum reachability. Original entry
representative asserts policy snapshot and correlated lease-only open after materialization. Actual
splitter/scrollbar press-move-release and old axis tests distinguish gestures from direct offset setup.
C# exact temp fixtures cover root switch/restart/bounds/reparse/edited/locked/child on-off/null/failure/
disconnect; no actual Shell launch. Product visual/native/physical/DPI/UIA acceptance remains separate.
### 7. Wrong vs Correct
Wrong: draw temp/history switches without consumers, clear memory and claim durable success, or
pass returned file path back as arbitrary Shell authority. Correct: original UI-state writer receipt,
original Core save order, materialize snapshot and native lease-only Open with original cache checks.
Wrong: tests/imports mean reference restored. Correct: report source/automated/physical/visual levels
separately; user-provided images uninspected means visual requirements remain unaccepted.

## Scenario: Approved six-item UI consistency repair (2026-10-28)
### 1. Scope / Trigger
User-approved task `10-28-ui-experience-consistency-repair` supersedes only the old pending-result
withholding, automatic selection-inspector coupling, row path metadata, Small shared Switch and
inline submit-validation expectations. No new engine, native owner, framework patch or theme.
### 2. Signatures
Original `quick_launch::search::apply_search_update`; `FileDialogSearchPanel` and
`PlaceFileDialogSearchWindow`; `MainWindow::{sync_download_detail_scope,open_download_detail}`;
`NewDownloadView::notify_validation_failure`; `DownloadSettingsView::save`; `settings_toggle`.
Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch/search.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/download_page.rs`,
`repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`,
`repo:rust/apps/nxtools-desktop/src/ui/components/settings.rs`.
### 3. Contracts
Current nonempty local results publish even while index pending. Empty pending preserves rows;
final empty clears. Keep original 65ms coalescing, generations, IME and old-result execution guard.
Attached InputGroup/source rails remain36DIP each; root border adds2DIP to requested height.
Source row and wrapper use min-width0/full available width: child-in-overflowing-parent is not
root containment. Native placement/focus/readback remain original C# responsibilities.
Download secondary line uses live total then durable total, formatted size or `大小未知`, not
save path/URL. Actual image40DIP/fallback30DIP/Shell source48px retain48DIP row and selection edge.
Full failure remains discoverable in original status/tooltip/details. Single click selects only;
one-task PopupMenu `查看详情` defers current module/selection/task checks then creates the original
DownloadDetailScope. Scope gates detail IO/render; close retires presentation only, while changed
selection/filter/navigation/window retirement invalidates old work. No second inspector controller.
Explicit probe/submit/Save validation uses existing NotificationCenter, preserves draft and emits
no duplicate inline banner or validation-driven resize. Persistent native-layout/security/risk
Alerts stay. Original keyboard repeat/modifier/IME/generation/actual viewport gates remain.
Shared settings_toggle is released official Medium36x20 in a36DIP control lane; custom SettingField
content does not inherit parent sizing. Routine explanatory copy may be shortened, never critical
destructive/restore/trust/privacy disclosures. No payload diagnostics or new notification manager.
Catalog hint omission must preserve same-row action alignment without fixed-height tiles or empty
hint placeholders. Current ListItem has centered internal wrappers and no public content_style;
its root-level public suffix hosts the same official Buttons, bottom-inset4, with bodypb11 reserving
the32DIPaction plus12DIPgap at normalrem. Keep content-derived row height and original callbacks.
### 4. Validation & Error Matrix
Empty pending -> retain last rows, not false final empty. Stale query -> discard. Invalid draft ->
one shared notice/zero probe or submit effect/no growth. Repeated held Enter -> no duplicate notice.
Old task/menu/retired scope -> no detail admission or stale reply. Unknown size -> unknown, not0.
Native clipping/DPI unavailable -> report acceptance gap; TestApp geometry is not native evidence.
### 5. Good / Base / Bad Cases
Good: local command appears before queued fake indexer reply; real row context menu explicitly opens
original details. Base: empty pending keeps old protected rows; current final result replaces them.
Bad: reject all pending results, test only Kbd inside an overflowing row, or open detail IO on click.
### 6. Tests Required
Production panel pending/local/current-generation; whole source row/root/header/Kbd containment;
actual bitmap40x40 and selection edges; pointer selection/no scope -> real View Details menu/scope;
original late-reply/ABA/selection/module/window retirement; invalid draft notification count/draft/
zero effects/no resize; keyboard held Enter/IME/modifier guards; actual official Medium geometry.
Affected download/settings/Quick Launch aggregates, strict lint, workspace test compile and scoped
format/source gates are reported separately from native/physical/DPI/visual/user acceptance.
### 7. Wrong vs Correct
Wrong: `if pending { return false; }`. Correct: reject only `pending && update_results.is_empty()`.
Wrong: selected task automatically creates a detail scope. Correct: explicit deferred menu action
creates the original current scope; selection only maintains or retires it.

## Scenario: Changed summon-hotkey reset (2026-10-28)
### 1. Scope / Trigger
User-approved Clipboard/Quick Launch global summon shortcut reset, left of the existing shortcut.
### 2. Signatures
`ClipboardApplication::{clipboard_hotkey_is_custom,can_reset_clipboard_hotkey,reset_clipboard_hotkey}`;
`QuickLaunchApplication::{hotkey_is_custom,can_reset_hotkey,reset_hotkey}`;
`hotkey_field(...,state,reset:Option<Button>,on_click)` preserves the original recorder/status.
Evidence: `repo:rust/apps/nxtools-desktop/src/clipboard.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/hotkey_reset_tests.rs`.
### 3. Contracts
Compare semantic modifiers/key (case-insensitive key), never display text. Defaults come from Core:
Clipboard Alt+C; QuickLaunch empty modifiers+Control/doubleCtrl. Nondefault binding alone displays
official Medium32outline `重置` immediately left of recorder. Default binding has no reset element.
Original Settings/custom slot and right edge/status remain. No Toolbar.child variant adaptation.
Reset only admits fixeddefault, connected/no pending configuration/no recording. Controller rechecks
after deferred UI callback; duplicate/pending/recording/disconnect cannot dispatch. Recorded arbitrary
bindings still require acknowledged recording. Both inputs share private original Configure*/pending
candidate submission and CommandResult/section-save owner, not fake recording or a new transaction.
Only matching native success publishes/saves default; failure keeps prior binding and existing
status/retry. Applied-but-disk-save-failed remains original explicit partial success. Existing request
ID flows through IPC/native diagnostics; low-frequency submitted/completed events omit binding text.
### 4. Validation & Error Matrix
Display-only difference -> no reset. Changed -> left reset. Waiting/applying/recording/disconnected
-> visible disabled if changed, no effect. Unmatched ACK -> no commit. Rejected ACK -> unchanged
memory/disk and retryable reset. Successful ACK -> default memory/disk, reset unmounted.
### 5. Good / Base / Bad Cases
Good: one official pointer or focused KeyUp gesture sends one Configure*, then matching ACK saves.
Base: existing recorder/Escape/focus-out and default no-button geometry unchanged.
Bad: compare localized label, write default before ACK, or forge recording-ready to call set_hotkey.
### 6. Tests Required
Actual two Settings pages: default absence/display-copy independence; left same-row32DIPbounds;
pointer and focused Enter release; pending repeat/no optimistic save; wrong/rejected/success ACK;
recording-before/afterpauseACK/disconnect UI+owner guards. Original hotkey aggregate retained.
TestApp `simulate_keystrokes` dispatches key input without the official Button release; use actual
KeyDownEvent+KeyUpEvent for activation, assert no command before release. No vendor/key-dispatch fix.
Native registration/physical/DPI/visual/user acceptance remain separate from these fakeIPC tests.
### 7. Wrong vs Correct
Wrong: reset -> set recording flags -> set_hotkey(default). Correct: fixed-default admission through
original configuration owner, preserving the existing acknowledged arbitrary-binding recording gate.

## Scenario: Automatic archive preview and complete drag selections
### 1. Scope / Trigger
Release38 archive row/Tree drag, automatic member preview and download context-menu ordering.
Evidence: `repo:rust/apps/nxtools-desktop/src/archive/model_drag.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/browser_drag.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/browser_preview.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/preview.rs`,
`repo:rust/apps/nxtools-desktop/src/archive/ui_state.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/download_page.rs`.
### 2. Signatures
`BrowserModel::freeze_drag_selection_authorized` / `freeze_drag_tree` return complete frozen
descriptors or a member-specific String error. `preview_visible`, `observe_preview_selection`,
`sync_preview_selection`, `preview_selection_matches` and `suspend_preview` retain the original
generation/fingerprint seam. UiState has no enablement field; legacy show_preview is ignored.
`BrowserSlot.preview_lease: Option<Lease>` owns the original visible Ready lease. `decode_preview`
transfers it through `complete_preview(...,lease,cx)` only after actual prepare returns and the
original selection/generation owner still matches. `cancel_preview` takes/releases it once.
### 3. Contracts
Initially there is no right preview slot, handle or reserved width. A supported single-file
selection automatically adds the existing official resizable preview panel; no toolbar/View-menu/
pane-close opt-in. Conditional child composition avoids the locked component's hidden-slot size
accounting while retaining official Tree/details/preview handles. Keep the original controller,
cache, cancellation and passive document/image rendering. Empty/unsupported/multiple/directory
selection, tab suspend, source invalidation and close clear prior content and reject stale results.
Cross-entity dispatch is next-turn and rechecks the fingerprint; preview never moves list/primary
input focus. Name mode fills the measured viewport before/after preview and splitter changes.
Successful current Ready retains the sealed native lease until selection/A→B→A, refresh/codepage,
tab suspension, close or disconnect retires it. Failed/stale decode completion releases the exact
original operation/session/generation/lease once; lost application owner sends that release directly.
An in-flight decoder still owns its lease until actual completion, even after browser retirement.
No cache-hit callback synchronously re-enters a leased GPUI entity. The original native cache may
hold a separate drag consumer only for exact source/session/generation/snapshot/codepage/depth/MOTW/
ordinal and independently valid encrypted-drag authorization. Preview closure releases only its share.
See [retained demand runtime](../../nxtools-archive/backend/product-read.md#scenario-drag-scoped-retained-materialization).

Freeze files by original ordinal and directory descendants by original subtree. Missing optional
metadata, unknown size or resolvable case/name/file-directory conflicts do not veto selection.
Resolve descriptor names only, merge original directory keys case-insensitively and preserve every
selected file ordinal. Collision suffixes shorten only the presentation stem at character boundaries
to fit existing native name/path budgets; original paths and ordinals remain intact. No separate
root/descriptor/file/request-count or512MiB drag cap. Whole-source/path/kind/integrity, real envelope/
native representation, general extraction resources and encrypted authorization remain. Genuine
failure names the member/restriction, never drops items. Original held4DIP/source HWND/connection-
locked IPC/OLE/async demand and cleanup owners remain.

Official Kit0.8.0 Empty/EmptyHeader/EmptyTitle/EmptyDescription retains stage-specific state copy;
the public title child carries a nonfocusable semantic Label. Existing retry/source controls are
Small outlined Buttons outside Toolbar.child transformations. File byte limit, read resources,
text lines, image dimensions, decode memory and document limits are distinct results; native
resource failure does not imply an oversized selected file. Configurable member budgets follow
[the frozen preview budget contract](archive-preview.md); source/spool/solid-work limits stay separate.
Download PopupMenu builds original capability-driven actions first, then appends the original
single-selection View Details callback last, without a leading separator or enablement change.
Existing correlated preview/drag native lifecycle diagnostics remain; no frame/payload probes.
### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Initial/empty/unsupported/multiple/directory selection | No preview slot or reserved right-hand width |
| Legacy show_preview=false with supported single file | Automatic preview; retain unrelated layout |
| Old generation or hidden/closed tab | Clear current content, no late publication or focus change |
| Unknown estimates / collision warning | Complete representable descriptors, actual native accounting |
| Unrepresentable original path/link/special kind | Specific member/reason; whole selection rejected |
| Native read resource limit | Read-stage title, not file-size title |
| Actual byte/line/image/document limit | Corresponding stage-specific visible title |
| Current visible Ready | Retain original native lease; no eager Release after decode |
| Failed/stale/owner-lost decode | Exact original lease Release once; no stale Ready |
| Download single/multiple selection | Details last only where originally available |
### 5. Good/Base/Bad Cases
Good: file a plus A/x and a/y yields one renamed directory containing both original descendants.
Base: selecting an 80 B text file automatically shows its current passive document.
Bad: historical false hides preview, filename renaming changes content identity, or a generic
security message masks optional metadata/collision handling defects.
### 6. Tests Required
Production ListItem/Tree gesture-to-command, complete ordinal and deferred lifecycle regressions;
actual GPUI preview state titles, focus, tab/close/cancel, legacy loader/reopen, absent/revealed pane,
Tree splitter in details/name mode, complete selection and download menu callback. Real worker/native
IStream regressions are separate from simulated GPUI and physical Explorer/installed/DPI/user visuals.
`repo:rust/apps/nxtools-desktop/src/archive/opt04_preview_tests.rs` asserts real background prepare,
visible Ready with no Release, exact once-only selection/refresh/tab/close release, A→B→A stale
rejection and invalid-UTF8 failure. Preserve decode-barrier close and original password-overlay tests.
### 7. Wrong vs Correct
Wrong: default enablement true while persisting an off switch, reject OutputCollision before a
source-bound lease, or test only strings. Correct: no enablement state, original selection-driven
controller, presentation-only descriptor naming and executable owner-boundary regressions.
Wrong: decode success -> Release -> visible Ready. Correct: decode owns lease -> current Ready
slot takes ownership -> existing retirement releases; stale/failed completion releases directly.

## Scenario: Current Quick Launch result availability
### 1. Scope / Trigger
Current local, indexed and extension search results arrive while another provider is still running.
Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/search.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/extension_providers.rs`,
`repo:docs/reference-code/quick-launch-research.md`.
### 2. Signatures
`handle_indexer_notification` accepts current batches then calls original `refresh_indexed_results`
for nonempty nonterminal or terminal batches. `search_update_pending` tracks lack of usable current
rows; `IndexSearchSession.complete` separately tracks terminal provider completion.
`apply_extension_batch` publishes trusted current rows without importing retained prior-query rows.
### 3. Contracts
Current nonempty local/merged/extension results are immediately visible and actionable; do not wait
for all providers or their terminal batch. Keep the existing65ms input coalescer, matching/ranking,
final merge, provider execution revalidation and once-only close-on-acceptance behavior. During a
new query, retained old rows are visual continuity only: Enter, shortcut, pointer and action mode
cannot execute them. Empty current pending batches preserve that noninteractive presentation; current
nonempty batches replace it and enable only current rows. Old generation/session/Input, hidden or
closed windows, disconnect and retired extension providers retain their original guards.
Existing correlated provider/publication diagnostics distinguish row availability from complete;
no query/payload logging or per-keystroke diagnostic stream. Desktop readiness does not establish
indexed-service computation latency.
### 4. Validation & Error Matrix
| Condition | Required result |
| --- | --- |
| Current local row, index provider unfinished | Visible/actionable; complete remains false |
| Current nonterminal index batch has rows | Publish/rank now; real Enter/action works |
| Current empty pending batch / retained old rows | Keep readable prior content and first-row selection paint; no stale action |
| Current nonempty extension batch | Only current merged rows enabled, no retained ordinary rows |
| Old generation / closed panel / retired provider | No publication or action |
| Current terminal empty batch / disconnect | Original final-clear/error lifecycle |
### 5. Good/Base/Bad Cases
Good: a local command launches before index completion; a current received index row opens before
terminal transport. Base: terminal results merge with original ordering and selection identity.
Bad: accepted=1/visible=0 until complete, or enabling retained rows merely because a provider runs.
### 6. Tests Required
`search_latency_ready_indexed_rows_publish_before_terminal_batch` drives actual Input/coalescer/
command/nonterminal batch/list bounds/action mode/Enter dispatch and old-row rejection.
`ux_repair_real_panel_publishes_local_command_while_index_is_pending` uses actual pointer dispatch;
`real_panel_late_batches_keep_identity_and_current_input_retires_actions` covers extension freshness.
Retain existing generation, IME, close, ranking, current-selection and action regressions. Controlled
publication timings are not live service, physical keyboard or installed-product performance.
### 7. Wrong vs Correct
Wrong: search_pending means any provider has unfinished work. Correct: current rows establish
availability; complete tracks provider termination; stale retained rows grant no execution authority.

## Scenario: Quick Launch continuous refresh and default selection
### 1. Scope / Trigger
User-approved Listary-informed result hierarchy and refresh/selection correction.
Evidence: `repo:rust/apps/nxtools-desktop/src/quick_launch/panel.rs`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/retained_text.rs`,
`repo:docs/reference-code/quick-launch-listary-results.md`.
### 2. Signatures
Original `QuickLaunchPanel::{schedule_search,run_search,replace_results,select_result,move_selection}`
owns `selection_explicit` and `committed_query`; existing immutable `QuickLaunchResults` owns display text.
### 3. Contracts
New input resets default selection to rank zero before dispatch. Default selection follows the first
ranked row across current batches. Explicit pointer/keyboard choice follows identity within that query,
including choosing the already-selected first row; disappearance falls back to default-first.
Extension publication delegates selection to the same panel, without a second selected-ID restore.
Retained prior-query rows keep normal foreground, committed-query highlights and first-row background;
they remain disabled for execution, action mode and preview. Pending arrows/shortcuts cannot move the
old selection. Current nonempty results stay immediately usable under the availability contract above.
Application rows without explicit subtitles render one centered16DIP title; other rows use15DIP
titles and12DIP parent-path/description text. Normal launcher bodies are52DIP with32DIP icons;
compact action and attached-dialog metrics remain40DIP. Retain4-or0external gap and620DIP width.
Launcher search header is58DIP with official Input Large44DIP,26DIP text/34DIP line,4DIP vertical
padding and24DIP search icon. Keep the original InputState and single visible input frame. Public
Input::with_size(Large) supplies the single-line tier; Input::h is multiline-only in this release.
Official Label middle ellipsis preserves full retained strings, accessible labels, tooltip and targets.
Ordinary typed search uses a stable620×530DIP expanded client with the existing470DIP results viewport
and monitor clamp. Pending/empty/error/current provider completion and result counts do not resize it;
the existing error footer stays inside that budget. Empty input/hide/reopen use60DIP search-only;
action and recent-directory modes retain their own geometry. The original panel observer schedules
native sizing outside render/owner leases. Existing C# SetWindowBounds/SwpNoActivate/readback,
one correlated in-flight resize, origin and preview docking remain. Hidden/retired callbacks cannot
replay a size. No native bounds animation or new protocol.
`QuickLaunchResultEntrance::{SlideUp,SlideFromRight,Fade,None}` persists through the original
section-owned saver; missing defaults SlideUp, failed save leaves live state unchanged. Released
Base motion::Timing/Easing::EaseOut supplies120ms finite content-only Tween,8DIP up or12DIP left
travel with fade. Each identity is admitted once per query generation, including delayed batches;
offscreen arrivals settle immediately. Stable row hit/focus/selection rails remain. Repaint, reorder,
icon completion, selection and virtualization do not replay motion. New query/hide/close/disconnect,
None and reduced/unknown policy settle immediately and retain no frame/timer owner. Diagnostics
remain bounded generation/count/state events; no query/payload/frame logging.
### 4. Validation & Error Matrix
New query -> default-first/old actions inert; current reordered batch -> rank zero unless explicit;
explicit identity removed -> first; pending empty -> retained paint; error -> existing status and
old-row rejection; app subtitle absent -> no second line; explicit subtitle -> preserve verbatim.
### 5. Good/Base/Bad Cases
Good: a better late result becomes default-first but cannot steal a deliberate choice. Base: ordinary
Enter opens a current result once. Bad: gray/clear every retained row or preserve default identity
across queries, or enable stale rows solely to avoid their disabled appearance.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/quick_launch/refresh_tests.rs` drives actual Input, batch replacement,
explicit first/keyboard selection, pending selected paint and application/file row bounds; also
0/1/many publication sizes, overflow last-row visibility, error footer, Unicode middle truncation,
both gap styles and coalesced native command/closed-session behavior. TestWindow resize requires
simulate_resize to deliver the native viewport event and device-pixel assertions use its real scale.
Retain old-row Enter/shortcut/pointer/action rejection, extension freshness, IME and focus tests.
Native-query resize tests reject count/provider-driven geometry changes and verify stable first reveal.
Tween tests cover late batches, once-only identity admission, selection/reorder/icon/scroll, retirement,
reduced/None modes, real paint offsets/opacity and finite idle settling. Header remains58/44DIP with
original input/focus and unclipped34DIP text.
Compiled synthetic paint/command boundaries are separate from physical HWND/DPI/user acceptance.
### 7. Wrong vs Correct
Wrong: every selected index means deliberate choice, and pending removes selection paint.
Correct: explicit intent controls identity preservation; pending controls execution, not text opacity.

## Scenario: Asynchronous download batches and destination conflict actions
### 1. Scope / Trigger
Download list batch/menu actions, existing delete confirmation and failed task windows. Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/batch.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/conflict.rs`, `repo:rust/apps/nxtools-desktop/src/main_window/download_page.rs`.
### 2. Signatures
`DownloadApplication::{batch_action,batch_busy,batch_loading,resolve_destination_conflict}`; `DownloadApplicationEvent::SelectBatchFailures { task_ids }`; `BATCH_DESTRUCTIVE_CONFIRMATION_THRESHOLD=20`; engine `request_batch_background` and `RetryWithAutoRename` are the only new dispatch seams.
### 3. Contracts
Install one correlated pending batch before original runtime blocking dispatch; duplicate submissions are ignored until actual completion. Completion crosses cx.defer, validates operation ID, clears busy, refreshes the authoritative list and defers one NotificationCenter summary. Failed-only action defers an application event, then MainWindow defers mutation of the original selected-ID set. No banner or extra notification manager.
Official Small icon Buttons keep ToolbarGroup.content composition and loading/disabled; related PopupMenuItems use official disabled semantics. Multi-selection exposes separate list-only Remove and file-deleting RemoveWithArtifact entries in the original toolbar/menu; the file-deleting entry follows task_removal_deletes_file. Cancel and RemoveWithArtifact at least20 deduplicated targets reuse DeleteDownloadView and its keyed native-close lifecycle with a quantity-labelled title. Closing/cancelling confirms nothing; list-only Remove never confirms. A batch confirmation cannot reuse a single-file action payload. Native close releases busy and notifies the original owner.
Before opening a destructive batch confirmation, close every target's existing single-item confirmation, not only the first target's window. Preserve unrelated single-item confirmations and the existing keyed native-close identity checks. Cancelling or completing the batch leaves no target-bound single-item confirmation behind. `batch_feedback_retires_every_target_confirmation_but_keeps_unrelated_window` drives actual Cancel/Confirm buttons with first/nonfirst target windows and one unrelated window.
Typed destination conflicts display explicit same-name reason and a separately truncated/full-semantic filename. Original failed440x196 client, fixed footer, keyed generation, placement and activation owners remain. Official Small primary numbered retry and outlined Cancel are deferred to the engine; ordinary nonconflict failure actions remain. Conflict presentation/choice/result share task-derived correlation identity with the retry command; diagnostics exclude names, paths and request payloads.
The user-approved2026-11-01 final U2 scope is numbered retry and Cancel; it supersedes the proposal's overwrite recommendation. The current conflict flow never replaces existing destination bytes.
### 4. Validation & Error Matrix
Busy/duplicate -> no second dispatch.19destructive targets -> no batch confirmation;20 -> one existing confirmation. Cancel/X -> no task mutation and busy released. Partial result -> successes retained, exact failed count and failed-only action. Worker/dispatch failure -> terminal busy cleanup, every original deduplicated target remains selectable through the existing notification ingress. Long filename/error -> body truncation/scroll, footer never displaced. Late batch result cannot retire a different operation.
### 5. Good/Base/Bad Cases
Good:300 actual removals await SQLite off UI while the next production window turn runs; final list equals engine. Base: one list-only removal has no confirmation. Bad: async syntax around blocking GPUI work, notify/update another leased owner synchronously, or label queue acceptance as completed file publication.
### 6. Tests Required
`downloads::batch_tests` covers real-store wait/production window, duplicate actual work,19/20confirmation cancellation and partial-result feedback; `downloads::batch::tests` covers whole-batch rejection/invalid-result failed-only actions. `main_window::download_page` checks official busy controls, production file-deleting toolbar/menu and failed-notification selection on original rows. `downloads::conflict_tests` exercises Light/Dark440x196 real view bounds, long filename semantics and actual persisted-conflict retry/cancel clicks without network. Compiled/TestApp results are not screenshots, physical input, native DPI, installed or user visual acceptance.
### 7. Wrong vs Correct
Wrong: UI `handle_command(Batch)` followed by synchronous toast/entity updates. Correct: pending ID -> original spawn_blocking -> next-turn matching result -> original refresh and deferred NotificationCenter/selection action.

## Scenario: Quick Launch observable keyboard-hook recovery
### 1. Scope / Trigger
PlatformHost observes hook install failure, owned-thread exit or reinstall; Quick Launch displays
the new status without changing its original summon/recording/persistence controller.
### 2. Signatures
`QuickLaunchKeyboardHealthChanged {correlation_id,state,native_error_code}`;
`QuickLaunchApplication::{keyboard_health_warning,reenable_keyboard_hotkey}`;
`PlatformBridge::quick_launch_panel_hotkey_available`; `Settings -> SettingPage -> SettingGroup -> SettingItem::render`.
### 3. Contracts
Only enabled double-Control outside recording shows warning; one deferred NotificationCenter
notice belongs to each unavailable episode. If a fault arrives during recording, retain it
without notifying; when the native recording-stop ACK makes double-Control relevant again,
show Alert and notify once. Repeated failures do not restart that notice; recovery resets
eligibility. Original ConfigureQuickLaunch builder/pending owner registers caller ID before
enqueue, matches receipt before clearing. No hook-loss polling. Released Component0.8.0
`Alert::warning(id,reason).title("双击 Ctrl 已失效")` uses Small size and numeric native code,
not arbitrary exception payload. Direct child of layout div
`Button::new(id).label("重新启用").outline().with_size(Size::Medium)` stays outlined, official
32-DIP tier. `.loading(bool).disabled(bool)` matches actual retry admission while either the
automatic ConfigureQuickLaunch or a manual retry is pending. No Toolbar.child ghost adaptation.
### 4. Validation & Error Matrix
`reinstalled`/matching successful retry hides Alert; an intervening health event advances the
revision and an older retry result cannot clear its later failure. A successful ordinary
ConfigureQuickLaunch clears a stale Hello capability warning only, never a newer concrete
native failure. Failure keeps retryable warning; wrong ID is ignored. Disconnect takes an
in-flight retry before retiring state and emits one `quick_launch.keyboard_reenable.completed`
terminal with its original correlation ID, `success=false`, `failure_kind=disconnected` and
elapsed milliseconds (no native code exists for disconnection). A completed result must not
produce a second terminal. Reconnect uses Hello capability snapshot, not guessed hook state.
Older desktop rejects unknown event and closes that pipe; host emits the additive event only
after HelloAck accepts `quick_launch_keyboard_health_v1`.
### 5. Good/Base/Bad Cases
Good: actual Button click enqueues one ConfigureQuickLaunch and matching success hides Alert.
Base: original hotkey reset/recording and custom/disabled settings remain authoritative.
Bad: pretend the hook is healthy from a nonzero handle, infer silent removal, or enqueue twice.
### 6. Tests Required
Protocol roundtrip/unknown event rejection, IPC negotiation and generation-scoped Hello snapshot,
production GPUI/TestApp Alert/real click/duplicate/wrong/failure/reconnect lifecycle, plus
automatic-configuration busy-button parity, recording fault -> native-stop ACK -> Alert and
one notice, and captured correlated single disconnect terminal. Owner tests also cover
Reinstalled -> ThreadExited -> delayed success and ordinary auto configuration recovering
Hello-unavailable without overwriting a newer native failure. Evidence:
`repo:rust/crates/nxtools-protocol/src/lib.rs#QuickLaunchKeyboardHealthChanged`,
`repo:rust/crates/nxtools-ipc/src/lib.rs#handle_connection`,
`repo:rust/apps/nxtools-desktop/src/quick_launch.rs#reenable_keyboard_hotkey`,
`repo:rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs#render_quick_find_control`,
`repo:rust/apps/nxtools-desktop/src/main_window/tests.rs#quick_launch_keyboard_health_auto_configuration_busy_matches_retry_button`,
`repo:rust/apps/nxtools-desktop/src/quick_launch/tests.rs#u3_followup_recording_fault_notifies_once_when_double_control_resumes`.
### 7. Wrong vs Correct
Wrong: second hotkey controller, synchronous cross-entity notice, stale ACK clears new state,
or a clickable button whose owner rejects a pending automatic configuration.
Correct: existing owner + next-turn episode notification + matching visible busy state and
original caller-owned pending ID closed by either result or disconnect.

## Scenario: Per-volume index maintenance in Settings
### 1. Scope / Trigger
Optional index maintenance projection in Quick Launch Settings → Index. Popup620×410, placement, focus, show/hide and ranking remain unchanged.
### 2. Signatures
`volume_scope_row(theme,cx,selector,title,metadata,VolumeScopeRowState,on_click)` uses the original row/selection/save owner. `VolumeScopeRowState` carries maintenance, checked and show_divider; maintenance is the protocol's optional typed value.
### 3. Contracts
Official Component0.8.0 Small `Tag::info/warning/success` owns status visuals; its public child contains official Label with matching official semantic foreground,10DIPtype/14DIPline height. Layout wrappers expose the same drawn text via Label role/aria_label/TestSupportExt. Status labels are 首次建立/正在重建/就绪. Rebuild detail includes 正在重建，搜索结果可能不完整 and the typed readable reason; tooltip retains full detail. Original40DIProw, checkbox, focusable single action and deferred save remain. Missing maintenance retains196DIPlegacy metadata rail; present state shares title/count/Tag plus optional detail. Ready empty volumes show0项 rather than 尚未建立索引.
### 4. Validation & Error Matrix
None -> legacy copy/layout; InitialBuilding -> informational Tag; Rebuilding -> warning Tag plus reason, never error treatment; Ready -> success Tag including empty snapshot. Actual faults retain the original error presentation. Long detail truncates inside its row with full tooltip.
### 5. Good / Base / Bad Cases
Good: wrapped journal shows 正在重建，搜索结果可能不完整 · 日志已覆盖 while last-good remains available. Base: old omitted-field status keeps original geometry. Bad: custom painted badge, new popup strip, or inferred Ready from item count.
### 6. Tests Required
`u4_index_settings_renders_per_volume_maintenance_within_minimum_width` renders real MainWindow Index Settings at920×480 in Light/Dark, asserts mounted Tag/description containment and actual text snapshots, scrolls the original viewport and pointer-toggles one original volume command. Seed a temporary config file so the original durable section-save can admit synchronization. Retain automatic/explicit selection, keyboard and ordinary-rem gates. Native DPI/accessibility/user visual acceptance is separate.
### 7. Wrong vs Correct
Wrong: call a controller directly and call it a production-page interaction test. Correct: render the original page, expose the row inside the actual window, send pointer input, and assert exactly one original saved settings command.

OPT-07 browser presentation follows [browser workflows](archive-browser-workflows.md): original Root Dialog child and fixed DialogFooter, stable query InputState, Checkbox::on_click, official virtual ListItem and Medium outlined actions. Query/RowId/final-submit and preview/readiness transition caches preserve the original lifecycle above. Unified batch/details/password decisions follow [batch consent](archive-batches.md#scenario-opt-07-unified-surface-and-protected-consent); controlled bounds/callback evidence remains distinct from physical/user acceptance.

## Scenario: Compact new-download intake and one-shot clipboard prefill
### 1. Scope / Trigger
User-approved2026-11-05 URL-only Add Download window. ABDM EnterNewURLPage -> BaseEnterNewURLComponent.onPageOpen is the first-open/single-link reference; legacy retains explicit Continue/probe and confirmation. See [download reference](../../../../docs/reference-code/download.md).
### 2. Signatures
`PlatformCommand::ReadClipboardText` encodes `{"command":"read_clipboard_text"}` with no payload. C# `ClipboardTextReader.ReadOnSta(CancellationToken,Func<string>?)` runs on the original BoundedStaOperationRunner; standard CommandResult carries success/error/native_error_code/data. `DownloadApplication::{request_new_download_clipboard,handle_new_download_clipboard}` and `NewDownloadView::{clipboard_prefill_eligible,prefill_clipboard_url}` retain the original owners.
Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/clipboard_prefill.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`, `repo:csharp/NxTools.PlatformHost/Windows/ClipboardTextReader.cs`.
### 3. Contracts
Initial client560x136 supersedes compact560x176 only. One official0.8.0 Medium InputGroup/default32px frame contains the retained URL InputState, leading official Link Icon and trailing XSmall ghost InputGroupButton24px. Idle guidance and separate label rail are absent; actual probing/error state retains bounded body scrolling and the original fixed44px footer. Continue/Cancel are adjacent on the right. Confirmed Form/Field, ordinary/media clients, native hidden-target handoff, placement, activation and deferred focus remain unchanged.

The leading Link uses the complete official assets enum but is not in Kit0.8.0's default component asset subset. Compose the selected released `icon_assets!` source with NxToolsAssets/default Assets; do not embed the full catalog, alias another glyph or delete the official addon inset to conceal a missing SVG. The production loader and prefix-click/Home insertion regression must both pass; enum compilation and geometry alone do not prove icon visibility.

Only creation of a manual blank window requests one read; reuse merely activates it. Install pending caller ID before dispatch. Native reads Unicode text on one bounded STA with2second logical wait; maximum65536UTF16code units keeps worst-case JSON escaping within the existing1MiB envelope. No truncation, watcher, history lookup, setting dependency, clipboard writes or automatic retry. Rust waits at most3seconds and retires the matching request only. Read failure is quiet; manual entry/paste remains available.
Rust chooses exactly one valid HTTP/HTTPS candidate, preserving its text; no link or multiple links leave the input untouched. Prefill never probes/submits/downloads. URL InputEvent::Change increments an edit revision, including edit-then-clear. Deferred result application checks the original window/weak view, zero edit/probe generation, blank URL, no external source/probe and no active handoff/probe/submit. Closed/reopened/replaced drafts and disconnected commands cannot receive stale text.
Diagnostics share the caller ID: Rust download.clipboard_prefill.started/finished records fixed outcome and elapsed; original C# command.received/completed retains native error/elapsed/has_data. No clipboard text, URL or request context is logged.
### 4. Validation & Error Matrix
Single link -> fill once/no work; empty/nonlink/unsupported scheme/multiple links -> no fill. Native busy/failure/too-large/cancel, queue refusal, timeout and disconnect -> retire/quiet/manual input intact. Changed revision, probe/handoff or original-window retirement -> no write. Repeated Open -> same window/no second read. Browser/Blob supplied source -> no automatic clipboard read and original source/context preserved.
### 5. Good / Base / Bad Cases
Good: copied link fills the original focused input while Continue remains explicit. Base: unavailable clipboard leaves a compact blank form. Bad: select an arbitrary link, overwrite an edited-then-cleared draft, read capture history instead of current clipboard, or auto-submit from a clipboard result.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/downloads/new_download_intake_tests.rs` drives original Open, typed request/result, stable Input entity, reuse, close/reopen, actual edits and failure/disconnect with zero task/probe effects; production bounds assert136px, absent idle guidance and adjacent right-side actions. Existing14/16/18Root, keyboard/IME, manual paste, spinner/reduced-motion, confirmation/media and fixed-footer tests remain. `repo:csharp/NxTools.PlatformHost.Tests/ClipboardTextTests.cs` injects synthetic text on the actual bounded STA and checks cancellation, HRESULT/privacy, empty and maximum/over-maximum text plus detached command-reader behavior. Protocol roundtrip checks the no-payload command. Controlled fixtures do not establish live clipboard/DPI/provider/installed or user visual acceptance.
### 7. Wrong vs Correct
Wrong: read on every render/activation and call begin_probe after filling. Correct: one original-window creation -> pending ID -> C# read -> next-turn eligible original InputState update, with explicit Continue unchanged.

Continuous-frame assertions advance the official InputGroup `duration_fast` on the TestApp executor before sampling; its border/background state transition uses `background_executor().now()`. Reduced motion remains immediate, inactive surfaces must settle, and only the active probe Spinner may sustain frames. Product motion tokens and vendor code remain unchanged.

## Scenario: Download-category extension pills
### 1. Scope / Trigger
User-provided removable wrapping extension-rule reference; existing selected-category editor only.
### 2. Signatures
`DownloadCategoryEditor::{extension_input,extension_editing,extension_values}`;
`DownloadSettingsView::{category_extension_field,subscribe_category_extensions,defer_extension_edit}`.
Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/category_extensions.rs` and
`repo:rust/apps/nxtools-desktop/src/downloads/settings_view.rs`.
### 3. Contracts
Existing Settings/Form/Field owns the surface. Public0.8.0 secondary outlined rounded Tag/Medium
hosts an official XSmall ghost removal Button; Small outlined Add and32DIP Input retain their
official focus/actions. Tags wrap/scroll in a bounded area; category action/footer remains reachable.
Original unsaved extension InputState remains authoritative; pending new input is a stable entity.
Enter/button confirmation strips leading dots/folds ASCII/deduplicates and retains compound suffixes.
Escape/Cancel discards only pending text and returns Add focus. Explicit Apply includes pending text;
close does not write. Deferred input writes run outside the leased settings owner, bound to current
category ID/value and saving state. URL-rule AND semantics, category order/directories and future-task
classification remain unchanged. Local edits do not introduce IPC or payload diagnostics.
### 4. Validation & Error Matrix
Duplicate/empty -> no duplicate or empty rule. Removed/deselected category or saving -> no stale
mutation. Save failure -> original retained draft/error; no new persistence owner. Many rules ->
real bounded scroll, not an enlarged window or clipped actions.
### 5. Good / Base / Bad Cases
Good: .ZIP and zip add once; tar.gz remains one rule; remove changes only the unsaved category.
Base: original explicit Save/Cancel and rule matching. Bad: custom painted close control, mutation
inside an input callback that reenters its entity, or tag edit immediately persisting settings.
### 6. Tests Required
Production category_extension_* tests cover pointer removal/no autosave, real scroll-to-Add,
Enter/dedup/compound suffix/pending Apply projection, Escape and saving-disabled removal. Original
category layout/search and settings picker/Apply/Cancel/failed-persistence/sort preservation remain.
Compiled scene assertions do not certify installed, physical/DPI or user visual acceptance.
### 7. Wrong vs Correct
Wrong: a debug rectangle means an actionable visible button. Correct: ensure the actual control
lies in both the form viewport and window content above its fixed footer, then click/type and verify
the original draft without disk effects. Reference: [download](../../../../docs/reference-code/download.md).

## Scenario: Link-panel current settings govern ordinary activation (B1 REF-01)
### 1. Scope / Trigger
User-approved ordinary-row routing supersedes the historical settings-independent row action. Explicit named Copy/open and local target behavior retain their original owners.
### 2. Signatures
`ClipboardApplication::dispatch_link_panel_route(route,target,target_kind,value_length,ordinary,cx)` reads current `config.clipboard.link_panel_copy_only` / `link_panel_auto_open_browser` only for ordinary activation. `forget_link_panel_if_current(window)` guards deferred callbacks. Existing setters and clipboard-section persistence remain authoritative.
### 3. Contracts
Copy-only wins for Web/file/directory. Otherwise ordinary Web activation with auto-open disabled copies without a Shell/browser command; enabled Web uses original Shell boundary. Local file reveal/directory open remain unchanged when copy-only is false. Explicit Copy always copies; named 打开链接/打开文件位置/打开文件夹 bypass automatic settings and do not write the clipboard. Production official ListItem pointer and Enter/Space use the same route, not a render-time settings snapshot. Close the owning window locally, then defer cross-entity application dispatch; no synchronous leased-view reentry or direct native launch in GPUI. A retired window's callback cannot clear a reopened panel. Reuse, focus, timers, IME guards, placement and generation retirement remain original owners.

Composition stays locked Kit0.8.0 ListItem/Label/StatusBar with official XSmall ghost Copy/open Buttons, distinct accessible names, flex-shrinking text and original420DIP panel/28DIP row. No Toolbar child adaptation, download action or painted ordinary-control substitute. Low-frequency copy/open lifecycle events retain correlation, target kind/length, fixed state/reason and elapsed/native result code without clipboard text, URL or path.

Explicit Copy emits copy_started before local close, then exactly one same-correlation terminal in the original deferred callback. Current or ownerless Copy writes once and emits copy_completed; an upgraded owner's stale window guard leaves clipboard/window/platform commands untouched and emits copy_cancelled with fixed reason=panel_replaced and elapsed_ms. No payload, extra deferred turn, retry or synchronous cross-entity update.
### 4. Validation & Error Matrix
copy_only=true -> clipboard bytes, zero open commands. copy_only=false + Web auto_open=false -> clipboard bytes, zero browser commands. Local targets -> original typed Shell action. Explicit named open -> original action independent of automatic flags. Stale deferred close/action -> no replacement mutation. Failed settings persistence -> existing error owner, no storage replacement.
### 5. Good/Base/Bad Cases
Good: settings change while the panel is open is honored at the actual click. Base: non-copy-only local file reveals and directory opens. Bad: row captures stale booleans during render, auto_open=false still launches a browser, or an old callback clears a new panel.
### 6. Tests Required
Production ListItem pointer and Enter/Space matrix asserts actual GPUI clipboard bytes and typed platform commands, not source tokens. Explicit actions, local targets, focus/reuse, persisted current settings and stale close/reopen are covered by ref01_ and existing clipboard/link-panel lifecycle tests. Ordinary-row and explicit Copy regressions establish the replacement in the real window-closed observer before deferred copying; assert sentinel clipboard, replacement window/application handle and no platform command afterward. `clipboard_link_copy_tests.rs` captures actual tracing for normal/stale/ownerless explicit Copy: three starts, exactly one same-ID completed/cancelled terminal per start, numeric elapsed, fixed stale reason and no synthetic payload. Do not assume simulate_click returns before deferred work drains. Source Kit/editor gates and compiled fixtures do not certify physical Windows clipboard, actual browser/Shell, IME/mixed-DPI or user visual acceptance.
### 7. Wrong vs Correct
Wrong: row kind alone determines browser launch; rendering leases update ClipboardApplication synchronously.
Correct: next-turn current owner settings select copy/open after current-window validation; explicit actions remain separate.

## Scenario: Compact download progress and category header
### 1. Scope / Trigger
User-approved filename hierarchy, primary transfer information, short colored response timing and
one-row category selection/rules actions. Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/task_window.rs`,
`repo:rust/apps/nxtools-desktop/src/downloads/settings_view.rs`,
`repo:docs/reference-code/download.md` and [server information](../../nxtools-download/backend/server-information.md).
### 2. Signatures
`task_window_filename`, `server_information_value`, original `details_expanded`;
`DownloadSettingsView::{sync_category_select,current_draft}`, official `SelectState`/`SelectEvent`.
### 3. Contracts
Keep440×184 collapsed/440×280 expanded,34DIP title and44DIP fixed footer. Filename14/20 exceeds12/16
body metrics; bounded full-name Tooltip remains. Official XSmall Progress/Small DescriptionList shows
bytes/speed/ETA without duplicate status. Host plus short route and `{N} ms` show the existing HTTP
request-to-response-headers observation:0–100success,101–300warning,301+danger. Missing observation
has no time. No peer IP in visible/accessibility/Tooltip/diagnostic output and no new network probe.
Resume/limit/connections/segments live in the original disclosure above its Table, not the collapsed
body. Original task-ID entity/HWND/generation, cancellation, completion transform and focus stay.
Category heading/help/real Medium32DIP Select/Small outlined Add share one wrapping header;260DIP
menu width/220DIP maximum height. Stable IDs mirror selected_category, never become another owner.
Input-driven synchronization defers beyond current entity lease; current_draft/Apply/Cancel remain.
### 4. Validation & Error Matrix
Unknown time -> no time element. Boundary100/101/300/301 -> exact semantic role. Long filename ->
bounded title/Tooltip and reachable footer. Narrow settings -> wrap/no clipped actions. Rename/order/
delete -> current stable-ID choice; Cancel/close -> no persistence; failed Apply -> retained draft.
### 5. Good/Base/Bad Cases
Good: filename stands above readable bytes/speed/ETA and one click exposes all part details.
Base: one original keyed progress window. Bad: permanent technical wall, raw IP, fake Ping, dropdown
substitute instead of Select, second saver or synchronous input-entity reentry.
### 6. Tests Required
download_layout_/server_info_ and original audit06_aux_task_details_last_segment cover actual render
bounds, typography, timing/no-IP, Select popup/scroll/draft,14/16/18Root, disclosure/cancel and footer.
Native/physical/mixed-DPI/user visual acceptance remains separate from controlled fixtures.
### 7. Wrong vs Correct
Wrong: enlarge whole window or discard technical data. Correct: fixed budget, primary hierarchy,
original disclosure and durable settings owner.

## Scenario: Selection-named archive creation and aligned settings
### 1. Scope / Trigger
User-approved ZIP/7z quick creation, separate editable New Archive, compact sources and Download-like
settings composition. Evidence: `repo:docs/reference-code/archive-polish.md`,
`repo:rust/apps/nxtools-desktop/src/archive/forms_layout.rs`,
`repo:rust/apps/nxtools-desktop/src/main_window/archive_page.rs`.
### 2. Signatures
`ArchiveMenuCommand::{CreateZip,CreateSevenZ,Create}`; ExplorerCommandHandler GetTitle/Invoke;
original Desktop archive intent, CreateDraft and creation Job.
### 3. Contracts
Fresh/missing fields recommend all three commands; explicit empty/custom/historical five-item arrays
remain exact. Integration stays disabled until explicit apply. Ordered selection supplies first-file
stem or folder name plus fixed extension; complete sources enter one original Job with fixed ZIP/7z,
no editable form. Editable Create remains separate. Correlation crosses helper/intent/native Job;
events carry count/state/elapsed/native codes, not paths. Existing validation/confirmation/cancel/
publication owners stay. Source rows scroll independently within144DIP; summary/output/footer remain.
Official Settings/Page/Group/Item use Download's filled groups and shared aligned rows,200-design-pixel
ordinary control lane/164-design-pixel sidebar seed. Original page/callback/section-save owners remain.
### 4. Validation & Error Matrix
Explicit selections -> preserved. Partial unreadable/retired selection -> no Job/form. Multi-source ->
one ordered job. Saved different format -> quick command fixed format. Long source list -> bounded
scroll/remove/footer. Settings failure -> original error owner and previous durable configuration.
### 5. Good/Base/Bad Cases
Good: selected sources create selection-name.zip without opening a form. Base: New Archive remains
editable. Bad: static Registry text masquerades as a dynamic title, selection is truncated, saved
choices are overwritten or only settings labels change while grouping/alignment stays inconsistent.
### 6. Tests Required
archive_polish_ Core/Protocol/Desktop and C# fixtures exercise defaults, parser, actual GetTitle/Invoke,
whole selection, original Job correlation, real source removal/footer and settings row bounds/callback.
Real Explorer/Registry/installed/physical/DPI/user acceptance requires separate evidence.
### 7. Wrong vs Correct
Wrong: parallel creation engine or ordinary painted settings controls. Correct: original Job/draft/save
owners with released official compounds and selection-aware Explorer commands.

## Scenario: Explicit link-panel download intake (B2 REF-04)
### 1. Scope / Trigger
Named 使用 NxTools 下载 for admitted HTTP(S) links. This extends B1's two-action panel, not its ordinary-row copy-only/auto-open routing or independent Copy/Open.
### 2. Signatures
`ClipboardApplication::{bind_downloads,take_link_panel_download_owner}`; `DownloadApplication::{admits_link_download,open_link_download,open_new_download_window_with_request}`; `NewDownloadInitialSource::Url`; original `NewDownloadView::{clipboard_prefill_eligible,prefill_clipboard_url}`. Evidence: `repo:rust/apps/nxtools-desktop/src/tools/link_panel.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`.
### 3. Contracts
Keep420DIP panel/28DIP rows, released ListItem/Label/StatusBar and XSmall ghost named Buttons with real name/action hit bounds. Local close precedes deferred original-panel validation and original DownloadApplication update; no synchronous leased-owner bounce. URL-only source fills the original InputState without browser auto-probe, clipboard read, task or HTTP work. Original Continue/confirmation, placement, focus/IME/paste and fixed handoff remain. Reuse one original Root/view/window; next-turn fill rechecks current window and pristine blank draft. Populated/edited-then-cleared/external/active drafts are preserved; explicit URL retires the old blank-window clipboard request. Carry original action correlation into native window commands; started/finished diagnostics have fixed outcome/elapsed, never URL/path/text.
### 4. Validation & Error Matrix
HTTP(S) -> URL-only intake. Local/unsupported target -> no action/rejected owner input. Duplicate -> activate/reuse, no overwrite of dirty draft. Replaced/closed source or missing owner -> retired terminal/no new intake. Late clipboard reply -> no draft mutation. Window failure -> original notification and fixed failed outcome.
### 5. Good / Base / Bad Cases
Good: actual named action fills one original input and awaits Continue. Base: duplicate preserves an existing draft. Bad: route through Download source's auto-probe, replace the original form/controller, or dispatch a retired panel action into another draft.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/downloads/new_download_link_tests.rs` drives actual pointer/semantic Tab/Enter/Space, stable input/window, localhost no-HTTP and engine no-task observations, pending clipboard retirement, user edit/clear, close-observer replacement and local/unsupported targets. Keep B1 REF-01/Copy and original intake suites. Current source/owned-fixture results: `repo:.trellis/tasks/11-10-reference-b2-intake-control/ref04-outcomes.md`; native/physical/IME/DPI/provider/visual/installed acceptance is separate.
### 7. Wrong vs Correct
Wrong: Some(Download(request)) followed by automatic begin_probe. Correct: Url-only source on the original owner, one deferred identity/draft-guarded fill and explicit Continue.

## Scenario: Deliberate speed-limit editing (B2 REF-06)
### 1. Scope / Trigger
Original global settings and expanded task-window limit inputs. Evidence: `repo:rust/apps/nxtools-desktop/src/downloads/speed_limit_edit.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs`.
### 2. Signatures
`SpeedLimitEdit::{new,changed,begin,finish}` is presentation-only. Existing `DownloadApplication::set_global_speed_limit` and `DownloadTaskAction::SetSpeedLimit { bytes_per_second: Option<u64> }` remain command owners. Official Base0.8.0 `InputEvent::{Change,PressEnter,Blur,Focus}` and public `input::Escape` use the stable original InputState.
### 3. Contracts
Change edits only local draft. Complete Enter/Blur parses trimmed u64 KiB with checked multiplication;0 maps to original None/unlimited. Pending UUID deduplicates Enter followed by Blur. Escape restores applied text after original input overlay/IME handling. Cross-owner submission/result work is deferred; task commands require original task/window/view/generation, and global commands require original settings window. Commit original SQLite intent before original limiter/event publication. Task/global/runtime minimum ceilings and waiter signalling stay unchanged. New task windows seed applied limits through existing full engine.task, because compact summaries omit request-only limits. Official Medium bordered96DIP task Input, Styled Label and explicit units/0-unlimited arrange inside original expanded440×280 geometry; collapsed440×184/title/footer/segments stay original. Diagnostics share the command UUID: edit_started then edit_applied/edit_failed/edit_cancelled, fixed scope/outcome and elapsed; no input/URL/path/credential/raw error.
Positive-byte command/config limits display ceil-KiB without becoming unlimited. An untouched Settings field preserves exact `SpeedLimitEdit.applied` bytes during unrelated Apply; only explicit edits convert checked KiB back to bytes. Every original task-window update reads current cap/probe from full engine intent outside render while keeping original compact status/progress/artifact and phase ownership. Compact omitted None never clears applied intent; unavailable full reads retain the last applied rate. The effective label and clean stable input follow current applied authority and external typed commands; dirty/pending/error text stays local. Base0.8.0 `InputState::set_value` emits no Change, so clean display synchronization is not a user edit.
### 4. Validation & Error Matrix
Empty/noninteger/negative/overflow -> local error, no command/applied change. Unchanged -> no-op. Persistence failure -> previous applied rate, retained draft/error/retry. Retired window -> cancelled terminal, no task mutation. Successful commit -> original durable value and one terminal. Pending -> disabled input, no duplicate command or Escape restoration during commit.
### 5. Good/Base/Bad Cases
Good: typing100 applies neither1 nor10; Enter+Blur saves100KiB/s once. Base:0 removes only that manual/task limit, leaving other ceilings intact. Bad: per-keystroke mutation, compact-summary None overwriting persisted intent, stale window mutation, or a replacement limiter/controller.
### 6. Tests Required
`repo:rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs` drives original official inputs, keyboard/blur/Escape, one TaskChanged, stable entities, SQLite failure/retry, reopen and source close. Expanded bounds tests deliver the original resize event. `repo:rust/crates/nxtools-download/tests/engine_contract.rs` exercises same-task HTTP/HLS/DASH waiting and completion after original cap changes; existing rate/runtime contracts retain aggregate budgets and durable failure semantics. Native/physical/IME/DPI/human visual/installed acceptance is separate.
`ref06_acceptance_global_{one_byte,sub_kib,fractional_kib}_survives_unrelated_apply` drives actual Settings Apply and SQLite reopen for1/1023/1537B/s. `ref06_acceptance_task_compact_refresh_tracks_authoritative_rate` uses typed engine commands and original compact/progress refreshes with no global ceiling and a128KiB/s ceiling; it observes the actual capabilities accessibility label, stable clean input, invalid dirty/error preservation, Escape to latest applied intent and window/SQLite reopen.
### 7. Wrong vs Correct
Wrong: per-keystroke mutation, converting an untouched rounded display back to bytes, or replacing current full task intent with compact None. Correct: draft-only Change, deferred identity-scoped Enter/Blur with one pending UUID, exact untouched-byte persistence and current full task projection at the original window update boundary.
