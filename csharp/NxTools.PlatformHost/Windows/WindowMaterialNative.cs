using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

// Zero is native acceptance, never visual/pixel evidence. No Accent getter exists.
internal interface IWindowMaterialNative
{
    int ValidateWindow(long window, uint processId);
    bool SupportsSystemBackdrop { get; }
    bool SupportsRoundedCorners { get; }
    bool SupportsAlphaComposition(long window);
    int SetDark(long window, bool dark);
    int SetBackdropType(long window, int type);
    int ExtendFrame(long window, bool fullClient);
    int SetRoundedCorners(long window);
    int SetAccent(long window, int state);
    int SetHostBackdropBrush(long window, bool enabled);
}

internal sealed class WindowMaterialNative : IWindowMaterialNative
{
    internal const int AccentDisabled = 0;
    // ACCENT_ENABLE_HOSTBACKDROP: DWM supplies the blurred host backdrop that the GPUI process's
    // Windows.UI.Composition visual paints (the Windows Terminal path). The Accent3 blur lags
    // window drags on Windows 10 1903+.
    internal const int AccentHostBackdrop = 5;
    // Bottom-only extension: a nonzero top inset makes DWM draw its own caption buttons.
    internal const int FullClientInset = 32767;

    public bool SupportsSystemBackdrop => ReadBuild(HasSystemBackdrop);
    public bool SupportsRoundedCorners => ReadBuild(HasRoundedCorners);

    private static bool ReadBuild(Func<uint, uint, bool> floor)
    {
        var result = ReadVersion(out var major, out var build);
        if (result != 0) throw new ExternalException("material_version_failed", result);
        return floor(major, build);
    }

    internal static bool HasSystemBackdrop(uint major, uint build) => major >= 10 && build >= 22621;
    internal static bool HasRoundedCorners(uint major, uint build) => major >= 10 && build >= 22000;

    public int ValidateWindow(long window, uint processId)
    {
        var hwnd = new IntPtr(window);
        if (window == 0 || !IsWindow(hwnd)) return 1400;
        if (GetWindowThreadProcessId(hwnd, out var actualProcess) == 0) return LastError();
        if (processId == 0 || actualProcess != processId) return 5;
        Marshal.SetLastPInvokeError(0);
        var style = IntPtr.Size == 8 ? GetWindowLongPtrW(hwnd, -16).ToInt64() : GetWindowLongW(hwnd, -16);
        var error = Marshal.GetLastWin32Error();
        if (style == 0 && error != 0) return error;
        if (!ProcessIdToSessionId(actualProcess, out var targetSession)) return LastError();
        if (!ProcessIdToSessionId((uint)Environment.ProcessId, out var hostSession)) return LastError();
        return IdentityError(window, GetAncestor(hwnd, 2).ToInt64(), (style & 0x40000000L) != 0,
            processId, actualProcess, targetSession, hostSession);
    }

    internal static int IdentityError(long window, long root, bool child, uint expectedProcess,
        uint actualProcess, uint targetSession, uint hostSession) =>
        window == 0 || root != window || child ? 1400
        : expectedProcess == 0 || expectedProcess != actualProcess || targetSession == 0 || targetSession != hostSession ? 5 : 0;

    public bool SupportsAlphaComposition(long window)
    {
        Marshal.SetLastPInvokeError(0);
        var style = IntPtr.Size == 8 ? GetWindowLongPtrW(new IntPtr(window), -20).ToInt64()
            : GetWindowLongW(new IntPtr(window), -20);
        var error = Marshal.GetLastWin32Error();
        if (style == 0 && error != 0) throw new System.ComponentModel.Win32Exception(error);
        return (style & 0x00200000L) != 0; // WS_EX_NOREDIRECTIONBITMAP selects GPUI's composition swapchain.
    }

