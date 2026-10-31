$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$busy=@(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe' });if($busy.Count){throw 'Rust not naturally idle'}
$files=@('rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/floating_assistant.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/download_page/projection_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_average_tests.rs','rust/apps/nxtools-desktop/src/main_window/tests.rs')
& rustfmt --edition 2024 --config skip_children=true @files
if($LASTEXITCODE){exit $LASTEXITCODE}
& rustfmt --edition 2024 --config skip_children=true --check @files *> (Join-Path $PSScriptRoot 'scoped-fmt.log');$fmtCode=$LASTEXITCODE
& git diff --check -- @files docs/reference-code/download.md *> (Join-Path $PSScriptRoot 'scoped-diff.log');$diffCode=$LASTEXITCODE
@{fmt=$fmtCode;diff=$diffCode;files=$files;environment=@{CARGO_BUILD_JOBS=$env:CARGO_BUILD_JOBS;CARGO_INCREMENTAL=$env:CARGO_INCREMENTAL;RUSTFLAGS=$env:RUSTFLAGS}} | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot 'scoped-checks.json') -Encoding UTF8
if($fmtCode -or $diffCode){throw 'Scoped formatting or whitespace failed'}
& powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$PSScriptRoot\run.ps1' -Name 'final-alias-build' -CargoArgs @('test','-p','nxtools-desktop','--bin','nxtools-desktop','--no-run')"
if($LASTEXITCODE){exit $LASTEXITCODE}
$exe=(Resolve-Path 'target\debug\deps\nxtools_desktop-bc43045ce72e2830.exe').Path
& powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$PSScriptRoot\run-exe.ps1' -Name 'final-alias-and-identity' -Executable '$exe' -TestArgs @('audit11_','accessible_name_','history_identity_','--test-threads=1') -Seconds 180"
exit $LASTEXITCODE
