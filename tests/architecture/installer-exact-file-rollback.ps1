$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'installer\close-owned-processes.ps1') -LibraryOnly
$work = Join-Path $env:PI_SCRATCH_DIR ('exact-rollback-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $work | Out-Null
try {
    Initialize-UpgradeNative
    $file = Join-Path $work 'synthetic-new-host.bin'
    [IO.File]::WriteAllText($file, 'synthetic exact installed artifact')
    $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
    $rejected = $false
    try { [NxToolsUpgradeProcess]::DeleteExactFile($file, ('0' * 64)) } catch { $rejected = $true }
    if (-not $rejected -or -not (Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file).Hash -ne $hash) {
        throw 'Wrong rollback hash deleted or modified an unrelated file.'
    }
    [NxToolsUpgradeProcess]::DeleteExactFile($file, $hash)
    if (Test-Path -LiteralPath $file) { throw 'Exact approved file did not delete through the verified handle.' }
    Write-Host 'PASS exact-handle rollback wrong hash retains file, approved hash deletes only that file (2 isolated assertions).'
} finally { Remove-Item -LiteralPath $work -Recurse -Force }
