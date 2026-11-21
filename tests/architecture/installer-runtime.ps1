param([ValidateSet('success','failure','owned-process','exited-process','exited-cim-null','trusted-success','trusted-failure','migration-success','migration-cancel','migration-failure','migration-tampered','repair-success','repair-cancel','repair-failure','repair-tampered','detection-failure','protected-success','protected-cancel','protected-failure','dependency-missing','dependency-open','dependency-decline','dependency-silent','dependency-launch-failure','dependency-browser-failure','archive-warning','archive-failure','archive-wrong-user')][string]$Scenario = 'success', [string]$UpgradeHelperPath = '', [string]$RuntimeProbePath = '', [switch]$Maintenance)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$source = Get-Content (Join-Path $root 'installer\nxtools-next.iss') -Raw -Encoding UTF8
$callback = [regex]::Match($source, '(?s)var\s+UpgradeFilesCommitted.*?(?=function SameOwnedPath)').Value
if (!$callback) { throw 'Missing production PrepareToInstall callback' }
if ($Maintenance) {
    if ($Scenario -ne 'success' -and !$Scenario.StartsWith('dependency-')) { throw 'Maintenance admission fixture supports only dependency scenarios and success' }
    $maintenanceSource = [IO.File]::ReadAllText((Join-Path $root 'installer\nxtools-maintenance.iss'))
    $maintenancePrepare = [regex]::Match($maintenanceSource, '(?s)function PrepareToInstall.*?(?=procedure InitializeWizard)').Value
    if (!$maintenancePrepare -or !$maintenanceSource.Contains('#include "runtime-dependencies.iss"')) { throw 'Missing maintenance dependency boundary' }
    $callback = [regex]::Replace($callback, '(?s)function PrepareToInstall.*?(?=procedure FinalizeUpgradeFiles)', $maintenancePrepare)
}
$callback = $callback.Replace('{#MyAppExeName}', 'NxTools.exe')
if ($Maintenance) { $callback = $callback.Replace('    ApplyInstalledArchiveIntegration();', '') }
$compiler = @(${env:ProgramFiles(x86)},$env:ProgramFiles) | ForEach-Object {Join-Path $_ 'Inno Setup 6\ISCC.exe'} | Where-Object {Test-Path $_} | Select-Object -First 1
$work = Join-Path $env:PI_SCRATCH_DIR ('installer-runtime-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $work | Out-Null
$target = Join-Path $work 'target'
New-Item -ItemType Directory $target | Out-Null
$owned = $null
$extraFiles = ''
$dependencyFailure = $Scenario.StartsWith('dependency-')
$probeDirectory = Join-Path $work 'runtime-probe'
New-Item -ItemType Directory $probeDirectory | Out-Null
$probePath = Join-Path $probeDirectory 'NxTools.PlatformHost.exe'
if ($RuntimeProbePath) {
    Copy-Item -LiteralPath $RuntimeProbePath -Destination $probePath
} elseif ($Scenario -eq 'dependency-launch-failure') {
    [IO.File]::WriteAllText($probePath, 'not an executable')
} else {
    $probeCode = if ($dependencyFailure) { '-2147450730' } else { '0' }
    Add-Type -TypeDefinition ('public static class DependencyProbeFixture { public static int Main(string[] args) { return args.Length == 1 && args[0] == "--check-runtime" ? ' + $probeCode + ' : 77; } }') -OutputAssembly $probePath -OutputType ConsoleApplication
}
$dependencyInclude = Join-Path $root 'installer\runtime-dependencies.iss'
if (Test-Path $dependencyInclude) {
    $dependencySource = [IO.File]::ReadAllText($dependencyInclude)
    # Only user choice and browser delivery are adapted; detection uses a real child exit.
    if ($dependencyFailure -and $Scenario -ne 'dependency-silent') {
        $dependencySource = $dependencySource.Replace('not WizardSilent', 'True')
        $answer = if ($Scenario -in @('dependency-open','dependency-browser-failure')) { 'IDYES' } else { 'IDNO' }
        $dependencySource = $dependencySource.Replace('SuppressibleMsgBox(DependencyPrompt, mbError, MB_YESNO, IDNO)', $answer)
    }
    $browserResult = if ($Scenario -eq 'dependency-browser-failure') { 'False' } else { 'True' }
    $browserAdapter = @"
function FixtureOpenDependency(const Verb, FileName, Params, WorkingDir: String; ShowCmd: Integer; Wait: TExecWait; var ResultCode: Integer): Boolean;
begin
  SaveStringToFile('$work\browser-url.txt', FileName, False);
  ResultCode := 31;
  Result := $browserResult;
end;
"@
    $dependencySource = $dependencySource.Replace("ShellExec('open',", "FixtureOpenDependency('open',")
    $callback = $callback.Replace('#include "runtime-dependencies.iss"', $browserAdapter + $dependencySource)
}
if ($Scenario -in @('trusted-success','trusted-failure')) {
    $trustedCode = if ($Scenario -eq 'trusted-failure') { 1 } else { 0 }
    $trustedHost = Join-Path $target 'NxTools.PlatformHost.exe'
    Add-Type -TypeDefinition ('public static class TrustedInstallFixture { public static int Main(string[] args) { return args.Length == 1 && args[0] == "--prepare-trusted-update" ? ' + $trustedCode + ' : 77; } }') -OutputAssembly $trustedHost -OutputType ConsoleApplication
    [IO.File]::WriteAllText((Join-Path $target 'NxTools.release.json'), 'synthetic callback marker; not a real release')
}
if ($Scenario -eq 'owned-process') {
    $fixtureExe = Join-Path $target 'NxTools.exe'
    Add-Type -TypeDefinition 'public static class RuntimeUpgradeFixture { public static void Main() { System.Threading.Thread.Sleep(60000); } }' -OutputAssembly $fixtureExe -OutputType ConsoleApplication
    $owned = Start-Process $fixtureExe -PassThru -WindowStyle Hidden
    [IO.File]::WriteAllText((Join-Path $target 'fixture-id.txt'),[string]$owned.Id)
    Copy-Item (Join-Path $root 'installer\close-owned-processes.ps1') (Join-Path $work 'upgrade-library.ps1')
    $extraFiles = 'Source: "' + $work + '\upgrade-library.ps1"; Flags: dontcopy'
    $callback = $callback.Replace("  ExtractTemporaryFile('close-owned-processes.ps1');", "  ExtractTemporaryFile('upgrade-library.ps1');`r`n  ExtractTemporaryFile('close-owned-processes.ps1');")
    # Only discovery is isolated; selection, same-handle native kill and wait are production.
    $helper = @'
param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[string]$NewManifestPath,[string]$NewSignaturePath)
$ErrorActionPreference='Stop'
$fixtureDirectory=$InstallDirectory
trap { $_ | Out-String | Set-Content (Join-Path $fixtureDirectory 'fixture-error.txt'); exit 1 }
. (Join-Path $PSScriptRoot 'upgrade-library.ps1') -LibraryOnly
$fixtureId=[int](Get-Content (Join-Path $fixtureDirectory 'fixture-id.txt'))
function Get-CimInstance { param($ClassName,$Filter)
    $entry=Get-Process -Id $fixtureId -ErrorAction SilentlyContinue
    if($entry -and (!$Filter -or $Filter -eq "Name='NxTools.exe'")) {
        [pscustomobject]@{ProcessId=$entry.Id;ExecutablePath=(Join-Path $fixtureDirectory 'NxTools.exe')}
    }
}
$targets=Get-UpgradeTargets $fixtureDirectory (Join-Path $fixtureDirectory 'unused') $null
$closed=Stop-UpgradeTargets $targets
if($closed -ne 1){exit 1}
[IO.File]::WriteAllText((Join-Path $fixtureDirectory 'helper-ran.txt'),'closed')
exit 0
'@
} else {
    $code = if($Scenario -eq 'failure'){1}else{0}
    $helper = "param([string]`$InstallDirectory,[string]`$CorrelationId,[string]`$StatePath,[string]`$NewManifestPath,[string]`$NewSignaturePath)`r`n[IO.File]::WriteAllText((Join-Path `$InstallDirectory 'helper-ran.txt'),'ran')`r`nexit $code"
}
if ($Scenario -in @('exited-process','exited-cim-null')) {
    if (!$UpgradeHelperPath) { $UpgradeHelperPath = Join-Path $root 'installer\close-owned-processes.ps1' }
    Copy-Item -LiteralPath $UpgradeHelperPath -Destination (Join-Path $work 'upgrade-library.ps1')
    $extraFiles = 'Source: "' + $work + '\upgrade-library.ps1"; Flags: dontcopy'
    $callback = $callback.Replace("  ExtractTemporaryFile('close-owned-processes.ps1');", "  ExtractTemporaryFile('upgrade-library.ps1');`r`n  ExtractTemporaryFile('close-owned-processes.ps1');")
    $helper = @'
param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[string]$NewManifestPath,[string]$NewSignaturePath)
$ErrorActionPreference='Stop'
$fixtureDirectory=$InstallDirectory
trap { $_ | Out-String | Set-Content (Join-Path $fixtureDirectory 'fixture-error.txt'); exit 1 }
. (Join-Path $PSScriptRoot 'upgrade-library.ps1') -LibraryOnly
$child=New-Object Diagnostics.Process
$child.StartInfo.FileName=Join-Path $env:SystemRoot 'System32\cmd.exe'
$child.StartInfo.Arguments='/d /c exit 0'
$child.StartInfo.UseShellExecute=$false
$child.StartInfo.CreateNoWindow=$true
try {
 if(!$child.Start()){throw 'Owned child start failed'}
 $retainedHandle=$child.Handle
 $fixtureId=$child.Id
 if(!$child.WaitForExit(10000)){throw 'Owned child exit failed'}
 $script:deliveredStale=$false
 $fixtureImage=$child.StartInfo.FileName
 function Get-CimInstance {param($ClassName,$Filter)
  if(!$script:deliveredStale -and $Filter -eq "Name='NxTools.exe'") {
   $script:deliveredStale=$true
   return [pscustomobject]@{ProcessId=$fixtureId;ExecutablePath=$fixtureImage}
  }
  return @()
 }
 $targets=@{$fixtureImage='owned-test'}
 if((Stop-UpgradeTargets $targets) -ne 0 -or !$script:deliveredStale){throw 'Exited discovery did not reach original stop boundary'}
 [IO.File]::WriteAllText((Join-Path $fixtureDirectory 'helper-ran.txt'),'exited process handled')
}finally{$child.Dispose()}
exit 0
'@
    if ($Scenario -eq 'exited-cim-null') { $helper = $helper.Replace('ExecutablePath=$fixtureImage', 'ExecutablePath=$null') }
}
[IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'),$helper,[Text.UTF8Encoding]::new($true))
[IO.File]::WriteAllText((Join-Path $work 'marker.txt'),'installed')
# Extraction-only inputs keep the existing callback scenarios source-compatible.
[IO.File]::WriteAllText((Join-Path $work 'NxTools.release.json'), '{"files":[]}')
[IO.File]::WriteAllText((Join-Path $work 'NxTools.release.sig'), 'runtime scenario fake sidecar')
$repair=$Scenario.StartsWith('repair-')
$migration = $Scenario.StartsWith('migration-') -or $repair
$detectCode = if ($repair) { 11 } elseif ($migration) { 10 } elseif ($Scenario -eq 'detection-failure') { 1 } else { 0 }
 [IO.File]::WriteAllText((Join-Path $work 'bootstrap-migration.ps1'), ('param([string]$ManifestPath,[string]$DiagnosticPath,[string]$CorrelationId,[long]$MinimumBootstrapMaintenanceSequence)' + "`r`n" + 'if($DiagnosticPath){[IO.File]::WriteAllText($DiagnosticPath,''{"event":"fixture_detection"}'')}' + "`r`n" + 'exit ' + $detectCode))
$payload = Join-Path $work 'payload'; New-Item -ItemType Directory -Path (Join-Path $payload 'nested') -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $payload 'NxTools.PlatformHost.exe'), 'same-release migration fixture; never executed')
[IO.File]::WriteAllText((Join-Path $payload 'nested\fixture.txt'), 'nested same-release bytes')
$payloadHash = (Get-FileHash (Join-Path $payload 'NxTools.PlatformHost.exe')).Hash
if ($Scenario -in @('migration-tampered','repair-tampered')) { $payloadHash = '0' * 64 }
 $callback = $callback.Replace('{#BootstrapMigrationHostSha256}', $payloadHash)
 $callback = $callback.Replace('{#BootstrapMigrationMinimumSequence}', '82')
