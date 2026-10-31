using System.Text.Json;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class TarStreamRiskTests
{
    private static void Check(bool value, string detail) => TarStreamSafetyTests.Check(value, detail);
    private static Task Until(Func<bool> condition) => TarStreamSafetyTests.Until(condition, "known stream Risk deadline");
    public static async Task KnownDeclarationsUseOriginalRiskTickets()
    {
#if !DEBUG
        await Task.CompletedTask;
        throw new TestSkippedException("Tiny known stream Risk uses only original lowered DEBUG thresholds.");
#else
        var package = TarStreamFixturePackage.Create("known_stream_risk");
        ArchivePasswordTicket? previous = null;
        foreach (var spec in new[] { new TarStreamFixtureSpec("known.xz", "Xz", false), new TarStreamFixtureSpec("known.zst", "Zstd", false) }) {
            foreach (var mode in new[] { "ordinary", "approve", "decline", "cancel", "stale" }) {
                await using var attempt = new TarStreamFixtureAttempt(package, spec.Format + "-" + mode, spec);
                Directory.CreateDirectory(attempt.Output);
                var old = Path.Combine(attempt.Output, "old"); File.WriteAllText(old, "untouched");
                var foreign = Path.Combine(attempt.Output, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                var before = Directory.GetFileSystemEntries(attempt.Output).Order().ToArray();
                const ulong entry = 8192, total = 16384;
                if (mode != "ordinary") attempt.Runner.RiskThresholdFixture = (1, 1);
                Check(attempt.Start(new(attempt.Id, "extract", attempt.Source, 936, TarStreamFixtureAttempt.Limits(entry, total),
                    OutputRoot: attempt.Output, OutputReadmission: TarStreamSafetyTests.Binding(attempt.Id),
                    ConflictPolicy: "overwrite", ExtractOptions: new(false))), "original known-stream admission");
                if (mode != "ordinary") {
                    await Until(() => attempt.Frames("risk_prompt").Length != 0 || attempt.Events.Any(e => e.Kind == "exited"));
                    var prompt = attempt.Frames("risk_prompt").Single().GetProperty("risk_prompt");
                    Check(prompt.GetProperty("declared_bytes").GetUInt64() == 4096
                        && prompt.GetProperty("packed_bytes").GetUInt64() == (ulong)new FileInfo(attempt.Source).Length
                        && prompt.GetProperty("admitted_entry_bytes").GetUInt64() == entry
                        && prompt.GetProperty("admitted_output_bytes").GetUInt64() == total,
                        "one logical stream has complete original declared/encoded bounds, not a TAR member ratio");
                    var ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                    Pending(attempt, before);
                    if (previous is not null) Check(!attempt.Runner.RespondRisk(attempt.Id, previous, entry), "cross-operation ticket cannot authorize");
                    Check(!attempt.Runner.RespondRisk(Guid.NewGuid(), ticket, entry), "wrong original operation refused");
                    if (mode == "decline") Check(attempt.Runner.RespondRisk(attempt.Id, ticket, null), "original typed decline");
                    else if (mode == "cancel") Check(attempt.Runner.Cancel(attempt.Id), "original waiting cancellation");
                    else {
                        if (mode == "stale") {
                            Check(attempt.Runner.RespondRisk(attempt.Id, ticket, entry + 1), "larger reply transported, not approved");
                            await Task.Delay(80); Pending(attempt, before);
                            Check(attempt.Runner.RespondRisk(attempt.Id, ticket with { Generation = ticket.Generation + 1 }, entry), "wrong generation transported only");
                            await Task.Delay(80); Pending(attempt, before);
                        }
                        Check(attempt.Runner.RespondRisk(attempt.Id, ticket, entry), "exact original admitted ceiling");
                    }
                    previous = ticket;
                }
                var terminal = await attempt.TerminalAsync();
                var denied = mode is "decline" or "cancel";
                Check(terminal.Category == (denied ? "cancelled" : "success") && terminal.FilesCommitted == (denied ? 0 : 1), "truthful original Risk terminal");
                if (denied) {
                    Check(attempt.Audit.Writes == 0 && attempt.Audit.Published == 0 && before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()), "denied Risk has zero payload/output effects");
                } else {
                    var expected = Enumerable.Repeat(spec.Format == "Xz" ? (byte)0 : (byte)'B', 4096).ToArray();
                    Check(File.ReadAllBytes(Path.Combine(attempt.Output, "known")).SequenceEqual(expected) && terminal.BytesCommitted == 4096 && attempt.Audit.Published == 1,
                        "original full decoder/native publication exact independent literal");
                }
                Check(attempt.Frames("risk_confirmed").Length == (denied || mode == "ordinary" ? 0 : 1)
                    && attempt.Frames("output_budget_shortfall").Length == 0, "Risk confirms current ceiling only, never budget readmission");
                Check(File.ReadAllText(old) == "untouched" && File.ReadAllText(foreign) == "foreign"
                    && Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 1, "only owned partials cleaned, prior bytes preserved");
            }
        }
#endif
    }
    private static void Pending(TarStreamFixtureAttempt attempt, string[] before)
    {
        Check(attempt.Runner.ActiveCount == 1 && !attempt.Events.Any(e => e.Kind == "exited") && attempt.Audit.Writes == 0 && attempt.Audit.Published == 0,
            "original wait retains worker and slot before decode/write");
        Check(attempt.Frames("risk_confirmed").Length == 0 && before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()), "invalid reply never changes current authority");
        Check(TarStreamFixturePackage.IsProtected(attempt.Source) && TarStreamFixturePackage.IsProtected(attempt.Package.Helper.Executable), "pending Risk keeps source/helper leases");
    }
}
