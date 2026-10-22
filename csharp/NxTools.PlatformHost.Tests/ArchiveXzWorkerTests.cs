using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: explicit independent small fixtures, GUID-owned tree, exclusively copied helper,
// real owned Windows Job/native output. No installation, registry, Shell or user paths.
internal static class ArchiveXzWorkerTests
{
    private static void Require(bool value, string detail)
    { if (!value) throw new InvalidOperationException(detail); }

    private static byte[] Payload() => Encoding.UTF8.GetBytes("独立 gzip fixture\n")
        .Concat(Enumerable.Range(0, 256).Select(i => (byte)i)).ToArray();

    private static async Task WithWorker(Func<DirectoryInfo, string, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and independent CPython stream fixtures required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-xz-read-");
        var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned, fixtures); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }

    private static async Task<(ArchiveWorkerEvent Terminal, JsonElement[] Frames)> Run(
        string operation, string source, string root, object limits,
        Action<ArchiveWorkerRunner, Guid>? configure = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        var id = Guid.NewGuid();
        configure?.Invoke(runner, id);
        try
        {
            Require(runner.Start(new(id, operation, source, 0, limits,
                OutputRoot: operation == "extract" ? root : null)), "XZ admission");
            var clock = Stopwatch.StartNew();
            while (!events.Any(e => e.Kind == "exited"))
            { Require(clock.ElapsedMilliseconds < 20000, "XZ worker deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync() && runner.ActiveCount == 0, "XZ Job/pipe drainage");
            var terminal = events.Single(e => e.Kind == "exited");
            Require(terminal.CleanupComplete, "XZ cleanup incomplete");
            var frames = events.Where(e => e.Kind == "frame" && e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
            Require(events.All(e => e.OperationId == id), "XZ operation correlation");
            Require(!frames.Any(f => f.GetProperty("op").GetString() == "native_call"), "private payload escaped pump");
            Require(Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "XZ partial leak");
            return (terminal, frames);
        }
        finally { runner.Stop(); Require(await runner.DrainAsync(), "XZ failure-path drainage"); }
    }

    public static Task OpenTestExtract() => WithWorker(async (owned, fixtures) =>
    {
        foreach (var mode in new[] { "single", "multi", "padding", "member_padding", "tar.xz", "TXZ" })
        {
            var compound = mode is "tar.xz" or "TXZ";
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, compound ? "independent.tar.xz" : mode is "multi" or "member_padding" ? "multi.xz" : "independent.xz"));
            if (mode == "padding") bytes = bytes.Concat(new byte[8]).ToArray();
            if (mode == "member_padding")
            {
                var second = Enumerable.Range(6, bytes.Length - 11).First(i => bytes.AsSpan(i, 6).SequenceEqual(new byte[] { 0xfd, 0x37, 0x7a, 0x58, 0x5a, 0 }));
                bytes = bytes.Take(second).Concat(new byte[4]).Concat(bytes.Skip(second)).ToArray();
            }
            var source = Path.Combine(owned.FullName, compound ? "payload." + mode : mode + ".xz");
            File.WriteAllBytes(source, bytes);
            var expected = mode is "multi" or "member_padding" ? "first\nsecond\n"u8.ToArray() : Payload();
            foreach (var operation in new[] { "open", "test", "extract" })
            {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var (terminal, frames) = await Run(operation, source, root.FullName,
                    new { max_entries = 100, max_entry_bytes = Payload().Length, max_output_bytes = 1048576 });
                Require(terminal.Category == "success" && terminal.FilesCommitted == (operation == "extract" ? 1 : 0), "XZ normal terminal: " + mode + "/" + operation);
                var completeFrames = frames.Where(f => f.GetProperty("op").GetString() == "complete").ToArray();
                Require(completeFrames.Length == 1, "complete count: " + mode + "/" + operation + "/" + string.Join(",", frames.Select(f => f.GetProperty("op").GetString())));
                var complete = completeFrames[0];
                if (operation == "open")
                {
                    var batches = frames.Where(f => f.GetProperty("op").GetString() == "entry_batch").ToArray();
                    Require(batches.Length > 0 && batches.All(batch => batch.GetProperty("format").GetString() == (compound ? "TarXz" : "Xz")), "XZ format");
                    var entries = batches.SelectMany(batch => batch.TryGetProperty("entries", out var items) ? items.EnumerateArray().ToArray() : Array.Empty<JsonElement>()).ToArray();
                    Require(entries.Length == (compound ? 3 : 1), "XZ entry count");
                    if (!compound) Require(entries[0].GetProperty("size").ValueKind == JsonValueKind.Null &&
                        entries[0].GetProperty("packed_size").GetInt64() == bytes.Length &&
                        entries[0].GetProperty("name").GetString() == mode, "XZ truthful metadata");
                }
                else Require(complete.GetProperty("files_verified").GetInt64() == 1 &&
                    complete.GetProperty("bytes_verified").GetInt64() == expected.Length, "XZ exact verified count");
                if (operation == "extract")
                {
                    Require(terminal.CommitState == "Committed", "XZ publication state");
                    var files = Directory.GetFiles(root.FullName, "*", SearchOption.AllDirectories);
                    Require(files.Length == 1 && File.ReadAllBytes(files[0]).SequenceEqual(expected), "independent XZ exact bytes/files");
                    if (compound) Require(Directory.Exists(Path.Combine(root.FullName, "资料", "empty")) &&
                        !File.Exists(Path.Combine(root.FullName, "link")), "TAR.XZ directory/link policy");
                }
                else Require(Directory.GetFileSystemEntries(root.FullName).Length == 0, "read-only operation created output");
                Require(File.ReadAllBytes(source).SequenceEqual(bytes), "XZ source changed");
            }
        }
    });

    private static uint Crc32(ReadOnlySpan<byte> bytes)
    {
        var crc = uint.MaxValue;
        foreach (var value in bytes)
        {
            crc ^= value;
            for (var bit = 0; bit < 8; bit++) crc = (crc >> 1) ^ ((crc & 1) == 0 ? 0u : 0xedb88320u);
        }
        return ~crc;
    }

    public static Task DamageAndBudgets() => WithWorker(async (owned, fixtures) =>
    {
        foreach (var compound in new[] { false, true })
        foreach (var mode in new[] { "header", "index", "footer", "payload", "truncated", "garbage", "bad_padding", "work", "metadata", "output", "entry", "members" })
        {
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, compound ? "independent.tar.xz" : mode == "members" ? "multi.xz" : "independent.xz"));
            var footer = bytes.Length - 12;
            var index = footer - checked(((int)BitConverter.ToUInt32(bytes, footer + 4) + 1) * 4);
            switch (mode)
            {
                case "header": bytes[8] ^= 1; break;
                case "index": bytes[index + 1] ^= 1; break;
                case "footer": bytes[footer] ^= 1; break;
                case "payload": bytes[34] ^= 0x40; break;
                case "truncated": Array.Resize(ref bytes, bytes.Length - 3); break;
                case "garbage": bytes = bytes.Concat("garbage"u8.ToArray()).ToArray(); break;
                case "bad_padding": bytes = bytes.Concat(new byte[1]).ToArray(); break;
                case "work":
                    // Admit a 64MiB Job work reservation, then reject a huge declared dictionary before allocation.
                    bytes[16] = 40;
                    var length = (bytes[12] + 1) * 4;
                    BitConverter.GetBytes(Crc32(bytes.AsSpan(12, length - 4))).CopyTo(bytes, 12 + length - 4);
                    break;
            }
            var source = Path.Combine(owned.FullName, "damaged" + (compound ? ".txz" : ".xz"));
            File.WriteAllBytes(source, bytes);
            foreach (var operation in new[] { "open", "test", "extract" })
            {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var preserved = Path.Combine(root.FullName, "unrelated.txt"); File.WriteAllText(preserved, "preserved");
                var budget = mode is "work" or "metadata" or "output" or "entry" or "members";
                var expectedError = budget ? "ResourceLimit" : "CorruptArchive";
                var (terminal, frames) = await Run(operation, source, root.FullName, new {
                    max_working_bytes = mode == "work" ? 67108864 : 536870912,
                    max_metadata_bytes = mode == "metadata" ? 1 : 1048576,
                    max_output_bytes = mode == "output" ? (compound ? 512 : 1) : 1048576,
                    max_entry_bytes = mode == "entry" ? 1 : 1048576,
                    max_entries = mode == "members" ? 1 : 100 });
                if (operation == "open" && ((!compound && mode == "payload") || (compound && mode == "entry")))
                    Require(terminal.Category == "success", "metadata-only admission contract");
                else
                {
                    var failures = frames.Where(f => f.GetProperty("op").GetString() == "failure").ToArray();
                    Require(failures.Length == 1, "failure count: " + compound + "/" + mode + "/" + operation + "/" + terminal.Category + "/" + string.Join(",", frames.Select(f => f.GetProperty("op").GetString())));
                    var failure = failures[0];
                    Require(failure.GetProperty("error_code").GetString() == expectedError &&
                        terminal.Category == (budget ? "resource_limit" : "worker_failure"), "XZ typed failure: " + compound + "/" + mode + "/" + operation + "/" + terminal.Category + "/" + failure);
                }
                Require(terminal.FilesCommitted == 0 && File.ReadAllText(preserved) == "preserved" &&
                    Directory.GetFileSystemEntries(root.FullName).Length == 1, "XZ failure published output");
                Require(File.ReadAllBytes(source).SequenceEqual(bytes), "failure changed compressed source");
            }
        }
    });

    public static Task CancellationAndSourceProtection() => WithWorker(async (owned, fixtures) =>
    {
        foreach (var compound in new[] { false, true })
        foreach (var mode in new[] { "normal", "cancel", "existing", "source_busy" })
        {
            var input = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var source = Path.Combine(input.FullName, compound ? "payload.txz" : "payload.xz");
            var bytes = File.ReadAllBytes(Path.Combine(fixtures, compound ? "independent.tar.xz" : "independent.xz"));
            File.WriteAllBytes(source, bytes);
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var final = compound ? Path.Combine(root.FullName, "资料", "payload.bin") : Path.Combine(root.FullName, "payload");
            if (mode == "existing") { Directory.CreateDirectory(Path.GetDirectoryName(final)!); File.WriteAllText(final, "existing"); }
            using var sourceWriter = mode == "source_busy" ? new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete) : null;
            var checkpoints = 0;
            var (terminal, _) = await Run("extract", source, root.FullName, new { max_entries = 100 }, (runner, id) =>
            {
                runner.OutputCommitCheckpointProbe = stage =>
                {
                    if (stage != "before_flush") return;
                    checkpoints++;
                    try { using var writer = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source lease writable"); }
                    catch (IOException) { }
                    try { File.Move(source, source + ".moved"); File.Move(source + ".moved", source); throw new InvalidOperationException("source lease renameable"); }
                    catch (IOException) { }
                    try { Directory.Move(input.FullName, input.FullName + ".moved"); Directory.Move(input.FullName + ".moved", input.FullName); throw new InvalidOperationException("source parent renameable"); }
                    catch (IOException) { }
                    if (mode == "cancel") Require(runner.Cancel(id), "XZ cancellation admission");
                };
            });
            Require(terminal.Category == (mode == "normal" ? "success" : mode == "cancel" ? "cancelled" : "worker_failure") &&
                terminal.FilesCommitted == (mode == "normal" ? 1 : 0), "XZ source/cancel terminal: " + compound + "/" + mode + "/" + terminal.Category);
            Require(checkpoints == (mode == "source_busy" ? 0 : 1), "XZ native commit boundary: " + compound + "/" + mode + "/" + checkpoints);
            Require(mode == "existing" ? File.ReadAllText(final) == "existing" : mode == "normal" ? File.ReadAllBytes(final).SequenceEqual(Payload()) : !File.Exists(final), "XZ source/cancel final");
            sourceWriter?.Dispose();
            Require(File.ReadAllBytes(source).SequenceEqual(bytes), "source protection changed bytes");
            using var released = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.None);
        }
    });
}
