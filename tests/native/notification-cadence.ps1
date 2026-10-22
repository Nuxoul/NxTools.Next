param([switch]$AuthorizeNative)
$ErrorActionPreference='Stop'
if(-not $AuthorizeNative){throw 'Owned Debug notification sampling requires -AuthorizeNative.'}
if(-not $env:PI_SCRATCH_DIR){throw 'Session scratch required.'}
$workspaceRoot=(Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$fixtureExe=Join-Path $workspaceRoot 'target\debug\nxtools-desktop.exe'
$adapterDll=Join-Path $workspaceRoot 'csharp\NxTools.PlatformHost\bin\Debug\net8.0-windows\win-x64\NxTools.PlatformHost.dll'
if(-not (Test-Path $fixtureExe) -or -not (Test-Path $adapterDll)){throw 'Build explicit Debug fixtures first; this wrapper never builds the product.'}
$busy=Get-CimInstance Win32_Process|Where-Object{$_.Name -match '^(cargo|rustc|MSBuild)\.exe$' -or ($_.Name -eq 'dotnet.exe' -and $_.CommandLine -notmatch 'Unity|VBCSCompiler')}
if($busy){throw 'Another build/test is active; do not duplicate it.'}
$ownedRoot=Join-Path $env:PI_SCRATCH_DIR ('notification-cadence-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $ownedRoot | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'notification-cadence\relay.csproj') $ownedRoot
Copy-Item (Join-Path $PSScriptRoot 'notification-cadence\Program.cs') $ownedRoot
& dotnet build (Join-Path $ownedRoot 'relay.csproj') -p:UseSharedCompilation=false
if($LASTEXITCODE){throw 'Owned relay build failed.'}
$relay=Join-Path $ownedRoot 'bin\Debug\net8.0-windows\relay.exe'
$runs=@()
foreach($mode in @('legacy','current','reduced')){
 $directory=Join-Path $ownedRoot $mode
 & $relay --authorize-owned-native $fixtureExe $adapterDll $directory $mode dark
 if($LASTEXITCODE){throw "Owned relay failed: $mode"}
 $runs+=@{mode=$mode;directory=$directory}
}
$runFile=Join-Path $ownedRoot 'runs.json'
$runs|ConvertTo-Json|Set-Content -Encoding UTF8 $runFile
& python (Join-Path $PSScriptRoot 'notification-cadence\analyze.py') $runFile
if($LASTEXITCODE){throw 'Cadence comparison failed.'}
Write-Output "Owned evidence: $ownedRoot"
