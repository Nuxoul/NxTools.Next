$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$dispatcherPath = Join-Path $root 'rust\apps\nxtools-desktop\src\platform_events.rs'
$source = Get-Content -Raw -LiteralPath $dispatcherPath
$quickLaunchPath = Join-Path $root 'rust\apps\nxtools-desktop\src\quick_launch.rs'
$quickLaunchSource = Get-Content -Raw -LiteralPath $quickLaunchPath
$panelRenderStart = $quickLaunchSource.IndexOf('impl Render for QuickLaunchPanel')
$panelRenderEnd = $quickLaunchSource.IndexOf('fn source_icon(', $panelRenderStart)
if ($panelRenderStart -lt 0 -or $panelRenderEnd -le $panelRenderStart) {
    throw 'Could not isolate the production Quick Launch panel renderer.'
}
$panelRender = $quickLaunchSource.Substring($panelRenderStart, $panelRenderEnd - $panelRenderStart)
foreach ($requiredPopupConstant in @(
    'const PANEL_WIDTH: f32 = 620.0',
    'const PANEL_HEIGHT: f32 = 410.0',
    'const SEARCH_HEADER_HEIGHT: f32 = 42.0',
    'const QUICK_LAUNCH_SEARCH_TEXT_SIZE: f32 = TEXT_BODY'
)) {
    if (-not $quickLaunchSource.Contains($requiredPopupConstant)) {
        throw 'Quick Launch must preserve the tokenized collapsed/expanded popup constants.'
    }
}
foreach ($requiredPopupRenderContract in @(
    'quick-launch-search-header',
    'quick-launch-results-surface',
    '.font_family(UI_FONT_FAMILY)',
    '.bg(theme.canvas)',
    '.text_color(theme.text)',
    '.border_color(theme.window_border)',
    '.border_color(theme.border_subtle)',
    '.text_color(theme.icon_muted)',
    '.line_height(px(LINE_HEIGHT_BODY))',
    '.when(self.results_visible'
)) {
    if (-not $panelRender.Contains($requiredPopupRenderContract)) {
        throw 'Quick Launch must preserve the tokenized collapsed/expanded popup contract.'
    }
}
if (-not $quickLaunchSource.Contains(
        'fn production_popup_keeps_search_only_collapsed_and_expands_one_stable_surface('
    )) {
    throw 'Quick Launch popup geometry requires a real production-window regression test.'
}
$searchInputStart = $panelRender.IndexOf('Input::new(&self.search_input)')
$searchInputEnd = $panelRender.IndexOf('.when(recent_button_visible', $searchInputStart)
if ($searchInputStart -lt 0 -or $searchInputEnd -le $searchInputStart) {
    throw 'Could not isolate the production Quick Launch search input.'
}
$searchInputRender = $panelRender.Substring($searchInputStart, $searchInputEnd - $searchInputStart)
foreach ($requiredSearchInputTypeRole in @(
    '.text_size(px(QUICK_LAUNCH_SEARCH_TEXT_SIZE))',
    '.line_height(px(LINE_HEIGHT_BODY))'
)) {
    if (-not $searchInputRender.Contains($requiredSearchInputTypeRole)) {
        throw 'Quick Launch search input must apply the shared Body type role directly.'
    }
}
$desktopMainPath = Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs'
$desktopMainSource = Get-Content -Raw -LiteralPath $desktopMainPath
$buildScriptPath = Join-Path $root 'scripts\build.ps1'
$buildScriptSource = Get-Content -Raw -LiteralPath $buildScriptPath
$manifestWriterPath = Join-Path $root 'scripts\write-release-binary-manifest.ps1'
$manifestWriterSource = Get-Content -Raw -LiteralPath $manifestWriterPath
$releaseManifestPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Generated\ReleaseBinaryManifest.g.cs'
$releaseManifestSource = Get-Content -Raw -LiteralPath $releaseManifestPath
$protocolPath = Join-Path $root 'rust\crates\nxtools-protocol\src\lib.rs'
$protocolSource = Get-Content -Raw -LiteralPath $protocolPath
$platformParserPath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformCommandParser.cs'
$platformParserSource = Get-Content -Raw -LiteralPath $platformParserPath
$platformPipePath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformPipeClient.cs'
$platformPipeSource = Get-Content -Raw -LiteralPath $platformPipePath
$windowsAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.cs'
$windowsAdapterSource = Get-Content -Raw -LiteralPath $windowsAdapterPath
$shellMenuPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ShellContextMenuSession.cs'
$shellMenuSource = Get-Content -Raw -LiteralPath $shellMenuPath
$shellIconPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ShellIconExtractor.cs'
$shellIconSource = Get-Content -Raw -LiteralPath $shellIconPath
$explorerAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ExplorerWindowAdapter.cs'
$explorerAdapterSource = if (Test-Path -LiteralPath $explorerAdapterPath) {
    Get-Content -Raw -LiteralPath $explorerAdapterPath
} else {
    ''
}
$fileDialogAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\FileDialogWindowAdapter.cs'
$fileDialogAdapterSource = if (Test-Path -LiteralPath $fileDialogAdapterPath) {
    Get-Content -Raw -LiteralPath $fileDialogAdapterPath
} else {
    ''
}
$desktopAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DesktopWindowAdapter.cs'
$desktopAdapterSource = if (Test-Path -LiteralPath $desktopAdapterPath) {
    Get-Content -Raw -LiteralPath $desktopAdapterPath
} else {
    ''
}
$directoryNavigationAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DirectoryNavigationAdapter.cs'
$directoryNavigationAdapterSource = if (Test-Path -LiteralPath $directoryNavigationAdapterPath) {
    Get-Content -Raw -LiteralPath $directoryNavigationAdapterPath
} else {
    ''
}
$thirdPartyAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ThirdPartyFileManagerAdapter.cs'
$thirdPartyAdapterSource = if (Test-Path -LiteralPath $thirdPartyAdapterPath) {
    Get-Content -Raw -LiteralPath $thirdPartyAdapterPath
} else {
    ''
}
$directoryOpusAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\DirectoryOpusFileManagerOperations.cs'
$directoryOpusAdapterSource = if (Test-Path -LiteralPath $directoryOpusAdapterPath) {
    Get-Content -Raw -LiteralPath $directoryOpusAdapterPath
} else {
    ''
}
$totalCommanderAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\TotalCommanderFileManagerOperations.cs'
$totalCommanderAdapterSource = if (Test-Path -LiteralPath $totalCommanderAdapterPath) {
    Get-Content -Raw -LiteralPath $totalCommanderAdapterPath
} else {
    ''
}
$oneCommanderAdapterPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\OneCommanderFileManagerOperations.cs'
$oneCommanderAdapterSource = if (Test-Path -LiteralPath $oneCommanderAdapterPath) {
    Get-Content -Raw -LiteralPath $oneCommanderAdapterPath
} else {
    ''
}
$boundedStaRunnerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\BoundedStaOperationRunner.cs'
$boundedStaRunnerSource = if (Test-Path -LiteralPath $boundedStaRunnerPath) {
    Get-Content -Raw -LiteralPath $boundedStaRunnerPath
} else {
    ''
}
$elevatedDirectoryNavigationPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ElevatedDirectoryNavigation.cs'
$elevatedDirectoryNavigationSource = if (Test-Path -LiteralPath $elevatedDirectoryNavigationPath) {
    Get-Content -Raw -LiteralPath $elevatedDirectoryNavigationPath
} else {
    ''
}
$elevatedHelperProcessBrokerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\ElevatedHelperProcessBroker.cs'
$elevatedHelperProcessBrokerSource = if (Test-Path -LiteralPath $elevatedHelperProcessBrokerPath) {
    Get-Content -Raw -LiteralPath $elevatedHelperProcessBrokerPath
} else {
    ''
}
$indexerServiceInstallerPath = Join-Path $root 'csharp\NxTools.PlatformHost\Windows\IndexerServiceInstaller.cs'
$indexerServiceInstallerSource = if (Test-Path -LiteralPath $indexerServiceInstallerPath) {
    Get-Content -Raw -LiteralPath $indexerServiceInstallerPath
} else {
    ''
}
$indexerServiceHostPath = Join-Path $root 'rust\apps\nxtools-indexer\src\windows_service_host.rs'
$indexerServiceHostSource = if (Test-Path -LiteralPath $indexerServiceHostPath) {
    Get-Content -Raw -LiteralPath $indexerServiceHostPath
} else {
    ''
}
$indexerElevatedBrokerPath = Join-Path $root 'rust\apps\nxtools-indexer\src\elevated_helper_broker.rs'
$indexerElevatedBrokerSource = if (Test-Path -LiteralPath $indexerElevatedBrokerPath) {
    Get-Content -Raw -LiteralPath $indexerElevatedBrokerPath
} else {
    ''
}
$localPipePath = Join-Path $root 'rust\crates\nxtools-ipc\src\local_pipe.rs'
$localPipeSource = if (Test-Path -LiteralPath $localPipePath) {
    Get-Content -Raw -LiteralPath $localPipePath
} else {
    ''
}
$platformCapabilitiesPath = Join-Path $root 'csharp\NxTools.PlatformHost\Protocol\PlatformCapabilities.cs'
$platformCapabilitiesSource = Get-Content -Raw -LiteralPath $platformCapabilitiesPath
$fileDialogSearchPath = Join-Path $root `
    'rust\apps\nxtools-desktop\src\quick_launch\file_dialog_search.rs'
$fileDialogSearchSource = if (Test-Path -LiteralPath $fileDialogSearchPath) {
    Get-Content -Raw -LiteralPath $fileDialogSearchPath
} else {
    ''
}
$autoQuickSwitchPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\AutoQuickSwitchTracker.cs'
$autoQuickSwitchSource = if (Test-Path -LiteralPath $autoQuickSwitchPath) {
    Get-Content -Raw -LiteralPath $autoQuickSwitchPath
} else {
    ''
}
$fileDialogRecognitionRetryPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\FileDialogRecognitionRetry.cs'
$fileDialogRecognitionRetrySource = if (
    Test-Path -LiteralPath $fileDialogRecognitionRetryPath
) {
    Get-Content -Raw -LiteralPath $fileDialogRecognitionRetryPath
} else {
    ''
}
$fileDialogSearchHostPath = Join-Path $root `
    'csharp\NxTools.PlatformHost\Windows\FileDialogSearchHost.cs'
