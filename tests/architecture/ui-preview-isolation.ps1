$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$main = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
$mainWindow = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs') -Raw
$downloads = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads.rs') -Raw
$preview = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\visual_preview.rs') -Raw
$fixture = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\visual_preview.rs') -Raw
$mainStart = $main.IndexOf('fn main() {')
$intercept = $main.IndexOf('main_window::visual_preview::requested', $mainStart)
$runtime = $main.IndexOf('let instance_wait = protected_restart_instance_wait();', $mainStart)
if ($mainStart -lt 0 -or $intercept -lt $mainStart -or $runtime -lt $intercept -or
    -not $main.Contains('Isolated UI preview is available only in Debug builds.')) {
    throw 'Explicit preview must be intercepted before real startup; Release must reject it.'
}
foreach ($module in @($mainWindow, $downloads)) {
    if ($module -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod visual_preview;') {
        throw 'Native preview modules must remain Debug-only.'
    }
}
foreach ($forbidden in @('DesktopRuntime::', 'AppPaths::discover', 'connect_local_', 'start_platform_server',
    'PlatformHostSupervisor', 'IndexerSupervisor', 'ClipboardCaptureWorkerHandle', 'begin_probe(', 'submit(',
    'open_new_download_window(', 'save_to(', 'load_or_create(')) {
    if ($preview.Contains($forbidden) -or $fixture.Contains($forbidden)) {
        throw "Preview isolation violation: $forbidden"
    }
}
foreach ($required in @('MainWindow::new', 'Root::new', 'apply_component_theme', 'new_invalid()')) {
    if (-not $preview.Contains($required)) { throw "Preview must reuse production rendering: $required" }
}
foreach ($required in @('DownloadEngine::open_in_memory()', 'drop(endpoint)', 'NewDownloadView::new',
    'new_download_window_options', 'new_download_window_size(confirmed)', 'example.test')) {
    if (-not $fixture.Contains($required)) { throw "Synthetic download fixture isolation missing: $required" }
}
$handoff = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\browser_handoff_impl.rs') -Raw
$newView = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\new_download_view.rs') -Raw
if (-not $handoff.Contains('options.display_id = display_id') -or
    [regex]::Matches($newView, '\.disabled\(self\.handoff_pending\)').Count -lt 2) {
    throw 'Fixed handoff must bind predecessor display and freeze URL/paste while pending.'
}

$transition = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\fixed_transition_preview.rs') -Raw
if ($downloads -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod fixed_transition_preview;' -or
    -not $transition.Contains('TcpListener::bind("127.0.0.1:0")') -or
    -not $transition.Contains('open_connection_for_contract_test()') -or
    -not $transition.Contains('isolated fixture rejects non-presentation command') -or
    -not $transition.Contains('PI_SCRATCH_DIR')) {
    throw 'Fixed-stage native integration fixture must remain Debug/localhost/scratch/owned-command only.'
}
foreach ($forbidden in @('DesktopRuntime::', 'AppPaths::discover', 'start_platform_server', 'connect_local_',
    'PlatformHostSupervisor', 'IndexerSupervisor', 'ClipboardCaptureWorkerHandle', '.submit(', 'load_or_create(', 'save_to(')) {
    if ($transition.Contains($forbidden)) { throw "Fixed-stage preview isolation violation: $forbidden" }
}

$globalOwner = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\network_settings.rs') -Raw
$globalPreview = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\network_settings\visual_preview.rs') -Raw
$completionPreview = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\downloads\completion_transition_preview.rs') -Raw
if ($globalOwner -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod visual_preview;' -or
    $downloads -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod completion_transition_preview;') {
    throw 'Global blacklist/completion previews must remain Debug-only.'
}
foreach ($isolated in @($globalPreview, $completionPreview)) {
    foreach ($forbidden in @('AppPaths::discover', 'load_or_create(', 'load_from(', 'save_to(', 'DesktopRuntime::', 'ClipboardCaptureWorkerHandle')) {
        if ($isolated.Contains($forbidden)) { throw "Synthetic settings/task preview isolation violation: $forbidden" }
    }
    if (-not $isolated.Contains('PI_SCRATCH_DIR')) { throw 'Preview temporary paths must stay in session scratch.' }
}

Write-Host 'Native UI preview isolation architecture gate passed (source boundary only).'
