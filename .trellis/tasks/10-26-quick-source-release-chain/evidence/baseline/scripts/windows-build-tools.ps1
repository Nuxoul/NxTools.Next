function Find-Fxc {
    if ($env:GPUI_FXC_PATH -and (Test-Path -LiteralPath $env:GPUI_FXC_PATH)) {
        return $env:GPUI_FXC_PATH
    }

    $fromPath = Get-Command 'fxc.exe' -ErrorAction SilentlyContinue
    if ($fromPath) {
        return $fromPath.Source
    }

    $sdkRoots = @(
        'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots'
    ) | ForEach-Object {
        (Get-ItemProperty -LiteralPath $_ -ErrorAction SilentlyContinue).KitsRoot10
    } | Where-Object { $_ } | Select-Object -Unique

    foreach ($sdkRoot in $sdkRoots) {
        $sdkBin = Join-Path $sdkRoot 'bin'
        if (Test-Path -LiteralPath $sdkBin) {
            $fxc = Get-ChildItem -LiteralPath $sdkBin -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                ForEach-Object { Join-Path $_.FullName 'x64\fxc.exe' } |
                Where-Object { Test-Path -LiteralPath $_ } |
                Select-Object -First 1
            if ($fxc) {
                return $fxc
            }
        }
    }

    return $null
}

function Initialize-LowMemoryRustBuildEnvironment {
    $env:CARGO_BUILD_JOBS = '1'
    $env:CARGO_INCREMENTAL = '0'
    $env:CARGO_PROFILE_DEV_DEBUG = '0'
    $env:CARGO_PROFILE_TEST_DEBUG = '0'
    $env:CARGO_PROFILE_RELEASE_DEBUG = '0'
    $env:CARGO_PROFILE_RELEASE_LTO = 'off'

    $requiredCodegenUnits = 256
    $codegenUnitsPattern = '(?i)(?:^|\s)-C\s*codegen-units(?:=|\s+)(\d+)'
    if ([string]::IsNullOrWhiteSpace($env:RUSTFLAGS)) {
        $env:RUSTFLAGS = "-C codegen-units=$requiredCodegenUnits"
    }
    elseif ($env:RUSTFLAGS -match $codegenUnitsPattern) {
        if ([int]$Matches[1] -ne $requiredCodegenUnits) {
            throw "RUSTFLAGS requests codegen-units=$($Matches[1]); NxTools.Next packaging requires $requiredCodegenUnits to control peak compiler memory."
        }
    }
    else {
        $env:RUSTFLAGS = "$($env:RUSTFLAGS.Trim()) -C codegen-units=$requiredCodegenUnits"
    }
}

function Assert-CargoBuildLaneIdle {
    $activeBuilds = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -match '^(cargo|rustc)\.exe$' })
    if ($activeBuilds.Count -ne 0) { throw 'Shared Cargo/rustc lane is occupied; wait for its owner, never terminate or duplicate it.' }
}

function Get-ReleaseBuildCommitHeadroomGiB {
    $memory = Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory -ErrorAction Stop
    $committedBytes = [double]$memory.CommittedBytes
    $commitLimit = [double]$memory.CommitLimit
    if ($commitLimit -le 0 -or $committedBytes -lt 0 -or $committedBytes -gt $commitLimit) {
        throw 'Windows returned invalid committed-memory counters.'
    }
    return ($commitLimit - $committedBytes) / 1GB
}

function Assert-ReleaseBuildCommitHeadroom(
    [ValidateRange(0.0, 1024.0)]
    [double]$MinimumGiB = 6.0) {
    $availableGiB = Get-ReleaseBuildCommitHeadroomGiB
    Write-Host ('Release build commit headroom: {0:N2} GiB (required: {1:N2} GiB).' -f `
        $availableGiB, $MinimumGiB)
    if ($availableGiB -lt $MinimumGiB) {
        throw ('Release build refused before Cargo because Windows commit headroom is only ' +
            ('{0:N2} GiB; at least {1:N2} GiB is required to avoid OS 1455/LNK1102/process crashes. ' -f `
                $availableGiB, $MinimumGiB) +
            'Wait for memory pressure to fall, increase the paging file, or explicitly lower ' +
            '-MinimumCommitHeadroomGiB after accepting the system-stability risk.')
    }
}

function Initialize-GpuiReleaseEnvironment {
    $fxcPath = Find-Fxc
    if (-not $fxcPath) {
        throw 'Release builds of GPUI 0.2.2 require fxc.exe. Install the Windows 10/11 SDK, or set GPUI_FXC_PATH to an existing x64 fxc.exe.'
    }
    $env:GPUI_FXC_PATH = $fxcPath
}
