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

function Initialize-GpuiReleaseEnvironment {
    $fxcPath = Find-Fxc
    if (-not $fxcPath) {
        throw 'Release builds of GPUI 0.2.2 require fxc.exe. Install the Windows 10/11 SDK, or set GPUI_FXC_PATH to an existing x64 fxc.exe.'
    }
    $env:GPUI_FXC_PATH = $fxcPath
}
