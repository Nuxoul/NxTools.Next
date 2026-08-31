$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$mainWindowPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs'
$settingsPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\settings.rs'
$mainWindowSource = Get-Content -Raw -LiteralPath $mainWindowPath
$settingsSource = Get-Content -Raw -LiteralPath $settingsPath

$splitSurfaceStart = $settingsSource.IndexOf('pub(crate) fn settings_split_surface(')
$splitSurfaceEnd = $settingsSource.IndexOf('pub(crate) fn settings_shell(', $splitSurfaceStart)
if ($splitSurfaceStart -lt 0 -or $splitSurfaceEnd -le $splitSurfaceStart) {
    throw 'Could not isolate the shared continuous split-settings surface.'
}
$splitSurface = $settingsSource.Substring(
    $splitSurfaceStart,
    $splitSurfaceEnd - $splitSurfaceStart
)
foreach ($requiredSplitContract in @(
    'SettingsSplitSurfaceIds',
    '.border_1()',
    '.border_color(theme.border_subtle)',
    '.bg(theme.surface)',
    '.flex_1()',
    '.min_w_0()',
    '.w(px(1.0))',
    '.w(px(secondary_width))',
    '.px(px(SPACING_12))'
)) {
    if (-not $splitSurface.Contains($requiredSplitContract)) {
        throw 'CL-01 requires one shared flexible/fixed split surface with a quiet center seam.'
    }
}
if ($splitSurface.Contains('.rounded(') -or
    $splitSurface.Contains('theme.accent') -or
    $splitSurface.Contains('shadow')) {
    throw 'CL-01 split settings must stay square, continuous and neutral.'
}

$clipboardRenderStart = $mainWindowSource.IndexOf('    fn render_clipboard(')
$clipboardRenderEnd = $mainWindowSource.IndexOf("`n    }`n}`n`nimpl Render for MainWindow", $clipboardRenderStart)
if ($clipboardRenderStart -lt 0 -or $clipboardRenderEnd -le $clipboardRenderStart) {
    throw 'Could not isolate the production clipboard workspace.'
}
$clipboardRender = $mainWindowSource.Substring(
    $clipboardRenderStart,
    $clipboardRenderEnd - $clipboardRenderStart
)
foreach ($requiredClipboardContract in @(
    'settings_split_surface(',
    'CLIPBOARD_SETTINGS_SECONDARY_WIDTH',
    'clipboard-settings-workspace',
    'clipboard-settings-split-surface',
    'clipboard-settings-primary-column',
    'clipboard-settings-column-divider',
    'clipboard-settings-secondary-column',
    '"呼出与快捷操作"',
    '"保留与隐私"'
)) {
    if (-not $clipboardRender.Contains($requiredClipboardContract)) {
        throw 'CL-01 clipboard settings must keep their existing primary/secondary ownership inside the shared surface.'
    }
}
if ($clipboardRender.Contains('.rounded(px(5.0))') -or
    $clipboardRender.Contains('.child(div().w(px(1.0)).bg(theme.border_subtle))')) {
    throw 'CL-01 must not retain the private rounded split-surface implementation.'
}

foreach ($requiredProductionEvidence in @(
    'fn clipboard_settings_keep_one_continuous_primary_secondary_surface(',
    'one continuous clipboard settings surface',
    'single clipboard settings column divider',
    'surface.left() - workspace.left()',
    'workspace.right() - surface.right()',
    'primary.right(), divider.left()',
    'divider.right(), secondary.left()',
    'secondary.size.width',
    'primary.size.width > secondary.size.width'
)) {
    if (-not $mainWindowSource.Contains($requiredProductionEvidence)) {
        throw 'CL-01 requires a real production-window regression for the protected split geometry.'
    }
}

$historyRowPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\history.rs'
$listPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\components\list.rs'
$historyRowSource = Get-Content -Raw -LiteralPath $historyRowPath
$listSource = Get-Content -Raw -LiteralPath $listPath

if (-not $listSource.Contains('pub(crate) fn compact_result_row(')) {
    throw 'CL-03 requires the shared compact_result_row shell in ui/components/list.rs.'
}
if (-not $historyRowSource.Contains('compact_result_row(')) {
    throw 'CL-03 requires clipboard history rows to render through the shared compact_result_row shell.'
}
if ($historyRowSource.Contains('fn history_row_border') -or
    $historyRowSource.Contains('fn history_row_background')) {
    throw 'CL-03 must retire the private hand-rolled history row border/background helpers.'
}

$popupModPath = Join-Path $root 'rust\apps\nxtools-desktop\src\ui\mod.rs'
$popupModSource = Get-Content -Raw -LiteralPath $popupModPath
if ($popupModSource.Contains('.text_xs()')) {
    throw 'CL-04 requires clipboard popup typography to use ui::typography tokens instead of gpui text_xs.'
}
$categoryTabStart = $popupModSource.IndexOf('fn category_tab(')
$categoryTabEnd = $popupModSource.IndexOf('fn dialog_button(', $categoryTabStart)
if ($categoryTabStart -lt 0 -or $categoryTabEnd -le $categoryTabStart) {
    throw 'Could not isolate the clipboard category tab renderer.'
}
$categoryTab = $popupModSource.Substring($categoryTabStart, $categoryTabEnd - $categoryTabStart)
foreach ($requiredTabContract in @(
    '.text_size(px(TEXT_LABEL))',
    '.line_height(px(LINE_HEIGHT_LABEL))',
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

Write-Host 'PASS clipboard history rows render through the shared compact result row shell'
Write-Host 'PASS clipboard category strip uses token typography with preserved drag, wheel and menu lifecycle'

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
foreach ($previewCapContract in @(
    'const IMAGE_RESOURCE_CACHE_CAPACITY: usize = 96;',
    'fn image_resource_cache_evicts_the_oldest_entry_beyond_capacity()'
)) {
    if (-not $popupModSource.Contains($previewCapContract)) {
        throw "CL-05 preview resource cap contract missing: $previewCapContract"
    }
}

Write-Host 'PASS clipboard preview shares the QL preview shell with a bounded 96-entry resource cache'
