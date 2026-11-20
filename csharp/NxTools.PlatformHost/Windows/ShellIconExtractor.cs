using System.Text;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ShellIconExtractor
{
    private const uint ShgfiIcon = 0x000000100;
    private const uint ShgfiLargeIcon = 0x000000000;
    private const uint ShgfiSmallIcon = 0x000000001;
    private const uint ShgfiUseFileAttributes = 0x000000010;
    private const uint ShgfiSystemIconIndex = 0x000004000;
    private const int ShilExtraLarge = 2;
    private const int ShilJumbo = 4;
    private const uint ImageListDrawTransparent = 0x00000001;
    private const uint FileAttributeNormal = 0x00000080;
    private const uint FileAttributeDirectory = 0x00000010;
    private const string FolderTypeTarget = "nxtools-folder-type:directory";
    private const string FileTypePrefix = "nxtools-file-type:.";
    private static readonly Guid ImageListInterfaceId = new("46EB5926-582E-4017-9FDF-E8998DAA0950");
    private static readonly Lazy<bool> ShellImageListWarmup = new(InitializeShellImageList,
        LazyThreadSafetyMode.ExecutionAndPublication);

    internal static bool ShellImageListInitialized =>
        ShellImageListWarmup.IsValueCreated && ShellImageListWarmup.Value;

    internal static WindowsCommandResult ExtractOnSta(
        ExtractShellIconCommand command,
        Func<ExtractShellIconCommand, WindowsCommandResult>? extractor = null)
        => ExtractOnSta(command, extractor, CancellationToken.None);

    internal static WindowsCommandResult ExtractOnSta(
        ExtractShellIconCommand command,
        CancellationToken cancellationToken)
        => ExtractOnSta(command, null, cancellationToken);

    internal static WindowsCommandResult ExtractOnSta(
        ExtractShellIconCommand command,
        Func<ExtractShellIconCommand, WindowsCommandResult>? extractor,
        CancellationToken cancellationToken)
    {
        var operation = extractor ?? (value => Extract(value, cancellationToken));
        if (!BoundedStaOperationRunner.TryRun(
                "NxTools.ShellIconExtraction",
                TimeSpan.FromSeconds(5),
                maxConcurrency: 8,
                operationCancellation =>
                {
                    operationCancellation.ThrowIfCancellationRequested();
                    WindowsCommandResult result;
                    try
                    {
                        result = operation(command);
                    }
                    catch (OperationCanceledException) when (operationCancellation.IsCancellationRequested)
                    {
                        throw;
                    }
                    catch (Exception exception)
                    {
                        result = new WindowsCommandResult(false, "shell_icon_extract_failed", Fatal: true,
                            NativeErrorCode: exception is System.ComponentModel.Win32Exception native
                                ? native.NativeErrorCode : exception.HResult);
                    }
                    operationCancellation.ThrowIfCancellationRequested();
                    return result;
                },
                out WindowsCommandResult? result,
                out var error,
                cancellationToken))
        {
            return new WindowsCommandResult(
                false,
                cancellationToken.IsCancellationRequested ? "shell_icon_cancelled" : "shell_icon_sta_unavailable",
                Fatal: true,
                NativeErrorCode: cancellationToken.IsCancellationRequested ? 995 :
                    error?.Contains("timed out", StringComparison.OrdinalIgnoreCase) == true ? 1460 : null);
        }
        return result ?? new WindowsCommandResult(false, "shell_icon_no_result");
    }

    public static WindowsCommandResult Extract(ExtractShellIconCommand command) =>
        Extract(command, CancellationToken.None);

    internal static WindowsCommandResult Extract(
        ExtractShellIconCommand command,
        CancellationToken cancellationToken,
        IconCache? cache = null,
        ShellItemImageFactory? imageFactory = null)
    {
        cancellationToken.ThrowIfCancellationRequested();
        if (command.Size is < 16 or > 256)
        {
            return new WindowsCommandResult(false, "shell_icon_size_unsupported", NativeErrorCode: 87);
        }
        var target = ResolveIconTarget(command.Target);
        cancellationToken.ThrowIfCancellationRequested();
        if (target is null)
        {
            return new WindowsCommandResult(false, "shell_icon_target_unavailable", NativeErrorCode: 2);
        }

        var targetType = target.IsNamespace ? "namespace" : target.UseFileAttributes
            ? (target.FileAttributes == FileAttributeDirectory ? "directory-type" : "file-type")
            : Directory.Exists(target.Path) ? "directory" : "file";
        // Namespace/attribute-only identities never probe filesystem paths or timestamps.
        var lastWriteTicks = !target.IsNamespace && !target.UseFileAttributes && (File.Exists(target.Path) || Directory.Exists(target.Path))
            ? File.GetLastWriteTimeUtc(target.Path).Ticks
            : 0;
        var key = IconCacheKey.Create(
            target.Path,
            target.IconIndex,
            command.Size,
            lastWriteTicks,
            !target.IsNamespace && target.IconIndex is null && command.Size > 32 ? targetType + "/image-list-v2" : targetType);
        string? extractionError = null;
        int? nativeErrorCode = null;
        var cacheHitOrCreated = (cache ?? IconCache.Shared).TryGetOrCreate(key, destination =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (target.IsNamespace)
            {
                var bitmap = (imageFactory ?? new ShellItemImageFactory()).Load(
                    target.Path, command.Size, cancellationToken, out var status);
                if (bitmap is null)
                {
                    nativeErrorCode = status;
                    extractionError = "shell_icon_query_failed";
                    return false;
                }
                try
                {
                    return WritePng(bitmap, destination, cancellationToken);
                }
                catch (ExternalException error)
                {
                    nativeErrorCode = error.ErrorCode;
                    extractionError = "shell_icon_png_encode_failed";
                    return false;
                }
            }
            var iconHandle = target.IconIndex is { } iconIndex
                ? ExtractIndexedIcon(target.Path, iconIndex, command.Size)
                : ExtractShellIcon(target, command.Size);
            if (iconHandle == IntPtr.Zero && !target.UseFileAttributes)
            {
                var extension = Path.GetExtension(target.Path);
                if (!string.IsNullOrWhiteSpace(extension))
                {
                    iconHandle = ExtractShellIcon(
                        new ShellIconTarget(extension, null, true, FileAttributeNormal),
                        command.Size);
                }
            }
            if (iconHandle == IntPtr.Zero)
            {
                nativeErrorCode = Marshal.GetLastWin32Error();
                extractionError = "shell_icon_query_failed";
                return false;
            }
            try
            {
                cancellationToken.ThrowIfCancellationRequested();
                var bitmap = System.Windows.Interop.Imaging.CreateBitmapSourceFromHIcon(
                    iconHandle,
                    System.Windows.Int32Rect.Empty,
                    System.Windows.Media.Imaging.BitmapSizeOptions.FromEmptyOptions());
                bitmap.Freeze();
                return WritePng(bitmap, destination, cancellationToken);
            }
            catch (ExternalException error)
            {
                nativeErrorCode = error.ErrorCode;
                extractionError = "shell_icon_png_encode_failed";
                return false;
            }
            finally
            {
                _ = DestroyIcon(iconHandle);
            }
        }, out var destinationPath, IsCachedPngValid);

        cancellationToken.ThrowIfCancellationRequested();
        if (cacheHitOrCreated)
        {
            return new WindowsCommandResult(true, Data: destinationPath);
        }
        return new WindowsCommandResult(false, extractionError ??
            "shell_icon_cache_failed", NativeErrorCode: nativeErrorCode);
    }

    private static bool WritePng(System.Windows.Media.Imaging.BitmapSource bitmap,
        string destination, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var encoder = new System.Windows.Media.Imaging.PngBitmapEncoder();
        encoder.Frames.Add(System.Windows.Media.Imaging.BitmapFrame.Create(bitmap));
        using var output = File.Create(destination);
        encoder.Save(output);
        cancellationToken.ThrowIfCancellationRequested();
        return true;
    }

    private static bool IsCachedPngValid(string path)
    {
        try
        {
            using var stream = File.OpenRead(path);
            var decoder = new System.Windows.Media.Imaging.PngBitmapDecoder(stream,
                System.Windows.Media.Imaging.BitmapCreateOptions.PreservePixelFormat,
                System.Windows.Media.Imaging.BitmapCacheOption.OnLoad);
            return decoder.Frames.Count == 1 && decoder.Frames[0].PixelWidth is > 0 and <= 256
                && decoder.Frames[0].PixelHeight is > 0 and <= 256;
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException
            or NotSupportedException or FileFormatException or ArgumentException or ExternalException)
        {
            return false;
        }
    }

    internal static uint ShellIconFlags(ShellIconTarget target, uint size) =>
        ShgfiIcon | (size <= 20 ? ShgfiSmallIcon : ShgfiLargeIcon)
        | (target.UseFileAttributes ? ShgfiUseFileAttributes : 0);

    internal static int ImageListSizeFor(uint size) => size <= 48 ? ShilExtraLarge : ShilJumbo;

    private static IntPtr ExtractShellIcon(ShellIconTarget target, uint size)
    {
        if (size > 32)
        {
            var highResolution = ExtractHighResolutionShellIcon(target, size);
            if (highResolution != IntPtr.Zero) return highResolution;
        }
        var flags = ShellIconFlags(target, size);
        var result = SHGetFileInfo(
            target.Path,
            target.FileAttributes,
            out var info,
            (uint)Marshal.SizeOf<ShFileInfo>(),
            flags);
        return result == IntPtr.Zero ? IntPtr.Zero : info.IconHandle;
    }

    internal static bool HasSmallArtworkOnJumboCanvas(System.Windows.Media.Imaging.BitmapSource bitmap)
    {
        if (bitmap.PixelWidth <= 48 || bitmap.PixelHeight <= 48) return false;
        var pixels = new byte[bitmap.PixelWidth * bitmap.PixelHeight * 4];
        var bgra = new System.Windows.Media.Imaging.FormatConvertedBitmap(bitmap,
            System.Windows.Media.PixelFormats.Bgra32, null, 0);
        bgra.CopyPixels(pixels, bitmap.PixelWidth * 4, 0);
        var left = bitmap.PixelWidth;
        var top = bitmap.PixelHeight;
        var right = -1;
        var bottom = -1;
        for (var y = 0; y < bitmap.PixelHeight; y++)
        for (var x = 0; x < bitmap.PixelWidth; x++)
        {
            if (pixels[(y * bitmap.PixelWidth + x) * 4 + 3] == 0) continue;
            left = Math.Min(left, x);
            right = Math.Max(right, x);
            top = Math.Min(top, y);
            bottom = Math.Max(bottom, y);
        }
        return right >= 0 && right - left + 1 <= 48 && bottom - top + 1 <= 48;
    }

    private static IntPtr ExtractHighResolutionShellIcon(ShellIconTarget target, uint size)
    {
        _ = ShellImageListWarmup.Value;
        var flags = ShgfiSystemIconIndex | (target.UseFileAttributes ? ShgfiUseFileAttributes : 0);
        if (SHGetFileInfo(target.Path, target.FileAttributes, out var info,
                (uint)Marshal.SizeOf<ShFileInfo>(), flags) == IntPtr.Zero)
            return IntPtr.Zero;
        return SelectImageListIcon(info.IconIndex, size, ExtractImageListIcon,
            HasSmallArtwork, icon => _ = DestroyIcon(icon));
    }

    internal static IntPtr SelectImageListIcon(int index, uint size,
        Func<int, int, IntPtr> extract, Func<IntPtr, bool> hasSmallArtwork, Action<IntPtr> destroy)
    {
        var listSize = ImageListSizeFor(size);
        var icon = extract(index, listSize);
        if (icon != IntPtr.Zero && listSize == ShilJumbo && hasSmallArtwork(icon))
        {
            var extraLarge = extract(index, ShilExtraLarge);
            if (extraLarge != IntPtr.Zero)
            {
                destroy(icon);
                return extraLarge;
            }
        }
        return icon;
    }

    private static bool HasSmallArtwork(IntPtr icon)
    {
        try
        {
            var bitmap = System.Windows.Interop.Imaging.CreateBitmapSourceFromHIcon(
                icon, System.Windows.Int32Rect.Empty,
                System.Windows.Media.Imaging.BitmapSizeOptions.FromEmptyOptions());
            return HasSmallArtworkOnJumboCanvas(bitmap);
        }
        catch (ExternalException) { return false; }
    }

    private static bool InitializeShellImageList()
    {
        if (SHGetFileInfo("nxtools-file", FileAttributeNormal, out var info,
                (uint)Marshal.SizeOf<ShFileInfo>(), ShgfiSystemIconIndex | ShgfiUseFileAttributes)
            == IntPtr.Zero)
            return false;
        var icon = ExtractImageListIcon(info.IconIndex, ShilExtraLarge);
        if (icon == IntPtr.Zero) return false;
        _ = DestroyIcon(icon);
        return true;
    }

    private static IntPtr ExtractImageListIcon(int index, int imageListSize)
    {
        var interfaceId = ImageListInterfaceId;
        var status = SHGetImageList(imageListSize, ref interfaceId, out var imageList);
        if (imageList == IntPtr.Zero) return IntPtr.Zero;
        try
        {
            if (status < 0) return IntPtr.Zero;
            // IUnknown has three slots; seven IImageList methods precede GetIcon.
            var table = Marshal.ReadIntPtr(imageList);
            var getIcon = Marshal.GetDelegateForFunctionPointer<ImageListGetIcon>(
                Marshal.ReadIntPtr(table, 10 * IntPtr.Size));
            var iconStatus = getIcon(imageList, index, ImageListDrawTransparent, out var icon);
            if (iconStatus >= 0) return icon;
            if (icon != IntPtr.Zero) _ = DestroyIcon(icon);
            return IntPtr.Zero;
        }
        finally
        {
            _ = Marshal.Release(imageList);
        }
    }

    private static IntPtr ExtractIndexedIcon(string target, int iconIndex, uint size)
    {
        var large = size > 20 ? new IntPtr[1] : null;
        var small = size <= 20 ? new IntPtr[1] : null;
        var count = ExtractIconEx(target, iconIndex, large, small, 1);
        return count == 0
            ? IntPtr.Zero
            : (large ?? small)?[0] ?? IntPtr.Zero;
    }

    internal static ShellIconTarget? ResolveIconTarget(string target)
    {
        // AppsFolder uses its full parsing name; invalid namespace targets never become file types.
        if (target.Trim().StartsWith("shell:", StringComparison.OrdinalIgnoreCase))
        {
            return PackagedApplicationTarget.IsValid(target)
                ? new ShellIconTarget(target, null, IsNamespace: true)
                : null;
        }
        // Reserved type targets are parsed before environment expansion, filesystem probes,
        // system-tool lookup or shortcut COM. Only fixed attributes reach SHGetFileInfo.
        var synthetic = target.Trim();
        if (synthetic == FolderTypeTarget)
        {
            return new ShellIconTarget("nxtools-folder", null, true, FileAttributeDirectory);
        }
        if (synthetic.StartsWith("nxtools-folder-type:", StringComparison.Ordinal))
        {
            return null;
        }
        if (synthetic == "nxtools-file-type:generic")
            return new ShellIconTarget("nxtools-file", null, true, FileAttributeNormal);
        if (synthetic.StartsWith("nxtools-file-type:", StringComparison.Ordinal))
        {
            if (!synthetic.StartsWith(FileTypePrefix, StringComparison.Ordinal)) return null;
            var typeExtension = synthetic[FileTypePrefix.Length..];
            return typeExtension.Length is >= 1 and <= 16 && typeExtension.All(char.IsAsciiLetterOrDigit)
                ? new ShellIconTarget("." + typeExtension.ToLowerInvariant(), null, true, FileAttributeNormal)
                : null;
        }
        var value = Environment.ExpandEnvironmentVariables(target.Trim());
        if (value.EndsWith(".lnk", StringComparison.OrdinalIgnoreCase) && File.Exists(value))
        {
            var shortcut = TryResolveShortcutIconTarget(value);
            if (shortcut is not null)
            {
                return shortcut;
            }
        }
        if (File.Exists(value) || Directory.Exists(value))
        {
            return new ShellIconTarget(value, null);
        }
        if (!ShellTarget.TryCreate(value, "system_tool", out _, out _))
        {
            var fileExtension = Path.GetExtension(value);
            return string.IsNullOrWhiteSpace(fileExtension)
                ? null
                : new ShellIconTarget(fileExtension, null, true, FileAttributeNormal);
        }
        var systemTarget = Path.Combine(Environment.SystemDirectory, value);
        if (File.Exists(systemTarget))
        {
            return new ShellIconTarget(systemTarget, null);
        }
        var windows = Environment.GetFolderPath(Environment.SpecialFolder.Windows);
        var windowsTarget = Path.Combine(windows, value);
        if (File.Exists(windowsTarget))
        {
            return new ShellIconTarget(windowsTarget, null);
        }
        var powershell = Path.Combine(
            Environment.SystemDirectory, "WindowsPowerShell", "v1.0", value);
        if (File.Exists(powershell))
        {
            return new ShellIconTarget(powershell, null);
        }
        var extension = Path.GetExtension(value);
        return string.IsNullOrWhiteSpace(extension)
            ? null
            : new ShellIconTarget(extension, null, true, FileAttributeNormal);
    }

    internal static string? ResolveNavigationDirectory(string target)
    {
        var value = Environment.ExpandEnvironmentVariables(target.Trim());
        if (string.IsNullOrWhiteSpace(value)
            || value.EndsWith(".url", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        var filesystemTarget = ResolveFilesystemTarget(value);
        if (filesystemTarget is null)
        {
            return null;
        }
        if (Directory.Exists(filesystemTarget))
        {
            return Path.GetFullPath(filesystemTarget);
        }
        if (!File.Exists(filesystemTarget))
        {
            return null;
        }
        var parent = Path.GetDirectoryName(Path.GetFullPath(filesystemTarget));
        return string.IsNullOrWhiteSpace(parent) ? null : parent;
    }

    internal static string? ResolveFilesystemTarget(string target)
    {
        var value = Environment.ExpandEnvironmentVariables(target.Trim());
        if (string.IsNullOrWhiteSpace(value))
        {
            return null;
        }
        if (value.EndsWith(".lnk", StringComparison.OrdinalIgnoreCase))
        {
            if (!File.Exists(value))
            {
                return null;
            }
            return TryResolveShortcut(value)?.TargetPath;
        }
        if (DirectoryNavigationTarget.TryResolve(value, out var resolvedDirectory))
        {
            return resolvedDirectory;
        }
        if (Directory.Exists(value) || File.Exists(value))
        {
            return value;
        }
        return ResolveSystemToolPath(value);
    }

    private static ShellIconTarget? TryResolveShortcutIconTarget(string shortcutPath)
    {
        var shortcut = TryResolveShortcut(shortcutPath);
        if (shortcut is null)
        {
            return null;
        }
        if (!string.IsNullOrWhiteSpace(shortcut.IconPath) && File.Exists(shortcut.IconPath))
        {
            return new ShellIconTarget(shortcut.IconPath, shortcut.IconIndex);
        }
        return shortcut.TargetPath is not null
            ? new ShellIconTarget(shortcut.TargetPath, null)
            : null;
    }

    private static ShortcutTarget? TryResolveShortcut(string shortcutPath)
    {
        object? shellLinkObject = null;
        try
        {
            shellLinkObject = new ShellLink();
            var shellLink = (IShellLinkW)shellLinkObject;
            ((IPersistFile)shellLinkObject).Load(shortcutPath, 0);

            var iconPath = new StringBuilder(1024);
            shellLink.GetIconLocation(iconPath, iconPath.Capacity, out var iconIndex);
            var expandedIconPath = Environment.ExpandEnvironmentVariables(iconPath.ToString());

            var targetPath = new StringBuilder(1024);
            shellLink.GetPath(targetPath, targetPath.Capacity, IntPtr.Zero, 0);
            var expandedTargetPath = Environment.ExpandEnvironmentVariables(targetPath.ToString());
            return new ShortcutTarget(
                File.Exists(expandedTargetPath) || Directory.Exists(expandedTargetPath)
                    ? expandedTargetPath
                    : null,
                !string.IsNullOrWhiteSpace(expandedIconPath) && File.Exists(expandedIconPath)
                    ? expandedIconPath
                    : null,
                iconIndex);
        }
        catch (COMException)
        {
            return null;
        }
        finally
        {
            if (shellLinkObject is not null)
            {
                _ = Marshal.FinalReleaseComObject(shellLinkObject);
            }
        }
    }

    private static string? ResolveSystemToolPath(string value)
    {
        if (!ShellTarget.TryCreate(value, "system_tool", out _, out _))
        {
            return null;
        }
        foreach (var candidate in new[]
                 {
                     Path.Combine(Environment.SystemDirectory, value),
                     Path.Combine(
                         Environment.GetFolderPath(Environment.SpecialFolder.Windows),
                         value),
                     Path.Combine(
                         Environment.SystemDirectory,
                         "WindowsPowerShell",
                         "v1.0",
                         value)
                 })
        {
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }
        return null;
    }

    [DllImport("shell32.dll", PreserveSig = true)]
    private static extern int SHGetImageList(
        int imageList,
        ref Guid interfaceId,
        out IntPtr result);

    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    private delegate int ImageListGetIcon(IntPtr imageList, int index, uint flags, out IntPtr icon);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr SHGetFileInfo(
        string path,
        uint fileAttributes,
        out ShFileInfo fileInfo,
        uint fileInfoSize,
        uint flags);

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern uint ExtractIconEx(
        string file,
        int iconIndex,
        [Out] IntPtr[]? largeIcons,
        [Out] IntPtr[]? smallIcons,
        uint iconCount);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool DestroyIcon(IntPtr icon);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct ShFileInfo
    {
        public IntPtr IconHandle;
        public int IconIndex;
        public uint Attributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)]
        public string DisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)]
        public string TypeName;
    }

    [ComImport]
    [Guid("00021401-0000-0000-C000-000000000046")]
    private sealed class ShellLink
    {
    }

    [ComImport]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    [Guid("000214F9-0000-0000-C000-000000000046")]
    private interface IShellLinkW
    {
        void GetPath([Out] StringBuilder file, int count, IntPtr findData, uint flags);
        void GetIDList(out IntPtr itemIdList);
        void SetIDList(IntPtr itemIdList);
        void GetDescription([Out] StringBuilder description, int count);
        void SetDescription(string description);
        void GetWorkingDirectory([Out] StringBuilder directory, int count);
        void SetWorkingDirectory(string directory);
        void GetArguments([Out] StringBuilder arguments, int count);
        void SetArguments(string arguments);
        void GetHotkey(out short hotkey);
        void SetHotkey(short hotkey);
        void GetShowCmd(out int showCommand);
        void SetShowCmd(int showCommand);
        void GetIconLocation([Out] StringBuilder iconPath, int count, out int iconIndex);
        void SetIconLocation(string iconPath, int iconIndex);
        void SetRelativePath(string path, uint reserved);
        void Resolve(IntPtr window, uint flags);
        void SetPath(string file);
    }
}

internal sealed record ShortcutTarget(string? TargetPath, string? IconPath, int IconIndex);

internal sealed record ShellIconTarget(
    string Path,
    int? IconIndex,
    bool UseFileAttributes = false,
    uint FileAttributes = 0,
    bool IsNamespace = false);
