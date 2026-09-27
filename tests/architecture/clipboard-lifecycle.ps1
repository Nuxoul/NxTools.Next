param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
)

$ErrorActionPreference = "Stop"

$sourcePath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\mod.rs"
$source = Get-Content -LiteralPath $sourcePath -Raw
$handlerStart = $source.IndexOf('pub(crate) fn handle_command_result(')
$hotKeyRegistrationPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\ClipboardHotKeyRegistration.cs"
$handlerEnd = $source.IndexOf('pub(crate) fn handle_hide_failure(', $handlerStart)
if ($handlerStart -lt 0 -or $handlerEnd -le $handlerStart) {
    throw "Could not isolate the clipboard command-result handler."
}

$handler = $source.Substring($handlerStart, $handlerEnd - $handlerStart)
if ($handler -match 'self\s*\.application\s*\.update\(cx') {
    throw @'
Clipboard action completion runs while ClipboardApplication is already leased by
the platform notification dispatcher. It must not synchronously update the same
application or GPUI will terminate with double_lease_panic / 0xc0000409.
'@
}
if (-not $handler.Contains('mark_used_async(item_id') -or
    -not $handler.Contains('panel.refresh_entries(cx)')) {
    throw "Clipboard usage persistence and copy-only refresh must remain worker-owned and deferred."
}
if ($handler.Contains('self.refresh_entries(cx)')) {
    throw "Copy-only completion must not synchronously query ClipboardApplication while it is leased."
}

Write-Host "Clipboard command-result lifecycle architecture gate passed."

$clipboardPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\clipboard.rs"
$clipboardUiPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\mod.rs"
$clipboardUiTestsPath = Join-Path $RepositoryRoot "rust\apps\nxtools-desktop\src\ui\tests.rs"
$clipboardHistoryPath = Join-Path $RepositoryRoot "rust\crates\nxtools-clipboard\src\lib.rs"
$protocolPath = Join-Path $RepositoryRoot "rust\crates\nxtools-protocol\src\lib.rs"
$eventWindowPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\WindowsPlatformAdapter.EventWindow.cs"
$debouncerPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost\Windows\ClipboardHotKeyDebouncer.cs"
$platformTestsPath = Join-Path $RepositoryRoot "csharp\NxTools.PlatformHost.Tests\Program.cs"

$clipboardSource = Get-Content -LiteralPath $clipboardPath -Raw
$clipboardUiSource = Get-Content -LiteralPath $clipboardUiPath -Raw
$clipboardUiTests = Get-Content -LiteralPath $clipboardUiTestsPath -Raw
$clipboardHistorySource = Get-Content -LiteralPath $clipboardHistoryPath -Raw
$protocolSource = Get-Content -LiteralPath $protocolPath -Raw
$eventWindowSource = Get-Content -LiteralPath $eventWindowPath -Raw
$debouncerSource = Get-Content -LiteralPath $debouncerPath -Raw
$platformTestsSource = Get-Content -LiteralPath $platformTestsPath -Raw
$hotKeyRegistrationSource = Get-Content -LiteralPath $hotKeyRegistrationPath -Raw

foreach ($requiredSourceContract in @(
    'struct PopupSession',
    'struct PreviewSession',
    'PopupLifecycle',
    'pub(crate) fn dismiss_preview',
    'pub(crate) fn hide_popup',
    'fn handle_preview_stage_result'
)) {
    if (-not $clipboardSource.Contains($requiredSourceContract)) {
        throw "Clipboard popup/preview lifecycle contract missing: $requiredSourceContract"
    }
}

foreach ($requiredDiagnosticContract in @(
    'PendingPopupVisibility',
    'clipboard.popup.opened',
    'clipboard.popup.close_requested',
    'clipboard.popup.close_completed',
    'clipboard.popup.visibility_result_stale',
    'clipboard.popup.close_failed',
    'PopupCloseReason::Foreground',
    'PopupCloseReason::Pointer'
)) {
    $diagnosticSources = $clipboardSource + $clipboardUiSource
    if (-not $diagnosticSources.Contains($requiredDiagnosticContract)) {
        throw "Clipboard popup diagnostic contract missing: $requiredDiagnosticContract"
    }
}

