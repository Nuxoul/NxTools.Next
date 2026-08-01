param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')

function Assert-NativeSuccess([string]$Step) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE"
    }
}

if ($Configuration -eq 'Release') {
    Initialize-GpuiReleaseEnvironment
}

Push-Location $root
try {
    & '.\tests\architecture\quick-launch-lifecycle.ps1'
    & '.\tests\architecture\indexer-lifecycle.ps1'

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

    if ($Configuration -eq 'Release') {
        cargo build --release -p nxtools-desktop -p nxtools-indexer
        Assert-NativeSuccess 'cargo release binaries'
        & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
            -Profile 'release'
    }
    else {
        cargo build -p nxtools-desktop -p nxtools-indexer
        Assert-NativeSuccess 'cargo debug binaries'
        & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
            -Profile 'debug'
    }

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
