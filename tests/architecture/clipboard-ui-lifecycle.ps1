$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$mainWindowPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs'
$mainWindowTestsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\tests.rs'
$settingsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\settings.rs'
$mainWindowSource = Get-Content -Raw -LiteralPath $mainWindowPath
$mainWindowTestsSource = Get-Content -Raw -LiteralPath $mainWindowTestsPath
$uiTestsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\tests.rs'
$settingsSource = Get-Content -Raw -LiteralPath $settingsPath
$clipboardPagePath = Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\clipboard_page.rs'
$clipboardPageSource = Get-Content -Raw -LiteralPath $clipboardPagePath
$uiTestsSource = Get-Content -Raw -LiteralPath $uiTestsPath

# Queue06 official Settings owns search, selection and scroll; business owners remain original.
foreach ($contract in @('Settings::new("clipboard-official-settings")', 'SettingPage::new(',
    'SettingGroup::new()', 'SettingItem::render(', 'aligned_setting_row(', '.justify_end()',
    '.resettable(false)', 'Form::vertical()', 'Field::new()', 'fn render_clipboard_control(',
    'clipboard-settings-workspace', 'clipboard-storage-path', '"呼出与快捷操作"', '"保留与隐私"', '"归档与存储"',
    'application.set_clipboard_modifier_action(modifier, action)', 'application.set_clipboard_retention(',
    'update_clipboard_capture_limit(', 'application.request_clipboard_export(cx)',
    'application.request_clipboard_import(cx)', 'application.request_clipboard_database_backup(cx)',
    'application.request_clipboard_database_compaction(cx)', 'application.request_clipboard_storage_migration(cx)',
    '.disabled(management_busy)', 'cx.defer(', 'this.hotkey_focus.focus(window, cx)',
    '"clipboard-hotkey-row"', '"clipboard-hotkey-help"', '"clipboard-hotkey-registration-status"')) {
    if (-not $clipboardPageSource.Contains($contract)) { throw "Clipboard Settings component/owner missing: $contract" }
}
if (-not $settingsSource.Contains('label.child(super::tooltip::description_help(help_selector, help))')) {
    throw 'Shared aligned setting row must retain actual title-adjacent help Tooltip.'
}
if ($clipboardPageSource.Contains('workbench_settings_section(') -or
    $clipboardPageSource.Contains('workbench_status_row(') -or
    $clipboardPageSource.Contains('.appearance(false)')) {
    throw 'Clipboard Settings must not retain whole-page hand-painted hierarchy or ordinary-control suppression.'
}
$clipboardKitTests = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\clipboard_page\kit_migration_tests.rs')
foreach ($contract in @('queue06_production_mounts_official_settings_search',
    'queue06_original_steps_preserve_section_disk_and_worker_root',
    'queue06_storage_actions_busy_cancel_and_error_never_run_data_operations',
    'queue06_minimum_scroll_reaches_storage_footer_and_keeps_notifications',
    'queue06_long_storage_path_stays_bounded_and_footer_reachable',
    'queue06_modifier_and_hotkey_cancel_keep_original_owner',
    'queue06_search_filters_without_saving_and_optional_help_is_real_tooltip',
    'window', 'simulate_resize', 'ClipboardCaptureWorkerHandle::spawn(', 'KeyUpEvent')) {
    if (-not $clipboardKitTests.Contains($contract)) { throw "Clipboard production regression missing: $contract" }
}

$historyRowPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\history.rs'
$historyRowSource = Get-Content -Raw -LiteralPath $historyRowPath
$historyRowSource = [regex]::Replace($historyRowSource, '(?m)^\s*//[^\r\n]*', '')

foreach ($contract in @('ListItem::new(', '.selected(selected)', '.aria_selected(selected)',
    'focused_action(', '.use_keyed_state(("history-row-focus", self.entry.id as u64)',
    'focus.contains_focused(window, cx)', 'Toolbar::new(', '.content(management_button(', 'Label::new(',
    'action_from_click(event, modifier_actions)', 'handler(action.clone(), event, window, cx)')) {
    if (-not $historyRowSource.Contains($contract)) { throw "Clipboard history component/owner missing: $contract" }
}
if ($historyRowSource.Contains('use crate::ui::components::list::compact_result_row')) {
    throw 'Clipboard history must use actual styled ListItem, not retired painted shell.'
}
if ($historyRowSource.Contains('fn history_row_border') -or
    $historyRowSource.Contains('fn history_row_background')) {
    throw 'CL-03 must retire the private hand-rolled history row border/background helpers.'
}

$popupModPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\mod.rs'
$popupModSource = Get-Content -Raw -LiteralPath $popupModPath
# Standalone comments are not executable wiring. Keep existing checks, then inspect the real owner slices.
$popupModSource = [regex]::Replace($popupModSource, '(?m)^\s*//[^\r\n]*', '')
foreach ($contract in @('focus_lost_subscription = cx.on_focus_lost(window,',
    'window.focus_lost_restore_target(cx).as_ref() == Some(&panel.focus_scope)',
    '.track_focus(&self.focus_scope)', 'focus_lost_subscription,')) {
    if (-not $popupModSource.Contains($contract)) { throw "Clipboard retained focus lifecycle missing: $contract" }
}
if ($popupModSource.Contains('.text_xs()')) {
    throw 'CL-04 requires clipboard popup typography to use ui::typography tokens instead of gpui text_xs.'
}
foreach ($contract in @('InputGroup::new("clipboard-search-group")',
    'InputGroupAddon::new("clipboard-search-actions")', 'InputGroupButton::new("search-scope-toggle")',
    'Input::new(&self.search_input)', 'Form::vertical()', 'Form::horizontal()', 'Field::new()',
    'self.render_markdown_editor(window, cx)', 'Input::new(&self.category_name_input)',
    'checkbox::Checkbox::new(', 'panel.toggle_search_scope(cx)', 'panel.save_edit(window, cx)',
    'panel.save_category_editor(window, cx)', 'panel.cancel_edit(window, cx)',
    'panel.cancel_category_editor(window, cx)', 'panel.restore_last_deleted(cx)',
    'CATEGORY_EDITOR_HEIGHT: f32 = 244.0', 'clipboard-category-editor-backdrop', '.occlude()',
    'let preview_enabled = !is_editing_category', 'self.query_dispatch_generation')) {
    if (-not $popupModSource.Contains($contract)) { throw "Clipboard search/editor component-owner missing: $contract" }
}
$markdownEdit = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\markdown_edit.rs')
$markdownEdit = [regex]::Replace($markdownEdit, '(?m)^\s*//[^\r\n]*', '')
foreach ($contract in @('Textarea::new(&self.edit_input)', 'BlockEditor::new(&self.edit_rich)', 'clipboard-rich-editor-scroll', 'window.defer(cx,', 'markdown_edit_value')) {
    if (-not $markdownEdit.Contains($contract)) { throw "Unified Clipboard editor/source ownership missing: $contract" }
}
foreach ($contract in @('edit_generation', 'edit_save_pending', 'panel.editing_item_id != Some(item_id)', 'panel.markdown_edit_value(cx) == text')) {
    if (-not $popupModSource.Contains($contract)) { throw "Clipboard save identity/draft gate missing: $contract" }
}
$beginStart = $popupModSource.IndexOf('fn begin_edit(')
$cancelStart = $popupModSource.IndexOf('fn cancel_edit(', $beginStart)
$saveStart = $popupModSource.IndexOf('fn save_edit(', $cancelStart)
$saveEnd = $popupModSource.IndexOf('fn restore_last_deleted(', $saveStart)
if ($beginStart -lt 0 -or $cancelStart -le $beginStart -or $saveStart -le $cancelStart -or $saveEnd -le $saveStart) {
    throw 'Clipboard edit lifecycle function boundaries missing.'
}
$beginEdit = $popupModSource.Substring($beginStart, $cancelStart - $beginStart)
$cancelEdit = $popupModSource.Substring($cancelStart, $saveStart - $cancelStart)
$saveEdit = $popupModSource.Substring($saveStart, $saveEnd - $saveStart)
if ($beginEdit -notmatch 'self\.edit_input\.update\([^;]+state\.set_value\(entry\.text\.clone\(\), window, cx\)' -or
    -not $beginEdit.Contains('self.prepare_markdown_edit(&entry.text, window, cx)') -or
    -not $cancelEdit.Contains('self.edit_save_pending = None;') -or
    -not $cancelEdit.Contains('self.editing_item_id = None;') -or
    -not $cancelEdit.Contains('self.search_input.focus_handle(cx).focus(window, cx);')) {
    throw 'Clipboard original full-item input/cancel/focus owner disconnected.'
}
if ($markdownEdit -notmatch '(?s)let body = if source_mode\s*\{\s*Textarea::new\(&self\.edit_input\).*?\}\s*else\s*\{' -or
    $markdownEdit -notmatch '\.child\(BlockEditor::new\(&self\.edit_rich\)' -or
    -not $markdownEdit.Contains('.child(body)') -or
    $markdownEdit.Contains('.passive(true)')) {
    throw 'Clipboard source/rich editor must mount original entities as interactive content.'
}
if ($saveEdit -notmatch '(?s)self\.edit_save_pending = Some\(operation_id\);.*?cx\.spawn\(.*?worker\.update_text_async\(item_id, &text\)\.await' -or
    $saveEdit -notmatch '(?s)if panel\.edit_generation != edit_generation\s*\|\| panel\.edit_save_pending != Some\(operation_id\)\s*\|\| panel\.editing_item_id != Some\(item_id\)\s*\{' -or
    -not $saveEdit.Contains('panel.markdown_edit_value(cx) == text') -or
    -not $saveEdit.Contains('cx.defer(move |cx|')) {
    throw 'Clipboard async save must retain pending-before-dispatch, draft identity and deferred focus.'
}
$markdownDocument = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\markdown_document.rs')
$markdownDocument = [regex]::Replace($markdownDocument, '(?m)^\s*//[^\r\n]*', '')
foreach ($contract in @('BlockEditor::new(&self.editor).passive(true)', 'state.set_readonly(true, cx)', 'ImagePolicy::Placeholder')) {
    if (-not $markdownDocument.Contains($contract)) { throw "Passive document owner disconnected: $contract" }
}
Write-Host 'PASS unified Clipboard editor mounts original interactive entities and passive documents with guarded save/cancel/focus owners (source only)'
if ($popupModSource.Contains('.appearance(false)') -or $popupModSource.Contains('.bordered(false)') -or
    $popupModSource.Contains('.focus_bordered(false)')) {
    throw 'Clipboard ordinary search/editor frames must belong to actual styled compounds/controls.'
}
$popupKitTests = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\kit_migration_tests.rs')
foreach ($contract in @('queue07_search_edit_scope_clear_preserves_input_and_query_generation',
    'queue07_text_form_edits_saves_temp_worker_and_restores_focus',
    'queue07_category_modal_occludes_search_and_saves_name_sources',
    'queue07_long_text_and_resize_keep_form_footer_and_cancel_identity',
    'queue07_released_tabs_direct_keyboard_does_not_replace_category_action')) {
    if (-not $popupKitTests.Contains($contract)) { throw "Clipboard production adoption regression missing: $contract" }
}
$categoryTabStart = $popupModSource.IndexOf('fn category_tab(')
$categoryTabEnd = $popupModSource.IndexOf('fn dialog_button(', $categoryTabStart)
if ($categoryTabStart -lt 0 -or $categoryTabEnd -le $categoryTabStart) {
    throw 'Could not isolate the clipboard category tab renderer.'
}
$categoryTab = $popupModSource.Substring($categoryTabStart, $categoryTabEnd - $categoryTabStart)
foreach ($requiredTabContract in @(
    'Tab::new()',
    '.underline()',
    '.xsmall()',
    '.track_focus(&focus)',
    'Tooltip::new(',
    'context_menu_shell(',
    '.on_drag(',
    '.on_drop(',
    '.on_scroll_wheel('
)) {
    if (-not $popupModSource.Contains($requiredTabContract)) {
        throw "CL-04 category strip contract missing: $requiredTabContract"
    }
}
if (-not $popupKitTests.Contains('feedback_category_tabs_pointer_and_focused_release_have_one_owner')) {
    throw 'Actual Tab pointer/focus/release production regression missing.'
}