$fileDialogSearchHostSource = if (Test-Path -LiteralPath $fileDialogSearchHostPath) {
    Get-Content -Raw -LiteralPath $fileDialogSearchHostPath
} else {
    ''
}
$ipcPath = Join-Path $root 'rust\crates\nxtools-ipc\src\lib.rs'
$ipcSource = Get-Content -Raw -LiteralPath $ipcPath
$indexerPath = Join-Path $root 'rust\apps\nxtools-indexer\src\lib.rs'
$indexerSource = Get-Content -Raw -LiteralPath $indexerPath
$finderPath = Join-Path $root 'rust\crates\nxtools-finder\src\lib.rs'
$finderSource = Get-Content -Raw -LiteralPath $finderPath

if ($source -notmatch 'quick_launch:\s*Entity<QuickLaunchApplication>') {
    throw @'
Quick-launch lifecycle regression: PlatformEventDispatcher must strongly own
QuickLaunchApplication. A WeakEntity-only graph drops the module before the
platform Connected event, so configure_quick_launch is never sent and the
global summon gesture appears to do nothing.
'@
}

Write-Host 'PASS platform dispatcher strongly owns quick-launch application'

if ($quickLaunchSource -match '(?s)fn handle_foreground_changed\s*\(.*?self\.close\(cx\)') {
    throw @'
Quick-launch lease regression: a foreground callback runs while
QuickLaunchApplication is already leased. QuickLaunchPanel must return a
dismissal decision to the application; it must not update the application
again from inside the nested panel update.
'@
}

if ($quickLaunchSource -match '(?s)fn handle_pointer_pressed\s*\(.*?self\.close\(cx\)') {
    throw @'
Quick-launch lease regression: a global-pointer callback runs while
QuickLaunchApplication is already leased. QuickLaunchPanel must return a
dismissal decision to the application; it must not update the application
again from inside the nested panel update.
'@
}

Write-Host 'PASS platform dismissal callbacks do not re-enter quick-launch application'

if ($quickLaunchSource -match '(?s)fn activate_selected\s*\(.*?self\.application\.update\(cx') {
    throw @'
Quick-launch lease regression: Enter is dispatched while QuickLaunchPanel is
leased. Executing the selected result must be deferred before the application
is updated, because execution writes status back into the same panel.
'@
}

if ($quickLaunchSource -match '(?s)fn close\s*\(.*?self\.application\s*\.update\(cx') {
    throw @'
Quick-launch lease regression: Escape is dispatched while QuickLaunchPanel is
leased. Hiding the popup must be deferred before the application is updated,
because application-level hide writes state back into the same panel.
'@
}

Write-Host 'PASS panel keyboard callbacks defer application side effects'

