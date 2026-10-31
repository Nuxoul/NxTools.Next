$ErrorActionPreference='Stop'
$checks=@(
 @('ui-related','test -p nxtools-desktop ui:: -- --test-threads=1'),
 @('downloads-related','test -p nxtools-desktop downloads:: -- --test-threads=1'),
 @('main-related','test -p nxtools-desktop main_window:: -- --test-threads=1'),
 @('global-related','test -p nxtools-desktop network_settings:: -- --test-threads=1'),
 @('audit05-protected','test -p nxtools-desktop audit05_ -- --test-threads=1'),
 @('audit04-protected','test -p nxtools-desktop audit04_ -- --test-threads=1')
)
$results=@()
foreach($check in $checks){
 & (Join-Path $PSScriptRoot 'run.ps1') -Name $check[0] -CargoArgs $check[1]
 $code=$LASTEXITCODE
 $results += [pscustomobject]@{name=$check[0];exit=$code}
 if($code -eq 2){break}
}
$results | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $PSScriptRoot 'regression-results.json')
if(@($results | Where-Object exit -ne 0).Count){exit 1}
