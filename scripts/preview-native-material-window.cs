using System;
using System.Diagnostics;
using System.IO;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public sealed class NativeMaterialPreviewException : Exception
{
    public string Stage { get; private set; }
    public int NativeResult { get; private set; }

    public NativeMaterialPreviewException(string stage, int nativeResult, int exceptionHResult)
        : base("Native material preview failed at " + stage + ".")
    {
        Stage = stage;
        NativeResult = nativeResult;
        HResult = exceptionHResult;
    }
}

public sealed class NativeMaterialPreviewReadback
{
    public long WindowHandle { get; internal set; }
    public int ProcessId { get; internal set; }
    public int Backdrop { get; internal set; }
    public int Corner { get; internal set; }
    public bool NcRenderingEnabled { get; internal set; }
    public int ReadBackdropHResult { get; internal set; }
    public int ReadCornerHResult { get; internal set; }
    public int ReadNcRenderingHResult { get; internal set; }
}

// DWM readback and Accent BOOL confirm native acceptance, not visible desktop blur.
public sealed class NativeMaterialPreviewReceipt
{
    public Guid Correlation { get; internal set; }
    public long WindowHandle { get; internal set; }
    public int ProcessId { get; internal set; }
    public uint OsMajor { get; internal set; }
    public uint OsMinor { get; internal set; }
    public uint OsBuild { get; internal set; }
    public int RequestedBackdrop { get; internal set; }
    public int Backdrop { get; internal set; }
    public string Mechanism { get; internal set; }
    public int RequestedAccentState { get; internal set; }
    public bool AccentAccepted { get; internal set; }
    public int AccentLastError { get; internal set; }
    public int Corner { get; internal set; }
    public bool NcRenderingEnabled { get; internal set; }
    public bool Dark { get; internal set; }
    public long ElapsedMilliseconds { get; internal set; }
    public int VersionNativeResult { get; internal set; }
    public int SetNcRenderingHResult { get; internal set; }
    public int ExtendFrameHResult { get; internal set; }
    public int SetDarkHResult { get; internal set; }
    public int SetCornerHResult { get; internal set; }
    public int SetBackdropHResult { get; internal set; }
    public int ReadNcRenderingHResult { get; internal set; }
    public int ReadCornerHResult { get; internal set; }
    public int ReadBackdropHResult { get; internal set; }
    public long FrameBeforeStyle { get; internal set; }
    public long FrameBeforeExtendedStyle { get; internal set; }
    public long FrameAfterStyle { get; internal set; }
    public long FrameAfterExtendedStyle { get; internal set; }
    public int FrameBeforeWindowWidth { get; internal set; }
    public int FrameBeforeWindowHeight { get; internal set; }
    public int FrameAfterWindowWidth { get; internal set; }
    public int FrameAfterWindowHeight { get; internal set; }
    public int FrameBeforeClientX { get; internal set; }
    public int FrameBeforeClientY { get; internal set; }
    public int FrameBeforeClientWidth { get; internal set; }
    public int FrameBeforeClientHeight { get; internal set; }
    public int FrameAfterClientX { get; internal set; }
    public int FrameAfterClientY { get; internal set; }
    public int FrameAfterClientWidth { get; internal set; }
    public int FrameAfterClientHeight { get; internal set; }
    public int FrameBeforeLeftInset { get; internal set; }
    public int FrameBeforeTopInset { get; internal set; }
    public int FrameBeforeRightInset { get; internal set; }
    public int FrameBeforeBottomInset { get; internal set; }
    public int FrameAfterLeftInset { get; internal set; }
    public int FrameAfterTopInset { get; internal set; }
    public int FrameAfterRightInset { get; internal set; }
    public int FrameAfterBottomInset { get; internal set; }
    public bool FrameClientGeometryPreserved { get; internal set; }
    public bool FrameForegroundPreserved { get; internal set; }
    public bool FrameClientOnly { get; internal set; }
}

public sealed class NativeMaterialPreviewThemeReceipt
{
    public Guid Correlation { get; internal set; }
    public long WindowHandle { get; internal set; }
    public int ProcessId { get; internal set; }
    public bool Dark { get; internal set; }
    public int SetDarkHResult { get; internal set; }
    public int ReadDarkHResult { get; internal set; }
    public bool DarkReadbackSupported { get; internal set; }
    public long ElapsedMilliseconds { get; internal set; }
}

