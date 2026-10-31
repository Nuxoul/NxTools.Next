$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256';$env:_LINK_='/DEBUG:NONE'
$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe','link.exe') -or $_.Name -like 'nxtools_desktop-*.exe'});if($busy.Count){throw 'Rust not naturally idle'}
$files=@('rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/detail_request.rs','rust/apps/nxtools-desktop/src/downloads/detail_request_tests.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/shell.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/download_page/detail_async_tests.rs','rust/crates/nxtools-download/src/engine/projection.rs','rust/crates/nxtools-download/src/storage.rs')
& rustfmt --edition 2024 --config skip_children=true @files;if($LASTEXITCODE){exit $LASTEXITCODE}
& rustfmt --edition 2024 --config skip_children=true --check @files *> (Join-Path $PSScriptRoot 'scoped-fmt-final.log');$fmt=$LASTEXITCODE
& git diff --check -- @files rust/apps/nxtools-desktop/Cargo.toml rust/crates/nxtools-download/Cargo.toml docs/reference-code/download.md *> (Join-Path $PSScriptRoot 'scoped-diff-final.log');$diff=$LASTEXITCODE
@{fmt=$fmt;diff=$diff;files=$files}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'scoped-checks-final.json') -Encoding UTF8
if($fmt -or $diff){throw 'Scoped format or whitespace failed'}
$checks=@(
 @{Name='strict-desktop-production-final';Args=@('clippy','-p','nxtools-desktop','--','-D','warnings')},
 @{Name='strict-desktop-tests-final';Args=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')},
 @{Name='workspace-check-final';Args=@('check','--workspace','--tests')},
 @{Name='workspace-fmt-final';Args=@('fmt','--all','--','--check')}
)
$results=@()
foreach($check in $checks){$literal=($check.Args|ForEach-Object {"'$_'"}) -join ',';& powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$PSScriptRoot\run.ps1' -Name '$($check.Name)' -CargoArgs @($literal)";$results+=@{name=$check.Name;exit=$LASTEXITCODE}}
$results|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'final-quality-results.json') -Encoding UTF8
