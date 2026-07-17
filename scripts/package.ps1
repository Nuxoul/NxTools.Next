param(
    [string]$OutputPath = 'artifacts\win-x64'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$destination = Join-Path $root $OutputPath

Push-Location $root
try {
    & (Join-Path $PSScriptRoot 'build.ps1') -Configuration Release
    dotnet publish '.\csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj' `
        -c Release `
        -r win-x64 `
        --self-contained false `
        -p:PublishSingleFile=true
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet publish failed with exit code $LASTEXITCODE"
    }

    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    Copy-Item '.\target\release\nxtools-desktop.exe' (Join-Path $destination 'NxTools.exe')
    Copy-Item '.\target\release\nxtools-indexer.exe' (Join-Path $destination 'NxTools.Indexer.exe')
    Copy-Item '.\csharp\NxTools.PlatformHost\bin\Release\net8.0-windows\win-x64\publish\NxTools.PlatformHost.exe' $destination
    Copy-Item '.\LICENSE-MIT', '.\LICENSE-APACHE' $destination
}
finally {
    Pop-Location
}

Write-Host "Package created at $destination"
