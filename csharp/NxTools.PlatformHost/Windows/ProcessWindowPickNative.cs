using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows;

internal sealed record ProcessPickWindowFacts(bool Exists, bool Visible, bool Cloaked, uint ProcessId,
    long Root, bool Desktop, int Left, int Top, uint Width, uint Height);

internal sealed class ProcessWindowPickNative(ProcessPickApi? operations = null) : IProcessWindowPickNative
{
    private readonly ProcessPickApi _api = operations ?? new ProcessPickApi();
    private readonly Stopwatch _clock = Stopwatch.StartNew();
    private ProcessPickApi.HookProc? _mouseCallback, _keyboardCallback;
    private IntPtr _mouseHook, _keyboardHook;
    private Func<ProcessPickInput, long, bool>? _input;
    private long _source, _overlay;
    public long ElapsedMilliseconds => _clock.ElapsedMilliseconds;

    public bool ValidateOwners(long source, long overlay, uint peer)
    {
        var owner = _api.Facts(source); var surface = _api.Facts(overlay);
        return source != 0 && overlay != 0 && source != overlay && peer != 0 && owner.Exists && owner.Visible &&
            !owner.Cloaked && owner.Root == source && owner.ProcessId == peer && surface.Exists &&
            surface.Root == overlay && surface.ProcessId == owner.ProcessId;
    }

    public ProcessPickViewport GetViewport() => _api.Viewport();
    public void PrepareOverlay(long overlay, ProcessPickViewport viewport)
    {
        if (_api.Facts(overlay).Visible) throw new Win32Exception(87);
        _api.Hide(overlay);
        _api.SetStyle(overlay, -16, (_api.GetStyle(overlay, -16) & ~0x00CF0000L & ~0x40000000L) | 0x80000000L);
        _api.SetStyle(overlay, -20, (_api.GetStyle(overlay, -20) & ~0x60301L) | 0x080800A0L);
        _api.InitializeLayeredOverlay(overlay);
        _api.Place(overlay, viewport, false);
        var final = _api.Facts(overlay);
        if (final.Visible || final.Left != viewport.Left || final.Top != viewport.Top || final.Width != viewport.Width || final.Height != viewport.Height ||
            _api.ClientViewport(overlay) != viewport || (_api.GetStyle(overlay, -20) & 0x080800A0L) != 0x080800A0L)
            throw new Win32Exception(13);
    }
    public void ShowOverlay(long overlay) => _api.Show(overlay);
    public void HideOverlay(long overlay) => _api.Hide(overlay);
    public bool IsPressed(ProcessPickInput input) => _api.Pressed(input == ProcessPickInput.LeftDown ? 1 : 27);

    public ProcessPickWindow? HitTest(long source, long overlay) => ReadWindow(_api.RootAtPointer(), source, overlay);
    public ProcessPickWindow? ReadWindow(long handle, long source, long overlay)
    {
        if (handle == 0 || handle == source || handle == overlay) return null;
        var facts = _api.Facts(handle);
        if (!facts.Exists || !facts.Visible || facts.Cloaked || facts.Desktop || facts.ProcessId == 0 || facts.Root != handle || facts.Width == 0 || facts.Height == 0)
            return null;
        return new(handle, facts.ProcessId, facts.Left, facts.Top, facts.Width, facts.Height);
    }
    public string? ResolveProcess(uint pid, out int error) => _api.ProcessName(pid, out error);

    public void InstallHooks(Func<ProcessPickInput, long, bool> input, long source, long overlay)
    {
        _input = input; _source = source; _overlay = overlay;
        _mouseCallback = MouseHook; _keyboardCallback = KeyboardHook;
        _mouseHook = _api.Install(14, _mouseCallback);
        if (_mouseHook == IntPtr.Zero) throw new Win32Exception(_api.LastError);
        _keyboardHook = _api.Install(13, _keyboardCallback);
        if (_keyboardHook == IntPtr.Zero) throw new Win32Exception(_api.LastError);
    }

