param(
    [string]$Executable = "$PSScriptRoot\..\..\artifacts\win-x64\NxTools.exe",
    [int]$WarmupSeconds = 5,
    [int]$SampleSeconds = 12,
    [int]$SampleMilliseconds = 250,
    [double]$SustainedThresholdPercent = 4.0,
    [double]$CoreEquivalentAverageThresholdPercent = 7.0,
    [switch]$HoverFirstItem,
    [switch]$ExerciseScroll,
    [switch]$AssertRegression
)

$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$processorCount = [Environment]::ProcessorCount

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class NxToolsCpuProbeInput
{
    [DllImport("user32.dll")]
    private static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    private const uint KeyUp = 0x0002;

    private delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential)]
    private struct Rect
    {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    private static extern bool GetWindowRect(IntPtr window, out Rect rect);

    [DllImport("user32.dll")]
    private static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    private static extern void mouse_event(uint flags, uint dx, uint dy, int data, UIntPtr extraInfo);

    private const uint MouseWheel = 0x0800;

    public static void OpenClipboardPanel()
    {
        keybd_event(0x12, 0, 0, UIntPtr.Zero);
        keybd_event(0x43, 0, 0, UIntPtr.Zero);
        keybd_event(0x43, 0, KeyUp, UIntPtr.Zero);
        keybd_event(0x12, 0, KeyUp, UIntPtr.Zero);
    }

    public static void DismissClipboardPanel()
    {
        keybd_event(0x1B, 0, 0, UIntPtr.Zero);
        keybd_event(0x1B, 0, KeyUp, UIntPtr.Zero);
    }

    public static void HoverFirstHistoryItem(uint processId)
    {
        var candidates = new List<Rect>();
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out var ownerProcessId);
            if (ownerProcessId == processId && IsWindowVisible(window) && GetWindowRect(window, out var rect))
            {
                var width = rect.Right - rect.Left;
                var height = rect.Bottom - rect.Top;
                if (width >= 260 && width <= 800 && height >= 280)
                    candidates.Add(rect);
            }
            return true;
        }, IntPtr.Zero);

        if (candidates.Count == 0)
            throw new InvalidOperationException("Could not find the visible clipboard panel window.");

        candidates.Sort((left, right) =>
            ((left.Right - left.Left) * (left.Bottom - left.Top))
                .CompareTo((right.Right - right.Left) * (right.Bottom - right.Top)));
        var panel = candidates[0];
        SetCursorPos(panel.Left + 48, panel.Top + 150);
    }

    public static void ScrollHistory(int wheelDelta)
    {
        mouse_event(MouseWheel, 0, 0, wheelDelta, UIntPtr.Zero);
    }
}
'@

