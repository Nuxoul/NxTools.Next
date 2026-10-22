# GPUI Kit adoption reinspection — previously modified Desktop UI

## Later implementation update — 2026-10-11
The findings below remain the dated source audit, not the latest state of every
named boundary. The subsequent authorized Quick Find slice adopted actual released
Settings/5Page/Group/Item/Field with official built-in search/filter/sidebar/scroll,
actual command Form/Field and original entities/callbacks. Removed old Quick Find
navigation/row/group/scroll presentation, fixed exclusion-help Tooltip and mapped
official group_box token to approved neutral surface. R01/R02/R11/R12 are resolved
only for those named Quick Find boundaries, not all families or business contents.
Final549/549/strict checks/7gates pass; all5topic/minimum-scrolled captures actually
inspected. User/physical/DPI/reader/installed pending. See latest
[owning ledger](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/adoption-ledger.md).

## Outcome, baseline and limits

The user's concern is confirmed: several previously modified surfaces still use
custom page/group/field chrome, or actual Kit leaf controls restyled to old geometry.
The current working tree does **not** constitute complete styled-component adoption.
This report is a single-agent, source-only reinspection against the
[complete official index](../reference-code/gpui-kit-component-index.md) and the
[approved task](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/design.md).
No product code was changed, and no Cargo, product/preview launch, screenshot,
physical/DPI/reader/native/service/Release check was performed in this review.

Baseline: branch `refactor/ui-gpui-kit`, current dirty working tree, HEAD checkpoint
`42f60a8`. Read current render/helper callers, public release entries and prior
[layout report](2026-10-09-feature-first-ui-migration.md),
[remaining layout report](2026-10-09-remaining-ui-layout-migration.md),
[Foundation report](2026-10-10-gpui-kit-component-foundation.md) and
[adoption ledger](../../.trellis/tasks/10-10-gpui-kit-component-visual-migration/adoption-ledger.md).
Dirty files contain accumulated work, so this report does not attribute every hunk
to this session or equate every earlier layout change with a styled migration claim.

The October9 layout work improved presentation/behavior before the October10
component-first brief. Its completion was a layout-slice result, **not** proof of
actual Kit compound adoption. Later Foundation and narrow Quick Find controls are
real, but their tests do not certify the surrounding page or later families.

Evidence distinguishes:
- **Adopted**: actual styled constructor in the production path, compatible adapter.
- **Partial**: real Kit control but custom compound or unexplained old appearance overrides.
- **Not adopted**: corresponding official presentation absent in the reviewed path.
- **Compatibility review pending**: candidate exists; no proven unsupported capability.
- **Retained narrow boundary**: documented native/gesture/focus exception; not a whole-page exemption.

Line numbers below describe this source snapshot and may move. Named symbols and
linked files are the durable entry points. Source findings are not visual defects
or proof of a behavior regression unless explicitly stated.

## Surface coverage and corrected status

Paths in the table are relative to `rust/apps/nxtools-desktop/src/`.

