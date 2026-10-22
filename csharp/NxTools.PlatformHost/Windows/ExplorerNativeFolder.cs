using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

internal interface IExplorerPidlPlatform
{
    IntPtr CreateAbsolute(string target);
    IntPtr Clone(IntPtr absolute);
    bool RemoveLast(IntPtr parent);
    IntPtr FindLast(IntPtr absolute);
    int OpenAndSelect(IntPtr parent, IntPtr relativeChild);
    void Free(IntPtr owned);
}

internal static class ExplorerNativeFolder
{
    internal static int Open(string target, bool isDirectory)
    {
        using var apartment = ExplorerShellApartment.Enter();
        if (apartment.Result < 0) return apartment.Result;
        if (!isDirectory) return Select(target, new NativePidlPlatform());
        var result = ShellExecute(IntPtr.Zero, "explore", target, null, null, 1).ToInt64();
        return result > 32 ? 0 : unchecked((int)(0x80070000u | (uint)(result == 0 ? 31 : result)));
    }

    // The child is a borrowed final SHITEMID inside the live absolute list, NOT
    // another absolute PIDL and NOT an allocation to free. Clone+remove yields
    // the exact parent Shell identity, including namespace/UNC paths.
    internal static int Select(string target, IExplorerPidlPlatform platform)
    {
        var absolute = platform.CreateAbsolute(target);
        if (absolute == IntPtr.Zero) return unchecked((int)0x80070002);
        try
        {
            var parent = platform.Clone(absolute);
            if (parent == IntPtr.Zero) return unchecked((int)0x8007000E);
            try
            {
                if (!platform.RemoveLast(parent)) return unchecked((int)0x80070057);
                var child = platform.FindLast(absolute);
                if (child == IntPtr.Zero) return unchecked((int)0x80070057);
                return platform.OpenAndSelect(parent, child);
            }
            finally { platform.Free(parent); }
        }
        finally { platform.Free(absolute); }
    }

    private sealed class NativePidlPlatform : IExplorerPidlPlatform
    {
        public IntPtr CreateAbsolute(string target) => ILCreateFromPath(target);
        public IntPtr Clone(IntPtr absolute) => ILClone(absolute);
        public bool RemoveLast(IntPtr parent) => ILRemoveLastID(parent);
        public IntPtr FindLast(IntPtr absolute) => ILFindLastID(absolute);
        public int OpenAndSelect(IntPtr parent, IntPtr child) => SHOpenFolderAndSelectItems(parent, 1, [child], 0);
        public void Free(IntPtr owned) => ILFree(owned);
    }

    [DllImport("shell32.dll", EntryPoint = "ILCreateFromPathW", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern IntPtr ILCreateFromPath(string path);
    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern IntPtr ILClone(IntPtr pidl);
    [DllImport("shell32.dll", ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ILRemoveLastID(IntPtr pidl);
    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern IntPtr ILFindLastID(IntPtr pidl);
    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern void ILFree(IntPtr pidl);
    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern int SHOpenFolderAndSelectItems(IntPtr parent, uint count,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 1)] IntPtr[] children, uint flags);
    [DllImport("shell32.dll", EntryPoint = "ShellExecuteW", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern IntPtr ShellExecute(IntPtr owner, string verb, string file, string? parameters, string? directory, int show);
}

// Owned only on the existing detached STA Shell worker. Both S_OK and S_FALSE
// increment COM's per-thread initialization count; failure must not uninitialize.
internal sealed class ExplorerShellApartment : IDisposable
{
    internal int Result { get; }
    private ExplorerShellApartment(int result) => Result = result;
    internal static ExplorerShellApartment Enter() => new(
        Thread.CurrentThread.GetApartmentState() == ApartmentState.STA
            ? CoInitializeEx(IntPtr.Zero, 2)
            : unchecked((int)0x80010106));
    public void Dispose()
    {
        if (Result >= 0) CoUninitialize();
    }
    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern int CoInitializeEx(IntPtr reserved, uint flags);
    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern void CoUninitialize();
}
