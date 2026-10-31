[CmdletBinding()]
param(
    [string]$SigningKeyPath,
    [long]$ReleaseSequence,
    [string]$PackageDirectory,
    [switch]$LibraryOnly
)
$ErrorActionPreference = 'Stop'

function Reserve-ReleaseSequence([string]$KeyPath, [long]$Sequence, [string]$PreviousPackage) {
    if ($Sequence -le 0 -or -not (Test-Path -LiteralPath $KeyPath -PathType Leaf)) {
        throw 'An existing explicit signing key and positive release sequence are required.'
    }
    $ledgerPath = [IO.Path]::GetFullPath($KeyPath) + '.release-sequence.json'
    $lockPath = $ledgerPath + '.lock'
    $sequenceLock = $null
    try {
        for ($attempt = 0; $attempt -lt 100; $attempt++) {
            try {
                $sequenceLock = [IO.File]::Open($lockPath, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
                break
            } catch [IO.IOException] {
                if ($attempt -eq 99) { throw 'Publisher sequence ledger is busy.' }
                Start-Sleep -Milliseconds 50
            }
        }
        $lastSequence = [long]0
        if (Test-Path -LiteralPath $ledgerPath -PathType Leaf) {
            $ledger = Get-Content -LiteralPath $ledgerPath -Raw -Encoding UTF8 | ConvertFrom-Json
            if (-not $ledger.sequence -or [long]$ledger.sequence -le 0) { throw 'Publisher sequence ledger is invalid; do not reset it.' }
            $lastSequence = [long]$ledger.sequence
        }
        if ($PreviousPackage) {
            $oldManifest = Join-Path $PreviousPackage 'NxTools.release.json'
            if (Test-Path -LiteralPath $oldManifest -PathType Leaf) {
                # This can only raise the rejection floor; it cannot authorize an update.
                if ((Get-Item -LiteralPath $oldManifest).Length -gt 1048576) { throw 'Previous release manifest is oversized.' }
                $previous = Get-Content -LiteralPath $oldManifest -Raw -Encoding UTF8 | ConvertFrom-Json
                $lastSequence = [Math]::Max($lastSequence, [long]$previous.release_sequence)
            }
        }
        if ($Sequence -le $lastSequence) { throw 'ReleaseSequence must exceed all reserved and previously packaged sequences.' }
        $temporary = $ledgerPath + '.' + [Guid]::NewGuid().ToString('N') + '.tmp'
        try {
            $bytes = [Text.Encoding]::UTF8.GetBytes(('{"sequence":' + $Sequence.ToString([Globalization.CultureInfo]::InvariantCulture) + '}'))
            $stream = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) } finally { $stream.Dispose() }
            if (Test-Path -LiteralPath $ledgerPath) {
                $backupLedger = $ledgerPath + '.' + [Guid]::NewGuid().ToString('N') + '.old'
                [IO.File]::Replace($temporary, $ledgerPath, $backupLedger)
                Remove-Item -LiteralPath $backupLedger -Force
            }
            else { [IO.File]::Move($temporary, $ledgerPath) }
        } finally { if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force } }
        Write-Host ('Reserved authenticated release sequence ' + $Sequence + ' before build; failures never reuse it.')
    } finally { if ($sequenceLock) { $sequenceLock.Dispose() } }
}

if (-not $LibraryOnly) { Reserve-ReleaseSequence $SigningKeyPath $ReleaseSequence $PackageDirectory }
