using System.Collections.Concurrent;
using System.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveCodecWorkerTests
{
    public static async Task IndependentRarAndSevenzReadTest()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtureRoot = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(source) || !File.Exists(source) || string.IsNullOrEmpty(fixtureRoot))
            throw new TestSkippedException("Explicit isolated-codecs Rust worker and independent fixture directory required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(installed)) throw new InvalidOperationException("Fixture will not replace an existing worker.");
        var created = false;
        try
        {
            File.Copy(source, installed, false); created = true;
            var scenarios = new[] { ("rar4.rar", (string?)null, "VERSION", 11UL), ("rar5.rar", (string?)null, "", 7UL),
                ("lzma.7z", (string?)null, "file.txt", 15UL), ("solid.7z", (string?)null, "", 0UL),
                ("non-solid.7z", (string?)null, "", 0UL), ("rar4-encrypted.rar", "unrar", ".gitignore", 18UL),
                ("rar5-header-encrypted.rar", "password", "", 0UL), ("encrypted.7z", "sevenz-rust", "", 0UL) };
            foreach (var (name, password, expectedName, expectedSize) in scenarios)
            {
                foreach (var operation in new[] { "open", "test" })
                {
                    var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                    using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                    var request = new ArchiveWorkerRequest(Guid.NewGuid(), operation, Path.Combine(fixtureRoot, name), 0,
                        new { max_entries = 10000 }, password, TimeoutMs: 10000);
                    if (!runner.Start(request)) throw new InvalidOperationException("Codec fixture admission rejected.");
                    var clock = Stopwatch.StartNew();
                    while (!events.Any(e => e.Kind == "exited"))
                    { if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("Codec fixture deadline."); await Task.Delay(20); }
                    var terminal = events.Last();
                    if (terminal.Category != "success" || !terminal.CleanupComplete)
                    {
                        var code = events.FirstOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure")?.Frame;
                        var category = code?.GetProperty("error_code").GetString();
                        throw new InvalidOperationException($"Codec scenario {name}/{operation} failed: {terminal.Category}/{category}");
                    }
                    if (operation == "open")
                    {
                        var entries = events.Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "entry_batch")
                            .SelectMany(e => e.Frame!.Value.GetProperty("entries").EnumerateArray()).ToArray();
                        if (entries.Length == 0) throw new InvalidOperationException("Independent codec lost entries.");
                        if (expectedName.Length != 0 && (entries[0].GetProperty("name").GetString() != expectedName || entries[0].GetProperty("size").GetUInt64() != expectedSize))
                            throw new InvalidOperationException($"Independent codec metadata mismatch: {name}/{operation}; expected size {expectedSize}, actual size {entries[0].GetProperty("size").GetUInt64()}.");
                    }
                    if (!await runner.DrainAsync()) throw new InvalidOperationException("Codec worker did not drain.");
                }
            }
        }
        finally { if (created) File.Delete(installed); }
    }
    public static async Task X86BcjLzma2Interop()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Matching worker and independent BCJ fixture required.");
        var fixture = Path.Combine(fixtures, "interop"); var source = Path.Combine(fixture, "x86-bcj-lzma2.7z");
        using var manifest = System.Text.Json.JsonDocument.Parse(File.ReadAllText(Path.Combine(fixture, "manifest.json")));
        var expected = File.ReadAllBytes(Path.Combine(fixture, "x86.bin"));
        var hash = Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(File.ReadAllBytes(source))).ToLowerInvariant();
        if (hash != manifest.RootElement.GetProperty("files").GetProperty("x86-bcj-lzma2.7z").GetProperty("sha256").GetString() || expected.Length != 4096)
            throw new InvalidOperationException("independent BCJ input identity");
        var listing = File.ReadAllText(Path.Combine(fixture, "producer-list.txt"));
        if (!listing.Contains("Method = LZMA2:12 BCJ") || !listing.Contains("Method = BCJ LZMA2:12")) throw new InvalidOperationException("actual x86 BCJ+LZMA2 coder proof missing");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(helper)) throw new InvalidOperationException("refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-bcj-"); var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            foreach (var operation in new[] { "test", "extract" }) {
                var output = operation == "extract" ? Path.Combine(owned.FullName, "output") : null;
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                if (!runner.Start(new(Guid.NewGuid(), operation, source, 0, new { max_entries = 100 }, OutputRoot: output))) throw new InvalidOperationException("BCJ original admission");
                var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("BCJ deadline"); await Task.Delay(20); }
                var terminal = events.Single(e => e.Kind == "exited");
                var complete = events.Single(e => e.Frame is { } f && f.GetProperty("op").GetString() == "complete").Frame!.Value;
                if (!await runner.DrainAsync() || !terminal.CleanupComplete || terminal.Category != "success" || terminal.FilesCommitted != (operation == "extract" ? 1 : 0) ||
                    complete.GetProperty("files_verified").GetUInt64() != 1 || complete.GetProperty("bytes_verified").GetUInt64() != 4096) throw new InvalidOperationException("BCJ counts/cleanup");
                if (output is not null && (Directory.GetFiles(output).Length != 1 || !File.ReadAllBytes(Path.Combine(output, "x86.bin")).SequenceEqual(expected) || Directory.GetFiles(output, ".nxpart-*").Length != 0))
                    throw new InvalidOperationException("BCJ independent native bytes/owned cleanup");
                using var released = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
        } finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    public static async Task SolidPackedMetadata()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Matching worker and independent solid/non-solid fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); if (File.Exists(helper)) throw new InvalidOperationException("refuse existing helper");
        var copied = false;
        try {
            File.Copy(worker, helper); copied = true;
            foreach (var solid in new[] { true, false }) {
                var source = Path.Combine(fixtures, solid ? "solid.7z" : "non-solid.7z");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                if (!runner.Start(new(Guid.NewGuid(), "open", source, 0, new { max_entries = 100 }))) throw new InvalidOperationException("solid metadata admission");
                var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("solid metadata deadline"); await Task.Delay(20); }
                var terminal = events.Single(e => e.Kind == "exited");
                if (!await runner.DrainAsync() || !terminal.CleanupComplete || terminal.Category != "success" || terminal.FilesCommitted != 0) throw new InvalidOperationException("metadata read-only cleanup");
                var entries = events.Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == "entry_batch").SelectMany(e => e.Frame!.Value.GetProperty("entries").EnumerateArray()).ToArray();
                if (entries.Length != 3) throw new InvalidOperationException("solid/non-solid exact entries");
                foreach (var (name, size, packed) in new[] { ("test/test1.txt", 3447UL, 1446UL), ("test/test2.txt", 2654UL, 1232UL) }) {
                    var entry = entries.Single(e => e.GetProperty("name").GetString()!.Replace('\\', '/') == name);
                    if (entry.GetProperty("size").GetUInt64() != size || (solid ? entry.GetProperty("packed_size").ValueKind != System.Text.Json.JsonValueKind.Null : entry.GetProperty("packed_size").GetUInt64() != packed))
                        throw new InvalidOperationException("shared solid block must remain unknown; non-solid must match independent packed bytes");
                }
                using var released = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
        } finally { if (copied) File.Delete(helper); }
    }
    public static async Task PasswordBudgetCorruptionAndCancel()
    {
        var source = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtureRoot = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(source) || !File.Exists(source) || string.IsNullOrEmpty(fixtureRoot))
            throw new TestSkippedException("Explicit isolated codec fixture inputs required.");
        var installed = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        if (File.Exists(installed)) throw new InvalidOperationException("Fixture will not replace an existing worker.");
        var ownedRoot = Directory.CreateTempSubdirectory("archive-codec-negative-").FullName;
        var created = false;
        try
        {
            File.Copy(source, installed, false); created = true;
            async Task Verify(string name, string operation, string? password, object limits, string[] expected)
            {
                var events = new ConcurrentQueue<ArchiveWorkerEvent>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var path = Path.IsPathRooted(name) ? name : Path.Combine(fixtureRoot, name);
                var request = new ArchiveWorkerRequest(Guid.NewGuid(), operation, path, 0, limits, password, TimeoutMs: 10000);
                if (!runner.Start(request)) throw new InvalidOperationException("negative codec admission");
                var clock = Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited"))
                { if (clock.ElapsedMilliseconds > 15000) throw new InvalidOperationException("negative codec deadline"); await Task.Delay(20); }
                var frame = events.SingleOrDefault(e => e.Frame is { } f && f.GetProperty("op").GetString() == "failure")?.Frame;
                var code = frame?.GetProperty("error_code").GetString();
                if (code is null || !expected.Contains(code) || !events.Last().CleanupComplete)
                    throw new InvalidOperationException($"negative codec {name}/{operation} expected {string.Join('/', expected)}, got {code}/{events.Last().Category}");
                if (!await runner.DrainAsync()) throw new InvalidOperationException("negative codec cleanup");
            }
            var defaults = new { max_entries = 10000 };
            await Verify("rar4-encrypted.rar", "test", null, defaults, ["PasswordRequired"]);
            await Verify("rar5-header-encrypted.rar", "open", null, defaults, ["PasswordRequired"]);
            await Verify("rar5-header-encrypted.rar", "test", "incorrect", defaults, ["WrongPassword", "AuthenticationOrDataError"]);
            await Verify("encrypted.7z", "test", null, defaults, ["PasswordRequired"]);
            await Verify("encrypted.7z", "test", "incorrect", defaults, ["AuthenticationOrDataError"]);
            await Verify("rar4.rar", "test", null, new { max_entry_bytes = 1 }, ["ResourceLimit"]);
            await Verify("lzma.7z", "test", null, new { max_output_bytes = 1 }, ["ResourceLimit"]);
            var damaged = Path.Combine(ownedRoot, "damaged.7z");
            var bytes = File.ReadAllBytes(Path.Combine(fixtureRoot, "lzma.7z"));
            bytes[40] ^= 0x80; File.WriteAllBytes(damaged, bytes);
            await Verify(damaged, "test", null, defaults, ["CorruptArchive", "AuthenticationOrDataError"]);
            var cancellation = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var cancelRunner = ArchiveWorkerRunner.ForPackage(cancellation.Enqueue);
            var cancelRequest = new ArchiveWorkerRequest(Guid.NewGuid(), "test", Path.Combine(fixtureRoot,"encrypted.7z"),0,defaults,"sevenz-rust");
            if (!cancelRunner.Start(cancelRequest) || !cancelRunner.Cancel(cancelRequest.OperationId) || !await cancelRunner.DrainAsync())
                throw new InvalidOperationException("actual codec cancel cleanup");
            if (cancellation.Last().Category != "cancelled" || !cancellation.Last().CleanupComplete)
                throw new InvalidOperationException("actual codec cancellation terminal");
        }
        finally { if (created) File.Delete(installed); Directory.Delete(ownedRoot,true); }
    }
}
