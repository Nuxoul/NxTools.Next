param(
    [Parameter(Mandatory = $true)]
    [string]$Scenario,
    [string]$Executable = "$PSScriptRoot\..\..\artifacts\win-x64\NxTools.exe",
    [int]$ProcessId = 0,
    [string]$OutputDirectory = "$PSScriptRoot\..\..\artifacts\ui-automation",
    [int]$StartupTimeoutMilliseconds = 10000
)

$ErrorActionPreference = 'Stop'
$scenarioPath = (Resolve-Path -LiteralPath $Scenario).Path
$scenarioData = Get-Content -LiteralPath $scenarioPath -Raw | ConvertFrom-Json
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runDirectory = Join-Path $OutputDirectory $runId
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null

try {
    Add-Type -AssemblyName System.Drawing.Common -ErrorAction Stop
}
catch {
    Add-Type -AssemblyName System.Drawing -ErrorAction Stop
}
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public sealed class NxToolsAutomationWindow
{
    public long Handle { get; set; }
    public int ProcessId { get; set; }
    public string Title { get; set; }
    public string ClassName { get; set; }
    public int Left { get; set; }
    public int Top { get; set; }
    public int Width { get; set; }
    public int Height { get; set; }
    public bool Visible { get; set; }
    public bool Topmost { get; set; }
}

