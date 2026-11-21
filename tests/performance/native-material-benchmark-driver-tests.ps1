# Windows PowerShell/.NET Framework; no window is shown and no foreign process is controlled.
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$sourcePath = Join-Path $PSScriptRoot 'NativeMaterialBenchmarkDriver.cs'
Add-Type -Path $sourcePath -ReferencedAssemblies System.dll, System.Core.dll, System.Drawing.dll, System.Windows.Forms.dll
$script:caseCount = 0
$script:assertionCount = 0
$script:caseNames = New-Object 'System.Collections.Generic.List[string]'
$script:gpuMeasuredReads = 0
$script:gpuUnavailableReads = 0
$script:liveSamples = @()
$script:sampleGapMilliseconds = 0

function Assert-True([bool] $Condition, [string] $Message) {
    $script:assertionCount++
    if (-not $Condition) { throw $Message }
}

function Test-Case([string] $Name, [scriptblock] $Body) {
    & $Body
    $script:caseCount++
    $script:caseNames.Add($Name)
}

function New-CounterValue([string] $Name, [double] $Value, [uint32] $Status = 0) {
    $item = New-Object NativeMaterialBenchmarkCounterValue
    $item.InstanceName = $Name
    $item.Value = $Value
    $item.Status = $Status
    return $item
}

function New-CounterBatch([NativeMaterialBenchmarkCounterValue[]] $Items = @(),
    [string] $Status = 'available', [Nullable[uint32]] $NativeCode = $null) {
    $batch = New-Object NativeMaterialBenchmarkCounterBatch
    $batch.Items = $Items
    $batch.Status = $Status
    $batch.NativeCode = $NativeCode
    return $batch
}

Test-Case 'PowerShell AST' {
    $parseTokens = $null
    $parseErrors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseFile($PSCommandPath, [ref] $parseTokens, [ref] $parseErrors)
    Assert-True ($parseErrors.Count -eq 0) 'PowerShell AST must have no parse errors.'
}

Test-Case 'PDH native layout' {
    $nestedFlags = [System.Reflection.BindingFlags]::NonPublic
    $valueType = [NativeMaterialBenchmarkSampler].GetNestedType('FormattedValue', $nestedFlags)
    $itemType = [NativeMaterialBenchmarkSampler].GetNestedType('CounterItem', $nestedFlags)
    Assert-True ([Runtime.InteropServices.Marshal]::SizeOf([Activator]::CreateInstance($valueType)) -eq 16) 'PDH formatted value must be 16 bytes.'
    Assert-True ([Runtime.InteropServices.Marshal]::OffsetOf($valueType, 'Value').ToInt32() -eq 8) 'PDH double must start at offset 8.'
    if ([IntPtr]::Size -eq 8) {
        Assert-True ([Runtime.InteropServices.Marshal]::SizeOf([Activator]::CreateInstance($itemType)) -eq 24) 'PDH x64 counter item must be 24 bytes.'
        Assert-True ([Runtime.InteropServices.Marshal]::OffsetOf($itemType, 'Value').ToInt32() -eq 8) 'PDH x64 item value must start at offset 8.'
    }
}

Test-Case 'Physical engine max, duplicate context sum, no cap' {
    $engines = New-CounterBatch @(
        (New-CounterValue 'pid_101_luid_0x00000000_0x0000abcd_phys_0_eng_0_engtype_3D' 55),
        (New-CounterValue 'pid_101_luid_0x00000000_0x0000abcd_phys_0_eng_0_engtype_3D#1' 60),
        (New-CounterValue 'pid_101_luid_0x00000000_0x0000abcd_phys_0_eng_1_engtype_Copy' 80),
        (New-CounterValue 'pid_101_luid_0x00000000_0x0000ffff_phys_0_eng_0_engtype_3D' 20)
    )
    $result = [NativeMaterialBenchmarkGpuAggregation]::Aggregate(@(101), $engines, $null, $null)[0]
    Assert-True ($result.GpuEngineStatus -eq 'available') 'Engine aggregate must be available.'
    Assert-True ($result.GpuMaxEnginePercent -eq 115) 'Max must be 115, not the sum 215 or a capped 100.'
    Assert-True ($result.GpuEnginePercent.Count -eq 3) 'Three physical engines must remain distinct.'
    Assert-True ($result.GpuEnginePercent['luid_0x00000000_0x0000abcd_phys_0_eng_0'] -eq 115) 'Duplicate contexts must be summed within their physical engine.'
}

