Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$buildScript = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\build.ps1')
$packageScript = Get-Content -Raw -LiteralPath (Join-Path $root 'scripts\package.ps1')
$toolScriptPath = Join-Path $root 'scripts\windows-build-tools.ps1'
$toolScript = Get-Content -Raw -LiteralPath $toolScriptPath
$zedSettingsPath = Join-Path $root '.zed\settings.json'

function Assert-Contains(
    [string]$Text,
    [string]$Pattern,
    [string]$Failure) {
    if ($Text -notmatch $Pattern) {
        throw $Failure
    }
}

Assert-Contains $toolScript 'function\s+Initialize-LowMemoryRustBuildEnvironment' `
    'shared Windows build tools must own the low-memory Rust environment'
Assert-Contains $toolScript "CARGO_BUILD_JOBS\s*=\s*'1'" `
    'the shared low-memory environment must serialize Cargo work'
Assert-Contains $toolScript "CARGO_INCREMENTAL\s*=\s*'0'" `
    'the shared low-memory environment must disable incremental compilation'
Assert-Contains $toolScript "CARGO_PROFILE_TEST_DEBUG\s*=\s*'0'" `
    'the shared low-memory environment must disable test debug information'
Assert-Contains $toolScript 'function\s+Assert-ReleaseBuildCommitHeadroom' `
    'Release packaging must have a commit-headroom safety preflight'

. $toolScriptPath
Initialize-LowMemoryRustBuildEnvironment
if ($env:CARGO_BUILD_JOBS -ne '1' -or
    $env:CARGO_INCREMENTAL -ne '0' -or
    $env:CARGO_PROFILE_DEV_DEBUG -ne '0' -or
    $env:CARGO_PROFILE_TEST_DEBUG -ne '0' -or
    $env:CARGO_PROFILE_RELEASE_DEBUG -ne '0' -or
    $env:CARGO_PROFILE_RELEASE_LTO -ne 'off' -or
    $env:RUSTFLAGS -notmatch '(?:^|\s)-C\s*codegen-units=256(?:\s|$)') {
    throw 'the shared low-memory initializer did not apply the complete Cargo environment'
}

foreach ($entry in @(
        @{ Name = 'build.ps1'; Source = $buildScript },
        @{ Name = 'package.ps1'; Source = $packageScript })) {
    Assert-Contains $entry.Source 'Initialize-LowMemoryRustBuildEnvironment' `
        "$($entry.Name) must initialize its own low-memory Rust environment"
    Assert-Contains $entry.Source 'cargo[^\r\n]+(?:--jobs|-j)\s+1' `
        "$($entry.Name) must make single-job Cargo execution explicit"
}

Assert-Contains $packageScript 'Assert-ReleaseBuildCommitHeadroom' `
    'package.ps1 must refuse an unsafe Release build before starting Cargo'
Assert-Contains $packageScript 'function\s+Recover-ExactPackageCommitHeadroom' `
    'package.ps1 must own its safe exact-package commit recovery step'
Assert-Contains $packageScript 'Recover-ExactPackageCommitHeadroom[\s\S]+Assert-ReleaseBuildCommitHeadroom' `
    'package.ps1 must retry commit headroom after safe recovery and before Cargo'
foreach ($packagedProcess in @(
        'NxTools.exe',
        'NxTools.Cli.exe',
        'NxTools.PlatformHost.exe')) {
    Assert-Contains $packageScript ([regex]::Escape("Name = '$packagedProcess'")) `
        "safe commit recovery must only target the exact packaged $packagedProcess path"
}
Assert-Contains $packageScript 'Stop-ExactPackagedProcess\s+\$entry\.Name\s+\(Join-Path\s+\$PackageDestination' `
    'safe commit recovery must resolve every process from the configured package destination'

foreach ($unsafeCleanerApi in @(
        'EmptyWorkingSet',
        'MemoryEmptyWorkingSets',
        'MemoryPurgeStandbyList',
        'NtSetSystemInformation')) {
    if ($buildScript -match $unsafeCleanerApi -or
        $packageScript -match $unsafeCleanerApi -or
        $toolScript -match $unsafeCleanerApi) {
        throw "build wrappers must not use system-wide memory cleaner API $unsafeCleanerApi"
    }
}

if ($buildScript -match 'cargo\s+(?:clippy|test)[^\r\n]*--release') {
    throw 'Release packaging must not optimize and link every workspace test target'
}

if (-not (Test-Path -LiteralPath $zedSettingsPath)) {
    throw 'the repository must keep editor file scanning out of build-output trees'
}
$zedSettings = Get-Content -Raw -LiteralPath $zedSettingsPath | ConvertFrom-Json
foreach ($excludedDirectory in @(
        '**/target',
        '**/target-ui-check',
        '**/artifacts',
        '**/bin',
        '**/obj')) {
    if ($excludedDirectory -notin $zedSettings.file_scan_exclusions) {
        throw "Zed file_scan_exclusions must contain $excludedDirectory"
    }
}

Assert-Contains $packageScript '\[switch\]\s*\$SkipInstaller' `
    'isolated validation packages must be able to skip the installer side effect'
Assert-Contains $packageScript 'if\s*\(-not\s+\$SkipInstaller\s+-and\s+\$iscc\s+-and\s+\$Configuration\s+-eq\s+''Release''\)' `
    'the installer must remain enabled by default and be explicitly skippable for validation packages'
Write-Host 'Package memory policy gate passed.'
