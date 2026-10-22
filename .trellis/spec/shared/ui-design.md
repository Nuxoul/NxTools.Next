# UI design and interaction contract

## Authority and scope
**Approved target:** [staged desktop design](../../tasks/10-02-staged-desktop-ui/design.md). **Observed implementation:** `repo:rust/apps/nxtools-desktop/src/theme.rs#Theme`, `repo:rust/apps/nxtools-desktop/src/ui/metrics.rs`, `repo:rust/apps/nxtools-desktop/src/ui/typography.rs`, `repo:rust/apps/nxtools-desktop/src/ui/motion.rs`. [Earlier remediation](../../../docs/ui-remediation-plan.md) and [GPUI reference](../../../docs/reference-code/gpui.md) remain historical source/behavior evidence, not numeric authority over the later approved brief. No product token consumer was changed by this standards task.

## Approved target / current / exception matrix
| Role | Approved target | Current evidence | Resolution / exception status |
| --- | --- | --- | --- |
| Base/surfaces | #18181b; #1f1f23/#121215/#27272a | Foundation source now maps these exact roles; selection/pressed tints derive from accent/base | Product mapping implemented2026-10-09; real screenshot/DPI visual approval pending |
| Border/focus/accent | User2026-10-10 rejects cyan; neutral action#bfc0c4/hover#d5d6da, selection#353539, keyboard focus#9d9ea3 | Theme/product/component roles now use separate neutral values; mouse focus ring suppressed without removing focus | Supersedes historical#38bdf8/#7dd3fc from installed rejection; actual new visual/DPI acceptance pending |
| Text | primary#f4f4f5, secondary#a1a1aa, muted#71717a | Foundation uses exact approved values | Actual CJK readability at supported DPI still requires visual review |
| Status colors | semantic green/amber/red | #6fbf8f/#d6a85f/#ff6b78 | Approved brief fixes roles, not exact values; reuse semantic values pending visual review, no invented replacement palette |
| Controls | 24/28/32 logical px by role | Foundation compact/default24, action28/input32; settings-toggle boundary tests use actual24 | Shared target implemented; fixed-client regressions retained; no oversized-window workaround |
| Layout spacing | strict 4px grid | Shared settings rows40/header28, menu/hotkey/field/state/diagnostic gaps4/8; module-specific measured roles remain | Consolidated migration retains ordinary176/278, media budget now HLS418/error456 and one-row DASH353 with real footer gates; no blind inflation |
| Radius | 3/6/8 or semantic pill | Control3/floating6 plus component large-dialog8 mapped | Shared roles implemented; actual native alpha corners/hit testing still independently accepted |
| Type/fonts | Centralized existing family unless demonstrated issue; compact legible text | YaHei UI/Consolas; 16/14/12/11/10, line22/20/18/16/14 | Retain roles/family absent measured issue; Body/Label for normal Chinese information, Meta10 for genuine metadata only; status glyph28 is not ordinary text |
| Focus | Visible keyboard-only semantic ring; original keyboard equivalents | Kit Base Button/focus_visible and Window::last_input_was_keyboard replace the retired project modality registry; one Base Root owns Tab/ShiftTab | Actual pointer/Tab/KeyUp/disabled regression passes; native/provider acceptance separate |
| Motion | Short user-triggered only; honor reduced preference | C# SPI_GETCLIENTAREAANIMATION -> dedup typed event/per-session snapshot -> Rust global; all current animation sites branch | Unknown/disconnect/failure reduce; local view observers defer repaint; synthetic state/render/close gates pass, live Windows toggle/reconnect pending |

Historical palette authority: original#0ea5e9 was replaced by#38bdf8/#7dd3fc, then explicitly rejected in installed2026-10-10 feedback. Those historical values are not the current neutral target.

Exceptions require: named role/surface, exact reason (content/native/hit target), measured regression, approved owner decision and verification evidence. Existing tests or comments alone cannot authorize a new exception. Native client/content budgets (ordinary download176/278, media-aware sizes), actual pixel readback, passive preview and activation/focus contracts are behavioral constraints; styling must not defeat them.

## Composition and semantic roles
A compact Windows productivity tool uses continuous work surfaces, clear module rail/search/list hierarchy, restrained separators and one primary action per decision area. No equal card walls, decorative gradient/glass/oversized radius or idle animation. Accent means action/focus/selection, not a large decorative background. Reuse components in `repo:rust/apps/nxtools-desktop/src/ui/components/action.rs`, field/list/settings/titlebar/menu/popover/state. Shell owns layout/chrome, not persistence or native policy. Real brand/SVG icons are registered through assets; font_family is one installed family, not CSS fallback string.

## Component state matrix (required on every changed actionable surface)
| State | Required behavior / assertion |
| --- | --- |
| Default | Stable label/baseline, semantic role, actual hit bounds; icon-only tooltip/name |
| Hover | Subtle semantic feedback, no layout shift or domain side effect |
| Pressed | Distinct active feedback; pointer capture/native gesture preserved; exactly one dispatch |
| Focus-visible | Visible semantic outline/equivalent; no clipping; Tab/arrow order and return-to-trigger |
| Selected/checked | Persistent independent state, not only hover; one selected segmented option, stable count cell width |
| Disabled | No pointer/keyboard dispatch; reason discoverable when useful; not merely low-opacity clickable text |
| Loading/busy | Stable geometry, bounded operation/ID; avoid duplicate submit; cancellation/retry policy follows owner |
| Empty | Genuine no-data vs filtered no-match; useful next action; never database/network error disguised as empty |
| Error | Local actionable typed failure, relevant native stage; keep draft/last-good state and retry reachability |
| Success | Observable result without focus theft; copy-only stays open where policy says; persisted != applied |

Evidence: `repo:rust/apps/nxtools-desktop/src/ui/components/state.rs#empty_state`, `repo:rust/apps/nxtools-desktop/src/ui/components/action.rs#action_button`, `repo:rust/apps/nxtools-desktop/src/quick_launch/actions.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings.rs`. Custom clickable divs do not automatically inherit Button keyboard/role/disabled behavior; audit in Foundation, do not pretend every current component meets this matrix.

