using System.ComponentModel;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface IOpenWithShellPlatform
{
    int ShowChooser(IntPtr owner, string target);
}

internal static class OpenWithShellDialog
{
    // Called inside the adapter's existing detached STA Shell worker, never the
    // serial command reader/EventWindow. No new worker or early acknowledgement.
    internal static WindowsCommandResult Show(string target, Action<string>? stageChanged = null) =>
        Show(target, IntPtr.Zero, new NativeOpenWithPlatform(), stageChanged);

    internal static WindowsCommandResult Show(string target, IntPtr owner,
        IOpenWithShellPlatform platform, Action<string>? stageChanged = null)
    {
        void Stage(string stage)
        {
            try { stageChanged?.Invoke(stage); }
            catch { /* Diagnostics must not change the chooser outcome. */ }
        }
        try
        {
            if (!Path.IsPathFullyQualified(target) || !File.Exists(target))
                return new WindowsCommandResult(false, "The file is no longer available for Open With.",
                    Data: "target_missing", NativeErrorCode: 2);
            Stage("open_with_requested");
            var result = platform.ShowChooser(owner, target);
            if (result is 1 or unchecked((int)0x800704C7))
            {
                Stage("cancelled");
                // Success=false deliberately cannot trigger success-only close.
                // Error=null + Data=cancelled must be handled before error toast.
                return new WindowsCommandResult(false, Data: "cancelled", NativeErrorCode: result);
            }
            if (result != 0)
            {
                Stage("open_with_failed");
                return new WindowsCommandResult(false, "Windows could not open the application chooser. Try again.",
                    Data: "open_with_failed", NativeErrorCode: result);
            }
            Stage("open_with_completed");
            return new WindowsCommandResult(true, Data: "open_with_completed");
        }
        catch (Exception error)
        {
            Stage("open_with_failed");
            return new WindowsCommandResult(false, "Windows could not open the application chooser. Try again.",
                Data: "open_with_failed", NativeErrorCode: error is Win32Exception native ? native.NativeErrorCode : error.HResult);
        }
    }

    private sealed class NativeOpenWithPlatform : IOpenWithShellPlatform
    {
        public int ShowChooser(IntPtr owner, string target)
        {
            using var apartment = ExplorerShellApartment.Enter();
            if (apartment.Result < 0) return apartment.Result;
            var info = new OpenAsInfo { File = target, Flags = 0x1 | 0x4 }; // ALLOW_REGISTRATION | EXEC
            return SHOpenWithDialog(owner, ref info);
        }
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct OpenAsInfo
    {
        [MarshalAs(UnmanagedType.LPWStr)] public string File;
        [MarshalAs(UnmanagedType.LPWStr)] public string? Class;
        public uint Flags;
    }
    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern int SHOpenWithDialog(IntPtr owner, ref OpenAsInfo info);
}
