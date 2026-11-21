[CmdletBinding(DefaultParameterSetName = 'Preview')]
param(
    [Parameter(ParameterSetName = 'Preview')]
    [Parameter(ParameterSetName = 'Capture')]
    [switch]$NoBuild,
    [Parameter(ParameterSetName = 'Preview')]
    [Parameter(ParameterSetName = 'Capture')]
    [switch]$Light,
    [Parameter(ParameterSetName = 'Test')]
    [switch]$TestOnly,
    [Parameter(ParameterSetName = 'Build')]
    [switch]$BuildOnly,
    [Parameter(ParameterSetName = 'Capture', Mandatory = $true)]
    [string]$CaptureDirectory,
    [switch]$PixelCoverage,
    [switch]$UnderBorder,
    [Parameter(ParameterSetName = 'Capture')]
    [switch]$NoShadow
)

$ErrorActionPreference = 'Stop'
if ($UnderBorder -and $PixelCoverage) { throw 'Choose one edge-composition experiment.' }
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
$scratchRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$variant = if ($UnderBorder) { 'under-border' } elseif ($PixelCoverage) { 'pixel-coverage' } else { 'source-over' }
$experimentRoot = Join-Path $scratchRoot "surface-$variant"
$fork = Join-Path $experimentRoot 'gpui-pre-windows'