## Keyboard, focus and accessibility
- Enter/Space activate the same eligible action as pointer; Escape dismisses the owning transient level and returns focus according to lifecycle, not unconditional app close. Tab/Shift+Tab order is logical; arrows navigate list/menu/segmented structures; no trapping unless a real modal owns it.
- Primary search popup remains activating; auxiliary preview stays non-activating. After first preview creation restore native primary HWND and GPUI search-input focus. Pointer and keyboard selection share one generation-scoped preview controller.
- Icon actions preserve tooltip and semantic name/role where supported; descriptions/errors need field association. Explicit Kit0.7.1 migration now supplies gpui-pre0.3.8 AccessKit/UIA and native focus-visible APIs, superseding GPUI0.2.2's source-level provider gap. API presence and headless snapshots do not prove actual screen-reader/provider acceptance. Do not inject a cross-process provider or claim installed acceptance from this local migration.
- Severity and selection use text/icon/shape as well as color. Inspect contrast and disabled legibility on actual rendering; do not invent measured compliance from hex alone. Long paths/errors wrap/clip within content, never push fixed footer out of bounds.

## DPI, surfaces and motion
Logical layout -> final device-pixel client bounds -> native apply/readback -> WM_SIZE viewport. Set final auxiliary bounds before reveal; default hidden HWND origin is not trustworthy. One native resize owner after real HWND creation; no competing GPUI resize. Use production render/hit-target bounds tests plus 100/125/150% visual/keyboard/pointer matrix when authorized, including move during probe and close/reopen generations. Transparent child clipping cannot prove transparent HWND corners/hit testing.

Motion responds to a user transition once and stops at rest. Reduced-motion reads the system client-area animation preference through C# and typed IPC; no fake app flag. Rust unknown/failure/disconnect defaults reduce and skips animation elements/timers while preserving immediate feedback. Global changes notify weak local views on the next turn; native bounds never animate. Per-frame logs/work are forbidden; timeouts don't mean UI-ready.

## Stage and acceptance handoff
Canonical sequence: **Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces**. Read the [owning plan](../../tasks/10-02-staged-desktop-ui/implement.md) before each stage. User amendment2026-10-09 authorizes implementing all remaining surfaces/common capabilities first and consolidating visual/interaction acceptance at final closeout; no separate user pause between modules. Preserve scoped baseline/state/regression and native evidence, not blind mass replacement. Link Detection stays default copy-only, six-second configurable generation-scoped hide, optional browser route and one popup; its [existing task](../../tasks/10-02-link-detector-panel/prd.md) owns behavior. Native/release/service operations still need their own authorization.

Consolidated source/check evidence: [2026-10-09 audit](../../../docs/audits/2026-10-09-consolidated-ui-polish.md). Final production/test lint and474 desktop tests pass, with11 source gates and native100% screenshot subset. Retired2/6 layout spacing tokens and Downloads raw off-grid gaps are gated; exact retained calendar/titlebar/footer/detail/meta/preview chrome roles are named migration debt in the audit, not blanket target compliance. Actual reader provider is unimplemented/blocked; full surface/state/native DPI/live motion/download recovery acceptance remains open. No package/install/service authority is implied.

## Whole-app layout-quality replan (design-only)
After user rejection of installed typography/alignment and palette-only polish, the approved next deliverable is [layout documentation and prototype](../../../docs/ui/layout-quality/design-system.md), with [complete inventory](../../../docs/ui/layout-quality/surface-inventory.md) and [per-family acceptance](../../../docs/ui/layout-quality/acceptance-matrix.md). Proposed workbench13/20 body,176px named module rail and related values remain candidates until user visual approval plus native comparison; current production tokens above are not automatically superseded. Preserve existing palette and compact/native budgets. Test actual text/columns/control centers and all states; token gates/no overflow are necessary but not aesthetic certification. Browser DOM/ARIA/screenshots do not prove GPUI, passive HWND/provider or installed-DPI behavior. Current scope excludes product migration/build/package/service/user data; future migration retains the canonical order and reference/lifecycle gates.

## Feature-first local implementation authorization
User now explicitly authorizes production layout migration after pushed checkpoint42f60a8, with existing functionality governing presentation. Never implement a reduced feature set copied from the browser prototype. Preserve every original field/control/action/state/shortcut and per-owner save policy; if a new layout does not fit, adjust composition/scroll/budget roles, not functionality. Foundation trial uses176px named/40px navigation and13/20 workbench roles while retaining existing compact roles/fixed windows. Native/visual comparison is still required; this does not declare all candidate numbers accepted on every surface. No palette rewrite, package/runtime/service or dependency/provider scope expansion.

## Remaining presentation slice verification
The feature-first remaining local slice implements auxiliary Downloads, advanced Quick Find, compact Clipboard editors, bounded notification copy/action and browser companion HTML/CSS/shadow hierarchy. Desktop494/494 and strict production/tests lint,workspace/fmt/Debug build pass; browser21 tests/38 isolated DOM checks pass. Workbench actions may explicitly use32px input tier in command/path/diagnostic forms; compact28px actions remain unchanged. Fixed/minimum GPUI tests use simulate_resize, not only window.resize. Constrained source/notification scrolling uses explicit GPUI overflow_y_scroll at the identified viewport with nonshrinking footer/action. Callback/owner preservation is checked, not a new keyboard/persistence/native owner. Five owned100% captures are acquired but uninspected; installed/full-state/mixed-DPI/U/provider acceptance stays open. [Exact outcome and limits](../../../docs/audits/2026-10-09-remaining-ui-layout-migration.md).

## Installed neutral/focus and idle-host correction
User screenshots reject cyan chrome and duplicate selected/focused navigation in Global/Downloads. Keep neutral base/type/native budgets; change semantic roles together, retain mouse focus but only draw ring after keyboard input. Window-keyed weak modality state observes frame-owned input events and refreshes only on modality changes; no polling, logs or second activation listener. Browser startup/diagnostics never launch Native Messaging; pending requests and full Blob operation own demand-only transport, released1s after idle with no autonomous reconnect. Passive snapshot must not launch desktop. Persistent LocalSystem Indexer is independent; its current Stopped state/restore and actual installer/file-lock acceptance remain separate.

## Readable neutral primary action states
Light neutral primary actions require explicit dark label foreground; disabled/busy state uses darker neutral background with opaque readable supporting text. Project primary_action_button owns label/style pairing and retains vendor Button's one disabled/keyboard/click boundary; do not add a second hover/activation owner or patch dependencies. Test paint-time text-style normal/hover/disabled plus fixed hit bounds and no disabled dispatch, not only theme color equality. Current settings Save/Apply messages distinguish durable saved config from failed Windows application.

