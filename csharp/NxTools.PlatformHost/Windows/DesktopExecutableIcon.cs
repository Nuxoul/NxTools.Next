namespace NxTools.PlatformHost.Windows;

using System.ComponentModel;
using System.Drawing;
using System.Runtime.InteropServices;

internal static class DesktopExecutableIcon
{
    private const uint LoadLibraryAsDataFile = 0x00000002;
    private const uint LoadLibraryAsImageResource = 0x00000020;
    private const int ResourceTypeGroupIcon = 14;
    internal const int ErrorResourceTypeNotFound = 1813;

    internal static Icon ExtractProductIcon(string desktopExecutable)
    {
        if (!File.Exists(desktopExecutable))
        {
            throw new FileNotFoundException(
                "The NxTools desktop executable was not found.",
                desktopExecutable);
        }

        var module = LoadLibraryEx(
            desktopExecutable,
            IntPtr.Zero,
            LoadLibraryAsDataFile | LoadLibraryAsImageResource);
        if (module == IntPtr.Zero)
        {
            throw new Win32Exception(
                Marshal.GetLastWin32Error(),
                "Windows could not inspect the NxTools desktop icon resources.");
        }

        try
        {
            var found = false;
            EnumResourceNames(
                module,
                new IntPtr(ResourceTypeGroupIcon),
                (_, _, _, _) =>
                {
                    found = true;
                    return false;
                },
                IntPtr.Zero);
            if (!found)
            {
                var nativeError = Marshal.GetLastWin32Error();
                throw new Win32Exception(
                    nativeError == 0 ? ErrorResourceTypeNotFound : nativeError,
                    "The NxTools desktop executable does not expose a product icon resource.");
            }

            return Icon.ExtractAssociatedIcon(desktopExecutable)
                ?? throw new Win32Exception(
                    ErrorResourceTypeNotFound,
                    "Windows could not extract the NxTools desktop product icon.");
        }
        finally
        {
            _ = FreeLibrary(module);
        }
    }

    private delegate bool EnumResourceNameCallback(
        IntPtr module,
        IntPtr resourceType,
        IntPtr resourceName,
        IntPtr parameter);

    [DllImport("kernel32.dll", EntryPoint = "LoadLibraryExW", SetLastError = true,
        CharSet = CharSet.Unicode)]
    private static extern IntPtr LoadLibraryEx(
        string fileName,
        IntPtr file,
        uint flags);

    [DllImport("kernel32.dll", EntryPoint = "EnumResourceNamesW", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumResourceNames(
        IntPtr module,
        IntPtr resourceType,
        EnumResourceNameCallback callback,
        IntPtr parameter);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FreeLibrary(IntPtr module);
}
