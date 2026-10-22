$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'scripts\reserve-release-sequence.ps1') -LibraryOnly
$work = Join-Path $env:PI_SCRATCH_DIR ('release-sequence-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $work | Out-Null
try {
    $key = Join-Path $work 'synthetic-key.marker'
    [IO.File]::WriteAllText($key, 'not a PEM; sequence test never reads private material')
    Reserve-ReleaseSequence $key 10 $work
    foreach ($rejectedSequence in @(9,10)) {
        $rejected = $false
        try { Reserve-ReleaseSequence $key $rejectedSequence $work } catch { $rejected = $true }
        if (-not $rejected) { throw 'Duplicate/downgrade publisher sequence was accepted.' }
    }
    [IO.File]::WriteAllText((Join-Path $work 'NxTools.release.json'), '{"release_sequence":15}')
    $rejected = $false
    try { Reserve-ReleaseSequence $key 14 $work } catch { $rejected = $true }
    if (-not $rejected) { throw 'Previous output sequence was ignored.' }
    Reserve-ReleaseSequence $key 16 $work
    $ledger = Get-Content -LiteralPath ($key + '.release-sequence.json') -Raw | ConvertFrom-Json
    if ($ledger.sequence -ne 16) { throw 'Durable publisher sequence not recorded.' }
    Write-Host 'PASS publisher sequence reserve, duplicate/downgrade rejection and prior-package floor (5 isolated assertions).'
} finally { Remove-Item -LiteralPath $work -Recurse -Force }
