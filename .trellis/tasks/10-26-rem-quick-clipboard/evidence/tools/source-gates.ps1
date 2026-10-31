$ErrorActionPreference='Stop'
$gateRoot=Join-Path $env:PI_SCRATCH_DIR 'rem-quick-clipboard'
$resultsPath=Join-Path $gateRoot 'source-gate-results.json'
if(Test-Path $resultsPath){throw 'Write-once results already exist'}
$results=@()
foreach($gate in @('download-ui-lifecycle','desktop-shell-lifecycle','clipboard-ui-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')) {
 $log=Join-Path $gateRoot ($gate+'.log'); if(Test-Path $log){throw 'Write-once gate log exists'}
 $started=[DateTime]::UtcNow
 & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 $code=$LASTEXITCODE
 $results += [pscustomobject]@{command="powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/$gate.ps1";exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');log=$log;level='Source/static only'}
 Write-Output "$gate exit=$code"; Get-Content $log -Tail 3
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')) {
 $log=Join-Path $gateRoot ($gate+'.log'); if(Test-Path $log){throw 'Write-once gate log exists'}
 $started=[DateTime]::UtcNow
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE
 $results += [pscustomobject]@{command="python tests/architecture/$gate.py";exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');log=$log;level='Source/document only'}
 Write-Output "$gate exit=$code"; Get-Content $log -Tail 3
}
$results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $resultsPath -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