// Only the standalone example's PID and exact title are eligible for mutation.
public static class NativeMaterialPreviewWindow
{
    private const string PreviewTitle = "NxTools \u2014 Native Material Preview (F2 / Esc)";
    private const int Failure = unchecked((int)0x80004005);
    private const int InvalidArgument = unchecked((int)0x80070057);
    private delegate bool EnumWindowCallback(IntPtr window, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct OsVersionInfo
    {
        public uint Size;
        public uint Major;
        public uint Minor;
        public uint Build;
        public uint PlatformId;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)]
        public string ServicePack;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Margins
    {
        public int Left;
        public int Right;
        public int Top;
        public int Bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct AccentPolicy
    {
        public int AccentState;
        public int AccentFlags;
        public int GradientColor;
        public int AnimationId;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct WindowCompositionAttribData
    {
        public int Attribute;
        public IntPtr Data;
        public UIntPtr Size;
    }

    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowCompositionAttribute(IntPtr window, ref WindowCompositionAttribData data);
    [DllImport("kernel32.dll", EntryPoint = "SetLastError", ExactSpelling = true)]
    private static extern void ClearLastError(uint code);

    [DllImport("ntdll.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern int RtlGetVersion(ref OsVersionInfo version);
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumWindows(EnumWindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr window, StringBuilder text, int capacity);
    [DllImport("dwmapi.dll")]
    private static extern int DwmSetWindowAttribute(IntPtr window, int attribute, ref int value, int size);
    [DllImport("dwmapi.dll")]
    private static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out int value, int size);
    [DllImport("dwmapi.dll")]
    private static extern int DwmExtendFrameIntoClientArea(IntPtr window, ref Margins margins);

    [StructLayout(LayoutKind.Sequential)]
    private struct FrameRect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)]
    private struct FramePoint { public int X, Y; }
    private sealed class FrameFacts
    {
        public FrameRect Outer, Client, Visible;
        public FramePoint Origin;
        public long Style, ExtendedStyle;
    }
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetWindowRect(IntPtr window, out FrameRect rect);
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetClientRect(IntPtr window, out FrameRect rect);
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientToScreen(IntPtr window, ref FramePoint point);
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW", SetLastError = true)]
    private static extern IntPtr GetWindowLongPtr64(IntPtr window, int index);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongW", SetLastError = true)]
    private static extern int GetWindowLong32(IntPtr window, int index);
    [DllImport("user32.dll", EntryPoint = "SetWindowLongPtrW", SetLastError = true)]
    private static extern IntPtr SetWindowLongPtr64(IntPtr window, int index, IntPtr value);
    [DllImport("user32.dll", EntryPoint = "SetWindowLongW", SetLastError = true)]
    private static extern int SetWindowLong32(IntPtr window, int index, int value);
    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("dwmapi.dll", EntryPoint = "DwmGetWindowAttribute")]
    private static extern int DwmGetFrameAttribute(IntPtr window, int attribute, out FrameRect value, int size);

    private static NativeMaterialPreviewException FrameNativeFailure(string stage)
    {
        int code = Marshal.GetLastWin32Error();
        return new NativeMaterialPreviewException(stage, code,
            code == 0 ? Failure : unchecked((int)(0x80070000u | ((uint)code & 0xffffu))));
    }

    private static long ReadStyle(IntPtr window, int index)
    {
        ClearLastError(0);
        long value = IntPtr.Size == 8 ? GetWindowLongPtr64(window, index).ToInt64() : GetWindowLong32(window, index);
        if (value == 0 && Marshal.GetLastWin32Error() != 0) throw FrameNativeFailure("frame_style_read");
        return value & 0xffffffffL;
    }

    private static void WriteStyle(IntPtr window, int processId, int index, long value)
    {
        VerifyIdentity(window, processId);
        ClearLastError(0);
        long previous = IntPtr.Size == 8 ? SetWindowLongPtr64(window, index, new IntPtr(value)).ToInt64()
            : SetWindowLong32(window, index, unchecked((int)value));
        if (previous == 0 && Marshal.GetLastWin32Error() != 0) throw FrameNativeFailure("frame_style_set");
    }

    private static FrameFacts ReadFrame(IntPtr window, int processId)
    {
        VerifyIdentity(window, processId);
        FrameFacts frame = new FrameFacts();
        if (!GetWindowRect(window, out frame.Outer)) throw FrameNativeFailure("frame_outer_read");
        if (!GetClientRect(window, out frame.Client)) throw FrameNativeFailure("frame_client_read");
        if (!ClientToScreen(window, ref frame.Origin)) throw FrameNativeFailure("frame_origin_read");
        int result = DwmGetFrameAttribute(window, 9, out frame.Visible, Marshal.SizeOf(typeof(FrameRect)));
        if (result != 0) throw new NativeMaterialPreviewException("frame_visible_read", result, result);
        frame.Style = ReadStyle(window, -16);
        frame.ExtendedStyle = ReadStyle(window, -20);
        return frame;
    }