function Invoke-PreviewCargo([string[]]$Arguments) {
    $forkPortable = $fork.Replace('\', '/')
    Assert-CargoBuildLaneIdle
    & cargo --config "patch.crates-io.gpui-pre-windows.path='$forkPortable'" @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Source-over preview Cargo step failed ($LASTEXITCODE)." }
}

Push-Location $root
try {
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
    if (-not [IO.Path]::IsPathRooted($targetRoot)) { $targetRoot = Join-Path $root $targetRoot }
    $ordinaryExecutable = Join-Path $targetRoot 'debug\examples\quick_launch_surface.exe'
    $candidateRoot = Join-Path $targetRoot "debug\preview-$variant"
    $candidateExecutable = Join-Path $candidateRoot 'quick_launch_surface.exe'
    $running = @(Get-CimInstance Win32_Process -Filter "Name = 'quick_launch_surface.exe'" |
        Where-Object { $_.ExecutablePath -eq $candidateExecutable -or $_.ExecutablePath -eq $ordinaryExecutable })
    if ($running.Count) { throw 'Close the standalone preview before building or reopening the comparison.' }
    if ($UnderBorder -and -not $NoBuild) {
        $buildMode = if ($TestOnly) { 'Test' } else { 'Build' }
        & (Join-Path $PSScriptRoot 'build-quick-launch-under-border.ps1') -TargetDirectory $targetRoot -Mode $buildMode
        if ($TestOnly -or $BuildOnly) { return }
    }

    if (-not $NoBuild -and -not $UnderBorder) {
        $env:CARGO_BUILD_JOBS = '1'
        $env:CARGO_INCREMENTAL = '0'
        $env:RUSTFLAGS = '-C codegen-units=256'
        Assert-CargoBuildLaneIdle
        New-Item -ItemType Directory -Force -Path $fork | Out-Null
        Copy-Item -Path (Join-Path $root 'vendor\gpui-pre-windows-039\*') -Destination $fork -Recurse -Force
        Copy-Item -LiteralPath (Join-Path $root 'tests\graphics\gpui-source-over.rs') `
            -Destination (Join-Path $fork 'src\source_over_tests.rs') -Force
        $rendererPath = Join-Path $fork 'src\directx_renderer.rs'
        $renderer = [IO.File]::ReadAllText($rendererPath)
        $originalFactor = 'desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_ONE;'
        if ([regex]::Matches($renderer, [regex]::Escape($originalFactor)).Count -ne 2) {
            throw 'The two upstream alpha blend sites no longer match this isolated experiment.'
        }
        # Zed e18de65b / PR65571: ordinary primitives and path sprites only.
        $renderer = $renderer.Replace($originalFactor, `
            'desc.RenderTarget[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;')
        $renderer += "`r`n#[cfg(test)]`r`n#[path = `"source_over_tests.rs`"]`r`nmod source_over_tests;`r`n"
        [IO.File]::WriteAllText($rendererPath, $renderer)
        if ($PixelCoverage) {
            $shaderPath = Join-Path $fork 'src\shaders.hlsl'
            $shader = [IO.File]::ReadAllText($shaderPath)
            $coverage = [IO.File]::ReadAllText((Join-Path $root 'tests\graphics\gpui-pixel-coverage.hlsl'))
            $sites = @(
                @('float4 quad_fragment(', "$coverage`r`nfloat4 quad_fragment("),
                @('float outer_sdf = quad_sdf_impl(corner_center_to_point, corner_radius);',
                    "float outer_sdf = quad_sdf_impl(corner_center_to_point, corner_radius);`r`n    float coverage_width = quad_pixel_width(corner_center_to_point, corner_radius);"),
                @('if (border_sdf < antialias_threshold) {', 'if (border_sdf < antialias_threshold * coverage_width) {'),
                @('saturate(antialias_threshold - inner_sdf)', 'quad_pixel_coverage(inner_sdf, coverage_width)'),
                @('saturate(antialias_threshold - outer_sdf)', 'quad_pixel_coverage(outer_sdf, coverage_width)')
            )
            foreach ($site in $sites) {
                if ([regex]::Matches($shader, [regex]::Escape($site[0])).Count -ne 1) {
                    throw 'The released quad coverage site no longer matches the isolated experiment.'
                }
                $shader = $shader.Replace($site[0], $site[1])
            }
            [IO.File]::WriteAllText($shaderPath, $shader)
        }
        Invoke-PreviewCargo -Arguments @('test', '--locked', '-p', 'gpui-pre-windows', '--lib',
            'transparent_surface_blend_states_use_source_over_alpha',
            '--', '--test-threads=1')
        Invoke-PreviewCargo -Arguments @('test', '--locked', '-p', 'nxtools-desktop',
            '--example', 'quick_launch_surface', '--', '--test-threads=1')
        if ($TestOnly) { return }

        $backup = Join-Path $experimentRoot 'ordinary-preview.exe'
        $hadOrdinaryExecutable = Test-Path -LiteralPath $ordinaryExecutable -PathType Leaf
        if ($hadOrdinaryExecutable) { Copy-Item -LiteralPath $ordinaryExecutable -Destination $backup -Force }
        try {
            Invoke-PreviewCargo -Arguments @('build', '--locked', '-p', 'nxtools-desktop',
                '--example', 'quick_launch_surface')
            New-Item -ItemType Directory -Force -Path $candidateRoot | Out-Null
            Copy-Item -LiteralPath $ordinaryExecutable -Destination $candidateExecutable -Force
        }
        finally {
            if ($hadOrdinaryExecutable) { Copy-Item -LiteralPath $backup -Destination $ordinaryExecutable -Force }
            elseif (Test-Path -LiteralPath $ordinaryExecutable) { Remove-Item -LiteralPath $ordinaryExecutable }
        }
        if ($BuildOnly) { return }
    }
    if (-not (Test-Path -LiteralPath $candidateExecutable -PathType Leaf)) {
        throw 'Build the selected comparison once before using -NoBuild.'
    }
    if ($CaptureDirectory) {
        if (-not [IO.Path]::IsPathRooted($CaptureDirectory)) { $CaptureDirectory = Join-Path $root $CaptureDirectory }
        $captureCorrelation = [Guid]::NewGuid().ToString()
        $captureArguments = @("--gpu-capture=`"$CaptureDirectory`"", "--correlation=$captureCorrelation")
        if ($Light) { $captureArguments += '--light' }
        if ($NoShadow) { $captureArguments += '--gpu-no-shadow' }
        New-Item -ItemType Directory -Force -Path $experimentRoot | Out-Null
        $captureStarted = [Diagnostics.Stopwatch]::StartNew()
        Write-Host "ui.surface_preview.capture.started variant=$variant correlation=$captureCorrelation"
        $captureProcess = Start-Process -FilePath $candidateExecutable -WorkingDirectory $root `
            -ArgumentList $captureArguments -PassThru `
            -RedirectStandardOutput (Join-Path $experimentRoot 'capture.out.log') `
            -RedirectStandardError (Join-Path $experimentRoot 'capture.err.log')
        $null = $captureProcess.Handle
        if (-not $captureProcess.WaitForExit(15000)) {
            Write-Host "ui.surface_preview.capture.failed correlation=$captureCorrelation stage=deadline elapsed_ms=$($captureStarted.ElapsedMilliseconds)"
            [void]$captureProcess.CloseMainWindow()
            throw 'GPU capture did not finish; rebuild the preview before using capture with -NoBuild.'
        }
        if ($captureProcess.ExitCode -ne 0 -or -not (Test-Path (Join-Path $CaptureDirectory 'frame.json'))) {
            Write-Host "ui.surface_preview.capture.failed correlation=$captureCorrelation exit_code=$($captureProcess.ExitCode) elapsed_ms=$($captureStarted.ElapsedMilliseconds)"
            throw 'GPU capture failed; no successful readback was produced.'
        }
        Write-Host "ui.surface_preview.capture.finished variant=$variant correlation=$captureCorrelation light=$([bool]$Light) elapsed_ms=$($captureStarted.ElapsedMilliseconds)"
        return
    }
    if (-not ('QuickLaunchPreviewWindow' -as [type])) {
        Add-Type -Path (Join-Path $PSScriptRoot 'preview-quick-launch-window.cs')
    }
    $correlation = [Guid]::NewGuid().ToString()
    $launchOptions = @{ FilePath = $candidateExecutable; WorkingDirectory = $root; PassThru = $true;
        ArgumentList = @("--correlation=$correlation") }
    if ($Light) { $launchOptions.ArgumentList += '--light' }
    $nativeStarted = [Diagnostics.Stopwatch]::StartNew()
    Write-Host "ui.surface_preview.native_frame.started variant=$variant correlation=$correlation"
    $previewProcess = Start-Process @launchOptions
    [QuickLaunchPreviewWindow]::Prepare($previewProcess.Id)
    Write-Host "ui.surface_preview.native_frame.finished variant=$variant correlation=$correlation enabled=false elapsed_ms=$($nativeStarted.ElapsedMilliseconds)"
    Write-Host "Preview comparison process: $($previewProcess.Id); variant=$variant. F2 switches theme; Escape closes."
}
finally {
    Pop-Location
}
