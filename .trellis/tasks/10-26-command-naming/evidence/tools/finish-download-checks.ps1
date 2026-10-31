$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$deadline=(Get-Date).AddSeconds(600)
do{$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe'});if(-not $busy.Count){break};Start-Sleep -Seconds 15}while((Get-Date)-lt $deadline)
if($busy.Count){throw 'Natural idle unavailable for final format'}
$preview=Join-Path $PSScriptRoot 'polish.format-preview.rs'
if(Test-Path $preview){throw 'Immutable preview exists'}
Copy-Item -LiteralPath 'rust\apps\nxtools-desktop\src\downloads\polish_tests.rs' -Destination $preview
& rustfmt --edition 2024 --config skip_children=true $preview
if($LASTEXITCODE -ne 0){throw 'Format preview failed'}
& python -c "from pathlib import Path; p=Path('.trellis/tasks/10-26-command-naming/evidence/baseline/rust/apps/nxtools-desktop/src/downloads/polish_tests.rs'); s=p.read_text(encoding='utf-8').replace('选择默认保存目录','选择默认保存目录…').replace('选择 FFmpeg 可执行文件','选择 FFmpeg 可执行文件…'); f=Path(r'$preview').read_text(encoding='utf-8'); assert f.startswith(s), 'Original prefix format drift'; print('PASS original Download test prefix plus exact two punctuation updates')"
if($LASTEXITCODE -ne 0){throw 'Old prefix guard failed'}
& rustfmt --edition 2024 --config skip_children=true rust\apps\nxtools-desktop\src\downloads\polish_tests.rs
if($LASTEXITCODE -ne 0){throw 'Scoped formatting failed'}
$checks=@(
 @{Name='audit08-final-twelve';Args=@('test','-p','nxtools-desktop','audit08_','--','--test-threads=1')},
 @{Name='downloads-final';Args=@('test','-p','nxtools-desktop','downloads::','--','--test-threads=1')},
 @{Name='names-final';Args=@('test','-p','nxtools-desktop','accessible_name_','--','--test-threads=1')}
)
$results=@();$runner=Join-Path $PSScriptRoot 'run.ps1'
foreach($check in $checks){
 $argumentLiteral=($check.Args | ForEach-Object {"'$_'"}) -join ','
 & powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$runner' -Name '$($check.Name)' -CargoArgs @($argumentLiteral)"
 $results+=@{name=$check.Name;exit=$LASTEXITCODE}
}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'final-download-results.json') -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