$panelCloseStart = $quickLaunchSource.IndexOf('fn close(&mut self, cx: &mut Context<Self>)')
$panelDismissStart = $quickLaunchSource.IndexOf(
    'fn should_dismiss_for_foreground(',
    $panelCloseStart
)
if ($panelCloseStart -lt 0 -or $panelDismissStart -le $panelCloseStart) {
    throw 'Quick-launch explicit-dismiss regression: panel close lifecycle could not be located.'
}
$panelCloseBody = $quickLaunchSource.Substring(
    $panelCloseStart,
    $panelDismissStart - $panelCloseStart
)
$dismissPlanStart = $quickLaunchSource.IndexOf('fn quick_launch_dismiss_commands(')
$actionLevelStart = $quickLaunchSource.IndexOf(
    '#[derive(Clone)]',
    $dismissPlanStart
)
if ($dismissPlanStart -lt 0 -or $actionLevelStart -le $dismissPlanStart) {
    throw 'Quick-launch explicit-dismiss regression: native dismissal plan could not be located.'
}
$dismissPlanBody = $quickLaunchSource.Substring(
    $dismissPlanStart,
    $actionLevelStart - $dismissPlanStart
)
if ($panelCloseBody -notmatch 'target\.window_handle' -or
    $panelCloseBody -notmatch 'hide_popup_and_restore' -or
    $dismissPlanBody -notmatch 'SetWindowVisibility' -or
    $dismissPlanBody -notmatch 'SetWindowTopmost' -or
    $dismissPlanBody -notmatch 'ActivateWindow' -or
    $quickLaunchSource -notmatch '(?s)fn toggle_panel.*?hide_popup_and_restore' -or
    $quickLaunchSource -notmatch 'escape_hides_the_popup_then_restores_the_captured_host_for_the_next_summon') {
    throw @'
Quick-launch explicit-dismiss regression: Escape and repeated-hotkey cancellation
must hide/unpin the popup and then restore the top-level HWND captured for that
session. Otherwise the next double-Ctrl request captures the hidden GPUI window
or another non-host window and loses Explorer/file-dialog directory actions.
Passive external-focus dismissal and successful actions must keep using the
non-restoring hide path so they do not steal focus from the user's new target.
'@
}

Write-Host 'PASS explicit dismissal restores the captured host for the next summon'

$keyboardMoveStart = $quickLaunchSource.IndexOf('fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>)')
$pointerMoveStart = $quickLaunchSource.IndexOf('fn pointer_moved_result(')
if ($keyboardMoveStart -lt 0 -or $pointerMoveStart -le $keyboardMoveStart) {
    throw 'Quick-launch keyboard-preview regression: keyboard selection callback boundaries could not be located.'
}
$keyboardMoveBody = $quickLaunchSource.Substring(
    $keyboardMoveStart,
    $pointerMoveStart - $keyboardMoveStart
)
if ($keyboardMoveBody -notmatch 'schedule_keyboard_selection_preview') {
    throw @'
Quick-launch keyboard-preview regression: moving selection with Up/Down must
schedule the same delayed preview as pointer hover for the newly selected
result. Updating only the selected index leaves keyboard navigation without a
preview window.
'@
}

Write-Host 'PASS keyboard selection schedules the selected result preview'

if ($protocolSource -notmatch 'SetWindowNoActivate' -or
    $platformParserSource -notmatch 'SetWindowNoActivate' -or
    $windowsAdapterSource -notmatch 'WsExNoActivate' -or
    $windowsAdapterSource -notmatch 'SetWindowNoActivate' -or
    $quickLaunchSource -notmatch 'PlatformCommand::SetWindowNoActivate') {
    throw @'
Quick-launch focus regression: the owned preview is interactive, so
focus:false and SW_SHOWNA are insufficient after a pointer press. Rust must
explicitly mark only the preview HWND as WS_EX_NOACTIVATE through the C#
Windows adapter, while the search panel remains activatable.
'@
}

Write-Host 'PASS preview native window is permanently non-activating'

$newPreviewStart = $quickLaunchSource.IndexOf('fn show_quick_launch_preview(')
$reusePreviewStart = $quickLaunchSource.IndexOf('fn stage_reused_quick_launch_preview(')
if ($newPreviewStart -lt 0 -or $reusePreviewStart -le $newPreviewStart) {
    throw 'Quick-launch first-preview regression: new-preview callback boundaries could not be located.'
}
$newPreviewBody = $quickLaunchSource.Substring(
    $newPreviewStart,
    $reusePreviewStart - $newPreviewStart
)
$boundsCommandIndex = $newPreviewBody.IndexOf('PlatformCommand::SetWindowBounds')
$visibilityCommandIndex = $newPreviewBody.IndexOf('PlatformCommand::SetWindowVisibility')
if ($boundsCommandIndex -lt 0 -or
    $visibilityCommandIndex -lt 0 -or
    $boundsCommandIndex -gt $visibilityCommandIndex) {
    throw @'
Quick-launch first-preview regression: a hidden GPUI Windows popup still has
CW_USEDEFAULT native bounds until GPUI applies its deferred placement. The C#
adapter must atomically apply the requested device-pixel bounds before the
first visibility command, otherwise the preview flashes at a temporary large
size on its first appearance.
'@
}

Write-Host 'PASS first preview applies final native bounds before becoming visible'

if ($newPreviewBody -notmatch 'self\.restore_quick_launch_panel_keyboard_focus\(cx\)') {
    throw @'
Quick-launch keyboard-focus regression: opening a second GPUI window may
change GPUI's active keyboard target even when the preview HWND has
WS_EX_NOACTIVATE. After the first preview window is created, the application
must explicitly return activation and the input focus handle to the primary
search panel.
'@
}
$focusRestoreStart = $newPreviewBody.IndexOf('fn restore_quick_launch_panel_keyboard_focus(')
if ($focusRestoreStart -lt 0) {
    throw 'Quick-launch keyboard-focus regression: focus restoration helper could not be located.'
}
$focusRestoreBody = $newPreviewBody.Substring($focusRestoreStart)
if ($focusRestoreBody -notmatch 'PlatformCommand::ActivateWindow' -or
    $focusRestoreBody -notmatch 'focus_search\(window, cx\)') {
    throw @'
Quick-launch keyboard-focus regression: restoration must reassert the primary
HWND through the C# Windows activation adapter and restore GPUI's search input
focus handle. Either half alone leaves one keyboard-routing layer stale.
'@
}

Write-Host 'PASS first preview returns GPUI keyboard focus to the search panel'

if ($windowsAdapterSource -notmatch '(?s)public WindowsCommandResult ActivateWindow\(ActivateWindowCommand command\)\s*=>\s*NativeMethods\.ActivateWindow\(\s*new IntPtr\(command\.WindowHandle\),\s*new IntPtr\(command\.WindowHandle\)\)') {
    throw @'
Quick-launch native-focus regression: foreground activation alone is not
keyboard focus. The GPUI activation command must ask the Windows adapter to
focus the primary HWND as well as make it foreground, otherwise a newly
created preview can leave arrow keys routed away from the result list.
'@
}

if ($windowsAdapterSource -match 'focusWindow\s*!=\s*target\s*&&' -or
    $windowsAdapterSource -notmatch '(?s)GetFocus\(\)\s*!=\s*focusWindow.*?SetFocus\(focusWindow\)') {
    throw @'
Quick-launch native-focus regression: NativeMethods.ActivateWindow must allow
the target top-level HWND itself to receive keyboard focus. Treating
focusWindow == target as a reason to skip SetFocus reports a false activation
success while Up/Down no longer reaches GPUI.
'@
}

Write-Host 'PASS window activation restores native keyboard focus to the GPUI panel'

