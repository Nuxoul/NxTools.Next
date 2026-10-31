$ErrorActionPreference='Stop'
$gateNames=@('download-ui-lifecycle','desktop-shell-lifecycle','clipboard-ui-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')
$results=@()
foreach($gate in $gateNames){
 $log=Join-Path $PSScriptRoot ($gate+'.log');if(Test-Path $log){throw 'immutable gate log exists'}
 & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 $code=$LASTEXITCODE
 $results += [pscustomobject]@{command="tests/architecture/$gate.ps1";exit=$code;log=$log}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 4
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')){
 $log=Join-Path $PSScriptRoot ($gate+'.log');if(Test-Path $log){throw 'immutable gate log exists'}
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE
 $results += [pscustomobject]@{command="tests/architecture/$gate.py";exit=$code;log=$log}
 Write-Output "$gate exit=$code";Get-Content $log -Tail 4
}
$results | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $PSScriptRoot 'source-gates.json')
if(@($results | Where-Object exit -ne 0).Count){exit 1}
