[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackageDirectory,
    [Parameter(Mandatory = $true)][string]$SigningKeyPath,
    [Parameter(Mandatory = $true)][ValidateRange(1, [long]::MaxValue)][long]$ReleaseSequence,
    [Guid]$ReleaseId = [Guid]::NewGuid()
)
$ErrorActionPreference = 'Stop'
if ($ReleaseId -eq [Guid]::Empty) { throw 'ReleaseId must not be empty.' }
if (-not (Test-Path -LiteralPath $SigningKeyPath -PathType Leaf)) { throw 'An existing explicit signing PEM is required.' }
if (-not (Test-Path -LiteralPath $PackageDirectory -PathType Container)) { throw 'PackageDirectory must exist.' }
$projectPath = Join-Path $PSScriptRoot '..\csharp\NxTools.PlatformHost.ReleaseSigning\NxTools.PlatformHost.ReleaseSigning.csproj'
# PS5/.NET Framework cannot import PEM or sign RSA-PSS. The isolated net8 helper links the production verifier.
# Existing sidecars are deliberately not overwritten: the packaging caller owns removing obsolete outputs.
$buildRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$buildPath = Join-Path $buildRoot ('rs-' + [Guid]::NewGuid().ToString('N').Substring(0, 12))
try {
    & dotnet build $projectPath --configuration Release --artifacts-path $buildPath --nologo
    if ($LASTEXITCODE -ne 0) { throw 'Release signing helper build failed.' }
    $helperPath = Join-Path $buildPath 'bin\NxTools.PlatformHost.ReleaseSigning\release\NxTools.PlatformHost.ReleaseSigning.dll'
    & dotnet $helperPath sign ([IO.Path]::GetFullPath($PackageDirectory)) ([IO.Path]::GetFullPath($SigningKeyPath)) $ReleaseSequence.ToString([Globalization.CultureInfo]::InvariantCulture) $ReleaseId.ToString('D')
    if ($LASTEXITCODE -ne 0) { throw 'Trusted release signing failed; no unsigned fallback is allowed.' }
} finally {
    if (Test-Path -LiteralPath $buildPath) { Remove-Item -LiteralPath $buildPath -Recurse -Force }
}
