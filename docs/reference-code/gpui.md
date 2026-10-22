# GPUI / Zed visual component reference index

Last verified: 2026-08-14.

## Scope

This note records only visual-component behavior used by the NxTools.Next P0 design-system work.
It does not make Zed a behavioral source of truth for download, clipboard, quick-launch or Windows
Shell workflows.

For new visual migration, use [the standards entry](../../.trellis/spec/index.md) and
[approved-target/current matrix](../../.trellis/spec/shared/ui-design.md). Numeric Zed/P0
measurements here remain historical reference evidence; the later approved staged design
governs target tokens. This does not retire lifecycle, native geometry, lease or accessibility
limitations recorded in this note.

## User-authorized0.8.0 upgrade (2026-10-21)
The user explicitly approved a coherent Kit/Base/Component/Assets0.8.0 and default gpui-pre0.3.9 upgrade after https://gpui-kit.com/zh-CN/releases/ review. Published crates were downloaded and SHA256checked against crates.io, not Git main.0.8.0 publishes text::editor_host and includes0.7.3typography/selection/focus changes, new scroll gesture ownership and input fixes. Existing grammar features remain. This supersedes prior version pins, not business/native behavior, visual acceptance or authority for GPUI Fast/Markdown editor/package/install.
Verified boundary: Component0.8.0 notification.rs is identical to published0.7.1 (SHA25609010c94024ee6e2580ecf557d97dcf276512a52be8cf298e0936088df0afeca), stillhardcoded5s; Windows0.3.9 events.rs still unconditionally MA_ACTIVATE. Carry only original Notification::duration forwarding and live WS_EX_NOACTIVATE interpretation to fresh licensed releasecopies, never transplant the whole old vendor. Existing native paths C#SetWindowNoActivate -> live HWNDstyle -> inprocess WM_MOUSEACTIVATE, and officialToastManagerclock remain authoritative. Preserve firsthidden/finalbounds/readback/frame/reveal, samewindowfocus, callbacksnextturn/fullpayload/privacy and all original failure/cancel/lateACK regressions. Sourcegate `tests/architecture/gpui-release-contracts.py` plus scoped strict/workspace/fullDesktop/lifecycle/native separately authorized acceptance. Owning `.trellis/tasks/10-21-gpui-kit-080-upgrade/`.

## Zed source entries

Root: `D:\Code\zed-main`

| File | Verified responsibility |
| --- | --- |
| `crates/ui/src/components/button/button_like.rs` | `ButtonSize` defines 18/22/28/32 px compact/default/medium/large controls. `ButtonLike` centralizes disabled, selected, focus, tooltip, accessibility and click behavior. Custom button-like presentation is explicitly discouraged when a prebuilt control suffices. |
| `crates/ui/src/components/button/icon_button.rs` | Icon-only controls share the same button state model. They require a screen-reader label and may share that label with their tooltip. |
| `crates/ui/src/components/button/toggle_button.rs` | Segmented groups own one selected index, equal-width cells by default, contiguous corner treatment and one group accessibility/focus order. Selected state is semantic rather than an unrelated colored chip. |
| `crates/ui/src/components/list/list_item.rs` | Selectable list rows use a continuous surface, neutral hover/active/selected backgrounds and one accessible actionable node; rounding is optional rather than mandatory per row. |
| `crates/ui/src/components/list/list.rs` | A List owns one full-width child stack and a real empty state instead of pagination chrome; item semantics remain in ListItem. |
| `crates/ui/src/components/label/loading_label.rs` | Loading text remains a muted label inside the owning surface. NxTools.Next keeps that restrained hierarchy but deliberately omits the repeating ellipsis animation because the local design contract forbids idle animation loops. |
| `crates/ui/src/components/popover.rs` | Popover is an elevated surface attached to its trigger rather than to the pointer. It provides the bounded presentation shell while the caller owns the selection model. |
| `crates/ui/src/components/context_menu.rs` | ContextMenu keeps one focused menu surface, real selectable descendants, fixed icon/title slots and separators between capability groups. A confirmed handler runs before the menu emits dismissal; action dispatch has its own short selected-state delay. Escape or dismissal returns control to the invoking context. |
| `crates/ui/src/components/notification/alert_modal.rs` | Dialog content and footer are separate regions; the footer groups secondary and primary actions rather than scattering actions through content. |
| `crates/workspace/src/toast_layer.rs` | Workspace toast ownership lives in a dedicated full-window overlay; the active surface is anchored at the bottom, occludes pointer input only within the toast, pauses dismissal while hovered and enters from below. |
| `crates/notifications/src/status_toast.rs` | Status toasts use one elevated surface, restrained padding/gaps, the standard label/button/icon primitives and an optional dismiss action instead of stacking independent card treatments. |

