$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$dispatcherPath = Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs'
$source = Get-Content -Raw -LiteralPath $dispatcherPath
$quickLaunchPath = Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch.rs'
$quickLaunchSource = Get-Content -Raw -LiteralPath $quickLaunchPath
$metricsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\metrics.rs'
$metricsSource = Get-Content -Raw -LiteralPath $metricsPath
$mainWindowPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs'
$mainWindowSource = Get-Content -Raw -LiteralPath $mainWindowPath
$hotkeyFieldPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\hotkey.rs'
$hotkeyFieldSource = Get-Content -Raw -LiteralPath $hotkeyFieldPath
$volumeScopePath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\volume_scope.rs'
$volumeScopeSource = Get-Content -Raw -LiteralPath $volumeScopePath
$previewPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\preview.rs'
$previewSource = Get-Content -Raw -LiteralPath $previewPath
$quickLaunchSmokePath = Join-Path $root 'tests\runtime\quick-launch-search-smoke.ps1'
$quickLaunchSmokeSource = Get-Content -Raw -LiteralPath $quickLaunchSmokePath
$iconsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\icons.rs'
$iconsSource = Get-Content -Raw -LiteralPath $iconsPath
$panelRenderStart = $quickLaunchSource.IndexOf('impl Render for QuickLaunchPanel')
$panelRenderEnd = $quickLaunchSource.IndexOf('fn source_icon(', $panelRenderStart)
if ($panelRenderStart -lt 0 -or $panelRenderEnd -le $panelRenderStart) {
    throw 'Could not isolate the production Quick Launch panel renderer.'
}
$panelRender = $quickLaunchSource.Substring($panelRenderStart, $panelRenderEnd - $panelRenderStart)
foreach ($requiredPopupConstant in @(
    'const PANEL_WIDTH: f32 = 620.0',
    'const PANEL_HEIGHT: f32 = 410.0',
    'const SEARCH_HEADER_HEIGHT: f32 = 42.0',
    'const QUICK_LAUNCH_SEARCH_TEXT_SIZE: f32 = TEXT_BODY'
)) {
    if (-not $quickLaunchSource.Contains($requiredPopupConstant)) {
        throw 'Quick Launch must preserve the tokenized collapsed/expanded popup constants.'
    }
}
foreach ($requiredPopupRenderContract in @(
    'quick-launch-search-header',
    'quick-launch-results-surface',
    '.font_family(UI_FONT_FAMILY)',
    '.bg(theme.canvas)',
    '.text_color(theme.text)',
    '.border_color(theme.window_border)',
    '.border_color(theme.border_subtle)',
    '.text_color(theme.icon_muted)',
    '.line_height(px(LINE_HEIGHT_BODY))',
    '.when(self.results_visible'
)) {
    if (-not $panelRender.Contains($requiredPopupRenderContract)) {
        throw 'Quick Launch must preserve the tokenized collapsed/expanded popup contract.'
    }
}
if (-not $quickLaunchSource.Contains(
        'fn production_popup_keeps_search_only_collapsed_and_expands_one_stable_surface('
    )) {
    throw 'Quick Launch popup geometry requires a real production-window regression test.'
}
$settingsPageStart = $mainWindowSource.IndexOf('    fn select_quick_launch_settings_page(')
$settingsPageEnd = $mainWindowSource.IndexOf('    fn render_notifications(', $settingsPageStart)
if ($settingsPageStart -lt 0 -or $settingsPageEnd -le $settingsPageStart) {
    throw 'Could not isolate the Quick Launch settings page lifecycle.'
}
$settingsPageLifecycle = $mainWindowSource.Substring(
    $settingsPageStart,
    $settingsPageEnd - $settingsPageStart
)
foreach ($requiredSettingsPageContract in @(
    'self.quick_launch_settings_page = page',
    'self.quick_find_scroll_animation = self.quick_find_scroll_animation.wrapping_add(1).max(1)',
    'self.quick_find_scroll_target = 0.0',
    '.set_offset(point(px(0.0), px(0.0)))'
)) {
    if (-not $settingsPageLifecycle.Contains($requiredSettingsPageContract)) {
        throw 'Quick Launch settings navigation must cancel stale scroll work and reset the selected page viewport.'
    }
}
foreach ($requiredSettingsPageRegression in @(
    'fn quick_launch_settings_navigation_mounts_only_the_selected_page(',
    'fn every_quick_launch_settings_frame_mounts_exactly_one_page('
)) {
    if (-not $mainWindowSource.Contains($requiredSettingsPageRegression)) {
        throw 'Quick Launch QL-07 requires real production-window navigation and exclusive-mount regressions.'
    }
}
$quickLaunchSettingsStart = $mainWindowSource.IndexOf('    fn render_quick_find(')
$quickLaunchSettingsEnd = $mainWindowSource.IndexOf(
    '    fn render_clipboard(',
    $quickLaunchSettingsStart
)
if ($quickLaunchSettingsStart -lt 0 -or $quickLaunchSettingsEnd -le $quickLaunchSettingsStart) {
    throw 'Could not isolate the Quick Launch settings render boundary.'
}
$quickLaunchSettings = $mainWindowSource.Substring(
    $quickLaunchSettingsStart,
    $quickLaunchSettingsEnd - $quickLaunchSettingsStart
)
foreach ($requiredHotkeyFieldToken in @(
    'CONTROL_HEIGHT_INPUT',
    'RADIUS_CONTROL',
    'TEXT_BODY',
    'LINE_HEIGHT_BODY',
    'TEXT_LABEL',
    'LINE_HEIGHT_LABEL',
    'UI_MONO_FONT_FAMILY'
)) {
    if (-not $hotkeyFieldSource.Contains($requiredHotkeyFieldToken)) {
        throw 'Quick Launch HotkeyField must use the shared input, radius and typography tokens.'
    }
}
foreach ($requiredHotkeyFieldState in @(
    'Applied',
    'Waiting',
    'Recording',
    'Applying',
    'Failed'
)) {
    if (-not $hotkeyFieldSource.Contains($requiredHotkeyFieldState) -or
        -not $quickLaunchSettings.Contains("HotkeyFieldState::$requiredHotkeyFieldState")) {
        throw 'Quick Launch HotkeyField must expose applied, waiting, recording, applying and failed states.'
    }
}
if (-not $quickLaunchSettings.Contains(
        'hotkey_field('
    ) -or
    -not $quickLaunchSettings.Contains(
        'hotkey_setting_row('
    ) -or
    -not $mainWindowSource.Contains(
        'fn quick_launch_hotkey_field_exposes_the_native_registration_lifecycle('
    ) -or
    -not $mainWindowSource.Contains(
        'PlatformCommand::SetQuickLaunchHotKeyRecording'
    ) -or
    -not $mainWindowSource.Contains(
        'PlatformCommand::ConfigureQuickLaunch'
    )) {
    throw 'QL-08 requires the shared HotkeyField and a real typed registration lifecycle regression.'
}
$hotkeyLifecycleStart = $mainWindowSource.IndexOf('    fn defer_hotkey_recording(')
$hotkeyLifecycleEnd = $mainWindowSource.IndexOf('    fn module_icon(', $hotkeyLifecycleStart)
if ($hotkeyLifecycleStart -lt 0 -or $hotkeyLifecycleEnd -le $hotkeyLifecycleStart) {
    throw 'Could not isolate the shared hotkey recording lifecycle.'
}
$hotkeyLifecycle = $mainWindowSource.Substring(
    $hotkeyLifecycleStart,
    $hotkeyLifecycleEnd - $hotkeyLifecycleStart
)
if (($hotkeyLifecycle.Split('cx.defer(move |cx|').Count - 1) -lt 3 -or
    -not $hotkeyLifecycle.Contains('fn defer_recorded_hotkey(') -or
    -not $hotkeyLifecycle.Contains('fn defer_all_hotkey_recording(')) {
    throw 'Hotkey recording and apply mutations must defer every cross-entity update.'
}
foreach ($requiredVolumeScopeContract in @(
    'Checkbox::new(',
    'VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT: f32 = 48.0',
    'VOLUME_SCOPE_ROW_HEIGHT: f32 = 40.0',
    'RADIUS_CONTROL',
    'TEXT_BODY',
    'LINE_HEIGHT_BODY',
    'TEXT_LABEL',
    'LINE_HEIGHT_LABEL',
    'quick-launch-volume-scope-automatic'
)) {
    if (-not $volumeScopeSource.Contains($requiredVolumeScopeContract)) {
        throw 'QL-09 volume scope must use one tokenized standard-checkbox list.'
    }
}
if ($volumeScopeSource.Contains('.flex_wrap()') -or
    $volumeScopeSource.Contains('MaterialIcon::Check') -or
    -not $quickLaunchSettings.Contains('volume_scope_setting(') -or
    -not $quickLaunchSettings.Contains('automatic_volume_scope_row(') -or
    -not $quickLaunchSettings.Contains('volume_scope_row(') -or
    $quickLaunchSettings.Contains('quick-launch-index-all-volumes') -or
    -not $mainWindowSource.Contains(
        'fn quick_launch_volume_scope_uses_one_checkbox_list_and_preserves_automatic_semantics('
    ) -or
    -not $mainWindowSource.Contains('fn assert_single_index_settings_command(') -or
    -not $mainWindowSource.Contains(
        'a rejected last-volume toggle must not persist or synchronize'
    )) {
    throw 'QL-09 must replace wrapping chips with the production VolumeScope regression.'
}
$volumeScopeRenderStart = $quickLaunchSettings.IndexOf('        let available_volumes =')
$volumeScopeRenderEnd = $quickLaunchSettings.IndexOf(
    '        let mut index_exclusions_control =',
    $volumeScopeRenderStart
)
if ($volumeScopeRenderStart -lt 0 -or $volumeScopeRenderEnd -le $volumeScopeRenderStart) {
    throw 'Could not isolate the Quick Launch volume-scope renderer.'
}
$volumeScopeRender = $quickLaunchSettings.Substring(
    $volumeScopeRenderStart,
    $volumeScopeRenderEnd - $volumeScopeRenderStart
)
if (($volumeScopeRender.Split('cx.defer(move |cx|').Count - 1) -lt 2 -or
    -not $volumeScopeRender.Contains('toggle_indexed_volume_selection(') -or
    -not $volumeScopeRender.Contains('application.set_indexed_volumes(')) {
    throw 'Volume-scope mutations must preserve canonical selection and defer cross-entity updates.'
}
$searchInputStart = $panelRender.IndexOf('Input::new(&self.search_input)')
$searchInputEnd = $panelRender.IndexOf('.when(recent_button_visible', $searchInputStart)
if ($searchInputStart -lt 0 -or $searchInputEnd -le $searchInputStart) {
    throw 'Could not isolate the production Quick Launch search input.'
}
$searchInputRender = $panelRender.Substring($searchInputStart, $searchInputEnd - $searchInputStart)
foreach ($requiredSearchInputTypeRole in @(
    '.text_size(px(QUICK_LAUNCH_SEARCH_TEXT_SIZE))',
    '.line_height(px(LINE_HEIGHT_BODY))'
)) {
    if (-not $searchInputRender.Contains($requiredSearchInputTypeRole)) {
        throw 'Quick Launch search input must apply the shared Body type role directly.'
    }
}
$resultRenderStart = $quickLaunchSource.IndexOf('    fn render_result(')
$resultRenderEnd = $quickLaunchSource.IndexOf('    fn render_action(', $resultRenderStart)
if ($resultRenderStart -lt 0 -or $resultRenderEnd -le $resultRenderStart) {
    throw 'Could not isolate the production Quick Launch result renderer.'
}
$resultRender = $quickLaunchSource.Substring(
    $resultRenderStart,
    $resultRenderEnd - $resultRenderStart
)
$rowShellStart = $quickLaunchSource.IndexOf('fn quick_launch_row_shell(')
$rowShellEnd = $quickLaunchSource.IndexOf('impl QuickLaunchPanel', $rowShellStart)
if ($rowShellStart -lt 0 -or $rowShellEnd -le $rowShellStart) {
    throw 'Could not isolate the shared Quick Launch result/action row shell.'
}
$rowShell = $quickLaunchSource.Substring($rowShellStart, $rowShellEnd - $rowShellStart)
foreach ($requiredResultConstant in @(
    'const COMPACT_RESULT_ROW_HEIGHT: f32 = 40.0',
    'const COMPACT_RESULT_ICON_RAIL_WIDTH: f32 = 26.0',
    'const COMPACT_RESULT_TRAILING_RAIL_WIDTH: f32 = 48.0'
)) {
    if (-not $metricsSource.Contains($requiredResultConstant)) {
        throw 'Quick Launch result rows must preserve the fixed three-rail geometry.'
    }
}
foreach ($requiredResultRenderContract in @(
    'quick-launch-result-icon-',
    'quick-launch-result-copy-',
    'quick-launch-result-title-',
    'quick-launch-result-subtitle-',
    'quick-launch-result-shortcut-',
    '.gap(px(SPACING_2))',
    '.text_size(px(TEXT_BODY))',
    '.line_height(px(LINE_HEIGHT_BODY))',
    '.text_size(px(TEXT_LABEL))',
    '.line_height(px(LINE_HEIGHT_LABEL))',
    '.size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))',
    '.w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))'
)) {
    if (-not $resultRender.Contains($requiredResultRenderContract)) {
        throw 'Quick Launch heterogeneous results must share one tokenized three-rail row.'
    }
}
if ($resultRender.Contains('RESULT_TITLE_SIZE') -or
    $resultRender.Contains('RESULT_DETAIL_SIZE') -or
    $resultRender.Contains('.gap(px(1.0))') -or
    $resultRender.Contains('.bg(theme.surface_raised)')) {
    throw 'Quick Launch result rows must not restore private type, spacing or decorative icon tiles.'
}
foreach ($requiredSelectionContract in @(
    '.on_mouse_move(',
    '.on_mouse_down(gpui::MouseButton::Left',
    'panel.select_result(index, cx)'
)) {
    if (-not $resultRender.Contains($requiredSelectionContract)) {
        throw 'Quick Launch keyboard and pointer selection must share one neutral production row state.'
    }
}
if (-not $rowShell.Contains('theme.element_selected') -or
    -not $rowShell.Contains('.h(px(COMPACT_RESULT_ROW_HEIGHT))') -or
    -not $rowShell.Contains('.gap(px(SPACING_8))') -or
    $rowShell.Contains('theme.accent_soft') -or
    -not $quickLaunchSource.Contains(
        'fn production_keyboard_pointer_movement_and_press_share_one_selection('
    ) -or
    -not $quickLaunchSource.Contains('fn stationary_pointer_does_not_steal_keyboard_selection(')) {
    throw 'Quick Launch QL-03 requires neutral selection plus real pointer and stationary-pointer regressions.'
}
$actionRenderStart = $quickLaunchSource.IndexOf('    fn render_action(')
$actionRenderEnd = $quickLaunchSource.IndexOf(
    'impl Render for QuickLaunchPanel',
    $actionRenderStart
)
if ($actionRenderStart -lt 0 -or $actionRenderEnd -le $actionRenderStart) {
    throw 'Could not isolate the production Quick Launch action renderer.'
}
$actionRender = $quickLaunchSource.Substring(
    $actionRenderStart,
    $actionRenderEnd - $actionRenderStart
)
foreach ($requiredActionRowContract in @(
    'quick_launch_row_shell(theme, selected)',
    'quick-launch-action-row-',
    'quick-launch-action-icon-',
    'quick-launch-action-copy-',
    'quick-launch-action-title-',
    'quick-launch-action-trailing-',
    '.size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))',
    '.w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))',
    '.text_size(px(TEXT_BODY))',
    '.line_height(px(LINE_HEIGHT_BODY))'
)) {
    if (-not $actionRender.Contains($requiredActionRowContract)) {
        throw 'Quick Launch action rows must reuse the tokenized ordinary-result row rails.'
    }
}
if ($actionRender.Contains('ACTION_TEXT_SIZE') -or
    $actionRender.Contains('.h(px(36.0))') -or
    $actionRender.Contains('FontWeight::MEDIUM')) {
    throw 'Quick Launch action rows must not retain their private 36 px/medium presentation.'
}
if (-not $quickLaunchSource.Contains('const QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT: f32 = 24.0')) {
    throw 'Quick Launch action context height must have one named production constant.'
}
foreach ($requiredActionContextContract in @(
    'quick-launch-action-context',
    '.h(px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT))',
    '.text_size(px(TEXT_LABEL))',
    '.line_height(px(LINE_HEIGHT_LABEL))'
)) {
    if (-not $panelRender.Contains($requiredActionContextContract)) {
        throw 'Quick Launch action mode must use one lightweight Label context line.'
    }
}
if ($panelRender.Contains('.bg(theme.element)') -or
    -not $quickLaunchSource.Contains(
        'fn action_mode_reuses_the_result_row_rails_and_only_adds_a_lightweight_context('
    )) {
    throw 'Quick Launch QL-04 must remain the same work surface rather than a second styled popup.'
}
$exitActionModeStart = $quickLaunchSource.IndexOf('    fn exit_action_mode(')
$exitActionModeEnd = $quickLaunchSource.IndexOf(
    '    fn replace_results(',
    $exitActionModeStart
)
if ($exitActionModeStart -lt 0 -or $exitActionModeEnd -le $exitActionModeStart) {
    throw 'Could not isolate the Quick Launch action-mode return boundary.'
}
$exitActionMode = $quickLaunchSource.Substring(
    $exitActionModeStart,
    $exitActionModeEnd - $exitActionModeStart
)
if ($exitActionMode.Contains('base_handle') -or
    -not $exitActionMode.Contains('.action_mode') -or
    -not $exitActionMode.Contains('.take()') -or
    -not $quickLaunchSource.Contains(
        'fn action_mode_returns_to_the_parent_viewport_on_its_first_frame('
    )) {
    throw 'Quick Launch action-mode return must preserve the parent viewport on its first frame.'
}
$firstFrameTestStart = $quickLaunchSource.IndexOf(
    '    fn action_mode_returns_to_the_parent_viewport_on_its_first_frame('
)
$firstFrameTestEnd = $quickLaunchSource.IndexOf(
    '    #[gpui::test]',
    $firstFrameTestStart + 20
)
if ($firstFrameTestStart -lt 0 -or $firstFrameTestEnd -le $firstFrameTestStart) {
    throw 'Could not isolate the Quick Launch action-mode first-frame regression.'
}
$firstFrameTest = $quickLaunchSource.Substring(
    $firstFrameTestStart,
    $firstFrameTestEnd - $firstFrameTestStart
)
foreach ($requiredFirstFrameContract in @(
    'for exit_key in ["escape", "left", "backspace"]',
    '.expect("render the first returned parent frame")',
    'quick-launch-result-title-24',
    'parent_scroll_offset'
)) {
    if (-not $firstFrameTest.Contains($requiredFirstFrameContract)) {
        throw 'Every root action-mode exit must render the preserved lower viewport on its first frame.'
    }
}
$catalogRefreshStart = $quickLaunchSource.IndexOf('    fn finish_catalog_refresh(')
$catalogRefreshEnd = $quickLaunchSource.IndexOf(
    '    fn schedule_quick_launch_preview(',
    $catalogRefreshStart
)
if ($catalogRefreshStart -lt 0 -or $catalogRefreshEnd -le $catalogRefreshStart) {
    throw 'Could not isolate the Quick Launch catalog-refresh publication boundary.'
}
$catalogRefresh = $quickLaunchSource.Substring(
    $catalogRefreshStart,
    $catalogRefreshEnd - $catalogRefreshStart
)
if (-not $catalogRefresh.Contains('quick_launch_catalog_refresh_query(') -or
    -not $catalogRefresh.Contains('panel.action_mode.is_some()') -or
    -not $quickLaunchSource.Contains(
        'fn catalog_refresh_never_replaces_the_frozen_action_mode_parent('
    )) {
    throw 'Catalog refresh must not replace or recenter the frozen parent while actions are open.'
}
if (-not $quickLaunchSmokeSource.Contains('IsActionModeSurface()') -or
    -not $quickLaunchSmokeSource.Contains(
        "throw 'Quick-launch did not render the selected first action row before capture.'"
    )) {
    throw 'The Release smoke must prove action-mode chrome rendered before accepting its screenshot.'
}
if (-not $quickLaunchSmokeSource.Contains('[switch]$CapturePreview') -or
    -not $quickLaunchSmokeSource.Contains('TryGetPanelAndPreviewBounds') -or
    -not $quickLaunchSmokeSource.Contains(
        'Quick-launch did not retain its fixed preview window before capture.'
    )) {
    throw 'The Release smoke must capture a real retained preview window together with the panel.'
}
if (-not $quickLaunchSmokeSource.Contains('[switch]$ReuseExistingProcess') -or
    -not $quickLaunchSmokeSource.Contains('$ownsProcess = -not $ReuseExistingProcess') -or
    -not $quickLaunchSmokeSource.Contains(
        'if ($ownsProcess -and $process -and -not $process.HasExited)'
    )) {
    throw 'The Release smoke must never terminate a desktop process that it only borrowed.'
}
if (-not $quickLaunchSource.Contains('fn quick_launch_shortcut_target(') -or
    -not $quickLaunchSource.Contains('visible_result_range: Range<usize>') -or
    -not $quickLaunchSource.Contains(
        'fn production_heterogeneous_result_rows_keep_one_three_rail_geometry('
    ) -or
    -not $quickLaunchSource.Contains(
        'fn result_shortcuts_address_only_the_visible_first_nine_rows('
    )) {
    throw 'Quick Launch result shortcut rails require visible-range routing and production coverage.'
}
$previewRenderStart = $quickLaunchSource.IndexOf('impl Render for QuickLaunchPreview')
$previewRenderEnd = $quickLaunchSource.IndexOf(
    'pub(crate) enum QuickLaunchApplicationEvent',
    $previewRenderStart
)
if ($previewRenderStart -lt 0 -or $previewRenderEnd -le $previewRenderStart) {
    throw 'Could not isolate the production Quick Launch preview renderer.'
}
$previewRender = $quickLaunchSource.Substring(
    $previewRenderStart,
    $previewRenderEnd - $previewRenderStart
)
foreach ($requiredPreviewConstant in @(
    'const QUICK_LAUNCH_PREVIEW_WIDTH: f32 = 400.0',
    'const QUICK_LAUNCH_PREVIEW_HEIGHT: f32 = 410.0',
    'const QUICK_LAUNCH_PREVIEW_HEADER_HEIGHT: f32 = 72.0',
    'const QUICK_LAUNCH_PREVIEW_FOOTER_HEIGHT: f32 = 30.0'
)) {
    if (-not $quickLaunchSource.Contains($requiredPreviewConstant)) {
        throw 'Quick Launch preview must preserve its fixed production skeleton.'
    }
}
foreach ($requiredPreviewRenderContract in @(
    'preview_shell(theme)',
    'preview_media_surface(theme)',
    'quick-launch-preview-header',
    'quick-launch-preview-content',
    'quick-launch-preview-footer',
    'quick-launch-preview-text-copy',
    '.p(px(SPACING_8))',
    '.text_size(px(TEXT_BODY))',
    '.line_height(px(LINE_HEIGHT_BODY))',
    '.text_size(px(TEXT_LABEL))',
    '.line_height(px(LINE_HEIGHT_LABEL))',
    '.items_start()',
    '.justify_start()',
    'PREVIEW_APPEAR_DURATION'
)) {
    if (-not $previewRender.Contains($requiredPreviewRenderContract)) {
        throw 'Quick Launch preview must reuse the shared tokenized shell and media surface.'
    }
}
if ($previewRender.Contains('QUICK_LAUNCH_PREVIEW_TITLE_SIZE') -or
    $previewRender.Contains('QUICK_LAUNCH_PREVIEW_DETAIL_SIZE') -or
    $previewRender.Contains('QUICK_LAUNCH_PREVIEW_TIMESTAMP_SIZE') -or
    $previewRender.Contains('FontWeight::MEDIUM') -or
    $previewRender.Contains('if has_timestamps { 72.0 } else { 52.0 }')) {
    throw 'Quick Launch preview must not restore private type roles, cards or dynamic shell geometry.'
}
foreach ($requiredSharedPreviewContract in @(
    'pub(crate) const PREVIEW_APPEAR_DURATION: Duration = Duration::from_millis(130)',
    'pub(crate) fn preview_shell(theme: Theme) -> Div',
    '.border_color(theme.window_border)',
    'pub(crate) fn preview_media_surface(theme: Theme) -> Div',
    '.bg(theme.surface)',
    '.rounded(px(COMPACT_RADIUS))'
)) {
    if (-not $previewSource.Contains($requiredSharedPreviewContract)) {
        throw 'Clipboard and Quick Launch must keep the shared PreviewShell/media presentation seam.'
    }
}
$mediaHelperStart = $previewSource.IndexOf('pub(crate) fn preview_media_surface(')
$mediaHelperEnd = $previewSource.IndexOf(
    'pub(crate) struct PreviewLayout',
    $mediaHelperStart
)
if ($mediaHelperStart -lt 0 -or $mediaHelperEnd -le $mediaHelperStart) {
    throw 'Could not isolate the shared preview media surface.'
}
$mediaHelper = $previewSource.Substring(
    $mediaHelperStart,
    $mediaHelperEnd - $mediaHelperStart
)
if ($mediaHelper.Contains('.flex()') -or
    $mediaHelper.Contains('.items_center()') -or
    $mediaHelper.Contains('.justify_center()')) {
    throw 'The shared media surface must not impose image alignment on text preview content.'
}
$schedulePreviewStart = $quickLaunchSource.IndexOf('    fn schedule_quick_launch_preview(')
$schedulePreviewEnd = $quickLaunchSource.IndexOf(
    '    fn schedule_quick_launch_preview_candidate(',
    $schedulePreviewStart
)
if ($schedulePreviewStart -lt 0 -or $schedulePreviewEnd -le $schedulePreviewStart) {
    throw 'Could not isolate the Quick Launch preview target-switch boundary.'
}
$schedulePreview = $quickLaunchSource.Substring(
    $schedulePreviewStart,
    $schedulePreviewEnd - $schedulePreviewStart
)
if ($schedulePreview.Contains('animate_quick_launch_preview_exit') -or
    -not $schedulePreview.Contains('cancel_quick_launch_preview_exit(cx)') -or
    -not $quickLaunchSource.Contains(
        'fn production_quick_launch_preview_uses_shared_fixed_shell_and_media_surface('
    ) -or
    -not $quickLaunchSource.Contains(
        'a visible reused shell must not play an exit before replacement'
    ) -or
    -not $quickLaunchSource.Contains(
        'cancelling an old exit must not animate the old payload back in'
    ) -or
    -not $quickLaunchSource.Contains(
        'the interrupted old payload must render without an animation wrapper'
    ) -or
    -not $quickLaunchSource.Contains(
        'a stale stage must not publish into a replacement native window'
    ) -or
    -not $quickLaunchSource.Contains(
        'production_quick_launch_text_preview_starts_at_the_media_inset'
    )) {
    throw 'A visible Quick Launch preview switch must keep its shell and run one replacement entrance.'
}
$stagePreviewStart = $quickLaunchSource.IndexOf('    fn stage_reused_quick_launch_preview(')
$stagePreviewEnd = $quickLaunchSource.IndexOf('    fn request_icons(', $stagePreviewStart)
if ($stagePreviewStart -lt 0 -or $stagePreviewEnd -le $stagePreviewStart) {
    throw 'Could not isolate the Quick Launch preview staging lifecycle.'
}
$stagePreview = $quickLaunchSource.Substring(
    $stagePreviewStart,
    $stagePreviewEnd - $stagePreviewStart
)
foreach ($requiredStageIdentityContract in @(
    'preview.identity != stage.window_identity',
    'preview.identity != window_identity',
    'window_handle: stage.window_identity.native',
    'window_handle: window_identity.native'
)) {
    if (-not $stagePreview.Contains($requiredStageIdentityContract)) {
        throw 'Every Quick Launch stage and next-frame callback must freeze both GPUI and native window identity.'
    }
}
if (-not $quickLaunchSource.Contains('struct QuickLaunchPreviewWindowIdentity') -or
    -not $quickLaunchSource.Contains('gpui: AnyWindowHandle') -or
    -not $quickLaunchSource.Contains('native: i64') -or
    -not $quickLaunchSource.Contains('window_identity: QuickLaunchPreviewWindowIdentity')) {
    throw 'Every Quick Launch pending stage must carry both GPUI and native window identity.'
}
if (-not $previewRender.Contains('if motion_suppressed') -or
    -not $previewRender.Contains('preview.into_any_element()')) {
    throw 'Cancelling an interrupted exit must render the old payload without any animation wrapper.'
}
Write-Host 'PASS quick-launch preview keeps one shared fixed shell and one replacement entrance'
$desktopMainPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs'
$desktopMainSource = Get-Content -Raw -LiteralPath $desktopMainPath
$buildScriptPath = Join-Path $root 'scripts\build.ps1'
$buildScriptSource = Get-Content -Raw -LiteralPath $buildScriptPath
$manifestWriterPath = Join-Path $root 'scripts\write-release-binary-manifest.ps1'
$manifestWriterSource = Get-Content -Raw -LiteralPath $manifestWriterPath
$releaseManifestPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Generated\ReleaseBinaryManifest.g.cs'
$releaseManifestSource = Get-Content -Raw -LiteralPath $releaseManifestPath
$protocolPath = Join-Path $root 'rust\crates\nxtools-protocol\src\lib.rs'
$protocolSource = Get-Content -Raw -LiteralPath $protocolPath
$platformParserPath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformCommandParser.cs'
$platformParserSource = Get-Content -Raw -LiteralPath $platformParserPath
$platformPipePath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformPipeClient.cs'
$platformPipeSource = Get-Content -Raw -LiteralPath $platformPipePath
$windowsAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs'
$windowsAdapterSource = Get-Content -Raw -LiteralPath $windowsAdapterPath
$shellMenuPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ShellContextMenuSession.cs'
$shellMenuSource = Get-Content -Raw -LiteralPath $shellMenuPath
$shellIconPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ShellIconExtractor.cs'
$shellIconSource = Get-Content -Raw -LiteralPath $shellIconPath
$explorerAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ExplorerWindowAdapter.cs'
$explorerAdapterSource = if (Test-Path -LiteralPath $explorerAdapterPath) {
    Get-Content -Raw -LiteralPath $explorerAdapterPath
} else {
    ''
}
$fileDialogAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\FileDialogWindowAdapter.cs'
$fileDialogAdapterSource = if (Test-Path -LiteralPath $fileDialogAdapterPath) {
    Get-Content -Raw -LiteralPath $fileDialogAdapterPath
} else {
    ''
}
$desktopAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DesktopWindowAdapter.cs'
$desktopAdapterSource = if (Test-Path -LiteralPath $desktopAdapterPath) {
    Get-Content -Raw -LiteralPath $desktopAdapterPath
} else {
    ''
}
$directoryNavigationAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DirectoryNavigationAdapter.cs'
$directoryNavigationAdapterSource = if (Test-Path -LiteralPath $directoryNavigationAdapterPath) {
    Get-Content -Raw -LiteralPath $directoryNavigationAdapterPath
} else {
    ''
}
$thirdPartyAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ThirdPartyFileManagerAdapter.cs'
$thirdPartyAdapterSource = if (Test-Path -LiteralPath $thirdPartyAdapterPath) {
    Get-Content -Raw -LiteralPath $thirdPartyAdapterPath
} else {
    ''
}
$directoryOpusAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DirectoryOpusFileManagerOperations.cs'
$directoryOpusAdapterSource = if (Test-Path -LiteralPath $directoryOpusAdapterPath) {
    Get-Content -Raw -LiteralPath $directoryOpusAdapterPath
} else {
    ''
}
$totalCommanderAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\TotalCommanderFileManagerOperations.cs'
$totalCommanderAdapterSource = if (Test-Path -LiteralPath $totalCommanderAdapterPath) {
    Get-Content -Raw -LiteralPath $totalCommanderAdapterPath
} else {
    ''
}
$oneCommanderAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\OneCommanderFileManagerOperations.cs'
$oneCommanderAdapterSource = if (Test-Path -LiteralPath $oneCommanderAdapterPath) {
    Get-Content -Raw -LiteralPath $oneCommanderAdapterPath
} else {
    ''
}
$boundedStaRunnerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BoundedStaOperationRunner.cs'
$boundedStaRunnerSource = if (Test-Path -LiteralPath $boundedStaRunnerPath) {
    Get-Content -Raw -LiteralPath $boundedStaRunnerPath
} else {
    ''
}
$elevatedDirectoryNavigationPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ElevatedDirectoryNavigation.cs'
$elevatedDirectoryNavigationSource = if (Test-Path -LiteralPath $elevatedDirectoryNavigationPath) {
    Get-Content -Raw -LiteralPath $elevatedDirectoryNavigationPath
} else {
    ''
}
$elevatedHelperProcessBrokerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ElevatedHelperProcessBroker.cs'
$elevatedHelperProcessBrokerSource = if (Test-Path -LiteralPath $elevatedHelperProcessBrokerPath) {
    Get-Content -Raw -LiteralPath $elevatedHelperProcessBrokerPath
} else {
    ''
}
$indexerServiceInstallerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\IndexerServiceInstaller.cs'
$indexerServiceInstallerSource = if (Test-Path -LiteralPath $indexerServiceInstallerPath) {
    Get-Content -Raw -LiteralPath $indexerServiceInstallerPath
} else {
    ''
}
$indexerServiceHostPath = Join-Path $root 'rust\apps\nxtools-indexer\src\windows_service_host.rs'
$indexerServiceHostSource = if (Test-Path -LiteralPath $indexerServiceHostPath) {
    Get-Content -Raw -LiteralPath $indexerServiceHostPath
} else {
    ''
}
$indexerElevatedBrokerPath = Join-Path $root 'rust\apps\nxtools-indexer\src\elevated_helper_broker.rs'
$indexerElevatedBrokerSource = if (Test-Path -LiteralPath $indexerElevatedBrokerPath) {
    Get-Content -Raw -LiteralPath $indexerElevatedBrokerPath
} else {
    ''
}
$localPipePath = Join-Path $root 'rust\crates\nxtools-ipc\src\local_pipe.rs'
$localPipeSource = if (Test-Path -LiteralPath $localPipePath) {
    Get-Content -Raw -LiteralPath $localPipePath
} else {
    ''
}
$platformCapabilitiesPath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformCapabilities.cs'
$platformCapabilitiesSource = Get-Content -Raw -LiteralPath $platformCapabilitiesPath
$fileDialogSearchPath = Join-Path $root `
    'rust\apps\nxtools-desktop\src\quick_launch\file_dialog_search.rs'
$fileDialogSearchSource = if (Test-Path -LiteralPath $fileDialogSearchPath) {
    Get-Content -Raw -LiteralPath $fileDialogSearchPath
} else {
    ''
}
$autoQuickSwitchPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\AutoQuickSwitchTracker.cs'
$autoQuickSwitchSource = if (Test-Path -LiteralPath $autoQuickSwitchPath) {
    Get-Content -Raw -LiteralPath $autoQuickSwitchPath
} else {
    ''
}
$fileDialogRecognitionRetryPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\FileDialogRecognitionRetry.cs'
$fileDialogRecognitionRetrySource = if (
    Test-Path -LiteralPath $fileDialogRecognitionRetryPath
) {
    Get-Content -Raw -LiteralPath $fileDialogRecognitionRetryPath
} else {
    ''
}
$fileDialogSearchHostPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\FileDialogSearchHost.cs'
$fileDialogSearchHostSource = if (Test-Path -LiteralPath $fileDialogSearchHostPath) {
    Get-Content -Raw -LiteralPath $fileDialogSearchHostPath
} else {
    ''
}
$ipcPath = Join-Path $root 'rust\crates\nxtools-ipc\src\lib.rs'
$ipcSource = Get-Content -Raw -LiteralPath $ipcPath
$indexerPath = Join-Path $root 'rust\apps\nxtools-indexer\src\lib.rs'
$indexerSource = Get-Content -Raw -LiteralPath $indexerPath
$finderPath = Join-Path $root 'rust\crates\nxtools-finder\src\lib.rs'
$finderSource = Get-Content -Raw -LiteralPath $finderPath

if ($source -notmatch 'quick_launch:\s*Entity<QuickLaunchApplication>') {
    throw @'
Quick-launch lifecycle regression: PlatformEventDispatcher must strongly own
QuickLaunchApplication. A WeakEntity-only graph drops the module before the
platform Connected event, so configure_quick_launch is never sent and the
global summon gesture appears to do nothing.
'@
}

Write-Host 'PASS platform dispatcher strongly owns quick-launch application'

if ($quickLaunchSource -match '(?s)fn handle_foreground_changed\s*\(.*?self\.close\(cx\)') {
    throw @'
Quick-launch lease regression: a foreground callback runs while
QuickLaunchApplication is already leased. QuickLaunchPanel must return a
dismissal decision to the application; it must not update the application
again from inside the nested panel update.
'@
}

if ($quickLaunchSource -match '(?s)fn handle_pointer_pressed\s*\(.*?self\.close\(cx\)') {
    throw @'
Quick-launch lease regression: a global-pointer callback runs while
QuickLaunchApplication is already leased. QuickLaunchPanel must return a
dismissal decision to the application; it must not update the application
again from inside the nested panel update.
'@
}

Write-Host 'PASS platform dismissal callbacks do not re-enter quick-launch application'

if ($quickLaunchSource -match '(?s)fn activate_selected\s*\(.*?self\.application\.update\(cx') {
    throw @'
Quick-launch lease regression: Enter is dispatched while QuickLaunchPanel is
leased. Executing the selected result must be deferred before the application
is updated, because execution writes status back into the same panel.
'@
}

if ($quickLaunchSource -match '(?s)fn close\s*\(.*?self\.application\s*\.update\(cx') {
    throw @'
Quick-launch lease regression: Escape is dispatched while QuickLaunchPanel is
leased. Hiding the popup must be deferred before the application is updated,
because application-level hide writes state back into the same panel.
'@
}

Write-Host 'PASS panel keyboard callbacks defer application side effects'

$panelCloseStart = $quickLaunchSource.IndexOf('fn close(&mut self, cx: &mut Context<Self>)')
$panelDismissStart = $quickLaunchSource.IndexOf(
    'fn should_dismiss_for_foreground(',
    $panelCloseStart
)
if ($panelCloseStart -lt 0 -or $panelDismissStart -le $panelCloseStart) {
    throw 'Quick-launch explicit-dismiss regression: panel close lifecycle could not be located.'
}
$panelCloseBody = $quickLaunchSource.Substring(
    $panelCloseStart,
    $panelDismissStart - $panelCloseStart
)
$dismissPlanStart = $quickLaunchSource.IndexOf('fn quick_launch_dismiss_commands(')
$actionLevelStart = $quickLaunchSource.IndexOf(
    '#[derive(Clone)]',
    $dismissPlanStart
)
if ($dismissPlanStart -lt 0 -or $actionLevelStart -le $dismissPlanStart) {
    throw 'Quick-launch explicit-dismiss regression: native dismissal plan could not be located.'
}
$dismissPlanBody = $quickLaunchSource.Substring(
    $dismissPlanStart,
    $actionLevelStart - $dismissPlanStart
)
if ($panelCloseBody -notmatch 'target\.window_handle' -or
    $panelCloseBody -notmatch 'hide_popup_and_restore' -or
    $dismissPlanBody -notmatch 'SetWindowVisibility' -or
    $dismissPlanBody -notmatch 'SetWindowTopmost' -or
    $dismissPlanBody -notmatch 'ActivateWindow' -or
    $quickLaunchSource -notmatch '(?s)fn toggle_panel.*?hide_popup_and_restore' -or
    $quickLaunchSource -notmatch 'escape_hides_the_popup_then_restores_the_captured_host_for_the_next_summon') {
    throw @'
Quick-launch explicit-dismiss regression: Escape and repeated-hotkey cancellation
must hide/unpin the popup and then restore the top-level HWND captured for that
session. Otherwise the next double-Ctrl request captures the hidden GPUI window
or another non-host window and loses Explorer/file-dialog directory actions.
Passive external-focus dismissal and successful actions must keep using the
non-restoring hide path so they do not steal focus from the user's new target.
'@
}

Write-Host 'PASS explicit dismissal restores the captured host for the next summon'

$keyboardMoveStart = $quickLaunchSource.IndexOf('fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>)')
$pointerMoveStart = $quickLaunchSource.IndexOf('fn pointer_moved_result(')
if ($keyboardMoveStart -lt 0 -or $pointerMoveStart -le $keyboardMoveStart) {
    throw 'Quick-launch keyboard-preview regression: keyboard selection callback boundaries could not be located.'
}
$keyboardMoveBody = $quickLaunchSource.Substring(
    $keyboardMoveStart,
    $pointerMoveStart - $keyboardMoveStart
)
if ($keyboardMoveBody -notmatch 'schedule_keyboard_selection_preview') {
    throw @'
Quick-launch keyboard-preview regression: moving selection with Up/Down must
schedule the same delayed preview as pointer hover for the newly selected
result. Updating only the selected index leaves keyboard navigation without a
preview window.
'@
}

Write-Host 'PASS keyboard selection schedules the selected result preview'

if ($protocolSource -notmatch 'SetWindowNoActivate' -or
    $platformParserSource -notmatch 'SetWindowNoActivate' -or
    $windowsAdapterSource -notmatch 'WsExNoActivate' -or
    $windowsAdapterSource -notmatch 'SetWindowNoActivate' -or
    $quickLaunchSource -notmatch 'PlatformCommand::SetWindowNoActivate') {
    throw @'
Quick-launch focus regression: the owned preview is interactive, so
focus:false and SW_SHOWNA are insufficient after a pointer press. Rust must
explicitly mark only the preview HWND as WS_EX_NOACTIVATE through the C#
Windows adapter, while the search panel remains activatable.
'@
}

Write-Host 'PASS preview native window is permanently non-activating'

$newPreviewStart = $quickLaunchSource.IndexOf('fn show_quick_launch_preview(')
$reusePreviewStart = $quickLaunchSource.IndexOf('fn stage_reused_quick_launch_preview(')
if ($newPreviewStart -lt 0 -or $reusePreviewStart -le $newPreviewStart) {
    throw 'Quick-launch first-preview regression: new-preview callback boundaries could not be located.'
}
$newPreviewBody = $quickLaunchSource.Substring(
    $newPreviewStart,
    $reusePreviewStart - $newPreviewStart
)
$boundsCommandIndex = $newPreviewBody.IndexOf('PlatformCommand::SetWindowBounds')
$visibilityCommandIndex = $newPreviewBody.IndexOf('PlatformCommand::SetWindowVisibility')
if ($boundsCommandIndex -lt 0 -or
    $visibilityCommandIndex -lt 0 -or
    $boundsCommandIndex -gt $visibilityCommandIndex) {
    throw @'
Quick-launch first-preview regression: a hidden GPUI Windows popup still has
CW_USEDEFAULT native bounds until GPUI applies its deferred placement. The C#
adapter must atomically apply the requested device-pixel bounds before the
first visibility command, otherwise the preview flashes at a temporary large
size on its first appearance.
'@
}

Write-Host 'PASS first preview applies final native bounds before becoming visible'

if ($newPreviewBody -notmatch 'self\.restore_quick_launch_panel_keyboard_focus\(cx\)') {
    throw @'
Quick-launch keyboard-focus regression: opening a second GPUI window may
change GPUI's active keyboard target even when the preview HWND has
WS_EX_NOACTIVATE. After the first preview window is created, the application
must explicitly return activation and the input focus handle to the primary
search panel.
'@
}
$focusRestoreStart = $newPreviewBody.IndexOf('fn restore_quick_launch_panel_keyboard_focus(')
if ($focusRestoreStart -lt 0) {
    throw 'Quick-launch keyboard-focus regression: focus restoration helper could not be located.'
}
$focusRestoreBody = $newPreviewBody.Substring($focusRestoreStart)
if ($focusRestoreBody -notmatch 'PlatformCommand::ActivateWindow' -or
    $focusRestoreBody -notmatch 'focus_search\(window, cx\)') {
    throw @'
Quick-launch keyboard-focus regression: restoration must reassert the primary
HWND through the C# Windows activation adapter and restore GPUI's search input
focus handle. Either half alone leaves one keyboard-routing layer stale.
'@
}

Write-Host 'PASS first preview returns GPUI keyboard focus to the search panel'

if ($windowsAdapterSource -notmatch '(?s)public WindowsCommandResult ActivateWindow\(ActivateWindowCommand command\)\s*=>\s*NativeMethods\.ActivateWindow\(\s*new IntPtr\(command\.WindowHandle\),\s*new IntPtr\(command\.WindowHandle\)\)') {
    throw @'
Quick-launch native-focus regression: foreground activation alone is not
keyboard focus. The GPUI activation command must ask the Windows adapter to
focus the primary HWND as well as make it foreground, otherwise a newly
created preview can leave arrow keys routed away from the result list.
'@
}

if ($windowsAdapterSource -match 'focusWindow\s*!=\s*target\s*&&' -or
    $windowsAdapterSource -notmatch '(?s)GetFocus\(\)\s*!=\s*focusWindow.*?SetFocus\(focusWindow\)') {
    throw @'
Quick-launch native-focus regression: NativeMethods.ActivateWindow must allow
the target top-level HWND itself to receive keyboard focus. Treating
focusWindow == target as a reason to skip SetFocus reports a false activation
success while Up/Down no longer reaches GPUI.
'@
}

Write-Host 'PASS window activation restores native keyboard focus to the GPUI panel'

$renderStagedStart = $quickLaunchSource.IndexOf('fn render_staged_quick_launch_preview(')
$requestIconsStart = $quickLaunchSource.IndexOf('fn request_icons(', $renderStagedStart)
if ($renderStagedStart -lt 0 -or $requestIconsStart -le $renderStagedStart) {
    throw 'Quick-launch preview regression: staged-preview renderer boundaries could not be located.'
}
$renderStagedBody = $quickLaunchSource.Substring(
    $renderStagedStart,
    $requestIconsStart - $renderStagedStart
)
if ($renderStagedBody -match 'window\.resize\(') {
    throw @'
Quick-launch preview focus regression: GPUI 0.2.2 implements Windows
Window::resize with SetWindowPos(SWP_NOMOVE) and omits SWP_NOACTIVATE. The
preview has a fixed size, so resizing the reused native preview steals
foreground focus from the result panel after keyboard navigation.
'@
}

Write-Host 'PASS reused fixed-size preview never invokes GPUI Windows resize'

$requestIconsStart = $quickLaunchSource.IndexOf('fn request_icons(')
$recordIconFailureStart = $quickLaunchSource.IndexOf(
    'fn record_icon_failure(',
    $requestIconsStart
)
if ($requestIconsStart -lt 0 -or $recordIconFailureStart -le $requestIconsStart) {
    throw 'Quick-launch icon request boundaries could not be located.'
}
$requestIconsBody = $quickLaunchSource.Substring(
    $requestIconsStart,
    $recordIconFailureStart - $requestIconsStart
)
if ($requestIconsBody -notmatch 'cx\.defer\(' -or
    $requestIconsBody -notmatch '(?s)cx\.defer\(.*?popup\s*\.panel\s*\.update') {
    throw @'
Quick-launch cached-icon reentrancy regression: request_icons can be called while
QuickLaunchPanel::run_search already owns the panel entity. Applying cached
icons with a synchronous popup.panel.update then triggers GPUI
double_lease_panic and aborts NxTools. The panel update must be deferred until
the current GPUI entity lease has ended.
'@
}

Write-Host 'PASS cached search icons defer panel updates past the active GPUI lease'

$presentedIconsStart = $quickLaunchSource.IndexOf('fn update_presented_results(')
if ($pointerMoveStart -lt 0 -or $presentedIconsStart -le $pointerMoveStart) {
    throw 'Quick-launch pointer lifecycle regression: result-pointer callback boundaries could not be located.'
}
$pointerMoveBody = $quickLaunchSource.Substring(
    $pointerMoveStart,
    $presentedIconsStart - $pointerMoveStart
)
if ($pointerMoveBody -match 'self\.application\.update\(cx' -or
    $pointerMoveBody -notmatch 'schedule_application_update\(') {
    throw @'
Quick-launch pointer lease regression: a result-row mouse callback runs while
QuickLaunchPanel is already leased. Icon requests must be scheduled for the
next application turn; a synchronous application update can write cached
icons back into the same panel and terminate the process with double_lease_panic.
'@
}

Write-Host 'PASS result-pointer icon requests defer application side effects'

if ($quickLaunchSource -match '\.context_menu\s*\(') {
    throw @'
Quick-launch action-mode regression: result actions must render inside the
launcher state machine. A detached generic context menu changes focus,
keyboard routing, filtering and parent-state restoration.
'@
}

if ($quickLaunchSource -notmatch '(?s)MouseButton::Right.*?enter_action_mode' -or
    $quickLaunchSource -notmatch '(?s)key\.eq_ignore_ascii_case\("right"\).*?enter_action_mode') {
    throw @'
Quick-launch action-mode regression: result right-click and a bare Right key
at the query end must enter the same in-window action-mode transition.
'@
}

if ($quickLaunchSource -match '(?s)fn activate_selected_action\s*\(.*?self\.application\.update\(cx') {
    throw @'
Quick-launch action-mode lease regression: Enter in the action list is
dispatched while QuickLaunchPanel is leased. Execution must be deferred before
updating QuickLaunchApplication because it writes status back into the panel.
'@
}

Write-Host 'PASS result actions use the shared in-window action mode'

if ($protocolSource -notmatch 'DirectoryNavigationContext' -or
    $protocolSource -notmatch 'DirectoryNavigationHost' -or
    $protocolSource -notmatch 'NavigateDirectory' -or
    $quickLaunchSource -notmatch 'directory_navigation_context' -or
    $quickLaunchSource -notmatch 'QuickLaunchAction::SwitchDirectory' -or
    $quickLaunchSource -notmatch 'PlatformCommand::NavigateDirectory' -or
    $platformParserSource -notmatch 'ParseNavigateDirectory' -or
    $platformPipeSource -notmatch '"navigate_directory"\s*=>\s*platformAdapter\.NavigateDirectory' -or
    $windowsAdapterSource -notmatch 'DirectoryNavigationAdapter\.TryCaptureLocal\(target\)' -or
    $windowsAdapterSource -notmatch 'QuickLaunchDirectoryContextResolvedEvent' -or
    $windowsAdapterSource -notmatch 'DirectoryNavigationAdapter\.Navigate\(\s*command,' -or
    $directoryNavigationAdapterSource -notmatch 'ExplorerWindowAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogWindowAdapter\.TryCapture' -or
    $explorerAdapterSource -notmatch '9BA05972-F6A8-11CF-A442-00A0C90A8F39' -or
    $explorerAdapterSource -notmatch 'Navigate2' -or
    $quickLaunchSource -match 'Navigate2|9BA05972-F6A8-11CF-A442-00A0C90A8F39') {
    throw @'
Quick-launch Explorer-context regression: Rust must preserve the Explorer HWND
captured at summon and expose the context action, while only the C# Windows
adapter may enumerate Shell Windows and invoke Navigate2. Re-reading the live
foreground window at execution can redirect the action to the GPUI popup or a
different Explorer window.
'@
}

if ($explorerAdapterSource -notmatch 'IExplorerBrowserDispatch' -or
    $explorerAdapterSource -notmatch '\[DispId\(-515\)\]' -or
    $explorerAdapterSource -notmatch '\[DispId\(500\)\]' -or
    $explorerAdapterSource -match 'dynamic\)candidate\)\.HWND|dynamic\)window\)\.Navigate2') {
    throw @'
Quick-launch Explorer-COM regression: Windows 11 Shell Windows can return
0x8000FFFF for .NET dynamic name lookup of HWND. HWND and Navigate2 must use
the explicit IWebBrowser2 dispatch identifiers instead of dynamic member names.
'@
}

$explorerNavigateIndex = $explorerAdapterSource.IndexOf('operations.TryNavigate(')
$explorerActivateIndex = $explorerAdapterSource.IndexOf('operations.TryActivate(')
if ($explorerNavigateIndex -lt 0 -or
    $explorerActivateIndex -le $explorerNavigateIndex -or
    $explorerAdapterSource -notmatch 'WindowsPlatformAdapter\.TryActivateNativeWindow\(windowHandle\)') {
    throw @'
Quick-launch Explorer-repeat regression: a successful same-window Navigate2
must restore the captured Explorer HWND to the foreground before the command
completes. Otherwise the next summon captures a non-Explorer foreground and
the Switch Directory action disappears or cannot be dispatched a second time.
'@
}

if ($quickLaunchSource -notmatch 'popup_session_id' -or
    $quickLaunchSource -notmatch 'pending_action_targets_session' -or
    $quickLaunchSource -notmatch '(?s)fn show_existing_popup.*?popup_session_id\s*=\s*uuid::Uuid::new_v4' -or
    $quickLaunchSource -notmatch '(?s)fn hide_popup.*?popup_session_id\s*=\s*uuid::Uuid::new_v4') {
    throw @'
Quick-launch action-session regression: correlated Windows results can arrive
after the popup has been hidden and reopened. Pending actions must retain the
summon session id, and every show/hide transition must rotate it so an old
success cannot hide the new popup and an old failure cannot overwrite it.
'@
}

Write-Host 'PASS directory navigation preserves Explorer summon context, foreground and the Rust/C# boundary'

if ($protocolSource -notmatch 'FileDialog' -or
    $protocolSource -notmatch 'navigation_directory' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogHost' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogWindowAdapter\.Navigate' -or
    $quickLaunchSource -notmatch 'DirectoryNavigationHost::FileDialog' -or
    $quickLaunchSource -notmatch '(?s)menu\s*\.as_ref\(\).*?navigation_directory\.clone\(\).*?Path::new\(directory\)\.is_absolute\(\)' -or
    $quickLaunchSource -match 'fn directory_navigation_target' -or
    $shellMenuSource -notmatch 'ShellIconExtractor\.ResolveNavigationDirectory\(target\)' -or
    $shellMenuSource -notmatch 'navigationDirectory' -or
    $shellIconSource -notmatch 'ResolveNavigationDirectory' -or
    $shellIconSource -notmatch 'TryResolveShortcut' -or
    $shellIconSource -notmatch 'ResolveSystemToolPath' -or
    $quickLaunchSource -notmatch 'pending_action_owns_foreground' -or
    $quickLaunchSource -notmatch 'begin_window_activation\(popup\.native_window_handle\)' -or
    $fileDialogAdapterSource -notmatch '"#32770"' -or
    $fileDialogAdapterSource -notmatch '"Breadcrumb Parent"' -or
    $fileDialogAdapterSource -notmatch 'FileNameEditId' -or
    $fileDialogAdapterSource -notmatch 'hasFileTypeComboBox\s*\|=\s*controlId\s*==\s*FileTypeComboId' -or
    $fileDialogAdapterSource -notmatch 'WindowsPlatformAdapter\.TryActivateNativeWindow' -or
    $fileDialogAdapterSource -notmatch 'ResolveNavigationEdit' -or
    $fileDialogAdapterSource -notmatch 'SendInput' -or
    $fileDialogAdapterSource -notmatch 'GetGUIThreadInfo' -or
    $fileDialogAdapterSource -notmatch 'PostMessage\(navigationEdit, WmKeyDown' -or
    $fileDialogAdapterSource -notmatch 'WaitForDirectory' -or
    $fileDialogAdapterSource -notmatch 'CdmGetFolderPath' -or
    $fileDialogAdapterSource -notmatch 'ReadProcessMemory' -or
    $quickLaunchSource -notmatch 'navigation_guard_owns_foreground' -or
    $quickLaunchSource -notmatch 'FAILED_NAVIGATION_FOREGROUND_GUARD' -or
    $quickLaunchSource -match '#32770|Breadcrumb Parent|WmSetText') {
    throw @'
Quick-launch file-dialog regression: the summon request must freeze a strictly
recognized native file-dialog HWND, Rust must expose directory navigation only
through a FileDialog-host action, and C# must validate/write/focus/submit that
same dialog. GPUI/Rust may not own #32770 control inspection or Win32 messages.
'@
}

$fileDialogPathCaptureStart = $fileDialogAdapterSource.IndexOf('private static string? TryReadCurrentDirectory(')
$fileDialogClassReadStart = $fileDialogAdapterSource.IndexOf('private static string GetWindowClass(')
if ($fileDialogPathCaptureStart -lt 0 -or
    $fileDialogClassReadStart -le $fileDialogPathCaptureStart) {
    throw 'Quick-launch file-dialog hotkey regression: bounded path-capture helpers could not be located.'
}
$fileDialogPathCaptureBody = $fileDialogAdapterSource.Substring(
    $fileDialogPathCaptureStart,
    $fileDialogClassReadStart - $fileDialogPathCaptureStart
)
if ($fileDialogPathCaptureBody -match 'Directory\.Exists|File\.Exists') {
    throw @'
Quick-launch file-dialog hotkey regression: reading the dialog breadcrumb may
parse a syntactically absolute path but must not touch the filesystem. An
offline UNC path can otherwise block the double-Ctrl summon past its 60 ms
target even though the window message itself has a bounded timeout.
'@
}

Write-Host 'PASS native file-dialog navigation preserves host identity and the Rust/C# boundary'

if ($protocolSource -notmatch '(?s)DirectoryNavigationHost\s*\{.*?Desktop' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopHost' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopWindowAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopWindowAdapter\.Navigate' -or
    $desktopAdapterSource -notmatch 'GetShellWindow' -or
    $desktopAdapterSource -notmatch '"Progman"' -or
    $desktopAdapterSource -notmatch '"WorkerW"' -or
    $desktopAdapterSource -notmatch '"SHELLDLL_DefView"' -or
    $desktopAdapterSource -notmatch 'GetWindowThreadProcessId' -or
    $desktopAdapterSource -notmatch 'ProcessName\.Equals\("explorer"' -or
    $desktopAdapterSource -notmatch 'IsShellDesktopIdentity' -or
    $desktopAdapterSource -notmatch 'Process\.Start' -or
    $desktopAdapterSource -match 'Navigate2|ShellWindowsClassId' -or
    $quickLaunchSource -notmatch 'recent_directory_mode' -or
    $quickLaunchSource -notmatch 'recent_directory_navigation_action' -or
    $quickLaunchSource -notmatch 'schedule_captured_directory_record' -or
    $quickLaunchSource -notmatch 'schedule_directory_record' -or
    $quickLaunchSource -notmatch 'FinderCatalog::persist_directory_visit' -or
    $quickLaunchSource -notmatch 'apply_recorded_directory_visit' -or
    $quickLaunchSource -notmatch 'captured_directory_reaches_sqlite_after_the_popup_session_changes' -or
    $quickLaunchSource -notmatch '(?s)fn schedule_directory_record.*?background_executor\(\).*?FinderCatalog::persist_directory_visit' -or
    $quickLaunchSource -notmatch '(?s)fn update_presented_results.*?self\.recent_directory_mode' -or
    $finderSource -notmatch 'quick_launch_directory_history' -or
    $finderSource -notmatch 'MAX_RECENT_DIRECTORIES' -or
    $finderSource -notmatch 'pub fn recent_directories' -or
    $finderSource -notmatch 'pub fn record_directory_visit' -or
    $finderSource -notmatch 'lexical_absolute_path' -or
    $desktopAdapterSource -match 'quick_launch_directory_history') {
    throw @'
Quick-launch desktop/recent-directory regression: C# must recognize only the
real Shell desktop and open its target through Shell rather than Explorer
Navigate2. Rust must own the bounded, persistent, deduplicated directory
history and GPUI must expose it only through an explicit recent-directory mode.
'@
}

Write-Host 'PASS desktop context and recent directories preserve host and ownership boundaries'

if ($protocolSource -notmatch '(?s)DirectoryNavigationHost\s*\{.*?ThirdPartyFileManager' -or
    $protocolSource -notmatch '(?s)DirectoryNavigationContext\s*\{.*?host_context_id' -or
    $protocolSource -notmatch '(?s)NavigateDirectoryCommand\s*\{.*?host_context_id' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerHost' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerAdapter\.Navigate' -or
    $thirdPartyAdapterSource -notmatch 'IThirdPartyFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'ThirdPartyFileManagerSessionRegistry' -or
    $thirdPartyAdapterSource -notmatch 'Guid\.NewGuid\(\)' -or
    $thirdPartyAdapterSource -notmatch 'HostContextId' -or
    $thirdPartyAdapterSource -notmatch 'DirectoryOpusFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'TotalCommanderFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'OneCommanderFileManagerOperations' -or
    $directoryOpusAdapterSource -notmatch '"dopus\.lister"' -or
    $directoryOpusAdapterSource -notmatch '"dopus\.filedisplaycontainer"' -or
    $directoryOpusAdapterSource -notmatch 'SendMessageTimeout' -or
    $totalCommanderAdapterSource -notmatch '"TTOTAL_CMD"' -or
    $totalCommanderAdapterSource -notmatch 'GetRequestWide' -or
    $totalCommanderAdapterSource -notmatch 'ChangeDirectory' -or
    $totalCommanderAdapterSource -notmatch 'SendMessageTimeout' -or
    $oneCommanderAdapterSource -notmatch '"CurrentPathGet"' -or
    $oneCommanderAdapterSource -notmatch '"CurrentPathSet"' -or
    $oneCommanderAdapterSource -notmatch 'focusWindowHandle' -or
    $oneCommanderAdapterSource -notmatch 'AutomationElement\.FromHandle' -or
    $oneCommanderAdapterSource -match '(?s)CapturePane\(.*?AutomationElement\.FocusedElement.*?private static OneCommanderNavigationResult' -or
    $oneCommanderAdapterSource -notmatch 'BoundedStaOperationRunner\.TryRun' -or
    $boundedStaRunnerSource -notmatch 'completed\.Wait\(timeout\)' -or
    $boundedStaRunnerSource -notmatch 'cancellation\.Cancel\(\)' -or
    $boundedStaRunnerSource -notmatch 'ActiveOperations' -or
    $thirdPartyAdapterSource -notmatch '_sessionOrderNodes' -or
    $thirdPartyAdapterSource -notmatch 'RemoveSession' -or
    $directoryOpusAdapterSource -match 'containers\[0\]' -or
    $quickLaunchSource -notmatch 'DirectoryNavigationHost::ThirdPartyFileManager' -or
    $quickLaunchSource -notmatch 'host_context_id' -or
    $quickLaunchSource -match 'DirectoryOpus|TotalCommander|OneCommander') {
    throw @'
Quick-launch third-party file-manager regression: Rust must expose one generic
third_party_file_manager host while C# owns strict Directory Opus, Total
Commander and OneCommander recognition, request-scoped active-pane capture and
bounded native/UIA navigation. The opaque pane token must survive Rust
orchestration unchanged, and adapters must never fall back to the first pane.
Product-specific Windows details must not leak into Rust/GPUI.
'@
}

Write-Host 'PASS third-party file managers preserve active panes and the Rust/C# boundary'

if ($protocolSource -notmatch 'ElevatedDirectoryHelperRequested' -or
    $protocolSource -notmatch 'EnsureElevatedDirectoryHelper' -or
    $protocolSource -notmatch 'QuickLaunchDirectoryContextResolved' -or
    $quickLaunchSource -notmatch 'elevated_directory_helper_command' -or
    $quickLaunchSource -notmatch 'acknowledged_elevated_helper_pipe' -or
    $quickLaunchSource -notmatch 'elevated_helper_status_matches_request' -or
    $quickLaunchSource -notmatch 'directory_context_resolution_matches' -or
    $quickLaunchSource -notmatch 'REQUEST_MAX_AGE_MS' -or
    $windowsAdapterSource -notmatch 'QuickLaunchDirectoryContextResolvedEvent' -or
    $windowsAdapterSource -notmatch 'IsQuickLaunchContextRequestCurrent' -or
    $windowsAdapterSource -notmatch 'Task\.Run' -or
    $directoryNavigationAdapterSource -notmatch 'ElevatedDirectoryNavigationClient\.IsElevatedContext' -or
    $directoryNavigationAdapterSource -notmatch 'RequiresElevatedHelper' -or
    $elevatedDirectoryNavigationSource -notmatch 'PipeOptions\.CurrentUserOnly' -or
    $elevatedDirectoryNavigationSource -notmatch 'RandomNumberGenerator\.GetBytes\(32\)' -or
    $elevatedDirectoryNavigationSource -notmatch 'GetNamedPipeClientProcessId' -or
    $elevatedDirectoryNavigationSource -notmatch 'ProcessIdToSessionId' -or
    $elevatedDirectoryNavigationSource -notmatch 'TokenElevation' -or
    $elevatedDirectoryNavigationSource -notmatch 'ElevatedDirectoryNavigationSessionRegistry' -or
    $elevatedDirectoryNavigationSource -notmatch 'BoundedTextFrameReader' -or
    $elevatedDirectoryNavigationSource -notmatch 'MaximumRendezvousAttempts' -or
    $elevatedDirectoryNavigationSource -notmatch '_sessionOrderNodes' -or
    $elevatedDirectoryNavigationSource -notmatch 'foregroundProvider\.Capture' -or
    $elevatedDirectoryNavigationSource -notmatch 'PreserveRequestedFocus' -or
    $elevatedDirectoryNavigationSource -notmatch 'terminateAfterResponse' -or
    $elevatedDirectoryNavigationSource -notmatch 'response\.Fatal' -or
    $explorerAdapterSource -notmatch 'BoundedStaOperationRunner\.TryRun' -or
    $explorerAdapterSource -notmatch 'Fatal: true' -or
    $explorerAdapterSource -notmatch 'cancellationToken\.ThrowIfCancellationRequested' -or
    $elevatedHelperProcessBrokerSource -notmatch 'WTSQueryUserToken' -or
    $elevatedHelperProcessBrokerSource -notmatch 'CreateProcessAsUser' -or
    $elevatedHelperProcessBrokerSource -notmatch 'winsta0\\default' -or
    $localPipeSource -notmatch 'GetNamedPipeClientProcessId' -or
    $localPipeSource -notmatch 'client_session_id' -or
    $indexerServiceHostSource -notmatch 'client_process_id' -or
    $indexerServiceHostSource -notmatch 'client_session_id' -or
    $indexerElevatedBrokerSource -notmatch 'authorized_packaged_desktop' -or
    $indexerElevatedBrokerSource -notmatch 'require_packaged_desktop' -or
    $indexerElevatedBrokerSource -notmatch 'registered_hash_matches' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsDesktopSha256' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsDesktopPath' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsPlatformHostSha256' -or
    $indexerElevatedBrokerSource -notmatch 'packaged_platform_host_path' -or
    $indexerElevatedBrokerSource -notmatch 'BROKER_TIMEOUT' -or
    $indexerElevatedBrokerSource -notmatch 'try_wait\(\)' -or
    $indexerServiceInstallerSource -notmatch 'DesktopHashValueName' -or
    $indexerServiceInstallerSource -notmatch 'PlatformHostHashValueName' -or
    $indexerServiceInstallerSource -notmatch 'DesktopPathValueName' -or
    $indexerServiceInstallerSource -notmatch 'ProtectedVendorDirectory' -or
    $indexerServiceInstallerSource -notmatch 'ProtectedServiceDirectory' -or
    $indexerServiceInstallerSource -notmatch 'HardenServiceDirectory' -or
    $indexerServiceInstallerSource -notmatch 'TrustedBinary\.Open' -or
    $indexerServiceInstallerSource -notmatch 'ReleaseBinaryManifest\.IndexerSha256' -or
    $indexerServiceInstallerSource -notmatch 'ReleaseBinaryManifest\.DesktopSha256' -or
    $indexerServiceInstallerSource -notmatch 'PinCurrentProcessImage' -or
    $indexerServiceInstallerSource -notmatch 'DeploymentSourceMatchesDestination' -or
    $indexerServiceInstallerSource -notmatch 'FileFlagOpenReparsePoint' -or
    $indexerServiceInstallerSource -notmatch 'GetNamedSecurityInfo' -or
    $indexerServiceInstallerSource -notmatch '"/reset"' -or
    $indexerServiceInstallerSource -notmatch 'icacls\.exe' -or
    $desktopMainSource -notmatch 'launch_protected_desktop_after_current_exit' -or
    $desktopMainSource -notmatch 'protected_package_matches_release' -or
    $desktopMainSource -notmatch 'NxTools\.Indexer\.exe' -or
    $desktopMainSource -notmatch 'NxTools\.PlatformHost\.exe' -or
    $oneCommanderAdapterSource -notmatch 'OneCommanderFocusTracker' -or
    $oneCommanderAdapterSource -notmatch 'AutomationElement\.FocusedElement' -or
    $oneCommanderAdapterSource -notmatch 'Fatal: true' -or
    $elevatedDirectoryNavigationSource -notmatch 'capture\.Failure\?\.Fatal' -or
    $releaseManifestSource -notmatch 'IndexerSha256' -or
    $releaseManifestSource -notmatch 'DesktopSha256' -or
    $manifestWriterSource -notmatch 'Get-FileHash' -or
    $buildScriptSource -notmatch 'write-release-binary-manifest\.ps1' -or
    $elevatedDirectoryNavigationSource -notmatch 'IsApprovedProtectedPlatformHost' -or
    $indexerElevatedBrokerSource -match 'runas') {
    throw @'
Quick-launch elevated-navigation regression: the medium-integrity Rust/GPUI
desktop must remain unelevated. A random current-user pipe and 256-bit token
must rendezvous with a fixed packaged C# helper launched by the LocalSystem
service in the actual client session. Both service and platform host must
authenticate real pipe-client identity, high-integrity capture must happen in
the helper, elevated contexts must remain opaque one-shot tokens, and all
cross-process waits must be bounded. No caller-controlled executable or
per-action runas fallback is allowed.
'@
}

Write-Host 'PASS elevated navigation preserves session identity, least privilege and bounded waits'

if ($platformCapabilitiesSource -notmatch 'directory_navigation' -or
    $ipcSource -notmatch 'directory_navigation' -or
    $platformCapabilitiesSource -match 'explorer_navigation|file_dialog_navigation' -or
    $ipcSource -match 'explorer_navigation|file_dialog_navigation') {
    throw @'
Quick-launch directory-navigation capability regression: Explorer, native
file dialogs and future third-party adapters share one tagged directory
navigation contract. Both endpoints must advertise that single capability;
host-specific wire capabilities would force Rust to know every Windows host.
'@
}

Write-Host 'PASS directory navigation uses one extensible tagged capability'

if ($protocolSource -notmatch 'QuickLaunchAutoSwitchRequested' -or
    $quickLaunchSource -notmatch 'auto_quick_switch_command' -or
    $autoQuickSwitchSource -notmatch 'KnownDialogLimit' -or
    $autoQuickSwitchSource -notmatch 'IsCurrent' -or
    $fileDialogRecognitionRetrySource -notmatch 'MaxAttempts|_maxAttempts' -or
    $windowsAdapterSource -notmatch 'FileDialogRecognitionTimerId' -or
    $windowsAdapterSource -notmatch 'IsPotentialHost' -or
    $windowsAdapterSource -notmatch 'RetryFileDialogRecognition' -or
    $windowsAdapterSource -notmatch 'TryGetCurrentDirectory' -or
    $windowsAdapterSource -notmatch 'QuickLaunchAutoSwitchRequestedEvent') {
    throw @'
Quick-launch automatic-switch regression: the Windows adapter must recognize
a previously known file-dialog -> Explorer -> same-dialog round trip, freeze
the Explorer path off the foreground hook, preserve that source while a newly
created dialog finishes initializing, and publish a generation-scoped request.
Rust must retain settings/freshness validation and issue the shared directory
navigation command instead of letting C# navigate implicitly.
'@
}

Write-Host 'PASS automatic quick switch preserves the Rust/C# ownership boundary'

if ($protocolSource -notmatch 'QuickLaunchFileDialogSearchHostChanged' -or
    $protocolSource -notmatch 'OpenFileInDialog' -or
    $platformCapabilitiesSource -notmatch 'file_dialog_search' -or
    $fileDialogSearchSource -notmatch 'WindowKind::PopUp' -or
    $fileDialogSearchSource -notmatch 'SetWindowOwner' -or
    $fileDialogSearchSource -notmatch 'IndexSearchContext' -or
    $fileDialogSearchSource -notmatch 'preferred_directory' -or
    $fileDialogSearchSource -notmatch 'file_dialog_default_source' -or
    $fileDialogSearchSource -notmatch 'Ctrl\+G' -or
    $fileDialogSearchSource -notmatch 'OpenFileInDialog' -or
    $fileDialogSearchHostSource -notmatch 'GetWindowRect' -or
    $fileDialogSearchHostSource -notmatch 'DwmGetWindowAttribute' -or
    $fileDialogSearchHostSource -notmatch 'DwmExtendedFrameBounds' -or
    $windowsAdapterSource -notmatch 'EventObjectCreate' -or
    $windowsAdapterSource -notmatch 'EventObjectShow' -or
    $windowsAdapterSource -notmatch 'ThirdPartyFileManagerHost' -or
    $windowsAdapterSource -notmatch 'EventObjectLocationChange') {
    throw @'
Quick-launch file-dialog search regression: C# may publish only exact HWND,
visible DWM placement/DPI/current-directory/latest-source facts and execute
native commands. The attached, reusable search surface, global query with
preferred-directory ranking, default Ctrl+G row and all visible rendering must
remain in Rust/GPUI, owned by the captured dialog. Child create/show events must
re-arm delayed recognition instead of waiting for a foreground round trip.
'@
}

Write-Host 'PASS file-dialog search remains a GPUI surface with exact native hosting'

if ($protocolSource -notmatch 'QueryShellActions' -or
    $protocolSource -notmatch 'InvokeShellAction' -or
    $platformParserSource -notmatch 'QueryShellActions' -or
    $platformParserSource -notmatch 'InvokeShellAction' -or
    -not (Test-Path -LiteralPath $shellMenuPath) -or
    $quickLaunchSource -notmatch 'QuickLaunchAction::Shell') {
    throw @'
Quick-launch action completeness regression: Listary-style action mode must
append the selected target's dynamic Windows Shell commands after first-class
actions. A fixed four-item array omits application-specific verbs and Shell
submenus, so the right-click action list is visibly incomplete.
'@
}

Write-Host 'PASS result actions append dynamic Windows Shell commands'

if ($protocolSource -notmatch 'QueryShellSubmenu' -or
    $protocolSource -notmatch 'icon_path' -or
    $protocolSource -notmatch 'submenu_id' -or
    $platformParserSource -notmatch 'QueryShellSubmenu' -or
    $shellMenuSource -notmatch 'IconPath' -or
    $shellMenuSource -notmatch 'SubmenuId' -or
    $shellMenuSource -notmatch 'ShellMenuIconStager\.Stage\(info\.ItemBitmap\)' -or
    $quickLaunchSource -notmatch 'icon_path' -or
    $shellMenuSource -match 'EnumerateMenu\(info\.SubMenu') {
    throw @'
Quick-launch Shell presentation regression: Listary reads each root item's
hbmpItem and enumerates a submenu only when the user enters that submenu.
The platform protocol must carry staged operation icons and an opaque submenu
id; eagerly walking every submenu makes the first action list slow and leaving
the icon out produces the empty left gutter visible in the product.
'@
}

Write-Host 'PASS Shell action icons are real and submenu enumeration is lazy'

if ($quickLaunchSource -match '正在交给 Windows 打开' -or
    $quickLaunchSource -match 'fn set_launching\s*\(') {
    throw @'
Quick-launch feedback regression: successful dispatch must not add a persistent
"opening" strip to the popup. Only actionable failures may use the compact
error overlay; successful execution is confirmed by the target application.
'@
}

Write-Host 'PASS successful quick-launch dispatch does not paint a status strip'

$actionDispatchStart = $quickLaunchSource.IndexOf('    fn execute(')
$togglePanelStart = $quickLaunchSource.IndexOf('    fn toggle_panel(', $actionDispatchStart)
$commandResultStart = $quickLaunchSource.IndexOf('    fn handle_command_result(')
$applyShellActionsStart = $quickLaunchSource.IndexOf('    fn apply_shell_actions(', $commandResultStart)
if ($actionDispatchStart -lt 0 -or $togglePanelStart -le $actionDispatchStart -or
    $commandResultStart -lt 0 -or $applyShellActionsStart -le $commandResultStart) {
    throw 'Quick-launch action diagnostic boundaries could not be located.'
}
$actionDispatchBody = $quickLaunchSource.Substring(
    $actionDispatchStart,
    $togglePanelStart - $actionDispatchStart
)
$commandResultBody = $quickLaunchSource.Substring(
    $commandResultStart,
    $applyShellActionsStart - $commandResultStart
)
if ($actionDispatchBody -notmatch 'quick_launch\.action\.accepted' -or
    $actionDispatchBody -notmatch 'quick_launch\.action\.rejected' -or
    $actionDispatchBody -notmatch 'send_command_correlated\(command, action_correlation_id\)' -or
    ([regex]::Matches(
        $actionDispatchBody,
        'correlation_id\s*=\s*%action_correlation_id'
    )).Count -lt 3 -or
    $actionDispatchBody -notmatch 'trigger\.diagnostic_label\(\)' -or
    $commandResultBody -notmatch 'quick_launch\.action\.completed' -or
    $commandResultBody -notmatch 'correlation_id\s*=\s*%action_correlation_id' -or
    $commandResultBody -notmatch 'elapsed_ms' -or
    $commandResultBody -notmatch 'native_error_code' -or
    $commandResultBody -notmatch 'outcome\s*=\s*"success"' -or
    $commandResultBody -notmatch 'outcome\s*=\s*"failure"') {
    throw @'
Quick-launch action diagnostic regression: dispatch must record accepted or
rejected intent with its trigger, then terminal success/failure must retain the
same IPC correlation id plus elapsed time and the native Windows error code.
'@
}

Write-Host 'PASS quick-launch actions preserve correlated terminal diagnostics'

$statusRenderStart = $quickLaunchSource.IndexOf('.when_some(self.status.clone(), |panel, status|')
$sourceIconStart = $quickLaunchSource.IndexOf('fn source_icon(')
if ($statusRenderStart -lt 0 -or $sourceIconStart -le $statusRenderStart) {
    throw 'Quick-launch layout regression: the status rendering block could not be located.'
}
$statusRender = $quickLaunchSource.Substring(
    $statusRenderStart,
    $sourceIconStart - $statusRenderStart
)
if ($statusRender -notmatch '\.absolute\(\)') {
    throw @'
Quick-launch layout regression: transient launch/error status must be an
absolute overlay. An in-flow status row steals height from the result list as
soon as an item is clicked, making the panel appear to shrink.
'@
}

Write-Host 'PASS actionable error status does not change quick-launch layout height'

if ($quickLaunchSource -notmatch '(?s)background\s*\.spawn\(async move \{ scan_application_roots\(&roots\) \}\)') {
    throw @'
Quick-launch refresh regression: filesystem catalog scanning must run on the
background executor. Running it inline blocks popup activation and typing.
'@
}

if ($quickLaunchSource -notmatch '(?s)fn finish_catalog_refresh.*?search_input\.read\(cx\).*?replace_results') {
    throw @'
Quick-launch refresh regression: a completed snapshot must rerun only the
query currently visible in the popup and replace that result set in place.
'@
}

Write-Host 'PASS application refresh stays off the UI thread and reapplies the current query'

if ($quickLaunchSource -notmatch 'fn update_presented_results' -or
    $quickLaunchSource -notmatch '(?s)uniform_list\(.*?visible_range.*?update_presented_results\(visible_range') {
    throw @'
Quick-launch icon-loading regression: every result row that becomes visible
through initial layout, wheel scrolling, scrollbar dragging or keyboard
navigation must request its Shell icon without waiting for pointer hover. The
virtual list's rendered range is the presentation boundary for all four paths.
'@
}

Write-Host 'PASS visible quick-launch rows request icons without pointer hover'

if ($iconsSource -notmatch 'fn shell_icon_path_is_renderable' -or
    $iconsSource -notmatch 'imagesize::size\(path\)' -or
    $quickLaunchSource -notmatch 'use crate::icons::\{MaterialIcon, shell_icon_path_is_renderable\}' -or
    $quickLaunchSource -notmatch '\.filter\(\|path\| shell_icon_path_is_renderable\(path\)\)' -or
    $quickLaunchSource -notmatch '\.with_loading\(' -or
    $quickLaunchSource -notmatch '\.with_fallback\(' -or
    $quickLaunchSource -notmatch 'quick_launch_result_fallback_icon') {
    throw @'
Quick-launch icon-visibility regression: an existing path is not enough to
replace the source icon. Platform results must be validated as renderable PNGs,
and loading or decode failure must retain a visible semantic fallback.
'@
}

Write-Host 'PASS invalid or loading Shell images retain visible result icons'

if ($shellIconSource -notmatch 'ShgfiUseFileAttributes' -or
    $shellIconSource -notmatch 'FileAttributeNormal' -or
    $shellIconSource -notmatch 'Path\.GetExtension') {
    throw @'
Quick-launch file-type icon regression: stale indexed paths must fall back to
the Windows icon registered for their extension. Requiring File.Exists leaves
versioned application/cache results blank even though the row is presented.
'@
}

Write-Host 'PASS stale file paths retain their Windows file-type icon'

if ($quickLaunchSource -notmatch 'SEARCH_INPUT_DEBOUNCE' -or
    $quickLaunchSource -notmatch 'query_requests_file_index' -or
    $indexerSource -notmatch 'struct SearchCandidate' -or
    $indexerSource -notmatch 'base_name_mask' -or
    $indexerSource -notmatch 'name_mask_may_match' -or
    $indexerSource -notmatch 'score_name' -or
    $indexerSource -notmatch 'normalized_path_separators') {
    throw @'
Quick-launch query-efficiency regression: rapid input must be coalesced, every
non-empty query must retain full indexed-file semantics, normal name searches
must use the compact mask/name prefilter before materializing cold paths, and
the scan must retain borrowed candidates instead of allocating owned paths for
every match.
'@
}

Write-Host 'PASS quick-launch input and MFT candidate work stay bounded'