$process = Start-Process -FilePath $resolvedExecutable -PassThru
try {
    Start-Sleep -Seconds $WarmupSeconds
    $process = Get-Process -Id $process.Id -ErrorAction Stop
    $shell = New-Object -ComObject WScript.Shell
    if (-not $shell.AppActivate($process.Id)) {
        throw "Could not activate NxTools process $($process.Id) before opening the clipboard panel."
    }
    Start-Sleep -Milliseconds 350
    [NxToolsCpuProbeInput]::OpenClipboardPanel()
    Start-Sleep -Milliseconds 350
    if ($HoverFirstItem -or $ExerciseScroll) {
        [NxToolsCpuProbeInput]::HoverFirstHistoryItem([uint32]$process.Id)
    }

    $platformHost = Get-Process -Name 'NxTools.PlatformHost' -ErrorAction Stop |
        Where-Object StartTime -ge $process.StartTime |
        Sort-Object StartTime -Descending |
        Select-Object -First 1
    $samples = [System.Collections.Generic.List[object]]::new()
    $previousDesktopCpu = $process.TotalProcessorTime.TotalSeconds
    $previousHostCpu = $platformHost.TotalProcessorTime.TotalSeconds
    $previousTime = [System.Diagnostics.Stopwatch]::GetTimestamp()
    $sampleCount = [Math]::Ceiling(($SampleSeconds * 1000) / $SampleMilliseconds)

    for ($index = 0; $index -lt $sampleCount; $index++) {
        if ($ExerciseScroll -and ($index % 2 -eq 0)) {
            $direction = if (($index / 2) % 2 -eq 0) { -120 } else { 120 }
            [NxToolsCpuProbeInput]::ScrollHistory($direction)
        }
        Start-Sleep -Milliseconds $SampleMilliseconds
        $process.Refresh()
        $platformHost.Refresh()
        $now = [System.Diagnostics.Stopwatch]::GetTimestamp()
        $desktopCpu = $process.TotalProcessorTime.TotalSeconds
        $hostCpu = $platformHost.TotalProcessorTime.TotalSeconds
        $elapsed = ($now - $previousTime) / [System.Diagnostics.Stopwatch]::Frequency
        $desktopCpuPercent = (($desktopCpu - $previousDesktopCpu) / $elapsed / $processorCount) * 100
        $hostCpuPercent = (($hostCpu - $previousHostCpu) / $elapsed / $processorCount) * 100
        $samples.Add([pscustomobject]@{
            Second = [Math]::Round((($index + 1) * $SampleMilliseconds) / 1000, 2)
            DesktopCpuPercent = [Math]::Round($desktopCpuPercent, 2)
            HostCpuPercent = [Math]::Round($hostCpuPercent, 2)
            CpuPercent = [Math]::Round($desktopCpuPercent + $hostCpuPercent, 2)
            CoreEquivalentCpuPercent = [Math]::Round(($desktopCpuPercent + $hostCpuPercent) * $processorCount, 2)
            DesktopWorkingSetMb = [Math]::Round($process.WorkingSet64 / 1MB, 2)
            DesktopPrivateMemoryMb = [Math]::Round($process.PrivateMemorySize64 / 1MB, 2)
            HostWorkingSetMb = [Math]::Round($platformHost.WorkingSet64 / 1MB, 2)
        })
        $previousDesktopCpu = $desktopCpu
        $previousHostCpu = $hostCpu
        $previousTime = $now
    }

    $average = ($samples | Measure-Object CpuPercent -Average).Average
    $coreEquivalentAverage = ($samples | Measure-Object CoreEquivalentCpuPercent -Average).Average
    $peak = ($samples | Measure-Object CpuPercent -Maximum).Maximum
    $peakWorkingSet = ($samples | Measure-Object DesktopWorkingSetMb -Maximum).Maximum
    $workingSetGrowth = $samples[-1].DesktopWorkingSetMb - $samples[0].DesktopWorkingSetMb
    $aboveThreshold = @($samples | Where-Object CpuPercent -ge $SustainedThresholdPercent).Count
    $sustainedSeconds = $aboveThreshold * $SampleMilliseconds / 1000

    $samples | Format-Table -AutoSize
    "AVERAGE_CPU_PERCENT={0:N2}" -f $average
    "AVERAGE_CORE_EQUIVALENT_CPU_PERCENT={0:N2}" -f $coreEquivalentAverage
    "PEAK_CPU_PERCENT={0:N2}" -f $peak
    "PEAK_DESKTOP_WORKING_SET_MB={0:N2}" -f $peakWorkingSet
    "DESKTOP_WORKING_SET_GROWTH_MB={0:N2}" -f $workingSetGrowth
    "SECONDS_AT_OR_ABOVE_{0:N1}_PERCENT={1:N2}" -f $SustainedThresholdPercent, $sustainedSeconds

    if ($AssertRegression -and $coreEquivalentAverage -ge $CoreEquivalentAverageThresholdPercent) {
        throw "Clipboard panel CPU regression: average single-core-equivalent CPU was $([Math]::Round($coreEquivalentAverage, 2))%, threshold is $CoreEquivalentAverageThresholdPercent%."
    }
    if ($AssertRegression -and $sustainedSeconds -ge 8) {
        throw "Clipboard panel CPU regression: normalized CPU stayed at or above $SustainedThresholdPercent% for $sustainedSeconds seconds."
    }
}
finally {
    [NxToolsCpuProbeInput]::DismissClipboardPanel()
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
