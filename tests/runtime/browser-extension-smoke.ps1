param(
    [string]$BrowserPath = '',
    [string]$ExtensionPath = "$PSScriptRoot\..\..\artifacts\win-x64\browser_extension\nxtools-interceptor",
    [string]$CliPath = "$PSScriptRoot\..\..\artifacts\win-x64\NxTools.Cli.exe",
    [string]$DownloadUrl = 'http://hkg.download.datapacket.com/100mb.bin',
    [long]$ExpectedBytes = 100000000,
    [string]$ExpectedSha256 = '',
    [string]$OutputDirectory = "$PSScriptRoot\..\..\artifacts\browser-automation"
)

$ErrorActionPreference = 'Stop'
$root = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$script = Join-Path $PSScriptRoot 'browser-extension-smoke.mjs'
if (-not $BrowserPath) {
    $playwrightChrome = Get-ChildItem `
        -Path (Join-Path $env:LOCALAPPDATA 'ms-playwright\chromium-*\chrome-win64\chrome.exe') `
        -File `
        -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    $BrowserPath = if ($playwrightChrome) {
        $playwrightChrome.FullName
    }
    else {
        'C:\Program Files\Google\Chrome\Application\chrome.exe'
    }
}
$arguments = @(
    $script,
    '--browser', [System.IO.Path]::GetFullPath($BrowserPath),
    '--extension', [System.IO.Path]::GetFullPath($ExtensionPath),
    '--cli', [System.IO.Path]::GetFullPath($CliPath),
    '--url', $DownloadUrl,
    '--output', [System.IO.Path]::GetFullPath($OutputDirectory)
)

Push-Location $root
try {
    $env:NXTOOLS_BROWSER_EXPECTED_BYTES = $ExpectedBytes.ToString(
        [System.Globalization.CultureInfo]::InvariantCulture)
    $env:NXTOOLS_BROWSER_EXPECTED_SHA256 = $ExpectedSha256
    & node @arguments
    if ($LASTEXITCODE -ne 0) {
        throw "Browser extension smoke failed with exit code $LASTEXITCODE."
    }
}
finally {
    Remove-Item Env:NXTOOLS_BROWSER_EXPECTED_BYTES -ErrorAction SilentlyContinue
    Remove-Item Env:NXTOOLS_BROWSER_EXPECTED_SHA256 -ErrorAction SilentlyContinue
    Pop-Location
}
