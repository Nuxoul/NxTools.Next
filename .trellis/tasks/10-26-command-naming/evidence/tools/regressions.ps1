$ErrorActionPreference='Stop'
$runner=Join-Path $PSScriptRoot 'run.ps1'
$checks=@(
 @{Name='downloads-related';Args=@('test','-p','nxtools-desktop','downloads::','--','--test-threads=1')},
 @{Name='ui-related';Args=@('test','-p','nxtools-desktop','ui::','--','--test-threads=1')},
 @{Name='main-window-related';Args=@('test','-p','nxtools-desktop','main_window::','--','--test-threads=1')},
 @{Name='audit04-protection';Args=@('test','-p','nxtools-desktop','audit04_','--','--test-threads=1')},
 @{Name='audit05-protection';Args=@('test','-p','nxtools-desktop','audit05_','--','--test-threads=1')},
 @{Name='audit06-protection';Args=@('test','-p','nxtools-desktop','audit06_','--','--test-threads=1')},
 @{Name='strict-tests';Args=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')},
 @{Name='strict-production';Args=@('clippy','-p','nxtools-desktop','--','-D','warnings')},
 @{Name='workspace-check';Args=@('check','--workspace','--tests')},
 @{Name='workspace-fmt';Args=@('fmt','--all','--','--check')}
)
$results=@()
foreach($check in $checks){
 $argumentLiteral=($check.Args | ForEach-Object {"'$_'"}) -join ','
 $invoke="& '$runner' -Name '$($check.Name)' -CargoArgs @($argumentLiteral)"
 & powershell -NoProfile -ExecutionPolicy Bypass -Command $invoke
 $code=$LASTEXITCODE
 $results+=@{name=$check.Name;exit=$code}
 Write-Output "CHECK $($check.Name) EXIT=$code"
}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'regression-results.json') -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
