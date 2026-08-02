param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:RUSTFLAGS = '-C codegen-units=256'

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
    & '.\tests\architecture\download-ui-lifecycle.ps1'
    & '.\tests\architecture\taskbar-overlay-lifecycle.ps1'

    node --check '.\browser_extension\nxtools-interceptor\background.js'
    Assert-NativeSuccess 'browser extension background syntax'
    node --check '.\browser_extension\nxtools-interceptor\popup.js'
    Assert-NativeSuccess 'browser extension popup syntax'
    node --test '.\browser_extension\nxtools-interceptor\tests\*.test.cjs'
    Assert-NativeSuccess 'browser extension tests'

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
        cargo build --release -p nxtools-desktop -p nxtools-indexer -p nxtools-cli
        Assert-NativeSuccess 'cargo release binaries'
        & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
            -Profile 'release'
    }
    else {
        cargo build -p nxtools-desktop -p nxtools-indexer -p nxtools-cli
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
