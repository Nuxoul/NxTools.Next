using System.Runtime.InteropServices;

namespace NxTools.PlatformHost.Windows;

/// <summary>
/// Resolves localized display paths back to real parsing paths. Directory Opus
/// renders profile folders in their localized display form (for example
/// C:\用户\Administrator\桌面 on a zh-CN system); those strings are not valid
/// file system paths, so every downstream Directory.Exists check and
/// navigation rejects them. Known folders are resolved per segment through
/// their shell display names.
/// </summary>
internal static class ThirdPartyFileManagerDisplayPath
{
    public static string? Canonicalize(
        string? path,
        Func<string, bool> directoryExists,
        IReadOnlyDictionary<string, string> displayAliases)
    {
        if (string.IsNullOrWhiteSpace(path))
        {
            return null;
        }
        if (directoryExists(path))
        {
            return path;
        }
        if (!Path.IsPathRooted(path))
        {
            return null;
        }
        var root = Path.GetPathRoot(path);
        if (string.IsNullOrEmpty(root))
        {
            return null;
        }
        var current = root;
        var replaced = false;
        foreach (var segment in path[root.Length..].Split(
                     ['\\', '/'],
                     StringSplitOptions.RemoveEmptyEntries))
        {
            var candidate = Path.Join(current, segment);
            if (directoryExists(candidate))
            {
                current = candidate;
                continue;
            }
            if (displayAliases.TryGetValue(segment, out var real) &&
                directoryExists(real))
            {
                current = real;
                replaced = true;
                continue;
            }
            return null;
        }
        return replaced ? current : path;
    }

    public static IReadOnlyDictionary<string, string> KnownFolderDisplayAliases() =>
        KnownFolderAliases.Value;

    private static readonly Lazy<IReadOnlyDictionary<string, string>> KnownFolderAliases =
        new(BuildKnownFolderDisplayAliases);

    private static IReadOnlyDictionary<string, string> BuildKnownFolderDisplayAliases()
    {
        var aliases = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var folder in new[]
                 {
                     KnownFolderIds.UsersProfile,
                     KnownFolderIds.Profile,
                     KnownFolderIds.Desktop,
                     KnownFolderIds.Documents,
                     KnownFolderIds.Downloads,
                     KnownFolderIds.Pictures,
                     KnownFolderIds.Music,
                     KnownFolderIds.Videos,
                 })
        {
            var real = GetKnownFolderPath(folder);
            if (string.IsNullOrWhiteSpace(real) || !Directory.Exists(real))
            {
                continue;
            }
            AddKnownFolderAliases(aliases, real);
        }

