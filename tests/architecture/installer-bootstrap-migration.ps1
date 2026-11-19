param([string]$EvidenceRoot=$env:PI_SCRATCH_DIR)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'installer\bootstrap-migration.ps1') -LibraryOnly
$checks=0
function Check($Condition,$Label){if(!$Condition){throw "FAIL $Label"};$script:checks++;Write-Output "PASS $Label"}
$key='A'*64;$hash='B'*64;$path='C:\ProgramData\NxTools\NxTools.Next\Updater\Bootstrap\NxTools.PlatformHost.exe'
$registration=@{ImagePath='"'+$path+'" --trusted-update-service';ObjectName='LocalSystem';NxToolsUpdateKeySha256=$key;NxToolsBootstrapSha256=$hash;Start=3}
Check ((Get-BootstrapMigrationDecision $key $null $path $null '' $false) -eq 'skip') 'new install skips maintenance'
Check ((Get-BootstrapMigrationDecision $key $registration $path $null $hash $false) -eq 'required') 'old approved updater needs migration'
$state=@{ProtocolVersion=1;Sequence=25;ReleaseId=[Guid]::NewGuid().ToString();ManifestSha256='C'*64;BootstrapSha256=$hash}
Check ((Get-BootstrapMigrationDecision $key $registration $path $state $hash $false) -eq 'skip') 'compatible updater skips elevation'
$compatibleState=@{}+$state;$compatibleState.Sequence=82
Check ((Get-BootstrapMigrationDecision $key $registration $path $compatibleState $hash $false 999 82) -eq 'skip') 'Bootstrap compatibility floor ignores ordinary release sequence'
Check ((Get-BootstrapMigrationDecision $key $registration $path $compatibleState $hash $false 999 83) -eq 'required') 'higher Bootstrap compatibility floor requires maintenance'
Check ((Get-BootstrapMigrationDecision $key $registration $path $null $hash $false 999 82) -eq 'required') 'Bootstrap compatibility floor requires marker'
Check ((Get-BootstrapMigrationDecision $key $registration $path $compatibleState $hash $true 999 82) -eq 'required') 'compatible Bootstrap still recovers interrupted maintenance'
$oldState=@{}+$compatibleState;$oldState.Sequence=81
Check ((Get-BootstrapMigrationDecision $key $registration $path $oldState $hash $false 999 82) -eq 'required') 'Bootstrap below compatibility floor still migrates'
Check ((Get-BootstrapMigrationDecision $key $registration $path $compatibleState $hash $false 999 0) -eq 'required') 'legacy sequence decision remains unchanged'
Check ((Get-BootstrapMigrationDecision $key $registration $path $state $hash $true) -eq 'required') 'interrupted maintenance resumes'
$legacyAutomatic=@{}+$registration;$legacyAutomatic.Start=2
Check ((Get-BootstrapMigrationDecision $key $legacyAutomatic $path $state $hash $false) -eq 'required') 'approved automatic updater requires explicit demand-lifetime maintenance'
$bad=@{}+$registration;$bad.NxToolsUpdateKeySha256='D'*64;$rejected=$false
try{Get-BootstrapMigrationDecision $key $bad $path $null $hash $false|Out-Null}catch{$rejected=$true}
Check $rejected 'foreign publisher rejects'
$rejected=$false;try{Get-BootstrapMigrationDecision $key $registration $path $null ('E'*64) $false|Out-Null}catch{$rejected=$true}
Check $rejected 'tampered updater rejects'
$source=[IO.File]::ReadAllText((Join-Path $root 'installer\nxtools-next.iss'))
Check ($source.Contains('EnsureBootstrapMigration') -and $source.Contains('-BootstrapMigrationPackage')) 'normal installer owns migration before suspension'
Check ($source.IndexOf('EnsureBootstrapMigration') -lt $source.IndexOf("Log('installer.upgrade_close.started")) 'migration detection precedes old file mutation'
Check (!$source.Contains('BootstrapMigrationZip') -and !$source.Contains('NxTools-bootstrap-migration.zip') -and $source.Contains("ExtractTemporaryFiles('{app}\*") -and $source.Contains('BootstrapMigrationHostSha256') -and $source.Contains('BootstrapMigrationMinimumSequence')) 'migration reuses normal installer payload without a second archive and passes Bootstrap floor'
. (Join-Path $root 'installer\close-owned-processes.ps1') -LibraryOnly
$work=Join-Path $EvidenceRoot ('migration-call-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Path $work|Out-Null
$exe=Join-Path $work 'NxTools.PlatformHost.exe'
Add-Type -TypeDefinition 'public static class MigrationCallFixture { public static int Main(string[] args) { if (args.Length == 2 && args[0] == "--repair-missing-trusted-bootstrap") { System.Guid repairId; if (!System.Guid.TryParse(args[1], out repairId) || repairId == System.Guid.Empty) return 89; System.IO.File.WriteAllText(System.IO.Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory,"repair-id.txt"),args[1]); } else if (args.Length != 1 || args[0] != "--maintain-trusted-bootstrap") return 88; return System.IO.File.Exists(System.IO.Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory,"fail.keep")) ? 7 : 0; } }' -OutputAssembly $exe -OutputType ConsoleApplication
Invoke-UpgradeBootstrapMigration $work 'fixture-migration-success'|Out-Null
Check $true 'actual preparation migration call passes the exact explicit command'
[IO.File]::WriteAllText((Join-Path $work 'fail.keep'),'fail')
$rejected=$false;try{Invoke-UpgradeBootstrapMigration $work 'fixture-migration-failure'|Out-Null}catch{$rejected=$true}
Check $rejected 'actual preparation migration failure rejects before file suspension'
Remove-Item -LiteralPath (Join-Path $work 'fail.keep')
Invoke-UpgradeBootstrapMigration $work 'fixture-repair-success' -RepairMissing|Out-Null
$repairIdText=[IO.File]::ReadAllText((Join-Path $work 'repair-id.txt'))
Check ([Guid]::Parse($repairIdText) -ne [Guid]::Empty) 'actual repair command carries nonempty correlation GUID'
[IO.File]::WriteAllText((Join-Path $work 'fail.keep'),'fail')
$rejected=$false;try{Invoke-UpgradeBootstrapMigration $work 'fixture-repair-failure' -RepairMissing|Out-Null}catch{$rejected=$true}
Check $rejected 'actual repair native failure prevents preparation'
$entryRoot=Join-Path $work 'entry';New-Item -ItemType Directory -Path $entryRoot|Out-Null
$entryFixture=Join-Path $entryRoot 'detect.ps1'
[IO.File]::WriteAllText($entryFixture, @'
param([string]$Source,[string]$FixtureRoot,[string]$Mode)
$ErrorActionPreference='Stop'
$env:ProgramData=$FixtureRoot
$global:fixtureRegistration=$null
$updater=Join-Path $FixtureRoot 'NxTools\NxTools.Next\Updater'
$global:fixtureUpdater=$updater
$global:fixtureMode=$Mode
$bootstrap=Join-Path $updater 'Bootstrap\NxTools.PlatformHost.exe'
$manifest=Join-Path $FixtureRoot 'manifest.json'
$diagnostic=Join-Path $FixtureRoot 'detection.jsonl'
New-Item -ItemType Directory -Path $FixtureRoot -Force|Out-Null
@{public_key_sha256=('A'*64);release_sequence=999}|ConvertTo-Json|Set-Content $manifest -Encoding UTF8
if($Mode -ne 'fresh') {
    $global:fixtureRegistration=@{ImagePath='"'+$bootstrap+'" --trusted-update-service';ObjectName='LocalSystem';NxToolsUpdateKeySha256=('A'*64);NxToolsBootstrapSha256=('B'*64)}
}
if($Mode -in @('compatible','old-bootstrap','missing-marker','journal','tampered','invalid-marker','rotate')) {
    New-Item -ItemType Directory -Path (Split-Path $bootstrap) -Force|Out-Null
    [IO.File]::WriteAllText($bootstrap,'owned fixture, never executed')
    $global:fixtureRegistration.NxToolsBootstrapSha256=(Get-FileHash $bootstrap).Hash
    $global:fixtureRegistration.Start=3
    $sequence=if($Mode -eq 'old-bootstrap'){81}else{82}
    @{ProtocolVersion=1;Sequence=$sequence;ReleaseId=[Guid]::NewGuid().ToString();ManifestSha256=('C'*64);BootstrapSha256=$global:fixtureRegistration.NxToolsBootstrapSha256}|ConvertTo-Json|Set-Content (Join-Path $updater 'bootstrap-maintenance-state.json') -Encoding UTF8
    if($Mode -eq 'missing-marker'){Remove-Item (Join-Path $updater 'bootstrap-maintenance-state.json')}
    if($Mode -eq 'journal'){[IO.File]::WriteAllText((Join-Path $updater 'bootstrap-maintenance.json'),'owned recovery fixture')}
    if($Mode -eq 'tampered'){$global:fixtureRegistration.NxToolsBootstrapSha256='B'*64}
    if($Mode -eq 'invalid-marker'){@{ProtocolVersion=1;Sequence=82;ReleaseId=[Guid]::NewGuid().ToString();ManifestSha256=('C'*64);BootstrapSha256=('B'*64)}|ConvertTo-Json|Set-Content (Join-Path $updater 'bootstrap-maintenance-state.json') -Encoding UTF8}
    if($Mode -eq 'rotate'){$global:fixtureRegistration.NxToolsUpdateKeySha256='D'*64}
}
if($Mode -eq 'foreign'){$global:fixtureRegistration.NxToolsUpdateKeySha256='D'*64}
function Get-ItemProperty { param($LiteralPath,$ErrorAction) return $global:fixtureRegistration }
function Get-Item {
    param($LiteralPath,[switch]$Force,$ErrorAction)
    if($global:fixtureMode -eq 'denied-root' -and $LiteralPath -eq $global:fixtureUpdater){throw [UnauthorizedAccessException]::new('fixture denied')}
    Microsoft.PowerShell.Management\Get-Item -LiteralPath $LiteralPath -Force:$Force -ErrorAction Stop
}
& $Source -ManifestPath $manifest -DiagnosticPath $diagnostic -CorrelationId 'fixture-bootstrap-detection' -MinimumBootstrapMaintenanceSequence 82
exit $LASTEXITCODE
'@)
foreach($shellDirectory in @('System32','SysWOW64')) {
    $nativeShell=Join-Path $env:SystemRoot ($shellDirectory+'\WindowsPowerShell\v1.0\powershell.exe')
    foreach($case in @(@{Mode='fresh';Exit=0;Reason='not_registered'},@{Mode='missing';Exit=11;Reason='missing_updater_root'},@{Mode='compatible';Exit=0;Reason='compatible'},@{Mode='old-bootstrap';Exit=10;Reason='migration_required'},@{Mode='missing-marker';Exit=10;Reason='migration_required'},@{Mode='journal';Exit=10;Reason='migration_required'},@{Mode='tampered';Exit=1;Reason='failed'},@{Mode='invalid-marker';Exit=1;Reason='failed'},@{Mode='rotate';Exit=12;Reason='trust_rotation_required'},@{Mode='foreign';Exit=1;Reason='failed'},@{Mode='denied-root';Exit=1;Reason='failed'})) {
        $fixtureRoot=Join-Path $entryRoot ($shellDirectory+'-'+$case.Mode)
        $output=& $nativeShell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $entryFixture -Source (Join-Path $root 'installer\bootstrap-migration.ps1') -FixtureRoot $fixtureRoot -Mode $case.Mode 2>&1
        $actualExit=$LASTEXITCODE
        Check ($actualExit -eq $case.Exit) ("actual detection entry "+$shellDirectory+' '+$case.Mode+' expected '+$case.Exit+' actual '+$actualExit)
        $events=@(Get-Content (Join-Path $fixtureRoot 'detection.jsonl')|ForEach-Object {$_|ConvertFrom-Json})
        Check ($events[-1].reason -eq $case.Reason -and $events[-1].correlation -eq 'fixture-bootstrap-detection') ('actual detection terminal '+$case.Mode)
        Check (@($events|Where-Object {$_|ConvertTo-Json -Compress|Select-String -SimpleMatch $fixtureRoot}).Count -eq 0) ('actual detection excludes paths '+$case.Mode)
    }
}
Check ($source.Contains('BootstrapRepairMissing') -and $source.Contains('bootstrap_migration.detected') -and $source.Contains('ResultCode = 11')) 'normal installer carries explicit missing-root repair and detection outcome'
Check ($source.Contains('PreservePreparationDiagnostics') -and $source.Contains('bootstrap-detection-')) 'detection preserves actual diagnostic before early rejection'
Check (([IO.File]::ReadAllText((Join-Path $root 'installer\close-owned-processes.ps1'))).Contains('--repair-missing-trusted-bootstrap')) 'elevated helper owns explicit missing-root repair command'
Check ((Get-BootstrapMigrationDecision $key $null $path $null '' $false) -eq 'skip') 'missing service remains first-install skip'
Check ((Get-BootstrapMigrationDecision $key $registration $path $state $hash $false) -eq 'skip') 'original compatible decision retained'
Check ((Get-BootstrapMigrationDecision $key $registration $path $null $hash $false) -eq 'required') 'original maintenance decision retained'

Write-Output "PASS installer bootstrap migration: $checks assertions; fake registration/read-only decisions/source wiring only"
