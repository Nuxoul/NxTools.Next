[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$SkipVerification,
    [switch]$CreateDistributionArchive,
    [string]$SigningKeyPath = $env:NXTOOLS_RELEASE_SIGNING_KEY
)

# Discover local publisher material, then delegate all signing/build/native ownership to package.ps1.
# CheckOnly never reads private-key contents, reserves a sequence, builds or stops a process.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
if ([string]::IsNullOrWhiteSpace($SigningKeyPath)) {
    $SigningKeyPath = Join-Path $env:USERPROFILE '.nxtools-release-keys\nxtools-next-release.pem'
}
$SigningKeyPath = [IO.Path]::GetFullPath($SigningKeyPath)
$ledgerPath = $SigningKeyPath + '.release-sequence.json'
$packageDirectory = Join-Path $root 'artifacts\win-x64'
$installerPath = Join-Path $root 'artifacts\installer\NxTools.Next-setup-0.7.0.exe'
if (-not (Test-Path -LiteralPath $SigningKeyPath -PathType Leaf)) {
    throw "Existing release PEM not found: $SigningKeyPath. See PACKAGING.md; do not generate a replacement key."
}
if (-not (Test-Path -LiteralPath $ledgerPath -PathType Leaf)) {
    throw "Publisher ledger missing: $ledgerPath. Restore the original ledger; do not reset the sequence."
}
function Read-PositiveSequence([string]$Path, [string]$Property) {
    if ((Get-Item -LiteralPath $Path).Length -gt 1048576) { throw "Sequence metadata is oversized: $Path" }
    $record = Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    $value = $record.$Property
    if ($value -isnot [int] -and $value -isnot [long]) { throw "Invalid integer $Property in $Path" }
    if ($value -le 0) { throw "Invalid positive $Property in $Path" }
    return [long]$value
}
$ledgerSequence = Read-PositiveSequence $ledgerPath 'sequence'
$packageSequence = [long]0
$manifestPath = Join-Path $packageDirectory 'NxTools.release.json'
if (Test-Path -LiteralPath $manifestPath -PathType Leaf) {
    $packageSequence = Read-PositiveSequence $manifestPath 'release_sequence'
}
$floor = [Math]::Max($ledgerSequence, $packageSequence)
if ($floor -eq [long]::MaxValue) { throw 'Release sequence exhausted; do not reset the ledger.' }
$nextSequence = $floor + [long]1
foreach ($tool in @('cargo.exe', 'dotnet.exe', 'git.exe', 'powershell.exe')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "Required tool missing: $tool. See PACKAGING.md." }
}
Initialize-LowMemoryRustBuildEnvironment
Initialize-GpuiReleaseEnvironment
$isccCandidates = @(${env:ProgramFiles(x86)}, $env:ProgramFiles) | Where-Object { $_ } | ForEach-Object { Join-Path $_ 'Inno Setup 6\ISCC.exe' }
$iscc = $isccCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 ISCC.exe is missing; install it before producing the installer.' }
$headroom = Get-ReleaseBuildCommitHeadroomGiB
$activeBuilds = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -match '^(cargo|rustc|clippy-driver|ISCC)\.exe$' })
$ready = $headroom -ge 6.0 -and $activeBuilds.Count -eq 0
[pscustomobject]@{
    Repository = $root
    SigningKeyPath = $SigningKeyPath
    PrivateKeyContentsReadByCheck = $false
    LedgerPath = $ledgerPath
    LedgerSequence = $ledgerSequence
    PackagedSequence = $packageSequence
    NextSequence = $nextSequence
    FxcPath = $env:GPUI_FXC_PATH
    InnoSetupPath = $iscc
    CommitHeadroomGiB = [Math]::Round($headroom, 2)
    MinimumCommitHeadroomGiB = 6.0
    ActiveBuildCount = $activeBuilds.Count
    Ready = $ready
    SequenceReserved = $false
    CheckOnly = [bool]$CheckOnly
    SkipVerification = [bool]$SkipVerification
    CreateDistributionArchive = [bool]$CreateDistributionArchive
    InstallerPath = $installerPath
} | ConvertTo-Json
if ($CheckOnly) { return }
if (-not $ready) { throw 'Packaging preflight refused: wait for the shared build lane or at least 6 GiB commit headroom. Do not kill unrelated processes.' }

