# Component-first UI selection

## Authority and required reading
User correction2026-10-15 rejects Extensions prose-list layout and borderless header actions; tests/imports do not override product intent. FIRST read this contract, then [current0.8.0 release/API delta](../../../docs/reference-code/gpui-kit-080.md), [historical79-entry table](../../../docs/reference-code/gpui-kit-component-table.md) and [API index](../../../docs/reference-code/gpui-kit-component-index.md), [UI authority](ui-design.md), and exact current released compound source BEFORE editing. The [offline viewer](../../../docs/reference-code/gpui-kit-components.html) preserves the2026-10-15pageaudit, not a GPUI prototype or fabricated new audit.

Evidence: `repo:Cargo.toml`, `repo:rust/apps/nxtools-desktop/src/main_window/tools_page.rs#render_tools`, `repo:rust/apps/nxtools-desktop/src/main_window/extensions_page/workspace.rs#render_workspace`. User-approved2026-10-21upgrade pins exact Kit0.8.0/defaultpre0.3.9; existing narrowduration/noactivatepatches carried only. Historicalcatalog0.7.3website/0.7.1sourcehashes remain historical. Diff and public text::editor_host are now available, not automatically adopted. No further upgrade/Fast/neweditor without approved scope.

## Mandatory selection order
1. Match the USER'S visual and interaction reference first. A named component does not dictate whether business items form rows, tiles or a grid.
2. Inspect official styled **compound**: Settings for settings, Form for an edit/decision, List/DataTable/Command for their real data/interaction needs, Dialog/Popover/Sheet for the correct overlay owner.
3. Reuse the existing product data, paging, selection, focus and native owners. Compose public styled parts/slots; do not introduce a competing controller just to use a component.
4. Choose visible variants/states deliberately. Verify final parent transformations, not only the child's builder calls.
5. Only use Base/custom ordinary controls after an exact released public capability gap is proved and a narrow exception approved. Layout divs are allowed; hand-painted replacements for available Button/Input/Switch/Dialog are not.

## Scenario-to-component table
| Need | First choice | Visible/behavior contract |
| --- | --- | --- |
| Tool/plugin catalog cells | Kit ListItem with Icon/Label/Tag and real Button actions; GroupBox for noninteractive semantic groups; GPUI grid/wrap only arranges cells | Follow Other Tools tile reference; NOT DataTable because the user said cells; NOT a long prose list. No imaginary Card/Grid component. Compact purpose/trigger/status on tile, full details on explicit open. Keep enable separate from run. |
| Standalone header actions: import, environment, chain, recent activity | Button::new(id).label(...).outline(), Medium/default32; one primary variant only when semantically primary | Recognizable button bounds in Default Light/Dark at rest; no bare-text/ghost action unless explicitly requested. Default/Outline/Primary/Disabled/Loading are distinct decisions. |
| Quiet editor command toolbar | Toolbar/ToolbarGroup child controls | child calls prepare_for_toolbar -> Button becomes ghost().compact(), then final size. This is intentional for quiet editor commands, NOT the default for bordered app-header actions. |
| Bordered actions within a semantic toolbar | Public Toolbar.content(custom layout with actual outlined Buttons); explicit control size | content bypasses sized-child adaptation; keep official buttons, verify roving focus and Tab/arrow behavior. Never patch dependencies or draw a fake border around text. |
| Multi-page settings | Settings -> SettingPage -> SettingGroup -> SettingItem -> SettingField | Keep save/draft owners and meaningful pages; a page full of divs inside one render slot is not full adoption. |
| Edit draft / confirm import or trust | Form/Field + Dialog content/footer/action/close parts | Body scrolls, Save/Cancel and warnings reachable; fixed native budgets and original cancellation. |
| Search input with actions | InputGroup and named addon/button/input parts | One painted frame, original stable InputState/IME. Simple prefix can use Input. |
| Single line / plain multiline / source code | Input / Textarea / Editor | Do not use Editor for ordinary prose. |
| Small static rows/columns / large sortable selected data | Table / DataTable respectively | Catalog tiles are neither by default; DataTable introduces its own state/delegate and needs compatibility proof. |
| Existing pageable history / command palette | Existing owner + stateless ListItem / Command if compatible | Preserve original bounded paging/session; no new Command controller for a simple menu. |
| Boolean setting / independent checks / exclusive option / pressed filter | Switch / Checkbox / RadioGroup / ToggleGroup | One requested-value callback, actual keyboard owner; not clickable text or drawn fake controls. |
| Current status / transient notice / missing data / unknown wait | Alert or Tag / existing Notification owner / Empty / Spinner | State is readable without color alone; no invented percentage or constant animation. |
| Metadata / rich prose / keyboard hint | DescriptionList / TextView / Kbd | Metadata must not replace primary purpose/entry; optional help uses Tooltip, real errors stay visible. |
| Local anchored content / modal choice / secondary drawer | Popover / Dialog or AlertDialog / Sheet | All are in-window; none replaces independent passive preview HWND lifecycle. |
| Navigation / same-surface partitions / hierarchical content | Sidebar / TabBar+Tab / Tree | Keep original selection/focus and no duplicated dispatcher. |
| Group of content / expandable technical detail | GroupBox / Accordion or Collapsible | Do not hide first-use purpose, critical warning, active error or primary action in hover/collapse. |

