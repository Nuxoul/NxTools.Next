$ErrorActionPreference='Stop'
$observationPath=Join-Path $env:PI_SCRATCH_DIR 'command-naming\idle-observation.json'
if(Test-Path $observationPath){throw 'Immutable observation exists'}
$observations=@();$deadline=(Get-Date).AddSeconds(180)
do {
 $rows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
 $observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$rows}
 if($rows.Count -eq 0){break}
 if((Get-Date) -lt $deadline){Start-Sleep -Seconds 30}
} while((Get-Date) -lt $deadline)
$rows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
$observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$rows}
@{purpose='Bounded read-only natural-idle observation; no Rust/process mutation or coordination';natural_idle=($rows.Count -eq 0);samples=$observations} | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $observationPath -Encoding UTF8
Write-Output "Natural idle: $($rows.Count -eq 0); samples: $($observations.Count)"
$rows | ConvertTo-Json -Depth 4