$renderStagedStart = $quickLaunchSource.IndexOf('fn render_staged_quick_launch_preview(')
$requestIconsStart = $quickLaunchSource.IndexOf('fn request_icons(', $renderStagedStart)
if ($renderStagedStart -lt 0 -or $requestIconsStart -le $renderStagedStart) {
    throw 'Quick-launch preview regression: staged-preview renderer boundaries could not be located.'
}
$renderStagedBody = $quickLaunchSource.Substring(
    $renderStagedStart,
    $requestIconsStart - $renderStagedStart
)
if ($renderStagedBody -match 'window\.resize\(') {
    throw @'
Quick-launch preview focus regression: GPUI 0.2.2 implements Windows
Window::resize with SetWindowPos(SWP_NOMOVE) and omits SWP_NOACTIVATE. The
preview has a fixed size, so resizing the reused native preview steals
foreground focus from the result panel after keyboard navigation.
'@
}

Write-Host 'PASS reused fixed-size preview never invokes GPUI Windows resize'

$requestIconsStart = $quickLaunchSource.IndexOf('fn request_icons(')
$recordIconFailureStart = $quickLaunchSource.IndexOf(
    'fn record_icon_failure(',
    $requestIconsStart
)
if ($requestIconsStart -lt 0 -or $recordIconFailureStart -le $requestIconsStart) {
    throw 'Quick-launch icon request boundaries could not be located.'
}
$requestIconsBody = $quickLaunchSource.Substring(
    $requestIconsStart,
    $recordIconFailureStart - $requestIconsStart
)
if ($requestIconsBody -notmatch 'cx\.defer\(' -or
    $requestIconsBody -notmatch '(?s)cx\.defer\(.*?popup\s*\.panel\s*\.update') {
    throw @'
Quick-launch cached-icon reentrancy regression: request_icons can be called while
QuickLaunchPanel::run_search already owns the panel entity. Applying cached
icons with a synchronous popup.panel.update then triggers GPUI
double_lease_panic and aborts NxTools. The panel update must be deferred until
the current GPUI entity lease has ended.
'@
}

Write-Host 'PASS cached search icons defer panel updates past the active GPUI lease'

$presentedIconsStart = $quickLaunchSource.IndexOf('fn request_presented_result_icons(')
if ($pointerMoveStart -lt 0 -or $presentedIconsStart -le $pointerMoveStart) {
    throw 'Quick-launch pointer lifecycle regression: result-pointer callback boundaries could not be located.'
}
$pointerMoveBody = $quickLaunchSource.Substring(
    $pointerMoveStart,
    $presentedIconsStart - $pointerMoveStart
)
if ($pointerMoveBody -match 'self\.application\.update\(cx' -or
    $pointerMoveBody -notmatch 'schedule_application_update\(') {
    throw @'
Quick-launch pointer lease regression: a result-row mouse callback runs while
QuickLaunchPanel is already leased. Icon requests must be scheduled for the
next application turn; a synchronous application update can write cached
icons back into the same panel and terminate the process with double_lease_panic.
'@
}

Write-Host 'PASS result-pointer icon requests defer application side effects'

if ($quickLaunchSource -match '\.context_menu\s*\(') {
    throw @'
Quick-launch action-mode regression: result actions must render inside the
launcher state machine. A detached generic context menu changes focus,
keyboard routing, filtering and parent-state restoration.
'@
}

if ($quickLaunchSource -notmatch '(?s)MouseButton::Right.*?enter_action_mode' -or
    $quickLaunchSource -notmatch '(?s)key\.eq_ignore_ascii_case\("right"\).*?enter_action_mode') {
    throw @'
Quick-launch action-mode regression: result right-click and a bare Right key
at the query end must enter the same in-window action-mode transition.
'@
}

if ($quickLaunchSource -match '(?s)fn activate_selected_action\s*\(.*?self\.application\.update\(cx') {
    throw @'
Quick-launch action-mode lease regression: Enter in the action list is
dispatched while QuickLaunchPanel is leased. Execution must be deferred before
updating QuickLaunchApplication because it writes status back into the panel.
'@
}

Write-Host 'PASS result actions use the shared in-window action mode'

if ($protocolSource -notmatch 'DirectoryNavigationContext' -or
    $protocolSource -notmatch 'DirectoryNavigationHost' -or
    $protocolSource -notmatch 'NavigateDirectory' -or
    $quickLaunchSource -notmatch 'directory_navigation_context' -or
    $quickLaunchSource -notmatch 'QuickLaunchAction::SwitchDirectory' -or
    $quickLaunchSource -notmatch 'PlatformCommand::NavigateDirectory' -or
    $platformParserSource -notmatch 'ParseNavigateDirectory' -or
    $platformPipeSource -notmatch '"navigate_directory"\s*=>\s*platformAdapter\.NavigateDirectory' -or
    $windowsAdapterSource -notmatch 'DirectoryNavigationAdapter\.TryCaptureLocal\(target\)' -or
    $windowsAdapterSource -notmatch 'QuickLaunchDirectoryContextResolvedEvent' -or
    $windowsAdapterSource -notmatch 'DirectoryNavigationAdapter\.Navigate\(\s*command,' -or
    $directoryNavigationAdapterSource -notmatch 'ExplorerWindowAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogWindowAdapter\.TryCapture' -or
    $explorerAdapterSource -notmatch '9BA05972-F6A8-11CF-A442-00A0C90A8F39' -or
    $explorerAdapterSource -notmatch 'Navigate2' -or
    $quickLaunchSource -match 'Navigate2|9BA05972-F6A8-11CF-A442-00A0C90A8F39') {
    throw @'
Quick-launch Explorer-context regression: Rust must preserve the Explorer HWND
captured at summon and expose the context action, while only the C# Windows
adapter may enumerate Shell Windows and invoke Navigate2. Re-reading the live
foreground window at execution can redirect the action to the GPUI popup or a
different Explorer window.
'@
}

if ($explorerAdapterSource -notmatch 'IExplorerBrowserDispatch' -or
    $explorerAdapterSource -notmatch '\[DispId\(-515\)\]' -or
    $explorerAdapterSource -notmatch '\[DispId\(500\)\]' -or
    $explorerAdapterSource -match 'dynamic\)candidate\)\.HWND|dynamic\)window\)\.Navigate2') {
    throw @'
Quick-launch Explorer-COM regression: Windows 11 Shell Windows can return
0x8000FFFF for .NET dynamic name lookup of HWND. HWND and Navigate2 must use
the explicit IWebBrowser2 dispatch identifiers instead of dynamic member names.
'@
}

$explorerNavigateIndex = $explorerAdapterSource.IndexOf('operations.TryNavigate(')
$explorerActivateIndex = $explorerAdapterSource.IndexOf('operations.TryActivate(')
if ($explorerNavigateIndex -lt 0 -or
    $explorerActivateIndex -le $explorerNavigateIndex -or
    $explorerAdapterSource -notmatch 'WindowsPlatformAdapter\.TryActivateNativeWindow\(windowHandle\)') {
    throw @'
Quick-launch Explorer-repeat regression: a successful same-window Navigate2
must restore the captured Explorer HWND to the foreground before the command
completes. Otherwise the next summon captures a non-Explorer foreground and
the Switch Directory action disappears or cannot be dispatched a second time.
'@
}