| Reviewed surface | Current production status | Conclusion |
| --- | --- | --- |
| `theme.rs::apply_component_theme` | Actual Kit Theme::update, released font-size/radius retained; neutral color mapping | Adopted theme foundation, not evidence every consumer preserves its grammar |
| `ui/components/action.rs` ordinary buttons and segments | Actual styled Small Button and ButtonGroup | Adopted shared primitives; callers can still override them (R11) |
| `ui/components/field.rs::field_shell` | Actual Medium styled Input, appearance/border/focus explicitly re-enabled | Adopted single-line chrome; labels/Form and multiline remain partial (R02/R04) |
| `ui/components/settings.rs::settings_toggle` | Actual Small controlled Switch | Adopted leaf, not Settings hierarchy; general helper has no explicit announced label |
| `main_window/shell.rs::render_sidebar` | Actual Sidebar/SidebarGroup plus styled focused module Buttons | Adopted with documented focus-binding compatibility composition |
| `main_window/tools_page.rs::ToolSettingsWindowBody` | Actual Form/Field, NumberInput, Select, Switch and Tooltip | Real compound adoption; custom feature window is the approved native/template host |
| Tools workspace card | Base focused whole-row owner plus styled Configure Button | Partial row presentation; ListItem compatibility not proven, no blanket completion |
| Shared `description_help` | Actual XSmall ghost Info Button and Tooltip, complete accessible description | Adopted explanatory help; one omitted consumer remains (R12) |
| Quick Find launcher search | Actual InputGroup/InputGroupAddon/Input on original entity | Adopted and user-approved; do not alter its accepted appearance in catch-up work |
| Quick Find settings/general/context | Actual three Switches inside custom navigation/groups/rows | Leaf-only adoption; Settings hierarchy absent (R01) |
| Quick Find command editor | Five styled original Inputs; real but overridden buttons/Checkbox; custom field/layout | Partial (R02/R11), master/detail controller remains original |
| Quick Find diagnostics/volume/path | Actual some Button/Checkbox/Tooltip; custom metric/row/group presentation | Partial; DescriptionList/ListItem/Settings capability review pending |
| File-dialog search | Direct appearance-suppressed Input plus custom icon/search rail | Not styled search-combination adoption (R03); native attached-window owner remains |
| Download main | Actual ButtonGroup filters; custom search/grid/progress/detail tab header | Partial (R03/R08/R09), Table/DataTable controller compatibility pending |
| Download settings | Some actual styled fields/Switch/Tooltip; custom navigation/groups, manual header Textarea | Partial (R01/R04), not full Settings/Form adoption |
| New download | Styled fields through shared helper, actual Popover behavior with suppressed appearance/custom surface, hand-painted radio rows | Partial (R02/R05/R06); preserve native fixed-window handoff |
| Schedule | Actual some Button/Input and Tooltip; manual enable button/time frame | Partial, Switch/Toggle and TimeField review missing (R05) |
| Task progress window | Original native owner, styled actions, manual aggregate progress | Partial (R08); domain segment dots are a separate custom visualization |
| Clipboard search/edit/category/settings/history | Actual some controls/Tooltip; direct suppressed Inputs/Textarea, custom grouping and row/category presentation | Partial (R01/R03/R04); history/session/drag/IME ownership must survive |
| Clipboard project menu/passive preview | One documented project Base Root menu and independent passive preview HWND | Retained narrow boundaries; not permission to exempt ordinary content/controls |
| Tools Link panel | Actual Icon/Tooltip and overridden expander Button; custom interactive rows/footer | Partial; ListItem/Toolbar/StatusBar compatibility review pending |
| Global settings/blacklist | Some shared styled Input/Switch/ButtonGroup; custom navigation/groups/checkbox marks/actions | Partial (R01/R05/R11); atomic save/cancel/busy owner remains |
| Notification stack | Original bounded notification manager with custom card/text/marker presentation and restyled Button | Not official Notification/Alert presentation adoption (R10) |
| Download shutdown/delete view and shared native dialog shell | Custom native host/body/footer with actual actions | Host exception does not certify compound body adoption; delete view carries retained/dead-code annotation, not evidence of a reachable live dialog |

## Standards findings: 12 confirmed adoption/coverage gaps

These breach the component-first target or lack a completed compound decision;
not all are newly introduced by the latest slice. See the phase distinction below.

### R01 — Settings pages still use custom hierarchy (high adoption priority)
- Quick Find: [quick_launch_page.rs](../../rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs), render_quick_find lines926–1045/1078–1125 and quick_find_group line1333; custom navigation and shared workbench grouping.
- Clipboard: [clipboard_page.rs](../../rust/apps/nxtools-desktop/src/main_window/clipboard_page.rs), lines1031–1056, three workbench_settings_section consumers.
- Download: [settings_view.rs](../../rust/apps/nxtools-desktop/src/downloads/settings_view.rs), lines2310–2375/2989–3056, custom navigation/title/state palette.
- Global: [blacklist.rs](../../rust/apps/nxtools-desktop/src/network_settings/blacklist.rs), render_navigation lines81–145; [network_settings.rs](../../rust/apps/nxtools-desktop/src/network_settings.rs), render settings_section/settings_row composition.
- Shared [settings.rs](../../rust/apps/nxtools-desktop/src/ui/components/settings.rs), workbench_settings_section/row lines225–285 explicitly retain legacy grouping.
- Official candidate: Settings/SettingPage/SettingGroup/SettingItem/SettingField. No actual use of that hierarchy in the reviewed production paths. Exact0.7.1 Page/Group render methods are not standalone public RenderOnce components; evaluate top-level public slots/internal selection/search/resize rather than promising an unsupported custom-Sidebar composition.
- Verdict: **not adopted at page level**, not a supported whole-page exception.