Test-Case 'Selected PID filtering and deduplication' {
    $engines = New-CounterBatch @(
        (New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0_eng_0_engtype_3D' 12),
        (New-CounterValue 'pid_777_luid_0x0_0xabcd_phys_0_eng_0_engtype_3D' 999),
        (New-CounterValue 'pid_777_luid_0x0_0xabcd_phys_0_eng_1_engtype_Copy' 999 5)
    )
    $memory = New-CounterBatch @(
        (New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0' 42),
        (New-CounterValue 'pid_777_luid_0x0_0xabcd_phys_0' 999999)
    )
    $result = [NativeMaterialBenchmarkGpuAggregation]::Aggregate(@(101, 101, 303), $engines, $memory, $memory)
    Assert-True ($result.Count -eq 2) 'Only unique requested PIDs may be returned.'
    Assert-True ($result[0].ProcessId -eq 101 -and $result[1].ProcessId -eq 303) 'Requested first-seen ordering must be preserved.'
    Assert-True ($result[0].GpuMaxEnginePercent -eq 12 -and $result[0].GpuDedicatedBytes -eq 42) 'Other PIDs cannot contribute to selected values.'
    Assert-True ($result[1].GpuEngineStatus -eq 'no_instance' -and $null -eq $result[1].GpuMaxEnginePercent) 'An absent selected instance is not measured zero.'
    Assert-True (($result | ConvertTo-Json -Depth 5) -notmatch '777|999999') 'Serialization cannot retain nonrequested IDs or values.'
}

Test-Case 'GPU memory adapter sum and measured zero' {
    $dedicated = New-CounterBatch @(
        (New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0' 1024),
        (New-CounterValue 'pid_101_luid_0x0_0xffff_phys_0' 2048)
    )
    $zero = New-CounterBatch @((New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0' 0 1))
    $result = [NativeMaterialBenchmarkGpuAggregation]::Aggregate(@(101), $null, $dedicated, $zero)[0]
    Assert-True ($result.GpuDedicatedMemoryStatus -eq 'available' -and $result.GpuDedicatedBytes -eq 3072) 'Selected adapter memory must be summed.'
    Assert-True ($result.GpuSharedMemoryStatus -eq 'available' -and $null -ne $result.GpuSharedBytes -and $result.GpuSharedBytes -eq 0) 'A valid NEW_DATA zero must remain measured zero.'
    Assert-True ($null -eq $result.GpuSharedMemoryNativeCode) 'Available memory cannot carry a failure code.'
}

Test-Case 'Unavailable versus absent memory' {
    $unavailable = New-CounterBatch @() 'counter_unavailable' ([Convert]::ToUInt32('C0000BB8', 16))
    $empty = New-CounterBatch
    $result = [NativeMaterialBenchmarkGpuAggregation]::Aggregate(@(101), $empty, $unavailable, $empty)[0]
    Assert-True ($result.GpuDedicatedMemoryStatus -eq 'counter_unavailable' -and $null -eq $result.GpuDedicatedBytes) 'Unavailable counters cannot become zero.'
    Assert-True ($result.GpuDedicatedMemoryNativeCode -eq [Convert]::ToUInt32('C0000BB8', 16)) 'Native PDH failure codes must survive aggregation.'
    Assert-True ($result.GpuSharedMemoryStatus -eq 'no_instance' -and $null -eq $result.GpuSharedBytes) 'Missing memory instance must be distinct from unavailable counter and zero.'
    Assert-True ($result.GpuEngineStatus -eq 'no_instance' -and $null -eq $result.GpuMaxEnginePercent) 'Empty engine arrays cannot become zero.'
}

Test-Case 'Invalid selected data cannot produce partial success' {
    $engines = New-CounterBatch @((New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0_eng_0_engtype_3D' 0 5))
    $memory = New-CounterBatch @(
        (New-CounterValue 'pid_101_luid_0x0_0xabcd_phys_0' 4096),
        (New-CounterValue 'pid_101_luid_0x0_0xffff_phys_0' 0 5)
    )
    $result = [NativeMaterialBenchmarkGpuAggregation]::Aggregate(@(101), $engines, $memory, $null)[0]
    Assert-True ($result.GpuEngineStatus -eq 'invalid_data' -and $null -eq $result.GpuMaxEnginePercent) 'Invalid engine status cannot become zero.'
    Assert-True ($result.GpuEngineNativeCode -eq 5) 'Engine item failure code must be preserved.'
    Assert-True ($result.GpuDedicatedMemoryStatus -eq 'invalid_data' -and $null -eq $result.GpuDedicatedBytes) 'Partial adapter data cannot be reported as total memory.'
    Assert-True ($result.GpuDedicatedMemoryNativeCode -eq 5) 'Memory item failure code must be preserved.'
}

Test-Case 'Backdrop construction and disposal without desktop effects' {
    $backdrop = New-Object NativeMaterialBenchmarkBackdrop
    try {
        Assert-True ($backdrop.ProcessId -eq $PID) 'Backdrop must belong to the sampling process.'
        Assert-True ($backdrop.WindowHandle -eq 0 -and -not $backdrop.IsMoving) 'Construction must not create a window or start movement.'
        Start-Sleep -Milliseconds 25
        Assert-True ($backdrop.PaintCount -eq 0 -and $backdrop.UpdateCount -eq 0) 'Unstarted backdrop must perform no background work.'
        $flags = [Reflection.BindingFlags]'Instance,NonPublic'
        Assert-True ($null -eq [NativeMaterialBenchmarkBackdrop].GetField('form', $flags).GetValue($backdrop)) 'Construction must not create a form or its timer.'
        Assert-True ($null -eq [NativeMaterialBenchmarkBackdrop].GetField('thread', $flags).GetValue($backdrop)) 'Construction must not start a thread.'
    }
    finally { $backdrop.Dispose() }
    $backdrop.Dispose()
    Assert-True ($backdrop.WindowHandle -eq 0 -and $backdrop.UpdateCount -eq 0) 'Repeated disposal must remain inert.'
}

Test-Case 'Actual current-process sampler: two reads and coherent GPU availability' {
    $sampler = New-Object NativeMaterialBenchmarkSampler
    try {
        $first = @($sampler.Read([int[]]@($PID)))
        $clock = [Diagnostics.Stopwatch]::StartNew()
        Start-Sleep -Milliseconds 1000
        $second = @($sampler.Read([int[]]@($PID)))
        $clock.Stop()
        $script:sampleGapMilliseconds = $clock.Elapsed.TotalMilliseconds
        Assert-True ($first.Count -eq 1 -and $second.Count -eq 1) 'Each read must contain only the one requested PID.'
        Assert-True ($first[0].ProcessId -eq $PID -and $second[0].ProcessId -eq $PID) 'Sampler must not append implicit PIDs.'
        $script:liveSamples = @($first[0], $second[0])
        foreach ($sample in $script:liveSamples) {
            Assert-True ($sample.ProcessStatus -eq 'available' -and $null -eq $sample.ProcessNativeCode) 'Current process metrics must be available without error.'
            Assert-True ($sample.PrivateBytes -gt 0 -and $sample.WorkingSetBytes -gt 0) 'Actual private and working-set bytes must be positive.'
            Assert-True ($null -ne $sample.CpuTotalMilliseconds -and $sample.CpuTotalMilliseconds -ge 0 -and $sample.HandleCount -gt 0) 'CPU and handle metrics must be actual measurements.'
            $gpuMetrics = @(
                @($sample.GpuEngineStatus, $sample.GpuMaxEnginePercent, $sample.GpuEngineNativeCode),
                @($sample.GpuDedicatedMemoryStatus, $sample.GpuDedicatedBytes, $sample.GpuDedicatedMemoryNativeCode),
                @($sample.GpuSharedMemoryStatus, $sample.GpuSharedBytes, $sample.GpuSharedMemoryNativeCode)
            )
            foreach ($metric in $gpuMetrics) {
                Assert-True ($metric[0] -in @('available', 'counter_unavailable', 'query_unavailable', 'pdh_unavailable', 'no_instance', 'no_data', 'collect_error', 'read_error', 'buffer_limit', 'invalid_data')) 'GPU status must be explicit.'
                if ($metric[0] -eq 'available') {
                    Assert-True ($null -ne $metric[1] -and $metric[1] -ge 0 -and $null -eq $metric[2]) 'Available GPU metric must have a real nonnegative value and no failure code.'
                    $script:gpuMeasuredReads++
                }
                else {
                    Assert-True ($null -eq $metric[1]) 'Unavailable GPU metric must be null, never fabricated zero.'
                    $script:gpuUnavailableReads++
                }
            }
        }
        Assert-True ($second[0].CpuTotalMilliseconds -ge $first[0].CpuTotalMilliseconds) 'Absolute CPU milliseconds must be monotonic.'
        Assert-True ($script:sampleGapMilliseconds -ge 1000) 'Actual sample gap must cover at least one second.'
        $roundTrip = $second[0] | ConvertTo-Json -Depth 5 | ConvertFrom-Json
        Assert-True ($roundTrip.ProcessId -eq $PID -and $roundTrip.PrivateBytes -gt 0) 'Scalar sample schema must serialize through ConvertTo-Json.'
    }
    finally { $sampler.Dispose() }
    $sampler.Dispose()
}

Test-Case 'Missing process remains unavailable' {
    $sampler = New-Object NativeMaterialBenchmarkSampler
    try {
        $missing = $sampler.Read([int[]]@([int]::MaxValue))[0]
        Assert-True ($missing.ProcessStatus -eq 'missing') 'An absent PID must report missing.'
        Assert-True ($null -eq $missing.CpuTotalMilliseconds -and $null -eq $missing.PrivateBytes -and $null -eq $missing.WorkingSetBytes -and $null -eq $missing.HandleCount) 'Missing process metrics must all remain null.'
    }
    finally { $sampler.Dispose() }
}

[pscustomobject]@{
    Result = 'passed'
    Cases = $script:caseCount
    Assertions = $script:assertionCount
    CaseNames = $script:caseNames.ToArray()
    ActualProcessReads = $script:liveSamples.Count
    SampleGapMilliseconds = [Math]::Round($script:sampleGapMilliseconds, 2)
    GpuMeasuredMetricReads = $script:gpuMeasuredReads
    GpuUnavailableMetricReads = $script:gpuUnavailableReads
    BackdropWindowsShown = 0
    Samples = $script:liveSamples
} | ConvertTo-Json -Depth 6
