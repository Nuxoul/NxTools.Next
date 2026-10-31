$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$main = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
$mainWindow = (@(Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs') -Raw) +
    (Get-ChildItem (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window') -Filter '*.rs' |
        Sort-Object Name | ForEach-Object { Get-Content $_.FullName -Raw })) -join "`n"
$platformEvents = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs') -Raw
$desktopBuild = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\build.rs') -Raw
$runDev = Get-Content (Join-Path $root 'scripts\run-dev.ps1') -Raw
$trayPresentation = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DesktopTrayPresentation.cs') -Raw
$windowsAdapter = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs') -Raw
$eventWindow = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.EventWindow.cs') -Raw
$indexerInstaller = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\IndexerServiceInstaller.cs') -Raw
$installerScript = Get-Content (Join-Path $root 'installer\nxtools-next.iss') -Raw
$desktopSources = (Get-ChildItem (Join-Path $root 'rust\apps\nxtools-desktop\src') -Filter '*.rs' -Recurse |
    ForEach-Object { Get-Content $_.FullName -Raw }) -join "`n"

if ($main -notmatch 'cfg_attr\(not\(debug_assertions\), windows_subsystem = "windows"\)') {
    throw 'Release desktop must use the Windows GUI subsystem.'
}
if ($main -notmatch 'SilentStart' -or $main -notmatch 'main_window_show\(self\)') {
    throw 'Silent startup must use the production hidden-window launch policy.'
}
if ($main -notmatch 'should_summon_running_instance' -or
    $main -notmatch 'startup_failure_is_interactive' -or
    $main -notmatch 'main_window_logical_size' -or
    $main -notmatch 'main_window_placement_for_display' -or
    $main -notmatch 'display_id: placement\.display_id' -or
    $main -match 'boot_dpi_compensated_size') {
    throw 'Desktop startup must keep silent lifecycle policy and bind bounded logical window bounds to the same native display.'
}

# A hidden GPUI constructor keeps CW_USEDEFAULT native bounds until prepared.
# Native restore routes must never see the advisory HWND before exact client readiness.
$mainPreparation = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events\main_preparation.rs') -Raw
$nativePreparation = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\MainWindowPreparation.cs') -Raw
$nativeMainWindow = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.MainWindow.cs') -Raw
foreach ($contract in @('let publish_main', 'if !main_show', 'MainWindowPreparation::new', 'main_preparation,')) {
    if (-not $main.Contains($contract)) { throw "Hidden main startup publication is missing $contract" }
}
foreach ($contract in @('PrepareMainWindow', 'preparation.pending = Some(id)', 'readback.matches(', 'cx.defer(', 'preparation.ready.take()', 'MAX_BUFFERED_STARTUP_NOTIFICATIONS', 'MAIN_PREPARATION_TIMEOUT')) {
    if (-not $mainPreparation.Contains($contract)) { throw "Hidden main startup readiness is missing $contract" }
}
if (-not $nativePreparation.Contains('actual.Client != target') -or -not $nativePreparation.Contains('actual.Visible') -or
    -not $nativeMainWindow.Contains('!IsWindowVisible(window)') -or -not $nativeMainWindow.Contains('SwpNoZOrder | SwpNoActivate')) {
    throw 'Main preparation requires a still-hidden owned HWND and exact native client readback without activation.'
}
if ($nativeMainWindow -match 'SwpShowWindow|ShowWindow\(|ActivateWindow\(') {
    throw 'Main preparation must never show or activate the primary window.'
}
if ($eventWindow -notmatch 'ResolveApprovedDesktopExecutablePath' -or
    $eventWindow -notmatch 'ApplyStartupRegistration' -or
    $eventWindow -match 'AppContext\.BaseDirectory.*NxTools\.exe') {
    throw 'Desktop shell startup registration must use the approved Desktop resolver, not a PlatformHost-directory guess.'
}
if ($windowsAdapter -notmatch 'ResolveApprovedDesktopExecutablePath' -or
    $windowsAdapter -notmatch 'IsVerifiedCurrentDesktop' -or
    $windowsAdapter -notmatch 'ReleaseBinaryManifest\.DesktopSha256' -or
    $indexerInstaller -notmatch 'VerifiedPackagedBinaryHashMatches' -or
    $indexerInstaller -notmatch 'IsApprovedProtectedDesktop') {
    throw 'Desktop shell requires a hash-verified current-package sibling or the approved protected fallback.'
}
if ($windowsAdapter -notmatch 'DeleteValue\("NxTools", throwOnMissingValue: false\)' -or
    $installerScript -notmatch 'ValueType: none; ValueName: "NxTools"; Flags: deletevalue') {
    throw 'Startup migration must remove the legacy duplicate NxTools Run value in both settings reconciliation and setup.'
}
# A variable use is not a declaration; never scan through later assignments or strings.
# Keep the value on one source line even when the call/declaration is multiline.
if ($desktopSources -match '(?:\bfont_family\s*\(\s*|\b(?:const|static)\s+UI_(?:MONO_)?FONT_FAMILY\s*:\s*[^=;()"{}\r\n]+=\s*)"[^"\r\n]*,') {
    throw 'GPUI font_family must name one actual family, not a CSS fallback list.'
}
if ($mainWindow -notmatch 'send_command\(PlatformCommand::BeginWindowMove') {
    throw 'Main titlebar drag must reach the typed native move command.'
}
if ($platformEvents -notmatch 'send_command_correlated' -or
    $platformEvents -notmatch 'TrayAction::Open' -or
    $platformEvents -notmatch 'TrayAction::Exit') {
    throw 'Tray restore and exit must retain their typed correlated lifecycle.'
}
foreach ($size in 16, 32, 48, 128) {
    if ($desktopBuild -notmatch "icon-$size\.png") {
        throw "Desktop Windows resource must include the $size px formal brand icon."
    }
}
if ($trayPresentation -notmatch 'Tooltip = "NxTools"' -or
    $trayPresentation -notmatch 'OpenLabel = "打开 NxTools"' -or
    $trayPresentation -notmatch 'ExitLabel = "退出"') {
    throw 'Native tray tooltip and menu labels must remain the bounded product presentation.'
}
if ($windowsAdapter -match 'trayIcon\.Icon = icon \?\?' -or
    $windowsAdapter -notmatch 'DesktopExecutableIcon\.ExtractProductIcon') {
    throw 'Native tray must reject a missing product icon instead of using a generic fallback.'
}
if ($runDev -notmatch '\$env:CARGO_BUILD_JOBS\s*=\s*''1''' -or
    $runDev -notmatch '\$env:CARGO_INCREMENTAL\s*=\s*''0''' -or
    $runDev -notmatch '\$env:RUSTFLAGS\s*=\s*''-C codegen-units=256''') {
    throw 'The development launcher must apply the low-memory Rust build environment.'
}
$buildMarker = $runDev.IndexOf('& cargo build -p nxtools-desktop --jobs 1')
$hostMarker = $runDev.IndexOf('Start-Process -FilePath $hostPath')
if ($buildMarker -lt 0 -or $hostMarker -lt 0 -or $buildMarker -gt $hostMarker) {
    throw 'The development launcher must finish the desktop build before starting PlatformHost.'
}


# Queue09 adopts Settings presentation without replacing global persistence/native owners.
$globalSettingsSource = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\network_settings.rs')
$globalRulesSource = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\network_settings\blacklist.rs')
foreach ($contract in @('Settings::new("global-official-settings")', 'SettingPage::new(',
    'SettingGroup::new()', 'SettingItem::render(', 'aligned_setting_row(', 'input_segmented_control(',
    '.resettable(false)', 'Form::vertical()', 'Field::new()', 'Switch::new(',
    'AppConfig::update_global_settings_with_blacklist(', 'self.publish_live_blacklist(correlation_id, cx)',
    'fn save(', 'cx.defer(move |cx|', 'self.pending_apply.is_some() || self.draft_busy.get()',
    'GLOBAL_SETTINGS_WINDOW_WIDTH: f32 = 720.0', 'GLOBAL_SETTINGS_WINDOW_HEIGHT: f32 = 480.0')) {
    if (-not $globalSettingsSource.Contains($contract)) { throw "Global Settings component/original owner missing: $contract" }
}
foreach ($contract in @('Checkbox::new(', '.on_change(on_change)', '.disable_clipboard = *requested',
    '.disable_quick_launch = *requested', 'Button::new(', 'fn add_rule(',
    'ProcessBlacklistRule::new(', 'self.rules.remove(index)', 'self.process_input')) {
    if (-not $globalRulesSource.Contains($contract)) { throw "Global rule component/draft owner missing: $contract" }
}
if ($globalRulesSource.Contains('fn scope_action(') -or $globalRulesSource.Contains('focusable_content(') -or
    $globalSettingsSource.Contains('fn render_navigation(')) {
    throw 'Global ordinary rules/navigation must not retain hand-painted Base action controls.'
}
$globalKitTests = Get-Content -Raw (Join-Path $root 'rust\apps\nxtools-desktop\src\network_settings\kit_migration_tests.rs')
foreach ($contract in @('queue09_scope_pointer_tab_release_is_independent_local_and_cancel_discards',
    'queue09_proxy_form_preserves_inputs_invalid_draft_and_single_atomic_apply',
    'queue09_failed_commit_keeps_draft_busy_guard_and_retry_uses_original_save',
    'queue09_long_error_and_busy_controls_keep_footer_and_draft_at_fixed_budget',
    'queue09_optional_help_tooltip_and_proxy_keyboard_keep_one_draft_dispatch', 'KeyDownEvent', 'KeyUpEvent')) {
    if (-not $globalKitTests.Contains($contract)) { throw "Global production regression missing: $contract" }
}
Write-Host 'PASS Global Settings/independent Checkbox presentation retains atomic-save/deferred/native ownership (source only)'

Write-Host 'Desktop shell lifecycle architecture gate passed.'
