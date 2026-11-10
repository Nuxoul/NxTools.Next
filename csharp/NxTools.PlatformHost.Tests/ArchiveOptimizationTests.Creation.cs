using System.Diagnostics;
using System.IO.Compression;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static partial class ArchiveOptimizationTests
{
    private static async Task<ArchiveJobTerminal> MeasuredJob(string root, string label, ArchiveJobRequest request,
        int trial, List<(string Label, double Ms)> timings, bool requireSuccess = true)
    {
        using var log = new WorkLog(); await using var session = ArchiveJobSession.ForOwnedTestPackage(root, log);
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
        var clock = Stopwatch.StartNew();
        log.Bind(session, command.OperationId, root);
        Check(session.Dispatch(JobWire(command)).Success, "measured job admission");
        while (true) {
            var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Check(value.Prompt is null, "owned no-conflict fixture must not prompt");
            if (value.Terminal is not { } terminal) continue;
            clock.Stop();
            if (terminal.Outcome != "succeeded" || !terminal.CleanupComplete)
                Console.WriteLine("ARCHIVE_OPT_FAILURE " + JsonSerializer.Serialize(new { operation = label, trial, elapsed_ms = clock.Elapsed.TotalMilliseconds,
                    terminal.Outcome, terminal.Code, terminal.NativeErrorCode, terminal.CommitState, terminal.OutputPublished,
                    terminal.CleanupComplete, terminal.FilesCommitted, terminal.BytesVerified,
                    workers = log.Events.Count(e => e.Name == "archive.worker.started"), work = log.Work,
                    stages = log.Events.Select(e => e.Name).Distinct().Take(24).ToArray() }));
            if (requireSuccess) Check(terminal.Outcome == "succeeded" && terminal.CleanupComplete, label + " terminal " + terminal.Code);
            Record(label, trial, clock, log, terminal.FilesCommitted, terminal.BytesCommitted, terminal.OutputPublished == true, terminal.CleanupComplete);
            if (terminal.Outcome == "succeeded") timings.Add((label, clock.Elapsed.TotalMilliseconds));
            Console.WriteLine("ARCHIVE_OPT_PUBLICATION " + JsonSerializer.Serialize(new { operation = label, trial, terminal.OutputPublished,
                terminal.FilesCommitted, terminal.FilesVerified, terminal.BytesVerified, terminal.CommitState, terminal.Journal }));
            return terminal;
        }
    }
    private static async Task<ArchiveBrowserContext> EditContext(string root, string path)
    {
        await using var session = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(wire); Check(session.Dispatch(wire).Success, "edit source browse");
        while (true) {
            var value = (await session.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(90))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (value.Batch is { } batch) Check(session.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "edit browse credit");
            if (value.Terminal is { } terminal) {
                Check(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser is not null, "edit browse terminal " + terminal.Code);
                return terminal.Browser!;
            }
        }
    }
    public static async Task CompletedCreationBaseline()
    {
        var root = Root(); var owned = Path.Combine(root, "output-" + Guid.NewGuid()); Directory.CreateDirectory(owned);
        var passed = false;
        try {
            var input = new byte[150000]; new Random(7123).NextBytes(input);
            var sources = new[] { Path.Combine(owned, "first.bin"), Path.Combine(owned, "second.bin") };
            foreach (var path in sources) File.WriteAllBytes(path, input);
            ArchiveJobRequest Create(string source, string destination, string name, bool test = false, ulong? volume = null) =>
                new() { Kind = "create", Paths = [source], Destination = destination, Create = new() {
                    Format = "zip", Level = "fast", Method = "deflate", StripSingleRoot = false, ExcludePatterns = [],
                    TestAfterCreate = test, Name = name, VolumeBytes = volume } };
            void Independent(string archive, int expected) {
                using var zip = ZipFile.OpenRead(archive); Check(zip.Entries.Count == expected, "created member count");
                foreach (var entry in zip.Entries) { using var stream = entry.Open(); using var bytes = new MemoryStream(); stream.CopyTo(bytes); Check(bytes.ToArray().SequenceEqual(input), "independent created payload"); }
            }
            var timings = new List<(string Label, double Ms)>();
            var splitFailures = new List<string>();
            for (var trial = 0; trial < 3; trial++) {
                var output = Path.Combine(owned, "trial-" + trial); Directory.CreateDirectory(output);
                var ordinary = Path.Combine(output, "ordinary"); Directory.CreateDirectory(ordinary);
                var made = await MeasuredJob(root, "create", Create(sources[0], ordinary, "created.zip"), trial, timings);
                Check(made.FilesCommitted == 1 && made.OutputPublished == true && made.Journal is { Count: 1 } && made.Journal[0].State == "Committed", "create native publication");
                var archive = Path.Combine(ordinary, "created.zip"); Independent(archive, 1);
                var each = Path.Combine(output, "each"); Directory.CreateDirectory(each);
                // Each-input is two original jobs, not one multi-source/single-target create request.
                for (var index = 0; index < sources.Length; index++) {
                    var name = $"item-{index}.zip";
                    var eachResult = await MeasuredJob(root, "create_each_" + index, Create(sources[index], each, name), trial, timings);
                    Check(eachResult.FilesCommitted == 1 && eachResult.OutputPublished == true && eachResult.Journal is { Count: 1 } && eachResult.Journal[0].State == "Committed", "each native publication");
                    Independent(Path.Combine(each, name), 1);
                }
                var tested = Path.Combine(output, "post-test"); Directory.CreateDirectory(tested);
                var created = await MeasuredJob(root, "create_post_test", Create(sources[0], tested, "tested.zip", test: true), trial, timings);
                Check(created.FilesCommitted == 1 && created.OutputPublished == true && created.Journal is { Count: 1 } && created.Journal[0].State == "Committed", "post-flag creation native publication");
                var postArchive = Path.Combine(tested, "tested.zip"); Independent(postArchive, 1);
                var postTest = await MeasuredJob(root, "post_create_explicit_test", new() { Kind = "test", Paths = [postArchive] }, trial, timings);
                Check(postTest.BytesVerified == (ulong)input.Length && postTest.FilesVerified == 1 && postTest.CleanupComplete, "explicit post-create Test decoded bytes");
                var context = await EditContext(root, archive);
                var added = await MeasuredJob(root, "add", new() { Kind = "modify", Paths = [archive, sources[1]], Modify = new(context, "add", [], null) }, trial, timings);
                Check(added.FilesCommitted == 1 && added.OutputPublished == true && added.Journal is { Count: 1 } && added.Journal[0].Replaced && added.Journal[0].State == "Committed", "Add native replacement");
                Independent(archive, 2);
                var split = Path.Combine(output, "split"); Directory.CreateDirectory(split);
                var volumes = await MeasuredJob(root, "create_split", Create(sources[0], split, "volumes.zip", volume: 65536), trial, timings);
                Check(volumes.OutputPublished == true && volumes.FilesCommitted >= 2 && volumes.Journal is { } journal &&
                    (ulong)journal.Count == volumes.FilesCommitted && journal.All(j => j.State == "Committed" && j.Published == true), "split native volume publication");
                var primary = Path.Combine(split, "volumes.zip");
                using (var family = ArchiveVolumeSources.Open(primary, 128UL << 20, 1UL << 30))
                using (var reservation = ArchiveSourceLease.ReserveActivity(primary, false, Guid.NewGuid())) {
                    Check(reservation.Identity == family.Requested.ActivityIdentity &&
                        reservation.Identity != family.Primary.ActivityIdentity, "requested volume identity differs from decoder primary");
                    Console.WriteLine("ARCHIVE_OPT_SPLIT_NATIVE " + JsonSerializer.Serialize(new { trial, family_count = family.Paths.Count,
                        family_verified = family.Identity().Length > 0, reservation_matches_requested = reservation.Identity == family.Requested.ActivityIdentity }));
                }
                var verified = await MeasuredJob(root, "split_test", new() { Kind = "test", Paths = [primary] }, trial, timings, requireSuccess: false);
                if (verified.Outcome != "succeeded" || verified.BytesVerified != (ulong)input.Length || !verified.CleanupComplete)
                    splitFailures.Add("trial=" + trial + " split_test=" + verified.Code);
                var extracted = Path.Combine(output, "split-extracted"); Directory.CreateDirectory(extracted);
                var splitExtract = await MeasuredJob(root, "split_extract", new() { Kind = "extract", Paths = [primary], Destination = extracted,
                    Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "skip", CopyMotw = false } }, trial, timings, requireSuccess: false);
                if (splitExtract.Outcome == "succeeded") {
                    Check(splitExtract.FilesCommitted == 1 && splitExtract.OutputPublished == true && splitExtract.CleanupComplete, "split native extraction publication");
                    Check(File.ReadAllBytes(Path.Combine(extracted, "first.bin")).SequenceEqual(input), "split actual output bytes");
                } else {
                    Check(splitExtract.FilesCommitted == 0 && splitExtract.OutputPublished == false && splitExtract.CleanupComplete, "split extraction failed without publication");
                    splitFailures.Add("trial=" + trial + " split_extract=" + splitExtract.Code);
                }
                Check(!Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "no creation partials");
                foreach (var path in sources) { using var lease = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); Check(lease.Length == input.Length, "source lease released"); }
            }
            foreach (var group in timings.GroupBy(t => t.Label)) {
                var values = group.Select(t => t.Ms).Order().ToArray();
                Check(values.Length == 3, "three completed " + group.Key + " trials");
                Console.WriteLine("ARCHIVE_OPT_CREATION_MEDIAN " + JsonSerializer.Serialize(new { operation = group.Key, trials_ms = values, median_ms = values[1], cache_policy = "first_measured_use_then_reused_source_no_flush" }));
            }
            Check(splitFailures.Count == 0, "split native read failures: " + string.Join(", ", splitFailures));
            passed = true;
        }
        finally {
            if (passed) Directory.Delete(owned, true);
            else Console.WriteLine("ARCHIVE_OPT_PRESERVED " + owned);
            File.Delete(Path.Combine(root, ArchiveReadHelper.DeploymentName));
            Console.WriteLine("ARCHIVE_OPT_CLEANUP helper_removed=true creation_outputs_removed=" + passed + " fixture_retained=true");
        }
    }
}
