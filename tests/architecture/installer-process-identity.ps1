param([string]$HelperPath = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (!$HelperPath) { $HelperPath = Join-Path $root 'installer\close-owned-processes.ps1' }
. $HelperPath -LibraryOnly
# Actual original close/identity owners; no real discovery, stop, Registry or installed mutation.
$script:resolver = ${function:Resolve-UpgradeUpdaterProcessId}
function Initialize-UpgradeNative { }
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
public static class NxToolsUpgradeProcess {
 public static readonly List<int> Stops = new List<int>();
 public static readonly List<int> Queries = new List<int>();
 public static string Image;
 public static bool Gone;
 public static bool Stop(int id,string expected) { Stops.Add(id); return true; }
 public static string QueryImage(int id) { Queries.Add(id); if(Gone) return null; if(!String.IsNullOrEmpty(Image)) return Image; throw new System.ComponentModel.Win32Exception(5); }
}
'@
$script:fixtureName='NxTools.PlatformHost.exe'
$script:fixturePid=7001
$script:fixtureImage=$null
$script:identityQueries=0
$script:updaterPid=7001
$script:updaterFailure=$false
$script:stoppedUpdaterProof=$false
function Get-CimInstance {
 param($ClassName,$Filter)
 if ($Filter -eq ("Name='"+$script:fixtureName+"'")) {
  if ([NxToolsUpgradeProcess]::Stops.Contains($script:fixturePid)) { return @() }
  return [pscustomobject]@{Name=$script:fixtureName;ProcessId=$script:fixturePid;ExecutablePath=$script:fixtureImage}
 }
 return @()
}
function Get-UpgradeUpdaterProcessId {
 $script:identityQueries++
 if ($script:stoppedUpdaterProof) { return &$script:resolver $script:stoppedRegistration $script:stoppedBootstrap ('B'*64) @(0,1,16) }
 if ($script:updaterFailure) { throw [IO.InvalidDataException]::new('Fixture updater identity rejected.') }
 return $script:updaterPid
}
$script:checks=0
function Check($Condition,[string]$Label) {
 if (!$Condition) { throw "FAIL $Label" }
 $script:checks++; Write-Output "PASS $Label"
}
$targets=@{'C:\owned\NxTools.PlatformHost.exe'='installed'}
try { $count=Stop-UpgradeTargets $targets 'C:\protected' }
catch { throw "RED original process-close rejects verified SYSTEM updater with CIM-null image: $($_.Exception.GetType().Name)" }
Check ($count -eq 0 -and $script:identityQueries -eq 1 -and [NxToolsUpgradeProcess]::Stops.Count -eq 0) 'CIM-null verified updater is preserved, never opened for termination'
foreach($scenario in @('different-pid','no-service','invalid-service','unreadable-desktop')) {
 $script:updaterPid=if($scenario-eq 'different-pid'){7002}elseif($scenario-eq 'no-service'){0}else{7001}
 $script:updaterFailure=$scenario-eq 'invalid-service'
 $script:fixtureName=if($scenario-eq 'unreadable-desktop'){'NxTools.exe'}else{'NxTools.PlatformHost.exe'}
 $failure=$null
 try { Stop-UpgradeTargets $targets 'C:\protected'|Out-Null } catch { $failure=$_.Exception; while($failure.InnerException){$failure=$failure.InnerException} }
 Check ($null-ne $failure -and [NxToolsUpgradeProcess]::Stops.Count-eq 0) "$scenario cannot bypass unknown process identity"
 if($scenario-ne 'invalid-service'){Check ($failure.NativeErrorCode-eq 5) "$scenario retains real access-denied outcome"}
}
$script:fixtureName='NxTools.PlatformHost.exe';$script:updaterFailure=$false;$script:updaterPid=7001
$script:fixtureImage='C:\foreign\NxTools.PlatformHost.exe'
Check ((Stop-UpgradeTargets $targets)-eq 0 -and [NxToolsUpgradeProcess]::Stops.Count-eq 0) 'readable unrelated same-name image stays untouched'
$script:fixtureImage='C:\owned\NxTools.PlatformHost.exe'
Check ((Stop-UpgradeTargets $targets)-eq 1 -and [NxToolsUpgradeProcess]::Stops.Count-eq 1) 'ordinary exact owned Host still follows original stop boundary'
$bootstrap='C:\ProgramData\NxTools\NxTools.Next\Updater\Bootstrap\NxTools.PlatformHost.exe'
$registration=@{ImagePath='"'+$bootstrap+'" --trusted-update-service';ObjectName='LocalSystem';NxToolsUpdateKeySha256=('A'*64);NxToolsBootstrapSha256=('B'*64)}
Check ((&$script:resolver $registration $bootstrap ('B'*64) @(7001,4,16))-eq 7001) 'original fixed registration/hash and SCM running own-process PID accepted'
Check ((&$script:resolver $null $bootstrap '' @())-eq 0) 'absent updater has no excluded PID'
Check ((&$script:resolver $registration $bootstrap ('B'*64) @(0,1,16))-eq 0) 'validated stopped Demand updater has no process exclusion'
foreach($scenario in @('wrong-path','wrong-args','wrong-account','bad-key','bad-hash','stopped','shared','zero-pid','overflow-pid','missing-status','stopped-bad-hash','stopped-wrong-path','stopped-shared')) {
 $copy=$registration.Clone();$status=@(7001,4,16);$actual='B'*64
 switch($scenario){
 'wrong-path'{$copy.ImagePath='"C:\foreign\NxTools.PlatformHost.exe" --trusted-update-service'}
 'wrong-args'{$copy.ImagePath='"'+$bootstrap+'"'}
 'wrong-account'{$copy.ObjectName='CurrentUser'}
 'bad-key'{$copy.NxToolsUpdateKeySha256='invalid'}
 'bad-hash'{$actual='C'*64}
 'stopped'{$status=@(7001,1,16)}
 'shared'{$status=@(7001,4,32)}
 'zero-pid'{$status=@(0,4,16)}
 'overflow-pid'{$status=@([uint32]::MaxValue,4,16)}
 'missing-status'{$status=@(7001)}
 'stopped-bad-hash'{$status=@(0,1,16);$actual='C'*64}
 'stopped-wrong-path'{$status=@(0,1,16);$copy.ImagePath='"C:\foreign\NxTools.PlatformHost.exe" --trusted-update-service'}
 'stopped-shared'{$status=@(0,1,32)}
 }
 $rejected=$false
 try { &$script:resolver $copy $bootstrap $actual $status|Out-Null } catch { $rejected=$true }
 Check $rejected "fixed updater proof rejects $scenario"
}
$script:stoppedUpdaterProof=$true;$script:stoppedRegistration=$registration;$script:stoppedBootstrap=$bootstrap
$script:fixtureName='NxTools.PlatformHost.exe';$script:fixtureImage=$null
[NxToolsUpgradeProcess]::Stops.Clear();[NxToolsUpgradeProcess]::Queries.Clear();[NxToolsUpgradeProcess]::Image='C:\owned\NxTools.PlatformHost.exe'
Check ((Stop-UpgradeTargets $targets)-eq 1 -and [NxToolsUpgradeProcess]::Queries.Count-eq 1 -and [NxToolsUpgradeProcess]::Stops.Count-eq 1) 'stopped updater continues CIM-null owned Host through exact native query and stop'
[NxToolsUpgradeProcess]::Stops.Clear();[NxToolsUpgradeProcess]::Queries.Clear();[NxToolsUpgradeProcess]::Image='C:\foreign\NxTools.PlatformHost.exe'
Check ((Stop-UpgradeTargets $targets)-eq 0 -and [NxToolsUpgradeProcess]::Queries.Count-eq 1 -and [NxToolsUpgradeProcess]::Stops.Count-eq 0) 'stopped updater does not authorize stopping a foreign Host'
[NxToolsUpgradeProcess]::Queries.Clear();[NxToolsUpgradeProcess]::Gone=$true
Check ((Stop-UpgradeTargets $targets)-eq 0 -and [NxToolsUpgradeProcess]::Queries.Count-eq 1 -and [NxToolsUpgradeProcess]::Stops.Count-eq 0) 'stopped updater permits original native proof of an exited Host'
[NxToolsUpgradeProcess]::Gone=$false;[NxToolsUpgradeProcess]::Image=$null
$failure=$null
try { Stop-UpgradeTargets $targets|Out-Null } catch { $failure=$_.Exception;while($failure.InnerException){$failure=$failure.InnerException} }
Check ($failure -is [ComponentModel.Win32Exception] -and $failure.NativeErrorCode-eq 5 -and $failure.Data['process_id']-eq $script:fixturePid -and [NxToolsUpgradeProcess]::Stops.Count-eq 0) 'stopped updater preserves actual live candidate query failure and PID attribution'
Write-Output "PASS installer process identity $script:checks checks; only fake discovery/native stops, no real Windows mutation"
