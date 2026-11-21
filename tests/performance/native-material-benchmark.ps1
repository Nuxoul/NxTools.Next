# Sequential, test-only benchmark. No elevation, installation or product configuration.
[CmdletBinding()]
param(
    [string] $OutputDirectory = '',
    [ValidateRange(2, 120)] [int] $Seconds = 15,
    [ValidateRange(1, 20)] [int] $Repetitions = 3,
    [ValidateSet('idle', 'background', 'input', 'scroll', 'multiwindow')]
    [string[]] $Scenarios = @('idle', 'background', 'input', 'scroll', 'multiwindow'),
    [string] $CandidateExecutable = '',
    [string] $FrameControlDirectory = ''
)

function ConvertFrom-MaterialBenchmarkEvent {
    param([string] $Line, [Guid] $Correlation)
    $pattern = '\bui\.material_benchmark\.(?<event>phase_started|phase_finished) correlation=(?<id>[0-9a-fA-F-]{36}) phase=(?<phase>idle|input|scroll) generation=(?<generation>\d+) updates=(?<updates>\d+) visible_row=(?<row>\d+)(?: max_rendered_row=(?<rendered>\d+))?\s*$'
    $match = [regex]::Match($Line, $pattern)
    if ($match.Success -and $match.Groups['id'].Value -eq $Correlation.ToString()) {
        return [pscustomobject]@{
            Event = $match.Groups['event'].Value; Phase = $match.Groups['phase'].Value
            Generation = [long]$match.Groups['generation'].Value
            Updates = [long]$match.Groups['updates'].Value; VisibleRow = [long]$match.Groups['row'].Value
            MaxRenderedRow = $(if ($match.Groups['rendered'].Success) { [long]$match.Groups['rendered'].Value } else { 0L })
        }
    }
    $match = [regex]::Match($Line, '\bui\.material_benchmark\.(?<event>completed) correlation=(?<id>[0-9a-fA-F-]{36})\s*$')
    if ($match.Success -and $match.Groups['id'].Value -eq $Correlation.ToString()) {
        return [pscustomobject]@{ Event = 'completed'; Phase = $null; Generation = 0L; Updates = 0L; VisibleRow = 0L }
    }
    return $null
}

function Get-MaterialBenchmarkOrder {
    param([int] $Repetition)
    if ($Repetition % 2 -eq 1) { return @('solid', 'acrylic') }
    return @('acrylic', 'solid')
}
function Read-MaterialBenchmarkLog {
    param([string] $Path)
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    $reader = New-Object IO.StreamReader $stream
    try { return $reader.ReadToEnd() }
    finally { $reader.Dispose() }
}