if ($quickLaunchSource -notmatch 'popup_session_id' -or
    $quickLaunchSource -notmatch 'pending_action_targets_session' -or
    $quickLaunchSource -notmatch '(?s)fn show_existing_popup.*?popup_session_id\s*=\s*uuid::Uuid::new_v4' -or
    $quickLaunchSource -notmatch '(?s)fn hide_popup.*?popup_session_id\s*=\s*uuid::Uuid::new_v4') {
    throw @'
Quick-launch action-session regression: correlated Windows results can arrive
after the popup has been hidden and reopened. Pending actions must retain the
summon session id, and every show/hide transition must rotate it so an old
success cannot hide the new popup and an old failure cannot overwrite it.
'@
}

Write-Host 'PASS directory navigation preserves Explorer summon context, foreground and the Rust/C# boundary'

if ($protocolSource -notmatch 'FileDialog' -or
    $protocolSource -notmatch 'navigation_directory' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogHost' -or
    $directoryNavigationAdapterSource -notmatch 'FileDialogWindowAdapter\.Navigate' -or
    $quickLaunchSource -notmatch 'DirectoryNavigationHost::FileDialog' -or
    $quickLaunchSource -notmatch '(?s)menu\s*\.as_ref\(\).*?navigation_directory\.clone\(\).*?Path::new\(directory\)\.is_absolute\(\)' -or
    $quickLaunchSource -match 'fn directory_navigation_target' -or
    $shellMenuSource -notmatch 'ShellIconExtractor\.ResolveNavigationDirectory\(target\)' -or
    $shellMenuSource -notmatch 'navigationDirectory' -or
    $shellIconSource -notmatch 'ResolveNavigationDirectory' -or
    $shellIconSource -notmatch 'TryResolveShortcut' -or
    $shellIconSource -notmatch 'ResolveSystemToolPath' -or
    $quickLaunchSource -notmatch 'pending_action_owns_foreground' -or
    $quickLaunchSource -notmatch 'begin_window_activation\(popup\.native_window_handle\)' -or
    $fileDialogAdapterSource -notmatch '"#32770"' -or
    $fileDialogAdapterSource -notmatch '"Breadcrumb Parent"' -or
    $fileDialogAdapterSource -notmatch 'FileNameEditId' -or
    $fileDialogAdapterSource -notmatch 'hasFileTypeComboBox\s*\|=\s*controlId\s*==\s*FileTypeComboId' -or
    $fileDialogAdapterSource -notmatch 'WindowsPlatformAdapter\.TryActivateNativeWindow' -or
    $fileDialogAdapterSource -notmatch 'ResolveNavigationEdit' -or
    $fileDialogAdapterSource -notmatch 'SendInput' -or
    $fileDialogAdapterSource -notmatch 'GetGUIThreadInfo' -or
    $fileDialogAdapterSource -notmatch 'PostMessage\(navigationEdit, WmKeyDown' -or
    $fileDialogAdapterSource -notmatch 'WaitForDirectory' -or
    $fileDialogAdapterSource -notmatch 'CdmGetFolderPath' -or
    $fileDialogAdapterSource -notmatch 'ReadProcessMemory' -or
    $quickLaunchSource -notmatch 'navigation_guard_owns_foreground' -or
    $quickLaunchSource -notmatch 'FAILED_NAVIGATION_FOREGROUND_GUARD' -or
    $quickLaunchSource -match '#32770|Breadcrumb Parent|WmSetText') {
    throw @'
Quick-launch file-dialog regression: the summon request must freeze a strictly
recognized native file-dialog HWND, Rust must expose directory navigation only
through a FileDialog-host action, and C# must validate/write/focus/submit that
same dialog. GPUI/Rust may not own #32770 control inspection or Win32 messages.
'@
}

$fileDialogPathCaptureStart = $fileDialogAdapterSource.IndexOf('private static string? TryReadCurrentDirectory(')
$fileDialogClassReadStart = $fileDialogAdapterSource.IndexOf('private static string GetWindowClass(')
if ($fileDialogPathCaptureStart -lt 0 -or
    $fileDialogClassReadStart -le $fileDialogPathCaptureStart) {
    throw 'Quick-launch file-dialog hotkey regression: bounded path-capture helpers could not be located.'
}
$fileDialogPathCaptureBody = $fileDialogAdapterSource.Substring(
    $fileDialogPathCaptureStart,
    $fileDialogClassReadStart - $fileDialogPathCaptureStart
)
if ($fileDialogPathCaptureBody -match 'Directory\.Exists|File\.Exists') {
    throw @'
Quick-launch file-dialog hotkey regression: reading the dialog breadcrumb may
parse a syntactically absolute path but must not touch the filesystem. An
offline UNC path can otherwise block the double-Ctrl summon past its 60 ms
target even though the window message itself has a bounded timeout.
'@
}

Write-Host 'PASS native file-dialog navigation preserves host identity and the Rust/C# boundary'

if ($protocolSource -notmatch '(?s)DirectoryNavigationHost\s*\{.*?Desktop' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopHost' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopWindowAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'DesktopWindowAdapter\.Navigate' -or
    $desktopAdapterSource -notmatch 'GetShellWindow' -or
    $desktopAdapterSource -notmatch '"Progman"' -or
    $desktopAdapterSource -notmatch '"WorkerW"' -or
    $desktopAdapterSource -notmatch '"SHELLDLL_DefView"' -or
    $desktopAdapterSource -notmatch 'GetWindowThreadProcessId' -or
    $desktopAdapterSource -notmatch 'ProcessName\.Equals\("explorer"' -or
    $desktopAdapterSource -notmatch 'IsShellDesktopIdentity' -or
    $desktopAdapterSource -notmatch 'Process\.Start' -or
    $desktopAdapterSource -match 'Navigate2|ShellWindowsClassId' -or
    $quickLaunchSource -notmatch 'recent_directory_mode' -or
    $quickLaunchSource -notmatch 'recent_directory_navigation_action' -or
    $quickLaunchSource -notmatch 'schedule_captured_directory_record' -or
    $quickLaunchSource -notmatch 'schedule_directory_record' -or
    $quickLaunchSource -notmatch 'FinderCatalog::persist_directory_visit' -or
    $quickLaunchSource -notmatch 'apply_recorded_directory_visit' -or
    $quickLaunchSource -notmatch 'captured_directory_reaches_sqlite_after_the_popup_session_changes' -or
    $quickLaunchSource -notmatch '(?s)fn schedule_directory_record.*?background_executor\(\).*?FinderCatalog::persist_directory_visit' -or
    $quickLaunchSource -notmatch '(?s)fn request_presented_result_icons.*?self\.recent_directory_mode' -or
    $finderSource -notmatch 'quick_launch_directory_history' -or
    $finderSource -notmatch 'MAX_RECENT_DIRECTORIES' -or
    $finderSource -notmatch 'pub fn recent_directories' -or
    $finderSource -notmatch 'pub fn record_directory_visit' -or
    $finderSource -notmatch 'lexical_absolute_path' -or
    $desktopAdapterSource -match 'quick_launch_directory_history') {
    throw @'
Quick-launch desktop/recent-directory regression: C# must recognize only the
real Shell desktop and open its target through Shell rather than Explorer
Navigate2. Rust must own the bounded, persistent, deduplicated directory
history and GPUI must expose it only through an explicit recent-directory mode.
'@
}