    // Fixed standalone surfaces only. Real WM_MOVE/WM_SIZE remain the GPUI geometry owner;
    // this does not refresh saved-border caches or authorize resizable product windows.
    private static void NormalizeFixedClientFrame(IntPtr window, int processId, NativeMaterialPreviewReceipt receipt)
    {
        IntPtr previousContext = SetThreadDpiAwarenessContext(new IntPtr(-4)); // PMv2, thread-local only
        if (previousContext == IntPtr.Zero) throw FrameNativeFailure("frame_dpi_enter");
        try
        {
            FrameFacts before = ReadFrame(window, processId);
            if ((before.Style & 0x00040000L) != 0)
                throw new NativeMaterialPreviewException("frame_resizable_rejected", 0, InvalidArgument);
            int width = before.Client.Right - before.Client.Left;
            int height = before.Client.Bottom - before.Client.Top;
            if (width <= 0 || height <= 0) throw new NativeMaterialPreviewException("frame_client_empty", 0, InvalidArgument);
            IntPtr foreground = GetForegroundWindow();
            long style = before.Style & ~0x00c00000L;
            long extendedStyle = before.ExtendedStyle & ~0x00000100L;
            if (style != before.Style || extendedStyle != before.ExtendedStyle)
            {
                if (style != before.Style) WriteStyle(window, processId, -16, style);
                if (extendedStyle != before.ExtendedStyle) WriteStyle(window, processId, -20, extendedStyle);
                VerifyIdentity(window, processId);
                if (!SetWindowPos(window, IntPtr.Zero, before.Origin.X, before.Origin.Y, width, height, 0x34))
                    throw FrameNativeFailure("frame_position"); // FRAMECHANGED | NOACTIVATE | NOZORDER
            }
            receipt.FrameBeforeStyle = before.Style;
            receipt.FrameBeforeExtendedStyle = before.ExtendedStyle;
            receipt.FrameBeforeWindowWidth = before.Outer.Right - before.Outer.Left;
            receipt.FrameBeforeWindowHeight = before.Outer.Bottom - before.Outer.Top;
            receipt.FrameBeforeClientX = before.Origin.X;
            receipt.FrameBeforeClientY = before.Origin.Y;
            receipt.FrameBeforeClientWidth = width;
            receipt.FrameBeforeClientHeight = height;
            receipt.FrameBeforeLeftInset = before.Origin.X - before.Visible.Left;
            receipt.FrameBeforeTopInset = before.Origin.Y - before.Visible.Top;
            receipt.FrameBeforeRightInset = before.Visible.Right - before.Origin.X - width;
            receipt.FrameBeforeBottomInset = before.Visible.Bottom - before.Origin.Y - height;
            VerifyFixedClientFrame(window, processId, receipt);
            receipt.FrameForegroundPreserved = foreground == GetForegroundWindow();
            if (!receipt.FrameForegroundPreserved) throw new NativeMaterialPreviewException("frame_foreground_changed", 0, Failure);
        }
        finally
        {
            if (SetThreadDpiAwarenessContext(previousContext) == IntPtr.Zero) throw FrameNativeFailure("frame_dpi_restore");
        }
    }

    private static void VerifyFixedClientFrame(IntPtr window, int processId, NativeMaterialPreviewReceipt receipt)
    {
        FrameFacts after = ReadFrame(window, processId);
        receipt.FrameAfterStyle = after.Style;
        receipt.FrameAfterExtendedStyle = after.ExtendedStyle;
        receipt.FrameAfterWindowWidth = after.Outer.Right - after.Outer.Left;
        receipt.FrameAfterWindowHeight = after.Outer.Bottom - after.Outer.Top;
        receipt.FrameAfterClientX = after.Origin.X;
        receipt.FrameAfterClientY = after.Origin.Y;
        receipt.FrameAfterClientWidth = after.Client.Right - after.Client.Left;
        receipt.FrameAfterClientHeight = after.Client.Bottom - after.Client.Top;
        receipt.FrameAfterLeftInset = after.Origin.X - after.Visible.Left;
        receipt.FrameAfterTopInset = after.Origin.Y - after.Visible.Top;
        receipt.FrameAfterRightInset = after.Visible.Right - after.Origin.X - receipt.FrameAfterClientWidth;
        receipt.FrameAfterBottomInset = after.Visible.Bottom - after.Origin.Y - receipt.FrameAfterClientHeight;
        receipt.FrameClientGeometryPreserved = receipt.FrameBeforeClientX == receipt.FrameAfterClientX &&
            receipt.FrameBeforeClientY == receipt.FrameAfterClientY && receipt.FrameBeforeClientWidth == receipt.FrameAfterClientWidth &&
            receipt.FrameBeforeClientHeight == receipt.FrameAfterClientHeight;
        receipt.FrameClientOnly = Math.Abs(receipt.FrameAfterLeftInset) <= 1 && Math.Abs(receipt.FrameAfterTopInset) <= 1 &&
            Math.Abs(receipt.FrameAfterRightInset) <= 1 && Math.Abs(receipt.FrameAfterBottomInset) <= 1;
        if (!receipt.FrameClientGeometryPreserved) throw new NativeMaterialPreviewException("frame_client_geometry_mismatch", 0, Failure);
        if (!receipt.FrameClientOnly) throw new NativeMaterialPreviewException("frame_visible_collar", 0, Failure);
        if (after.Style != (receipt.FrameBeforeStyle & ~0x00c00000L) ||
            after.ExtendedStyle != (receipt.FrameBeforeExtendedStyle & ~0x00000100L))
            throw new NativeMaterialPreviewException("frame_style_mismatch", 0, Failure);
    }

