param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [string]$OutputPath = '',
    [switch]$SkipVerification,
    [switch]$SkipInstaller,
    [switch]$CreateDistributionArchive,
    [ValidateRange(0.0, 1024.0)]
    [double]$MinimumCommitHeadroomGiB = 6.0,
    [string]$SigningKeyPath = $env:NXTOOLS_RELEASE_SIGNING_KEY,
    [long]$ReleaseSequence = 0,
    [switch]$LegacyUnsigned,
    [switch]$IsolatedStaging
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
Initialize-LowMemoryRustBuildEnvironment
$profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
if (-not $OutputPath) {
    $OutputPath = if ($Configuration -eq 'Release') {
        'artifacts\win-x64'
    }
    else {
        'artifacts\win-x64-debug'
    }
}
$destination = if ([IO.Path]::IsPathRooted($OutputPath)) { [IO.Path]::GetFullPath($OutputPath) } else { [IO.Path]::GetFullPath((Join-Path $root $OutputPath)) }
$buildRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
if ($IsolatedStaging) {
    $scratchPrefix = [IO.Path]::GetFullPath($buildRoot).TrimEnd('\') + '\'
    if ($Configuration -ne 'Release' -or -not $SkipVerification -or -not $SkipInstaller -or $LegacyUnsigned -or
        $SigningKeyPath -or $ReleaseSequence -ne 0 -or -not $destination.StartsWith($scratchPrefix, [StringComparison]::OrdinalIgnoreCase) -or
        (Test-Path -LiteralPath $destination)) { throw 'IsolatedStaging requires a fresh scratch output, Release, SkipVerification, SkipInstaller, no signing key/sequence/LegacyUnsigned. It never reserves production identity.' }
}
$packageBuild = Join-Path $buildRoot ('package-build-' + [Guid]::NewGuid().ToString('N'))
$generatedDirectory = Join-Path $packageBuild 'generated'
try {
New-Item -ItemType Directory -Force -Path $generatedDirectory | Out-Null
$trustInput = ''
$ownedTestKeyPath = ''
$restartIndexerServiceOnFailure = $false
$packageCompleted = $false

if (-not $IsolatedStaging -and $Configuration -eq 'Release' -and -not $LegacyUnsigned) {
    if (-not $SigningKeyPath -or $ReleaseSequence -le 0) {
        throw 'Trusted Release packaging requires an explicit signing PEM and increasing -ReleaseSequence. Use -LegacyUnsigned only for an explicitly non-trusted legacy package.'
    }
    & (Join-Path $PSScriptRoot 'reserve-release-sequence.ps1') `
        -SigningKeyPath $SigningKeyPath -ReleaseSequence $ReleaseSequence -PackageDirectory $destination
    & (Join-Path $PSScriptRoot 'write-update-trust.ps1') -SigningKeyPath $SigningKeyPath
}
elseif ($LegacyUnsigned) {
    $trustFile = Join-Path $root 'csharp\NxTools.PlatformHost\Generated\UpdateTrust.g.cs'
    [IO.File]::WriteAllText($trustFile, "namespace NxTools.PlatformHost.Generated;`ninternal static class UpdateTrust { public const string PublicKeySpkiBase64 = `"`"; }`n")
}
function Copy-PackageFile([string]$Source, [string]$Destination) {
    for ($attempt = 0; $attempt -lt 75; $attempt++) {
        try {
            Copy-Item -LiteralPath $Source -Destination $Destination -Force -ErrorAction Stop
            return
        }
        catch [System.IO.IOException] {
            if ($attempt -eq 74) {
                throw
            }
            Start-Sleep -Milliseconds 200
        }
    }
}

function Stop-ExactPackagedProcess(
    [string]$ProcessName,
    [string]$ExpectedPath) {
    $fullExpectedPath = [System.IO.Path]::GetFullPath($ExpectedPath)
    $stoppedAny = $false
    $processes = Get-CimInstance Win32_Process -Filter "Name='$ProcessName'" `
        -ErrorAction SilentlyContinue
    foreach ($process in $processes) {
        if (-not $process.ExecutablePath) {
            continue
        }
        $fullProcessPath = [System.IO.Path]::GetFullPath($process.ExecutablePath)
        if ($fullProcessPath -ne $fullExpectedPath) {
            continue
        }
        Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
        Wait-Process -Id $process.ProcessId -Timeout 5 -ErrorAction SilentlyContinue
        $stoppedAny = $true
        Write-Host "Stopped packaged process $ProcessName (PID $($process.ProcessId))."
    }
    return $stoppedAny
}

function Recover-ExactPackageCommitHeadroom(
    [double]$MinimumGiB,
    [string]$PackageDestination) {
    $beforeGiB = Get-ReleaseBuildCommitHeadroomGiB
    if ($beforeGiB -ge $MinimumGiB) {
        return $false
    }

    Write-Host ('Release commit headroom is below the safe threshold; stopping only exact ' +
        'package-path processes before the final preflight.')
    $stoppedAny = $false
    foreach ($entry in @(
            @{ Name = 'NxTools.exe'; File = 'NxTools.exe' },
            @{ Name = 'NxTools.Cli.exe'; File = 'NxTools.Cli.exe' },
            @{ Name = 'NxTools.BrowserHost.exe'; File = 'NxTools.BrowserHost.exe' },
            @{ Name = 'NxTools.PlatformHost.exe'; File = 'NxTools.PlatformHost.exe' })) {
        if (Stop-ExactPackagedProcess $entry.Name (Join-Path $PackageDestination $entry.File)) {
            $stoppedAny = $true
        }
    }

    if ($stoppedAny) {
        Start-Sleep -Milliseconds 500
        $afterGiB = Get-ReleaseBuildCommitHeadroomGiB
        Write-Host ('Release commit headroom after exact-package recovery: {0:N2} GiB ' +
            '(before: {1:N2} GiB).' -f $afterGiB, $beforeGiB)
    }
    return $stoppedAny
}

Push-Location $root
try {
    if ($IsolatedStaging) {
        # Claim the exact fresh key before creation: even create-then-throw is cleaned.
        $ownedTestKeyPath = Join-Path $packageBuild 'TEST-ONLY-private.pem'
        $SigningKeyPath = $ownedTestKeyPath
        & (Join-Path $PSScriptRoot 'new-release-signing-key.ps1') -KeyPath $ownedTestKeyPath
        $ReleaseSequence = 1 # isolated test-key namespace only; no publisher ledger reservation
        $trustInput = Join-Path $generatedDirectory 'UpdateTrust.g.cs'
        & (Join-Path $PSScriptRoot 'write-update-trust.ps1') -SigningKeyPath $ownedTestKeyPath -OutputPath $trustInput
        [IO.File]::WriteAllText(($destination + '.STAGING-NOT-INSTALLABLE.txt'), 'ISOLATED TEST-KEY STAGING ONLY. Not a production-trusted or installable release. No installer/registration/deployment authorization.')
    }
    if ($Configuration -eq 'Release') {
        Initialize-GpuiReleaseEnvironment
        if (-not $IsolatedStaging) {
            $null = Recover-ExactPackageCommitHeadroom -MinimumGiB $MinimumCommitHeadroomGiB -PackageDestination $destination
        }
        Assert-ReleaseBuildCommitHeadroom -MinimumGiB $MinimumCommitHeadroomGiB
    }
    if (-not $SkipVerification) {
        & (Join-Path $PSScriptRoot 'build.ps1') `
            -Configuration $Configuration `
            -MinimumCommitHeadroomGiB $MinimumCommitHeadroomGiB
    }
    else {
        if ($Configuration -eq 'Release') {
        Assert-CargoBuildLaneIdle
        cmd.exe /d /c "cargo build --release --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host 2>&1"
        if ($LASTEXITCODE -ne 0) { throw "cargo product build failed: $LASTEXITCODE" }
        Assert-CargoBuildLaneIdle
        cmd.exe /d /c "cargo build --release --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs 2>&1"
        }
        else {
        Assert-CargoBuildLaneIdle
        cmd.exe /d /c "cargo build --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host 2>&1"
        if ($LASTEXITCODE -ne 0) { throw "cargo product build failed: $LASTEXITCODE" }
        Assert-CargoBuildLaneIdle
        cmd.exe /d /c "cargo build --jobs 1 -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs 2>&1"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    }
    # Desktop -> ShellSelectionHelper -> complete approval -> PlatformHost: acyclic identity order.
    & (Join-Path $PSScriptRoot 'publish-release-hosts.ps1') -Configuration $Configuration `
        -GeneratedDirectory $generatedDirectory -BuildDirectory $packageBuild -UpdateTrustPath $trustInput

    if (-not $IsolatedStaging) {
    $null = Stop-ExactPackagedProcess 'NxTools.exe' (Join-Path $destination 'NxTools.exe')
    $null = Stop-ExactPackagedProcess 'NxTools.Cli.exe' (Join-Path $destination 'NxTools.Cli.exe')
    $null = Stop-ExactPackagedProcess `
        'NxTools.BrowserHost.exe' `
        (Join-Path $destination 'NxTools.BrowserHost.exe')
    $null = Stop-ExactPackagedProcess `
        'NxTools.PlatformHost.exe' `
        (Join-Path $destination 'NxTools.PlatformHost.exe')
    $packagedIndexer = [System.IO.Path]::GetFullPath((Join-Path $destination 'NxTools.Indexer.exe'))
    $installedService = Get-CimInstance Win32_Service -Filter "Name='NxToolsIndexerV1'" `
        -ErrorAction SilentlyContinue
    if ($installedService) {
        $serviceExecutable = if ($installedService.PathName -match '^"([^"]+)"') {
            $Matches[1]
        }
        else {
            ($installedService.PathName -split '\s+', 2)[0]
        }
        if ([System.IO.Path]::GetFullPath($serviceExecutable) -eq $packagedIndexer -and
            $installedService.State -eq 'Running') {
            Stop-Service -Name $installedService.Name -Force
            $service = Get-Service -Name $installedService.Name
            $service.WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Stopped,
                [TimeSpan]::FromSeconds(15))
            $restartIndexerServiceOnFailure = $true
        }
    }
    }
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    Copy-PackageFile ".\target\$profile\nxtools-desktop.exe" (Join-Path $destination 'NxTools.exe')
    Copy-PackageFile ".\target\$profile\nxtools-indexer.exe" (Join-Path $destination 'NxTools.Indexer.exe')
    Copy-PackageFile ".\target\$profile\nxtools-cli.exe" (Join-Path $destination 'NxTools.Cli.exe')
    Copy-PackageFile ".\target\$profile\nxtools-browser-host.exe" (Join-Path $destination 'NxTools.BrowserHost.exe')
    Copy-PackageFile ".\target\$profile\nxtools-archive-worker.exe" (Join-Path $destination 'NxTools.ArchiveWorker.exe')
    foreach ($name in @('NxTools.ShellSelectionHelper.exe', 'NxTools.ShellSelectionHelper.dll', 'NxTools.ShellSelectionHelper.deps.json', 'NxTools.ShellSelectionHelper.runtimeconfig.json')) {
        Copy-PackageFile (Join-Path $packageBuild "shell-publish\$name") (Join-Path $destination $name)
    }
    if ($Configuration -eq 'Release') {
        & '.\tests\architecture\desktop-release-subsystem.ps1' `
            -DesktopExecutable (Join-Path $destination 'NxTools.exe')
        & '.\tests\architecture\desktop-product-icon.ps1' `
            -DesktopExecutable (Join-Path $destination 'NxTools.exe')
        & '.\tests\architecture\bundled-extensions.ps1' `
            -DesktopExecutable (Join-Path $destination 'NxTools.exe')
    }
    Copy-PackageFile (Join-Path $packageBuild 'host-publish\NxTools.PlatformHost.exe') (Join-Path $destination 'NxTools.PlatformHost.exe')
    $packagedBrowserExtensionRoot = Join-Path $destination 'browser_extension'
    New-Item -ItemType Directory -Force -Path $packagedBrowserExtensionRoot | Out-Null
    Copy-Item '.\browser_extension\nxtools-interceptor' `
        $packagedBrowserExtensionRoot `
        -Recurse `
        -Force
    Copy-Item '.\LICENSE-MIT', '.\LICENSE-APACHE', '.\THIRD_PARTY_NOTICES.md' $destination
    & python (Join-Path $PSScriptRoot 'package-archive-source.py') `
        --output (Join-Path $root 'artifacts\installer\third_party\archive-worker\archive-worker-source.zip')
    if ($LASTEXITCODE -ne 0) { throw 'Archive source distribution failed; do not sign an incomplete package.' }
    Remove-Item (Join-Path $destination 'NxTools.release.json'), (Join-Path $destination 'NxTools.release.sig') -Force -ErrorAction SilentlyContinue
    if ($Configuration -eq 'Release' -and -not $LegacyUnsigned) {
        & (Join-Path $PSScriptRoot 'write-trusted-release-manifest.ps1') `
            -PackageDirectory $destination -SigningKeyPath $SigningKeyPath -ReleaseSequence $ReleaseSequence
    }
    else {
        Remove-Item (Join-Path $destination 'NxTools.release.json'), (Join-Path $destination 'NxTools.release.sig') -Force -ErrorAction SilentlyContinue
    }
    $packageCompleted = $true
    if ($IsolatedStaging) {
        Copy-Item -LiteralPath $trustInput -Destination ($destination + '.TEST-public-trust.g.cs')
        $retainedGeneratedDirectory = $destination + '.generated-inputs'
        New-Item -ItemType Directory -Path $retainedGeneratedDirectory | Out-Null
        foreach ($name in @('DesktopIdentity.g.cs', 'ReleaseBinaryManifest.g.cs', 'ArchiveReleaseManifest.g.cs', 'UpdateTrust.g.cs')) {
            Copy-Item -LiteralPath (Join-Path $generatedDirectory $name) -Destination (Join-Path $retainedGeneratedDirectory $name)
        }
        $retainedHostAssembly = $destination + '.TEST-PlatformHost.dll'
        Copy-Item -LiteralPath (Join-Path $packageBuild 'host-build\bin\NxTools.PlatformHost\release_win-x64\NxTools.PlatformHost.dll') -Destination $retainedHostAssembly
        [IO.File]::WriteAllText(($destination + '.build-inputs.json'), (@{ generated_directory=$retainedGeneratedDirectory; host_assembly=$retainedHostAssembly; test_only=$true; production_sequence_reserved=$false } | ConvertTo-Json))
        Remove-Item -LiteralPath $ownedTestKeyPath -Force
    }
}
finally {
    try {
        if ($ownedTestKeyPath -and (Test-Path -LiteralPath $ownedTestKeyPath -PathType Leaf)) {
            Remove-Item -LiteralPath $ownedTestKeyPath -Force
        }
        if ($restartIndexerServiceOnFailure -and -not $packageCompleted) {
            Start-Service -Name 'NxToolsIndexerV1'
            $service = Get-Service -Name 'NxToolsIndexerV1'
            $service.WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Running,
                [TimeSpan]::FromSeconds(15))
        }
    }
    finally {
        Pop-Location
    }
}

Write-Host "Package created at $destination"

# Build the user-facing installer when Inno Setup is available.
$isccCandidates = @(
    ${env:ProgramFiles(x86)},
    $env:ProgramFiles
) | ForEach-Object { Join-Path $_ 'Inno Setup 6\ISCC.exe' }
$iscc = $isccCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $SkipInstaller -and $iscc -and $Configuration -eq 'Release') {
    $installerScript = Join-Path $root 'installer\nxtools-next.iss'
    # Migration extracts the original normal-install entries from Inno's single payload.
    # Pin the executable before running it; its original verifier checks the complete signed set.
    $migrationHostHash = (Get-FileHash -LiteralPath (Join-Path $destination 'NxTools.PlatformHost.exe') -Algorithm SHA256).Hash
    $migrationInput = Join-Path $root 'artifacts\installer\bootstrap-migration-input.iss'
    [IO.Directory]::CreateDirectory((Split-Path -Parent $migrationInput)) | Out-Null
    [IO.File]::WriteAllText($migrationInput, ('#define BootstrapMigrationHostSha256 "' + $migrationHostHash + '"' + "`n"), [Text.UTF8Encoding]::new($true))
    & (Join-Path $root 'tests\architecture\trusted-release-sequence.ps1')
    & (Join-Path $root 'tests\architecture\installer-exact-file-rollback.ps1')
    # Real exited-process race must pass in the architecture used by Inno as well as x64.
    foreach ($nativeShellDirectory in @('SysWOW64', 'System32')) {
        $nativeShell = Join-Path $env:SystemRoot ($nativeShellDirectory + '\WindowsPowerShell\v1.0\powershell.exe')
        & $nativeShell -NoProfile -ExecutionPolicy Bypass -File `
            (Join-Path $root 'tests\architecture\installer-process-identity.ps1') `
            -HelperPath (Join-Path $root 'installer\close-owned-processes.ps1')
        if ($LASTEXITCODE -ne 0) { throw "Installer process identity regression failed: $nativeShellDirectory" }
        & $nativeShell -NoProfile -ExecutionPolicy Bypass -File `
            (Join-Path $root 'tests\architecture\installer-process-exit-race.ps1')
        if ($LASTEXITCODE -ne 0) { throw "Installer exited-process regression failed: $nativeShellDirectory" }
    }
    foreach ($scenario in @('success', 'failure', 'owned-process', 'exited-process', 'exited-cim-null', 'trusted-success', 'trusted-failure', 'migration-success', 'migration-cancel', 'migration-failure', 'migration-tampered', 'repair-success', 'repair-cancel', 'repair-failure', 'repair-tampered', 'detection-failure')) {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File `
            (Join-Path $root 'tests\architecture\installer-runtime.ps1') `
            -Scenario $scenario
        if ($LASTEXITCODE -ne 0) {
            throw "Installer runtime regression failed: $scenario"
        }
    }
    foreach ($scenario in @('success', 'rollback', 'rollback-after-file', 'commit-failure', 'trusted-failure')) {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File `
            (Join-Path $root 'tests\architecture\installer-browserhost-replacement.ps1') `
            -Scenario $scenario
        if ($LASTEXITCODE -ne 0) {
            throw "BrowserHost file replacement regression failed: $scenario"
        }
    }
    if (Test-Path $installerScript) {
        & $iscc $installerScript | Write-Host
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Installer created at $(Join-Path $root 'artifacts\installer')"
        }
        else {
            throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
        }
    }
}
if ($restartIndexerServiceOnFailure -and $packageCompleted) {
    Write-Host 'The legacy writable-path indexer service remains stopped; the next desktop launch will request one-time migration to the protected service directory.'
}
if ($CreateDistributionArchive -and -not $SkipInstaller -and $iscc -and $Configuration -eq 'Release') {
    & python (Join-Path $PSScriptRoot 'package-archive-source.py') `
        --output (Join-Path $root 'artifacts\installer\third_party\archive-worker\archive-worker-source.zip') `
        --installer (Join-Path $root 'artifacts\installer\NxTools.Next-setup-0.7.0.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Complete release distribution failed.' }
}
}
finally {
    if (Test-Path -LiteralPath $packageBuild) {
        Remove-Item -LiteralPath $packageBuild -Recurse -Force
    }
}
