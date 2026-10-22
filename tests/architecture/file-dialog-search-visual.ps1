$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$sourcePath = Join-Path $repositoryRoot "rust\apps\nxtools-desktop\src\quick_launch\file_dialog_search.rs"
$source = Get-Content -Raw $sourcePath
$rowMatch = [regex]::Match(
    $source,
    '(?s)fn row\(\s*&self,\s*index: usize,\s*window: &mut Window,\s*cx: &mut Context<Self>,?\s*\).*?(?=\n    \})')
if (-not $rowMatch.Success) {
    throw "Could not locate the file-dialog search result-row renderer."
}
if ($rowMatch.Value -notmatch '\.w_full\(\)') {
    throw "File-dialog result selection only paints the content width instead of the full row."
}
$defaultRowMatch = [regex]::Match(
    $source,
    '(?s)fn default_source_row\(\s*&self,\s*window: &mut Window,\s*cx: &mut Context<Self>,?\s*\).*?(?=\n    fn row)')
if (-not $defaultRowMatch.Success) {
    throw "Could not locate the file-dialog default-source renderer."
}
$renderMatch = [regex]::Match(
    $source,
    '(?s)impl Render for FileDialogSearchPanel.*?(?=\nimpl QuickLaunchApplication)')
if (-not $renderMatch.Success) {
    throw "Could not locate the file-dialog search production renderer."
}
foreach ($requiredToken in @(
    'SPACING_4',
    'SPACING_8',
    'SPACING_12',
    'TEXT_BODY',
    'LINE_HEIGHT_BODY',
    'LINE_HEIGHT_LABEL'
)) {
    if ($rowMatch.Value -notmatch "\b$requiredToken\b") {
        throw "File-dialog result rows do not use the shared $requiredToken design token."
    }
}
foreach ($required in @('InputGroup::new("file-dialog-search-group")',
    'InputGroupAddon::new("file-dialog-search-icon")',
    'InputGroupAddonAlignment::InlineEnd', 'Input::new(&self.search_input)', 'SPACING_12')) {
    if (-not $renderMatch.Value.Contains($required)) {
        throw "File-dialog search must retain real Kit compound frame/addons/original input: $required"
    }
}
if ($renderMatch.Value.Contains('.appearance(false)') -or
    $renderMatch.Value.Contains('.bordered(false)') -or
    $renderMatch.Value.Contains('.focus_bordered(false)')) {
    throw 'Attached search must not suppress standalone appearance and reconstruct the frame.'
}
foreach ($requiredToken in @('SPACING_8', 'SPACING_12', 'TEXT_BODY', 'LINE_HEIGHT_BODY', 'LINE_HEIGHT_LABEL')) {
    if ($defaultRowMatch.Value -notmatch "\b$requiredToken\b") {
        throw "File-dialog default-source row does not use the shared $requiredToken design token."
    }
}
$rowRegression = Get-Content -Raw (Join-Path $repositoryRoot 'rust\apps\nxtools-desktop\src\quick_launch\polish_tests.rs')
if (-not $rowRegression.Contains('polished_file_dialog_row_keyboard_keeps_fixed_rails_and_search_navigation')) {
    throw 'Compact file-dialog readability must retain production row geometry regression.'
}
foreach ($staleToken in @(
    'META_TEXT_SIZE',
    'RESULT_DETAIL_SIZE',
    'RESULT_TITLE_SIZE',
    'SEARCH_TEXT_SIZE'
)) {
    if ($source -match "\b$staleToken\b") {
        throw "File-dialog search still uses the private $staleToken typography scale."
    }
}
if ($renderMatch.Value -notmatch '\.border_color\(theme\.window_border\)') {
    throw "File-dialog search is missing the shared semantic window boundary."
}
if ($renderMatch.Value -match '(?s)id\("file-dialog-search-window"\).*?\.shadow_') {
    throw "File-dialog search paints an exterior shadow inside its exact-size opaque GPUI root."
}
if ($source -notmatch 'window_background:\s*WindowBackgroundAppearance::Opaque' -or
    $source -notmatch 'window_decorations:\s*Some\(WindowDecorations::Client\)') {
    throw "File-dialog search no longer delegates exterior elevation to its opaque client-decorated native HWND."
}
if ($source -notmatch '\bCOMPACT_RESULT_ICON_RAIL_WIDTH\b' -or
    $source -notmatch '\bCOMPACT_RESULT_ROW_HEIGHT\b' -or
    $source -notmatch '\bCOMPACT_RESULT_TRAILING_RAIL_WIDTH\b') {
    throw "File-dialog search does not consume the shared compact-result rail metrics."
}

Write-Output "PASS file-dialog search uses shared typography, rails, native boundary and full-row selection"
