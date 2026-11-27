[CmdletBinding()]
param(
    [ValidateSet('Mica', 'MicaAlt', 'Acrylic', 'Accent4', 'Solid')][string]$Mode = 'Mica',
    [switch]$Light,
    [ValidateSet('', 'Clipboard', 'Search', 'Clipboard,Search')][string]$Open = '',
    [switch]$NoBuild,
    [switch]$BuildOnly
)

# Native preview of the reworked frosted theme (task 11-23-acrylic-theme-rework).
# F2 light/dark, F3 next material, F4 clipboard popup, F5 search popup, F9 GPU frame capture,
# Escape closes the focused window. Diagnostics go to the session's stdout.log.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')

Push-Location $root
try {
    $targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
    if (-not [IO.Path]::IsPathRooted($targetRoot)) { $targetRoot = Join-Path $root $targetRoot }
    $previewExecutable = Join-Path $targetRoot 'debug\examples\acrylic_rework_preview.exe'
    $existingPreview = @(Get-CimInstance Win32_Process -Filter "Name = 'acrylic_rework_preview.exe'" |
        Where-Object { $_.ExecutablePath -eq $previewExecutable })
    if (-not $NoBuild) {
        if ($existingPreview.Count -ne 0) {
            throw 'The preview is open. Close it with Escape before rebuilding.'
        }
        Initialize-LowMemoryRustBuildEnvironment
        Assert-CargoBuildLaneIdle
        cargo build --locked -p nxtools-desktop --example acrylic_rework_preview
        if ($LASTEXITCODE -ne 0) { throw "Acrylic preview build failed ($LASTEXITCODE)." }
        if ($BuildOnly) { return }
    }
    if (-not (Test-Path -LiteralPath $previewExecutable -PathType Leaf)) {
        throw 'Build the acrylic preview before using -NoBuild.'
    }

    $modeArgument = @{ Mica = 'mica'; MicaAlt = 'mica_alt'; Acrylic = 'acrylic'; Accent4 = 'accent4'; Solid = 'solid' }[$Mode]
    $correlation = [Guid]::NewGuid()
    $scratchRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
    $session = Join-Path $scratchRoot "acrylic-rework-$correlation"
    New-Item -ItemType Directory -Path $session | Out-Null
    # Every argument is a ValidateSet token, a switch or a GUID-derived path.
    $previewArguments = @("--mode=$modeArgument", "--correlation=$correlation", "--capture-dir=$session")
    if ($Light) { $previewArguments += '--light' }
    if ($Open) { $previewArguments += ('--open=' + $Open.ToLowerInvariant()) }
    $process = Start-Process -FilePath $previewExecutable -ArgumentList $previewArguments -PassThru `
        -RedirectStandardOutput (Join-Path $session 'stdout.log') -RedirectStandardError (Join-Path $session 'stderr.log')
    Write-Host "ui.acrylic_preview.launched correlation=$correlation pid=$($process.Id) mode=$modeArgument light=$([bool]$Light)"
    Write-Host "Logs and F9 captures: $session"
    Write-Host 'F2 light/dark | F3 next material | F4 clipboard | F5 search | F9 GPU capture | Esc close'
}
finally { Pop-Location }
