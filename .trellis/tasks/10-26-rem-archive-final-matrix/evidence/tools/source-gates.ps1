$ErrorActionPreference='Stop'
$gateRoot=Join-Path $env:PI_SCRATCH_DIR 'rem-archive-final-matrix'
$resultPath=Join-Path $gateRoot 'source-gate-results.json'
if(Test-Path $resultPath){throw 'Immutable results exist'}
$results=@()
foreach($gate in @('archive-release-chain','download-ui-lifecycle','desktop-shell-lifecycle','clipboard-ui-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')){
 $log=Join-Path $gateRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable gate log exists'}
 $started=[DateTime]::UtcNow
 if($gate -eq 'archive-release-chain'){
  & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" -SourceOnly *> $log
 } else {
  & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 }
 $code=$LASTEXITCODE
 $results+=[pscustomobject]@{command="powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/$gate.ps1$(if($gate -eq 'archive-release-chain'){' -SourceOnly'})";exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');log=$log;level='SOURCE ONLY; no native or package rejection execution'}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')){
 $log=Join-Path $gateRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable gate log exists'}
 $started=[DateTime]::UtcNow
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE
 $results+=[pscustomobject]@{command="python tests/architecture/$gate.py";exit=$code;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');log=$log;level='Source/document/provenance only'}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
$results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $resultPath -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
