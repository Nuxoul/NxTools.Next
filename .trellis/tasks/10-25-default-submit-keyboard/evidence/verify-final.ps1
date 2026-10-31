$ErrorActionPreference='Continue'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$root=Join-Path $env:PI_SCRATCH_DIR 'item05'
$checks=@(
 @{name='focused-final';command='cargo test -p nxtools-desktop audit05_ -- --test-threads=1'},
 @{name='archive-related';command='cargo test -p nxtools-desktop archive:: -- --test-threads=1'},
 @{name='downloads-related';command='cargo test -p nxtools-desktop downloads:: -- --test-threads=1'},
 @{name='history-identity-protected';command='cargo test -p nxtools-desktop history_identity_ -- --test-threads=1'},
 @{name='accessible-names-protected';command='cargo test -p nxtools-desktop accessible_name_ -- --test-threads=1'},
 @{name='sort-keyboard-protected';command='cargo test -p nxtools-desktop sort_keyboard_ -- --test-threads=1'},
 @{name='desktop-strict-clippy';command='cargo clippy -p nxtools-desktop --tests -- -D warnings'},
 @{name='workspace-check';command='cargo check --workspace --tests'},
 @{name='workspace-fmt';command='cargo fmt --all -- --check'}
)
$results=New-Object 'System.Collections.Generic.List[object]'
foreach($check in $checks){
 $idle=[Diagnostics.Stopwatch]::StartNew()
 while(@(Get-Process cargo,rustc -ErrorAction SilentlyContinue).Count -gt 0){if($idle.Elapsed.TotalMinutes -gt 90){throw 'Other Cargo did not become idle; no duplicate build started'};Start-Sleep -Seconds 10}
 $log=Join-Path $root ($check.name+'.log')
 Write-Output ('BEGIN '+$check.name)
 $clock=[Diagnostics.Stopwatch]::StartNew()
 cmd /d /c ($check.command+' 2>&1') > $log
 $code=$LASTEXITCODE
 $results.Add([pscustomobject]@{name=$check.name;command=$check.command;exit_code=$code;elapsed_seconds=[Math]::Round($clock.Elapsed.TotalSeconds,2);log=$log})
 ConvertTo-Json -InputObject @($results.ToArray()) -Depth 4|Set-Content -Encoding UTF8 (Join-Path $root 'final-verification.json')
 Get-Content -Encoding UTF8 $log | Select-Object -Last 26
 Write-Output ('END '+$check.name+' exit='+$code)
 if($check.name -eq 'focused-final' -and $code -ne 0){exit $code}
}
$results|Format-Table name,exit_code,elapsed_seconds