    public int SetDark(long window, bool dark) => SetAttribute(window, 20, dark ? 1 : 0);
    public int SetBackdropType(long window, int type) => SetAttribute(window, 38, type);
    // DWMWA_USE_HOSTBACKDROPBRUSH (build 22000): the Windows 11 switch for host backdrop brushes.
    public int SetHostBackdropBrush(long window, bool enabled) => SetAttribute(window, 17, enabled ? 1 : 0);
    // DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND, then DWMWA_BORDER_COLOR = DWMWA_COLOR_NONE.
    public int SetRoundedCorners(long window)
    {
        var result = SetAttribute(window, 33, 2);
        return result != 0 ? result : SetAttribute(window, 34, unchecked((int)0xFFFFFFFE));
    }
    private static int SetAttribute(long window, int attribute, int value) =>
        DwmSetWindowAttribute(new IntPtr(window), attribute, ref value, sizeof(int));

    internal static Margins FrameMargins(bool fullClient) => new() { Bottom = fullClient ? FullClientInset : 0 };
    public int ExtendFrame(long window, bool fullClient)
    {
        var margins = FrameMargins(fullClient);
        return DwmExtendFrameIntoClientArea(new IntPtr(window), ref margins);
    }

    internal static AccentPolicy Policy(int state) => state switch
    {
        AccentDisabled => new() { AccentState = AccentDisabled, AccentFlags = 2, GradientColor = 0, AnimationId = 0 },
        AccentHostBackdrop => new() { AccentState = AccentHostBackdrop, AccentFlags = 0, GradientColor = 0, AnimationId = 0 },
        _ => throw new ArgumentOutOfRangeException(nameof(state))
    };

    public int SetAccent(long window, int state)
    {
        var policy = Policy(state);
        var size = Marshal.SizeOf<AccentPolicy>();
        var allocation = Marshal.AllocHGlobal(size);
        try
        {
            Marshal.StructureToPtr(policy, allocation, false);
            var data = new CompositionData { Attribute = 19, Data = allocation, Size = (nuint)size };
            Marshal.SetLastPInvokeError(0);
            return SetWindowCompositionAttribute(new IntPtr(window), ref data) ? 0 : LastError();
        }
        finally { Marshal.FreeHGlobal(allocation); }
    }

    internal static int LastError()
    {
        var error = Marshal.GetLastWin32Error();
        return error == 0 ? -1 : error;
    }

    internal static int ReadVersion(out uint major, out uint build)
    {
        var info = new OsVersion { Size = (uint)Marshal.SizeOf<OsVersion>(), ServicePack = "" };
        var result = RtlGetVersion(ref info);
        major = info.Major;
        build = info.Build;
        return result;
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct AccentPolicy { internal int AccentState, AccentFlags, GradientColor, AnimationId; }
    [StructLayout(LayoutKind.Sequential)]
    internal struct CompositionData { internal int Attribute; internal nint Data; internal nuint Size; }
    [StructLayout(LayoutKind.Sequential)]
    internal struct Margins { internal int Left, Right, Top, Bottom; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct OsVersion
    {
        internal uint Size, Major, Minor, Build, Platform;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] internal string ServicePack;
    }

    [DllImport("ntdll.dll", ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern int RtlGetVersion(ref OsVersion info);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowCompositionAttribute(nint window, ref CompositionData data);
    [DllImport("dwmapi.dll", ExactSpelling = true)]
    private static extern int DwmSetWindowAttribute(nint window, int attribute, ref int value, int size);
    [DllImport("dwmapi.dll", ExactSpelling = true)]
    private static extern int DwmExtendFrameIntoClientArea(nint window, ref Margins margins);
    [DllImport("user32.dll", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool IsWindow(nint window);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(nint window, out uint processId);
    [DllImport("user32.dll", ExactSpelling = true)]
    private static extern nint GetAncestor(nint window, uint flags);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    private static extern nint GetWindowLongPtrW(nint window, int index);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    private static extern int GetWindowLongW(nint window, int index);
    [DllImport("kernel32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool ProcessIdToSessionId(uint processId, out uint sessionId);
}
