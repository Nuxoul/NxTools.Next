using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Tests;

// Reviewed GUID temporary trees, explicit independent fixtures and owned Rust Job only.
internal static class ArchiveZipReadRemediationTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private const string Password = "synthetic-private-ZIP-secret";
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
            => Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    private static async Task WithWorker(Func<DirectoryInfo, string, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit Rust worker and independent ZIP fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe"); Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-zip-remediation-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned, fixtures); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<(ArchiveWorkerEvent Terminal, JsonElement[] Frames)> Run(ArchiveWorkerRequest request, Action<ArchiveWorkerRunner>? setup = null, IPlatformLog? log = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log); setup?.Invoke(runner);
        Require(runner.Start(request), "ZIP remediation admission"); var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 20000, "ZIP remediation deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "ZIP remediation drain");
        var terminal = events.Single(e => e.Kind == "exited"); Require(terminal.CleanupComplete, "ZIP cleanup incomplete");
        return (terminal, events.Where(e => e.Kind == "frame" && e.Frame is not null).Select(e => e.Frame!.Value).ToArray());
    }
    public static Task DirectoryAndCommentBoundaries() => WithWorker(async (owned, fixtures) => {
        foreach (var mode in new[] { "normal", "corrupt", "unsupported" })
        {
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, "independent-directory.zip"));
            var central = Enumerable.Range(0, bytes.Length - 3).First(i => bytes.AsSpan(i, 4).SequenceEqual("PK\u0001\u0002"u8));
            var offset = 30 + BitConverter.ToUInt16(bytes, 26);
            if (mode == "corrupt") bytes[offset] = 255;
            if (mode == "unsupported") { bytes[8] = 222; bytes[central + 10] = 222; }
            var source = Path.Combine(owned.FullName, "directory.zip"); File.WriteAllBytes(source, bytes);
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))); var keep = Path.Combine(root.FullName, "user.txt"); File.WriteAllText(keep, "user");
            var (terminal, frames) = await Run(new(Guid.NewGuid(), "extract", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName));
            Require(terminal.Category == (mode == "normal" ? "success" : "worker_failure") && terminal.FilesCommitted == (mode == "normal" ? 1 : 0), "directory terminal: " + mode);
            if (mode == "normal") Require(Directory.Exists(Path.Combine(root.FullName, "folder")) && File.ReadAllText(Path.Combine(root.FullName, "keep.txt")) == "independent selected file\n", "valid directory native output");
            else { Require(!Directory.Exists(Path.Combine(root.FullName, "folder")), "invalid directory created"); Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == (mode == "corrupt" ? "CorruptArchive" : "UnsupportedMethod"), "directory typed failure"); }
            Require(File.ReadAllText(keep) == "user" && Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "directory user/partial preservation");
        }
        foreach (var budget in new[] { 1, 300000 })
        {
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, "empty.zip")); BitConverter.GetBytes((ushort)65535).CopyTo(bytes, bytes.Length - 2); bytes = bytes.Concat(Enumerable.Repeat((byte)'a', 65535)).ToArray();
            var source = Path.Combine(owned.FullName, "comment.zip"); File.WriteAllBytes(source, bytes);
            var (terminal, frames) = await Run(new(Guid.NewGuid(), "open", source, 0, new { max_metadata_bytes = budget }));
            Require(terminal.Category == (budget == 1 ? "resource_limit" : "success"), "comment budget terminal");
            if (budget == 1) Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == "ResourceLimit", "comment fixed budget error");
        }
    });
    public static Task AesReadsAndAuthentication() => WithWorker(async (owned, fixtures) => {
        foreach (var name in new[] { "independent-ae1-128.zip", "independent-ae1-192.zip", "independent-ae1-256.zip", "independent-ae2-128.zip", "independent-ae2-192.zip", "independent-ae2-256.zip", "independent-ae2-store.zip" })
        foreach (var mode in new[] { "normal", "missing", "wrong", "salt", "verifier", "cipher", "mac", "empty_mac", "directory_mac", "local_extra", "version", "strength", "crc", "entry", "total", "metadata", "cancel", "existing", "truncated", "io" })
        foreach (var operation in mode is "cancel" or "existing" or "io" ? new[] { "extract" } : mode == "normal" ? new[] { "open", "test", "extract" } : new[] { "test", "extract" })
        {
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, name));
            var central = Enumerable.Range(0, bytes.Length - 3).First(i => bytes.AsSpan(i, 4).SequenceEqual("PK\u0001\u0002"u8));
            var localExtra = 30 + BitConverter.ToUInt16(bytes, 26); var centralExtra = central + 46 + BitConverter.ToUInt16(bytes, central + 28);
            var offset = localExtra + BitConverter.ToUInt16(bytes, 28); var packed = (int)BitConverter.ToUInt32(bytes, 18); var salt = (bytes[localExtra + 8] + 1) * 4;
            if (mode == "salt") bytes[offset] ^= 1;
            if (mode == "verifier") bytes[offset + salt] ^= 1;
            if (mode == "cipher") bytes[offset + salt + 2] ^= 0x40;
            if (mode == "mac") bytes[offset + packed - 1] ^= 1;
            if (mode == "empty_mac") { var next = offset + packed; var data = next + 30 + BitConverter.ToUInt16(bytes, next + 26) + BitConverter.ToUInt16(bytes, next + 28); bytes[data + (int)BitConverter.ToUInt32(bytes, next + 18) - 1] ^= 1; }
            if (mode == "local_extra") bytes[localExtra + 4] = bytes[localExtra + 4] == 1 ? (byte)2 : (byte)1;
            if (mode == "directory_mac") bytes[central - 1] ^= 1;
            if (mode == "version") { bytes[localExtra + 4] = 3; bytes[centralExtra + 4] = 3; }
            if (mode == "strength") { bytes[localExtra + 8] = 4; bytes[centralExtra + 8] = 4; }
            if (mode == "crc") { bytes[14] ^= 1; bytes[central + 16] ^= 1; }
            if (mode == "truncated") { BitConverter.GetBytes((uint)(packed - 1)).CopyTo(bytes, 18); BitConverter.GetBytes((uint)(packed - 1)).CopyTo(bytes, central + 20); }
            var source = Path.Combine(owned.FullName, "private-sentinel.zip"); File.WriteAllBytes(source, bytes);
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))); var final = Path.Combine(root.FullName, "资料", "payload.bin");
            var user = Path.Combine(root.FullName, "user.txt"); File.WriteAllText(user, "unrelated user bytes");
            if (mode == "existing") { Directory.CreateDirectory(Path.GetDirectoryName(final)!); File.WriteAllText(final, "prior target"); }
            var id = Guid.NewGuid(); var capture = new CaptureLog(); var barriers = 0;
            var (terminal, frames) = await Run(new(id, operation, source, 0,
                new { max_entries = 100, max_entry_bytes = mode == "entry" ? 1 : 1048576, max_output_bytes = mode == "total" ? 1 : 1048576, max_metadata_bytes = mode == "metadata" ? 1 : 1048576 },
                Password: mode == "missing" ? null : mode == "wrong" ? "wrong-private-secret" : Password, OutputRoot: operation == "extract" ? root.FullName : null), runner => {
                    runner.OutputCommitCheckpointProbe = stage => { if (stage != "before_flush") return; barriers++; try { using var writer = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("AES source lease writable"); } catch (IOException) { } if (mode == "cancel") Require(runner.Cancel(id), "AES cancellation"); if (mode == "io") throw new IOException("controlled native output failure"); };
                }, capture);
            var success = mode == "normal" || mode == "crc" && name.Contains("ae2", StringComparison.Ordinal);
            var resource = mode is "entry" or "total" or "metadata";
            var expectedError = resource ? "ResourceLimit" : mode is "version" or "strength" ? "UnsupportedEncryption" : mode == "local_extra" ? "CorruptArchive" : mode == "missing" ? "PasswordRequired" : mode == "existing" ? "OutputCollision" : mode == "io" ? "IoFailure" : "AuthenticationOrDataError";
            Require(terminal.Category == (success ? "success" : mode == "cancel" ? "cancelled" : resource ? "resource_limit" : "worker_failure"), "AES terminal: " + name + "/" + mode + "/" + operation + "/" + terminal.Category);
            if (!success && mode != "cancel") Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == expectedError, "AES typed failure: " + name + "/" + mode);
            var committed = operation != "extract" ? 0 : success ? 2 : mode == "empty_mac" ? 1 : 0;
            Require(terminal.FilesCommitted == committed, "AES commit count");
            if (operation == "extract" && (success || mode == "empty_mac")) {
                var expected = System.Text.Encoding.UTF8.GetBytes("独立 ZIP crypto fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
                Require(File.ReadAllBytes(final).SequenceEqual(expected), "AES native exact bytes");
                if (success) Require(File.ReadAllBytes(Path.Combine(root.FullName, "empty.bin")).Length == 0 && Directory.Exists(Path.Combine(root.FullName, "empty-dir")) && barriers == 2, "AES empty/source barriers");
                else Require(!File.Exists(Path.Combine(root.FullName, "empty.bin")), "AES empty authentication bypass");
            } else if (mode == "existing") Require(File.ReadAllText(final) == "prior target", "AES prior target altered");
            else Require(!File.Exists(final), "AES invalid publication");
            Require(File.ReadAllText(user) == "unrelated user bytes" && File.ReadAllBytes(source).SequenceEqual(bytes) && Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "AES owned cleanup/source preservation");
            var diagnostic = JsonSerializer.Serialize(capture.Records.Select(record => new { record.Name, record.Id, record.Fields }));
            Require(!diagnostic.Contains(Password, StringComparison.Ordinal) && !diagnostic.Contains(source, StringComparison.Ordinal) && !diagnostic.Contains("payload.bin", StringComparison.Ordinal) && capture.Records.Count < 40 && capture.Records.All(record => record.Id == id), "AES diagnostic privacy/correlation/noise");
        }
    });
    public static Task EncryptedCombinations() => WithWorker(async (owned, fixtures) => {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(fixtures, "zip-combinations.json"))); Require(manifest.RootElement.GetArrayLength() == 33, "combination selected count");
        foreach (var item in manifest.RootElement.EnumerateArray())
        foreach (var operation in new[] { "test", "extract" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))); var name = item.GetProperty("name").GetString()!;
            var (terminal, frames) = await Run(new(Guid.NewGuid(), operation, Path.Combine(fixtures, name), 0, new { max_entries = 100 }, Password: Password, OutputRoot: operation == "extract" ? root.FullName : null));
            var success = item.GetProperty("error").ValueKind == JsonValueKind.Null;
            Require(terminal.Category == (success ? "success" : "worker_failure") && terminal.FilesCommitted == (success && operation == "extract" ? 2 : 0), "combination terminal: " + name);
            if (!success) Require(frames.Single(f => f.GetProperty("op").GetString() == "failure").GetProperty("error_code").GetString() == item.GetProperty("error").GetString(), "combination fixed error");
            if (success && operation == "extract") {
                var expected = System.Text.Encoding.UTF8.GetBytes("独立 ZIP method fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
                Require(File.ReadAllBytes(Path.Combine(root.FullName, "资料", "payload.bin")).SequenceEqual(expected) && File.ReadAllBytes(Path.Combine(root.FullName, "empty.bin")).Length == 0 && Directory.Exists(Path.Combine(root.FullName, "empty-dir")), "combination independent bytes");
            } else Require(Directory.GetFiles(root.FullName, "*", SearchOption.AllDirectories).Length == 0, "combination invalid output");
            Require(Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "combination partial leak");
        }
    });
    public static Task NoEosAndWorkingBudget() => WithWorker(async (owned, fixtures) => {
        foreach (var operation in new[] { "test", "extract" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var (terminal, _) = await Run(new(Guid.NewGuid(), operation, Path.Combine(fixtures, "independent-lzma-no-eos.zip"), 0, new { max_entries = 100 }, OutputRoot: operation == "extract" ? root.FullName : null));
            Require(terminal.Category == "success" && terminal.FilesCommitted == (operation == "extract" ? 1 : 0), "genuine noEOS native result");
            if (operation == "extract") {
                var expected = System.Text.Encoding.UTF8.GetBytes("独立 ZIP method fixture\n").Concat(Enumerable.Range(0, 1024).Select(i => (byte)i)).ToArray();
                Require(File.ReadAllBytes(Path.Combine(root.FullName, "资料", "payload.bin")).SequenceEqual(expected), "noEOS independent bytes");
            }
        }
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        Require(!runner.Start(new(Guid.NewGuid(), "test", Path.Combine(fixtures, "independent-ae2-256.zip"), 0, new { max_working_bytes = 524288 }, Password: Password)), "below-minimum AES work admitted");
        Require(runner.ActiveCount == 0, "rejected AES work launched child");
    });
}
