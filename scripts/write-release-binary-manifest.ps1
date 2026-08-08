param(
    [ValidateSet('debug', 'release')]
    [string]$Profile
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$targetDirectory = Join-Path $root "target\$Profile"
$indexer = Join-Path $targetDirectory 'nxtools-indexer.exe'
$desktop = Join-Path $targetDirectory 'nxtools-desktop.exe'
$browserHost = Join-Path $targetDirectory 'nxtools-browser-host.exe'
foreach ($path in @($indexer, $desktop, $browserHost)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Trusted release binary is missing: $path"
    }
}

$indexerHash = (Get-FileHash -LiteralPath $indexer -Algorithm SHA256).Hash
$desktopHash = (Get-FileHash -LiteralPath $desktop -Algorithm SHA256).Hash
$browserHostHash = (Get-FileHash -LiteralPath $browserHost -Algorithm SHA256).Hash
$content = @"
namespace NxTools.PlatformHost.Generated;

internal static class ReleaseBinaryManifest
{
    public const string IndexerSha256 =
        "$indexerHash";
    public const string DesktopSha256 =
        "$desktopHash";
    public const string BrowserHostSha256 =
        "$browserHostHash";
}
"@

$destination = Join-Path $root `
    'csharp\NxTools.PlatformHost\Generated\ReleaseBinaryManifest.g.cs'
$temporary = "$destination.$([Guid]::NewGuid().ToString('N')).tmp"
try {
    [System.IO.File]::WriteAllText(
        $temporary,
        $content,
        [System.Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $destination -Force
}
finally {
    Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
}

Write-Host "Embedded trusted Rust binary hashes for profile '$Profile'."
