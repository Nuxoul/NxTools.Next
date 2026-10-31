param([ValidateSet('success','failure','owned-process','trusted-success','trusted-failure','migration-success','migration-cancel','migration-failure','migration-tampered')][string]$Scenario = 'success')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$source = Get-Content (Join-Path $root 'installer\nxtools-next.iss') -Raw -Encoding UTF8
$callback = [regex]::Match($source, '(?s)var\s+UpgradeFilesCommitted.*?(?=function SameOwnedPath)').Value
if (!$callback) { throw 'Missing production PrepareToInstall callback' }
$compiler = @(${env:ProgramFiles(x86)},$env:ProgramFiles) | ForEach-Object {Join-Path $_ 'Inno Setup 6\ISCC.exe'} | Where-Object {Test-Path $_} | Select-Object -First 1
$work = Join-Path $env:PI_SCRATCH_DIR ('installer-runtime-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $work | Out-Null
$target = Join-Path $work 'target'
New-Item -ItemType Directory $target | Out-Null
$owned = $null
$extraFiles = ''
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
[IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'),$helper,[Text.UTF8Encoding]::new($true))
[IO.File]::WriteAllText((Join-Path $work 'marker.txt'),'installed')
# Extraction-only inputs keep the existing callback scenarios source-compatible.
[IO.File]::WriteAllText((Join-Path $work 'NxTools.release.json'), '{"files":[]}')
[IO.File]::WriteAllText((Join-Path $work 'NxTools.release.sig'), 'runtime scenario fake sidecar')
$migration = $Scenario.StartsWith('migration-')
$detectCode = if ($migration) { 10 } else { 0 }
[IO.File]::WriteAllText((Join-Path $work 'bootstrap-migration.ps1'), ('param([string]$ManifestPath)' + "`r`nexit $detectCode"))
$payload = Join-Path $work 'payload'; New-Item -ItemType Directory -Path (Join-Path $payload 'nested') -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $payload 'NxTools.PlatformHost.exe'), 'same-release migration fixture; never executed')
[IO.File]::WriteAllText((Join-Path $payload 'nested\fixture.txt'), 'nested same-release bytes')
$payloadHash = (Get-FileHash (Join-Path $payload 'NxTools.PlatformHost.exe')).Hash
if ($Scenario -eq 'migration-tampered') { $payloadHash = '0' * 64 }
$callback = $callback.Replace('{#BootstrapMigrationHostSha256}', $payloadHash)
$payloadFiles = if ($migration) { 'Source: "' + $payload + '\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs' } else { '' }
if ($migration) {
    # Test-only adapter: never invokes runas or a real administrator maintenance action.
    $callback = $callback.Replace("ShellExec('runas', PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)", "Exec(PowerShellPath, Parameters, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)")
    $answer = if ($Scenario -eq 'migration-cancel') { 'IDCANCEL' } else { 'IDOK' }
    $callback = [regex]::Replace($callback, "MsgBox\('[^']*', mbConfirmation, MB_OKCANCEL\)", $answer)
    $code = if ($Scenario -eq 'migration-failure') { 1 } else { 0 }
    $helper = 'param([string]$InstallDirectory,[string]$CorrelationId,[string]$StatePath,[string]$NewManifestPath,[string]$NewSignaturePath,[string]$BootstrapMigrationPackage)' + "`r`n" +
        'if (!$BootstrapMigrationPackage -or [IO.File]::ReadAllText((Join-Path $BootstrapMigrationPackage ''NxTools.PlatformHost.exe'')) -ne ''same-release migration fixture; never executed'' -or [IO.File]::ReadAllText((Join-Path $BootstrapMigrationPackage ''nested\fixture.txt'')) -ne ''nested same-release bytes'') { exit 88 }; [IO.File]::WriteAllText((Join-Path $InstallDirectory ''helper-ran.txt''),''migration then preparation'')' + "`r`nexit $code"
    [IO.File]::WriteAllText((Join-Path $work 'close-owned-processes.ps1'), $helper, [Text.UTF8Encoding]::new($true))
}
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
$payloadFiles
$extraFiles
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
    if ($Scenario -eq 'migration-cancel') {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and $runtimeText -notmatch 'bootstrap_migration.elevation_requested'
    } elseif ($Scenario -eq 'migration-tampered') {
        $passed = $runner.ExitCode -ne 0 -and !$helperRan -and !$markerInstalled -and $runtimeText -notmatch 'bootstrap_migration.elevation_requested'
    } elseif ($Scenario -eq 'migration-failure') {
        $passed = $runner.ExitCode -ne 0 -and $helperRan -and !$markerInstalled
    } elseif ($Scenario -eq 'migration-success') {
        $passed = $runner.ExitCode -eq 0 -and $helperRan -and $markerInstalled -and ([regex]::Matches($runtimeText,'bootstrap_migration.elevation_requested')).Count -eq 1
    } else {
    if($Scenario -eq 'trusted-failure') {
        $passed = $runner.ExitCode -eq 9 -and $helperRan -and $markerInstalled -and
            $runtimeText -match 'trusted_update.started' -and $runtimeText -notmatch 'trusted_update.completed'
    } elseif($Scenario -eq 'failure') {
        $passed = $runner.ExitCode -ne 0 -and $helperRan -and !$markerInstalled -and $runtimeText -match 'upgrade_close.failed'
    } else {
        $passed = $runner.ExitCode -eq 0 -and $helperRan -and $markerInstalled -and $runtimeText -match 'upgrade_close.completed'
    }
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
