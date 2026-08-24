param(
    [string]$Executable = "$PSScriptRoot\..\..\artifacts\win-x64\NxTools.exe",
    [int]$StartupMilliseconds = 2200,
    [int]$ResultTimeoutMilliseconds = 2500,
    [switch]$UseRealPlatformHost,
    [switch]$EnterActionsMode,
    [string]$ScreenshotPath
)

$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
if (-not [string]::IsNullOrWhiteSpace($ScreenshotPath) -and -not $UseRealPlatformHost) {
    throw 'Quick-launch screenshots require -UseRealPlatformHost so native Shell icons are real.'
}

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Pipes;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Threading;

public static class NxToolsQuickLaunchSmokeInput
{
    private static IntPtr focusedPanel;

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
    private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SendMessageW(IntPtr window, uint message, UIntPtr word, IntPtr data);

    [DllImport("user32.dll")]
    private static extern IntPtr GetDC(IntPtr window);

    [DllImport("user32.dll")]
    private static extern int ReleaseDC(IntPtr window, IntPtr deviceContext);

    [DllImport("gdi32.dll")]
    private static extern uint GetPixel(IntPtr deviceContext, int x, int y);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindowExW(
        IntPtr parent,
        IntPtr childAfter,
        string className,
        string windowName);

    [DllImport("user32.dll")]
    private static extern bool PostMessageW(IntPtr window, uint message, IntPtr word, IntPtr data);

    private const uint LeftDown = 0x0002;
    private const uint LeftUp = 0x0004;
    private const uint WindowKeyDown = 0x0100;
    private const uint VirtualKeyRight = 0x27;
    private const uint InvalidPixel = 0xffffffff;

    private static void Tap(byte virtualKey)
    {
        keybd_event(virtualKey, 0, 0, UIntPtr.Zero);
        keybd_event(virtualKey, 0, KeyUp, UIntPtr.Zero);
    }

    public static void ParkPointer()
    {
        SetCursorPos(0, 0);
    }

    public static void OpenQuickLaunch()
    {
        Tap(0x11);
        Thread.Sleep(110);
        Tap(0x11);
    }

    public static bool RequestPanelThroughRealPlatformHost(uint processId)
    {
        var messageOnlyWindow = IntPtr.Zero;
        while ((messageOnlyWindow = FindWindowExW(
                    new IntPtr(-3),
                    messageOnlyWindow,
                    null,
                    "NxTools.PlatformHost.Events")) != IntPtr.Zero)
        {
            GetWindowThreadProcessId(messageOnlyWindow, out var ownerProcessId);
            if (ownerProcessId == processId)
                return PostMessageW(messageOnlyWindow, 0x0312, new IntPtr(2), IntPtr.Zero);
        }
        return false;
    }

    public static void TypeQuery()
    {
        var target = focusedPanel != IntPtr.Zero ? focusedPanel : GetForegroundWindow();
        SendMessageW(target, 0x0102, new UIntPtr('q'), IntPtr.Zero);
    }

    public static void EnterActionsMode()
    {
        var target = focusedPanel != IntPtr.Zero ? focusedPanel : GetForegroundWindow();
        SendMessageW(target, WindowKeyDown, new UIntPtr(VirtualKeyRight), IntPtr.Zero);
    }

    public static bool IsActionModeSurface()
    {
        if (focusedPanel == IntPtr.Zero || !GetWindowRect(focusedPanel, out var rect))
            return false;
        var deviceContext = GetDC(IntPtr.Zero);
        if (deviceContext == IntPtr.Zero)
            return false;
        try
        {
            // In action mode the first selected 40 px row occupies this upper sample, while the
            // lower sample is the following canvas row. The ordinary result sweep selects a lower
            // row, so both samples remain canvas-colored if Right never reached the panel.
            var selectedRow = GetPixel(deviceContext, rect.Left + 300, rect.Top + 80);
            var followingRow = GetPixel(deviceContext, rect.Left + 300, rect.Top + 120);
            return selectedRow != InvalidPixel &&
                followingRow != InvalidPixel &&
                selectedRow != followingRow;
        }
        finally
        {
            ReleaseDC(IntPtr.Zero, deviceContext);
        }
    }

    public static void PointAtFirstAction()
    {
        if (focusedPanel != IntPtr.Zero && GetWindowRect(focusedPanel, out var rect))
            SetCursorPos(rect.Left + 100, rect.Top + 80);
    }

