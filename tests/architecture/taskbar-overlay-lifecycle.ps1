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
if (-not $source.Contains('TASKBAR_OVERLAY_ENTER_DURATION') -or
    -not $source.Contains('TASKBAR_OVERLAY_EXIT_DURATION')) {
    throw "Taskbar overlay visibility must define bounded enter and exit motion durations."
}
if (-not $source.Contains('TASKBAR_OVERLAY_ENTER_DURATION: Duration = Duration::from_millis(90)')) {
    throw "Taskbar overlay enter motion must reach its visible state within 90 ms."
}
if (-not $source.Contains('(0.45 + 0.55 * delta, 1.5 * (1.0 - delta))')) {
    throw "Taskbar overlay enter motion must be perceptible from its first rendered frame."
}
if (-not $source.Contains('complete_visibility_exit')) {
    throw "Taskbar overlay native hide must be delayed until the GPUI exit transition completes."
}
$deferStart = $observer.IndexOf('cx.defer(')
$beforeDefer = $observer.Substring(0, $deferStart)
if ($beforeDefer.Contains('application.read(cx)') -or
    $beforeDefer.Contains('application.update(cx')) {
    throw "Taskbar overlay observer must not synchronously re-enter DownloadApplication."
}
foreach ($requiredPresentationContract in @(
    'download-taskbar-overlay-content',
    'download-taskbar-overlay-metrics',
    'download-taskbar-overlay-progress',
    'download-taskbar-overlay-edit-border',
    'download-taskbar-overlay-resize-handle',
    'download-taskbar-overlay-transferred',
    '.font_family(UI_FONT_FAMILY)',
    '.font_family(UI_MONO_FONT_FAMILY)',
    '.text_size(px(TEXT_LABEL))',
    '.line_height(px(LINE_HEIGHT_LABEL))'
)) {
    if (-not $source.Contains($requiredPresentationContract)) {
        throw "Taskbar overlay normal/edit presentation must expose stable readable telemetry geometry."
    }
}
foreach ($forbiddenEditingLayout in @(
    'editing_padding',
    'saturating_sub(if *editing',
    '.px(px(if *editing',
    '.py(px(if *editing'
)) {
    if ($source.Contains($forbiddenEditingLayout)) {
        throw "Taskbar overlay edit chrome must not consume or shift the normal content frame."
    }
}
$renderStart = $source.IndexOf('impl Render for DownloadTaskbarOverlayView')
$metricsStart = $source.IndexOf('fn taskbar_metrics_element', $renderStart)
if ($renderStart -lt 0 -or $metricsStart -le $renderStart) {
    throw "Could not isolate the taskbar overlay production renderer."
}
$render = $source.Substring($renderStart, $metricsStart - $renderStart)
$applicationUpdates = [regex]::Matches($render, 'application\.update\(cx').Count
$deferredApplicationUpdates = [regex]::Matches(
    $render,
    'cx\.defer\(move \|cx\| \{\s*application\.update\(cx'
).Count
if ($render.Contains('this.application.update(cx') -or
    $applicationUpdates -ne $deferredApplicationUpdates) {
    throw "Taskbar overlay pointer callbacks must defer cross-entity application mutation."
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
$openStart = $downloads.IndexOf('pub(crate) fn open_taskbar_overlay(')
$intentStart = $downloads.IndexOf('fn taskbar_overlay_native_close_intent(', $openStart)
$nativeHandlerStart = $downloads.IndexOf('fn handle_native_taskbar_overlay_close(', $intentStart)
$placementStart = $downloads.IndexOf('fn apply_taskbar_overlay_placement(', $nativeHandlerStart)
if ($openStart -lt 0 -or $intentStart -le $openStart -or
    $nativeHandlerStart -le $intentStart -or $placementStart -le $nativeHandlerStart) {
    throw "Could not isolate the taskbar overlay native close boundary."
}
$openOverlay = $downloads.Substring($openStart, $intentStart - $openStart)
$closeCallback = $openOverlay.IndexOf('on_window_should_close')
$closeDefer = $openOverlay.IndexOf('cx.defer(', $closeCallback)
$closeUpdate = $openOverlay.IndexOf('application.update(cx,', $closeCallback)
if ($closeCallback -lt 0 -or $closeDefer -lt 0 -or $closeUpdate -lt 0 -or
    $closeUpdate -lt $closeDefer) {
    throw "Taskbar overlay native close must decide synchronously and defer application mutation."
}
$beforeCloseDefer = $openOverlay.Substring($closeCallback, $closeDefer - $closeCallback)
if ($beforeCloseDefer.Contains('.update(cx')) {
    throw "Taskbar overlay native close must not mutate DownloadApplication before the deferred turn."
}

$intentHandler = $downloads.Substring($intentStart, $nativeHandlerStart - $intentStart)
if (-not $intentHandler.Contains('self.taskbar_overlay_window != Some(closing_window)') -or
    -not $intentHandler.Contains('edit_correlation_id: self.taskbar_overlay_edit_correlation_id')) {
    throw "Taskbar overlay native close intent must capture the current window and edit correlation identity."
}
$nativeHandler = $downloads.Substring($nativeHandlerStart, $placementStart - $nativeHandlerStart)
foreach ($requiredNativeCloseGuard in @(
    'self.taskbar_overlay_window != Some(closing_window)',
    'self.taskbar_overlay_edit_draft.is_some()',
    'self.taskbar_overlay_edit_correlation_id == edit_correlation_id'
)) {
    if (-not $nativeHandler.Contains($requiredNativeCloseGuard)) {
        throw "Deferred taskbar overlay close cleanup must reject stale window and edit identities."
    }
}
if (-not $downloads.Contains('fn stale_native_taskbar_overlay_close_cannot_cancel_the_current_edit(')) {
    throw "Taskbar overlay native-close correlation requires a real stale-window lifecycle regression test."
}

Write-Host "Taskbar overlay lifecycle architecture gate passed."
