param([string]$EvidenceRoot = (Join-Path $env:PI_SCRATCH_DIR 'phase-f-release-stage-20261023-090605'))
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'installer\close-owned-processes.ps1') -LibraryOnly
# Hard guards: no discovery or real termination is permitted in this regression.
function Get-CimInstance { throw 'Forbidden real process discovery in library-only regression.' }
function Stop-UpgradeTargets { param($Targets, $ProtectedRoot) return 0 }
if (!(Get-Command Suspend-UpgradeBrowserHost).Parameters.ContainsKey('NewManifestPath')) {
    throw 'RED: existing suspend owner lacks complete-set predeclared manifest transaction.'
}
$names = @('NxTools.exe','NxTools.PlatformHost.exe','NxTools.Indexer.exe','NxTools.Cli.exe','NxTools.BrowserHost.exe',
    'NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe','NxTools.ShellSelectionHelper.dll',
    'NxTools.ShellSelectionHelper.deps.json','NxTools.ShellSelectionHelper.runtimeconfig.json','NxTools.release.json','NxTools.release.sig')
$work = Join-Path $EvidenceRoot ('installer-set-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
$script:checks = 0
function Assert-Set($Condition, [string]$Label) {
    if (!$Condition) { throw "FAIL $Label" }
    $script:checks++
    Write-Output "PASS $Label"
}
$script:fixtureNumber = 0
function New-SetFixture([string]$Label, [string[]]$Missing = @()) {
    $script:fixtureNumber++
    $area = Join-Path $work ('f' + $script:fixtureNumber)
    $installed = Join-Path $area 'i'
    $package = Join-Path $area 'p'
    New-Item -ItemType Directory -Path $installed,$package | Out-Null
    $hashes = @{}
    foreach ($name in $names) {
        if ($name -notin $Missing) { [IO.File]::WriteAllText((Join-Path $installed $name), ('old fixture ' + $name)) }
        if ($name -notin @('NxTools.release.json','NxTools.release.sig')) {
            [IO.File]::WriteAllText((Join-Path $package $name), ('new fixture ' + $name))
            $hashes[$name] = (Get-FileHash -LiteralPath (Join-Path $package $name)).Hash
        }
    }
    $manifest = Join-Path $package 'NxTools.release.json'
    [IO.File]::WriteAllText($manifest, (@{files=@($hashes.Keys | Sort-Object | ForEach-Object { @{path=$_;sha256=$hashes[$_]} })} | ConvertTo-Json -Depth 5))
    [IO.File]::WriteAllText((Join-Path $package 'NxTools.release.sig'), 'fake detached signature; no trust or native operation')
    return @{ Installed=$installed; Package=$package; Manifest=$manifest; Signature=(Join-Path $package 'NxTools.release.sig'); State=(Join-Path $installed '.NxTools.BrowserHost-upgrade.json'); Missing=$Missing }
}
function Start-Set($Fixture) {
    Suspend-UpgradeBrowserHost $Fixture.Installed $Fixture.State ([Guid]::NewGuid().ToString('N').Substring(0,16)) -NewManifestPath $Fixture.Manifest -NewSignaturePath $Fixture.Signature
}
function Publish-Set($Fixture) {
    foreach ($name in $names) { [IO.File]::Copy((Join-Path $Fixture.Package $name), (Join-Path $Fixture.Installed $name)) }
}
function Assert-OldSet($Fixture, [string[]]$Except = @()) {
    foreach ($name in $names) {
        if ($name -in $Except) { continue }
        $path = Join-Path $Fixture.Installed $name
        if ($name -in $Fixture.Missing) { Assert-Set (!(Test-Path -LiteralPath $path)) "absent old remains absent $name" }
        else { Assert-Set ((Test-Path -LiteralPath $path) -and [IO.File]::ReadAllText($path) -eq ('old fixture ' + $name)) "exact old restored $name" }
    }
}
function Assert-NewSet($Fixture, [string[]]$Except = @()) {
    $mismatches = @()
    foreach ($name in $names) {
        if ($name -in $Except) { continue }
        $path = Join-Path $Fixture.Installed $name
        $matches = (Test-Path -LiteralPath $path -PathType Leaf) -and
            (Get-FileHash -LiteralPath $path).Hash -eq (Get-FileHash -LiteralPath (Join-Path $Fixture.Package $name)).Hash
        if ($matches) { Assert-Set $true "complete new set preserved $name" }
        else { $mismatches += $name; Write-Output "FAIL complete new set preserved $name" }
    }
    if ($mismatches.Count) { throw "FAIL complete new set changed after cleanup: $($mismatches -join ',')" }
}
# Fault injection wraps only the file move; the production owner, journal and rollback execute.
$script:realMove = ${function:Move-UpgradeFile}
$script:realRestore = ${function:Restore-UpgradeFile}
$script:realRemove = ${function:Remove-UpgradeExactFile}
$script:realJournal = ${function:Write-UpgradeJournal}
$script:realRemoveJournal = ${function:Remove-UpgradeJournal}
try {
    # Concrete commit-cleanup regression: first backup is really deleted through
    # DeleteExactFile, then the second deletion fails. No new member may roll back.
    $f = New-SetFixture 'partial-cleanup'
    Start-Set $f
    Publish-Set $f
    $script:cleanupCalls = 0
    $script:firstBackupDeleted = $false
    function Remove-UpgradeExactFile([string]$Path, [string]$ExpectedHash) {
        $before = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
        $member = $before.entries | Where-Object backup_name -eq ([IO.Path]::GetFileName($Path))
        if ($Path.EndsWith('.upgrade-old') -and ($before.phase -ne 'committing' -or !$member.cleanup_attempted)) { throw 'Cleanup lacks durable commit/attempted record.' }
        if ($Path.EndsWith('.upgrade-old')) {
            $script:cleanupCalls++
            if ($script:cleanupCalls -eq 2) { throw 'injected second backup DeleteExactFile failure' }
        }
        & $script:realRemove $Path $ExpectedHash
        if ($script:cleanupCalls -eq 1 -and $Path.EndsWith('.upgrade-old')) { $script:firstBackupDeleted = !(Test-Path -LiteralPath $Path) }
    }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Set-Item Function:Remove-UpgradeExactFile $script:realRemove
    Assert-Set ($failed -and $script:firstBackupDeleted -and $script:cleanupCalls -eq 2) 'second cleanup failure follows actual first backup deletion'
    Assert-NewSet $f
    Assert-Set (Test-Path -LiteralPath $f.State) 'cleanup failure retains journal'
    $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
    Assert-Set ($record.phase -eq 'committing' -and $record.entries[0].cleanup_attempted -and $record.entries[1].cleanup_attempted) 'partial cleanup retains durable attempted commit decision'
    $journalHash = (Get-FileHash -LiteralPath $f.State).Hash
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and (Get-FileHash -LiteralPath $f.State).Hash -eq $journalHash) 'rollback refused at committing without changing journal'
    Assert-NewSet $f
    $tampered = Join-Path $f.Installed 'NxTools.Cli.exe'
    [IO.File]::WriteAllText($tampered, 'foreign new member must survive')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Assert-Set ($failed -and [IO.File]::ReadAllText($tampered) -eq 'foreign new member must survive' -and (Test-Path -LiteralPath $f.State)) 'resume revalidates entire new set before any cleanup'
    Assert-NewSet $f @('NxTools.Cli.exe')
    [IO.File]::Copy((Join-Path $f.Package 'NxTools.Cli.exe'), $tampered, $true)
    $foreignBackup = Join-Path $f.Installed $record.entries[1].backup_name
    [IO.File]::WriteAllText($foreignBackup, 'foreign backup must survive cleanup')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Assert-Set ($failed -and [IO.File]::ReadAllText($foreignBackup) -eq 'foreign backup must survive cleanup' -and (Test-Path -LiteralPath $f.State)) 'resume refuses foreign backup and keeps journal'
    Assert-NewSet $f
    [IO.File]::WriteAllText($foreignBackup, ('old fixture ' + $record.entries[1].name))
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    Assert-NewSet $f
    Assert-Set (!(Test-Path -LiteralPath $f.State) -and @(Get-ChildItem -LiteralPath $f.Installed -Filter '*.upgrade-old' -Force).Count -eq 0) 'partial cleanup retry removes remaining exact backups and clears journal'
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    Assert-NewSet $f
    $f = New-SetFixture 'cleanup-delete-then-throw'
    Start-Set $f
    Publish-Set $f
    $script:cleanupCalls = 0
    function Remove-UpgradeExactFile([string]$Path, [string]$ExpectedHash) {
        & $script:realRemove $Path $ExpectedHash
        $script:cleanupCalls++
        if ($script:cleanupCalls -eq 2) { throw 'injected second backup deleted then throw' }
    }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Set-Item Function:Remove-UpgradeExactFile $script:realRemove
    Assert-Set ($failed -and $script:cleanupCalls -eq 2 -and (Test-Path -LiteralPath $f.State)) 'backup deletion mutation-then-throw retains resumable journal'
    Assert-NewSet $f
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    Assert-NewSet $f
    Assert-Set (!(Test-Path -LiteralPath $f.State)) 'retry accepts only durably attempted missing backups'
    $f = New-SetFixture 'foreign-backup-at-delete'
    Start-Set $f
    Publish-Set $f
    $script:foreignCleanupPath = ''
    function Remove-UpgradeExactFile([string]$Path, [string]$ExpectedHash) {
        $script:foreignCleanupPath = $Path
        [IO.File]::WriteAllText($Path, 'foreign backup at exact deletion boundary')
        & $script:realRemove $Path $ExpectedHash
    }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Set-Item Function:Remove-UpgradeExactFile $script:realRemove
    Assert-Set ($failed -and [IO.File]::ReadAllText($script:foreignCleanupPath) -eq 'foreign backup at exact deletion boundary' -and (Test-Path -LiteralPath $f.State)) 'same-handle exact hash rejects foreign backup after preflight'
    Assert-NewSet $f
    # Actual durable publication can itself mutate then throw: no cleanup has
    # run, but resumption must honor the published point of no return.
    $f = New-SetFixture 'commit-publication-throw'
    Start-Set $f
    Publish-Set $f
    function Write-UpgradeJournal([string]$StateFile, $State) {
        & $script:realJournal $StateFile $State
        if ($State.phase -eq 'committing') { throw 'injected durable publication then throw' }
    }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Set-Item Function:Write-UpgradeJournal $script:realJournal
    $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
    Assert-Set ($failed -and $record.phase -eq 'committing' -and @($record.entries | Where-Object cleanup_attempted).Count -eq 0) 'published committing survives mutation-then-throw before first cleanup'
    Assert-NewSet $f
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    Assert-NewSet $f
    Assert-Set (!(Test-Path -LiteralPath $f.State)) 'real published commit with intact backups resumes cleanup'
    # A retained committed journal is not proof by itself. Require the complete
    # new set plus attempted-and-absent evidence for every old backup.
    foreach ($phase in @('committing','committed')) {
        $f = New-SetFixture ('missing-cleanup-evidence-' + $phase)
        Start-Set $f
        Publish-Set $f
        $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
        $record.phase = $phase
        Remove-UpgradeExactFile (Join-Path $f.Installed $record.entries[0].backup_name) $record.entries[0].old_sha256
        Write-UpgradeJournal $f.State $record
        $failed = $false
        try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
        Assert-Set ($failed -and (Test-Path -LiteralPath $f.State)) "missing durable cleanup attempt rejects false success $phase"
        Assert-NewSet $f
    }
    $f = New-SetFixture 'journal-delete-failure'
    Start-Set $f
    Publish-Set $f
    function Remove-UpgradeJournal([string]$StateFile) { throw 'injected final journal deletion failure' }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Set-Item Function:Remove-UpgradeJournal $script:realRemoveJournal
    $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
    Assert-Set ($failed -and $record.phase -eq 'committed' -and @($record.entries | Where-Object { !$_.cleanup_attempted }).Count -eq 0) 'journal deletion failure retains completed cleanup evidence'
    Assert-NewSet $f
    $journalHash = (Get-FileHash -LiteralPath $f.State).Hash
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and (Get-FileHash -LiteralPath $f.State).Hash -eq $journalHash) 'rollback refused at committed without changing journal'
    Assert-NewSet $f
    $tampered = Join-Path $f.Installed 'NxTools.ArchiveWorker.exe'
    [IO.File]::WriteAllText($tampered, 'foreign committed member')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Assert-Set ($failed -and (Test-Path -LiteralPath $f.State) -and [IO.File]::ReadAllText($tampered) -eq 'foreign committed member') 'committed retry cannot clear journal with tampered new set'
    Assert-NewSet $f @('NxTools.ArchiveWorker.exe')
    [IO.File]::Copy((Join-Path $f.Package 'NxTools.ArchiveWorker.exe'), $tampered, $true)
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    Assert-Set (!(Test-Path -LiteralPath $f.State)) 'verified committed retry clears retained journal'
    Assert-NewSet $f
    foreach ($name in $names) {
        $f = New-SetFixture ('move-throw-' + $name)
        $script:failMember = $name
        $script:mutatedMember = $false
        function Move-UpgradeFile([string]$Source, [string]$Destination) {
            $before = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
            $member = $before.entries | Where-Object name -eq ([IO.Path]::GetFileName($Source))
            if (!$member.attempted -or @($before.entries).Count -ne $names.Count) { throw 'Missing complete pre-mutation attempted journal.' }
            & $script:realMove $Source $Destination
            if ([IO.Path]::GetFileName($Source) -eq $script:failMember) {
                $script:mutatedMember = !(Test-Path -LiteralPath $Source) -and (Test-Path -LiteralPath $Destination)
                throw 'injected mutation-then-throw'
            }
        }
        $failed = $false
        try { Start-Set $f } catch { $failed = $true }
        Assert-Set ($failed -and $script:mutatedMember) "mutation-then-throw rejected $name"
        Assert-OldSet $f
        Assert-Set (Test-Path -LiteralPath $f.State) "failure retains durable journal $name"
        $journal = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
        Assert-Set (($journal.entries | Where-Object name -eq $name).attempted -and $journal.failure_count -gt 0) "attempted recorded before move $name"
    }
    Set-Item Function:Move-UpgradeFile $script:realMove
    # Inno's replacement loop can fail AFTER publishing any member. The same
    # transaction must recover all original files, including members not copied.
    foreach ($faultMember in @($names[5..9])) {
        $f = New-SetFixture ('publish-throw-' + $faultMember)
        Start-Set $f
        $failed = $false
        try {
            foreach ($name in $names) {
                [IO.File]::Copy((Join-Path $f.Package $name), (Join-Path $f.Installed $name))
                if ($name -eq $faultMember) { throw 'fake Inno mutation-then-throw' }
            }
        } catch { $failed = $true; Complete-UpgradeBrowserHost $f.Installed $f.State }
        Assert-Set $failed "partial publication failure $faultMember"
        Assert-OldSet $f
    }
    $f = New-SetFixture 'full-rollback'
    Start-Set $f
    Publish-Set $f
    Complete-UpgradeBrowserHost $f.Installed $f.State
    Assert-OldSet $f
    $f = New-SetFixture 'missing-old' @('NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.dll','NxTools.release.sig')
    Start-Set $f
    $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
    Assert-Set (@($record.entries | Where-Object { !$_.existed }).Count -eq 3) 'old absence captured before installation'
    Publish-Set $f
    Complete-UpgradeBrowserHost $f.Installed $f.State
    Assert-OldSet $f
    $f = New-SetFixture 'foreign'
    Start-Set $f
    Publish-Set $f
    $foreign = Join-Path $f.Installed 'NxTools.ShellSelectionHelper.dll'
    [IO.File]::WriteAllText($foreign, 'foreign must survive')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and [IO.File]::ReadAllText($foreign) -eq 'foreign must survive') 'foreign replacement retained and rollback fails closed'
    Assert-OldSet $f @('NxTools.ShellSelectionHelper.dll')
    Assert-Set (Test-Path -LiteralPath $f.State) 'foreign conflict retains journal'
    $f = New-SetFixture 'wrong-newhash' @('NxTools.ArchiveWorker.exe')
    $manifest = Get-Content -LiteralPath $f.Manifest -Raw | ConvertFrom-Json
    ($manifest.files | Where-Object path -eq 'NxTools.ArchiveWorker.exe').sha256 = '0' * 64
    [IO.File]::WriteAllText($f.Manifest, ($manifest | ConvertTo-Json -Depth 5))
    Start-Set $f
    Publish-Set $f
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
    Assert-Set ($failed -and (Test-Path -LiteralPath (Join-Path $f.Installed 'NxTools.ArchiveWorker.exe'))) 'wrong predeclared new hash never grants deletion authority'
    Assert-OldSet $f @('NxTools.ArchiveWorker.exe')
    Assert-Set (Test-Path -LiteralPath $f.State) 'incorrect newhash retains failed journal'
    $f = New-SetFixture 'restore-throw'
    Start-Set $f
    Publish-Set $f
    $script:restoreAttempts = @()
    function Restore-UpgradeFile([string]$Source, [string]$Destination) {
        $script:restoreAttempts += [IO.Path]::GetFileName($Destination)
        if ([IO.Path]::GetFileName($Destination) -eq 'NxTools.ShellSelectionHelper.dll') { throw 'injected restore failure' }
        & $script:realRestore $Source $Destination
    }
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $_.Exception.ToString() -match 'compensation' }
    Assert-Set ($failed -and $script:restoreAttempts.Count -eq $names.Count) 'one restore fails but every other attempted member is compensated'
    Assert-Set (($script:restoreAttempts -join '|') -eq (($names[($names.Count-1)..0]) -join '|')) 'compensation visits every member in reverse order'
    Assert-OldSet $f @('NxTools.ShellSelectionHelper.dll')
    Assert-Set (Test-Path -LiteralPath $f.State) 'partial restore retains journal'
    Set-Item Function:Restore-UpgradeFile $script:realRestore
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and (Test-Path -LiteralPath $f.State)) 'partial recovery retry cannot falsely finalize successfully'
    Assert-OldSet $f
    $f = New-SetFixture 'commit'
    Start-Set $f
    Publish-Set $f
    Complete-UpgradeBrowserHost $f.Installed $f.State -Committed
    foreach ($name in $names) { Assert-Set ((Get-FileHash -LiteralPath (Join-Path $f.Installed $name)).Hash -eq (Get-FileHash -LiteralPath (Join-Path $f.Package $name)).Hash) "commit preserves new $name" }
    Assert-Set (!(Test-Path -LiteralPath $f.State)) 'verified commit clears journal'
    foreach ($phase in @('preparing','rolling_back')) {
        $f = New-SetFixture ('interrupted-' + $phase)
        Start-Set $f
        $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
        $record.phase = $phase
        Write-UpgradeJournal $f.State $record
        $failed = $false
        try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed } catch { $failed = $true }
        Assert-Set ($failed -and (Test-Path -LiteralPath $f.State)) "interrupted phase retains journal and rejects commit $phase"
    }
    $f = New-SetFixture 'pending-only'
    [IO.File]::WriteAllText(($f.State + '.pending'), 'interrupted initial journal')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and (Test-Path -LiteralPath ($f.State + '.pending'))) 'interrupted initial journal cannot vanish as success'
    Assert-OldSet $f
    $f = New-SetFixture 'oldhash-tamper'
    Start-Set $f
    Publish-Set $f
    $record = Get-Content -LiteralPath $f.State -Raw | ConvertFrom-Json
    $entry = $record.entries | Where-Object name -eq 'NxTools.ShellSelectionHelper.dll'
    $backup = Join-Path $f.Installed $entry.backup_name
    [IO.File]::WriteAllText($backup, 'foreign backup must survive')
    $failed = $false
    try { Complete-UpgradeBrowserHost $f.Installed $f.State } catch { $failed = $true }
    Assert-Set ($failed -and [IO.File]::ReadAllText($backup) -eq 'foreign backup must survive') 'oldhash mismatch prevents restoration/deletion of foreign backup'
    Assert-OldSet $f @('NxTools.ShellSelectionHelper.dll')
    $f = New-SetFixture 'all-old-missing' $names
    Start-Set $f
    Publish-Set $f
    Complete-UpgradeBrowserHost $f.Installed $f.State
    Assert-OldSet $f
    $f = New-SetFixture 'protected-approval'
    $approval = @{ServicePath=('"' + (Join-Path $f.Installed 'NxTools.Indexer.exe') + '" --service'); DesktopPath=(Join-Path $f.Installed 'NxTools.exe'); Hashes=@{}; ArchiveHashesJson=''}
    foreach ($name in @('NxTools.exe','NxTools.PlatformHost.exe','NxTools.Indexer.exe','NxTools.BrowserHost.exe')) {
        $approval.Hashes[$name] = (Get-FileHash -LiteralPath (Join-Path $f.Installed $name)).Hash
    }
    $archiveHashes = @{}
    foreach ($name in @($names[5..9])) { $archiveHashes[$name] = (Get-FileHash -LiteralPath (Join-Path $f.Installed $name)).Hash }
    $approval.ArchiveHashesJson = $archiveHashes | ConvertTo-Json
    $targets = Get-UpgradeTargets $f.Package $f.Installed $approval
    Assert-Set (!$targets.ContainsKey((Join-Path $f.Installed 'NxTools.Indexer.exe'))) 'protected indexer service never admitted for termination'
    foreach ($name in @('NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe')) { Assert-Set ($targets.ContainsKey((Join-Path $f.Installed $name))) "complete protected approvals admit exact $name" }
    foreach ($name in @('NxTools.exe','NxTools.PlatformHost.exe','NxTools.Indexer.exe','NxTools.BrowserHost.exe') + @($names[5..9])) {
        $path = Join-Path $f.Installed $name
        [IO.File]::WriteAllText($path, 'tampered protected fixture')
        $targets = Get-UpgradeTargets $f.Package $f.Installed $approval
        foreach ($newExe in @('NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe')) { Assert-Set (!$targets.ContainsKey((Join-Path $f.Installed $newExe))) "protected $newExe excluded after $name mismatch" }
        [IO.File]::WriteAllText($path, ('old fixture ' + $name))
    }
    foreach ($json in @('', '{}', '{invalid', ('x' * 4097))) {
        $approval.ArchiveHashesJson = $json
        $targets = Get-UpgradeTargets $f.Package $f.Installed $approval
        Assert-Set (!$targets.ContainsKey((Join-Path $f.Installed 'NxTools.ArchiveWorker.exe'))) 'missing/malformed/oversized archive approval fails closed'
    }
    # Original single-host API remains available, without any process stop.
    $f = New-SetFixture 'legacy'
    Suspend-UpgradeBrowserHost $f.Installed $f.State 'legacy-file-fixture'
    Complete-UpgradeBrowserHost $f.Installed $f.State
    Assert-OldSet $f
    Write-Output "PASS installer-set library-only: $script:checks assertions; no installer/process/service/registry/COM/Explorer. evidence=$work"
} finally {
    Set-Item Function:Move-UpgradeFile $script:realMove
    Set-Item Function:Restore-UpgradeFile $script:realRestore
    Set-Item Function:Remove-UpgradeExactFile $script:realRemove
    Set-Item Function:Write-UpgradeJournal $script:realJournal
    Set-Item Function:Remove-UpgradeJournal $script:realRemoveJournal
}