    private static OsVersionInfo ReadVersion()
    {
        OsVersionInfo version = new OsVersionInfo();
        int size = Marshal.SizeOf(typeof(OsVersionInfo));
        if (size != 276) throw new NativeMaterialPreviewException("version_structure", size, Failure);
        version.Size = (uint)size;
        int result = RtlGetVersion(ref version);
        if (result != 0) throw new NativeMaterialPreviewException("version_query", result, result);
        return version;
    }

    public static uint GetWindowsBuild()
    {
        return ReadVersion().Build;
    }

    private static bool Matches(IntPtr window, int processId)
    {
        uint owner;
        if (window == IntPtr.Zero || processId <= 0 || GetWindowThreadProcessId(window, out owner) == 0)
            return false;
        if (owner != (uint)processId) return false;
        StringBuilder title = new StringBuilder(128);
        GetWindowText(window, title, title.Capacity);
        return String.Equals(title.ToString(), PreviewTitle, StringComparison.Ordinal);
    }

    private static void VerifyIdentity(IntPtr window, int processId)
    {
        if (!Matches(window, processId))
            throw new NativeMaterialPreviewException("window_identity", 0, InvalidArgument);
    }

    // The caller supplies only the newly launched process, never an arbitrary title.
    public static long[] FindWindows(int processId)
    {
        if (processId <= 0) throw new NativeMaterialPreviewException("process_identity", 0, InvalidArgument);
        System.Collections.Generic.List<long> windows = new System.Collections.Generic.List<long>();
        bool enumerated = EnumWindows(delegate(IntPtr window, IntPtr parameter)
        {
            if (Matches(window, processId)) windows.Add(window.ToInt64());
            return true;
        }, IntPtr.Zero);
        if (!enumerated)
        {
            int code = Marshal.GetLastWin32Error();
            throw new NativeMaterialPreviewException("window_enumeration", code,
                code == 0 ? Failure : unchecked((int)(0x80070000u | (uint)code)));
        }
        return windows.ToArray();
    }

    public static long FindWindow(int processId)
    {
        if (processId <= 0) throw new NativeMaterialPreviewException("process_identity", 0, InvalidArgument);
        Stopwatch elapsed = Stopwatch.StartNew();
        Process process;
        try { process = Process.GetProcessById(processId); }
        catch (ArgumentException) { throw new NativeMaterialPreviewException("process_missing", 0, Failure); }
        using (process)
        {
            while (elapsed.ElapsedMilliseconds < 5000)
            {
                if (process.HasExited) throw new NativeMaterialPreviewException("process_exited", process.ExitCode, Failure);
                IntPtr selected = IntPtr.Zero;
                bool enumerated = EnumWindows(delegate(IntPtr window, IntPtr parameter)
                {
                    if (!Matches(window, processId)) return true;
                    selected = window;
                    return false;
                }, IntPtr.Zero);
                if (selected != IntPtr.Zero)
                {
                    VerifyIdentity(selected, processId);
                    return selected.ToInt64();
                }
                if (!enumerated)
                {
                    int code = Marshal.GetLastWin32Error();
                    throw new NativeMaterialPreviewException("window_enumeration", code,
                        code == 0 ? Failure : unchecked((int)(0x80070000u | (uint)code)));
                }
                int remaining = 5000 - (int)elapsed.ElapsedMilliseconds;
                if (remaining > 0) Thread.Sleep(Math.Min(20, remaining));
            }
        }
        throw new NativeMaterialPreviewException("window_timeout", 0, Failure);
    }

