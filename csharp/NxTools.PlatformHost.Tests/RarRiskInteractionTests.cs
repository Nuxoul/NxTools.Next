using System.Collections.Concurrent;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Exact attributed libarchive data; stock decoder hashes are independent of NxTools output.
internal static class RarRiskInteractionTests
{
    private static void Check(bool value, string detail) => RarRiskReadmissionTests.Check(value, detail);
    private static Task Until(Func<bool> condition) => RarRiskReadmissionTests.Until(condition, "RAR risk interaction deadline");
    private sealed record Compressed(RarFixtureSpec Spec, int Prompts, (string Name, ulong Size, string Hash)[] Files);
    private static readonly Compressed[] Inputs = [
        new(new("test_read_format_rar_compress_normal.rar", "Rar4", "", "", "7d32e2ef511d0ade372345440838165df828197eff4d7816880dafde690cce03"), 2, [
            ("LibarchiveAddingTest.html",20111,"ee16390e87152d7dec632ae2b37566da6135966e46b5166e6f40a1c827fa61ad"),
            ("testdir/test.txt",20,"5a5f16e01faf8adf92eb4499a2d3e93010c4b41dbb7f698f4a8466d9f58e6dd2"),
            ("testdir/LibarchiveAddingTest.html",20111,"ee16390e87152d7dec632ae2b37566da6135966e46b5166e6f40a1c827fa61ad")]),
        new(new("test_read_format_rar5_compressed.rar", "Rar5", "", "", "68f9d7e2662cd5bc5607ba9c6bf1edbf121cf3b08011a7d8643091791750a7c8"), 1, [
            ("test.bin",1200,"588870a2dade35c2650fbb7898c9a9c7f21fce7c281198604e8d0c9737f2c375")])
    ];
    public static async Task ActualRiskTickets()
    {
#if !DEBUG
        await Task.CompletedTask;
        throw new TestSkippedException("Only existing lowered DEBUG risk thresholds can use tiny positive samples.");
#else
        var package = RarFixturePackage.Create("compressed_risk_tickets");
        ArchivePasswordTicket? previous = null;
        foreach (var input in Inputs) foreach (var mode in new[] { "ordinary", "approve", "decline", "cancel", "stale" }) {
            await using var attempt = new RarFixtureAttempt(package, input.Spec.Format + "-" + mode, input.Spec);
            Directory.CreateDirectory(attempt.Output);
            var sentinel = Path.Combine(attempt.Output, "old-target"); File.WriteAllText(sentinel, "untouched");
            var foreign = Path.Combine(attempt.Output, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
            var before = Directory.GetFileSystemEntries(attempt.Output).Order().ToArray();
            if (mode != "ordinary") attempt.Runner.RiskThresholdFixture = (1, 1);
            const ulong admitted = 1UL << 20;
            Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, RarFixtureAttempt.Limits(admitted, admitted),
                OutputRoot: attempt.Output, ConflictPolicy: "overwrite", ExtractOptions: new(false))), "original compressed RAR admission");
            var declined = mode is "decline" or "cancel";
            ArchivePasswordTicket? first = null;
            if (mode != "ordinary") {
                for (var index = 0; index < (declined ? 1 : input.Prompts); index++) {
                    await Until(() => attempt.Frames("risk_prompt").Length > index || attempt.Events.Any(e => e.Kind == "exited"));
                    Check(attempt.Frames("risk_prompt").Length == index + 1, "exact reliable member Risk prompt");
                    var prompt = attempt.Frames("risk_prompt")[index].GetProperty("risk_prompt");
                    Check(prompt.GetProperty("admitted_entry_bytes").GetUInt64() == admitted && prompt.GetProperty("admitted_output_bytes").GetUInt64() == admitted,
                        "Risk reports original immutable ceilings");
                    var expanded = prompt.GetProperty("declared_bytes").GetUInt64(); var packed = prompt.GetProperty("packed_bytes").GetUInt64();
                    Check((input.Spec.Format == "Rar4" ? expanded == 20111 && packed == 7091 : expanded == 1200 && packed == 361), "exact independently listed positive member attribution");
                    var ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Check(ticket.OperationId == attempt.Id && ticket.PromptId != Guid.Empty, "original current-operation ticket");
                    AssertPending(attempt, before, index);
                    if (previous is not null && previous.OperationId != attempt.Id) Check(!attempt.Runner.RespondRisk(attempt.Id, previous, admitted), "cross-operation ticket refused");
                    Check(!attempt.Runner.RespondRisk(Guid.NewGuid(), ticket, admitted), "wrong operation cannot target live ticket");
                    if (mode == "decline") Check(attempt.Runner.RespondRisk(attempt.Id, ticket, null), "typed decline");
                    else if (mode == "cancel") Check(attempt.Runner.Cancel(attempt.Id), "cancel pending original wait");
                    else {
                        if (mode == "stale") {
                            Check(attempt.Runner.RespondRisk(attempt.Id, ticket, admitted + 1), "larger reply transported only");
                            await Task.Delay(60); AssertPending(attempt, before, index);
                            Check(attempt.Runner.RespondRisk(attempt.Id, ticket with { Generation = ticket.Generation + 1 }, admitted), "wrong generation transported only");
                            await Task.Delay(60); AssertPending(attempt, before, index);
                            if (first is not null) { Check(attempt.Runner.RespondRisk(attempt.Id, first, admitted), "duplicate earlier member ticket transported"); await Task.Delay(60); AssertPending(attempt, before, index); }
                        }
                        Check(attempt.Runner.RespondRisk(attempt.Id, ticket, admitted), "exact original ceiling approval");
                    }
                    first ??= ticket; previous = ticket;
                }
            }
            var terminal = await attempt.TerminalAsync();
            Check(attempt.Frames("output_budget_shortfall").Length == 0, "Risk never mutates output budgets or creates readmission");
            if (declined) {
                Check(terminal.Category == "cancelled" && terminal.FilesCommitted == 0 && attempt.Audit.Writes == 0 && attempt.Audit.Published == 0,
                    "decline/cancel stops before all output");
                Check(before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()), "no root/partial/output effects during rejected Risk");
            } else {
                Check(terminal.Category == (input.Spec.Format == "Rar4" ? "succeeded_with_warnings" : "success") && terminal.FilesCommitted == input.Files.Length,
                    "original compressed extraction and link-warning truth");
                foreach (var expected in input.Files) AssertHash(attempt.Output, expected.Name, expected.Size, expected.Hash);
                Check(attempt.Audit.Published == input.Files.Length && terminal.BytesCommitted == input.Files.Aggregate(0UL, (n, file) => n + file.Size), "native output counts and bytes");
            }
            Check(attempt.Frames("risk_confirmed").Length == (mode == "ordinary" || declined ? 0 : input.Prompts), "only exact current tickets confirmed");
            Check(File.ReadAllText(sentinel) == "untouched" && File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 1,
                "existing and foreign partial bytes retained");
        }