## NxTools.Next P0 invariants

- The main titlebar is one 36 px chrome row. It uses the official 18 px brand asset, Body/Label text,
  one 28 px subtle global-settings action and a contiguous 36 px Windows-control group. The utility
  group stays outside the drag region at the 920 px production minimum; visual changes must retain
  the typed PlatformHost move/minimize/maximize/restore/close paths.
- Caption controls use four distinct embedded 12 px SVG assets. Maximize and restore are selected
  from the observed window state and must never fall back to hand-drawn borders or font glyphs. Each
  icon-only hit target exposes a stable Chinese tooltip.
- Main-window notifications live in a bottom-right overlay, not against the titlebar or module
  header. At the 920×480 production minimum, all three visible notifications must remain below the
  module action band and inside a 12 px client inset. Each 336 px surface is capped at 112 px,
  uses the shared raised surface/subtle border/small shadow, a 4 px radius and one 3 px semantic
  edge; severity color does not fill the whole card.
- Notification titles/messages use Body/Label roles, metadata alone uses Meta, and all visible
  actions use the shared 28 px desktop action tier. Dismiss/action clicks defer the cross-entity
  `NotificationCenter` update to the next application turn and freeze the rendered `(id,
  generation)` target, so an old card cannot dismiss a same-ID replacement. Existing
  generation-scoped timeout and maximum-three ownership remain authoritative.
- Normal actions, icon actions and segmented options consume the same four control heights.
- Icon-only actions expose a visible tooltip; their icon is not itself a separate hit target. Zed's
  current GPUI layer also supplies an accessibility label, but NxTools.Next's pinned GPUI 0.2.2 /
  gpui-component 0.5.1 API does not yet expose the equivalent role/label builder. DS-08 therefore
  remains open until that dependency seam is available or a typed adapter is added.
- A segmented control has exactly one selected option, stable equal-height cells and a single outer
  border/radius. Selecting an option changes local draft state before any cross-entity operation.
- Count-bearing status segments keep the count inside the equal-width cell; the count is metadata,
  not a nested badge or an independently sized chip. Changing counts must not move sibling cells.
- Setting sections and rows fill the parent content width. Ordinary Chinese title/description text
  uses Body/Label roles (12/11 px); 10 px Meta is reserved for true metadata.
- Field and path-picker shells keep a 32 px input boundary and place the picker action at the field
  edge. Validation feedback is an inline semantic row, not a floating notification.
- Compact list rows are continuous surfaces with neutral hover/selection. StatusBadge is reserved
  for status, not navigation or ordinary labels.
- SettingsShell owns one client titlebar, one scrollable content region and one fixed dialog footer.
  The shell does not own application persistence or native-window policy.
- A compact settings shell must size its production client area to the content it actually owns.

## Reusable feature settings window

Verified current NxTools.Next call chain before extracting the tool-settings shell:

- `rust/apps/nxtools-desktop/src/main_window/tools_page.rs::render_tools` opens one dedicated GPUI
  window when a tool card is clicked; `open_tool_settings_window` snapshots the persisted link-detector
  values from `ClipboardApplication` and creates `ToolSettingsWindowBody`.
- `ToolSettingsWindowBody` owns the edit-time view snapshot. `ClipboardApplication` remains the sole
  settings/persistence owner through `set_link_detector_enabled`, `set_link_panel_auto_hide_ms`,
  `set_link_panel_auto_open_browser`, `set_link_panel_copy_only` and
  `set_link_panel_position`.
  `ClipboardSettings::link_panel_position` is a serde-compatible enum with `center` as the
  default, so existing configuration files retain the current centered behavior.
