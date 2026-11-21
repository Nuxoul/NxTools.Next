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
$coreFork = Join-Path $fixture 'gpui-pre'
$windowsFork = Join-Path $fixture 'gpui-pre-windows'
$manifest = Join-Path $fixture 'Cargo.toml'

function Replace-One([string]$Path, [string]$Before, [string]$After) {
    $text = [IO.File]::ReadAllText($Path).Replace("`r`n", "`n")
    if ([regex]::Matches($text, [regex]::Escape($Before)).Count -ne 1) {
        throw "The pinned source site no longer matches: $([IO.Path]::GetFileName($Path))"
    }
    [IO.File]::WriteAllText($Path, $text.Replace($Before, $After))
}
function Invoke-FixtureCargo([string[]]$Arguments) {
    Assert-CargoBuildLaneIdle
    $command = $Arguments[0]
    $remaining = $Arguments[1..($Arguments.Count - 1)]
    & cargo $command --manifest-path $manifest --target-dir $TargetDirectory @remaining
    if ($LASTEXITCODE) { throw "Under-border Cargo step failed ($LASTEXITCODE)." }
}

Push-Location $root
try {
    if ($Mode -eq 'Clippy') {
        Invoke-FixtureCargo @('clippy', '--locked', '--bin', 'quick_launch_surface', '--', '-D', 'warnings')
        return
    }
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $coreSources = @(Get-ChildItem (Join-Path $cargoHome 'registry\src') -Directory |
        ForEach-Object { Join-Path $_.FullName 'gpui-pre-0.3.9' } | Where-Object { Test-Path $_ })
    if ($coreSources.Count -ne 1) { throw 'Expected one installed gpui-pre0.3.9 source for this pinned fixture.' }
    New-Item -ItemType Directory -Force -Path $coreFork, $windowsFork | Out-Null
    Copy-Item -Path (Join-Path $coreSources[0] '*') -Destination $coreFork -Recurse -Force
    Copy-Item -Path (Join-Path $root 'vendor\gpui-pre-windows-039\*') -Destination $windowsFork -Recurse -Force
    Replace-One (Join-Path $coreFork 'src\scene.rs') '    Dashed = 1,' @'
    Dashed = 1,
    /// Background coverage compensated for a later solid border draw.
    UnderBorder = 2,
'@
    Replace-One (Join-Path $coreFork 'src\style.rs') '            border_color.a = 0.;' @'
            border_color.a = 0.;
            let compensate_border = self.is_border_visible()
                && matches!(self.border_style, BorderStyle::Solid);
'@
    Replace-One (Join-Path $coreFork 'src\style.rs') @'
                background_color.unwrap_or_default(),
                Edges::default(),
                border_color,
                self.border_style,
'@ @'
                background_color.unwrap_or_default(),
                if compensate_border { self.border_widths.to_pixels(rem_size) } else { Edges::default() },
                if compensate_border { self.border_color.unwrap_or_default() } else { border_color },
                if compensate_border { BorderStyle::UnderBorder } else { self.border_style },
'@
    $rendererPath = Join-Path $windowsFork 'src\directx_renderer.rs'
    $renderer = [IO.File]::ReadAllText($rendererPath)
    $oldFactor = 'desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ONE;'
    if ([regex]::Matches($renderer, [regex]::Escape($oldFactor)).Count -ne 2) { throw 'Expected the two pinned source-over sites.' }
    $renderer = $renderer.Replace($oldFactor, 'desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;')
    $renderer += "`n#[cfg(test)]`n#[path = `"source_over_tests.rs`"]`nmod source_over_tests;`n"
    [IO.File]::WriteAllText($rendererPath, $renderer)
    Copy-Item (Join-Path $root 'tests\graphics\gpui-source-over.rs') (Join-Path $windowsFork 'src\source_over_tests.rs') -Force
    $compensation = [IO.File]::ReadAllText((Join-Path $root 'tests\graphics\gpui-under-border.hlsl'))
    Replace-One (Join-Path $windowsFork 'src\shaders.hlsl') '    // Negative when inside the border' ($compensation + '    // Negative when inside the border')

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
gpui-kit = { version = "=0.8.0", features = ["test-support", "tree-sitter-bash", "tree-sitter-c", "tree-sitter-cpp", "tree-sitter-csharp", "tree-sitter-css", "tree-sitter-go", "tree-sitter-html", "tree-sitter-java", "tree-sitter-javascript", "tree-sitter-markdown", "tree-sitter-python", "tree-sitter-rust", "tree-sitter-sql", "tree-sitter-tsx", "tree-sitter-typescript"] }
image = { version = "=0.25.10", default-features = false, features = ["png"] }
serde_json = "1.0.140"
uuid = { version = "1.16.0", features = ["v4"] }
[patch.crates-io]
gpui-pre = { path = "$corePortable" }
gpui-pre-windows = { path = "$windowsPortable" }
gpui-component = { path = "$rootPortable/vendor/gpui-component-080" }
"@
    [IO.File]::WriteAllText($manifest, $cargoManifest)
    $lock = Join-Path $fixture 'Cargo.lock'
    if (-not (Test-Path $lock)) { Copy-Item (Join-Path $root 'Cargo.lock') $lock }
    $selectionPath = Join-Path $fixture 'backend-selection.txt'
    cmd /d /c "cargo tree --manifest-path `"$manifest`" --offline --target x86_64-pc-windows-msvc -p gpui-pre -p gpui-pre-windows --depth 0 --prefix none > `"$selectionPath`" 2>&1"
    if ($LASTEXITCODE) { throw 'Isolated Windows dependency resolution failed; production lock was not modified.' }
    $selection = Get-Content $selectionPath -Raw
    foreach ($expected in @(@('gpui-pre', $coreFork), @('gpui-pre-windows', $windowsFork))) {
        if (-not $selection.Contains("$($expected[0]) v0.3.9 ($($expected[1]))")) {
            throw 'The isolated manifest did not select the expected patched core/backend.'
        }
    }
    Invoke-FixtureCargo @('test', '--locked', '-p', 'gpui-pre-windows', '--lib', 'transparent_surface_blend_states_use_source_over_alpha', '--', '--test-threads=1')
    Invoke-FixtureCargo @('test', '--locked', '--bin', 'quick_launch_surface', '--', '--test-threads=1')
    if ($Mode -eq 'Test') { return }
    Invoke-FixtureCargo @('build', '--locked', '--bin', 'quick_launch_surface')
    $candidateRoot = Join-Path $TargetDirectory "debug\$CandidateDirectory"
    New-Item -ItemType Directory -Force -Path $candidateRoot | Out-Null
    Copy-Item (Join-Path $TargetDirectory 'debug\quick_launch_surface.exe') (Join-Path $candidateRoot 'quick_launch_surface.exe') -Force
}
finally { Pop-Location }