    private IntPtr MouseHook(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0 && message.ToInt64() is 0x201 or 0x202)
        {
            var mouse = Marshal.PtrToStructure<ProcessPickApi.MouseData>(data);
            // Only the native root/eligibility lookup and a fixed-size input decision run here.
            // ResolveProcess and all event/log work happen on a later timer turn.
            if ((mouse.Flags & 1) == 0 && HandleMouse(message.ToInt32() == 0x201, mouse.X, mouse.Y)) return new IntPtr(1);
        }
        return _api.Next(code, message, data);
    }
    internal bool HandleMouse(bool down, int x, int y)
    {
        var root = down ? _api.RootAt(x, y) : 0;
        if (down && ReadWindow(root, _source, _overlay) is null) root = 0;
        return _input?.Invoke(down ? ProcessPickInput.LeftDown : ProcessPickInput.LeftUp, root) == true;
    }
    private IntPtr KeyboardHook(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0 && message.ToInt64() is 0x100 or 0x101 or 0x104 or 0x105)
        {
            var keyboard = Marshal.PtrToStructure<ProcessPickApi.KeyboardData>(data);
            if (keyboard.Key == 27 && (keyboard.Flags & 0x10) == 0 && HandleEscape(message.ToInt64() is 0x100 or 0x104)) return new IntPtr(1);
        }
        return _api.Next(code, message, data);
    }
    internal bool HandleEscape(bool down) => _input?.Invoke(down ? ProcessPickInput.EscapeDown : ProcessPickInput.EscapeUp, 0) == true;
    public void RemoveHooks()
    {
        if (_keyboardHook != IntPtr.Zero) { _api.Unhook(_keyboardHook); _keyboardHook = IntPtr.Zero; }
        if (_mouseHook != IntPtr.Zero) { _api.Unhook(_mouseHook); _mouseHook = IntPtr.Zero; }
        _input = null;
        _mouseCallback = _keyboardCallback = null;
    }
}

