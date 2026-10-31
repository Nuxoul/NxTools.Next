$ErrorActionPreference='Stop'
$observationRoot=Join-Path $env:PI_SCRATCH_DIR 'rem-quick-clipboard'
$observationPath=Join-Path $observationRoot ('idle-observation-'+(Get-Date -Format 'yyyyMMdd-HHmmss')+'.json')
$observations=@()
$deadline=(Get-Date).AddSeconds(300)
do {
 $processRows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
 $observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$processRows}
 if ($processRows.Count -eq 0) {break}
 if ((Get-Date) -lt $deadline) {Start-Sleep -Seconds 30}
} while ((Get-Date) -lt $deadline)
$finalRows=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' } | Select-Object ProcessId,ParentProcessId,CreationDate,Name,CommandLine)
$observations+=@{timestamp_utc=[DateTime]::UtcNow.ToString('o');processes=$finalRows}
$result=@{purpose='Bounded read-only natural-idle observation; no Rust command, process mutation or coordination';natural_idle=($finalRows.Count -eq 0);samples=$observations}
$result | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $observationPath -Encoding UTF8
Write-Output "Observation: $observationPath"
$result | ConvertTo-Json -Depth 7
