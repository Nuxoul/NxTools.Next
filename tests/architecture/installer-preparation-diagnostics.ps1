$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$source=[IO.File]::ReadAllText((Join-Path $root 'installer\close-owned-processes.ps1'))
$work=Join-Path $env:PI_SCRATCH_DIR ('installer-diagnostics-'+[Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($work)|Out-Null
$entry='if ($LibraryOnly) { return }'
if(([regex]::Matches($source,[regex]::Escape($entry))).Count -ne 1){throw 'Missing unique original helper entry'}
$checks=0
foreach($scenario in @('pending','private-error','access-denied','sink-unavailable','success','updater-cim-null','unknown-host-cim-null','known-stop-query-denied')) {
 $area=Join-Path $work $scenario
 $installed=Join-Path $area 'installed'
 [IO.Directory]::CreateDirectory($installed)|Out-Null
 $journal=Join-Path $installed '.NxTools.BrowserHost-upgrade.json'
 if($scenario -in @('pending','sink-unavailable')){[IO.File]::WriteAllText(($journal+'.pending'),'foreign recovery marker')}
 $manifest=Join-Path $area 'manifest.json'
 $names=@('NxTools.exe','NxTools.PlatformHost.exe','NxTools.Indexer.exe','NxTools.Cli.exe','NxTools.BrowserHost.exe','NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe','NxTools.ShellSelectionHelper.dll','NxTools.ShellSelectionHelper.deps.json','NxTools.ShellSelectionHelper.runtimeconfig.json')
 $members=@(foreach($name in $names){@{path=$name;sha256=('A'*64)}})
 [IO.File]::WriteAllText($manifest,(@{files=$members}|ConvertTo-Json -Depth 5))
 $signature=Join-Path $area 'signature.bin';[IO.File]::WriteAllText($signature,'synthetic signature; never trusted/deployed')
 $discovery=@'
function Test-Path { param($LiteralPath,$Path,$PathType)
 $candidate=if($LiteralPath){$LiteralPath}else{$Path}
 if($candidate -eq 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1'){return $false}
 $parameters=@{LiteralPath=$candidate}
 if($PathType){$parameters.PathType=$PathType}
 Microsoft.PowerShell.Management\Test-Path @parameters
}
function Get-UpgradeTargets { param($InstallRoot,$ProtectedRoot,$Approval) return @{} }
function Stop-UpgradeTargets { param($Targets,$ProtectedRoot) return 0 }
'@
 if($scenario -eq 'private-error'){$discovery+="`r`nfunction Stop-UpgradeTargets { param(`$Targets,`$ProtectedRoot) throw [IO.InvalidDataException]::new('SECRET-DIAGNOSTIC-PAYLOAD C:\\private\\member password') }`r`n"}
 if($scenario -eq 'access-denied'){$discovery+="`r`nfunction Stop-UpgradeTargets { param(`$Targets,`$ProtectedRoot) throw [ComponentModel.Win32Exception]::new(5) }`r`n"}
 if($scenario -in @('updater-cim-null','unknown-host-cim-null')) {
  $discovery=$discovery.Replace('function Stop-UpgradeTargets { param($Targets,$ProtectedRoot) return 0 }','')
  $verified=if($scenario -eq 'updater-cim-null'){7001}else{7002}
  $discovery+="`r`nfunction Get-CimInstance { param(`$ClassName,`$Filter) if (`$Filter -eq `"Name='NxTools.PlatformHost.exe'`") { [pscustomobject]@{Name='NxTools.PlatformHost.exe';ProcessId=7001;ExecutablePath=`$null} } }`r`nfunction Get-UpgradeUpdaterProcessId { return $verified }`r`n"
  $discovery+=@'

function Initialize-UpgradeNative {
 Add-Type -TypeDefinition 'public static class NxToolsUpgradeProcess { public static string QueryImage(int id) { var failure=new System.ComponentModel.Win32Exception(5); failure.Data["native_api"]="open"; throw failure; } }'
}
'@
 }
 if($scenario -eq 'known-stop-query-denied') {
  $discovery=$discovery.Replace('function Stop-UpgradeTargets { param($Targets,$ProtectedRoot) return 0 }','')
  $discovery=$discovery.Replace('function Get-UpgradeTargets { param($InstallRoot,$ProtectedRoot,$Approval) return @{} }','function Get-UpgradeTargets { param($InstallRoot,$ProtectedRoot,$Approval) return @{(Join-Path $InstallRoot ''NxTools.exe'')=''installed''} }')
  $discovery+=@'

function Get-CimInstance { param($ClassName,$Filter) if($Filter -eq "Name='NxTools.exe'"){[pscustomobject]@{ProcessId=7001;ExecutablePath=(Join-Path $InstallDirectory 'NxTools.exe')}} }
function Initialize-UpgradeNative {
 Add-Type -TypeDefinition 'public static class NxToolsUpgradeProcess { public static bool Stop(int id,string expected) { var failure=new System.ComponentModel.Win32Exception(5); failure.Data["native_api"]="query_image"; throw failure; } }'
}
'@
 }
 $fixture=Join-Path $area 'helper.ps1'
 [IO.File]::WriteAllText($fixture,$source.Replace($entry,$discovery+"`r`n"+$entry),[Text.UTF8Encoding]::new($true))
 $diagnostic=Join-Path $area 'preparation.jsonl'
 if($scenario -eq 'sink-unavailable'){[IO.Directory]::CreateDirectory($diagnostic)|Out-Null}
 $info=New-Object Diagnostics.ProcessStartInfo
 $info.FileName='C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe'
 $info.Arguments='-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "'+$fixture+'" -InstallDirectory "'+$installed+'" -StatePath "'+$journal+'" -CorrelationId "fixture-'+$scenario+'" -NewManifestPath "'+$manifest+'" -NewSignaturePath "'+$signature+'" -DiagnosticPath "'+$diagnostic+'"'
 $info.UseShellExecute=$false;$info.CreateNoWindow=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
 $child=New-Object Diagnostics.Process;$child.StartInfo=$info
 if(!$child.Start()){throw 'Fixture child did not start'}
 $stdoutTask=$child.StandardOutput.ReadToEndAsync();$stderrTask=$child.StandardError.ReadToEndAsync()
 if(!$child.WaitForExit(20000)){throw 'Fixture child timed out'};$child.WaitForExit()
 [IO.File]::WriteAllText((Join-Path $area 'stdout.log'),$stdoutTask.Result);[IO.File]::WriteAllText((Join-Path $area 'stderr.log'),$stderrTask.Result)
 $expected=if($scenario -in @('success','updater-cim-null')){0}elseif($scenario -in @('access-denied','unknown-host-cim-null','known-stop-query-denied')){5}else{1}
 if($child.ExitCode -ne $expected){throw "FAIL $scenario exit expected=$expected actual=$($child.ExitCode)"}
 if($scenario -eq 'sink-unavailable'){
  if([IO.File]::ReadAllText(($journal+'.pending')) -ne 'foreign recovery marker'){throw 'Diagnostic failure changed recovery marker'}
 } else {
  if(!(Test-Path -LiteralPath $diagnostic -PathType Leaf)){throw "FAIL $scenario missing persistent preparation diagnostics; actual helper exit=$($child.ExitCode)"}
  $raw=[IO.File]::ReadAllText($diagnostic)
  if($raw -match 'SECRET-DIAGNOSTIC-PAYLOAD|password|private\\member' -or $raw.Contains($installed)){throw 'Diagnostics exposed private error/path payload'}
  $records=@(Get-Content -LiteralPath $diagnostic -Encoding UTF8|ForEach-Object{$_|ConvertFrom-Json})
  $terminal=$records[-1]
  if($records.Count -lt 2 -or $records.Count -gt 16 -or (Get-Item -LiteralPath $diagnostic).Length -gt 65536){throw 'Diagnostic bound/count mismatch'}
  if($terminal.correlation -ne ('fixture-'+$scenario) -or $terminal.elapsed_ms -lt 0){throw 'Missing original correlation/elapsed'}
  if($terminal.elevated -isnot [bool]){throw 'Missing actual token elevation fact'}
  if($scenario -eq 'unknown-host-cim-null' -and ($terminal.process_id-ne 7001 -or $terminal.process_name-ne 'NxTools.PlatformHost.exe' -or $terminal.native_api-ne 'open')){throw 'CIM-null failure missing bounded native identity diagnostics'}
  if($scenario -eq 'known-stop-query-denied' -and ($terminal.process_id-ne 7001 -or $terminal.process_name-ne 'NxTools.exe' -or $terminal.native_api-ne 'query_image')){throw 'Known-image Stop failure missing native PID/API diagnostics'}
  if($scenario -in @('success','updater-cim-null')){
   if($terminal.event -ne 'completed' -or (Get-Content -LiteralPath $journal -Raw|ConvertFrom-Json).phase -ne 'suspended'){throw 'Missing actual successful preparation terminal'}
  } else {
   if($terminal.event -ne 'failed' -or !$terminal.exception_type -or $terminal.script_line -le 0){throw 'Missing actual failure type/line'}
   $expectedStage=if($scenario -eq 'pending'){'file_preparation'}else{'process_close'}
   if($terminal.stage -ne $expectedStage -or $terminal.native_code -ne $(if($scenario -in @('access-denied','unknown-host-cim-null','known-stop-query-denied')){5}else{0})){throw 'Wrong failure stage/native code'}
   if($scenario -eq 'pending' -and [IO.File]::ReadAllText(($journal+'.pending')) -ne 'foreign recovery marker'){throw 'Failure diagnostics changed recovery evidence'}
  }
 }
 $checks++;Write-Output "PASS actual helper diagnostics $scenario exit=$($child.ExitCode)"
}
Write-Output "PASS preparation diagnostics $checks scenarios; no real process/service/installed mutation; evidence=$work"
