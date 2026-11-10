using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveOptimizationTests
{
    public static Task Opt05CompletedFormatBaseline() => Opt05Formats(false);
    public static Task Opt05FormatPlanReuse() => Opt05Formats(true);

    // Original Read/Job/Entry routes over explicitly marked scratch fixtures and native outputs.
    private static async Task Opt05Formats(bool assertReuse)
    {
        var root = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_OPT_FORMAT_ROOT")!;
        Check(Path.IsPathFullyQualified(root) && File.Exists(Path.Combine(root, "owned-opt-fixtures")), "owned format root required");
        ArchiveReadHelper.GuardOwnedRoot(root);
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE")!;
        Check(Path.IsPathFullyQualified(worker) && File.Exists(worker), "matching current worker required");
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        Check(!File.Exists(helper), "previous helper released");
        using var manifest = JsonDocument.Parse(File.ReadAllText(Path.Combine(root, "fixture-provenance.json")));
        var fixtures = manifest.RootElement.EnumerateArray().Where(value => value.GetProperty("name").GetString() is "targz" or "tarxz").ToArray();
        Check(fixtures.Length == 2, "two independently prepared compound fixtures");
        var results = new List<Observation>();
        File.Copy(worker, helper);
        Console.WriteLine("ARCHIVE_OPT05_FORMAT_PAIR " + JsonSerializer.Serialize(new {
            configuration = "Debug", host = typeof(ArchiveWorkerRunner).Assembly.Location,
            tests = typeof(ArchiveOptimizationTests).Assembly.Location, worker,
            worker_bytes = new FileInfo(worker).Length, worker_modified_utc = File.GetLastWriteTimeUtc(worker),
            cache_policy = "no_system_cache_flush", root
        }));
        try {
            foreach (var fixture in fixtures) {
                var name = fixture.GetProperty("name").GetString()!;
                var folder = Path.Combine(root, name);
                var source = Path.Combine(folder, fixture.GetProperty("archive").GetString()!);
                Check(new FileInfo(source).Length == fixture.GetProperty("archive_bytes").GetInt64(), "retained source size");
                var expected = Directory.GetFiles(Path.Combine(folder, "expected"), "*", SearchOption.AllDirectories)
                    .ToDictionary(path => Path.GetRelativePath(Path.Combine(folder, "expected"), path).Replace('\\', '/'), File.ReadAllBytes);
                Check(expected.Count == fixture.GetProperty("files").GetArrayLength(), "independent expected count");
                for (var trial = 0; trial < (assertReuse ? 1 : 3); trial++) {
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
            }
            if (Environment.GetEnvironmentVariable("NXTOOLS_OPT05_FORMAT_REPORT") is { Length: > 0 } report) {
                using var output = new FileStream(report, FileMode.CreateNew, FileAccess.Write, FileShare.None);
                JsonSerializer.Serialize(output, results, new JsonSerializerOptions { WriteIndented = true });
            }
            foreach (var group in results.GroupBy(value => value.Operation)) {
                var elapsed = group.Select(value => value.ElapsedMs).Order().ToArray();
                Console.WriteLine("ARCHIVE_OPT05_MEDIAN " + JsonSerializer.Serialize(new {
                    operation = group.Key, trials = elapsed.Length, median_ms = elapsed[elapsed.Length / 2]
                }));
            }
            if (assertReuse) {
                ulong Count(Observation value, string key) => value.Work.Aggregate(0UL, (sum, work) => sum + work.GetProperty(key).GetUInt64());
                var failures = results.Where(value => Count(value, "outer_decodes") != 1 || Count(value, "tar_record_scans") != 1)
                    .Select(value => new { value.Operation, outer = Count(value, "outer_decodes"), scans = Count(value, "tar_record_scans") }).ToArray();
                Check(failures.Length == 0, "one verified compound spool and catalog parse per original Run: " + JsonSerializer.Serialize(failures));
            }
        }
        finally {
            File.Delete(helper);
            Check(!Directory.EnumerateDirectories(root, "output-*").Any(), "owned output roots removed");
            Console.WriteLine("ARCHIVE_OPT05_FORMAT_CLEANUP helper_removed=true outputs_removed=true sources_and_independent_expectations_retained=true");
        }
    }
}
