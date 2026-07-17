$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$hostPath = Join-Path $root 'csharp\NxTools.PlatformHost\bin\Debug\net8.0-windows\win-x64\NxTools.PlatformHost.exe'

if (-not (Test-Path -LiteralPath $hostPath)) {
    & (Join-Path $PSScriptRoot 'build.ps1')
}

$platformHost = Start-Process -FilePath $hostPath -WindowStyle Hidden -PassThru
try {
    Push-Location $root
    cargo run -p nxtools-desktop
    if ($LASTEXITCODE -ne 0) {
        throw "NxTools desktop exited with code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
    if (-not $platformHost.HasExited) {
        Stop-Process -Id $platformHost.Id
    }
}
