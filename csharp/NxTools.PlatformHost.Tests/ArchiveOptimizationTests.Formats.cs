using System.Diagnostics;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveOptimizationTests
{
    // Effects: explicitly marked owned source copies, native output/cache and helper only.
    // Expected payloads are prepared by an independent decoder before measurement.
    public static async Task CompletedFormatBaseline()
    {
        var root = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT")!;
        Check(Path.IsPathFullyQualified(root) && File.Exists(Path.Combine(root, "owned-opt-fixtures")), "owned format root required");
        ArchiveReadHelper.GuardOwnedRoot(root);
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")!;
        Check(Path.IsPathFullyQualified(worker) && File.Exists(worker), "frozen worker required");
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        Check(!File.Exists(helper), "previous helper released");
        using var manifest = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "fixture-provenance.json")));
        Check(manifest.RootElement.GetArrayLength() == 4, "four explicit format fixtures");
        File.Copy(worker, helper);
        Console.WriteLine("ARCHIVE_OPT_FORMAT_PAIR " + JsonSerializer.Serialize(new {
            configuration = "Debug", host = typeof(ArchiveWorkerRunner).Assembly.Location,
            tests = typeof(ArchiveOptimizationTests).Assembly.Location, worker,
            worker_bytes = new FileInfo(worker).Length, worker_modified_utc = File.GetLastWriteTimeUtc(worker),
            cache_policy = "no_system_cache_flush", root
        }));
        try {
            foreach (var fixture in manifest.RootElement.EnumerateArray()) {
                var name = fixture.GetProperty("name").GetString()!;
                Check(name is "sevenz-solid" or "rar5-solid" or "targz" or "tarxz", "fixed format selection");
                var folder = Path.Combine(root, name);
                var source = Path.Combine(folder, fixture.GetProperty("archive").GetString()!);
                Check(new FileInfo(source).Length == fixture.GetProperty("archive_bytes").GetInt64(), "retained source size");
                var expected = Directory.GetFiles(Path.Combine(folder, "expected"), "*", SearchOption.AllDirectories)
                    .ToDictionary(path => Path.GetRelativePath(Path.Combine(folder, "expected"), path).Replace('\\', '/'), File.ReadAllBytes);
                Check(expected.Count == fixture.GetProperty("files").GetArrayLength(), "independent expected file count");
                var results = new List<Observation>();
                for (var trial = 0; trial < 3; trial++) {
                    var (browser, files, observation) = await FormatBrowse(root, source, name, trial, expected);
                    results.Add(observation);
                    var selected = files.OrderBy(entry => entry.Ordinal).Last();
                    results.Add(await FormatJob(root, source, name, trial, browser, null, expected, true));
                    results.Add(await FormatMaterialize(root, source, name, trial, browser, selected, expected[selected.Name]));
                    results.Add(await FormatJob(root, source, name, trial, browser, null, expected, false));
                    results.Add(await FormatJob(root, source, name, trial, browser, selected,
                        new Dictionary<string, byte[]> { [selected.Name] = expected[selected.Name] }, false));
                    using var released = File.Open(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
                }
                foreach (var group in results.GroupBy(value => value.Operation)) {
                    var elapsed = group.Select(value => value.ElapsedMs).Order().ToArray();
                    Console.WriteLine("ARCHIVE_OPT_MEDIAN " + JsonSerializer.Serialize(new {
                        operation = group.Key, trials = elapsed.Length, median_ms = elapsed[1]
                    }));
                }
            }
        }
        finally {
            File.Delete(helper);
            Check(!Directory.EnumerateDirectories(root, "output-*").Any(), "format output roots removed");
            Console.WriteLine("ARCHIVE_OPT_FORMAT_CLEANUP helper_removed=true outputs_removed=true sources_and_independent_expectations_retained=true");
        }
    }

    private static async Task<(ArchiveBrowserContext Browser, ArchiveReadEntry[] Files, Observation Observation)> FormatBrowse(
        string root, string source, string name, int trial, Dictionary<string, byte[]> expected)
    {
        using var log = new WorkLog();
        await using var session = ArchiveReadSession.ForOwnedTestPackage(root, log);
        var wire = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(wire);
        log.Bind(session, identity.OperationId, root);
        var clock = Stopwatch.StartNew();
        Check(session.Dispatch(wire).Success, "format browser dispatch");
        var entries = new List<ArchiveReadEntry>();
        while (true) {
            var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload
                .GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (value.Batch is { } batch) {
                entries.AddRange(batch.Entries);
                Check(session.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId,
                    identity.Generation, sequence: batch.Sequence, version: 2)).Success, "format metadata credit");
            }
            if (value.Terminal is not { } terminal) continue;
            clock.Stop();
            Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null,
                "format browser terminal " + name + " " + terminal.Code);
            var files = entries.Where(entry => entry.Kind == "File").ToArray();
            Check(files.Length == expected.Count && files.All(entry => expected.TryGetValue(entry.Name, out var bytes)
                && entry.Size == (ulong)bytes.Length), "original ordinal/name/size catalog matches independent expectation");
            return (terminal.Browser!, files, Record(name + "/browser_open", trial, clock, log, (ulong)files.Length, 0, false, true));
        }
    }

    private static async Task<Observation> FormatJob(string root, string source, string name, int trial,
        ArchiveBrowserContext browser, ArchiveReadEntry? selected, Dictionary<string, byte[]> expected, bool test)
    {
        using var log = new WorkLog();
        await using var session = ArchiveJobSession.ForOwnedTestPackage(root, log);
        var destination = Path.Combine(root, "output-" + Guid.NewGuid());
        if (!test) Directory.CreateDirectory(destination);
        var request = test ? new ArchiveJobRequest { Kind = "test", Paths = [source] }
            : new ArchiveJobRequest { Kind = "extract", Paths = [source], Destination = destination,
                Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "skip", CopyMotw = false,
                    Selection = selected is null ? null : new(browser, [selected.Ordinal]) } };
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(),
            Generation = 1, Action = "start", Request = request };
        log.Bind(session, command.OperationId, root);
        var clock = Stopwatch.StartNew();
        try {
            Check(session.Dispatch(JobWire(command)).Success, "format job dispatch");
            while (true) {
                var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload
                    .GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                Check(value.Prompt is null, "ordinary format baseline must not wait on a prompt");
                if (value.Terminal is not { } terminal) continue;
                clock.Stop();
                var bytes = expected.Values.Aggregate(0UL, (sum, payload) => sum + (ulong)payload.Length);
                Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete, "format job terminal " + name + " " + terminal.Code);
                if (test) Check(terminal.FilesVerified == (ulong)expected.Count && terminal.BytesVerified == bytes && terminal.FilesCommitted == 0,
                    "format Test counts without publication");
                else {
                    Check(terminal.FilesCommitted == (ulong)expected.Count && terminal.BytesCommitted == bytes, "format publication counts");
                    var actual = Directory.GetFiles(destination, "*", SearchOption.AllDirectories);
                    Check(actual.Length == expected.Count && actual.All(path => {
                        var relative = Path.GetRelativePath(destination, path).Replace('\\', '/');
                        return expected.TryGetValue(relative, out var payload) && File.ReadAllBytes(path).SequenceEqual(payload);
                    }), "exact independent payloads and no predecessor/partial outputs");
                }
                return Record(name + (test ? "/test" : selected is null ? "/extract_all" : "/extract_selected_last"),
                    trial, clock, log, (ulong)expected.Count, bytes, !test, terminal.CleanupComplete);
            }
        }
        finally { if (Directory.Exists(destination)) Directory.Delete(destination, true); }
    }

    private static async Task<Observation> FormatMaterialize(string root, string source, string name, int trial,
        ArchiveBrowserContext browser, ArchiveReadEntry selected, byte[] expected)
    {
        using var log = new WorkLog();
        await using var session = ArchiveEntrySession.ForOwnedTestPackage(root, log);
        var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(),
            Generation = 1, Action = "materialize", Path = source, Ordinal = selected.Ordinal, Browser = browser, CopyMotw = false };
        ProtocolEnvelope Wire(ArchiveEntryCommand value) => ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = value }, value.OperationId);
        log.Bind(session, command.OperationId, root);
        var clock = Stopwatch.StartNew();
        var reply = await session.Dispatch(Wire(command)).WaitAsync(TimeSpan.FromSeconds(90));
        clock.Stop();
        var result = JsonSerializer.Deserialize<ArchiveEntryResult>(reply.Data!, JsonDefaults.Options)!;
        Check(reply.Success && result.Success && result.CleanupComplete && result.Path is not null,
            "format materialize terminal " + name + " " + result.Code);
        Check(File.ReadAllBytes(result.Path!).SequenceEqual(expected), "original selected ordinal materializes independent exact bytes");
        var observation = Record(name + "/materialize_last", trial, clock, log, 1, (ulong)expected.Length, true, result.CleanupComplete);
        Check((await session.Dispatch(Wire(command with { Action = "release", Path = null, Ordinal = null, Browser = null,
            CopyMotw = null, LeaseId = result.LeaseId }))).Success && !File.Exists(result.Path), "format cache lease released");
        return observation;
    }
}
