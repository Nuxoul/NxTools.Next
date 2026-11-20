using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

// Windows Shell SDK interfaces; all RCWs, property values and display-name memory stay on the caller STA.
internal static class AppsFolderApplicationSource
{
    private static readonly Guid AppsFolder = new("1E87508D-89C2-42F0-8A7E-645A0F50CA58");
    private static readonly Guid EnumItemsHandler = new("94F60519-2850-4924-AA5A-D15E84868039");
    private static readonly Guid PropertyStoreHandler = new("0384E1A4-1523-439C-A4C8-AB911052F586");
    private static readonly PropertyKey PackageFamily = new(new("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), 17);
    private static readonly PropertyKey AppUserModelId = new(new("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), 5);

    internal static IEnumerable<InstalledApplicationFacts> Enumerate(CancellationToken cancellationToken)
    {
        if (Thread.CurrentThread.GetApartmentState() != ApartmentState.STA)
            throw new InvalidOperationException("AppsFolder enumeration requires STA ownership.");
        IShellItem? folder = null;
        object? enumeration = null;
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var folderId = AppsFolder;
            var itemId = typeof(IShellItem).GUID;
            Check(SHGetKnownFolderItem(ref folderId, 0, IntPtr.Zero, ref itemId, out folder));
            if (folder is null) throw InvalidNativeResult();
            var handlerId = EnumItemsHandler;
            var enumId = typeof(IEnumShellItems).GUID;
            Check(folder.BindToHandler(IntPtr.Zero, ref handlerId, ref enumId, out enumeration));
            var items = enumeration as IEnumShellItems ?? throw InvalidNativeResult();
            while (true)
            {
                cancellationToken.ThrowIfCancellationRequested();
                IShellItem? item = null;
                try
                {
                    var status = items.Next(1, out item, out var fetched);
                    Check(status);
                    if (status == 1 && fetched == 0 && item is null) yield break;
                    if (status != 0 || fetched != 1 || item is null) throw InvalidNativeResult();
                    yield return ReadFacts(item);
                }
                finally { Release(item); }
            }
        }
        finally
        {
            try { Release(enumeration); }
            finally { Release(folder); }
        }
    }

    private static InstalledApplicationFacts ReadFacts(IShellItem item)
    {
        object? properties = null;
        try
        {
            var handler = PropertyStoreHandler;
            var interfaceId = typeof(IPropertyStore).GUID;
            Check(item.BindToHandler(IntPtr.Zero, ref handler, ref interfaceId, out properties));
            var store = properties as IPropertyStore ?? throw InvalidNativeResult();
            var family = ReadString(store, PackageFamily);
            if (string.IsNullOrWhiteSpace(family)) return new(family, null, null);
            var aumid = ReadString(store, AppUserModelId);
            if (string.IsNullOrWhiteSpace(aumid)) return new(family, aumid, null);
            IntPtr displayName = IntPtr.Zero;
            try
            {
                Check(item.GetDisplayName(0, out displayName)); // SIGDN_NORMALDISPLAY: localized Shell name.
                return new(family, aumid, Marshal.PtrToStringUni(displayName));
            }
            finally { Marshal.FreeCoTaskMem(displayName); }
        }
        finally { Release(properties); }
    }

    private static string? ReadString(IPropertyStore store, PropertyKey key)
    {
        var value = new PropVariant();
        try
        {
            Check(store.GetValue(ref key, out value));
            return value.Type switch
            {
                31 => Marshal.PtrToStringUni(value.Pointer), // VT_LPWSTR
                8 when value.Pointer != IntPtr.Zero => Marshal.PtrToStringBSTR(value.Pointer), // VT_BSTR
                _ => null
            };
        }
        finally { Check(PropVariantClear(ref value)); }
    }

    private static void Check(int status) => Marshal.ThrowExceptionForHR(status);
    private static COMException InvalidNativeResult() => new("AppsFolder returned an invalid native result.", unchecked((int)0x8000FFFF));
    private static void Release(object? value)
    {
        if (value is not null && Marshal.IsComObject(value)) Marshal.FinalReleaseComObject(value);
    }

    [DllImport("shell32.dll", ExactSpelling = true)]
    private static extern int SHGetKnownFolderItem(ref Guid folderId, uint flags, IntPtr token, ref Guid interfaceId,
        [MarshalAs(UnmanagedType.Interface)] out IShellItem? item);

    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern int PropVariantClear(ref PropVariant value);

    [StructLayout(LayoutKind.Sequential)]
    private readonly struct PropertyKey(Guid formatId, uint propertyId)
    {
        private readonly Guid _formatId = formatId;
        private readonly uint _propertyId = propertyId;
    }

    [StructLayout(LayoutKind.Explicit, Size = 24)]
    private struct PropVariant
    {
        [FieldOffset(0)] internal ushort Type;
        [FieldOffset(8)] internal IntPtr Pointer;
    }

    [ComImport, Guid("43826D1E-E718-42EE-BC55-A1E261C37BFE"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellItem
    {
        [PreserveSig] int BindToHandler(IntPtr context, ref Guid handlerId, ref Guid interfaceId,
            [MarshalAs(UnmanagedType.Interface)] out object? result);
        [PreserveSig] int GetParent(out IShellItem parent);
        [PreserveSig] int GetDisplayName(uint kind, out IntPtr name);
        [PreserveSig] int GetAttributes(uint mask, out uint attributes);
        [PreserveSig] int Compare(IShellItem other, uint hint, out int order);
    }

    [ComImport, Guid("70629033-E363-4A28-A567-0DB78006E6D7"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IEnumShellItems
    {
        [PreserveSig] int Next(uint count, [MarshalAs(UnmanagedType.Interface)] out IShellItem? item, out uint fetched);
        [PreserveSig] int Skip(uint count);
        [PreserveSig] int Reset();
        [PreserveSig] int Clone(out IEnumShellItems copy);
    }

    [ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IPropertyStore
    {
        [PreserveSig] int GetCount(out uint count);
        [PreserveSig] int GetAt(uint index, out PropertyKey key);
        [PreserveSig] int GetValue(ref PropertyKey key, out PropVariant value);
        [PreserveSig] int SetValue(ref PropertyKey key, ref PropVariant value);
        [PreserveSig] int Commit();
    }
}
