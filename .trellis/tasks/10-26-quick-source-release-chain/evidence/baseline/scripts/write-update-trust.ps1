[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$SigningKeyPath,
    [string]$OutputPath = (Join-Path $PSScriptRoot '..\csharp\NxTools.PlatformHost\Generated\UpdateTrust.g.cs')
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $SigningKeyPath -PathType Leaf)) { throw 'An existing explicit signing PEM is required; this script never creates a key.' }
$projectPath = Join-Path $PSScriptRoot '..\csharp\NxTools.PlatformHost.ReleaseSigning\NxTools.PlatformHost.ReleaseSigning.csproj'
$buildRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$buildPath = Join-Path $buildRoot ('rs-' + [Guid]::NewGuid().ToString('N').Substring(0, 12))
try {
    & dotnet build $projectPath --configuration Release --artifacts-path $buildPath --nologo
    if ($LASTEXITCODE -ne 0) { throw 'Release signing helper build failed.' }
    $helperPath = Join-Path $buildPath 'bin\NxTools.PlatformHost.ReleaseSigning\release\NxTools.PlatformHost.ReleaseSigning.dll'
    & dotnet $helperPath trust ([IO.Path]::GetFullPath($SigningKeyPath)) ([IO.Path]::GetFullPath($OutputPath))
    if ($LASTEXITCODE -ne 0) { throw 'Public release trust generation failed.' }
} finally {
    if (Test-Path -LiteralPath $buildPath) { Remove-Item -LiteralPath $buildPath -Recurse -Force }
}
