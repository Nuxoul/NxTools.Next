using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only tiny GUID-owned scratch trees, the explicit actual worker and existing Python oracle.
// Inputs/outputs are retained, never recycled/deleted. Cleanup removes only our copied helper.
internal static class ArchiveLongPathTests
{
    private static readonly byte[] Payload = Encoding.UTF8.GetBytes("long-path independent content 合成\n");
    private static readonly byte[] OldTarget = "previous target must survive before publication"u8.ToArray();
    private static readonly DateTime Stamp = new(2020, 2, 3, 4, 5, 6, DateTimeKind.Unspecified);
    private static readonly string MemberDirectory = string.Join('/', Enumerable.Range(0, 7).Select(i => "成员目录" + i + new string('m', 42)));
    private static readonly string Member = MemberDirectory + "/内容.bin";
    private static readonly string[] CandidateModes = ["new", "overwrite", "unapproved", "target_changed", "cancel_before", "native_denied", "cancel_during", "postcheck"];
    private static object Limits => new { max_entries = 100, max_metadata_bytes = 16777216, max_working_bytes = 67108864, max_entry_bytes = 1048576, max_output_bytes = 4194304 };
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static string LongDirectory(string root, string kind)
    {
        var path = Path.Combine(root, kind);
        for (var i = 0; path.Length < 420; i++) path = Path.Combine(path, "长路径目录" + i + new string('a', 40));
        Directory.CreateDirectory(path);
        Require(path.Length > 400 && path[1..3] == ":\\" && !path.StartsWith(@"\\?\", StringComparison.Ordinal), "ordinary local long path required");
        Require(path[3..].Split('\\').All(part => part.Length <= 255), "legal component length required");
        return path;
    }
    private static async Task WithWorker(Func<string, List<object>, Task> body)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        Require(!string.IsNullOrEmpty(worker) && File.Exists(worker), "explicit actual Rust worker required; not a skipped acceptance");
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR");
        Require(!string.IsNullOrEmpty(scratch) && Directory.Exists(scratch), "explicit session scratch required");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var root = Directory.CreateDirectory(Path.Combine(scratch!, "archive-long-path-" + Guid.NewGuid())).FullName;
        File.WriteAllText(Path.Combine(root, "owned-long-path-fixture"), "v1\n");
        var expectedHelper = SHA256.HashData(File.ReadAllBytes(worker!));
        var evidence = new List<object>(); var copied = false;
        try {
            File.Copy(worker!, helper, false); copied = true;
            await body(root, evidence);
            using var output = new FileStream(Path.Combine(root, "evidence.json"), FileMode.CreateNew, FileAccess.Write);
            JsonSerializer.Serialize(output, new { schema = 1, status = "passed", inputs_retained = true, records = evidence }, new JsonSerializerOptions { WriteIndented = true });
        }
        catch (Exception failure) {
            using var output = new FileStream(Path.Combine(root, "evidence.json"), FileMode.CreateNew, FileAccess.Write);
            JsonSerializer.Serialize(output, new { schema = 1, status = "failed", failure_type = failure.GetType().Name, inputs_retained = true, records = evidence }, new JsonSerializerOptions { WriteIndented = true });
            throw;
        }
        finally {
            if (copied) {
                Require(SHA256.HashData(File.ReadAllBytes(helper)).SequenceEqual(expectedHelper), "copied helper changed; retain rather than delete unknown image");
                File.Delete(helper); // Only this exact create-new copy, after runner disposal/drain.
            }
        }
    }
    private static void Zip(string path, IReadOnlyDictionary<string, byte[]> entries)
    {
        using var file = new FileStream(path, FileMode.CreateNew, FileAccess.Write);
        using var archive = new ZipArchive(file, ZipArchiveMode.Create);
        foreach (var pair in entries) {
            var entry = archive.CreateEntry(pair.Key, CompressionLevel.NoCompression); entry.LastWriteTime = new DateTimeOffset(Stamp);
            entry.ExternalAttributes = 0;
            using var stream = entry.Open(); stream.Write(pair.Value);
        }
    }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string name, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Records.Enqueue((name, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
    }
    private static async Task<ArchiveWorkerEvent> Run(ArchiveWorkerRequest request, List<object> evidence, string mode,
        int memberLength, Action<ArchiveWorkerRunner>? setup = null,
        Func<ArchiveWorkerRunner, ConcurrentQueue<ArchiveWorkerEvent>, Task>? afterStart = null)
    {
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log); setup?.Invoke(runner);
        Require(runner.Start(request), "long-path real runner admission");
        if (afterStart is not null) await afterStart(runner, events);
        var clock = Stopwatch.StartNew();
        while (!events.Any(item => item.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 20000, "long-path operation deadline"); await Task.Delay(10); }
        Require(await runner.DrainAsync() && runner.ActiveCount == 0, "actual worker/pumps/Job/slot cleanup");
        var terminal = events.Single(item => item.Kind == "exited");
        Require(events.Any(item => item.Kind == "started") && terminal.CleanupComplete, "actual child started and cleanup confirmed");
        var completed = log.Records.Single(item => item.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled");
        Require(completed.Id == request.OperationId && Equals(completed.Fields["category"], terminal.Category) &&
            Equals(completed.Fields["native_error_code"], terminal.NativeErrorCode) && Equals(completed.Fields["files_committed"], terminal.FilesCommitted) &&
            Equals(completed.Fields["commit_state"], terminal.CommitState) && Equals(completed.Fields["elapsed_ms"], terminal.ElapsedMs), "terminal/log facts differ");
        var diagnostic = JsonSerializer.Serialize(log.Records.Select(item => new { item.Name, item.Id, item.Fields }));
        Require(!diagnostic.Contains(request.Path, StringComparison.Ordinal) && !diagnostic.Contains("成员目录", StringComparison.Ordinal) &&
            !diagnostic.Contains("内容.bin", StringComparison.Ordinal) && !diagnostic.Contains("independent content", StringComparison.Ordinal) && log.Records.Count < 50, "path/member/content diagnostic leak or high-frequency logging");
        var textFields = log.Records.SelectMany(item => item.Fields.Values).OfType<string>();
        Require(textFields.All(value => !value.Contains(request.Path, StringComparison.Ordinal) && !value.Contains("成员目录", StringComparison.Ordinal) &&
            !value.Contains("内容.bin", StringComparison.Ordinal) && !value.Contains("independent content", StringComparison.Ordinal)), "unescaped diagnostic payload leak");
        if (mode.StartsWith("unsafe-", StringComparison.Ordinal))
            Require(events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure").Frame!.Value.GetProperty("error_code").GetString() == "UnsafePath", "dangerous member precise rejection");
        if (mode == "direct_unapproved")
            Require(events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "failure").Frame!.Value.GetProperty("error_code").GetString() == "OutputCollision", "existing target without a Rust conflict policy must be refused");
        evidence.Add(new { operation_id = request.OperationId, operation = request.Operation, mode,
            source_utf16 = request.Path.Length, source_utf8 = Encoding.UTF8.GetByteCount(request.Path),
            output_root_utf16 = request.OutputRoot?.Length, member_utf16 = memberLength,
            terminal.Category, terminal.NativeErrorCode, terminal.FilesCommitted, terminal.CommitState, terminal.CleanupComplete,
            terminal.ElapsedMs, terminal.ExitCode, log_records = log.Records.Count });
        return terminal;
    }
    private static void NoOwnedPartial(string root, string? external = null)
    {
        var observed = Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories);
        Require(observed.Length == (external is null ? 0 : 1) && (external is null || observed.Single() == external && File.ReadAllText(external) == "external partial"), "owned partial cleanup altered/leaked files");
    }
    private static void ExactLayout(string root, IReadOnlyDictionary<string, byte[]> expected)
    {
        var directories = new HashSet<string>(StringComparer.Ordinal);
        foreach (var name in expected.Keys) {
            var parts = name.TrimEnd('/').Split('/');
            for (var i = 1; i <= parts.Length - (name.EndsWith('/') ? 0 : 1); i++) directories.Add(string.Join('/', parts.Take(i)));
        }
        var files = expected.Where(pair => !pair.Key.EndsWith('/')).ToArray();
        Require(Directory.GetFiles(root, "*", SearchOption.AllDirectories).Select(path => Path.GetRelativePath(root, path).Replace('\\', '/')).Order(StringComparer.Ordinal)
            .SequenceEqual(files.Select(pair => pair.Key).Order(StringComparer.Ordinal)), "independent exact extracted files");
        Require(Directory.GetDirectories(root, "*", SearchOption.AllDirectories).Select(path => Path.GetRelativePath(root, path).Replace('\\', '/')).Order(StringComparer.Ordinal)
            .SequenceEqual(directories.Order(StringComparer.Ordinal)), "independent exact extracted directories");
        foreach (var pair in files) Require(File.ReadAllBytes(Path.Combine(root, pair.Key.Replace('/', '\\'))).SequenceEqual(pair.Value), "independent extracted bytes");
    }
    public static Task ExtractionLayout() => WithWorker(async (owned, evidence) => {
        var source = Path.Combine(LongDirectory(owned, "archive"), "源归档.zip");
        var output = LongDirectory(owned, "extract");
        var expected = new Dictionary<string, byte[]> { [Member] = Payload, [MemberDirectory + "/空文件.bin"] = [], [MemberDirectory + "/空目录/"] = [] };
        Zip(source, expected); var before = SHA256.HashData(File.ReadAllBytes(source));
        var terminal = await Run(new(Guid.NewGuid(), "extract", source, 0, Limits, OutputRoot: output), evidence, "layout", Member.Length);
        Require(terminal.Category == "success" && terminal.FilesCommitted == 2 && terminal.CommitState == "Committed", "long-path layout publication");
        ExactLayout(output, expected); NoOwnedPartial(output);
        var final = Path.Combine(output, Member.Replace('/', '\\'));
        Require(final.Length > 700 && File.GetLastWriteTime(final) == Stamp, "long final path and independent DOS timestamp");
        Require(SHA256.HashData(File.ReadAllBytes(source)).SequenceEqual(before), "source archive altered");
        using var released = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        evidence.Add(new { final_utf16 = final.Length, final_utf8 = Encoding.UTF8.GetByteCount(final), member_utf16 = Member.Length, entries = expected.Count, exact_structure = true, source_released = true });
    });
    private static IReadOnlyDictionary<string, string>? Confirmation(string root, string relative, string mode)
    {
        if (mode is "new" or "unapproved") return null;
        using var session = ArchiveOutputSession.Open(root, Guid.NewGuid());
        return new Dictionary<string, string> { [relative] = session.CaptureTargetIdentity(relative) };
    }
    private static Action<ArchiveWorkerRunner> PublicationProbe(Guid id, string mode, Action reached, Action published, string final) => runner => {
        runner.OutputCommitCheckpointProbe = stage => {
            if (stage == "before_recheck" && mode == "target_changed") { reached(); File.WriteAllBytes(final, "external changed target"u8.ToArray()); }
            if (stage == "target_rechecked" && mode == "cancel_before") { reached(); Require(runner.Cancel(id), "original precommit cancellation"); }
            if (stage == "before_flush" && mode == "native_denied") { reached(); throw new ArchiveOutputException("flush_file", 5); }
            if (stage == "commit_in_flight" && mode == "cancel_during") { reached(); Require(runner.Cancel(id), "original in-flight cancellation"); }
            if (stage == "native_published") { Require(runner.InspectOutputCommit(id).FilesCommitted == 1, "native success publication count recorded immediately"); published(); }
            if (stage == "native_committed" && mode == "postcheck") { reached(); throw new ArchiveOutputException("inspect_identity", 5); }
        };
    };
    private static bool Published(string mode) => mode is "new" or "overwrite" or "cancel_during" or "postcheck";
    private static void PublicationResult(ArchiveWorkerEvent terminal, string mode, int reached, int published, string final)
    {
        var category = mode switch { "new" or "overwrite" => "success", "unapproved" => "worker_failure", "target_changed" => "target_changed",
            "cancel_before" => "cancelled", "native_denied" => "flush_file", "cancel_during" => "cancelled_with_committed_output", _ => "committed_but_verification_failed" };
        Require(terminal.Category == category && terminal.FilesCommitted == (Published(mode) ? 1 : 0) && published == (Published(mode) ? 1 : 0), "long-path truthful publication: " + mode + "/" + terminal.Category);
        if (mode is not ("new" or "overwrite" or "unapproved")) Require(reached == 1, "deterministic original boundary not reached");
        if (Published(mode)) Require(terminal.CommitState == (mode == "postcheck" ? "CommittedButVerificationFailed" : "Committed"), "published native state lost");
        if (mode is "native_denied" or "postcheck") Require(terminal.NativeErrorCode == 5, "injected native code lost");
        if (!Published(mode)) Require(File.ReadAllBytes(final).SequenceEqual(mode == "target_changed" ? "external changed target"u8.ToArray() : OldTarget), "precommit failure changed previous target");
    }
    public static Task ExtractionPublication() => WithWorker(async (owned, evidence) => {
        var source = Path.Combine(LongDirectory(owned, "archive"), "源归档.zip"); Zip(source, new Dictionary<string, byte[]> { [Member] = Payload });
        var sourceHash = SHA256.HashData(File.ReadAllBytes(source));
        foreach (var mode in new[] { "new", "direct_unapproved", "ask_overwrite", "overwrite", "skip", "auto_rename", "cancel_open", "cancel_closed", "write_failure" }) {
            var output = LongDirectory(owned, "extract-" + mode); var final = Path.Combine(output, Member.Replace('/', '\\'));
            var parent = Path.GetDirectoryName(final)!; Directory.CreateDirectory(parent);
            var conflict = mode switch { "ask_overwrite" => "ask", "overwrite" => "overwrite", "skip" => "skip", "auto_rename" => "auto_rename", _ => null };
            var existing = mode is "direct_unapproved" or "ask_overwrite" or "overwrite" or "skip" or "auto_rename";
            if (existing) File.WriteAllBytes(final, OldTarget);
            var external = Path.Combine(parent, ".nxpart-external"); File.WriteAllText(external, "external partial");
            var id = Guid.NewGuid(); var reached = 0;
            Action<ArchiveWorkerRunner> setup = runner => {
                runner.OutputCommitCheckpointProbe = stage => {
                    if (mode == "cancel_open" && stage == "before_metadata") {
                        Require(Interlocked.Increment(ref reached) == 1, "cancel-before-close boundary repeated");
                        Require(runner.Cancel(id), "cancel current incomplete long-path output");
                    }
                    if (mode == "cancel_closed" && stage == "native_closed") {
                        Require(Interlocked.Increment(ref reached) == 1, "closed-output boundary repeated");
                        Require(runner.Cancel(id), "cancel after ordinary output close");
                    }
                };
                if (mode == "write_failure") runner.OutputWriteCheckpointProbe = _ => {
                    Require(Interlocked.Increment(ref reached) == 1, "write-failure boundary repeated");
                    throw new ArchiveOutputException("write_file", 112);
                };
            };
            Func<ArchiveWorkerRunner, ConcurrentQueue<ArchiveWorkerEvent>, Task>? answer = null;
            if (mode == "ask_overwrite") answer = async (runner, events) => {
                var clock = Stopwatch.StartNew();
                while (!events.Any(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "conflict_prompt") &&
                    !events.Any(item => item.Kind == "exited")) {
                    Require(clock.ElapsedMilliseconds < 10000, "long-path conflict prompt deadline"); await Task.Delay(10);
                }
                var prompt = events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "conflict_prompt").Frame!.Value.GetProperty("conflict_prompt");
                Require(prompt.GetProperty("existing_bytes").GetUInt64() == (ulong)OldTarget.Length, "long-path conflict existing bytes");
                var ticket = prompt.GetProperty("ticket").Deserialize<ArchivePasswordTicket>()!;
                Require(runner.RespondConflict(id, ticket, "overwrite"), "long-path Ask overwrite decision");
                Interlocked.Increment(ref reached);
            };
            var terminal = await Run(new(id, "extract", source, 0, Limits, OutputRoot: output, ConflictPolicy: conflict),
                evidence, mode, Member.Length, setup, answer);
            Require(terminal.CleanupComplete, "long-path direct output cleanup: " + mode);
            var renamed = Path.Combine(parent, Path.GetFileNameWithoutExtension(final) + " (2)" + Path.GetExtension(final));
            switch (mode) {
                case "new":
                case "overwrite":
                    Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed" &&
                        File.ReadAllBytes(final).SequenceEqual(Payload), "long-path direct success: " + mode);
                    break;
                case "direct_unapproved":
                    Require(terminal.Category == "worker_failure" && terminal.FilesCommitted == 0 && File.ReadAllBytes(final).SequenceEqual(OldTarget), "unapproved long-path overwrite changed target");
                    break;
                case "ask_overwrite":
                    Require(reached == 1 && terminal.Category == "success" && terminal.FilesCommitted == 1 &&
                        File.ReadAllBytes(final).SequenceEqual(Payload), "Ask overwrite did not directly truncate the approved final name");
                    break;
                case "skip":
                    Require(terminal.Category == "succeeded_with_warnings" && terminal.FilesCommitted == 0 && terminal.FilesSkipped == 1 &&
                        File.ReadAllBytes(final).SequenceEqual(OldTarget), "Skip changed existing long-path target");
                    break;
                case "auto_rename":
                    Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && File.ReadAllBytes(final).SequenceEqual(OldTarget) &&
                        File.ReadAllBytes(renamed).SequenceEqual(Payload), "AutoRename did not use a new final long-path name");
                    break;
                case "cancel_open":
                    Require(reached == 1 && terminal.Category == "cancelled" && terminal.FilesCommitted == 0 &&
                        terminal.CommitState == "FailedBeforeCommit" && !File.Exists(final), "open-output cancellation did not delete the incomplete final name");
                    break;
                case "cancel_closed":
                    Require(reached == 1 && terminal.Category == "cancelled_with_committed_output" && terminal.FilesCommitted == 1 &&
                        terminal.CommitState == "Committed" && File.ReadAllBytes(final).SequenceEqual(Payload), "closed-output cancellation did not retain the completed result");
                    break;
                case "write_failure":
                    Require(reached == 1 && terminal.Category == "write_file" && terminal.NativeErrorCode == 112 && terminal.FilesCommitted == 0 &&
                        terminal.CommitState == "FailedBeforeCommit" && !File.Exists(final), "native write failure did not delete the incomplete final name");
                    break;
            }
            var completedPath = mode == "auto_rename" ? renamed : final;
            if (File.Exists(completedPath) && File.ReadAllBytes(completedPath).SequenceEqual(Payload))
                Require(final.Length > 700 && File.GetLastWriteTime(completedPath) == Stamp, "direct long-path metadata lost: " + mode);
            NoOwnedPartial(output, external);
            Require(SHA256.HashData(File.ReadAllBytes(source)).SequenceEqual(sourceHash), "source preserved across direct output outcomes");
            evidence.Add(new { mode, final_utf16 = final.Length, reached, terminal.Category, terminal.FilesCommitted, terminal.FilesSkipped,
                terminal.CommitState, terminal.NativeErrorCode, external_partial_retained = true });
        }
        using var released = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
    });
    public static Task CreationPublication() => WithWorker(async (owned, evidence) => {
        var source = LongDirectory(owned, "source"); var child = Path.Combine(source, MemberDirectory.Replace('/', '\\')); Directory.CreateDirectory(child);
        File.WriteAllBytes(Path.Combine(child, "内容.bin"), Payload); File.WriteAllBytes(Path.Combine(child, "空文件.bin"), []); Directory.CreateDirectory(Path.Combine(child, "空目录"));
        var expected = new Dictionary<string, byte[]> { [Member] = Payload, [MemberDirectory + "/空文件.bin"] = [], [MemberDirectory + "/空目录/"] = [] };
        var parts = MemberDirectory.Split('/'); for (var i = 1; i <= parts.Length; i++) expected.Add(string.Join('/', parts.Take(i)) + "/", []);
        foreach (var mode in CandidateModes) {
            var output = LongDirectory(owned, "create-" + mode); const string name = "创建输出.zip"; var final = Path.Combine(output, name);
            if (mode != "new") File.WriteAllBytes(final, OldTarget);
            var external = Path.Combine(output, ".nxpart-external"); File.WriteAllText(external, "external partial");
            var confirmation = Confirmation(output, name, mode); var id = Guid.NewGuid(); var reached = 0; var published = 0; var stableEnumerations = 0;
            var probe = PublicationProbe(id, mode, () => reached++, () => published++, final);
            var terminal = await Run(new(id, "create_zip", source, 0, Limits, OutputRoot: output, ConfirmedOverwrites: confirmation, CreateName: name, CompressionLevel: "normal", ZipOptions: new(StripSingleRoot: true)),
                evidence, mode, Member.Length, runner => {
                    probe(runner); runner.SourceNamespacePathProbe = path => { Require(path.StartsWith(@"\\?\Volume{", StringComparison.Ordinal) && path.Length > 400, "held-directory-derived long volume namespace"); stableEnumerations++; return path; };
                });
            PublicationResult(terminal, mode, reached, published, final);
            Require(stableEnumerations > 0 && File.ReadAllBytes(Path.Combine(child, "内容.bin")).SequenceEqual(Payload), "real long source enumeration/content");
            // External Python needs an extended Win32 path here; public product requests above remain ordinary DOS paths.
            if (Published(mode)) await ArchiveZipCreationTests.Independent(@"\\?\" + final, expected, 8);
            NoOwnedPartial(output, external); ExactLayout(source, expected);
            evidence.Add(new { mode, source_file_utf16 = Path.Combine(child, "内容.bin").Length, final_utf16 = final.Length, expected_entries = expected.Count,
                stable_enumerations = stableEnumerations, reached, native_publications = published, independent_oracle = Published(mode), injected_native_code = mode is "native_denied" or "postcheck" ? 5 : (int?)null });
        }
        using var released = new FileStream(Path.Combine(child, "内容.bin"), FileMode.Open, FileAccess.ReadWrite, FileShare.None);
    });
    public static Task UnsafeMembers() => WithWorker(async (owned, evidence) => {
        var parent = LongDirectory(owned, "unsafe"); var output = Path.Combine(parent, "destination"); Directory.CreateDirectory(output);
        var sentinel = Path.Combine(parent, "outside.txt"); File.WriteAllText(sentinel, "outside unchanged");
        var names = new[] { "../outside.txt", "/absolute.txt", @"C:\absolute.txt", @"C:relative.txt", @"\\server\share\payload", @"\\?\C:\payload", "file:stream", "CON.txt" };
        for (var i = 0; i < names.Length; i++) {
            var source = Path.Combine(parent, "unsafe-" + i + ".zip"); Zip(source, new Dictionary<string, byte[]> { [names[i]] = Payload });
            var terminal = await Run(new(Guid.NewGuid(), "extract", source, 0, Limits, OutputRoot: output), evidence, "unsafe-" + i, names[i].Length);
            Require(terminal.Category == "worker_failure" && terminal.FilesCommitted == 0 && terminal.CleanupComplete, "dangerous member policy weakened");
            Require(Directory.GetFileSystemEntries(output).Length == 0 && File.ReadAllText(sentinel) == "outside unchanged", "dangerous member wrote outside destination");
        }
        foreach (var invalid in new[] { @"\\?\C:\private.zip", @"\\?\Volume{00000000-0000-0000-0000-000000000000}\private.zip", @"\\server\share\private.zip", @"\??\C:\private.zip" }) {
            try { using var lease = ArchiveSourceLease.Open(invalid); throw new InvalidOperationException("internal namespace accepted as public source"); }
            catch (ArchiveOutputException failure) { Require(failure.Stage == "invalid_source", "public namespace rejection category"); }
            try { using var outputSession = ArchiveOutputSession.Prepare(invalid, Guid.NewGuid()); throw new InvalidOperationException("internal namespace accepted as public output"); }
            catch (ArchiveOutputException failure) { Require(failure.Stage == "invalid_root", "public output namespace rejection category"); }
        }
        evidence.Add(new { unsafe_members_rejected = names.Length, internal_public_prefixes_rejected = 4, outside_sentinel_unchanged = true });
    });
}