## GPUI Kit0.7.1 local migration status
Desktop production/test API and dependency migration uses one coherent Kit/Base/Component/Assets0.7.1 + gpui-pre0.3.8 graph on Rust1.96.0. Theme::update synchronizes colors/renderable tokens/Base; explicit button palettes preserve neutral states. Textarea and intentional silent hydration retain search/discovery behavior. Base Root/native focus modality replace old compensation. Clipboard history now uses a single project Base Root menu plugin, not parallel styled ContextMenu/PopupMenu: flat shell, viewport-clamped root/child, bounded scroll/24px reachable parent edge, stable marker-label-arrow rails and full labels in tooltips. ShellRS menu/state composition was rechecked and independently adapted, noGPL code copied. One armed keyboard-release owner shares original eligible actions with pointer; deferred session/frame-lifetime cleanup/callback prevents leased-entity reentry. Full desktop510/510,strict production/tests Clippy,workspace --tests/fmt pass. Owned passive-preview HWND pointer/focus,installed/mixed-DPI/provider acceptance remains unexecuted; source validation is not product-release certification. See [Kit reference](../../../docs/reference-code/gpui-kit-071.md).

Native Kit repair checkpoint: actual96DPI owned Debug HWND client/hide/reuse and synthetic UIA named Edit/Button/focusable checks pass. Historical WM_MOUSEACTIVATE1 despite WS_EX_NOACTIVATE was repaired by explicit user-approved local Windows0.3.8 message branch; flagged HWND returns3,ordinary/policy-disabled HWND returns1. C# owns passive style and final client bounds; in-process GPUI interprets live style without focus stealing/injection. Quick/Clipboard All each2/2 pass; controller8/8/full desktop519/519/strict lint/workspace/fmt pass. Clipboard first preview starts hidden, confirms noactivate→bounds→topmost→show, and reuse reconfirms passive policy; caller-ID/HWND-checked3s deadlines and cancellation prevent stale reveal/hide of replacement HWND. Actual physical clicks,125/150/mixed-DPI,reader/full product-tree and installed acceptance remain unverified. [Native evidence](../../../docs/audits/2026-10-10-gpui-kit-native-acceptance.md).

## Approved component-first option A (2026-10-10)
The user explicitly rejects framework/API-only migration and approves actual styled Kit0.7.1 controls under `.trellis/tasks/10-10-gpui-kit-component-visual-migration/`. This later decision supersedes ordinary-control legacy24/28/32px/type/radius3 styling above where it conflicts with released Kit defaults/official compact tiers. Keep neutral semantic colors andinstalled CJK family,not legacy chrome: default typography/radius/padding/borders/state grammar remain Kit-owned. Do not disable appearance andpaint a duplicate outer control;compound InputGroup may own one frame. Preserve all product features,save policies,native fixed clients,result density/gesture owners,passive preview anddeferred leased-entity boundaries. Outer native alpha/titlebar/OLE/custom business cells require narrow source-backed exceptions,not whole-page exemptions. Foundation actual production shell/settings rendering requires user approval before broad rollout. Current519 passing tests and15:12 package are technical/API baseline only,not styled-component visual acceptance.

Component-first Foundation pilot evidence: real styled Button/Input/Switch/ButtonGroup andSidebar+focused styled Buttons adopted;Tools460×320 usesSmallForm/Field. Kit default16/13px type,radius6 preserved;CJK family/neutral palette adapted,ordinarybackgroundopaque. Same-scope public keyedfocus seam is needed because SidebarMenuItem lackshandle/tabbinding;actualnavigationTab/ShiftTab/release/held/modifier/restoration tests pass. Disabled primary keepsonlyreadablecolorpair,no geometry reconstruction. Actualproduction96DPI main/default/min/tool captures inspected;528/528andstrictchecks pass,userFoundationapprovalpending. Legacyradio/list/multiline/compactlabels/workbench/Popover/per-surfaceadoptionremainsplanned,notblanketexceptions. [Precise Foundation evidence](../../../docs/audits/2026-10-10-gpui-kit-component-foundation.md).

## Foundation compound alignment gate
Latest user2026-10-11 amendment rejects feature controls in the titlebar. Shared `FeatureSettingsWindowSpec` supplies title, description, enabled_label and enabled state; `feature_settings_window` owns a title/close-only48px drag header with divider, then one scroll body. Feature activation (label + optional-description Tooltip + status + Switch) is the first body row, followed by consumer content. Keep460x320 client, existing normal activation, one enable callback/deferred persistence and presentation-only close. Status never belongs beside window close. Ordinary Tool controls retain Kit defaults; layout/color/geometry tests and screenshots do not establish visual approval. Navigation remains176px rail/fixed icon/elastic left label and original focus owner.

## Compact tool settings row composition
`main_window/tools_page.rs::compact_tool_field` is a real Kit Field: title and information trigger share one line in264px information width; controls consume remaining168px at460px client, using32px minimum containers without fixed124x24 boxes. NumberInput/Select retain released default32px and Switch default36x20. Optional feature explanations use shared `ui/components/tooltip.rs::description_help`, not permanent subordinate text rows; Form Small controls row spacing, zero unused Field secondary gap. Current-frame Tooltip lifecycle, full accessible description/Tab, component containment, step2/range/invalid/Blur/cancel/mutual exclusion/config-save regressions are required. No second popup manager or duplicate callback; whole-surface visual/user approval stays separate.

Visible tool Configure is a styled Kit Small outline Button with its own click/focus owner. User2026-10-14 explicitly supersedes the former whole-row settings dispatcher: card pointer/Enter/Space toggles through the original deferred Clipboard section setter; Configure alone opens/reuses settings and stops propagation before its callback. Keep180x160grid/keyedwindow and original save/error owner. Production regression covers durable pointer/keyboard toggling and Configure-no-bubble; geometry/unit results are not installed visual acceptance.