public static class NxToolsWindowsAutomation
{
    private const int GwlExStyle = -20;
    private const long WsExTopmost = 0x00000008L;
    private const uint WmChar = 0x0102;
    private const uint WmKeyDown = 0x0100;
    private const uint WmKeyUp = 0x0101;
    private const uint WmMouseMove = 0x0200;
    private const uint WmLeftButtonDown = 0x0201;
    private const uint WmLeftButtonUp = 0x0202;
    private const uint WmClose = 0x0010;
    private const uint MkLButton = 0x0001;
    private const uint PrintWindowRenderFullContent = 0x00000002;

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
    private static extern bool GetWindowRect(IntPtr window, out Rect rectangle);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr window, StringBuilder text, int maximumCount);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder text, int maximumCount);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    private static extern int GetWindowLong32(IntPtr window, int index);

    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern IntPtr GetWindowLongPtr64(IntPtr window, int index);

    [DllImport("user32.dll")]
    private static extern bool PrintWindow(IntPtr window, IntPtr deviceContext, uint flags);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageW(IntPtr window, uint message, UIntPtr word, IntPtr data);

    [DllImport("user32.dll")]
    private static extern bool PostMessageW(IntPtr window, uint message, UIntPtr word, IntPtr data);

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("user32.dll")]
    private static extern bool AttachThreadInput(uint attach, uint attachTo, bool value);

    [DllImport("user32.dll")]
    private static extern bool BringWindowToTop(IntPtr window);

    private static long ExtendedStyle(IntPtr window)
    {
        return IntPtr.Size == 8
            ? GetWindowLongPtr64(window, GwlExStyle).ToInt64()
            : GetWindowLong32(window, GwlExStyle);
    }

    public static NxToolsAutomationWindow[] Windows(int processId)
    {
        var result = new List<NxToolsAutomationWindow>();
        EnumWindows((window, _) =>
        {
            uint owner;
            Rect rectangle;
            GetWindowThreadProcessId(window, out owner);
            if (owner != (uint)processId || !GetWindowRect(window, out rectangle))
                return true;
            var title = new StringBuilder(512);
            var className = new StringBuilder(256);
            GetWindowText(window, title, title.Capacity);
            GetClassName(window, className, className.Capacity);
            result.Add(new NxToolsAutomationWindow
            {
                Handle = window.ToInt64(),
                ProcessId = processId,
                Title = title.ToString(),
                ClassName = className.ToString(),
                Left = rectangle.Left,
                Top = rectangle.Top,
                Width = rectangle.Right - rectangle.Left,
                Height = rectangle.Bottom - rectangle.Top,
                Visible = IsWindowVisible(window),
                Topmost = (ExtendedStyle(window) & WsExTopmost) != 0
            });
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }

    public static void Click(long handle, int x, int y)
    {
        var window = new IntPtr(handle);
        var data = new IntPtr((y << 16) | (x & 0xffff));
        PostMessageW(window, WmMouseMove, UIntPtr.Zero, data);
        PostMessageW(window, WmLeftButtonDown, new UIntPtr(MkLButton), data);
        PostMessageW(window, WmLeftButtonUp, UIntPtr.Zero, data);
    }

    public static void TypeText(long handle, string text)
    {
        var window = new IntPtr(handle);
        foreach (var character in text)
        {
            SendMessageW(window, WmChar, new UIntPtr(character), IntPtr.Zero);
            Thread.Sleep(4);
        }
    }

    public static void Key(long handle, int virtualKey)
    {
        var window = new IntPtr(handle);
        SendMessageW(window, WmKeyDown, new UIntPtr((uint)virtualKey), IntPtr.Zero);
        SendMessageW(window, WmKeyUp, new UIntPtr((uint)virtualKey), IntPtr.Zero);
        Thread.Sleep(2);
    }

    public static void Close(long handle)
    {
        PostMessageW(new IntPtr(handle), WmClose, UIntPtr.Zero, IntPtr.Zero);
    }

    public static bool Activate(long handle)
    {
        var target = new IntPtr(handle);
        var currentThread = GetCurrentThreadId();
        uint targetOwner;
        var targetThread = GetWindowThreadProcessId(target, out targetOwner);
        var foreground = GetForegroundWindow();
        uint foregroundOwner;
        var foregroundThread = foreground == IntPtr.Zero
            ? 0
            : GetWindowThreadProcessId(foreground, out foregroundOwner);
        var attachedToTarget = targetThread != 0 && targetThread != currentThread &&
            AttachThreadInput(currentThread, targetThread, true);
        var attachedToForeground = foregroundThread != 0 &&
            foregroundThread != currentThread && foregroundThread != targetThread &&
            AttachThreadInput(currentThread, foregroundThread, true);
        try
        {
            BringWindowToTop(target);
            SetForegroundWindow(target);
            return GetForegroundWindow() == target;
        }
        finally
        {
            if (attachedToForeground)
                AttachThreadInput(currentThread, foregroundThread, false);
            if (attachedToTarget)
                AttachThreadInput(currentThread, targetThread, false);
        }
    }

    public static bool Print(long handle, IntPtr deviceContext)
    {
        return PrintWindow(new IntPtr(handle), deviceContext, PrintWindowRenderFullContent);
    }
}
'@

function Save-AutomationWindow([object]$window, [string]$path) {
    if ($window.Width -le 0 -or $window.Height -le 0) {
        return $false
    }
    $bitmap = [System.Drawing.Bitmap]::new(
        $window.Width,
        $window.Height,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $deviceContext = $graphics.GetHdc()
        try {
            $printed = [NxToolsWindowsAutomation]::Print($window.Handle, $deviceContext)
        }
        finally {
            $graphics.ReleaseHdc($deviceContext)
        }
        if (-not $printed) {
            $graphics.CopyFromScreen(
                $window.Left,
                $window.Top,
                0,
                0,
                [System.Drawing.Size]::new($window.Width, $window.Height))
        }
        $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        return $true
    }
    finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function Resolve-NxToolsProcess {
    if ($ProcessId -gt 0) {
        return Get-Process -Id $ProcessId -ErrorAction Stop
    }
    $existing = Get-Process -Name NxTools -ErrorAction SilentlyContinue |
        Where-Object { $_.Responding } |
        Sort-Object StartTime -Descending
    foreach ($candidate in $existing) {
        $visibleWindows = @([NxToolsWindowsAutomation]::Windows($candidate.Id)) |
            Where-Object { $_.Visible }
        if ($visibleWindows.Count -gt 0) {
            return $candidate
        }
    }
    $resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
    Start-Process -FilePath $resolvedExecutable | Out-Null
    $deadline = [DateTime]::UtcNow.AddMilliseconds($StartupTimeoutMilliseconds)
    do {
        $candidates = Get-Process -Name NxTools -ErrorAction SilentlyContinue |
            Where-Object { $_.Responding } |
            Sort-Object StartTime -Descending
        foreach ($candidate in $candidates) {
            $visibleWindows = @([NxToolsWindowsAutomation]::Windows($candidate.Id)) |
                Where-Object { $_.Visible }
            if ($visibleWindows.Count -gt 0) {
                return $candidate
            }
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "NxTools did not expose a responsive process within $StartupTimeoutMilliseconds ms."
}

function Select-AutomationWindow([object]$selector) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds(
        $(if ($null -ne $selector.timeout_ms) { [int]$selector.timeout_ms } else { 5000 }))
    do {
        $windows = @([NxToolsWindowsAutomation]::Windows($script:process.Id))
        $matches = $windows | Where-Object {
            ($null -eq $selector.visible -or $_.Visible -eq [bool]$selector.visible) -and
            ($null -eq $selector.title -or $_.Title -eq [string]$selector.title) -and
            ($null -eq $selector.class_name -or $_.ClassName -eq [string]$selector.class_name) -and
            ($null -eq $selector.min_width -or $_.Width -ge [int]$selector.min_width) -and
            ($null -eq $selector.max_width -or $_.Width -le [int]$selector.max_width) -and
            ($null -eq $selector.min_height -or $_.Height -ge [int]$selector.min_height) -and
            ($null -eq $selector.max_height -or $_.Height -le [int]$selector.max_height)
        } | Sort-Object @{ Expression = { $_.Width * $_.Height }; Descending = $true }
        if ($null -ne $matches -and @($matches).Count -gt 0) {
            return @($matches)[0]
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "No NxTools window matched selector: $($selector | ConvertTo-Json -Compress)"
}

$process = Resolve-NxToolsProcess
$results = [System.Collections.Generic.List[object]]::new()
$currentWindow = $null
$stepNumber = 0

try {
    foreach ($step in $scenarioData.steps) {
        $stepNumber++
        $started = [DateTime]::UtcNow
        switch ([string]$step.action) {
            'select_window' {
                $currentWindow = Select-AutomationWindow $step.selector
            }
            'close_matching_windows' {
                $matches = @([NxToolsWindowsAutomation]::Windows($process.Id)) | Where-Object {
                    ($null -eq $step.selector.visible -or $_.Visible -eq [bool]$step.selector.visible) -and
                    ($null -eq $step.selector.title -or $_.Title -eq [string]$step.selector.title) -and
                    ($null -eq $step.selector.min_width -or $_.Width -ge [int]$step.selector.min_width) -and
                    ($null -eq $step.selector.max_width -or $_.Width -le [int]$step.selector.max_width) -and
                    ($null -eq $step.selector.min_height -or $_.Height -ge [int]$step.selector.min_height) -and
                    ($null -eq $step.selector.max_height -or $_.Height -le [int]$step.selector.max_height)
                }
                foreach ($match in $matches) {
                    [NxToolsWindowsAutomation]::Close($match.Handle)
                }
                if ($matches.Count -gt 0) { Start-Sleep -Milliseconds 200 }
                $currentWindow = $null
            }
            'wait' {
                Start-Sleep -Milliseconds ([int]$step.milliseconds)
            }
            'click' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before click.' }
                $x = if ($null -ne $step.x_ratio) {
                    [int][Math]::Round($currentWindow.Width * [double]$step.x_ratio)
                } else { [int]$step.x }
                $y = if ($null -ne $step.y_ratio) {
                    [int][Math]::Round($currentWindow.Height * [double]$step.y_ratio)
                } else { [int]$step.y }
                [NxToolsWindowsAutomation]::Click($currentWindow.Handle, $x, $y)
            }
            'type_text' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before type_text.' }
                $text = if ($null -ne $step.text_env) {
                    [Environment]::GetEnvironmentVariable([string]$step.text_env)
                } else { [string]$step.text }
                if ($null -eq $text) { throw "Environment value '$($step.text_env)' is missing." }
                [NxToolsWindowsAutomation]::TypeText($currentWindow.Handle, $text)
            }
            'key' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before key.' }
                [NxToolsWindowsAutomation]::Key($currentWindow.Handle, [int]$step.virtual_key)
            }
            'repeat_key' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before repeat_key.' }
                for ($index = 0; $index -lt [int]$step.count; $index++) {
                    [NxToolsWindowsAutomation]::Key($currentWindow.Handle, [int]$step.virtual_key)
                }
            }
            'activate' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before activate.' }
                if (-not [NxToolsWindowsAutomation]::Activate($currentWindow.Handle)) {
                    throw "Windows rejected foreground activation for HWND $($currentWindow.Handle)."
                }
            }
            'capture' {
                if ($null -eq $currentWindow) { throw 'A window must be selected before capture.' }
                $name = if ($null -ne $step.name) { [string]$step.name } else { "step-$stepNumber" }
                $path = Join-Path $runDirectory "$name.png"
                if (-not (Save-AutomationWindow $currentWindow $path)) {
                    throw "Failed to capture HWND $($currentWindow.Handle)."
                }
            }
            'assert_window' {
                $currentWindow = Select-AutomationWindow $step.selector
                if ($null -ne $step.topmost -and $currentWindow.Topmost -ne [bool]$step.topmost) {
                    throw "Expected topmost=$($step.topmost), actual=$($currentWindow.Topmost)."
                }
            }
            'assert_file' {
                $path = if ($null -ne $step.path_env) {
                    [Environment]::GetEnvironmentVariable([string]$step.path_env)
                } else { [string]$step.path }
                if ([string]::IsNullOrWhiteSpace($path)) {
                    throw 'assert_file requires path or path_env.'
                }
                $deadline = [DateTime]::UtcNow.AddMilliseconds(
                    $(if ($null -ne $step.timeout_ms) { [int]$step.timeout_ms } else { 10000 }))
                do {
                    if (Test-Path -LiteralPath $path -PathType Leaf) {
                        $file = Get-Item -LiteralPath $path
                        $minimumBytes = if ($null -ne $step.minimum_bytes_env) {
                            [long][Environment]::GetEnvironmentVariable(
                                [string]$step.minimum_bytes_env)
                        } elseif ($null -ne $step.minimum_bytes) {
                            [long]$step.minimum_bytes
                        } else { 0L }
                        if ($file.Length -ge $minimumBytes) {
                            break
                        }
                    }
                    Start-Sleep -Milliseconds 100
                } while ([DateTime]::UtcNow -lt $deadline)
                if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                    throw "Expected download artifact was not created: $path"
                }
                $file = Get-Item -LiteralPath $path
                $minimumBytes = if ($null -ne $step.minimum_bytes_env) {
                    [long][Environment]::GetEnvironmentVariable(
                        [string]$step.minimum_bytes_env)
                } elseif ($null -ne $step.minimum_bytes) {
                    [long]$step.minimum_bytes
                } else { 0L }
                if ($file.Length -lt $minimumBytes) {
                    throw "Download artifact is too small: $($file.Length) bytes."
                }
                $exactBytes = if ($null -ne $step.exact_bytes_env) {
                    $exactValue = [Environment]::GetEnvironmentVariable([string]$step.exact_bytes_env)
                    if ([string]::IsNullOrWhiteSpace($exactValue)) { -1L } else { [long]$exactValue }
                } elseif ($null -ne $step.exact_bytes) {
                    [long]$step.exact_bytes
                } else { -1L }
                if ($exactBytes -ge 0 -and $file.Length -ne $exactBytes) {
                    throw "Download artifact length is wrong: expected $exactBytes, actual $($file.Length)."
                }
                $expectedSha256 = if ($null -ne $step.sha256_env) {
                    [Environment]::GetEnvironmentVariable([string]$step.sha256_env)
                } else { [string]$step.sha256 }
                if (-not [string]::IsNullOrWhiteSpace($expectedSha256)) {
                    $hashAlgorithm = [System.Security.Cryptography.SHA256]::Create()
                    $fileStream = [System.IO.File]::OpenRead($path)
                    try {
                        $actualSha256 = ([BitConverter]::ToString(
                            $hashAlgorithm.ComputeHash($fileStream))).Replace('-', '')
                    }
                    finally {
                        $fileStream.Dispose()
                        $hashAlgorithm.Dispose()
                    }
                    if (-not $actualSha256.Equals($expectedSha256, [StringComparison]::OrdinalIgnoreCase)) {
                        throw "Download artifact SHA-256 is wrong: expected $expectedSha256, actual $actualSha256."
                    }
                }
            }
            default { throw "Unknown automation action '$($step.action)'." }
        }
        $results.Add([ordered]@{
            step = $stepNumber
            action = [string]$step.action
            status = 'passed'
            elapsed_ms = [Math]::Round(([DateTime]::UtcNow - $started).TotalMilliseconds, 2)
            window_handle = if ($null -ne $currentWindow) { $currentWindow.Handle } else { $null }
        })
    }
    $status = 'passed'
}
catch {
    $status = 'failed'
    $results.Add([ordered]@{
        step = $stepNumber
        action = if ($null -ne $step) { [string]$step.action } else { $null }
        status = 'failed'
        error = $_.Exception.Message
    })
}

$windows = @([NxToolsWindowsAutomation]::Windows($process.Id))
$report = [ordered]@{
    run_id = $runId
    scenario = $scenarioData.name
    status = $status
    process_id = $process.Id
    process_path = $process.Path
    started_at = (Get-Date).ToString('o')
    steps = $results
    windows = $windows
}
$reportPath = Join-Path $runDirectory 'report.json'
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
$report | ConvertTo-Json -Depth 8
if ($status -ne 'passed') { exit 1 }
