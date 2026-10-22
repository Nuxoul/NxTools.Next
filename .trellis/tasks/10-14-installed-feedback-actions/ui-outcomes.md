# UI action and hotkey alignment corrections

## Reference and component boundary

User explicitly supersedes the historical whole-card-settings action: card body toggles Link Detection, Configure alone opens keyed settings. Legacy has no Link Detection tool equivalent, so this is not claimed as old NxTools behavior. Complete official0.7.1 index and released ListItem/Button/SettingItem/Form API inspected. Keep compound Settings hierarchy/public render slot and one original section/persistence owner; no replacement state manager or dependency change. Card180x160 remains.

Clipboard reference rechecked D:\Code\NxTools\Views\MainWindow.axaml1460–1495: summon hotkey labels and editor share a row, registration state separately visible. Next recorder already preserves original acknowledged RegisterHotKey suspend/cancel/reapply flow; only wide-slot Form vertical presentation was wrong.

## Red evidence

Real `tool_card_toggles_and_configure_opens_without_toggling` failed on actual card click not enabling, before product change. Production bounds hotkey regression measured row368,211/520x76 versus editor396,235.5/64x32 (24.5px below first row); this is screenshot symptom, not helper-only geometry.

## Product changes

main_window/tools_page.rs: body focused_action routes pointer and GPUI generated Enter/Space click once to cx.defer, then reads latest ClipboardApplication enabled state and calls original set_link_detector_enabled. Configure gets its own official Button click/focus stop, stops propagation before opening/reusing settings. No synchronous cross-owner lease update. Original persistence errors/owner remain.

main_window/clipboard_page.rs: slot0 retains Settings::render and original recorder Button/callback; uses existing aligned_setting_row with title-adjacent help and right-aligned control lane. Registration text remains a separate visible row. No title duplication/recording/native policy change.

## Initial green

Actual pointer toggle/temporary durable save/Configure-no-bubble and existing shared settings Toggle/position owner test passes. New title/recordercenter<=2px,rightedge<=2px,firstrow/statusbelow at920and1060 widths passes. Existing modifier+hotkeycancel owner passes. Full integration and strict/source checks recorded after actual run; no screenshots/live native acceptance/package certified here.

## Integrated checks

Desktop679/679 and strict tests Clippy/fmt passed before final keyboard extension.
Configure Tab/Enter extension initially failed because the historical MainWindow fixture
was not wrapped in Kit Root (which owns Tab traversal); corrected to actual Root host,
then focused pointer/Enter/Space/Tab-to-Configure/window-reuse regression passed. No
product keyboard workaround or extra dispatcher added. C# full audited223/223 build
0warnings/errors passes after child-HWND native traversal repair.4UIlifecycle source
gates pass with shared-help call chain/card split owner guards updated, not removed.
Final full/check/lint/fmt results appended after actual run.

Explorer remains an installed acceptance gap: repaired a reproduced browser-child root
classification omission, but passive enumeration returned0Explorerwindows and the owned
chain deliberately ends at an injected GetItemObject failure. It does not prove complete
real selected-target success or that this is the user's only failure. See explorer-outcomes.md.
No automatic package/install/user Explorer mutation performed.

Final integrated run: Desktop679/679,strict testsClippy,workspace--testscompile,fmt all
exit0. C#223/223,0warnings/errors.4UIsourcegates+spec87docs/12owners+scopeddiff pass.
No product package or native live action executed; previous installer remains unchanged.