## Optional feature explanations: Kit Tooltip
User explicitly authorizes this explanatory-copy-only migration across Downloads, Quick Find, Clipboard, Tools and global settings in canonical order, without advancing full component phases. Use a discoverable title-adjacent official XSmall ghost Info Button and real Kit Tooltip; preserve complete original copy, automatic text wrapping, theme typography and one existing hover/long-press owner. Help has a full accessible name and participates in Tab. Released Tooltip does NOT automatically show on keyboard focus; do not claim otherwise or add a parallel focus popup. Whole-row/native selection targets may use their existing Tooltip instead of nested help buttons to preserve one activation owner. Shared workbench/labeled rows and per-family download/quick-find/diagnostic/volume/schedule entry points adopt this rule. Distinguish optional explanation from actual runtime state: registration/scan/index status, storage/log paths, install progress/failure, error/validation/risk/units remain visible through explicit status rendering. Source/compiled checks, isolated96DPI UIA/message checks, actual visual inspection and physical/DPI/reader/installed acceptance must be reported separately.

## Quick Find search composition / reference boundary
User2026-10-11 prioritizes3.Quick Find before2.Downloads and approves Steward
layout/component reference ONLY, preserving all existing dimensions/functions/
behavior. `quick_launch/panel.rs::render` uses real released Kit InputGroup with
Search addon and existing InputState, one compound frame/default32px inside42px
header. No caller appearance suppression or duplicate input frame. Keep620x44/410,
two-line40px results,26px icon/48px hints,uniform_list,actions/IME/generation/native
focus/passive400x410 preview. Steward custom SearchInput/ResultList are precedent,
not drop-in controls or permission to change engine/rules/palette/native sizes.
Production adoption snapshot/containment and addon focus/input identity gates are
in quick_launch/polish_tests.rs. Explicit Debug synthetic layout preview is
NXTOOLS_PREVIEW_QUICK_LAYOUT and remains startup-isolated; content HWND is visual
only, never proof of passive policy. User acceptance and actual physical/DPI/native
checks stay separate from compiled/visual evidence.

Unified Quick Find paint correction: user rejects nested Kit input plus square
outline in collapsed, results AND action modes. All modes keep original1px layout
reservations/client sizes but paint shell border/search header bottom divider in
canvas. InputGroup alone owns visible input outline and keyboard-focus presentation;
no mode-specific enclosing outline or independent shell focus stripe. Retain620x44/
410,42px header32px group,all result rails/contents/native policies. Gallery images
have no added borders. `tests/architecture/quick-launch-search-outline.ps1` checks
three actual PNGs and verifies input boundary remains. Collapsed-only fix and
expanded-window_border policy above are superseded, not design acceptance. This
does not implement native HWND rounding/transparency.

User-approved running Quick Find effect refinement: shared search header horizontal
padding is4px (previous8px) so input extends outward8px total without changing Kit
radius/default32px height,42px header or client sizes. Same inset across collapsed,
results/actions; production geometry asserts symmetric4px. Preserve outer1px layout
reservation/single visible input outline, original state and interactions.

## Scenario: Quick Find controlled settings and command Inputs
### 1. Scope / Trigger
Replacing the three settings toggles or five custom-command field frames; launcher
and other controls are not part of this narrow adoption slice.
### 2. Signatures
`compact_toggle(theme, id, label, enabled, on_change: Fn(&bool, &mut Window, &mut App))`;
`command_input_field(theme, selector, label, input: &Entity<InputState>)`.
Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs#compact_toggle`,
`repo:rust/apps/nxtools-desktop/src/ui/components/custom_command_editor.rs#command_input_field`.
### 3. Contracts
Real released Switch owns semantics/focus/one on_change. Copy requested bool before
cx.defer into original application setter; do not invert a captured old value.
Retain32px layout slot and center default36x20 track; never stretch the track.
Styled default32px Input owns its only painted frame and receives original entity
and aria_label. Labels/composition/picker/save/selection owners remain unchanged.
Debug-only NXTOOLS_PREVIEW_QUICK_SETTINGS requires PI_SCRATCH_DIR, empty catalog
roots and disconnected bridges; never instantiate the ordinary product runtime.
### 4. Validation & Error Matrix
Pointer/Space release -> one requested value/one original configuration command;
repaint -> same Input entity/bounds; failed original persistence -> existing error
owner remains, no new success claim. User/native/physical acceptance stays separate.
### 5. Good/Base/Bad Cases
Good: controlled checked snapshot agrees with temporary disk and typed command.
Base: unchanged command entity edits Chinese text and repaints within one frame.
Bad: Button painted as switch,20px page slot, captured inversion or duplicate frame.
### 6. Tests Required
`quick_settings_toggle_matches_real_released_switch` compares released geometry and
Switch role/checked/pointer/Space. `quick_settings_switches_preserve_single_deferred_section_save`
checks all3 production controls, temporary disk and ConfigureQuickLaunch payload.
`custom_command_input_has_one_full_width_kit_frame` checks real frame/edit/entity.
Retain master/picker/cancel/stale/save and minimum/long-content bounds regressions.
Final545/545 and inspected96DPI captures are slice evidence, not whole-stage approval.
### 7. Wrong vs Correct
Wrong: `!captured_enabled`, appearance(false) plus painted wrapper border.
Correct: copied `*requested` through existing cx.defer; `Input::new(input).aria_label(label)`
with no competing chrome. See owning task ledger for remaining adoption/acceptance.

## Scenario: Official component selection before UI implementation
### 1. Scope / Trigger
Every Desktop UI selection/migration, including page structures, groups, navigation,
forms, lists, status and overlays; not only ordinary Button/Input/Switch leaves.
### 2. Signatures
The owning task/ledger records: surface, official compound/leaf candidates, exact
release file/symbol, decision, compatibility regression, exception and acceptance.
Required discovery: [complete official index](../../../docs/reference-code/gpui-kit-component-index.md).
### 3. Contracts
Read the corresponding official compound first, then locked0.7.1 source/public
slots and original product/reference ownership. Prefer styled Component; Base or
custom divs require a narrow verified boundary, not familiarity or old layout.
Settings pages must evaluate Settings/Page/Group/Item/Field; forms evaluate Form/
Field; grids evaluate Table/DataTable; action rails evaluate Toolbar/StatusBar.
Record unreviewed as unreviewed, not unsupported. No silent feature/size/controller
change to force adoption; request approval for actual conflicts.
### 4. Validation & Error Matrix
Missing selection record -> slice not ready for implementation/closeout. Website
snippet/release mismatch -> follow exact release and verify consumer. Proven public
API gap -> narrow exception with source+regression, not whole-page exemption.
### 5. Good/Base/Bad Cases
Good: Settings public render/element slots reviewed with original save owner.
Base: official leaf used inside an already verified compound.
Bad: swap Switch/Input while retaining custom settings page and claim full adoption.
### 6. Tests Required
Maintain catalog name/URL coverage and valid published source entries when refreshed;
spec-contracts verifies local navigation only. Each implementation still needs real
production adoption/state/bounds/focus/lifecycle regression and separate visual/user
acceptance. Availability/source inspection is not behavior or adoption evidence.
### 7. Wrong vs Correct
Wrong: old div layout because familiar; tests green means migration complete.
Correct: exact official compound candidate -> public API/callback review -> tested
integration or justified narrow exception -> ledger with separate evidence levels.

