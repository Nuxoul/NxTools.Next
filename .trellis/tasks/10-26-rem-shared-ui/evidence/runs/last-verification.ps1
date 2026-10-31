$ErrorActionPreference='Stop'
$waitStart=Get-Date
while(@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') }).Count){if(((Get-Date)-$waitStart).TotalMinutes -gt 20){throw 'Natural Rust idle wait reached bound'};Start-Sleep -Seconds 10}
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$formatPaths=Get-Content (Join-Path $PSScriptRoot 'format-paths.json') -Raw | ConvertFrom-Json
$fmtLog=Join-Path $PSScriptRoot 'scoped-fmt-final.log'
foreach($formatPath in $formatPaths){& rustfmt --edition 2024 --config skip_children=true --check $formatPath *> $fmtLog; if($LASTEXITCODE -ne 0){Get-Content $fmtLog;throw "Scoped formatting failed $formatPath"}}
'Scoped rustfmt check PASS:22 task-related Rust sources;skip_children=true' | Set-Content -Encoding UTF8 $fmtLog
& (Join-Path $PSScriptRoot 'run.ps1') -Name fixed-exceptions-exact-final -CargoArgs 'test -p nxtools-desktop audit06_shared_clipboard_fixed -- --test-threads=1'
& (Join-Path $PSScriptRoot 'run.ps1') -Name quick-presentation-final -CargoArgs 'test -p nxtools-desktop quick_launch::polish_tests:: -- --test-threads=1'
& (Join-Path $PSScriptRoot 'run.ps1') -Name strict-tests-delivery -CargoArgs 'clippy -p nxtools-desktop --tests -- -D warnings'
& (Join-Path $PSScriptRoot 'run.ps1') -Name workspace-check-delivery -CargoArgs 'check --workspace --tests'
& (Join-Path $PSScriptRoot 'run.ps1') -Name workspace-fmt-delivery -CargoArgs 'fmt --all -- --check'
