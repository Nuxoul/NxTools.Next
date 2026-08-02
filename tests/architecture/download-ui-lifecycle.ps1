param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\downloads.rs"
$source = Get-Content -LiteralPath $sourcePath -Raw
$nativeHostPath = Join-Path $RepositoryRoot "rust\apps\nxtools-cli\src\browser_native_host.rs"
$nativeHost = Get-Content -LiteralPath $nativeHostPath -Raw
$downloadIpcPath = Join-Path $RepositoryRoot "rust\crates\nxtools-download-ipc\src\lib.rs"
$downloadIpc = Get-Content -LiteralPath $downloadIpcPath -Raw
$platformAdapterPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs"
$platformAdapter = Get-Content -LiteralPath $platformAdapterPath -Raw
$nativeFileDragCoordinatorPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\NativeFileDragCoordinator.cs"
$nativeFileDragCoordinator = Get-Content -LiteralPath $nativeFileDragCoordinatorPath -Raw
$nativeFileDragPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\NativeFileDragService.cs"
$nativeFileDrag = Get-Content -LiteralPath $nativeFileDragPath -Raw

function Assert-SourceContains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Pattern,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $source.Contains($Pattern)) {
        throw $Message
    }
}

Assert-SourceContains `
    -Pattern 'fn download_window_close_button(theme: Theme)' `
    -Message "Download windows must share one explicit client-drawn close control."
Assert-SourceContains `
    -Pattern '.on_click(|_, window, _| window.remove_window())' `
    -Message "Client-drawn download titlebars must close through an explicit GPUI action."
Assert-SourceContains `
    -Pattern 'this.file_name.clone(), resolved_file_name, confirmed' `
    -Message "Probe completion must carry the resolved file name back to the visible input."
Assert-SourceContains `
    -Pattern 'input.set_value(resolved_file_name, window, cx);' `
    -Message "Probe completion must hydrate the visible file-name input."
Assert-SourceContains `
    -Pattern 'window.resize(new_download_window_size(confirmed));' `
    -Message "The add window must expand only after probing succeeds."
if (-not $nativeHost.Contains('BrowserDownloadTarget::OpenDialog') -or
    -not $nativeHost.Contains('connect_download_ui_or_start(executable)')) {
    throw "Browser openDialog requests must cross the visible desktop UI boundary."
}
if ($nativeHost.Contains('payload.open_dialog || payload.start_immediately == Some(false)')) {
    throw "Browser openDialog must never be mapped to a queue-only task submission."
}
if (-not $downloadIpc.Contains('DOWNLOAD_UI_PIPE_NAME') -or
    -not $source.Contains('handle_browser_download_dialog')) {
    throw "The authenticated browser UI pipe must terminate at the GPUI download dialog."
}
if (-not $source.Contains('external_request_context') -or
    -not $source.Contains('browser_file_name_is_hint')) {
    throw "The visible dialog must preserve browser credentials and treat its filename as a probe hint."
}
Assert-SourceContains `
    -Pattern 'DownloadWindowPhase::Progress => size(px(440.0), px(176.0))' `
    -Message "The progress window must remain compact and content-driven."
Assert-SourceContains `
    -Pattern 'raise_terminal_window_without_activation' `
    -Message "A terminal download transition must return the window to the visible Z-order without stealing focus."
$completedStart = $source.IndexOf("if status == DownloadStatus::Completed")
$failedStart = $source.IndexOf("if status == DownloadStatus::Failed", $completedStart)
if ($completedStart -lt 0 -or $failedStart -le $completedStart) {
    throw "Could not isolate the completed download presentation."
}
$completedBody = $source.Substring($completedStart, $failedStart - $completedStart)
$dragStart = $completedBody.IndexOf('.id("download-complete-drag")')
$revealStart = $completedBody.IndexOf('Button::new("download-complete-reveal")', $dragStart)
if ($dragStart -lt 0 -or $revealStart -le $dragStart) {
    throw "Could not isolate the completed artifact drag source."
}
$dragBody = $completedBody.Substring($dragStart, $revealStart - $dragStart)
if (-not $dragBody.Contains(".on_mouse_down(MouseButton::Left")) {
    throw "The native file drag must start from the same pressed-pointer callback as the reference implementations."
}
if ($dragBody.Contains(".on_drag(")) {
    throw "GPUI's internal drag loop must not compete with the native Windows OLE drag loop."
}
if ($dragBody.Contains("cx.defer(")) {
    throw "Deferring the native drag command can release the pointer before OLE receives it."
}
if (-not $dragBody.Contains("source_window_handle")) {
    throw "The pressed GPUI window HWND must cross the protocol seam with the completed artifact."
}
if (-not $platformAdapter.Contains("_fileDragCoordinator.BeginAsync(command, correlationId, cancellationToken)")) {
    throw "The Windows adapter must hand native file drag to the non-blocking coordinator."
}
if (-not $nativeFileDragCoordinator.Contains("NativeFileDragService.TryDragFile")) {
    throw "The native file-drag coordinator must use the reference-compatible OLE file-drag source."
}
if ($platformAdapter.Contains("_fileDragSource.DoDragDrop(")) {
    throw "The hidden WinForms control drag path can strand the native OLE loop."
}
if (-not $nativeFileDrag.Contains("WmCancelMode") -or
    -not $nativeFileDrag.Contains("SendMessageTimeout")) {
    throw "The C# drag adapter must release GPUI's cross-thread pointer capture before entering OLE."
}
if (-not $nativeFileDrag.Contains("GetAsyncKeyState")) {
    throw "The C# drag adapter must reject a gesture that ended before the OLE handoff."
}
if (-not $nativeFileDrag.Contains("private static extern int DoDragDrop(") -or
    -not $nativeFileDrag.Contains("TryReleaseSourcePointerCapture(sourceWindow")) {
    throw "The native OLE drag loop must release the visible GPUI source HWND capture before entering DoDragDrop."
}

Assert-SourceContains `
    -Pattern 'request_progress_passive(task_id)' `
    -Message "Automatic download admission must open its progress window without stealing focus."
Assert-SourceContains `
    -Pattern 'download.task_window.admission_wave' `
    -Message "Automatic download admission waves must retain one correlation-scoped diagnostic boundary."

if ($source.Contains('"等待链接探测"')) {
    throw "The initial add window must not reserve a blank probe card."
}

$newViewStart = $source.IndexOf("impl NewDownloadView {")
$requestStart = $source.IndexOf("    fn request(", $newViewStart)
if ($newViewStart -lt 0 -or $requestStart -le $newViewStart) {
    throw "Could not isolate the destination picker lifecycle."
}
$newViewBody = $source.Substring($newViewStart, $requestStart - $newViewStart)
if ($newViewBody.Contains("confirmed_request = None") -or $newViewBody.Contains("probe = None")) {
    throw "Changing only the local destination must not invalidate remote probe metadata."
}

Write-Host "Download UI lifecycle architecture gate passed."
