# Component-first desktop design

## Decision and change boundary
User option A is explicit: retain released Kit0.7.1 styled-component type/geometry/padding/radius/borders/state grammar,adapt only approved graphite/neutral semantic colors. Existing APIs already run on Kit;the behavior gap is the visible control/composition layer still suppresses/reconstructs component appearance. Smallest fix is replacing those presentation adapters and consumers,not another framework or domain rewrite.

This task supersedes historical presentation-only control/type/radius constants where they conflict with A. It does not supersede source-backed behavior,native clients,primary/passive activation policy,Rust/GPUI/C# ownership or any accumulated dirty fixes. Update the shared UI target/current matrix at Foundation implementation so future work cannot silently restore the rejected legacy visual overrides. No spec/product edits during this planning turn.

## Visual system
- Base palette: canvas#18181b,surface#1f1f23,raised#27272a,text#f4f4f5,secondary#a1a1aa,neutral action#bfc0c4. Preserve distinct hover#d5d6da,selection#353539 andkeyboard focus#9d9ea3;semantic success/warning/error remain semantic. Theme color remapping must retain readable foreground anddistinct states,not flatten states to one tone.
- Typography: released Kit default16px base/13px mono and hierarchy;existing installed YaHei UI/Consolas family retained for CJK/native compatibility. Do not wrap ordinary component labels in old12px/13px divs. Small/XSmall/compact APIs control density,not global rem/zoom scaling. Domain metadata can use documented secondary tier without shrinking controls.
- Radius/border/spacing: released theme radius6px and component-specific defaults. No blanket radius3px,borderless or fixed28px overrides. Container width,flex/scroll and native chrome are layout contracts;ordinary controls own their own painted boundary.
- Layout: continuous compact Windows workbench,not showroom cards. Use Sidebar fornavigation,SettingPage/Group/Item forconfiguration and Form/Field fordecisions. Use plain group variants where supported;one existing commit footer per decision. Do not add search/reset/collapse/resize behavior solely because the compound API supports it.

```text
Main1060×650 / min920×480
┌ native titlebar / caption behavior retained ─────────────────┐
│ Kit Sidebar176 │ page heading / existing toolbar             │
│ four modules   │ Kit settings groups OR business list        │
│                │ bounded scroll / retained detail tabs       │
└────────────────┴ existing notifications / status semantics ──┘

Compact fixed window
┌ native host/title strip ─────────────────────────────────────┐
│ Kit compact Form/Field → flexible bounded content/scroll     │
├ existing stable commit footer with Kit buttons ──────────────┤
└──────────────────────────────────────────────────────────────┘
```

Visual individuality is the product's compact neutral workbench and native behavior;no invented hero,decorative gradient,card wall or arbitrary new palette. User chose actual Kit appearance,not a new bespoke design.

## Component boundary
Presentation adapters may own stable IDs,semantic variant,official size tier,icon/label/tooltip/accessibility label,existing focus binding andcallback routing. They must return real Kit styled components or compound composition. They must not paint a second input/button frame,reimplement Switch/Radio/Progress,force legacy label styles,or create another activation dispatcher. Prefer no helper when direct component use is clearer.

Existing `focused_action`/`focusable_content` may remain only for genuinely custom row/native gesture targets,with named exception. Ordinary buttons become component Button;Switch/Radio/Checkbox use their own single controlled-value callback andfocus owner. Preserve pointer no-focus where an actual history/search contract requires it;do not append custom KeyUp handling to an eligible component.

## State and data flow
Existing domain/worker snapshot → current local view draft/entity → Kit controlled component → existing local event handler → deferred cross-owner effect → original worker/native command → correlated terminal result → local state/notification. No change to schema,save policy,queues ornative dispatch.

- `SettingField` getter/setter closures must not synchronously lease the currently rendering owner. Use immutable local projection or existing local state,then schedule cross-owner changes after leases release. The `element/render` seam supports existing Input entities,hotkey recorders andfile pickers without creating a second state/store.
- Input/Textarea keep existing entities andsilent programmatic hydration;user edits,paste,clear,IME/search generation remain explicit original intent. Do not replace owner entity every render.
- Compound Settings internal selection/search may be used only when it maps toexisting behavior. If its built-in resize/search/reset changes product semantics,use Sidebar + Form/Field composition inside the existing owner instead. This is a layout compatibility choice,not permission to restore custom input chrome.
- ListItem/TabBar/Progress/Badge/Alert can project existing data;never introduce a competing List/DataTable/Command controller for an existing generation/paging/multi-selection owner. Evaluate DataTable at the boundary with actual Ctrl/Shift/sort/context/OLE tests;if incompatible,retain business grid andadopt actual leaf components plus documented exceptions.
- Notification manager remains bounded/deduplicated existing owner. Render Kit Notification only if it fits existing semantics/bounds without a second timer/list manager;otherwise use Kit Alert/Progress/Button presentation under same bounded host.

