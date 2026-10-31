param([Parameter(Mandatory=$true)][string]$EvidenceRoot)
$ErrorActionPreference='Stop'
$repositoryRoot=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$roundRoot=[IO.Path]::GetFullPath($EvidenceRoot)
$scratchRoot=[IO.Path]::GetFullPath($env:PI_SCRATCH_DIR).TrimEnd('\')+'\'
if(-not $roundRoot.StartsWith($scratchRoot,[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $roundRoot -PathType Container)){throw 'Explicit existing session scratch evidence root required.'}
function Require-Exit([string]$Log){if($LASTEXITCODE -ne 0){Get-Content -LiteralPath $Log -Tail 30;throw "Fixture build failed; see $Log"}}
function Replace-Once([string]$Path,[string]$Old,[string]$New){
    $value=[IO.File]::ReadAllText($Path)
    $first=$value.IndexOf($Old,[StringComparison]::Ordinal)
    if($first -lt 0 -or $value.IndexOf($Old,$first+$Old.Length,[StringComparison]::Ordinal) -ge 0){throw "Unexpected scratch fixture source: $Path"}
    [IO.File]::WriteAllText($Path,$value.Substring(0,$first)+$New+$value.Substring($first+$Old.Length),[Text.UTF8Encoding]::new($false))
}
$hostProject=Join-Path $repositoryRoot 'csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj'
$initialLog=Join-Path $roundRoot '01-host-initial.log'
& dotnet build $hostProject --no-restore -o (Join-Path $roundRoot 'host-initial') *> $initialLog
Require-Exit $initialLog
$lane=@(Get-CimInstance Win32_Process -Filter "Name = 'cargo.exe' OR Name = 'rustc.exe'")
if($lane.Count -gt 0){throw 'Shared Cargo lane occupied; do not start a duplicate build.'}
$env:CARGO_BUILD_JOBS='1';$env:CARGO_INCREMENTAL='0';$env:RUSTFLAGS='-C codegen-units=256'
Push-Location $repositoryRoot
try {
    $featureLog=Join-Path $roundRoot '02-worker-features.log'
    & cmd /d /c 'cargo tree -p nxtools-archive --features isolated-codecs -e features 2>&1' > $featureLog
    Require-Exit $featureLog
    $lane=@(Get-CimInstance Win32_Process -Filter "Name = 'cargo.exe' OR Name = 'rustc.exe'")
    if($lane.Count -gt 0){throw 'Shared Cargo lane occupied before matching worker rebuild.'}
    $workerLog=Join-Path $roundRoot '03-worker-build.log'
    & cmd /d /c 'cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs 2>&1' > $workerLog
    Require-Exit $workerLog
} finally {Pop-Location}
$identity=New-Item -ItemType Directory -Path (Join-Path $roundRoot 'identity-input')
foreach($pair in @(@('NxTools.exe','nxtools-desktop.exe'),@('NxTools.Indexer.exe','nxtools-indexer.exe'),@('NxTools.BrowserHost.exe','nxtools-browser-host.exe'))){Copy-Item -LiteralPath (Join-Path $repositoryRoot ('artifacts\win-x64\'+$pair[0])) -Destination (Join-Path $identity.FullName $pair[1])}
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'target\debug\nxtools-archive-worker.exe') -Destination (Join-Path $identity.FullName 'nxtools-archive-worker.exe')
& (Join-Path $repositoryRoot 'scripts\write-release-binary-manifest.ps1') -Profile debug -TargetDirectory $identity.FullName -ShellHelperDirectory (Join-Path $repositoryRoot 'artifacts\win-x64') -OutputPath (Join-Path $roundRoot 'ReleaseBinaryManifest.g.cs') -ArchiveOutputPath (Join-Path $roundRoot 'ArchiveReleaseManifest.g.cs') *> (Join-Path $roundRoot '04-original-identity-generator.log')
$sourceRoot=Join-Path $roundRoot 'fixture-source'
New-Item -ItemType Directory -Path $sourceRoot | Out-Null
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'Directory.Build.props') -Destination $sourceRoot
$sourceProjectRoot=Join-Path $repositoryRoot 'csharp\NxTools.PlatformHost'
$destinationProjectRoot=Join-Path $sourceRoot 'csharp\NxTools.PlatformHost'
Get-ChildItem -LiteralPath $sourceProjectRoot -Recurse -File | Where-Object {$_.FullName -notmatch '\\(bin|obj)\\'} | ForEach-Object {
    $relative=$_.FullName.Substring($sourceProjectRoot.Length).TrimStart('\')
    $destination=Join-Path $destinationProjectRoot $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
    Copy-Item -LiteralPath $_.FullName -Destination $destination
}
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'tests\integration\ArchiveCliLifecycleFixture.cs') -Destination $destinationProjectRoot
$program=Join-Path $destinationProjectRoot 'Program.cs'
Replace-Once $program 'using var archiveCliLog = PlatformLog.CreateDefault();' 'using var archiveCliLog = ArchiveCliLifecycleFixture.CreateLog();'
$hostSource=Join-Path $destinationProjectRoot 'Protocol\ArchiveCliHost.cs'
Replace-Once $hostSource 'jobs = new ArchiveJobSession(log);' "jobs = new ArchiveJobSession(log);`n            ArchiveCliLifecycleFixture.Configure(jobs, pipe);"
Replace-Once $hostSource 'if (outgoing.Writer.TryWrite(envelope)) return true;' "ArchiveCliLifecycleFixture.Observe(envelope);`n            if (outgoing.Writer.TryWrite(envelope)) return true;"
$fixtureLog=Join-Path $roundRoot '05-instrumented-host-build.log'
& dotnet build (Join-Path $destinationProjectRoot 'NxTools.PlatformHost.csproj') -o (Join-Path $roundRoot 'fixture-bin') "-p:ArchiveReleaseManifestPath=$(Join-Path $roundRoot 'ArchiveReleaseManifest.g.cs')" "-p:ReleaseBinaryManifestPath=$(Join-Path $roundRoot 'ReleaseBinaryManifest.g.cs')" *> $fixtureLog
Require-Exit $fixtureLog
Copy-Item -LiteralPath (Join-Path $repositoryRoot 'target\debug\nxtools-cli.exe') -Destination (Join-Path $roundRoot 'fixture-bin\NxTools.Cli.exe')
Copy-Item -LiteralPath (Join-Path $identity.FullName 'nxtools-archive-worker.exe') -Destination (Join-Path $roundRoot 'fixture-bin\NxTools.ArchiveWorker.exe')
$rows=@(foreach($name in @('NxTools.Cli.exe','NxTools.PlatformHost.dll','NxTools.ArchiveWorker.exe')){$path=Join-Path $roundRoot ('fixture-bin\'+$name);[pscustomobject]@{Name=$name;SHA256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash;Bytes=(Get-Item -LiteralPath $path).Length}})
$rows | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $roundRoot 'fixture-identities.json') -Encoding UTF8
$rows | ConvertTo-Json
Write-Output 'Prepared scratch-only checkpoint host; product CLI/source sessions/runner unchanged.'
