$ErrorActionPreference='Stop'
$results=@()
foreach($gate in @('download-ui-lifecycle','desktop-shell-lifecycle','clipboard-ui-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')){
 $log=Join-Path $PSScriptRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 $code=$LASTEXITCODE;$results+=@{command="tests/architecture/$gate.ps1";exit=$code;level='source only'};Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')){
 $log=Join-Path $PSScriptRoot ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE;$results+=@{command="tests/architecture/$gate.py";exit=$code;level='source/document only'};Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
$results | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot 'source-gate-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