$callback = $callback.Replace('{localappdata}\NxTools\NxTools.Next\logs\installer\', $work + '\diagnostics\')
# Diagnostic argument adaptation is applied below, after scenario-specific helpers are selected.
$payloadFiles = if ($migration) { 'Source: "' + $payload + '\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs' } else { '' }
if ($migration) {
    # Test-only adapter: never invokes runas or a real administrator maintenance action.
    $callback = $callback.Replace("ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)", "Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)")
    $answer = if ($Scenario -in @('migration-cancel','repair-cancel')) { 'IDCANCEL' } else { 'IDOK' }
    $callback = [regex]::Replace($callback, "MsgBox\(Prompt, mbConfirmation, MB_OKCANCEL\)", $answer)
    $code = if ($Scenario -in @('migration-failure','repair-failure')) { 1 } else { 0 }
    $expectedRepair=if($repair){'$true'}else{'$false'}
    $helper = 'param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[string]$NewManifestPath,[string]$NewSignaturePath,[string]$BootstrapMigrationPackage,[switch]$RepairMissingBootstrap)' + "`r`n" +
        'if([bool]$RepairMissingBootstrap -ne '+$expectedRepair+'){exit 89}; ' +
        'if (!$BootstrapMigrationPackage -or [IO.File]::ReadAllText((Join-Path $BootstrapMigrationPackage ''NxTools.PlatformHost.exe'')) -ne ''same-release migration fixture; never executed'' -or [IO.File]::ReadAllText((Join-Path $BootstrapMigrationPackage ''nested\fixture.txt'')) -ne ''nested same-release bytes'') { exit 88 }; [IO.File]::WriteAllText((Join-Path $InstallDirectory ''helper-ran.txt''),''migration then preparation'')' + "`r`nexit $code"
    [IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'), $helper, [Text.UTF8Encoding]::new($true))
}
$protected = $Scenario.StartsWith('protected-')
# Discovery and UAC are test adapters only; run the actual production callback and selection.
$serviceProbe = "RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1')"
$callback = $callback.Replace($serviceProbe, $(if ($protected) { 'True' } else { 'False' }))
if ($protected) {
    $elevatedCode = if ($Scenario -eq 'protected-failure') { 7 } else { 0 }
    $ordinaryBranch = if ($Scenario -eq 'protected-success') { '[IO.File]::WriteAllText((Join-Path $InstallDirectory ''helper-ran.txt''),''ordinary preparation''); exit 0' } else { 'exit 5' }
    $helper = 'param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[string]$NewManifestPath,[string]$NewSignaturePath,[switch]$FixtureElevated)' + "`r`n" +
        'if (!$FixtureElevated) { '+$ordinaryBranch+' }; [IO.File]::WriteAllText((Join-Path $InstallDirectory ''helper-ran.txt''),''ran''); [IO.File]::WriteAllText((Join-Path $InstallDirectory ''elevated-ran.txt''),''ran'')' + "`r`nexit $elevatedCode"
    [IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'), $helper, [Text.UTF8Encoding]::new($true))
    $launchBody = if ($Scenario -eq 'protected-cancel') { '  ResultCode := 1223; Result := False;' } else { '  Result := Exec(PowerShellPath, Parameters + '' -FixtureElevated'', '''', SW_HIDE, ewWaitUntilTerminated, ResultCode);' }
    $adapter = "function FixtureElevated(PowerShellPath, Parameters: String; var ResultCode: Integer): Boolean;`r`nbegin`r`n$launchBody`r`nend;`r`n`r`n"
    $callback = $callback.Replace('function EnsureBootstrapMigration(): String;', $adapter + 'function EnsureBootstrapMigration(): String;')
    $callback = $callback.Replace("ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)", 'FixtureElevated(PowerShellPath, Parameters, ResultCode)')
}
$helperFile = Join-Path $work 'close-owned-processes.ps1'
$helperText = [IO.File]::ReadAllText($helperFile).Replace('[string]$NewSignaturePath', '[string]$DiagnosticPath,[string]$NewSignaturePath')
$helperText = [regex]::Replace($helperText, '(?m)^(param\([^\r\n]+\))', '$1' + "`r`n" + 'if ($DiagnosticPath) { [IO.File]::WriteAllText($DiagnosticPath,''{"schema":1,"event":"fixture"}'') }')
$archivePayload = ''
$archiveCode = switch ($Scenario) { 'archive-warning' { 10 } 'archive-failure' { 11 } 'archive-wrong-user' { 12 } default { 0 } }
if (!$Maintenance) {
    $archiveDesktop = Join-Path $work 'NxTools.exe'
    $archiveStub = @"
public static class InstallerArchiveFixture {
    public static int Main(string[] args) {
        System.Guid correlation;
        if (args.Length != 3 || args[0] != "--install-archive-integration" ||
            !System.Guid.TryParseExact(args[1], "D", out correlation) || correlation == System.Guid.Empty ||
            (args[2] != "fresh" && args[2] != "upgrade")) return 91;
        System.IO.File.WriteAllText(System.IO.Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory, "archive-apply.txt"), args[1]);
        return $archiveCode;
    }
}
"@
    Add-Type -TypeDefinition $archiveStub -OutputAssembly $archiveDesktop -OutputType ConsoleApplication
    $archivePayload = 'Source: "' + $archiveDesktop + '"; DestDir: "{app}"; Flags: ignoreversion'
}
[IO.File]::WriteAllText($helperFile, $helperText, [Text.UTF8Encoding]::new($true))
$definition = @"
[Setup]
AppName=NxTools isolated installer runtime
AppVersion=1
DefaultDirName=$target
OutputDir=$work
OutputBaseFilename=runtime-fixture
PrivilegesRequired=lowest
Uninstallable=no
CreateUninstallRegKey=no
UsePreviousAppDir=no
DisableDirPage=yes
DisableProgramGroupPage=yes
CloseApplications=no
[Files]
Source: "$work\close-owned-processes.ps1"; Flags: dontcopy
Source: "$work\NxTools.release.json"; Flags: dontcopy
Source: "$work\NxTools.release.sig"; Flags: dontcopy
Source: "$work\bootstrap-migration.ps1"; Flags: dontcopy
Source: "$probePath"; Flags: dontcopy
$payloadFiles
$extraFiles
$archivePayload
Source: "$work\marker.txt"; DestDir: "{app}"
[Code]
$callback
"@
$runner = $null
try {
    [IO.File]::WriteAllText((Join-Path $work 'fixture.iss'),$definition,[Text.UTF8Encoding]::new($true))
    & $compiler (Join-Path $work 'fixture.iss') *> (Join-Path $work 'compile.log')
    if($LASTEXITCODE -ne 0){throw "Fixture compile failed: $work"}
    $log = Join-Path $work 'runtime.log'
    $runner = Start-Process (Join-Path $work 'runtime-fixture.exe') -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="'+$log+'"') -PassThru
    if (!$runner.WaitForExit(30000)) { throw 'Installer fixture timed out' }
    $runtimeText = Get-Content $log -Raw
    $helperRan = Test-Path (Join-Path $target 'helper-ran.txt')
    $markerInstalled = Test-Path (Join-Path $target 'marker.txt')
    if ($dependencyFailure) {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and
            $runtimeText -match 'installer.dependency.failed' -and
            $runtimeText -notmatch 'bootstrap_migration.detecting|upgrade_close.started|trusted_update.started' -and
            $runtimeText -match 'https://dotnet.microsoft.com/zh-cn/download/dotnet/8.0'
        $browserFile = Join-Path $work 'browser-url.txt'
        if ($Scenario -in @('dependency-open','dependency-browser-failure')) {
            $passed = $passed -and (Test-Path $browserFile) -and
                ([IO.File]::ReadAllText($browserFile) -eq 'https://dotnet.microsoft.com/zh-cn/download/dotnet/8.0')
        } else { $passed = $passed -and !(Test-Path $browserFile) }
        if ($Scenario -eq 'dependency-browser-failure') { $passed = $passed -and $runtimeText -match 'installer.dependency.download_failed.*code=31' }
    } elseif ($Maintenance) {
        $passed = $runner.ExitCode -eq 0 -and !$helperRan -and $markerInstalled -and
            $runtimeText -match 'installer.dependency.completed' -and
            $runtimeText -notmatch 'bootstrap_migration.detecting|upgrade_close.started'
    } elseif ($Scenario -eq 'protected-success') {
        $passed = $runner.ExitCode -eq 0 -and $helperRan -and $markerInstalled -and !(Test-Path (Join-Path $target 'elevated-ran.txt')) -and $runtimeText -notmatch 'protected_elevation_requested|upgrade_close.elevation_required'
    } elseif ($Scenario -eq 'protected-cancel') {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and $runtimeText -match 'upgrade_close.failed.*code=1223'
    } elseif ($Scenario -eq 'protected-failure') {
        $passed = $runner.ExitCode -ne 0 -and $helperRan -and !$markerInstalled -and (Test-Path (Join-Path $target 'elevated-ran.txt')) -and $runtimeText -match 'upgrade_close.failed.*code=7'
    } elseif ($Scenario -in @('migration-cancel','repair-cancel','detection-failure')) {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and $runtimeText -notmatch 'bootstrap_migration.elevation_requested'
    } elseif ($Scenario -in @('migration-tampered','repair-tampered')) {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and $runtimeText -notmatch 'bootstrap_migration.elevation_requested'
    } elseif ($Scenario -in @('migration-failure','repair-failure')) {
        $passed = $runner.ExitCode -ne 0 -and $helperRan -and !$markerInstalled
    } elseif ($Scenario -in @('migration-success','repair-success')) {
        $passed = $runner.ExitCode -eq 0 -and $helperRan -and $markerInstalled -and ([regex]::Matches($runtimeText,'bootstrap_migration.elevation_requested')).Count -eq 1
    } else {
    if ($Scenario.StartsWith('archive-')) {
        $passed = $runner.ExitCode -eq $archiveCode -and $helperRan -and $markerInstalled
    } elseif ($Scenario -eq 'trusted-failure') {
        $passed = $runner.ExitCode -eq 9 -and $helperRan -and $markerInstalled -and
            $runtimeText -match 'trusted_update.started' -and $runtimeText -notmatch 'trusted_update.completed'
    } elseif($Scenario -eq 'failure') {
        $passed = $runner.ExitCode -ne 0 -and $helperRan -and !$markerInstalled -and $runtimeText -match 'upgrade_close.failed'
    } else {
        $passed = $runner.ExitCode -eq 0 -and $helperRan -and $markerInstalled -and $runtimeText -match 'upgrade_close.completed'
    }
    }
    $diagnosticFiles = @(Get-ChildItem -LiteralPath (Join-Path $work 'diagnostics') -File -Recurse -ErrorAction SilentlyContinue)
    $archiveReceipt = Join-Path $target 'archive-apply.txt'
    if (!$Maintenance -and $markerInstalled -and $Scenario -ne 'trusted-failure') {
        $passed = $passed -and (Test-Path -LiteralPath $archiveReceipt)
        if (Test-Path -LiteralPath $archiveReceipt) {
            $receipt = [regex]::Escape([IO.File]::ReadAllText($archiveReceipt))
            $passed = $passed -and $runtimeText -match ('installer.archive_integration.terminal correlation=' + $receipt + ' .*started=1 code=' + $archiveCode + '\b')
        }
    } else {
        $passed = $passed -and !(Test-Path -LiteralPath $archiveReceipt) -and $runtimeText -notmatch 'installer.archive_integration.started'
    }
    if ($helperRan) {
        $passed = $passed -and @($diagnosticFiles | Where-Object Name -eq 'launch.txt').Count -eq 1 -and @($diagnosticFiles | Where-Object Name -eq 'preparation.jsonl').Count -eq 1
    } elseif ($Scenario -eq 'protected-cancel') {
        $passed = $passed -and @($diagnosticFiles | Where-Object Name -eq 'launch.txt').Count -eq 1
    }
    if (!$dependencyFailure -and !$Maintenance) {
        $passed=$passed -and @($diagnosticFiles|Where-Object Name -eq 'bootstrap-detection.jsonl').Count -eq 1
        $passed=$passed -and $runtimeText -match ('bootstrap_migration.detected started=1 exit='+$detectCode)
        $passed=$passed -and $runtimeText -match 'installer.dependency.completed'
    } else {
        $passed=$passed -and $diagnosticFiles.Count -eq 0
        if (!$dependencyFailure) { $passed=$passed -and $runtimeText -match 'installer.dependency.completed' }
    }
    $passed = $passed -and $runtimeText -notmatch 'Type Mismatch'
    if ($Scenario -eq 'trusted-failure') {
        $passed = $passed -and $runtimeText -match 'trusted_update.failed.*code=1'
    } else {
        $passed = $passed -and $runtimeText -notmatch 'Runtime error'
    }
    if ($Scenario -eq 'trusted-success') { $passed = $passed -and $runtimeText -match 'trusted_update.completed' }
    if($owned){$owned.Refresh();$passed=$passed -and $owned.HasExited}
    Write-Host "Runtime fixture $Scenario : exit=$($runner.ExitCode) pass=$passed artifacts=$work"
    Select-String -Path $log -Pattern 'Exception|Mismatch|Runtime error|upgrade_close|Installation process' | ForEach-Object {$_.Line | Write-Host}
    if(!$passed){throw 'Production installer runtime regression failed'}
} finally {
    foreach($entry in @($runner,$owned)) {
        if(!$entry){continue}
        $entry.Refresh()
        if(!$entry.HasExited){$entry.Kill();$entry.WaitForExit(5000) | Out-Null}
        $entry.Dispose()
    }
}