    private static int SetAttribute(IntPtr window, int processId, int attribute, int value, string stage)
    {
        VerifyIdentity(window, processId);
        int result = DwmSetWindowAttribute(window, attribute, ref value, sizeof(int));
        if (result != 0) throw new NativeMaterialPreviewException(stage, result, result);
        return result;
    }

    private static int ReadAttribute(IntPtr window, int processId, int attribute, string stage, out int value)
    {
        VerifyIdentity(window, processId);
        int result = DwmGetWindowAttribute(window, attribute, out value, sizeof(int));
        if (result != 0) throw new NativeMaterialPreviewException(stage, result, result);
        return result;
    }

    public static NativeMaterialPreviewReadback VerifyBackdrop(int processId, long windowHandle, int expectedBackdrop)
    {
        IntPtr window = new IntPtr(windowHandle);
        NativeMaterialPreviewReadback readback = new NativeMaterialPreviewReadback();
        readback.WindowHandle = windowHandle;
        readback.ProcessId = processId;
        int value;
        readback.ReadBackdropHResult = ReadAttribute(window, processId, 38, "backdrop_read", out value);
        readback.Backdrop = value;
        if (value != expectedBackdrop)
            throw new NativeMaterialPreviewException("backdrop_mismatch", readback.ReadBackdropHResult, Failure);
        readback.ReadCornerHResult = ReadAttribute(window, processId, 33, "corner_read", out value);
        readback.Corner = value;
        readback.ReadNcRenderingHResult = ReadAttribute(window, processId, 1, "nc_rendering_read", out value);
        readback.NcRenderingEnabled = value != 0;
        return readback;
    }

    public static bool TryParseThemeRequest(string line, Guid correlation, out bool dark)
    {
        string prefix = "ui.material_preview.theme_requested correlation=" + correlation.ToString("D") + " dark=";
        dark = false;
        if (String.Equals(line, prefix + "true", StringComparison.Ordinal)) { dark = true; return true; }
        return String.Equals(line, prefix + "false", StringComparison.Ordinal);
    }

    // Ignored lines do not touch the HWND. Accepted lines synchronously use the same native owner.
    public static bool TryApplyThemeRequest(string line, Guid correlation, int processId, long windowHandle,
        out NativeMaterialPreviewThemeReceipt receipt)
    {
        receipt = null;
        bool dark;
        if (!TryParseThemeRequest(line, correlation, out dark)) return false;
        Stopwatch elapsed = Stopwatch.StartNew();
        IntPtr window = new IntPtr(windowHandle);
        NativeMaterialPreviewThemeReceipt applied = new NativeMaterialPreviewThemeReceipt();
        applied.Correlation = correlation;
        applied.ProcessId = processId;
        applied.WindowHandle = windowHandle;
        applied.Dark = dark;
        applied.SetDarkHResult = SetAttribute(window, processId, 20, dark ? 1 : 0, "theme_dark_set");
        VerifyIdentity(window, processId);
        int value;
        applied.ReadDarkHResult = DwmGetWindowAttribute(window, 20, out value, sizeof(int));
        // Some DWM versions accept Set20 but do not expose Get20 (E_INVALIDARG).
        if (applied.ReadDarkHResult == 0)
        {
            applied.DarkReadbackSupported = true;
            if (value != (dark ? 1 : 0))
                throw new NativeMaterialPreviewException("theme_dark_mismatch", 0, Failure);
        }
        else if (applied.ReadDarkHResult != InvalidArgument)
            throw new NativeMaterialPreviewException("theme_dark_read", applied.ReadDarkHResult, applied.ReadDarkHResult);
        applied.ElapsedMilliseconds = elapsed.ElapsedMilliseconds;
        receipt = applied;
        return true;
    }

