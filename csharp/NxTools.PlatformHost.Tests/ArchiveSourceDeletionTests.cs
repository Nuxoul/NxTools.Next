using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO.Compression;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Only owned fixtures + recording backend. This file must never call Shell recycle APIs.
internal static class ArchiveSourceDeletionTests
{
    private static void Require(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    private sealed class Backend : IArchiveRecycleBackend
    {
        internal int Calls;
        internal readonly List<Guid> Ids = [];
        internal ArchiveRecycleKind Kind = ArchiveRecycleKind.Failed;
        internal Func<ArchiveRecycleSource, CancellationToken, ArchiveRecycleNativeResult>? Action;
        public ArchiveRecycleNativeResult Recycle(ArchiveRecycleSource source, CancellationToken cancellation)
        {
            Require(Thread.CurrentThread.GetApartmentState() == ApartmentState.STA, "recycle owner must be STA");
            Interlocked.Increment(ref Calls); Ids.Add(source.Id);
            if (Action is { } action) return action(source, cancellation);
            return new(Kind, Kind == ArchiveRecycleKind.Failed ? 5 : null);
        }
    }
    private static async Task WithWorker(Func<DirectoryInfo, Task> action)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit actual Rust worker required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-source-delete-");
        var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await action(owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static string Zip(DirectoryInfo owned)
    {
        var source = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + ".zip");
        using var zip = ZipFile.Open(source, ZipArchiveMode.Create);
        using var writer = new StreamWriter(zip.CreateEntry("payload.txt").Open()); writer.Write("known synthetic source");
        return source;
    }
    private static async Task<ArchiveWorkerEvent> Wait(ArchiveWorkerRunner runner, ConcurrentQueue<ArchiveWorkerEvent> events)
    {
        var clock = Stopwatch.StartNew();
        while (!events.Any(item => item.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 20000, "source-delete deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "source-delete drain");
        return events.Single(item => item.Kind == "exited");
    }
    private static void RequireSourceProtected(string source)
    {
        try { using var writer = new FileStream(source, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); throw new InvalidOperationException("source lease released before output boundary"); }
        catch (IOException) { }
    }
    public static Task ConsentAndFailureProjection() => WithWorker(async owned => {
        foreach (var consent in new bool?[] { null, false, true }) {
            var source = Zip(owned);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var backend = new Backend();
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend);
            var id = Guid.NewGuid();
            Require(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: output, DeleteSource: consent)), "delete admission");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed", "recycle failure changed archive success");
            Require(backend.Calls == (consent == true ? 1 : 0), "default/consent backend invocation");
            Require(terminal.SourceDeletion?.Reason == (consent == true ? "completed" : "consent_missing"), "separate recycle outcome");
            if (consent == true) {
                Require(terminal.SourceDeletion!.Sources.Single().Kind == ArchiveRecycleKind.Failed && terminal.SourceDeletion.Sources.Single().NativeCode == 5, "typed native recycle result");
                Require(!runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: output, DeleteSource: true)), "repeat operation authorized");
                Require(backend.Calls == 1, "duplicate recycle");
            }
            Require(File.Exists(source) && File.ReadAllText(Path.Combine(output, "payload.txt")) == "known synthetic source", "recording backend changed source/output");
        }
    });
    public static Task CreationCoverageAndNativeRefusal() => WithWorker(async owned => {
        foreach (var mode in new[] { "files", "directory", "default_excluded", "strip_empty", "active_temp", "native_refused" }) {
            var input = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"));
            IReadOnlyList<string> sources;
            if (mode is "directory" or "default_excluded" or "strip_empty") {
                Directory.CreateDirectory(input);
                if (mode != "strip_empty") File.WriteAllText(Path.Combine(input, "payload.txt"), "included");
                if (mode == "default_excluded") File.WriteAllText(Path.Combine(input, "Thumbs.db"), "must be retained");
                sources = [input];
            } else { File.WriteAllText(input, "first"); var second = input + "-second"; File.WriteAllText(second, "second"); sources = [input, second]; }
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var backend = new Backend { Kind = ArchiveRecycleKind.Unsupported };
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = mode == "native_refused" ? ArchiveWorkerRunner.ForPackage(events.Enqueue) : ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend);
            var request = new ArchiveWorkerRequest(Guid.NewGuid(), "create_zip", input, 0, new { max_entries = 100 }, OutputRoot: output,
                CreateName: "created.zip", CompressionLevel: "normal", SourcePaths: sources,
                ZipOptions: new(StripSingleRoot: mode == "strip_empty"), DeleteSource: true, ActiveTemporarySource: mode == "active_temp");
            Require(runner.Start(request), "creation delete admission");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1, "recycle coverage changed creation");
            var expected = mode switch { "files" => "completed", "default_excluded" => "members_excluded", "directory" or "strip_empty" => "directory_coverage_unproven", "active_temp" => "active_temporary_source", _ => "native_protection_unproven" };
            Require(terminal.SourceDeletion?.Reason == expected, "coverage refusal: " + mode + "/" + terminal.SourceDeletion?.Reason);
            Require(backend.Calls == (mode == "files" ? 2 : 0), "ineligible backend count: " + mode);
            using var zip = ZipFile.OpenRead(Path.Combine(output, "created.zip"));
            if (mode == "files") Require(zip.Entries.Count == 2 && terminal.SourceDeletion!.Sources.All(item => item.Kind == ArchiveRecycleKind.Unsupported), "per-source unsupported results");
            if (mode == "default_excluded") Require(File.ReadAllText(Path.Combine(input, "Thumbs.db")) == "must be retained", "excluded data changed");
        }
    });

    public static Task OnceOnlyUnknownAndBinding() => WithWorker(async owned => {
        var source = Zip(owned); var output = Directory.CreateDirectory(Path.Combine(owned.FullName, "output")).FullName;
        var events = new ConcurrentQueue<ArchiveWorkerEvent>();
        using var entered = new ManualResetEventSlim(); using var release = new ManualResetEventSlim();
        IArchiveRecycleAttempt? originalSession = null;
        var backend = new Backend { Action = (_, _) => { entered.Set(); Require(release.Wait(10000), "release original backend"); throw new IOException("synthetic unknown acknowledgement"); } };
        using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend);
        runner.RecycleSessionProbe = (stage, session) => {
            if (stage == "captured") {
                originalSession = session;
                Require(session.ExecuteAsync(session.OperationId, session.Token).GetAwaiter().GetResult().Reason == "not_qualified", "capture must not imply completion");
            } else {
                Require(ReferenceEquals(originalSession, session), "completion changed original session");
                Require(session.ExecuteAsync(Guid.NewGuid(), session.Token).GetAwaiter().GetResult().Reason == "authorization_mismatch", "wrong job consumed original grant");
                Require(session.ExecuteAsync(session.OperationId, Guid.NewGuid()).GetAwaiter().GetResult().Reason == "authorization_mismatch", "wrong token consumed original grant");
            }
        };
        var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", source, 0, new { max_entries = 100 }, OutputRoot: output, DeleteSource: true);
        Require(runner.Start(request), "binding actual job");
        try {
            Require(entered.Wait(10000), "original backend start");
            var session = originalSession ?? throw new InvalidOperationException("original session missing");
            var concurrent = await Task.WhenAll(Enumerable.Range(0, 8).Select(_ => session.ExecuteAsync(request.OperationId, session.Token)));
            Require(concurrent.All(result => result.Reason == "already_consumed"), "concurrent duplicate dispatched");
            Require(!runner.Start(request with { }), "cloned original operation admitted twice");
        }
        finally { release.Set(); }
        var terminal = await Wait(runner, events);
        Require(terminal.Category == "success" && terminal.SourceDeletion!.Sources.Single().Kind == ArchiveRecycleKind.OutcomeUnknown && backend.Calls == 1, "unknown must not retry or rewrite archive");
        var retiredSession = originalSession ?? throw new InvalidOperationException("original session missing");
        Require((await retiredSession.ExecuteAsync(request.OperationId, retiredSession.Token)).Reason == "retired" && backend.Calls == 1, "late retired duplicate");
    });
    [System.Runtime.InteropServices.DllImport("kernel32.dll", EntryPoint = "CreateFileW", ExactSpelling = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode, SetLastError = true)]
    private static extern Microsoft.Win32.SafeHandles.SafeFileHandle OpenDeleteAccess(string path, uint access, uint share, IntPtr security, uint disposition, uint flags, IntPtr template);
    public static Task LeaseCloseResolveRace()
    {
        var owned = Directory.CreateTempSubdirectory("archive-recycle-race-");
        try {
            var source = Path.Combine(owned.FullName, "source.bin"); var original = source + ".original";
            File.WriteAllText(source, "original object");
            object before;
            using (var lease = ArchiveSourceLease.Open(source)) {
                lease.RecycleSource(Guid.NewGuid()).Verify();
                using var denied = OpenDeleteAccess(source, 0x10000, 7, IntPtr.Zero, 3, 0, IntPtr.Zero);
                Require(denied.IsInvalid && System.Runtime.InteropServices.Marshal.GetLastWin32Error() == 32, "live lease must deny DELETE access without performing deletion");
                using var oracle = File.OpenHandle(source, FileMode.Open, FileAccess.Read, FileShare.Read);
                before = OutputNative.Identity(oracle);
            }
            // Deliberately model only the unprotected name-resolution gap; no Shell call/delete.
            File.Move(source, original); File.WriteAllText(source, "replacement object");
            using var replacement = File.OpenHandle(source, FileMode.Open, FileAccess.Read, FileShare.Read);
            Require(!before.Equals(OutputNative.Identity(replacement)), "path resolver must see replacement identity");
            Require(File.ReadAllText(original) == "original object" && File.ReadAllText(source) == "replacement object", "non-destructive race fixture");
        }
        finally { owned.Delete(true); }
        return Task.CompletedTask;
    }

    public static Task FailureAndCancellationMatrix() => WithWorker(async owned => {
        foreach (var mode in new[] { "metadata_failure", "motw_failure", "directory_failure", "write_failure", "cancel_open", "cancel_closed", "active_temp" }) {
            var source = Zip(owned); var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            if (mode == "directory_failure") {
                using var zip = ZipFile.Open(source, ZipArchiveMode.Update);
                zip.CreateEntry("empty/").ExternalAttributes = 0x10;
            }
            if (mode == "motw_failure") File.WriteAllText(source + ":Zone.Identifier", "[ZoneTransfer]\r\nZoneId=3\r\n");
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new Backend();
            using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend);
            var id = Guid.NewGuid(); var hits = 0;
            runner.OutputCommitCheckpointProbe = stage => {
                var expected = mode switch {
                    "metadata_failure" or "cancel_open" => "before_metadata",
                    "motw_failure" => "before_motw",
                    "directory_failure" => "before_directory_metadata",
                    "cancel_closed" or "active_temp" => "native_closed",
                    _ => null
                };
                if (stage != expected) return;
                RequireSourceProtected(source);
                Require(Interlocked.Increment(ref hits) == 1, "output boundary repeated: " + mode);
                if (mode == "metadata_failure" || mode == "directory_failure") throw new ArchiveOutputException("metadata_time_failed", 5);
                if (mode == "motw_failure") throw new ArchiveOutputException("motw_write_failed", 5);
                if (mode == "cancel_open") Require(runner.Cancel(id), "cancel incomplete ordinary output");
                if (mode == "cancel_closed") Require(runner.Cancel(id), "cancel after ordinary output close");
            };
            if (mode == "write_failure") runner.OutputWriteCheckpointProbe = _ => {
                RequireSourceProtected(source);
                Require(Interlocked.Increment(ref hits) == 1, "write boundary repeated");
                throw new ArchiveOutputException("write_file", 112);
            };
            Require(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: output,
                DeleteSource: true, ActiveTemporarySource: mode == "active_temp")), "failure admission");
            var terminal = await Wait(runner, events); var final = Path.Combine(output, "payload.txt");
            Require(hits == 1, "configured cancellation/failure boundary not reached: " + mode);
            var expectedCategory = mode switch {
                "metadata_failure" or "directory_failure" => "metadata_time_failed",
                "motw_failure" => "motw_write_failed",
                "write_failure" => "write_file",
                "cancel_open" => "cancelled",
                "cancel_closed" => "cancelled_with_committed_output",
                _ => "success"
            };
            var committed = mode is "directory_failure" or "cancel_closed" or "active_temp" ? 1 : 0;
            Require(terminal.Category == expectedCategory && terminal.FilesCommitted == committed &&
                terminal.CommitState == (committed == 1 ? "Committed" : "FailedBeforeCommit"), "direct output terminal truth: " + mode + "/" + terminal.Category);
            if (mode is "metadata_failure" or "motw_failure" or "directory_failure") Require(terminal.NativeErrorCode == 5, "metadata native code lost: " + mode);
            if (mode == "write_failure") Require(terminal.NativeErrorCode == 112, "write native code lost");
            if (committed == 1) Require(File.ReadAllText(final) == "known synthetic source", "completed ordinary output was not retained: " + mode);
            else Require(!File.Exists(final), "incomplete ordinary output was not deleted: " + mode);
            Require(backend.Calls == 0, "failed, cancelled, or active temporary source reached recycle backend: " + mode);
            Require(terminal.SourceDeletion?.Reason == (mode == "active_temp" ? "active_temporary_source" : "archive_not_fully_succeeded"),
                "source-delete refusal reason: " + mode + "/" + terminal.SourceDeletion?.Reason);
            Require(File.Exists(source) && !Directory.EnumerateFiles(output, ".nxpart-*", SearchOption.AllDirectories).Any(), "source or ordinary direct cleanup truth: " + mode);
            using var released = new FileStream(source, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        }
    });
    public static Task CodecFullExtraction() => WithWorker(async owned => {
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES") ?? throw new TestSkippedException("Explicit codec fixtures required.");
        foreach (var name in new[] { "lzma.7z", "rar4.rar", "rar5.rar" }) {
            var source = Path.Combine(owned.FullName, name); File.Copy(Path.Combine(fixtures, name), source);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, name + "-output")).FullName;
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new Backend { Kind = ArchiveRecycleKind.Recycled };
            using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend);
            Require(runner.Start(new(Guid.NewGuid(), "extract", source, 0, new { max_entries = 10000 }, OutputRoot: output, DeleteSource: true)), "codec delete admission");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && backend.Calls == 1 && terminal.SourceDeletion?.Sources.Single().Kind == ArchiveRecycleKind.Recycled,
                "full codec extraction qualification: " + name + "/" + terminal.SourceDeletion?.Reason);
        }
    });
    private sealed class CaptureLog : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        internal bool Reject;
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (Reject) throw new InvalidOperationException("synthetic log failure");
            Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
        }
    }
    public static Task PartialResultsCancellationAndPrivacy() => WithWorker(async owned => {
        foreach (var mode in new[] { "partial", "cancel", "unknown", "log_throws" }) {
            var sentinel = "source-delete-private-sentinel";
            var first = Path.Combine(owned.FullName, Guid.NewGuid().ToString("N") + sentinel); var second = first + "-second";
            File.WriteAllText(first, "payload secret sentinel"); File.WriteAllText(second, "second secret");
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog { Reject = mode == "log_throws" };
            var backend = new Backend();
            using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend, log);
            var id = Guid.NewGuid();
            backend.Action = (_, cancellation) => {
                Require(runner.ActiveCount == 1 && !events.Any(item => item.Kind == "exited"), "recycle released operation before actual terminal");
                var denied = false;
                try { using var writer = File.Open(first, FileMode.Open, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete); }
                catch (IOException) { denied = true; }
                Require(denied, "source lease released during recycle");
                if (mode == "cancel") { Require(runner.Cancel(id), "recycle cancel acceptance"); Require(cancellation.IsCancellationRequested, "cancel did not reach execution"); return new(ArchiveRecycleKind.Cancelled, 1223); }
                if (mode == "unknown") throw new IOException("private path/message must never be logged");
                return new(backend.Calls == 1 ? ArchiveRecycleKind.Recycled : ArchiveRecycleKind.Failed, backend.Calls == 1 ? null : 5);
            };
            Require(runner.Start(new(id, "create_zip", first, 0, new { max_entries = 100 }, OutputRoot: output,
                CreateName: "created.zip", CompressionLevel: "normal", SourcePaths: new[] { first, second }, DeleteSource: true)), "partial recycle admission");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && terminal.FilesCommitted == 1 && terminal.CommitState == "Committed", "recycle cancellation/failure rewrote archive success");
            var results = terminal.SourceDeletion!.Sources;
            Require(results.Count == 2 && results.Select(item => item.SourceId).Distinct().Count() == 2, "bounded exact per-source result set");
            if (mode == "cancel") Require(backend.Calls == 1 && results[0].Kind == ArchiveRecycleKind.Cancelled && results[1].Kind == ArchiveRecycleKind.NotAttempted, "cancelled/remaining outcomes");
            else if (mode == "unknown") Require(backend.Calls == 1 && results[0].Kind == ArchiveRecycleKind.OutcomeUnknown && results[1].Kind == ArchiveRecycleKind.NotAttempted, "unknown must stop and never retry");
            else Require(backend.Calls == 2 && results[0].Kind == ArchiveRecycleKind.Recycled && results[1].Kind == ArchiveRecycleKind.Failed && results[1].NativeCode == 5, "partial success projection");
            Require(File.ReadAllText(first) == "payload secret sentinel" && File.ReadAllText(second) == "second secret" && File.Exists(Path.Combine(output, "created.zip")), "recording backend changed sources/output");
            if (!log.Reject) {
                var records = log.Records.Where(item => item.Name.StartsWith("archive.recycle.", StringComparison.Ordinal)).ToArray();
                Require(records.Select(item => item.Name).SequenceEqual(new[] { "archive.recycle.qualified", "archive.recycle.accepted", "archive.recycle.started", "archive.recycle.terminal" }), "bounded diagnostic lifecycle");
                Require(records.All(item => item.Id == id && item.Fields.ContainsKey("elapsed_ms")), "correlation/elapsed lost");
                var text = System.Text.Json.JsonSerializer.Serialize(records.Select(item => new { item.Name, item.Id, item.Fields }));
                Require(!text.Contains(sentinel) && !text.Contains("payload secret") && !text.Contains(first) && !text.Contains("SourceId"), "diagnostic privacy");
            }
        }
    });
    public static Task RetiredEvidenceCannotBindReplacement() => WithWorker(async owned => {
        foreach (var isolatedBackend in new[] { false, true }) {
            var source = Zip(owned); var original = source + ".original"; var originalBytes = File.ReadAllBytes(source);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var foreign = Path.Combine(output, ".nxpart-foreign"); File.WriteAllText(foreign, "foreign partial");
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new Backend { Kind = ArchiveRecycleKind.Recycled };
            IArchiveRecycleAttempt? originalSession = null;
            using var runner = isolatedBackend ? ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend) : ArchiveWorkerRunner.ForPackage(events.Enqueue);
            runner.RecycleSessionProbe = (stage, session) => {
                if (stage == "captured") originalSession = session;
                if (stage == "qualified") Require(session.ExecuteAsync(session.OperationId, session.Token).GetAwaiter().GetResult().Reason == "owner_required", "observer started original backend");
            };
            var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", source, 0, new { max_entries = 100 }, OutputRoot: output, DeleteSource: true);
            Require(runner.Start(request), "original production admission");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && terminal.SourceDeletion?.Reason == (isolatedBackend ? "completed" : "native_protection_unproven"), "original production terminal");
            var callsBeforeReplay = backend.Calls;
            Require(callsBeforeReplay == (isolatedBackend ? 1 : 0), "legal original session must execute once only");
            var evidence = events.Single(item => item.Frame is { } frame && frame.GetProperty("op").GetString() == "complete").Frame!.Value.GetProperty("source_delete");
            var session = originalSession ?? throw new InvalidOperationException("production session missing");
            File.Move(source, original); File.WriteAllText(source, "replacement never archived");
            using var replacementLease = ArchiveSourceLease.Open(source);
            replacementLease.RecycleSource(request.OperationId).Verify(); // A new lease cannot mint a session.
            foreach (var replay in new[] { request, request with { } }) {
                Require((await session.ExecuteAsync(replay.OperationId, session.Token)).Reason == "retired", "retired source session revived");
                Require(!runner.Start(replay), "old/clone operation replay admitted");
                Require(!runner.Write(replay.OperationId, evidence), "old worker evidence accepted as new authority");
            }
            Require(backend.Calls == callsBeforeReplay, "retired source or request clone dispatched again");
            Require(typeof(ArchiveWorkerRunner).Assembly.GetType("NxTools.PlatformHost.Windows.Archive.ArchiveRecycleAuthorization") is null, "external qualification factory still exists");
            Require(File.ReadAllBytes(original).SequenceEqual(originalBytes) && File.ReadAllText(source) == "replacement never archived" && File.ReadAllText(foreign) == "foreign partial" &&
                File.ReadAllText(Path.Combine(output, "payload.txt")) == "known synthetic source", "replay mutated original/replacement/output/foreign partial");
        }
    });
    public static Task OriginalSessionRejectsChangedEvidence() => WithWorker(async owned => {
        foreach (var mode in new[] { "wrong_job", "wrong_source", "duplicate_source", "extra_field" }) {
            var source = Zip(owned); var before = File.ReadAllBytes(source);
            var output = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N"))).FullName;
            var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var backend = new Backend(); IArchiveRecycleAttempt? captured = null;
            using var runner = ArchiveWorkerRunner.ForRecycleFixture(events.Enqueue, backend, evidenceProbe: evidence => {
                var changed = System.Text.Json.Nodes.JsonNode.Parse(evidence.GetRawText())!;
                if (mode == "wrong_job") changed["operation_id"] = Guid.NewGuid();
                if (mode == "wrong_source") changed["source_ids"] = new System.Text.Json.Nodes.JsonArray(System.Text.Json.Nodes.JsonValue.Create(Guid.NewGuid()));
                if (mode == "duplicate_source") changed["source_ids"] = new System.Text.Json.Nodes.JsonArray(changed["source_ids"]![0]!.DeepClone(), changed["source_ids"]![0]!.DeepClone());
                if (mode == "extra_field") changed["path"] = "untrusted source path";
                return System.Text.Json.JsonSerializer.SerializeToElement(changed);
            });
            runner.RecycleSessionProbe = (stage, session) => { if (stage == "captured") captured = session; };
            var request = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", source, 0, new { max_entries = 100 }, OutputRoot: output, DeleteSource: true);
            Require(runner.Start(request), "original evidence-bound job");
            var terminal = await Wait(runner, events);
            Require(terminal.Category == "success" && backend.Calls == 0 && terminal.SourceDeletion?.Reason ==
                (mode is "wrong_job" or "extra_field" ? "invalid_evidence" : "source_set_mismatch"), "changed evidence escaped original session: " + mode);
            var session = captured ?? throw new InvalidOperationException("original session not captured");
            Require((await session.ExecuteAsync(request.OperationId, session.Token)).Reason == "retired", "refused original session revived");
            Require(File.ReadAllBytes(source).SequenceEqual(before) && File.ReadAllText(Path.Combine(output, "payload.txt")) == "known synthetic source", "changed evidence modified files");
        }
    });
}
