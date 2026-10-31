$ErrorActionPreference='Stop'
$gateRoot=Join-Path $env:PI_SCRATCH_DIR 'cursor-semantics'
$resultPath=Join-Path $gateRoot 'source-gate-results.json'
if(Test-Path $resultPath){throw 'Immutable results exist'}
$results=@()
foreach($gate in @('download-ui-lifecycle','desktop-shell-lifecycle','clipboard-ui-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')){
 $log=Join-Path $gateRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 $code=$LASTEXITCODE
 $results+=[pscustomobject]@{command="powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/$gate.ps1";exit=$code;log=$log;level='Source only'}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')){
 $log=Join-Path $gateRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE
 $results+=[pscustomobject]@{command="python tests/architecture/$gate.py";exit=$code;log=$log;level='Source/document/provenance only'}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
$results | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $resultPath -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
