$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$installerText = Get-Content (Join-Path $root 'installer\nxtools-next.iss') -Raw
if ($installerText -notmatch 'function PrepareToInstall' -or
    $installerText -notmatch 'ExtractTemporaryFile\(''close-owned-processes.ps1''\)' -or
    $installerText -notmatch 'dontcopy' -or
    $installerText -notmatch 'ShellExec\(''runas''' -or
    $installerText -notmatch 'ResultCode = 5' -or
    $installerText -notmatch 'ewWaitUntilTerminated') {
    throw 'Installer must close owned old instances before copying files, retry access denied elevated, and fail closed.'
}
. (Join-Path $root 'installer\close-owned-processes.ps1') -LibraryOnly
$scratchRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$fixtureRoot = Join-Path $scratchRoot ('installer-upgrade-' + [Guid]::NewGuid().ToString('N'))
$installed = Join-Path $fixtureRoot 'installed'
$protected = Join-Path $fixtureRoot 'protected'
$unrelated = Join-Path $fixtureRoot 'unrelated'
$fixtureProcesses = @()
$checks = 0
function Assert-Upgrade($Condition, [string]$Label) {
    if (!$Condition) { throw "FAIL $Label" }
    $script:checks++
    Write-Host "PASS $Label"
}
try {
    New-Item -ItemType Directory -Path $installed,$protected,$unrelated | Out-Null
    $approval = @{ServicePath='"' + (Join-Path $protected 'NxTools.Indexer.exe') + '" --service';DesktopPath=(Join-Path $protected 'NxTools.exe');Hashes=@{}}
    foreach ($name in @('NxTools.exe','NxTools.PlatformHost.exe','NxTools.Indexer.exe','NxTools.BrowserHost.exe')) {
        $fixtureFile = Join-Path $protected $name
        [IO.File]::WriteAllText($fixtureFile, 'synthetic old ' + $name)
        $approval.Hashes[$name] = (Get-FileHash $fixtureFile).Hash
    }
    $targets = Get-UpgradeTargets $installed $protected $approval
    Assert-Upgrade ($targets.ContainsKey((Join-Path $installed 'NxTools.exe'))) 'installed exact path admitted'
    Assert-Upgrade ($targets.ContainsKey((Join-Path $protected 'NxTools.exe'))) 'approved old protected desktop admitted despite different new hash'
    Assert-Upgrade (!$targets.ContainsKey((Join-Path $protected 'NxTools.Indexer.exe'))) 'indexer service never terminated'
    Assert-Upgrade (!$targets.ContainsKey((Join-Path $unrelated 'NxTools.exe'))) 'same-name unrelated path excluded'
    $approval.Hashes['NxTools.PlatformHost.exe'] = 'invalid'
    $rejected = Get-UpgradeTargets $installed $protected $approval
    Assert-Upgrade (!$rejected.ContainsKey((Join-Path $protected 'NxTools.exe'))) 'partial/mismatched protected tuple rejected'
    $approval.ServicePath = '"' + (Join-Path $unrelated 'NxTools.Indexer.exe') + '" --service'
    $rejected = Get-UpgradeTargets $installed $protected $approval
    Assert-Upgrade (!$rejected.ContainsKey((Join-Path $protected 'NxTools.exe'))) 'service name without exact registered path rejected'

    # Actual native termination boundary, only two owned temporary fixture EXEs.
    $fixtureExe = Join-Path $installed 'NxTools.exe'
    Add-Type -TypeDefinition 'public static class UpgradeFixture { public static void Main() { System.Threading.Thread.Sleep(60000); } }' -OutputAssembly $fixtureExe -OutputType ConsoleApplication
    $unrelatedExe = Join-Path $unrelated 'NxTools.exe'
    Copy-Item $fixtureExe $unrelatedExe
    $ownedProcess = Start-Process -FilePath $fixtureExe -PassThru -WindowStyle Hidden
    $otherProcess = Start-Process -FilePath $unrelatedExe -PassThru -WindowStyle Hidden
    $fixtureProcesses = @($ownedProcess,$otherProcess)
    Initialize-UpgradeNative
    Assert-Upgrade (![NxToolsUpgradeProcess]::Stop($otherProcess.Id, $fixtureExe)) 'same handle revalidates image before termination'
    $otherProcess.Refresh()
    Assert-Upgrade (!$otherProcess.HasExited) 'unrelated same-name process stays alive'
    # Inject discovery only; production selection, native handle check and wait run.
    function Get-CimInstance { param($ClassName,$Filter)
        foreach ($candidate in $fixtureProcesses) {
            $candidate.Refresh()
            if (!$candidate.HasExited -and (!$Filter -or $Filter -eq "Name='NxTools.exe'")) {
                if ($script:simulateUnavailableImage) {
                    [pscustomobject]@{ProcessId=$candidate.Id;ExecutablePath=$null}
                    continue
                }
                [pscustomobject]@{ProcessId=$candidate.Id;ExecutablePath=$candidate.MainModule.FileName}
            }
        }
    }
    $isolatedTargets = Get-UpgradeTargets $installed $protected $null
    $script:simulateUnavailableImage = $true
    $deniedCode = 0
    try { Stop-UpgradeTargets $isolatedTargets | Out-Null } catch { $deniedCode = $_.Exception.NativeErrorCode }
    Assert-Upgrade ($deniedCode -eq 5) 'unreadable process identity requests elevation instead of guessing'
    $script:simulateUnavailableImage = $false
    $unapprovedBlocked = $false
    try { Stop-UpgradeTargets @{} $unrelated | Out-Null } catch { $unapprovedBlocked = $_.Exception.Message -like '*identity is not approved*' }
    $otherProcess.Refresh()
    Assert-Upgrade ($unapprovedBlocked -and !$otherProcess.HasExited) 'unapproved protected desktop blocks upgrade without termination'
    $closed = Stop-UpgradeTargets $isolatedTargets
    $ownedProcess.Refresh(); $otherProcess.Refresh()
    Assert-Upgrade ($closed -eq 1 -and $ownedProcess.HasExited) 'installer helper terminates and waits for exact old desktop'
    Assert-Upgrade (!$otherProcess.HasExited) 'full cleanup preserves unrelated same-name process'
    Assert-Upgrade ((Stop-UpgradeTargets $isolatedTargets) -eq 0) 'cleanup is idempotent after old process exits'
    $hostFile = Join-Path $installed 'NxTools.BrowserHost.exe'
    $stateFile = Join-Path $fixtureRoot 'host-state.json'
    [IO.File]::WriteAllText($hostFile, 'old host bytes')
    $oldHostHash = (Get-FileHash $hostFile).Hash
    Suspend-UpgradeBrowserHost $installed $stateFile 'fixture-relaunch'
    Assert-Upgrade (!(Test-Path $hostFile) -and (Test-Path $stateFile)) 'old browser host launch path unavailable during replacement'
    Suspend-UpgradeBrowserHost $installed $stateFile 'fixture-relaunch'
    Complete-UpgradeBrowserHost $installed $stateFile
    Assert-Upgrade ((Get-FileHash $hostFile).Hash -eq $oldHostHash -and !(Test-Path $stateFile)) 'cancel restores exact old host and clears recovery state'
    Suspend-UpgradeBrowserHost $installed $stateFile 'fixture-commit'
    [IO.File]::WriteAllText($hostFile, 'new host bytes')
    $newHostHash = (Get-FileHash $hostFile).Hash
    Complete-UpgradeBrowserHost $installed $stateFile -Committed
    Complete-UpgradeBrowserHost $installed $stateFile -Committed
    Assert-Upgrade ((Get-FileHash $hostFile).Hash -eq $newHostHash -and !(Test-Path $stateFile)) 'commit preserves new host and finalization is idempotent'
    Suspend-UpgradeBrowserHost $installed $stateFile 'fixture-tamper'
    $savedState = Get-Content $stateFile -Raw
    $badState = $savedState | ConvertFrom-Json
    $badState.backup_name = '..\unrelated\NxTools.exe'
    [IO.File]::WriteAllText($stateFile, ($badState | ConvertTo-Json))
    $rejectedState = $false
    try { Complete-UpgradeBrowserHost $installed $stateFile -Committed } catch { $rejectedState = $_.Exception -is [IO.InvalidDataException] }
    Assert-Upgrade $rejectedState 'recovery rejects path traversal before file mutation'
    [IO.File]::WriteAllText($stateFile, $savedState)
    Complete-UpgradeBrowserHost $installed $stateFile
    Assert-Upgrade ((Get-FileHash $hostFile).Hash -eq $newHostHash) 'valid recovery remains available after rejecting tampered record'
    Write-Host "Installer upgrade regression: $checks/$checks passed (isolated fixture; no real app/service mutation)."
} finally {
    foreach ($candidate in $fixtureProcesses) {
        $candidate.Refresh()
        if (!$candidate.HasExited) { $candidate.Kill(); $candidate.WaitForExit(5000) | Out-Null }
        $candidate.Dispose()
    }
    Remove-Item $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue
}
