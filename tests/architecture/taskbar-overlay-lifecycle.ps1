param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\download_taskbar_overlay.rs"
$source = Get-Content -LiteralPath $sourcePath -Raw
$observerStart = $source.IndexOf('let subscription = cx.observe(&application')
$boundsObserverStart = $source.IndexOf('let bounds_subscription =', $observerStart)
if ($observerStart -lt 0 -or $boundsObserverStart -le $observerStart) {
    throw "Could not isolate the taskbar overlay application observer."
}

$observer = $source.Substring($observerStart, $boundsObserverStart - $observerStart)
if (-not $observer.Contains('cx.defer(')) {
    throw "Taskbar overlay model refresh must be deferred past the observed application lease."
}
$deferStart = $observer.IndexOf('cx.defer(')
$beforeDefer = $observer.Substring(0, $deferStart)
if ($beforeDefer.Contains('application.read(cx)') -or
    $beforeDefer.Contains('application.update(cx')) {
    throw "Taskbar overlay observer must not synchronously re-enter DownloadApplication."
}

$downloadsPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\downloads.rs"
$downloads = Get-Content -LiteralPath $downloadsPath -Raw
$handlerStart = $downloads.IndexOf('pub(crate) fn handle_platform_notification')
$handlerEnd = $downloads.IndexOf('fn handle_directory_result', $handlerStart)
if ($handlerStart -lt 0 -or $handlerEnd -le $handlerStart) {
    throw "Could not isolate the download platform notification handler."
}
$handler = $downloads.Substring($handlerStart, $handlerEnd - $handlerStart)
if (-not $handler.Contains('PlatformNotification::Connected')) {
    throw "Taskbar overlay placement must be restored after PlatformHost connects."
}
if (-not $handler.Contains('apply_taskbar_overlay_placement')) {
    throw "The PlatformHost reconnect branch must replay the persisted taskbar overlay placement."
}

$placementPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\TaskbarOverlayPlacement.cs"
$placement = Get-Content -LiteralPath $placementPath -Raw
$visibilityStart = $placement.IndexOf('internal static WindowsCommandResult SetVisibility')
$passiveStyleStart = $placement.IndexOf('internal static bool HasPassiveOverlayStyles', $visibilityStart)
if ($visibilityStart -lt 0 -or $passiveStyleStart -le $visibilityStart) {
    throw "Could not isolate the native taskbar-overlay visibility boundary."
}
$visibility = $placement.Substring($visibilityStart, $passiveStyleStart - $visibilityStart)
if (-not $visibility.Contains('HwndTopmost') -or
    -not $visibility.Contains('SetWindowPos(') -or
    -not $visibility.Contains('IsTopmost(window)')) {
    throw "Showing the taskbar overlay must atomically restore and verify HWND_TOPMOST."
}

$adapterPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs"
$adapter = Get-Content -LiteralPath $adapterPath -Raw
$foregroundStart = $adapter.IndexOf('private void OnForegroundChanged(')
$processForegroundStart = $adapter.IndexOf('private void ProcessForeground(', $foregroundStart)
if ($foregroundStart -lt 0 -or $processForegroundStart -le $foregroundStart) {
    throw "Could not isolate the Windows foreground-change callback."
}
$foreground = $adapter.Substring($foregroundStart, $processForegroundStart - $foregroundStart)
if (-not $foreground.Contains('ReapplyTaskbarOverlayPlacement();')) {
    throw "A foreground-window transition must reassert the tracked taskbar-overlay placement."
}
if (-not $adapter.Contains('["verified_topmost"] = TaskbarOverlayPlacement.IsTopmost(')) {
    throw "Taskbar-overlay visibility diagnostics must record the verified topmost state."
}

Write-Host "Taskbar overlay lifecycle architecture gate passed."