// Virtual OS calls are the native-boundary fixture seam; product uses only these Win32 implementations.
internal class ProcessPickApi
{
    internal virtual int LastError => Marshal.GetLastWin32Error();
    internal virtual ProcessPickWindowFacts Facts(long handle)
    {
        var hwnd = new IntPtr(handle);
        var exists = IsWindow(hwnd);
        GetWindowThreadProcessId(hwnd, out var pid);
        var root = GetAncestor(hwnd, 2).ToInt64();
        var cloaked = 0;
        _ = DwmGetWindowAttribute(hwnd, 14, out cloaked, sizeof(int));
        var name = new StringBuilder(64); _ = GetClassNameW(hwnd, name, name.Capacity);
        var desktop = hwnd == GetDesktopWindow() || hwnd == GetShellWindow() || name.ToString() is "Progman" or "WorkerW";
        var validRect = GetWindowRect(hwnd, out var rect) && rect.Right > rect.Left && rect.Bottom > rect.Top;
        return new(exists, IsWindowVisible(hwnd), cloaked != 0, pid, root, desktop, rect.Left, rect.Top,
            validRect ? checked((uint)(rect.Right - (long)rect.Left)) : 0, validRect ? checked((uint)(rect.Bottom - (long)rect.Top)) : 0);
    }
    internal virtual ProcessPickViewport Viewport()
    {
        var width = GetSystemMetrics(78); var height = GetSystemMetrics(79);
        if (width <= 0 || height <= 0) throw new Win32Exception(13);
        return new(GetSystemMetrics(76), GetSystemMetrics(77), (uint)width, (uint)height);
    }
    internal virtual ProcessPickViewport ClientViewport(long handle)
    {
        var hwnd = new IntPtr(handle); var origin = new Point();
        if (!GetClientRect(hwnd, out var rect) || !ClientToScreen(hwnd, ref origin)) throw new Win32Exception(Marshal.GetLastWin32Error());
        return new(origin.X, origin.Y, checked((uint)(rect.Right - rect.Left)), checked((uint)(rect.Bottom - rect.Top)));
    }
    internal virtual long GetStyle(long hwnd, int index) => GetWindowLongPtrW(new IntPtr(hwnd), index).ToInt64();
    internal virtual void SetStyle(long hwnd, int index, long value)
    {
        Marshal.SetLastPInvokeError(0);
        if (SetWindowLongPtrW(new IntPtr(hwnd), index, new IntPtr(value)) == IntPtr.Zero && Marshal.GetLastWin32Error() != 0)
            throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    internal virtual void InitializeLayeredOverlay(long hwnd)
    {
        if (!SetLayeredWindowAttributes(new IntPtr(hwnd), 0, 255, 2)) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    internal virtual void Place(long hwnd, ProcessPickViewport viewport, bool visible)
    {
        if (!SetWindowPos(new IntPtr(hwnd), new IntPtr(-1), viewport.Left, viewport.Top, checked((int)viewport.Width), checked((int)viewport.Height),
            0x10 | 0x20 | 0x200 | (visible ? 0x40u : 0u))) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    internal virtual void Show(long hwnd)
    {
        if (!SetWindowPos(new IntPtr(hwnd), new IntPtr(-1), 0, 0, 0, 0, 0x1 | 0x2 | 0x10 | 0x40 | 0x200)) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    internal virtual void Hide(long hwnd)
    {
        if (!IsWindow(new IntPtr(hwnd))) return;
        if (!SetWindowPos(new IntPtr(hwnd), IntPtr.Zero, 0, 0, 0, 0, 0x1 | 0x2 | 0x4 | 0x10 | 0x80 | 0x200)) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    internal virtual bool Pressed(int key) => (GetAsyncKeyState(key) & 0x8000) != 0;
    internal virtual long RootAtPointer() => GetCursorPos(out var point) ? RootAt(point.X, point.Y) : 0;
    internal virtual long RootAt(int x, int y) => GetAncestor(WindowFromPoint(new Point { X = x, Y = y }), 2).ToInt64();
    internal virtual string? ProcessName(uint pid, out int error)
    {
        using var process = OpenProcess(0x1000, false, pid);
        if (process.IsInvalid) { error = Marshal.GetLastWin32Error(); return null; }
        var image = new StringBuilder(32768); var length = image.Capacity;
        if (!QueryFullProcessImageNameW(process, 0, image, ref length)) { error = Marshal.GetLastWin32Error(); return null; }
        error = 0;
        return Path.GetFileNameWithoutExtension(image.ToString()) + ".exe";
    }
    internal virtual IntPtr Install(int kind, HookProc callback) => SetWindowsHookExW(kind, callback, GetModuleHandleW(null), 0);
    internal virtual void Unhook(IntPtr hook) => _ = UnhookWindowsHookEx(hook);
    internal virtual IntPtr Next(int code, IntPtr message, IntPtr data) => CallNextHookEx(IntPtr.Zero, code, message, data);

    internal delegate IntPtr HookProc(int code, IntPtr message, IntPtr data);
    [StructLayout(LayoutKind.Sequential)] internal struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] internal struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] internal struct MouseData { public int X, Y; public uint Mouse, Flags, Time; public UIntPtr Extra; }
    [StructLayout(LayoutKind.Sequential)] internal struct KeyboardData { public uint Key, Scan, Flags, Time; public UIntPtr Extra; }
    [DllImport("user32.dll")] private static extern bool IsWindow(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] private static extern IntPtr GetAncestor(IntPtr hwnd, uint flags);
    [DllImport("user32.dll")] private static extern IntPtr GetDesktopWindow();
    [DllImport("user32.dll")] private static extern IntPtr GetShellWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern int GetClassNameW(IntPtr hwnd, StringBuilder name, int capacity);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool ClientToScreen(IntPtr hwnd, ref Point point);
    [DllImport("dwmapi.dll")] private static extern int DwmGetWindowAttribute(IntPtr hwnd, int attribute, out int value, int size);
    [DllImport("user32.dll")] private static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr GetWindowLongPtrW(IntPtr hwnd, int index);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SetWindowLongPtrW(IntPtr hwnd, int index, IntPtr value);
    [DllImport("user32.dll", SetLastError = true)] private static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool SetLayeredWindowAttributes(IntPtr hwnd, uint colorKey, byte alpha, uint flags);
    [DllImport("user32.dll")] private static extern short GetAsyncKeyState(int key);
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] private static extern IntPtr WindowFromPoint(Point point);
    [DllImport("kernel32.dll", SetLastError = true)] private static extern SafeProcessHandle OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] private static extern bool QueryFullProcessImageNameW(SafeProcessHandle process, uint flags, StringBuilder image, ref int length);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] private static extern IntPtr GetModuleHandleW(string? name);
    [DllImport("user32.dll", SetLastError = true)] private static extern IntPtr SetWindowsHookExW(int kind, HookProc callback, IntPtr module, uint thread);
    [DllImport("user32.dll")] private static extern bool UnhookWindowsHookEx(IntPtr hook);
    [DllImport("user32.dll")] private static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr message, IntPtr data);
}
