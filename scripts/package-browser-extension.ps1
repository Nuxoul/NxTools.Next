param(
    [Parameter(Mandatory = $true)]
    [string]$Destination
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$source = Join-Path $root 'browser_extension\nxtools-interceptor'
$destinationRoot = [System.IO.Path]::GetFullPath($Destination)
$sourcePackage = Join-Path $destinationRoot 'nxtools-interceptor'
$firefoxStage = Join-Path $destinationRoot 'nxtools-interceptor-firefox'

function Copy-ExtensionFiles([string]$Target) {
    New-Item -ItemType Directory -Force -Path $Target | Out-Null
    Get-ChildItem -LiteralPath $source -File -Recurse |
        Where-Object { $_.FullName -notlike "*\tests\*" } |
        ForEach-Object {
            $relative = [System.IO.Path]::GetRelativePath($source, $_.FullName)
            $targetFile = Join-Path $Target $relative
            $targetDirectory = Split-Path -Parent $targetFile
            New-Item -ItemType Directory -Force -Path $targetDirectory | Out-Null
            Copy-Item -LiteralPath $_.FullName -Destination $targetFile -Force
        }
}

New-Item -ItemType Directory -Force -Path $destinationRoot | Out-Null
Copy-ExtensionFiles $sourcePackage
Copy-ExtensionFiles $firefoxStage

$firefoxManifestPath = Join-Path $firefoxStage 'manifest.json'
$firefoxManifest = Get-Content -LiteralPath $firefoxManifestPath -Raw | ConvertFrom-Json -AsHashtable
$firefoxManifest.Remove('key')
$firefoxManifest['background'] = @{
    scripts = @('native-transport.js', 'handoff-policy.js', 'background.js')
}
$firefoxManifest['permissions'] = @(
    $firefoxManifest['permissions'] | Where-Object { $_ -ne 'declarativeNetRequest' }
)
$firefoxJson = $firefoxManifest | ConvertTo-Json -Depth 20
[System.IO.File]::WriteAllText(
    $firefoxManifestPath,
    $firefoxJson,
    [System.Text.UTF8Encoding]::new($false))

$chromiumZip = Join-Path $destinationRoot 'NxTools_Chrome_Edge_Extension.zip'
$firefoxXpi = Join-Path $destinationRoot 'NxTools_Firefox_Extension.xpi'
Compress-Archive -Path (Join-Path $sourcePackage '*') -DestinationPath $chromiumZip -Force
$firefoxZip = [System.IO.Path]::ChangeExtension($firefoxXpi, '.zip')
Compress-Archive -Path (Join-Path $firefoxStage '*') -DestinationPath $firefoxZip -Force
Move-Item -LiteralPath $firefoxZip -Destination $firefoxXpi -Force

Write-Host "Browser extension packages created at $destinationRoot"
