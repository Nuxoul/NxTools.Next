using System.Runtime.InteropServices;
using System.Security.Principal;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

internal static class ArchiveInstallCommands
{
    internal const string CheckUser = "--check-archive-install-user";
    internal const string Uninstall = "--uninstall-archive-integration";
    internal const int WrongUser = 12;

    internal static int Run(string[] args)
    {
        if (args.Length != 2 || !Guid.TryParseExact(args[1], "D", out var correlationId) || correlationId == Guid.Empty)
            return 2;
        if (!IsInteractiveUser()) return WrongUser;
        if (args[0] == CheckUser) return 0;
        using var log = PlatformLog.CreateDefault();
        return Revert(ArchiveIntegrationService.CreateProduct(log), correlationId);
    }

    internal static int Revert(ArchiveIntegrationService service, Guid correlationId)
    {
        var observed = service.ExecuteBounded(new(ArchiveIntegrationService.WireVersion, "revert", 0, false, false, []), correlationId);
        return observed.Failure is null && observed.AppliedRevision is null &&
            observed.State is ArchiveShellIntegrationStates.Reverted or ArchiveShellIntegrationStates.NotApplied ? 0 : 11;
    }

    // Setup normally has Inno's original-user process. An already elevated/SYSTEM invocation
    // can lack it: verify that a fallback token still represents this session's desktop user.
    private static bool IsInteractiveUser()
    {
        try
        {
            using var current = WindowsIdentity.GetCurrent();
            if (current.IsSystem || current.User is null) return false;
            var shell = GetShellWindow();
            if (shell == IntPtr.Zero || GetWindowThreadProcessId(shell, out var pid) == 0) return false;
            using var process = OpenProcess(0x1000, false, pid); // PROCESS_QUERY_LIMITED_INFORMATION
            if (process.IsInvalid || !OpenProcessToken(process, 0x0008, out var token)) return false;
            using (token)
            using (var identity = new WindowsIdentity(token.DangerousGetHandle()))
                return identity.User is { } shellUser && current.User.Equals(shellUser);
        }
        catch (Exception) { return false; }
    }

    [DllImport("user32.dll")]
    private static extern IntPtr GetShellWindow();
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern SafeProcessHandle OpenProcess(uint access, bool inherit, uint processId);
    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool OpenProcessToken(SafeProcessHandle process, uint access, out SafeAccessTokenHandle token);
}