## Scenario: Quick Find official Settings and command Form
### 1. Scope / Trigger
User2026-10-11 explicitly permits official Settings built-in search/presentation
features and rejects the old settings visuals. Preserve all existing functions,
not the old hand-painted page/scroll grammar; no library fork or dependency change.
### 2. Signatures
`MainWindow::render_quick_find` constructs released Settings/SettingPage/SettingGroup/
SettingItem/SettingField; `render_quick_find_control(page, slot, cx)` projects existing
business content via public render slots. `command_input_field(...) -> Field` and
actual Form own command labels/grid/footer on original InputState entities.
Evidence: `repo:rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs`,
`repo:rust/apps/nxtools-desktop/src/ui/components/custom_command_editor.rs`.
### 3. Contracts
Official private SettingsState owns selection/search/filter/sidebar resizing/scroll;
initial-page seed is not mirrored active state. No project parallel scroll controller,
render-time selection mutation or private API access. resettable(false) preserves
existing saves without adding reset semantics. Mounted exclusion content binds
exclusion_focus and its own key listener; search/Input/other pages must not delete
exclusions. Recorders/pickers/save/setters keep original defer/identity/native owners.
Fill groups use official group_box token mapped to approved surface/foreground;
do not paint another GroupBox. Command Form has bounded scroll and reachable Save
at920x480; do not enlarge the native client. Optional help remains real Tooltip.
### 4. Validation & Error Matrix
Search match -> official page/item filtering; no match -> unmount business content;
clear -> official current-page retention. Detached diagnostic controls -> no action.
Pointer/Space setting -> one original section save/typed command. Stale/cancelled
picker -> unchanged current draft. Unmounted/unfocused index -> no exclusion keys.
### 5. Good/Base/Bad Cases
Good: Keywords selects Commands, log keyword selects Diagnostics, no-match unmounts.
Base: original five command entities edit and save through actual Form/Fields.
Bad: hide search/copy old entire page into one slot/guess controlled selection/private
state or accept clipped Save from its mere presence in debug bounds.
### 6. Tests Required
Production settings_compatibility_tests, official navigation/long-list/current-frame
mount checks, diagnostic detach, hotkey lifecycle, volume/exclusion keyboard/picker,
command master/cancel/stale/save/minimum scroll and disk/one-command Switch tests.
Default token comparison and actual owned screenshots stay separate from user,
physical/DPI/reader/installed acceptance. Final549tests are scoped slice evidence.
### 7. Wrong vs Correct
Wrong: project quick_launch_settings_page treated as official active selection.
Correct: official Settings chooses page; actual mounted exclusion FocusHandle owns
Up/Down/Escape/Delete/Backspace, while existing deferred domain callbacks stay intact.

## Quick Find row/metadata contract
Use InputGroup32 in attached36px rail and stateless ListItem on original owners;
no second controller/selection stripe. Keep40/48px density. Delete40x40 uses public
custom Size: released icon Medium/Large both32; preserve one styled Button action.
DescriptionList values use natural text_sm lines within48px, not clipped default
text. Require real bounds/state/identity/deferred-action checks; visuals stay separate.
Exact APIs/tests/limits: [queue01 outcome](../../tasks/10-10-gpui-kit-component-visual-migration/ui-queue-outcomes.md).

## Approved installed-feedback amendment (2026-10-12)
The later [installed-feedback task](../../tasks/10-12-installed-feedback-capabilities/prd.md) supersedes matching historical rules above: Link capture only identifies; explicit open never copies; secondary Copy alone writes Clipboard. Remove copy-only/browser gates from runtime while retaining readable legacy config. Tools use neutral tiles/subordinate status; notifications have opaque backing. Download progress omits URL/path, uses two-column metrics and a semantic segment table, one-second timestamped-byte display sampling. Ordinary confirmed new-download client is content-derived246px instead of historical278px; compact176 and media/error budgets retain their separate owners. A200px schedule Popover with124px calendar viewport keeps its trigger reachable; dates retain original hit size and scrolling. System/Light/Dark propagates Kit and custom palette without recreating windows. Taskbar Root has per-instance transparent fill; actual DWM alpha remains a separate acceptance gate.
Executable contracts and verification limits: [Desktop quality](../nxtools-desktop/backend/quality-guidelines.md). These amendments do not certify visual, installed, mixed-DPI or LocalSystem acceptance.

## Official Kit theme authority amendment (2026-10-13)
User explicitly requests the built-in GPUI Kit Default Light/Default Dark themes after rejecting independent project recoloring. This supersedes historical palette hex values and neutral-selection overrides above, not geometry, behavior or native ownership. `theme.rs::apply_active_theme` calls released ComponentTheme::change and only adapts installed CJK/mono fonts; application color overrides are forbidden. Custom business painting projects current `ComponentTheme::global(cx).colors` through Theme::active, never a separate dark/light table. Sidebar Ghost selected grammar is Kit-owned (secondary_active), lists use actual list_active including alpha, status text projects official base semantic colors. Per-window taskbar Root transparency remains native presentation, not a global palette override. Source gate rejects application palette literals/Kit recoloring; production render, current-palette and full-color equality tests protect the real entry. Actual installed/native/mixed-DPI/reader acceptance remains separate.