### R02 — Editor labels and forms remain custom despite styled Inputs
- [field.rs](../../rust/apps/nxtools-desktop/src/ui/components/field.rs)::labeled_control lines60–97 and path_picker_shell lines116–131 are custom label/field composition, explicitly pending complete Form.
- [custom_command_editor.rs](../../rust/apps/nxtools-desktop/src/ui/components/custom_command_editor.rs)::command_input_field lines474–501 supplies styled Input but retains custom label row; detail body lines385–469 has no Form/Field.
- New-download and global/Clipboard editor consumers retain these/manual draft layouts.
- Official candidate: Form/Field, InputGroup for compatible addons, existing entities in public custom slots. Tools shows a real successful Form/Field adoption; that success cannot be generalized to these editors.
- Verdict: **leaf-only/partial**. Preserve explicit-save drafts, picker generations and original entity identities when evaluating.

### R03 — Three search surfaces still suppress styled Input chrome
- [download_page.rs](../../rust/apps/nxtools-desktop/src/main_window/download_page.rs), lines1010–1058: painted border/radius/search icon/clear assembly plus direct appearance(false)/bordered(false)/focus_bordered(false).
- [ui/mod.rs](../../rust/apps/nxtools-desktop/src/ui/mod.rs), lines2187–2232: Clipboard custom search frame/scope button plus suppressed direct Input.
- [file_dialog_search.rs](../../rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs), lines754–797: custom search icon/rail and direct suppressed Input.
- Official candidate: InputGroup/addons or Input prefix/suffix. The adopted Quick launcher InputGroup is evidence this equivalent exists, not an automatic identical geometry solution for these native budgets.
- Verdict: **not adopted search composition**, no verified public-capability exception.

### R04 — Three multiline/category fields retain hand-painted frames
- [field.rs](../../rust/apps/nxtools-desktop/src/ui/components/field.rs)::field_shell_with_height lines11–31 paints the border/radius/background; multiline_field_shell routes here.
- [settings_view.rs](../../rust/apps/nxtools-desktop/src/downloads/settings_view.rs), lines942–948: default headers Textarea suppresses appearance/border inside that manual frame.
- [ui/mod.rs](../../rust/apps/nxtools-desktop/src/ui/mod.rs), lines2605–2619: Clipboard edit Textarea suppressed inside custom frame; lines2722–2739: category Input suppressed inside custom frame.
- Official candidate: default styled Textarea/Input, Form/Field; preserve internal scroll and minimum fixed-client budgets.
- Verdict: **presentation not adopted**, despite actual Textarea API migration.

### R05 — Ordinary binary/exclusive controls still painted by the project
- [list.rs](../../rust/apps/nxtools-desktop/src/ui/components/list.rs)::radio_list_row lines144–200 draws ring/dot, invoked by new-download selection rows ([new_download_view.rs](../../rust/apps/nxtools-desktop/src/downloads/new_download_view.rs), lines2297/2524).
- [schedule_view.rs](../../rust/apps/nxtools-desktop/src/downloads/schedule_view.rs), lines381–432 draws clickable "开启/关闭" div and a manual time frame around suppressed Input.
- [blacklist.rs](../../rust/apps/nxtools-desktop/src/network_settings/blacklist.rs)::scope_action lines18–64 draws square/check Icon inside a Base row owner instead of actual Checkbox.
- Official candidate: Radio/RadioGroup, Switch or Toggle according to existing meaning, TimeField after HH:MM policy compatibility, Checkbox with one dispatcher.
- Existing full-row focus/single-action contracts must survive; they are not proof that styled controls cannot fit.
- Verdict: **not actual ordinary-control adoption**; full-row ownership needs a measured integration, not duplicated child callbacks.

### R06 — Official Popover behavior is used but its presentation is suppressed
- [new_download_view.rs](../../rust/apps/nxtools-desktop/src/downloads/new_download_view.rs), lines1226–1229 and1486: actual Popover appearance(false); content uses project popover_shell.
- [popover.rs](../../rust/apps/nxtools-desktop/src/ui/components/popover.rs), lines7–26 paints radius/border/background/shadow; category content lines1689–1715 composes custom list_box.
- Official candidate: styled Popover/content and Select/Combobox/List where compatible. Schedule and category drafts must keep existing commit/cancel callbacks.
- Verdict: **behavior API adoption, legacy surface presentation**, not default styled Popover adoption.

### R07 — Shared empty/error/loading surfaces are manual div compositions
- [state.rs](../../rust/apps/nxtools-desktop/src/ui/components/state.rs), empty_state/progress_state/inline_error_state route to manual work_state lines130–215; current loading icon is not Spinner and errors use custom edge.
- [field.rs](../../rust/apps/nxtools-desktop/src/ui/components/field.rs)::inline_error lines134–153 paints its own danger panel.
- Used by Quick launcher/file-dialog search, Clipboard and Download list/detail states.
- Official candidate: Empty named parts, Alert, Spinner/appropriate Progress. Actual progress_state is a loading message, so a percentage Progress substitution alone would be the wrong semantic choice.
- Verdict: **not adopted**. Review real public composition under existing action/focus/bounds before replacement.