# Persist logs and the previous package locally, not in an AI session's temporary scratch.
$runDirectory = Join-Path $root ('.diagnostics\packaging\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
foreach ($entry in @(@{ Source = $packageDirectory; Name = 'previous-package' }, @{ Source = (Split-Path -Parent $installerPath); Name = 'previous-installer' })) {
    if (Test-Path -LiteralPath $entry.Source -PathType Container) {
        Copy-Item -LiteralPath $entry.Source -Destination (Join-Path $runDirectory $entry.Name) -Recurse
    }
}
$statusPath = Join-Path $runDirectory 'status.json'
$started = Get-Date -Format o
$state = @{ state = 'Building'; started = $started; sequence = $nextSequence; skip_verification = [bool]$SkipVerification; create_distribution_archive = [bool]$CreateDistributionArchive; installation_executed = $false }
$state | ConvertTo-Json | Set-Content -LiteralPath $statusPath -Encoding UTF8
$arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'package.ps1'), '-Configuration', 'Release', '-SigningKeyPath', $SigningKeyPath, '-ReleaseSequence', $nextSequence.ToString([Globalization.CultureInfo]::InvariantCulture))
if ($SkipVerification) { $arguments += '-SkipVerification' }
if ($CreateDistributionArchive) { $arguments += '-CreateDistributionArchive' }
try {
    Write-Host "Packaging sequence $nextSequence. Logs and backup: $runDirectory"
    # Capture OS streams without turning normal native stderr (Cargo progress) into PowerShell errors.
    # Windows PowerShell Start-Process joins ArgumentList, so retain paths containing spaces.
    $quotedArguments = @($arguments | ForEach-Object { '"' + $_.Replace('"', '\"') + '"' })
    $packageProcess = Start-Process -FilePath (Get-Command 'powershell.exe').Source `
        -ArgumentList ($quotedArguments -join ' ') -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput (Join-Path $runDirectory 'package.log') `
        -RedirectStandardError (Join-Path $runDirectory 'package.stderr.log')
    $packageProcess.WaitForExit()
    $packageExit = $packageProcess.ExitCode
    $state.package_process_id = $packageProcess.Id
    $state.package_exit_code = $packageExit
    if ($null -eq $packageExit) { throw 'Packager process exit code is unavailable; inspect the retained process and logs before retrying.' }
    if ($packageExit -ne 0) { throw "package.ps1 failed with exit $packageExit. Reserved sequences are never reused; see package.log and package.stderr.log." }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ([long]$manifest.release_sequence -ne $nextSequence) { throw 'Produced manifest sequence does not match this run.' }
    foreach ($file in $manifest.files) {
        $member = Join-Path $packageDirectory $file.path
        if ((Get-Item -LiteralPath $member).Length -ne [long]$file.size -or (Get-FileHash -LiteralPath $member -Algorithm SHA256).Hash -ne $file.sha256) {
            throw "Packaged member hash/size mismatch: $($file.path)"
        }
    }
    $installerHash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash
    "$installerHash  $([IO.Path]::GetFileName($installerPath))" | Set-Content -LiteralPath ($installerPath + '.sha256') -Encoding ASCII
    $state.state = 'Completed'
    $state.installer = $installerPath
    $state.installer_sha256 = $installerHash
    $state.installer_bytes = (Get-Item -LiteralPath $installerPath).Length
    $state.manifest_files_checked = @($manifest.files).Count
    $state.authenticode = (Get-AuthenticodeSignature -LiteralPath $installerPath).Status.ToString()
    Write-Host "Installer: $installerPath"
    Write-Host "SHA256: $installerHash"
}
catch {
    $state.state = 'Failed'
    $state.failure = $_.Exception.Message
    throw
}
finally {
    $state.finished = Get-Date -Format o
    $state | ConvertTo-Json | Set-Content -LiteralPath $statusPath -Encoding UTF8
}
