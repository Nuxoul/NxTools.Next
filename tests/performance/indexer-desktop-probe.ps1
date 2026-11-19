function Invoke-IndexerDesktopSearch {
    param(
        [Parameter(Mandatory = $true)][string]$Query,
        [string]$ExpectedPath,
        [int]$TimeoutMs = 10000
    )
    $ErrorActionPreference = 'Stop'
    $packageRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\artifacts\win-x64'))
    $protectedRoot = Join-Path $env:ProgramData 'NxTools\NxTools.Next\Service'
    $desktopPath = Join-Path $protectedRoot 'NxTools.exe'
    $platformPath = Join-Path $protectedRoot 'NxTools.PlatformHost.exe'
    $approval = Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1'
    if ([IO.Path]::GetFullPath([string]$approval.NxToolsDesktopPath) -ne $desktopPath -or
        [string]$approval.NxToolsDesktopSha256 -ne (Get-FileHash $desktopPath -Algorithm SHA256).Hash -or
        [string]$approval.NxToolsDesktopSha256 -ne (Get-FileHash (Join-Path $packageRoot 'NxTools.exe') -Algorithm SHA256).Hash -or
        [string]$approval.NxToolsPlatformHostSha256 -ne (Get-FileHash $platformPath -Algorithm SHA256).Hash -or
        [string]$approval.NxToolsPlatformHostSha256 -ne (Get-FileHash (Join-Path $packageRoot 'NxTools.PlatformHost.exe') -Algorithm SHA256).Hash) {
        throw 'Desktop/PlatformHost do not match the approved package.'
    }
    $desktopProcess = @(Get-CimInstance Win32_Process -Filter "Name='NxTools.exe'" |
        Where-Object { $_.ExecutablePath -eq $desktopPath })
    if ($desktopProcess.Count -ne 1) { throw 'Exactly one approved product Desktop must be running.' }
    $platformProcess = @(Get-CimInstance Win32_Process -Filter "Name='NxTools.PlatformHost.exe'" |
        Where-Object { $_.ParentProcessId -eq $desktopProcess[0].ProcessId -and $_.ExecutablePath -eq $platformPath })
    if ($platformProcess.Count -ne 1) { throw 'The approved Desktop PlatformHost is unavailable.' }

    if (-not ('NxTools.QuickLaunchBenchmark.Launcher' -as [type])) {
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        Add-Type -AssemblyName WindowsBase
        $references = @('System.dll', 'System.Core.dll',
            [System.Windows.Automation.AutomationElement].Assembly.Location,
            [System.Windows.Automation.ControlType].Assembly.Location,
            [System.Windows.Rect].Assembly.Location)
        Add-Type -TypeDefinition (Get-Content (Join-Path $PSScriptRoot 'QuickLaunchComparisonDriver.cs') -Raw -Encoding UTF8) -ReferencedAssemblies $references
    }
    $launcher = [NxTools.QuickLaunchBenchmark.Launcher]::Find('nxtools', [uint32]$desktopProcess[0].ProcessId)
    if (-not $launcher) {
        [NxTools.QuickLaunchBenchmark.Native]::ShowNxTools([uint32]$platformProcess[0].ProcessId)
        $showDeadline = [DateTime]::UtcNow.AddSeconds(6)
        do {
            Start-Sleep -Milliseconds 100
            $launcher = [NxTools.QuickLaunchBenchmark.Launcher]::Find('nxtools', [uint32]$desktopProcess[0].ProcessId)
        } while (-not $launcher -and [DateTime]::UtcNow -lt $showDeadline)
    }
    if (-not $launcher) { throw 'The approved Desktop did not expose its launcher.' }
    $launcher.RequireEmpty()
    $null = [NxTools.QuickLaunchBenchmark.Native]::SetForegroundWindow($launcher.Window)
    Start-Sleep -Milliseconds 150
    $clock = [Diagnostics.Stopwatch]::StartNew()
    try {
        do {
            $sample = $launcher.Query($Query, [NxTools.QuickLaunchBenchmark.ExpectedTarget[]]@(), 1500, 300, 5000, 50)
            if ($sample.error -or -not $sample.focus_ok -or -not $sample.input_restored) {
                throw "Actual Desktop search failed: $($sample.error); input_restored=$($sample.input_restored)."
            }
            $matches = @($sample.rows | Where-Object { -not $ExpectedPath -or $_.path -eq $ExpectedPath })
            if ($matches.Count -gt 0) {
                return [pscustomobject]@{
                    DesktopPid = [uint32]$desktopProcess[0].ProcessId
                    Query = $Query
                    ResultCount = $sample.rows.Count
                    MatchedCount = $matches.Count
                    FirstPath = $matches[0].path
                    WallMs = $clock.ElapsedMilliseconds
                    InputRestored = $sample.input_restored
                }
            }
            Start-Sleep -Milliseconds 200
        } while ($clock.ElapsedMilliseconds -lt $TimeoutMs)
        throw "The approved Desktop returned no matching result within ${TimeoutMs}ms."
    }
    finally {
        if ($launcher.Foreground -and $launcher.InputValue -eq '') {
            [NxTools.QuickLaunchBenchmark.Native]::Escape($launcher.Window)
        }
    }
}