    public static bool TryFocusCollapsedPanel(uint processId)
    {
        var selectedWindow = IntPtr.Zero;
        var selected = new Rect();
        var selectedWidth = int.MaxValue;
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out var ownerProcessId);
            if (ownerProcessId == processId && IsWindowVisible(window) && GetWindowRect(window, out var rect))
            {
                var width = rect.Right - rect.Left;
                var height = rect.Bottom - rect.Top;
                if (width >= 360 && width <= 800 && height >= 35 && height <= 99 && width < selectedWidth)
                {
                    selectedWindow = window;
                    selected = rect;
                    selectedWidth = width;
                }
            }
            return true;
        }, IntPtr.Zero);

        if (selectedWindow == IntPtr.Zero)
            return false;

        focusedPanel = selectedWindow;
        SetForegroundWindow(selectedWindow);
        SetCursorPos((selected.Left + selected.Right) / 2, (selected.Top + selected.Bottom) / 2);
        mouse_event(LeftDown, 0, 0, 0, UIntPtr.Zero);
        mouse_event(LeftUp, 0, 0, 0, UIntPtr.Zero);
        return true;
    }

    public static int[] GetFocusedPanelBounds()
    {
        if (focusedPanel == IntPtr.Zero || !GetWindowRect(focusedPanel, out var rect))
            throw new InvalidOperationException("The quick-launch result window is unavailable for capture.");
        return new[] { rect.Left, rect.Top, rect.Right - rect.Left, rect.Bottom - rect.Top };
    }

    public static bool TrySweepVisibleResults(uint processId)
    {
        var candidates = FindWindows(processId, 360, 800, 100, 800);
        if (candidates.Count == 0)
            return false;

        candidates.Sort((left, right) =>
            ((left.Right - left.Left) * (left.Bottom - left.Top))
                .CompareTo((right.Right - right.Left) * (right.Bottom - right.Top)));
        var popup = candidates[0];
        Thread.Sleep(250);
        var bottom = Math.Min(popup.Bottom - 24, popup.Top + 82 + (9 * 46));
        for (var y = popup.Top + 82; y <= bottom; y += 46)
        {
            SetCursorPos(popup.Left + 72, y);
            Thread.Sleep(80);
            SetCursorPos(popup.Left + 87, y);
            Thread.Sleep(80);
        }
        return true;
    }

    private static List<Rect> FindWindows(
        uint processId,
        int minimumWidth,
        int maximumWidth,
        int minimumHeight,
        int maximumHeight)
    {
        var candidates = new List<Rect>();
        EnumWindows((window, _) =>
        {
            GetWindowThreadProcessId(window, out var ownerProcessId);
            if (ownerProcessId == processId && IsWindowVisible(window) && GetWindowRect(window, out var rect))
            {
                var width = rect.Right - rect.Left;
                var height = rect.Bottom - rect.Top;
                if (width >= minimumWidth && width <= maximumWidth &&
                    height >= minimumHeight && height <= maximumHeight)
                    candidates.Add(rect);
            }
            return true;
        }, IntPtr.Zero);

        return candidates;
    }
}

public sealed class NxToolsQuickLaunchProtocolProbe : IDisposable
{
    private readonly NamedPipeClientStream pipe;
    private readonly StreamReader reader;
    private readonly StreamWriter writer;
    private readonly Thread pump;
    private readonly string iconPath;
    private volatile bool stopping;

    private NxToolsQuickLaunchProtocolProbe(
        NamedPipeClientStream pipe,
        StreamReader reader,
        StreamWriter writer,
        string iconPath)
    {
        this.pipe = pipe;
        this.reader = reader;
        this.writer = writer;
        this.iconPath = iconPath;
        pump = new Thread(PumpCommands) { IsBackground = true, Name = "NxTools smoke protocol pump" };
    }

    public static NxToolsQuickLaunchProtocolProbe Connect(string iconPath)
    {
        var pipe = new NamedPipeClientStream(
            ".",
            "nxtools-next-platform-v1",
            PipeDirection.InOut,
            PipeOptions.None);
        pipe.Connect(4000);
        var reader = new StreamReader(pipe, new UTF8Encoding(false), false, 4096, true);
        var writer = new StreamWriter(pipe, new UTF8Encoding(false), 4096, true)
        {
            AutoFlush = true,
            NewLine = "\n"
        };
        var helloId = Guid.NewGuid();
        writer.WriteLine(JsonSerializer.Serialize(new
        {
            protocol_version = 1,
            id = helloId,
            correlation_id = (Guid?)null,
            type = "hello",
            payload = new
            {
                client_name = "quick_launch_smoke_probe",
                client_version = "1.0.0",
                capabilities = new[]
                {
                    "window_topmost",
                    "window_visibility",
                    "window_activation",
                    "shell_icon_extraction",
                    "quick_launch_panel_hotkey"
                }
            }
        }));

        var acknowledgement = reader.ReadLine();
        if (acknowledgement == null)
            throw new EndOfStreamException("Desktop closed the pipe before acknowledging the smoke probe.");
        using (var document = JsonDocument.Parse(acknowledgement))
        {
            var root = document.RootElement;
            if (root.GetProperty("type").GetString() != "hello_ack" ||
                root.GetProperty("correlation_id").GetGuid() != helloId)
                throw new InvalidDataException("Desktop returned an invalid smoke-probe handshake.");
        }

        var probe = new NxToolsQuickLaunchProtocolProbe(pipe, reader, writer, iconPath);
        probe.pump.Start();
        return probe;
    }