    private static void ApplyAccent(IntPtr window, int processId, NativeMaterialPreviewReceipt receipt)
    {
        int policySize = Marshal.SizeOf(typeof(AccentPolicy));
        int dataSize = Marshal.SizeOf(typeof(WindowCompositionAttribData));
        if (policySize != 16 || dataSize != (IntPtr.Size == 8 ? 24 : 12))
            throw new NativeMaterialPreviewException("accent_structure", dataSize, Failure);
        // Released GPUI Blurred policy: state4/flags0/ABGR01000000/animation0.
        // Solid uses the same owner's disabled policy, not another backdrop mechanism.
        AccentPolicy policy = new AccentPolicy();
        policy.AccentState = receipt.RequestedAccentState;
        policy.AccentFlags = receipt.RequestedAccentState == 4 ? 0 : 2;
        policy.GradientColor = receipt.RequestedAccentState == 4 ? 0x01000000 : 0;
        policy.AnimationId = 0;
        IntPtr allocation = Marshal.AllocHGlobal(policySize);
        try
        {
            Marshal.StructureToPtr(policy, allocation, false);
            WindowCompositionAttribData data = new WindowCompositionAttribData();
            data.Attribute = 19; // WCA_ACCENT_POLICY
            data.Data = allocation;
            data.Size = new UIntPtr((uint)policySize);
            VerifyIdentity(window, processId);
            ClearLastError(0);
            receipt.AccentAccepted = SetWindowCompositionAttribute(window, ref data);
            int nativeError = Marshal.GetLastWin32Error();
            receipt.AccentLastError = receipt.AccentAccepted ? 0 : nativeError;
            if (!receipt.AccentAccepted)
                throw new NativeMaterialPreviewException("accent_set", nativeError,
                    nativeError == 0 ? Failure : unchecked((int)(0x80070000u | ((uint)nativeError & 0xffffu))));
        }
        finally { Marshal.FreeHGlobal(allocation); }
    }

    public static NativeMaterialPreviewReceipt Prepare(int processId, bool acrylic, bool light, Guid correlation)
    {
        return PrepareCore(processId, 0, acrylic, light, correlation);
    }

    public static NativeMaterialPreviewReceipt Prepare(int processId, long hwnd, bool acrylic, bool light, Guid correlation)
    {
        VerifyIdentity(new IntPtr(hwnd), processId);
        return PrepareCore(processId, hwnd, acrylic, light, correlation);
    }

    private static NativeMaterialPreviewReceipt PrepareCore(int processId, long hwnd, bool acrylic, bool light, Guid correlation)
    {
        Stopwatch elapsed = Stopwatch.StartNew();
        OsVersionInfo version = ReadVersion();
        if (version.Major < 10 || version.Build < 22621)
            throw new NativeMaterialPreviewException("material_unsupported_build", 0, unchecked((int)0x80070032));
        IntPtr window = new IntPtr(hwnd == 0 ? FindWindow(processId) : hwnd);
        VerifyIdentity(window, processId);
        NativeMaterialPreviewReceipt receipt = new NativeMaterialPreviewReceipt();
        receipt.Correlation = correlation;
        receipt.WindowHandle = window.ToInt64();
        receipt.ProcessId = processId;
        receipt.OsMajor = version.Major;
        receipt.OsMinor = version.Minor;
        receipt.OsBuild = version.Build;
        receipt.RequestedBackdrop = 1; // DWMSBT_NONE for both modes; Accent is the sole blur mechanism.
        receipt.Mechanism = "wca_accent_policy";
        receipt.RequestedAccentState = acrylic ? 4 : 0;
        receipt.Dark = !light;
        receipt.VersionNativeResult = 0;
        NormalizeFixedClientFrame(window, processId, receipt);
        // Independently proven frame policy; it does not establish visible Acrylic.
        receipt.SetNcRenderingHResult = SetAttribute(window, processId, 2, 1, "nc_rendering_set");
        Margins margins = new Margins();
        margins.Left = margins.Right = margins.Top = margins.Bottom = -1;
        VerifyIdentity(window, processId);
        receipt.ExtendFrameHResult = DwmExtendFrameIntoClientArea(window, ref margins);
        if (receipt.ExtendFrameHResult != 0)
            throw new NativeMaterialPreviewException("frame_extend", receipt.ExtendFrameHResult, receipt.ExtendFrameHResult);
        receipt.SetDarkHResult = SetAttribute(window, processId, 20, light ? 0 : 1, "dark_set");
        receipt.SetCornerHResult = SetAttribute(window, processId, 33, 2, "corner_set");
        receipt.SetBackdropHResult = SetAttribute(window, processId, 38, receipt.RequestedBackdrop, "backdrop_set");
        ApplyAccent(window, processId, receipt);
        NativeMaterialPreviewReadback readback = VerifyBackdrop(processId, receipt.WindowHandle, receipt.RequestedBackdrop);
        receipt.Backdrop = readback.Backdrop;
        receipt.Corner = readback.Corner;
        receipt.NcRenderingEnabled = readback.NcRenderingEnabled;
        receipt.ReadBackdropHResult = readback.ReadBackdropHResult;
        receipt.ReadCornerHResult = readback.ReadCornerHResult;
        receipt.ReadNcRenderingHResult = readback.ReadNcRenderingHResult;
        if (receipt.Corner != 2) throw new NativeMaterialPreviewException("corner_mismatch", receipt.ReadCornerHResult, Failure);
        if (receipt.NcRenderingEnabled) throw new NativeMaterialPreviewException("nc_rendering_mismatch", receipt.ReadNcRenderingHResult, Failure);
        IntPtr frameContext = SetThreadDpiAwarenessContext(new IntPtr(-4));
        if (frameContext == IntPtr.Zero) throw FrameNativeFailure("frame_dpi_enter");
        try { VerifyFixedClientFrame(window, processId, receipt); }
        finally
        {
            if (SetThreadDpiAwarenessContext(frameContext) == IntPtr.Zero) throw FrameNativeFailure("frame_dpi_restore");
        }
        receipt.ElapsedMilliseconds = elapsed.ElapsedMilliseconds;
        return receipt;
    }
}

