$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
function Assert-Has([string]$relative, [string]$pattern, [string]$reason) {
    $text = [IO.File]::ReadAllText((Join-Path $root $relative))
    if ($text -notmatch $pattern) { throw "$reason ($relative)" }
}
Assert-Has 'csharp/NxTools.PlatformHost/Windows/SystemMotionPreference.cs' 'SpiGetClientAreaAnimation\s*=\s*0x1042' 'Native Windows client-area SPI missing'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/SystemMotionPreference.cs' 'SystemParametersInfoW' 'Native API missing'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs' 'SystemEvents.UserPreferenceChanged\s*\+=' 'System broadcast hook missing'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs' 'SystemEvents.UserPreferenceChanged\s*-=' 'System broadcast cleanup missing'
Assert-Has 'csharp/NxTools.PlatformHost/Protocol/PlatformPipeClient.cs' 'DiscardQueuedEventsForNewSession\(\);[\s\S]*?SystemMotionSnapshot[\s\S]*?WriteAsync\(writer, PlatformEventEnvelopeFactory.Create' 'Per-session snapshot ordering missing'
Assert-Has 'rust/apps/nxtools-desktop/src/platform_events.rs' 'PlatformEvent::SystemMotionPreferenceChanged' 'Desktop event apply missing'
Assert-Has 'rust/apps/nxtools-desktop/src/ui/motion.rs' 'fn motion_allowed\(cx: &App\)' 'Render policy API missing'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/PendingMotionPublication.cs' 'pending is null \|\| !publish\(pending\)\) return null;[\s\S]*?_pending = null' 'Queue-full publication must retain its single latest slot'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/PendingMotionPublication.cs' 'if \(!_stopped\) _pending = new' 'Latest motion preference must supersede the pending slot'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs' 'case WmTimer when message.WParam.ToInt64\(\) == SystemMotionRetryTimerId:[\s\S]*?PublishPendingSystemMotion' 'Motion retry must use the original event STA'
Assert-Has 'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs' '_pendingMotion.Stop\(\)' 'Motion slot must retire on Stop'
Assert-Has 'rust/apps/nxtools-desktop/src/ui/motion.rs' 'observe_global::<UiMotionPreference>[\s\S]*?cx.defer\(move \|cx\|[\s\S]*?view.update' 'Preference observer must defer weak local view notification'
Assert-Has 'rust/apps/nxtools-desktop/src/main_window/notifications.rs' 'notification.exiting && crate::ui::motion::reduced_motion\(cx\)' 'A reduced mid-exit notification must not reappear'
foreach ($consumer in @(
    'rust/apps/nxtools-desktop/src/downloads/new_download_view.rs',
    'rust/apps/nxtools-desktop/src/downloads/task_window.rs',
    'rust/apps/nxtools-desktop/src/download_taskbar_overlay.rs',
    'rust/apps/nxtools-desktop/src/quick_launch/panel.rs',
    'rust/apps/nxtools-desktop/src/quick_launch/preview.rs',
    'rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs',
    'rust/apps/nxtools-desktop/src/ui/preview.rs',
    'rust/apps/nxtools-desktop/src/main_window/notifications.rs'
)) {
    Assert-Has $consumer '(motion_allowed|reduced_motion|animated_surface)' 'Animation consumer must read the shared system policy'
}
Write-Output 'PASS ui motion native/IPC/desktop static ownership (not native toggle acceptance)'