# Dot-sourcing exposes only the pure parser/order seams to fixture tests.
if ($MyInvocation.InvocationName -eq '.') { return }
Set-StrictMode -Version Latest
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $env:PI_SCRATCH_DIR ('native-material-benchmark-' + [Guid]::NewGuid().ToString('N')) }
if (-not $CandidateExecutable) { $CandidateExecutable = Join-Path $PSScriptRoot '..\..\target\debug\preview-native-material\quick_launch_surface.exe' }
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$CandidateExecutable = (Resolve-Path -LiteralPath $CandidateExecutable).Path
$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
if ($FrameControlDirectory) {
    $FrameControlDirectory = (Resolve-Path -LiteralPath $FrameControlDirectory).Path
    if (-not (Test-Path -LiteralPath (Join-Path $FrameControlDirectory 'broker-ready.json'))) { throw 'Temporary frame capture broker is not ready.' }
}
Add-Type -Path (Join-Path $root 'scripts\preview-native-material-window.cs')
Add-Type -Path (Join-Path $PSScriptRoot 'NativeMaterialBenchmarkDriver.cs') -ReferencedAssemblies System.dll, System.Core.dll, System.Drawing.dll, System.Windows.Forms.dll
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class NativeMaterialBenchmarkClose {
    [DllImport("user32.dll", EntryPoint="PostMessageW", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool PostMessage(IntPtr window, uint message, IntPtr first, IntPtr second);
    public static void Close(long handle) {
        if (!PostMessage(new IntPtr(handle), 0x10, IntPtr.Zero, IntPtr.Zero))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    }
}
'@

$helperProcessId = [Diagnostics.Process]::GetCurrentProcess().Id
$sessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
$dwmProcesses = @(Get-Process -Name dwm | Where-Object { $_.SessionId -eq $sessionId })
if ($dwmProcesses.Count -ne 1) { throw 'Expected exactly one DWM process in the benchmark session.' }
$dwmProcessId = $dwmProcesses[0].Id
$runIndex = 0
foreach ($scenario in $Scenarios) {
    for ($repetition = 1; $repetition -le $Repetitions; $repetition++) {
        foreach ($material in (Get-MaterialBenchmarkOrder $repetition)) {
            $runIndex++
            $correlation = [Guid]::NewGuid()
            $runDirectory = Join-Path $OutputDirectory ('{0:D3}-{1}-{2}-{3}' -f $runIndex, $scenario, $repetition, $material)
            $null = New-Item -ItemType Directory -Path $runDirectory
            $stdoutPath = Join-Path $runDirectory 'stdout.log'
            $stderrPath = Join-Path $runDirectory 'stderr.log'
            $targetProcess = $null; $sampler = $null; $backdrop = $null; $sampleWriter = $null; $eventWriter = $null
            $runClock = [Diagnostics.Stopwatch]::StartNew()
            $metadata = [ordered]@{
                SchemaVersion = 1; Correlation = $correlation.ToString(); Scenario = $scenario
                Material = $material; Repetition = $repetition; Seconds = $Seconds
                StopwatchFrequency = [Diagnostics.Stopwatch]::Frequency; PollMilliseconds = 250
                BoundaryObservation = 'Parent log polling timestamps; delayed by polling and scheduling. Use inner boundaries trimmed by 0.5 seconds during analysis.'
                AnalysisBoundaryTrimSeconds = 0.5; Status = 'started'
            }
            try {
                $backdrop = New-Object NativeMaterialBenchmarkBackdrop
                $backdrop.Start()
                $moving = $scenario -in @('background', 'multiwindow')
                $backdrop.SetMoving($moving)
                $backdropCheck = [Diagnostics.Stopwatch]::StartNew()
                $initialUpdates = $backdrop.UpdateCount
                while ($backdrop.IsMoving -ne $moving -or ($moving -and $backdrop.UpdateCount -le $initialUpdates)) {
                    if ($backdropCheck.Elapsed.TotalSeconds -ge 2) { throw 'Backdrop movement did not reach the requested state.' }
                    Start-Sleep -Milliseconds 25
                }
                $sampler = New-Object NativeMaterialBenchmarkSampler
                $sampleWriter = New-Object IO.StreamWriter (Join-Path $runDirectory 'samples.jsonl'), $false, ([Text.UTF8Encoding]::new($false))
                $eventWriter = New-Object IO.StreamWriter (Join-Path $runDirectory 'events.jsonl'), $false, ([Text.UTF8Encoding]::new($false))
                $launchQpc = [Diagnostics.Stopwatch]::GetTimestamp()
                $launchUtc = [DateTime]::UtcNow.ToString('o')
                $launchClock = [Diagnostics.Stopwatch]::StartNew()
                $arguments = @('--native-material-preview', "--material=$material", "--benchmark=$scenario", "--benchmark-seconds=$Seconds", "--correlation=$correlation")
                $targetProcess = Start-Process -FilePath $CandidateExecutable -ArgumentList $arguments -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
                $null = $targetProcess.Handle
                $metadata.LaunchQpc = $launchQpc; $metadata.LaunchUtc = $launchUtc
                $metadata.Roles = [ordered]@{ gpui = $targetProcess.Id; helper = $helperProcessId; dwm = $dwmProcessId }
                $metadata.BackdropWindow = $backdrop.WindowHandle
                $selectedIds = [int[]]@($targetProcess.Id, $helperProcessId, $dwmProcessId)
                $expectedCount = if ($scenario -eq 'multiwindow') { 3 } else { 1 }
                $expectedPhase = if ($scenario -in @('input', 'scroll')) { $scenario } else { 'idle' }
                $prepared = @{}; $receipts = New-Object 'System.Collections.Generic.List[object]'
                $lineCount = 0; $phaseStarted = $null; $phaseFinished = $null; $completed = $false
                $baseline = $true; $motionAtPhaseStart = 0L
                $traceRequested = $false
                while ($true) {
                    $pollClock = [Diagnostics.Stopwatch]::StartNew()
                    $qpcBefore = [Diagnostics.Stopwatch]::GetTimestamp()
                    $utcBefore = [DateTime]::UtcNow.ToString('o')
                    if ($null -eq $phaseFinished) {
                        $samples = @($sampler.Read($selectedIds))
                        $qpcAfter = [Diagnostics.Stopwatch]::GetTimestamp()
                        $sampleWriter.WriteLine(([ordered]@{
                            Qpc = $qpcBefore; QpcAfter = $qpcAfter; Utc = $utcBefore; Baseline = $baseline
                            Stage = $(if ($null -eq $phaseStarted) { 'warmup' } else { 'measurement' })
                            BackdropMoving = $backdrop.IsMoving; BackdropUpdates = $backdrop.UpdateCount
                            BackdropPaints = $backdrop.PaintCount; Samples = $samples
                        } | ConvertTo-Json -Depth 8 -Compress))
                        $baseline = $false
                    }
                    if (Test-Path -LiteralPath $stdoutPath) {
                        # Only complete lines are consumed; keep the full unmodified redirected log.
                        $text = Read-MaterialBenchmarkLog $stdoutPath
                        $lines = $text -split "`n"
                        for (; $lineCount -lt $lines.Count - 1; $lineCount++) {
                            $phaseEvent = ConvertFrom-MaterialBenchmarkEvent $lines[$lineCount] $correlation
                            if ($null -eq $phaseEvent) { continue }
                            $observed = [Diagnostics.Stopwatch]::GetTimestamp()
                            $eventWriter.WriteLine(([ordered]@{ ObservedQpc = $observed; ObservedUtc = [DateTime]::UtcNow.ToString('o'); Event = $phaseEvent } | ConvertTo-Json -Compress))
                            if ($phaseEvent.Event -eq 'phase_started') {
                                if ($null -ne $phaseStarted -or $prepared.Count -ne $expectedCount -or @([NativeMaterialPreviewWindow]::FindWindows($targetProcess.Id)).Count -ne $expectedCount) { throw 'Unexpected phase start or native preparation incomplete.' }
                                if ($FrameControlDirectory -and -not (Test-Path -LiteralPath (Join-Path $FrameControlDirectory "started-$correlation.json"))) { throw 'Frame capture did not start before the measured phase.' }
                                if ($phaseEvent.Phase -ne $expectedPhase -or $phaseEvent.Generation -ne 1 -or $phaseEvent.Updates -ne 0 -or $phaseEvent.VisibleRow -ne 0) { throw 'Invalid phase start scalars.' }
                                $phaseStarted = $observed; $motionAtPhaseStart = $backdrop.UpdateCount
                            }
                            elseif ($phaseEvent.Event -eq 'phase_finished') {
                                if ($null -eq $phaseStarted -or $null -ne $phaseFinished -or $phaseEvent.Phase -ne $expectedPhase -or $phaseEvent.Generation -ne 1) { throw 'Unexpected phase finish.' }
                                if ($expectedPhase -ne 'idle' -and $phaseEvent.Updates -le 0) { throw 'Input/scroll task made no updates.' }
                                if ($expectedPhase -eq 'scroll' -and $phaseEvent.MaxRenderedRow -le 0) { throw 'Virtual list did not render later rows.' }
                                if ($moving -and (-not $backdrop.IsMoving -or $backdrop.UpdateCount -le $motionAtPhaseStart)) { throw 'Backdrop did not move during measurement.' }
                                if (-not $moving -and ($backdrop.IsMoving -or $backdrop.UpdateCount -ne $initialUpdates)) { throw 'Static backdrop unexpectedly moved.' }
                                $phaseFinished = $observed
                            }
                            else {
                                if ($null -eq $phaseFinished -or $completed) { throw 'Unexpected benchmark completion.' }
                                $completed = $true
                            }
                        }
                    }
                    if ($prepared.Count -lt $expectedCount) {
                        if ($launchClock.Elapsed.TotalSeconds -ge 7) { throw 'Native preparation exceeded the seven-second deadline.' }
                        $windows = @([NativeMaterialPreviewWindow]::FindWindows($targetProcess.Id))
                        if ($windows.Count -gt $expectedCount) { throw 'Unexpected number of owned preview windows.' }
                        foreach ($windowHandle in $windows) {
                            if (-not $prepared.ContainsKey($windowHandle)) {
                                $receipt = [NativeMaterialPreviewWindow]::Prepare($targetProcess.Id, [long]$windowHandle, ($material -eq 'acrylic'), $false, $correlation)
                                $receipts.Add($receipt); $prepared[$windowHandle] = $true
                                ConvertTo-Json -InputObject @($receipts.ToArray()) -Depth 6 | Set-Content -LiteralPath (Join-Path $runDirectory 'native-receipts.json') -Encoding UTF8
                                if ($launchClock.Elapsed.TotalSeconds -ge 7) { throw 'Native preparation exceeded the seven-second deadline.' }
                            }
                        }
                        if ($prepared.Count -eq $expectedCount) { $metadata.PrepareMilliseconds = $launchClock.Elapsed.TotalMilliseconds }
                        if ($prepared.Count -eq $expectedCount -and $FrameControlDirectory -and -not $traceRequested) {
                            $request = @{correlation=$correlation.ToString();processId=$targetProcess.Id;dwmProcessId=$dwmProcessId;seconds=$Seconds}
                            $requestPath = Join-Path $FrameControlDirectory "request-$correlation.json"
                            $request | ConvertTo-Json | Set-Content -LiteralPath ($requestPath+'.tmp') -Encoding UTF8
                            Move-Item -LiteralPath ($requestPath+'.tmp') -Destination $requestPath
                            $traceRequested = $true
                        }
                    }
                    $targetProcess.Refresh()
                    if ($targetProcess.HasExited) {
                        $targetProcess.WaitForExit()
                        if (-not $completed -or $null -eq $phaseFinished -or $targetProcess.ExitCode -ne 0) { throw 'Target exited without successful phase completion and exit code zero.' }
                        break
                    }
                    if ($null -ne $phaseFinished -and ([Diagnostics.Stopwatch]::GetTimestamp() - $phaseFinished) / [Diagnostics.Stopwatch]::Frequency -gt 5) { throw 'Target did not exit normally within five seconds of phase finish.' }
                    if ($launchClock.Elapsed.TotalSeconds -gt (8 + $Seconds + 10)) { throw 'Benchmark exceeded its bounded lifecycle deadline.' }
                    $sleepMilliseconds = [Math]::Max(0, 250 - [int]$pollClock.ElapsedMilliseconds)
                    if ($sleepMilliseconds -gt 0) { Start-Sleep -Milliseconds $sleepMilliseconds }
                }
                if ($FrameControlDirectory) {
                    $traceDeadline = [Diagnostics.Stopwatch]::StartNew()
                    $finishedPath = Join-Path $FrameControlDirectory "finished-$correlation.json"
                    while (-not (Test-Path -LiteralPath $finishedPath)) {
                        if ((Test-Path -LiteralPath (Join-Path $FrameControlDirectory "failed-$correlation.json")) -or (Test-Path -LiteralPath (Join-Path $FrameControlDirectory 'broker-failed.json'))) { throw 'Temporary frame capture failed; inspect its raw diagnostics.' }
                        if ($traceDeadline.Elapsed.TotalSeconds -gt 30) { throw 'Temporary frame capture completion timed out.' }
                        Start-Sleep -Milliseconds 100
                    }
                    $metadata.Frames = Get-Content -LiteralPath $finishedPath -Raw | ConvertFrom-Json
                    if (@($metadata.Frames.captures | Where-Object { $_.exitCode -ne 0 }).Count) { throw 'PresentMon returned a nonzero exit code.' }
                }
                $metadata.PhaseStartedObservedQpc = $phaseStarted; $metadata.PhaseFinishedObservedQpc = $phaseFinished
                $metadata.ExitCode = $targetProcess.ExitCode; $metadata.Status = 'passed'
            }
            catch {
                $metadata.Status = 'failed'; $metadata.Failure = $_.Exception.Message
                throw
            }
            finally {
                try {
                    if ($null -ne $targetProcess -and -not $targetProcess.HasExited) {
                        foreach ($ownedWindow in [NativeMaterialPreviewWindow]::FindWindows($targetProcess.Id)) {
                            [NativeMaterialBenchmarkClose]::Close($ownedWindow)
                        }
                        if (-not $targetProcess.WaitForExit(5000)) { Write-Warning "Owned benchmark process $($targetProcess.Id) did not close normally; it was not terminated." }
                    }
                }
                finally {
                    if ($null -ne $sampleWriter) { $sampleWriter.Dispose() }
                    if ($null -ne $eventWriter) { $eventWriter.Dispose() }
                    if ($null -ne $sampler) { $sampler.Dispose() }
                    if ($null -ne $backdrop) { $backdrop.Dispose() }
                    if ($null -ne $targetProcess) { $targetProcess.Dispose() }
                    $metadata.ElapsedMilliseconds = $runClock.Elapsed.TotalMilliseconds
                    $metadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $runDirectory 'metadata.json') -Encoding UTF8
                }
            }
        }
    }
}
Write-Output ([pscustomobject]@{ Result = 'passed'; Runs = $runIndex; OutputDirectory = $OutputDirectory })