- The current tool window is a fixed 460×320 logical-pixel `WindowKind::Normal` surface. Closing it
  is presentation-only; it must not disable the tool or stop clipboard capture.
- There is no verified legacy NxTools feature with the same generic “other tool settings window”
  contract. Keep the extraction presentation-only and preserve the link-detector-specific behavior.

Template invariants:

- One shared feature-settings template owns a title/close-only native drag header,
  its divider, then one scroll body containing a labeled enable row (status + Switch
  and optional-description Tooltip) and feature-specific application-owned content.
- The enabled toggle updates local view state synchronously, then defers the write to
  `ClipboardApplication` until the next GPUI application turn. It must not synchronously re-enter a
  parent/application entity from a window callback.
- Closing removes only the settings window. Reopening a card reuses/raises its live settings window;
  a stale handle is discarded before a replacement window is opened.
- Keep the compact 460×320 client size, shared Body/Label typography, canvas/surface palette and
  continuous content area. Do not add nested cards or move native window APIs into the GPUI shell.
- Regression gates exercise the real tool card → settings window path, observe the initial enabled
  state, toggle it through the shared switch, verify content remains mounted, and close the window.
- User-approved 2026-10-11 amendment supersedes the earlier title/status/Switch
  alignment candidate: feature activation belongs in the body, never beside close.
  `FeatureSettingsWindowSpec` supplies enabled_label and optional description;
  the later explicit Tooltip amendment places description beside the body label,
  not in a permanent text row. Preserve callback/normal activation/460x320 budget.
  Rechecked tools_page::open_tool_settings_window normal focus/show and Root chain;
  no legacy counterpart exists. Passive clipboard preview is not this window policy.
- The link-detector settings window exposes one position dropdown owned by
  `ToolSettingsWindowBody`; selecting an option updates the local draft and defers the persisted
  write to `ClipboardApplication`, just like the existing toggles.
- Link-panel placement is computed in Rust from the selected display bounds and fixed panel size:
  top-left and top-right use the shared 24 px display inset, while center uses both display axes.
  The resulting rectangle is clamped to the selected display before crossing the typed native
  position/bounds boundary. Reusing the panel must reapply both position and size for every capture.
- A successful asynchronous download probe changes the add window's content and its required
  height as one lifecycle transition. Native `SetWindowBoundsCentered` is the only production
  resize boundary after a real HWND exists; the resize request is generation-scoped and retried at
  bounded intervals so a Windows move/size modal loop cannot leave the confirmed body clipped.
  Conforming controls inside an oversized fixed viewport is not a valid Zed-like result: the final
  section/note keeps only the ordinary 12 px content inset above the fixed footer. The old NxTools
  `Views/SettingsWindow.axaml` used a 984×612 workspace because it contained many global sections;
  that size is not inherited by the smaller Next global-settings subset.
- Module-header actions use the shared 28 px action tier and one 4 px group gap. A secondary sort
  action stays adjacent to its segmented filter instead of being pushed into an isolated far-edge
  text target. These are layout invariants; search/filter/sort continue to own only local view state.
- Download, quick-launch and clipboard use one 48 px `ModuleHeader`: Title is 14/20, bounded
  metadata is 11/16, and only machine-like counts use the Mono role. Module switching must not
  change the workbench's vertical origin or invent a module-specific title scale.
- The main sidebar remains a fixed 208 px rail with 42 px rows. Its two-line label uses Body/Label
  roles and a stable selector; selection changes presentation only and keeps module ownership in
  `MainWindow`.
- Popover/ListBox shells are bounded, use the 6 px floating radius and never create an unbounded
  child list. Tooltip text may reveal a complete bounded path but must not be logged.
- Non-list overlays such as the delayed-start calendar reuse the same Popover shell and shared
  action tier; their domain-specific content remains owned by the caller rather than by the visual
  wrapper.
