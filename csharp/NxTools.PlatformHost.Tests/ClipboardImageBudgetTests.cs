using System.Drawing;
using System.Drawing.Imaging;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class ClipboardImageBudgetTests
{
    private static ClipboardCaptureLimits Limits => new(8 * 1024 * 1024,
        ClipboardCaptureLimits.MinImageBytes, ClipboardCaptureLimits.MinImagePixels);

    public static async Task NativePixelsBeforeEncoding()
    {
        var encodes = 0;
        using var fixture = new ClipboardStagingTests.Fixture(content => content with
        {
            Image = () => new Bitmap(1001, 1000),
            SaveImage = (image, path, _) => { encodes++; image.Save(path, ImageFormat.Png); }
        }, limits: Limits);
        fixture.Capture();
        await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
        Assert(encodes == 0, "over-pixel-budget native image must be rejected before PNG encoding");
        Assert(!fixture.PayloadFiles().Any(), "rejected native image has no staging payload");
        Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent { Image: null, Text: "fixture" },
            "independent text survives image rejection");
    }

    public static async Task FilePixelsBeforeEncoding()
    {
        var encodes = 0;
        var nativeImageReads = 0;
        string source = "";
        using var fixture = new ClipboardStagingTests.Fixture(content => content with
        {
            Files = () => [source],
            Image = () => { nativeImageReads++; return null; },
            SaveImage = (image, path, _) => { encodes++; image.Save(path, ImageFormat.Png); }
        }, limits: Limits);
        source = Path.Combine(fixture.Root, "source.png");
        using (var image = new Bitmap(1001, 1000)) image.Save(source, ImageFormat.Png);
        fixture.Capture();
        await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
        Assert(encodes == 0, "over-pixel-budget file image must be rejected before PNG encoding");
        Assert(nativeImageReads == 0, "known image rejection must not materialize another clipboard bitmap");
        Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent capture &&
            capture.Image is null && capture.Paths.SequenceEqual(new[] { source }), "file representation survives image rejection");
        Assert(File.Exists(source) && !fixture.PayloadFiles().Any(), "borrowed source remains and no encoded staging exists");
    }

    public static async Task EncodedOutputStopsAtBudget()
    {
        foreach (var fromFile in new[] { false, true })
        {
            var encodes = 0;
            long observedLength = -1;
            string source = "";
            using var fixture = new ClipboardStagingTests.Fixture(content => content with
            {
                Files = () => fromFile ? [source] : [],
                Image = () => fromFile ? null : NoiseBitmap(),
                SaveImage = (image, path, maximum) =>
                {
                    encodes++;
                    try { ClipboardImageEncoding.SavePng(image, path, maximum); }
                    finally { observedLength = File.Exists(path) ? new FileInfo(path).Length : -1; }
                }
            }, limits: Limits);
            if (fromFile)
            {
                source = Path.Combine(fixture.Root, "source.png");
                using var image = NoiseBitmap();
                image.Save(source, ImageFormat.Png);
            }
            fixture.Capture();
            await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
            Assert(encodes == 1, "within-pixel-budget image reaches the actual PNG encoder once");
            Assert(observedLength >= 0 && (ulong)observedLength <= Limits.MaxImageBytes,
                "PNG output must stop before exceeding the configured byte budget");
            Assert(!fixture.PayloadFiles().Any(), "over-byte-budget PNG is not left in staging");
            Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent capture &&
                capture.Image is null && capture.Text == "fixture", "independent content survives encoded-byte rejection");
            if (fromFile)
            {
                Assert(File.Exists(source), "encoded-byte rejection never deletes the source file");
                Assert(((ClipboardCapturedEvent)item!).Paths.SequenceEqual(new[] { source }),
                    "published file representation survives encoded-byte rejection");
            }
        }
    }

    public static async Task PixelBoundaryRoundTrip()
    {
        foreach (var fromFile in new[] { false, true })
        {
            string source = "";
            using var fixture = new ClipboardStagingTests.Fixture(content => content with
            {
                Files = () => fromFile ? [source] : [],
                Image = () => fromFile ? null : BoundaryBitmap()
            }, limits: Limits);
            if (fromFile)
            {
                source = Path.Combine(fixture.Root, "source.png");
                using var original = BoundaryBitmap();
                original.Save(source, ImageFormat.Png);
            }
            fixture.Capture();
            await fixture.Window.ClipboardPublication.WaitAsync(TimeSpan.FromSeconds(5));
            Assert(fixture.Events.Reader.TryRead(out var item) && item is ClipboardCapturedEvent { Image: not null }, "image exactly at the pixel limit is accepted");
            var capture = (ClipboardCapturedEvent)item!;
            Assert(capture.Image!.Width == 1000 && capture.Image.Height == 1000, "accepted dimensions are not scaled");
            using (var decoded = new Bitmap(capture.Image.StagingPath))
                Assert(decoded.GetPixel(0, 0).ToArgb() == Color.FromArgb(127, 23, 45, 67).ToArgb(), "PNG preserves pixel alpha and color");
            WindowsPlatformAdapter.CleanupClipboardCapturedStaging(capture);
            if (fromFile) Assert(File.Exists(source), "successful promotion preserves borrowed source");
        }
    }

    public static Task EncodedBoundaryRoundTrip()
    {
        using var fixture = new ClipboardStagingTests.Fixture();
        using var image = NoiseBitmap();
        using var oracle = new MemoryStream();
        image.Save(oracle, ImageFormat.Png);
        var expected = oracle.ToArray();
        Assert(expected.Length > (int)ClipboardCaptureLimits.MinImageBytes, "exact-byte boundary is within supported capture settings");
        using var owned = new ClipboardCaptureFiles();
        var complete = owned.CreatePath(fixture.Run.DirectoryPath, ".png");
        ClipboardImageEncoding.SavePng(image, complete, (ulong)expected.Length);
        Assert(File.ReadAllBytes(complete).SequenceEqual(expected), "exact-byte budget preserves the original PNG encoder output");
        var rejected = owned.CreatePath(fixture.Run.DirectoryPath, ".png");
        var failed = false;
        try { ClipboardImageEncoding.SavePng(image, rejected, (ulong)expected.Length - 1); }
        catch (ClipboardImageBudgetExceededException budget) { failed = budget.Reason == "max_image_bytes"; }
        Assert(failed && new FileInfo(rejected).Length < expected.Length, "one byte below required output rejects before writing past the limit");
        return Task.CompletedTask;
    }

    public static Task ConversionFailuresAndCanonicalFormats()
    {
        using var fixture = new ClipboardStagingTests.Fixture();
        var source = Path.Combine(fixture.Root, "source.png");
        using var image = new Bitmap(2, 2);
        image.SetPixel(0, 0, Color.FromArgb(255, 17, 34, 51));
        image.Save(source, ImageFormat.Png);
        var failed = ClipboardImagePromotion.TryPromote([source], null, fixture.Run.DirectoryPath,
            (_, path, _) =>
            {
                File.WriteAllBytes(path, [137, 80, 78, 71]);
                throw new System.Runtime.InteropServices.ExternalException("fixture PNG conversion failure");
            }, limits: Limits);
        Assert(failed is null && !fixture.PayloadFiles().Any() && File.Exists(source), "codec failure cleans partial output without deleting input");
        var png = File.ReadAllBytes(source);
        Assert(ClipboardImageCompatibility.TryCreateCanonicalBitmapFormats(png, out var v5, out var dib), "valid small PNG retains canonical conversion");
        Assert(v5 is { Length: 140 } && dib is { Length: 56 }, "canonical output retains 124/40-byte headers and 32bpp pixels");
        Assert(dib!.AsSpan(40, 4).SequenceEqual(new byte[] { 51, 34, 17, 255 }), "canonical BGRA pixel remains unchanged");
        File.WriteAllBytes(source, [137, 80, 78, 71]);
        Assert(ClipboardImagePromotion.TryPromote([source], null, fixture.Run.DirectoryPath, limits: Limits) is null,
            "corrupt image input is rejected by the existing codec");
        Assert(!ClipboardImageCompatibility.TryCreateCanonicalBitmapFormats([137, 80, 78, 71], out v5, out dib) && v5 is null && dib is null,
            "failed canonical conversion has no partial outputs");
        Assert(File.Exists(source) && !fixture.PayloadFiles().Any(), "invalid input creates no staging and remains borrowed");
        return Task.CompletedTask;
    }

    private static Bitmap BoundaryBitmap()
    {
        var image = new Bitmap(1000, 1000);
        image.SetPixel(0, 0, Color.FromArgb(127, 23, 45, 67));
        return image;
    }

    private static Bitmap NoiseBitmap()
    {
        var image = new Bitmap(640, 640, PixelFormat.Format32bppArgb);
        var pixels = new byte[640 * 640 * 4];
        new Random(17).NextBytes(pixels);
        for (var index = 3; index < pixels.Length; index += 4) pixels[index] = 255;
        var bits = image.LockBits(new Rectangle(0, 0, 640, 640), ImageLockMode.WriteOnly, PixelFormat.Format32bppArgb);
        try { System.Runtime.InteropServices.Marshal.Copy(pixels, 0, bits.Scan0, pixels.Length); }
        finally { image.UnlockBits(bits); }
        return image;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
