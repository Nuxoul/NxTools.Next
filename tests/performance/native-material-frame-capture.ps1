# Temporary elevated, PID-scoped ETW and DWM observer. No installation or permission changes.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$ControlDirectory,
    [Parameter(Mandatory=$true)][string]$CandidateExecutable,
    [Parameter(Mandatory=$true)][string]$PresentMonExecutable,
    [Parameter(Mandatory=$true)][int]$ExpectedSessionId
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$principal=New-Object Security.Principal.WindowsPrincipal($identity)
if(-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Temporary frame capture requires elevation.'}
$control=[IO.Path]::GetFullPath($ControlDirectory)
$candidate=[IO.Path]::GetFullPath($CandidateExecutable)
$presentMon=[IO.Path]::GetFullPath($PresentMonExecutable)
if(-not (Test-Path -LiteralPath $control -PathType Container)){throw 'Missing capture control directory.'}
if(-not (Test-Path -LiteralPath $candidate -PathType Leaf)){throw 'Missing isolated candidate.'}
if(-not (Test-Path -LiteralPath $presentMon -PathType Leaf)){throw 'Missing PresentMon executable.'}
Add-Type -Path (Join-Path $PSScriptRoot 'NativeMaterialBenchmarkDriver.cs') -ReferencedAssemblies System.dll,System.Core.dll,System.Drawing.dll,System.Windows.Forms.dll
$observerProcessId=[Diagnostics.Process]::GetCurrentProcess().Id
$started=[Diagnostics.Stopwatch]::StartNew()
function Save-CaptureState([string]$Name,$Value){
    $temporary=Join-Path $control ($Name+'.tmp')
    $Value | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination (Join-Path $control $Name) -Force
}
Save-CaptureState 'broker-ready.json' @{processId=$observerProcessId;sessionId=[Diagnostics.Process]::GetCurrentProcess().SessionId;qpcFrequency=[Diagnostics.Stopwatch]::Frequency}
$completed=New-Object 'System.Collections.Generic.HashSet[string]'
try {
    while($started.Elapsed.TotalMinutes -lt 25 -and -not (Test-Path -LiteralPath (Join-Path $control 'stop'))){
        foreach($requestFile in @(Get-ChildItem -LiteralPath $control -Filter 'request-*.json' -File)){
            if(-not $completed.Add($requestFile.Name)){continue}
            $request=Get-Content -LiteralPath $requestFile.FullName -Raw | ConvertFrom-Json
            $correlation=[Guid]::Parse([string]$request.correlation).ToString('D')
            if($requestFile.Name -ne "request-$correlation.json"){throw 'Capture request identity mismatch.'}
            $seconds=[int]$request.seconds
            if($seconds -lt 1 -or $seconds -gt 120){throw 'Capture duration outside fixture bounds.'}
            $appId=[int]$request.processId
            $dwmId=[int]$request.dwmProcessId
            $app=[Diagnostics.Process]::GetProcessById($appId)
            $dwm=[Diagnostics.Process]::GetProcessById($dwmId)
            if($app.SessionId -ne $ExpectedSessionId -or $dwm.SessionId -ne $ExpectedSessionId){throw 'Capture process is not in the authorized session.'}
            if(-not [string]::Equals($app.MainModule.FileName,$candidate,[StringComparison]::OrdinalIgnoreCase)){throw 'Capture candidate path mismatch.'}
            if($dwm.ProcessName -ne 'dwm'){throw 'Capture target is not the session DWM.'}
            $captures=@();$sampler=$null;$sampleWriter=$null
            try {
                foreach($target in @(@{role='gpui';id=$appId},@{role='dwm';id=$dwmId})){
                    $role=$target.role
                    $csv=Join-Path $control "$correlation-$role.csv"
                    $arguments=@('--process_id',[string]$target.id,'--timed',[string]($seconds+8),'--terminate_after_timed',
                        '--session_name',"NxToolsMaterial-$correlation-$role",'--output_file',('"'+$csv+'"'),
                        '--no_console_stats','--no_track_input','--qpc_time','--v2_metrics')
                    if($role -eq 'gpui'){$arguments+='--terminate_on_proc_exit'}
                    $trace=Start-Process -FilePath $presentMon -ArgumentList $arguments -PassThru -WindowStyle Hidden `
                        -RedirectStandardOutput (Join-Path $control "$correlation-$role.stdout.log") `
                        -RedirectStandardError (Join-Path $control "$correlation-$role.stderr.log")
                    $null=$trace.Handle
                    $captures+=@{role=$role;process=$trace;csv=$csv;targetProcessId=$target.id}
                }
                Start-Sleep -Milliseconds 500
                foreach($capture in $captures){if($capture.process.HasExited){throw "PresentMon exited before capture: role=$($capture.role) exit=$($capture.process.ExitCode)"}}
                $roles=@{dwm=$dwmId;capture_helper=$observerProcessId;presentmon_gpui=$captures[0].process.Id;presentmon_dwm=$captures[1].process.Id}
                $selectedIds=[int[]]@($roles.Values)
                $sampler=New-Object NativeMaterialBenchmarkSampler
                $samplePath=Join-Path $control "$correlation-samples.jsonl"
                $sampleWriter=New-Object IO.StreamWriter $samplePath,$false,([Text.UTF8Encoding]::new($false))
                Save-CaptureState "started-$correlation.json" @{status='capturing';correlation=$correlation;qpc=[Diagnostics.Stopwatch]::GetTimestamp()}
                $captureClock=[Diagnostics.Stopwatch]::StartNew()
                while(@($captures | Where-Object {-not $_.process.HasExited}).Count){
                    if($captureClock.Elapsed.TotalSeconds -gt ($seconds+15)){throw 'Owned timed PresentMon capture exceeded its deadline.'}
                    $poll=[Diagnostics.Stopwatch]::StartNew()
                    $qpc=[Diagnostics.Stopwatch]::GetTimestamp()
                    $samples=@($sampler.Read($selectedIds))
                    $sampleWriter.WriteLine((@{Qpc=$qpc;QpcAfter=[Diagnostics.Stopwatch]::GetTimestamp();Samples=$samples} | ConvertTo-Json -Depth 8 -Compress))
                    $delay=[Math]::Max(0,250-[int]$poll.ElapsedMilliseconds)
                    if($delay){Start-Sleep -Milliseconds $delay}
                }
                $results=@()
                foreach($capture in $captures){
                    $capture.process.WaitForExit()
                    $results+=@{role=$capture.role;targetProcessId=$capture.targetProcessId;exitCode=$capture.process.ExitCode;csv=$capture.csv}
                }
                $sampleWriter.Dispose();$sampleWriter=$null
                Save-CaptureState "finished-$correlation.json" @{status='finished';correlation=$correlation;captures=$results;
                    qpc=[Diagnostics.Stopwatch]::GetTimestamp();samplerPath=$samplePath;roles=$roles}
            }
            catch {
                Save-CaptureState "failed-$correlation.json" @{status='failed';correlation=$correlation;message=$_.Exception.Message}
                # Each owned capture has a bounded timer; never stop an existing ETW session.
                foreach($capture in $captures){[void]$capture.process.WaitForExit(150000)}
                throw
            }
            finally {
                if($sampleWriter){$sampleWriter.Dispose()}
                if($sampler){$sampler.Dispose()}
                foreach($capture in $captures){$capture.process.Dispose()}
                $app.Dispose();$dwm.Dispose()
            }
        }
        Start-Sleep -Milliseconds 100
    }
    Save-CaptureState 'broker-finished.json' @{status='finished';elapsedSeconds=$started.Elapsed.TotalSeconds}
}
catch {
    Save-CaptureState 'broker-failed.json' @{status='failed';message=$_.Exception.Message;elapsedSeconds=$started.Elapsed.TotalSeconds}
    exit 1
}
