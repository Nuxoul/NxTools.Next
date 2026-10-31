param([string]$EvidenceRoot=$env:PI_SCRATCH_DIR)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $root 'installer\bootstrap-migration.ps1') -LibraryOnly
$checks=0
function Check($Condition,$Label){if(!$Condition){throw "FAIL $Label"};$script:checks++;Write-Output "PASS $Label"}
$key='A'*64;$hash='B'*64;$path='C:\ProgramData\NxTools\NxTools.Next\Updater\Bootstrap\NxTools.PlatformHost.exe'
$registration=@{ImagePath='"'+$path+'" --trusted-update-service';ObjectName='LocalSystem';NxToolsUpdateKeySha256=$key;NxToolsBootstrapSha256=$hash}
Check ((Get-BootstrapMigrationDecision $key $null $path $null '' $false) -eq 'skip') 'new install skips maintenance'
Check ((Get-BootstrapMigrationDecision $key $registration $path $null $hash $false) -eq 'required') 'old approved updater needs migration'
$state=@{ProtocolVersion=1;Sequence=25;ReleaseId=[Guid]::NewGuid().ToString();ManifestSha256='C'*64;BootstrapSha256=$hash}
Check ((Get-BootstrapMigrationDecision $key $registration $path $state $hash $false) -eq 'skip') 'compatible updater skips elevation'
Check ((Get-BootstrapMigrationDecision $key $registration $path $state $hash $true) -eq 'required') 'interrupted maintenance resumes'
$bad=@{}+$registration;$bad.NxToolsUpdateKeySha256='D'*64;$rejected=$false
try{Get-BootstrapMigrationDecision $key $bad $path $null $hash $false|Out-Null}catch{$rejected=$true}
Check $rejected 'foreign publisher rejects'
$rejected=$false;try{Get-BootstrapMigrationDecision $key $registration $path $null ('E'*64) $false|Out-Null}catch{$rejected=$true}
Check $rejected 'tampered updater rejects'
$source=[IO.File]::ReadAllText((Join-Path $root 'installer\nxtools-next.iss'))
Check ($source.Contains('EnsureBootstrapMigration') -and $source.Contains('-BootstrapMigrationPackage')) 'normal installer owns migration before suspension'
Check ($source.IndexOf('EnsureBootstrapMigration') -lt $source.IndexOf("Log('installer.upgrade_close.started")) 'migration detection precedes old file mutation'
Check (!$source.Contains('BootstrapMigrationZip') -and !$source.Contains('NxTools-bootstrap-migration.zip') -and $source.Contains("ExtractTemporaryFiles('{app}\*')") -and $source.Contains('BootstrapMigrationHostSha256')) 'migration reuses normal installer payload without a second archive'
. (Join-Path $root 'installer\close-owned-processes.ps1') -LibraryOnly
$work=Join-Path $EvidenceRoot ('migration-call-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Path $work|Out-Null
$exe=Join-Path $work 'NxTools.PlatformHost.exe'
Add-Type -TypeDefinition 'public static class MigrationCallFixture { public static int Main(string[] args) { if (args.Length != 1 || args[0] != "--maintain-trusted-bootstrap") return 88; return System.IO.File.Exists(System.IO.Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory,"fail.keep")) ? 7 : 0; } }' -OutputAssembly $exe -OutputType ConsoleApplication
Invoke-UpgradeBootstrapMigration $work 'fixture-migration-success'|Out-Null
Check $true 'actual preparation migration call passes the exact explicit command'
[IO.File]::WriteAllText((Join-Path $work 'fail.keep'),'fail')
$rejected=$false;try{Invoke-UpgradeBootstrapMigration $work 'fixture-migration-failure'|Out-Null}catch{$rejected=$true}
Check $rejected 'actual preparation migration failure rejects before file suspension'
Write-Output "PASS installer bootstrap migration: $checks assertions; fake registration/read-only decisions/source wiring only"