Write-Host 'PASS clipboard history uses actual stateless ListItem/Toolbar/Label on original variable-height/action owners'
Write-Host 'PASS clipboard actual Kit Tab keeps one focused click owner with preserved drag, wheel and menu lifecycle'

$previewPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\preview.rs'
$previewSource = Get-Content -Raw -LiteralPath $previewPath
foreach ($previewContract in @(
    'pub(crate) fn preview_shell(theme: Theme) -> Div',
    'pub(crate) fn preview_media_surface(theme: Theme) -> Div',
    'struct ClipboardPreview',
    'clipboard_preview_uses_the_shared_neutral_shell_and_media_surface'
)) {
    if (-not $previewSource.Contains($previewContract)) {
        throw "CL-05 shared preview contract missing: $previewContract"
    }
}
if (-not $popupModSource.Contains('const IMAGE_RESOURCE_CACHE_CAPACITY: usize = 96;')) {
    throw 'CL-05 preview resource cap constant is missing.'
}
if (-not $uiTestsSource.Contains('fn image_resource_cache_evicts_the_oldest_entry_beyond_capacity()')) {
    throw 'CL-05 preview resource cap regression is missing.'
}

Write-Host 'PASS clipboard preview shares the QL preview shell with a bounded 96-entry resource cache'