## Exact released0.8.0 gotcha: Toolbar still changes appearance
Published current Component0.8.0 toolbar.rs::ToolbarItem::sized calls item.prepare_for_toolbar().with_size(size); button/button.rs::Sizable::prepare_for_toolbar remains self.ghost().compact(). Toolbar defaultSmall; parent changes variant AND size. Same behavior as historical0.7.1. Use public Toolbar.content for recognizably outlined app actions, never assume Medium restores outline.

Correct public composition candidate (a source-verified recipe, NOT a claim of native/keyboard acceptance):
```rust
use gpui_kit::{ParentElement as _, Styled as _, div};
use gpui_kit::component::{Sizable as _, Size, button::Button, toolbar::Toolbar};

let header = Toolbar::new("extension-header")
    .with_size(Size::Medium)
    .content(div().flex_1().child("扩展"))
    .content(div().flex().items_center().gap_2()
        .child(Button::new("extension-import").label("导入扩展")
            .outline().with_size(Size::Medium)));
```
Keep callbacks deferred through the existing owner. Test actual final variant/paint, disabled/loading, pointer, Tab/ShiftTab and toolbar arrows; public composition alone does not prove keyboard compatibility.

## User-approved Extensions target, not yet implemented by this documentation change
- Use the recognizable catalog-cell/tile form referenced from Other Tools. Existing Tools uses a180x160 ListItem-based cell; that measurement is reference evidence, not a requirement to squeeze all plugin content into exactly180x160.
- Tile hierarchy: icon/name, concise purpose, trigger type/status, clear detail/configuration action. Put full handler/declaration/history detail in the existing detail surface, not repeated paragraphs on every tile.
- Header operations need visible official outlined button boundaries. Compact overflow choices may use official Menu/DropdownButton if required, not a row of unframed labels.
- These requirements supersede the earlier Extensions list-first presentation. Preserve import/runtime/trust/settings/events/download chains/manual feedback behavior. This document does not implement the redesign or grant packaging/install authority.

## Acceptance checklist for every UI task
- Record reference shape, exact component/compound, locked source/API, variant/size, parent adaptation and any narrow exception before editing.
- Compare actual current render with user reference, including idle button boundaries/cell separation; type imports, heights and test counts are NOT visual approval.
- Exercise pointer and keyboard, focus restoration, disabled/loading/error/empty/success, default/minimum and Light/Dark; long text scrolls without hiding actions.
- Report separately: source review, compiled behavior, actual screenshot inspection, physical/DPI/native and USER acceptance. Never mark a user-rejected screenshot approved.
- No palette fork, unapproved dependency/backend upgrade, registry edits or replacement runtime to force visuals. Only two carried source patches in0.8.0releaseindex are approved; missing review means unreviewed, not unsupported.

## Reproducible documentation gate
Run `python tests/architecture/gpui-component-catalog.py`, `python tests/architecture/gpui-release-contracts.py` and `python tests/architecture/spec-contracts.py`. Catalog preserves historical79rows/78sourceentries and validates current0.8.0authority links; releasegate checks coherent0.8.0/pre0.3.9and narrowupstreamdeltas. These are source/docs checks, not livewebsiteAPI, compiledinteraction, native or visualacceptance. Provenance [historicalcatalog](../../../docs/reference-code/gpui-kit-component-catalog.json), [currentrelease](../../../docs/reference-code/gpui-kit-080.md).

## Explicit2026-10-22 scope amendment
The approved six-item plan adds a pinned unified Markdown document/editor backend and project Graphite Light/Dark defaults through public Kit APIs. This is an explicit exception to historical no-new-editor/built-in-palette-only wording, not permission for further vendor edits or controls painted as substitutes. Settings remain official32DIP dropdown fields, edit modes Small outlined Buttons outside Toolbar.child adaptations. Framework patches remain exactly notification.rs/events.rs; separate editor provenance permits only element.rs passive mode and3cull.rs stable lowbits. See [current contracts](../nxtools-desktop/backend/ui-experience-contracts.md), [editor](../../../docs/reference-code/markdown-editor.md) and [theme](../../../docs/reference-code/graphite-theme.md).
The later integration-closeout amendment additionally authorizes only one equivalent serializer
expression and rustfmt in that fragment/passive-link if; exact reverse-hash checks enforce this
boundary. See the current editor reference; no full-vendor formatting or further source permission.

## Explicit2026-10-23 notification header amendment
User approves optional generic `Notification::header_action` within existing notification.rs only, to place actual XSmall ghost Copy before original Close. Same officialtoolbar/list/clock/exit/nativeowners; headerfocus keeps controls visible across GPUI keyboard-modality hover suppression. No globalCtrlC/customkeydispatcher/newclipboardengine or further vendorfile. Duration and header APIs are preserved at https://github.com/Nuxoul/gpui-kit, not an automatic dependency switch. Exact behavior/tests: [notification contract](../nxtools-desktop/backend/ui-experience-contracts.md); current [release delta](../../../docs/reference-code/gpui-kit-080.md).
