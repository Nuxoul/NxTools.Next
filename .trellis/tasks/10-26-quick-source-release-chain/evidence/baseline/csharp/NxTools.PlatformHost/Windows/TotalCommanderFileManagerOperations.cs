using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal interface ITotalCommanderWindowOperations
{
    bool IsMainWindow(long windowHandle);

    bool TryGetSourceDirectory(
        long windowHandle,
        out string? currentDirectory);

    bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        out string? error);

    bool TryActivate(long windowHandle, out string? error);
}

internal sealed class TotalCommanderFileManagerOperations :
    IThirdPartyFileManagerOperations
{
    private readonly ITotalCommanderWindowOperations _windows;

    public TotalCommanderFileManagerOperations()
        : this(new Win32TotalCommanderWindowOperations())
    {
    }

    internal TotalCommanderFileManagerOperations(
        ITotalCommanderWindowOperations windows)
    {
        _windows = windows;
    }

    public string Id => "Total Commander";

    public bool TryCapture(
        ForegroundContext foreground,
        out ThirdPartyFileManagerCapture? capture,
        out WindowsCommandResult? failure)
    {
        capture = null;
        failure = null;
        if (!foreground.ProcessName.StartsWith(
                "totalcmd",
                StringComparison.OrdinalIgnoreCase) ||
            !_windows.IsMainWindow(foreground.WindowHandle) ||
            !_windows.TryGetSourceDirectory(
                foreground.WindowHandle,
                out var capturedDirectory) ||
            !ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                capturedDirectory))
        {
            return false;
        }
        capture = new ThirdPartyFileManagerCapture(
            capturedDirectory!,
            new TotalCommanderPaneState(capturedDirectory!));
        return true;
    }

    public bool CanNavigate(long windowHandle, object paneState) =>
        paneState is TotalCommanderPaneState totalCommanderPane &&
        _windows.IsMainWindow(windowHandle) &&
        _windows.TryGetSourceDirectory(
            windowHandle,
            out var currentDirectory) &&
        ThirdPartyFileManagerPath.SameDirectory(
            currentDirectory,
            totalCommanderPane.CapturedSourceDirectory);

    public WindowsCommandResult Navigate(
        long windowHandle,
        string targetDirectory,
        object paneState)
    {
        if (paneState is not TotalCommanderPaneState)
        {
            return new WindowsCommandResult(
                false,
                "The captured Total Commander source pane context is invalid.");
        }
        return _windows.TryNavigate(
                windowHandle,
                targetDirectory,
                out var error)
            ? new WindowsCommandResult(true)
            : new WindowsCommandResult(
                false,
                error ??
                "The Total Commander window could not be navigated.");
    }

    public bool TryActivate(
        long windowHandle,
        object paneState,
        out string? error) =>
        paneState is TotalCommanderPaneState
            ? _windows.TryActivate(windowHandle, out error)
            : FailInvalidPane(out error);

    private static bool FailInvalidPane(out string? error)
    {
        error = "The captured Total Commander source pane context is invalid.";
        return false;
    }
}

internal sealed record TotalCommanderPaneState(
    string CapturedSourceDirectory);

