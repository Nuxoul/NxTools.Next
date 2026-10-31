param([Parameter(Mandatory=$true)][string]$Name,[Parameter(Mandatory=$true)][string[]]$CargoArgs)
$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256';$env:_LINK_='/DEBUG:NONE'
$root=Join-Path $env:PI_SCRATCH_DIR 'quick-retained-text'
$log=Join-Path $root ($Name+'.log');$record=Join-Path $root ($Name+'.json')
if((Test-Path $log) -or (Test-Path $record)){throw 'Immutable command log exists'}
$deadline=(Get-Date).AddSeconds(600);$observations=@()
do {
 $busy=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' })
 $observations+=@{utc=[DateTime]::UtcNow.ToString('o');processes=@($busy | Select-Object ProcessId,ParentProcessId,Name,CommandLine)}
 if($busy.Count -eq 0){break}
 if((Get-Date)-lt $deadline){Start-Sleep -Seconds 15}
}while((Get-Date)-lt $deadline)
$observations | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $root ($Name+'.admission.json')) -Encoding UTF8
if($busy.Count){throw "Natural Rust idle unavailable after bounded wait: $($busy.ProcessId -join ',')"}
$started=[DateTime]::UtcNow
& cmd /d /c "cargo $($CargoArgs -join ' ') > `"$log`" 2>&1"
$code=$LASTEXITCODE
@{command='cargo '+($CargoArgs -join ' ');exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');environment=@{CARGO_BUILD_JOBS=$env:CARGO_BUILD_JOBS;CARGO_INCREMENTAL=$env:CARGO_INCREMENTAL;RUSTFLAGS=$env:RUSTFLAGS;_LINK_=$env:_LINK_};log=$log} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $record -Encoding UTF8
Get-Content -LiteralPath $log -Tail 55
exit $code
