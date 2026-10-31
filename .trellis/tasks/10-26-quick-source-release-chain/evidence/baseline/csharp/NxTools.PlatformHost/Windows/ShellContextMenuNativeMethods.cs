using System.Runtime.InteropServices;
using System.Text;

namespace NxTools.PlatformHost.Windows;

internal static class ShellContextMenuNativeMethods
{
    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214E4-0000-0000-C000-000000000046")]
    internal interface IContextMenu
    {
        [PreserveSig]
        int QueryContextMenu(IntPtr menu, uint index, uint firstCommand, uint lastCommand, uint flags);

        [PreserveSig]
        int InvokeCommand(ref CommandInfo command);

        [PreserveSig]
        int GetCommandString(
            UIntPtr command,
            uint flags,
            IntPtr reserved,
            StringBuilder name,
            uint maximumCharacters);
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214F4-0000-0000-C000-000000000046")]
    internal interface IContextMenu2
    {
        [PreserveSig]
        int QueryContextMenu(IntPtr menu, uint index, uint firstCommand, uint lastCommand, uint flags);

        [PreserveSig]
        int InvokeCommand(ref CommandInfo command);

        [PreserveSig]
        int GetCommandString(
            UIntPtr command,
            uint flags,
            IntPtr reserved,
            StringBuilder name,
            uint maximumCharacters);

        [PreserveSig]
        int HandleMenuMessage(uint message, IntPtr wParam, IntPtr lParam);
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("BCFCE0A0-EC17-11D0-8D10-00A0C90F2719")]
    internal interface IContextMenu3
    {
        [PreserveSig]
        int QueryContextMenu(IntPtr menu, uint index, uint firstCommand, uint lastCommand, uint flags);

        [PreserveSig]
        int InvokeCommand(ref CommandInfo command);

        [PreserveSig]
        int GetCommandString(
            UIntPtr command,
            uint flags,
            IntPtr reserved,
            StringBuilder name,
            uint maximumCharacters);

        [PreserveSig]
        int HandleMenuMessage(uint message, IntPtr wParam, IntPtr lParam);

        [PreserveSig]
        int HandleMenuMessage2(uint message, IntPtr wParam, IntPtr lParam, out IntPtr result);
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214E6-0000-0000-C000-000000000046")]
    internal interface IShellFolder
    {
        void ParseDisplayName(
            IntPtr owner,
            IntPtr bindContext,
            [MarshalAs(UnmanagedType.LPWStr)] string displayName,
            out uint eaten,
            out IntPtr item,
            ref uint attributes);

        void EnumObjects(IntPtr owner, uint flags, out IntPtr items);
        void BindToObject(IntPtr item, IntPtr bindContext, [In] ref Guid id, out IntPtr result);
        void BindToStorage(IntPtr item, IntPtr bindContext, [In] ref Guid id, out IntPtr result);
        void CompareIDs(IntPtr parameter, IntPtr first, IntPtr second);
        void CreateViewObject(IntPtr owner, [In] ref Guid id, out IntPtr result);
        void GetAttributesOf(
            uint count,
            [MarshalAs(UnmanagedType.LPArray)] IntPtr[] items,
            ref uint attributes);

        void GetUIObjectOf(
            IntPtr owner,
            uint count,
            [MarshalAs(UnmanagedType.LPArray)] IntPtr[] items,
            [In] ref Guid id,
            ref uint reserved,
            out IntPtr result);

        void GetDisplayNameOf(IntPtr item, uint flags, out StringReturn name);
        void SetNameOf(
            IntPtr owner,
            IntPtr item,
            [MarshalAs(UnmanagedType.LPWStr)] string name,
            uint flags,
            out IntPtr renamedItem);
    }

    [StructLayout(LayoutKind.Sequential)]
    internal struct CommandInfo
    {
        internal int Size;
        internal int Mask;
        internal IntPtr Owner;
        internal IntPtr Verb;
        internal IntPtr Parameters;
        internal IntPtr Directory;
        internal int Show;
        internal int HotKey;
        internal IntPtr Icon;
    }

    [StructLayout(LayoutKind.Explicit, Size = 264)]
    internal struct StringReturn
    {
        [FieldOffset(0)] internal uint Type;
        [FieldOffset(4)] internal IntPtr OleString;
        [FieldOffset(4)] internal uint Offset;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    internal struct MenuItemInfo
    {
        internal uint Size;
        internal uint Mask;
        internal uint Type;
        internal uint State;
        internal uint CommandId;
        internal IntPtr SubMenu;
        internal IntPtr CheckedBitmap;
        internal IntPtr UncheckedBitmap;
        internal UIntPtr ItemData;
        internal IntPtr Text;
        internal uint TextLength;
        internal IntPtr ItemBitmap;
    }

    internal const uint MenuItemState = 0x00000001;
    internal const uint MenuItemId = 0x00000002;
    internal const uint MenuItemSubMenu = 0x00000004;
    internal const uint MenuItemString = 0x00000040;
    internal const uint MenuItemBitmap = 0x00000080;
    internal const uint MenuItemType = 0x00000100;
    internal const uint MenuTypeSeparator = 0x00000800;
    internal const uint MenuStateDisabled = 0x00000003;
    internal const uint InitializeMenuPopup = 0x0117;

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    internal static extern int SHParseDisplayName(
        [MarshalAs(UnmanagedType.LPWStr)] string name,
        IntPtr bindContext,
        out IntPtr item,
        uint attributesIn,
        out uint attributesOut);

    [DllImport("shell32.dll")]
    internal static extern int SHBindToParent(
        IntPtr item,
        [In] ref Guid id,
        out IntPtr parent,
        ref IntPtr relativeItem);

    [DllImport("user32.dll", SetLastError = true)]
    internal static extern IntPtr CreatePopupMenu();

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool DestroyMenu(IntPtr menu);

    [DllImport("user32.dll")]
    internal static extern int GetMenuItemCount(IntPtr menu);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool GetMenuItemInfo(
        IntPtr menu,
        uint item,
        [MarshalAs(UnmanagedType.Bool)] bool byPosition,
        ref MenuItemInfo info);

    [DllImport("gdi32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static extern bool DeleteObject(IntPtr value);

    [DllImport("ole32.dll")]
    internal static extern int OleInitialize(IntPtr reserved);

    [DllImport("ole32.dll")]
    internal static extern void OleUninitialize();

    [DllImport("ole32.dll")]
    internal static extern void CoTaskMemFree(IntPtr memory);
}