## Native and capability exceptions
These are candidate exceptions,not blanket whole-surface exemptions. Record exact call sites/constraint/test in adoption-ledger.md during implementation.
1. Transparent native outer shell,window drag/resize/caption hit regions andOLE source gesture: existing HWND/C# ownership cannot be replaced by a component modal ormouse-only dummy.
2. Quick/Clipboard passive previews: existing one-window generation controller,hidden/noactivate/final-client/show sequence,cancellation andfocus restoration remain. Content controls adopt Kit;do not embed activating hover-card/dialog as HWND substitute.
3. Clipboard history submenu: current user-approved project single Base Root owner exists because released styled PopupMenu overflowed300px andprivate positioning has no equivalent override. Keep positioning/session/keyboard owner;align row typography/palette with Kit where compatible,retain narrow overflow/focus tests. Ordinary dropdowns stay Kit-owned.
4. Hotkey capture: original recorder/native registration policy has no drop-in text field equivalent. Style through Kit field/container/label while retaining capture owner.
5. Business list/cells: variable Clipboard image/file rows anddownload segment/verification/timeline payload may need custom content slots,not custom ordinary controls ornew stores.

## Fixed budget contract
Preserve existing native dimensions: main1060×650/min920×480;new download560×176→278,DASH353/HLS418/error456;task440×184/expanded280/failed196;download settings984×650,schedule430×326,shutdown450×200;global720×480;Quick620×44/410,preview400×410;Clipboard300×440 default/user resize;Tools460×320,Link420/derived height;notifications max3,width336/max112. Existing result-density/search/icon/trailing/native-hit values remain named constraints. If a leaf control cannot fit,use official tier,composition/scroll andproduction measured regression;never inflate native windows,drop controls orshrink global rem. An actual irresolvable conflict requires user decision,not silent custom fallback.

## Evidence and rollout
Before editing each migrated interaction,read indexed legacy note then recorded reference entry/constructor/base/native boundary;update verified invariants andadd red-capable boundary tests. Existing reference evidence is not a waiver for changed call chains.

Foundation pilot includes theme andactual shared controls + production main Sidebar + representative Tools settings460×320. Show default/minimum main andcompact form with real Kit Button/Input/Switch/Radio/Select,not only a standalone component showroom. Isolated synthetic view models avoid user config/store/service. Pilot screenshots must be opened andactually inspected;verify target PID/title/size. User approves pilot before broad rollout,then follow canonical order. Tests compare project adapters/production controls against released Kit default/official compact geometry under the same neutral theme,not old constants. Palette andsemantic-state tests are necessary,not aesthetic proof.

For every family,record adoption,feature regression,visual inspection,user acceptance andnative/physical/DPI/reader status separately. The current519 tests and96DPI HWND message gate are a baseline only. No claim of full reader orinstalled acceptance. Final package/install/service changes require separate instruction.

## Diagnostics and rollback
Reuse payload-free existing lifecycle IDs andmeaningful transitions;no render/frame/pointer logs. New cross-owner operations,if unavoidable,must define accepted/terminal/elapsed events before completion. No query,text,path,URL,credentials orraw native strings.

Before implementation capture the dirty current baseline/diff/untracked list in scratch;record source file hashes. User checkpoint42f60a8 is historical,not safe wholesale restore over later fixes. Rollback only task-owned presentation hunks after re-reading live files;no reset,branch switch,registry editing orconcurrent same-file writers. Keep exact Kit0.7.1/backend0.3.8 andapproved local passive branch;no dependency/toolchain changes. Do notoverwrite15:12 package during this task absent new packaging request.

## Technical risk gates
Default typography will redistribute content;official compact tiers must be measured in each fixed budget. Compound Settings/render closures can re-enter leased entities;local snapshots/deferred updates are mandatory. Styled component row keyboard semantics may differ fromold outer Button;one actual Tab/KeyUp dispatcher andeligibility test required. Source inspection does not prove adapter compilation/actual visual/physical input. Capability checks occur in their owning stage with real tests;one narrow incompatibility blocks that substitution,not unrelated stages. No broad optional review loops.

## Foundation normalization after rejected pilots
Use one semantic compact settings row: fixed left label/help lane and right control lane, with 24px main line, 4px separation and 16px supporting line. Align each control to the main-line center rather than the combined two-line block. Keep descriptions in the label lane; never hang unrelated help under the control column. Kit Form/Field owns field composition; label_fn supplies the two-line information slot, not a duplicate painted control. Use one fixed column width across the four Tool fields and preserve full help via tooltip. Reuse named compound-header lane metrics, not ad-hoc label heights.

Tool workspace keeps its original whole-row activation/focus and description/status. Replace decorative '配置 →' text with a real styled Kit Button that does not install a second click or keyboard handler; the existing parent row remains the sole dispatcher and inner control is non-tab-stop/pointer-no-focus. Verify pointer on both row and visible action opens/reuses the same keyed window exactly once. No business/native/persistence changes, no Downloads rollout. Actual screenshot inspection remains required and unavailable tools must be reported, not converted into visual approval.