foreach ($requiredUiContract in @(
    'pub(crate) fn handle_foreground_changed',
    'pub(crate) fn handle_pointer_pressed',
    'schedule_entity_update',
    'mark_used_async(item_id'
)) {
    if (-not $clipboardUiSource.Contains($requiredUiContract)) {
        throw "Clipboard UI lifecycle contract missing: $requiredUiContract"
    }
}

foreach ($requiredHistoryContract in @(
    'fn non_adjacent_duplicate_reuses_and_revives_the_existing_record()',
    'fn equal_visible_text_with_different_native_formats_stays_distinct()',
    'pub fn mark_used(',
    'pub fn cleanup(',
    'retention_days.clamp(1, 3650)',
    'is_favorite = 0',
    'is_pinned = 0'
)) {
    if (-not $clipboardHistorySource.Contains($requiredHistoryContract)) {
        throw "Clipboard history identity/retention contract missing: $requiredHistoryContract"
    }
}

foreach ($requiredPointerContract in @(
    'sampled_at_unix_ms',
    'event_sequence',
    'PointerPressContext'
)) {
    if (-not $protocolSource.Contains($requiredPointerContract)) {
        throw "Pointer event sequencing contract missing: $requiredPointerContract"
    }
}

foreach ($requiredHotKeyContract in @(
    'ClipboardHotKeyRegistration',
    'SetClipboardHotKeyRecordingCommand',
    '_clipboardHotKeyDebouncer.TryAccept(Environment.TickCount64)',
    'ClipboardHotKeyDebouncer.DebounceMilliseconds == 350'
)) {
    $sourceToCheck = if ($requiredHotKeyContract.Contains('Debouncer.DebounceMilliseconds')) {
        $platformTestsSource
    } elseif ($requiredHotKeyContract.Contains('debouncer')) {
        $eventWindowSource
    } else {
        $eventWindowSource + $debouncerSource
    }
    if (-not $sourceToCheck.Contains($requiredHotKeyContract)) {
        throw "Clipboard native hotkey contract missing: $requiredHotKeyContract"
    }
}

if ($hotKeyRegistrationSource.Contains('SetWindowsHookEx') -or
    -not $hotKeyRegistrationSource.Contains('BeginRecording()') -or
    -not $hotKeyRegistrationSource.Contains('CancelRecording()')) {
    throw "Clipboard hotkey registration must not install a permanent low-level hook and must expose an explicit recording lifecycle."
}

foreach ($requiredRegression in @(
    'external_foreground_window_hides_open_popup_but_paste_activation_does_not',
    'transient_text_services_hosts_never_hide_the_panel',
    'popup_and_preview_are_exempt_but_other_windows_hide_after_opening_grace',
    'same_process_foreground_window_hides_open_popup_but_popup_preview_and_paste_do_not',
    'pointer_press_outside_the_panel_dismisses_it_even_in_the_app',
    'pointer_press_sampled_before_popup_session_does_not_dismiss_new_popup',
    'confirmed_clipboard_action_marks_the_item_used_before_the_panel_closes',
    'popup_close_diagnostics_keep_reason_session_and_native_correlation_fields',
    'HotKeyRecordingReleasesAndRestoresShortcut',
    'ClipboardHotKeyUses350MsDebounce',
    'empty_capture_with_native_formats_is_ignored',
    'EmptyClipboardPayloadsAreIgnored',
    'plain_text_image_action_suppression_matches_the_written_path',
    'clipboard_category_editor_consumes_pointer_events_before_history_rows',
    'PureImageFormatsGainBothFallbacks',
    'RichImageTargetOmitsFileDropFormat',
    'PasteRetriesTransientTargetActivation'
)) {
    $regressionSources = $clipboardSource + $clipboardUiTests + $clipboardHistorySource + $platformTestsSource
    if (-not $regressionSources.Contains($requiredRegression)) {
        throw "Clipboard reference regression evidence missing: $requiredRegression"
    }
}

Write-Host "Clipboard reference behavior and regression architecture gate passed."