### R08 — Aggregate download progress is manual
- [download_page.rs](../../rust/apps/nxtools-desktop/src/main_window/download_page.rs), lines1641–1657: manually sized track/fill.
- [task_window.rs](../../rust/apps/nxtools-desktop/src/downloads/task_window.rs), lines784–798: manual aggregate ratio track.
- Official candidate: Progress with value/loading/color/label; published progress/progress.rs provides these APIs. Domain segment dots/verification visualizations remain distinct from aggregate progress and are not automatically replaced.
- Verdict: **not official Progress presentation**; prior layout/terminal tests are not component-adoption evidence.

### R09 — Detail tabs and action/status rails lack compound adoption
- [download_page.rs](../../rust/apps/nxtools-desktop/src/main_window/download_page.rs), lines2194–2249: div plus restyled Button underline row instead of TabBar/Tab.
- Reviewed header/action/footer compositions in Download, Link panel and supporting surfaces are custom flex containers, with no Toolbar/StatusBar selection record.
- Official candidates: TabBar/Tab, Toolbar/ToolbarGroup/StatusBar. Pure flex containers are not inherently wrong; the missing compound evaluation means these cannot be declared migrated. Preserve five detail tabs and reset-scroll behavior.
- Verdict: detail Tabs **not adopted**; action/status rails **compatibility review pending**, not an invented unsupported-capability blocker.

### R10 — Notification manager preserved, but official presentation not evaluated/adopted
- [main_window/notifications.rs](../../rust/apps/nxtools-desktop/src/main_window/notifications.rs), render_notifications lines17–29/52–182/187–206 manually lays out stack/card/marker/text.
- Official candidates: Notification for compatible presentation, or Alert/Progress/Button under the existing manager if a specific manager/budget conflict is proven.
- Keep max3/width336/max112, generation dismissal, original timers and one action owner. Do not install a second toast manager.
- Verdict: **custom presentation**, original manager is a legitimate ownership constraint but not a blanket custom-card exception.

### R11 — Kit controls are still overridden to old visual grammar at consumers
- [quick_launch_page.rs](../../rust/apps/nxtools-desktop/src/main_window/quick_launch_page.rs), lines1099–1118: Button h_full/legacy radius/type/line-height/painted left selection border.
- [custom_command_editor.rs](../../rust/apps/nxtools-desktop/src/ui/components/custom_command_editor.rs), lines315–352/517–523: actual Button/Checkbox with forced32px height and13/20 text. Five newly styled Input frames are real; the surrounding controls are not default-tier adoption.
- [notifications.rs](../../rust/apps/nxtools-desktop/src/main_window/notifications.rs), lines151–162, and [link_panel.rs](../../rust/apps/nxtools-desktop/src/tools/link_panel.rs), lines309–321 retain height/radius/type overrides.
- Official tiers/variants and neutral palette are available. Legacy sizes may be a native-budget constraint, but the owning decision must compare real official tiers instead of forcing old constants by default.
- Verdict: **actual components, partial presentation adoption**, not absence of Kit. Shared control tests do not certify these overrides.

### R12 — Optional-description Tooltip migration missed the exclusion header
- [path_list.rs](../../rust/apps/nxtools-desktop/src/ui/components/path_list.rs), lines110–117 still renders "目录及其所有子项不会进入速启结果" as a permanent secondary explanation.
- This is optional feature-help copy, not actual path/state/error/risk data. Earlier user requirement explicitly moved such explanations to actual Kit Tooltip.
- Existing path Tooltip protects full path values but does not satisfy this header-help requirement.
- Verdict: **confirmed incomplete explanatory slice**, independently of Settings page migration. Preserve complete copy via official help trigger; no production fix in this review.

## Spec/completion accounting

