# Released GPUI Kit0.7.1 component evidence

## Source and authority
Official component index https://gpui-kit.com/zh-CN/component/ was opened and inspected in the current conversation. It is an adoption reference,not proof of the exact installed release API. Published Apache-2.0 sources were inspected under session scratch `gpui-kit-071-api/gpui-component-0.7.1/src`;locked crate source is the execution authority. Existing provenance/version evidence: docs/reference-code/gpui-kit-071.md. No external documentation/code was copied here;below is a project-authored source summary. ShellRS is GPL precedent only,no implementation reuse.

## Verified entries
| Released entry | Observed capability | Task consequence |
| --- | --- | --- |
| theme/mod.rs:771-774 | Default font_size16px,mono13px,radius6px | Preserve released baseline instead of theme.rs:104-107 old type/radius remapping;keep installed CJK font family as compatibility,not reduced size |
| button/button.rs:241-265,599-708 | Medium default,theme radius,official XSmall/Small/Medium/Large geometry;normal h_5/h_6/h_8 tiers,compact padding;variants and label/icon/loading | Prefer component Button and documented Sizable/compact APIs;no forced old28px/radius3/child text-color workaround |
| input/input.rs;input/textarea.rs | Real styled input/textarea on existing state entities | Remove unexplained appearance(false)/bordered(false) plus outer painted shell;preserve silent hydration/user change semantics |
| form/form.rs:29-100,120-125;form/field.rs:100-223,242-268 | Horizontal/vertical/columns,label/description,footer,compact spacing | Compose forms without changing draft/submission ownership;no extra form auto-save or hidden workflow |
| switch.rs:78-97,133-170 | Controlled checked value;on_click/on_change are one callback;Sizable/disabled/tab-stop/keyed focus | Replace custom toggle with real Switch;write requested bool once and notify existing owner;do not wrap another keyboard activation owner |
| radio.rs:41-120,277-352 | Radio/RadioGroup,controlled checked/selected_index,on_change,disabled/tab stops | Replace self-painted selection circles;preserve full-row choice semantics without double dispatch/tab stops |
| setting/settings.rs:21-58,62-109,132-165 | Settings→Page→Group→Item→Field;Sidebar,search input;default250px sidebar/range160..360 with public overrides | Product rail widths and fixed clients are layout contracts,not permission to override control visuals;do not add new resize behavior silently |
| setting/fields/mod.rs:138-283 | Value/getter and setter closures,Switch/Checkbox/Input/dropdown/NumberInput,element/render seam | Existing owner can remain authoritative;custom field seam for hotkey/file picker or existing Input entity,not a second store |
| setting/fields/bool.rs:33-55 | Standard bool field renders actual Switch or Checkbox | Real controls survive compound settings composition |
| setting/fields/element.rs:9-34,81-90 | Public render-field callback | Preserve existing view/controller leases;never call back synchronously into leased application from render getter |
| sidebar/menu.rs:114-203;sidebar/mod.rs:238-294 | Sidebar/Menu/MenuItem with labels/icons,children,active state | Replace main/setting navigation visuals,retain original module transition/focus callbacks |
| list/list_item.rs:45-132 | Selected/confirmed/disabled/accessibility label/suffix and click/mousedown/enter hooks | Prefer component row presentation while keeping established paging/selection/generation owner;no second List controller merely to use a component |
| tab/tab.rs:478-483;tab/tab_bar.rs:59,145-157 | Tab/TabBar children and selected_index | Filter/detail/category tabs adopt actual component when drag/wheel/context semantics remain possible |
| progress/progress.rs:26,56 | Progress(id).value(f32) | Adopt presentation,keep task/domain progress source andbounded repaint policy |
| dialog/dialog.rs:329-460;dialog/content.rs;header.rs;footer.rs | Root-based modal and compound title/content/footer/action parts | Only equivalent in-window overlays may use modal API;independent native fixed windows keep HWND lifecycle and use compatible content parts |
| notification.rs:174-384 | Notification title/message/content/actions/close | Presentation candidate only;existing bounded max3,width336/max112/dedup/domain notification owner remains authority;verify whether direct rendering supports it before replacing manager |

## Styling policy under user option A
Use semantic colors only to keep graphite/neutral action direction and readable state pairs. Keep Kit spacing,type size,radius,padding,focus and state grammar. Product outer window transparency/native drag/hit regions are narrow native exceptions,not global transparent control backgrounds. No generic arbitrary Styled overrides on ordinary controls unless ledger has an approved capability/native reason. Standalone borderless search variants must use a documented Kit role andbe listed,not appearance(false) everywhere.

## Non-claims and capability gates
This source inspection does not compile a new adapter,prove user visual approval,physical input or screen-reader acceptance. During execution,prove official tiers in actual production GPUI bounds before choosing compact variants. Settings callback adapters must use local snapshot or deferred cross-owner update;do not synchronously reread a leased entity. List/DataTable/Command substitution must not change domain paging,Ctrl/Shift selection,IME,action modes or host integration. Component existence alone is not compatibility proof.

## Complete discovery prerequisite — 2026-10-11
The earlier selected-entry research is not a complete library audit. Start with
[the official component index](../../../../docs/reference-code/gpui-kit-component-index.md)
for all78 Component families, Base/Shell navigation, public release entries,
compound selection and website/API mismatches. Settings page-level adoption is
still incomplete; existing leaf tests do not close it. Exact Settings/Page/Group/
Item/Field chain is rechecked there, including private Page/Group render visibility,
internal keyed selection/search and public field/custom-item slots. Do not infer
that custom Sidebar can directly render a SettingPage without public API evidence.