- Context menus reuse the dependency's keyboard/dismiss/focus lifecycle while NxTools.Next owns one
  shared 28 px item presentation, a stable icon slot, 12 px labels and normal/danger tones. Their
  shell stays between 176 and 240 px; destructive rows use danger text/hover semantics rather than
  a permanent filled background.
- Dense task tables keep a flexible two-line identity column beside fixed numeric columns. Live
  values reuse the owning application's bounded presentation snapshot; rendering does not create a
  second polling or diagnostic lifecycle. Ordinary file names use Body typography and paths/errors
  use Label typography, with full clipped failures available through an existing detail surface or
  tooltip.
- Empty, loading and recoverable-error presentation stays inside the content boundary that owns the
  missing data; it does not create a centered card, shadow or second surface. The three shared states
  use one 18 px icon slot, Body title, Label description, 8 px text gap and 12 px content inset.
  Empty is neutral, loading uses only the accent icon, and error uses only danger icon/text plus a
  narrow semantic edge. Loading is static unless the underlying operation already exposes bounded
  progress; rendering does not start a timer or animation loop.
- Interactive empty states use the same `ui/components/state.rs` module as passive states. The
  action wrapper is `flex_none` and delegates geometry to the 72 px work-state contract; callers
  provide only the deferred action. A page must not wrap the state in `size_full()` to make it
  clickable, because that turns a compact status row into a large empty panel.
- Old NxTools clipboard history used a centered muted 12 px `暂无剪贴板历史` label, while the LAN
  history window used a decorative 38 px icon/title stack. SH-07 preserves the useful empty-state
  conditions and explanatory copy, but standardizes their presentation on the denser Zed list
  boundary rather than carrying both legacy visual dialects forward.
- GPUI entity callbacks update their own draft synchronously and defer any cross-entity operation to
  the next application turn.

## Required regressions

- A 920 px production main window proves titlebar utilities remain inside the titlebar and outside
  the drag region, window controls are contiguous, and real clicks still emit the typed native
  commands without destroying the owned `MainWindow` entity.
- The asset registry gate loads every caption SVG, and the maximized-state selector proves maximize
  and restore map to distinct IDs and assets; the real-HWND gate remains authoritative for native
  maximize/restore state changes. Hovering a production caption target must render its tooltip.
- A production settings window proves titlebar/content/footer geometry through the shared shell.
- The production global-settings window proves its actual native bounds and that the final content
  row ends at the normal inset above the fixed footer, without a large unused viewport.
- A production module header proves all ordinary actions are 28 px, remain inside the header, and
  form one stable action group; the sort trigger remains adjacent to and inside the filter strip.
- One production `MainWindow` switches through all three real sidebar rows and proves identical
  48 px module-header and 20 px title-line geometry without recreating the shell.
- A real segmented option and toggle click mutate only the local settings draft.
- A production count-bearing segmented group proves all cells have equal geometry and that a real
  click changes only the owning view's presentation filter.
- Apply and Cancel continue through the existing production callbacks; component extraction must not
  replace those lifecycle gates with component-only unit tests.
- Long inline error text remains inside the settings content width and above the fixed footer.
- A production task row opens the real context menu, proves the shared width/row geometry and
  capability omissions, and Escape dismisses it while returning focus to the invoking surface.
- Asset, tooltip or list presentation must not add Header, Cookie, path or other sensitive values to
  diagnostics.
- A production 920×480 `MainWindow` with progress, failure and detailed success notifications proves
  the three-card stack stays below titlebar/module actions and inside the client edge; it also clicks
  the real 28 px action and dismiss targets and observes generation-scoped exit state.
- Production download, quick-launch and clipboard surfaces exercise the shared empty/loading/error
  states and prove identical icon/title/description geometry without changing their existing query,
  task-detail or clipboard-history ownership.

## Foundation start evidence — 2026-10-09

