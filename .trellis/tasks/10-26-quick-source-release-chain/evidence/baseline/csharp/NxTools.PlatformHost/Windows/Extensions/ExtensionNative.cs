using System.Runtime.InteropServices;
using System.Security.Principal;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Extensions;

// Every returned kernel handle has exactly one owner. Pseudo current-process handles are
// borrowed only for the duration of a native call. No Job or parent handle is inheritable.
internal sealed class ExtensionKernelHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    private ExtensionKernelHandle() : base(true) { }
    internal ExtensionKernelHandle(IntPtr value) : base(true) => SetHandle(value);
    protected override bool ReleaseHandle() => ExtensionNative.CloseHandle(handle);
}

internal static class ExtensionNative
{
    internal const uint Synchronize = 0x100000, QueryLimited = 0x1000;
    [StructLayout(LayoutKind.Sequential)] internal struct SecurityAttributes
    { internal int Length; internal IntPtr Descriptor; internal int Inherit; }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] internal struct StartupInfo
    {
        internal int Size; internal IntPtr Reserved, Desktop, Title;
        internal uint X, Y, XSize, YSize, XCountChars, YCountChars, FillAttribute, Flags;
        internal ushort ShowWindow, ReservedSize; internal IntPtr ReservedBytes, Input, Output, Error;
    }
    [StructLayout(LayoutKind.Sequential)] internal struct StartupInfoEx
    { internal StartupInfo Startup; internal IntPtr Attributes; }
    [StructLayout(LayoutKind.Sequential)] internal struct ProcessInfo
    { internal IntPtr Process, Thread; internal uint ProcessId, ThreadId; }
    [StructLayout(LayoutKind.Sequential)] internal struct BasicLimit
    {
        internal long ProcessTime, JobTime; internal uint Flags;
        internal UIntPtr MinWorkingSet, MaxWorkingSet; internal uint ActiveProcesses;
        internal UIntPtr Affinity; internal uint Priority, Scheduling;
    }
    [StructLayout(LayoutKind.Sequential)] internal struct IoCounters
    { internal ulong ReadOps, WriteOps, OtherOps, ReadBytes, WriteBytes, OtherBytes; }
    [StructLayout(LayoutKind.Sequential)] internal struct ExtendedLimit
    {
        internal BasicLimit Basic; internal IoCounters Io;
        internal UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
    }
    [StructLayout(LayoutKind.Sequential)] internal struct BasicAccounting
    {
        internal long UserTime, KernelTime, PeriodUserTime, PeriodKernelTime;
        internal uint PageFaults, TotalProcesses, ActiveProcesses, TerminatedProcesses;
    }

    [DllImport("kernel32.dll", EntryPoint = "CloseHandle", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", EntryPoint = "GetCurrentProcess", ExactSpelling = true)]
    internal static extern IntPtr CurrentProcess();
    [DllImport("kernel32.dll", EntryPoint = "CreateJobObjectW", ExactSpelling = true, SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern ExtensionKernelHandle CreateJob(IntPtr attributes, string? name);
    [DllImport("kernel32.dll", EntryPoint = "SetInformationJobObject", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool SetJob(ExtensionKernelHandle job, int kind, ref ExtendedLimit limits, int size);
    [DllImport("kernel32.dll", EntryPoint = "QueryInformationJobObject", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool QueryJob(ExtensionKernelHandle job, int kind, out BasicAccounting info, int size, IntPtr returned);
    [DllImport("kernel32.dll", EntryPoint = "QueryInformationJobObject", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool QueryCurrentLimits(IntPtr job, int kind, out ExtendedLimit info, int size, IntPtr returned);
    [DllImport("kernel32.dll", EntryPoint = "AssignProcessToJobObject", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool AssignJob(ExtensionKernelHandle job, IntPtr process);
    [DllImport("kernel32.dll", EntryPoint = "TerminateJobObject", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool TerminateJob(ExtensionKernelHandle job, uint code);
    [DllImport("kernel32.dll", EntryPoint = "TerminateProcess", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool TerminateProcess(ExtensionKernelHandle process, uint code);
    [DllImport("kernel32.dll", EntryPoint = "WaitForSingleObject", ExactSpelling = true, SetLastError = true)]
    internal static extern uint Wait(ExtensionKernelHandle handle, uint milliseconds);
    [DllImport("kernel32.dll", EntryPoint = "GetExitCodeProcess", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] internal static extern bool ExitCode(ExtensionKernelHandle process, out uint code);
    [DllImport("kernel32.dll", EntryPoint = "ResumeThread", ExactSpelling = true, SetLastError = true)]
    internal static extern uint Resume(ExtensionKernelHandle thread);
    [DllImport("kernel32.dll", EntryPoint = "IsProcessInJob", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool IsInJob(ExtensionKernelHandle process, IntPtr job, [MarshalAs(UnmanagedType.Bool)] out bool result);
    [DllImport("kernel32.dll", EntryPoint = "CreatePipe", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool CreatePipe(out SafeFileHandle read, out SafeFileHandle write, ref SecurityAttributes attributes, uint size);
    [DllImport("kernel32.dll", EntryPoint = "SetHandleInformation", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool SetHandleInformation(SafeFileHandle handle, uint mask, uint flags);
    [DllImport("kernel32.dll", EntryPoint = "InitializeProcThreadAttributeList", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool InitializeAttributes(IntPtr list, int count, uint flags, ref UIntPtr size);
    [DllImport("kernel32.dll", EntryPoint = "UpdateProcThreadAttribute", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool UpdateAttribute(IntPtr list, uint flags, UIntPtr attribute, IntPtr value, UIntPtr size, IntPtr previous, IntPtr returned);
    [DllImport("kernel32.dll", EntryPoint = "DeleteProcThreadAttributeList", ExactSpelling = true)]
    private static extern void DeleteAttributes(IntPtr list);
    [DllImport("kernel32.dll", EntryPoint = "CreateProcessW", ExactSpelling = true, SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool CreateProcess(string application, StringBuilder commandLine, IntPtr processAttributes, IntPtr threadAttributes,
        [MarshalAs(UnmanagedType.Bool)] bool inherit, uint flags, IntPtr environment, string directory, ref StartupInfoEx startup, out ProcessInfo info);
    [DllImport("advapi32.dll", EntryPoint = "OpenProcessToken", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool OpenToken(IntPtr process, uint access, out ExtensionKernelHandle token);
    [DllImport("advapi32.dll", EntryPoint = "GetTokenInformation", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool TokenInfo(ExtensionKernelHandle token, int kind, IntPtr buffer, int length, out int returned);
    [DllImport("advapi32.dll", EntryPoint = "GetSidSubAuthorityCount", ExactSpelling = true)]
    private static extern IntPtr SidCount(IntPtr sid);
    [DllImport("advapi32.dll", EntryPoint = "GetSidSubAuthority", ExactSpelling = true)]
    private static extern IntPtr SidAuthority(IntPtr sid, uint index);
    [DllImport("kernel32.dll", EntryPoint = "OpenProcess", ExactSpelling = true, SetLastError = true)]
    internal static extern ExtensionKernelHandle OpenProcess(uint access, [MarshalAs(UnmanagedType.Bool)] bool inherit, uint pid);
    [DllImport("kernel32.dll", EntryPoint = "QueryFullProcessImageNameW", ExactSpelling = true, SetLastError = true, CharSet = CharSet.Unicode)]
    [return: MarshalAs(UnmanagedType.Bool)] private static extern bool ImageName(ExtensionKernelHandle process, uint flags, StringBuilder path, ref uint length);

    internal static void Require(bool result, string stage)
    { if (!result) throw new ExtensionFailure(stage, Marshal.GetLastWin32Error()); }
    internal static bool InJob(ExtensionKernelHandle process)
    { Require(IsInJob(process, IntPtr.Zero, out var inJob), "query_process_job"); return inJob; }
    internal static ExtendedLimit CurrentJobLimits()
    {
        Require(QueryCurrentLimits(IntPtr.Zero, 9, out var limits, Marshal.SizeOf<ExtendedLimit>(), IntPtr.Zero), "query_current_job_limits");
        return limits;
    }

    internal static ExtensionKernelHandle MakeJob(long memory = 0, int processes = 0)
    {
        var job = CreateJob(IntPtr.Zero, null);
        if (job.IsInvalid) { job.Dispose(); throw new ExtensionFailure("create_job", Marshal.GetLastWin32Error()); }
        try
        {
            var limits = new ExtendedLimit { Basic = new BasicLimit { Flags = 0x2000 } }; // KILL_ON_JOB_CLOSE; no breakaway.
            if (memory != 0) { limits.Basic.Flags |= 0x200; limits.JobMemory = (UIntPtr)(ulong)memory; }
            if (processes != 0) { limits.Basic.Flags |= 0x8; limits.Basic.ActiveProcesses = (uint)processes; }
            Require(SetJob(job, 9, ref limits, Marshal.SizeOf<ExtendedLimit>()), "configure_job");
            return job;
        }
        catch { job.Dispose(); throw; }
    }
    internal static bool JobEmpty(ExtensionKernelHandle job)
    {
        Require(QueryJob(job, 1, out var accounting, Marshal.SizeOf<BasicAccounting>(), IntPtr.Zero), "query_job");
        return accounting.ActiveProcesses == 0;
    }

    internal static (SafeFileHandle Ours, SafeFileHandle Child) Pipe(bool reads)
    {
        var security = new SecurityAttributes { Length = Marshal.SizeOf<SecurityAttributes>(), Inherit = 1 };
        Require(CreatePipe(out var read, out var write, ref security, 8192), "create_pipe");
        var ours = reads ? read : write;
        var child = reads ? write : read;
        try { Require(SetHandleInformation(ours, 1, 0), "pipe_inheritance"); return (ours, child); }
        catch { read.Dispose(); write.Dispose(); throw; }
    }

    // Attribute storage and the three handle values are alive through CreateProcessW only.
    // Caller owns all pipes before this call and owns the returned process/thread afterward.
    internal static (ExtensionKernelHandle Process, ExtensionKernelHandle Thread, uint Pid) Spawn(
        ExtensionStartRequest request, SafeFileHandle input, SafeFileHandle output, SafeFileHandle error) =>
        Spawn(request.Executable, request.Arguments, request.WorkingDirectory, input, output, error);

    // Low-level suspended, allowlisted stdio spawn; callers own their executable policy.
    internal static (ExtensionKernelHandle Process, ExtensionKernelHandle Thread, uint Pid) Spawn(
        string executable, IReadOnlyList<string> arguments, string workingDirectory,
        SafeFileHandle input, SafeFileHandle output, SafeFileHandle error)
    {
        UIntPtr size = UIntPtr.Zero;
        InitializeAttributes(IntPtr.Zero, 1, 0, ref size);
        var list = Marshal.AllocHGlobal(checked((int)size.ToUInt64()));
        var handles = Marshal.AllocHGlobal(IntPtr.Size * 3);
        var initialized = false;
        try
        {
            Require(InitializeAttributes(list, 1, 0, ref size), "initialize_attributes");
            initialized = true;
            Marshal.WriteIntPtr(handles, input.DangerousGetHandle());
            Marshal.WriteIntPtr(handles, IntPtr.Size, output.DangerousGetHandle());
            Marshal.WriteIntPtr(handles, IntPtr.Size * 2, error.DangerousGetHandle());
            Require(UpdateAttribute(list, 0, (UIntPtr)0x20002, handles, (UIntPtr)(IntPtr.Size * 3), IntPtr.Zero, IntPtr.Zero), "handle_allowlist");
            var startup = new StartupInfoEx
            {
                Startup = new StartupInfo { Size = Marshal.SizeOf<StartupInfoEx>(), Flags = 0x100,
                    Input = input.DangerousGetHandle(), Output = output.DangerousGetHandle(), Error = error.DangerousGetHandle() },
                Attributes = list
            };
            Require(CreateProcess(executable, new StringBuilder(CommandLine(executable, arguments)),
                IntPtr.Zero, IntPtr.Zero, true, 0x4 | 0x08000000 | 0x80000, IntPtr.Zero,
                workingDirectory, ref startup, out var info), "create_process");
            return (new(info.Process), new(info.Thread), info.ProcessId);
        }
        finally
        {
            if (initialized) DeleteAttributes(list);
            Marshal.FreeHGlobal(list); Marshal.FreeHGlobal(handles);
            GC.KeepAlive(input); GC.KeepAlive(output); GC.KeepAlive(error);
        }
    }

    internal static string CommandLine(string executable, IEnumerable<string> arguments)
    {
        static string Quote(string arg)
        {
            var result = new StringBuilder("\"");
            var slashes = 0;
            foreach (var c in arg)
            {
                if (c == '\\') { slashes++; continue; }
                result.Append('\\', c == '"' ? slashes * 2 + 1 : slashes).Append(c);
                slashes = 0;
            }
            return result.Append('\\', slashes * 2).Append('"').ToString();
        }
        return string.Join(" ", new[] { executable }.Concat(arguments).Select(Quote));
    }

    private static T ReadToken<T>(ExtensionKernelHandle token, int kind, Func<IntPtr, T> read)
    {
        TokenInfo(token, kind, IntPtr.Zero, 0, out var size);
        if (size <= 0 || size > 65536) throw new ExtensionFailure("token_information", Marshal.GetLastWin32Error());
        var buffer = Marshal.AllocHGlobal(size);
        try { Require(TokenInfo(token, kind, buffer, size, out _), "token_information"); return read(buffer); }
        finally { Marshal.FreeHGlobal(buffer); }
    }
    internal static (string Sid, int Session, int Integrity, bool Elevated) Identity(IntPtr process)
    {
        Require(OpenToken(process, 8, out var token), "open_token");
        using (token)
        {
            return (ReadToken(token, 1, p => new SecurityIdentifier(Marshal.ReadIntPtr(p)).Value),
                ReadToken(token, 12, Marshal.ReadInt32),
                ReadToken(token, 25, p =>
                {
                    var sid = Marshal.ReadIntPtr(p);
                    var count = Marshal.ReadByte(SidCount(sid));
                    if (count == 0) throw new ExtensionFailure("token_integrity");
                    return Marshal.ReadInt32(SidAuthority(sid, (uint)(count - 1)));
                }), ReadToken(token, 20, p => Marshal.ReadInt32(p) != 0));
        }
    }
    internal static void Ordinary(IntPtr process)
    {
        var identity = Identity(process);
        if (identity.Elevated || identity.Integrity >= 0x3000 || identity.Sid == "S-1-5-18")
            throw new ExtensionFailure("token_not_ordinary");
    }
    internal static string ProcessImage(ExtensionKernelHandle process)
    {
        var path = new StringBuilder(32768); uint size = (uint)path.Capacity;
        Require(ImageName(process, 0, path, ref size), "parent_image");
        return path.ToString();
    }
}