# Clipboard's production trigger must never fall back to a clipped source-window menu or HMENU.
$projectMenuSource = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\project_menu.rs')
$nativeMenuSource = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\native_context_menu.rs')
$styledMenuSource = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\styled_menu_surface.rs')
foreach ($contract in @('PopupMenu::build(', 'PopupMenuItem::submenu(', 'PopupMenuItem::element(', '.checked(', '.disabled(', '.scrollable(true)', 'DismissEvent', 'Root::new(', 'WindowBackgroundAppearance::Transparent')) {
    if (-not $styledMenuSource.Contains($contract)) { throw "Official outside-HWND menu missing: $contract" }
}
foreach ($contract in @('popup_window_handle:', '.take()', 'window.defer(', 'window.draw(cx).clear(cx)', 'cx.defer(show)', 'pending.command_id == command_id', 'fn handle_notification', 'source_process_id:', 'trigger.upgrade()', 'styled_menu_surface::open(')) {
    if (-not $nativeMenuSource.Contains($contract)) { throw "Styled menu lifecycle missing: $contract" }
}
if ($nativeMenuSource.Contains('window.on_next_frame(')) {
    throw 'Hidden menu preparation must not await an ordinary Windows frame before reveal.'
}
$nativeMenuTests = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\native_context_menu_tests.rs')
foreach ($contract in @('hidden_menu_preparation_builds_scene_without_platform_frame_before_reveal', 'hidden_menu_mismatched_native_viewport_never_reveals', 'hidden_menu_retired_trigger_or_disconnect_cannot_reveal')) {
    if (-not $nativeMenuTests.Contains($contract)) { throw "Hidden menu lifecycle regression missing: $contract" }
}
if ($projectMenuSource.Contains('menu.open(items, token, position, previous_focus, window, cx)')) {
    throw 'Production trigger must not silently fall back to the clipped source-window menu.'
}
$nativeMenuAdapter = Get-Content -Raw -LiteralPath (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\NativeContextMenuCoordinator.cs')
foreach ($contract in @('PrepareStyledSurface(', 'MonitorFromPoint', 'GetMonitorInfoW', 'ClientToScreen', 'PopupWindowHandle')) {
    if (-not $nativeMenuAdapter.Contains($contract)) { throw "Styled menu native placement missing: $contract" }
}
$menuTests = Get-Content -Raw -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\styled_menu_surface_tests.rs')
foreach ($contract in @('official_menu_renders_outside_source_budget', 'disabled_ancestor', 'terminal_is_once_deferred', 'stale_trigger')) {
    if (-not $menuTests.Contains($contract)) { throw "Production menu regression missing: $contract" }
}
Write-Host 'PASS Clipboard official PopupMenu has independent GPUI surface and native placement/session guards (source only)'

$linkPanelSource = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\tools\link_panel.rs')
foreach ($contract in @('ListItem::new(', 'StatusBar::new()', 'Label::new(', '.small()',
    'window.remove_window();', 'cx.defer(move |cx|', 'application.dispatch_link_panel_route(',
    'link_panel_height(this.links.len(), true)', 'fn arm_auto_hide(', 'fn swap_links(')) {
    if (-not $linkPanelSource.Contains($contract)) { throw "Link component/original owner missing: $contract" }
}
$toolSource = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\tools_page.rs')
foreach ($contract in @('ListItem::new("tool-link-detector-item")', 'focused_action(card.id,',
    'Label::new(card.title)', '.bg(theme.surface)', '.text_xs()', 'let toggle_enabled = cx.listener(',
    'application.set_link_detector_enabled(!application.link_detector_enabled(), cx)',
    'cx.stop_propagation();', 'open_tool_settings_window(this, card, cx);',
    'NumberInput::new(', 'Select::new(', 'Form::horizontal()')) {
    if (-not $toolSource.Contains($contract)) { throw "Tools presentation/original settings missing: $contract" }
}
if (-not $mainWindowTestsSource.Contains('tool_card_toggles_and_configure_opens_without_toggling') -or
    -not $clipboardKitTests.Contains('clipboard_hotkey_recorder_is_right_aligned_beside_title')) {
    throw 'Card toggle/Configure isolation and hotkey first-row/right-edge regression missing.'
}
foreach ($contract in @('Empty::new()', 'EmptyHeader::new()', 'EmptyTitle::new()',
    'Alert::error("clipboard-history-query-alert"', 'entries_empty && !search_loading && query_failed')) {
    if (-not $popupModSource.Contains($contract)) { throw "Clipboard distinct state presentation missing: $contract" }
}
Write-Host 'PASS Link/Tools stateless presentation keeps original close/defer/timer/settings boundaries (source only)'
