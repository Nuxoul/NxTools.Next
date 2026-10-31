# GPUI Kit component index: current0.8.0 delta and historical0.7.1 map

## Current product0.8.0 (2026-10-21)
Read [current release/API/patch delta](gpui-kit-080.md) FIRST. User explicitly authorized exactKit0.8.0/defaultpre0.3.9; priorpins below are historical, not a prohibition on this approvedupgrade. All priorcatalogsourcecoordinates still have0.8.0releasefiles; Diff/publiceditor_host now exist. CurrentToolbarparentghost+compactrule persists. The historicaltable/pagehashes below are NOT rewritten as a new79pageaudit and do not certify every0.8.0method/interaction. Newvendorpaths and patchprovenance are in currentreleaseindex; use exact0.8.0source for newwork.

## Historical component re-audit (2026-10-15)
FIRST read the short mandatory [component selection rules](../../.trellis/spec/shared/ui-component-selection.md), then the [79-entry historical table](gpui-kit-component-table.md) or [offline searchable viewer](gpui-kit-components.html). The 2026-10-15 audit found website 0.7.3 and 79 Component pages; all 79 were fetched/read, with Diff then unavailable in product-locked 0.7.1 (no module/export). The 78-entry table below remains the detailed **historical locked 0.7.1 source map**, not a fresh 0.8.0 inventory. Base 47/Shell 15 were not refreshed in that audit. [Machine-readable provenance](gpui-kit-component-catalog.json) retains its page/source digests and version distinction.

User rejected the latest Extensions gallery: long prose rows must yield to Other Tools-style catalog cells/tiles, and standalone header commands need recognizable outlined official Buttons. This is not fixed by importing components or passing height tests. Source-verified0.7.1 call chain: Toolbar.child -> ToolbarItem::sized -> Sizable::prepare_for_toolbar -> Button.ghost().compact(), then with_size. The parent changes **variant as well as size**. Toolbar.content can host public custom layout of actual outlined Buttons without that adaptation; final focus/keyboard/paint must still be tested. ListItem/Label/Icon/Tag/Button can compose cells; there is no inspected public Card/Grid component to invent, nor reason to use DataTable for catalog tiles. The component-selection rules supersede the old list-first presentation; this documentation change does not implement it.

Notification selection correction2026-10-17: the historical queue10 custom Alert/manager decision below is superseded by verified and user-approved official Notification/NotificationList/ToastStack integration. One official list owns fixed5second/group pause/card/close/stack/animation; the existing NotificationCenter is only bounded business ingress/generation/source/action metadata, not a competing display clock. Public content supports two real Buttons; official placement settings move existing entities without timer reset. See [capability proof](../../.trellis/tasks/10-16-menu-notification-refinement/official-notification-capability.md), [approved boundary](../../.trellis/tasks/10-16-menu-notification-refinement/official-notification-integration.md) and [source/native limits](../../.trellis/tasks/10-16-menu-notification-refinement/official-notification-integration-outcomes.md). No dependency upgrade or package acceptance is implied.

Historical user-approved 2026-10-20 amendment: the then-pinned 0.7.1 carried a minimal licensed local Component Notification::duration(Duration) patch after 0.7.3 and 0.8.0 source verification found identical hardcoded 5s/no duration API. The original official list/clock/pause/card/stack/close/animation remains; no second manager. NxTools config 1–60/default 6 snapshots only new notices, raw unconfigured upstream remains 5. Project public content fixes 54px clipping with natural height/bounded scrolling and actual native allocation. The old patch at `vendor/gpui-component-071/NXTOOLS-PATCH.md` is now inactive; the same narrow exception is carried to 0.8.0 as recorded in [current release](gpui-kit-080.md). Neither historical source review nor this upgrade implies installed acceptance.

## Purpose and authority

This is the mandatory discovery index for Desktop UI work, not a claim of completed
adoption. Inspect the official compound component before deciding to compose a
page from leaf controls or keep the old hand-painted layout. The user requested
this index after the Settings page-level component was overlooked despite being
in the approved migration plan.

