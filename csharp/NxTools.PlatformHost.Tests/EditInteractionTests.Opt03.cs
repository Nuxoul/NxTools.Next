using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Tests;

// Effects: explicit frozen worker and independent readers; only GUID-owned PI_SCRATCH_DIR
// archives/sources through original Job/native publication, no GUI/Shell/user archives.
internal static partial class EditInteractionTests
{
    internal static Task Opt03AddWireValidation()
    {
        var result = new ArchiveJobAddResult(300, [new("folder/new (2).txt", "renamed"), new("folder/old.txt", "overwritten")]);
        Check(result.Valid && !result.ToString().Contains("folder/",StringComparison.Ordinal) &&
            !result.Members[0].ToString().Contains("folder/",StringComparison.Ordinal), "bounded private Add display result");
        var encoded = JsonSerializer.SerializeToElement(result, JsonDefaults.Options);
        Check(encoded.GetProperty("total").GetUInt64() == 300 &&
            encoded.GetProperty("members")[0].GetProperty("outcome").GetString() == "renamed", "exact snake-case result wire");
        var decoded = encoded.Deserialize<ArchiveJobAddResult>(ArchiveJobWire.Strict);
        Check(decoded?.Valid == true && decoded.Total == 300 && decoded.Members.Select(member => (member.Path,member.Outcome)).SequenceEqual(result.Members.Select(member => (member.Path,member.Outcome))), "strict roundtrip");
        foreach (var invalid in new[] { new ArchiveJobAddResult(1, result.Members),
            new ArchiveJobAddResult(1, [new("../escape", "added")]),
            new ArchiveJobAddResult(1, [new("folder/ok", "skip")]),
            new ArchiveJobAddResult(1, [new(new string('x', 256), "added")]) })
            Check(!invalid.Valid, "reject invalid shape/path/outcome");
        Check(!new ArchiveJobAddResult(257, Enumerable.Repeat(new ArchiveJobAddMember("item.txt","added"),257).ToArray()).Valid,
            "bounded member subset");
        var unknown = JsonSerializer.SerializeToNode(result, JsonDefaults.Options)!;
        unknown["members"]![0]!["other"] = true;
        try { unknown.Deserialize<ArchiveJobAddResult>(ArchiveJobWire.Strict); throw new InvalidOperationException("unknown member accepted"); }
        catch (JsonException) { }
        var legacy = new ArchiveJobTerminal("succeeded",AddResult:null);
        Check(!JsonSerializer.SerializeToElement(legacy, JsonDefaults.Options).TryGetProperty("add_result", out _), "legacy terminal omission");
        return Task.CompletedTask;
    }

    internal static async Task Opt03NativeAddResults()
    {
        foreach (var format in new[] { "Zip", "SevenZ", "Tar", "TarGz", "TarXz" })
        foreach (var mode in new[] { "overwrite", "auto_rename", "skip", "empty" })
        {
            var fixture = new Opt02Fixture(format);
            await fixture.Seed();
            var context = await fixture.Browse();
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(fixture.Root);
            var name = format.StartsWith("Tar", StringComparison.Ordinal) ? "child.txt" : "same.txt";
            var sources = mode == "empty"
                ? new[] { fixture.Folder("result", "new-empty") }
                : new[] { fixture.Input("one", name, "new-one"), fixture.Input("two", name, "new-two") };
            var result = await Run(jobs, fixture.Add(context, mode == "empty" ? null : "folder/", sources),
                mode == "empty" ? null : new[] { mode == "auto_rename" ? "auto_rename" : mode },
                applyAll: mode != "empty", expectedPrompts: mode == "empty" ? 0 : 1);
            var expected = new Dictionary<string, string?>(fixture.Expected);
            (string path, string outcome)[] selected = mode switch {
                "overwrite" => [("folder/" + name, "overwritten")],
                "auto_rename" => [("folder/" + name[..^4] + " (2).txt", "renamed"), ("folder/" + name[..^4] + " (3).txt", "renamed")],
                "empty" => [("new-empty/", "added")],
                _ => []
            };
            if (mode == "overwrite") expected["folder/" + name] = "new-two";
            if (mode == "auto_rename") { expected[selected[0].path] = "new-one"; expected[selected[1].path] = "new-two"; }
            if (mode == "empty") expected["new-empty/"] = null;
            await fixture.Oracle(expected);
            Check(result.CleanupComplete && result.OutputPublished == (selected.Length > 0) &&
                result.FilesCommitted == (selected.Length > 0 ? 1ul : 0ul) &&
                result.AddResult?.Total == (selected.Length > 0 ? (ulong)selected.Length : null),
                "OPT03 actual publication/result gate: " + format + "/" + mode + "/" + result.Code);
            if (selected.Length == 0) { Check(result.AddResult is null, "OPT03 skip does not invent members"); continue; }
            Check(result.AddResult!.Members.Select(member => (member.Path, member.Outcome)).SequenceEqual(selected),
                "OPT03 final native Add member paths/outcomes: " + format + "/" + mode);
            var wire = JsonSerializer.SerializeToElement(result, JsonDefaults.Options);
            Check(wire.GetProperty("add_result").GetProperty("total").GetUInt64() == (ulong)selected.Length,
                "OPT03 terminal JSON publication");
        }
        var lateFixture = new Opt02Fixture("Zip");
        await lateFixture.Seed();
        var lateContext = await lateFixture.Browse();
        var lateSource = lateFixture.Input("late", "fresh.txt", "late-added");
        await using var lateJobs = ArchiveJobSession.ForOwnedTestPackage(lateFixture.Root);
        var lateCommand = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(),
            Generation = 1, Action = "start", Request = lateFixture.Add(lateContext, "folder/", lateSource) };
        var cancelled = false;
        lateJobs.ProbeOutputCommitForFixture(stage => {
            if (stage == "native_published" && !cancelled) {
                cancelled = true;
                Check(lateJobs.Dispatch(Wire(lateCommand with { Action = "cancel", Request = null })).Success,
                    "OPT03 late Add cancel admission");
            }
        });
        Check(lateJobs.Dispatch(Wire(lateCommand)).Success, "OPT03 original late Add start");
        ArchiveJobTerminal? lateTerminal = null;
        while (lateTerminal is null)
            lateTerminal = (await lateJobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(60)))
                .Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!.Terminal;
        Check(cancelled && lateTerminal.Outcome == "cancelled" && lateTerminal.FilesCommitted == 1 &&
            lateTerminal.CommitState == "Committed" && lateTerminal.OutputPublished == true &&
            lateTerminal.CleanupComplete && lateTerminal.AddResult is null && File.ReadAllText(lateSource) == "late-added",
            "OPT03 late Add retains publication without success selection");
        var lateExpected = new Dictionary<string, string?>(lateFixture.Expected) { ["folder/fresh.txt"] = "late-added" };
        await lateFixture.Oracle(lateExpected);
    }
}
