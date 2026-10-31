$ErrorActionPreference='Continue'
$env:CARGO_BUILD_JOBS='1'; $env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C codegen-units=256'
$evidenceRoot=Join-Path $env:PI_SCRATCH_DIR 'item06'
$checks=@(
 @{name='focused-final';command='cargo test -p nxtools-desktop audit06_ -- --test-threads=1'},
 @{name='downloads-related';command='cargo test -p nxtools-desktop downloads:: -- --test-threads=1'},
 @{name='main-window-related';command='cargo test -p nxtools-desktop main_window:: -- --test-threads=1'},
 @{name='default-submit-protected';command='cargo test -p nxtools-desktop audit05_ -- --test-threads=1'},
 @{name='history-identity-protected';command='cargo test -p nxtools-desktop history_identity_ -- --test-threads=1'},
 @{name='accessible-names-protected';command='cargo test -p nxtools-desktop accessible_name_ -- --test-threads=1'},
 @{name='sort-keyboard-protected';command='cargo test -p nxtools-desktop sort_keyboard_ -- --test-threads=1'},
 @{name='prompt-display-protected';command='cargo test -p nxtools-desktop audit04_ -- --test-threads=1'},
 @{name='desktop-strict-clippy';command='cargo clippy -p nxtools-desktop --tests -- -D warnings'},
 @{name='workspace-check';command='cargo check --workspace --tests'},
 @{name='workspace-fmt';command='cargo fmt --all -- --check'}
)
$checkResults=New-Object 'System.Collections.Generic.List[object]'
foreach($check in $checks){
 $idleClock=[Diagnostics.Stopwatch]::StartNew()
 while(@(Get-Process cargo,rustc -ErrorAction SilentlyContinue).Count -gt 0){if($idleClock.Elapsed.TotalMinutes -gt 90){throw 'Other Cargo did not become idle; no duplicate build started'}; Start-Sleep -Seconds 10}
 $logPath=Join-Path $evidenceRoot ($check.name+'.log')
 Write-Output ('BEGIN '+$check.name)
 $checkClock=[Diagnostics.Stopwatch]::StartNew()
 cmd /d /c ($check.command+' 2>&1') > $logPath
 $checkExit=$LASTEXITCODE
 $checkResults.Add([pscustomobject]@{name=$check.name;command=$check.command;exit_code=$checkExit;elapsed_seconds=[Math]::Round($checkClock.Elapsed.TotalSeconds,2);log=$logPath})
 ConvertTo-Json -InputObject @($checkResults.ToArray()) -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $evidenceRoot 'focused-verification.json')
 Get-Content -Encoding UTF8 $logPath | Select-Object -Last 22
 Write-Output ('END '+$check.name+' exit='+$checkExit)
 if($checkExit -ne 0){exit $checkExit}
}
$checkResults | Format-Table name,exit_code,elapsed_seconds
