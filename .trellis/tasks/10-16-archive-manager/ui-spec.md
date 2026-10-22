# Archive manager — UI component and decision-surface specification

Status: approved implementation specification; no product rendering/interaction acceptance yet. Read with [design§6](design.md), [PRD](prd.md), [selection rules](../../spec/shared/ui-component-selection.md), [component table](../../../docs/reference-code/gpui-kit-component-table.md), [locked index](../../../docs/reference-code/gpui-kit-component-index.md) and full current [UI authority](../../spec/shared/ui-design.md). Context injection is not a full read.

## 1. Reference shape and target appearance

Reference: [Bandizip screenshot](assets/bandizip-layout-reference.png). Preserve title/menu/actions → left tree/right details → optional comment → status. Do not copy the blue gradient/white palette or hand-paint large toolbar pictograms. Kit Default Light/Dark owns ordinary control paint, fonts, radius and state grammar.

Approved browser client960×640/min720×480 logical px; tree200 (clamped160–280), flexible list. Main1060×650/min920×480 and176 rail unchanged. Minimum size uses horizontal list scroll/official overflow, not lost functionality or inflated client. Commands never beside close.

```text
┌ <archive name> — NxTools                       native window controls ┐
│ 文件  编辑  查看  工具                                                  │
│ [打开▾] [解压▾] [新建] [添加*] [删除*] [测试] [查看▾]                  │
├───────────────────┬──────────────────────────────────────────────────┤
│ archive.zip       │ [filter when requested / current folder]          │
│  └ folder tree    │ 名称      原始大小  压缩大小  类型  修改日期        │
│                   │ .. / files and child folders                     │
│                   ├──────────────────────────────────────────────────┤
│                   │ bounded archive comment when nonempty/requested  │
├───────────────────┴──────────────────────────────────────────────────┤
│ 文件/文件夹/选中统计                         format/encryption/solid    │
└──────────────────────────────────────────────────────────────────────┘
* Required Phase E mutations enabled only when real writable capability exists; must ship before first release.
```

## 2. Released component-selection ledger

`C:`=published gpui-component0.7.1/src; `B:`=gpui-base0.7.1/src. Product remains `gpui-kit = =0.7.1`. Entries reflect reviewed public interfaces, not tested integration.

| Surface / shape | Released interface | Variant, size and parent adaptation | Required compatibility |
| --- | --- | --- | --- |
| Settings | `component::setting::{Settings,SettingPage,SettingGroup,SettingItem,SettingField}`; C:setting/{settings,page,group,item}.rs and fields/mod.rs | released defaults; SettingField::{switch,dropdown,number_input,render}; resettable(false) until reset is designed | official search/selection/scroll, original save owner; no entire custom page in one render slot |
| Quick actions / toolbar | C:toolbar.rs Toolbar::new/content; C:button/button.rs Button::new().outline() | Medium/default32; actual outlined Buttons in Toolbar.content. Toolbar.child(Button) forces ghost+compact | idle Light/Dark bounds/paint, disabled/loading, pointer/Tab/arrows |
| Open/Extract split | C:button/dropdown_button.rs DropdownButton::{new,button,dropdown_menu,outline} | Medium in public content; main/menu trigger separate accessible names | one main action vs menu activation; no duplicate submit |
| Menu row | C:menu/app_menu_bar.rs AppMenuBar::new/reload evaluated | AppMenuBar uses global menus; prefer per-browser official Button+DropdownMenu/PopupMenu until focused-window menu isolation is proved | no mutation of other windows' menus; keyboard open/arrow/Escape/focus return |
| Tree | C:tree.rs Tree::new(&Entity<TreeState>,render_item→ListItem); B:tree.rs TreeState::{set_items,set_selected_item,reveal_item} | official ListItem/Tree/Icon/Label defaults | TreeEvent has Expanded/Collapsed, not selection-changed; observe ID safely and defer writeback |
| File details | C:table/{data_table,state,delegate,column}.rs DataTable/TableState/TableDelegate/Column | official Small/default density after comparison; keep virtualization/column resize | TableSelection single-row; §3 multi-select spike mandatory |
| Splitter | C:resizable.rs h_resizable/resizable_panel/ResizablePanelGroup/ResizableState | public size/size_range/on_resize, official handle | real bounds/persistence; keyboard/accessibility not inferred |
| Status | C:status_bar.rs StatusBar::{new,left,right}+Label/Tag | official defaults | source totals/selected totals distinct; unknown as — |
| Create/extract forms | C:form/{form,field}.rs Form/Field; one Root per native GPUI window | real Input/Select/Checkbox/RadioGroup/Button, Medium actions; scroll body fixed footer | stable entities, field validation, keyboard/close/cancel, minimum content |
| Password | C:input/input.rs Input::mask_toggle; B:input/base/state.rs InputState::masked/set_masked | official masked Input, visible field label; never OTP | per-prompt lifetime, no logs/argv, wrong/ambiguous-auth distinction |
| Conflict/danger | Dialog/AlertDialog public parts + DescriptionList + RadioGroup/Checkbox | default styled modal and footer Buttons | prompt ID/generation; once-only answer; close=cancel |
| Progress | C:progress/progress.rs Progress+Spinner/Label and official Buttons | known totals only; reduced motion honored | source known/unknown progress, no fake pause; terminal taskbar clear |
| Empty/errors | Empty,Alert,existing NotificationCenter | official semantic states | no-data/no-match/load-failure distinct; warnings≠success |
| Filter/path | InputGroup+original InputState; native picker Button | one official input frame, no second enclosing border | IME/clear/focus/long CJK |
| Comment | bounded plain read-only TextView; required Phase E comment editor Textarea | no disabled-editable-looking frame | never execute HTML/script/escape sequences |

