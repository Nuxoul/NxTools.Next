param([ValidateSet('success','rollback','rollback-after-file','commit-failure','trusted-failure')][string]$Scenario='success')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$work=Join-Path $env:PI_SCRATCH_DIR ('browserhost-replacement-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $work | Out-Null
$target=Join-Path $work 'installed'
New-Item -ItemType Directory $target | Out-Null
$oldPath=Join-Path $target 'NxTools.BrowserHost.exe'
$newPath=Join-Path $work 'new-host.exe'
$library=Join-Path $root 'installer\close-owned-processes.ps1'
Add-Type -TypeDefinition 'public static class BrowserReplacementFixture { public static void Main() { System.Threading.Thread.Sleep(60000); } }' -OutputAssembly $oldPath -OutputType ConsoleApplication
Add-Type -TypeDefinition 'public static class NewBrowserReplacementFixture { public static void Main() { System.Console.WriteLine("new package fixture"); } }' -OutputAssembly $newPath -OutputType ConsoleApplication
$expectedNewHash=(Get-FileHash $newPath).Hash
$expectedOldHash=(Get-FileHash $oldPath).Hash
if($expectedNewHash -eq $expectedOldHash){throw 'Replacement fixture must differ from old file'}
$runner=$null
$lateHost=$null
$initialHost=$null
try {
    $initialHost=Start-Process $oldPath -PassThru -WindowStyle Hidden
    [IO.File]::WriteAllText((Join-Path $target 'initial-pid.txt'),[string]$initialHost.Id)
    # Exact production Setup callbacks; discovery alone is isolated to test PIDs.
    $production=Get-Content (Join-Path $root 'installer\nxtools-next.iss') -Raw -Encoding UTF8
    $callback=[regex]::Match($production,'(?s)var\s+UpgradeFilesCommitted.*?(?=function SameOwnedPath)').Value
    if(!$callback){throw 'Missing production callbacks'}
    # The archive callback has its own installer fixture; this scenario only stages BrowserHost.
    $callback = $callback.Replace('{#MyAppExeName}', 'NxTools.exe').Replace('    ApplyInstalledArchiveIntegration();', '')
    $callback = $callback.Replace('#include "runtime-dependencies.iss"', [IO.File]::ReadAllText((Join-Path $root 'installer\runtime-dependencies.iss')))
    $probePath = Join-Path $work 'NxTools.PlatformHost.exe'
    Add-Type -TypeDefinition 'public static class ReplacementDependencyProbe { public static int Main(string[] args) { return args.Length == 1 && args[0] == "--check-runtime" ? 0 : 77; } }' -OutputAssembly $probePath -OutputType ConsoleApplication
    $callback = $callback.Replace('{localappdata}\NxTools\NxTools.Next\logs\installer\', $work + '\diagnostics\')
    # Never let installed service discovery request real UAC in this isolated fixture.
    $callback = $callback.Replace("RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1')", 'False')
    $callback = [regex]::Replace($callback, '(?s)function EnsureBootstrapMigration\(\): String;.*?(?=function PrepareToInstall)', "function EnsureBootstrapMigration(): String;`r`nbegin`r`n  BootstrapMigrationDirectory := ''; Result := '';`r`nend;`r`n`r`n")
    if($Scenario -eq 'rollback'){
        $anchor='  if CurStep = ssPostInstall then'
        if(!$callback.Contains($anchor)){throw 'Missing step seam'}
        $callback=$callback.Replace($anchor,'  if CurStep = ssInstall then RaiseException(''isolated pre-copy failure'');' + "`r`n" + $anchor)
    }
    $extract="  ExtractTemporaryFile('close-owned-processes.ps1');"
    if(!$callback.Contains($extract)){throw 'Missing extraction seam'}
    $callback=$callback.Replace($extract,"  ExtractTemporaryFile('upgrade-library.ps1');`r`n"+$extract)
    Copy-Item $library (Join-Path $work 'upgrade-library.ps1')
    # This retained scenario deliberately exercises the legacy single-host API.
    # Supply extraction inputs, but keep rollback identity fixed BEFORE install.
    [IO.File]::WriteAllText((Join-Path $work 'NxTools.release.json'), '{"files":[]}')
    [IO.File]::WriteAllText((Join-Path $work 'NxTools.release.sig'), 'legacy scenario fake sidecar')
    [IO.File]::WriteAllText((Join-Path $target 'expected-new-sha256.txt'), $expectedNewHash)
    $wrapper=@'
param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[switch]$Finalize,[switch]$Committed,[string]$RollbackNewSha256,[string]$NewManifestPath,[string]$NewSignaturePath,[string]$DiagnosticPath)
$ErrorActionPreference='Stop'
$fixtureDirectory=$InstallDirectory
$fixtureState=$StatePath
$fixtureOperation=$CorrelationId
$fixtureFinalize=[bool]$Finalize
$fixtureCommitted=[bool]$Committed
$fixtureRollbackHash=$RollbackNewSha256
if (!$fixtureRollbackHash) { $fixtureRollbackHash = [IO.File]::ReadAllText((Join-Path $fixtureDirectory 'expected-new-sha256.txt')) }
trap { $_ | Out-String | Set-Content (Join-Path $fixtureDirectory 'fixture-error.txt'); exit 1 }
. (Join-Path $PSScriptRoot 'upgrade-library.ps1') -LibraryOnly
function Get-CimInstance { param($ClassName,$Filter)
    foreach($idFile in @('initial-pid.txt','late-pid.txt')){
        if(Test-Path (Join-Path $fixtureDirectory $idFile)){
            $fixtureId=[int](Get-Content (Join-Path $fixtureDirectory $idFile))
            $entry=Get-Process -Id $fixtureId -ErrorAction SilentlyContinue
            if($entry -and (!$Filter -or $Filter -eq "Name='NxTools.BrowserHost.exe'")){
                [pscustomobject]@{ProcessId=$entry.Id;ExecutablePath=(Join-Path $fixtureDirectory 'NxTools.BrowserHost.exe')}
            }
        }
    }
}
if($fixtureFinalize){
    if($fixtureCommitted -and (Test-Path (Join-Path $fixtureDirectory 'fail-commit.txt'))){
        Copy-Item $fixtureState (Join-Path $fixtureDirectory 'retained-state.json')
        exit 1
    }
    Complete-UpgradeBrowserHost $fixtureDirectory $fixtureState -Committed:$fixtureCommitted -RollbackNewSha256 $fixtureRollbackHash
    exit 0
}
$targets=Get-UpgradeTargets $fixtureDirectory (Join-Path $fixtureDirectory 'unused') $null
Stop-UpgradeTargets $targets | Out-Null
Suspend-UpgradeBrowserHost $fixtureDirectory $fixtureState $fixtureOperation
[IO.File]::WriteAllText((Join-Path $fixtureDirectory 'cleanup-done.txt'),'done')
# Deterministic Chrome-style relaunch after PrepareToInstall cleanup: it must
# find no executable at the original path, rather than reopen the old image.
if(Test-Path (Join-Path $fixtureDirectory 'NxTools.BrowserHost.exe')){
    $entry=Start-Process (Join-Path $fixtureDirectory 'NxTools.BrowserHost.exe') -PassThru -WindowStyle Hidden
    [IO.File]::WriteAllText((Join-Path $fixtureDirectory 'late-pid.txt'),[string]$entry.Id)
}
exit 0
'@
    [IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'),$wrapper,[Text.UTF8Encoding]::new($true))
    $failureFile=''
    if ($Scenario -eq 'trusted-failure') {
        $trustedHost = Join-Path $target 'NxTools.PlatformHost.exe'
        Add-Type -TypeDefinition 'public static class TrustedBrowserRollbackFixture { public static int Main(string[] args) { return 1; } }' -OutputAssembly $trustedHost -OutputType ConsoleApplication
        [IO.File]::WriteAllText((Join-Path $target 'NxTools.release.json'), 'synthetic callback marker')
    }
    $lockStream=$null
    if($Scenario -eq 'rollback-after-file'){
        $locked=Join-Path $target 'locked.txt'
        [IO.File]::WriteAllText($locked,'old second file')
        [IO.File]::WriteAllText((Join-Path $work 'second.txt'),'new second file')
        $failureFile='Source: "'+$work+'\second.txt"; DestDir: "{app}"; DestName: "locked.txt"; Flags: ignoreversion'
        $lockStream=[IO.File]::Open($locked,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None)
    }
    $definition=@"
[Setup]
AppName=NxTools BrowserHost isolated replacement
AppVersion=1
DefaultDirName=$target
OutputDir=$work
OutputBaseFilename=replacement-fixture
PrivilegesRequired=lowest
Uninstallable=no
CreateUninstallRegKey=no
UsePreviousAppDir=no
DisableDirPage=yes
DisableProgramGroupPage=yes
CloseApplications=no
[Files]
Source: "$work\close-owned-processes.ps1"; Flags: dontcopy
Source: "$work\upgrade-library.ps1"; Flags: dontcopy
Source: "$work\NxTools.release.json"; Flags: dontcopy
Source: "$work\NxTools.release.sig"; Flags: dontcopy
Source: "$probePath"; Flags: dontcopy
Source: "$newPath"; DestDir: "{app}"; DestName: "NxTools.BrowserHost.exe"; Flags: ignoreversion
$failureFile
[Code]
$callback
"@
    [IO.File]::WriteAllText((Join-Path $work 'fixture.iss'),$definition,[Text.UTF8Encoding]::new($true))
    $compiler=@(${env:ProgramFiles(x86)},$env:ProgramFiles)|ForEach-Object{Join-Path $_ 'Inno Setup 6\ISCC.exe'}|Where-Object{Test-Path $_}|Select-Object -First 1
    & $compiler (Join-Path $work 'fixture.iss') *> (Join-Path $work 'compile.log')
    if($LASTEXITCODE -ne 0){throw "Compile failed: $work"}
    $log=Join-Path $work 'runtime.log'
    if($Scenario -eq 'commit-failure'){[IO.File]::WriteAllText((Join-Path $target 'fail-commit.txt'),'fail')}
    $runner=Start-Process (Join-Path $work 'replacement-fixture.exe') -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="'+$log+'"') -PassThru
    if(!$runner.WaitForExit(30000)){throw 'Setup timed out'}
    $text=Get-Content $log -Raw
    $initialHost.Refresh()
    $pass=(Test-Path $oldPath) -and $initialHost.HasExited -and (Test-Path (Join-Path $target 'cleanup-done.txt')) -and !(Test-Path (Join-Path $target 'late-pid.txt')) -and $text -notmatch 'Type Mismatch'
    if($Scenario -eq 'success'){
        $pass=$pass -and $runner.ExitCode -eq 0 -and (Get-FileHash $oldPath).Hash -eq $expectedNewHash -and $text -notmatch 'DeleteFile failed|upgrade_files.failed'
    } elseif($Scenario -eq 'commit-failure'){
        $pass=$pass -and $runner.ExitCode -eq 9 -and (Get-FileHash $oldPath).Hash -eq $expectedNewHash -and $text -match 'upgrade_files.failed' -and (Test-Path (Join-Path $target 'retained-state.json'))
    } else {
        $pass=$pass -and $runner.ExitCode -ne 0 -and (Get-FileHash $oldPath).Hash -eq $expectedOldHash -and $text -notmatch 'upgrade_files.failed'
        if($Scenario -eq 'rollback-after-file'){$pass=$pass -and $text -match 'locked.txt' -and $text -match 'Successfully installed the file' -and $text -match 'uninstallation process succeeded'}
        if($Scenario -eq 'trusted-failure'){$pass=$pass -and $runner.ExitCode -eq 9 -and $text -match 'trusted_update.failed'}
    }
    $backups=@(Get-ChildItem $target -Filter '*.upgrade-old' -Force)
    $pass=$pass -and $backups.Count -eq $(if($Scenario -eq 'commit-failure'){1}else{0})
    $pass=$pass -and (Test-Path (Join-Path $target '.NxTools.BrowserHost-upgrade.json')) -eq ($Scenario -eq 'commit-failure')
    Write-Host "BrowserHost actual replacement $Scenario : exit=$($runner.ExitCode) pass=$pass artifacts=$work"
    Select-String -Path $log -Pattern 'DeleteFile|code 5|upgrade_close|upgrade_files|fixture.host|Installation process' |ForEach-Object{$_.Line|Write-Host}
    if(!$pass){
        if(Test-Path (Join-Path $target 'fixture-error.txt')){Get-Content (Join-Path $target 'fixture-error.txt') | Write-Host}
        throw 'BrowserHost replacement/rollback regression failed'
    }
} finally {
    if($lockStream){$lockStream.Dispose()}
    if(Test-Path (Join-Path $target 'late-pid.txt')){$lateHost=Get-Process -Id ([int](Get-Content (Join-Path $target 'late-pid.txt'))) -ErrorAction SilentlyContinue}
    foreach($entry in @($runner,$lateHost,$initialHost)){
        if($entry){$entry.Refresh();if(!$entry.HasExited){$entry.Kill();$entry.WaitForExit(5000)|Out-Null};$entry.Dispose()}
    }
    if($Scenario -eq 'commit-failure' -and (Test-Path (Join-Path $target 'retained-state.json'))){
        . $library -LibraryOnly
        Complete-UpgradeBrowserHost $target (Join-Path $target 'retained-state.json') -Committed
    }
}
