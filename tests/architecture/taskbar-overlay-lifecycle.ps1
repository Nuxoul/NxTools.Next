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

Write-Host "Taskbar overlay lifecycle architecture gate passed."
