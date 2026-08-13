# GPUI / Zed visual component reference index

Last verified: 2026-08-13.

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
| `crates/ui/src/components/popover.rs` | Popover is an elevated surface attached to its trigger rather than to the pointer. It provides the bounded presentation shell while the caller owns the selection model. |
| `crates/ui/src/components/notification/alert_modal.rs` | Dialog content and footer are separate regions; the footer groups secondary and primary actions rather than scattering actions through content. |

## NxTools.Next P0 invariants

- Normal actions, icon actions and segmented options consume the same four control heights.
- Icon-only actions expose a visible tooltip; their icon is not itself a separate hit target. Zed's
  current GPUI layer also supplies an accessibility label, but NxTools.Next's pinned GPUI 0.2.2 /
  gpui-component 0.5.1 API does not yet expose the equivalent role/label builder. DS-08 therefore
  remains open until that dependency seam is available or a typed adapter is added.
- A segmented control has exactly one selected option, stable equal-height cells and a single outer
  border/radius. Selecting an option changes local draft state before any cross-entity operation.
- Setting sections and rows fill the parent content width. Ordinary Chinese title/description text
  uses Body/Label roles (12/11 px); 10 px Meta is reserved for true metadata.
- Field and path-picker shells keep a 32 px input boundary and place the picker action at the field
  edge. Validation feedback is an inline semantic row, not a floating notification.
- Compact list rows are continuous surfaces with neutral hover/selection. StatusBadge is reserved
  for status, not navigation or ordinary labels.
- SettingsShell owns one client titlebar, one scrollable content region and one fixed dialog footer.
  The shell does not own application persistence or native-window policy.
- Popover/ListBox shells are bounded, use the 6 px floating radius and never create an unbounded
  child list. Tooltip text may reveal a complete bounded path but must not be logged.
- GPUI entity callbacks update their own draft synchronously and defer any cross-entity operation to
  the next application turn.

## Required regressions

- A production settings window proves titlebar/content/footer geometry through the shared shell.
- A real segmented option and toggle click mutate only the local settings draft.
- Apply and Cancel continue through the existing production callbacks; component extraction must not
  replace those lifecycle gates with component-only unit tests.
- Long inline error text remains inside the settings content width and above the fixed footer.
- Asset, tooltip or list presentation must not add Header, Cookie, path or other sensitive values to
  diagnostics.
