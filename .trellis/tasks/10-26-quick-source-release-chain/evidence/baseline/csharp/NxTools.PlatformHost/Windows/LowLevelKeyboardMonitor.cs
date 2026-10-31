using System.ComponentModel;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Thin Windows adapter for global keyboard facts. Product gesture semantics live in the detector,
/// and the hook callback never performs foreground capture, IPC or UI work.
/// </summary>
internal sealed class LowLevelKeyboardMonitor : IDisposable
{
    private const int WhKeyboardLl = 13;
    private const int WmKeyDown = 0x0100;
    private const int WmKeyUp = 0x0101;
    private const int WmSysKeyDown = 0x0104;
    private const int WmSysKeyUp = 0x0105;
    private const uint LlkhfInjected = 0x00000010;
    private const uint LlkhfLowerIlInjected = 0x00000002;

    private readonly Action<int, uint> _keyDown;
    private readonly Action<int> _keyUp;
    private readonly HookProcedure _procedure;
    private IntPtr _hook;

    public LowLevelKeyboardMonitor(Action<int, uint> keyDown, Action<int> keyUp)
    {
        _keyDown = keyDown;
        _keyUp = keyUp;
        _procedure = HookCallback;
        _hook = NativeMethods.SetWindowsHookEx(
            WhKeyboardLl,
            _procedure,
            NativeMethods.GetModuleHandle(null),
            0);
        if (_hook == IntPtr.Zero)
        {
            throw new Win32Exception(Marshal.GetLastWin32Error());
        }
    }

    public bool IsInstalled => _hook != IntPtr.Zero;

    public void Dispose()
    {
        if (_hook == IntPtr.Zero)
        {
            return;
        }
        _ = NativeMethods.UnhookWindowsHookEx(_hook);
        _hook = IntPtr.Zero;
        GC.SuppressFinalize(this);
    }

    private IntPtr HookCallback(int code, IntPtr message, IntPtr data)
    {
        if (code >= 0)
        {
            var key = Marshal.PtrToStructure<KeyboardHookData>(data);
            if ((key.Flags & (LlkhfInjected | LlkhfLowerIlInjected)) == 0)
            {
                switch (message.ToInt32())
                {
                    case WmKeyDown:
                    case WmSysKeyDown:
                        _keyDown(unchecked((int)key.VirtualKey), key.Timestamp);
                        break;
                    case WmKeyUp:
                    case WmSysKeyUp:
                        _keyUp(unchecked((int)key.VirtualKey));
                        break;
                }
            }
        }
        return NativeMethods.CallNextHookEx(_hook, code, message, data);
    }

    [StructLayout(LayoutKind.Sequential)]
    private readonly struct KeyboardHookData
    {
        public readonly uint VirtualKey;
        public readonly uint ScanCode;
        public readonly uint Flags;
        public readonly uint Timestamp;
        public readonly UIntPtr ExtraInfo;
    }

    private delegate IntPtr HookProcedure(int code, IntPtr message, IntPtr data);

    private static class NativeMethods
    {
        [DllImport("user32.dll", SetLastError = true)]
        public static extern IntPtr SetWindowsHookEx(
            int hookId,
            HookProcedure procedure,
            IntPtr module,
            uint threadId);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool UnhookWindowsHookEx(IntPtr hook);

        [DllImport("user32.dll")]
        public static extern IntPtr CallNextHookEx(
            IntPtr hook,
            int code,
            IntPtr message,
            IntPtr data);

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr GetModuleHandle(string? moduleName);
    }
}