    public void RequestPanel()
    {
        Write(new
        {
            protocol_version = 1,
            id = Guid.NewGuid(),
            correlation_id = (Guid?)null,
            type = "platform_event",
            payload = new
            {
                @event = "quick_launch_panel_requested",
                data = new
                {
                    target = new
                    {
                        window_handle = 0,
                        focus_window_handle = 0,
                        process_id = 0,
                        process_name = "quick-launch-smoke",
                        window_title = "quick-launch-smoke"
                    },
                    cursor_x = 0,
                    cursor_y = 0,
                    cursor_display_index = 0,
                    cursor_monitor_dpi = 96,
                    requested_at_unix_ms = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()
                }
            }
        });
    }

    private void PumpCommands()
    {
        try
        {
            while (!stopping)
            {
                var line = reader.ReadLine();
                if (line == null)
                    return;
                using var document = JsonDocument.Parse(line);
                var root = document.RootElement;
                if (root.GetProperty("type").GetString() != "platform_command")
                    continue;
                var commandId = root.GetProperty("id").GetGuid();
                var command = root.GetProperty("payload").GetProperty("command").GetString();
                Write(new
                {
                    protocol_version = 1,
                    id = Guid.NewGuid(),
                    correlation_id = (Guid?)commandId,
                    type = "platform_command_result",
                    payload = new
                    {
                        success = true,
                        error = (string)null,
                        data = command == "extract_shell_icon" ? iconPath : null
                    }
                });
            }
        }
        catch (IOException) when (stopping)
        {
        }
        catch (ObjectDisposedException) when (stopping)
        {
        }
    }

    private void Write(object envelope)
    {
        lock (writer)
            writer.WriteLine(JsonSerializer.Serialize(envelope));
    }

    public void Dispose()
    {
        stopping = true;
        try { pipe.Dispose(); } catch (ObjectDisposedException) { }
        pump.Join(1000);
        try { reader.Dispose(); } catch (ObjectDisposedException) { }
        try { writer.Dispose(); } catch (ObjectDisposedException) { }
    }
}
'@

$existing = Get-Process -Name NxTools -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and [IO.Path]::GetFullPath($_.Path) -eq $resolvedExecutable }
if ($existing) {
    $existing | Stop-Process -Force
}