// One instance owns one requested standalone executable and its asynchronous output lifetime.
public sealed class NativeMaterialPreviewSession : IDisposable
{
    private readonly object gate = new object();
    private readonly Process process;
    private readonly Guid correlation;
    private readonly StreamWriter outputLog;
    private readonly StreamWriter errorLog;
    private readonly Stopwatch elapsed = Stopwatch.StartNew();
    private bool started;
    private bool disposed;
    private bool? pendingDark;
    private NativeMaterialPreviewReceipt receipt;
    private NativeMaterialPreviewThemeReceipt lastThemeReceipt;
    private NativeMaterialPreviewException failure;
    private int processId;
    private int? exitCode;
    private int themeApplyCount;

    public NativeMaterialPreviewReceipt Receipt { get { lock (gate) { return receipt; } } }
    public NativeMaterialPreviewThemeReceipt LastThemeReceipt { get { lock (gate) { return lastThemeReceipt; } } }
    public NativeMaterialPreviewException Failure { get { lock (gate) { return failure; } } }
    public int ProcessId { get { lock (gate) { return processId; } } }
    public int? ExitCode { get { lock (gate) { return exitCode; } } }
    public int ThemeApplyCount { get { lock (gate) { return themeApplyCount; } } }

    // arguments contain only launcher-validated flags, enum tokens and the correlation GUID.
    public NativeMaterialPreviewSession(string executable, string arguments, string workingDirectory,
        Guid correlation, string stdoutLog, string stderrLog)
    {
        this.correlation = correlation;
        outputLog = new StreamWriter(stdoutLog, false, new UTF8Encoding(false));
        try { errorLog = new StreamWriter(stderrLog, false, new UTF8Encoding(false)); }
        catch { outputLog.Dispose(); throw; }
        outputLog.AutoFlush = true;
        errorLog.AutoFlush = true;
        process = new Process();
        process.StartInfo = new ProcessStartInfo(executable, arguments);
        process.StartInfo.WorkingDirectory = workingDirectory;
        process.StartInfo.UseShellExecute = false;
        process.StartInfo.RedirectStandardOutput = true;
        process.StartInfo.RedirectStandardError = true;
        process.OutputDataReceived += OnOutput;
        process.ErrorDataReceived += OnError;
    }

    // Called under gate; no raw child lines, paths or exception messages enter diagnostics.
    private void Log(StreamWriter writer, string stage, string scalars)
    {
        try
        {
            writer.WriteLine("ui.native_material.session." + stage + " correlation=" + correlation.ToString("D") +
                " pid=" + processId.ToString(CultureInfo.InvariantCulture) +
                " elapsed_ms=" + elapsed.ElapsedMilliseconds.ToString(CultureInfo.InvariantCulture) + " " + scalars);
        }
        catch (IOException) { } // Diagnostics do not change the native result.
        catch (ObjectDisposedException) { }
    }

    private void RetainFailure(Exception exception, string stage)
    {
        NativeMaterialPreviewException native = exception as NativeMaterialPreviewException;
        if (native == null) native = new NativeMaterialPreviewException(stage, 0, exception.HResult);
        if (failure == null) failure = native;
        Log(errorLog, "failed", "stage=" + native.Stage + " hresult=" + native.HResult.ToString(CultureInfo.InvariantCulture) +
            " native_result=" + native.NativeResult.ToString(CultureInfo.InvariantCulture));
    }

    private void ApplyTheme(string line)
    {
        NativeMaterialPreviewThemeReceipt applied;
        if (!NativeMaterialPreviewWindow.TryApplyThemeRequest(line, correlation, processId, receipt.WindowHandle, out applied)) return;
        lastThemeReceipt = applied;
        themeApplyCount++;
        Log(outputLog, "theme_applied", "dark=" + (applied.Dark ? "true" : "false") +
            " hresult=" + applied.SetDarkHResult.ToString(CultureInfo.InvariantCulture) +
            " read_hresult=" + applied.ReadDarkHResult.ToString(CultureInfo.InvariantCulture) +
            " readback_supported=" + (applied.DarkReadbackSupported ? "true" : "false"));
    }

