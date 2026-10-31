using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Independent stock-7z inputs only. Retain every owned source/output/report, including failures.
internal static class SevenzRiskReadmissionTests
{
    internal static void Check(bool value, string detail)
    { if (!value) throw new InvalidOperationException(detail); }

    internal static async Task Until(Func<bool> condition, string detail)
    {
        var clock = Stopwatch.StartNew();
        while (!condition()) {
            Check(clock.ElapsedMilliseconds < 10000, detail);
            await Task.Delay(10);
        }
    }

    private static byte[] Expected(string name) => name == "first.bin"
        ? Enumerable.Range(0, 4096).Select(i => (byte)(i % 256)).ToArray()
        : Enumerable.Repeat((byte)'B', 2048).ToArray();

    public static async Task ActualSevenzPackedMetadata()
    {
        var package = SevenzFixturePackage.Create("packed_metadata");
        foreach (var solid in new[] { false, true }) {
            await using var attempt = new SevenzFixtureAttempt(package, solid ? "solid" : "nonsolid", solid ? "solid.7z" : "nonsolid.7z");
            var request = new ArchiveWorkerRequest(attempt.Id, "open", attempt.Source, 936, SevenzFixtureAttempt.Limits());
            Check(attempt.Runner.Start(request), "original metadata admission");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "success" && terminal.FilesCommitted == 0 && attempt.Audit.WriteBytes == 0, "metadata has no output authority");
            var entries = attempt.Frames("entry_batch").SelectMany(f => f.GetProperty("entries").EnumerateArray()).ToArray();
            Check(entries.Length == 2, "exact independent fixture member count");
            foreach (var name in new[] { "first.bin", "second.bin" }) {
                var entry = entries.Single(e => e.GetProperty("name").GetString() == name);
                Check(entry.GetProperty("size").GetUInt64() == (ulong)Expected(name).Length, "exact declared member size");
                var packed = entry.GetProperty("packed_size");
                Check(solid ? packed.ValueKind == JsonValueKind.Null : packed.ValueKind == JsonValueKind.Number && packed.GetUInt64() > 0,
                    "shared solid has no member ratio; independent blocks have positive packed bytes");
            }
            Check(attempt.Frames("risk_prompt").Length == 0 && !Directory.Exists(attempt.Output), "open never prompts or creates output");
        }
    }

    public static async Task ActualSevenzRiskAndSolidSemantics()
    {
#if !DEBUG
        await Task.CompletedTask;
        throw new TestSkippedException("Tiny 7z risk scenarios require the original DEBUG-only lowered threshold seam.");
#else
        var package = SevenzFixturePackage.Create("risk_and_solid");
        ArchivePasswordTicket? previousOperation = null;
        foreach (var mode in new[] { "ordinary", "approve", "decline", "original_cancel", "stale_and_larger", "solid", "solid_entry_limit", "solid_total_limit" }) {
            var solid = mode.StartsWith("solid", StringComparison.Ordinal);
            await using var attempt = new SevenzFixtureAttempt(package, mode, solid ? "solid.7z" : "nonsolid.7z");
            Directory.CreateDirectory(attempt.Output);
            var original = new byte[] { 11, 22, 33, 44 };
            var firstPath = Path.Combine(attempt.Output, "first.bin");
            File.WriteAllBytes(firstPath, original);
            var foreign = Path.Combine(attempt.Output, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
            var before = Directory.GetFileSystemEntries(attempt.Output).Order().ToArray();
            ulong entryLimit = mode == "solid_entry_limit" ? 3072UL : 8192UL;
            ulong outputLimit = mode == "solid_total_limit" ? 5000UL : 16384UL;
            if (mode != "ordinary") attempt.Runner.RiskThresholdFixture = (1, 1);
            var request = new ArchiveWorkerRequest(attempt.Id, "extract", attempt.Source, 936,
                SevenzFixtureAttempt.Limits(entryLimit, outputLimit), OutputRoot: attempt.Output,
                ConflictPolicy: "overwrite", ExtractOptions: new(false));
            Check(attempt.Runner.Start(request), "original 7z risk admission: " + mode);
            var prompting = mode is "approve" or "decline" or "original_cancel" or "stale_and_larger";
            if (prompting) {
                await Pending(attempt, 1, before, firstPath, original);
                var prompt = attempt.Frames("risk_prompt")[0].GetProperty("risk_prompt");
                var first = Ticket(prompt, attempt.Id, 0, 4096, entryLimit, outputLimit);
                if (previousOperation is not null)
                    Check(!attempt.Runner.RespondRisk(attempt.Id, previousOperation, entryLimit), "cross-operation ticket rejected at original boundary");
                Check(!attempt.Runner.RespondRisk(Guid.NewGuid(), first, entryLimit), "ticket cannot target another operation");
                if (mode == "decline") Check(attempt.Runner.RespondRisk(attempt.Id, first, null), "original typed decline");
                else if (mode == "original_cancel") Check(attempt.Runner.Cancel(attempt.Id), "original pending-wait cancellation");
                else {
                    if (mode == "stale_and_larger") {
                        await Transport(attempt, first, entryLimit + 1);
                        await StillPending(attempt, 1, 0, before, firstPath, original);
                        await Transport(attempt, first with { Generation = first.Generation + 1 }, entryLimit);
                        await StillPending(attempt, 1, 0, before, firstPath, original);
                    }
                    await Transport(attempt, first, entryLimit);
                    await Pending(attempt, 2, before, firstPath, original);
                    var nextPrompt = attempt.Frames("risk_prompt")[1].GetProperty("risk_prompt");
                    var second = Ticket(nextPrompt, attempt.Id, 1, 2048, entryLimit, outputLimit);
                    Check(second != first && second.PromptId != first.PromptId, "fresh member-scoped risk ticket");
                    Check(attempt.Frames("risk_confirmed").Length == 1, "first approval authorizes only its member");
                    if (mode == "stale_and_larger") {
                        await Transport(attempt, first, entryLimit); // Duplicate/stale ticket transported, not C# policy.
                        await StillPending(attempt, 2, 1, before, firstPath, original);
                    }
                    await Transport(attempt, second, entryLimit);
                    previousOperation = second;
                }
            }
            var terminal = await attempt.TerminalAsync();
            var cancelled = mode is "decline" or "original_cancel";
            var hardLimit = mode is "solid_entry_limit" or "solid_total_limit";
            if (cancelled || hardLimit) {
                Check(terminal.FilesCommitted == 0 && terminal.BytesCommitted == 0 && attempt.Audit.WriteBytes == 0 && attempt.Audit.Published == 0,
                    "7z confirms ALL wanted members before ANY decode/write/publication");
                Check(attempt.Frames("failure").Single().GetProperty("error_code").GetString() == (cancelled ? "Cancelled" : "ResourceLimit"), "original cancellation/hard budget category");
                Check(!cancelled || terminal.Category == "cancelled", "wait cancellation is not committed output");
                Check((terminal.Journal?.Count ?? 0) == 0, "declined/preflight-rejected operation has no native transactions");
                Check(before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()) && File.ReadAllBytes(firstPath).SequenceEqual(original), "zero output effects on rejection/cancel");
            } else {
                Check(terminal.Category == "success" && terminal.FilesCommitted == 2 && terminal.BytesCommitted == 6144, "exact native success count and bytes");
                Check(attempt.Audit.WriteBytes == 6144 && attempt.Audit.Published == 2, "actual native write/publication observations");
                AssertBytes(attempt.Output, "first.bin"); AssertBytes(attempt.Output, "second.bin");
                AssertPublishedJournal(terminal, 2, 6144);
            }
            Check(attempt.Frames("risk_confirmed").Length == (prompting && !cancelled ? 2 : 0), "only exact current admission answers confirmed");
            Check(attempt.Frames("risk_prompt").Length == (prompting ? cancelled ? 1 : 2 : 0), "ordinary/shared-solid/hard-refusal never fabricate a ratio prompt");
            Check(File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 1, "only owned partials cleaned");
            Check(!attempt.Runner.RespondRisk(attempt.Id, new(attempt.Id, Guid.NewGuid(), 1), entryLimit), "retired operation cannot approve");
        }
#endif
    }

    public static async Task ActualSevenzPublicationCancellationAndSkip()
    {
        var package = SevenzFixturePackage.Create("publication_cancel_skip");
        // A supplied corruption fixture is mandatory once present; no fallback/generated corruption.
        var modes = new List<string> { "solid_skip", "cancel_after_commit" };
        if (File.Exists(Path.Combine(package.Fixtures, "corrupt-solid.7z"))) modes.Add("corrupt_solid");
        foreach (var mode in modes) {
            await using var attempt = new SevenzFixtureAttempt(package, mode, mode == "corrupt_solid" ? "corrupt-solid.7z" : "solid.7z");
            Directory.CreateDirectory(attempt.Output);
            var original = new byte[] { 91, 92, 93 };
            var first = Path.Combine(attempt.Output, "first.bin");
            if (mode != "cancel_after_commit") File.WriteAllBytes(first, original);
            if (mode == "cancel_after_commit") attempt.Audit.CancelAfterCommit = true;
            Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, SevenzFixtureAttempt.Limits(),
                OutputRoot: attempt.Output, ConflictPolicy: mode == "solid_skip" ? "skip" : "overwrite", ExtractOptions: new(false))), "original publication admission");
            var terminal = await attempt.TerminalAsync();
            Check(attempt.Frames("risk_prompt").Length == 0, "tiny solid has no reliable member ratio");
            if (mode == "solid_skip") {
                Check(terminal.Category == "succeeded_with_warnings" && terminal.FilesCommitted == 1 && terminal.FilesSkipped == 1 && terminal.BytesCommitted == 2048, "truthful skip terminal");
                Check(File.ReadAllBytes(first).SequenceEqual(original), "skipped existing bytes retained");
                AssertBytes(attempt.Output, "second.bin");
                Check(attempt.Audit.WriteBytes == 2048 && attempt.Audit.Published == 1 && attempt.Audit.Checkpoints.Contains("source_precommit"), "remaining solid member fully decoded before publication");
                AssertPublishedJournal(terminal, 1, 2048);
            } else if (mode == "cancel_after_commit") {
                Check(attempt.Audit.CancelAccepted && terminal.Category == "cancelled_with_committed_output" && terminal.FilesCommitted == 1 && terminal.BytesCommitted == 2048, "original postpublication cancellation preserves native truth");
                // Current original 7z owner publishes the staged tokens in reverse order.
                AssertBytes(attempt.Output, "second.bin");
                Check(!File.Exists(first) && attempt.Audit.Published == 1 && attempt.Audit.WriteBytes == 6144, "one publication retained; other staged token cleaned without rerun");
                AssertPublishedJournal(terminal, 1, 2048);
            } else {
                Check(attempt.Frames("failure").Single().GetProperty("error_code").GetString() == "CorruptArchive", "actual solid checksum corruption reported");
                Check(terminal.FilesCommitted == 0 && terminal.BytesCommitted == 0 && attempt.Audit.Published == 0 && File.ReadAllBytes(first).SequenceEqual(original)
                    && !File.Exists(Path.Combine(attempt.Output, "second.bin")), "no invalid solid publication or old-target replacement");
                Check(terminal.Journal?.All(j => j.Published == false) != false, "corrupt staged output never published");
            }
            Check(Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 0 && File.Exists(attempt.Source), "actual owned cleanup retains original archive");
        }
    }

    public static async Task ActualSevenzLowTotalPreflight()
    {
        var package = SevenzFixturePackage.Create("low_total_preflight");
        await using var attempt = new SevenzFixtureAttempt(package, "total", "nonsolid.7z");
        var binding = JsonSerializer.SerializeToElement(new { logical_id = attempt.Id, expected_source = (object?)null });
        Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, SevenzFixtureAttempt.Limits(8192, 1),
            OutputRoot: attempt.Output, OutputReadmission: binding, ExtractOptions: new(false))), "original low total admission");
        var terminal = await attempt.TerminalAsync();
        var shortage = attempt.Frames("output_budget_shortfall");
        Check(shortage.Length == 1, "low total must reach declared 7z shortfall, not encoded-source rejection");
        Check(shortage[0].GetProperty("output_budget_shortfall").GetProperty("required").GetProperty("output_bytes").GetUInt64() == 6144,
            "exact independent output declaration");
        Check(terminal.FilesCommitted == 0 && terminal.CleanupComplete && !Directory.Exists(attempt.Output) && attempt.Audit.WriteBytes == 0,
            "original actual cleanup before finite budget approval, no output effects");
    }

    public static async Task ActualSevenzTestHasNoRiskInteraction()
    {
        var package = SevenzFixturePackage.Create("test_no_risk");
        await using var attempt = new SevenzFixtureAttempt(package, "test", "nonsolid.7z");
        Check(attempt.Runner.Start(new(attempt.Id, "test", attempt.Source, 936, SevenzFixtureAttempt.Limits())), "original no-output TEST admission");
        var terminal = await attempt.TerminalAsync();
        Check(terminal.Category == "success" && terminal.FilesCommitted == 0 && terminal.CleanupComplete && attempt.Audit.WriteBytes == 0,
            "no-output TEST does not grant extraction risk authority");
        Check(attempt.Frames("risk_prompt").Length == 0 && attempt.Frames("risk_confirmed").Length == 0 && !Directory.Exists(attempt.Output),
            "TEST never prompts or silently confirms extraction risk");
    }

    private static void AssertBytes(string output, string name) => Check(File.ReadAllBytes(Path.Combine(output, name)).SequenceEqual(Expected(name)), "exact independent native fixture bytes");
    private static void AssertPublishedJournal(ArchiveWorkerEvent terminal, int count, ulong bytes)
    {
        var published = terminal.Journal?.Where(j => j.Published == true).ToArray() ?? [];
        Check(published.Length == count && published.Aggregate(0UL, (n, j) => n + j.Bytes) == bytes
            && published.All(j => j.State == "Committed" && j.Cleanup is ("retained" or "retained_replacement")), "original native journal matches retained publications");
    }
    private static ArchivePasswordTicket Ticket(JsonElement prompt, Guid id, ulong ordinal, ulong declared, ulong entry, ulong output)
    {
        var ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
        Check(ticket.OperationId == id && ticket.PromptId != Guid.Empty && ticket.Generation > 0 && prompt.GetProperty("ordinal").GetUInt64() == ordinal
            && prompt.GetProperty("declared_bytes").GetUInt64() == declared && prompt.GetProperty("packed_bytes").GetUInt64() > 0
            && prompt.GetProperty("admitted_entry_bytes").GetUInt64() == entry && prompt.GetProperty("admitted_output_bytes").GetUInt64() == output, "exact unchanged original finite limits/member ticket");
        return ticket;
    }
    private static async Task Pending(SevenzFixtureAttempt attempt, int count, string[] before, string first, byte[] original)
    {
        await Until(() => attempt.Frames("risk_prompt").Length >= count || attempt.Events.Any(e => e.Kind == "exited"), "real 7z risk prompt deadline");
        AssertPending(attempt, count, before, first, original);
    }
    private static void AssertPending(SevenzFixtureAttempt attempt, int count, string[] before, string first, byte[] original)
    {
        Check(attempt.Frames("risk_prompt").Length == count && attempt.Runner.ActiveCount == 1 && !attempt.Events.Any(e => e.Kind == "exited"), "original worker remains AwaitingRisk");
        Check(attempt.Audit.WriteBytes == 0 && attempt.Audit.Published == 0 && attempt.Runner.InspectOutputCommit(attempt.Id).FilesCommitted == 0
            && before.SequenceEqual(Directory.GetFileSystemEntries(attempt.Output).Order()) && File.ReadAllBytes(first).SequenceEqual(original), "risk precedes every current native partial/write/publish");
        Check(SevenzFixturePackage.IsProtected(attempt.Source) && SevenzFixturePackage.IsProtected(attempt.Package.Helper.Executable), "source AND helper lease held during risk wait");
    }
    private static async Task StillPending(SevenzFixtureAttempt attempt, int prompts, int confirms, string[] before, string first, byte[] original)
    {
        await Task.Delay(100); // Input delivery was observed independently; wrong answers must not wake the member.
        AssertPending(attempt, prompts, before, first, original);
        Check(attempt.Frames("risk_confirmed").Length == confirms, "stale/wrong-generation/larger answer cannot authorize");
    }
    private static async Task Transport(SevenzFixtureAttempt attempt, ArchivePasswordTicket ticket, ulong limit)
    {
        var delivered = attempt.Audit.RiskReplies;
        Check(attempt.Runner.RespondRisk(attempt.Id, ticket, limit), "original bounded risk transport");
        await Until(() => attempt.Audit.RiskReplies > delivered || attempt.Events.Any(e => e.Kind == "exited"), "risk transport delivery deadline");
        Check(attempt.Audit.RiskReplies == delivered + 1, "answer written on original control pipe");
    }
}