Approved staged UI tokens override the historical Zed numbers above, not lifecycle semantics. Before shared-shell changes, rechecked Zed button_like.rs (one state/focus/action boundary) and list_item.rs (continuous selected rows); old NxTools MainWindow.axaml/axaml.cs constructor/Open/Closed/OnClosing/tray flow and WindowAnimationService Attach/Open/closing cancellation/final Close. Avalonia native/base implementation is absent. Current MainWindow render/titlebar routes native commands through C#; local module switch preserves controller state and clears outgoing menu focus. Tools render -> open_tool_settings_window owns one registered window per tool; tool settings persist through deferred ClipboardApplication updates. Foundation changes no Windows show/hide/resize owner.

Pinned vendor Button supplies focus/tab/disabled/style processing; its own render has no activation listener because GPUI0.2.2 elements/div.rs::paint_click_listeners generates Enter/Space ClickEvent::Keyboard on a focused node. Focus is therefore the missing boundary on the old custom sidebar, not another activation framework. Reuse track_focus/on_click, add visible focus/Tab navigation and suppress held activation before the built-in click path; prove single dispatch. Native screenshots/provider evidence remain separate from TestAppContext bounds.

Foundation implemented approved palette and shared control/radius mapping,48px four-grid module rail and continuous Tools representative row on2026-10-09. Fixed42px settings-row budget retained after actual media/network regressions; no global content-height workaround. Directed review verified GPUI tab stops globally sort unscoped explicit indices, so Foundation uses render order; actual Downloads->QuickFind and reverse focus covered. Primary hover/pressed remain distinct. Detailed actual commands/red evidence/native visual limits: docs/audits/2026-10-09-foundation-ui-polish.md. No native screenshots or phase acceptance inferred from444 desktop tests.

Native visual follow-up now supplies Debug-only isolated production MainWindow/Root and synthetic NewDownloadView rendering with owned-HWND PrintWindow screenshots at100% DPI. MainWindow window-message pointer and Tab/Enter selection verified. Entry intercepts before all runtime/protected/summon initialization; Release rejects it. In-memory download fixture does not probe/submit or connect platform. See Foundation audit follow-up; browser screenshot guest unavailable, so PNG acquisition is not agent aesthetic approval or C# resize/pointer/DPI acceptance.

## Consolidated common UI verification — 2026-10-09
User approved all module/common capability implementation before consolidated acceptance. Rechecked old SettingsWindow.axaml.cs constructor WindowAnimationService.Attach, OnOpened/ConfigureForCurrentScope, native drag and apply/close handlers plus old MainWindow Open/Closed/tray-preserving close. Current settings_shell keeps fixed footer and bounded content; settings_toggle was pointer-only. Zed ButtonLike tracks focus and accessible roles, whereas pinned GPUI0.2.2 exposes focused KeyUp clicks but its source contains no accessibility/accesskit/WM_GETOBJECT/UIAutomation node provider or role builder. A platform.rs label match belongs to unrelated platform data, not rendered elements. Actual reader names/roles require an unavailable GPUI-native provider seam; tooltips are not a replacement. Shared toggle/caption keyboard work uses existing keyed focus/GPUI click, preserves native drag area and defers cross-owner effects.

### Shared selection-row keyboard continuation
Rechecked ABDM AddDownloadPage.kt `OnDuplicateStrategySolutionItem` (single row clickable plus same checkbox callback) and legacy AddDownloadWindow.axaml ListBox/HLS/duplicate selectors and category ComboBox. Shared `compact_list_row` and `radio_list_row` currently own a single selection callback, not lifecycle or native policy. Preserve that callback and measured40px row/content/radio-dot geometry; add keyed focus through the existing GPUI KeyUp click boundary and an overlay focus ring, never a second Enter handler. A disabled category is visually disabled for automatic classification but remains selectable in settings for editing; it is not a disabled action. Compact result horizontal padding migrates7→8 without changing row heights. Existing category search menus retain one Up/Down/Enter controller. Native show/hide, cancellation, center/placement and auxiliary focus boundaries are unchanged.

