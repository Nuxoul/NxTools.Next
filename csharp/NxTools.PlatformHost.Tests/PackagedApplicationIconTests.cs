using System.Runtime.InteropServices;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Injected COM leaf, self-owned HBITMAPs and GUID-owned PNG cache only. No Shell COM or app launch.
internal static class PackagedApplicationIconTests
{
    private const string First = @"shell:AppsFolder\Future.Unknown_fixture!App.main";
    private const string Second = @"shell:AppsFolder\Another.Unknown_fixture!App.main";

    internal static Task NamespaceResolutionAndCacheIdentity()
    {
        var resolved = ShellIconExtractor.ResolveIconTarget(First);
        Assert(resolved is { IsNamespace: true, UseFileAttributes: false, IconIndex: null } && resolved.Path == First,
            "AUMID suffix must not become a file extension or a filesystem target.");
        var key = IconCacheKey.Create(First, null, 32, 123, "namespace");
        Assert(key.TargetPath == First && key.Selector == "namespace/icon-only" && key.LastWriteTicks == 0,
            "Namespace identity bypasses GetFullPath and filesystem timestamps.");
        Assert(key != IconCacheKey.Create(Second, null, 32, 123, "namespace"), "Unknown apps have distinct identities.");
        Assert(key != IconCacheKey.Create(First, null, 48, 123, "namespace"), "Requested size remains part of identity.");
        foreach (var invalid in new[]
        {
            @"shell:AppsFolder\", @"shell:AppsFolder\family!", @"shell:AppsFolder\!App.main",
            @"shell:AppsFolder\family!app!other", @"shell:AppsFolder\family!app.exe\child",
            @"shell:AppsFolder\family!app/name", @"shell:AppsFolder\family!app name",
            @"shell:appsfolder\family!app", @"shell:AppsFolder\family!app%20",
            " " + First, First + " ", @"shell:AppsFolder\family!" + new string('a', 128)
        })
            Assert(ShellIconExtractor.ResolveIconTarget(invalid) is null, "Invalid namespace must not use type-icon fallback.");
        return Task.CompletedTask;
    }

