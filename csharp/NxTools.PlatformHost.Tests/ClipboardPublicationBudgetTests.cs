using System.Drawing;
using System.Drawing.Imaging;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ClipboardPublicationBudgetTests
{
    private const long Budget = 3 * 1024 * 1024;
    private const int LargePayload = 2 * 1024 * 1024;

    private sealed class Log : IPlatformLog
    {
        internal bool ThrowDrops;
        internal readonly List<(string Name, IReadOnlyDictionary<string, object?> Fields)> Entries = [];
        public void Write(string level, string eventName, Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null)
        {
            Entries.Add((eventName, fields ?? new Dictionary<string, object?>()));
            if (ThrowDrops && eventName == "clipboard.capture_event_dropped") throw new IOException("fixture log failure");
        }
    }

    public static async Task BlockedConsumerRejectsBytes()
    {
        foreach (var throwDrops in new[] { false, true })
        {
            using var entered = new ManualResetEventSlim();
            using var release = new ManualResetEventSlim();
            var log = new Log { ThrowDrops = throwDrops };
            using var fixture = new ClipboardStagingTests.Fixture(content => content with
            {
                SourceIcon = (uint _, out string? error) =>
                {
                    error = null; entered.Set(); Wait(release); return null;
                }
            }, log: log, publicationByteLimit: Budget);
            try
            {
                Assert(fixture.Window.ClipboardPublicationByteLimit == Budget, "fixture uses byte-admission boundary");
                Queue(fixture, "first", LargePayload);
                Wait(entered);
                var admitted = fixture.Window.ClipboardPublicationState;
                Assert(admitted.Pending == 1 && admitted.RetainedBytes >= LargePayload && admitted.RetainedBytes <= Budget,
                    "actual array payload is charged while consumer is blocked");
                var imagePath = Path.Combine(fixture.Run.DirectoryPath, Guid.NewGuid().ToString("N") + ".png");
                using (var image = new Bitmap(2, 2)) image.Save(imagePath, ImageFormat.Png);
                Queue(fixture, "second", LargePayload, new(imagePath, 2, 2));
                Assert(!File.Exists(imagePath), "byte-rejected capture releases staging immediately");
                Assert(fixture.Window.ClipboardPublicationState == admitted, "rejection never consumes a slot or byte reservation");
                var dropped = log.Entries.Single(item => item.Name == "clipboard.capture_event_dropped");
                Assert(Equals(dropped.Fields["reason"], "publication_byte_budget_full") &&
                    Convert.ToInt64(dropped.Fields["retained_payload_bytes"]) == admitted.RetainedBytes,
                    "bounded drop diagnostic preserves admission facts");
                var diagnostic = JsonSerializer.Serialize(dropped.Fields);
                Assert(!diagnostic.Contains(imagePath) && !diagnostic.Contains("second"), "diagnostic contains no capture paths or text");
            }
            finally { release.Set(); }
            await Drain(fixture);
            var first = Take(fixture, "first");
            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(first);
        }
    }

    public static async Task AcceptedOrderAndReuse()
    {
        using var firstEntered = new ManualResetEventSlim();
        using var firstRelease = new ManualResetEventSlim();
        using var secondEntered = new ManualResetEventSlim();
        using var secondRelease = new ManualResetEventSlim();
        var reads = 0;
        using var fixture = new ClipboardStagingTests.Fixture(content => content with
        {
            SourceIcon = (uint _, out string? error) =>
            {
                error = null;
                var call = Interlocked.Increment(ref reads);
                if (call == 1) { firstEntered.Set(); Wait(firstRelease); }
                if (call == 2) { secondEntered.Set(); Wait(secondRelease); }
                return null;
            }
        }, publicationByteLimit: Budget);
        try
        {
            Queue(fixture, "first", LargePayload); Wait(firstEntered);
            Queue(fixture, "rejected", LargePayload);
            Queue(fixture, "third", 64 * 1024);
            Assert(fixture.Window.ClipboardPublicationState is { Pending: 2 } state && state.RetainedBytes <= Budget,
                "smaller admitted payload fits alongside active capture");
            firstRelease.Set(); Wait(secondEntered);
            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "first"));
            Assert(fixture.Window.ClipboardPublicationState is { Pending: 1 } remaining && remaining.RetainedBytes < LargePayload,
                "completed capture releases its bytes before later work finishes");
            secondRelease.Set(); await Drain(fixture);
            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "third"));
            Queue(fixture, "reused", LargePayload); await Drain(fixture);
            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "reused"));
        }
        finally { firstRelease.Set(); secondRelease.Set(); }
    }

    public static async Task OversizedCaptureIsExclusive()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var fixture = new ClipboardStagingTests.Fixture(content => content with
        {
            SourceIcon = (uint _, out string? error) => { error = null; entered.Set(); Wait(release); return null; }
        }, publicationByteLimit: Budget);
        try
        {
            Queue(fixture, "oversized", 4 * 1024 * 1024); Wait(entered);
            var alone = fixture.Window.ClipboardPublicationState;
            Assert(alone.Pending == 1 && alone.RetainedBytes > Budget, "one legal large capture is admitted into an empty tail");
            Queue(fixture, "must_not_stack", 1);
            Assert(fixture.Window.ClipboardPublicationState == alone, "oversized capture excludes every additional payload");
        }
        finally { release.Set(); }
        await Drain(fixture);
        WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "oversized"));
        Queue(fixture, "after_large", 1024); await Drain(fixture);
        WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "after_large"));
    }

    public static async Task Utf16TextIsCharged()
    {
        using var entered = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        using var fixture = new ClipboardStagingTests.Fixture(content => content with
        {
            SourceIcon = (uint _, out string? error) => { error = null; entered.Set(); Wait(release); return null; }
        }, publicationByteLimit: Budget);
        try
        {
            Queue(fixture, new string('x', 900_000), 1024); Wait(entered);
            var first = fixture.Window.ClipboardPublicationState;
            Assert(first.RetainedBytes >= 1_800_000 + 1024, "retained UTF-16 text is charged in addition to arrays");
            Queue(fixture, new string('y', 900_000), 1024);
            Assert(fixture.Window.ClipboardPublicationState == first, "text-heavy payload cannot evade byte admission");
        }
        finally { release.Set(); }
        await Drain(fixture);
        Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent { Text.Length: 900_000 }, "only accepted text is published");
        WindowsPlatformAdapter.CleanupClipboardCapturedStaging((ClipboardCapturedEvent)item!);
    }

    public static async Task CancellationWaitsForActualRetirement()
    {
        foreach (var stop in new[] { false, true })
        {
            using var entered = new ManualResetEventSlim();
            using var release = new ManualResetEventSlim();
            using var fixture = new ClipboardStagingTests.Fixture(content => content with
            {
                SourceIcon = (uint _, out string? error) => { error = null; entered.Set(); Wait(release); return null; }
            }, publicationByteLimit: Budget);
            try
            {
                Queue(fixture, "active", LargePayload); Wait(entered);
                Queue(fixture, "waiting", 128 * 1024);
                var retained = fixture.Window.ClipboardPublicationState;
                if (stop) fixture.Window.StopClipboardCapture();
                else
                {
                    var pause = fixture.Window.EnqueueClipboardPause(new(true), Guid.NewGuid(), default);
                    fixture.Window.DispatchClipboardMessage(0x8010); Assert((await pause).Success, "pause applied");
                    var resume = fixture.Window.EnqueueClipboardPause(new(false), Guid.NewGuid(), default);
                    fixture.Window.DispatchClipboardMessage(0x8010); Assert((await resume).Success, "resume applied");
                }
                Assert(fixture.Window.ClipboardPublicationState == retained, "logical cancellation cannot free bytes still held by blocked work");
            }
            finally { release.Set(); }
            await Drain(fixture);
            Assert(!fixture.Events.Reader.TryRead(out _) && !fixture.PayloadFiles().Any(), "cancelled epochs clean staging and cannot publish");
            if (!stop)
            {
                Queue(fixture, "fresh", LargePayload); await Drain(fixture);
                WindowsPlatformAdapter.CleanupClipboardCapturedStaging(Take(fixture, "fresh"));
            }
        }
    }

    public static async Task HandoffAndFailuresRelease()
    {
        foreach (var mode in new[] { "handoff", "icon_copy", "partial_write", "closed", "full" })
        {
            string borrowed = "";
            using var fixture = new ClipboardStagingTests.Fixture(content => content with
            {
                SourceIcon = (uint _, out string? error) => { error = null; return mode == "icon_copy" ? borrowed : null; },
                WriteFormat = (path, bytes) =>
                {
                    if (mode == "partial_write") { File.WriteAllBytes(path, bytes[..16]); throw new IOException("fixture partial write"); }
                    File.WriteAllBytes(path, bytes);
                }
            }, publicationByteLimit: Budget);
            borrowed = Path.Combine(fixture.Root, "borrowed.png");
            File.WriteAllBytes(borrowed, [1, 2]);
            using var held = mode == "icon_copy" ? new FileStream(borrowed, FileMode.Open, FileAccess.Read, FileShare.None) : null;
            if (mode == "closed") fixture.Events.Writer.TryComplete();
            if (mode == "full") fixture.Events.Writer.TryWrite(new ClipboardCapturedEvent("occupied", null, [], null,
                ClipboardStagingTests.Fixture.Source, 1, []));
            Queue(fixture, mode, LargePayload); await Drain(fixture);
            if (mode is "handoff" or "partial_write")
            {
                var capture = Take(fixture, mode);
                if (mode == "handoff")
                {
                    Assert(capture.NativeFormats is { Count: 1 } && new FileInfo(capture.NativeFormats[0].DataPath).Length == LargePayload,
                        "handoff keeps complete native content on disk");
                    var envelope = PlatformEventEnvelopeFactory.Create(capture);
                    Assert(JsonSerializer.SerializeToUtf8Bytes(envelope, JsonDefaults.Options).Length < 16 * 1024,
                        "existing pipe handoff contains paths and metadata, not the retained byte array");
                }
                else Assert(capture.NativeFormats is { Count: 0 }, "failed format is not handed off");
                // The same cleanup boundary is used by forwarding failure and session retirement.
                WindowsPlatformAdapter.CleanupClipboardCapturedStaging(capture);
            }
            Assert(!fixture.PayloadFiles().Any() && File.Exists(borrowed), "failure/receiver retirement cleans owned payloads but preserves borrowed files");
        }
    }

    private static void Queue(ClipboardStagingTests.Fixture fixture, string text, int bytes, ClipboardImageData? image = null) =>
        fixture.Window.QueueCapturedEvent(text, image, [], [new(0xC001, "fixture.bytes", new byte[bytes], 0)],
            ClipboardStagingTests.Fixture.Source, 1);

    private static async Task Drain(ClipboardStagingTests.Fixture fixture)
    {
        await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
        Assert(fixture.Window.ClipboardPublicationState == (0, 0L), "all completed work releases byte and count reservations");
    }

    private static ClipboardCapturedEvent Take(ClipboardStagingTests.Fixture fixture, string text)
    {
        Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent { } capture && capture.Text == text,
            "accepted capture order/content remains unchanged");
        return (ClipboardCapturedEvent)item!;
    }

    private static void Wait(ManualResetEventSlim gate)
    {
        if (!gate.Wait(TimeSpan.FromSeconds(5))) throw new TimeoutException("fixture consumer boundary");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