// Shared by the fixture-only stdio adapter; never selects/copies a helper into the harness bin.
internal sealed class SevenzFixturePackage
{
    internal string Root { get; }
    internal string Fixtures { get; }
    internal ArchiveReadHelper Helper { get; }
    private SevenzFixturePackage(string root, string fixtures, ArchiveReadHelper helper)
    { Root = root; Fixtures = fixtures; Helper = helper; }
    internal static SevenzFixturePackage Create(string label, bool requireFixtures = true)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_EVIDENCE_ROOT");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_SEVENZ_FIXTURES");
        if (string.IsNullOrWhiteSpace(worker) || !File.Exists(worker) || string.IsNullOrWhiteSpace(evidence) || !Path.IsPathFullyQualified(evidence) || !Directory.Exists(evidence)
            || requireFixtures && (string.IsNullOrWhiteSpace(fixtures) || !File.Exists(Path.Combine(fixtures, "nonsolid.7z")) || !File.Exists(Path.Combine(fixtures, "solid.7z"))))
            throw new TestSkippedException("Explicit matching worker, stock-7z fixtures and fresh owned evidence parent required.");
        var root = Directory.CreateDirectory(Path.Combine(evidence, "archive-product-" + Guid.NewGuid())).FullName;
        var deployment = Directory.CreateDirectory(Path.Combine(root, "package")).FullName;
        ArchiveReadHelper.GuardOwnedRoot(deployment);
        var destination = Path.Combine(deployment, ArchiveReadHelper.DeploymentName);
        File.Copy(worker, destination, overwrite: false);
        var originalHash = Hash(worker); var stagedHash = Hash(destination);
        SevenzRiskReadmissionTests.Check(originalHash == stagedHash, "matching staged helper fingerprint");
        File.WriteAllText(Path.Combine(root, "attribution.json"), JsonSerializer.Serialize(new {
            scenario = label, worker_path = Path.GetFullPath(worker), worker_sha256 = originalHash,
            staged_helper_path = destination, staged_helper_sha256 = stagedHash, retained = true
        }));
        return new(root, fixtures ?? "", ArchiveReadHelper.OwnedPackage(deployment));
    }
    internal static string Hash(string path) { using var stream = File.OpenRead(path); return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant(); }
    internal static bool IsProtected(string path)
    {
        try { using var stream = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); return false; }
        catch (IOException failure) when ((failure.HResult & 0xffff) is 32 or 33) { return true; }
    }
    internal static bool IsReleased(string path)
    {
        try { using var stream = File.Open(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); return true; }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
    }
}

