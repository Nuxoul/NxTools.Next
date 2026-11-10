using System.Threading.Channels;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ClipboardStagingTests
{
    public static Task PartialFormatWrite()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "clipboard-staging-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var window = new WindowsPlatformAdapter.EventWindow(Channel.CreateBounded<WindowsPlatformEvent>(1).Writer,
                NullPlatformLog.Instance, new ClipboardCaptureOperations(
                    () => 0, () => 1, () => new(0, 0, 0, "fixture", ""), (_, _) => { }, () => { }, () => { }));
            try
            {
                var result = window.StageNativeFormats([new(8, "", [1, 2, 3, 4], 0)], root,
                    (path, bytes) =>
                    {
                        File.WriteAllBytes(path, bytes[..2]);
                        throw new IOException("fixture partial write");
                    });
                Assert(result.Count == 0, "failed format must not be handed to the consumer");
                Assert(!Directory.EnumerateFiles(root).Any(), "partial format file must be removed");
            }
            finally { window.StopClipboardCapture(); }
        }
        finally { Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    public static Task PartialImageWrite()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "clipboard-staging-" + Guid.NewGuid().ToString("N"));
        var staging = Path.Combine(root, "staging");
        Directory.CreateDirectory(root);
        try
        {
            var source = Path.Combine(root, "source.png");
            using (var image = new System.Drawing.Bitmap(2, 2)) image.Save(source, System.Drawing.Imaging.ImageFormat.Png);
            var result = ClipboardImagePromotion.TryPromote([source], null, staging,
                (_, path, _) =>
                {
                    File.WriteAllBytes(path, [1, 2]);
                    throw new IOException("fixture partial PNG");
                });
            Assert(result is null, "failed PNG must not be handed to the consumer");
            Assert(!Directory.EnumerateFiles(staging).Any(), "partial PNG must be removed");
            Assert(File.Exists(source), "borrowed source must remain");
        }
        finally { Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    public static Task CaptureFailureBeforePublication()
    {
        using var fixture = new Fixture(content => content with { Text = () => throw new IOException("fixture after PNG") });
        var failed = false;
        try { fixture.Capture(); }
        catch (IOException) { failed = true; }
        Assert(failed, "controlled post-image read failure must reach the actual capture boundary");
        Assert(!fixture.PayloadFiles().Any(), "PNG created before publication failure must be removed");
        Assert(!fixture.Events.Reader.TryRead(out _), "failed capture cannot publish an event");
        return Task.CompletedTask;
    }

    public static async Task StoppedPublication()
    {
        using var fixture = new Fixture();
        var path = Path.Combine(fixture.Run.DirectoryPath, Guid.NewGuid().ToString("N") + ".png");
        File.WriteAllBytes(path, [1, 2]);
        fixture.Window.StopClipboardCapture();
        fixture.Window.QueueCapturedEvent("fixture", new(path, 2, 2), [], [], Fixture.Source, 1);
        await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
        Assert(!fixture.Events.Reader.TryRead(out _), "retired capture must not publish");
        Assert(!File.Exists(path), "retired capture must release its image");
    }

    public static async Task PublicationOwnership()
    {
        foreach (var accepted in new[] { true, false })
        {
            var shared = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
                "clipboard-icon-" + Guid.NewGuid().ToString("N") + ".png");
            File.WriteAllBytes(shared, [7, 8, 9]);
            try
            {
                using var fixture = new Fixture(content => content with
                {
                    Formats = _ => [new(8, "", [1, 2, 3], 0)],
                    SourceIcon = (uint _, out string? error) => { error = null; return shared; }
                });
                if (!accepted) fixture.Events.Writer.TryWrite(new ClipboardCapturedEvent("occupied", null, [], null, Fixture.Source, 1, []));
                fixture.Capture();
                await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
                Assert(File.Exists(shared), "shared icon cache is borrowed, not capture-owned");
                if (accepted)
                {
                    Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent, "capture reaches original event queue");
                    var capture = (ClipboardCapturedEvent)item!;
                    Assert(File.Exists(capture.Image!.StagingPath), "successful handoff keeps PNG alive");
                    Assert(capture.NativeFormats!.Count == 1 && File.ReadAllBytes(capture.NativeFormats[0].DataPath).SequenceEqual(new byte[] { 1, 2, 3 }), "consumer receives complete native bytes");
                    Assert(capture.SourceIconStagingPath != shared && File.ReadAllBytes(capture.SourceIconStagingPath!).SequenceEqual(new byte[] { 7, 8, 9 }), "consumer owns a separate icon copy");
                    fixture.Window.StopClipboardCapture();
                    ClipboardStagingDirectory.Recover(fixture.Root);
                    Assert(File.Exists(capture.Image.StagingPath), "live Desktop protects handed-off files after Host lease closes");
                    WindowsPlatformAdapter.CleanupClipboardCapturedStaging(capture);
                }
                Assert(!fixture.PayloadFiles().Any(), "rejected or consumer-retired capture releases all owned payloads");
            }
            finally { File.Delete(shared); }
        }
    }

    public static async Task TailRetirement()
    {
        foreach (var transition in new[] { "pause", "pause_resume", "stop" })
        {
            using var entered = new ManualResetEventSlim();
            using var release = new ManualResetEventSlim();
            using var fixture = new Fixture(content => content with
            {
                Formats = _ => [new(8, "", [1, 2], 0)],
                WriteFormat = (path, bytes) =>
                {
                    entered.Set();
                    if (!release.Wait(TimeSpan.FromSeconds(5))) throw new TimeoutException("fixture release");
                    File.WriteAllBytes(path, bytes);
                }
            });
            try
            {
                fixture.Capture();
                Assert(entered.Wait(TimeSpan.FromSeconds(5)), "real tail reaches format writer");
                if (transition != "stop")
                {
                    var applied = fixture.Window.EnqueueClipboardPause(new(true), Guid.NewGuid(), default);
                    fixture.Window.DispatchClipboardMessage(0x8010);
                    Assert((await applied).Success, "pause applied through original owner");
                    if (transition == "pause_resume")
                    {
                        var resumed = fixture.Window.EnqueueClipboardPause(new(false), Guid.NewGuid(), default);
                        fixture.Window.DispatchClipboardMessage(0x8010);
                        Assert((await resumed).Success, "resume applied through original owner");
                    }
                }
                else
                {
                    // Tiny synthetic files exercise the actual count admission, not memory pressure.
                    for (var index = 0; index < 64; index++) fixture.Capture();
                    Assert(fixture.PayloadFiles().Count() == 64, "65th pending capture is rejected and cleaned");
                    fixture.Window.StopClipboardCapture();
                }
            }
            finally { release.Set(); }
            await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
            Assert(!fixture.Events.Reader.TryRead(out _), "retired tail cannot publish after pause or stop");
            Assert(!fixture.PayloadFiles().Any(), "retired tail releases image and native files");
        }
    }

    public static Task RecoveryOwnership()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "clipboard-recovery-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        var dead = new ClipboardStagingDirectory.ProcessIdentity(int.MaxValue, 1);
        var current = ClipboardStagingDirectory.ProcessIdentity.Current;
        try
        {
            using var activeLease = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, dead));
            var activePath = Seed(activeLease);
            Assert(ClipboardStagingDirectory.Recover(root).DeletedFiles == 0 && File.Exists(activePath), "live Host lease wins even when metadata looks inactive");
            using var writer = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(current, dead));
            var writerPath = Seed(writer); writer.Dispose();
            using var consumer = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, current));
            var consumerPath = Seed(consumer); consumer.Dispose();
            using var unknown = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, null));
            var unknownPath = Seed(unknown); unknown.Dispose();
            using var orphan = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, dead));
            var orphanPath = Seed(orphan); orphan.Dispose();
            var legacy = Path.Combine(root, Guid.NewGuid().ToString("N") + ".png");
            File.WriteAllBytes(legacy, [5]);
            var result = ClipboardStagingDirectory.Recover(root);
            Assert(result.DeletedFiles == 1 && !Directory.Exists(orphan.DirectoryPath), "only confirmed inactive run is retired");
            Assert(new[] { activePath, writerPath, consumerPath, unknownPath, legacy }.All(File.Exists), "active Host/Desktop, unknown consumer and legacy flat file remain");
        }
        finally { Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    public static Task RecoveryBoundsAndRetry()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(), "clipboard-recovery-" + Guid.NewGuid().ToString("N"));
        var dead = new ClipboardStagingDirectory.ProcessIdentity(int.MaxValue, 1);
        try
        {
            using var run = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, dead));
            var locked = Seed(run);
            run.Dispose();
            using (var held = new FileStream(locked, FileMode.Open, FileAccess.Read, FileShare.Read))
            {
                var blocked = ClipboardStagingDirectory.Recover(root);
                Assert(blocked.FailedFiles == 1 && File.Exists(locked), "in-use file remains with its recovery marker");
            }
            Assert(ClipboardStagingDirectory.Recover(root).DeletedFiles == 1, "later recovery retries physical deletion");
            using var large = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, dead));
            for (var index = 0; index < ClipboardStagingDirectory.RecoveryFileLimit + 3; index++) Seed(large);
            large.Dispose();
            var bounded = ClipboardStagingDirectory.Recover(root);
            Assert(bounded.DeletedFiles <= ClipboardStagingDirectory.RecoveryFileLimit && Directory.Exists(large.DirectoryPath), "one recovery pass has a finite file budget");
            ClipboardStagingDirectory.Recover(root);
            Assert(!Directory.Exists(large.DirectoryPath), "remaining files are reclaimed on a later pass");
            for (var index = 0; index < ClipboardStagingDirectory.RecoveryRunLimit + 1; index++)
            {
                using var extra = ClipboardStagingDirectory.Create(root, new ClipboardStagingDirectory.RunOwner(dead, dead));
                Seed(extra);
            }
            var runs = ClipboardStagingDirectory.Recover(root);
            Assert(runs.Runs == ClipboardStagingDirectory.RecoveryRunLimit && Directory.EnumerateDirectories(root).Count() == 1, "run enumeration is bounded");
            ClipboardStagingDirectory.Recover(root);
            Assert(!Directory.EnumerateFileSystemEntries(root).Any(), "all inactive runs eventually retire");
        }
        finally { Directory.Delete(root, recursive: true); }
        return Task.CompletedTask;
    }

    private static string Seed(ClipboardStagingDirectory run)
    {
        var path = Path.Combine(run.DirectoryPath, Guid.NewGuid().ToString("N") + ".bin");
        File.WriteAllBytes(path, [1]);
        return path;
    }

    internal sealed class Fixture : IDisposable
    {
        internal static readonly ForegroundContext Source = new(0, 0, 0, "fixture", "");
        internal readonly string Root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath(),
            "clipboard-staging-" + Guid.NewGuid().ToString("N"));
        internal readonly ClipboardStagingDirectory Run;
        internal readonly Channel<WindowsPlatformEvent> Events = Channel.CreateBounded<WindowsPlatformEvent>(1);
        internal readonly WindowsPlatformAdapter.EventWindow Window;
        internal Fixture(Func<ClipboardContentOperations, ClipboardContentOperations>? change = null, IPlatformLog? log = null,
            ClipboardCaptureLimits? limits = null, long? publicationByteLimit = null)
        {
            Run = ClipboardStagingDirectory.Create(Root, Environment.ProcessId);
            var content = new ClipboardContentOperations(_ => [], () => [], () => new System.Drawing.Bitmap(2, 2),
                () => "fixture", ClipboardImageEncoding.SavePng, File.WriteAllBytes,
                (uint _, out string? error) => { error = null; return null; });
            Window = new(Events.Writer, log ?? NullPlatformLog.Instance, new ClipboardCaptureOperations(
                () => 0, () => 1, () => Source, (_, _) => { }, () => { }, () => { }), change?.Invoke(content) ?? content, Run, limits, publicationByteLimit);
        }
        internal void Capture() => Window.ReadClipboardContent(Source, 1);
        internal IEnumerable<string> PayloadFiles() => Directory.Exists(Run.DirectoryPath)
            ? Directory.EnumerateFiles(Run.DirectoryPath).Where(file => Path.GetExtension(file) is ".png" or ".bin") : [];
        public void Dispose()
        {
            Window.StopClipboardCapture();
            Window.ClipboardPublication.GetAwaiter().GetResult();
            Run.Dispose();
            Directory.Delete(Root, recursive: true);
        }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
