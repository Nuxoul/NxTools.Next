[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$DesktopPath,
    [Parameter(Mandatory = $true)][string]$OutputPath
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $DesktopPath -PathType Leaf)) { throw 'Actual Desktop output is required before helper compilation.' }
$hash = (Get-FileHash -LiteralPath $DesktopPath -Algorithm SHA256).Hash
$destination = [IO.Path]::GetFullPath($OutputPath)
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
$content = @"
// Generated from the actual Desktop output before helper compilation. Empty source fallback refuses.
namespace NxTools.ShellSelectionHelper.Generated;
internal static class DesktopReleaseIdentity
{
    internal const string Sha256 = "$hash";
}
"@
$temporary = "$destination.$([Guid]::NewGuid().ToString('N')).tmp"
try {
    [IO.File]::WriteAllText($temporary, $content, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $destination -Force
} finally { Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue }
Write-Host 'Bound ShellSelectionHelper to actual Desktop output.'
