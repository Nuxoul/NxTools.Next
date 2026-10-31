$ErrorActionPreference='Stop'
$results=@()
foreach($entry in @(
 @{name='release-chain-final';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\quick-launch-release-chain-tests.ps1 -SourceOnly'},
 @{name='archive-release-source';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\archive-release-chain.ps1 -SourceOnly'},
 @{name='indexer-source';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\indexer-lifecycle.ps1'},
 @{name='platformhost-test-selection';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\platformhost-test-selection.ps1'},
 @{name='browser-signing-isolation';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\browser-signing-isolation.ps1'},
 @{name='package-memory-policy';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\package-memory-policy.ps1'},
 @{name='trusted-release-sequence';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\trusted-release-sequence.ps1'},
 @{name='desktop-shell-source';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\desktop-shell-lifecycle.ps1'},
 @{name='quick-full-source-final';command='powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\quick-launch-lifecycle.ps1'},
 @{name='spec-contracts';command='python tests/architecture/spec-contracts.py'},
 @{name='diff-check';command='git diff --check'}
)){
 $log=Join-Path $PSScriptRoot ($entry.name+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 $started=[DateTime]::UtcNow
 & cmd /d /c "$($entry.command) > `"$log`" 2>&1"
 $code=$LASTEXITCODE;$results+=@{name=$entry.name;command=$entry.command;exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o')}
 Write-Output "$($entry.name) exit=$code";Get-Content $log -Tail 5
}
$results|ConvertTo-Json -Depth 4|Set-Content (Join-Path $PSScriptRoot 'check-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
