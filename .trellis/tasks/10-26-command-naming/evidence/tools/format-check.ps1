$ErrorActionPreference='Stop'
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
$formatRoot=Join-Path $env:PI_SCRATCH_DIR 'command-naming\format-preview'
New-Item -ItemType Directory -Force -Path $formatRoot | Out-Null
$rustPaths=@('rust\apps\nxtools-desktop\src\main_window\command_naming_tests.rs','rust\apps\nxtools-desktop\src\main_window\clipboard_page\kit_migration_tests.rs')
$deadline=(Get-Date).AddSeconds(600)
do{$busy=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -in @('cargo.exe','rustc.exe') -or $_.Name -like 'nxtools_desktop-*.exe'});if(-not $busy.Count){break};Start-Sleep -Seconds 15}while((Get-Date)-lt $deadline)
if($busy.Count){throw 'Natural idle unavailable before scoped rustfmt'}
foreach($rustPath in $rustPaths){
 $copy=Join-Path $formatRoot ((Split-Path $rustPath -Parent | Split-Path -Leaf)+'-'+(Split-Path $rustPath -Leaf))
 if(Test-Path $copy){throw 'Immutable format preview exists'}
 Copy-Item -LiteralPath $rustPath -Destination $copy
 & rustfmt --edition 2024 --config skip_children=true $copy
 if($LASTEXITCODE -ne 0){throw 'Scratch formatting failed'}
}
