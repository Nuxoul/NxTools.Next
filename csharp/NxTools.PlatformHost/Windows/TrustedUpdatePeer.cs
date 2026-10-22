using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows;

// Called only after protected registration/bootstrap validation. SCM is the authority for
// the live own-process service PID; a medium-integrity client need not open a SYSTEM process.
internal static class TrustedUpdatePeer
{
    internal readonly record struct ServiceIdentity(uint ProcessId, uint State, uint Type);

    internal static void Verify(SafePipeHandle pipe, Func<ServiceIdentity>? readService = null)
    {
        if (!GetNamedPipeServerProcessId(pipe, out var serverId))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        var service = (readService ?? ReadServiceIdentity)();
        if (serverId == 0 || service.ProcessId != serverId || service.State != 4 || service.Type != 0x10)
            throw new InvalidDataException("Updater pipe is not the approved running own-process service.");
    }

    private static ServiceIdentity ReadServiceIdentity()
    {
        using var manager = OpenSCManagerW(null, null, 1); // SC_MANAGER_CONNECT only.
        if (manager.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
        using var service = OpenServiceW(manager, TrustedUpdateBroker.ServiceName, 4); // SERVICE_QUERY_STATUS only.
        if (service.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error());
        if (!QueryServiceStatusEx(service, 0, out var status, (uint)Marshal.SizeOf<ServiceStatusProcess>(), out _))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        return new(status.ProcessId, status.CurrentState, status.ServiceType);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ServiceStatusProcess
    {
        internal uint ServiceType;
        internal uint CurrentState;
        internal uint ControlsAccepted;
        internal uint Win32ExitCode;
        internal uint ServiceSpecificExitCode;
        internal uint CheckPoint;
        internal uint WaitHint;
        internal uint ProcessId;
        internal uint ServiceFlags;
    }

    private sealed class ServiceHandle : SafeHandleZeroOrMinusOneIsInvalid
    {
        private ServiceHandle() : base(true) { }
        protected override bool ReleaseHandle() => CloseServiceHandle(handle);
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
    private static extern ServiceHandle OpenSCManagerW(string? machineName, string? databaseName, uint access);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
    private static extern ServiceHandle OpenServiceW(ServiceHandle manager, string name, uint access);
    [DllImport("advapi32.dll", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool QueryServiceStatusEx(ServiceHandle service, uint level,
        out ServiceStatusProcess status, uint size, out uint needed);
    [DllImport("advapi32.dll", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseServiceHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetNamedPipeServerProcessId(SafePipeHandle pipe, out uint processId);
}
