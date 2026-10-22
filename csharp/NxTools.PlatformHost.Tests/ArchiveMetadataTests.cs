using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only GUID-owned synthetic archives/ADS/output. The oracle uses independent Windows file info.
internal static class ArchiveMetadataTests
{
    private static void Require(bool condition, string detail)
    { if (!condition) throw new InvalidOperationException(detail); }

    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker))
            throw new TestSkippedException("Explicit actual Rust worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-metadata-");
        var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally {
            if (copied) File.Delete(helper);
            foreach (var file in owned.EnumerateFiles("*", SearchOption.AllDirectories)) file.Attributes &= ~FileAttributes.ReadOnly;
            foreach (var directory in owned.EnumerateDirectories("*", SearchOption.AllDirectories)) directory.Attributes &= ~FileAttributes.ReadOnly;
            owned.Delete(true);
        }
    }

    private static async Task<ArchiveWorkerEvent> Extract(string archive, string output, ArchiveExtractOptions? options = null, string category = "success", int files = 1)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        Require(runner.Start(new(Guid.NewGuid(), "extract", archive, 0, new { max_entries = 100 }, OutputRoot: output, ExtractOptions: options)), "metadata admission");
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited"))
        { Require(clock.ElapsedMilliseconds < 20000, "metadata worker deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "metadata worker cleanup");
        var terminal = events.Single(e => e.Kind == "exited");
        Require(terminal.Category == category && terminal.FilesCommitted == files && terminal.CleanupComplete,
            "metadata terminal: " + terminal.Category + "/" + terminal.NativeErrorCode + "/" + events.FirstOrDefault(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "failure")?.Frame?.GetProperty("error_code").GetString());
        Require(!Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "owned partial leak");
        return terminal;
    }

    public static Task ZipDosModified() => WithWorker(async owned => {
        var archive = Path.Combine(owned.FullName, "synthetic.zip");
        // ZIP DOS is an unspecified local wall clock, not UTC. This date is not a DST transition.
        var wall = new DateTime(2020, 2, 3, 4, 5, 6, DateTimeKind.Unspecified);
        Require(!TimeZoneInfo.Local.IsAmbiguousTime(wall) && !TimeZoneInfo.Local.IsInvalidTime(wall), "fixture requires unambiguous local time");
        using (var zip = ZipFile.Open(archive, ZipArchiveMode.Create)) {
            var entry = zip.CreateEntry("payload.txt", CompressionLevel.NoCompression);
            entry.LastWriteTime = new DateTimeOffset(wall, TimeZoneInfo.Local.GetUtcOffset(wall));
            using var writer = new StreamWriter(entry.Open()); writer.Write("synthetic metadata payload");
        }
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        await Extract(archive, output);
        var path = Path.Combine(output, "payload.txt");
        Require(File.ReadAllText(path) == "synthetic metadata payload", "metadata content changed");
        var actual = File.GetLastWriteTimeUtc(path);
        var expected = TimeZoneInfo.ConvertTimeToUtc(wall, TimeZoneInfo.Local);
        Require(actual == expected, "DOS modified oracle: expected " + expected.ToString("O") + ", actual " + actual.ToString("O"));
    });
    private static void RawZip(string path, byte[] extra, byte host = 0, uint attributes = 0, ushort date = 20515, ushort time = 8355)
    {
        // One stored ASCII 'x', CRC32=8cdc1683; independent fixed ZIP layout.
        var name = System.Text.Encoding.ASCII.GetBytes("payload.txt");
        using var writer = new BinaryWriter(File.Create(path));
        writer.Write(0x04034b50u); writer.Write((ushort)20); writer.Write((ushort)0); writer.Write((ushort)0);
        writer.Write(time); writer.Write(date); writer.Write(0x8cdc1683u); writer.Write(1u); writer.Write(1u);
        writer.Write((ushort)name.Length); writer.Write((ushort)extra.Length); writer.Write(name); writer.Write(extra); writer.Write((byte)'x');
        var central = (uint)writer.BaseStream.Position;
        writer.Write(0x02014b50u); writer.Write((ushort)((host << 8) | 20)); writer.Write((ushort)20);
        writer.Write((ushort)0); writer.Write((ushort)0); writer.Write(time); writer.Write(date);
        writer.Write(0x8cdc1683u); writer.Write(1u); writer.Write(1u); writer.Write((ushort)name.Length);
        writer.Write((ushort)extra.Length); writer.Write((ushort)0); writer.Write((ushort)0); writer.Write((ushort)0);
        writer.Write(attributes); writer.Write(0u); writer.Write(name); writer.Write(extra);
        var centralSize = (uint)writer.BaseStream.Position - central;
        writer.Write(0x06054b50u); writer.Write((ushort)0); writer.Write((ushort)0); writer.Write((ushort)1); writer.Write((ushort)1);
        writer.Write(centralSize); writer.Write(central); writer.Write((ushort)0);
    }
    private static byte[] NtfsExtra(long ticks)
    {
        using var bytes = new MemoryStream(); using var writer = new BinaryWriter(bytes);
        writer.Write((ushort)10); writer.Write((ushort)32); writer.Write(0u); writer.Write((ushort)1); writer.Write((ushort)24);
        writer.Write(ticks); writer.Write(0L); writer.Write(0L); return bytes.ToArray();
    }
    public static Task ZipUtcModified() => WithWorker(async owned => {
        var expected = new DateTime(2021, 7, 8, 9, 10, 11, DateTimeKind.Utc).AddTicks(1234567);
        var archive = Path.Combine(owned.FullName, "utc.zip"); RawZip(archive, NtfsExtra(expected.ToFileTimeUtc()));
        using (var oracle = ZipFile.OpenRead(archive)) { using var payload = oracle.Entries.Single().Open(); Require(payload.ReadByte() == 'x', "independent ZIP fixture"); }
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        await Extract(archive, output);
        Require(File.GetLastWriteTimeUtc(Path.Combine(output, "payload.txt")) == expected, "NTFS100ns UTC did not override DOS");
    });
    public static Task ZipAttributes() => WithWorker(async owned => {
        foreach (var bits in new uint[] { 0, 1, 2, 3, 7 }) {
            var archive = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + ".zip");
            RawZip(archive, [], attributes: bits);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            await Extract(archive, output);
            var actual = File.GetAttributes(Path.Combine(output, "payload.txt"));
            Require((actual & (FileAttributes.ReadOnly | FileAttributes.Hidden | FileAttributes.System)) == (FileAttributes)(bits & 3),
                "Windows readonly/hidden whitelist not restored: " + bits);
        }
    });
    public static Task MotwDefault() => WithWorker(async owned => {
        var archive = Path.Combine(owned.FullName, "marked.zip"); RawZip(archive, [], attributes: 3);
        var mark = System.Text.Encoding.UTF8.GetBytes("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://synthetic-secret.invalid/private\r\n");
        File.WriteAllBytes(archive + ":Zone.Identifier", mark);
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        await Extract(archive, output);
        var path = Path.Combine(output, "payload.txt");
        Require(File.Exists(path + ":Zone.Identifier"), "default MOTW not copied");
        Require(File.ReadAllBytes(path + ":Zone.Identifier").SequenceEqual(mark), "exact synthetic MOTW differs");
        Require(File.ReadAllBytes(archive + ":Zone.Identifier").SequenceEqual(mark), "source MOTW changed");
        var multi = Path.Combine(owned.FullName, "multiple.zip");
        using (var zip = ZipFile.Open(multi, ZipArchiveMode.Create)) {
            foreach (var name in new[] { "first.txt", "nested/second.txt" }) { using var writer = new StreamWriter(zip.CreateEntry(name).Open()); writer.Write("multiple synthetic"); }
        }
        File.WriteAllBytes(multi + ":Zone.Identifier", mark);
        var multiOutput = Directory.CreateDirectory(Path.Combine(owned.FullName, "multiple")).FullName;
        await Extract(multi, multiOutput, files: 2);
        foreach (var name in new[] { "first.txt", "nested/second.txt" }) Require(File.ReadAllBytes(Path.Combine(multiOutput, name) + ":Zone.Identifier").SequenceEqual(mark), "multi-file MOTW differs");
    });
    public static Task Capabilities()
    {
        var old = new[] { "source_lease_v1", "budget_reservations_v1" };
        Require(!ArchiveWorkerRunner.SupportsCapabilities("extract", System.Text.Json.JsonSerializer.SerializeToElement(old)), "missing extraction metadata capabilities accepted");
        var all = old.Concat(new[] { "extraction_metadata_v1", "motw_v1" }).ToArray();
        Require(ArchiveWorkerRunner.SupportsCapabilities("extract", System.Text.Json.JsonSerializer.SerializeToElement(all)), "new extract capabilities rejected");
        foreach (var name in all) Require(!ArchiveWorkerRunner.SupportsCapabilities("extract", System.Text.Json.JsonSerializer.SerializeToElement(all.Where(item => item != name).ToArray())), "missing extract capability accepted");
        return Task.CompletedTask;
    }
    public static Task MotwOptionsAndBudget() => WithWorker(async owned => {
        foreach (var length in new[] { -1, 0, 65536, 65537 }) {
            var archive = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + ".zip"); RawZip(archive, []);
            var mark = length < 0 ? null : Enumerable.Range(0, length).Select(index => (byte)(index * 17)).ToArray();
            if (mark is not null) File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            foreach (var enabled in new[] { true, false }) {
                var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
                var denied = enabled && length > 65536;
                await Extract(archive, output, new(enabled), denied ? "motw_budget" : "success", denied ? 0 : 1);
                var ads = Path.Combine(output, "payload.txt") + ":Zone.Identifier";
                if (!denied && enabled && mark is not null) Require(File.ReadAllBytes(ads).SequenceEqual(mark), "bounded exact MOTW bytes");
                else Require(!File.Exists(ads), "disabled/absent/refused MOTW output");
                if (denied) Require(!Directory.EnumerateFiles(output).Any(), "over-budget MOTW published output");
            }
            if (mark is not null) Require(File.ReadAllBytes(archive + ":Zone.Identifier").SequenceEqual(mark), "source ADS preserved");
        }
    });
    public static Task DirectoryFinalization() => WithWorker(async owned => {
        var archive = Path.Combine(owned.FullName, "directories.zip");
        var wall = new DateTime(2019, 4, 5, 6, 7, 8, DateTimeKind.Unspecified);
        var expected = TimeZoneInfo.ConvertTimeToUtc(wall, TimeZoneInfo.Local);
        using (var zip = ZipFile.Open(archive, ZipArchiveMode.Create)) {
            foreach (var name in new[] { "outer/", "outer/inner/", "outer/inner/payload.txt", "existing/" }) {
                var entry = zip.CreateEntry(name, CompressionLevel.NoCompression);
                entry.LastWriteTime = new DateTimeOffset(wall, TimeZoneInfo.Local.GetUtcOffset(wall));
                if (!name.EndsWith('/')) { using var writer = new StreamWriter(entry.Open()); writer.Write("child"); }
            }
        }
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        var existing = Directory.CreateDirectory(Path.Combine(output, "existing")).FullName;
        var preserved = new DateTime(2018, 1, 2, 3, 4, 5, DateTimeKind.Utc); Directory.SetLastWriteTimeUtc(existing, preserved);
        await Extract(archive, output);
        Require(Directory.GetLastWriteTimeUtc(Path.Combine(output, "outer")) == expected, "parent directory final mtime overwritten by child");
        Require(Directory.GetLastWriteTimeUtc(Path.Combine(output, "outer", "inner")) == expected, "nested directory mtime not restored");
        Require(Directory.GetLastWriteTimeUtc(existing) == preserved, "preexisting directory metadata overwritten");
    });
    public static Task TarModified() => WithWorker(async owned => {
        var expected = new DateTimeOffset(2022, 3, 4, 5, 6, 7, TimeSpan.Zero).AddTicks(1234567);
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit independent Python metadata generator required.");
        var start = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add(Path.Combine(fixtures, "generate-metadata.py")); start.ArgumentList.Add(owned.FullName);
        using (var generator = Process.Start(start)!) { await generator.WaitForExitAsync(); Require(generator.ExitCode == 0, "independent metadata generator"); }
        foreach (var suffix in new[] { ".tar", ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst" }) {
            var archive = Path.Combine(owned.FullName, "synthetic" + suffix);
            var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray(); File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            await Extract(archive, output);
            var file = Path.Combine(output, "nested", "payload.txt");
            Require(File.GetLastWriteTimeUtc(file) == expected.UtcDateTime, "TAR/PAX member time differs");
            Require(Directory.GetLastWriteTimeUtc(Path.Combine(output, "nested")) == expected.UtcDateTime, "TAR/PAX directory time differs");
            Require(File.ReadAllBytes(file + ":Zone.Identifier").SequenceEqual(mark), "compound did not retain original source MOTW");
        }
    });
    private static byte[] UtExtra(byte flags, uint seconds)
    {
        using var bytes = new MemoryStream(); using var writer = new BinaryWriter(bytes);
        writer.Write((ushort)0x5455); writer.Write((ushort)5); writer.Write(flags); writer.Write(seconds); return bytes.ToArray();
    }
    public static Task ZipTimeEdges() => WithWorker(async owned => {
        foreach (var sample in new[] { "ut_central", "unix_zero", "filetime_zero", "absent", "invalid_dos", "overflow", "malformed", "unix_attrs" }) {
            var archive = Path.Combine(owned.FullName, sample + ".zip");
            var extra = sample == "ut_central" ? UtExtra(7, 1600000001) : sample == "unix_zero" ? UtExtra(1, 0) : sample == "filetime_zero" ? NtfsExtra(0) : sample == "overflow" ? NtfsExtra(long.MaxValue) : sample == "malformed" ? new byte[] { 10, 0, 1, 0, 0 } : [];
            RawZip(archive, extra, host: sample == "unix_attrs" ? (byte)3 : (byte)0, attributes: 3, date: sample is "absent" or "invalid_dos" ? (ushort)0 : (ushort)20515, time: sample == "invalid_dos" ? (ushort)1 : sample == "absent" ? (ushort)0 : (ushort)8355);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, sample)).FullName;
            var invalid = sample is "invalid_dos" or "overflow" or "malformed";
            var refusedZero = sample == "filetime_zero";
            var before = DateTime.UtcNow.AddSeconds(-2); await Extract(archive, output, category: invalid ? "worker_failure" : refusedZero ? "metadata_time_not_supported" : "success", files: invalid || refusedZero ? 0 : 1);
            if (invalid || refusedZero) { Require(!Directory.EnumerateFiles(output).Any(), "invalid/unrepresentable metadata publication"); continue; }
            var path = Path.Combine(output, "payload.txt"); var actual = File.GetLastWriteTimeUtc(path);
            if (sample == "ut_central") Require(actual == DateTimeOffset.FromUnixTimeSeconds(1600000001).UtcDateTime, "central UT flags/mtime length");
            if (sample == "unix_zero") Require(actual == DateTime.UnixEpoch, "Unix epoch zero lost");
            if (sample == "absent") Require(actual >= before && actual <= DateTime.UtcNow.AddSeconds(2), "missing time fabricated");
            if (sample == "unix_attrs") Require((File.GetAttributes(path) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == 0, "Unix attrs replayed as Windows flags");
        }
    });
    public static Task PaxTimeEdges() => WithWorker(async owned => {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE"); var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit independent Python metadata generator required.");
        await RunOracle(python, owned.FullName, Path.Combine(fixtures, "generate-metadata.py"), owned.FullName);
        var expected = new[] { DateTime.UnixEpoch.AddTicks(-1), DateTime.UnixEpoch.AddTicks(-11234568), DateTime.UnixEpoch, DateTimeOffset.FromUnixTimeSeconds(1646370367).UtcDateTime.AddTicks(1234567) };
        for (var index = 0; index < 6; index++) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "edge-" + index)).FullName;
            await Extract(Path.Combine(owned.FullName, "edge-" + index + ".tar"), output, category: index < 4 ? "success" : "worker_failure", files: index < 4 ? 1 : 0);
            if (index < 4) Require(File.GetLastWriteTimeUtc(Path.Combine(output, "payload.txt")) == expected[index], "signed PAX earlier100ns rounding: " + index);
            else Require(!Directory.EnumerateFiles(output).Any(), "invalid PAX published");
        }
    });
    public static Task StreamsWithoutMetadata() => WithWorker(async owned => {
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit independent streams required.");
        foreach (var suffix in new[] { "bz2", "xz", "zst", "gz" }) {
            var archive = Path.Combine(owned.FullName, "payload." + suffix); var bytes = File.ReadAllBytes(Path.Combine(fixtures, "independent." + suffix));
            if (suffix == "gz") bytes.AsSpan(4, 4).Clear(); File.WriteAllBytes(archive, bytes);
            File.SetLastWriteTimeUtc(archive, new DateTime(2001, 1, 1, 0, 0, 0, DateTimeKind.Utc));
            var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray(); File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, suffix)).FullName; var before = DateTime.UtcNow.AddSeconds(-2);
            await Extract(archive, output); var path = Path.Combine(output, "payload");
            Require(File.GetLastWriteTimeUtc(path) >= before && File.GetLastWriteTimeUtc(path) <= DateTime.UtcNow.AddSeconds(2), "stream missing metadata fabricated");
            Require((File.GetAttributes(path) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == 0 && File.ReadAllBytes(path + ":Zone.Identifier").SequenceEqual(mark), "stream metadata absence/MOTW");
        }
    });
    public static Task RarMetadata() => WithWorker(async owned => {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE"); var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit independent RAR generator required.");
        await RunOracle(python, owned.FullName, Path.Combine(fixtures, "generate-rar-metadata.py"), owned.FullName);
        foreach (var name in new[] { "synthetic-rar4-windows.rar", "synthetic-rar5-windows.rar", "synthetic-rar5-unix.rar" }) {
            var archive = Path.Combine(owned.FullName, name); var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray(); File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            await Extract(archive, output); var file = Path.Combine(output, "metadata", "payload.txt");
            Require(File.Exists(file), "RAR output member missing: " + name + "; actual=" + string.Join(',', Directory.EnumerateFiles(output, "*", SearchOption.AllDirectories).Select(path => Path.GetRelativePath(output, path))));
            var expected = name.Contains("rar4", StringComparison.Ordinal) ? TimeZoneInfo.ConvertTimeToUtc(new DateTime(2020, 2, 3, 4, 5, 6, DateTimeKind.Unspecified), TimeZoneInfo.Local) : new DateTime(2022, 3, 4, 5, 6, 7, DateTimeKind.Utc).AddTicks(1234567);
            Require(File.ReadAllText(file) == "synthetic rar metadata\n" && File.GetLastWriteTimeUtc(file) == expected, "RAR normalized FILETIME: " + name + " expected=" + expected.ToString("O") + " actual=" + File.GetLastWriteTimeUtc(file).ToString("O") + " length=" + new FileInfo(file).Length);
            var flags = name.Contains("windows", StringComparison.Ordinal) ? FileAttributes.ReadOnly | FileAttributes.Hidden : 0;
            Require((File.GetAttributes(file) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == flags, "RAR host-qualified attributes");
            var directory = Path.Combine(output, "metadata"); Require(Directory.GetLastWriteTimeUtc(directory) == expected && (File.GetAttributes(directory) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == flags, "RAR directory finalization");
            Require(File.ReadAllBytes(file + ":Zone.Identifier").SequenceEqual(mark), "RAR original MOTW");
        }
    });
    private static async Task RunOracle(string executable, string workingDirectory, params string[] arguments)
    {
        var start = new ProcessStartInfo(executable) { WorkingDirectory = workingDirectory, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var argument in arguments) start.ArgumentList.Add(argument);
        using var process = Process.Start(start)!;
        var stdout = process.StandardOutput.ReadToEndAsync(); var stderr = process.StandardError.ReadToEndAsync();
        await process.WaitForExitAsync(); await Task.WhenAll(stdout, stderr);
        Require(process.ExitCode == 0, "independent metadata oracle failed: " + process.ExitCode);
    }
    public static Task SevenzMetadata() => WithWorker(async owned => {
        var oracle = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_7ZIP_ORACLE");
        if (string.IsNullOrEmpty(oracle) || !File.Exists(oracle)) throw new TestSkippedException("Explicit external 7-Zip oracle required.");
        var source = Directory.CreateDirectory(Path.Combine(owned.FullName, "input", "nested")).FullName;
        var expected = new DateTime(2021, 7, 8, 9, 10, 11, DateTimeKind.Utc).AddTicks(1234567);
        var file = Path.Combine(source, "payload.txt"); File.WriteAllText(file, "synthetic sevenz metadata");
        File.SetLastWriteTimeUtc(file, expected); File.SetAttributes(file, FileAttributes.ReadOnly | FileAttributes.Hidden);
        Directory.SetLastWriteTimeUtc(source, expected); File.SetAttributes(source, File.GetAttributes(source) | FileAttributes.Hidden);
        var archive = Path.Combine(owned.FullName, "synthetic.7z");
        await RunOracle(oracle, Path.GetDirectoryName(source)!, "a", "-t7z", "-mx=0", "-mtm=on", "-mta=off", "-mtc=off", archive, "nested");
        var oracleOutput = Directory.CreateDirectory(Path.Combine(owned.FullName, "oracle")).FullName;
        await RunOracle(oracle, owned.FullName, "x", "-y", "-o" + oracleOutput, archive);
        var oracleFile = Path.Combine(oracleOutput, "nested", "payload.txt");
        Require(File.GetLastWriteTimeUtc(oracleFile) == expected && (File.GetAttributes(oracleFile) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == (FileAttributes.ReadOnly | FileAttributes.Hidden), "independent 7z fixture metadata");
        var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray(); File.WriteAllBytes(archive + ":Zone.Identifier", mark);
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName; await Extract(archive, output);
        var extracted = Path.Combine(output, "nested", "payload.txt");
        Require(File.GetLastWriteTimeUtc(extracted) == expected, "7z FILETIME not restored");
        Require((File.GetAttributes(extracted) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == (FileAttributes.ReadOnly | FileAttributes.Hidden), "7z Windows attributes not restored");
        Require(Directory.GetLastWriteTimeUtc(Path.Combine(output, "nested")) == expected && File.GetAttributes(Path.Combine(output, "nested")).HasFlag(FileAttributes.Hidden), "7z directory metadata not finalized");
        Require(File.ReadAllBytes(extracted + ":Zone.Identifier").SequenceEqual(mark), "7z original MOTW");
    });
    private sealed class MetadataLog(bool reject = false) : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (reject) throw new InvalidOperationException("synthetic rejected log");
            Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
        }
    }
    public static Task NativeLifecycle() => WithWorker(async owned => {
        foreach (var mode in new[] { "time_denied", "attrs_denied", "ads_unsupported", "ads_denied", "cancel_metadata", "directory_denied", "directory_cancel", "postcommit", "reject_log", "overwrite", "no_replace" }) {
            var archive = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + ".zip");
            var isDirectory = mode is "directory_denied" or "directory_cancel";
            if (isDirectory) {
                using var zip = ZipFile.Open(archive, ZipArchiveMode.Create);
                zip.CreateEntry("nested/"); using var writer = new StreamWriter(zip.CreateEntry("nested/payload.txt").Open()); writer.Write("x");
            } else RawZip(archive, [], attributes: 3);
            var mark = "[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://metadata-sensitive.invalid/secret\r\n"u8.ToArray();
            File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var path = Path.Combine(output, isDirectory ? "nested/payload.txt" : "payload.txt");
            var external = Path.Combine(output, ".nxpart-external"); File.WriteAllText(external, "external sentinel");
            Dictionary<string, string>? overwrites = null;
            if (mode is "overwrite" or "no_replace") {
                File.WriteAllText(path, "old target");
                if (mode == "overwrite") { using var capture = ArchiveOutputSession.Open(output, Guid.NewGuid()); overwrites = new() { ["payload.txt"] = capture.CaptureTargetIdentity("payload.txt") }; }
            }
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new MetadataLog(mode == "reject_log"); var id = Guid.NewGuid();
            using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log); var reached = false;
            runner.OutputCommitCheckpointProbe = stage => {
                var target = mode is "time_denied" or "attrs_denied" ? "before_metadata" : mode is "ads_unsupported" or "ads_denied" ? "before_motw" :
                    mode == "cancel_metadata" ? "metadata_applied" : mode is "directory_denied" or "directory_cancel" ? "before_directory_metadata" : mode == "postcommit" ? "native_committed" : "unused";
                if (stage != target) return; reached = true;
                if (mode is "cancel_metadata" or "directory_cancel") { Require(runner.Cancel(id), "metadata cancellation admission"); return; }
                throw new ArchiveOutputException(mode == "time_denied" || mode == "directory_denied" ? "metadata_time_failed" : mode == "attrs_denied" ? "metadata_attributes_failed" :
                    mode == "ads_unsupported" ? "motw_not_supported" : mode == "ads_denied" ? "motw_write_failed" : "inspect_identity", mode == "ads_unsupported" ? 50 : 5);
            };
            Require(runner.Start(new(id, "extract", archive, 0, new { max_entries = 100 }, OutputRoot: output, ConfirmedOverwrites: overwrites)), "lifecycle admission");
            var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 20000, "lifecycle deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync(), "lifecycle drain"); var terminal = events.Single(e => e.Kind == "exited");
            var published = mode is "directory_denied" or "directory_cancel" or "postcommit" or "reject_log" or "overwrite";
            var category = mode is "time_denied" or "directory_denied" ? "metadata_time_failed" : mode == "attrs_denied" ? "metadata_attributes_failed" :
                mode == "ads_unsupported" ? "motw_not_supported" : mode == "ads_denied" ? "motw_write_failed" : mode == "cancel_metadata" ? "cancelled" :
                mode == "directory_cancel" ? "cancelled_with_committed_output" : mode == "postcommit" ? "committed_but_verification_failed" : mode == "no_replace" ? "worker_failure" : "success";
            Require(terminal.Category == category && terminal.FilesCommitted == (published ? 1 : 0) && terminal.CleanupComplete, "metadata lifecycle " + mode + ": " + terminal.Category);
            if (mode is not ("reject_log" or "overwrite" or "no_replace")) Require(reached, "metadata boundary not reached: " + mode);
            if (isDirectory) Require(terminal.CommitState == "Committed", "directory terminal erased published state: " + terminal.CommitState);
            if (mode.EndsWith("denied", StringComparison.Ordinal) || mode is "ads_unsupported" or "postcommit") Require(terminal.NativeErrorCode == (mode == "ads_unsupported" ? 50 : 5), "first metadata native code lost");
            Require(File.ReadAllText(external) == "external sentinel" && Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).SequenceEqual(new[] { external }), "metadata partial ownership");
            if (published) Require(File.ReadAllText(path) == "x" && File.ReadAllBytes(path + ":Zone.Identifier").SequenceEqual(mark), "published metadata output lost");
            else if (mode == "no_replace") Require(File.ReadAllText(path) == "old target", "no-replace target changed");
            else Require(!File.Exists(path), "failed metadata published output");
            Require(File.ReadAllBytes(archive + ":Zone.Identifier").SequenceEqual(mark), "metadata source mark changed");
            if (mode != "reject_log") {
                var record = log.Records.Single(item => item.Name is "archive.worker.failed" or "archive.worker.completed" or "archive.worker.cancelled");
                Require(record.Id == id && Equals(record.Fields["category"], terminal.Category) && Equals(record.Fields["native_error_code"], terminal.NativeErrorCode) && Equals(record.Fields["files_committed"], terminal.FilesCommitted), "metadata diagnostic correlation/facts");
                var serialized = System.Text.Json.JsonSerializer.Serialize(log.Records.Select(item => new { item.Name, item.Id, item.Fields }));
                Require(!serialized.Contains("metadata-sensitive", StringComparison.Ordinal) && !serialized.Contains("ZoneTransfer", StringComparison.Ordinal) && !serialized.Contains(owned.FullName, StringComparison.Ordinal) && !serialized.Contains("payload.txt", StringComparison.Ordinal), "metadata diagnostic secret leak");
            }
        }
    });
    public static Task AdsCapabilityAndSourceBinding() => WithWorker(async owned => {
        foreach (var mode in new[] { "source_unsupported", "output_unsupported", "disabled", "held_source" }) {
            var archive = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + ".zip"); RawZip(archive, []);
            var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray(); File.WriteAllBytes(archive + ":Zone.Identifier", mark);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            var probes = 0;
            runner.NamedStreamsFlagsProbe = (writing, flags) => { probes++; return mode == "disabled" || mode == "source_unsupported" && !writing || mode == "output_unsupported" && writing ? flags & ~0x40000u : flags; };
            var held = false;
            if (mode == "held_source") runner.OutputCommitCheckpointProbe = stage => {
                if (stage != "before_metadata") return; held = true;
                try { using var write = new FileStream(archive, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source write was allowed"); }
                catch (IOException failure) { Require((failure.HResult & 0xffff) == 32, "source lease native failure"); }
            };
            Require(runner.Start(new(Guid.NewGuid(), "extract", archive, 0, new { max_entries = 100 }, OutputRoot: output, ExtractOptions: new(mode != "disabled"))), "ADS capability admission");
            var clock = Stopwatch.StartNew(); while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 20000, "ADS capability deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync(), "ADS capability drain"); var terminal = events.Single(e => e.Kind == "exited");
            var unsupported = mode.EndsWith("unsupported", StringComparison.Ordinal);
            Require(terminal.Category == (unsupported ? "motw_not_supported" : "success") && terminal.FilesCommitted == (unsupported ? 0 : 1) && terminal.NativeErrorCode is null && terminal.CleanupComplete, "ADS capability conflated with absence: " + mode + "/" + terminal.Category);
            Require(mode == "disabled" ? probes == 0 : probes > 0, "ADS capability option boundary");
            if (mode == "held_source") Require(held && File.ReadAllBytes(Path.Combine(output, "payload.txt") + ":Zone.Identifier").SequenceEqual(mark), "MOTW source lease binding");
            Require(File.ReadAllBytes(archive + ":Zone.Identifier").SequenceEqual(mark), "ADS source retained");
        }
    });
    public static Task GuardedMetadataCleanup() => WithWorker(async owned => {
        foreach (var mutation in new[] { "plain_ads_added", "none", "attributes", "ads_added", "ads_changed" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var path = Path.Combine(output, "payload.txt");
            using var session = ArchiveOutputSession.Open(output, Guid.NewGuid());
            var mark = "[ZoneTransfer]\r\nZoneId=3\r\n"u8.ToArray();
            if (mutation == "ads_changed") session.BindZoneIdentifier(mark);
            var token = session.BeginFile("payload.txt"); session.Write(token, "x"u8.ToArray());
            if (mutation != "plain_ads_added") session.SetMetadata(token, new(null, true, true));
            Require(session.Commit(token).Category == "committed", "metadata cleanup publication");
            var modified = File.GetLastWriteTimeUtc(path);
            if (mutation == "attributes") File.SetAttributes(path, File.GetAttributes(path) & ~FileAttributes.Hidden);
            if (mutation is "plain_ads_added" or "ads_added" or "ads_changed") {
                var attributes = File.GetAttributes(path); File.SetAttributes(path, attributes & ~FileAttributes.ReadOnly);
                File.WriteAllText(path + ":Zone.Identifier", "externally modified mark"); File.SetAttributes(path, attributes);
            }
            File.SetLastWriteTimeUtc(path, modified); // Old identity/mtime/content credentials still match.
            var result = session.Cancel(token, cleanupNewOutput: true);
            Require(result.Category == (mutation == "none" ? "new_output_removed" : "changed_retained"), "metadata cleanup discarded external " + mutation + ": " + result.Category);
            Require(File.Exists(path) == (mutation != "none"), "metadata cleanup file retention");
            await Task.CompletedTask;
        }
    });
    public static Task GzipModified() => WithWorker(async owned => {
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(fixtures)) throw new TestSkippedException("Explicit independent stream fixtures required.");
        var archive = Path.Combine(owned.FullName, "payload.gz");
        var bytes = File.ReadAllBytes(Path.Combine(fixtures, "independent.gz"));
        System.Buffers.Binary.BinaryPrimitives.WriteUInt32LittleEndian(bytes.AsSpan(4, 4), 1600000001); File.WriteAllBytes(archive, bytes);
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName; await Extract(archive, output);
        Require(File.GetLastWriteTimeUtc(Path.Combine(output, "payload")) == DateTimeOffset.FromUnixTimeSeconds(1600000001).UtcDateTime, "Gzip header MTIME not restored");
    });
}
