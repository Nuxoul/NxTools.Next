# Audit #7 cursor inventory — pending production paint-hit verification

## Evidence level and rule
Source review only. **No effective red, repair or compiled acceptance.** Original design guide Pointer conventions470–475 and Button-vs-Link507–532 require Arrow for ordinary native commands/selections; PointingHand only for actual links/link-like content. True manipulation cursors remain. Existing names/appearance/focus/keyboard/disabled/drag/click/callback/rem behavior is immutable for this cursor-only task.

## All current application cursor_pointer declarations
Source search found12 declarations across9files.11 are in production,1 is in a file-level test-only helper. These are declarations/candidates, not12 proven painted cursor defects; nested official controls may override a parent. No deletion was made.

| Source / line / actual element | Semantic classification / actual consumer | Required cursor verification and preservation |
| --- | --- | --- |
| ui/components/action.rs39, focused_action | Generic Base Button with original tracked FocusHandle and one click owner. Used by FocusableContent, Clipboard history/add-category, QuickLaunch row wrapper, Tools catalog/state/preview/setting controls, Download categories. Command/selection, not URL authority. | Pending actual current-frame normal/hover/disabled hit. Enabled hand is explicit; disabled omits that declaration but descendant requests still matter. Preserve keyed focus, stop_propagation, disabled and single original ClickEvent. Do not blanket override nested real text/link/manipulation. |
| ui/components/path_list.rs161, path_row | Official ListItem selects exclusion row; trailing official delete40x40 has its own callback. Main Quick settings index/exclusions consumes this helper. It does not open the displayed path. | Arrow ordinary row, current hit over text and nested delete. Preserve exact path Tooltip, original select/remove callbacks, row selection/keyboard owner, fixed40 delete hit and rem text/rails. |
| ui/components/volume_scope.rs85 and159 | automatic_volume_scope_row and volume_scope_row: original checkbox-like selection through FocusableContent; display Checkbox Small/tab_stopfalse. Main Quick settings production callers727/768 defer original indexed-volume setter. | Arrow over row/label/checkbox, checked/unchecked and still-current interaction. Preserve one owner and no inert added checkbox tab stop. No actual indexing/service/config mutation in new cursor fixture. |
| ui/components/custom_command_editor.rs231 | command_master_row ListItem -> original focusable_content. Main Quick settings857 opens retained command editor/draft, does not execute command. | Arrow for enabled and disabled BUSINESS RECORD (disabled record remains editable). Do not disable UI based on command.enabled. Preserve selection/draft callback/entities/IDs/rem layout and #13 Strings. |
| quick_launch/panel.rs1142 | render_result inner div, wrapped by original quick_launch_focusable_row -> focused_action; actual launcher result/recent directory presentation. | Ordinary result-command row requires actual paint-hit test. A result target containing URL is not by itself permission to turn the whole command palette row into a Link; separately verify any truly link-like public content. Preserve query/IME, local selection, pointer gate, current generation preview and original deferred activation. |
| quick_launch/panel.rs1340 | render_action enabled inner row, original action mode/Shell/submenu command. Disabled actions do not add this declaration. | Arrow normal/hover/disabled, interior and wrapper hit. Preserve activatable eligibility, submenu/parent query restoration, original keyboard/once-only action owner. |
| quick_launch/file_dialog_search.rs473 and580 | Attached default-source row navigates original selected directory; result row selects/navigates. Both wrapped by original keyed quick_launch_focusable_row. | Arrow normal/hover/disabled where supported. These are commands, not external-resource Links. Preserve host placement/input/IME/query/focus/selection/shortcut/cancel/native policies. |
| downloads/new_download_view.rs1736 | Real official ListItem in category Popover, existing query/highlight/category selection and focus return. | Arrow category choice. Preserve actual category callback/Enter/focus/ScrollHandle, fixed popup/client and accepted rem density; no picker/probe/download action in cursor fixture. |
| ui/mod.rs2801 | Actual add-Clipboard-category focused_action, editor-open command and category reorder drop target. | Arrow for ordinary add command. A drag-over/drop target is not automatically a constant Grab cursor. Preserve original reorder/drop/editor/click/keyboard owner; active drag cursor may supersede ordinary hit. |
| ui/components/list.rs35 | **TEST_ONLY**: first line #![cfg(test)] gates entire compact_list_row file. It wraps focusable_content in fixtures; not a production owner. | Do not cite as production red. Any consistency cleanup must remain separate and must not weaken old keyboard/selection tests. |

