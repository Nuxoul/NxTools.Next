$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$folder=$PSScriptRoot;$results=@()
$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -match '^(cargo|rustc|rustfmt|clippy-driver|link|nxtools_desktop-.*)\.exe$'});if($busy.Count){throw 'Rust not idle'}
$files=@('rust/apps/nxtools-desktop/src/quick_launch.rs','rust/apps/nxtools-desktop/src/quick_launch/panel.rs','rust/apps/nxtools-desktop/src/quick_launch/actions.rs','rust/apps/nxtools-desktop/src/quick_launch/tests.rs','rust/apps/nxtools-desktop/src/quick_launch/extension_providers.rs','rust/apps/nxtools-desktop/src/quick_launch/retained_text.rs','rust/apps/nxtools-desktop/src/quick_launch/retained_text_tests.rs')
$log=Join-Path $folder 'scoped-fmt.log';if(Test-Path $log){throw 'Immutable log exists'}
& rustfmt --edition 2024 --config skip_children=true --check @files *> $log
$results+=@{command='rustfmt --edition 2024 --config skip_children=true --check [7 scoped files]';exit=$LASTEXITCODE;files=$files};Get-Content $log -Tail 12
$log=Join-Path $folder 'workspace-fmt.log';if(Test-Path $log){throw 'Immutable log exists'}
& cmd /d /c "cargo fmt --all -- --check > `"$log`" 2>&1"
$results+=@{command='cargo fmt --all -- --check';exit=$LASTEXITCODE};Get-Content $log -Tail 12
$log=Join-Path $folder 'diff-check.log';if(Test-Path $log){throw 'Immutable log exists'}
& git diff --check *> $log
$results+=@{command='git diff --check';exit=$LASTEXITCODE};Get-Content $log -Tail 12
foreach($gate in @('quick-launch-lifecycle','desktop-shell-lifecycle','ui-token-boundaries','ui-consistency','ui-state-lifecycle','ui-motion-policy')){
 $log=Join-Path $folder ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & powershell -NoProfile -ExecutionPolicy Bypass -File "tests\architecture\$gate.ps1" *> $log
 $code=$LASTEXITCODE;$results+=@{command="tests/architecture/$gate.ps1";exit=$code};Write-Output "$gate exit=$code";Get-Content $log -Tail 5
}
foreach($gate in @('gpui-component-catalog','gpui-release-contracts','markdown-editor-contracts','spec-contracts')){
 $log=Join-Path $folder ($gate+'.log');if(Test-Path $log){throw 'Immutable log exists'}
 & python "tests/architecture/$gate.py" *> $log
 $code=$LASTEXITCODE;$results+=@{command="tests/architecture/$gate.py";exit=$code};Write-Output "$gate exit=$code";Get-Content $log -Tail 2
}
$results|ConvertTo-Json -Depth 5|Set-Content (Join-Path $folder 'finish-check-results.json') -Encoding UTF8
if(@($results|Where-Object exit -ne 0).Count){exit 1}
