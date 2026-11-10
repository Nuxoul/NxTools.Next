param(
    [switch]$LibraryOnly,
    [string]$ManifestPath,
    [string]$DiagnosticPath,
    [string]$CorrelationId
)
$ErrorActionPreference = 'Stop'

# Read-only detection; repair still requires explicit elevated same-key admission.
function Assert-BootstrapRegistration([string]$ExpectedKeyHash, $Registration, [string]$ExpectedBootstrap) {
    if ($ExpectedKeyHash -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid installer release key identity.' }
    if ($null -eq $Registration -or
        ![string]::Equals($Registration.ImagePath, ('"' + $ExpectedBootstrap + '" --trusted-update-service'), [StringComparison]::OrdinalIgnoreCase) -or
        ![string]::Equals($Registration.ObjectName, 'LocalSystem', [StringComparison]::OrdinalIgnoreCase) -or
        ![string]::Equals($Registration.NxToolsUpdateKeySha256, $ExpectedKeyHash, [StringComparison]::OrdinalIgnoreCase) -or
        $Registration.NxToolsBootstrapSha256 -notmatch '^[a-fA-F0-9]{64}$') {
        throw 'Updater ownership or original publisher key mismatch; installation stopped.'
    }
}

function Get-BootstrapMigrationDecision([string]$ExpectedKeyHash, $Registration, [string]$ExpectedBootstrap, $MaintenanceState, [string]$ActualHash, [bool]$JournalExists) {
    if ($ExpectedKeyHash -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid installer release key identity.' }
    if ($null -eq $Registration) { return 'skip' }
    Assert-BootstrapRegistration $ExpectedKeyHash $Registration $ExpectedBootstrap
    if ($ActualHash -notmatch '^[a-fA-F0-9]{64}$' -or
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
    # Approved older bootstraps registered Auto and keep a resident listener. Replace them
    # through explicit same-key maintenance, not a configuration-only ordinary request.
    if ($Registration.Start -ne 3) { return 'required' }
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

function Write-BootstrapDetection([string]$EventName, [string]$Reason, $Failure = $null) {
    if (!$DiagnosticPath -or $CorrelationId -notmatch '^[a-zA-Z0-9-]{1,100}$') { return }
    try {
        if ((Test-Path -LiteralPath $DiagnosticPath) -and (Get-Item -LiteralPath $DiagnosticPath).Length -gt 61440) { return }
        $record = [ordered]@{schema=1;correlation=$CorrelationId;event=$EventName;stage=$script:detectionStage;reason=$Reason;elapsed_ms=$script:detectionWatch.ElapsedMilliseconds;process_bits=([IntPtr]::Size*8)}
        if ($Failure) {
            $record.exception_type=$Failure.Exception.GetType().FullName
            $record.script_line=$Failure.InvocationInfo.ScriptLineNumber
            $record.hresult=$Failure.Exception.HResult
            $exception=$Failure.Exception
            while ($exception.InnerException) { $exception=$exception.InnerException }
            if ($exception -is [ComponentModel.Win32Exception]) { $record.native_code=$exception.NativeErrorCode }
        }
        $line=($record|ConvertTo-Json -Compress)+[Environment]::NewLine
        if ([Text.Encoding]::UTF8.GetByteCount($line) -le 4096) { [IO.File]::AppendAllText($DiagnosticPath,$line,[Text.UTF8Encoding]::new($false)) }
    } catch { # Diagnostics cannot change the read-only decision.
    }
}

if ($LibraryOnly) { return }
$script:detectionWatch=[Diagnostics.Stopwatch]::StartNew()
$script:detectionStage='manifest'
Write-BootstrapDetection 'started' 'checking'
try {
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.public_key_sha256 -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid installer release key identity.' }
    $script:detectionStage='registration'
    try { $registration = Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsUpdateV1' -ErrorAction Stop }
    catch [Management.Automation.ItemNotFoundException] { $registration=$null }
    if ($null -eq $registration) {
        Write-BootstrapDetection 'completed' 'not_registered'
        exit 0
    }
    $root = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Updater'
    $bootstrap = Join-Path $root 'Bootstrap\NxTools.PlatformHost.exe'
    $markerPath = Join-Path $root 'bootstrap-maintenance-state.json'
    Assert-BootstrapRegistration $manifest.public_key_sha256 $registration $bootstrap
    $script:detectionStage='directory'
    $rootMissing=$false
    try { $null=Get-Item -LiteralPath $root -Force -ErrorAction Stop }
    catch [Management.Automation.ItemNotFoundException] { $rootMissing=$true }
    Assert-BootstrapPlainPath $root
    if ($rootMissing) {
        Write-BootstrapDetection 'completed' 'missing_updater_root'
        exit 11
    }
    $script:detectionStage='bootstrap'
    Assert-BootstrapPlainPath $bootstrap
    Assert-BootstrapPlainPath $markerPath
    $actual = (Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256 -ErrorAction Stop).Hash
    $marker=$null
    $script:detectionStage='marker'
    if (Test-Path -LiteralPath $markerPath) {
        if ((Get-Item -LiteralPath $markerPath).Length -gt 16384) { throw 'Migration marker exceeds bound.' }
        $marker = Get-Content -LiteralPath $markerPath -Raw -Encoding UTF8 | ConvertFrom-Json
    }
    $script:detectionStage='decision'
    $decision = Get-BootstrapMigrationDecision $manifest.public_key_sha256 $registration $bootstrap $marker $actual (Test-Path -LiteralPath (Join-Path $root 'bootstrap-maintenance.json'))
    if ($decision -eq 'required') { Write-BootstrapDetection 'completed' 'migration_required'; exit 10 }
    Write-BootstrapDetection 'completed' 'compatible'
    exit 0
} catch {
    Write-BootstrapDetection 'failed' 'failed' $_
    Write-Output 'installer.bootstrap_migration.detection_failed; no mutation performed'
    exit 1
}
