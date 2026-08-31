param(
    [Parameter(Mandatory = $true)]
    [int]$ProcessId,
    [Parameter(Mandatory = $true)]
    [string]$OutPath
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

Add-Type @'
using System;
using System.Runtime.InteropServices;
public class WinCapture {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr lp);
  public static IntPtr MainVisibleWindow(uint target) {
    IntPtr best = IntPtr.Zero;
    int bestArea = 0;
    EnumWindows((h, lp) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target && IsWindowVisible(h)) {
        RECT r; GetWindowRect(h, out r);
        int area = (r.R - r.L) * (r.B - r.T);
        if (area > bestArea) { bestArea = area; best = h; }
      }
      return true;
    }, IntPtr.Zero);
    return best;
  }
}
'@

$window = [WinCapture]::MainVisibleWindow($ProcessId)
if ($window -eq [IntPtr]::Zero) {
    throw "No visible top-level window found for process $ProcessId."
}

[WinCapture]::SetForegroundWindow($window) | Out-Null
Start-Sleep -Milliseconds 600

$rect = New-Object WinCapture+RECT
if (-not [WinCapture]::GetWindowRect($window, [ref]$rect)) {
    throw "GetWindowRect failed for window $window."
}

$width = $rect.R - $rect.L
$height = $rect.B - $rect.T
if ($width -le 0 -or $height -le 0) {
    throw "Window $window has an empty rect."
}

$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($rect.L, $rect.T, 0, 0, (New-Object System.Drawing.Size($width, $height)))
} finally {
    $graphics.Dispose()
}

$outFull = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutPath)
$directory = Split-Path -Parent $outFull
New-Item -ItemType Directory -Force -Path $directory | Out-Null
$bitmap.Save($outFull, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
Write-Host "Saved $outFull (${width}x${height} from window $window)."
