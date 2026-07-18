param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [string]$OutputPath = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
if (-not $OutputPath) {
    $OutputPath = if ($Configuration -eq 'Release') {
        'artifacts\win-x64'
    }
    else {
        'artifacts\win-x64-debug'
    }
}
$destination = Join-Path $root $OutputPath

Push-Location $root
try {
    & (Join-Path $PSScriptRoot 'build.ps1') -Configuration $Configuration
    if ($Configuration -eq 'Release') {
        cargo build --release -p nxtools-desktop -p nxtools-indexer
    }
    else {
        cargo build -p nxtools-desktop -p nxtools-indexer
    }
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
    dotnet publish '.\csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj' `
        -c $Configuration `
        -r win-x64 `
        --self-contained false `
        -p:PublishSingleFile=true
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet publish failed with exit code $LASTEXITCODE"
    }

    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    Remove-Item (Join-Path $destination 'THIRD_PARTY_NOTICES.md') -Force -ErrorAction SilentlyContinue
    Copy-Item ".\target\$profile\nxtools-desktop.exe" (Join-Path $destination 'NxTools.exe')
    Copy-Item ".\target\$profile\nxtools-indexer.exe" (Join-Path $destination 'NxTools.Indexer.exe')
    Copy-Item ".\csharp\NxTools.PlatformHost\bin\$Configuration\net8.0-windows\win-x64\publish\NxTools.PlatformHost.exe" $destination
    Copy-Item '.\LICENSE-MIT', '.\LICENSE-APACHE' $destination
}
finally {
    Pop-Location
}

Write-Host "Package created at $destination"