    internal static Task RealExtractUsesFactoryPngAndOriginalStaCache()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-packaged-icons-{Guid.NewGuid():N}");
        try
        {
            var cache = new IconCache(root);
            var native = new OwnedImageNative();
            var factory = new ShellItemImageFactory(native);
            WindowsCommandResult Extract(string target, uint size = 32) => ShellIconExtractor.ExtractOnSta(
                new ExtractShellIconCommand(target, size),
                command => ShellIconExtractor.Extract(command, CancellationToken.None, cache, factory));
            var first = Extract(First);
            Assert(first.Success && first.Data is not null && Path.GetDirectoryName(first.Data) == root,
                "Real resolver/cache/factory/PNG path must publish only into the owned cache.");
            Assert(native.Targets.SequenceEqual([First]) && native.Sizes.SequenceEqual([new ShellImageSize(32, 32)]),
                "Factory receives the unchanged namespace and requested device-pixel size.");
            Assert(native.InterfaceIds.All(value => value == new Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b")) &&
                native.Flags.All(value => value == 4) && native.Apartments.All(value => value == ApartmentState.STA),
                "Only IShellItemImageFactory ICONONLY on the original STA is allowed.");
            using (var stream = File.OpenRead(first.Data!))
            {
                var png = new System.Windows.Media.Imaging.PngBitmapDecoder(stream,
                    System.Windows.Media.Imaging.BitmapCreateOptions.PreservePixelFormat,
                    System.Windows.Media.Imaging.BitmapCacheOption.OnLoad).Frames[0];
                Assert(png.PixelWidth == 32 && png.PixelHeight == 32, "Actual HBITMAP pixels survive PNG publication.");
            }
            Assert(Extract(First).Data == first.Data && native.Creates == 1, "Same key reuses original IconCache.");
            var second = Extract(Second);
            Assert(second.Success && second.Data != first.Data && native.Creates == 2, "Unknown app artwork never aliases by suffix.");
            var large = Extract(First, 48);
            Assert(large.Success && large.Data != first.Data && native.Sizes.Last() == new ShellImageSize(48, 48),
                "Size uses a separate factory/cache request.");
            Assert(native.Releases == 3 && native.Deletes == 3 && native.LiveBitmaps.Count == 0,
                "Every native bitmap and factory pointer is released exactly once.");
            Assert(cache.MemoryEntryCount == 3 && Directory.GetFiles(root).Length == 3, "Original bounded cache owns all PNGs.");
            var before = native.Creates;
            var invalid = Extract(@"shell:AppsFolder\family!app/path.exe");
            Assert(!invalid.Success && native.Creates == before, "Malformed namespace is rejected before factory or type fallback.");
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    internal static Task HResultRetryCancellationAndCleanup()
    {
        var root = Path.Combine(Path.GetTempPath(), $"nxtools-packaged-failure-{Guid.NewGuid():N}");
        try
        {
            const int failure = unchecked((int)0x80040154);
            foreach (var createFails in new[] { true, false })
            {
                var native = new OwnedImageNative { CreateStatus = createFails ? failure : 0, ImageStatus = failure };
                var cache = new IconCache(Path.Combine(root, createFails ? "create" : "image"));
                var result = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(First, 32),
                    command => ShellIconExtractor.Extract(command, CancellationToken.None, cache, new ShellItemImageFactory(native)));
                Assert(!result.Success && result.NativeErrorCode == failure && result.Error == "shell_icon_query_failed",
                    "Native HRESULT crosses the real Extract path without extension fallback or raw exception text.");
                Assert(native.Releases == 1 && native.Deletes == (createFails ? 0 : 1) && native.LiveBitmaps.Count == 0,
                    "Failure-returned native resources still require cleanup.");
                Assert(cache.MemoryEntryCount == 0 && Directory.GetFiles(Path.GetDirectoryName(cache.GetPath(
                    IconCacheKey.Create(First, null, 32, 0, "namespace")))!).Length == 0, "Failed extraction publishes no PNG.");
            }
            var pending = new OwnedImageNative { PendingReplies = 1 };
            var pendingResult = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(First, 32),
                command => ShellIconExtractor.Extract(command, CancellationToken.None,
                    new IconCache(Path.Combine(root, "pending")), new ShellItemImageFactory(pending)));
            Assert(pendingResult.Success && pending.Sizes.Count == 2 && pending.Deletes == 2 && pending.Releases == 1,
                "E_PENDING retry stays in one factory lease and deletes superseded bitmaps.");
            var exhausted = new OwnedImageNative { PendingReplies = 10 };
            var exhaustedResult = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(First, 32),
                command => ShellIconExtractor.Extract(command, CancellationToken.None,
                    new IconCache(Path.Combine(root, "exhausted")), new ShellItemImageFactory(exhausted)));
            Assert(!exhaustedResult.Success && exhaustedResult.NativeErrorCode == unchecked((int)0x8000000A) &&
                exhausted.Sizes.Count == 6 && exhausted.Deletes == 6 && exhausted.Releases == 1,
                "Pending retries are finite and preserve terminal HRESULT/cleanup.");
            using var cancellation = new CancellationTokenSource();
            using var completed = new ManualResetEventSlim();
            var cancelled = new OwnedImageNative { AfterImage = cancellation.Cancel };
            var cancelledResult = ShellIconExtractor.ExtractOnSta(new ExtractShellIconCommand(First, 32),
                command =>
                {
                    try
                    {
                        return ShellIconExtractor.Extract(command, cancellation.Token,
                            new IconCache(Path.Combine(root, "cancelled")), new ShellItemImageFactory(cancelled));
                    }
                    finally { completed.Set(); }
                }, cancellation.Token);
            Assert(completed.Wait(TimeSpan.FromSeconds(2)), "Await actual native cleanup, not only logical cancellation.");
            Assert(!cancelledResult.Success && cancelledResult.NativeErrorCode == 995 &&
                cancelled.Releases == 1 && cancelled.Deletes == 1 && cancelled.LiveBitmaps.Count == 0,
                "Cancellation after GetImage retires pixels and releases native ownership.");
        }
        finally { if (Directory.Exists(root)) Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    private sealed class OwnedImageNative : IShellItemImageNative
    {
        internal readonly List<string> Targets = [];
        internal readonly List<Guid> InterfaceIds = [];
        internal readonly List<ShellImageSize> Sizes = [];
        internal readonly List<uint> Flags = [];
        internal readonly List<ApartmentState> Apartments = [];
        internal readonly HashSet<IntPtr> LiveBitmaps = [];
        internal int Creates, Releases, Deletes;
        internal int CreateStatus, ImageStatus, PendingReplies;
        internal Action? AfterImage;
        public int Create(string target, Guid interfaceId, out IntPtr factory)
        {
            Targets.Add(target); InterfaceIds.Add(interfaceId); Apartments.Add(Thread.CurrentThread.GetApartmentState());
            Creates++; factory = new IntPtr(42); return CreateStatus;
        }
        public int GetImage(IntPtr factory, ShellImageSize size, uint flags, out IntPtr bitmap)
        {
            Assert(factory == new IntPtr(42), "Exact factory lease must reach GetImage.");
            Sizes.Add(size); Flags.Add(flags); Apartments.Add(Thread.CurrentThread.GetApartmentState());
            using var source = new System.Drawing.Bitmap(size.Width, size.Height);
            using (var graphics = System.Drawing.Graphics.FromImage(source)) graphics.Clear(System.Drawing.Color.CornflowerBlue);
            bitmap = source.GetHbitmap(); LiveBitmaps.Add(bitmap);
            AfterImage?.Invoke();
            return PendingReplies-- > 0 ? unchecked((int)0x8000000A) : ImageStatus;
        }
        public void Release(IntPtr factory)
        {
            Assert(factory == new IntPtr(42), "Release must match the factory lease."); Releases++;
        }
        public void DeleteBitmap(IntPtr bitmap)
        {
            Assert(LiveBitmaps.Remove(bitmap), "Delete only an owned bitmap, exactly once.");
            Assert(DeleteObject(bitmap), "Owned HBITMAP must be deleted."); Deletes++;
        }
        [DllImport("gdi32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool DeleteObject(IntPtr bitmap);
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