#endif
    }
    private static void AssertPending(RarFixtureAttempt attempt, string[] before, int confirmed)
    {
        Check(attempt.Runner.ActiveCount == 1 && !attempt.Events.Any(e => e.Kind == "exited") && attempt.Audit.Writes == 0 && attempt.Audit.Published == 0,
            "original Risk wait retains worker/slot, no current decode/output");
        Check(attempt.Frames("risk_confirmed").Length == confirmed && before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()), "invalid reply has no authority");
        Check(RarFixturePackage.IsProtected(attempt.Source) && RarFixturePackage.IsProtected(attempt.Package.Helper.Executable), "Risk retains original leases");
    }
    private static void AssertHash(string root, string name, ulong size, string hash)
    {
        var path = Path.Combine(root, name); Check(File.Exists(path) && (ulong)new FileInfo(path).Length == size && RarFixturePackage.Hash(path) == hash, "independent decoded payload hash");
    }
    public static async Task ActualSolidSkipAndIntegrity()
    {
        var package = RarFixturePackage.Create("solid_skip_integrity");
        var spec = new RarFixtureSpec("test_read_format_rar5_multiple_files_solid.rar", "Rar5", "", "", "0aa2c652c06304dedd93019663c1d52d5e85055d5fdb8045bfcc4345b90b4a0f");
        var hashes = new[] { "7d89f86f9f69d744ffff3fc043e15bf89fc3ffc134ffcbb31d164a99bb8b67b0", "f81e6fceeeab366306b23466bf6bb3aac2875e0906dc20a8652be0696ceb15a2", "5e621f2b6ce8fed758c3df8221f994eda55d1e432c7cc4349c34a30ec2e1c43d", "2627f40180217252956edb9a426e8d3e344adaf89019d3bccbe04f6c3416dcdd" };
        foreach (var mode in new[] { "open", "skip", "corrupt" }) {
            await using var attempt = new RarFixtureAttempt(package, mode, spec);
            if (mode != "open") { Directory.CreateDirectory(attempt.Output); File.WriteAllText(Path.Combine(attempt.Output, "test1.bin"), "old"); }
            if (mode == "corrupt") { var data = File.ReadAllBytes(attempt.Source); data[100] ^= 0x40; File.WriteAllBytes(attempt.Source, data); }
#if DEBUG
            if (mode != "open") attempt.Runner.RiskThresholdFixture = (1, 1);
#endif
            Check(attempt.Runner.Start(new(attempt.Id, mode == "open" ? "open" : "extract", attempt.Source, 936, RarFixtureAttempt.Limits(1UL << 20, 1UL << 20),
                OutputRoot: mode == "open" ? null : attempt.Output, ConflictPolicy: mode == "open" ? null : "skip", ExtractOptions: mode == "open" ? null : new(false))), "original solid admission");
            var terminal = await attempt.TerminalAsync();
            Check(attempt.Frames("risk_prompt").Length == 0 && attempt.Frames("risk_confirmed").Length == 0, "solid archive including first non-continuation member has unknown ratio");
            if (mode == "open") {
                var entries = attempt.Frames("entry_batch").SelectMany(f => f.GetProperty("entries").EnumerateArray()).ToArray();
                Check(terminal.Category == "success" && entries.Length == 4 && entries.All(e => e.GetProperty("size").GetUInt64() == 4096 && e.GetProperty("packed_size").ValueKind == JsonValueKind.Null), "known solid declarations and unknown shared packed");
            } else if (mode == "skip") {
                Check(terminal.Category == "succeeded_with_warnings" && terminal.FilesSkipped == 1 && terminal.FilesCommitted == 3 && terminal.BytesCommitted == 12288, "solid skip fully decodes remaining dictionary chain");
                for (var index = 1; index < 4; index++) AssertHash(attempt.Output, $"test{index + 1}.bin", 4096, hashes[index]);
                Check(File.ReadAllText(Path.Combine(attempt.Output, "test1.bin")) == "old", "skip retains old bytes");
            } else {
                Check(terminal.FilesCommitted == 0 && attempt.Audit.Published == 0 && File.ReadAllText(Path.Combine(attempt.Output, "test1.bin")) == "old", "corruption cannot publish staged solid bytes");
                Check(attempt.Frames("failure").Single().GetProperty("error_code").GetString() == "CorruptArchive", "native integrity failure not a budget shortage");
            }
            Check(attempt.Frames("output_budget_shortfall").Length == 0, "ordinary solid integrity failure never requests new budget");
        }
    }
    public static async Task ActualVolumePackedUnknown()
    {
        var package = RarFixturePackage.Create("volume_packed_unknown");
        foreach (var family in new[] { (Stem: "test_rar_multivolume_single_file.part", Width: 1, Count: 3, Files: 1, Bytes: 20111UL), (Stem: "test_read_format_rar5_multiarchive_solid.part", Width: 2, Count: 4, Files: 9, Bytes: 117398UL) }) {
            var root = Directory.CreateDirectory(Path.Combine(package.Root, "family" + family.Width)).FullName;
            var paths = Enumerable.Range(1, family.Count).Select(i => Path.Combine(root, family.Stem + i.ToString("D" + family.Width) + ".rar")).ToArray();
            foreach (var path in paths) File.Copy(Path.Combine(package.Fixtures, Path.GetFileName(path)), path);
            foreach (var missing in new[] { false, true }) {
                if (missing) File.Move(paths[^1], paths[^1] + ".missing");
                var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var audit = new RarFixtureAudit(); var id = Guid.NewGuid();
                using var runner = ArchiveWorkerRunner.ForProductRead(package.Helper, events.Enqueue, (op, frame, cancellation) => { events.Enqueue(new("frame", op, Frame: frame.Clone())); return Task.FromResult<string?>(null); }, audit);
                Check(runner.Start(new(id, "open", paths[0], 936, RarFixtureAttempt.Limits(1UL << 20, 1UL << 20))), "original ordered volume admission");
                await Until(() => events.Any(e => e.Kind == "exited")); Check(await runner.DrainAsync(), "actual volume cleanup");
                var terminal = events.Single(e => e.Kind == "exited");
                var frames = events.Where(e => e.Frame is not null).Select(e => e.Frame!.Value).ToArray();
                if (!missing) {
                    var entries = frames.Where(f => f.GetProperty("op").GetString() == "entry_batch").SelectMany(f => f.GetProperty("entries").EnumerateArray()).ToArray();
                    Check(terminal.Category == "success" && entries.Length == family.Files && entries.All(e => e.GetProperty("packed_size").ValueKind == JsonValueKind.Null)
                        && entries.Aggregate(0UL, (n, e) => n + e.GetProperty("size").GetUInt64()) == family.Bytes, "split/solid packed unknown, full member declarations not multiplied by parts");
                } else Check(terminal.Category != "success" && terminal.FilesCommitted == 0, "missing original family member rejects, no sibling substitution");
                Check(terminal.CleanupComplete && runner.ActiveCount == 0 && paths.Where(File.Exists).All(RarFixturePackage.IsReleased) && RarFixturePackage.IsReleased(package.Helper.Executable), "all original volume/helper leases released only after actual cleanup");
                File.WriteAllText(Path.Combine(root, missing ? "missing-result.json" : "metadata-result.json"), JsonSerializer.Serialize(new { terminal = RarFixtureAudit.SafeEvent(terminal), entries = frames.Where(f => f.GetProperty("op").GetString() == "entry_batch").Select(f => f.Clone()).ToArray() }));
            }
        }
    }
}
