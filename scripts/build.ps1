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

    python '.\tests\architecture\build-host-bindings.tests.py'
    Assert-NativeSuccess 'verified Host fixture build bindings'
    Assert-CargoBuildLaneIdle
    cargo fmt --all --check
    Assert-NativeSuccess 'cargo fmt'

    Assert-CargoBuildLaneIdle
    cargo clippy --workspace --all-targets --jobs 1 -- -D warnings
    Assert-NativeSuccess 'cargo clippy'

    Assert-CargoBuildLaneIdle
    cargo test --workspace --jobs 1
    Assert-NativeSuccess 'cargo test'

    if ($Configuration -eq 'Release') {
        Assert-CargoBuildLaneIdle
        cargo build --release --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        Assert-NativeSuccess 'cargo release binaries'
        Assert-CargoBuildLaneIdle
        cargo build --release --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs
        Assert-NativeSuccess 'cargo archive isolated-codecs worker'
    }
    else {
        Assert-CargoBuildLaneIdle
        cargo build --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        Assert-NativeSuccess 'cargo debug binaries'
        Assert-CargoBuildLaneIdle
        cargo build --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs
        Assert-NativeSuccess 'cargo archive isolated-codecs worker'
    }
    # ShellSelectionHelper binds only after actual Desktop build, before PlatformHost approval.
    $buildScratch = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
    $hostBuildRoot = Join-Path $buildScratch ('vh-' + [Guid]::NewGuid().ToString('N'))
    & (Join-Path $PSScriptRoot 'publish-release-hosts.ps1') -Configuration $Configuration `
        -GeneratedDirectory (Join-Path $hostBuildRoot 'generated') -BuildDirectory $hostBuildRoot

    # Tests exercising Production() must bind to the exact binaries just verified above.
    dotnet build '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' -c $Configuration `
        "-p:ReleaseBinaryManifestPath=$(Join-Path $hostBuildRoot 'generated\ReleaseBinaryManifest.g.cs')" `
        "-p:ArchiveReleaseManifestPath=$(Join-Path $hostBuildRoot 'generated\ArchiveReleaseManifest.g.cs')" `
        "-p:ShellSelectionIdentityPath=$(Join-Path $hostBuildRoot 'generated\DesktopIdentity.g.cs')"
    Assert-NativeSuccess 'dotnet build'

    if ($Configuration -eq 'Release') {
        # Release must reject private fixture controls; lower-threshold behavior is tested
        # below using matching Debug binaries rather than weakening production predicates.
        dotnet run --project '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' `
            -c Release --no-build -- --fixtures 'archive HZ4 process fixture' 'archive HZ4 TERM01'
        Assert-NativeSuccess 'C# Release fixture capability gates'

    }
    & (Join-Path $PSScriptRoot 'test-platformhost-fixtures.ps1') `
        -Configuration $Configuration -HostBuildRoot $hostBuildRoot
}
finally {
    Pop-Location
}
