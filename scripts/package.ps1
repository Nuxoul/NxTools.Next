param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [string]$OutputPath = '',
    [switch]$SkipVerification
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
$profile = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
if (-not $OutputPath) {
    $OutputPath = if ($Configuration -eq 'Release') {
        'artifacts\win-x64'
    }
    else {
        'artifacts\win-x64-debug'
    }
}
$destination = Join-Path $root $OutputPath
$restartIndexerServiceOnFailure = $false
$packageCompleted = $false

function Copy-PackageFile([string]$Source, [string]$Destination) {
    for ($attempt = 0; $attempt -lt 75; $attempt++) {
        try {
            Copy-Item -LiteralPath $Source -Destination $Destination -Force -ErrorAction Stop
            return
        }
        catch [System.IO.IOException] {
            if ($attempt -eq 74) {
                throw
            }
            Start-Sleep -Milliseconds 200
        }
    }
}

function Stop-ExactPackagedProcess(
    [string]$ProcessName,
    [string]$ExpectedPath) {
    $fullExpectedPath = [System.IO.Path]::GetFullPath($ExpectedPath)
    $processes = Get-CimInstance Win32_Process -Filter "Name='$ProcessName'" `
        -ErrorAction SilentlyContinue
    foreach ($process in $processes) {
        if (-not $process.ExecutablePath) {
            continue
        }
        $fullProcessPath = [System.IO.Path]::GetFullPath($process.ExecutablePath)
        if ($fullProcessPath -ne $fullExpectedPath) {
            continue
        }
        Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
        Write-Host "Stopped packaged process $ProcessName (PID $($process.ProcessId))."
    }
}

Push-Location $root
try {
    if ($Configuration -eq 'Release') {
        Initialize-GpuiReleaseEnvironment
    }
    if (-not $SkipVerification) {
        & (Join-Path $PSScriptRoot 'build.ps1') -Configuration $Configuration
    }
    if ($Configuration -eq 'Release') {
        cargo build --release -p nxtools-desktop -p nxtools-indexer -p nxtools-cli
    }
    else {
        cargo build -p nxtools-desktop -p nxtools-indexer -p nxtools-cli
    }
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
    & (Join-Path $PSScriptRoot 'write-release-binary-manifest.ps1') `
        -Profile $profile
    dotnet publish '.\csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj' `
        -c $Configuration `
        -r win-x64 `
        --self-contained false `
        -p:PublishSingleFile=true
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet publish failed with exit code $LASTEXITCODE"
    }

    Stop-ExactPackagedProcess 'NxTools.exe' (Join-Path $destination 'NxTools.exe')
    Stop-ExactPackagedProcess 'NxTools.Cli.exe' (Join-Path $destination 'NxTools.Cli.exe')
    Stop-ExactPackagedProcess `
        'NxTools.PlatformHost.exe' `
        (Join-Path $destination 'NxTools.PlatformHost.exe')
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    $packagedIndexer = [System.IO.Path]::GetFullPath((Join-Path $destination 'NxTools.Indexer.exe'))
    $installedService = Get-CimInstance Win32_Service -Filter "Name='NxToolsIndexerV1'" `
        -ErrorAction SilentlyContinue
    if ($installedService) {
        $serviceExecutable = if ($installedService.PathName -match '^"([^"]+)"') {
            $Matches[1]
        }
        else {
            ($installedService.PathName -split '\s+', 2)[0]
        }
        if ([System.IO.Path]::GetFullPath($serviceExecutable) -eq $packagedIndexer -and
            $installedService.State -eq 'Running') {
            Stop-Service -Name $installedService.Name -Force
            $service = Get-Service -Name $installedService.Name
            $service.WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Stopped,
                [TimeSpan]::FromSeconds(15))
            $restartIndexerServiceOnFailure = $true
        }
    }
    Copy-PackageFile ".\target\$profile\nxtools-desktop.exe" (Join-Path $destination 'NxTools.exe')
    Copy-PackageFile ".\target\$profile\nxtools-indexer.exe" (Join-Path $destination 'NxTools.Indexer.exe')
    Copy-PackageFile ".\target\$profile\nxtools-cli.exe" (Join-Path $destination 'NxTools.Cli.exe')
    Copy-PackageFile ".\csharp\NxTools.PlatformHost\bin\$Configuration\net8.0-windows\win-x64\publish\NxTools.PlatformHost.exe" (Join-Path $destination 'NxTools.PlatformHost.exe')
    Copy-Item '.\LICENSE-MIT', '.\LICENSE-APACHE', '.\THIRD_PARTY_NOTICES.md' $destination
    $packageCompleted = $true
}
finally {
    try {
        if ($restartIndexerServiceOnFailure -and -not $packageCompleted) {
            Start-Service -Name 'NxToolsIndexerV1'
            $service = Get-Service -Name 'NxToolsIndexerV1'
            $service.WaitForStatus(
                [System.ServiceProcess.ServiceControllerStatus]::Running,
                [TimeSpan]::FromSeconds(15))
        }
    }
    finally {
        Pop-Location
    }
}

Write-Host "Package created at $destination"
if ($restartIndexerServiceOnFailure -and $packageCompleted) {
    Write-Host 'The legacy writable-path indexer service remains stopped; the next desktop launch will request one-time migration to the protected service directory.'
}