- **Foundation shared ordinary controls**: actual adoption is retained, not withdrawn. Theme/Button/Input/Switch/ButtonGroup are genuine. Shared multiline, radio/list/state and legacy grouping were already documented as pending; they must not be silently counted as completed when consumers are touched.
- **Tools settings**: Form/Field/NumberInput/Select/Switch/Tooltip adopted in production; the approved feature template/native host remains custom by design. Tools workspace/Link rows are separate and still partial.
- **Quick Find**: search InputGroup and the three-switch/five-input slice are genuine; Settings hierarchy/editor form/navigation/diagnostics/path presentation are incomplete. User-approved launcher effect stays untouched; custom result/action rows are not a reason to reopen its pixels automatically.
- **Cross-family Tooltip**: official shared help is genuine, but R12 shows its coverage was not complete. Updating descriptions did not migrate all surrounding settings/pages.
- **Downloads/Clipboard/global/notifications**: earlier layout work is present; full styled-component phases are not complete. R01–R11 record real outstanding work, not proof of a newly failed behavior test.
- The old ledger's initial table still says planned while appendices record actual adoption. A dated current-status report/table must govern; neither old planned nor broad historical "implemented surfaces" wording alone is sufficient.

## Retained narrow boundaries and non-findings

1. **Main module sidebar focus adapter**: actual Sidebar+styled Buttons, not a fake Sidebar. Published SidebarMenuItem lacks the required existing FocusHandle/Tab binding. Production adapter seeds the public keyed focus seam; existing navigation geometry/release/focus/unmount tests exist. Retain unless a newly verified public API removes the need.
2. **Clipboard narrow project menu**: published PopupMenu failed the300px submenu boundary and private placement/flat-surface gap was recorded. Project menu owns one Root/session, clamps/scrolls/deferred callbacks/focus and has dedicated tests. This is an already approved narrow exception, not an excuse to custom-paint ordinary settings controls.
3. **Independent native windows/passive previews/OLE**: Rust state/layout, GPUI rendering and C# native HWND ownership remain. Dialog/HoverCard/Sheet cannot simply replace these controllers. Custom host shell does not exempt contained inputs/actions/forms.
4. **Hotkey recorder**: Kbd displays a shortcut but does not record/register it. Preserve recorder/native applied-vs-requested state; consider official field/content slots for surrounding presentation only.
5. **Business rows/controllers**: Quick result/action rows, Clipboard variable history/category gestures, Download multiselect/OLE grid, command master, volume/path/Link rows remain custom. ListItem/List/DataTable/Command are candidates, but this source review does not demonstrate drop-in compatibility. Their exceptions are **review pending**, not automatically approved merely because the controller is custom.
6. **Plain text previews**: plain literal text must not become Markdown/HTML just because TextView exists. DescriptionList/Label content can be evaluated separately from passive lifetime.
7. **field_shell false-positive**: download_settings_input, several new-download fields, global proxy/process input pass appearance(false) into shared field_shell; that helper subsequently forces appearance(true), bordered(true), focus_bordered(true). The actual final Input is styled. These redundant call-site flags are not proof of suppressed runtime chrome. Direct suppressed search/textarea/category/time inputs in R03/R04/R05 do not pass through that helper and remain findings.
8. **No runtime failure claim**: this review did not run tests or interact with the product. Existing545-test/96DPI evidence remains historical scoped evidence, not a new audit PASS or reader/native acceptance.

## Correction order recommendation (not implementation authorization)

Keep canonical user order **1. Foundation -> 3. Quick Find -> 2. Downloads ->
4. Clipboard + Tools -> 5. Global settings + supporting surfaces**. Do not renumber,
move later phases forward, or reopen accepted search visuals.

1. In current **3. Quick Find**, evaluate actual Settings public hierarchy/slots with original page/saves, then Form/Field for command editor, exact consumer tiers and R12 help coverage. Carry index/diagnostic/recorder/controller boundaries, fixed sizes and red-capable production tests.
2. When **2. Downloads** is authorized, address Settings/Form/search/multiline/Radio/Popover/schedule/Progress/Tabs/Toolbar/DataTable compatibility as a coherent surface migration, not another list of leaf swaps.
3. **4. Clipboard + Tools**: search/editor/category Form/Field/textarea/settings and row/action composition; retain narrow project menu/passive ownership.
4. **5. Global settings + supporting surfaces**: Settings/Form/navigation/real Checkbox and notification presentation under original atomic/manager owners.
5. Shared changes require scoped consumers/budgets before propagation; no generic full Form rewrite that repeats the earlier fixed-window regression. Record source/API, actual adoption, behavior, inspected rendering, user and native acceptance separately.

## Review verification

Read production sources and caller/helper chains cited above, exact0.7.1 public
entries for Settings/Form/Radio/ListItem/Progress/Empty/Sidebar, and historical
exceptions/test declarations. Grep scans were supporting discovery only, not the
basis for claiming behavior/absence across arbitrary uninspected code.
Only review documentation/ledger records are written. Run spec-contracts, local
report-link verification, owning task context validation and git diff --check.
No product correction, Cargo/preview/native operation, commit or push is performed.
