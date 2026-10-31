param([string]$Name='shared-red',[string]$CargoArgs='test -p nxtools-desktop audit06_shared -- --test-threads=1 --nocapture')
$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$waitingSince=Get-Date
while(@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') }).Count -ne 0){
 if(((Get-Date)-$waitingSince).TotalMinutes -gt 20){throw 'Natural Rust idle not reached within bounded wait'}
 Start-Sleep -Seconds 10
}
$log=Join-Path $PSScriptRoot ($Name+'.log')
if(Test-Path $log){throw 'Immutable run name already exists'}
$started=Get-Date
cmd /d /c "cargo $CargoArgs 2>&1" > $log
$code=$LASTEXITCODE
[pscustomobject]@{name=$Name;command="cargo $CargoArgs";exit=$code;elapsed=((Get-Date)-$started).TotalSeconds;finished=(Get-Date).ToString('o')} | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $PSScriptRoot ($Name+'.json'))
Get-Content $log -Tail 65
exit $code