## Extensions workspace correction (2026-10-15)
User explicitly requires Extensions as first-class left workspace item immediately belowClipboard and aboveOtherTools; no Tools nesting/backTools. Original176rail/main1060x650/min920x480 and KitDefaultthemes retained. The earlier list-first overview is **superseded by the later user rejection below**; it is historical implementation evidence, not the approved target. Package detail keeps meaningful Settings pages, pinnedSaveCancel/retainedInputdrafts and exactwarning at trust/import. PublicSettings::header_style hides redundantinnersearch; module switch reuses manager without config/native effect. Historical721Desktop/source/captures prove neither current visual approval nor requested catalog-cell compliance. Owning historical contract: [workspace UI correction](../../tasks/10-15-extension-workspace-ui/outcomes.md).

## Extension trust / action consistency amendment (2026-10-15)
User rejects installed inline trust technical wall and inconsistent extension action heights. Extension text actions now use released Medium/default32 across overview/details/settings/confirmations, while icon-only semantic controls keep their own tiers; no global restyling or custom heights. Toolbar defaults Small and overrides child sizes: set its public with_size(Medium) at the compound owner, not merely the Button. Trust uses real Kit Dialog/DescriptionList/GroupBox/Accordion, readonly runtime summary instead of disabled editable-looking Input, fixed warning and32px footer, and fixed recovery actions for missing/failed runtime. Hash/sourcepath/budgets remain collapsed technical details, declarations visible in scroll body. Existing DefaultLight/Dark/fonts/windows/native owners preserved. PATH discovery on explicit open never grants trust/enable or writes config; detected runtime verified again at actual approval. [Owning task](../../tasks/10-15-extension-trust-path/design.md).

## Explicit component/visual rejection and catalog-cell target (2026-10-15)
User rejects the extension-feedback gallery's long prose package rows and unframed header commands, explicitly referencing Other Tools cells/tiles. This supersedes prior list-first guidance and any AI-only visual closeout; it does not undo tested runtime/clipboard/trust behavior. Follow [short mandatory component selection rules](ui-component-selection.md), [current79-entry table](../../../docs/reference-code/gpui-kit-component-table.md) and locked source before UI edits. Catalog tiles use official ListItem/Icon/Label/Tag/Button public composition; no imaginary Card/Grid import, no DataTable because the user said cells. Header import/runtime/chain/activity actions need recognizable official outlined Buttons at rest. Locked0.7.1 Toolbar.child calls Button::prepare_for_toolbar -> ghost().compact(); Medium controls size, not this variant transformation. Use a source-verified public composition when preserving outlines and test final keyboard/paint behavior. Documentation/call-site audit is completed separately from the still-unimplemented product visual correction; no dependency upgrade or installed acceptance implied.

### Authorized source implementation checkpoint
The preceding documentation-only/unimplemented wording describes the earlier checkpoint, not the current source. The later authorized correction implements catalog cells, state-driven explicit next steps, outlined header Buttons through released Toolbar.content, and fixed manual-feedback actions. Real render-bound layout red/green, Light/Dark border paint, official toolbar keyboard roving and trust/usage side-effect regressions pass; final Desktop754/754/strict/workspace/fmt/source gates are recorded in [redesign outcomes](../../tasks/10-15-extension-action-feedback/redesign-outcomes.md). Ten new isolated GPUI captures are generated/nonblank-checked, **not directly visually inspected** because viewer tools failed. No user visual, installed/native or Release acceptance implied; the earlier user rejection remains authoritative.

## Contextual Clipboard/plugin UX amendment (2026-10-15)
User rejects sequence9's modal chooser/report inside300x440 Clipboard, forced252px catalog cells, fixed360px feedback and repetitive Accordion. New authorized target: direct extension submenu leaf -> compact same-panel progress/outcome/actions, explicit details only; content-driven cells/details with bounded maximum rather than filler height. Downloads ordinary commands precede extension submenu and destructive commands follow. Clipboard context menu must genuinely exceed its HWND, not merely a list clip. Locked Kit NativeMenu source documents OS popup capability; C# native adapter owns HMENU/TrackPopupMenuEx via typed session, Rust original callbacks/state and GPUI original panel; this is system-native menu presentation, not styledPopupMenu adoption. Downloads uses official PopupMenu. Disconnected window-local fixture fallback is not production overflow policy. Preserve original trust, pending/cancel/late host evidence/exact-origin result navigation. Owned native popup/provider cascade/cancel passed, Desktop759/Protocol47/C#252 and compiled gates pass;10newGPUIcaptures generated but unviewed due tool failure. [Current outcomes and limitations](../../tasks/10-15-extension-action-feedback/ux-outcomes.md); not installed/physical/DPI/user visual acceptance.

## Clipboard source-item feedback correction
The latest user request rejects the preceding progress region ABOVE history. Processing/outcome belongs INSIDE the exact context-menu source ListItem, keyed by immutable history item ID, not current selection/index. Preserve original content/metadata, original ListState remeasure/scroll owner and native menu/runtime/effect/result boundaries. One compact lower slot separates readable status/indicator from outlined32px actions, with deliberate24px paging/icon close; control gestures cannot bubble into row paste. Released Spinner is visible+pending+motion-allowed only, static under unknown/reduced policy and terminal. Prepaint clipped bounds with deferred visibility-change notify avoids reentering ListState during measurement; no new timers, logs, height animation or automatic Dialog/window. [Current item-local evidence](../../tasks/10-15-extension-action-feedback/item-feedback-outcomes.md) records placement/status-width red-green, actual click/frame/identity regressions, compiled/source checks and4 actual generated but unviewed GPUI captures. Earlier top-feedback images remain rejected historical evidence; source tests are not visual/user/installed acceptance and sequence9 is unchanged.

## Installed menu / six-position notification amendment (2026-10-16)
User rejects the white OS HMENU renderer; the outside-source-HWND requirement remains. Clipboard uses released official PopupMenu on one independent GPUI popup viewport, with C# source/hidden-popup ownership, work-area final client readback and native placement; Rust builds a correct-size hidden scene through explicit Window::draw after leases, then defers current-session reveal -> activation. Hidden ordinary frame callbacks are not a first-show gate. No colored HMENU substitute, palette fork or production clipped fallback. Default menu rows/icons/check/cascade grammar stays Kit-owned; source ID/session/trigger guards and deferred first-wins callbacks remain. First inactive preparation state must not dismiss the menu before it has actually activated.

