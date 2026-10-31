$ErrorActionPreference='Stop'
$checks=@(
 @{Name='strict-desktop-tests';Args=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')},
 @{Name='strict-desktop-production';Args=@('clippy','-p','nxtools-desktop','--','-D','warnings')},
 @{Name='strict-download-tests';Args=@('clippy','-p','nxtools-download','--tests','--','-D','warnings')},
 @{Name='strict-download-production';Args=@('clippy','-p','nxtools-download','--','-D','warnings')},
 @{Name='workspace-check';Args=@('check','--workspace','--tests')},
 @{Name='workspace-fmt';Args=@('fmt','--all','--','--check')}
)
$results=@()
foreach($check in $checks){
 $literal=($check.Args|ForEach-Object {"'$_'"}) -join ','
 & powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$PSScriptRoot\run.ps1' -Name '$($check.Name)' -CargoArgs @($literal)"
 $code=$LASTEXITCODE;$results+=@{name=$check.Name;exit=$code};Write-Output "CHECK $($check.Name) EXIT=$code"
}
$results|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'quality-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
