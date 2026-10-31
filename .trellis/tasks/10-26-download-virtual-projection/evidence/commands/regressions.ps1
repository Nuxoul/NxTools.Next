$ErrorActionPreference='Stop'
$exe=(Resolve-Path 'target\debug\deps\nxtools_desktop-bc43045ce72e2830.exe').Path
$checks=@(
 @{Name='download-page-related';Args=@('main_window::download_page::','--test-threads=1')},
 @{Name='download-main-related';Args=@('main_window::tests::download','main_window::tests::production_download','main_window::tests::selected_download','main_window::tests::selected_download_detail','main_window::tests::queue_position','main_window::tests::clicking_download','main_window::tests::clicking_a_download','main_window::tests::double_clicking_a_download','main_window::tests::completed_download','main_window::tests::desktop_snapshot_projection','--test-threads=1','--nocapture')},
 @{Name='downloads-related';Args=@('downloads::','--test-threads=1')},
 @{Name='protected-related';Args=@('audit03_','audit04_','audit05_','audit06_','audit08_','audit10_','--test-threads=1')}
)
$results=@()
foreach($check in $checks){
 $literal=($check.Args|ForEach-Object {"'$_'"}) -join ','
 $invoke="& '$PSScriptRoot\run-exe.ps1' -Name '$($check.Name)' -Executable '$exe' -TestArgs @($literal) -Seconds 900"
 & powershell -NoProfile -ExecutionPolicy Bypass -Command $invoke
 $code=$LASTEXITCODE;$results+=@{name=$check.Name;exit=$code};Write-Output "CHECK $($check.Name) EXIT=$code"
}
$results | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot 'regression-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
