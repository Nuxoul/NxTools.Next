$ErrorActionPreference='Stop'
$runner=Join-Path $PSScriptRoot 'run.ps1'
$checks=@(
 @{Name='names-protection';Args=@('test','-p','nxtools-desktop','accessible_name_','--','--test-threads=1')},
 @{Name='history-identity-protection';Args=@('test','-p','nxtools-desktop','history_identity_','--','--test-threads=1')},
 @{Name='sort-protection';Args=@('test','-p','nxtools-desktop','sort_keyboard_','--','--test-threads=1')},
 @{Name='quick-presentation-related';Args=@('test','-p','nxtools-desktop','quick_launch::polish_tests::','--','--test-threads=1')}
)
$results=@()
foreach($check in $checks){
 $argumentLiteral=($check.Args | ForEach-Object {"'$_'"}) -join ','
 $invoke="& '$runner' -Name '$($check.Name)' -CargoArgs @($argumentLiteral)"
 & powershell -NoProfile -ExecutionPolicy Bypass -Command $invoke
 $results+=@{name=$check.Name;exit=$LASTEXITCODE}
}
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe'})
if($busy.Count){throw 'Natural idle unavailable for scoped fmt'}
$files=@('rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/clipboard_page.rs','rust/apps/nxtools-desktop/src/main_window/clipboard_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/tools_page.rs','rust/apps/nxtools-desktop/src/main_window/tools_page/currency_settings.rs','rust/apps/nxtools-desktop/src/main_window/tools_page/floating_assistant_settings.rs','rust/apps/nxtools-desktop/src/main_window/floating_assistant_tests.rs','rust/apps/nxtools-desktop/src/ui/components/menu.rs','rust/apps/nxtools-desktop/src/main_window/command_naming_tests.rs')
$log=Join-Path $PSScriptRoot 'scoped-fmt.log'
& rustfmt --edition 2024 --config skip_children=true --check @files *> $log
$code=$LASTEXITCODE;$results+=@{name='scoped-fmt';exit=$code}
Write-Output "scoped fmt exit=$code";Get-Content $log -Tail 10
& git diff --check -- @files *> (Join-Path $PSScriptRoot 'scoped-diff.log')
$results+=@{name='scoped-diff';exit=$LASTEXITCODE}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'last-check-results.json') -Encoding UTF8
if(@($results | Where-Object exit -ne 0).Count){exit 1}
