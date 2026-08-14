$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$main = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
$mainWindow = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs') -Raw
$platformEvents = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs') -Raw
$desktopBuild = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\build.rs') -Raw
$trayPresentation = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DesktopTrayPresentation.cs') -Raw
$windowsAdapter = Get-Content (Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs') -Raw
$desktopSources = (Get-ChildItem (Join-Path $root 'rust\apps\nxtools-desktop\src') -Filter '*.rs' -Recurse |
    ForEach-Object { Get-Content $_.FullName -Raw }) -join "`n"

if ($main -notmatch 'cfg_attr\(not\(debug_assertions\), windows_subsystem = "windows"\)') {
    throw 'Release desktop must use the Windows GUI subsystem.'
}
if ($main -notmatch 'SilentStart' -or $main -notmatch 'main_window_show\(self\)') {
    throw 'Silent startup must use the production hidden-window launch policy.'
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

Write-Host 'Desktop shell lifecycle architecture gate passed.'
