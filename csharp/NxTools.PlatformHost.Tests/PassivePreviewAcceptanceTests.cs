using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Automation;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class PassivePreviewAcceptanceTests
{
    [StructLayout(LayoutKind.Sequential)] private struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, System.Text.StringBuilder text, int count);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] private static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] private static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SendMessageTimeout(IntPtr window, uint message, IntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();

    private static IntPtr Owned(string variable, string expectedTitle)
    {
        var expectedProcess = uint.Parse(Environment.GetEnvironmentVariable("NXTOOLS_ACCEPTANCE_PID") ?? "0");
        var window = new IntPtr(long.Parse(Environment.GetEnvironmentVariable(variable) ?? "0"));
        _ = GetWindowThreadProcessId(window, out var actualProcess);
        if (expectedProcess == 0 || actualProcess != expectedProcess)
            throw new InvalidOperationException("Explicit owned Debug acceptance HWND/PID required.");
        var title = new System.Text.StringBuilder(200);
        _ = GetWindowText(window, title, title.Capacity);
        if (title.ToString() != expectedTitle)
            throw new InvalidOperationException("Synthetic fixture title mismatch; ordinary product targets prohibited.");
        using var process = Process.GetProcessById((int)expectedProcess);
        var expectedPath = Environment.GetEnvironmentVariable("NXTOOLS_ACCEPTANCE_EXE") ?? "";
        if (!string.Equals(process.MainModule?.FileName, expectedPath, StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("Owned executable path mismatch.");
        return window;
    }
    private static IntPtr Primary() => Owned("NXTOOLS_ACCEPTANCE_PRIMARY", "NxTools Native Acceptance — synthetic primary");
    private static IntPtr Passive() => Owned("NXTOOLS_ACCEPTANCE_PASSIVE", "NxTools Native Acceptance — synthetic passive");
    public static Task OwnedPassivePreviewRejectsMouseActivation()
    {
        var primary = Primary();
        var passive = Passive();
        if (IsWindowVisible(passive)) throw new InvalidOperationException("Passive fixture must begin hidden.");
        var noActivate = WindowsPlatformAdapter.SetNativeWindowNoActivateForTest(passive.ToInt64(), true);
        if (!noActivate.Success) throw new InvalidOperationException("Production noactivate application failed.");
        var before = GetForegroundWindow();
        if (SendMessageTimeout(passive, 0x0021, primary, new IntPtr((0x0201 << 16) | 1), 2, 2000, out var response) == IntPtr.Zero)
            throw new InvalidOperationException("Owned WM_MOUSEACTIVATE boundary timed out.");
        Console.WriteLine($"NATIVE_PASSIVE mouse_activate_result={response.ToInt64()} expected=3 dpi={GetDpiForWindow(passive)} foreground_unchanged={before == GetForegroundWindow()}");
        if (response.ToInt64() != 3)
            throw new InvalidOperationException("GPUI preview returns MA_ACTIVATE despite applied WS_EX_NOACTIVATE; passive pointer focus is not preserved.");
        if (SendMessageTimeout(primary, 0x0021, primary, new IntPtr((0x0201 << 16) | 1), 2, 2000, out var primaryResponse) == IntPtr.Zero || primaryResponse.ToInt64() != 1)
            throw new InvalidOperationException("Primary search window must retain MA_ACTIVATE.");
        if (!WindowsPlatformAdapter.SetNativeWindowNoActivateForTest(passive.ToInt64(), false).Success ||
            SendMessageTimeout(passive, 0x0021, primary, new IntPtr((0x0201 << 16) | 1), 2, 2000, out var restoredResponse) == IntPtr.Zero || restoredResponse.ToInt64() != 1)
            throw new InvalidOperationException("Disabling passive policy must restore ordinary activation.");
        if (!WindowsPlatformAdapter.SetNativeWindowNoActivateForTest(passive.ToInt64(), true).Success)
            throw new InvalidOperationException("Passive policy re-enable failed.");
        Console.WriteLine("NATIVE_PASSIVE primary_activate=1 disabled_policy_activate=1 enabled_policy_noactivate=3");
        return Task.CompletedTask;
    }
    public static Task OwnedClientAndUiaAcceptance()
    {
        var primary = Primary();
        var passive = Passive();
        var dpi = GetDpiForWindow(primary);
        var scale = dpi / 96.0;
        var logicalWidth = double.Parse(Environment.GetEnvironmentVariable("NXTOOLS_ACCEPTANCE_WIDTH") ?? "0", System.Globalization.CultureInfo.InvariantCulture);
        var logicalHeight = double.Parse(Environment.GetEnvironmentVariable("NXTOOLS_ACCEPTANCE_HEIGHT") ?? "0", System.Globalization.CultureInfo.InvariantCulture);
        var width = (int)Math.Round(logicalWidth * scale);
        var height = (int)Math.Round(logicalHeight * scale);
        if (width <= 0 || height <= 0) throw new InvalidOperationException("Explicit logical fixture budget required.");
        var invalid = WindowsPlatformAdapter.SetNativeWindowBoundsForTest(passive.ToInt64(), 900, 180, 0, height);
        if (invalid.Success) throw new InvalidOperationException("Invalid client budget must be rejected.");
        var before = GetForegroundWindow();
        var result = WindowsPlatformAdapter.SetNativeWindowBoundsForTest(passive.ToInt64(), 900, 180, width, height);
        if (!result.Success) throw new InvalidOperationException("Production final bounds application failed.");
        var shown = WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(passive.ToInt64(), true);
        if (!shown.Success) throw new InvalidOperationException("Production show-noactivate failed.");
        Thread.Sleep(300);
        if (!GetClientRect(passive, out var client)) throw new InvalidOperationException("Final native client unavailable.");
        var clientMatches = client.Right == width && client.Bottom == height;
        if (GetForegroundWindow() != before) throw new InvalidOperationException("Passive final-bounds/show stole foreground.");
        Console.WriteLine($"NATIVE_CLIENT dpi={dpi} width={client.Right} height={client.Bottom} expected_width={width} expected_height={height} matches={clientMatches} foreground_unchanged=true");
        var root = AutomationElement.FromHandle(primary);
        AutomationElement? input = null;
        AutomationElement? action = null;
        for (var attempt = 0; attempt < 10; attempt++)
        {
            input = root.FindFirst(TreeScope.Descendants, new PropertyCondition(AutomationElement.NameProperty, "Synthetic search input"));
            action = root.FindFirst(TreeScope.Descendants, new PropertyCondition(AutomationElement.NameProperty, "Synthetic acceptance action"));
            if (input is not null && action is not null) break;
            Thread.Sleep(100);
        }
        if (input is null || action is null) throw new InvalidOperationException("Actual UIA did not expose the synthetic named input/action.");
        Console.WriteLine($"NATIVE_UIA input_type={input.Current.ControlType.Id} action_type={action.Current.ControlType.Id} input_focusable={input.Current.IsKeyboardFocusable}");
        if (!input.Current.IsKeyboardFocusable || action.Current.ControlType != ControlType.Button)
            throw new InvalidOperationException("Actual UIA control roles/focus capability mismatch.");
        var hidden = WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(passive.ToInt64(), false);
        if (!hidden.Success || IsWindowVisible(passive)) throw new InvalidOperationException("Passive hide failed.");
        // Reapply and re-show the same owned HWND: no recreation or stale old size.
        var reapplied = WindowsPlatformAdapter.SetNativeWindowBoundsForTest(passive.ToInt64(), 920, 200, width, height);
        var reshown = WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(passive.ToInt64(), true);
        if (!reapplied.Success || !reshown.Success || !GetClientRect(passive, out var reused) || reused.Right != width || reused.Bottom != height)
            throw new InvalidOperationException("Reused passive HWND lost its client budget.");
        if (GetForegroundWindow() != before) throw new InvalidOperationException("Reused passive HWND stole foreground.");
        if (!WindowsPlatformAdapter.SetNativeWindowVisibilityForTest(passive.ToInt64(), false).Success)
            throw new InvalidOperationException("Reused passive hide failed.");
        if (!clientMatches) throw new InvalidOperationException("Production preview bounds apply outer dimensions rather than the requested client budget.");
        return Task.CompletedTask;
    }
}
