$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$hostPath = Join-Path $root 'csharp\NxTools.PlatformHost\bin\Debug\net8.0-windows\win-x64\NxTools.PlatformHost.exe'

# Build the desktop before starting PlatformHost. GPUI's first compile can
# exceed PlatformHost's initial pipe-connect window; starting the adapter
# first would leave the desktop stuck in "waiting for Windows adapter".
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:RUSTFLAGS = '-C codegen-units=256'

Push-Location $root
try {
    if (-not (Test-Path -LiteralPath $hostPath)) {
        & (Join-Path $PSScriptRoot 'build.ps1')
    }
    else {
        & cargo build -p nxtools-desktop --jobs 1
    }
    if ($LASTEXITCODE -ne 0) {
        throw "NxTools desktop build failed with exit code $LASTEXITCODE"
    }

    $platformProcess = Start-Process -FilePath $hostPath -WindowStyle Hidden -PassThru
    try {
        & cargo run -p nxtools-desktop
        if ($LASTEXITCODE -ne 0) {
            throw "NxTools desktop exited with code $LASTEXITCODE"
        }
    }
    finally {
        if (-not $platformProcess.HasExited) {
            Stop-Process -Id $platformProcess.Id
        }
    }
}
finally {
    Pop-Location
}