The earlier window-internal interpretation was explicitly rejected by the user's installed screenshot. Six positions mean the **source monitor's screen work area**, never main/Clipboard content bounds. Keep SystemSettings.notification_position/default BottomRight and original draft/atomic Apply/Cancel; saved location publishes next-turn only. One existing bounded NotificationCenter owns notifications/generations/lifetimes; one small passive independent GPUI popup renders all retained notices, not a full-screen input-catching overlay or second manager. C# validates pipe-peer/source/popup HWND ownership, source-monitor work area and target DPI, then NOACTIVATE/tool/topmost/final client readback while hidden. Hide before reuse; Rust builds the correctly sized hidden scene after its host lease and only then requests nonactivating reveal. Hidden Windows HWNDs do not deliver ordinary frame callbacks: on_next_frame/refresh alone was proven insufficient by the real fixture, so use the public draw boundary and deferred readiness, not show-first or a timeout-as-ready. Actions still claim live generation and execute on the original source window, never popup; stale/closed/duplicate/timeout/disconnect cannot reveal or dispatch. Directory progress/success/failure goes through the same center and must not write its page status banner. Card GroupBox/Alert/buttons and source-local pending spinner remain; no native system notification broker, palette/dependency change or plugin effect. [Screen correction](../../tasks/10-16-menu-notification-refinement/screen-notification.md) supersedes earlier host assumptions; [current evidence](../../tasks/10-16-menu-notification-refinement/screen-notification-outcomes.md) separates actual owned screen coordinates from installed/physical/mixed-DPI/visual acceptance. Download group centering/clear and24px wrappers remain unchanged.

## Notification frame encloses message and actions
User rejects the preceding message-only Alert border with action/close controls visually outside. The complete notification now uses released GroupBox::outline public child/content_style as one visible frame containing Alert message/details and all official Buttons, with opaque theme backing. Do not use GroupBox.footer for controls: it is outside the surface. Inner Alert public border/background/padding style removes its separate frame while retaining official semantic text/icon; original manager/position/generation/actions remain. Regressions must identify the actual painted positive-border quad and assert actions/close inset inside it, not merely inside an invisible layout rectangle. [Current correction evidence](../../tasks/10-16-menu-notification-refinement/notification-frame-outcomes.md) supersedes the earlier screenshot appearance, preserving user rejection; generated images/paint tests are not user visual acceptance.

## Current official notification and feedback rule (2026-10-17)
After verifying locked0.7.1 capabilities, user explicitly accepts official5seconds, whole-list hover/focus remaining-time pause and hover-only close, then authorizes implementation. This supersedes the custom GroupBox/Alert frame and configurable1..60/default6 timing above. Use released Notification/NotificationList/ToastStack as the sole card/stack/countdown/transition owner. NotificationCenter retains max3 business records, invocation IDs/generations/source and once-only action claims, not another timer. Distinct invocations must not share a category key; same invocation progress->terminal reuses its own key. Two official Small24Buttons use the public content slot; official close/card grammar is unchanged. Keep legacy Core duration readable and unchanged on unrelated saves, but show honest read-only5second settings information.
One small passive screen popup preserves C# monitor/DPI/style/readback and hidden explicit draw/deferred reveal. Pointer AND keyboard expansion allocates native space; collapse/blur/focused retirement/empty/reopen release it without permanent transparent area or hide/reveal flicker. Native allocation may observe public content focus/measurement and pointer bounds, never drive official offsets/expiry/focus. Position-only changes use official global placement and preserve entity/countdown. Official close completion removes only exact ID+generation; actions claim then defer original-source dispatch. Native motion snapshot/error/disconnect reaches GPUI reduce-motion as well as existing app policy. [Current contract](../../tasks/10-16-menu-notification-refinement/official-notification-integration.md) and [performed evidence/limits](../../tasks/10-16-menu-notification-refinement/official-notification-integration-outcomes.md).
All Desktop transient operation feedback uses the existing ingress, never page/panel Saved/enabled/disabled/copy badges. Busy Extensions retains catalog/search. Keep real field validation, explicit trust/risk, durable query/task/registration state and user-opened reports; browser/CLI/startup channels remain independent. [Inventory](../../tasks/10-16-menu-notification-refinement/transient-message-audit.md). Current source is not in sequence15; compiled/owned-HWND/generated captures do not certify physical/installed/mixed-DPI/reader/user visual acceptance.

## Notification natural-height and configurable-duration amendment (2026-10-20)
User rejects clipped multiline/currency notification and explicitly approves1–60seconds/default6 after verified0.7.3/latest0.8.0 still hardcode5. This supersedes only the preceding fixed5/read-onlyduration and project54pxtextcap, not official card/stack/hover/focus remaining-time/close/animation ownership. Keep Kit0.7.1,336card/360nativewidth/400nativeheight/sixpositions/passivepolicy. Project public content grows naturally up to per-retained-card share of400edge/padding/peek budget; only then text scrolls,with nonshrinking officialSmall24actions. Short fulltext must initially fit; real100linebody must scrolltoend;3expandedlongcards/actionhitbounds fitactualnativeviewport. Local Apache2 Component0.7.1 patch adds duration builder forwarding into originalToastOptions,unconfiguredupstream remains5; no secondclock/customcard/registryedit/frameworkupgrade. Global Settings restores real32px NumberInput in168px lane,originalApplyCancel/ownedatomictransaction,rangevalidation/clampedstep and busyguards. Disable InputState defaultstep when one project Step callback owns range; never combine competing writes. Save/startup publish configured duration for newly admitted notifications; activecard entity/generation/remainingtime unchanged. Exactpatch/provenance: `repo:vendor/gpui-component-071/NXTOOLS-PATCH.md`; owning `.trellis/tasks/10-20-notification-sizing-duration/`. Compiled/source results are not installedphysical/DPI/reader/user visual acceptance.

## Clipboard interaction correction (2026-10-20)
User rejects the duration-picker pause button and classifier-gated Markdown presentation. Clipboard's extreme top-right now requires one official Small ghost Pause/Play icon Button: one click pauses, second resumes, no duration menu. Preserve native ACK/privacy/reconnect/error retry; pending prevents duplicate dispatch and failure remains visible. Text preview defaults to safe official Markdown without Auto/Language interaction; optional source/code inspection is not a prerequisite. History text entries also render bounded safe Markdown summaries with cached official TextView, compact bounded multiline height, no implicit resource/URL action and unchanged full-by-ID paste payload. Page/Ctrl+number shortcuts must work after focus leaves search, retain original control Enter/Space ownership, accept Ctrl-first key release and legacy empty-search digits. Owning task `.trellis/tasks/10-20-clipboard-interaction-feedback/`; compiled/source regressions do not certify installed/physical/DPI/reader/user visual acceptance.