Custom layout divs may arrange these components. No imaginary Card/Grid, no ordinary control appearance(false)+fake painted substitute, no palette fork/dependency patch. Exact source gap plus approved narrow exception is required before using a custom business row layout.

## 3. DataTable multi-select spike

Reviewed source shows TableSelection::{None,Row,Column,Cell}; TableEvent::{SelectRow,DoubleClickedRow,RightClickedRow}; TableDelegate::render_tr returns Stateful<Div>. DataTable subsequently sets `.aria_selected(is_selected)` and its own row click/keyboard actions. row_selectable(false) also disables normal row navigation/double-click. A HashSet and custom row paint are not sufficient proof of keyboard/UIA multi-select.

First build a disconnected fixture with the actual released component: Ctrl/Shift pointer, Shift+arrow range, Ctrl+A, right-click selection retention, sort/filter/scroll identity and UIA selected semantics. Require one owner, not delegated and business duplicate dispatch. Use public slots only. If insufficient, record exact gap and request approval for narrowly scoped ListItem+GPUI virtual-list business rows, preserving official ordinary controls/theme. No silent loss of multi-select to satisfy DataTable adoption. Update the ledger with actual selected solution and regression evidence before browser implementation.

2026-10-21 authorized minimal spike now has [compiled boundary evidence and exact public sources](phase-c-component-spike.md).
Stock Ctrl/Shift/keyboard remain single-row; a real public row callback retains multipleIDs but
DataTable overwrites final selected semantics and disabling row selection disables builtin row
keyboard/double-click. Public deferred Tree ID sync passes the small fixture. Required multi-select
gate is NOT passed. Narrow archive-details-only ListItem/virtual-list fallback awaits explicit
confirmation; it is not implemented or a blanket custom-control exception. Native GUI/UIA/visual
and sort/filter/scroll identity acceptance remain separate/unperformed.

## 4. Menus, keyboard and focus

- 文件: Open/New/Recent/Close archive/Open location/Test/Extract/Close window. 编辑: Copy relative path; required Phase E Add/Delete/Rename/New folder/Comment. 查看: Tree/Comment/Columns/Encoding. 工具: Settings. Excluded Scan/Repair/Recover omitted, not fake enabled.
- Context: Open/Extract selected/all/here/Copy relative path, then required Phase E mutations. Immutable IDs/session/revision, never current row indices.
- Enter activates one eligible focused item. Multi-file selection does not spawn unlimited apps; require deliberate choice/confirmation. Ctrl+A scoped to focused list, not active Input. Backspace does not steal text edits. F2/Del only when writable; folders first under ascending/descending sorting; equal values get stable tie-breaks.
- Browser is activating, not passive: never WS_EX_NOACTIVATE on primary archive UI. Rust layout→C# placement/readback→GPUI rendering; no competing native resize owner.
- Tree/list synchronization deduplicates node ID and defers cross-entity work. Escape dismisses current transient level, restores live trigger focus. Close archive returns start/open state; close browser retires entity but not independent jobs. Progress background never steals focus.

