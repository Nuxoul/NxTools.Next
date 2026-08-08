param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',
    [string]$OutputPath = '',
    [switch]$SkipVerification,
    [ValidateRange(0.0, 1024.0)]
    [double]$MinimumCommitHeadroomGiB = 6.0
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'windows-build-tools.ps1')
Initialize-LowMemoryRustBuildEnvironment
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
    $stoppedAny = $false
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
        Wait-Process -Id $process.ProcessId -Timeout 5 -ErrorAction SilentlyContinue
        $stoppedAny = $true
        Write-Host "Stopped packaged process $ProcessName (PID $($process.ProcessId))."
    }
    return $stoppedAny
}

function Recover-ExactPackageCommitHeadroom(
    [double]$MinimumGiB,
    [string]$PackageDestination) {
    $beforeGiB = Get-ReleaseBuildCommitHeadroomGiB
    if ($beforeGiB -ge $MinimumGiB) {
        return $false
    }

    Write-Host ('Release commit headroom is below the safe threshold; stopping only exact ' +
        'package-path processes before the final preflight.')
    $stoppedAny = $false
    foreach ($entry in @(
            @{ Name = 'NxTools.exe'; File = 'NxTools.exe' },
            @{ Name = 'NxTools.Cli.exe'; File = 'NxTools.Cli.exe' },
            @{ Name = 'NxTools.BrowserHost.exe'; File = 'NxTools.BrowserHost.exe' },
            @{ Name = 'NxTools.PlatformHost.exe'; File = 'NxTools.PlatformHost.exe' })) {
        if (Stop-ExactPackagedProcess $entry.Name (Join-Path $PackageDestination $entry.File)) {
            $stoppedAny = $true
        }
    }

    if ($stoppedAny) {
        Start-Sleep -Milliseconds 500
        $afterGiB = Get-ReleaseBuildCommitHeadroomGiB
        Write-Host ('Release commit headroom after exact-package recovery: {0:N2} GiB ' +
            '(before: {1:N2} GiB).' -f $afterGiB, $beforeGiB)
    }
    return $stoppedAny
}

Push-Location $root
try {
    if ($Configuration -eq 'Release') {
        Initialize-GpuiReleaseEnvironment
        $null = Recover-ExactPackageCommitHeadroom `
            -MinimumGiB $MinimumCommitHeadroomGiB `
            -PackageDestination $destination
        Assert-ReleaseBuildCommitHeadroom -MinimumGiB $MinimumCommitHeadroomGiB
    }
    if (-not $SkipVerification) {
        & (Join-Path $PSScriptRoot 'build.ps1') `
            -Configuration $Configuration `
            -MinimumCommitHeadroomGiB $MinimumCommitHeadroomGiB
    }
    else {
        if ($Configuration -eq 'Release') {
            cargo build --release --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        }
        else {
            cargo build --jobs 1 -p nxtools-desktop -p nxtools-indexer -p nxtools-cli -p nxtools-browser-host
        }
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
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

    $null = Stop-ExactPackagedProcess 'NxTools.exe' (Join-Path $destination 'NxTools.exe')
    $null = Stop-ExactPackagedProcess 'NxTools.Cli.exe' (Join-Path $destination 'NxTools.Cli.exe')
    $null = Stop-ExactPackagedProcess `
        'NxTools.BrowserHost.exe' `
        (Join-Path $destination 'NxTools.BrowserHost.exe')
    $null = Stop-ExactPackagedProcess `
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
    Copy-PackageFile ".\target\$profile\nxtools-browser-host.exe" (Join-Path $destination 'NxTools.BrowserHost.exe')
    Copy-PackageFile ".\csharp\NxTools.PlatformHost\bin\$Configuration\net8.0-windows\win-x64\publish\NxTools.PlatformHost.exe" (Join-Path $destination 'NxTools.PlatformHost.exe')
    $packagedBrowserExtensionRoot = Join-Path $destination 'browser_extension'
    New-Item -ItemType Directory -Force -Path $packagedBrowserExtensionRoot | Out-Null
    Copy-Item '.\browser_extension\nxtools-interceptor' `
        $packagedBrowserExtensionRoot `
        -Recurse `
        -Force
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
