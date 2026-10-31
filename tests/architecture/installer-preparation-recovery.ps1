param(
    [ValidateSet('all','committed','committing')][string]$Scenario = 'all',
    [string]$EvidenceRoot = $env:PI_SCRATCH_DIR
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$helperPath = Join-Path $root 'installer\close-owned-processes.ps1'
. $helperPath -LibraryOnly
function Get-CimInstance { throw 'Forbidden process discovery in preparation regression.' }
function Stop-UpgradeTargets { param($Targets,$ProtectedRoot) return 0 }
# This is the exact default preparation call reached by Inno's helper invocation, not a test-chosen Finalize.
$text = [IO.File]::ReadAllText($helperPath)
$match = [regex]::Match($text, '(?m)^\s*((?:Suspend|Prepare)-UpgradeBrowserHost \$InstallDirectory \$StatePath \$CorrelationId -NewManifestPath \$NewManifestPath -NewSignaturePath \$NewSignaturePath)\s*$')
if (!$match.Success) { throw 'Actual installer preparation entry not found.' }
$script:prepareCall = [scriptblock]::Create($match.Groups[1].Value)
$work = Join-Path $env:PI_SCRATCH_DIR ('ipr-' + [Guid]::NewGuid().ToString('N').Substring(0,12))
New-Item -ItemType Directory -Path $work | Out-Null
$names = @(Get-UpgradeOwnedNames)
$script:checks = 0
$script:number = 0
function Check($Condition, [string]$Label) {
    if (!$Condition) { throw "FAIL $Label" }
    $script:checks++; Write-Output "PASS $Label"
}
function New-Package([string]$Directory, [string]$Version) {
    New-Item -ItemType Directory -Path $Directory | Out-Null
    $files = @()
    foreach ($name in $names) {
        if ($name -in @('NxTools.release.json','NxTools.release.sig')) { continue }
        [IO.File]::WriteAllText((Join-Path $Directory $name), "$Version $name")
        $files += @{path=$name;sha256=(Get-FileHash -LiteralPath (Join-Path $Directory $name)).Hash}
    }
    [IO.File]::WriteAllText((Join-Path $Directory 'NxTools.release.json'), (@{files=$files}|ConvertTo-Json -Depth 5))
    [IO.File]::WriteAllText((Join-Path $Directory 'NxTools.release.sig'), "$Version fake signature; no signing")
}
function New-Fixture {
    $script:number++
    $area = Join-Path $work ([string]$script:number)
    $installed = Join-Path $area 'installed'
    $v2 = Join-Path $area 'v2'; $v3 = Join-Path $area 'v3'
    New-Item -ItemType Directory -Path $installed | Out-Null
    foreach ($name in $names) { [IO.File]::WriteAllText((Join-Path $installed $name), "old $name") }
    New-Package $v2 'v2'; New-Package $v3 'v3'
    return @{Installed=$installed;State=(Join-Path $installed '.NxTools.BrowserHost-upgrade.json');V2=$v2;V3=$v3}
}
function Invoke-InnoPreparation($Fixture, [string]$Operation, [string]$Package) {
    $InstallDirectory=$Fixture.Installed; $StatePath=$Fixture.State; $CorrelationId=$Operation
    $NewManifestPath=Join-Path $Package 'NxTools.release.json'; $NewSignaturePath=Join-Path $Package 'NxTools.release.sig'
    & $script:prepareCall
}
function Publish($Fixture, [string]$Package) {
    foreach ($name in $names) { [IO.File]::Copy((Join-Path $Package $name), (Join-Path $Fixture.Installed $name)) }
}
function Check-Bytes([string]$Path,[string]$Expected,[string]$Label) {
    Check ((Test-Path -LiteralPath $Path -PathType Leaf) -and (Get-FileHash -LiteralPath $Path).Hash -eq (Get-FileHash -LiteralPath $Expected).Hash) $Label
}
$script:realRemove=${function:Remove-UpgradeExactFile}
$script:realRemoveJournal=${function:Remove-UpgradeJournal}
$script:realMove=${function:Move-UpgradeFile}
$script:realRestore=${function:Restore-UpgradeFile}
try {
    foreach ($phase in $(if ($Scenario -eq 'all') { @('committed','committing') } else { @($Scenario) })) {
        $f=New-Fixture
        Invoke-InnoPreparation $f 'first-attempt' $f.V2 | Out-Null
        Publish $f $f.V2
        if ($phase -eq 'committed') {
            function Remove-UpgradeJournal { param($StateFile) throw 'fixture journal delete failed after commit' }
        } else {
            $script:removeCalls=0
            function Remove-UpgradeExactFile { param($Path,$ExpectedHash)
                $script:removeCalls++
                if ($script:removeCalls -eq 2) { throw 'fixture second backup cleanup failed' }
                & $script:realRemove $Path $ExpectedHash
            }
        }
        $failed=$false
        try { Complete-UpgradeBrowserHost $f.Installed $f.State -Committed | Out-Null } catch { $failed=$true }
        Set-Item Function:Remove-UpgradeJournal $script:realRemoveJournal
        Set-Item Function:Remove-UpgradeExactFile $script:realRemove
        $old=Get-Content -LiteralPath $f.State -Raw|ConvertFrom-Json
        Check ($failed -and $old.phase -eq $phase) "first attempt retains $phase journal"
        foreach ($name in $names) { Check-Bytes (Join-Path $f.Installed $name) (Join-Path $f.V2 $name) "complete v2 retained before second $phase preparation $name" }
        $script:beforeFresh=$false
        function Move-UpgradeFile { param($Source,$Destination)
            if (!$script:beforeFresh) {
                $fresh=Get-Content -LiteralPath $f.State -Raw|ConvertFrom-Json
                Check ($fresh.operation -eq 'second-attempt') 'old recovery completed before new transaction journal'
                foreach ($name in $names) { Check-Bytes (Join-Path $f.Installed $name) (Join-Path $f.V2 $name) "no postcommit rollback before fresh suspension $name" }
                $script:beforeFresh=$true
            }
            & $script:realMove $Source $Destination
        }
        # Crucial: second invocation uses the same production preparation entry; never manual Complete here.
        Invoke-InnoPreparation $f 'second-attempt' $f.V3 | Out-Null
        Set-Item Function:Move-UpgradeFile $script:realMove
        $fresh=Get-Content -LiteralPath $f.State -Raw|ConvertFrom-Json
        Check ($script:beforeFresh -and $fresh.phase -eq 'suspended' -and $fresh.operation -eq 'second-attempt') "second $phase preparation admitted new transaction"
        foreach ($entry in $fresh.entries) {
            Check-Bytes (Join-Path $f.Installed $entry.backup_name) (Join-Path $f.V2 $entry.name) "new transaction retains exact v2 backup $($entry.name)"
        }
    }
    if ($Scenario -eq 'all') {
        # Valid interrupted precommit state is compensated by the same owner before fresh suspension.
        $f=New-Fixture
        Invoke-InnoPreparation $f 'interrupted-before-commit' $f.V2 | Out-Null
        Publish $f $f.V2
        Invoke-InnoPreparation $f 'after-precommit-recovery' $f.V3 | Out-Null
        $fresh=Get-Content $f.State -Raw|ConvertFrom-Json
        Check ($fresh.operation -eq 'after-precommit-recovery' -and $fresh.phase -eq 'suspended') 'precommit recovery admits fresh transaction'
        foreach ($entry in $fresh.entries) { Check ([IO.File]::ReadAllText((Join-Path $f.Installed $entry.backup_name)) -eq "old $($entry.name)") "precommit exact old bytes recovered $($entry.name)" }
        # A restore failure still attempts all other items and cannot admit a new transaction.
        $f=New-Fixture; Invoke-InnoPreparation $f 'restore-failure' $f.V2 | Out-Null; Publish $f $f.V2
        $script:restores=@()
        function Restore-UpgradeFile { param($Source,$Destination)
            $script:restores += [IO.Path]::GetFileName($Destination)
            if ([IO.Path]::GetFileName($Destination) -eq 'NxTools.ShellSelectionHelper.dll') { throw 'fixture restore denied' }
            & $script:realRestore $Source $Destination
        }
        $failed=$false; try { Invoke-InnoPreparation $f 'must-not-start' $f.V3 | Out-Null } catch { $failed=$true }
        Set-Item Function:Restore-UpgradeFile $script:realRestore
        $record=Get-Content $f.State -Raw|ConvertFrom-Json
        Check ($failed -and $script:restores.Count -eq $names.Count -and $record.operation -eq 'restore-failure') 'failed recovery attempts all reverse restores and blocks fresh transaction'
        Invoke-InnoPreparation $f 'retry-after-restore' $f.V3 | Out-Null
        Check ((Get-Content $f.State -Raw|ConvertFrom-Json).operation -eq 'retry-after-restore') 'fully recovered precommit failure can retry without manual journal deletion'
        # New manifest hashes must never grant deletion rights for an OLD transaction's foreign file.
        $f=New-Fixture; Invoke-InnoPreparation $f 'old-authority' $f.V2 | Out-Null; Publish $f $f.V2
        $foreign=Join-Path $f.Installed 'NxTools.ShellSelectionHelper.dll'
        [IO.File]::WriteAllText($foreign,'foreign bytes remain')
        $newManifest=Join-Path $f.V3 'NxTools.release.json'
        $newRecord=Get-Content $newManifest -Raw|ConvertFrom-Json
        ($newRecord.files|Where-Object path -eq 'NxTools.ShellSelectionHelper.dll').sha256=(Get-FileHash $foreign).Hash
        [IO.File]::WriteAllText($newManifest,($newRecord|ConvertTo-Json -Depth 5))
        $failed=$false;try { Invoke-InnoPreparation $f 'new-hashes-cannot-authorize-old-delete' $f.V3 | Out-Null } catch { $failed=$true }
        Check ($failed -and [IO.File]::ReadAllText($foreign) -eq 'foreign bytes remain' -and (Get-Content $f.State -Raw|ConvertFrom-Json).operation -eq 'old-authority') 'new manifest cannot authorize deleting old transaction foreign file'
        foreach ($damage in @('invalid-json','pending')) {
            $f=New-Fixture; Invoke-InnoPreparation $f 'damaged-original' $f.V2 | Out-Null
            if ($damage -eq 'invalid-json') { [IO.File]::WriteAllText($f.State,'{invalid') }
            else { [IO.File]::WriteAllText(($f.State+'.pending'),'unproved publication') }
            $before=(Get-FileHash $f.State).Hash
            $failed=$false;try { Invoke-InnoPreparation $f 'must-not-start' $f.V3 | Out-Null } catch { $failed=$true }
            Check ($failed -and (Get-FileHash $f.State).Hash -eq $before) "unproved $damage journal refused and preserved"
        }
        foreach ($mutation in @('root','schema','phase','operation')) {
            $f=New-Fixture; Invoke-InnoPreparation $f 'valid-original' $f.V2 | Out-Null
            $record=Get-Content $f.State -Raw|ConvertFrom-Json
            switch ($mutation) {
                'root' { $record.root=$f.V3 }
                'schema' { $record.schema=999 }
                'phase' { $record.phase='unknown' }
                'operation' { $record.operation='../foreign' }
            }
            [IO.File]::WriteAllText($f.State,($record|ConvertTo-Json -Depth 8))
            $before=(Get-FileHash $f.State).Hash
            $failed=$false;try { Invoke-InnoPreparation $f 'must-not-start' $f.V3 | Out-Null } catch { $failed=$true }
            Check ($failed -and (Get-FileHash $f.State).Hash -eq $before) "mismatched $mutation journal rejects and remains unchanged"
        }
    }
    [pscustomobject]@{assertions=$script:checks;scenario=$Scenario;entry=$match.Groups[1].Value;work=$work;system_mutations=0} |
        ConvertTo-Json | Set-Content (Join-Path $EvidenceRoot ('installer-preparation-results-'+[IO.Path]::GetFileName($work)+'.json')) -Encoding UTF8
    Write-Output "PASS installer preparation recovery: $script:checks assertions; two real preparation entries, LibraryOnly/fake process boundaries; evidence=$work"
} finally {
    Set-Item Function:Remove-UpgradeExactFile $script:realRemove
    Set-Item Function:Remove-UpgradeJournal $script:realRemoveJournal
    Set-Item Function:Move-UpgradeFile $script:realMove
    Set-Item Function:Restore-UpgradeFile $script:realRestore
}
