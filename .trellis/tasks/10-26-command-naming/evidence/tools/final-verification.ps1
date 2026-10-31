$ErrorActionPreference='Stop'
$runner=Join-Path $PSScriptRoot 'run.ps1'
$checks=@(
 @{Name='strict-tests-final';Args=@('clippy','-p','nxtools-desktop','--tests','--','-D','warnings')},
 @{Name='strict-production-final';Args=@('clippy','-p','nxtools-desktop','--','-D','warnings')},
 @{Name='workspace-check-final';Args=@('check','--workspace','--tests')},
 @{Name='workspace-fmt-final';Args=@('fmt','--all','--','--check')}
)
$results=@()
foreach($check in $checks){
 $argumentLiteral=($check.Args | ForEach-Object {"'$_'"}) -join ','
 & powershell -NoProfile -ExecutionPolicy Bypass -Command "& '$runner' -Name '$($check.Name)' -CargoArgs @($argumentLiteral)"
 $results+=@{name=$check.Name;exit=$LASTEXITCODE}
}
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe'})
if($busy.Count){throw 'Natural idle unavailable before final scoped fmt'}
$files=@('rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/new_download_view.rs','rust/apps/nxtools-desktop/src/downloads/settings_view.rs','rust/apps/nxtools-desktop/src/downloads/polish_tests.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/clipboard_page.rs','rust/apps/nxtools-desktop/src/main_window/clipboard_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/tools_page.rs','rust/apps/nxtools-desktop/src/main_window/tools_page/currency_settings.rs','rust/apps/nxtools-desktop/src/main_window/tools_page/floating_assistant_settings.rs','rust/apps/nxtools-desktop/src/main_window/floating_assistant_tests.rs','rust/apps/nxtools-desktop/src/ui/components/menu.rs','rust/apps/nxtools-desktop/src/main_window/command_naming_tests.rs')
& rustfmt --edition 2024 --config skip_children=true --check @files *> (Join-Path $PSScriptRoot 'scoped-fmt-final.log')
$results+=@{name='scoped-fmt-final';exit=$LASTEXITCODE}
& git diff --check -- @files *> (Join-Path $PSScriptRoot 'scoped-diff-final.log')
$results+=@{name='scoped-diff-final';exit=$LASTEXITCODE}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'final-verification-results.json') -Encoding UTF8
$results | Format-Table -AutoSize
if(@($results | Where-Object exit -ne 0).Count){exit 1}
