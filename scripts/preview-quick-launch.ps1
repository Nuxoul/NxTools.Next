[CmdletBinding(DefaultParameterSetName = 'Preview')]
param(
    [Parameter(ParameterSetName = 'Preview')]
    [switch]$NoBuild,
    [Parameter(ParameterSetName = 'Preview')]
    [switch]$Light,
    [Parameter(ParameterSetName = 'Build')]
    [switch]$BuildOnly,
    [Parameter(ParameterSetName = 'Test')]
    [switch]$TestOnly
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')

Push-Location $root
try {
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
    if (-not [IO.Path]::IsPathRooted($targetRoot)) { $targetRoot = Join-Path $root $targetRoot }
    $previewExecutable = Join-Path $targetRoot 'debug\examples\quick_launch_surface.exe'
    $existingPreview = @(Get-CimInstance Win32_Process -Filter "Name = 'quick_launch_surface.exe'" |
        Where-Object { $_.ExecutablePath -eq $previewExecutable })
    if ($existingPreview.Count -ne 0) {
        throw 'The preview is already open. Close it with Escape before reopening.'
    }
    if (-not $NoBuild) {
        $env:CARGO_BUILD_JOBS = '1'
        $env:CARGO_INCREMENTAL = '0'
        $env:RUSTFLAGS = '-C codegen-units=256'
        Assert-CargoBuildLaneIdle
        if ($TestOnly) {
            cargo test --locked -p nxtools-desktop --example quick_launch_surface -- --test-threads=1
        }
        else {
            cargo build --locked -p nxtools-desktop --example quick_launch_surface
        }
        if ($LASTEXITCODE -ne 0) { throw "GPUI preview build/test failed ($LASTEXITCODE)." }
        if ($BuildOnly -or $TestOnly) { return }
    }

    if (-not (Test-Path -LiteralPath $previewExecutable -PathType Leaf)) {
        throw 'Preview executable is missing. Run this script without -NoBuild first.'
    }
    Write-Host 'Independent GPUI preview: type to inspect; F2 switches theme; Escape closes.'
    Write-Host 'No NxTools runtime, service, clipboard capture, configuration or installer is started.'
    if (-not ('QuickLaunchPreviewWindow' -as [type])) {
        Add-Type -Path (Join-Path $PSScriptRoot 'preview-quick-launch-window.cs')
    }
    $correlation = [Guid]::NewGuid().ToString()
    $launchOptions = @{ FilePath = $previewExecutable; WorkingDirectory = $root; PassThru = $true;
        ArgumentList = @("--correlation=$correlation") }
    if ($Light) { $launchOptions.ArgumentList += '--light' }
    $nativeStarted = [Diagnostics.Stopwatch]::StartNew()
    Write-Host "ui.surface_preview.native_frame.started correlation=$correlation"
    $previewProcess = Start-Process @launchOptions
    [QuickLaunchPreviewWindow]::Prepare($previewProcess.Id)
    Write-Host "ui.surface_preview.native_frame.finished correlation=$correlation enabled=false elapsed_ms=$($nativeStarted.ElapsedMilliseconds)"
    Write-Host "Preview process: $($previewProcess.Id)"
}
finally {
    Pop-Location
}