Write-Host 'PASS desktop context and recent directories preserve host and ownership boundaries'

if ($protocolSource -notmatch '(?s)DirectoryNavigationHost\s*\{.*?ThirdPartyFileManager' -or
    $protocolSource -notmatch '(?s)DirectoryNavigationContext\s*\{.*?host_context_id' -or
    $protocolSource -notmatch '(?s)NavigateDirectoryCommand\s*\{.*?host_context_id' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerHost' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerAdapter\.TryCapture' -or
    $directoryNavigationAdapterSource -notmatch 'ThirdPartyFileManagerAdapter\.Navigate' -or
    $thirdPartyAdapterSource -notmatch 'IThirdPartyFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'ThirdPartyFileManagerSessionRegistry' -or
    $thirdPartyAdapterSource -notmatch 'Guid\.NewGuid\(\)' -or
    $thirdPartyAdapterSource -notmatch 'HostContextId' -or
    $thirdPartyAdapterSource -notmatch 'DirectoryOpusFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'TotalCommanderFileManagerOperations' -or
    $thirdPartyAdapterSource -notmatch 'OneCommanderFileManagerOperations' -or
    $directoryOpusAdapterSource -notmatch '"dopus\.lister"' -or
    $directoryOpusAdapterSource -notmatch '"dopus\.filedisplaycontainer"' -or
    $directoryOpusAdapterSource -notmatch 'SendMessageTimeout' -or
    $totalCommanderAdapterSource -notmatch '"TTOTAL_CMD"' -or
    $totalCommanderAdapterSource -notmatch 'GetRequestWide' -or
    $totalCommanderAdapterSource -notmatch 'ChangeDirectory' -or
    $totalCommanderAdapterSource -notmatch 'SendMessageTimeout' -or
    $oneCommanderAdapterSource -notmatch '"CurrentPathGet"' -or
    $oneCommanderAdapterSource -notmatch '"CurrentPathSet"' -or
    $oneCommanderAdapterSource -notmatch 'focusWindowHandle' -or
    $oneCommanderAdapterSource -notmatch 'AutomationElement\.FromHandle' -or
    $oneCommanderAdapterSource -match '(?s)CapturePane\(.*?AutomationElement\.FocusedElement.*?private static OneCommanderNavigationResult' -or
    $oneCommanderAdapterSource -notmatch 'BoundedStaOperationRunner\.TryRun' -or
    $boundedStaRunnerSource -notmatch 'completed\.Wait\(timeout\)' -or
    $boundedStaRunnerSource -notmatch 'cancellation\.Cancel\(\)' -or
    $boundedStaRunnerSource -notmatch 'ActiveOperations' -or
    $thirdPartyAdapterSource -notmatch '_sessionOrderNodes' -or
    $thirdPartyAdapterSource -notmatch 'RemoveSession' -or
    $directoryOpusAdapterSource -match 'containers\[0\]' -or
    $quickLaunchSource -notmatch 'DirectoryNavigationHost::ThirdPartyFileManager' -or
    $quickLaunchSource -notmatch 'host_context_id' -or
    $quickLaunchSource -match 'DirectoryOpus|TotalCommander|OneCommander') {
    throw @'
Quick-launch third-party file-manager regression: Rust must expose one generic
third_party_file_manager host while C# owns strict Directory Opus, Total
Commander and OneCommander recognition, request-scoped active-pane capture and
bounded native/UIA navigation. The opaque pane token must survive Rust
orchestration unchanged, and adapters must never fall back to the first pane.
Product-specific Windows details must not leak into Rust/GPUI.
'@
}

Write-Host 'PASS third-party file managers preserve active panes and the Rust/C# boundary'

