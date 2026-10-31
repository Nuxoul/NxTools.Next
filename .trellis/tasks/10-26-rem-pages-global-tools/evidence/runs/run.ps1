param([string]$Name,[string]$CargoArgs)
$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$waitingSince=Get-Date
while(@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') }).Count){if(((Get-Date)-$waitingSince).TotalMinutes -gt 20){throw 'Natural idle wait reached bound'};Start-Sleep -Seconds 10}
$log=Join-Path $PSScriptRoot ($Name+'.log');if(Test-Path $log){throw 'Immutable run exists'}
$started=Get-Date
cmd /d /c "cargo $CargoArgs 2>&1" > $log
$code=$LASTEXITCODE
[pscustomobject]@{name=$Name;command="cargo $CargoArgs";exit=$code;elapsed=((Get-Date)-$started).TotalSeconds;finished=(Get-Date).ToString('o')} | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $PSScriptRoot ($Name+'.json'))
Get-Content $log -Tail 55
exit $code