internal sealed class SevenzFixtureAudit : IPlatformLog
{
    internal readonly ConcurrentQueue<object> Logs = new();
    internal readonly ConcurrentQueue<string> Checkpoints = new();
    private long writeBytes;
    private int writes, published, nativeCount, riskReplies, sourceAcquired, preflightChecks, preflightEffects;
    internal long WriteBytes => Interlocked.Read(ref writeBytes);
    internal int Writes => Volatile.Read(ref writes);
    internal int Published => Volatile.Read(ref published);
    internal int NativeCount => Volatile.Read(ref nativeCount); // Actual native_result writes on original pipe, not accepted frames.
    internal int RiskReplies => Volatile.Read(ref riskReplies);
    internal bool SourceAcquired => Volatile.Read(ref sourceAcquired) != 0;
    internal int PreflightChecks => Volatile.Read(ref preflightChecks);
    internal int PreflightEffects => Volatile.Read(ref preflightEffects);
    internal bool CancelAfterCommit, CancelAccepted;
    internal void WriteLog(string eventName, Guid? id) => Logs.Enqueue(new { name = eventName, operation_id = id });
    public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
    {
        WriteLog(eventName, correlationId);
        if (eventName == "archive.worker.source_acquired") { Interlocked.Exchange(ref sourceAcquired, 1); Checkpoints.Enqueue("source_acquired"); }
    }
    internal void Install(ArchiveWorkerRunner runner, Guid id, string output)
    {
        runner.ControlInputCheckpointProbe = op => {
            if (op == "native_result") Interlocked.Increment(ref nativeCount);
            if (op == "risk_reply") Interlocked.Increment(ref riskReplies);
        };
        var decoded = false;
        runner.SourceCheckpointProbe = stage => {
            Checkpoints.Enqueue(stage);
            if (stage == "source_decoding") decoded = true;
            if (stage == "source_precommit" && !decoded) {
                Interlocked.Increment(ref preflightChecks);
                if (Directory.Exists(output)) Interlocked.Increment(ref preflightEffects);
            }
        };
        runner.OutputWriteCheckpointProbe = count => {
            SevenzRiskReadmissionTests.Check(SourceAcquired, "source validation/acquisition must precede actual native output");
            SevenzRiskReadmissionTests.Check(Checkpoints.Contains("source_validated") && Checkpoints.Contains("source_decoding"),
                "original source validation and decode checkpoints precede native output");
            Interlocked.Add(ref writeBytes, count); Interlocked.Increment(ref writes);
        };
        runner.OutputCommitCheckpointProbe = stage => {
            if (stage != "native_published") return;
            var count = Interlocked.Increment(ref published);
            if (CancelAfterCommit && count == 1) CancelAccepted = runner.Cancel(id);
        };
    }
    internal static object SafeEvent(ArchiveWorkerEvent e) => new {
        kind = e.Kind, operation_id = e.OperationId, category = e.Category, native_error_code = e.NativeErrorCode,
        elapsed_ms = e.ElapsedMs, cleanup_complete = e.CleanupComplete, files_committed = e.FilesCommitted,
        bytes_committed = e.BytesCommitted, files_skipped = e.FilesSkipped, commit_state = e.CommitState,
        frame_op = e.Frame is { } f && f.TryGetProperty("op", out var op) ? op.GetString() : null,
        error_code = e.Frame is { } failure && failure.TryGetProperty("error_code", out var error) ? error.GetString() : null,
        prompt = e.Frame is { } risk && risk.TryGetProperty("risk_prompt", out var prompt) ? (object)new {
            ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>(), ordinal = prompt.GetProperty("ordinal").GetUInt64(),
            admitted_entry_bytes = prompt.GetProperty("admitted_entry_bytes").GetUInt64(), admitted_output_bytes = prompt.GetProperty("admitted_output_bytes").GetUInt64()
        } : null
    };
    internal static object[] SafeJournal(ArchiveWorkerEvent? terminal) => terminal?.Journal?.Select(j => (object)new {
        ordinal = j.Ordinal, state = j.State, replaced = j.Replaced, bytes = j.Bytes, published = j.Published, cleanup = j.Cleanup
    }).ToArray() ?? [];
}