## focused_action actual production consumers
- ui/history.rs396: immutable item-ID ancestry and original paste/action readiness. #10 stable focus, row removal/armed-release cancellation and full payload policy stay unchanged.
- ui/mod.rs2777: add category/editor/drop target. Category keyboard/drop owner remains.
- quick_launch/panel.rs21: shared wrapper used by launcher results/action rows and attached search/default rows. Inner cursor declarations must be verified together, not hidden by an outer-only fix.
- main_window/tools_page.rs165: actual catalog tile toggle owner, Configure remains separate official Button/no bubble. Tools fixed180x160 remains.
- main_window/tools_page/{currency_settings.rs35,floating_assistant_settings.rs52}: original local preview/sample setting controls, not external links.
- ui/components/state.rs203: interactive state action, not URL.
- downloads/settings_view.rs1159: category master FocusableContent.
- volume_scope/custom_command_editor helpers above; path_list uses original official ListItem directly.

Search did not establish an actual product Link::new/.href call in Desktop src. That absence is not a universal no-link claim: pinned document/rendered text internals may contain link content. Preserve genuine external links, including official/public Link when exercised as a control fixture; never launch href/network or replace passive document behavior. Text-buffer `.text()` calls are unrelated to ButtonVariants::text; grep hits alone do not classify links.

## Exact released source / parent adaptation
- Component0.8.0 button/button.rs654–664 explicitly sets cursor_default, then interactive Link/Text variant gets cursor_pointer. Its ordinary Primary/Outline/Ghost sizing/focus/disabled/paint is untouched. No current production Link/Text Button-variant invocation was established by the source search; do not change vendor or assume every arbitrary text child is a link.
- Component0.8.0 link.rs69–104 uses real external href/on_click and cursor_pointer, with caller style refinement. This legitimate source remains unchanged. A Label/Button whose title says Open/View is not automatically a Link.
- Actual ListItem public Styled refinement can carry an application cursor while retaining selected/hover/state grammar. Existing Toolbar.content preserves outlined variants; no parent adaptation/compound/controller changes are needed for a cursor-only repair.
- Base0.8.0 Button/Checkbox/Tab current render sources were inspected read-only. No new focus/key dispatcher is allowed. Private source presence is not public test API.
- Published pre0.3.9 window.rs1150–1163 resolves the painted frame's cursor requests in reverse order by actual hitbox. reset_cursor_style5538–5545 defaults to Arrow and forwards to platform only while hovered. Descendant requests and active-drag window cursor must be tested through the original boundary.
- Frame.cursor_style is pub(crate), TestPlatform.active_cursor is private and TestAppContext retains its platform privately. No compliant public final-cursor getter/test observation recipe has been proved/compiled this turn. This remains **UNFINISHED TEST-SEAM INVESTIGATION**, not a framework capability refusal or authorization to patch vendor/recreate the resolver.

## Preserved true manipulation / external-resource exceptions
| Exact source / operation | Preserve |
| --- | --- |
| Component0.8.0 link.rs89 and interactive Button Link/Text semantics at button.rs663 | Genuine external resource hand; test render/hit only, never invoke href/open_url/network. Application command misusing Link would require its own actual evidence; none repaired here. |
| ui/mod.rs3434 panel resize -> original begin_window_resize | EResize; real resize operation, not command-style hand. No native call or hit size change. |
| download_taskbar_overlay.rs626 editing-only -> BeginWindowMove | Move only during existing edit mode and original gesture. No capture/drag/native change. |
| download_taskbar_overlay.rs791 resize handle -> original resize_listener | EResize; preserve exact hit region and original once-only start. |
| ui/mod.rs2665 panel header | Existing cursor_default remains, along with move/header gesture policy. |
| Official Input/Textarea/editor text, released resizable/splitter/scrollbar, active GPUI/native drag | Retain original Text/Resize/Grab/Prohibited/active drag cursor by real operation. No custom cursor blanket and no vendor/protocol/native change. Actual exception matrix NOT RUN here. |

## Required implementation/acceptance still open
1. Re-observe natural idle, re-ground dirty bytes; finish compliant public/test paint-hit observation recipe without vendor/alternative resolver.
2. Effective red on actual focused_action and actual production consumers with fresh frame/hit, normal/hover/disabled and nested official controls, then minimal cursor-only changes. Do not treat source matches as executed red.
3. Verify genuine link/text/resize/active-drag exception and no accidental inheritance loss; actual synthetic pointer/Tab/KeyUp/held/modifier/disabled/release cancel and original callback counts. No direct business-call shortcut.
4. Run relevant UI/QuickLaunch/Downloads/affected MainWindow/settings/Clipboard and #1/#9/#10/first-seven protections, strict production/tests lint/check/fmt/source gates. No full Desktop. Existing rem repairs and incomplete #6 remain as recorded, not reopened or closed.
