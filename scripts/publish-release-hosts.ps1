[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('Debug','Release')][string]$Configuration,
    [Parameter(Mandatory = $true)][string]$GeneratedDirectory,
    [Parameter(Mandatory = $true)][string]$BuildDirectory,
    [string]$TargetDirectory = '',
    [switch]$SkipPlatformHost,
    [string]$UpdateTrustPath = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$profile = $Configuration.ToLowerInvariant()
if (-not $TargetDirectory) { $TargetDirectory = Join-Path $root "target\$profile" }
# Both directories are borrowed outputs. Their caller owns cleanup after test builds
# and package copies have consumed the generated inputs and published binaries.
New-Item -ItemType Directory -Force -Path $GeneratedDirectory | Out-Null
$identityPath = Join-Path $GeneratedDirectory 'DesktopIdentity.g.cs'
& (Join-Path $PSScriptRoot 'write-shell-selection-identity.ps1') -DesktopPath (Join-Path $TargetDirectory 'nxtools-desktop.exe') -OutputPath $identityPath
$helperPublish = Join-Path $BuildDirectory 'shell-publish'
& dotnet publish (Join-Path $root 'csharp\NxTools.ShellSelectionHelper\NxTools.ShellSelectionHelper.csproj') -c $Configuration -r win-x64 --self-contained false -p:PublishSingleFile=false "-p:ShellSelectionIdentityPath=$identityPath" --artifacts-path (Join-Path $BuildDirectory 'shell-build') -o $helperPublish --nologo
if ($LASTEXITCODE -ne 0) { throw "ShellSelectionHelper publish failed: $LASTEXITCODE" }
$releasePath = Join-Path $GeneratedDirectory 'ReleaseBinaryManifest.g.cs'
$archivePath = Join-Path $GeneratedDirectory 'ArchiveReleaseManifest.g.cs'
& (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') -Profile $profile -TargetDirectory $TargetDirectory -ShellHelperDirectory $helperPublish -OutputPath $releasePath -ArchiveOutputPath $archivePath
if (-not $SkipPlatformHost) {
    $publishArgs = @('publish', (Join-Path $root 'csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj'), '-c', $Configuration, '-r', 'win-x64', '--self-contained', 'false', '-p:PublishSingleFile=true', "-p:ReleaseBinaryManifestPath=$releasePath", "-p:ArchiveReleaseManifestPath=$archivePath", '--artifacts-path', (Join-Path $BuildDirectory 'host-build'), '-o', (Join-Path $BuildDirectory 'host-publish'), '--nologo')
    if ($UpdateTrustPath) { $publishArgs += "-p:UpdateTrustPath=$UpdateTrustPath" }
    & dotnet @publishArgs
    if ($LASTEXITCODE -ne 0) { throw "PlatformHost publish failed: $LASTEXITCODE" }
}
