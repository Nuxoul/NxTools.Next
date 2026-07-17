param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Assert-NativeSuccess([string]$Step) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE"
    }
}

function Find-Fxc {
    if ($env:GPUI_FXC_PATH -and (Test-Path -LiteralPath $env:GPUI_FXC_PATH)) {
        return $env:GPUI_FXC_PATH
    }

    $fromPath = Get-Command 'fxc.exe' -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    $sdkBin = 'C:\Program Files (x86)\Windows Kits\10\bin'
    if (Test-Path -LiteralPath $sdkBin) {
        return Get-ChildItem -LiteralPath $sdkBin -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName 'x64\fxc.exe' } |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
    }

    return $null
}

if ($Configuration -eq 'Release') {
    $fxcPath = Find-Fxc
    if (-not $fxcPath) {
        throw 'Release builds of GPUI 0.2.2 require fxc.exe. Install the Windows 10/11 SDK, or set GPUI_FXC_PATH to an existing x64 fxc.exe.'
    }
    $env:GPUI_FXC_PATH = $fxcPath
}

Push-Location $root
try {
    cargo fmt --all --check
    Assert-NativeSuccess 'cargo fmt'

    if ($Configuration -eq 'Release') {
        cargo clippy --workspace --all-targets --release -- -D warnings
    }
    else {
        cargo clippy --workspace --all-targets -- -D warnings
    }
    Assert-NativeSuccess 'cargo clippy'

    if ($Configuration -eq 'Release') {
        cargo test --workspace --release
    }
    else {
        cargo test --workspace
    }
    Assert-NativeSuccess 'cargo test'

    dotnet build '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' -c $Configuration
    Assert-NativeSuccess 'dotnet build'

    dotnet run `
        --project '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' `
        -c $Configuration `
        --no-build
    Assert-NativeSuccess 'C# protocol tests'
}
finally {
    Pop-Location
}
