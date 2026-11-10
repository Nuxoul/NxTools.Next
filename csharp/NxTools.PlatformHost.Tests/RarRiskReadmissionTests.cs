using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Independently attributed upstream RAR inputs only; never generate RAR or erase evidence.
// Deliberately unregistered: the owning lane must select the exact methods and matching worker.
internal static class RarRiskReadmissionTests
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

    private static readonly RarFixtureSpec[] Plain = [
        new("rar4.rar", "Rar4", "VERSION", "unrar-0.4.0", "2c595ee9aaea2c837bd0e11a659b9b62b46251f18ab7b6675748da0fadd34460"),
        new("rar5.rar", "Rar5", "unicodefilename❤️.txt", "foobar\n", "404c872a59c60f5040586065a4f0b30478139a405284154479cc9b57f7bf2815")
    ];
    private static readonly RarFixtureSpec[] Encrypted = [
        new("rar4-encrypted.rar", "Rar4", ".gitignore", "target\nCargo.lock\n", "3215288892bfae2e641786952ab2547ea72cdd58c9c544604cd63e242fa07318"),
        new("rar5-header-encrypted.rar", "Rar5", ".gitignore", "target\nCargo.lock\n", "dd47390a7fc73391183b38a4c140b595fdd52ab22afbdfa4e41db5023fc55f1b")
    ];

    public static async Task ActualRarOrdinaryOpenAndTest()
    {
        var package = RarFixturePackage.Create("ordinary_open_test");
        foreach (var spec in Plain) {
            foreach (var operation in new[] { "open", "test" }) {
                await using var attempt = new RarFixtureAttempt(package, spec.Format + "-" + operation, spec);
                // No DEBUG Risk fields or lowered thresholds: real ordinary open/TEST only.
                Check(attempt.Runner.Start(new(attempt.Id, operation, attempt.Source, 936, RarFixtureAttempt.Limits())), "original RAR read admission");
                var terminal = await attempt.TerminalAsync();
                Check(terminal.Category == "success" && terminal.FilesCommitted == 0 && terminal.BytesCommitted == 0
                    && attempt.Audit.WriteBytes == 0 && attempt.Audit.Published == 0 && (terminal.Journal?.Count ?? 0) == 0,
                    "open/TEST have no native output authority");
                NoRisk(attempt);
                Check(!Directory.Exists(attempt.Output), "read does not create extraction root");
                if (operation == "open") {
                    var batches = attempt.Frames("entry_batch");
                    Check(batches.Length > 0 && batches.All(f => f.GetProperty("format").GetString() == spec.Format), "actual protocol Rar4/Rar5 metadata attribution");
                    var entries = batches.SelectMany(f => f.GetProperty("entries").EnumerateArray()).ToArray();
                    Check(entries.Length == 1, "exact independent one-member RAR metadata");
                    var entry = entries.Single();
                    Check(entry.GetProperty("name").GetString() == spec.Name && entry.GetProperty("size").GetUInt64() == spec.Size,
                        "literal upstream RAR member name and declared bytes");
                    var packed = entry.GetProperty("packed_size");
                    // RAR4 FILE_HEAD PackSize=21; RAR5 file block data_size=7 in these exact inputs.
                    Check(packed.ValueKind == JsonValueKind.Number && packed.GetUInt64() == (spec.Format == "Rar4" ? 21UL : 7UL),
                        "exact independent nonsolid/nonsplit packed bytes, never total archive size or a guessed ratio");
                    File.WriteAllText(Path.Combine(attempt.Root, "metadata-observation.json"), JsonSerializer.Serialize(new {
                        format = spec.Format, fixture_sha256 = spec.Sha256, size = spec.Size, packed_size = packed.GetUInt64()
                    }));
                } else {
                    var complete = attempt.Frames("complete").Single();
                    Check(complete.GetProperty("files_verified").GetUInt64() == 1 && complete.GetProperty("bytes_verified").GetUInt64() == spec.Size,
                        "original no-output RAR_TEST fully verifies the literal payload length");
                }
            }
        }
    }

    public static async Task ActualRarLowTotalPreflight()
    {
        var package = RarFixturePackage.Create("low_total_preflight");
        var failures = new List<string>();
        // Collect BOTH actual native results even when the first regression is still red.
        foreach (var spec in Plain) {
            try {
                await using var attempt = new RarFixtureAttempt(package, spec.Format + "-total1", spec);
                var binding = JsonSerializer.SerializeToElement(new { logical_id = attempt.Id, expected_source = (object?)null });
                Check(!Directory.Exists(attempt.Output), "fresh output root absent before native admission");
                Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, RarFixtureAttempt.Limits(1024, 1),
                    OutputRoot: attempt.Output, OutputReadmission: binding, ExtractOptions: new(false))), "original RAR low-total admission");
                var terminal = await attempt.TerminalAsync();
                var shortages = attempt.Frames("output_budget_shortfall");
                Check(shortages.Length == 1, "total1 must reach RAR declaration, not reject encoded source");
                var shortfall = shortages.Single().GetProperty("output_budget_shortfall");
                Check(shortfall.GetProperty("current").GetProperty("entry_bytes").GetUInt64() == 1024
                    && shortfall.GetProperty("current").GetProperty("output_bytes").GetUInt64() == 1
                    && shortfall.GetProperty("required").GetProperty("entry_bytes").GetUInt64() == spec.Size
                    && shortfall.GetProperty("required").GetProperty("output_bytes").GetUInt64() == spec.Size,
                    "exact unchanged admission and independent 11/7-byte original declaration");
                var source = shortfall.GetProperty("source");
                using (var stream = File.OpenRead(attempt.Source))
                    Check(source.GetProperty("digest").EnumerateArray().Select(b => b.GetByte()).SequenceEqual(SHA256.HashData(stream)), "shortfall carries actual original Rust source digest");
                using (var lease = ArchiveSourceLease.Open(attempt.Source))
                    Check(source.GetProperty("native").GetString() == lease.ReadmissionIdentity(), "shortfall binds exact original native source identity/version");
                Check(terminal.Category == "resource_limit" && terminal.FilesCommitted == 0 && terminal.BytesCommitted == 0
                    && (terminal.Journal?.Count ?? 0) == 0 && attempt.Audit.Writes == 0 && attempt.Audit.Published == 0
                    && !Directory.Exists(attempt.Output), "declaration shortfall has zero native output/root effects");
                Check(attempt.Frames("failure").Single().GetProperty("error_code").GetString() == "ResourceLimit", "original finite declaration failure category");
                Check(attempt.Audit.SourceAcquired && attempt.Audit.Checkpoints.Contains("source_validated")
                    && attempt.Audit.Checkpoints.Contains("source_precommit") && !attempt.Audit.Checkpoints.Contains("source_decoding"),
                    "validated original source reaches preflight without payload decode");
                NoRisk(attempt);
            }
            catch (Exception failure) {
                var outcome = spec.Format + ":" + failure.GetType().Name;
                failures.Add(outcome);
                File.WriteAllText(Path.Combine(package.Root, spec.Format + "-failure.json"), JsonSerializer.Serialize(new {
                    format = spec.Format, exception_type = failure.GetType().Name,
                    detail = failure is InvalidOperationException ? failure.Message : null, retained = true
                }));
            }
        }
        File.WriteAllText(Path.Combine(package.Root, "low-total-results.json"), JsonSerializer.Serialize(new {
            scenarios = Plain.Select(s => s.Format).ToArray(), failures, retained = true
        }));
        Check(failures.Count == 0, "RAR4/RAR5 low-total native failures (both collected): " + string.Join(", ", failures));
    }

    public static async Task ActualRarConflictSkipAndOverwrite()
    {
        var package = RarFixturePackage.Create("conflict_skip_overwrite");
        foreach (var spec in Plain) {
            foreach (var policy in new[] { "skip", "overwrite" }) {
                await using var attempt = new RarFixtureAttempt(package, spec.Format + "-" + policy, spec);
                Directory.CreateDirectory(attempt.Output);
                var target = Path.Combine(attempt.Output, spec.Name);
                var original = new byte[] { 91, 92, 93, 94 };
                File.WriteAllBytes(target, original);
                var foreign = Path.Combine(attempt.Output, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign");
                Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, RarFixtureAttempt.Limits(),
                    OutputRoot: attempt.Output, ConflictPolicy: policy, ExtractOptions: new(false))), "original RAR conflict admission");
                var terminal = await attempt.TerminalAsync();
                NoRisk(attempt);
                if (policy == "skip") {
                    Check(terminal.Category == "succeeded_with_warnings" && terminal.FilesSkipped == 1 && terminal.FilesCommitted == 0
                        && terminal.BytesCommitted == 0 && attempt.Audit.Writes == 0 && attempt.Audit.Published == 0,
                        "original skip has no replacement/write/publication");
                    Check(File.ReadAllBytes(target).SequenceEqual(original), "skipped user target unchanged");
                    AssertPublishedJournal(terminal, 0, 0);
                } else {
                    Check(terminal.Category == "success" && terminal.FilesSkipped == 0 && terminal.FilesCommitted == 1
                        && terminal.BytesCommitted == spec.Size && attempt.Audit.WriteBytes == (long)spec.Size && attempt.Audit.Published == 1,
                        "one original native replacement with exact literal output bytes");
                    AssertBytes(attempt, spec);
                    AssertPublishedJournal(terminal, 1, spec.Size);
                    Check(terminal.Journal!.Single(j => j.Published == true).Replaced == true, "native journal attributes real replacement");
                }
                Check(File.ReadAllText(foreign) == "foreign" && Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 1,
                    "only operation-owned partials cleaned; foreign file retained");
                Check(attempt.Frames("output_budget_shortfall").Length == 0, "ordinary conflict never finances extraction readmission");
            }
        }
    }

    public static async Task ActualRarPasswordPreparation()
    {
        var package = RarFixturePackage.Create("password_preparation");
        foreach (var spec in Encrypted) {
            await using var attempt = new RarFixtureAttempt(package, spec.Format + "-password", spec);
            Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, RarFixtureAttempt.Limits(),
                OutputRoot: attempt.Output, PasswordInteraction: true, ConflictPolicy: "overwrite", ExtractOptions: new(false))), "original encrypted RAR admission");
            await AwaitPassword(attempt, 1, "PasswordRequired");
            var first = PasswordStates(attempt, "awaiting_password").Single().GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
            Check(first.OperationId == attempt.Id && first.PromptId != Guid.Empty && first.Generation > 0, "original correlated password ticket");
            // Only RespondPassword transports answers. All finite TEST/retry/extract policy stays in Rust.
            if (spec.Format == "Rar5") {
                Check(attempt.Runner.RespondPassword(attempt.Id, first, "incorrect-rar-fixture-secret"), "original wrong password transport");
                await AwaitPassword(attempt, 2, "WrongPassword");
                var current = PasswordStates(attempt, "awaiting_password").Last().GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                Check(current != first && current.OperationId == attempt.Id, "explicit BAD_PASSWORD gets fresh original ticket");
                Check(attempt.Runner.RespondPassword(attempt.Id, current, "password"), "original corrected password transport");
            } else Check(attempt.Runner.RespondPassword(attempt.Id, first, "unrar"), "original required password transport");
            var terminal = await attempt.TerminalAsync();
            Check(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.BytesCommitted == 18
                && attempt.Audit.WriteBytes == 18 && attempt.Audit.Published == 1, "one original encrypted native extraction");
            var preparations = PasswordStates(attempt, "running");
            Check(preparations.Length == (spec.Format == "Rar5" ? 3 : 2)
                && preparations.Select(p => p.GetProperty("attempt_id").GetGuid()).Distinct().Count() == preparations.Length
                && preparations.All(p => p.GetProperty("attempt_id").GetGuid() != attempt.Id), "finite original no-output TEST preparations are distinct from extraction");
            Check(PasswordStates(attempt, "extracting").Length == 1 && attempt.Frames("output_budget_shortfall").Length == 0,
                "successful preparation starts exactly one extraction; no C# retry/readmission loop");
            AssertBytes(attempt, spec); AssertPublishedJournal(terminal, 1, 18); NoRisk(attempt);
            var safe = JsonSerializer.Serialize(attempt.Events.Select(RarFixtureAudit.SafeEvent)) + JsonSerializer.Serialize(attempt.Audit.Logs.ToArray());
            Check(!safe.Contains("incorrect-rar-fixture-secret", StringComparison.Ordinal) && !safe.Contains("\"password\":", StringComparison.Ordinal), "secret-free retained event/diagnostic summaries");
        }
    }

    public static async Task ActualRarPostpublicationCancellation()
    {
        var package = RarFixturePackage.Create("postpublication_cancel");
        var spec = Plain[0];
        await using var attempt = new RarFixtureAttempt(package, "Rar4-cancel-after-commit", spec);
        attempt.Audit.CancelAfterCommit = true;
        Check(attempt.Runner.Start(new(attempt.Id, "extract", attempt.Source, 936, RarFixtureAttempt.Limits(),
            OutputRoot: attempt.Output, ConflictPolicy: "overwrite", ExtractOptions: new(false))), "original postpublication cancel admission");
        var terminal = await attempt.TerminalAsync();
        Check(attempt.Audit.CancelAccepted && terminal.Category == "cancelled_with_committed_output" && terminal.FilesCommitted == 1
            && terminal.BytesCommitted == spec.Size && attempt.Audit.Published == 1 && attempt.Audit.WriteBytes == (long)spec.Size,
            "actual postpublication cancellation retains original native truth, never a second extraction");
        AssertBytes(attempt, spec); AssertPublishedJournal(terminal, 1, spec.Size); NoRisk(attempt);
        Check(attempt.Frames("output_budget_shortfall").Length == 0 && Directory.GetFiles(attempt.Output, ".nxpart-*", SearchOption.AllDirectories).Length == 0
            && File.Exists(attempt.Source), "no upgrade/rerun/rollback or source deletion after publication");
    }

    private static void NoRisk(RarFixtureAttempt attempt) => Check(attempt.Frames("risk_prompt").Length == 0
        && attempt.Frames("risk_confirmed").Length == 0, "tiny ordinary RAR cannot fabricate or approve an extraction risk ratio");
    private static void AssertBytes(RarFixtureAttempt attempt, RarFixtureSpec spec) => Check(
        File.ReadAllBytes(Path.Combine(attempt.Output, spec.Name)).SequenceEqual(Encoding.UTF8.GetBytes(spec.Literal)), "independent literal RAR payload bytes");
    private static void AssertPublishedJournal(ArchiveWorkerEvent terminal, int count, ulong bytes)
    {
        var published = terminal.Journal?.Where(j => j.Published == true).ToArray() ?? [];
        Check(published.Length == count && published.Aggregate(0UL, (n, j) => n + j.Bytes) == bytes
            && published.All(j => j.State == "Committed" && j.Cleanup is ("retained" or "retained_replacement")), "original native journal matches retained bytes/publications");
    }
    private static JsonElement[] PasswordStates(RarFixtureAttempt attempt, string state) => attempt.Frames("password_state")
        .Select(f => f.GetProperty("password_state")).Where(s => s.GetProperty("state").GetString() == state).ToArray();
    private static async Task AwaitPassword(RarFixtureAttempt attempt, int count, string code)
    {
        await Until(() => PasswordStates(attempt, "awaiting_password").Length >= count || attempt.Events.Any(e => e.Kind == "exited"), "original RAR password deadline");
        var waiting = PasswordStates(attempt, "awaiting_password");
        Check(waiting.Length == count && waiting.Last().GetProperty("error_code").GetString() == code
            && attempt.Runner.ActiveCount == 1 && !attempt.Events.Any(e => e.Kind == "exited"), "original Required/WrongPassword wait owns the live Job");
        Check(attempt.Audit.Writes == 0 && attempt.Audit.Published == 0 && PasswordStates(attempt, "extracting").Length == 0
            && (!Directory.Exists(attempt.Output) || !Directory.EnumerateFileSystemEntries(attempt.Output).Any()), "full no-output TEST preparation precedes any extraction effects");
        Check(RarFixturePackage.IsProtected(attempt.Source) && RarFixturePackage.IsProtected(attempt.Package.Helper.Executable), "password wait retains original source/helper leases");
    }
}

