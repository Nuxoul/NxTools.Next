using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Deterministic red-capable ABI evidence for the Win32 wrapper, not an alternative publisher.
internal static class ArchiveRenameAbiTests
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct RenameInfo
    { internal uint Flags; internal IntPtr Root; internal uint NameLength; internal char Name; }
    [DllImport("kernel32.dll", EntryPoint = "SetFileInformationByHandle", ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetInfo(SafeFileHandle source, int infoClass, IntPtr info, uint size);
    public static Task CountedNameMustStillBeTerminated()
    {
        var root = Directory.CreateTempSubdirectory("archive-overwrite-abi-");
        try {
            if (Marshal.OffsetOf<RenameInfo>(nameof(RenameInfo.Root)).ToInt32() != 8 ||
                Marshal.OffsetOf<RenameInfo>(nameof(RenameInfo.NameLength)).ToInt32() != 16 ||
                Marshal.OffsetOf<RenameInfo>(nameof(RenameInfo.Name)).ToInt32() != 20)
                throw new InvalidOperationException("SDK x64 FILE_RENAME_INFO ABI differs");
            foreach (var terminated in new[] { false, true }) {
                var directory = Directory.CreateDirectory(Path.Combine(root.FullName, Guid.NewGuid().ToString("N")));
                var target = Path.Combine(directory.FullName, "target"); File.WriteAllText(target, "old");
                var temp = Path.Combine(directory.FullName, "source"); File.WriteAllText(temp, "complete");
                using var source = OutputNative.Open(null, @"\??\" + temp, directory: false);
                var name = Encoding.Unicode.GetBytes(target);
                var payload = Encoding.Unicode.GetBytes(target + (terminated ? "\0" : "") + "TAIL\0");
                var block = Marshal.AllocHGlobal(20 + payload.Length);
                try {
                    Marshal.Copy(new byte[20 + payload.Length], 0, block, 20 + payload.Length);
                    Marshal.StructureToPtr(new RenameInfo { Flags = 3, Root = IntPtr.Zero, NameLength = (uint)name.Length }, block, false);
                    Marshal.Copy(payload, 0, IntPtr.Add(block, 20), payload.Length);
                    if (!SetInfo(source, 22, block, (uint)(20 + payload.Length)))
                        throw new InvalidOperationException("owned Win32 ABI probe:" + Marshal.GetLastWin32Error());
                }
                finally { Marshal.FreeHGlobal(block); }
                source.Dispose();
                if (File.ReadAllText(target) != (terminated ? "complete" : "old") || File.Exists(target + "TAIL") != !terminated)
                    throw new InvalidOperationException("Win32 counted-name termination evidence differs");
            }
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }
}