        var profile = Environment.GetFolderPath(
            Environment.SpecialFolder.UserProfile);
        AddKnownFolderAliases(
            aliases,
            Directory.GetParent(profile)?.FullName);
        return aliases;
    }

    private static void AddKnownFolderAliases(
        Dictionary<string, string> aliases,
        string? real)
    {
        if (string.IsNullOrWhiteSpace(real) || !Directory.Exists(real))
        {
            return;
        }
        AddAlias(aliases, GetShellItemDisplayName(real), real);
        AddAlias(aliases, GetLocalizedFolderDisplayName(real), real);
    }

    private static void AddAlias(
        Dictionary<string, string> aliases,
        string? display,
        string real)
    {
        if (string.IsNullOrWhiteSpace(display))
        {
            return;
        }
        aliases[display] = real;
        var displaySegment = Path.GetFileName(display.TrimEnd('\\', '/'));
        if (!string.IsNullOrWhiteSpace(displaySegment))
        {
            aliases[displaySegment] = real;
        }
    }


    internal static string? GetLocalizedFolderDisplayName(string realPath)
    {
        if (string.IsNullOrWhiteSpace(realPath))
        {
            return null;
        }
        try
        {
            var info = new ShellFileInfo
            {
                DisplayName = string.Empty,
                TypeName = string.Empty
            };
            var result = SHGetFileInfo(
                realPath,
                0,
                ref info,
                (uint)Marshal.SizeOf<ShellFileInfo>(),
                ShgfiDisplayName);
            return result == IntPtr.Zero ||
                string.IsNullOrWhiteSpace(info.DisplayName)
                ? null
                : info.DisplayName.Trim();
        }
        catch
        {
            return null;
        }
    }

    private static string? GetKnownFolderPath(in Guid folderId)
    {
        var result = SHGetKnownFolderPath(folderId, 0, IntPtr.Zero, out var path);
        if (result != 0)
        {
            return null;
        }
        try
        {
            return Marshal.PtrToStringUni(path);
        }
        finally
        {
            Marshal.FreeCoTaskMem(path);
        }
    }

    internal static string? GetShellItemDisplayName(string realPath)
    {
        var shellItemId = new Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe");
        var result = SHCreateItemFromParsingName(
            realPath,
            IntPtr.Zero,
            shellItemId,
            out var item);
        if (result != 0 || item == IntPtr.Zero)
        {
            return null;
        }
        try
        {
            // IShellItem::GetDisplayName sits at vtable slot 5 behind IUnknown.
            var vtbl = Marshal.ReadIntPtr(item);
            var getDisplayName = Marshal.ReadIntPtr(vtbl, 5 * IntPtr.Size);
            var invoke = Marshal.GetDelegateForFunctionPointer<GetDisplayNameDelegate>(
                getDisplayName);
            var displayName = invoke(item, SigdnNormalDisplay, out var buffer);
            if (displayName != 0)
            {
                return null;
            }
            try
            {
                return Marshal.PtrToStringUni(buffer);
            }
            finally
            {
                Marshal.FreeCoTaskMem(buffer);
            }
        }
        finally
        {
            Marshal.Release(item);
        }
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)]
    private struct ShellFileInfo
    {
        public IntPtr Icon;
        public int IconIndex;
        public uint Attributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
        public string DisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)]
        public string TypeName;
    }

    private const uint ShgfiDisplayName = 0x000000200;

    [DllImport("shell32.dll", CharSet = CharSet.Auto)]
    private static extern IntPtr SHGetFileInfo(
        string path,
        uint fileAttributes,
        ref ShellFileInfo fileInfo,
        uint fileInfoSize,
        uint flags);

    // SIGDN_NORMALDISPLAY: the localized child name (e.g. 桌面 for Desktop on
    // a zh-CN system), which is exactly the per-segment form Directory Opus
    // renders in its friendly path bar.
    private const int SigdnNormalDisplay = 0;

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHGetKnownFolderPath(
        in Guid folderId,
        uint flags,
        IntPtr token,
        out IntPtr path);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHCreateItemFromParsingName(
        string path,
        IntPtr bindContext,
        in Guid riid,
        out IntPtr shellItem);

    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    private delegate int GetDisplayNameDelegate(
        IntPtr shellItem,
        int sigdnName,
        out IntPtr displayName);

    private static class KnownFolderIds
    {
        public static readonly Guid UsersProfile =
            new("0762D272-C50A-4BB0-A382-697DCD2939BB");
        public static readonly Guid Profile =
            new("5E6C858F-0E22-4760-9AFE-EA3317B67173");
        public static readonly Guid Desktop =
            new("B4BFCC3A-DB2C-424C-B029-7FE99A87C641");
        public static readonly Guid Documents =
            new("FDD39AD0-238F-46AF-ADB4-6C85480369C7");
        public static readonly Guid Downloads =
            new("374DE290-123F-4565-9164-39C4925E467B");
        public static readonly Guid Pictures =
            new("33E28130-4E1E-4676-835A-98395C3BC3BB");
        public static readonly Guid Music =
            new("4BD8D571-6D19-48D3-BE97-422220080E43");
        public static readonly Guid Videos =
            new("18989B1D-99B5-455B-841C-AB7C74E4DDFC");
    }
}