- Historical inventory date: 2026-10-11; product dependency at that time was `gpui-kit = "=0.7.1"`. Current [Cargo.toml](../../Cargo.toml) pins 0.8.0.
- Historical [Component catalog](https://gpui-kit.com/zh-CN/component/) snapshot: 78 named documentation entries, excluding the landing page; the separate 2026-10-15 audit recorded 79. Counts describe documentation families, not Rust types.
- Historical [Base catalog](https://gpui-kit.com/zh-CN/base/):47 navigation entries including Getting Started/shared infrastructure; not refreshed in2026-10-15 Component-only re-audit.
- Historical [Shell catalog](https://gpui-kit.com/zh-CN/shell/):15 documentation entries, not15styled Rust components; not refreshed this time.
- The historical 78-page audit and later 79-page audit inspected purpose/import/public entries; neither claims every method/example compiled, every demo executed or every interaction visually accepted.
- The later historical website selector was 0.7.3 and differed from product-locked 0.7.1. For current work follow exact published 0.8.0 source and [release delta](gpui-kit-080.md), not guessed imports or historical API limits.
- Published-source notation below: `C:` means `gpui-component-0.7.1/src/`, `B:` means `gpui-base-0.7.1/src/`, `G:` means `gpui-pre-0.3.8/src/`. File paths are crate-relative, not workspace files. Public import prefixes are `gpui_kit::component::`, `gpui_kit::base::` and `gpui_kit::` respectively.
- On this workstation the verified published snapshots are under `C:\Users\Administrator\.pi-desktop\scratch\b54c35cc-1a3b-45d3-ba44-64d6f226efc4\gpui-kit-071-reference\`. They are a local lookup convenience; restore the exact published version if unavailable, never silently substitute main/latest.
- Provenance, native exceptions and detailed release contracts: [gpui-kit-071.md](gpui-kit-071.md) and [release research](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/research/kit-release-contracts.md).
- This is a project-authored summary, not copied documentation. Official prose/diagrams are credited to GPUI Kit / Longbridge under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); linked code examples and published software retain Apache-2.0. Raw page snapshots and intermediate inventories stay in session scratch.

## Start with the surface, not the control

| Product need | Inspect first | Then inspect | Ownership constraint |
| --- | --- | --- | --- |
| Multi-page application settings | **Settings / SettingPage / SettingGroup / SettingItem / SettingField** | Sidebar, GroupBox, Form; custom `render` / `element` slots | Keep section saves and existing owners; no blanket custom-page exemption |
| Edit a draft and submit/cancel | **Form / Field** | Input, Textarea, NumberInput, Select, Checkbox, Radio; Dialog content parts | Form is layout, not a replacement persistence/validation engine |
| Search with icons, clear or action addons | **InputGroup and named addons/controls** | Input prefix/suffix for simpler cases | One painted frame; retain original entity and IME intent |
| Ordinary single-line / multiline / source text | **Input / Textarea / Editor**, respectively | Their corresponding state/event types | Do not choose Editor just to get a larger text box |
| Single choice / searchable multi-choice | **Select / Combobox**, respectively | SearchableList delegates/groups | Preserve confirmation/cancel and original value owner |
| Boolean setting / independent flags / toolbar pressed state | **Switch / Checkbox / Toggle**, respectively | RadioGroup for exclusive choices; ToggleGroup for button selections | Controlled requested values and one dispatcher |
| Module or settings navigation | **Sidebar / SidebarMenu / SidebarMenuItem** | Tabs/TabBar for same-surface partitions | Preserve page selection, keyboard traversal and unmount focus |
| Small static rows and columns | **Table** | DescriptionList for key-value metadata | No DataTable controller for a static summary |
| Large sortable/selectable grid | **DataTable / TableDelegate / TableState** | Column, sort/fixed columns, context/custom cells | Measure Ctrl/Shift, persisted sort, OLE and controller compatibility |
| Searchable/selectable business list | **List / ListDelegate / ListState**, or ListItem as leaf presentation | VirtualList for low-level variable-size viewport rendering | Do not replace an established generation/paging controller by accident |
| Command palette / searchable action collection | **Command / CommandState / CommandItem / CommandGroup** | InputGroup, ListItem, Kbd if existing controller must remain | Evaluate first; no automatic substitution of Quick Find engine/session |
| Action row / bottom information rail | **Toolbar / ToolbarGroup / StatusBar** | Button, DropdownButton, Separator, Label, Kbd | Preserve actual actions and original dispatch owner |
| Empty / error / loading / progress | **Empty / Alert / Spinner or Skeleton / Progress** | Tag/Badge, Shimmer only for appropriate temporary text | Real status, validation and warnings stay visible |
| Local confirmation / anchored form / help / rich hover | **AlertDialog or Dialog / Popover / Tooltip / HoverCard** | Sheet for an in-window edge panel | In-window overlays are not independent passive HWND replacements |
| Rich text and metadata | **TextView / DescriptionList** | Label, Link, Image, Icon, Tag | Preserve information and safe rendering/selection policy |
| Workspace split / docked panels | **Resizable / Dock** | Tab panels and panel traits | Do not introduce resize/docking behavior without product authorization |

## Complete official Component directory (78 entries)

Each row gives the official documentation link, published entry point and purpose.
The source entry is verified as a file/public declaration or re-export; compatibility
with an NxTools surface must still be tested before adoption. Named compound parts
are included so that discovery does not stop at Button/Input/Switch.

### Settings, forms and semantic grouping (5)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Settings](https://gpui-kit.com/zh-CN/component/settings) | `C:setting/settings.rs`, `C:setting/page.rs`, `C:setting/group.rs`, `C:setting/item.rs`, `C:setting/fields/mod.rs`; `setting::{Settings, SettingPage, SettingGroup, SettingItem, SettingField}` | Full settings hierarchy; typed Switch/Checkbox/Input/dropdown/number fields and custom render/element slots; see detailed selection below |
| [Form](https://gpui-kit.com/zh-CN/component/form) | `C:form/form.rs`, `C:form/field.rs`; `form::{Form, Field, field, h_form, v_form}` | Label/control layout, columns, spans, descriptions and footer; application retains values/validation/submit |
| [GroupBox](https://gpui-kit.com/zh-CN/component/group-box) | `C:group_box.rs`; `GroupBox`, `GroupBoxVariant`, `GroupBoxVariants` | Titled semantic group; Normal/Outline/Fill, content styling and outside-surface footer |
| [DescriptionList](https://gpui-kit.com/zh-CN/component/description-list) | `C:description_list.rs`; `DescriptionList`, `DescriptionItem`, `DescriptionText` | Read-only key-value information, columns/spans and custom values; not an editing form |
| [Questionnaire](https://gpui-kit.com/zh-CN/component/questionnaire) | `C:questionnaire/components.rs`, `C:questionnaire/mod.rs`; `QuestionnaireState` and title/choice/input/error/progress/action parts | Ordered questions/answers and navigation; do not introduce a new wizard into an existing save workflow |

### Inputs and selection (15)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Input](https://gpui-kit.com/zh-CN/component/input) | `C:input/input.rs`, `C:input/state.rs`, `C:input/mod.rs`; `input::{Input, InputState, InputEvent}` | Single-line styled editing; prefix/suffix, validation and original stable state entity |
| [Input Group](https://gpui-kit.com/zh-CN/component/input-group) | `C:input/group.rs`; `InputGroup`, `InputGroupAddon`, `InputGroupInput`, `InputGroupTextarea`, `InputGroupButton`, `InputGroupText`, alignment/control types | One compound frame around text/icon/button/toolbar addons; no competing manual border |
| [Textarea](https://gpui-kit.com/zh-CN/component/textarea) | `C:input/textarea.rs`, `C:input/mod.rs`; `Textarea`, `TextareaState` | Ordinary multiline text, row growth and internal scrolling |
| [Editor](https://gpui-kit.com/zh-CN/component/editor) | `C:input/editor.rs`, `C:input/mod.rs`; `Editor`, `EditorState`; Base input language rules | Source-code editing, syntax/search/indentation/decorations; not the default settings text field |
| [NumberInput](https://gpui-kit.com/zh-CN/component/number-input) | `C:input/number_input.rs`, `C:input/mod.rs`; `NumberInput`, `InputState`, `NumberInputEvent`, `NumberStep` | Numeric editing and increment/decrement; check range/step/intermediate/Blur policy before conversion |
| [OtpInput](https://gpui-kit.com/zh-CN/component/otp-input) | `C:input/otp_input.rs`, `C:input/mod.rs`; `OtpInput`, `OtpState` | Segmented OTP/PIN editing; no new authentication feature implied |
| [Select](https://gpui-kit.com/zh-CN/component/select) | `C:select.rs`; `Select`, `SelectState`, `SelectEvent`, delegate/item exports | Simple single selection with optional search, grouping, clear and confirm/cancel |
| [Combobox](https://gpui-kit.com/zh-CN/component/combobox) | `C:combobox.rs`; `Combobox`, `ComboboxState`, `ComboboxEvent`, `ComboboxTriggerContext`; SearchableList | Searchable single/multi selection and custom trigger/item; exact release type is TriggerContext |
| [Switch](https://gpui-kit.com/zh-CN/component/switch) | `C:switch.rs`; `switch::Switch` | Boolean setting; controlled `on_change`, accessible label, disabled/focus and official tiers |
| [Checkbox](https://gpui-kit.com/zh-CN/component/checkbox) | `C:checkbox.rs`; `checkbox::Checkbox` | Independent selection flags; requested checked value, no nested duplicate row dispatcher |
| [Radio](https://gpui-kit.com/zh-CN/component/radio) | `C:radio.rs`; `radio::{Radio, RadioGroup}` | Exclusive choice; prefer group semantics rather than painted circles |
| [Toggle](https://gpui-kit.com/zh-CN/component/toggle) | `C:button/toggle.rs`; `button::{Toggle, ToggleGroup}` | Pressed button selection for toolbars/filters; distinct from a settings Switch |
| [Slider](https://gpui-kit.com/zh-CN/component/slider) | `C:slider.rs`; Base slider state/event re-exports | Bounded single/range numeric selection; application retains value/units |
| [ColorPicker](https://gpui-kit.com/zh-CN/component/color-picker) | `C:color_picker.rs`; `ColorPicker`, `ColorSelect`, Base color state/event | Color panel or field trigger; do not alter approved product palette without authorization |
| [Rating](https://gpui-kit.com/zh-CN/component/rating) | `C:rating.rs`; `Rating` | Star rating value, read-only/interactive; no current product workflow implied |

### Time and date (3)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Calendar](https://gpui-kit.com/zh-CN/component/calendar) | `C:time/calendar.rs`; `calendar::Calendar` and Base state/date/matcher exports | Calendar view, single/range selection, disabled dates and navigation |
| [DatePicker](https://gpui-kit.com/zh-CN/component/date-picker) | `C:time/date_picker.rs`; `DatePicker`, `DatePickerState`, `DatePickerEvent`, date/time/preset types | Date/range popup; optional time precision; preserve schedule confirmation/time zone semantics |
| [TimeField](https://gpui-kit.com/zh-CN/component/time-field) | `C:time/time_field.rs`; `TimeField`, Base state/event/hour-cycle/precision exports | Segmented time-of-day input; not a duration or arbitrary timestamp replacement |

### Actions, navigation and workspace (13)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Button](https://gpui-kit.com/zh-CN/component/button) | `C:button/button.rs`, `C:button/button_group.rs`, `C:button/button_icon.rs`; Button/variants/Group/Icon | Styled actions, official tiers, loading/icon/accessibility; one callback/focus owner |
| [DropdownButton](https://gpui-kit.com/zh-CN/component/dropdown_button) | `C:button/dropdown_button.rs`; `button::DropdownButton` | Split primary action and dropdown trigger; inner Button retains action-specific options |
| [Sidebar](https://gpui-kit.com/zh-CN/component/sidebar) | `C:sidebar/mod.rs`, `C:sidebar/menu.rs`, header/footer/group files | Sidebar/Header/Footer/Group/Menu/MenuItem/ToggleButton; inspect public focus seam before integration |
| [Tabs](https://gpui-kit.com/zh-CN/component/tabs) | `C:tab/tab.rs`, `C:tab/tab_bar.rs`; `tab::{Tab, TabBar}` | Partition selection; underline/pill/segmented and prefix/suffix/menu; preserve reorder/wheel where required |
| [Menu](https://gpui-kit.com/zh-CN/component/menu) | `C:menu/popup_menu.rs`, context/dropdown/app-menu modules | PopupMenu/PopupMenuItem/ContextMenuExt/DropdownMenu/AppMenuBar; one menu session and retained narrow-menu exception |
| [Command](https://gpui-kit.com/zh-CN/component/command) | `C:command/command.rs`, `C:command/state.rs`, `C:command/item.rs` | Command/State/Entry/Item/Group; search/selection/focus/loading controller must match existing session before replacement |
| [Toolbar](https://gpui-kit.com/zh-CN/component/toolbar) | `C:toolbar.rs`; `Toolbar`, `ToolbarGroup` | Transparent action composition; original outer surface owns background; inspect before hand-built action divs |
| [StatusBar](https://gpui-kit.com/zh-CN/component/status-bar) | `C:status_bar.rs`; `StatusBar` | Left/center/right information and short actions; use existing lifecycle/status source |
| [Stepper](https://gpui-kit.com/zh-CN/component/stepper) | `C:stepper/mod.rs`, `C:stepper/item.rs`; `Stepper`, `StepperItem` | Workflow-stage display; **not** the plus/minus numeric NumberInput |
| [Accordion](https://gpui-kit.com/zh-CN/component/accordion) | `C:accordion.rs`; `Accordion`, `AccordionItem` | Expandable sections, single/multiple and disabled states; no unsolicited collapsed settings |
| [Collapsible](https://gpui-kit.com/zh-CN/component/collapsible) | `C:collapsible.rs`; `Collapsible` | Simple open/content composition, distinct from a whole accordion collection |
| [Dock](https://gpui-kit.com/zh-CN/component/dock) | `C:dock/mod.rs`, `C:dock/panel.rs`; `DockSkin`, panel traits and Base DockArea re-exports | Styled docked/tabbed/split workspace; preserve existing fixed windows unless docking is requested |
| [Resizable](https://gpui-kit.com/zh-CN/component/resizable) | `C:resizable.rs`; `h_resizable`, `v_resizable`, panel/state re-exports | Split panels/handles/constraints; not permission to change fixed product geometry |

### Lists, tables and viewport rendering (6)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [List](https://gpui-kit.com/zh-CN/component/list) | `C:list/list.rs`, `C:list/delegate.rs`, `C:list/list_item.rs` | List/State/Delegate/Event plus ListItem/SeparatorItem; search/groups/loading/infinite scroll and custom rows |
| [VirtualList](https://gpui-kit.com/zh-CN/component/virtual-list) | `C:virtual_list.rs`; Base re-exported VirtualList/ScrollHandle/h/v functions | Low-level viewport rendering with variable sizes; keep existing data/generation/paging owner |
| [Table](https://gpui-kit.com/zh-CN/component/table) | `C:table/table.rs`; Table/Header/Body/Footer/Row/Head/Cell/Caption | Stateless small/static table; no sorting/virtualized data controller |
| [DataTable](https://gpui-kit.com/zh-CN/component/data-table) | `C:table/data_table.rs`, `C:table/state.rs`, `C:table/delegate.rs`, `C:table/column.rs` | DataTable/TableState/TableDelegate/Column; virtualized sorting/selection/custom cells/context; migration capability gate required |
| [Pagination](https://gpui-kit.com/zh-CN/component/pagination) | `C:pagination.rs`; `Pagination` | Explicit page controls; do not silently replace existing cursor/infinite paging |
| [Scrollable](https://gpui-kit.com/zh-CN/component/scrollable) | `C:scroll/scrollable.rs`, `C:scroll/mod.rs`; Scrollable/Element and Base Scrollbar re-exports | Scroll viewport/bar/handle composition; ensure one actual scroll owner and reachable footer |

### Overlays and window infrastructure (8)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Dialog](https://gpui-kit.com/zh-CN/component/dialog) | `C:dialog/dialog.rs`, content/header/title/description/footer modules | Root-hosted modal and DialogContent/Header/Title/Description/Footer/Action/Close; application retains draft/cancel/submit |
| [AlertDialog](https://gpui-kit.com/zh-CN/component/alert-dialog) | `C:dialog/alert_dialog.rs`; `dialog::AlertDialog` | Explicit confirmation/danger-response overlay; not a substitute for a separately owned HWND |
| [Popover](https://gpui-kit.com/zh-CN/component/popover) | `C:popover.rs`; `Popover` | Anchored click/local form; trigger and render closure; one overlay/session owner |
| [HoverCard](https://gpui-kit.com/zh-CN/component/hover-card) | `C:hover_card.rs`; `HoverCard` and Base state | Delayed rich hover content; does not replace passive Quick/Clipboard native preview lifecycle |
| [Tooltip](https://gpui-kit.com/zh-CN/component/tooltip) | `C:tooltip.rs`; `Tooltip`; Button/GPUI integration and Base overlay | Supplementary help; exact 0.7.1 default hover/long-press does not establish automatic focus reveal |
| [Sheet](https://gpui-kit.com/zh-CN/component/sheet) | `C:sheet.rs`; `Sheet`, `SheetSettings`, WindowExt | Root-hosted edge drawer with optional overlay/resize; existing native feature windows remain independent |
| [Focus Trap](https://gpui-kit.com/zh-CN/component/focus-trap) | `B:focus_trap.rs`, `C:lib.rs`; `FocusTrapElement` re-export | Constrain Tab in a modal scope; Dialog/Sheet already handle this; avoid another focus owner |
| [Root View](https://gpui-kit.com/zh-CN/component/root) | `B:root.rs`, `C:root.rs`, `C:lib.rs`; `Root`, Kit `open_window` | One Root per window, overlay/focus/text-selection host; Kit open_window wraps content automatically |

### Feedback, status and loading (9)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Alert](https://gpui-kit.com/zh-CN/component/alert) | `C:alert.rs`; `Alert`, `AlertVariant` | Inline important info/success/warning/error, title/icon/close/banner |
| [Empty](https://gpui-kit.com/zh-CN/component/empty) | `C:empty.rs`; Empty/Header/Media/Title/Description/Content | Structured no-results/first-use state with original action callbacks |
| [Notification](https://gpui-kit.com/zh-CN/component/notification) | `C:notification.rs`; Notification/Type/Delivery/Settings/List; WindowExt | Root-hosted toast system; do not add a second manager beside existing bounded notifications |
| [Progress](https://gpui-kit.com/zh-CN/component/progress) | `C:progress/progress.rs`, `C:progress/progress_circle.rs` | Progress/ProgressCircle, determinate/indeterminate; domain progress source remains original |
| [Spinner](https://gpui-kit.com/zh-CN/component/spinner) | `C:spinner.rs`; `Spinner` | Busy indicator; honor existing reduced-motion policy and no idle animation work |
| [Skeleton](https://gpui-kit.com/zh-CN/component/skeleton) | `C:skeleton.rs`; `Skeleton` | Loading placeholders with stable bounds; not real content/error suppression |
| [Shimmer](https://gpui-kit.com/zh-CN/component/shimmer) | `C:shimmer.rs`; `ShimmerText`, `ShimmerStyle`, `ShimmerSpread` | Temporary loading-text highlight; application owns state, honor reduced motion |
| [Badge](https://gpui-kit.com/zh-CN/component/badge) | `C:badge.rs`; `Badge` | Count/dot/icon indicator over another element; not a general metadata table |
| [Tag](https://gpui-kit.com/zh-CN/component/tag) | `C:tag.rs`; `Tag`, `TagVariant` | Compact classification/priority/status label, semantic colors and variants |

### Text, media and visual foundations (8)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Label](https://gpui-kit.com/zh-CN/component/label) | `C:label.rs`; `Label`, `HighlightsMatch` | General text, secondary/highlight/mask; avoid restoring legacy forced control typography |
| [TextView](https://gpui-kit.com/zh-CN/component/text-view) | `C:text/mod.rs`, `C:text/compat.rs`, `B:text/mod.rs` | TextView/markdown/html and Base text infrastructure; rich display/selection/plugins, not editing |
| [Kbd](https://gpui-kit.com/zh-CN/component/kbd) | `C:kbd.rs`; `Kbd` | Platform-aware shortcut display and action bindings; **not** a hotkey recorder |
| [Icon](https://gpui-kit.com/zh-CN/component/icon) | `C:icon.rs`; root `Icon`, asset `IconName` | SVG path/bytes and icon composition; retain project asset precedence and accessible action names |
| [Image](https://gpui-kit.com/zh-CN/component/image) | `G:elements/img.rs`, `G:elements/svg.rs`; root `img`, `svg` | GPUI rendering functions, not a separate Component Image struct; retain image/cache lifetime |
| [Avatar](https://gpui-kit.com/zh-CN/component/avatar) | `C:avatar/avatar.rs`, `C:avatar/avatar_group.rs` | Avatar/Group, image/initials/fallback; no current workflow implied |
| [Carousel](https://gpui-kit.com/zh-CN/component/carousel) | `C:carousel/carousel.rs`, `C:carousel/state.rs` | Carousel/State/Content/Item/Previous/Next/Pagination, controlled selected item |
| [Theme](https://gpui-kit.com/zh-CN/component/theme) | `C:theme/mod.rs`, `C:theme/registry.rs`, `C:sizing.rs`, `C:styled.rs` | Theme/ActiveTheme/registry/tokens/Sizable; preserve released grammar with approved semantic colors only |

### Conversation, visualization and platform-dependent features (11)

| Official entry | Published source / public API | Purpose and composition seam |
| --- | --- | --- |
| [Attachment](https://gpui-kit.com/zh-CN/component/attachment) | `C:attachment.rs`; Attachment/Media/Content/Title/Description/Actions/Group/Status | File/media presentation and action slots, not upload/download storage ownership |
| [Bubble](https://gpui-kit.com/zh-CN/component/bubble) | `C:bubble.rs`; Bubble/Content/Group/Reactions | Conversation content surface/alignment/reactions; not a generic settings card |
| [Message](https://gpui-kit.com/zh-CN/component/message) | `C:message.rs`; Message/Group/Avatar/Header/Content/Footer | Message-row slots; data/sender/time/delivery/action remain application-owned |
| [Marker](https://gpui-kit.com/zh-CN/component/marker) | `C:marker.rs`; Marker/Icon/Content plus alignment/loading/variant | Full-width timeline/system/status row; inspect before inventing another status wrapper |
| [MessageScroller](https://gpui-kit.com/zh-CN/component/message-scroller) | `C:message_scroller.rs`; MessageScroller/State | Variable rows and append/prepend/tail following; not an automatic Clipboard history-controller replacement |
| [Chart](https://gpui-kit.com/zh-CN/component/chart) | `C:chart/mod.rs` and chart files | Line/Bar/Area/Pie/Radar/Candlestick/Sankey charts; reuse current data, no new telemetry feature |
| [Plot](https://gpui-kit.com/zh-CN/component/plot) | `C:plot/mod.rs`, `C:plot/tooltip.rs`, `B:plot/mod.rs` | Base scales/shapes/axes plus styled plot Tooltip/CrossLine/Dot; distinct from UI Tooltip |
| [Speech](https://gpui-kit.com/zh-CN/component/speech) | `C:speech/mod.rs`, state/button/waveform/recognizer modules | Experimental audio/recognizer integration; platform/feature capability not approved by component migration |
| [TitleBar](https://gpui-kit.com/zh-CN/component/title-bar) | `C:title_bar.rs`; root `TitleBar` | Platform caption presentation/window options; existing C# native hit/command owner remains authoritative |
| [Clipboard](https://gpui-kit.com/zh-CN/component/clipboard) | `C:clipboard.rs`; `clipboard::Clipboard` | Copy action button/feedback, **not** capture/history/paste service; product clipboard Windows boundary remains C# |
| [Tree](https://gpui-kit.com/zh-CN/component/tree) | `C:tree.rs`; Tree/tree and Base TreeState/Item/Entry | Hierarchical selection/expansion/custom rows; data and lazy loading remain application-owned |

## Published Component entries outside the 78-page navigation

Do not infer absence from a missing sidebar page. These are additional public
module/helper entries found in release `C:lib.rs`; inspect them when applicable.

| Release entry | Source | Discovery use |
| --- | --- | --- |
| `breadcrumb::{Breadcrumb, BreadcrumbItem}` | `C:breadcrumb.rs` | Hierarchical navigation trail; not automatically a new product path control |
| `link::Link` | `C:link.rs` | Accessible actionable link presentation |
| `separator::{Separator, SeparatorStyle}` | `C:separator.rs` | Semantic divider; distinguish from Marker or a status row |
| `searchable_list` | `C:searchable_list.rs`, delegate/state/vec/item/change files | Shared searchable item/delegate/group/state support for Select/Combobox |
| `native_menu::NativeMenu` | `C:native_menu/mod.rs` | Platform-native menu boundary; evaluate against C# ownership before adopting |
| `history` | `C:history.rs` | Base history compatibility re-exports; navigation history is not undo history |
| `highlighter` | `C:highlighter/mod.rs` | Editor/text syntax infrastructure; feature/platform and language registry constraints |
| `global_state::GlobalState` | `C:global_state.rs` | Shared Kit/Base state plumbing, not application domain state |
| `WindowExt` | `C:window_ext.rs` | Official Dialog/Sheet/Notification/window presentation integration |
| `WindowBorder`, `window_border`, `window_paddings` | `C:window_border.rs` | Window surface/decorations; does not override approved native clients |
| `Sizable`, `Size`, `StyleSized`, `StyledExt`, `ThemeStyled` | `C:sizing.rs`, `C:styled.rs`, `C:lib.rs` | Official tiers/composition extensions, not blanket legacy style overrides |
| `IndexPath` | `C:index_path.rs` | Group/item identity for delegates |
| Inspector, touch-selection, root/time helpers | `C:lib.rs`, `C:inspector.rs`, `C:touch_selection/mod.rs`, `C:time/mod.rs` | Debug-only/internal/re-export boundaries; read visibility/cfg before importing |

## Base directory (47 official navigation entries)

Base supplies behavior/controlled state/focus/semantics and intentionally leaves
visual presentation to the consumer. **It is not the default escape hatch for
ordinary controls under the approved styled-component migration.** Choose a styled
Component first; use Base only for a documented capability/native boundary or a
truly custom content surface. All source paths below are under B unless stated.

| Official entry | Published entry / purpose |
| --- | --- |
| [Getting Started](https://gpui-kit.com/zh-CN/base/getting-started) | `B:lib.rs`, `B:root.rs`; init/Root and Base-vs-Component boundary |
| [Text Selection](https://gpui-kit.com/zh-CN/base/text-selection) | `B:text_selection.rs`; window-level selection handles/scopes/projection |
| [Motion](https://gpui-kit.com/zh-CN/base/motion) | `B:motion.rs`, `B:animation.rs`; transitions/springs/presence, reduced motion integration |
| [TextView](https://gpui-kit.com/zh-CN/base/text-view) | `B:text/mod.rs`; rich text/render/selection/plugins |
| [VirtualList](https://gpui-kit.com/zh-CN/base/virtual-list) | `B:virtual_list.rs`; low-level variable-size viewport |
| [Dock](https://gpui-kit.com/zh-CN/base/dock) | `B:dock/mod.rs`; docking model/layout/drag behaviors; styled DockSkin is in Component |
| [History](https://gpui-kit.com/zh-CN/base/history) | `B:history.rs`, `B:undo_history.rs`; navigation branches vs grouped undo transactions |
| [Plot](https://gpui-kit.com/zh-CN/base/plot) | `B:plot/mod.rs`; scales/shapes/axes/hover primitives |
| [Accordion](https://gpui-kit.com/zh-CN/base/primitives/accordion) | `B:accordion.rs`; root/item/header/trigger/panel parts |
| [Alert Dialog](https://gpui-kit.com/zh-CN/base/primitives/alert-dialog) | `B:alert_dialog.rs`; confirmation popup/backdrop/title/action/cancel parts |
| [Avatar](https://gpui-kit.com/zh-CN/base/primitives/avatar) | `B:avatar.rs`; image/fallback behavior |
| [Button](https://gpui-kit.com/zh-CN/base/primitives/button) | `B:button.rs`; interaction/disabled/focus/semantics, not styled appearance |
| [Calendar](https://gpui-kit.com/zh-CN/base/primitives/calendar) | `B:calendar.rs`; calendar state/date/matcher/navigation |
| [Checkbox](https://gpui-kit.com/zh-CN/base/primitives/checkbox) | `B:checkbox.rs`; checked/indeterminate and indicator parts |
| [Collapsible](https://gpui-kit.com/zh-CN/base/primitives/collapsible) | `B:collapsible.rs`; open/content behavior |
| [Color Picker](https://gpui-kit.com/zh-CN/base/primitives/color-picker) | `B:color_picker.rs`; color state/swatches/sliders |
| [Combobox](https://gpui-kit.com/zh-CN/base/primitives/combobox) | `B:combobox.rs`; popup selection behavior |
| [Date Picker](https://gpui-kit.com/zh-CN/base/primitives/date-picker) | `B:date_picker.rs`; date popup primitive |
| [Dialog](https://gpui-kit.com/zh-CN/base/primitives/dialog) | `B:dialog.rs`; handle/reasons/popup/trigger/backdrop/title/close parts |
| [Editor](https://gpui-kit.com/zh-CN/base/primitives/editor) | `B:input/mod.rs`; editor/state behavior without styled surface |
| [Hover Card](https://gpui-kit.com/zh-CN/base/primitives/hover-card) | `B:hover_card.rs`; hover open/close timing/state |
| [Input](https://gpui-kit.com/zh-CN/base/primitives/input) | `B:input/mod.rs`; Input/InputBase state/events/semantics |
| [Link](https://gpui-kit.com/zh-CN/base/primitives/link) | `B:link.rs`; interactive link primitive |
| [Nav Stack](https://gpui-kit.com/zh-CN/base/primitives/nav-stack) | `B:nav_stack.rs`; navigation pages/operations/motion/state |
| [Number Input](https://gpui-kit.com/zh-CN/base/primitives/number-input) | `B:number_input.rs`; numeric step/actions and part behavior |
| [OTP Input](https://gpui-kit.com/zh-CN/base/primitives/otp-input) | `B:otp_input.rs`; segmented OTP state/events |
| [Pagination](https://gpui-kit.com/zh-CN/base/primitives/pagination) | `B:pagination.rs`; page state/items |
| [Popover](https://gpui-kit.com/zh-CN/base/primitives/popover) | `B:popover.rs`; anchored popup state |
| [Popup](https://gpui-kit.com/zh-CN/base/primitives/popup) | `B:popup.rs`, `B:positioner.rs`; general overlay placement/lifetime |
| [Progress](https://gpui-kit.com/zh-CN/base/primitives/progress) | `B:progress.rs`; track/indicator primitives |
| [Radio](https://gpui-kit.com/zh-CN/base/primitives/radio) | `B:radio.rs`; single-radio semantics |
| [Radio Group](https://gpui-kit.com/zh-CN/base/primitives/radio-group) | `B:radio_group.rs`; exclusive group interaction |
| [Resizable](https://gpui-kit.com/zh-CN/base/primitives/resizable) | `B:resizable/mod.rs`; panel model/handles/constraints |
| [Scrollbar](https://gpui-kit.com/zh-CN/base/primitives/scrollbar) | `B:scrollbar.rs`; axes/handles/track/thumb/motion/styles |
| [Select](https://gpui-kit.com/zh-CN/base/primitives/select) | `B:select.rs`; single-choice popup primitive |
| [Sheet](https://gpui-kit.com/zh-CN/base/primitives/sheet) | `B:sheet.rs`; edge-panel behavior |
| [Slider](https://gpui-kit.com/zh-CN/base/primitives/slider) | `B:slider.rs`; thumb/track/indicator/value behavior |
| [Switch](https://gpui-kit.com/zh-CN/base/primitives/switch) | `B:switch.rs`; checked root/track/thumb semantics |
| [Table](https://gpui-kit.com/zh-CN/base/primitives/table) | `B:table.rs`; table semantic parts |
| [Tabs](https://gpui-kit.com/zh-CN/base/primitives/tabs) | `B:tabs.rs`; selected tab/group interaction |
| [Textarea](https://gpui-kit.com/zh-CN/base/primitives/textarea) | `B:input/mod.rs`; multiline editing state/behavior |
| [Time Field](https://gpui-kit.com/zh-CN/base/primitives/time-field) | `B:time_field.rs`; segmented time state/parts |
| [Toast](https://gpui-kit.com/zh-CN/base/primitives/toast) | `B:toast.rs`; manager/stack/timing/motion primitives |
| [Toggle](https://gpui-kit.com/zh-CN/base/primitives/toggle) | `B:toggle.rs`; pressed-button semantics |
| [Toggle Group](https://gpui-kit.com/zh-CN/base/primitives/toggle-group) | `B:toggle_group.rs`; selection group semantics |
| [Tooltip](https://gpui-kit.com/zh-CN/base/primitives/tooltip) | `B:tooltip.rs`; request/overlay/positioner/transition infrastructure |
| [Tree](https://gpui-kit.com/zh-CN/base/primitives/tree) | `B:tree.rs`; state/entries/items/keyboard behavior |

Additional Base exports not independently listed in the official sidebar include
Root/RootPlugin, Toolbar/ToolbarGroup, Questionnaire, FocusTrapElement, theme tokens,
measurements, AutoScroll, touch selection and test-support. `B:lib.rs` is the public
export authority; do not treat every internal source declaration as a public API.

## Shell documentation boundary (15 official entries)

Shell is a preview JavaScript/QuickJS extension/runtime layer built on Base, not a
Rust styled-settings alternative. Its documentation navigation was indexed; its
runtime/source was **not** reviewed or added to this project. No dependency,
plugin engine or controller transplant is authorized by this catalog.

| Official entry | Navigation purpose |
| --- | --- |
| [Introduction](https://gpui-kit.com/zh-CN/shell) | Runtime/extension layer scope |
| [Getting Started](https://gpui-kit.com/zh-CN/shell/getting-started) | Shell application setup |
| [Examples](https://gpui-kit.com/zh-CN/shell/examples) | Script examples |
| [Elements](https://gpui-kit.com/zh-CN/shell/elements) | Script element surface |
| [Styling](https://gpui-kit.com/zh-CN/shell/styling) | Script-side styling |
| [State and Views](https://gpui-kit.com/zh-CN/shell/state) | Script view/state lifecycle |
| [Overlays](https://gpui-kit.com/zh-CN/shell/overlays) | Overlay integration |
| [Capabilities](https://gpui-kit.com/zh-CN/shell/capabilities) | Host-granted system capabilities |
| [Dependencies](https://gpui-kit.com/zh-CN/shell/dependencies) | Script dependency boundary |
| [API Reference](https://gpui-kit.com/zh-CN/shell/api) | Runtime API navigation |
| [Hosting](https://gpui-kit.com/zh-CN/shell/hosting) | Rust host integration |
| [HostModule](https://gpui-kit.com/zh-CN/shell/host-module) | Host module interface |
| [Dock and Panels](https://gpui-kit.com/zh-CN/shell/dock) | Script dock/panel seam |
| [Performance](https://gpui-kit.com/zh-CN/shell/performance) | Reference performance discussion, not rerun evidence |
| [The Engine Seam](https://gpui-kit.com/zh-CN/shell/engine) | Engine/runtime separation |

## Compound-component details that must not be overlooked

### Settings is the first candidate for settings pages

Verified release call chain:
`Settings::render` -> internal keyed SettingsState/search/filter/sidebar ->
active `SettingPage::render` -> group list -> `SettingGroup::render` ->
`SettingItem` -> typed field renderer or custom field renderer -> styled control.
Read `C:setting/settings.rs`, `page.rs`, `group.rs`, `item.rs`,
`fields/{mod,bool,string,dropdown,number,element}.rs` together before adapting.

- Top-level Settings exposes `.pages`, `.sidebar_width`, `.sidebar_size_range`, `.sidebar_style`, `.header_style`, `.default_selected_index`, `.with_group_variant` and official size tiers.
- Default sidebar is 250px, range 160..360; current product settings rail is a separate existing geometry contract. The release constructs a search Input and h_resizable internally; there is no dedicated public `searchable(false)` / `resizable(false)` builder in the inspected Settings API. A compatibility question is not an automatic rejection: measure the actual public composition and obtain approval for genuine behavior changes.
- Selected page/search state is internally keyed; `default_selected_index` is initialization, not a controlled selection setter. No public selection-change callback was found in this inspected API. Preserve current page restoration/unmount behavior through a tested integration, not a guessed setter.
- SettingPage supports title/icon/description/title_suffix/groups/header_style and `.resettable(false)`. Default resettable is true; do not add product reset behavior inadvertently.
- SettingGroup supports title/description/items, per-group `.variant(GroupBoxVariant::Normal)` and a footer outside the surface. There is no need to preserve a hand-painted group merely to avoid cards.
- SettingItem::new provides ordinary title/description/field and horizontal/vertical layouts; SettingItem::render is a full custom-content slot. Custom content does not mean custom ordinary controls or copying the whole old page into one slot and declaring adoption complete.
- SettingField::switch/checkbox/input/dropdown/scrollable_dropdown/number_input use typed getters/setters. SettingField::render/element and SettingFieldElement can preserve original hotkey recorders, picker compositions, InputState entities, diagnostics and domain editors.
- Getters cannot synchronously re-lease the currently rendered owner; project adapters use immutable projection/local state. Setters preserve original save policy and deferred cross-owner updates. The built-in typed string field is not automatically suitable for an explicit-save editor with existing InputState entities.
- Optional-description Tooltip behavior is an approved product requirement. Inspect title_suffix/field/custom-item composition; do not silently restore permanent descriptions or reject the whole Settings component because a standard label lacks a suffix slot.
- SettingPage::render is `pub(super)` and SettingGroup::render is `pub(crate)` in this release. They are **not standalone public RenderOnce components**. Do not promise direct `.child(SettingPage)` composition with a custom Sidebar without a verified public API.

### Form, Table/DataTable, List/VirtualList and Command

- Form/Field is for draft/decision layout; Settings is for settings-page hierarchy. Prefer the relevant compound rather than merely replacing child Input/Switch.
- Table is static semantic composition; DataTable adds delegate/state/virtualization/selection/sorting. Read both before choosing a business grid exception.
- ListItem can adopt row presentation without replacing an existing controller. List and VirtualList are not interchangeable; inspect state/delegate/viewport ownership and variable-height behavior.
- Command is a real command palette with its own search/focus/selection/loading state. Inspect it before hand-building command UI, but do not substitute it for Quick Find's engine/generation/action/native session without matching behavior.
- Dialog/Sheet/Popover/HoverCard are window-internal presentations. Neither familiarity nor their existence justifies changing native passive-preview lifecycle.

## Website-to-release discrepancies already verified

These are evidence-level warnings, not reasons to ignore the library.

| Website observation | Exact release finding | Required action |
| --- | --- | --- |
| Combobox import example includes `ComboboxTriggerCtx` | `C:combobox.rs` declares `ComboboxTriggerContext` | Use published spelling and compile the chosen consumer recipe |
| VirtualList example refers to `ScrollbarState` | `C:scroll/mod.rs` exports Base `ScrollbarHandle` and related types, not that listed State | Follow release scroll API; do not invent a compatibility type |
| Tooltip introduction mentions focus-triggered display | Previously traced default0.7.1 Tooltip integration is hover/long-press, not automatic keyboard-focus reveal | Retain accessible names/Tab but test actual display behavior; no new focus popup controller |
| Settings examples access options fields directly | `C:setting/settings.rs::RenderOptions` fields are private; public methods include `size()` and `is_disabled()` | Use published accessors |
| Sidebar snippet can omit the ID | Published `Sidebar::new` requires an ID | Follow locked signature, not an incomplete snippet |
| Root page includes package-level initialization examples | Project uses Kit facade init/open_window and one Base Root | Follow [release boundary](gpui-kit-071.md), no new direct GPUI-stack dependency |

## Current NxTools adoption is separate from availability

Authoritative surface/evidence ledger: [adoption-ledger.md](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/adoption-ledger.md).
This table is a navigation snapshot, not a fresh whole-product adoption audit.

Latest user-authorized [post-gallery amendment](../audits/gpui-kit-ui-feedback.md)
supersedes matching historical row decisions below: realTimeField/ListItem/Clipboard
Tab/Small24pxsearch adopted,Calendarpadding/tier measured;625tests/strict/buildpass.
Browserrestored:30newPNG actuallyviewed afterlocalcontrol-lane/proxy/schedulecorrections;
user/fullstate/physical/DPI/readerinstalled acceptance remainsopen.

| Surface | Already evidenced | Explicitly still incomplete / next selection review |
| --- | --- | --- |
| Quick Find launcher | Actual InputGroup + addon, original input/controller and user-approved search effect | Do not reopen approved search UI just to increase component usage |
| Quick Find settings | Actual released Settings/5Page/Group/Item/Field and command Form; queue01 attachedInputGroup/statelessListItem/DescriptionList,queue10styledrecorder,615-test frozen11/strict/build evidence | Original entities/action/native owners retained; named40pxdelete adaptation. No wholecontent/visual/physical/DPI/reader/installed acceptance; approvedlauncher untouched |
| Representative Tools compact settings | Actual Form/Field, NumberInput/Select/Switch and Tooltip under approved native budget | Not evidence that all settings surfaces use official compound components |
| Downloads | Settings/Form/Textarea(queue02); main InputGroup/Toolbar/Tabs/Progress/DescriptionList(queue03); new-download Form/Radio/styledPopover/ListItem(queue04); supporting controls/DialogContentFooter/Alert(queue05+10);frozen11 615tests/strict/Debugbuild | Grid/date-grid/TimeField narrowdecisions remain. Categorymaster compact_list_row still unimplemented adoption,not testedAPIexception. Nativecaption/content budgets retained,finalvisual/native pending |
| Clipboard + Tools | Settings(queue06); search/editor compounds(queue07); statelesshistoryListItem/Toolbar/Buttons/Label/Empty/Alert,ToolsListItemTag/LinkStatusBar(queue08);sharedstates10/frozen11checks | Originalentities/worker/cache/query/action/native/projectmenu owners preserved. Categorynav/adddrop07decision remains;finalvisual/native12pending,not wholefamilyadoption |
| Global settings/supporting surfaces | GlobalSettings Settings/Form/Switch/Checkbox(queue09); shared Empty/Alert/loadingSpinner, notification Alert/Label/Button under originalmanager, taskstates/menu/recorder/close/back-to-top(queue10),615tests/strictchecks/11gates | Original native budgets/atomicSave/deferred/state owners retained. DirectNotification dismiss/completion adapter narrowly limited; alternatives unverified.03/04/05/07/10 decisions and finalvisual/native12 pending; not whole-stage or all-custom-content adoption |

Queue11 reconciles all rows without productsource changes. Exact constructors/publicAPI,
unimplementedcoverage, five deferredrecipes and acceptance limits: [draft final report](../audits/gpui-kit-ui-queue-final.md).
Passing615tests/strict/build and ownednative96DPI fixtures do not mean all78families migrated.

## Mandatory selection record for every UI slice

Before implementation record the following in the owning task/ledger:

| Required field | What to record |
| --- | --- |
| Surface and smallest behavior gap | Page/compound/row/leaf, existing owner and invariant |
| Official candidates | Compound first, then leaf alternatives; exact entries from this index |
| Evidence | Official URL plus published crate/version/file/symbol and reviewed callback/render chain |
| Decision | `adopt`, `adapt through public slots`, `compatibility test pending`, or `retain narrow exception` |
| Compatibility | Fixed geometry, data/controller, InputState identity/IME, one callback/focus owner, deferred writes and native lifecycle |
| Exception | Exact unsupported public capability, source location, real regression, rejected alternatives and user approval if behavior changes |
| Acceptance | Red-capable boundary regression; actual adoption/behavior/render/user/native evidence separately |

Do not decide "custom because familiar", "existing tests pass", "fixed size" or
"preserve behavior" without inspecting the compound API and testing the specific
conflict. Missing evidence means **unreviewed**, not unsupported. An import or a
single old page embedded in SettingItem::render is not page-level adoption.

## Maintaining and checking this index

1. Refresh official Component/Base/Shell sidebar navigation after an authorized version change; compare names and URLs, not only counts.
2. Match document families against exact published lib.rs public modules/re-exports and verify source entry files. Also scan public entries without sidebar pages.
3. Read purpose/composition/import sections; inspect detailed selected APIs before code changes. Record website/release discrepancies explicitly.
4. Update selection/adoption evidence in the task ledger, not by marking this availability catalog "all adopted".
5. Run `python tests/architecture/spec-contracts.py`, owning task validation and `git diff --check` for documentation changes. External catalog/source coverage checking is a separate check; spec-contracts does not validate those external APIs.

This index has no product behavior change, Cargo/build/package/install/service run,
benchmark execution, physical input/DPI/reader or full-component visual acceptance.
