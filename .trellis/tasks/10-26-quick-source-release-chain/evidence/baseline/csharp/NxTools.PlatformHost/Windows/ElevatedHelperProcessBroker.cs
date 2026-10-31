using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal static class ElevatedHelperProcessBroker
{
    private const int TokenLinkedToken = 19;
    private const uint MaximumAllowed = 0x02000000;
    private const int SecurityImpersonation = 2;
    private const int TokenPrimary = 1;
    private const uint CreateUnicodeEnvironment = 0x00000400;
    private const uint DetachedProcess = 0x00000008;

    public static int Launch(
        uint sessionId,
        string pipeName,
        string authenticationToken)
    {
        if (!IsValidRendezvous(pipeName, authenticationToken))
        {
            return 2;
        }
        try
        {
            return TryLaunch(
                sessionId,
                pipeName,
                authenticationToken,
                out var error)
                ? 0
                : ReportFailure(error);
        }
        catch (Exception error) when (
            error is Win32Exception or
            InvalidOperationException)
        {
            return ReportFailure(error.Message);
        }
    }

    internal static bool IsValidRendezvous(
        string pipeName,
        string authenticationToken) =>
        pipeName.StartsWith(
            "nxtools-next-elevated-nav-",
            StringComparison.Ordinal) &&
        pipeName.Length <= 96 &&
        pipeName.All(character =>
            char.IsAsciiLetterOrDigit(character) || character == '-') &&
        authenticationToken.Length == 64 &&
        authenticationToken.All(char.IsAsciiHexDigit);

    private static bool TryLaunch(
        uint sessionId,
        string pipeName,
        string authenticationToken,
        out string? error)
    {
        error = null;
        EnablePrivilege("SeTcbPrivilege");
        EnablePrivilege("SeAssignPrimaryTokenPrivilege");
        EnablePrivilege("SeIncreaseQuotaPrivilege");

        var userToken = IntPtr.Zero;
        var linkedToken = IntPtr.Zero;
        var primaryToken = IntPtr.Zero;
        var environment = IntPtr.Zero;
        try
        {
            if (!WTSQueryUserToken(sessionId, out userToken))
            {
                error =
                    $"WTSQueryUserToken failed for session {sessionId} ({Marshal.GetLastWin32Error()}).";
                return false;
            }
            var launchToken = userToken;
            if (TryGetLinkedToken(userToken, out linkedToken) &&
                IsTokenAdministrator(linkedToken))
            {
                launchToken = linkedToken;
            }
            else if (!IsTokenAdministrator(userToken))
            {
                error =
                    "The current Windows account has no elevated token for the same-integrity helper.";
                return false;
            }
            if (!DuplicateTokenEx(
                    launchToken,
                    MaximumAllowed,
                    IntPtr.Zero,
                    SecurityImpersonation,
                    TokenPrimary,
                    out primaryToken))
            {
                error = $"DuplicateTokenEx failed ({Marshal.GetLastWin32Error()}).";
                return false;
            }
            if (!CreateEnvironmentBlock(
                    out environment,
                    primaryToken,
                    false))
            {
                error =
                    $"CreateEnvironmentBlock failed ({Marshal.GetLastWin32Error()}).";
                return false;
            }
            var executable = Environment.ProcessPath
                ?? throw new InvalidOperationException(
                    "Cannot locate NxTools.PlatformHost.exe.");
            var startup = new StartupInfo
            {
                Size = Marshal.SizeOf<StartupInfo>(),
                Desktop = @"winsta0\default"
            };
            var commandLine = new StringBuilder(
                $"\"{executable}\" --elevated-directory-helper {pipeName} {authenticationToken}");
            if (!CreateProcessAsUser(
                    primaryToken,
                    null,
                    commandLine,
                    IntPtr.Zero,
                    IntPtr.Zero,
                    false,
                    CreateUnicodeEnvironment | DetachedProcess,
                    environment,
                    Path.GetDirectoryName(executable),
                    ref startup,
                    out var process))
            {
                error =
                    $"CreateProcessAsUser failed ({Marshal.GetLastWin32Error()}).";
                return false;
            }
            if (process.Process != IntPtr.Zero)
            {
                CloseHandle(process.Process);
            }
            if (process.Thread != IntPtr.Zero)
            {
                CloseHandle(process.Thread);
            }
            return true;
        }
        finally
        {
            if (environment != IntPtr.Zero)
            {
                DestroyEnvironmentBlock(environment);
            }
            if (primaryToken != IntPtr.Zero)
            {
                CloseHandle(primaryToken);
            }
            if (linkedToken != IntPtr.Zero)
            {
                CloseHandle(linkedToken);
            }
            if (userToken != IntPtr.Zero)
            {
                CloseHandle(userToken);
            }
        }
    }

    private static bool TryGetLinkedToken(
        IntPtr token,
        out IntPtr linkedToken)
    {
        linkedToken = IntPtr.Zero;
        var size = Marshal.SizeOf<TokenLinkedTokenData>();
        var buffer = Marshal.AllocHGlobal(size);
        try
        {
            if (!GetTokenInformation(
                    token,
                    TokenLinkedToken,
                    buffer,
                    size,
                    out _))
            {
                return false;
            }
            linkedToken = Marshal
                .PtrToStructure<TokenLinkedTokenData>(buffer)
                .LinkedToken;
            return linkedToken != IntPtr.Zero;
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    private static bool IsTokenAdministrator(IntPtr token)
    {
        try
        {
            using var identity = new WindowsIdentity(token);
            return new WindowsPrincipal(identity).IsInRole(
                WindowsBuiltInRole.Administrator);
        }
        catch
        {
            return false;
        }
    }

    private static void EnablePrivilege(string privilegeName)
    {
        if (!OpenProcessToken(
                Process.GetCurrentProcess().Handle,
                0x0020 | 0x0008,
                out var token))
        {
            return;
        }
        try
        {
            if (!LookupPrivilegeValue(
                    null,
                    privilegeName,
                    out var luid))
            {
                return;
            }
            var privileges = new TokenPrivileges
            {
                PrivilegeCount = 1,
                Privileges = new LuidAndAttributes
                {
                    Luid = luid,
                    Attributes = 0x00000002
                }
            };
            _ = AdjustTokenPrivileges(
                token,
                false,
                ref privileges,
                0,
                IntPtr.Zero,
                IntPtr.Zero);
        }
        finally
        {
            CloseHandle(token);
        }
    }

    private static int ReportFailure(string? error)
    {
        Console.Error.WriteLine(
            error ?? "The elevated directory helper could not be launched.");
        return 1;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TokenLinkedTokenData
    {
        public IntPtr LinkedToken;
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

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct StartupInfo
    {
        public int Size;
        public string? Reserved;
        public string? Desktop;
        public string? Title;
        public int X;
        public int Y;
        public int XSize;
        public int YSize;
        public int XCountChars;
        public int YCountChars;
        public int FillAttribute;
        public int Flags;
        public short ShowWindow;
        public short Reserved2;
        public IntPtr Reserved2Pointer;
        public IntPtr StandardInput;
        public IntPtr StandardOutput;
        public IntPtr StandardError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ProcessInformation
    {
        public IntPtr Process;
        public IntPtr Thread;
        public uint ProcessId;
        public uint ThreadId;
    }

    [DllImport("wtsapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool WTSQueryUserToken(
        uint sessionId,
        out IntPtr token);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DuplicateTokenEx(
        IntPtr existingToken,
        uint desiredAccess,
        IntPtr tokenAttributes,
        int impersonationLevel,
        int tokenType,
        out IntPtr newToken);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetTokenInformation(
        IntPtr token,
        int tokenInformationClass,
        IntPtr tokenInformation,
        int tokenInformationLength,
        out int returnLength);

    [DllImport("userenv.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateEnvironmentBlock(
        out IntPtr environment,
        IntPtr token,
        [MarshalAs(UnmanagedType.Bool)] bool inherit);

    [DllImport("userenv.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyEnvironmentBlock(IntPtr environment);

    [DllImport(
        "advapi32.dll",
        CharSet = CharSet.Unicode,
        SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateProcessAsUser(
        IntPtr token,
        string? applicationName,
        StringBuilder commandLine,
        IntPtr processAttributes,
        IntPtr threadAttributes,
        [MarshalAs(UnmanagedType.Bool)] bool inheritHandles,
        uint creationFlags,
        IntPtr environment,
        string? currentDirectory,
        ref StartupInfo startupInfo,
        out ProcessInformation processInformation);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenProcessToken(
        IntPtr process,
        uint desiredAccess,
        out IntPtr token);

    [DllImport(
        "advapi32.dll",
        CharSet = CharSet.Unicode,
        SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool LookupPrivilegeValue(
        string? systemName,
        string name,
        out Luid luid);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AdjustTokenPrivileges(
        IntPtr token,
        [MarshalAs(UnmanagedType.Bool)] bool disableAllPrivileges,
        ref TokenPrivileges newState,
        uint bufferLength,
        IntPtr previousState,
        IntPtr returnLength);

    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);
}