## Foundation role/tier candidate — 2026-10-11

The latest user feedback separates component adoption, official size-tier choice
and whole-page composition. Default controls are a candidate to inspect, not an
automatic aesthetic pass. This candidate supersedes the24px Tool field main lane
above: NumberInput/Select retain released default32px frames; default Switch
retains36x20 track; the label/control lanes have32px minimum, not forced heights.
Small Form keeps its6px row spacing. One264px information column uses its full
width; remaining width belongs to controls (currently168px at460px client), not
a separately hard-coded124px box. The custom label_fn already contains help, so
Field's unused secondary-row gap is zeroed; full tooltip help remains available.

Title/status/Switch/close stay aligned on the existing32px title line. Status and
Switch are one feature-state group; close is a separate window action with16px
inter-group spacing versus8px within the feature group. Description remains its
own bounded row. This changes composition only, not component internals or
window drag/activation/persistence/callback ownership.

Retain460x320 and every field/action. Actual default-vs-Small comparisons,
containment and interaction/native fixture checks must pass, then inspect the
production screenshots and obtain user approval. The current screenshots are
captured but NOT inspected because the calling-session Browser view is unavailable.
Do not advance Foundation or infer visual acceptance from538 passing tests.

## Approved shared-template and Tooltip amendment — supersedes earlier header candidates

User rejects the right-heavy titlebar status/Switch grouping. Titlebar now contains
only title and close, followed by divider; labeled enable/state/Switch belongs to
body. FeatureSettingsWindowSpec explicitly supplies enabled_label; consumer has no
duplicate shell. Later explicit user instruction replaces all optional per-feature
description rows with Kit Tooltip across Downloads/Quick Find/Clipboard/settings.
Template description is attached to the body enable label, not window close.
Each field uses a compact title-adjacent official information trigger. Full copy
is retained in Tooltip and accessible name. Default hover/long-press remains Kit
owned; Tab reaches help but does not automatically show Tooltip. No custom manager.

One explanatory-copy migration across families is authorized, not advancement of
their full UI phases. Actual scan/index/registration/install status and paths stay
visible via explicit status-row helpers. Errors/warnings/validation are not hidden.
No business/save/native lifecycle change. Fixed460x320 Tool client and other native
budgets stay unchanged; overflow remains with original scroll owner. Latest source
checks pass540/540, but latest real captures are uninspected due Browser view issue.
User Foundation visual approval/physical/high-DPI/reader/installed acceptance pending.

## Quick Find / Steward planning amendment — 2026-10-11

User changes family execution priority to Foundation -> Quick Find -> Downloads ->
Clipboard + Tools -> Global settings + supporting surfaces; retain stage titles.
Steward supplied MIT snapshot and static image are reviewed (reference-code/steward.md).
Search-first single surface, continuous rows, stable icon/title/metadata alignment,
subtle selected-vs-hover and bounded state-only redraw are useful precedent.
Retain neutral approved theme/Kit0.7.1 and original entity/controller/native owners.

User clarification: reference Steward's UI layout and actual component composition
ONLY. Preserve every existing Next window/client/search/row/icon/trailing/preview
dimension, visible information/behavior, empty-query policy, actions, shortcuts,
IME, selection, query generation/cancellation, file-dialog integration and native
show/hide/focus policy. No A/B geometry decision exists; the assistant's proposed
760px/60px/42px route was unsolicited and is withdrawn.

Inspect how Steward composes search/results/metadata/selection with GPUI and
Component primitives, then independently adapt compatible layout within existing
Next budgets using locked Kit0.7.1. Steward's hand-written SearchInput and custom
ResultList/div visible slice are reference architecture, not drop-in Kit Input/List
components or permission to replace Next input/result controllers.

Feature/performance findings remain read-only reference assessment, not feature
addition, ranking/DSL/indexer/service rewrite, benchmark run or new acceptance
threshold. No background screenshot sampling, theme/native/dependency/toolchain
change. This correction edits planning artifacts only; product code unchanged.

## Official Settings user amendment — 2026-10-11
The user clarifies that feature preservation does not prohibit built-in official
search or presentation features. Adopt released Settings hierarchy directly with
search/filter/resizable sidebar/scroll; do not fork the library or keep old settings
visuals. This supersedes earlier no-new-search/resize presentation constraints for
this Settings adoption only, not domain/native/window budgets or reset/persistence.
Official selection owns pages; scoped mounted business content owns its shortcuts.
Public per-field/custom-item slots keep original recorders, volume/path editors,
command master/Form and diagnostics. Existing initial-page fixture seed is not a
private selection mirror. All5topics/functions and original command/input/save
identities survive; approved launcher unchanged. Actual latest evidence in ledger.
