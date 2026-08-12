$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$main = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
$mainWindow = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\main_window.rs') -Raw
$platformEvents = Get-Content (Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs') -Raw
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

Write-Host 'Desktop shell lifecycle architecture gate passed.'
