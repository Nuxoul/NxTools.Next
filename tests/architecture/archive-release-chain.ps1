param([switch]$SourceOnly)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$packagePath = Join-Path $root 'scripts\package.ps1'
$package = Get-Content -LiteralPath $packagePath -Raw
$publisher = Get-Content -LiteralPath (Join-Path $root 'scripts\publish-release-hosts.ps1') -Raw
$generator = Get-Content -LiteralPath (Join-Path $root 'scripts\write-release-binary-manifest.ps1') -Raw
$redirect = Get-Content -LiteralPath (Join-Path $root 'rust\apps\nxtools-desktop\src\main.rs') -Raw
foreach ($name in @('NxTools.ArchiveWorker.exe','NxTools.ShellSelectionHelper.exe','NxTools.ShellSelectionHelper.dll','NxTools.ShellSelectionHelper.deps.json','NxTools.ShellSelectionHelper.runtimeconfig.json')) {
    if (-not $package.Contains($name) -or -not $redirect.Contains($name)) { throw "Incomplete original package/redirect ownership: $name" }
}
foreach ($token in @('IsolatedStaging','SkipInstaller','STAGING-NOT-INSTALLABLE','-not $IsolatedStaging -and','--features isolated-codecs')) {
    if (-not $package.Contains($token)) { throw "Missing staging/worker boundary: $token" }
}
if ($publisher.IndexOf('write-shell-selection-identity.ps1') -gt $publisher.IndexOf('write-release-binary-manifest.ps1')) { throw 'Identity dependency order is cyclic or stale.' }
if (-not $generator.Contains('$ArchiveOutputPath') -or -not $publisher.Contains('ArchiveReleaseManifestPath')) { throw 'Explicit external generated approval is required.' }
if (-not $SourceOnly) {
    $before = @('ReleaseBinaryManifest.g.cs','UpdateTrust.g.cs') | ForEach-Object { Get-FileHash -LiteralPath (Join-Path $root "csharp\NxTools.PlatformHost\Generated\$_") }
    foreach ($parameters in @(
        @{IsolatedStaging=$true;OutputPath=(Join-Path $root 'artifacts\win-x64');SkipInstaller=$true;SkipVerification=$true},
        @{IsolatedStaging=$true;OutputPath=(Join-Path $env:PI_SCRATCH_DIR ('reject-'+[Guid]::NewGuid()));SkipVerification=$true},
        @{IsolatedStaging=$true;OutputPath=(Join-Path $env:PI_SCRATCH_DIR ('reject-'+[Guid]::NewGuid()));SkipInstaller=$true;SkipVerification=$true;LegacyUnsigned=$true}
    )) {
        $rejected = $false
        try { & $packagePath @parameters } catch { $rejected = $_.Exception.Message -like 'IsolatedStaging requires*' }
        if (-not $rejected) { throw 'Unsafe staging invocation was not rejected before build/trust mutation.' }
    }
    foreach ($entry in $before) { if ((Get-FileHash -LiteralPath $entry.Path).Hash -ne $entry.Hash) { throw 'Staging preflight mutated production generated input.' } }
}
Write-Output 'PASS archive release chain source ownership and unsafe staging rejection; no product/service/Registry/COM execution.'