internal sealed class SevenzFixtureAttempt : IAsyncDisposable
{
    internal SevenzFixturePackage Package { get; }
    internal Guid Id { get; } = Guid.NewGuid();
    internal string Root { get; }
    internal string Source { get; }
    internal string Output { get; }
    internal ConcurrentQueue<ArchiveWorkerEvent> Events { get; } = new();
    internal SevenzFixtureAudit Audit { get; } = new();
    internal ArchiveWorkerRunner Runner { get; }
    internal SevenzFixtureAttempt(SevenzFixturePackage package, string label, string fixture)
    {
        Package = package; Root = Directory.CreateDirectory(Path.Combine(package.Root, label)).FullName;
        Source = Path.Combine(Root, "source.7z"); Output = Path.Combine(Root, "output");
        File.Copy(Path.Combine(package.Fixtures, fixture), Source, overwrite: false);
        // Attribution is private fixture evidence, separate from safe event/diagnostic summaries.
        File.WriteAllText(Path.Combine(Root, "source-attribution.json"), JsonSerializer.Serialize(new {
            fixture_path = Path.Combine(package.Fixtures, fixture), source_path = Source, sha256 = SevenzFixturePackage.Hash(Source)
        }));
        Runner = ArchiveWorkerRunner.ForBrowserEntry(package.Helper, Events.Enqueue, Audit);
        Audit.Install(Runner, Id, Output);
    }
    internal static object Limits(ulong entry = 8192, ulong output = 16384) => new {
        max_entries = 100UL, max_metadata_bytes = 1UL << 20, max_working_bytes = 64UL << 20,
        max_entry_bytes = entry, max_output_bytes = output
    };
    internal JsonElement[] Frames(string op) => Events.Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == op).Select(e => e.Frame!.Value).ToArray();
    internal async Task<ArchiveWorkerEvent> TerminalAsync()
    {
        await SevenzRiskReadmissionTests.Until(() => Events.Any(e => e.Kind == "exited"), "original worker terminal deadline");
        SevenzRiskReadmissionTests.Check(await Runner.DrainAsync(), "original actual drain");
        var terminal = Events.Single(e => e.Kind == "exited");
        SevenzRiskReadmissionTests.Check(terminal.CleanupComplete && Runner.ActiveCount == 0 && Events.Count(e => e.Kind == "started") == 1
            && Events.All(e => e.OperationId == Id), "one correlated original worker/Job with actual cleanup; no replacement");
        SevenzRiskReadmissionTests.Check(Audit.NativeCount >= Audit.Writes + Audit.Published && Audit.NativeCount > 0,
            "actual original native_result acknowledgements include observed writes/publications");
        SevenzRiskReadmissionTests.Check(SevenzFixturePackage.IsReleased(Source) && SevenzFixturePackage.IsReleased(Package.Helper.Executable), "exclusive source AND helper reopen after cleanup");
        return terminal;
    }
    public async ValueTask DisposeAsync()
    {
        Runner.Stop(); var drained = await Runner.DrainAsync();
        var terminal = Events.LastOrDefault(e => e.Kind == "exited");
        File.WriteAllText(Path.Combine(Root, "events.json"), JsonSerializer.Serialize(new {
            operation_id = Id, drained, active = Runner.ActiveCount, started = Events.Count(e => e.Kind == "started"),
            source_released = SevenzFixturePackage.IsReleased(Source), helper_released = SevenzFixturePackage.IsReleased(Package.Helper.Executable),
            native_count = Audit.NativeCount, native_write_count = Audit.Writes, native_write_bytes = Audit.WriteBytes,
            native_publications = Audit.Published, journal = SevenzFixtureAudit.SafeJournal(terminal),
            checkpoints = Audit.Checkpoints.ToArray(), events = Events.Select(SevenzFixtureAudit.SafeEvent).ToArray(), diagnostics = Audit.Logs.ToArray()
        }));
        Runner.Dispose();
        SevenzRiskReadmissionTests.Check(drained, "original runner cleanup incomplete; retained all evidence");
    }
}