## 5. Decision surfaces and validation

Dimensions below are layout starting points for these new surfaces, not approved fixed-native budgets. Do not alter existing main/Clipboard/Quick Find windows. Forms are dedicated activating GPUI windows when invoked without a browser owner (e.g. CLI); local conflict/password confirmations use the owning window's official Dialog and one Root. No stacked duplicate Roots.

### Extract form

- Proposed width560, bounded body/max screen work area, nonshrinking32px official footer.
- Destination InputGroup with Browse Button; native picker owned by request ID/draft generation. Show canonical target preview locally, never log it.
- RadioGroup 全部文件/选定文件 (selected disabled when none); show count and selection source.
- Target strategy Ask/custom vs Smart/Here/Name is explicit. Show the final derived folder before Start; never apply name folder twice.
- Checkbox 打开目标文件夹, 完成后保留窗口, 记住设置. Delete source is per-job, off by default, disabled for selected extraction with visible reason; destructive consent remains visible.
- Conflict Select Ask/Overwrite/Skip/AutoRename. Missing destination prompts creation; low space shows required/available and proceed/cancel; denied path actionable, no automatic elevation.
- Primary 解压, outline 取消. Pending closes/cancels dialog draft, not unrelated jobs. Submit snapshots exact selected IDs/destination/options once; busy disables duplicate submission.

### Create form

- Proposed width680, flexible source list+bounded options scroll, fixed footer. Add files/Add folders/Remove source are real Buttons and native pickers; source enumeration asynchronous with its own generation/cancel.
- Required filename and destination, type-specific extension handling with explicit mismatch validation. Show eligible formats and semantic levels; no universal integer-level control.
- Password fields masked, optional confirmation/show. Password mismatch keeps Start disabled; changing to a nonencrypted format prompts to discard secret state instead of silently ignoring it. Secret fields clear on close/submit retirement where possible.
- ZIP method/encryption,7z solid/header encryption and thread options are shown only when backend implements them. TAR no level/password. GZ/XZ single regular-file-only, or explicit separately compress each eligible file; folders cannot be silently serialized as a single stream.
- Test-after-create defaults true. Delete sources forces successful full verification and explicit Recycle Bin consent. Exclusions validate globs; single-root stripping only when unambiguous; duplicate relative source names prompt/reject, not silent loss.
- Required Phase E split size accepts bounded positive integer+unit and per-format minimum; no ad-hoc text parsing/overflow/negative values.
- Primary 创建,outline 取消; field-specific errors visible and associated. Dialog resize never loses footer or source controls.

### Password / conflict / progress

- Password dialog shows why requested (open/test/extract), masked input and retry reason. It never guesses PasswordRequired by matching an error message; ambiguous damage/authentication explained truthfully. Apply-all session/batch scope, not global persistent cache.
- Conflict compares existing and incoming size/time/path locally using DescriptionList; Skip/Overwrite/Rename/Cancel + Apply-all this job. File-vs-directory conflict cannot blindly overwrite recursively; output identity rechecked after answer.
- Progress shows job kind/phase,current item, known done/total,speed/elapsed/ETA when valid. Paused vs awaiting password/conflict vs queued are distinct. Stop explicit; Background/close presentation-only. Unsupported pause visibly unavailable, never a decorative button.
- Terminal errors/warnings remain readable; Retry creates a new guarded operation; actions only for valid output. Notification uses existing source-bound center, not a second toast runtime.

## 6. Visual and interaction acceptance

All actionable surfaces: Default/Hover/Pressed/Focus-visible/Selected/Disabled/Loading/Empty/Error/Success. Compare actual official final parent variant, not imports or geometry alone. Text and icon/name communicate state without color alone. Icons use project assets/official licensed additions with attribution.

Required captures: module settings; ready browser; empty/error/password browser; selected rows/context menu; extract/create/conflict/progress. Test default/minimum × Light/Dark with long CJK and paths; individually inspect each capture. Add production bounds/hit/focus/one-dispatch tests, then separately actual pointer/keyboard100/125/150%, mixed monitors, UIA/provider and user approval. Generated but unviewed screenshots are not inspected; AI visual review is not user acceptance.
