$ErrorActionPreference='Stop'
$observationRoot=Join-Path $env:PI_SCRATCH_DIR 'cursor-semantics'
$path=Join-Path $observationRoot 'idle-observation.json'
if(Test-Path $path){throw 'Immutable observation exists'}
$observations=@();$deadline=(Get-Date).AddSeconds(300)
do {
 $rows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
 $observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$rows}
 if($rows.Count -eq 0){break}
 if((Get-Date) -lt $deadline){Start-Sleep -Seconds 30}
} while((Get-Date) -lt $deadline)
$rows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
$observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$rows}
$result=@{purpose='Bounded read-only natural-idle observation; no Rust command, process mutation or coordination';natural_idle=($rows.Count -eq 0);samples=$observations}
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $path -Encoding UTF8
Write-Output "Natural idle: $($result.natural_idle); samples: $($observations.Count)"
$rows | ConvertTo-Json -Depth 4
