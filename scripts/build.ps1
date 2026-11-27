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

function Find-TarStreamOracle {
    if ($env:NXTOOLS_TAR_STREAM_ORACLE) {
        if (-not (Test-Path -LiteralPath $env:NXTOOLS_TAR_STREAM_ORACLE -PathType Leaf)) {
            throw 'NXTOOLS_TAR_STREAM_ORACLE must point to an existing 7z.exe or 7zz.exe.'
        }
        return (Get-Item -LiteralPath $env:NXTOOLS_TAR_STREAM_ORACLE).FullName
    }
    $fromPath = Get-Command '7z.exe', '7zz.exe' -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($fromPath) { return $fromPath.Source }
    if ($env:ProgramFiles) {
        $installed = Join-Path $env:ProgramFiles '7-Zip\7z.exe'
        if (Test-Path -LiteralPath $installed -PathType Leaf) { return $installed }
    }
    throw 'The TAR/stream fixtures require an independent 7-Zip Zstandard decoder. Put 7z.exe or 7zz.exe on PATH, or set NXTOOLS_TAR_STREAM_ORACLE to its existing path.'
}

if ($Configuration -eq 'Release') {
    Initialize-GpuiReleaseEnvironment
    Assert-ReleaseBuildCommitHeadroom -MinimumGiB $MinimumCommitHeadroomGiB
}

$hostBuildRoot = $null
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
    $tarStreamOracle = Find-TarStreamOracle
    Assert-CargoBuildLaneIdle
    cargo fmt --all --check
    Assert-NativeSuccess 'cargo fmt'

    Assert-CargoBuildLaneIdle
    cargo clippy --workspace --all-targets --exclude gpui-kit-markdown-editor --jobs 1 -- -D warnings
    Assert-NativeSuccess 'first-party cargo clippy'
    Assert-CargoBuildLaneIdle
    cargo clippy -p gpui-kit-markdown-editor --lib --jobs 1 -- -D warnings
    Assert-NativeSuccess 'fixed editor library clippy'

    $originalFixtureEnvironment = @{}
    foreach ($name in @('PI_SCRATCH_DIR', 'TEMP', 'TMP', 'STORY_MD', 'NXTOOLS_TAR_STREAM_CHAIN_CASES',
        'NXTOOLS_TAR_STREAM_EVIDENCE_ROOT', 'NXTOOLS_TAR_STREAM_FIXTURES', 'NXTOOLS_TAR_STREAM_HOST',
        'NXTOOLS_ARCHIVE_WORKER_FIXTURE', 'NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE', 'NXTOOLS_OPT03_CREATE_WIRE')) {
        $originalFixtureEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
    }
    $fixtureScratch = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
    $rustFixtureRoot = $null
    try {
        $rustFixtureRoot = (New-Item -ItemType Directory -Path (Join-Path $fixtureScratch ('rust-tests-' + [Guid]::NewGuid().ToString('N')))).FullName
        $env:PI_SCRATCH_DIR = $rustFixtureRoot
        $env:TEMP = $rustFixtureRoot
        $env:TMP = $rustFixtureRoot
        $env:STORY_MD = $null
        $env:NXTOOLS_TAR_STREAM_CHAIN_CASES = $null
        $env:NXTOOLS_TAR_STREAM_EVIDENCE_ROOT = (New-Item -ItemType Directory -Path (Join-Path $rustFixtureRoot 'tar-stream')).FullName
        $env:NXTOOLS_TAR_STREAM_FIXTURES = Join-Path $env:NXTOOLS_TAR_STREAM_EVIDENCE_ROOT 'fixtures'
        python '.\tests\integration\prepare-tar-stream-safety-fixtures.py' `
            --root $env:NXTOOLS_TAR_STREAM_EVIDENCE_ROOT --oracle $tarStreamOracle > $null
        Assert-NativeSuccess 'independent TAR/stream fixture preparation'

        Assert-CargoBuildLaneIdle
        cargo build --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs
        Assert-NativeSuccess 'Debug isolated-codecs workspace fixture worker'
        # Cargo's default workspace test can replace the canonical Debug worker without these features.
        $env:NXTOOLS_ARCHIVE_WORKER_FIXTURE = Join-Path $rustFixtureRoot 'nxtools-archive-worker.exe'
        Copy-Item -LiteralPath '.\target\debug\nxtools-archive-worker.exe' -Destination $env:NXTOOLS_ARCHIVE_WORKER_FIXTURE

        # The isolated entries use the default Debug apphost path, not signed release bindings.
        $testsProject = '.\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj'
        dotnet build $testsProject -c Debug
        Assert-NativeSuccess 'Debug workspace fixture apphost'
        $env:NXTOOLS_TAR_STREAM_HOST = Join-Path $root 'csharp\NxTools.PlatformHost.Tests\bin\Debug\net8.0-windows\NxTools.PlatformHost.Tests.exe'
        $env:NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE = Join-Path $rustFixtureRoot 'archive-ac-progress.json'
        $env:NXTOOLS_OPT03_CREATE_WIRE = $rustFixtureRoot
        dotnet run --project $testsProject -c Debug --no-build -- --fixtures `
            'archive OPT01 AC repair production progress wire' 'OPT03 create actual native result locations'
        Assert-NativeSuccess 'Rust workspace C# wire fixture exports'

        Assert-CargoBuildLaneIdle
        cargo test --workspace --jobs 1
        Assert-NativeSuccess 'cargo test'
    }
    finally {
        foreach ($name in $originalFixtureEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($name, $originalFixtureEnvironment[$name])
        }
        if ($rustFixtureRoot) { Remove-Item -LiteralPath $rustFixtureRoot -Recurse -Force }
    }

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
    try {
        if ($hostBuildRoot -and (Test-Path -LiteralPath $hostBuildRoot)) {
            Remove-Item -LiteralPath $hostBuildRoot -Recurse -Force
        }
    }
    finally {
        Pop-Location
    }
}
