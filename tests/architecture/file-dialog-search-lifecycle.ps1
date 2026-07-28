$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$fileDialogSearchPath = Join-Path $repositoryRoot "rust\apps\nxtools-desktop\src\quick_launch\file_dialog_search.rs"
$quickLaunchPath = Join-Path $repositoryRoot "rust\apps\nxtools-desktop\src\quick_launch.rs"
$fileDialogSearch = Get-Content -Raw $fileDialogSearchPath
$quickLaunch = Get-Content -Raw $quickLaunchPath

$handlerMatch = [regex]::Match(
    $fileDialogSearch,
    '(?s)pub\(super\) fn handle_file_dialog_search_host.*?(?=\n    fn open_file_dialog_search_window)')
if (-not $handlerMatch.Success) {
    throw "Could not locate the file-dialog search host lifecycle handler."
}
$createMatch = [regex]::Match(
    $handlerMatch.Value,
    '(?s)if self\.file_dialog_search\.window\.is_none\(\).*?(?=\n        \} else \{)')
if (-not $createMatch.Success) {
    throw "Could not locate the initial hidden GPUI window branch."
}
if ($createMatch.Value -notmatch 'schedule_application_update' -or
    $createMatch.Value -notmatch '\breturn;') {
    throw "Initial hidden GPUI placement/visibility is still applied in the creation turn."
}
if ($quickLaunch -match 'self\.handle_file_dialog_search_foreground\(&foreground, cx\);') {
    throw "Generic foreground dismissal still races the authoritative file-dialog host visibility event."
}
if ($fileDialogSearch -notmatch 'SetWindowTopmostCommand' -or
    $fileDialogSearch -notmatch '(?s)SetWindowTopmostCommand\s*\{.*?enabled:\s*true') {
    throw "The fixed file-dialog bar does not reproduce Listary's one-time non-activating Topmost initialization."
}

Write-Output "PASS file-dialog search defers first native show and has one visibility authority"