## Clipboard content-height screenshot correction (2026-10-20)
User rejects screenshot's clipped left Markdown rows/right preview and redundant Markdown/language button. Supersedes fixed24/40/96prose heights and four-line clamp, not bounded400character data projection. Text rows naturally grow from actual official TextView/available width, keep compact minimum/paging/actions. Preview measures actual rich layout before native final client, caps only at work-area budget and provides a genuinely sized scroll viewport/last line with separate footer. Existing passive HWND/generation/native ACK/focus/reuse remains; first final-size hidden frame explicitly drawn before show. Keep one Small outlined source/render toggle; automatic code/Markdown recognition remains, no language menu. Feathering must not conceal short-content layout defects. Owning `.trellis/tasks/10-20-clipboard-content-height/`; source/compiled bounds are not installedphysical/DPI/reader/user visual approval.

## Clipboard separated-cell and icon-fade amendment (2026-10-20)
User supplies distinct spaced cells and solid-to-transparent gradient reference, superseding continuous borderless history rows for Clipboard only. Retain vertical list/natural Markdown height/original320rowpaging/actions/preview. Use8DIPactualgap outside clickable row, public official ListItem border1/theme border/rounded_md/idle surface; official hover/selected/focus remain, no custom control repaint. Localized24DIPsource-icon/name rail uses static native90degree gradient from source application's actual icon dominant RGB to the same color alpha0. User's explicit correction rejects the previous FilesYellow/ImageBlue/TextNeutral mapping: identical source icons give identical hue across all content kinds; missing/unreadable icons use theme-neutral fallback. Kind selects only the missing-icon glyph. Original icon stays crisp; gradient never covers body or actions. Bounded managed-source PNG extraction/cache runs on the existing serialized worker, never render IO/decoding, application scanning or native/protocol changes. Small icons are not enlarged, avoiding neutral-edge contamination. This narrow request authorizes localized icon-derived tint, not a global palette/theme fork or saturated whole-card recoloring. Owning correction `.trellis/tasks/10-20-clipboard-icon-dominant-color/` supersedes only the earlier row-separation task's hue source; painted-scene/bounds tests are not screenshot/physical/DPI/user acceptance.

## Clipboard text-card maximum amendment (2026-10-20)
User explicitly limits the text card itself to210DIP; implement direct constant, not a new settings feature. Supersedes only unbounded list growth above: short text retains natural official TextView height/minimum62, long text summary is clipped at whole-card210 including24DIPsource header/border/padding and existing inline feedback. External8DIPgap is not part of210 (slot<=218). Reserve feedback's76DIPallowance by reducing body budget, never clipping its action row; retain no nested text scroll owner. Actual original full preview/scroll/footer/native lifecycle and full-by-IDpaste remain unchanged. Ordinary images retain144DIPmedia/190DIPcard maximum, files62DIPcard, both excluding gap/optional feedback. Source-icon hue and official states unaffected. Task `.trellis/tasks/10-20-clipboard-text-height-cap/`; actual production bounds/action regressions remain distinct from screenshot/physical/DPI/installed/user visual acceptance.

## Clipboard input-owner and preview-cancellation correction (2026-10-21)
User reports Ctrl+index never pastes, typing is lost outside search, and pause/shortcuts show an unrelated item preview. Preserve original InputGroup/InputState and primary activating popup; restore search on summon/activation and when GPUI focus is absent. Window-scoped interceptor bridges a digit gesture before focused-element dispatch can lose it; immutable item/generation armed on down, one original-target paste on matching up, Ctrl-first release/repeats/editor/dialog/menu/IME guards retained. Type-anywhere platform text/IME fallback delegates to the original InputState only when a noninput control is focused, using actual input bounds; it does not reconstruct text from key labels or steal another real input. Pause/play, accepted number shortcut, actual item action and search changes defer original preview dismissal, retiring candidate/generation/native pending work without another preview controller. Ordinary control Enter/Space still has one official owner. Bounded semantic shortcut accepted/rejected/cancelled/dispatched events share the original action/IPC correlation; no general keystroke/query/text telemetry. Task `.trellis/tasks/10-21-clipboard-input-preview/`; synthetic events and read-only installed logs are not physical/IME/native/visual acceptance.

## User-approved GPUI Kit0.8.0 upgrade (2026-10-21)
Current Kit/Base/Component/Assets0.8.0/defaultpre0.3.9 supersedes only the historical0.7.1versionpin, not earlier user layout/behavior corrections. Follow [current release/API delta](../../../docs/reference-code/gpui-kit-080.md); historical79pageaudit/hashdata remainshistorical. Official0.8.0typography/selection/focus/scroll/accessibility improvements are retained, with originalcontent/nativeviewport budgets and actualproductionregressions. ExistingToolbarghost+compacttransformation persists; disabledcontrols nowexplicitSome(true), Settingsdropdownaccessiblelabel inherits title+value. Only originalNotificationduration and Windowslive-noactivatepatches carried to newreleasecopies; no palettefork/Fast/newMarkdowneditor/newfeature. `repo:tests/architecture/gpui-release-contracts.py` verifies coherency/sourceonly; source/compiled/native/visual/installedacceptance mustremainseparate.

## User-approved unified document and graphite amendment (2026-10-22)
The exact approved six-item plan intentionally supersedes earlier built-in-default-only/no-editor restrictions, not optional official themes, fonts, geometry, source-icon hue or native/action owners. Default Light/Dark/System now select original graphite/indigo configs through public Kit ThemeRegistry/Theme::update/change. Four Clipboard/Quick Launch document surfaces use pinned Apache editor passive/interactive modes; source payloads stay independent. Image modes defaultAuto; launcher externalgap defaults4DIP/optional0. Detailed executable signatures, failure and acceptance boundaries: [current experience contracts](../nxtools-desktop/backend/ui-experience-contracts.md). Evidence: `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/apps/nxtools-desktop/src/ui/markdown_document.rs`, `repo:docs/reference-code/graphite-theme.md`. Historical palette/release audits above remain historical. Actual96DPI AI-inspectedGPUIcaptures and owned180Hzcadence are not user/physical/mixedDPI/reader/installed approval.
