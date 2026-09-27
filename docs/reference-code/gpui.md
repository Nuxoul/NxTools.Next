# GPUI / Zed visual component reference index

Last verified: 2026-08-14.

## Scope

This note records only visual-component behavior used by the NxTools.Next P0 design-system work.
It does not make Zed a behavioral source of truth for download, clipboard, quick-launch or Windows
Shell workflows.

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

- One shared feature-settings shell owns the title/description header, enabled status, one binary
  toggle and an icon-only close action with a visible tooltip. Feature-specific settings are passed
  as content and remain owned by their existing Rust application.
- The enabled toggle updates local view state synchronously, then defers the write to
  `ClipboardApplication` until the next GPUI application turn. It must not synchronously re-enter a
  parent/application entity from a window callback.
- Closing removes only the settings window. Reopening a card reuses/raises its live settings window;
  a stale handle is discarded before a replacement window is opened.
- Keep the compact 460×320 client size, shared Body/Label typography, canvas/surface palette and
  continuous content area. Do not add nested cards or move native window APIs into the GPUI shell.
- Regression gates exercise the real tool card → settings window path, observe the initial enabled
  state, toggle it through the shared switch, verify content remains mounted, and close the window.
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
