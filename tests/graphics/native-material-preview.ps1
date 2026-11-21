[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$correlation = [Guid]::NewGuid()
$started = [Diagnostics.Stopwatch]::StartNew()

try {
    # Parse both entry points without executing the launcher or its Cargo builder.
    foreach ($scriptPath in @((Join-Path $root 'scripts\preview-native-material.ps1'), $PSCommandPath)) {
        $parseTokens = $null
        $parseErrors = $null
        $null = [Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$parseTokens, [ref]$parseErrors)
        if ($parseErrors.Count) { throw 'Native material PowerShell AST validation failed.' }
    }
    Write-Host "ui.native_material.fixture.ast_passed correlation=$correlation scripts=2"

    $helperSource = [IO.File]::ReadAllText((Join-Path $root 'scripts\preview-native-material-window.cs'))
    $fixtureSource = @'
public sealed class NativeMaterialFixtureResult
{
    public int Assertions { get; internal set; }
    public NativeMaterialPreviewReceipt Acrylic { get; internal set; }
    public NativeMaterialPreviewReceipt Solid { get; internal set; }
    public NativeMaterialPreviewThemeReceipt LightTheme { get; internal set; }
    public NativeMaterialPreviewThemeReceipt DarkTheme { get; internal set; }
    public NativeMaterialPreviewReceipt CaptionAcrylic { get; internal set; }
    public NativeMaterialPreviewReceipt CaptionSolid { get; internal set; }
}

public static class NativeMaterialFixture
{
    [System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
    private struct Rect { public int Left; public int Top; public int Right; public int Bottom; }
    [System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]
    private struct Point { public int X; public int Y; }
    [System.Runtime.InteropServices.DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(System.IntPtr window, int attribute, ref int value, int size);
    [System.Runtime.InteropServices.DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(System.IntPtr window, int attribute, out int value, int size);
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern System.IntPtr GetForegroundWindow();
    [System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool GetClientRect(System.IntPtr window, out Rect rect);
    [System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool ClientToScreen(System.IntPtr window, ref Point point);

    [System.Runtime.InteropServices.DllImport("user32.dll", CharSet = System.Runtime.InteropServices.CharSet.Unicode, SetLastError = true)]
    private static extern System.IntPtr CreateWindowEx(uint extendedStyle, string className, string title, uint style,
        int x, int y, int width, int height, System.IntPtr parent, System.IntPtr menu, System.IntPtr instance, System.IntPtr parameter);
    [System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool DestroyWindow(System.IntPtr window);
    [System.Runtime.InteropServices.DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")]
    private static extern System.IntPtr GetWindowLongPtr64(System.IntPtr window, int index);
    [System.Runtime.InteropServices.DllImport("user32.dll", EntryPoint = "GetWindowLongW")]
    private static extern int GetWindowLong32(System.IntPtr window, int index);
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern System.IntPtr SetThreadDpiAwarenessContext(System.IntPtr context);
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern System.IntPtr GetThreadDpiAwarenessContext();
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool AreDpiAwarenessContextsEqual(System.IntPtr first, System.IntPtr second);
    [System.Runtime.InteropServices.DllImport("user32.dll")]
    private static extern System.IntPtr GetWindow(System.IntPtr window, uint command);
    [System.Runtime.InteropServices.DllImport("dwmapi.dll", EntryPoint = "DwmGetWindowAttribute")]
    private static extern int ReadVisibleFrame(System.IntPtr window, int attribute, out Rect rect, int size);

    private static long Style(System.IntPtr window, int index)
    {
        return (System.IntPtr.Size == 8 ? GetWindowLongPtr64(window, index).ToInt64() : GetWindowLong32(window, index)) & 0xffffffffL;
    }

    private static bool ClientOnly(System.IntPtr window)
    {
        Rect visible;
        int result = ReadVisibleFrame(window, 9, out visible, System.Runtime.InteropServices.Marshal.SizeOf(typeof(Rect)));
        if (result != 0) throw new NativeMaterialPreviewException("fixture_visible_frame", result, result);
        Rect client = ClientBounds(window);
        return System.Math.Abs(client.Left - visible.Left) <= 1 && System.Math.Abs(client.Top - visible.Top) <= 1 &&
            System.Math.Abs(client.Right - visible.Right) <= 1 && System.Math.Abs(client.Bottom - visible.Bottom) <= 1;
    }

    private static void CheckFixedReceipt(NativeMaterialPreviewReceipt receipt, long style, long extendedStyle)
    {
        Check(receipt.FrameBeforeStyle == style && receipt.FrameBeforeExtendedStyle == extendedStyle, "caption_before_style");
        Check(receipt.FrameAfterStyle == (style & ~0xc00000L) && receipt.FrameAfterExtendedStyle == (extendedStyle & ~0x100L), "caption_only_bits_removed");
        Check((receipt.FrameAfterStyle & 0xb0000L) == (style & 0xb0000L), "caption_sysmenu_minmax_preserved");
        Check((receipt.FrameAfterExtendedStyle & 0x240000L) == 0x240000L, "caption_noredir_appwindow_preserved");
        Check(receipt.FrameClientGeometryPreserved && receipt.FrameForegroundPreserved && receipt.FrameClientOnly, "caption_frame_receipt");
        Check(receipt.FrameAfterWindowWidth == receipt.FrameAfterClientWidth && receipt.FrameAfterWindowHeight == receipt.FrameAfterClientHeight, "caption_outer_equals_client");
    }

    private static void RunFixedFrame(System.Guid correlation, int processId, NativeMaterialFixtureResult result)
    {
        System.IntPtr priorContext = SetThreadDpiAwarenessContext(new System.IntPtr(-4));
        Check(priorContext != System.IntPtr.Zero, "fixture_dpi_enter");
        System.IntPtr window = System.IntPtr.Zero;
        try
        {
            const string title = "NxTools \u2014 Native Material Preview (F2 / Esc)";
            // Hidden fixed top-level STATIC has a real caption, edge, SysMenu, min/max and NOREDIR.
            window = CreateWindowEx(0x240100, "STATIC", title, 0x04cb0000, 140, 160, 496, 548,
                System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero);
            Check(window != System.IntPtr.Zero, "caption_create");
            long style = Style(window, -16), extendedStyle = Style(window, -20);
            Check((style & 0xc00000L) == 0xc00000L && (extendedStyle & 0x100L) != 0, "caption_fixture_precondition");
            Check(!ClientOnly(window), "caption_collar_red_precondition");
            Check(NativeMaterialPreviewWindow.FindWindow(processId) == window.ToInt64(), "caption_unique_selection");
            Rect client = ClientBounds(window);
            System.IntPtr foreground = GetForegroundWindow();
            System.IntPtr above = GetWindow(window, 3), below = GetWindow(window, 2);
            System.IntPtr callerContext = GetThreadDpiAwarenessContext();
            result.CaptionAcrylic = NativeMaterialPreviewWindow.Prepare(processId, true, false, correlation);
            CheckFixedReceipt(result.CaptionAcrylic, style, extendedStyle);
            Check(result.CaptionAcrylic.FrameBeforeLeftInset != 0 || result.CaptionAcrylic.FrameBeforeTopInset != 0 ||
                result.CaptionAcrylic.FrameBeforeRightInset != 0 || result.CaptionAcrylic.FrameBeforeBottomInset != 0, "caption_nonzero_before_receipt");
            Check(ClientOnly(window), "caption_acrylic_visible_alignment");
            CheckUnchanged(window, client, foreground, "caption_acrylic");
            Check(GetWindow(window, 3) == above && GetWindow(window, 2) == below, "caption_z_order_preserved");
            Check(AreDpiAwarenessContextsEqual(callerContext, GetThreadDpiAwarenessContext()), "caption_dpi_restored");
            long normalizedStyle = Style(window, -16), normalizedExtendedStyle = Style(window, -20);
            NativeMaterialPreviewReceipt repeat = NativeMaterialPreviewWindow.Prepare(processId, true, false, correlation);
            CheckFixedReceipt(repeat, normalizedStyle, normalizedExtendedStyle);
            CheckUnchanged(window, client, foreground, "caption_acrylic_repeat");
            Check(ClientOnly(window), "caption_acrylic_repeat_alignment");
            result.CaptionSolid = NativeMaterialPreviewWindow.Prepare(processId, false, true, correlation);
            CheckFixedReceipt(result.CaptionSolid, normalizedStyle, normalizedExtendedStyle);
            CheckUnchanged(window, client, foreground, "caption_solid");
            Check(ClientOnly(window) && result.CaptionSolid.RequestedAccentState == 0 && result.CaptionSolid.AccentAccepted, "caption_solid_alignment_policy");
            repeat = NativeMaterialPreviewWindow.Prepare(processId, false, true, correlation);
            CheckFixedReceipt(repeat, normalizedStyle, normalizedExtendedStyle);
            CheckUnchanged(window, client, foreground, "caption_solid_repeat");
            Check(ClientOnly(window), "caption_solid_repeat_alignment");
            Check(DestroyWindow(window), "caption_destroy");
            window = System.IntPtr.Zero;

            window = CreateWindowEx(0x240100, "STATIC", title, 0x04cb0000, 140, 160, 496, 548,
                System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero);
            Check(window != System.IntPtr.Zero && !ClientOnly(window), "solid_caption_collar_red_precondition");
            style = Style(window, -16); extendedStyle = Style(window, -20);
            client = ClientBounds(window); foreground = GetForegroundWindow();
            result.CaptionSolid = NativeMaterialPreviewWindow.Prepare(processId, false, true, correlation);
            CheckFixedReceipt(result.CaptionSolid, style, extendedStyle);
            CheckUnchanged(window, client, foreground, "solid_fresh_caption");
            Check(ClientOnly(window) && result.CaptionSolid.RequestedAccentState == 0 && result.CaptionSolid.AccentAccepted, "solid_fresh_caption_alignment_policy");
            Check(DestroyWindow(window), "solid_caption_destroy");
            window = System.IntPtr.Zero;

            window = CreateWindowEx(0x240100, "STATIC", title, 0x04cf0000, 140, 160, 496, 548,
                System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero, System.IntPtr.Zero);
            Check(window != System.IntPtr.Zero, "thickframe_create");
            style = Style(window, -16); extendedStyle = Style(window, -20);
            client = ClientBounds(window); foreground = GetForegroundWindow();
            callerContext = GetThreadDpiAwarenessContext();
            Reject(delegate { NativeMaterialPreviewWindow.Prepare(processId, true, false, correlation); }, "frame_resizable_rejected");
            Check(Style(window, -16) == style && Style(window, -20) == extendedStyle, "thickframe_rejected_without_style_write");
            CheckUnchanged(window, client, foreground, "thickframe_rejected");
            Check(AreDpiAwarenessContextsEqual(callerContext, GetThreadDpiAwarenessContext()), "thickframe_dpi_restored");
        }
        finally
        {
            if (window != System.IntPtr.Zero) DestroyWindow(window);
            SetThreadDpiAwarenessContext(priorContext);
        }
    }

    private static int assertions;
    private static void Check(bool condition, string stage)
    {
        if (!condition) throw new NativeMaterialPreviewException("fixture_" + stage, 0, unchecked((int)0x80004005));
        assertions++;
    }

    private static Rect ClientBounds(System.IntPtr window)
    {
        Rect rect;
        if (!GetClientRect(window, out rect)) throw new System.ComponentModel.Win32Exception(System.Runtime.InteropServices.Marshal.GetLastWin32Error());
        Point origin = new Point();
        if (!ClientToScreen(window, ref origin)) throw new System.ComponentModel.Win32Exception(System.Runtime.InteropServices.Marshal.GetLastWin32Error());
        rect.Right += origin.X;
        rect.Bottom += origin.Y;
        rect.Left += origin.X;
        rect.Top += origin.Y;
        return rect;
    }

    private static void CheckUnchanged(System.IntPtr window, Rect before, System.IntPtr foreground, string stage)
    {
        Rect after = ClientBounds(window);
        Check(before.Left == after.Left && before.Top == after.Top && before.Right == after.Right && before.Bottom == after.Bottom, stage + "_client_bounds");
        Check(GetForegroundWindow() == foreground, stage + "_foreground");
    }

    private static NativeMaterialPreviewException Reject(System.Action action, string expectedStage)
    {
        try { action(); }
        catch (NativeMaterialPreviewException exception)
        {
            Check(exception.Stage == expectedStage, "rejection_stage");
            Check(exception.HResult < 0, "rejection_hresult");
            return exception;
        }
        throw new NativeMaterialPreviewException("fixture_missing_" + expectedStage, 0, unchecked((int)0x80004005));
    }

    private static void CheckTheme(NativeMaterialPreviewThemeReceipt receipt, bool dark, System.IntPtr window)
    {
        Check(receipt != null && receipt.Dark == dark && receipt.SetDarkHResult == 0, "theme_native_set");
        int value;
        int nativeResult = DwmGetWindowAttribute(window, 20, out value, sizeof(int));
        Check(receipt.ReadDarkHResult == nativeResult, "theme_read_result");
        if (nativeResult == 0)
            Check(receipt.DarkReadbackSupported && value == (dark ? 1 : 0), "theme_native_readback");
        else
            Check(!receipt.DarkReadbackSupported && nativeResult == unchecked((int)0x80070057), "theme_readback_unsupported");
    }

    public static NativeMaterialFixtureResult Run(System.Guid correlation)
    {
        assertions = 0;
        System.Windows.Forms.Form form = new System.Windows.Forms.Form();
        try
        {
            // One hidden, owned HWND: no Show, activation, normal PlatformHost or product runtime.
            form.Text = "NxTools \u2014 Native Material Preview (F2 / Esc)";
            form.FormBorderStyle = System.Windows.Forms.FormBorderStyle.None;
            form.StartPosition = System.Windows.Forms.FormStartPosition.Manual;
            form.Location = new System.Drawing.Point(80, 80);
            form.ClientSize = new System.Drawing.Size(420, 280);
            System.IntPtr window = form.Handle;
            int processId;
            using (System.Diagnostics.Process process = System.Diagnostics.Process.GetCurrentProcess()) { processId = process.Id; }
            Rect client = ClientBounds(window);
            System.IntPtr foreground = GetForegroundWindow();
            Check(NativeMaterialPreviewWindow.FindWindow(processId) == window.ToInt64(), "owned_selection");
            Reject(delegate { NativeMaterialPreviewWindow.VerifyBackdrop(processId + 1, window.ToInt64(), 1); }, "window_identity");
            Reject(delegate { NativeMaterialPreviewWindow.FindWindow(0); }, "process_identity");

            form.Text = "NxTools - Native Material Preview (F2 / Esc)";
            Reject(delegate { NativeMaterialPreviewWindow.VerifyBackdrop(processId, window.ToInt64(), 1); }, "window_identity");
            Reject(delegate { NativeMaterialPreviewWindow.FindWindow(processId); }, "window_timeout");
            form.Text = "NxTools \u2014 Native Material Preview (F2 / Esc)";

            // DWM readback-guard test only: no Accent getter or visual Acrylic assertion.
            int none = 1;
            int nativeResult = DwmSetWindowAttribute(window, 38, ref none, sizeof(int));
            if (nativeResult != 0) throw new NativeMaterialPreviewException("fixture_none_set", nativeResult, nativeResult);
            Check(NativeMaterialPreviewWindow.VerifyBackdrop(processId, window.ToInt64(), 1).Backdrop == 1, "none_baseline");
            NativeMaterialPreviewException rejected = Reject(delegate { NativeMaterialPreviewWindow.VerifyBackdrop(processId, window.ToInt64(), 3); }, "backdrop_mismatch");
            Check(rejected.NativeResult == 0, "dwm_readback_guard_rejects_none_as_three");
            CheckUnchanged(window, client, foreground, "dwm_readback_guard");

            NativeMaterialFixtureResult result = new NativeMaterialFixtureResult();
            result.Acrylic = NativeMaterialPreviewWindow.Prepare(processId, true, false, correlation);
            Check(result.Acrylic.WindowHandle == window.ToInt64() && result.Acrylic.ProcessId == processId, "acrylic_identity");
            Check(result.Acrylic.Correlation == correlation && result.Acrylic.OsBuild >= 22621, "acrylic_correlation_build");
            Check(result.Acrylic.RequestedBackdrop == 1 && result.Acrylic.Backdrop == 1, "acrylic_dwm_none");
            Check(result.Acrylic.Mechanism == "wca_accent_policy" && result.Acrylic.RequestedAccentState == 4, "acrylic_accent_policy");
            Check(result.Acrylic.AccentAccepted && result.Acrylic.AccentLastError == 0, "acrylic_accent_bool_acceptance");
            Check(result.Acrylic.Corner == 2 && !result.Acrylic.NcRenderingEnabled, "acrylic_corner_frame");
            Check(result.Acrylic.Dark && result.Acrylic.ExtendFrameHResult == 0, "acrylic_dark_extend");
            CheckUnchanged(window, client, foreground, "acrylic");

            string themePrefix = "ui.material_preview.theme_requested correlation=" + correlation.ToString("D") + " dark=";
            NativeMaterialPreviewThemeReceipt theme;
            Check(!NativeMaterialPreviewWindow.TryApplyThemeRequest("ui.material_preview.theme_requested correlation=" + System.Guid.NewGuid().ToString("D") + " dark=false", correlation, processId, window.ToInt64(), out theme) && theme == null, "wrong_correlation_ignored");
            Check(!NativeMaterialPreviewWindow.TryApplyThemeRequest("ui.material_preview.started", correlation, processId, window.ToInt64(), out theme) && theme == null, "noop_line_ignored");
            Check(!NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "False", correlation, processId, window.ToInt64(), out theme) && theme == null, "non_boolean_ignored");
            Check(!NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "false trailing", correlation, processId, window.ToInt64(), out theme) && theme == null, "suffix_ignored");
            Reject(delegate { NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "false", correlation, processId + 1, window.ToInt64(), out theme); }, "window_identity");
            form.Text = "NxTools - Native Material Preview (F2 / Esc)";
            Reject(delegate { NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "false", correlation, processId, window.ToInt64(), out theme); }, "window_identity");
            form.Text = "NxTools \u2014 Native Material Preview (F2 / Esc)";
            Check(NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "false", correlation, processId, window.ToInt64(), out theme), "light_request_accepted");
            result.LightTheme = theme;
            CheckTheme(theme, false, window);
            CheckUnchanged(window, client, foreground, "light_theme");
            Check(NativeMaterialPreviewWindow.TryApplyThemeRequest(themePrefix + "true", correlation, processId, window.ToInt64(), out theme), "dark_request_accepted");
            result.DarkTheme = theme;
            CheckTheme(theme, true, window);
            CheckUnchanged(window, client, foreground, "dark_theme");
            Check(NativeMaterialPreviewWindow.VerifyBackdrop(processId, window.ToInt64(), 1).Corner == 2, "theme_preserves_backdrop_corner");

            result.Solid = NativeMaterialPreviewWindow.Prepare(processId, false, true, correlation);
            Check(result.Solid.RequestedBackdrop == 1 && result.Solid.Backdrop == 1, "solid_reset");
            Check(result.Solid.Mechanism == "wca_accent_policy" && result.Solid.RequestedAccentState == 0, "solid_accent_disabled_policy");
            Check(result.Solid.AccentAccepted && result.Solid.AccentLastError == 0, "solid_accent_bool_acceptance");
            Check(result.Solid.Corner == 2 && !result.Solid.NcRenderingEnabled && !result.Solid.Dark, "solid_corner_frame_light");
            CheckUnchanged(window, client, foreground, "solid");
            form.Text = "Owned material fixture retired";
            RunFixedFrame(correlation, processId, result);
            result.Assertions = assertions;
            return result;
        }
        finally
        {
            form.Close();
            form.Dispose();
        }
    }
}
'@
    if (-not ('NativeMaterialFixture' -as [type])) {
        Add-Type -TypeDefinition ($helperSource + "`r`n" + $fixtureSource) `
            -ReferencedAssemblies 'System.Windows.Forms', 'System.Drawing'
    }
    $windowsBuild = [NativeMaterialPreviewWindow]::GetWindowsBuild()
    if ($windowsBuild -lt 22621) {
        Write-Host "ui.native_material.fixture.skipped correlation=$correlation stage=unsupported_build build=$windowsBuild elapsed_ms=$($started.ElapsedMilliseconds)"
        exit 3
    }
    Write-Host "ui.native_material.fixture.started correlation=$correlation build=$windowsBuild"
    $fixtureResult = [NativeMaterialFixture]::Run($correlation)
    Write-Host ('ui.native_material.fixture.acrylic_receipt ' + ($fixtureResult.Acrylic | ConvertTo-Json -Compress))
    Write-Host ('ui.native_material.fixture.solid_receipt ' + ($fixtureResult.Solid | ConvertTo-Json -Compress))
    Write-Host ('ui.native_material.fixture.light_theme_receipt ' + ($fixtureResult.LightTheme | ConvertTo-Json -Compress))
    Write-Host ('ui.native_material.fixture.dark_theme_receipt ' + ($fixtureResult.DarkTheme | ConvertTo-Json -Compress))
    Write-Host ('ui.native_material.fixture.caption_acrylic_receipt ' + ($fixtureResult.CaptionAcrylic | ConvertTo-Json -Compress))
    Write-Host ('ui.native_material.fixture.caption_solid_receipt ' + ($fixtureResult.CaptionSolid | ConvertTo-Json -Compress))
    Write-Host "ui.native_material.fixture.passed correlation=$correlation assertions=$($fixtureResult.Assertions) elapsed_ms=$($started.ElapsedMilliseconds)"
    exit 0
}
catch {
    $failure = $_.Exception
    while ($failure.InnerException) { $failure = $failure.InnerException }
    $stage = 'fixture'
    $nativeResult = 0
    if ($failure.GetType().Name -eq 'NativeMaterialPreviewException') { $stage = $failure.Stage; $nativeResult = $failure.NativeResult }
    Write-Host "ui.native_material.fixture.failed correlation=$correlation stage=$stage hresult=$($failure.HResult) native_result=$nativeResult elapsed_ms=$($started.ElapsedMilliseconds)"
    Write-Error 'Native material fixture failed; inspect the correlated stage and native results.' -ErrorAction Continue
    exit 1
}
