param(
    [string]$Executable = "$PSScriptRoot\..\..\artifacts\win-x64\NxTools.exe",
    [int]$ProcessId = 0,
    [int]$Port = 38741,
    [string]$DownloadUrl = '',
    [long]$ExpectedBytes = 100000000,
    [string]$OutputDirectory = "$PSScriptRoot\..\..\artifacts\ui-automation"
)

$ErrorActionPreference = 'Stop'
$readyFile = $null
$fixture = $null
if ([string]::IsNullOrWhiteSpace($DownloadUrl)) {
    $readyFile = Join-Path ([System.IO.Path]::GetTempPath()) "nxtools-ui-http-$Port.ready"
    if (Test-Path -LiteralPath $readyFile) {
        Remove-Item -LiteralPath $readyFile -Force
    }
    $shellPath = (Get-Process -Id $PID).Path
    $fixture = Start-Process -FilePath $shellPath -WindowStyle Hidden -PassThru -ArgumentList @(
        '-NoProfile',
        '-File',
        (Join-Path $PSScriptRoot 'http-download-fixture.ps1'),
        '-Port',
        $Port,
        '-ReadyFile',
        $readyFile
    )
}

try {
    if ($null -ne $fixture) {
        $deadline = [DateTime]::UtcNow.AddSeconds(5)
        while (-not (Test-Path -LiteralPath $readyFile)) {
            if ($fixture.HasExited) {
                throw 'The deterministic HTTP fixture exited before becoming ready.'
            }
            if ([DateTime]::UtcNow -ge $deadline) {
                throw 'The deterministic HTTP fixture did not become ready within five seconds.'
            }
            Start-Sleep -Milliseconds 50
        }
        $DownloadUrl = "http://127.0.0.1:$Port/nxtools-ui-smoke.bin"
        $ExpectedBytes = 33554432
    }
    $env:NXTOOLS_UI_DOWNLOAD_URL = $DownloadUrl
    $downloadDirectory = Join-Path $OutputDirectory 'downloads'
    New-Item -ItemType Directory -Path $downloadDirectory -Force | Out-Null
    $env:NXTOOLS_UI_DOWNLOAD_DIRECTORY = (Resolve-Path -LiteralPath $downloadDirectory).Path
    $env:NXTOOLS_UI_DOWNLOAD_NAME = "nxtools-ui-$((Get-Date).ToString('yyyyMMdd-HHmmss')).bin"
    $env:NXTOOLS_UI_DOWNLOAD_PATH = Join-Path $env:NXTOOLS_UI_DOWNLOAD_DIRECTORY $env:NXTOOLS_UI_DOWNLOAD_NAME
    $env:NXTOOLS_UI_EXPECTED_BYTES = $ExpectedBytes.ToString(
        [System.Globalization.CultureInfo]::InvariantCulture)
    $arguments = @{
        Scenario = (Join-Path $PSScriptRoot 'download-ui-smoke.json')
        Executable = $Executable
        OutputDirectory = $OutputDirectory
    }
    if ($ProcessId -gt 0) {
        $arguments.ProcessId = $ProcessId
    }
    & (Join-Path $PSScriptRoot 'windows-ui-automation.ps1') @arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Remove-Item Env:NXTOOLS_UI_DOWNLOAD_URL -ErrorAction SilentlyContinue
    Remove-Item Env:NXTOOLS_UI_DOWNLOAD_DIRECTORY -ErrorAction SilentlyContinue
    Remove-Item Env:NXTOOLS_UI_DOWNLOAD_NAME -ErrorAction SilentlyContinue
    Remove-Item Env:NXTOOLS_UI_DOWNLOAD_PATH -ErrorAction SilentlyContinue
    Remove-Item Env:NXTOOLS_UI_EXPECTED_BYTES -ErrorAction SilentlyContinue
    if ($null -ne $fixture -and -not $fixture.HasExited) {
        Stop-Process -Id $fixture.Id -Force
        $fixture.WaitForExit()
    }
    if ($null -ne $readyFile -and (Test-Path -LiteralPath $readyFile)) {
        Remove-Item -LiteralPath $readyFile -Force
    }
}
