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
$urlHelperStart = $newView.IndexOf('fn new_download_url_control_layout(')
$urlHelperEnd = $newView.IndexOf('fn new_download_media_radio(', $urlHelperStart)
if ($urlHelperStart -lt 0 -or $urlHelperEnd -le $urlHelperStart) { throw 'New-download URL/paste helper boundary missing.' }
$urlHelper = $newView.Substring($urlHelperStart, $urlHelperEnd - $urlHelperStart)
$compactUrlStart = $newView.IndexOf('fn new_download_compact_url_control(')
if ($compactUrlStart -lt 0) { throw 'New-download compact URL/paste helper boundary missing.' }
$compactUrlHelper = $newView.Substring($compactUrlStart)
if (-not $handoff.Contains('options.display_id = display_id') -or
    -not [regex]::IsMatch($newView, '(?s)new_download_compact_url_control\(\s*&self\.url,\s*self\.handoff_pending,?\s*\)') -or
    [regex]::Matches($urlHelper, '\.disabled\(disabled\)').Count -lt 2 -or
    -not $urlHelper.Contains('Input::new(input)') -or -not $urlHelper.Contains('Button::new("download-paste-url")') -or
    [regex]::Matches($compactUrlHelper, '\.disabled\(disabled\)').Count -lt 2 -or
    -not $compactUrlHelper.Contains('InputGroup::new("download-new-url-group")') -or
    -not $compactUrlHelper.Contains('InputGroupButton::new("download-paste-url")')) {
    throw 'Fixed handoff must bind predecessor display and freeze the actual URL Input/paste Button through one helper.'
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

$nativeAcceptance = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\native_acceptance.rs')
$ui = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\mod.rs')
if ($ui -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod native_acceptance;' -or
    -not $preview.Contains('NXTOOLS_PREVIEW_NATIVE_ACCEPTANCE')) { throw 'Native acceptance must stay explicit Debug-only.' }
foreach ($forbidden in @('DesktopRuntime::','AppPaths::discover','ClipboardCaptureWorkerHandle','PlatformHostSupervisor','IndexerSupervisor','connect_local_','load_or_create(','save_to(')) {
    if ($nativeAcceptance.Contains($forbidden)) { throw "Native acceptance isolation violation: $forbidden" }
}
foreach ($required in @('QuickLaunchPreview::new','ClipboardPreview::new','WeakEntity::new_invalid()','show: false')) {
    if (-not $nativeAcceptance.Contains($required)) { throw "Native acceptance must reuse isolated production previews: $required" }
}

$manifest = Get-Content -Raw (Join-Path $root 'Cargo.toml')
$backend = Get-Content -Raw (Join-Path $root 'vendor\gpui-pre-windows-0312\src\events.rs')
if (-not $manifest.Contains('gpui-pre-windows = { path = "vendor/gpui-pre-windows-0312" }') -or
    -not $backend.Contains('GetWindowLongPtrW(handle, GWL_EXSTYLE)') -or
    -not $backend.Contains('WS_EX_NOACTIVATE') -or -not $backend.Contains('MA_NOACTIVATE')) {
    throw 'Approved Kit Windows passive-message boundary patch is missing.'
}

$quickOwner = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch.rs')
$quickLayout = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch\visual_preview.rs')
if ($quickOwner -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod visual_preview;' -or
    -not $preview.Contains('NXTOOLS_PREVIEW_QUICK_LAYOUT')) { throw 'Quick Find layout preview must remain explicit Debug-only.' }
foreach ($forbidden in @('DesktopRuntime::','AppPaths::discover','load_from(','load_or_create(','save_to(','connect_local_','ClipboardCaptureWorkerHandle','IndexerSupervisor')) {
    if ($quickLayout.Contains($forbidden)) { throw "Quick Find layout isolation violation: $forbidden" }
}
foreach ($required in @('QuickLaunchPanel::new','Root::new','WeakEntity::new_invalid()','QuickLaunchPreview::new')) {
    if (-not $quickLayout.Contains($required)) { throw "Quick Find preview must use isolated production views: $required" }
}

$quickSettings = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\visual_preview\quick_settings.rs')
foreach ($forbidden in @('DesktopRuntime::','AppPaths::discover','load_from(','load_or_create(','save_to(','connect_local_','ClipboardCaptureWorkerHandle','IndexerSupervisor')) {
    if ($quickSettings.Contains($forbidden)) { throw "Quick settings isolation violation: $forbidden" }
}
foreach ($required in @('PI_SCRATCH_DIR','FinderCatalog::open','&[]','drop(endpoint)','MainWindow::new','Root::new')) {
    if (-not $quickSettings.Contains($required)) { throw "Quick settings synthetic boundary missing: $required" }
}
if (-not $preview.Contains('NXTOOLS_PREVIEW_QUICK_SETTINGS')) { throw 'Quick settings preview must remain explicit Debug-only.' }

$finalGallery = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window\visual_preview\final_gallery.rs')
$clipboardGallery = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\ui\final_gallery.rs')
if (-not $preview.Contains('NXTOOLS_PREVIEW_FINAL_GALLERY') -or
    $ui -notmatch '#\[cfg\(debug_assertions\)\]\s+pub\(crate\) mod final_gallery;') { throw 'Final gallery must remain inside explicit Debug startup interception.' }
foreach ($source in @($finalGallery,$clipboardGallery,$fixture,$globalPreview)) {
    foreach ($forbidden in @('DesktopRuntime::','AppPaths::discover','load_from(','load_or_create(','save_to(','connect_local_','start_platform_server','PlatformHostSupervisor','IndexerSupervisor','.begin_probe(','.submit(')) {
        if ($source.Contains($forbidden)) { throw "Final gallery isolation violation: $forbidden" }
    }
}
foreach ($required in @('PI_SCRATCH_DIR','final-gallery-','scratch.join("clipboard.db")','drop(endpoint)','MainWindow::new','Root::new','shutdown()')) {
    if (-not $finalGallery.Contains($required)) { throw "Final gallery scratch/owner isolation missing: $required" }
}
if (-not $clipboardGallery.Contains('ClipboardPanel::new') -or -not $fixture.Contains('DownloadEngine::open_in_memory()')) { throw 'Final gallery must reuse isolated production view/models.' }
Write-Host 'Final gallery Debug/scratch/disconnected production-render isolation passed (source only).'