if ($protocolSource -notmatch 'ElevatedDirectoryHelperRequested' -or
    $protocolSource -notmatch 'EnsureElevatedDirectoryHelper' -or
    $protocolSource -notmatch 'QuickLaunchDirectoryContextResolved' -or
    $quickLaunchSource -notmatch 'elevated_directory_helper_command' -or
    $quickLaunchSource -notmatch 'acknowledged_elevated_helper_pipe' -or
    $quickLaunchSource -notmatch 'elevated_helper_status_matches_request' -or
    $quickLaunchSource -notmatch 'directory_context_resolution_matches' -or
    $quickLaunchSource -notmatch 'REQUEST_MAX_AGE_MS' -or
    $windowsAdapterSource -notmatch 'QuickLaunchDirectoryContextResolvedEvent' -or
    $windowsAdapterSource -notmatch 'IsQuickLaunchContextRequestCurrent' -or
    $windowsAdapterSource -notmatch 'Task\.Run' -or
    $directoryNavigationAdapterSource -notmatch 'ElevatedDirectoryNavigationClient\.IsElevatedContext' -or
    $directoryNavigationAdapterSource -notmatch 'RequiresElevatedHelper' -or
    $elevatedDirectoryNavigationSource -notmatch 'PipeOptions\.CurrentUserOnly' -or
    $elevatedDirectoryNavigationSource -notmatch 'RandomNumberGenerator\.GetBytes\(32\)' -or
    $elevatedDirectoryNavigationSource -notmatch 'GetNamedPipeClientProcessId' -or
    $elevatedDirectoryNavigationSource -notmatch 'ProcessIdToSessionId' -or
    $elevatedDirectoryNavigationSource -notmatch 'TokenElevation' -or
    $elevatedDirectoryNavigationSource -notmatch 'ElevatedDirectoryNavigationSessionRegistry' -or
    $elevatedDirectoryNavigationSource -notmatch 'BoundedTextFrameReader' -or
    $elevatedDirectoryNavigationSource -notmatch 'MaximumRendezvousAttempts' -or
    $elevatedDirectoryNavigationSource -notmatch '_sessionOrderNodes' -or
    $elevatedDirectoryNavigationSource -notmatch 'foregroundProvider\.Capture' -or
    $elevatedDirectoryNavigationSource -notmatch 'PreserveRequestedFocus' -or
    $elevatedDirectoryNavigationSource -notmatch 'terminateAfterResponse' -or
    $elevatedDirectoryNavigationSource -notmatch 'response\.Fatal' -or
    $explorerAdapterSource -notmatch 'BoundedStaOperationRunner\.TryRun' -or
    $explorerAdapterSource -notmatch 'Fatal: true' -or
    $explorerAdapterSource -notmatch 'cancellationToken\.ThrowIfCancellationRequested' -or
    $elevatedHelperProcessBrokerSource -notmatch 'WTSQueryUserToken' -or
    $elevatedHelperProcessBrokerSource -notmatch 'CreateProcessAsUser' -or
    $elevatedHelperProcessBrokerSource -notmatch 'winsta0\\default' -or
    $localPipeSource -notmatch 'GetNamedPipeClientProcessId' -or
    $localPipeSource -notmatch 'client_session_id' -or
    $indexerServiceHostSource -notmatch 'client_process_id' -or
    $indexerServiceHostSource -notmatch 'client_session_id' -or
    $indexerElevatedBrokerSource -notmatch 'authorized_packaged_desktop' -or
    $indexerElevatedBrokerSource -notmatch 'require_packaged_desktop' -or
    $indexerElevatedBrokerSource -notmatch 'registered_hash_matches' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsDesktopSha256' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsDesktopPath' -or
    $indexerElevatedBrokerSource -notmatch 'NxToolsPlatformHostSha256' -or
    $indexerElevatedBrokerSource -notmatch 'packaged_platform_host_path' -or
    $indexerElevatedBrokerSource -notmatch 'BROKER_TIMEOUT' -or
    $indexerElevatedBrokerSource -notmatch 'try_wait\(\)' -or
    $indexerServiceInstallerSource -notmatch 'DesktopHashValueName' -or
    $indexerServiceInstallerSource -notmatch 'PlatformHostHashValueName' -or
    $indexerServiceInstallerSource -notmatch 'DesktopPathValueName' -or
    $indexerServiceInstallerSource -notmatch 'ProtectedVendorDirectory' -or
    $indexerServiceInstallerSource -notmatch 'ProtectedServiceDirectory' -or
    $indexerServiceInstallerSource -notmatch 'HardenServiceDirectory' -or
    $indexerServiceInstallerSource -notmatch 'TrustedBinary\.Open' -or
    $indexerServiceInstallerSource -notmatch 'ReleaseBinaryManifest\.IndexerSha256' -or
    $indexerServiceInstallerSource -notmatch 'ReleaseBinaryManifest\.DesktopSha256' -or
    $indexerServiceInstallerSource -notmatch 'PinCurrentProcessImage' -or
    $indexerServiceInstallerSource -notmatch 'DeploymentSourceMatchesDestination' -or
    $indexerServiceInstallerSource -notmatch 'FileFlagOpenReparsePoint' -or
    $indexerServiceInstallerSource -notmatch 'GetNamedSecurityInfo' -or
    $indexerServiceInstallerSource -notmatch '"/reset"' -or
    $indexerServiceInstallerSource -notmatch 'icacls\.exe' -or
    $desktopMainSource -notmatch 'launch_protected_desktop_after_current_exit' -or
    $desktopMainSource -notmatch 'protected_package_matches_release' -or
    $desktopMainSource -notmatch 'NxTools\.Indexer\.exe' -or
    $desktopMainSource -notmatch 'NxTools\.PlatformHost\.exe' -or
    $oneCommanderAdapterSource -notmatch 'OneCommanderFocusTracker' -or
    $oneCommanderAdapterSource -notmatch 'AutomationElement\.FocusedElement' -or
    $oneCommanderAdapterSource -notmatch 'Fatal: true' -or
    $elevatedDirectoryNavigationSource -notmatch 'capture\.Failure\?\.Fatal' -or
    $releaseManifestSource -notmatch 'IndexerSha256' -or
    $releaseManifestSource -notmatch 'DesktopSha256' -or
    $manifestWriterSource -notmatch 'Get-FileHash' -or
    $buildScriptSource -notmatch 'write-release-binary-manifest\.ps1' -or
    $elevatedDirectoryNavigationSource -notmatch 'IsApprovedProtectedPlatformHost' -or
    $indexerElevatedBrokerSource -match 'runas') {
    throw @'
Quick-launch elevated-navigation regression: the medium-integrity Rust/GPUI
desktop must remain unelevated. A random current-user pipe and 256-bit token
must rendezvous with a fixed packaged C# helper launched by the LocalSystem
service in the actual client session. Both service and platform host must
authenticate real pipe-client identity, high-integrity capture must happen in
the helper, elevated contexts must remain opaque one-shot tokens, and all
cross-process waits must be bounded. No caller-controlled executable or
per-action runas fallback is allowed.
'@
}

Write-Host 'PASS elevated navigation preserves session identity, least privilege and bounded waits'

if ($platformCapabilitiesSource -notmatch 'directory_navigation' -or
    $ipcSource -notmatch 'directory_navigation' -or
    $platformCapabilitiesSource -match 'explorer_navigation|file_dialog_navigation' -or
    $ipcSource -match 'explorer_navigation|file_dialog_navigation') {
    throw @'
Quick-launch directory-navigation capability regression: Explorer, native
file dialogs and future third-party adapters share one tagged directory
navigation contract. Both endpoints must advertise that single capability;
host-specific wire capabilities would force Rust to know every Windows host.
'@
}

Write-Host 'PASS directory navigation uses one extensible tagged capability'

if ($protocolSource -notmatch 'QuickLaunchAutoSwitchRequested' -or
    $quickLaunchSource -notmatch 'auto_quick_switch_command' -or
    $autoQuickSwitchSource -notmatch 'KnownDialogLimit' -or
    $autoQuickSwitchSource -notmatch 'IsCurrent' -or
    $fileDialogRecognitionRetrySource -notmatch 'MaxAttempts|_maxAttempts' -or
    $windowsAdapterSource -notmatch 'FileDialogRecognitionTimerId' -or
    $windowsAdapterSource -notmatch 'IsPotentialHost' -or
    $windowsAdapterSource -notmatch 'RetryFileDialogRecognition' -or
    $windowsAdapterSource -notmatch 'TryGetCurrentDirectory' -or
    $windowsAdapterSource -notmatch 'QuickLaunchAutoSwitchRequestedEvent') {
    throw @'
Quick-launch automatic-switch regression: the Windows adapter must recognize
a previously known file-dialog -> Explorer -> same-dialog round trip, freeze
the Explorer path off the foreground hook, preserve that source while a newly
created dialog finishes initializing, and publish a generation-scoped request.
Rust must retain settings/freshness validation and issue the shared directory
navigation command instead of letting C# navigate implicitly.
'@
}

Write-Host 'PASS automatic quick switch preserves the Rust/C# ownership boundary'

