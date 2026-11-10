using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned source copies, original worker/Job/cache/output only; no Shell or system state.
internal static class ArchiveCompatibilityTests
{
    private static void Check(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string Root()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (worker is null || !File.Exists(worker)) throw new TestSkippedException("Explicit matching worker required");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Scratch required");
        var root = Path.Combine(scratch, "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
        return root;
    }
    private static ProtocolEnvelope Entry(ArchiveEntryCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = command }, command.OperationId);
    private static async Task<ArchiveJobTerminal> Job(string root, ArchiveJobRequest request)
    {
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
        Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId)).Success, "original Job admission");
        while (true) {
            var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Check(item.Prompt is null, "unexpected prompt: " + item.Prompt?.Kind);
            if (item.Terminal is { } terminal) return terminal;
        }
    }
    private static async Task<(ArchiveBrowserContext Context, List<ArchiveReadEntry> Entries)> Browse(string root, string path)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var command = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(command);
        Check(read.Dispatch(command).Success, "original Read admission");
        var entries = new List<ArchiveReadEntry>();
        while (true) {
            var envelope = await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
            var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (item.Batch is { } batch) {
                entries.AddRange(batch.Entries);
                Check(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original metadata credit");
            }
            if (item.Terminal is { } terminal) {
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "browse: " + terminal.Code);
                return (terminal.Browser!, entries);
            }
        }
    }
    private static async Task Preview(string root, string path, ArchiveBrowserContext context, ulong ordinal, byte[] expected, bool interaction)
    {
        await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
        var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
            Action = "preview_text", Path = path, Browser = context, Ordinal = ordinal, CopyMotw = false, PasswordInteraction = interaction };
        var outer = await entries.Dispatch(Entry(command)).WaitAsync(TimeSpan.FromSeconds(40));
        var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
        Check(outer.Success && result.Success && result.CleanupComplete && result.Path is not null, "small selected preview: " + result.Code);
        Check(File.ReadAllBytes(result.Path!).SequenceEqual(expected), "exact selected preview bytes");
        Check((await entries.Dispatch(Entry(command with { Action = "release", Path = null, Browser = null, Ordinal = null, CopyMotw = null, PasswordInteraction = false, LeaseId = result.LeaseId }))).Success, "preview lease release");
        Check(!File.Exists(result.Path), "released output removed");
    }
    internal static async Task SevenzPreviewSourceBudget()
    {
        var root = Root();
        try {
            var source = Path.Combine(root, "source"); Directory.CreateDirectory(source);
            var small = Enumerable.Repeat((byte)'p', 80).ToArray();
            File.WriteAllBytes(Path.Combine(source, "small.txt"), small);
            var random = new byte[192 * 1024]; new Random(624).NextBytes(random);
            File.WriteAllBytes(Path.Combine(source, "large.bin"), random);
            var created = await Job(root, new() { Kind = "create", Paths = [source], Destination = root,
                Create = new() { Format = "seven_z", Name = "preview.7z", Level = "fast", Solid = false, EncryptHeader = false, StripSingleRoot = true, ExcludePatterns = [], TestAfterCreate = false } });
            Check(created.Outcome == "succeeded" && created.CleanupComplete, "7z fixture creation: " + created.Code);
            var path = Path.Combine(root, "preview.7z");
            Check(new FileInfo(path).Length > 64 * 1024, "encoded source exceeds text output quota");
            var browse = await Browse(root, path);
            var ordinal = browse.Entries.Single(entry => entry.Name == "small.txt").Ordinal;
            await Preview(root, path, browse.Context, ordinal, small, false);
            await Preview(root, path, browse.Context, ordinal, small, true);
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1,
                Action = "preview_text", Path = path, Browser = browse.Context, Ordinal = browse.Entries.Single(entry => entry.Name == "large.bin").Ordinal, CopyMotw = false, PasswordInteraction = true };
            var outer = await entries.Dispatch(Entry(command)).WaitAsync(TimeSpan.FromSeconds(40));
            var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
            Check(!outer.Success && !result.Success && result.Code == "resource_limit" && result.Path is null && result.CleanupComplete, "selected output cap remains enforced");
            using var released = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        } finally { Directory.Delete(root, true); }
    }
    internal static async Task AdditionalFormatsCorpus()
    {
        var corpus = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_COMPAT_CORPUS");
        if (corpus is null || !File.Exists(Path.Combine(corpus, "manifest.json"))) throw new TestSkippedException("Explicit archive corpus required");
        var root = Root();
        try {
            foreach (var fixture in new[] { "image/cab-mszip.cab", "image/cab-lzx.cab", "image/iso9660-only.iso", "image/iso-joliet.iso", "image/iso-rockridge.iso", "image/iso-udf.iso", "image/wim-lzms.wim", "image/ar-archive.a", "image/cpio-newc.cpio", "image/cpio-bsdtar.cpio", "stream/single.lzma", "stream/single.lz4", "stream/single-block.lz4", "stream/single.br" }) {
                var path = Path.Combine(root, Path.GetFileName(fixture));
                File.Copy(Path.Combine(corpus, "samples", fixture), path);
                var browse = await Browse(root, path);
                Check(browse.Entries.Count > 0, fixture + " original catalog");
                var tested = await Job(root, new() { Kind = "test", Paths = [path] });
                Check(tested.Outcome == "succeeded" && tested.CleanupComplete, fixture + " original Test: " + tested.Code);
                var destination = Path.Combine(root, Path.GetFileName(fixture) + "-out"); Directory.CreateDirectory(destination);
                var extracted = await Job(root, new() { Kind = "extract", Paths = [path], Destination = destination,
                    Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false } });
                if (fixture == "image/cpio-bsdtar.cpio") {
                    Check(extracted.Outcome == "failed" && extracted.Code == "unsafe_path" && extracted.FilesCommitted == 0 && extracted.CleanupComplete, "producer's invalid Windows names are not silently rewritten");
                    var safeMember = browse.Entries.Single(entry => entry.Name == "content/readme.txt");
                    await Preview(root, path, browse.Context, safeMember.Ordinal, File.ReadAllBytes(Path.Combine(corpus, "src", "content", "readme.txt")), true);
                    continue;
                }
                Check(extracted.Outcome is "succeeded" or "succeeded_with_warnings" && extracted.CleanupComplete, fixture + " native Extract: " + extracted.Code);
                var actualFiles = Directory.GetFiles(destination, "*", SearchOption.AllDirectories);
                Check(actualFiles.Length > 0 && (ulong)actualFiles.Length == extracted.FilesCommitted, fixture + " actual committed files");
                VerifyCorpusPayloads(corpus, fixture, destination, actualFiles);
                using var released = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            }
            await CabPreviewSourceBudget();
        } finally { Directory.Delete(root, true); }
    }
    private static async Task CabPreviewSourceBudget()
    {
        var root = Root();
        try {
            var small = Enumerable.Repeat((byte)'p', 80).ToArray();
            File.WriteAllBytes(Path.Combine(root, "small.txt"), small);
            File.WriteAllBytes(Path.Combine(root, "large.bin"), new byte[128 * 1024]);
            var ddf = Path.Combine(root, "fixture.ddf");
            File.WriteAllLines(ddf, [".OPTION EXPLICIT", ".Set CabinetNameTemplate=preview.cab", $".Set DiskDirectoryTemplate=\"{root}\"", ".Set CompressionType=MSZIP", ".Set Cabinet=on", ".Set Compress=on", ".Set MaxDiskSize=0", $"\"{Path.Combine(root, "small.txt")}\" small.txt", $"\"{Path.Combine(root, "large.bin")}\" large.bin"]);
            var start = new System.Diagnostics.ProcessStartInfo(Path.Combine(Environment.SystemDirectory, "makecab.exe")) { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true, WorkingDirectory = root };
            start.ArgumentList.Add("/F"); start.ArgumentList.Add(ddf);
            using var child = System.Diagnostics.Process.Start(start)!;
            var output = child.StandardOutput.ReadToEndAsync(); var error = child.StandardError.ReadToEndAsync();
            try { await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(20)); }
            catch (TimeoutException) { child.Kill(); await child.WaitForExitAsync(); throw; }
            await output; await error;
            Check(child.ExitCode == 0, "independent bounded CAB fixture creation");
            var path = Path.Combine(root, "preview.cab");
            var browse = await Browse(root, path);
            Check(browse.Entries.Single(entry => entry.Name == "large.bin").Size == 128 * 1024, "unselected declared bytes exceed text quota");
            await Preview(root, path, browse.Context, browse.Entries.Single(entry => entry.Name == "small.txt").Ordinal, small, true);
            await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
            var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "preview_text", Path = path, Browser = browse.Context, Ordinal = browse.Entries.Single(entry => entry.Name == "large.bin").Ordinal, CopyMotw = false, PasswordInteraction = true };
            var outer = await entries.Dispatch(Entry(command)).WaitAsync(TimeSpan.FromSeconds(40));
            var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
            Check(!outer.Success && result.Code == "resource_limit" && result.Path is null && result.CleanupComplete, "CAB selected output quota remains unchanged");
        } finally { Directory.Delete(root, true); }
    }
    private static void VerifyCorpusPayloads(string corpus, string fixture, string destination, string[] actualFiles)
    {
        if (fixture.EndsWith(".iso", StringComparison.Ordinal)) {
            var expected = new[] { "readme.txt", "crlf.txt", "zero.txt", "images/logo.png", "docs/notes.md" }.Select(name => File.ReadAllBytes(Path.Combine(corpus, "src", "content", name))).ToList();
            if (fixture == "image/iso-joliet.iso") expected.Add(System.Text.Encoding.UTF8.GetBytes("中文内容\n"));
            Check(actualFiles.Length == expected.Count, fixture + " complete independent image members");
            foreach (var file in actualFiles) { var bytes = File.ReadAllBytes(file); var index = expected.FindIndex(value => value.SequenceEqual(bytes)); Check(index >= 0, fixture + " independent image bytes"); expected.RemoveAt(index); }
            return;
        }
        foreach (var file in actualFiles) {
            var relative = Path.GetRelativePath(destination, file).Replace('\\', '/');
            byte[] expected;
            if (fixture.StartsWith("stream/", StringComparison.Ordinal)) expected = File.ReadAllBytes(Path.Combine(corpus, "src", "stream", "text-512k.txt"));
            else if (relative == "odd.txt") expected = System.Text.Encoding.ASCII.GetBytes("odd length payload");
            else if (fixture.StartsWith("image/cab-", StringComparison.Ordinal)) expected = File.ReadAllBytes(Path.Combine(corpus, "src", "cab-stage", relative == "notes.md" ? "docs/notes.md" : relative));
            else if (fixture is "image/ar-archive.a" or "image/cpio-newc.cpio" or "image/wim-lzms.wim") expected = File.ReadAllBytes(Path.Combine(corpus, "src", "content", relative));
            else expected = File.ReadAllBytes(Path.Combine(corpus, "src", relative));
            Check(File.ReadAllBytes(file).SequenceEqual(expected), fixture + " independent bytes: " + relative);
        }
    }
    internal static async Task SevenzMethodsCorpus()
    {
        var corpus = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_COMPAT_CORPUS");
        if (corpus is null || !File.Exists(Path.Combine(corpus, "manifest.json"))) throw new TestSkippedException("Explicit archive corpus required");
        var root = Root();
        try {
            foreach (var name in new[] { "7z-ppmd.7z", "7z-bzip2.7z", "7z-deflate.7z", "7z-bcj2-exe.7z" }) {
                var path = Path.Combine(root, name);
                File.Copy(Path.Combine(corpus, "samples", "7z", name), path);
                var browse = await Browse(root, path);
                var expectedFiles = (ulong)browse.Entries.Count(entry => entry.Kind == "File");
                var expectedBytes = browse.Entries.Where(entry => entry.Kind == "File").Aggregate(0UL, (total, entry) => checked(total + entry.Size!.Value));
                Check(expectedFiles > 0, "independent method member catalog");
                var tested = await Job(root, new() { Kind = "test", Paths = [path] });
                Check(tested.Outcome == "succeeded" && tested.CleanupComplete && tested.FilesVerified == expectedFiles && tested.BytesVerified == expectedBytes, name + " complete Test: " + tested.Code);
            }
        } finally { Directory.Delete(root, true); }
    }
    internal static async Task SplitZipPreviewSourceBudget()
    {
        var root = Root();
        try {
            var source = Path.Combine(root, "source"); Directory.CreateDirectory(source);
            var small = Enumerable.Repeat((byte)'p', 80).ToArray();
            File.WriteAllBytes(Path.Combine(source, "small.txt"), small);
            var random = new byte[192 * 1024]; new Random(624).NextBytes(random);
            File.WriteAllBytes(Path.Combine(source, "large.bin"), random);
            var created = await Job(root, new() { Kind = "create", Paths = [source], Destination = root,
                Create = new() { Format = "zip", Method = "deflate", Name = "preview.zip", Level = "fast", VolumeBytes = 65536, StripSingleRoot = true, ExcludePatterns = [], TestAfterCreate = false } });
            Check(created.Outcome == "succeeded" && created.CleanupComplete && created.FilesCommitted > 1, "split ZIP fixture creation: " + created.Code);
            var path = Path.Combine(root, "preview.zip");
            var browse = await Browse(root, path);
            var ordinal = browse.Entries.Single(entry => entry.Name == "small.txt").Ordinal;
            await Preview(root, path, browse.Context, ordinal, small, true);
        } finally { Directory.Delete(root, true); }
    }
    internal static async Task PySevenzCorpus()
    {
        var corpus = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_COMPAT_CORPUS");
        if (corpus is null || !File.Exists(Path.Combine(corpus, "manifest.json"))) throw new TestSkippedException("Explicit archive corpus required");
        var root = Root();
        try {
            var path = Path.Combine(root, "py7zr.7z");
            File.Copy(Path.Combine(corpus, "samples", "7z", "7z-py7zr-producer.7z"), path);
            var tested = await Job(root, new() { Kind = "test", Paths = [path] });
            Check(tested.Outcome == "succeeded" && tested.CleanupComplete, "py7zr Test: " + tested.Code);
            var browse = await Browse(root, path);
            var expected = File.ReadAllBytes(Path.Combine(corpus, "src", "content", "readme.txt"));
            var ordinal = browse.Entries.Single(entry => entry.Name.Replace('\\', '/').EndsWith("/readme.txt", StringComparison.Ordinal)).Ordinal;
            await Preview(root, path, browse.Context, ordinal, expected, true);
        } finally { Directory.Delete(root, true); }
    }
    internal static async Task NumberedZipCorpus()
    {
        var corpus = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_COMPAT_CORPUS");
        if (corpus is null || !File.Exists(Path.Combine(corpus, "manifest.json"))) throw new TestSkippedException("Explicit archive corpus required");
        var root = Root();
        try {
            var family = Path.Combine(root, "family"); Directory.CreateDirectory(family);
            foreach (var part in Directory.GetFiles(Path.Combine(corpus, "samples", "split"), "zip-split-1m.zip.*")) File.Copy(part, Path.Combine(family, Path.GetFileName(part)));
            var path = Path.Combine(family, "zip-split-1m.zip.001");
            Check(Directory.GetFiles(family).Length == 5, "five independent 7-Zip segments");
            var browse = await Browse(root, path);
            Check(browse.Entries.Count(entry => entry.Kind == "File") == 2, "both original members");
            var tested = await Job(root, new() { Kind = "test", Paths = [path] });
            Check(tested.Outcome == "succeeded" && tested.BytesVerified == 20UL * 1024 * 1024 && tested.CleanupComplete, "numbered ZIP Test: " + tested.Code);
            var output = Path.Combine(root, "output"); Directory.CreateDirectory(output);
            var extracted = await Job(root, new() { Kind = "extract", Paths = [path], Destination = output,
                Extract = new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false } });
            Check(extracted.Outcome == "succeeded" && extracted.FilesCommitted == 2 && extracted.CleanupComplete, "numbered ZIP Extract: " + extracted.Code);
            foreach (var expected in Directory.GetFiles(Path.Combine(corpus, "src", "big"))) {
                var actual = Directory.GetFiles(output, Path.GetFileName(expected), SearchOption.AllDirectories).Single();
                Check(File.ReadAllBytes(actual).SequenceEqual(File.ReadAllBytes(expected)), "exact independent split payload");
            }
            using (var leases = ArchiveVolumeSources.Open(path, 128UL << 20, 100UL << 30)) {
                Check(leases.Paths.Count == 5, "all split source leases");
                try { using var writer = File.Open(Path.Combine(family, "zip-split-1m.zip.003"), FileMode.Open, FileAccess.Write, FileShare.ReadWrite); throw new InvalidOperationException("unprotected later volume"); } catch (IOException) { }
            }
            File.Delete(Path.Combine(family, "zip-split-1m.zip.002"));
            var missing = await Job(root, new() { Kind = "test", Paths = [path] });
            Check(missing.Outcome == "failed" && missing.Code == "missing_volume" && missing.CleanupComplete && missing.FilesCommitted == 0, "explicit missing volume: " + missing.Code);
        } finally { Directory.Delete(root, true); }
    }
}