internal sealed record RarFixtureSpec(string FileName, string Format, string Name, string Literal, string Sha256)
{ internal ulong Size => (ulong)Encoding.UTF8.GetByteCount(Literal); }

internal sealed class RarFixturePackage
{
    internal string Root { get; }
    internal string Fixtures { get; }
    internal ArchiveReadHelper Helper { get; }
    private RarFixturePackage(string root, string fixtures, ArchiveReadHelper helper)
    { Root = root; Fixtures = fixtures; Helper = helper; }
    internal static RarFixturePackage Create(string label, bool requireFixtures = true)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var evidence = Environment.GetEnvironmentVariable("NXTOOLS_RAR_EVIDENCE_ROOT");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_RAR_FIXTURES");
        if (string.IsNullOrWhiteSpace(worker) || !Path.IsPathFullyQualified(worker) || !File.Exists(worker)
            || string.IsNullOrWhiteSpace(evidence) || !Path.IsPathFullyQualified(evidence) || !Directory.Exists(evidence)
            || requireFixtures && (string.IsNullOrWhiteSpace(fixtures) || !Path.IsPathFullyQualified(fixtures) || !Directory.Exists(fixtures)))
            throw new TestSkippedException("Explicit matching isolated-codecs worker, attributed RAR fixtures and owned evidence parent required.");
        var root = Directory.CreateDirectory(Path.Combine(evidence, "archive-product-" + Guid.NewGuid())).FullName;
        var deployment = Directory.CreateDirectory(Path.Combine(root, "package")).FullName;
        ArchiveReadHelper.GuardOwnedRoot(deployment);
        var destination = Path.Combine(deployment, ArchiveReadHelper.DeploymentName);
        File.Copy(worker, destination, overwrite: false);
        var originalHash = Hash(worker); var stagedHash = Hash(destination);
        RarRiskReadmissionTests.Check(originalHash == stagedHash, "matching owned helper fingerprint");
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
    internal static bool IsWithin(string path, string root) => Path.IsPathFullyQualified(path) && Path.IsPathFullyQualified(root)
        && Path.GetFullPath(path).StartsWith(Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
    internal static string SignatureFormat(Stream stream)
    {
        var signature = new byte[8]; stream.ReadExactly(signature);
        if (signature.AsSpan(0, 7).SequenceEqual(new byte[] { 0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x00 })) return "Rar4";
        if (signature.AsSpan().SequenceEqual(new byte[] { 0x52, 0x61, 0x72, 0x21, 0x1a, 0x07, 0x01, 0x00 })) return "Rar5";
        throw new InvalidDataException("An actual RAR4/RAR5 signature is required.");
    }
}

internal sealed class RarFixtureAudit : IPlatformLog
{
    internal readonly ConcurrentQueue<object> Logs = new();
    internal readonly ConcurrentQueue<string> Checkpoints = new();
    private long writeBytes;
    private int writes, published, nativeCount, sourceAcquired, preflightChecks, preflightEffects;
    internal long WriteBytes => Interlocked.Read(ref writeBytes);
    internal int Writes => Volatile.Read(ref writes);
    internal int Published => Volatile.Read(ref published);
    internal int NativeCount => Volatile.Read(ref nativeCount);
    internal bool SourceAcquired => Volatile.Read(ref sourceAcquired) != 0;
    internal int PreflightChecks => Volatile.Read(ref preflightChecks);
    internal int PreflightEffects => Volatile.Read(ref preflightEffects);
    internal bool CancelAfterCommit, CancelAccepted;
    public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
    {
        Logs.Enqueue(new { name = eventName, operation_id = correlationId });
        if (eventName == "archive.worker.source_acquired") { Interlocked.Exchange(ref sourceAcquired, 1); Checkpoints.Enqueue("source_acquired"); }
    }
    internal void Install(ArchiveWorkerRunner runner, Guid id, string output)
    {
        runner.ControlInputCheckpointProbe = op => { if (op == "native_result") Interlocked.Increment(ref nativeCount); };
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
            RarRiskReadmissionTests.Check(SourceAcquired && Checkpoints.Contains("source_validated") && Checkpoints.Contains("source_decoding"),
                "original RAR acquisition/validation/decoding precede every actual native write");
            Interlocked.Add(ref writeBytes, count); Interlocked.Increment(ref writes);
        };
        runner.OutputCommitCheckpointProbe = stage => {
            if (stage != "native_closed") return;
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
        password_state = e.Frame is { } state && state.TryGetProperty("password_state", out var password) ? password.Clone() : (JsonElement?)null,
        shortfall = e.Frame is { } shortage && shortage.TryGetProperty("output_budget_shortfall", out var shortfall) ? shortfall.Clone() : (JsonElement?)null
    };
    internal static object[] SafeJournal(ArchiveWorkerEvent? terminal) => terminal?.Journal?.Select(j => (object)new {
        ordinal = j.Ordinal, state = j.State, replaced = j.Replaced, bytes = j.Bytes, published = j.Published, cleanup = j.Cleanup
    }).ToArray() ?? [];
}

internal sealed class RarFixtureAttempt : IAsyncDisposable
{
    internal RarFixturePackage Package { get; }
    internal Guid Id { get; } = Guid.NewGuid();
    internal string Root { get; }
    internal string Source { get; }
    internal string Output { get; }
    internal ConcurrentQueue<ArchiveWorkerEvent> Events { get; } = new();
    internal RarFixtureAudit Audit { get; } = new();
    internal ArchiveWorkerRunner Runner { get; }
    internal RarFixtureAttempt(RarFixturePackage package, string label, RarFixtureSpec fixture)
    {
        Package = package; Root = Directory.CreateDirectory(Path.Combine(package.Root, label)).FullName;
        Source = Path.Combine(Root, "source.rar"); Output = Path.Combine(Root, "output");
        var original = Path.Combine(package.Fixtures, fixture.FileName);
        RarRiskReadmissionTests.Check(File.Exists(original), "required attributed RAR fixture is missing: " + fixture.FileName);
        File.Copy(original, Source, overwrite: false);
        var hash = RarFixturePackage.Hash(Source);
        File.WriteAllText(Path.Combine(Root, "source-attribution.json"), JsonSerializer.Serialize(new {
            fixture_path = original, source_path = Source, sha256 = hash, expected_sha256 = fixture.Sha256, format = fixture.Format
        }));
        RarRiskReadmissionTests.Check(hash == fixture.Sha256, "exact upstream manifest RAR fingerprint");
        using (var stream = File.OpenRead(Source)) RarRiskReadmissionTests.Check(RarFixturePackage.SignatureFormat(stream) == fixture.Format, "actual RAR signature matches fixture attribution");
        Runner = ArchiveWorkerRunner.ForProductRead(package.Helper, Events.Enqueue, (id, frame, cancellation) => {
            Events.Enqueue(new("frame", id, Frame: frame.Clone()));
            return Task.FromResult<string?>(null);
        }, Audit);
        Audit.Install(Runner, Id, Output);
    }
    internal static object Limits(ulong entry = 1024, ulong output = 4096) => new {
        max_entries = 100UL, max_metadata_bytes = 1UL << 20, max_working_bytes = 64UL << 20,
        max_entry_bytes = entry, max_output_bytes = output
    };
    internal JsonElement[] Frames(string op) => Events.Where(e => e.Frame is { } f && f.GetProperty("op").GetString() == op).Select(e => e.Frame!.Value).ToArray();
    internal async Task<ArchiveWorkerEvent> TerminalAsync()
    {
        await RarRiskReadmissionTests.Until(() => Events.Any(e => e.Kind == "exited"), "actual original RAR exited deadline");
        RarRiskReadmissionTests.Check(await Runner.DrainAsync(), "actual original RAR drain");
        var terminal = Events.Single(e => e.Kind == "exited");
        RarRiskReadmissionTests.Check(terminal.CleanupComplete && Runner.ActiveCount == 0 && Events.Count(e => e.Kind == "started") == 1
            && Events.All(e => e.OperationId == Id), "one correlated original runner/Job and actual cleanup, no replacement");
        RarRiskReadmissionTests.Check(Audit.NativeCount >= Audit.Writes + Audit.Published && Audit.NativeCount > 0, "actual original native_result pipe acknowledgements observed");
        RarRiskReadmissionTests.Check(RarFixturePackage.IsReleased(Source) && RarFixturePackage.IsReleased(Package.Helper.Executable), "exclusive source AND helper reopen after actual cleanup");
        return terminal;
    }
    public async ValueTask DisposeAsync()
    {
        Runner.Stop(); var drained = await Runner.DrainAsync();
        try {
            var terminal = Events.LastOrDefault(e => e.Kind == "exited");
            File.WriteAllText(Path.Combine(Root, "events.json"), JsonSerializer.Serialize(new {
                operation_id = Id, drained, active = Runner.ActiveCount, started = Events.Count(e => e.Kind == "started"),
                source_released = RarFixturePackage.IsReleased(Source), helper_released = RarFixturePackage.IsReleased(Package.Helper.Executable),
                source_acquired = Audit.SourceAcquired, native_count = Audit.NativeCount, native_write_count = Audit.Writes,
                native_write_bytes = Audit.WriteBytes, native_publications = Audit.Published, journal = RarFixtureAudit.SafeJournal(terminal),
                checkpoints = Audit.Checkpoints.ToArray(), events = Events.Select(RarFixtureAudit.SafeEvent).ToArray(), diagnostics = Audit.Logs.ToArray()
            }));
        }
        finally { Runner.Dispose(); }
        RarRiskReadmissionTests.Check(drained, "original runner cleanup incomplete; all RAR evidence retained");
    }
}
