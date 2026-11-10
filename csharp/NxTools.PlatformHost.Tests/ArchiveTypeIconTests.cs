using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: fixed-target parsing, bounded owned STA, Windows type/completed-file icon
// queries, and one GUID-owned file/cache. Never IconCache.Shared, member paths, Shell open or .lnk COM.
internal static class ArchiveTypeIconTests
{
    internal static Task FixedFolderTargetHasDirectoryAttributes()
    {
        var folder = ShellIconExtractor.ResolveIconTarget("nxtools-folder-type:directory")
            ?? throw new InvalidOperationException("Fixed directory type target must resolve.");
        Assert(folder is { UseFileAttributes: true, FileAttributes: 0x10, IconIndex: null },
            "Folder type must resolve without a real archive member directory.");
        Assert(folder.Path == "nxtools-folder", "Only the fixed non-member placeholder reaches Shell.");
        Assert((ShellIconExtractor.ShellIconFlags(folder, 32) & 0x10) != 0,
            "Native SHGetFileInfo must use SHGFI_USEFILEATTRIBUTES.");
        var extension = ShellIconExtractor.ResolveIconTarget("nxtools-file-type:.lnk");
        Assert(extension is { UseFileAttributes: true, FileAttributes: 0x80, IconIndex: null, Path: ".lnk" },
            "Existing extension-only requests stay types, never shortcut resolution.");
        foreach (var invalid in new[]
        {
            "nxtools-folder-type:directory/member.lnk",
            "nxtools-folder-type:directory\\member.exe",
            "nxtools-folder-type:other",
            "nxtools-file-type:.bad-ext",
            "nxtools-file-type:.exe/member.lnk",
            "nxtools-file-type:.12345678901234567"
        })
        {
            Assert(ShellIconExtractor.ResolveIconTarget(invalid) is null,
                "Malformed reserved targets must fail closed, not fall through to filesystem/Shell parsing.");
        }
        return Task.CompletedTask;
    }

