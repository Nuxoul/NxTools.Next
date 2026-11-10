using System.ComponentModel;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal delegate IntPtr KeyboardHookProcedure(int code, IntPtr message, IntPtr data);

[StructLayout(LayoutKind.Sequential)]
internal struct KeyboardHookData
{
    public uint VirtualKey;
    public uint ScanCode;
    public uint Flags;
    public uint Timestamp;
    public UIntPtr ExtraInfo;
}

// The fixture supplies only this native boundary; thread ownership and callback routing stay real.
internal interface IKeyboardHookPlatform
{
    uint InitializeThread();
    IntPtr Install(KeyboardHookProcedure procedure);
    void RunMessageLoop();
    bool PostQuit(uint threadId);
    void Uninstall(IntPtr hook);
    IntPtr CallNext(IntPtr hook, int code, IntPtr message, IntPtr data);
}

internal sealed class WindowsKeyboardHookPlatform : IKeyboardHookPlatform
{
    public uint InitializeThread()
    {
        // Create the thread message queue before publishing its ID to Dispose.
        _ = NativeMethods.PeekMessage(out _, IntPtr.Zero, 0, 0, 0);
        return NativeMethods.GetCurrentThreadId();
    }

    public IntPtr Install(KeyboardHookProcedure procedure)
    {
        var hook = NativeMethods.SetWindowsHookEx(13, procedure, NativeMethods.GetModuleHandle(null), 0);
        if (hook == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
        return hook;
    }

    public void RunMessageLoop()
    {
        int result;
        while ((result = NativeMethods.GetMessage(out var message, IntPtr.Zero, 0, 0)) > 0)
        {
            _ = NativeMethods.TranslateMessage(ref message);
            _ = NativeMethods.DispatchMessage(ref message);
        }
        if (result < 0) throw new Win32Exception(Marshal.GetLastWin32Error());
    }

    public bool PostQuit(uint threadId) => NativeMethods.PostThreadMessage(threadId, 0x0012, UIntPtr.Zero, IntPtr.Zero);
    public void Uninstall(IntPtr hook) => _ = NativeMethods.UnhookWindowsHookEx(hook);
    public IntPtr CallNext(IntPtr hook, int code, IntPtr message, IntPtr data) =>
        NativeMethods.CallNextHookEx(hook, code, message, data);

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeMessage
    {
        public IntPtr Window;
        public uint Message;
        public UIntPtr WParam;
        public IntPtr LParam;
        public uint Time;
        public int X;
        public int Y;
        public uint Private;
    }

    private static class NativeMethods
    {
        [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr GetModuleHandle(string? moduleName);
        [DllImport("user32.dll", SetLastError = true)]
        public static extern IntPtr SetWindowsHookEx(int hookId, KeyboardHookProcedure procedure, IntPtr module, uint threadId);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)] public static extern bool UnhookWindowsHookEx(IntPtr hook);
        [DllImport("user32.dll")] public static extern IntPtr CallNextHookEx(IntPtr hook, int code, IntPtr message, IntPtr data);
        [DllImport("user32.dll", SetLastError = true)]
        public static extern int GetMessage(out NativeMessage message, IntPtr window, uint minimum, uint maximum);
        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)] public static extern bool PeekMessage(out NativeMessage message, IntPtr window, uint minimum, uint maximum, uint remove);
        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)] public static extern bool TranslateMessage(ref NativeMessage message);
        [DllImport("user32.dll")] public static extern IntPtr DispatchMessage(ref NativeMessage message);
        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)] public static extern bool PostThreadMessage(uint threadId, uint message, UIntPtr wParam, IntPtr lParam);
    }
}
