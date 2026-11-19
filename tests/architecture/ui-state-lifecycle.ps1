param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$statePath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\components\state.rs"
$mainPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\main_window.rs"
$quickPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\quick_launch.rs"
$clipboardPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\mod.rs"
$state = Get-Content -LiteralPath $statePath -Raw
$main = (@(Get-Content -LiteralPath $mainPath -Raw) +
    (Get-ChildItem (Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\main_window") -Filter '*.rs' |
        Sort-Object Name | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw })) -join "`n"
$quick = (@(Get-Content -LiteralPath $quickPath -Raw) +
    (Get-ChildItem (Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\quick_launch") -Filter '*.rs' |
        Sort-Object Name | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw })) -join "`n"
$clipboard = (Get-Content -LiteralPath $clipboardPath -Raw) +
    (Get-Content -LiteralPath (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\ui\history_filters.rs') -Raw)

foreach ($contract in @(
    'pub(crate) fn empty_state(',
    'pub(crate) fn progress_state(',
    'pub(crate) fn inline_error_state(',
    'pub(crate) const WORK_STATE_ICON_SIZE: f32 = 18.0;',
    'pub(crate) const WORK_STATE_MIN_HEIGHT: f32 = 72.0;'
)) {
    if (-not $state.Contains($contract)) {
        throw "Shared work-state component contract is missing: $contract"
    }
}
foreach ($component in @('Empty::new()', 'EmptyContent::new()', 'Alert::error(', 'Spinner::new().small()', 'motion_allowed(cx)')) {
    if (-not $state.Contains($component)) { throw "Shared state actual component/motion boundary missing: $component" }
}
if ($state.Contains('Progress::new(')) { throw 'progress_state is loading copy, not a fabricated determinate percentage.' }

if (-not $main.Contains('"download-empty-state"') -or
    -not $main.Contains('"download-detail-segments-progress"') -or
    -not $main.Contains('"download-detail-log-error"')) {
    throw "Download empty, progress and error states must use the shared work-state boundary."
}

if (-not $quick.Contains('"quick-launch-results-empty"') -or
    -not $quick.Contains('"quick-launch-search-feedback"') -or
    -not $quick.Contains('SEARCH_FEEDBACK_DELAY') -or
    -not $quick.Contains('motion_allowed(cx)') -or
    -not $quick.Contains('"quick-launch-actions-empty"')) {
    throw "Quick-launch must separate empty/action states from delayed, motion-aware input feedback."
}

if (-not $clipboard.Contains('"clipboard-history-empty"')) {
    throw "Clipboard history must use the shared empty-state boundary."
}

if ($state.Contains('.with_animation(') -or $state.Contains('.with_animations(')) {
    throw "Shared idle work states must not start a repeating animation loop."
}

Write-Host "Shared UI state lifecycle architecture gate passed."