Shared-settings continuation rechecked Zed list_item.rs `on_click`/accessible actionable node and old SettingsWindow.axaml command/control composition. Pinned vendor Checkbox has its own focused KeyUp click even with no callback; a nested display-only checkbox therefore must not expose an inert tab stop. Volume-scope rows will own one focus/click action, with noninteractive checkbox glyphs. Custom-command master selection remains editor-only (no execution), measured18+4+16 fits40px; enabled/delete remain detail actions. Exclusion paths retain the existing parent Up/Down/Delete controller. `hotkey_field` entry is pointer-only before this continuation; the MainWindow recording controller only captures KeyDown while `recording_hotkey` is set, so a single focused KeyUp entry safely transfers focus into that controller without recording its activation key. Applying/waiting/recording eligibility must be preserved. No native placement/show/failure/cancellation boundary changes.

## Feature-first production layout migration after checkpoint42f60a8
User explicitly authorizes local UI implementation with original functionality as the source of truth: browser prototype omissions never authorize removal of controls, settings, actions, shortcuts or failure states. Rechecked Zed ListItem start/end/selected/focus boundary; legacy MainWindow constructor/OnOpened/OnClosed/OnClosing: startup/services deferred after show, close-to-tray cancels close, explicit exit follows base final close. Legacy SettingsWindow constructor attaches WindowAnimationService, Open configures scope, Apply/Cancel owner commands then Close, unhandled final close cancels; Avalonia native base remains unavailable. Next MainWindow module switches retain owners/state; native titlebar/window controls still route to C# NativeMethods visibility/minimize/maximize. Initial migration changes rail/type/representative form projection only, preserves titlebar/global-settings entry and all module/settings operations. Module descriptions become tooltips, not deleted information. Native popup/download budgets and shared compact typography remain separate from workbench type roles.

## Remaining supporting surfaces: notification message/action budget
Rechecked Zed notifications/status_toast.rs new/action/render boundaries and existing Next NotificationCenter generations,MainWindow render overlay/action/dismiss. Legacy owner show/close/tray semantics remain unchanged. Notification layout uses bounded scrollable text/details separate from stable action/footer so long messages cannot clip the operation. Same max3/bottom-right336x112 budget,caller generation/deferred dismiss,one KeyUp and reduced-motion semantics preserved; no changes to lifecycle controller/native show boundary.

### Remaining bounded scroll/action layout recheck (2026-10-09)
Zed `crates/notifications/src/status_toast.rs` was inspected for message/action separation only, not notification lifetime/native ownership. NxTools.Next retains max3/336px/112px cards and generation-scoped deferred dismissal. Long title/message/details use an identified bounded GPUI scroll viewport with a separate nonshrinking28px action; no new timer/frame logs or omitted details. Clipboard long-source editor uses the same principle within its existing budget. Actual production bounds regressions caught scrollbar-wrapper overflow/action displacement; explicit overflow_y_scroll at the constrained container fixes them. GPUI test window.resize alone can leave1920×1080 viewport; fixed/minimum fixtures also simulate_resize before asserting containment. This is real compiled geometry/action evidence, not native screen-reader or aesthetic acceptance. [Audit](../audits/2026-10-09-remaining-ui-layout-migration.md).

### Installed feedback: neutral navigation and focus-visible correction (2026-10-10)
User rejects bright cyan actions/selection/focus and the mouse-retained rectangle in both Global and Download settings. Rechecked Zed ButtonLike:797–860: selected/hover/active roles remain separate and focus_visible applies keyboard presentation, not a permanent pointer outline. ListItem:312–327 separates focused border from neutral ghost selected/hover surfaces. Pinned GPUI0.2.2 has no focus_visible API; its window.on_key_event/on_mouse_event are paint-phase, frame-owned listeners, and canvas supplies a safe registration seam. Rechecked ABDM AddDownloadPage:91–114 first, then legacy SettingsWindow constructor→WindowAnimationService.Attach→base OnOpened/ConfigureForCurrentScope and SettingsNav_Click→SetActiveNav→scroll; Avalonia native base remains unavailable. Only Next presentation/input-modality changes are authorized: retain mouse focus, Tab/Shift+Tab, original KeyUp single activation/disabled guard, field input/cancellation/entity/native owners and all measured budgets. Neutral graphite roles supersede the earlier user-approved cyan color; no dependency/vendor change or second activation framework.

