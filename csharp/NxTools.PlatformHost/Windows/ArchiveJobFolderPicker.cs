using System.Runtime.InteropServices;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record ArchiveFolderDialog(string InitialDirectory, uint Options)
{
    internal bool Multiselect => (Options & 0x200) != 0;
}

internal sealed class ArchivePickerLimitException : Exception { }

// The caller owns the bounded STA and cancellation registration through actual modal return.
internal sealed class ArchiveJobFolderPicker(
    Func<ArchiveFolderDialog, IntPtr, string[]?>? present = null)
{
    internal string[] Show(IntPtr owner, string initialDirectory, bool multiselect)
    {
        // FOS_NOCHANGEDIR | PICKFOLDERS | FORCEFILESYSTEM | PATHMUSTEXIST; only source roots allow many.
        var options = 0x8u | 0x20u | 0x40u | 0x800u | (multiselect ? 0x200u : 0u);
        return (present ?? Present)(new(initialDirectory, options), owner) ?? [];
    }

    private static string[]? Present(ArchiveFolderDialog settings, IntPtr owner)
    {
        var type = Type.GetTypeFromCLSID(new Guid("DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7"), throwOnError: true)!;
        var dialog = (IFileOpenDialog)Activator.CreateInstance(type)!;
        try
        {
            Marshal.ThrowExceptionForHR(dialog.GetOptions(out var options));
            Marshal.ThrowExceptionForHR(dialog.SetOptions((options & ~0x200u) | settings.Options));
            Marshal.ThrowExceptionForHR(dialog.SetTitle("选择文件夹"));
            if (settings.InitialDirectory.Length != 0)
            {
                var iid = typeof(IShellItem).GUID;
                Marshal.ThrowExceptionForHR(SHCreateItemFromParsingName(settings.InitialDirectory, IntPtr.Zero, in iid, out var initial));
                try { Marshal.ThrowExceptionForHR(dialog.SetFolder(initial)); }
                finally { Marshal.ReleaseComObject(initial); }
            }
            var shown = dialog.Show(owner);
            if (shown == unchecked((int)0x800704C7)) return null; // HRESULT_FROM_WIN32(ERROR_CANCELLED)
            Marshal.ThrowExceptionForHR(shown);
            Marshal.ThrowExceptionForHR(dialog.GetResults(out var items));
            try
            {
                Marshal.ThrowExceptionForHR(items.GetCount(out var count));
                if (count > ArchiveOwnedPickerResult.MaxPaths) throw new ArchivePickerLimitException();
                var paths = new string[count];
                for (uint index = 0; index < count; index++)
                {
                    Marshal.ThrowExceptionForHR(items.GetItemAt(index, out var item));
                    try
                    {
                        // SIGDN_FILESYSPATH: folders are filesystem roots, never display names as authority.
                        Marshal.ThrowExceptionForHR(item.GetDisplayName(0x80058000, out paths[index]));
                    }
                    finally { Marshal.ReleaseComObject(item); }
                }
                return paths;
            }
            finally { Marshal.ReleaseComObject(items); }
        }
        finally { Marshal.ReleaseComObject(dialog); }
    }

    // Windows SDK shobjidl_core.h order, including every inherited IModalWindow/IFileDialog slot.
    [ComImport, Guid("D57C7288-D4AD-4768-BE02-9D969532D960"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IFileOpenDialog
    {
        [PreserveSig] int Show(IntPtr owner);
        [PreserveSig] int SetFileTypes(uint count, IntPtr filters);
        [PreserveSig] int SetFileTypeIndex(uint index);
        [PreserveSig] int GetFileTypeIndex(out uint index);
        [PreserveSig] int Advise(IntPtr events, out uint cookie);
        [PreserveSig] int Unadvise(uint cookie);
        [PreserveSig] int SetOptions(uint options);
        [PreserveSig] int GetOptions(out uint options);
        [PreserveSig] int SetDefaultFolder(IShellItem folder);
        [PreserveSig] int SetFolder(IShellItem folder);
        [PreserveSig] int GetFolder(out IShellItem folder);
        [PreserveSig] int GetCurrentSelection(out IShellItem item);
        [PreserveSig] int SetFileName([MarshalAs(UnmanagedType.LPWStr)] string name);
        [PreserveSig] int GetFileName(out IntPtr name);
        [PreserveSig] int SetTitle([MarshalAs(UnmanagedType.LPWStr)] string title);
        [PreserveSig] int SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string label);
        [PreserveSig] int SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string label);
        [PreserveSig] int GetResult(out IShellItem item);
        [PreserveSig] int AddPlace(IShellItem item, uint placement);
        [PreserveSig] int SetDefaultExtension([MarshalAs(UnmanagedType.LPWStr)] string extension);
        [PreserveSig] int Close(int result);
        [PreserveSig] int SetClientGuid(in Guid client);
        [PreserveSig] int ClearClientData();
        [PreserveSig] int SetFilter(IntPtr filter);
        [PreserveSig] int GetResults(out IShellItemArray items);
    }

    [ComImport, Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItem
    {
        [PreserveSig] int BindToHandler(IntPtr bindContext, in Guid handler, in Guid iid, out IntPtr result);
        [PreserveSig] int GetParent(out IShellItem parent);
        [PreserveSig] int GetDisplayName(uint kind, [MarshalAs(UnmanagedType.LPWStr)] out string name);
        [PreserveSig] int GetAttributes(uint mask, out uint attributes);
        [PreserveSig] int Compare(IShellItem other, uint hint, out int order);
    }

    [ComImport, Guid("B63EA76D-1F85-456F-A19C-48159EFA858B"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItemArray
    {
        [PreserveSig] int BindToHandler(IntPtr bindContext, in Guid handler, in Guid iid, out IntPtr result);
        [PreserveSig] int GetPropertyStore(uint flags, in Guid iid, out IntPtr store);
        [PreserveSig] int GetPropertyDescriptionList(IntPtr key, in Guid iid, out IntPtr descriptions);
        [PreserveSig] int GetAttributes(uint flags, uint mask, out uint attributes);
        [PreserveSig] int GetCount(out uint count);
        [PreserveSig] int GetItemAt(uint index, out IShellItem item);
        [PreserveSig] int EnumItems(out IntPtr enumerator);
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern int SHCreateItemFromParsingName(string name, IntPtr bindContext, in Guid iid, out IShellItem item);
}