    internal static Task FolderPngUsesOwnedCacheAndOriginalSta()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-folder-icon-{Guid.NewGuid():N}");
        try
        {
            var cache = new IconCache(root);
            var command = new ExtractShellIconCommand("nxtools-folder-type:directory", 32);
            var first = ShellIconExtractor.ExtractOnSta(command,
                value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
            Assert(first.Success && first.Data is not null, "Windows directory type returns cached PNG.");
            Assert(Path.GetDirectoryName(first.Data!) == root, "Never writes production IconCache.Shared.");
            var png = File.ReadAllBytes(first.Data!);
            Assert(png.Length > 8 && png.AsSpan(0, 8).SequenceEqual(new byte[] { 137, 80, 78, 71, 13, 10, 26, 10 }),
                "Native HICON conversion produces actual PNG bytes.");
            var second = ShellIconExtractor.ExtractOnSta(command,
                value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
            Assert(second.Success && second.Data == first.Data && cache.MemoryEntryCount == 1,
                "Every archive folder reuses the single native cache resource.");
            Assert(Directory.GetFiles(root).Length == 1, "No archive extraction or extra member output.");
            using var cancellation = new CancellationTokenSource();
            cancellation.Cancel();
            try
            {
                ShellIconExtractor.Extract(command, cancellation.Token, cache);
                throw new InvalidOperationException("Cancelled extraction must not run.");
            }
            catch (OperationCanceledException) { }
            var completedPath = Path.Combine(root, "completed.zip");
            File.WriteAllBytes(completedPath,
                [0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            var legacyKey = IconCacheKey.Create(".zip", null, 96, 0, "file-type");
            Assert(cache.TryGetOrCreate(legacyKey, path =>
            {
                File.WriteAllBytes(path, png);
                return true;
            }, out var legacyPath), "Owned legacy low-resolution cache fixture is available.");
            foreach (var size in new uint[] { 48, 96 })
            {
                foreach (var target in new[] { "nxtools-folder-type:directory", "nxtools-file-type:.zip", completedPath })
                {
                    var largeCommand = new ExtractShellIconCommand(target, size);
                    var large = ShellIconExtractor.ExtractOnSta(largeCommand,
                        value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
                    Assert(large.Success && large.Data is not null, "High-resolution type query returns PNG.");
                    using var imageStream = File.OpenRead(large.Data!);
                    var image = new System.Windows.Media.Imaging.PngBitmapDecoder(imageStream,
                        System.Windows.Media.Imaging.BitmapCreateOptions.PreservePixelFormat,
                        System.Windows.Media.Imaging.BitmapCacheOption.OnLoad).Frames[0];
                    Assert(image.PixelWidth >= size && image.PixelHeight >= size,
                        $"Requested {size}px type icon must not enlarge a {image.PixelWidth}x{image.PixelHeight}px Shell bitmap.");
                    if (target == "nxtools-file-type:.zip" && size == 96)
                        Assert(large.Data != legacyPath, "Large Shell icons must not reuse the legacy low-resolution cache key.");
                    Assert(image.PixelWidth <= 256 && image.PixelHeight <= 256, "Native icon pixels remain bounded.");
                    Console.WriteLine($"ICON requested={size} actual={image.PixelWidth}x{image.PixelHeight}");
                    var reused = ShellIconExtractor.ExtractOnSta(largeCommand,
                        value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
                    Assert(reused.Success && reused.Data == large.Data, "Large type icons reuse their bounded cache entry.");
                }
            }
        }
        finally
        {
            if (Directory.Exists(root)) Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    internal static async Task ParallelPngExtractionKeepsNativeImageListReferences()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-parallel-icon-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var cache = new IconCache(Path.Combine(root, "icons"));
            using var lanes = new SemaphoreSlim(8);
            var tasks = Enumerable.Range(0, 48).Select(async index =>
            {
                await lanes.WaitAsync();
                try
                {
                    return await Task.Run(() =>
                    {
                        var source = Path.Combine(root, $"completed-{index}.zip");
                        File.WriteAllBytes(source, [0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                        return ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(source, 48),
                            value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
                    });
                }
                finally { lanes.Release(); }
            });
            var results = await Task.WhenAll(tasks);
            Assert(results.All(result => result.Success && result.Data is not null),
                $"Every actual parallel Shell query must retain its own image-list interface; failures={results.Count(result => !result.Success)}.");
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    internal static Task CorruptPngRegeneratesThroughOriginalSta()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-icon-recovery-{Guid.NewGuid():N}");
        try
        {
            var command = new ExtractShellIconCommand("nxtools-file-type:.zip", 48);
            WindowsCommandResult ExtractWith(IconCache cache) => ShellIconExtractor.ExtractOnSta(command,
                value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
            var first = ExtractWith(new IconCache(root));
            Assert(first.Success && first.Data is not null, "Owned Shell icon fixture must be available.");
            File.WriteAllBytes(first.Data!, [137, 80, 78, 71, 13, 10, 26, 10]);
            var recovered = ExtractWith(new IconCache(root));
            Assert(recovered.Success && recovered.Data == first.Data, "A corrupt cached PNG must be rebuilt in the same request.");
            using var stream = File.OpenRead(recovered.Data!);
            var image = new System.Windows.Media.Imaging.PngBitmapDecoder(stream,
                System.Windows.Media.Imaging.BitmapCreateOptions.PreservePixelFormat,
                System.Windows.Media.Imaging.BitmapCacheOption.OnLoad).Frames[0];
            Assert(image.PixelWidth >= 48 && image.PixelHeight >= 48, "Recovered cache must contain real native icon pixels.");
        }
        finally
        {
            if (Directory.Exists(root)) Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    internal static Task NativeFailureRetainsSafeCategoryAndCode()
    {
        const int nativeCode = unchecked((int)0x80040154);
        var result = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand("nxtools-file-type:.zip", 48),
            _ => throw new System.Runtime.InteropServices.COMException("private-test-payload", nativeCode));
        Assert(!result.Success && result.NativeErrorCode == nativeCode, "The original STA boundary must retain the native HRESULT.");
        Assert(result.Error == "shell_icon_extract_failed", "Only a fixed failure category may cross the diagnostic boundary.");
        return Task.CompletedTask;
    }

    internal static Task CompletedExecutableUsesOwnedCache()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-executable-icon-{Guid.NewGuid():N}");
        Directory.CreateDirectory(root);
        try
        {
            var executable = Path.Combine(root, "completed.exe");
            File.Copy(Environment.ProcessPath ?? throw new InvalidOperationException("Fixture executable unavailable."), executable);
            var cache = new IconCache(Path.Combine(root, "icons"));
            foreach (var target in new[] { "nxtools-file-type:.exe", executable })
            {
                var result = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(target, 48),
                    value => ShellIconExtractor.Extract(value, CancellationToken.None, cache));
                Assert(result.Success && result.Data is not null, "Both executable type and completed executable must resolve.");
                using var stream = File.OpenRead(result.Data!);
                var image = new System.Windows.Media.Imaging.PngBitmapDecoder(stream,
                    System.Windows.Media.Imaging.BitmapCreateOptions.PreservePixelFormat,
                    System.Windows.Media.Imaging.BitmapCacheOption.OnLoad).Frames[0];
                Assert(image.PixelWidth > 0 && image.PixelHeight > 0, "Native executable icon must be a readable PNG.");
            }
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
