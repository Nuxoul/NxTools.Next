param(
    [Parameter(Mandatory = $true)][ValidateSet('debug', 'release')][string]$Profile,
    [string]$TargetDirectory = '',
    [string]$ShellHelperDirectory = '',
    [string]$OutputPath = '',
    [string]$ArchiveOutputPath = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $TargetDirectory) { $TargetDirectory = Join-Path $root "target\$Profile" }
if (-not $ShellHelperDirectory) { throw 'Published ShellSelectionHelper directory is required before PlatformHost identity generation.' }
if (-not $OutputPath) { $OutputPath = Join-Path $root 'csharp\NxTools.PlatformHost\Generated\ReleaseBinaryManifest.g.cs' }
if (-not $ArchiveOutputPath) { $ArchiveOutputPath = Join-Path $root 'csharp\NxTools.PlatformHost\Generated\ArchiveReleaseManifest.g.cs' }
function Get-RequiredHash([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Trusted release binary is missing: $Path" }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
}
function Write-Generated([string]$Path, [string]$Content) {
    $destination = [IO.Path]::GetFullPath($Path)
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    $temporary = "$destination.$([Guid]::NewGuid().ToString('N')).tmp"
    try {
        [IO.File]::WriteAllText($temporary, $Content, [Text.UTF8Encoding]::new($false))
        Move-Item -LiteralPath $temporary -Destination $destination -Force
    } finally { Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue }
}
$indexerHash = Get-RequiredHash (Join-Path $TargetDirectory 'nxtools-indexer.exe')
$desktopHash = Get-RequiredHash (Join-Path $TargetDirectory 'nxtools-desktop.exe')
$browserHostHash = Get-RequiredHash (Join-Path $TargetDirectory 'nxtools-browser-host.exe')
$workerHash = Get-RequiredHash (Join-Path $TargetDirectory 'nxtools-archive-worker.exe')
$archiveRows = @('        ["NxTools.ArchiveWorker.exe"] = "' + $workerHash + '",')
foreach ($name in @('NxTools.ShellSelectionHelper.exe', 'NxTools.ShellSelectionHelper.dll', 'NxTools.ShellSelectionHelper.deps.json', 'NxTools.ShellSelectionHelper.runtimeconfig.json')) {
    $hash = Get-RequiredHash (Join-Path $ShellHelperDirectory $name)
    $archiveRows += '        ["' + $name + '"] = "' + $hash + '",'
}
# All inputs must exist before either generated output is changed.
Write-Generated $OutputPath @"
namespace NxTools.PlatformHost.Generated;
internal static class ReleaseBinaryManifest
{
    public const string IndexerSha256 = "$indexerHash";
    public const string DesktopSha256 = "$desktopHash";
    public const string BrowserHostSha256 = "$browserHostHash";
}
"@
Write-Generated $ArchiveOutputPath @"
namespace NxTools.PlatformHost.Generated;
internal static class ArchiveReleaseManifest
{
    internal static readonly IReadOnlyDictionary<string, string> Files = new Dictionary<string, string>(StringComparer.Ordinal)
    {
$($archiveRows -join "`n")
    };
}
"@
Write-Host "Embedded complete release binary hashes for profile '$Profile'."
