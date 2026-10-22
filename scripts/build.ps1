param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',
    [ValidateRange(0.0, 1024.0)]
    [double]$MinimumCommitHeadroomGiB = 6.0
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
Initialize-LowMemoryRustBuildEnvironment

function Assert-NativeSuccess([string]$Step) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE"
    }
}

if ($Configuration -eq 'Release') {
    Initialize-GpuiReleaseEnvironment
    Assert-ReleaseBuildCommitHeadroom -MinimumGiB $MinimumCommitHeadroomGiB
}

Push-Location $root
try {
    & '.\tests\architecture\quick-launch-lifecycle.ps1'
    & '.\tests\architecture\indexer-lifecycle.ps1'
    & '.\tests\architecture\desktop-shell-lifecycle.ps1'
    & '.\tests\architecture\platformhost-test-selection.ps1'
    & '.\tests\architecture\browser-signing-isolation.ps1'
    & '.\tests\architecture\trusted-release-sequence.ps1'
    & '.\tests\architecture\installer-exact-file-rollback.ps1'

    cargo fmt --all --check
    Assert-NativeSuccess 'cargo fmt'

    cargo clippy --workspace --all-targets --jobs 1 -- -D warnings
    Assert-NativeSuccess 'cargo clippy'

    cargo test --workspace --jobs 1
    Assert-NativeSuccess 'cargo test'

    if ($Configuration -eq 'Release') {
        cargo build --release --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        Assert-NativeSuccess 'cargo release binaries'
        & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
            -Profile 'release'
    }
    else {
        cargo build --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        Assert-NativeSuccess 'cargo debug binaries'
        & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
            -Profile 'debug'
    }

    dotnet build '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' -c $Configuration
    Assert-NativeSuccess 'dotnet build'

    dotnet run `
        --project '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' `
        -c $Configuration `
        --no-build -- --fixtures
    Assert-NativeSuccess 'C# isolated fixture tests'
}
finally {
    Pop-Location
}
