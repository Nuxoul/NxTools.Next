$ErrorActionPreference='Continue'
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
$revisionRoot=Join-Path $env:PI_SCRATCH_DIR 'item04\revision-r1-r2'
$checks=@(
 @{name='standard-audit04';arguments=@('test','-p','nxtools-desktop','audit04_','--','--test-threads=1')},
 @{name='original-archive-job-regressions';arguments=@('test','-p','nxtools-desktop','archive::application::jobs::tests::','--','--test-threads=1','--skip','audit04_')},
 @{name='desktop-strict-clippy';arguments=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')}
)
$results=New-Object 'System.Collections.Generic.List[object]'
foreach($check in $checks){
 $idleClock=[Diagnostics.Stopwatch]::StartNew()
 while(@(Get-Process cargo,rustc -ErrorAction SilentlyContinue).Count -gt 0){
  if($idleClock.Elapsed.TotalMinutes -gt 90){throw 'Other Cargo not idle; no duplicate build started'}
  Start-Sleep -Seconds 10
 }
 Write-Output ('BEGIN '+$check.name)
 $checkClock=[Diagnostics.Stopwatch]::StartNew()
 $arguments=$check.arguments
 & cargo @arguments 2>&1 | Tee-Object -FilePath (Join-Path $revisionRoot ($check.name+'.log'))
 $checkExit=$LASTEXITCODE
 $results.Add([pscustomobject]@{name=$check.name;command=('cargo '+($arguments -join ' '));exit_code=$checkExit;elapsed_seconds=[Math]::Round($checkClock.Elapsed.TotalSeconds,2)})
 ConvertTo-Json -InputObject @($results.ToArray()) -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $revisionRoot 'verification-summary.json')
 Write-Output ('END '+$check.name+' exit='+$checkExit)
}
rustfmt --check --edition 2024 --config skip_children=true rust/apps/nxtools-desktop/src/archive/job_tests.rs 2>&1 | Tee-Object -FilePath (Join-Path $revisionRoot 'job-tests-scoped-fmt.log')
$results.Add([pscustomobject]@{name='job-tests-scoped-fmt';command='rustfmt --check --edition 2024 --config skip_children=true rust/apps/nxtools-desktop/src/archive/job_tests.rs';exit_code=$LASTEXITCODE})
ConvertTo-Json -InputObject @($results.ToArray()) -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $revisionRoot 'verification-summary.json')
$results | Format-Table name,exit_code,elapsed_seconds
if(@($results | Where-Object {$_.exit_code -ne 0}).Count -gt 0){exit 1}