$process = Start-Process -FilePath $resolvedExecutable -WorkingDirectory (Split-Path -Parent $resolvedExecutable) -PassThru
$probe = $null
try {
    Start-Sleep -Milliseconds $StartupMilliseconds
    $process.Refresh()
    if ($process.HasExited) {
        throw "NxTools exited during startup with code $($process.ExitCode)."
    }

    $platformHost = $null
    $platformHostDeadline = [DateTime]::UtcNow.AddSeconds(6)
    $expectedPlatformHost = [IO.Path]::GetFullPath([IO.Path]::Combine(
        (Split-Path -Parent $resolvedExecutable),
        'NxTools.PlatformHost.exe'))
    while (-not $platformHost -and [DateTime]::UtcNow -lt $platformHostDeadline) {
        $platformHost = Get-Process -Name 'NxTools.PlatformHost' -ErrorAction SilentlyContinue |
            Where-Object {
                $_.StartTime -ge $process.StartTime -and
                $_.Path -and
                [IO.Path]::GetFullPath($_.Path) -eq $expectedPlatformHost
            } |
            Sort-Object StartTime -Descending |
            Select-Object -First 1
        if (-not $platformHost) {
            Start-Sleep -Milliseconds 50
        }
    }
    if (-not $platformHost) {
        throw 'NxTools did not launch PlatformHost before the smoke-test timeout.'
    }
    if ($UseRealPlatformHost) {
        $requestedThroughPlatformHost = $false
        $messageWindowDeadline = [DateTime]::UtcNow.AddSeconds(3)
        while (-not $requestedThroughPlatformHost -and
            [DateTime]::UtcNow -lt $messageWindowDeadline) {
            $requestedThroughPlatformHost =
                [NxToolsQuickLaunchSmokeInput]::RequestPanelThroughRealPlatformHost(
                    [uint32]$platformHost.Id)
            if (-not $requestedThroughPlatformHost) {
                Start-Sleep -Milliseconds 25
            }
        }
        if (-not $requestedThroughPlatformHost) {
            throw 'The real PlatformHost message window was unavailable for the quick-launch request.'
        }
    }
    else {
        Stop-Process -Id $platformHost.Id -Force
        Start-Sleep -Milliseconds 250
        $probe = [NxToolsQuickLaunchProtocolProbe]::Connect($resolvedExecutable)
        $probe.RequestPanel()
    }

    $panelDeadline = [DateTime]::UtcNow.AddMilliseconds(2500)
    $focusedPanel = $false
    while (-not $focusedPanel -and [DateTime]::UtcNow -lt $panelDeadline) {
        Start-Sleep -Milliseconds 15
        $focusedPanel = [NxToolsQuickLaunchSmokeInput]::TryFocusCollapsedPanel([uint32]$process.Id)
    }
    if (-not $focusedPanel) {
        throw 'Quick-launch did not publish its collapsed search panel before the timeout.'
    }
    Start-Sleep -Milliseconds 120
    [NxToolsQuickLaunchSmokeInput]::TypeQuery()
    $resultDeadline = [DateTime]::UtcNow.AddMilliseconds($ResultTimeoutMilliseconds)
    $sweptResults = $false
    while (-not $sweptResults -and [DateTime]::UtcNow -lt $resultDeadline) {
        Start-Sleep -Milliseconds 15
        $process.Refresh()
        if ($process.HasExited) {
            throw "NxTools crashed while publishing search results with code $($process.ExitCode)."
        }
        $sweptResults = [NxToolsQuickLaunchSmokeInput]::TrySweepVisibleResults([uint32]$process.Id)
    }
    if (-not $sweptResults) {
        throw 'Quick-launch did not publish a populated result window before the timeout.'
    }
    if ($EnterActionsMode) {
        [NxToolsQuickLaunchSmokeInput]::EnterActionsMode()
        $actionModeDeadline = [DateTime]::UtcNow.AddSeconds(3)
        $actionModeVisible = $false
        while (-not $actionModeVisible -and [DateTime]::UtcNow -lt $actionModeDeadline) {
            Start-Sleep -Milliseconds 25
            $actionModeVisible = [NxToolsQuickLaunchSmokeInput]::IsActionModeSurface()
        }
        if (-not $actionModeVisible) {
            throw 'Quick-launch did not render the selected first action row before capture.'
        }
        [NxToolsQuickLaunchSmokeInput]::PointAtFirstAction()
    }
    $visualSettleMilliseconds = if ($UseRealPlatformHost) { 5000 } else { 900 }
    Start-Sleep -Milliseconds $visualSettleMilliseconds
    if (-not [string]::IsNullOrWhiteSpace($ScreenshotPath)) {
        Add-Type -AssemblyName System.Drawing.Common
        $captureBounds = [NxToolsQuickLaunchSmokeInput]::GetFocusedPanelBounds()
        $shadowPadding = 8
        $resolvedScreenshot = [IO.Path]::GetFullPath($ScreenshotPath)
        $screenshotDirectory = Split-Path -Parent $resolvedScreenshot
        if (-not [string]::IsNullOrWhiteSpace($screenshotDirectory)) {
            [IO.Directory]::CreateDirectory($screenshotDirectory) | Out-Null
        }
        $bitmap = [Drawing.Bitmap]::new(
            $captureBounds[2] + (2 * $shadowPadding),
            $captureBounds[3] + (2 * $shadowPadding))
        try {
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.CopyFromScreen(
                    $captureBounds[0] - $shadowPadding,
                    $captureBounds[1] - $shadowPadding,
                    0,
                    0,
                    [Drawing.Size]::new(
                        $captureBounds[2] + (2 * $shadowPadding),
                        $captureBounds[3] + (2 * $shadowPadding)))
            }
            finally {
                $graphics.Dispose()
            }
            $bitmap.Save($resolvedScreenshot, [Drawing.Imaging.ImageFormat]::Png)
        }
        finally {
            $bitmap.Dispose()
        }
    }

    $process.Refresh()
    if ($process.HasExited) {
        throw "NxTools crashed after search-result hover with code $($process.ExitCode)."
    }

    "PASS quick-launch search results survive first-row pointer hover (pid=$($process.Id))"
}
finally {
    if ($probe) {
        $probe.Dispose()
    }
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
