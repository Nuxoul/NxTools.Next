using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: pure fixed-target parsing, bounded owned STA, Windows attribute-only icon
// query, and one GUID-owned cache. Never IconCache.Shared, member paths, Shell open or .lnk COM.
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
        }
        finally
        {
            if (Directory.Exists(root)) Directory.Delete(root, recursive: true);
        }
        return Task.CompletedTask;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
