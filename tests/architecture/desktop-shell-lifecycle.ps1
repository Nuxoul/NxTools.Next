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
if ($eventWindow -notmatch 'ResolveApprovedDesktopExecutablePath' -or
    $eventWindow -notmatch 'ApplyStartupRegistration' -or
    $eventWindow -match 'AppContext\.BaseDirectory.*NxTools\.exe') {
    throw 'Desktop shell startup registration must use the approved Desktop resolver, not a PlatformHost-directory guess.'
}
if ($windowsAdapter -notmatch 'ResolveApprovedDesktopExecutablePath' -or
    $indexerInstaller -notmatch 'IsApprovedProtectedDesktop' -or
    $indexerInstaller -notmatch 'ReleaseBinaryManifest\.DesktopSha256') {
    throw 'Desktop startup registration must require the protected Desktop path and release hash.'
}
if ($windowsAdapter -notmatch 'DeleteValue\("NxTools", throwOnMissingValue: false\)' -or
    $installerScript -notmatch 'ValueType: none; ValueName: "NxTools"; Flags: deletevalue') {
    throw 'Startup migration must remove the legacy duplicate NxTools Run value in both settings reconciliation and setup.'
}
if ($desktopSources -match '(?:font_family\(|UI_(?:MONO_)?FONT_FAMILY[^=]*=\s*)"[^"]*,') {
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


Write-Host 'Desktop shell lifecycle architecture gate passed.'
