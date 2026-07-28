param(
    [string]$ServiceName = 'NxToolsIndexerV1',
    [int]$WarmupSeconds = 2,
    [int]$SampleSeconds = 8,
    [int]$ReadyTimeoutSeconds = 180,
    [double]$AverageCpuThresholdPercent = 2.0,
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

if ($RestartService) {
    Restart-Service -Name $ServiceName -Force
    (Get-Service -Name $ServiceName).WaitForStatus('Running', [TimeSpan]::FromSeconds(20))
}

$service = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'"
if (-not $service -or $service.State -ne 'Running' -or $service.ProcessId -eq 0) {
    throw "Service '$ServiceName' is not running."
}

$processId = [uint32]$service.ProcessId
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
    '.',
    'nxtools-next-indexer-v1',
    [System.IO.Pipes.PipeDirection]::InOut,
    [System.IO.Pipes.PipeOptions]::Asynchronous)
$pipe.Connect(3000)
$writer = [System.IO.StreamWriter]::new(
    $pipe,
    [System.Text.UTF8Encoding]::new($false),
    4096,
    $true)
$writer.AutoFlush = $true
$reader = [System.IO.StreamReader]::new(
    $pipe,
    [System.Text.UTF8Encoding]::new($false),
    $false,
    4096,
    $true)
$hello = @{
    protocol_version = 1
    id = [guid]::NewGuid().ToString()
    correlation_id = $null
    type = 'hello'
    payload = @{
        client_name = 'indexer_idle_cpu_probe'
        client_version = '0.1.0'
        capabilities = @('index_status', 'file_search')
    }
} | ConvertTo-Json -Compress -Depth 8
$writer.WriteLine($hello)
$null = $reader.ReadLine()
$initialStatus = ($reader.ReadLine() | ConvertFrom-Json).payload.data
"INITIAL_PHASE=$($initialStatus.phase)"
"INITIAL_INDEXED_ITEMS=$($initialStatus.indexed_items)"

$status = $initialStatus
$readyDeadline = [DateTime]::UtcNow.AddSeconds($ReadyTimeoutSeconds)
while ($status.phase -ne 'ready' -and [DateTime]::UtcNow -lt $readyDeadline) {
    $remaining = [Math]::Max(1, [int]($readyDeadline - [DateTime]::UtcNow).TotalMilliseconds)
    $read = $reader.ReadLineAsync()
    if (-not $read.Wait($remaining) -or $null -eq $read.Result) {
        break
    }
    $frame = $read.Result | ConvertFrom-Json
    if ($frame.type -eq 'indexer_event' -and $frame.payload.event -eq 'status') {
        $status = $frame.payload.data
    }
}
if ($status.phase -ne 'ready') {
    $reader.Dispose()
    $writer.Dispose()
    $pipe.Dispose()
    throw "Indexer did not become ready within $ReadyTimeoutSeconds seconds; phase=$($status.phase), indexed_items=$($status.indexed_items)."
}
"SAMPLED_PHASE=$($status.phase)"
"SAMPLED_INDEXED_ITEMS=$($status.indexed_items)"

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
    $samples.Add([pscustomobject]@{
        Second = $index + 1
        CpuPercent = [Math]::Round($cpu / $processorCount, 2)
        CoreEquivalentCpuPercent = [Math]::Round($cpu, 2)
        WorkingSetMB = [Math]::Round($workingSetBytes / 1MB, 1)
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

if ($AssertRegression -and $average -ge $AverageCpuThresholdPercent) {
    $reader.Dispose()
    $writer.Dispose()
    $pipe.Dispose()
    throw "Indexer idle CPU regression: normalized average was $([Math]::Round($average, 2))%, threshold is $AverageCpuThresholdPercent%."
}

$reader.Dispose()
$writer.Dispose()
$pipe.Dispose()
