[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$KeyPath,
    [ValidateSet(3072, 4096)][int]$KeySize = 3072
)
$ErrorActionPreference = 'Stop'
$repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$absoluteKeyPath = [IO.Path]::GetFullPath($KeyPath)
if (Test-Path -LiteralPath $absoluteKeyPath) { throw 'Refusing to overwrite existing signing key.' }
if (-not (Test-Path -LiteralPath ([IO.Path]::GetDirectoryName($absoluteKeyPath)) -PathType Container)) { throw 'Choose an existing private directory outside the repository and artifacts.' }
$projectPath = Join-Path $repositoryRoot 'csharp\NxTools.PlatformHost.ReleaseSigning\NxTools.PlatformHost.ReleaseSigning.csproj'
# Explicit provisioning only. Never called by packaging or trust generation.
# The helper creates the file with a protected owner-only ACL before writing PKCS8 bytes.
$buildRoot = if ($env:PI_SCRATCH_DIR) { $env:PI_SCRATCH_DIR } else { [IO.Path]::GetTempPath() }
$buildPath = Join-Path $buildRoot ('rs-' + [Guid]::NewGuid().ToString('N').Substring(0, 12))
try {
    & dotnet build $projectPath --configuration Release --artifacts-path $buildPath --nologo
    if ($LASTEXITCODE -ne 0) { throw 'Release signing helper build failed.' }
    $helperPath = Join-Path $buildPath 'bin\NxTools.PlatformHost.ReleaseSigning\release\NxTools.PlatformHost.ReleaseSigning.dll'
    & dotnet $helperPath new-key $absoluteKeyPath $repositoryRoot $KeySize.ToString([Globalization.CultureInfo]::InvariantCulture)
    if ($LASTEXITCODE -ne 0) { throw 'Release key provisioning failed.' }
} finally {
    if (Test-Path -LiteralPath $buildPath) { Remove-Item -LiteralPath $buildPath -Recurse -Force }
}
