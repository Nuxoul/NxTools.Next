param(
    [switch]$LibraryOnly,
    [string]$ManifestPath
)
$ErrorActionPreference = 'Stop'

# Read-only detection. A marker is accepted only with the fixed approved service image/key.
function Get-BootstrapMigrationDecision([string]$ExpectedKeyHash, $Registration, [string]$ExpectedBootstrap, $MaintenanceState, [string]$ActualHash, [bool]$JournalExists) {
    if ($ExpectedKeyHash -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid installer release key identity.' }
    if ($null -eq $Registration) { return 'skip' }
    if (![string]::Equals($Registration.ImagePath, ('"' + $ExpectedBootstrap + '" --trusted-update-service'), [StringComparison]::OrdinalIgnoreCase) -or
        ![string]::Equals($Registration.ObjectName, 'LocalSystem', [StringComparison]::OrdinalIgnoreCase) -or
        ![string]::Equals($Registration.NxToolsUpdateKeySha256, $ExpectedKeyHash, [StringComparison]::OrdinalIgnoreCase) -or
        $ActualHash -notmatch '^[a-fA-F0-9]{64}$' -or
        ![string]::Equals($Registration.NxToolsBootstrapSha256, $ActualHash, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Updater ownership or original publisher key mismatch; installation stopped.'
    }
    if ($JournalExists) { return 'required' }
    if ($null -eq $MaintenanceState) { return 'required' }
    if ($MaintenanceState.ProtocolVersion -ne 1 -or $MaintenanceState.Sequence -le 0 -or
        ![Guid]::TryParse([string]$MaintenanceState.ReleaseId, [ref]([Guid]::Empty)) -or
        $MaintenanceState.ManifestSha256 -notmatch '^[a-fA-F0-9]{64}$' -or
        ![string]::Equals($MaintenanceState.BootstrapSha256, $ActualHash, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Updater migration state does not match the approved image; installation stopped.'
    }
    return 'skip'
}


function Assert-BootstrapPlainPath([string]$Path) {
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        if ((Test-Path -LiteralPath $current) -and ((Get-Item -LiteralPath $current -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Reparse updater path rejected.'
        }
        $parent = [IO.Directory]::GetParent($current)
        $current = if ($parent) { $parent.FullName } else { $null }
    }
}

if ($LibraryOnly) { return }
try {
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $registration = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsUpdateV1' -ErrorAction SilentlyContinue
    $root = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Updater'
    $bootstrap = Join-Path $root 'Bootstrap\NxTools.PlatformHost.exe'
    $markerPath = Join-Path $root 'bootstrap-maintenance-state.json'
    $marker = $null; $actual = ''
    if ($registration) {
        Assert-BootstrapPlainPath $bootstrap
        Assert-BootstrapPlainPath $markerPath
        $actual = (Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256).Hash
        if (Test-Path -LiteralPath $markerPath) {
            if ((Get-Item -LiteralPath $markerPath).Length -gt 16384) { throw 'Migration marker exceeds bound.' }
            $marker = Get-Content -LiteralPath $markerPath -Raw -Encoding UTF8 | ConvertFrom-Json
        }
    }
    $decision = Get-BootstrapMigrationDecision $manifest.public_key_sha256 $registration $bootstrap $marker $actual (Test-Path -LiteralPath (Join-Path $root 'bootstrap-maintenance.json'))
    if ($decision -eq 'required') { exit 10 }
    exit 0
} catch {
    Write-Output 'installer.bootstrap_migration.detection_failed; no mutation performed'
    exit 1
}
