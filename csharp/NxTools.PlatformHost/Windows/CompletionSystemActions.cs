using System.ComponentModel;
using System.Diagnostics;
using System.Media;
using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface ICompletionSystemActions
{
    WindowsCommandResult PlayNotificationSound();

    WindowsCommandResult RequestShutdown(ushort delaySeconds);

    WindowsCommandResult RequestSuspend(bool hibernate);
}

internal sealed class WindowsCompletionSystemActions : ICompletionSystemActions
{
    private const ushort MaximumShutdownDelaySeconds = 60;
    private const int ErrorNotSupported = 50;
    private const uint TokenAdjustPrivileges = 0x0020;
    private const uint TokenQuery = 0x0008;
    private const uint SePrivilegeEnabled = 0x00000002;
    private const string ShutdownPrivilege = "SeShutdownPrivilege";

    public WindowsCommandResult PlayNotificationSound()
    {
        try
        {
            SystemSounds.Asterisk.Play();
            return new WindowsCommandResult(true);
        }
        catch (Exception error) when (error is InvalidOperationException or ExternalException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public WindowsCommandResult RequestShutdown(ushort delaySeconds)
    {
        if (delaySeconds > MaximumShutdownDelaySeconds)
        {
            return new WindowsCommandResult(false, "Shutdown delay exceeds the bounded policy.");
        }
        try
        {
            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = Path.Combine(Environment.SystemDirectory, "shutdown.exe"),
                UseShellExecute = false,
                CreateNoWindow = true,
                ArgumentList =
                {
                    "/s",
                    "/t",
                    delaySeconds.ToString(System.Globalization.CultureInfo.InvariantCulture),
                    "/d",
                    "p:4:1"
                }
            });
            if (process is null)
            {
                return new WindowsCommandResult(false, "Windows did not start the shutdown adapter.");
            }
            if (!process.WaitForExit(5_000))
            {
                try
                {
                    process.Kill(entireProcessTree: true);
                    process.WaitForExit(2_000);
                }
                catch
                {
                    // The original bounded failure remains authoritative.
                }
                return new WindowsCommandResult(false, "Windows shutdown scheduling timed out.");
            }
            return process.ExitCode == 0
                ? new WindowsCommandResult(true)
                : new WindowsCommandResult(
                    false,
                    "Windows rejected the shutdown request.",
                    NativeErrorCode: process.ExitCode);
        }
        catch (Win32Exception error)
        {
            return new WindowsCommandResult(false, error.Message, NativeErrorCode: error.NativeErrorCode);
        }
        catch (Exception error) when (error is InvalidOperationException or ArgumentException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    public WindowsCommandResult RequestSuspend(bool hibernate)
    {
        if (!(hibernate ? IsPwrHibernateAllowed() : IsPwrSuspendAllowed()))
        {
            return new WindowsCommandResult(
                false,
                "Windows does not support the requested suspend state.",
                NativeErrorCode: ErrorNotSupported);
        }

        if (!OpenProcessToken(
                GetCurrentProcess(),
                TokenAdjustPrivileges | TokenQuery,
                out var token))
        {
            var nativeErrorCode = Marshal.GetLastPInvokeError();
            return new WindowsCommandResult(
                false,
                "Windows could not open the process token for suspend.",
                NativeErrorCode: nativeErrorCode);
        }

        var restorePrivilege = false;
        TokenPrivileges previousState = default;
        try
        {
            if (!LookupPrivilegeValue(null, ShutdownPrivilege, out var luid))
            {
                var nativeErrorCode = Marshal.GetLastPInvokeError();
                return new WindowsCommandResult(
                    false,
                    "Windows could not resolve the shutdown privilege.",
                    NativeErrorCode: nativeErrorCode);
            }

            var privileges = new TokenPrivileges
            {
                PrivilegeCount = 1,
                Privileges = new LuidAndAttributes
                {
                    Luid = luid,
                    Attributes = SePrivilegeEnabled
                }
            };
            Marshal.SetLastPInvokeError(0);
            if (!AdjustTokenPrivilegesCapture(
                    token,
                    false,
                    ref privileges,
                    (uint)Marshal.SizeOf<TokenPrivileges>(),
                    out previousState,
                    out _))
            {
                var nativeErrorCode = Marshal.GetLastPInvokeError();
                return new WindowsCommandResult(
                    false,
                    "Windows could not enable the shutdown privilege.",
                    NativeErrorCode: nativeErrorCode);
            }
            var privilegeError = Marshal.GetLastPInvokeError();
            if (privilegeError != 0)
            {
                return new WindowsCommandResult(
                    false,
                    "Windows could not enable the shutdown privilege.",
                    NativeErrorCode: privilegeError);
            }
            restorePrivilege = true;

            Marshal.SetLastPInvokeError(0);
            if (!SetSuspendState(hibernate, false, false))
            {
                var nativeErrorCode = Marshal.GetLastPInvokeError();
                return new WindowsCommandResult(
                    false,
                    "Windows rejected the suspend request.",
                    NativeErrorCode: nativeErrorCode);
            }
            return new WindowsCommandResult(true);
        }
        finally
        {
            if (restorePrivilege)
            {
                _ = AdjustTokenPrivilegesRestore(
                    token,
                    false,
                    ref previousState,
                    0,
                    IntPtr.Zero,
                    IntPtr.Zero);
            }
            _ = CloseHandle(token);
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Luid
    {
        public uint LowPart;
        public int HighPart;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct LuidAndAttributes
    {
        public Luid Luid;
        public uint Attributes;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TokenPrivileges
    {
        public uint PrivilegeCount;
        public LuidAndAttributes Privileges;
    }

    [DllImport("powrprof.dll", EntryPoint = "IsPwrSuspendAllowed", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    private static extern bool IsPwrSuspendAllowed();

    [DllImport("powrprof.dll", EntryPoint = "IsPwrHibernateAllowed", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    private static extern bool IsPwrHibernateAllowed();

    [DllImport("powrprof.dll", EntryPoint = "SetSuspendState", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    private static extern bool SetSuspendState(
        [MarshalAs(UnmanagedType.U1)] bool hibernate,
        [MarshalAs(UnmanagedType.U1)] bool forceCritical,
        [MarshalAs(UnmanagedType.U1)] bool disableWakeEvent);

    [DllImport("kernel32.dll", EntryPoint = "GetCurrentProcess", ExactSpelling = true)]
    private static extern IntPtr GetCurrentProcess();

    [DllImport("advapi32.dll", EntryPoint = "OpenProcessToken", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenProcessToken(
        IntPtr process,
        uint desiredAccess,
        out IntPtr token);

    [DllImport("advapi32.dll", EntryPoint = "LookupPrivilegeValueW", ExactSpelling = true, SetLastError = true,
        CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool LookupPrivilegeValue(
        string? systemName,
        string name,
        out Luid luid);

    [DllImport("advapi32.dll", EntryPoint = "AdjustTokenPrivileges", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AdjustTokenPrivilegesCapture(
        IntPtr token,
        [MarshalAs(UnmanagedType.Bool)] bool disableAllPrivileges,
        ref TokenPrivileges newState,
        uint bufferLength,
        out TokenPrivileges previousState,
        out uint returnLength);

    [DllImport("advapi32.dll", EntryPoint = "AdjustTokenPrivileges", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AdjustTokenPrivilegesRestore(
        IntPtr token,
        [MarshalAs(UnmanagedType.Bool)] bool disableAllPrivileges,
        ref TokenPrivileges newState,
        uint bufferLength,
        IntPtr previousState,
        IntPtr returnLength);

    [DllImport("kernel32.dll", EntryPoint = "CloseHandle", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);
}
