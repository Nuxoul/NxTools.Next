$ErrorActionPreference='Stop'
$exe=(Resolve-Path 'target\debug\deps\nxtools_desktop-5008a12526e9ebe3.exe').Path
$checks=@(
 @{Name='download-page-related-01';Args=@('main_window::download_page::','--test-threads=1')},
 @{Name='download-main-related-01';Args=@('main_window::tests::download','main_window::tests::production_download','main_window::tests::selected_download','main_window::tests::queue_position','main_window::tests::clicking_download','main_window::tests::clicking_a_download','main_window::tests::double_clicking_a_download','main_window::tests::completed_download','main_window::tests::desktop_snapshot_projection','--test-threads=1')},
 @{Name='downloads-lifecycle-no-network';Args=@('downloads::detail_request_tests::','downloads::directory_picker_tests::','downloads::new_download_view::keyboard_tests::','downloads::progress_display::','downloads::task_window::auxiliary_theme_tests::','downloads::tests::authoritative_snapshot','downloads::tests::progress_events','downloads::tests::stale_progress','downloads::tests::live_progress','downloads::tests::removal_waits','downloads::tests::production_task_window','downloads::tests::reopening_a_task_window','downloads::tests::stale_window_cleanup','downloads::tests::a_late_activation','downloads::tests::a_native_task_window_close','downloads::tests::segment_status','downloads::tests::download_rows_use','--test-threads=1')}
)
$results=@()
foreach($check in $checks){
 $literal=($check.Args|ForEach-Object {"'$_'"}) -join ','
 & powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$PSScriptRoot\run-exe.ps1' -Name '$($check.Name)' -Executable '$exe' -TestArgs @($literal) -Seconds 300"
 $code=$LASTEXITCODE;$results+=@{name=$check.Name;exit=$code};Write-Output "CHECK $($check.Name) EXIT=$code"
}
$results|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'regression-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
