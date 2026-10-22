using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Explicit independent tiny fixtures, GUID-owned source/output and owned Job only.
internal static class ArchiveZipMethodWorkerTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    public static Task ZstdRead() => ReadMethod("independent-zstd.zip");
    public static Task XzRead() => ReadMethod("independent-xz.zip");
    public static Task LzmaRead() => ReadMethod("independent-lzma.zip");
    public static Task Deflate64Read() => ReadMethod("independent-deflate64.zip");
    private const string FixturePassword = "synthetic-private-ZIP-secret";
    public static Task ZipCryptoRead() => ReadMethod("independent-zipcrypto.zip", FixturePassword);
    public static Task ZipCryptoDescriptorRead() => ReadMethod("independent-zipcrypto-dd.zip", FixturePassword, true);
    private static async Task ReadMethod(string fixture, string? password = null, bool normalOnly = false)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built Rust worker and independent ZIP fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-zip-method-");
        var copied = false;
        try
        {
            File.Copy(worker, helper, false); copied = true;
            foreach (var mode in normalOnly ? new[] { "normal" } : password is not null ? new[] { "normal", "missing", "wrong", "header", "cipher", "crc", "truncated", "unknown", "budget", "metadata", "existing", "cancel" } : new[] { "normal", "crc", "truncated", "unknown", "budget", "metadata", "existing", "cancel" })
            foreach (var operation in mode is "existing" or "cancel" ? new[] { "extract" } : new[] { "open", "test", "extract" })
            {
                var bytes = File.ReadAllBytes(Path.Combine(fixtures, fixture));
                var central = Enumerable.Range(0, bytes.Length - 3).First(i => bytes.AsSpan(i, 4).SequenceEqual("PK\u0001\u0002"u8));
                if (mode == "crc") { bytes[14] ^= 1; bytes[central + 16] ^= 1; }
                if (mode == "truncated") { var size = BitConverter.ToUInt32(bytes, 18) - 1; BitConverter.GetBytes(size).CopyTo(bytes, 18); BitConverter.GetBytes(size).CopyTo(bytes, central + 20); }
                if (mode == "unknown") { bytes[8] = 222; bytes[central + 10] = 222; }
                var offset = 30 + BitConverter.ToUInt16(bytes, 26) + BitConverter.ToUInt16(bytes, 28);
                if (mode == "header") bytes[offset + 11] ^= 1;
                if (mode == "cipher") bytes[offset + 20] ^= 0x40;
                var source = Path.Combine(owned.FullName, "source.zip"); File.WriteAllBytes(source, bytes);
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var final = Path.Combine(root.FullName, "资料", "payload.bin");
                if (mode == "existing") { Directory.CreateDirectory(Path.GetDirectoryName(final)!); File.WriteAllText(final, "user bytes"); }
                var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var id = Guid.NewGuid(); var barriers = 0;
                runner.OutputCommitCheckpointProbe = stage => {
                    if (stage != "before_flush") return;
                    barriers++;
                    try { using var denied = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("ZIP source lease writable"); }
                    catch (IOException) { }
                    if (mode == "cancel") Require(runner.Cancel(id), "ZIP precommit cancellation");
                };
                Require(runner.Start(new(id, operation, source, 0,
                    new { max_entries = 100, max_entry_bytes = mode == "budget" ? 1 : 1048576, max_metadata_bytes = mode == "metadata" ? 1 : 1048576 },
                    Password: mode == "missing" ? null : mode == "wrong" ? "wrong-private-secret" : password,
                    OutputRoot: operation == "extract" ? root.FullName : null)), "ZIP worker admission");
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 15000, "ZIP worker deadline"); await Task.Delay(20); }
                Require(await runner.DrainAsync(), "ZIP drain");
                var terminal = events.Single(e => e.Kind == "exited");
                var success = mode == "normal" || operation == "open" && mode is "crc" or "truncated" or "unknown" or "budget" or "missing" or "wrong" or "header" or "cipher";
                Require(terminal.Category == (success ? "success" : mode == "cancel" ? "cancelled" : mode is "budget" or "metadata" ? "resource_limit" : "worker_failure") && terminal.CleanupComplete,
                    "ZIP terminal: " + fixture + "/" + mode + "/" + operation + "/" + terminal.Category);
                var frames = events.Where(e => e.Kind == "frame" && e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                Require(!frames.Any(f => f.GetProperty("op").GetString() == "native_call"), "private output escaped");
                if (!success && mode is not "cancel")
                    Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() ==
                        (mode is "budget" or "metadata" ? "ResourceLimit" : mode == "unknown" ? "UnsupportedMethod" : mode == "existing" ? "OutputCollision" : mode == "missing" ? "PasswordRequired" : password is not null ? "AuthenticationOrDataError" : "CorruptArchive"), "ZIP fixed failure category");
                if (success && operation == "open")
                {
                    var entries = frames.Where(f => f.GetProperty("op").GetString() == "entry_batch").SelectMany(f => f.GetProperty("entries").EnumerateArray().ToArray()).ToArray();
                    Require(entries.Length == 3 && entries.Select(e => e.GetProperty("ordinal").GetInt64()).SequenceEqual(new long[] { 0, 1, 2 }), "ZIP original ordinals");
                }
                if (success && operation == "test") Require(frames.Single(f => f.GetProperty("op").GetString() == "complete").GetProperty("bytes_verified").GetUInt64() == 1050, "ZIP verified bytes");
                Require(terminal.FilesCommitted == (success && operation == "extract" ? 2 : 0), "ZIP committed count");
                if (success && operation == "extract")
                {
                    var expected = Encoding.UTF8.GetBytes(password is null ? "独立 ZIP method fixture\n" : "独立 ZIP crypto fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
                    Require(File.ReadAllBytes(final).SequenceEqual(expected) && File.ReadAllBytes(Path.Combine(root.FullName, "empty.bin")).Length == 0 && Directory.Exists(Path.Combine(root.FullName, "empty-dir")), "ZIP exact native output");
                    Require(barriers == 2, "ZIP native source checks");
                }
                else if (mode == "existing") Require(File.ReadAllText(final) == "user bytes", "existing target altered");
                else Require(Directory.GetFiles(root.FullName, "*", SearchOption.AllDirectories).Length == 0, "failed ZIP published output");
                Require(Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0 && File.ReadAllBytes(source).SequenceEqual(bytes), "ZIP partial/source cleanup");
            }
        }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