if ($protocolSource -notmatch 'QuickLaunchFileDialogSearchHostChanged' -or
    $protocolSource -notmatch 'OpenFileInDialog' -or
    $platformCapabilitiesSource -notmatch 'file_dialog_search' -or
    $fileDialogSearchSource -notmatch 'WindowKind::PopUp' -or
    $fileDialogSearchSource -notmatch 'SetWindowOwner' -or
    $fileDialogSearchSource -notmatch 'IndexSearchContext' -or
    $fileDialogSearchSource -notmatch 'preferred_directory' -or
    $fileDialogSearchSource -notmatch 'file_dialog_default_source' -or
    $fileDialogSearchSource -notmatch 'Ctrl\+G' -or
    $fileDialogSearchSource -notmatch 'OpenFileInDialog' -or
    $fileDialogSearchHostSource -notmatch 'GetWindowRect' -or
    $fileDialogSearchHostSource -notmatch 'DwmGetWindowAttribute' -or
    $fileDialogSearchHostSource -notmatch 'DwmExtendedFrameBounds' -or
    $windowsAdapterSource -notmatch 'EventObjectCreate' -or
    $windowsAdapterSource -notmatch 'EventObjectShow' -or
    $windowsAdapterSource -notmatch 'ThirdPartyFileManagerHost' -or
    $windowsAdapterSource -notmatch 'EventObjectLocationChange') {
    throw @'
Quick-launch file-dialog search regression: C# may publish only exact HWND,
visible DWM placement/DPI/current-directory/latest-source facts and execute
native commands. The attached, reusable search surface, global query with
preferred-directory ranking, default Ctrl+G row and all visible rendering must
remain in Rust/GPUI, owned by the captured dialog. Child create/show events must
re-arm delayed recognition instead of waiting for a foreground round trip.
'@
}

Write-Host 'PASS file-dialog search remains a GPUI surface with exact native hosting'

if ($protocolSource -notmatch 'QueryShellActions' -or
    $protocolSource -notmatch 'InvokeShellAction' -or
    $platformParserSource -notmatch 'QueryShellActions' -or
    $platformParserSource -notmatch 'InvokeShellAction' -or
    -not (Test-Path -LiteralPath $shellMenuPath) -or
    $quickLaunchSource -notmatch 'QuickLaunchAction::Shell') {
    throw @'
Quick-launch action completeness regression: Listary-style action mode must
append the selected target's dynamic Windows Shell commands after first-class
actions. A fixed four-item array omits application-specific verbs and Shell
submenus, so the right-click action list is visibly incomplete.
'@
}

Write-Host 'PASS result actions append dynamic Windows Shell commands'

if ($protocolSource -notmatch 'QueryShellSubmenu' -or
    $protocolSource -notmatch 'icon_path' -or
    $protocolSource -notmatch 'submenu_id' -or
    $platformParserSource -notmatch 'QueryShellSubmenu' -or
    $shellMenuSource -notmatch 'IconPath' -or
    $shellMenuSource -notmatch 'SubmenuId' -or
    $shellMenuSource -notmatch 'ShellMenuIconStager\.Stage\(info\.ItemBitmap\)' -or
    $quickLaunchSource -notmatch 'icon_path' -or
    $shellMenuSource -match 'EnumerateMenu\(info\.SubMenu') {
    throw @'
Quick-launch Shell presentation regression: Listary reads each root item's
hbmpItem and enumerates a submenu only when the user enters that submenu.
The platform protocol must carry staged operation icons and an opaque submenu
id; eagerly walking every submenu makes the first action list slow and leaving
the icon out produces the empty left gutter visible in the product.
'@
}

Write-Host 'PASS Shell action icons are real and submenu enumeration is lazy'

if ($quickLaunchSource -match '正在交给 Windows 打开' -or
    $quickLaunchSource -match 'fn set_launching\s*\(') {
    throw @'
Quick-launch feedback regression: successful dispatch must not add a persistent
"opening" strip to the popup. Only actionable failures may use the compact
error overlay; successful execution is confirmed by the target application.
'@
}

Write-Host 'PASS successful quick-launch dispatch does not paint a status strip'

$statusRenderStart = $quickLaunchSource.IndexOf('.when_some(self.status.clone(), |panel, status|')
$sourceIconStart = $quickLaunchSource.IndexOf('fn source_icon(')
if ($statusRenderStart -lt 0 -or $sourceIconStart -le $statusRenderStart) {
    throw 'Quick-launch layout regression: the status rendering block could not be located.'
}
$statusRender = $quickLaunchSource.Substring(
    $statusRenderStart,
    $sourceIconStart - $statusRenderStart
)
if ($statusRender -notmatch '\.absolute\(\)') {
    throw @'
Quick-launch layout regression: transient launch/error status must be an
absolute overlay. An in-flow status row steals height from the result list as
soon as an item is clicked, making the panel appear to shrink.
'@
}

Write-Host 'PASS actionable error status does not change quick-launch layout height'

if ($quickLaunchSource -notmatch '(?s)background\s*\.spawn\(async move \{ scan_application_roots\(&roots\) \}\)') {
    throw @'
Quick-launch refresh regression: filesystem catalog scanning must run on the
background executor. Running it inline blocks popup activation and typing.
'@
}

if ($quickLaunchSource -notmatch '(?s)fn finish_catalog_refresh.*?search_input\.read\(cx\).*?replace_results') {
    throw @'
Quick-launch refresh regression: a completed snapshot must rerun only the
query currently visible in the popup and replace that result set in place.
'@
}

Write-Host 'PASS application refresh stays off the UI thread and reapplies the current query'

if ($quickLaunchSource -notmatch 'fn request_presented_result_icons' -or
    $quickLaunchSource -notmatch '(?s)uniform_list\(.*?visible_range.*?request_presented_result_icons\(visible_range') {
    throw @'
Quick-launch icon-loading regression: every result row that becomes visible
through initial layout, wheel scrolling, scrollbar dragging or keyboard
navigation must request its Shell icon without waiting for pointer hover. The
virtual list's rendered range is the presentation boundary for all four paths.
'@
}

Write-Host 'PASS visible quick-launch rows request icons without pointer hover'

if ($shellIconSource -notmatch 'ShgfiUseFileAttributes' -or
    $shellIconSource -notmatch 'FileAttributeNormal' -or
    $shellIconSource -notmatch 'Path\.GetExtension') {
    throw @'
Quick-launch file-type icon regression: stale indexed paths must fall back to
the Windows icon registered for their extension. Requiring File.Exists leaves
versioned application/cache results blank even though the row is presented.
'@
}

Write-Host 'PASS stale file paths retain their Windows file-type icon'

if ($quickLaunchSource -notmatch 'SEARCH_INPUT_DEBOUNCE' -or
    $quickLaunchSource -notmatch 'query_requests_file_index' -or
    $indexerSource -notmatch 'struct SearchCandidate' -or
    $indexerSource -notmatch 'base_name_mask' -or
    $indexerSource -notmatch 'name_mask_may_match' -or
    $indexerSource -notmatch 'score_name' -or
    $indexerSource -notmatch 'normalized_path_separators') {
    throw @'
Quick-launch query-efficiency regression: rapid input must be coalesced, every
non-empty query must retain full indexed-file semantics, normal name searches
must use the compact mask/name prefilter before materializing cold paths, and
the scan must retain borrowed candidates instead of allocating owned paths for
every match.
'@
}

Write-Host 'PASS quick-launch input and MFT candidate work stay bounded'
