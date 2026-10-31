$ErrorActionPreference='Stop'
$task='.trellis\tasks\10-26-final-convergence-admission'
$results=@()
foreach($entry in @(
 @{name='spec-contracts';command='python tests/architecture/spec-contracts.py'},
 @{name='diff-check';command='git diff --check'}
)) {
 $log=Join-Path $task ('evidence\'+$entry.name+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 $started=[DateTime]::UtcNow
 & cmd /d /c "$($entry.command) > `"$log`" 2>&1"
 $code=$LASTEXITCODE
 $results+=@{name=$entry.name;command=$entry.command;exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');scope='documentation/whitespace only; not integrated product verification';sha256=(Get-FileHash -LiteralPath $log -Algorithm SHA256).Hash.ToLowerInvariant()}
 Write-Output "$($entry.name) exit=$code";Get-Content $log -Tail 5
}
$results|ConvertTo-Json -Depth 5|Set-Content (Join-Path $task 'evidence\command-results.json') -Encoding UTF8
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $task 'evidence\document-checks.ps1')
if(@($results|Where-Object exit -ne 0).Count){exit 1}
exit 0