    private void OnOutput(object sender, DataReceivedEventArgs args)
    {
        lock (gate)
        {
            try
            {
                if (disposed || args.Data == null) return;
                bool dark;
                if (!NativeMaterialPreviewWindow.TryParseThemeRequest(args.Data, correlation, out dark))
                {
                    Log(outputLog, "stdout_received", "characters=" + args.Data.Length.ToString(CultureInfo.InvariantCulture));
                    return;
                }
                Log(outputLog, "theme_requested", "dark=" + (dark ? "true" : "false"));
                if (failure != null) return;
                if (receipt == null) { pendingDark = dark; return; }
                ApplyTheme(args.Data);
            }
            catch (Exception exception) { RetainFailure(exception, "stdout_callback"); }
        }
    }

    private void OnError(object sender, DataReceivedEventArgs args)
    {
        lock (gate)
        {
            try
            {
                if (!disposed && args.Data != null)
                    Log(errorLog, "stderr_received", "characters=" + args.Data.Length.ToString(CultureInfo.InvariantCulture));
            }
            catch (Exception exception) { RetainFailure(exception, "stderr_callback"); }
        }
    }

    public void Start(bool acrylic, bool light)
    {
        lock (gate)
        {
            if (started || disposed) throw new InvalidOperationException("Preview session cannot be started twice.");
            try
            {
                if (!process.Start()) throw new NativeMaterialPreviewException("process_start", 0, unchecked((int)0x80004005));
                started = true;
                processId = process.Id;
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
                Log(outputLog, "started", "");
            }
            catch (Exception exception) { RetainFailure(exception, "process_start"); throw; }
        }
        // Leave callbacks free to retain the latest theme while the HWND is being prepared.
        try
        {
            NativeMaterialPreviewReceipt initial = NativeMaterialPreviewWindow.Prepare(processId, acrylic, light, correlation);
            lock (gate)
            {
                receipt = initial;
                Log(outputLog, "prepared", "hwnd=" + receipt.WindowHandle.ToString(CultureInfo.InvariantCulture) +
                    " backdrop=" + receipt.Backdrop.ToString(CultureInfo.InvariantCulture) +
                    " corner=" + receipt.Corner.ToString(CultureInfo.InvariantCulture) +
                    " mechanism=" + receipt.Mechanism +
                    " accent_state=" + receipt.RequestedAccentState.ToString(CultureInfo.InvariantCulture) +
                    " accent_accepted=" + (receipt.AccentAccepted ? "true" : "false") +
                    " accent_error=" + receipt.AccentLastError.ToString(CultureInfo.InvariantCulture) +
                    " dark=" + (receipt.Dark ? "true" : "false"));
                if (pendingDark.HasValue && failure == null)
                {
                    string line = "ui.material_preview.theme_requested correlation=" + correlation.ToString("D") +
                        " dark=" + (pendingDark.Value ? "true" : "false");
                    pendingDark = null;
                    ApplyTheme(line);
                }
                if (failure != null) throw failure;
            }
        }
        catch (Exception exception)
        {
            lock (gate) { RetainFailure(exception, "native_prepare"); }
            throw;
        }
    }

    // The parameterless Process wait drains both asynchronous readers after native process exit.
    // Native failures remain available separately; this return value is the actual child exit code.
    public int WaitForExit()
    {
        lock (gate)
        {
            if (exitCode.HasValue) return exitCode.Value;
            if (!started || disposed) throw new InvalidOperationException("Preview session is not running.");
        }
        process.WaitForExit();
        lock (gate)
        {
            exitCode = process.ExitCode;
            Log(outputLog, "exited", "exit_code=" + exitCode.Value.ToString(CultureInfo.InvariantCulture) +
                " theme_apply_count=" + themeApplyCount.ToString(CultureInfo.InvariantCulture) +
                " native_failed=" + (failure != null ? "true" : "false"));
            return exitCode.Value;
        }
    }

    // Never kills a process. A live owned preview must close before Dispose can finish draining.
    public void Dispose()
    {
        lock (gate) { if (disposed) return; }
        if (started) WaitForExit();
        lock (gate)
        {
            if (disposed) return;
            disposed = true;
            process.OutputDataReceived -= OnOutput;
            process.ErrorDataReceived -= OnError;
            process.Dispose();
            Log(outputLog, "disposed", "");
            outputLog.Dispose();
            errorLog.Dispose();
        }
    }
}
