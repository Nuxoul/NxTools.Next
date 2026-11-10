[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$HostExePath,
    [Parameter(Mandatory = $true)][string]$DesktopExePath,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [ValidateRange(0, 2147483647)][int]$TargetProcessId = 0,
    [ValidateRange(0, 2147483647)][int]$ExpectedParentProcessId = 0,
    [ValidateRange(5, 600)][int]$DurationSeconds = 60,
    [ValidateRange(5, 60)][int]$IntervalSeconds = 5
)

function Test-HostMemoryIdentity {
    param($Candidate, $Parent, [string]$HostPath, [string]$DesktopPath, [int]$ExpectedParent = 0)
    if ($null -eq $Candidate -or $null -eq $Parent -or
        [string]::IsNullOrWhiteSpace($Candidate.ExecutablePath) -or
        [string]::IsNullOrWhiteSpace($Parent.ExecutablePath)) { return $false }
    if (-not [string]::Equals($Candidate.ExecutablePath, $HostPath, [StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals($Parent.ExecutablePath, $DesktopPath, [StringComparison]::OrdinalIgnoreCase) -or
        $Candidate.ParentProcessId -ne $Parent.ProcessId -or $Candidate.SessionId -ne $Parent.SessionId -or
        $null -eq $Candidate.CreationDate -or $null -eq $Parent.CreationDate -or
        $Parent.CreationDate -gt $Candidate.CreationDate -or
        ($ExpectedParent -ne 0 -and $Parent.ProcessId -ne $ExpectedParent)) { return $false }
    # Ordinary Desktop startup has exactly these two arguments. Do not persist command lines.
    $role = [regex]::Match([string]$Candidate.CommandLine, '^\s*(?:"[^"]+"|\S+)\s+--parent-pid\s+(\d+)\s*$')
    return $role.Success -and $role.Groups[1].Value -eq [string]$Parent.ProcessId
}

function Get-HostMemoryCandidates {
    param([string]$HostPath, [string]$DesktopPath, [int]$SelectedId, [int]$ExpectedParent)
    $facts = @(Get-CimInstance Win32_Process -Filter "Name = 'NxTools.PlatformHost.exe'" -OperationTimeoutSec 3 -ErrorAction Stop)
    foreach ($candidate in $facts) {
        if (($SelectedId -ne 0 -and $candidate.ProcessId -ne $SelectedId) -or
            -not [string]::Equals($candidate.ExecutablePath, $HostPath, [StringComparison]::OrdinalIgnoreCase)) { continue }
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId = $($candidate.ParentProcessId)" -OperationTimeoutSec 3 -ErrorAction Stop
        if (Test-HostMemoryIdentity $candidate $parent $HostPath $DesktopPath $ExpectedParent) {
            [pscustomobject]@{ Target = $candidate; Parent = $parent }
        }
    }
}

# Dot sourcing exposes only the identity helpers for isolated fixtures.
if ($MyInvocation.InvocationName -eq '.') { return }
$ErrorActionPreference = 'Stop'
$report = [ordered]@{
    SchemaVersion = 1; Status = 'starting'; ObservedUtc = [DateTimeOffset]::UtcNow.ToString('o')
    Samples = 0; GcCountersPresent = $false; IntervalSeconds = $IntervalSeconds; DurationSeconds = $DurationSeconds
}
$collector = $null
$outputReady = $false
$rows = [Collections.Generic.List[object]]::new()
$exitCode = 2
try {
    $HostExePath = [IO.Path]::GetFullPath($HostExePath)
    $DesktopExePath = [IO.Path]::GetFullPath($DesktopExePath)
    $matchedHosts = @(Get-HostMemoryCandidates $HostExePath $DesktopExePath $TargetProcessId $ExpectedParentProcessId)
    $report.MatchedHosts = $matchedHosts.Count
    if ($matchedHosts.Count -ne 1) {
        $report.Status = if ($matchedHosts.Count -eq 0) { 'no_matching_ordinary_host' } else { 'ambiguous_ordinary_host' }
    } else {
        $selected = $matchedHosts[0]
        $targetId = [int]$selected.Target.ProcessId
        $parentId = [int]$selected.Parent.ProcessId
        $targetCreated = $selected.Target.CreationDate
        $parentCreated = $selected.Parent.CreationDate
        $report.Identity = [ordered]@{
            Role = 'ordinary_platform_host'; ProcessId = $targetId; ParentProcessId = $parentId
            HostExePath = $HostExePath; DesktopExePath = $DesktopExePath
            HostCreatedUtc = $targetCreated.ToUniversalTime().ToString('o')
            ParentCreatedUtc = $parentCreated.ToUniversalTime().ToString('o')
            SessionId = $selected.Target.SessionId
        }
        $counterTool = Get-Command dotnet-counters -CommandType Application -ErrorAction Stop
        if (Test-Path -LiteralPath $OutputDirectory) { throw 'OutputDirectory must be a new directory.' }
        $null = New-Item -ItemType Directory -Path $OutputDirectory
        $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
        $outputReady = $true
        $runtimePath = Join-Path $OutputDirectory 'runtime.csv'
        $processPath = Join-Path $OutputDirectory 'process.csv'
        $provider = 'EventCounters\System.Runtime[gc-heap-size,gc-committed,gen-2-gc-count,alloc-rate]'
        $counterArguments = @('collect', '--process-id', "$targetId", '--counters', $provider,
            '--refresh-interval', "$IntervalSeconds", '--duration', ([TimeSpan]::FromSeconds($DurationSeconds).ToString('c')),
            '--resume-runtime:false', '--format', 'csv', '--output', ('"' + $runtimePath + '"'))
        $collector = Start-Process -FilePath $counterTool.Source -ArgumentList $counterArguments -PassThru -NoNewWindow `
            -RedirectStandardOutput (Join-Path $OutputDirectory 'counter.stdout.txt') `
            -RedirectStandardError (Join-Path $OutputDirectory 'counter.stderr.txt')
        $report.Status = 'completed'
        $report.CaptureStartedUtc = [DateTimeOffset]::UtcNow.ToString('o')
        $report.CounterProvider = $provider
        $report.RuntimeTimestampTimeZone = [TimeZoneInfo]::Local.Id
        $clock = [Diagnostics.Stopwatch]::StartNew()
        $nativeStart = $null
        do {
            $current = @(Get-HostMemoryCandidates $HostExePath $DesktopExePath $targetId $parentId)
            if ($current.Count -ne 1 -or $current[0].Target.CreationDate -ne $targetCreated -or
                $current[0].Parent.CreationDate -ne $parentCreated) { $report.Status = 'target_identity_lost'; break }
            if ($collector.HasExited) { $report.Status = 'counter_exited_early'; break }
            $observedProcess = Get-Process -Id $targetId -ErrorAction Stop
            try {
                $observedProcess.Refresh()
                $started = $observedProcess.StartTime.ToUniversalTime().Ticks
                if ($null -ne $nativeStart -and $started -ne $nativeStart) { $report.Status = 'target_identity_lost'; break }
                $nativeStart = $started
                $row = [pscustomobject]@{
                    TimestampUtc = [DateTimeOffset]::UtcNow.ToString('o')
                    ElapsedSeconds = [Math]::Round($clock.Elapsed.TotalSeconds, 3)
                    PrivateCommitBytes = $observedProcess.PrivateMemorySize64
                    WorkingSetBytes = $observedProcess.WorkingSet64
                    Handles = $observedProcess.HandleCount
                    Threads = $observedProcess.Threads.Count
                }
                $row | Export-Csv -LiteralPath $processPath -NoTypeInformation -Encoding UTF8 -Append
                $rows.Add($row)
            } finally { $observedProcess.Dispose() }
            $remaining = $DurationSeconds - $clock.Elapsed.TotalSeconds
            if ($remaining -le 0) { break }
            Start-Sleep -Milliseconds ([int][Math]::Ceiling(1000 * [Math]::Min($IntervalSeconds, $remaining)))
        } while ($rows.Count -le [Math]::Ceiling($DurationSeconds / $IntervalSeconds))
        if ($report.Status -eq 'completed' -and -not $collector.WaitForExit(15000)) { $report.Status = 'counter_stop_timeout' }
        $exitCode = 1
    }
} catch {
    $report.Status = 'capture_failed'
    $report.FailureType = $_.Exception.GetType().FullName
    $report.FailureHResult = $_.Exception.HResult
    $exitCode = 1
} finally {
    if ($null -ne $collector) {
        try {
            # This is only the collector we started, never the observed Host or Desktop.
            if (-not $collector.HasExited) { $collector.Kill(); $null = $collector.WaitForExit(5000) }
            if ($collector.HasExited) { $report.CounterExitCode = $collector.ExitCode }
        } catch {
            $report.Status = 'counter_cleanup_failed'
            $report.CleanupHResult = $_.Exception.HResult
        } finally { $collector.Dispose() }
    }
    if ($outputReady) {
        $report.Samples = $rows.Count
        if ($rows.Count -gt 0) {
            $report.First = $rows[0]
            $report.Last = $rows[$rows.Count - 1]
            $report.PeakPrivateCommitBytes = ($rows | Measure-Object PrivateCommitBytes -Maximum).Maximum
        }
        try {
            if (Test-Path -LiteralPath $runtimePath) {
                $runtimeRows = @(Import-Csv -LiteralPath $runtimePath)
                $names = @($runtimeRows | ForEach-Object { $_.'Counter Name' } | Sort-Object -Unique)
                $report.GcCountersPresent = [bool](($names -match '^GC Heap Size') -and ($names -match '^GC Committed Bytes'))
            }
        } catch { $report.GcCountersPresent = $false }
        if ($report.Status -eq 'completed') {
            if ($report.Samples -gt 0 -and $report.GcCountersPresent -and $report.CounterExitCode -eq 0) { $exitCode = 0 }
            else { $report.Status = 'incomplete_counters'; $exitCode = 1 }
        }
        $report.CaptureEndedUtc = [DateTimeOffset]::UtcNow.ToString('o')
        $report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'summary.json') -Encoding UTF8
    }
}
$report | ConvertTo-Json -Depth 6
exit $exitCode
