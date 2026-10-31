$ErrorActionPreference='Stop'
$results=@()
foreach($path in @('tests\architecture\quick-launch-lifecycle.ps1','tests\architecture\quick-launch-release-chain.ps1','tests\architecture\quick-launch-release-chain-tests.ps1')){
 $sourceTokens=$null;$sourceErrors=$null
 $null=[Management.Automation.Language.Parser]::ParseFile((Join-Path (Get-Location) $path),[ref]$sourceTokens,[ref]$sourceErrors)
 if($sourceErrors.Count){throw "PowerShell parse failed: $path"}
 $results+=@{path=$path;parse='PASS'}
}
$log=Join-Path $PSScriptRoot 'sourceonly-required.log';if(Test-Path $log){throw 'Immutable log exists'}
& cmd /d /c "powershell -NoProfile -ExecutionPolicy Bypass -File tests\architecture\quick-launch-release-chain-tests.ps1 > `"$log`" 2>&1"
$code=$LASTEXITCODE
if($code -eq 0 -or -not (Get-Content -Raw $log).Contains('This regression requires -SourceOnly')){throw 'Missing SourceOnly must reject before fixtures'}
$results+=@{name='SourceOnly required';exit=$code;expected_rejection='PASS'}
$results+=@{name='PSScriptAnalyzer';available=[bool](Get-Module -ListAvailable PSScriptAnalyzer)}
$results|ConvertTo-Json -Depth 4|Set-Content (Join-Path $PSScriptRoot 'final-source-results.json') -Encoding UTF8
$results|ConvertTo-Json -Depth 4