internal sealed class Win32TotalCommanderWindowOperations :
    ITotalCommanderWindowOperations
{
    private const uint WmCopyData = 0x004A;
    private const uint SmtoAbortIfHung = 0x0002;
    private const uint MsgFilterAllow = 1;
    private const uint QueryTimeoutMs = 120;
    private const uint CommandTimeoutMs = 1_000;
    private const int GetRequestWide = 'G' + 256 * 'W';
    private const int GetReplyWide = 'R' + 256 * 'W';
    private const int ChangeDirectory = 'C' + 256 * 'D';
    private const string QueryWindowClass =
        "NxToolsTotalCommanderQueryWindowV1";
    private static readonly IntPtr MessageOnlyWindow = new(-3);
    private static readonly object WindowClassGate = new();
    private static WindowProcedure? _windowProcedure;
    private static bool _windowClassReady;

    [ThreadStatic]
    private static string? _capturedPath;

    public bool IsMainWindow(long windowHandle)
    {
        var hwnd = new IntPtr(windowHandle);
        return hwnd != IntPtr.Zero &&
            IsWindow(hwnd) &&
            GetWindowClass(hwnd).Equals(
                "TTOTAL_CMD",
                StringComparison.OrdinalIgnoreCase) &&
            WindowProcessNameStartsWith(hwnd, "totalcmd");
    }

    public bool TryGetSourceDirectory(
        long windowHandle,
        out string? currentDirectory)
    {
        currentDirectory = null;
        var totalCommander = new IntPtr(windowHandle);
        if (!IsMainWindow(windowHandle) || !EnsureQueryWindowClass())
        {
            return false;
        }
        var receiver = CreateWindowEx(
            0,
            QueryWindowClass,
            null,
            0,
            0,
            0,
            0,
            0,
            MessageOnlyWindow,
            IntPtr.Zero,
            GetModuleHandle(null),
            IntPtr.Zero);
        if (receiver == IntPtr.Zero)
        {
            return false;
        }
        try
        {
            _ = ChangeWindowMessageFilterEx(
                receiver,
                WmCopyData,
                MsgFilterAllow,
                IntPtr.Zero);
            _capturedPath = null;
            var command = Encoding.ASCII.GetBytes("SP\0");
            var pinned = GCHandle.Alloc(command, GCHandleType.Pinned);
            try
            {
                var data = new CopyDataStruct
                {
                    DataId = new IntPtr(GetRequestWide),
                    ByteCount = command.Length,
                    Data = pinned.AddrOfPinnedObject()
                };
                var sent = SendMessageTimeout(
                    totalCommander,
                    WmCopyData,
                    receiver,
                    ref data,
                    SmtoAbortIfHung,
                    QueryTimeoutMs,
                    out _);
                if (sent == IntPtr.Zero)
                {
                    return false;
                }
            }
            finally
            {
                pinned.Free();
            }
            if (!ThirdPartyFileManagerPath.LooksLikeRootedDirectory(
                    _capturedPath))
            {
                return false;
            }
            currentDirectory = NormalizeDirectory(_capturedPath!);
            return true;
        }
        finally
        {
            _ = DestroyWindow(receiver);
        }
    }

    public bool TryNavigate(
        long windowHandle,
        string targetDirectory,
        out string? error)
    {
        if (!IsMainWindow(windowHandle))
        {
            error = "The captured Total Commander window is no longer available.";
            return false;
        }
        if (!TryActivate(windowHandle, out error))
        {
            return false;
        }

        Thread.Sleep(40);
        var bytes = BuildChangeDirectoryPayload(targetDirectory);
        var pinned = GCHandle.Alloc(bytes, GCHandleType.Pinned);
        try
        {
            var data = new CopyDataStruct
            {
                DataId = new IntPtr(ChangeDirectory),
                ByteCount = bytes.Length,
                Data = pinned.AddrOfPinnedObject()
            };
            var sent = SendMessageTimeout(
                new IntPtr(windowHandle),
                WmCopyData,
                IntPtr.Zero,
                ref data,
                SmtoAbortIfHung,
                CommandTimeoutMs,
                out _);
            if (sent == IntPtr.Zero)
            {
                error = "Total Commander did not accept the directory command before timeout.";
                return false;
            }
        }
        finally
        {
            pinned.Free();
        }

        var deadline = Stopwatch.StartNew();
        while (deadline.ElapsedMilliseconds < 1_000)
        {
            if (TryGetSourceDirectory(
                    windowHandle,
                    out var currentDirectory) &&
                ThirdPartyFileManagerPath.SameDirectory(
                    currentDirectory,
                    targetDirectory))
            {
                error = null;
                return true;
            }
            Thread.Sleep(25);
        }
        error = "Total Commander did not switch the captured source panel to the requested directory.";
        return false;
    }

    public bool TryActivate(long windowHandle, out string? error)
    {
        if (WindowsPlatformAdapter.TryActivateNativeWindow(windowHandle))
        {
            error = null;
            return true;
        }
        error = "Windows did not restore the captured Total Commander window.";
        return false;
    }

    internal static byte[] BuildChangeDirectoryPayload(string targetDirectory)
    {
        var payload = new List<byte>(targetDirectory.Length + 8)
        {
            0xEF,
            0xBB,
            0xBF
        };
        payload.AddRange(Encoding.UTF8.GetBytes(targetDirectory));
        payload.Add(0x0D);
        payload.Add(0x00);
        payload.Add((byte)'S');
        payload.Add(0x00);
        return payload.ToArray();
    }

    private static string NormalizeDirectory(string path) =>
        path.Length > 3
            ? path.TrimEnd(Path.DirectorySeparatorChar)
            : path;

    private static bool EnsureQueryWindowClass()
    {
        lock (WindowClassGate)
        {
            if (_windowClassReady)
            {
                return true;
            }
            _windowProcedure = QueryWindowProcedure;
            var windowClass = new WindowClass
            {
                WindowProcedure = _windowProcedure,
                Instance = GetModuleHandle(null),
                ClassName = QueryWindowClass
            };
            var atom = RegisterClass(ref windowClass);
            _windowClassReady = atom != 0 || Marshal.GetLastWin32Error() == 1410;
            return _windowClassReady;
        }
    }

    private static IntPtr QueryWindowProcedure(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam)
    {
        if (message == WmCopyData && lParam != IntPtr.Zero)
        {
            var data = Marshal.PtrToStructure<CopyDataStruct>(lParam);
            if (data.DataId == new IntPtr(GetReplyWide) &&
                data.Data != IntPtr.Zero &&
                data.ByteCount > 1)
            {
                _capturedPath = Marshal.PtrToStringUni(
                    data.Data,
                    data.ByteCount / 2)?.TrimEnd('\0');
                return new IntPtr(1);
            }
        }
        return DefWindowProc(window, message, wParam, lParam);
    }

    private static bool WindowProcessNameStartsWith(
        IntPtr window,
        string expectedPrefix)
    {
        _ = GetWindowThreadProcessId(window, out var processId);
        if (processId == 0)
        {
            return false;
        }
        try
        {
            using var process = Process.GetProcessById((int)processId);
            return process.ProcessName.StartsWith(
                expectedPrefix,
                StringComparison.OrdinalIgnoreCase);
        }
        catch (ArgumentException)
        {
            return false;
        }
    }

    private static string GetWindowClass(IntPtr window)
    {
        var value = new StringBuilder(256);
        return GetClassName(window, value, value.Capacity) > 0
            ? value.ToString()
            : string.Empty;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct CopyDataStruct
    {
        public IntPtr DataId;
        public int ByteCount;
        public IntPtr Data;
    }

    private delegate IntPtr WindowProcedure(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WindowClass
    {
        public uint Style;
        public WindowProcedure WindowProcedure;
        public int ClassExtraBytes;
        public int WindowExtraBytes;
        public IntPtr Instance;
        public IntPtr Icon;
        public IntPtr Cursor;
        public IntPtr Background;
        [MarshalAs(UnmanagedType.LPWStr)]
        public string? MenuName;
        [MarshalAs(UnmanagedType.LPWStr)]
        public string ClassName;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern ushort RegisterClass(ref WindowClass windowClass);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr CreateWindowEx(
        uint extendedStyle,
        string className,
        string? windowName,
        uint style,
        int x,
        int y,
        int width,
        int height,
        IntPtr parent,
        IntPtr menu,
        IntPtr instance,
        IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern IntPtr DefWindowProc(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyWindow(IntPtr window);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindow(IntPtr window);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr window,
        StringBuilder value,
        int maximum);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(
        IntPtr window,
        out uint processId);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr GetModuleHandle(string? moduleName);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ChangeWindowMessageFilterEx(
        IntPtr window,
        uint message,
        uint action,
        IntPtr changeInfo);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(
        IntPtr window,
        uint message,
        IntPtr wParam,
        ref CopyDataStruct lParam,
        uint flags,
        uint timeout,
        out IntPtr result);
}
