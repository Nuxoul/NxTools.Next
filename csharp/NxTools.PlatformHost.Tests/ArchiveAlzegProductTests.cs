using System.IO.Compression;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Actual original Read/Browser/Job/worker/native output over GUID-owned CC0 copies only.
internal static class ArchiveAlzegProductTests
{
    private static string Fixtures => Path.Combine(Directory.GetCurrentDirectory(), "rust", "crates", "nxtools-archive", "tests", "fixtures", "alzeg");
    private const string Sentence = "The quick brown fox jumps over the lazy dog. ";

    internal static async Task OrdinaryAlzBrowseTestExtract()
    {
        await Exact(["store_old.alz"], new() { ["text.txt"] = Repeat(80) });
        await Exact(["deflate_empty.alz"], Five());
    }
    internal static async Task OrdinaryEggBrowseTestExtract()
    {
        foreach (var name in new[] { "store5.egg", "deflate5.egg", "mixed_store_deflate5.egg", "lzma5.egg" })
            await Exact([name], Five());
        await Exact(["bzip2_big.egg"], new() { ["huge.txt"] = Repeat(180000) });
        await Exact(["empty_noblock.egg"], new() { ["empty.txt"] = [] });
    }
    internal static async Task AlzDlzBrowseTestExtract()
    {
        foreach (var fixture in new[] { "bzip2.alz", "bzip2_dlz.alz", "spec_alz_dlz_multiblock.alz" })
            await Exact([fixture], EncryptedExpected(fixture, "dlz"));
    }
    internal static async Task AlzDlzFailureLifecycle()
    {
        foreach (var mode in new[] { "damaged", "entry", "cancel", "write" })
            await WithSources(["spec_alz_dlz_multiblock.alz"], async (root, paths) =>
            {
                if (mode == "damaged") { var bytes = File.ReadAllBytes(paths[0]); bytes[23] ^= 1; File.WriteAllBytes(paths[0], bytes); }
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var request = Request("extract", paths[0], root);
                if (mode == "entry") request = request with { Limits = new() { MaxEntryBytes = 1 } };
                var command = Start(request); var cleanup = false; var triggered = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup") RejectWriter(paths[0]);
                    if (stage == "before_cleanup") cleanup = true;
                    if (!triggered && (mode == "cancel" && stage == "source_precommit" || mode == "write" && stage == "candidate_write"))
                    {
                        triggered = true;
                        if (mode == "write") throw new ArchiveOutputException("io_failure", 112);
                        Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original DLZ Job cancel");
                    }
                });
                Require(jobs.Dispatch(Wire(command)).Success, "original DLZ Job admission");
                var result = await Finish(jobs);
                var expected = mode switch { "damaged" => "corrupt_archive", "entry" => "resource_limit", "cancel" => "cancelled", _ => "io_failure" };
                Require(result.Outcome == (mode == "cancel" ? "cancelled" : "failed") && result.Code == expected, "typed DLZ failure " + result.Code);
                Require(cleanup && result.CleanupComplete && result.FilesCommitted == 0 && (mode is not ("cancel" or "write") || triggered), "actual DLZ stop and cleanup");
                Require(mode != "write" || result.NativeErrorCode == 112, "first DLZ native error retained");
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "no incomplete DLZ native output");
            });
    }
    internal static async Task EggSolidBrowseTestExtract()
    {
        foreach (var fixture in new[] { "solid.egg", "spec_egg_solid_deflate_three_blocks.egg" })
            await Exact([fixture], EncryptedExpected(fixture, "solid"));
    }
    internal static async Task EggSolidSelectedTailKeepsPredecessorsAndLimit()
    {
        foreach (var names in new[] { new[] { "solid.egg" }, new[] { "spec_egg_solid_deflate_three_blocks.egg" },
            Parts(Families(".alz").Single(item => item.GetProperty("first").GetString() == "split_store.alz")),
            Parts(Families(".egg").Single(item => item.GetProperty("first").GetString() == "split_store.vol1.egg")) })
            await WithSources(names, async (root, paths) =>
            {
                var (opened, catalog) = await Read(root, paths[0]);
                Require(opened.Status == "succeeded" && opened.Browser is not null, "solid Browse before selection " + opened.Code);
                var tail = catalog[^1];
                var expected = (names.Length == 1 ? EncryptedExpected(names[0], "solid") : Expected(Families(names[0].EndsWith(".alz", StringComparison.Ordinal) ? ".alz" : ".egg").Single(item => item.GetProperty("first").GetString() == names[0])))[tail.Name];
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var request = Request("extract", paths[0], root, new(opened.Browser!, [tail.Ordinal])) with
                { Limits = new() { MaxMetadataBytes = 8192, MaxEntryBytes = (ulong)expected.Length, MaxOutputBytes = (ulong)expected.Length } };
                var result = await Job(jobs, request);
                Require(result.Outcome == "succeeded" && result.FilesCommitted == 1 && result.BytesCommitted == (ulong)expected.Length && result.CleanupComplete, "selected solid tail with original output credit " + result.Code);
                var files = Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories);
                Require(files.Length == 1 && Path.GetFileName(files[0]) == tail.Name && File.ReadAllBytes(files[0]).SequenceEqual(expected), "only selected exact tail after predecessor decode");
                Released(paths);
            });
    }
    internal static async Task EggSolidFailureLifecycle()
    {
        foreach (var mode in new[] { "damaged", "entry", "cancel", "write" })
            await WithSources(["spec_egg_solid_deflate_three_blocks.egg"], async (root, paths) =>
            {
                if (mode == "damaged")
                {
                    using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "solid", "manifest.json")));
                    var header = manifest.RootElement.GetProperty("fixtures")[1].GetProperty("blocks")[2].GetProperty("header").GetInt32();
                    var bytes = File.ReadAllBytes(paths[0]); bytes[header + 14] ^= 1; File.WriteAllBytes(paths[0], bytes);
                }
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var request = Request("extract", paths[0], root);
                if (mode == "entry") request = request with { Limits = new() { MaxEntryBytes = 1 } };
                var command = Start(request); var cleanup = false; var triggered = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup") RejectWriter(paths[0]);
                    if (stage == "before_cleanup") cleanup = true;
                    if (!triggered && (mode == "cancel" && stage == "source_precommit" || mode == "write" && stage == "candidate_write"))
                    {
                        triggered = true;
                        if (mode == "write") throw new ArchiveOutputException("io_failure", 112);
                        Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original solid Job cancel");
                    }
                });
                Require(jobs.Dispatch(Wire(command)).Success, "original solid Job admission");
                var result = await Finish(jobs);
                var code = mode switch { "damaged" => "corrupt_archive", "entry" => "resource_limit", "cancel" => "cancelled", _ => "io_failure" };
                Require(result.Outcome == (mode == "cancel" ? "cancelled" : "failed") && result.Code == code, "typed solid failure " + result.Code);
                Require(cleanup && result.CleanupComplete && result.FilesCommitted == 0 && (mode is not ("cancel" or "write") || triggered), "actual solid stop and cleanup");
                Require(mode != "write" || result.NativeErrorCode == 112, "first solid native error retained");
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "no unverified or incomplete solid output");
            });
    }
    internal static async Task EggAzoBrowseTestExtract()
    {
        foreach (var fixture in new[] { "azo.egg", "optimal5.egg", "spec_egg_azo_multiblock.egg" })
            await Exact([fixture], EncryptedExpected(fixture, "azo"));
        await Exact(["azo_pe.egg"], EncryptedExpected("azo_pe.egg", "azo-pe"));
    }
    internal static async Task EggAzoFailureLifecycle()
    {
        foreach (var mode in new[] { "damaged", "entry", "cancel", "write" })
            await WithSources(["spec_egg_azo_multiblock.egg"], async (root, paths) =>
            {
                if (mode == "damaged") { var bytes = File.ReadAllBytes(paths[0]); bytes[88] ^= 1; File.WriteAllBytes(paths[0], bytes); }
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var request = Request("extract", paths[0], root);
                if (mode == "entry") request = request with { Limits = new() { MaxEntryBytes = 1 } };
                var command = Start(request); var cleanup = false; var triggered = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup") RejectWriter(paths[0]);
                    if (stage == "before_cleanup") cleanup = true;
                    if (!triggered && (mode == "cancel" && stage == "source_precommit" || mode == "write" && stage == "candidate_write"))
                    {
                        triggered = true;
                        if (mode == "write") throw new ArchiveOutputException("io_failure", 112);
                        Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original AZO Job cancel");
                    }
                });
                Require(jobs.Dispatch(Wire(command)).Success, "original AZO Job admission");
                var result = await Finish(jobs);
                var code = mode switch { "damaged" => "corrupt_archive", "entry" => "resource_limit", "cancel" => "cancelled", _ => "io_failure" };
                Require(result.Outcome == (mode == "cancel" ? "cancelled" : "failed") && result.Code == code, "typed AZO failure " + result.Code);
                Require(cleanup && result.CleanupComplete && result.FilesCommitted == 0 && (mode is not ("cancel" or "write") || triggered), "actual AZO stop and cleanup");
                Require(mode != "write" || result.NativeErrorCode == 112, "first AZO native error retained");
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "no unverified or incomplete AZO output");
            });
    }
    internal static async Task EggPosixKindsBrowseTestExtract()
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "kinds-short", "manifest.json")));
        foreach (var fixture in new[] { "posix.egg", "spec_egg_kind_headers.egg" })
            await WithSources([fixture], async (root, paths) =>
            {
                var family = manifest.RootElement.GetProperty("fixtures").EnumerateArray().Single(item => item.GetProperty("fixture").GetString() == fixture);
                var members = family.GetProperty("members").EnumerateArray().ToArray();
                var (opened, entries) = await Read(root, paths[0]);
                Require(opened.Status == "succeeded" && opened.CleanupComplete && opened.Browser is not null && entries.Length == members.Length, "POSIX/kind Browse " + opened.Code);
                for (var index = 0; index < members.Length; index++)
                {
                    var member = members[index]; var actual = entries[index];
                    Require(actual.Ordinal == (ulong)index && actual.Name == member.GetProperty("name").GetString() && actual.Kind == member.GetProperty("kind").GetString() && actual.Size == member.GetProperty("size").GetUInt64(), "header-order independent native catalog kind");
                }
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var cleanup = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup") RejectWriter(paths[0]);
                    if (stage == "before_cleanup") cleanup = true;
                });
                var tested = await Job(jobs, Request("test", paths[0], root));
                Require(tested.Outcome == "succeeded" && tested.FilesVerified == family.GetProperty("files").GetUInt64() && tested.BytesVerified == family.GetProperty("materializable_bytes").GetUInt64() && tested.CleanupComplete, "POSIX/kind File-only Test " + tested.Code);
                var selection = fixture == "posix.egg" ? new ArchiveJobSelection(opened.Browser!, entries.Select(item => item.Ordinal).ToArray()) : null;
                var result = await Job(jobs, Request("extract", paths[0], root, selection));
                Require(result.Outcome == (fixture == "posix.egg" ? "succeeded" : "succeeded_with_warnings") && result.FilesCommitted == family.GetProperty("files").GetUInt64() && result.BytesCommitted == family.GetProperty("materializable_bytes").GetUInt64() && result.FilesSkipped == (fixture == "posix.egg" ? 0u : 4u) && result.CleanupComplete && cleanup, "truthful native regular/skipped-special totals " + result.Code);
                Require(Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Length == (int)result.FilesCommitted, "special entries never materialize as regular output");
                foreach (var member in members.Where(item => item.GetProperty("kind").GetString() == "File"))
                {
                    using var file = File.OpenRead(Path.Combine(Fixtures, member.GetProperty("expected_gzip").GetString()!.Replace('/', Path.DirectorySeparatorChar)));
                    using var gzip = new GZipStream(file, CompressionMode.Decompress); using var bytes = new MemoryStream(); gzip.CopyTo(bytes);
                    var output = Path.Combine(root, "output", member.GetProperty("name").GetString()!.Replace('/', Path.DirectorySeparatorChar));
                    Require(File.ReadAllBytes(output).SequenceEqual(bytes.ToArray()), "independent POSIX/native bytes");
                    if (fixture == "posix.egg") Require(File.GetLastWriteTimeUtc(output) == DateTimeOffset.FromUnixTimeSeconds(member.GetProperty("posix").GetProperty("mtime").GetInt64()).UtcDateTime, "genuine POSIX native mtime restored exactly");
                }
                if (fixture != "posix.egg") Require(Directory.Exists(Path.Combine(root, "output", "empty-directory")), "original native directory output");
                Released(paths);
            });
    }
    internal static async Task EggShortAndPosixFailureLifecycle()
    {
        foreach (var method in new[] { 1, 2, 5, 6 })
            await WithSources([$"spec_egg_short_{method}.egg"], async (root, paths) =>
            {
                var (read, entries) = await Read(root, paths[0]);
                Require(read.Status == "failed" && read.Code == "unsupported_encryption" && read.CleanupComplete && entries.Length == 0, "typed unauthenticated-short Browse " + read.Code);
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var writes = 0; var cleanup = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage == "candidate_write") writes++;
                    if (stage == "before_cleanup") { cleanup = true; RejectWriter(paths[0]); }
                });
                foreach (var kind in new[] { "test", "extract" })
                {
                    var result = await Job(jobs, Request(kind, paths[0], root));
                    Require(result.Outcome == "failed" && result.Code == "unsupported_encryption" && result.FilesCommitted == 0 && result.CleanupComplete && cleanup && writes == 0, "short-header Job rejects without output " + result.Code);
                }
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "no short-profile output");
            });
        foreach (var mode in new[] { "cancel", "write" })
            await WithSources(["posix.egg"], async (root, paths) =>
            {
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var command = Start(Request("extract", paths[0], root)); var triggered = false; var cleanup = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup") RejectWriter(paths[0]);
                    if (stage == "before_cleanup") cleanup = true;
                    if (!triggered && (mode == "cancel" && stage == "source_precommit" || mode == "write" && stage == "candidate_write"))
                    {
                        triggered = true;
                        if (mode == "write") throw new ArchiveOutputException("io_failure", 112);
                        Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original POSIX Job cancel");
                    }
                });
                Require(jobs.Dispatch(Wire(command)).Success, "POSIX failure Job admission");
                var result = await Finish(jobs);
                Require(triggered && cleanup && result.CleanupComplete && result.FilesCommitted == 0 && result.Outcome == (mode == "cancel" ? "cancelled" : "failed") && result.Code == (mode == "cancel" ? "cancelled" : "io_failure"), "actual POSIX stop/first-error cleanup " + result.Code);
                Require(mode != "write" || result.NativeErrorCode == 112, "POSIX first native code retained");
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "no incomplete POSIX output");
            });
    }
    internal static async Task RawUnsafeNamesBrowseTestRejectExtract()
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "names", "manifest.json")));
        var members = manifest.RootElement.GetProperty("unsafe_names").EnumerateArray().ToArray();
        foreach (var fixture in new[] { "spec_raw_names.alz", "spec_raw_names.egg" })
            await WithSources([fixture], async (root, paths) =>
            {
                var (read, entries) = await Read(root, paths[0]);
                Require(read.Status == "succeeded" && read.CleanupComplete && read.Browser is not null && entries.Length == members.Length, "raw-path Browse " + read.Code);
                for (var index = 0; index < members.Length; index++)
                    Require(entries[index].Ordinal == (ulong)index && entries[index].Name == members[index].GetProperty("name").GetString() && entries[index].Size == members[index].GetProperty("size").GetUInt64() && entries[index].WarningCodes.Contains("UnsafePath"), "original raw name and unsafe warning preserved");
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var writes = 0; var cleanup = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage == "candidate_write") writes++;
                    if (stage == "before_cleanup") { cleanup = true; RejectWriter(paths[0]); }
                });
                var tested = await Job(jobs, Request("test", paths[0], root));
                Require(tested.Outcome == "succeeded" && tested.FilesVerified == (ulong)members.Length && tested.BytesVerified == (ulong)members.Sum(member => member.GetProperty("size").GetInt64()) && tested.CleanupComplete, "raw-path read-only integrity Test " + tested.Code);
                var result = await Job(jobs, Request("extract", paths[0], root, new(read.Browser!, entries.Select(entry => entry.Ordinal).ToArray())));
                Require(result.Outcome == "failed" && result.Code == "unsafe_path" && result.FilesCommitted == 0 && result.CleanupComplete && cleanup && writes == 0, "unsafe path rejected before candidate output " + result.Code);
                Require(!Directory.GetFileSystemEntries(Path.Combine(root, "output")).Any(), "no sanitized or escaping native output");
                Released(paths);
            });
    }
    internal static async Task EggNameDeclarationsTypedCleanup()
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "names", "manifest.json")));
        foreach (var profile in manifest.RootElement.GetProperty("profiles").EnumerateArray())
            await WithSources([profile.GetProperty("fixture").GetString()!], async (root, paths) =>
            {
                var code = profile.GetProperty("typed").GetString() == "UnsupportedMethod" ? "unsupported_method" : "unsupported_encryption";
                var (read, entries) = await Read(root, paths[0]);
                Require(read.Status == "failed" && read.Code == code && read.CleanupComplete && entries.Length == 0, "name declaration Browse " + read.Code);
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var writes = 0; var cleanup = false;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage == "candidate_write") writes++;
                    if (stage == "before_cleanup") { cleanup = true; if (profile.GetProperty("fixture").GetString() == "spec_egg_global_encrypt.egg") Released(paths); else RejectWriter(paths[0]); }
                });
                foreach (var kind in new[] { "test", "extract" })
                {
                    var result = await Job(jobs, Request(kind, paths[0], root));
                    Require(result.Outcome == "failed" && result.Code == code && result.FilesCommitted == 0 && result.CleanupComplete && cleanup && writes == 0, "name declaration Job is typed and output-free " + result.Code);
                }
                Require(!Directory.GetFileSystemEntries(Path.Combine(root, "output")).Any(), "no unsupported name output");
                Released(paths);
            });
    }
    internal static async Task SplitAlzBrowseTestExtract()
    {
        foreach (var family in Families(".alz")) await Exact(Parts(family), Expected(family));
    }
    internal static async Task SplitEggBrowseTestExtract()
    {
        foreach (var family in Families(".egg"))
        {
            var parts = Parts(family);
            await Exact(parts, Expected(family));
            await Exact(parts, Expected(family), parts.Length / 2);
        }
    }
    internal static async Task MissingCorruptOutcomesAndCleanup()
    {
        foreach (var suffix in new[] { ".alz", ".egg" })
            foreach (var code in new[] { "missing_volume", "corrupt_archive" })
                await WithSources(Parts(Families(suffix).First()), async (root, paths) =>
                {
                    if (code == "missing_volume") File.Delete(paths[^1]);
                    else
                    {
                        var bytes = File.ReadAllBytes(paths[0]);
                        bytes[suffix == ".alz" ? bytes.Length - 1 : 29] ^= 0x80;
                        File.WriteAllBytes(paths[0], bytes);
                    }
                    var (read, entries) = await Read(root, paths[0]);
                    Require(read.Status != "succeeded" && read.Code == code && read.CleanupComplete && entries.Length == 0, "typed native Browse failure " + read.Code);
                    await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                    foreach (var kind in new[] { "test", "extract" })
                    {
                        var result = await Job(jobs, Request(kind, paths[0], root));
                        Require(result.Outcome == "failed" && result.Code == code && result.FilesCommitted == 0 && result.CleanupComplete, "typed native " + kind + " failure " + result.Code);
                    }
                    Require(!Directory.EnumerateFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "failed source creates no output");
                });
    }
    internal static async Task ChangedContinuationRejectsOldBrowser()
    {
        foreach (var suffix in new[] { ".alz", ".egg" })
            await WithSources(Parts(Families(suffix).First()), async (root, paths) =>
            {
                var (read, entries) = await Read(root, paths[0]);
                Require(read.Status == "succeeded" && read.Browser is not null, "browser before source replacement " + read.Code);
                Released(paths);
                var changed = File.ReadAllBytes(paths[1]); changed[40] ^= 1; File.WriteAllBytes(paths[1], changed);
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var result = await Job(jobs, Request("extract", paths[0], root, new(read.Browser!, entries.Select(entry => entry.Ordinal).ToArray())));
                Require(result.Outcome == "failed" && result.Code == "source_changed" && result.FilesCommitted == 0 && result.CleanupComplete, "old all-source Browser rejects changed continuation " + result.Code);
                Require(!Directory.EnumerateFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "stale selection writes nothing");
            });
    }
    internal static Task CancelRetainsAllSourcesThroughCleanup() => FailureLifecycle("cancel");
    internal static Task NativeWriteFailureRetainsAllSourcesThroughCleanup() => FailureLifecycle("write");

    internal static async Task LowOutputReadmissionUsesCompleteSourceBinding()
    {
        foreach (var first in new[] { "split_store.alz", "split_store.vol1.egg" })
        {
            var family = Families(first.EndsWith(".alz", StringComparison.Ordinal) ? ".alz" : ".egg").Single(item => item.GetProperty("first").GetString() == first);
            var expected = Expected(family);
            await WithSources(Parts(family), async (root, paths) =>
            {
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var destination = Path.Combine(root, "readmitted");
                var request = Request("extract", paths[0], root) with
                {
                    Destination = destination, Limits = new() { MaxMetadataBytes = 8192, MaxEntryBytes = 1, MaxOutputBytes = 1 },
                    Readmission = new(Guid.NewGuid(), null),
                };
                var shortfall = await Job(jobs, request);
                Require(shortfall.Outcome == "failed" && shortfall.Code == "resource_limit" && shortfall.OutputShortfall is not null && shortfall.FilesCommitted == 0 && shortfall.CleanupComplete && !Directory.Exists(destination), "original source-bound output shortfall before any output " + shortfall.Code);
                var shortage = shortfall.OutputShortfall!;
                Require(shortage.Current.EntryBytes == 1 && shortage.Current.OutputBytes == 1 && shortage.Required.EntryBytes == 40000 && shortage.Required.OutputBytes == 160000, "exact four-member declared output shortfall");
                Released(paths);
                Directory.CreateDirectory(destination);
                var approved = request with
                {
                    Limits = request.Limits with { MaxEntryBytes = shortage.Required.EntryBytes, MaxOutputBytes = shortage.Required.OutputBytes },
                    Readmission = request.Readmission with { ExpectedSource = shortage.Source },
                };
                var result = await Job(jobs, approved);
                Require(result.Outcome == "succeeded" && result.FilesCommitted == 4 && result.BytesCommitted == 160000 && result.CleanupComplete, "output-only approved retry " + result.Code);
                Require(Directory.GetFiles(destination, "*", SearchOption.AllDirectories).Length == 4, "exact retry outputs");
                foreach (var pair in expected) Require(File.ReadAllBytes(Path.Combine(destination, pair.Key)).SequenceEqual(pair.Value), "readmitted bytes equal independent oracle");
                Released(paths);
            });
        }
    }
    internal static async Task SelectedMemberKeepsOriginalOutputLimit()
    {
        foreach (var first in new[] { "split_store.alz", "split_store.vol1.egg" })
        {
            var family = Families(first.EndsWith(".alz", StringComparison.Ordinal) ? ".alz" : ".egg").Single(item => item.GetProperty("first").GetString() == first);
            await WithSources(Parts(family), async (root, paths) =>
            {
                var (opened, catalog) = await Read(root, paths[0]);
                Require(opened.Status == "succeeded" && opened.Browser is not null, "Browse before selected-member quota");
                await using var entries = ArchiveEntrySession.ForOwnedTestPackage(root);
                var command = new ArchiveEntryCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "materialize", Path = paths[0], Ordinal = catalog[0].Ordinal, Browser = opened.Browser, CopyMotw = false };
                var outer = await entries.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_entry", data = command }, command.OperationId), 1).WaitAsync(TimeSpan.FromSeconds(60));
                var result = JsonSerializer.Deserialize<ArchiveEntryResult>(outer.Data!, JsonDefaults.Options)!;
                Require(!outer.Success && result.Code == "selected_output_limit" && result.Path is null && result.CleanupComplete && entries.ActiveWorkerCountForFixture == 0, "selected-member typed quota and actual cleanup " + result.Code);
                Require(!Directory.GetFiles(root, "manifest.json", SearchOption.AllDirectories).Any(), "failed selection grants no cache lease");
                Released(paths);
            });
        }
    }

    internal static async Task EncryptedAlzBrowseTestExtract()
    {
        foreach (var fixture in new[] { "enc_store.alz", "enc_deflate.alz", "zip20.alz" })
            await WithSources([fixture], async (root, paths) =>
            {
                var expected = EncryptedExpected(fixture);
                var (opened, catalog) = await Read(root, paths[0]);
                Require(opened.Status == "succeeded" && opened.Browser?.Format == "Alz" && opened.CleanupComplete && catalog.Length == expected.Count, "clear encrypted catalog " + opened.Code);
                foreach (var entry in catalog)
                {
                    Require(expected.TryGetValue(entry.Name, out var bytes) && entry.Size == (ulong)bytes.Length && entry.Encrypted == (entry.Name != "empty.txt"), "exact encrypted catalog name/size/flag");
                    if (entry.Name is "뷁테스트.txt" or "한글파일.txt") Require(entry.Provenance == "Heuristic" && entry.Codepage == 949, "encrypted ALZ truthful CP949 names");
                }
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var total = (ulong)expected.Values.Sum(bytes => (long)bytes.Length);
                var tested = await PasswordJob(jobs, Request("test", paths[0], root), root, paths[0], "test1234");
                Require(tested.Outcome == "succeeded" && tested.FilesVerified == (ulong)expected.Count && tested.BytesVerified == total && tested.CleanupComplete, "encrypted Test exact totals " + tested.Code);
                Released(paths);
                var selection = new ArchiveJobSelection(opened.Browser!, catalog.Select(entry => entry.Ordinal).ToArray());
                var extracted = await PasswordJob(jobs, Request("extract", paths[0], root, selection), root, paths[0], "test1234");
                Require(extracted.Outcome == "succeeded" && extracted.FilesCommitted == (ulong)expected.Count && extracted.BytesCommitted == total && extracted.CleanupComplete, "encrypted Browser-bound extract " + extracted.Code);
                var output = Path.Combine(root, "output");
                Require(Directory.GetFiles(output, "*", SearchOption.AllDirectories).Length == expected.Count, "exact encrypted output set");
                foreach (var pair in expected) Require(File.ReadAllBytes(Path.Combine(output, pair.Key.Replace('/', Path.DirectorySeparatorChar))).SequenceEqual(pair.Value), "decrypted native bytes equal independent oracle");
            });
    }
    internal static async Task EncryptedAlzPasswordFailureLifecycle()
    {
        foreach (var fixture in new[] { "enc_store.alz", "enc_deflate.alz" })
            foreach (var mode in new[] { "wrong", "cancel", "damaged" })
                await WithSources([fixture], async (root, paths) =>
                {
                    if (mode == "damaged")
                    {
                        var bytes = File.ReadAllBytes(paths[0]); bytes[fixture == "enc_store.alz" ? 51 : 48] ^= 0x80; File.WriteAllBytes(paths[0], bytes);
                    }
                    await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                    var writes = 0; var cleanup = false;
                    jobs.ProbeLifecycleForFixture(stage =>
                    {
                        if (stage == "candidate_write") writes++;
                        if (stage == "before_cleanup") { cleanup = true; RejectWriter(paths[0]); }
                    });
                    var terminal = await PasswordJob(jobs, Request("extract", paths[0], root), root, paths[0], mode == "cancel" ? null : mode == "wrong" ? "wrong-alz-password" : "test1234");
                    Require(terminal.Outcome == (mode == "cancel" ? "cancelled" : "failed") && terminal.Code == (mode == "cancel" ? "cancelled" : "authentication_or_data_error"), "truthful password/ciphertext terminal " + terminal.Code);
                    Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && cleanup && writes == 0 && !Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "failed preparation never writes; actual cleanup");
                });
    }
    internal static async Task EncryptedEggBrowseTestExtract()
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "egg-encrypted", "manifest.json")));
        foreach (var family in manifest.RootElement.GetProperty("fixtures").EnumerateArray())
        {
            var fixture = family.GetProperty("fixture").GetString()!;
            await WithSources([fixture], async (root, paths) =>
            {
                var expected = EncryptedExpected(fixture, "egg-encrypted");
                var (opened, catalog) = await Read(root, paths[0]);
                var members = family.GetProperty("members").EnumerateArray().ToArray();
                Require(opened.Status == "succeeded" && opened.Browser?.Format == "Egg" && opened.CleanupComplete && catalog.Length == members.Length, "clear EGG encrypted catalog " + opened.Code);
                for (var index = 0; index < catalog.Length; index++)
                    Require(catalog[index].Name == members[index].GetProperty("name").GetString() && catalog[index].Size == members[index].GetProperty("size").GetUInt64() && catalog[index].Encrypted == members[index].GetProperty("encrypted").GetBoolean(), "exact EGG catalog names sizes encryption flags");
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var total = family.GetProperty("bytes").GetUInt64();
                var tested = await PasswordJob(jobs, Request("test", paths[0], root), root, paths[0], "test1234");
                Require(tested.Outcome == "succeeded" && tested.FilesVerified == (ulong)expected.Count && tested.BytesVerified == total && tested.CleanupComplete, "encrypted EGG Test exact totals " + tested.Code);
                Released(paths);
                var selection = new ArchiveJobSelection(opened.Browser!, catalog.Select(entry => entry.Ordinal).ToArray());
                var extracted = await PasswordJob(jobs, Request("extract", paths[0], root, selection), root, paths[0], "test1234");
                Require(extracted.Outcome == "succeeded" && extracted.FilesCommitted == (ulong)expected.Count && extracted.BytesCommitted == total && extracted.CleanupComplete, "encrypted EGG Browser-bound native extraction " + extracted.Code);
                var output = Path.Combine(root, "output");
                Require(Directory.GetFiles(output, "*", SearchOption.AllDirectories).Length == expected.Count, "exact EGG output set");
                foreach (var pair in expected) Require(File.ReadAllBytes(Path.Combine(output, pair.Key.Replace('/', Path.DirectorySeparatorChar))).SequenceEqual(pair.Value), "EGG native bytes equal independent oracle");
            });
        }
    }
    internal static async Task EncryptedEggPasswordFailureLifecycle()
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "egg-encrypted", "manifest.json")));
        foreach (var fixture in new[] { "enc_zip20.egg", "enc_aes128.egg", "enc_aes256.egg", "lea128.egg", "lea256.egg" })
            foreach (var mode in new[] { "wrong", "cancel", "ciphertext", "tag" })
                await WithSources([fixture], async (root, paths) =>
                {
                    var member = manifest.RootElement.GetProperty("fixtures").EnumerateArray().Single(f => f.GetProperty("fixture").GetString() == fixture).GetProperty("members").EnumerateArray().First(m => m.GetProperty("encrypted").GetBoolean());
                    if (mode is "ciphertext" or "tag")
                    {
                        var offset = mode == "ciphertext" ? member.GetProperty("blocks")[0].GetProperty("offset").GetInt32() : member.GetProperty("encrypt_info_offset").GetInt32() + member.GetProperty("encrypt_info_size").GetInt32() - 1;
                        var bytes = File.ReadAllBytes(paths[0]); bytes[offset] ^= 0x80; File.WriteAllBytes(paths[0], bytes);
                    }
                    await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                    var writes = 0; var cleanup = false;
                    jobs.ProbeLifecycleForFixture(stage =>
                    {
                        if (stage == "candidate_write") writes++;
                        if (stage == "before_cleanup") { cleanup = true; RejectWriter(paths[0]); }
                    });
                    var terminal = await PasswordJob(jobs, Request("extract", paths[0], root), root, paths[0], mode == "cancel" ? null : mode == "wrong" ? "wrong-alz-password" : "test1234");
                    Require(terminal.Outcome == (mode == "cancel" ? "cancelled" : "failed") && terminal.Code == (mode == "cancel" ? "cancelled" : "authentication_or_data_error"), "truthful EGG password/authentication outcome " + terminal.Code);
                    Require(terminal.FilesCommitted == 0 && terminal.CleanupComplete && cleanup && writes == 0 && !Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "EGG failed preparation never writes and actual cleanup completes");
                });
    }
    private static async Task<ArchiveJobTerminal> PasswordJob(ArchiveJobSession jobs, ArchiveJobRequest request, string root, string source, string? password)
    {
        var command = Start(request); var prompts = 0;
        Require(jobs.Dispatch(Wire(command)).Success, "original encrypted Job admission");
        while (true)
        {
            var frame = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(60))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Require(frame.OperationId == command.OperationId, "original password Job correlation");
            if (frame.Prompt is { } prompt)
            {
                Require(++prompts == 1 && prompt.Kind == "password" && prompt.ErrorCode == "PasswordRequired", "one original password ticket before output");
                Require(!Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "password wait has no output");
                RejectWriter(source); RejectWriter(Path.Combine(root, ArchiveReadHelper.DeploymentName));
                var answer = new ArchiveJobReply { Ticket = prompt.Ticket, Kind = "password", ApplyToAll = false, Password = password is null ? null : Encoding.UTF8.GetBytes(password) };
                Require(!jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = answer with { Ticket = prompt.Ticket with { Generation = 99 } } })).Success, "stale ticket rejected");
                Require(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = answer })).Success, "private current password answer");
            }
            if (frame.Terminal is { } terminal) { Require(prompts == 1, "actual encrypted Job reaches original password session " + terminal.Code); return terminal; }
        }
    }
    private static Dictionary<string, byte[]> EncryptedExpected(string fixture, string profile = "encrypted")
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", profile, "manifest.json")));
        return manifest.RootElement.GetProperty("fixtures").EnumerateArray().Single(item => item.GetProperty("fixture").GetString() == fixture).GetProperty("members").EnumerateArray().ToDictionary(member => member.GetProperty("name").GetString()!, member =>
        {
            using var file = File.OpenRead(Path.Combine(Fixtures, "expected", profile, member.GetProperty("expected").GetString()!));
            using var gzip = new GZipStream(file, CompressionMode.Decompress); using var output = new MemoryStream(); gzip.CopyTo(output); return output.ToArray();
        });
    }

    private static async Task FailureLifecycle(string mode)
    {
        foreach (var suffix in new[] { ".alz", ".egg" })
            await WithSources(Parts(Families(suffix).First()), async (root, paths) =>
            {
                await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
                var command = Start(Request("extract", paths[0], root));
                var triggered = false; var cleanup = false; var commits = 0;
                jobs.ProbeLifecycleForFixture(stage =>
                {
                    if (stage is "source_validated" or "source_precommit" or "before_cleanup")
                        foreach (var path in paths) RejectWriter(path);
                    if (stage == "before_cleanup") cleanup = true;
                    if (stage == "source_precommit") commits++;
                    if (!triggered && (mode == "cancel" && stage == "source_precommit" || mode == "write" && stage == "candidate_write"))
                    {
                        triggered = true;
                        if (mode == "write") throw new ArchiveOutputException("io_failure", 112);
                        Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original current Job cancellation");
                    }
                });
                Require(jobs.Dispatch(Wire(command)).Success, "failure lifecycle Job admission");
                var result = await Finish(jobs);
                Require(triggered && cleanup && result.CleanupComplete && result.FilesCommitted == 0, "actual failure barrier and resource cleanup");
                Require(result.Outcome == (mode == "cancel" ? "cancelled" : "failed") && result.Code == (mode == "cancel" ? "cancelled" : "io_failure"), "typed original failure " + result.Code);
                Require(mode != "write" || result.NativeErrorCode == 112, "first native write code retained");
                Require(mode != "cancel" || commits == 1, "cancel before first close");
                Require(!Directory.EnumerateFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories).Any(), "unfinished output removed after actual exit");
            });
    }

    private static async Task Exact(string[] names, Dictionary<string, byte[]> expected, int entry = 0)
    {
        await WithSources(names, async (root, paths) =>
        {
            var source = paths[entry];
            var (opened, entries) = await Read(root, source);
            Require(opened.Status == "succeeded" && opened.CleanupComplete && opened.Browser is not null, "production Browse terminal " + opened.Code);
            var format = names[0].EndsWith(".alz", StringComparison.Ordinal) ? "Alz" : "Egg";
            Require(opened.Browser!.Format == format && opened.Browser.Valid, "actual public Browser format");
            Require(entries.Length == expected.Count && entries.Select(item => item.Ordinal).SequenceEqual(Enumerable.Range(0, expected.Count).Select(index => (ulong)index)), "complete original catalog ordinals");
            foreach (var item in entries)
            {
                Require(expected.TryGetValue(item.Name, out var bytes) && item.Size == (ulong)bytes.Length && item.Kind == "File", "independent exact name/kind/size");
                if (item.Name is "뷁테스트.txt" or "한글파일.txt") Require(item.Provenance == "Heuristic" && item.Codepage == 949, "truthful Auto CP949 provenance");
            }
            Released(paths);
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var tested = await Job(jobs, Request("test", source, root));
            var total = (ulong)expected.Values.Sum(bytes => (long)bytes.Length);
            Require(tested.Outcome == "succeeded" && tested.FilesVerified == (ulong)expected.Count && tested.BytesVerified == total && tested.CleanupComplete, "original Test exact totals " + tested.Code);
            Released(paths);
            var selection = new ArchiveJobSelection(opened.Browser, entries.Select(item => item.Ordinal).ToArray());
            var extracted = await Job(jobs, Request("extract", source, root, selection));
            Require(extracted.Outcome == "succeeded" && extracted.FilesCommitted == (ulong)expected.Count && extracted.BytesCommitted == total && extracted.CleanupComplete, "Browser-bound native extract totals " + extracted.Code);
            var output = Path.Combine(root, "output");
            Require(Directory.EnumerateFiles(output, "*", SearchOption.AllDirectories).Count() == expected.Count, "no missing or extra native outputs");
            foreach (var pair in expected) Require(File.ReadAllBytes(Path.Combine(output, pair.Key.Replace('/', Path.DirectorySeparatorChar))).SequenceEqual(pair.Value), "native output equals independent bytes");
        });
    }

    private static async Task<(ArchiveReadTerminal Terminal, ArchiveReadEntry[] Entries)> Read(string root, string path)
    {
        await using var read = ArchiveReadSession.ForOwnedTestPackage(root);
        var wire = ArchiveProductReadTests.Command(path: path, version: 2, limits: new() { BrowserContext = true });
        var identity = ArchiveReadWire.Parse(wire);
        Require(read.Dispatch(wire).Success, "original Read admission");
        var entries = new List<ArchiveReadEntry>();
        while (true)
        {
            var frame = (await read.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(60))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
            if (frame.Batch is { } batch)
            {
                entries.AddRange(batch.Entries);
                Require(read.Dispatch(ArchiveProductReadTests.Command("ack", identity.OperationId, identity.SessionId, identity.Generation, sequence: batch.Sequence, version: 2)).Success, "original metadata credit");
            }
            if (frame.Terminal is { } terminal) return (terminal, entries.ToArray());
        }
    }
    private static ArchiveJobRequest Request(string kind, string path, string root, ArchiveJobSelection? selection = null) => new()
    {
        Kind = kind, Paths = [path], Destination = kind == "extract" ? Path.Combine(root, "output") : null,
        Extract = kind == "extract" ? new() { Strategy = "here", Conflict = "skip", SingleFileFolder = false, CopyMotw = false, Selection = selection } : null,
    };
    private static ArchiveJobCommand Start(ArchiveJobRequest request) => new() { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start", Request = request };
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    private static async Task<ArchiveJobTerminal> Job(ArchiveJobSession jobs, ArchiveJobRequest request)
    {
        Require(jobs.Dispatch(Wire(Start(request))).Success, "original Job admission " + request.Kind);
        return await Finish(jobs);
    }
    private static async Task<ArchiveJobTerminal> Finish(ArchiveJobSession jobs)
    {
        while (true)
        {
            var frame = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(60))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            Require(frame.Prompt is null, "unexpected prompt " + frame.Prompt?.Kind);
            if (frame.Terminal is { } terminal) return terminal;
        }
    }
    private static async Task WithSources(string[] names, Func<string, string[], Task> test)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE") ?? throw new InvalidOperationException("Explicit matching built worker required");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("Explicit scratch required");
        var root = Directory.CreateDirectory(Path.Combine(scratch, "archive-product-" + Guid.NewGuid())).FullName;
        try
        {
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var sources = Directory.CreateDirectory(Path.Combine(root, "sources")).FullName;
            Directory.CreateDirectory(Path.Combine(root, "output"));
            var paths = names.Select(name => Path.Combine(sources, name)).ToArray();
            for (var index = 0; index < names.Length; index++) File.Copy(Path.Combine(Fixtures, names[index]), paths[index]);
            await test(root, paths);
            Released(paths.Where(File.Exists));
        }
        finally { Directory.Delete(root, true); }
    }
    private static JsonElement[] Families(string suffix)
    {
        using var manifest = JsonDocument.Parse(File.ReadAllBytes(Path.Combine(Fixtures, "expected", "split", "manifest.json")));
        return manifest.RootElement.GetProperty("families").EnumerateArray().Where(family => family.GetProperty("first").GetString()!.EndsWith(suffix, StringComparison.Ordinal)).Select(family => family.Clone()).ToArray();
    }
    private static string[] Parts(JsonElement family) => family.GetProperty("parts").EnumerateArray().Select(part => part.GetString()!).ToArray();
    private static Dictionary<string, byte[]> Expected(JsonElement family) => family.GetProperty("members").EnumerateArray().ToDictionary(member => member.GetProperty("name").GetString()!, member =>
    {
        using var file = File.OpenRead(Path.Combine(Fixtures, "expected", "split", member.GetProperty("expected").GetString()!));
        using var gzip = new GZipStream(file, CompressionMode.Decompress);
        using var output = new MemoryStream(); gzip.CopyTo(output); return output.ToArray();
    });
    private static Dictionary<string, byte[]> Five() => new()
    {
        ["empty.txt"] = [], ["precompressed.gz"] = File.ReadAllBytes(Path.Combine(Fixtures, "expected", "1.bin")),
        ["prog.sys"] = File.ReadAllBytes(Path.Combine(Fixtures, "expected", "2.bin")), ["random.bin"] = File.ReadAllBytes(Path.Combine(Fixtures, "expected", "3.bin")),
        ["text.txt"] = File.ReadAllBytes(Path.Combine(Fixtures, "expected", "4.bin")),
    };
    private static byte[] Repeat(int count) => Encoding.ASCII.GetBytes(string.Concat(Enumerable.Repeat(Sentence, count)));
    private static void Released(IEnumerable<string> paths) { foreach (var path in paths) using (var file = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { } }
    private static void RejectWriter(string path)
    {
        try { using var file = File.Open(path, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); }
        catch (IOException) { return; }
        throw new InvalidOperationException("all native sources must remain protected through cleanup");
    }
    private static void Require(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
}
