param(
    [string]$ServiceName = 'NxToolsIndexerV1',
    [int]$WarmupSeconds = 2,
    [int]$SampleSeconds = 8,
    [int]$ReadyTimeoutSeconds = 180,
    [double]$AverageCpuThresholdPercent = 2.0,
    [double]$AverageIoThresholdKiBPerSecond = 64.0,
    [switch]$RestartService,
    [switch]$AssertRegression
)

$ErrorActionPreference = 'Stop'
$processorCount = [Environment]::ProcessorCount
$packagedExecutable = [System.IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot '..\..\artifacts\win-x64\NxTools.Indexer.exe'))
$expectedExecutable = [System.IO.Path]::GetFullPath(
    (Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.Indexer.exe'))
$packagedDesktop = [System.IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot '..\..\artifacts\win-x64\NxTools.exe'))
$packagedPlatformHost = [System.IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot '..\..\artifacts\win-x64\NxTools.PlatformHost.exe'))
$expectedDesktop = [System.IO.Path]::GetFullPath(
    (Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.exe'))
$expectedPlatformHost = [System.IO.Path]::GetFullPath(
    (Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service\NxTools.PlatformHost.exe'))
$service = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'"
if (-not $service) {
    throw "Service '$ServiceName' is not installed."
}
$registeredExecutable = if ($service.PathName -match '^"([^"]+)"') {
    $Matches[1]
}
else {
    ($service.PathName -split '\s+', 2)[0]
}
if ([System.IO.Path]::GetFullPath($registeredExecutable) -ne $expectedExecutable) {
    throw "Service '$ServiceName' points to '$registeredExecutable', expected '$expectedExecutable'."
}
$serviceKey = Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName"
$approvedHash = [string]$serviceKey.NxToolsBinarySha256
$packagedHash = (Get-FileHash -LiteralPath $packagedExecutable -Algorithm SHA256).Hash
$protectedHash = (Get-FileHash -LiteralPath $expectedExecutable -Algorithm SHA256).Hash
if (-not $approvedHash -or
    $approvedHash -ne $packagedHash -or
    $approvedHash -ne $protectedHash) {
    throw "Protected indexer bytes do not match the administrator-approved Release package."
}
$desktopHash = [string]$serviceKey.NxToolsDesktopSha256
$platformHostHash = [string]$serviceKey.NxToolsPlatformHostSha256
if ([System.IO.Path]::GetFullPath([string]$serviceKey.NxToolsDesktopPath) -ne
    $expectedDesktop -or
    $desktopHash -ne (Get-FileHash -LiteralPath $packagedDesktop -Algorithm SHA256).Hash -or
    $desktopHash -ne (Get-FileHash -LiteralPath $expectedDesktop -Algorithm SHA256).Hash -or
    $platformHostHash -ne (Get-FileHash -LiteralPath $packagedPlatformHost -Algorithm SHA256).Hash -or
    $platformHostHash -ne (Get-FileHash -LiteralPath $expectedPlatformHost -Algorithm SHA256).Hash) {
    throw "Protected desktop or PlatformHost bytes do not match the administrator-approved Release package."
}
if ($service.StartName -ne 'LocalSystem') {
    throw "Service '$ServiceName' runs as '$($service.StartName)', expected LocalSystem."
}

. (Join-Path $PSScriptRoot 'indexer-desktop-probe.ps1')
$cacheRoot = Join-Path $env:ProgramData 'NxTools\NxTools.Next\indexer'
$baseBefore = @(Get-ChildItem $cacheRoot -Filter '*.idx' | Sort-Object Name | ForEach-Object {
    [pscustomobject]@{ Name = $_.Name; Length = $_.Length; LastWriteTicks = $_.LastWriteTimeUtc.Ticks }
})
if ($RestartService) {
    if ($baseBefore.Count -eq 0) { throw 'Restart acceptance requires an existing persisted base.' }
    Restart-Service -Name $ServiceName -Force
    (Get-Service -Name $ServiceName).WaitForStatus('Running', [TimeSpan]::FromSeconds(20))
}
$service = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'"
if (-not $service -or $service.State -ne 'Running' -or $service.ProcessId -eq 0) {
    throw "Service '$ServiceName' is not running."
}
$processId = [uint32]$service.ProcessId
$search = Invoke-IndexerDesktopSearch -Query 'NxTools.Indexer.exe' -ExpectedPath $expectedExecutable -TimeoutMs ($ReadyTimeoutSeconds * 1000)
"AUTHENTICATED_DESKTOP_PID=$($search.DesktopPid)"
"SAMPLED_PHASE=search_verified"
if ($RestartService) {
    $baseAfter = @(Get-ChildItem $cacheRoot -Filter '*.idx' | Sort-Object Name | ForEach-Object {
        [pscustomobject]@{ Name = $_.Name; Length = $_.Length; LastWriteTicks = $_.LastWriteTimeUtc.Ticks }
    })
    if ((ConvertTo-Json -InputObject $baseBefore -Compress) -ne (ConvertTo-Json -InputObject $baseAfter -Compress)) {
        throw 'Service restart replaced a persisted base instead of restoring it.'
    }
    foreach ($base in $baseAfter) {
        $delta = Join-Path $cacheRoot ([IO.Path]::ChangeExtension($base.Name, '.delta'))
        if (-not (Test-Path $delta) -or (Get-Item $delta).Length -eq 0) { throw 'A restored base has no persisted delta.' }
    }
    "RESTORED_BASE_FILES=$($baseAfter.Count)"
    "PERSISTED_DELTA_FILES=$($baseAfter.Count)"
}
Start-Sleep -Seconds $WarmupSeconds
$idSamples = (Get-Counter '\Process(*)\ID Process').CounterSamples
$instance = $idSamples |
    Where-Object { [uint32]$_.CookedValue -eq $processId } |
    Select-Object -ExpandProperty InstanceName -First 1
if (-not $instance) {
    throw "Could not map service PID $processId to a Windows Process performance counter."
}

$counterSets = Get-Counter -Counter @(
    "\Process($instance)\% Processor Time",
    "\Process($instance)\IO Read Bytes/sec",
    "\Process($instance)\IO Write Bytes/sec",
    "\Process($instance)\IO Other Bytes/sec",
    "\Process($instance)\Working Set - Private"
) -SampleInterval 1 -MaxSamples $SampleSeconds
$samples = [System.Collections.Generic.List[object]]::new()
for ($index = 0; $index -lt $counterSets.Count; $index++) {
    $cpu = $counterSets[$index].CounterSamples |
        Where-Object Path -Like '*\% processor time' |
        Select-Object -ExpandProperty CookedValue -First 1
    $workingSetBytes = $counterSets[$index].CounterSamples |
        Where-Object Path -Like '*\working set - private' |
        Select-Object -ExpandProperty CookedValue -First 1
    $readBytes = $counterSets[$index].CounterSamples |
        Where-Object Path -Like '*\io read bytes/sec' |
        Select-Object -ExpandProperty CookedValue -First 1
    $writeBytes = $counterSets[$index].CounterSamples |
        Where-Object Path -Like '*\io write bytes/sec' |
        Select-Object -ExpandProperty CookedValue -First 1
    $otherBytes = $counterSets[$index].CounterSamples |
        Where-Object Path -Like '*\io other bytes/sec' |
        Select-Object -ExpandProperty CookedValue -First 1
    $samples.Add([pscustomobject]@{
        Second = $index + 1
        CpuPercent = [Math]::Round($cpu / $processorCount, 2)
        CoreEquivalentCpuPercent = [Math]::Round($cpu, 2)
        WorkingSetMB = [Math]::Round($workingSetBytes / 1MB, 1)
        ReadKiBPerSecond = $readBytes / 1KB
        WriteKiBPerSecond = $writeBytes / 1KB
        OtherKiBPerSecond = $otherBytes / 1KB
    })
}

$average = ($samples | Measure-Object CpuPercent -Average).Average
$coreAverage = ($samples | Measure-Object CoreEquivalentCpuPercent -Average).Average
$peak = ($samples | Measure-Object CpuPercent -Maximum).Maximum
$workingSet = ($samples | Measure-Object WorkingSetMB -Maximum).Maximum

$samples | Format-Table -AutoSize
"AVERAGE_CPU_PERCENT={0:N2}" -f $average
"AVERAGE_CORE_EQUIVALENT_CPU_PERCENT={0:N2}" -f $coreAverage
"PEAK_CPU_PERCENT={0:N2}" -f $peak
"PEAK_WORKING_SET_MB={0:N1}" -f $workingSet
$averageReadKiB = ($samples | Measure-Object ReadKiBPerSecond -Average).Average
$averageWriteKiB = ($samples | Measure-Object WriteKiBPerSecond -Average).Average
"AVERAGE_PROCESS_READ_KIB_PER_SECOND={0:N2}" -f $averageReadKiB
"AVERAGE_PROCESS_WRITE_KIB_PER_SECOND={0:N2}" -f $averageWriteKiB
"IO_SCOPE=Process logical file/IPC counters, not physical-disk attribution"

if ($AssertRegression -and $average -ge $AverageCpuThresholdPercent) {
    throw "Indexer idle CPU regression: normalized average was $([Math]::Round($average, 2))%, threshold is $AverageCpuThresholdPercent%."
}
if ($AssertRegression -and ($averageReadKiB + $averageWriteKiB) -ge $AverageIoThresholdKiBPerSecond) {
    throw "Indexer idle I/O regression: process read+write average $([Math]::Round($averageReadKiB + $averageWriteKiB, 2)) KiB/s, threshold $AverageIoThresholdKiBPerSecond KiB/s."
}

$finalService = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'"
if ($finalService.State -ne 'Running' -or $finalService.ProcessId -ne $processId) {
    throw 'Indexer service process changed during idle sampling.'
}
