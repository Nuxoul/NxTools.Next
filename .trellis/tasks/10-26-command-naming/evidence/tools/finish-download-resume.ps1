$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$deadline=(Get-Date).AddSeconds(600)
do{$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe'});if(-not $busy.Count){break};Start-Sleep -Seconds 15}while((Get-Date)-lt $deadline)
if($busy.Count){throw 'Natural idle unavailable for final format'}
& python (Join-Path $PSScriptRoot 'check-polish-prefix.py')
if($LASTEXITCODE -ne 0){throw 'Old prefix guard failed'}
& rustfmt --edition 2024 --config skip_children=true rust\apps\nxtools-desktop\src\downloads\polish_tests.rs
if($LASTEXITCODE -ne 0){throw 'Scoped formatting failed'}
$checks=@(
 @{Name='audit08-final-twelve';Args=@('test','-p','nxtools-desktop','audit08_','--','--test-threads=1')},
 @{Name='downloads-final';Args=@('test','-p','nxtools-desktop','downloads::','--','--test-threads=1')},
 @{Name='names-final';Args=@('test','-p','nxtools-desktop','accessible_name_','--','--test-threads=1')}
)
$results=@();$runner=Join-Path $PSScriptRoot 'run.ps1'
foreach($check in $checks){
 $argumentLiteral=($check.Args | ForEach-Object {"'$_'"}) -join ','
 & powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$runner' -Name '$($check.Name)' -CargoArgs @($argumentLiteral)"
 $results+=@{name=$check.Name;exit=$LASTEXITCODE}
}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'final-download-results.json') -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
