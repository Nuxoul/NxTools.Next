param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$downloadsFiles = @(
    'downloads.rs',
    'downloads\artifact_actions.rs',
    'downloads\delete_confirmation.rs',
    'downloads\browser_handoff_impl.rs',
    'downloads\completion_shutdown_impl.rs',
    'downloads\new_download_view.rs',
    'downloads\schedule_view.rs',
    'downloads\shutdown_window.rs',
    'downloads\settings_view.rs',
    'downloads\task_window.rs',
    'downloads\task_window_details.rs',
    'downloads\window_support.rs'
)
$source = ($downloadsFiles | ForEach-Object {
    Get-Content -LiteralPath (Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\$_") -Raw
}) -join "`n"
$dialogComponentPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\components\dialog.rs"
$dialogComponent = Get-Content -LiteralPath $dialogComponentPath -Raw
$platformAdapterPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs"
$platformAdapter = Get-Content -LiteralPath $platformAdapterPath -Raw
$nativeFileDragCoordinatorPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\NativeFileDragCoordinator.cs"
$nativeFileDragCoordinator = Get-Content -LiteralPath $nativeFileDragCoordinatorPath -Raw
$nativeFileDragPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\NativeFileDragService.cs"
$nativeFileDrag = Get-Content -LiteralPath $nativeFileDragPath -Raw

$settingsSource = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\settings_view.rs')
foreach ($contract in @('Settings::new("download-settings-kit")', 'SettingPage::new(',
    '.resettable(false)', 'SettingGroup::new()', 'SettingItem::render(', 'aligned_setting_row(', '.justify_end()',
    'Form::new()', 'Textarea::new(&self.default_request_headers)', 'fn render_setting_control(',
    'fn save(', 'application.apply_settings(draft, correlation_id, cx)')) {
    if (-not $settingsSource.Contains($contract)) { throw "Download settings component/draft boundary missing: $contract" }
}
if ($settingsSource.Contains('fn download_settings_navigation_item(') -or
    $settingsSource.Contains('fn download_settings_matrix_section(') -or
    $settingsSource.Contains('multiline_field_shell(') -or
    $settingsSource.Contains('pub(crate) scroll: ScrollHandle')) {
    throw 'Official Settings must own page navigation/scroll and styled Textarea its sole frame.'
}
$mainDownloadSource = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\main_window\download_page.rs')
foreach ($contract in @('InputGroup::new("download-search-group")', 'InputGroupButton::new("download-search-clear")',
    'Toolbar::new("download-header-toolbar")', 'ToolbarGroup::new("download-header-command-group")',
    'Toolbar::new("download-batch-kit-toolbar")', 'TabBar::new("download-detail-tab-bar")',
    'DownloadDetailTab::ALL', 'fn download_aggregate_progress(', 'Progress::new(', 'DescriptionList::horizontal()',
    'download_selection_anchor', 'uniform_list(')) {
    if (-not $mainDownloadSource.Contains($contract)) { throw "Download main component/ownership boundary missing: $contract" }
}
if ($mainDownloadSource.Contains('.appearance(false)') -or $mainDownloadSource.Contains('.focus_bordered(false)')) {
    throw 'Download main search must retain its actual styled InputGroup frame.'
}

$newDownloadSource = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\new_download_view.rs')
foreach ($contract in @('Form::horizontal()', 'Form::new()', 'Field::new()', 'RadioGroup::new(',
    'Radio::new(', 'StatefulInteractiveElement::on_click(radio', 'ListItem::new(',
    'Popover::new("download-new-schedule-popover")', 'Popover::new("download-new-category-popover")',
    'fn new_download_compact_url_control(', 'InputGroup::new("download-new-url-group")',
    'InputGroupButton::new("download-paste-url")', 'fn fixed_confirmation_draft(',
    'fn request(', 'fn begin_probe(', 'fn submit(')) {
    if (-not $newDownloadSource.Contains($contract)) { throw "New-download component/original owner missing: $contract" }
}
if ($newDownloadSource.Contains('popover_shell(') -or $newDownloadSource.Contains('labeled_control(') -or
    $newDownloadSource.Contains('radio_list_row(') -or $newDownloadSource.Contains('.appearance(false)')) {
    throw 'New-download must use actual Form/Radio/styled Popover, not old ordinary chrome.'
}

$scheduleContent = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\schedule_view.rs')
$taskContent = (Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\task_window.rs')) + (Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\task_window_details.rs'))
$supportContent = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\window_support.rs')
foreach ($contract in @('Form::horizontal()', 'Field::new()', 'StatusBar::new()', 'Switch::new(id)',
    'Toggle::new(', 'this.enabled_start = *requested', 'this.enabled_stop = *requested',
    'TimeField::new(input)', 'HourCycle::H23', 'TimePrecision::Minute', 'build_download_schedule(', 'DialogContent::new()', 'DialogFooter::new()')) {
    if (-not $scheduleContent.Contains($contract)) { throw "Schedule component/original draft missing: $contract" }
}
if ($scheduleContent.Contains('ToggleGroup::new(')) {
    throw 'Weekdays remain independent toggles; TimeField uses user-approved valid-time editing and original Apply validation.'
}
foreach ($contract in @('Progress::new("download-task-window-progress")', '.xsmall()',
    'Toolbar::new("download-task-window-toolbar")', 'ToolbarGroup::new("download-task-window-action-group")',
    'Checkbox::new(', 'save_completion_intention(', 'TabBar::new("download-task-detail-tabs")', 'download-window-options', 'DialogFooter::new()',
    'fn replace_snapshot(', 'fn segment_progress_table(', 'segment_display_status(', 'Table::new()')) {
    if (-not $taskContent.Contains($contract)) { throw "Task component/phase/domain owner missing: $contract" }
}
if (-not $taskContent.Contains('DescriptionList::horizontal()')) { throw 'Task metadata must use real DescriptionList.' }
foreach ($entry in @('delete_confirmation.rs', 'shutdown_window.rs')) {
    $content = Get-Content -Raw (Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\downloads\$entry")
    foreach ($contract in @('DialogContent::new()', 'DialogFooter::new()', 'Alert::', 'cx.defer(')) {
        if (-not $content.Contains($contract)) { throw "Danger inner-content boundary missing in ${entry}: $contract" }
    }
    if ($content.Contains('AlertDialog::new(') -or $content.Contains('Dialog::new(')) {
        throw 'Independent native supporting windows must not become Root modal hosts.'
    }
}
if (-not $mainDownloadSource.Contains('application.request_delete_confirmation(task_id, cx)')) {
    throw 'Deletion view live menu reachability must not be replaced by test-only adoption.'
}

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

function Assert-NativeCloseDefersMutation {
    param(
        [Parameter(Mandatory = $true)]
        [string]$MethodStart,
        [Parameter(Mandatory = $true)]
        [string]$MethodEnd,
        [Parameter(Mandatory = $true)]
        [string]$DeferredHandler
    )

    $start = $source.IndexOf($MethodStart)
    $end = $source.IndexOf($MethodEnd, $start)
    if ($start -lt 0 -or $end -le $start) {
        throw "Could not isolate auxiliary-dialog native close lifecycle."
    }
    $method = $source.Substring($start, $end - $start)
    $callback = $method.IndexOf('on_window_should_close')
    $defer = $method.IndexOf('cx.defer(', $callback)
    $update = $method.IndexOf('.update(cx,', $callback)
    if ($callback -lt 0 -or $defer -lt 0 -or $update -lt 0 -or $update -lt $defer -or
        -not $method.Contains($DeferredHandler)) {
        throw "Auxiliary-dialog native close callbacks must decide synchronously and defer guarded registration mutation."
    }
}

Assert-SourceContains `
    -Pattern 'titlebar::controlled_titlebar(' `
    -Message "Download windows must share one explicit client-drawn close control."
Assert-SourceContains `
    -Pattern '|_, window, _| window.remove_window()' `
    -Message "Client-drawn download titlebars must close through an explicit GPUI action."
Assert-SourceContains `
    -Pattern 'this.file_name.clone(),' `
    -Message "Probe completion must carry the resolved file name back to the visible input."
Assert-SourceContains `
    -Pattern 'input.set_value(resolved_file_name, window, cx);' `
    -Message "Probe completion must hydrate the visible file-name input."
Assert-SourceContains `
    -Pattern '.map(new_download_window_size_for_probe)' `
    -Message "The add window must choose its confirmed size from the typed probe result."
Assert-SourceContains `
    -Pattern 'PresentFixedWindowOverAnchor' `
    -Message "Successful probe must present a NEW final-size confirmation window through the native adapter."
Assert-SourceContains `
    -Pattern 'apply_new_download_window_resize_with_correlation(' `
    -Message "The add window must apply native resizing through platform host with a correlated generation."
Assert-SourceContains `
    -Pattern 'this.probe_generation != generation' `
    -Message "A stale probe generation must not resize a newer add window state."
Assert-SourceContains `
    -Pattern 'window_min_size: Some(new_download_window_size(false))' `
    -Message "The add window minimum must allow browser and retry probes to return to compact height."
Assert-SourceContains `
    -Pattern 'new_download_window_options(window_bounds)' `
    -Message "The add window must pass the tested compact minimum through the production GPUI window-open boundary."
$fixedOwner = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\browser_handoff_impl.rs') -Raw
$fixedView = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\new_download_view.rs') -Raw
foreach ($fixedContract in @('options.show = false', 'options.focus = false', 'fixed_source_is_current', 'FIXED_CONFIRMATION_IPC_TIMEOUT', 'cleanup_new_download_handoff', '重试打开确认')) {
    Assert-SourceContains -Pattern $fixedContract -Message "Fixed confirmation handoff must preserve hidden create, identity/cancel, bounded IPC and retry contracts."
}
$install = $fixedOwner.IndexOf('self.pending_new_download_handoff = Some(')
$dispatch = $fixedOwner.IndexOf('send_command_with_id(command, operation_id)')
if ($install -lt 0 -or $dispatch -le $install) { throw 'Fixed handoff pending ownership must be installed before dispatch.' }
if ($fixedView.Contains('NEW_DOWNLOAD_CONFIRMED_GATE_TIMEOUT') -or $fixedView.Contains('download.confirmed_gate.expired')) { throw 'Successful probes must not be erased by an in-place resize watchdog.' }
$finishStart = $fixedOwner.IndexOf('pub(crate) fn finish_new_download_handoff(')
$finishEnd = $fixedOwner.IndexOf('pub(crate) fn import_browser_blob(', $finishStart)
$finishBody = $fixedOwner.Substring($finishStart, $finishEnd - $finishStart)
if ($finishBody.Contains('window.activate_window()')) { throw 'Fixed hidden target must not replay GPUI initial_placement on activation.' }
Assert-SourceContains `
    -Pattern 'size: size(px(440.0), px(184.0))' `
    -Message "The progress window must remain compact and content-driven."
Assert-SourceContains `
    -Pattern 'size: size(px(440.0), px(196.0))' `
    -Message "The failed window must remain compact and content-driven."
Assert-SourceContains `
    -Pattern 'size: size(px(440.0), px(184.0))' `
    -Message "Ordinary completion must keep the collapsed progress geometry."
$taskLayout = Get-Content (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\task_window.rs') -Raw
$taskOwner = Get-Content (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\downloads\task_windows_impl.rs') -Raw
if ([regex]::Matches($taskLayout, 'size: size\(px\(440\.0\), px\(184\.0\)\)').Count -ne 2 -or
    -not $taskOwner.Contains('if previous_size != Some(next_size)') -or
    -not $taskOwner.Contains('window_min_size: Some(task_window_minimum_size())')) {
    throw 'Task completion must preserve same-size geometry and permit taller phases to collapse.'
}
Assert-SourceContains `
    -Pattern 'fn download_task_window_phase_layout(' `
    -Message "Task-window phase size and selector metadata must stay centralized."
Assert-SourceContains `
    -Pattern 'fn download_task_window_shell(' `
    -Message "Progress, failed and completed task windows must share one production skeleton."
Assert-SourceContains `
    -Pattern 'raise_terminal_window_without_activation' `
    -Message "A terminal download transition must return the window to the visible Z-order without stealing focus."
foreach ($requiredDialogShellContract in @(
    'pub(crate) struct DialogShellIds',
    'pub(crate) fn dialog_shell(',
    '.child(titlebar)',
    '.h(px(SETTINGS_FOOTER_HEIGHT))'
)) {
    if (-not $dialogComponent.Contains($requiredDialogShellContract)) {
        throw "Auxiliary download dialogs must share the design-system DialogShell geometry."
    }
}
if ($dialogComponent.Contains('.font_family(')) {
    throw 'Auxiliary download dialogs must inherit the configured UI font from the component root.'
}
$themeSource = Get-Content -Raw (Join-Path $RepositoryRoot 'rust\apps\nxtools-desktop\src\theme.rs')
foreach ($fontContract in @('pub(crate) fn set_ui_font_family(', 'theme.font_family = resolved;', 'component.font_family = family;')) {
    if (-not $themeSource.Contains($fontContract)) {
        throw 'The configured UI font must propagate through both font updates and theme changes.'
    }
}
foreach ($requiredDialogRole in @(
    'download-schedule-dialog-shell',
    'download-schedule-secondary-cancel-action',
    'download-schedule-primary-apply-action',
    'download-delete-dialog-shell',
    'download-delete-secondary-cancel-action',
    'download-delete-danger-confirm-action',
    'download-shutdown-dialog-shell',
    'download-shutdown-secondary-cancel-action',
    'download-shutdown-danger-now-action'
)) {
    Assert-SourceContains `
        -Pattern $requiredDialogRole `
        -Message "Schedule, delete and shutdown dialogs must expose stable shell and action roles."
}
foreach ($requiredDialogLifecycle in @(
    'application.close_schedule_window(cx)',
    'application.close_delete_confirmation(task_id, cx)',
    'application.confirm_delete_artifact(task_id, cx)',
    'application.cancel_shutdown_countdown(',
    'handle_native_delete_confirmation_close',
    'handle_native_schedule_window_close',
    'can_native_close_shutdown_countdown',
    'finish_native_close_shutdown_countdown'
)) {
    Assert-SourceContains `
        -Pattern $requiredDialogLifecycle `
        -Message "Auxiliary dialog actions must retain their owned close/cancel/confirm lifecycle."
}
Assert-NativeCloseDefersMutation `
    -MethodStart 'pub(crate) fn request_delete_confirmation(' `
    -MethodEnd 'fn close_delete_confirmation(' `
    -DeferredHandler 'handle_native_delete_confirmation_close'
Assert-NativeCloseDefersMutation `
    -MethodStart 'pub(crate) fn open_schedule_window(' `
    -MethodEnd 'fn close_schedule_window(' `
    -DeferredHandler 'handle_native_schedule_window_close'
Assert-NativeCloseDefersMutation `
    -MethodStart 'fn open_shutdown_countdown(' `
    -MethodEnd 'fn close_shutdown_window(' `
    -DeferredHandler 'finish_native_close_shutdown_countdown'
foreach ($requiredNativeCloseGuard in @(
    'self.delete_confirmation_windows.get(&task_id) == Some(&closing_window)',
    'self.schedule_window == Some(closing_window)',
    'countdown.correlation_id == correlation_id',
    'window.correlation_id == correlation_id'
)) {
    Assert-SourceContains `
        -Pattern $requiredNativeCloseGuard `
        -Message "Deferred auxiliary-dialog cleanup must remain guarded by the current window or correlation identity."
}
$deleteDialogStart = $source.IndexOf("impl Render for DeleteDownloadView")
$deleteDialogEnd = $source.IndexOf("pub(crate) enum NewDownloadInitialSource", $deleteDialogStart)
$shutdownDialogStart = $source.IndexOf("impl Render for DownloadShutdownCountdownView")
$shutdownDialogEnd = $source.IndexOf("pub(crate) struct DownloadSettingsInput", $shutdownDialogStart)
$scheduleDialogStart = $source.IndexOf("impl Render for DownloadScheduleView")
$scheduleDialogEnd = $source.IndexOf("fn schedule_trigger_row", $scheduleDialogStart)
if ($deleteDialogStart -lt 0 -or $deleteDialogEnd -le $deleteDialogStart -or
    $shutdownDialogStart -lt 0 -or $shutdownDialogEnd -le $shutdownDialogStart -or
    $scheduleDialogStart -lt 0 -or $scheduleDialogEnd -le $scheduleDialogStart) {
    throw "Could not isolate the three auxiliary download dialogs."
}
$deleteDialogBody = $source.Substring($deleteDialogStart, $deleteDialogEnd - $deleteDialogStart)
$shutdownDialogBody = $source.Substring($shutdownDialogStart, $shutdownDialogEnd - $shutdownDialogStart)
$scheduleDialogBody = $source.Substring($scheduleDialogStart, $scheduleDialogEnd - $scheduleDialogStart)
foreach ($dialogBody in @($deleteDialogBody, $shutdownDialogBody, $scheduleDialogBody)) {
    if (-not $dialogBody.Contains("dialog_shell(") -or
        -not $dialogBody.Contains("client_titlebar(") -or
        -not $dialogBody.Contains("dialog_footer(")) {
        throw "Every DL-27 auxiliary window must render through DialogShell, ClientTitlebar and DialogFooter."
    }
    if (-not $dialogBody.Contains("cx.defer(")) {
        throw "Auxiliary dialog callbacks must defer cross-entity application work."
    }
}
if (-not $deleteDialogBody.Contains("danger_hover_button_variant(theme, cx)") -or
    -not $shutdownDialogBody.Contains("danger_hover_button_variant(theme, cx)")) {
    throw "Delete and Shutdown Now must share the neutral-at-rest danger-hover action variant."
}
if ($deleteDialogBody.Contains(".danger()") -or $shutdownDialogBody.Contains(".primary()")) {
    throw "Destructive dialog actions must not use permanent danger or accent-primary styling."
}
if (-not $scheduleDialogBody.Contains("primary_action_button(") -or -not $scheduleDialogBody.Contains('download-schedule-primary-apply-action')) {
    throw "Schedule Apply must remain the sole accent-primary auxiliary-dialog action."
}
$completedStart = $source.IndexOf("if status == DownloadStatus::Completed")
$failedStart = $source.IndexOf("if status == DownloadStatus::Failed", $completedStart)
if ($completedStart -lt 0 -or $failedStart -le $completedStart) {
    throw "Could not isolate the completed download presentation."
}
$completedBody = $source.Substring($completedStart, $failedStart - $completedStart)
$completedActionStart = $source.IndexOf("enum CompletedTaskWindowAction")
$completedActionEnd = $source.IndexOf("pub(crate) fn download_window_titlebar", $completedActionStart)
if ($completedActionStart -lt 0 -or $completedActionEnd -le $completedActionStart) {
    throw "Could not isolate the completed action model."
}
$completedActionBody = $source.Substring($completedActionStart, $completedActionEnd - $completedActionStart)
foreach ($requiredActionContract in @(
    'const ALL: [Self; 3] = [Self::Drag, Self::Reveal, Self::Open]',
    'download-complete-secondary-drag-action',
    'download-complete-secondary-reveal-action',
    'download-complete-primary-open-action',
    'let spec = action.spec()',
    'action_button(spec.button_id, label)',
    'icon_action_button(spec.button_id, spec.icon, "拖拽下载文件")',
    'tooltip: Some("拖拽文件")',
    'tooltip_selector: Some("download-complete-drag-tooltip")',
    'primary_action_button(',
    'CompletedTaskWindowEmphasis::Outline => match spec.label',
    '.outline()'
)) {
    if (-not $completedActionBody.Contains($requiredActionContract)) {
        throw "The completed action hierarchy must derive its shared primary and secondary controls from one model."
    }
}
if ($completedActionBody.Contains("Delete")) {
    throw "Delete must not enter the completed-window action model."
}
$dragStart = $completedBody.IndexOf("CompletedTaskWindowAction::Drag")
$revealStart = $completedBody.IndexOf("CompletedTaskWindowAction::Reveal", $dragStart)
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
if (-not $platformAdapter.Contains("NativeFileDragCoordinator") -or
    -not $nativeFileDragCoordinator.Contains("NativeFileDragService.TryDragFile")) {
    throw "The Windows adapter must use the reference-compatible OLE file-drag source."
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
if (-not $nativeFileDrag.Contains("private static extern int DoDragDrop")) {
    throw "The native Shell drag loop must receive the visible GPUI source HWND."
}

if ($source.Contains('"等待链接探测"')) {
    throw "The initial add window must not reserve a blank probe card."
}

$newViewStart = $source.IndexOf("impl NewDownloadView {")
$requestStart = $source.IndexOf("    pub(crate) fn request(", $newViewStart)
if ($newViewStart -lt 0 -or $requestStart -le $newViewStart) {
    throw "Could not isolate the destination picker lifecycle."
}
$newViewBody = $source.Substring($newViewStart, $requestStart - $newViewStart)
if ($newViewBody.Contains("confirmed_request = None") -or $newViewBody.Contains("probe = None")) {
    throw "Changing only the local destination must not invalidate remote probe metadata."
}


$clientResize = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.ClientResize.cs') -Raw
$nativeResize = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.NativeMethods.cs') -Raw
if ($clientResize.Contains('ClientResizePostMessage') -or $clientResize.Contains('PostClientResizeMessage')) {
    throw 'Client-size synchronization must not queue a stale requested-size WM_SIZE payload.'
}
if (-not $clientResize.Contains('RefreshCurrentClientSize') -or -not $clientResize.Contains('EntryPoint = "SendMessageW"') -or
    $clientResize.Contains('SendMessageTimeoutW') -or -not $clientResize.Contains('ref ClientResizeWindowPosition')) {
    throw 'Current-client refresh must retain WINDOWPOS until owner-thread processing finishes, within fixed native workers.'
}
$commitBoundary = $nativeResize.IndexOf('beforeCommit?.Invoke();')
$retirementReadback = $nativeResize.IndexOf('var final = CaptureWindowPresentation(pending.Window);', $commitBoundary)
$retire = $nativeResize.IndexOf('var committed = stable', $commitBoundary)
if ($commitBoundary -lt 0 -or $retirementReadback -lt $commitBoundary -or $retire -lt $retirementReadback) {
    throw 'Centered resize must read the final native client at the pending-retirement boundary.'
}

Write-Host "Download UI lifecycle architecture gate passed."
