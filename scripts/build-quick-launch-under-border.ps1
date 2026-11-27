param(
    [Parameter(Mandatory = $true)][string]$TargetDirectory,
    [ValidateSet('Build', 'Test', 'Clippy')][string]$Mode = 'Build',
    [ValidateSet('preview-under-border', 'preview-native-material')]
    [string]$CandidateDirectory = 'preview-under-border'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:RUSTFLAGS = '-C codegen-units=256'
Assert-CargoBuildLaneIdle
$scratchRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$fixture = Join-Path $scratchRoot 'surface-under-border'
$manifest = Join-Path $fixture 'Cargo.toml'
$coreFork = Join-Path $root 'vendor\gpui-pre-0312'
$windowsFork = Join-Path $root 'vendor\gpui-pre-windows-0312'

function Invoke-FixtureCargo([string[]]$Arguments) {
    Assert-CargoBuildLaneIdle
    $command = $Arguments[0]
    $remaining = $Arguments[1..($Arguments.Count - 1)]
    & cargo $command --manifest-path $manifest --target-dir $TargetDirectory @remaining
    if ($LASTEXITCODE) { throw "Under-border Cargo step failed ($LASTEXITCODE)." }
}

Push-Location $root
try {
    New-Item -ItemType Directory -Force -Path $fixture | Out-Null
    $rootPortable = $root.Replace('\', '/')
    $corePortable = $coreFork.Replace('\', '/')
    $windowsPortable = $windowsFork.Replace('\', '/')
    $cargoManifest = @"
[workspace]
[package]
name = "nxtools-under-border-preview"
version = "0.0.0"
edition = "2024"
autobins = false
[[bin]]
name = "quick_launch_surface"
path = "$rootPortable/rust/apps/nxtools-desktop/examples/quick_launch_surface.rs"
[dependencies]
gpui-kit = { version = "=0.10.0", features = ["test-support", "tree-sitter-bash", "tree-sitter-c", "tree-sitter-cpp", "tree-sitter-csharp", "tree-sitter-css", "tree-sitter-go", "tree-sitter-html", "tree-sitter-java", "tree-sitter-javascript", "tree-sitter-markdown", "tree-sitter-python", "tree-sitter-rust", "tree-sitter-sql", "tree-sitter-tsx", "tree-sitter-typescript"] }
image = { version = "=0.25.10", default-features = false, features = ["png"] }
serde_json = "1.0.140"
uuid = { version = "1.16.0", features = ["v4"] }
[patch.crates-io]
gpui-pre = { path = "$corePortable" }
gpui-pre-windows = { path = "$windowsPortable" }
gpui-component = { path = "$rootPortable/vendor/gpui-component-0100" }
gpui-base = { path = "$rootPortable/vendor/gpui-base-0100" }
"@
    [IO.File]::WriteAllText($manifest, $cargoManifest)
    $lock = Join-Path $fixture 'Cargo.lock'
    if (-not (Test-Path $lock)) { Copy-Item (Join-Path $root 'Cargo.lock') $lock }
    $selectionPath = Join-Path $fixture 'backend-selection.txt'
    cmd /d /c "cargo tree --manifest-path `"$manifest`" --offline --target x86_64-pc-windows-msvc -p gpui-pre -p gpui-pre-windows --depth 0 --prefix none > `"$selectionPath`" 2>&1"
    if ($LASTEXITCODE) { throw 'Isolated Windows dependency resolution failed.' }
    $selection = Get-Content $selectionPath -Raw
    foreach ($expected in @(@('gpui-pre', $coreFork), @('gpui-pre-windows', $windowsFork))) {
        if (-not $selection.Contains("$($expected[0]) v0.3.12 ($($expected[1]))")) {
            throw 'The isolated manifest did not select the pinned production core/backend.'
        }
    }
    if ($Mode -eq 'Clippy') {
        Invoke-FixtureCargo @('clippy', '--locked', '--bin', 'quick_launch_surface', '--', '-D', 'warnings')
        return
    }
    Invoke-FixtureCargo @('test', '--locked', '-p', 'gpui-pre-windows', '--lib', 'source_over_tests', '--', '--test-threads=1')
    Invoke-FixtureCargo @('test', '--locked', '--bin', 'quick_launch_surface', '--', '--test-threads=1')
    if ($Mode -eq 'Test') { return }
    Invoke-FixtureCargo @('build', '--locked', '--bin', 'quick_launch_surface')
    $candidateRoot = Join-Path $TargetDirectory "debug\$CandidateDirectory"
    New-Item -ItemType Directory -Force -Path $candidateRoot | Out-Null
    Copy-Item (Join-Path $TargetDirectory 'debug\quick_launch_surface.exe') (Join-Path $candidateRoot 'quick_launch_surface.exe') -Force
}
finally { Pop-Location }
