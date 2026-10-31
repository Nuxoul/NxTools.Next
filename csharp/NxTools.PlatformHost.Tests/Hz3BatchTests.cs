using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Exact owned native -> original Job terminal -> Desktop bridge evidence. No product/Shell/system effects.
internal static class Hz3BatchTests
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    internal static async Task NativeDirectoryPublicationAndZeroOutput()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var expected = Environment.GetEnvironmentVariable("NXTOOLS_EDIT_INTERACTION_WORKER_SHA256");
        if (worker is null || !File.Exists(worker) || expected is not { Length: 64 })
            throw new TestSkippedException("Explicit current matching frozen worker/hash required");
        Check(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(worker))).Equals(expected, StringComparison.OrdinalIgnoreCase), "matching worker");
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_HZ3_NATIVE_EVIDENCE");
        Check(evidence is not null, "Explicit owned HZ3 terminal evidence file required");
        var cases = new List<object>();
        var publicationFacts = new List<bool>();
        foreach (var directory in new[] { true, false })
        {
            var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-product-" + Guid.NewGuid());
            Directory.CreateDirectory(root);
            File.Copy(worker, Path.Combine(root, ArchiveReadHelper.DeploymentName));
            var source = Path.Combine(root, "owned.zip");
            var target = Path.Combine(root, "target");
            Directory.CreateDirectory(target);
            using (var zip = ZipFile.Open(source, ZipArchiveMode.Create))
            {
                if (directory) zip.CreateEntry("published-empty/");
                else { using var writer = new StreamWriter(zip.CreateEntry("unpublished.txt").Open()); writer.Write("owned payload"); }
            }
            var original = File.ReadAllBytes(source);
            var observationCommand = new ArchiveSpaceCommand(2, Guid.NewGuid(), Guid.NewGuid(), 1, source, target, null, null);
            var before = ArchiveSpacePreflight.Observe(observationCommand);
            Check(before.Success, "original native baseline");
            var baseline = JsonSerializer.Deserialize<ArchiveSpaceObservation>(before.Data!, JsonDefaults.Options)!;
            await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
            var hits = 0;
            jobs.ProbeOutputCommitForFixture(stage => {
                if (stage == (directory ? "before_directory_metadata" : "before_flush")) { hits++; throw new ArchiveOutputException("io_failure"); }
            });
            var command = new ArchiveJobCommand {
                Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
                Request = new() { Kind = "extract", Paths = [source], Destination = target,
                    Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "ask", CopyMotw = false } }
            };
            Check(jobs.Dispatch(ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId)).Success, "original Job dispatch");
            ArchiveJobTerminal? terminal = null;
            while (terminal is null)
            {
                var envelope = await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(40));
                var item = envelope.Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
                Check(item.Prompt is null, "no prompt needed");
                terminal = item.Terminal;
            }
            Check(hits == 1 && terminal is { Outcome: "failed", Code: "io_failure", FilesCommitted: 0, CleanupComplete: true }, "real requested native failure/cleanup");
            Check(File.ReadAllBytes(source).SequenceEqual(original), "source unchanged");
            Check(directory ? Directory.Exists(Path.Combine(target, "published-empty")) && terminal.ResultLocation?.Target == Path.Combine(target, "published-empty")
                : Directory.GetFileSystemEntries(target).Length == 0 && terminal.ResultLocation is null, "surviving directory / real zero-output control");
            Check(!directory || terminal.Journal is { Count: 0 } && terminal.CommitState is null, "directory is outside file journal");
            var fresh = ArchiveSpacePreflight.Observe(observationCommand with { OperationId = Guid.NewGuid(), ExpectedSource = baseline.SourceSet!.Identity, ExpectedTarget = baseline.TargetIdentity });
            Check(fresh.Success, "source/target root observation still matches");
            var wireTerminal = JsonSerializer.SerializeToElement(terminal, JsonDefaults.Options);
            publicationFacts.Add(wireTerminal.TryGetProperty("output_published", out var published) && published.ValueKind == (directory ? JsonValueKind.True : JsonValueKind.False));
            cases.Add(new { name = directory ? "directory-published" : "zero-output", command, baseline, terminal, root, hits, source_unchanged = true, fresh_observation_matches = true });
            File.WriteAllText(Path.Combine(root, "terminal.json"), wireTerminal.GetRawText());
        }
        // Export measured original terminals even on RED; never hand-author safety facts in Desktop tests.
        Directory.CreateDirectory(Path.GetDirectoryName(evidence!)!);
        File.WriteAllText(evidence!, JsonSerializer.Serialize(new { worker_sha256 = expected, cases }, JsonDefaults.Options));
        Check(publicationFacts.All(value => value), "native owner must explicitly distinguish directory publication from proven zero output");
    }
    internal static Task PublicationWireAndRetainedDirectories()
    {
        const string legacy = "{\"outcome\":\"failed\",\"code\":\"io_failure\",\"journal\":[]}";
        var terminal = JsonSerializer.Deserialize<ArchiveJobTerminal>(legacy, ArchiveJobWire.Strict)!;
        Check(terminal.OutputPublished is null, "legacy missing publication remains unknown");
        Check(!JsonSerializer.SerializeToElement(terminal, JsonDefaults.Options).TryGetProperty("output_published", out _), "legacy omission preserved");
        foreach (var value in new[] { "null", "false", "true" })
        {
            var wire = legacy[..^1] + ",\"output_published\":" + value + "}";
            var parsed = JsonSerializer.Deserialize<ArchiveJobTerminal>(wire, ArchiveJobWire.Strict)!;
            Check(parsed.OutputPublished == (value == "null" ? (bool?)null : value == "true"), "symmetric optional boolean");
        }
        foreach (var value in new[] { "0", "\"false\"", "{}", "[]" })
        {
            try { JsonSerializer.Deserialize<ArchiveJobTerminal>(legacy[..^1] + ",\"output_published\":" + value + "}", ArchiveJobWire.Strict); throw new InvalidOperationException("nonboolean fact admitted"); }
            catch (JsonException) { }
        }
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        var untouched = ArchiveOutputSession.Prepare(root, Guid.NewGuid());
        Check(untouched.OutputPublished() is null, "live owner cannot claim final zero output");
        untouched.Dispose();
        Check(untouched.OutputPublished() == false && Directory.GetFileSystemEntries(root).Length == 0, "actual readonly Prepare proves zero after cleanup");
        var missing = Path.Combine(root, "retained-root", "uncreated-tail");
        var prepared = ArchiveOutputSession.Prepare(missing, Guid.NewGuid());
        var checkpoints = 0;
        try { prepared.CreatePreparedRoot(() => { if (++checkpoints == 2) throw new ArchiveOutputException("io_failure"); }); throw new InvalidOperationException("expected root creation checkpoint"); }
        catch (ArchiveOutputException failure) { Check(failure.Stage == "io_failure", "original boundary failure"); }
        prepared.Dispose();
        Check(prepared.OutputPublished() == true && Directory.Exists(Path.Combine(root, "retained-root")) && !Directory.Exists(missing), "partial root publication survives failure; never infer zero from file journal");
        var ancestry = ArchiveOutputSession.Open(root, Guid.NewGuid());
        var token = ancestry.BeginFile("retained-parent/unpublished.txt");
        Check(ancestry.Write(token, "owned"u8.ToArray()), "original partial write");
        ancestry.Dispose();
        Check(ancestry.OutputPublished() == true && Directory.Exists(Path.Combine(root, "retained-parent")) && Directory.GetFiles(Path.Combine(root, "retained-parent")).Length == 0, "created ancestry retained while exact unpublished partial cleans");
        var unknown = ArchiveOutputSession.Open(root, Guid.NewGuid());
        token = unknown.BeginFile("unknown.txt");
        Check(unknown.Write(token, "owned"u8.ToArray()), "uncertain candidate write");
        unknown.CommitCheckpointProbe = stage => { if (stage == "commit_in_flight") throw new ArchiveOutputException("rename_no_replace", 64); };
        Check(unknown.Commit(token).State == ArchiveCommitState.CommitOutcomeUnknown, "original unknown native precedence");
        unknown.Dispose();
        Check(unknown.OutputPublished() is null && unknown.CommitJournal().Single().Published is null, "unknown publication never becomes zero");
        return Task.CompletedTask;
    }
}
