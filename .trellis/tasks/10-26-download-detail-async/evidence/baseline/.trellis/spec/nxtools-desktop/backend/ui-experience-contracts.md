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
