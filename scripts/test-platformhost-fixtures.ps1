param(
    [Parameter(Mandatory = $true)][ValidateSet('Debug', 'Release')][string]$Configuration,
    [Parameter(Mandatory = $true)][string]$HostBuildRoot
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
Initialize-LowMemoryRustBuildEnvironment
function Assert-NativeSuccess([string]$Step) {
    if ($LASTEXITCODE -ne 0) { throw "$Step failed with exit code $LASTEXITCODE" }
}
$originalWorkerFixture = $env:NXTOOLS_ARCHIVE_WORKER_FIXTURE
$originalLifecycleEvidence = $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT
$originalVolumeEvidence = $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT
$fixtureBuildRoot = $null
Push-Location $root
try {
    if ($Configuration -eq 'Release') {
        # These Debug outputs exist only for lower-threshold behavioral verification.
        # Release product outputs and their identity graph are not replaced or republished.
        Assert-ReleaseBuildCommitHeadroom -MinimumGiB 6
        Assert-CargoBuildLaneIdle
        cargo build --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        Assert-NativeSuccess 'matching Debug fixture binaries'
        Assert-CargoBuildLaneIdle
        cargo build --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs
        Assert-NativeSuccess 'matching Debug isolated-codecs fixture worker'
        $fixtureBuildRoot = Join-Path (Split-Path -Parent $HostBuildRoot) ('vf-' + [Guid]::NewGuid().ToString('N'))
        & (Join-Path $PSScriptRoot 'publish-release-hosts.ps1') -Configuration Debug `
            -GeneratedDirectory (Join-Path $fixtureBuildRoot 'generated') -BuildDirectory $fixtureBuildRoot
        dotnet build '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' -c Debug `
            "-p:ReleaseBinaryManifestPath=$(Join-Path $fixtureBuildRoot 'generated\ReleaseBinaryManifest.g.cs')" `
            "-p:ArchiveReleaseManifestPath=$(Join-Path $fixtureBuildRoot 'generated\ArchiveReleaseManifest.g.cs')" `
            "-p:ShellSelectionIdentityPath=$(Join-Path $fixtureBuildRoot 'generated\DesktopIdentity.g.cs')"
        Assert-NativeSuccess 'matching Debug bound fixture host'
    }
    $fixtureEvidence = Join-Path $root ('.diagnostics\verification\fixtures-' + [Guid]::NewGuid().ToString('N'))
    if (-not $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT -or -not $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT) {
        New-Item -ItemType Directory -Path $fixtureEvidence | Out-Null
    }
    if (-not $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT) { $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT = $fixtureEvidence }
    if (-not $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT) { $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT = $fixtureEvidence }
    Write-Host "Fixture evidence retained: $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT; $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT"
    $env:NXTOOLS_ARCHIVE_WORKER_FIXTURE = Join-Path $root 'target\debug\nxtools-archive-worker.exe'
    dotnet run `
        --project '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj' `
        -c Debug `
        --no-build -- --fixtures
    Assert-NativeSuccess 'C# isolated fixture tests'
}
finally {
    $env:NXTOOLS_ARCHIVE_WORKER_FIXTURE = $originalWorkerFixture
    $env:NXTOOLS_ARCHIVE_CLI_LIFECYCLE_EVIDENCE_ROOT = $originalLifecycleEvidence
    $env:NXTOOLS_ARCHIVE_VOLUME_FIXTURE_ROOT = $originalVolumeEvidence
    try {
        if ($fixtureBuildRoot -and (Test-Path -LiteralPath $fixtureBuildRoot)) {
            Remove-Item -LiteralPath $fixtureBuildRoot -Recurse -Force
        }
    }
    finally {
        Pop-Location
    }
}