### Continuous settings navigation correction (2026-10-10)
User-installed screenshot shows General/Network/Blacklist labels on unequal intrinsic-width hover/selected tiles. Rechecked Zed ListItem:303–327 full-width continuous row and independent hover/selected state, legacy SettingsNav_Click→SetActiveNav→scroll with unchanged constructor/base lifecycle; ABDM add/config presentation inspected first for download scope. Production regression ran red at40px and now asserts all three actual Global rows fill the128px rail's111px inner width (16px insets+1px separator) at720×480 through hover/selection states. Shared focusable-content wrapper owns explicit stretched column geometry so child100% width cannot resolve to label intrinsic width. Download navigation uses its existing48px tier and147px inner visual within8px margins; an explicit100% width plus margins overflowed and was removed, preserving automatic stretch. Selected hover retains selected plane. No controller/Save/Cancel/native focus/placement/dependency change.

### Apply-button foreground state regression (2026-10-10)
User reports white label blending into light neutral Apply. Zed ButtonLike disabled/hover states inspected for independent readable text/background, not native ownership. Real GPUI paint-time text-style fixture with current action_button/variant is red: inherited foreground overridden by component hover/state styling. Project-owned primary action must explicitly set label foreground and disabled background/foreground, preserving32px footer/input tier, disabled eligibility and original click/KeyUp boundary. Vendor source only inspected, no dependency/vendor patch. Test normal/hover/disabled actual painted text, not only theme constants; existing screenshot isn't proof of which transient state produced it.

## User-supplied ShellRS visual reference
2026-10-10 user requests renewed UI reference against D:\Code\shell-rs-main. [ShellRS UI note](shellrs-ui.md) indexes verified theme/settings/form/nav/table/status/Action/Radio/dialog composition and source tests, with current-stack/API and GPL-3.0-only boundaries. Learn coherent component/state/alignment principles, not only palette. No executable build/launch, copied code/dependency migration, numeric target override, native acceptance or Next product edit performed by this reference study.

## Queue10 shared state/notification prerequisite
Rechecked ABDM Desktop AppComponent::sendNotification/showNotification tag effect and
originalNxTools WindowAnimationService Attach/Open/Closing/publicClose plus settings
ApplyCancel and popup HidePopup/one passivepreview chain. Legacy nativeAvalonia base
unavailable; no equivalent legacy boundedNextNotificationCenter claimed. Next manager
max3/upsertID/generation/6-10s terminal timers/120ms exit and deferred actiondismiss remain.
Released0.7.1 Empty/Alert/Spinner/Notification publiccontent/action/autohide/on_close and
Notification::dismiss/privateDismissRequest/NotificationList lifecycle reviewed before
changes. Test actual directpresentation/manager seam,not reject by manager existence.
Sharedprogress_state is loading copy,not taskpercentage; keep loading/error/empty distinct.
NativeDialog/titlebar/OLE/passivehosts remain; only compatible innerpresentation changes.

Queue10 verified selection: Empty/EmptyContent/literalLabel, XSmallAlert and loading
SmallSpinner only under original motion policy; no determinate loading percentage.
Notification public standalone render succeeds, but dismiss emits privateDismissRequest
and direct on_close0 without NotificationList completion. Only this adapter limited;
other public adapters unverified. Original bounded manager retained with actual semantic
Alert ORIGINAL copy/Labeldetails/Smallaction/XSmallclose and tested nonoverlap/scroll.
PopupMenu ElementItem publicicon/naturalLabel preserves danger copy without customrow;
default26px menu owns chrome. Recorder entry actual32outlinedButton/Label keeps capture
owner; nativeclienttitle36/draghost retained, closeSmall24 removes duplicatehoverpaint.
Original nativeDialog/OLE/passive/Settings/Form/Footer owners unchanged. Exact tests,
consumer classification/limits and pending decision: owning ui-queue-outcomes.md10.
615Desktop/strictchecks/11sourcegates are not visual/native/physical/DPI/reader acceptance.
