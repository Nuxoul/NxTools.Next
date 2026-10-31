$ErrorActionPreference='Continue'
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
$verifyRoot=Join-Path $env:PI_SCRATCH_DIR 'item04'
$verifyCommands=@(
 @{name='archive-clippy-final';args=@('clippy','-p','nxtools-archive','--features','isolated-codecs','--tests','--','-D','warnings')},
 @{name='desktop-clippy-final';args=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')},
 @{name='protocol-clippy-final';args=@('clippy','-p','nxtools-protocol','--tests','--','-D','warnings')},
 @{name='worker-direct-regressions';args=@('test','-p','nxtools-archive','--features','isolated-codecs','--test','prompt_display_contract','--test','worker_wire','--test','worker_extraction_contract','--test','interaction_contract','--test','job_execution_contract','--test','output_readmission_contract','--test','worker_cancel_contract','--','--test-threads=1')},
 @{name='ipc-regressions';args=@('test','-p','nxtools-ipc','--lib','--','--test-threads=1')},
 @{name='protocol-archive-final';args=@('test','-p','nxtools-protocol','archive_job::','--lib','--','--test-threads=1')},
 @{name='desktop-object-final';args=@('test','-p','nxtools-desktop','audit04_','--','--test-threads=1')},
 @{name='workspace-check';args=@('check','--workspace','--tests')},
 @{name='workspace-fmt';args=@('fmt','--all','--check')}
)
$verifyRows=@()
foreach($verifyCommand in $verifyCommands){
 $waitClock=[Diagnostics.Stopwatch]::StartNew()
 while(@(Get-Process cargo,rustc -ErrorAction SilentlyContinue).Count -gt 0){if($waitClock.Elapsed.TotalMinutes -gt 90){throw 'Other Cargo not idle; no duplicate build started'};Start-Sleep -Seconds 10}
 $verifyLog=Join-Path $verifyRoot ($verifyCommand.name+'.log')
 Write-Output ('BEGIN '+$verifyCommand.name)
 $verifyClock=[Diagnostics.Stopwatch]::StartNew()
 & cargo @($verifyCommand.args) 2>&1 | Tee-Object -FilePath $verifyLog
 $verifyExit=$LASTEXITCODE
 $verifyRows+= [pscustomobject]@{name=$verifyCommand.name;command=('cargo '+($verifyCommand.args -join ' '));exit_code=$verifyExit;elapsed_seconds=[Math]::Round($verifyClock.Elapsed.TotalSeconds,2);log=$verifyLog}
 $verifyRows | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $verifyRoot 'verification-summary.json')
 Write-Output ('END '+$verifyCommand.name+' exit='+$verifyExit)
}
$verifyRows | Format-Table name,exit_code,elapsed_seconds
