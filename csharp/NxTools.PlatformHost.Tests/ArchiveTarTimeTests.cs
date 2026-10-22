using System.Collections.Concurrent;
using System.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// GUID-owned synthetic archives/output, exact helper copy, owned Jobs; no live system mutation.
internal static class ArchiveTarTimeTests
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
        var owned = Directory.CreateTempSubdirectory("archive-tar-time-");
        var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally {
            if (copied) File.Delete(helper);
            owned.Delete(true);
        }
    }

    [System.Runtime.InteropServices.DllImport("kernel32.dll", EntryPoint = "GetFileTime", ExactSpelling = true, SetLastError = true)]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static extern bool ReadTimes(Microsoft.Win32.SafeHandles.SafeFileHandle file, out long creation, out long access, out long write);

    // File.GetLastWriteTimeUtc internally converts through local time and can overflow at9999.
    // Read the independent Windows UTC FILETIME directly, with no product metadata adapter.
    private static DateTime NativeModified(string path)
    {
        using var file = File.OpenHandle(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        Require(ReadTimes(file, out _, out _, out var write), "native file-time oracle: " + System.Runtime.InteropServices.Marshal.GetLastWin32Error());
        return DateTime.FromFileTimeUtc(write);
    }
    private static async Task Generate(DirectoryInfo owned)
    {
        var python = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_PYTHON_ORACLE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_ZIP_FIXTURES");
        if (string.IsNullOrEmpty(python) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit independent Python TAR generator required.");
        var start = new ProcessStartInfo(python) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        start.ArgumentList.Add(Path.Combine(fixtures, "generate-tar-times.py"));
        start.ArgumentList.Add(owned.FullName);
        using var generator = Process.Start(start)!;
        var stdout = generator.StandardOutput.ReadToEndAsync();
        var stderr = generator.StandardError.ReadToEndAsync();
        await generator.WaitForExitAsync();
        await Task.WhenAll(stdout, stderr);
        Require(generator.ExitCode == 0, "independent TAR generator failed: " + generator.ExitCode);
    }

    private static async Task<ArchiveWorkerEvent> Extract(string archive, string output, string category = "success", int files = 1,
        string? domainError = null, IReadOnlyDictionary<string, string>? overwrites = null, Func<int>? committedOracle = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
        Require(runner.Start(new(Guid.NewGuid(), "extract", archive, 0, new { max_entries = 100 }, OutputRoot: output,
            ExtractOptions: new ArchiveExtractOptions(false), ConfirmedOverwrites: overwrites)), "TAR time admission");
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited")) {
            Require(clock.ElapsedMilliseconds < 20000, "TAR time worker deadline");
            await Task.Delay(20);
        }
        Require(await runner.DrainAsync(), "TAR time worker cleanup");
        var terminal = events.Single(e => e.Kind == "exited");
        var failure = events.FirstOrDefault(e => e.Frame is { } frame && frame.GetProperty("op").GetString() == "failure")?.Frame?.GetProperty("error_code").GetString();
        var expectedFiles = committedOracle?.Invoke() ?? files;
        Require(terminal.Category == category && terminal.FilesCommitted == expectedFiles && terminal.CleanupComplete,
            $"TAR time terminal: {terminal.Category}/{failure}/committed={terminal.FilesCommitted}/cleanup={terminal.CleanupComplete}");
        if (domainError is not null) Require(failure == domainError, "exact typed failure: " + failure);
        if (category == "success" || expectedFiles > 0) Require(terminal.CommitState == "Committed", "published facts lost: " + terminal.CommitState);
        Require(terminal.NativeErrorCode is null, "no fabricated native error");
        var external = Path.Combine(output, ".nxpart-external");
        Require(Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories)
            .All(path => path == external && File.ReadAllText(path) == "external sentinel"), "owned partial leak/external partial changed");
        return terminal;
    }

    public static Task GnuNegative() => WithWorker(async owned => {
        await Generate(owned);
        foreach (var suffix in new[] { ".tar", ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            await Extract(Path.Combine(owned.FullName, "gnu-negative" + suffix), output);
            var path = Path.Combine(output, "payload.txt");
            Require(File.ReadAllBytes(path).SequenceEqual("x"u8.ToArray()), "GNU negative exact payload");
            Require(File.GetLastWriteTimeUtc(path) == new DateTime(1969, 12, 31, 23, 59, 59, DateTimeKind.Utc),
                "GNU -1 exact pre1970 Windows mtime: " + suffix);
        }
    });

    public static Task PaxGlobalClear() => WithWorker(async owned => {
        await Generate(owned);
        foreach (var suffix in new[] { ".tar", ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            await Extract(Path.Combine(owned.FullName, "pax-global-clear" + suffix), output, files: 2);
            Require(File.ReadAllBytes(Path.Combine(output, "first.txt")).SequenceEqual("x"u8.ToArray()), "first exact payload");
            Require(File.ReadAllBytes(Path.Combine(output, "second.txt")).SequenceEqual("y"u8.ToArray()), "second exact payload");
            Require(File.GetLastWriteTimeUtc(Path.Combine(output, "first.txt")) == new DateTime(2017, 7, 14, 2, 40, 0, DateTimeKind.Utc), "global precedence");
            Require(File.GetLastWriteTimeUtc(Path.Combine(output, "second.txt")) == new DateTime(2020, 9, 13, 12, 26, 41, DateTimeKind.Utc), "global clear restores header: " + suffix);
        }
    });

    public static Task PaxLocalScope() => WithWorker(async owned => {
        await Generate(owned);
        var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        var before = DateTime.UtcNow.AddSeconds(-2);
        await Extract(Path.Combine(owned.FullName, "pax-local.tar"), output, files: 5);
        var names = new[] { "override.txt", "global-again.txt", "deleted.txt", "global-still.txt", "header.txt" };
        var expected = new long?[] { 1400000000, 1500000000, null, 1500000000, 1600000004 };
        for (var index = 0; index < names.Length; index++) {
            var path = Path.Combine(output, names[index]);
            Require(File.ReadAllBytes(path).SequenceEqual(new[] { (byte)('a' + index) }), "local exact content");
            var actual = File.GetLastWriteTimeUtc(path);
            if (expected[index] is long seconds) Require(actual == DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime, "local/global precedence: " + index);
            else Require(actual >= before && actual <= DateTime.UtcNow.AddSeconds(2), "local empty must not replay header/global");
        }
    });

    public static Task TimeEdgesAndRetention() => WithWorker(async owned => {
        await Generate(owned);
        foreach (var (name, seconds) in new (string, long?)[] {
            ("gnu-negative-day", -86401), ("gnu-year1900", -2208988800), ("gnu-zero", 0),
            ("gnu-positive", 1600000001), ("gnu-lower-native", -11644473599), ("gnu-upper-native", 253402300799),
            ("header-absent", null), ("gnu-below-native", null), ("gnu-above-native", null),
            ("header-invalid-marker", null), ("header-overflow", null), ("header-invalid-octal", null) }) {
            try {
                var output = Directory.CreateDirectory(Path.Combine(owned.FullName, name)).FullName;
                var external = Path.Combine(output, ".nxpart-external"); File.WriteAllText(external, "external sentinel");
                var old = Path.Combine(output, "old.txt"); File.WriteAllText(old, "old target");
                var invalid = seconds is null && name != "header-absent";
                var before = DateTime.UtcNow.AddSeconds(-2);
                await Extract(Path.Combine(owned.FullName, name + ".tar"), output, invalid ? "worker_failure" : "success", invalid ? 0 : 1,
                    domainError: invalid ? "InvalidMetadata" : null);
                Require(File.ReadAllText(old) == "old target" && File.ReadAllText(external) == "external sentinel", "external preservation");
                var path = Path.Combine(output, "payload.txt");
                if (invalid) Require(!File.Exists(path), "invalid time published");
                else {
                    Require(File.ReadAllBytes(path).SequenceEqual("x"u8.ToArray()), "edge exact payload");
                    var actual = NativeModified(path);
                    if (seconds is long value) Require(actual == DateTimeOffset.FromUnixTimeSeconds(value).UtcDateTime, "edge native time: " + name);
                    else Require(actual >= before && actual <= DateTime.UtcNow.AddSeconds(2), "missing time fabricated");
                }
            } catch (Exception failure) { throw new InvalidOperationException("TAR edge " + name + ": " + failure, failure); }
        }
        // Staged token ordering is unspecified: independently observe any published first file.
        foreach (var scope in new[] { "x", "g" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "invalid-" + scope)).FullName;
            var old = Path.Combine(output, "second.txt"); File.WriteAllText(old, "old target");
            File.SetLastWriteTimeUtc(old, DateTime.UnixEpoch);
            var external = Path.Combine(output, ".nxpart-external"); File.WriteAllText(external, "external sentinel");
            Dictionary<string, string> overwrites;
            using (var capture = ArchiveOutputSession.Open(output, Guid.NewGuid()))
                overwrites = new() { ["second.txt"] = capture.CaptureTargetIdentity("second.txt") };
            await Extract(Path.Combine(owned.FullName, "pax-invalid-" + scope + ".tar"), output, "worker_failure", 1,
                domainError: "InvalidMetadata", overwrites: overwrites,
                committedOracle: () => File.Exists(Path.Combine(output, "first.txt")) ? 1 : 0);
            if (File.Exists(Path.Combine(output, "first.txt"))) {
                Require(File.ReadAllBytes(Path.Combine(output, "first.txt")).SequenceEqual("x"u8.ToArray()), "committed first changed");
                Require(NativeModified(Path.Combine(output, "first.txt")) == DateTimeOffset.FromUnixTimeSeconds(1600000000).UtcDateTime, "committed first time");
            }
            Require(File.ReadAllText(old) == "old target" && File.GetLastWriteTimeUtc(old) == DateTime.UnixEpoch, "old target changed before invalid metadata rejection");
            Require(File.ReadAllText(external) == "external sentinel", "foreign partial removed");
        }
        foreach (var scope in new[] { "x", "g" }) {
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "directory-" + scope)).FullName;
            var external = Path.Combine(output, ".nxpart-external"); File.WriteAllText(external, "external sentinel");
            await Extract(Path.Combine(owned.FullName, "pax-invalid-directory-" + scope + ".tar"), output,
                "worker_failure", 1, domainError: "InvalidMetadata");
            var path = Path.Combine(output, "nested", "payload.txt");
            Require(File.ReadAllBytes(path).SequenceEqual("x"u8.ToArray()), "directory failure rolled back published file");
            Require(NativeModified(path) == DateTimeOffset.FromUnixTimeSeconds(1600000001).UtcDateTime, "published child time");
            Require(File.ReadAllText(external) == "external sentinel", "directory failure removed external partial");
        }
    });
}
