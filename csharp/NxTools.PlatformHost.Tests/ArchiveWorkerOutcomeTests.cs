using System.Collections.Concurrent;
using System.Diagnostics;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: attributed tiny archive, copied owned helper/Job and GUID-owned files/ACL/junction only.
internal static class ArchiveWorkerOutcomeTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private sealed class CaptureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<(string Name, Guid? Id, IReadOnlyDictionary<string, object?> Fields)> Records = new();
        internal bool Reject;
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            if (Reject) throw new InvalidOperationException("fixture diagnostic rejection");
            Records.Enqueue((eventName, correlationId, new Dictionary<string, object?>(fields ?? new Dictionary<string, object?>())));
        }
    }
    private static async Task WithWorker(Func<string, DirectoryInfo, Task> test)
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and attributed7z fixture required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "refuse existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-outcome-"); var copied = false;
        try { File.Copy(worker, helper, false); copied = true; await test(Path.Combine(fixtures, "lzma.7z"), owned); }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
    private static async Task<ArchiveWorkerEvent> Finish(ArchiveWorkerRunner runner, ConcurrentQueue<ArchiveWorkerEvent> events)
    {
        var clock = Stopwatch.StartNew();
        while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 15000, "worker terminal deadline"); await Task.Delay(20); }
        Require(await runner.DrainAsync(), "worker drain");
        return events.Single(e => e.Kind == "exited");
    }
    private static string Confirm(string root)
    {
        using var capture = ArchiveOutputSession.Open(root, Guid.NewGuid());
        return capture.CaptureTargetIdentity("file.txt");
    }
    private static void RestoreAcl(string path, FileSecurity original)
    {
        original.SetSecurityDescriptorSddlForm(original.GetSecurityDescriptorSddlForm(AccessControlSections.Access), AccessControlSections.Access);
        new FileInfo(path).SetAccessControl(original);
    }
    public static Task ActualTargetFailures() => WithWorker(async (source, owned) => {
        var failures = new List<string>();
        foreach (var entrance in new[] { "begin", "commit" })
        foreach (var mode in new[] { "changed", "missing", "directory", "reparse", "busy", "denied" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var path = Path.Combine(root.FullName, "file.txt"); File.WriteAllText(path, "original");
            var userPartial = Path.Combine(root.FullName, ".nxpart-user"); File.WriteAllText(userPartial, "user");
            var outside = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            File.WriteAllText(Path.Combine(outside.FullName, "sentinel"), "outside");
            var expected = Confirm(root.FullName); var reached = false;
            IDisposable? occupation = null; FileSecurity? previousAcl = null;
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
            void Mutate() {
                if (reached) return; reached = true;
                switch (mode) {
                    case "changed": File.WriteAllText(path, "external"); break;
                    case "missing": File.Delete(path); break;
                    case "directory": File.Delete(path); Directory.CreateDirectory(path); break;
                    case "reparse":
                        File.Delete(path);
                        using (var command = Process.Start(new ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{path}\" \"{outside.FullName}\"") {
                            UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!) {
                            command.StandardOutput.ReadToEnd(); command.StandardError.ReadToEnd();
                            Require(command.WaitForExit(5000) && command.ExitCode == 0, "owned junction creation");
                        }
                        break;
                    case "busy": occupation = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None); break;
                    case "denied":
                        var file = new FileInfo(path); previousAcl = file.GetAccessControl();
                        var restricted = file.GetAccessControl();
                        restricted.AddAccessRule(new FileSystemAccessRule(WindowsIdentity.GetCurrent().User!, FileSystemRights.ReadData, AccessControlType.Deny));
                        file.SetAccessControl(restricted); break;
                }
            }
            if (entrance == "begin") runner.SourceCheckpointProbe = stage => { if (stage == "source_validated") Mutate(); };
            else runner.OutputCommitCheckpointProbe = stage => { if (stage == "before_recheck") Mutate(); };
            try {
                Require(runner.Start(new(Guid.NewGuid(), "extract", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName,
                    ConfirmedOverwrites: new Dictionary<string, string> { ["file.txt"] = expected })), "target test admission");
                var terminal = await Finish(runner, events);
                occupation?.Dispose(); occupation = null;
                if (previousAcl is not null) { RestoreAcl(path, previousAcl); previousAcl = null; }
                var native = mode == "missing" ? 2 : mode == "busy" ? 32 : mode == "denied" ? 5 : (int?)null;
                Require(reached, "target barrier not reached");
                Require(terminal.Category == "target_" + mode && terminal.CommitState == "FailedBeforeCommit" &&
                    terminal.NativeErrorCode == native && terminal.FilesCommitted == 0 && terminal.CleanupComplete,
                    $"{entrance}/{mode}: category={terminal.Category},state={terminal.CommitState},native={terminal.NativeErrorCode},count={terminal.FilesCommitted},cleanup={terminal.CleanupComplete}");
                if (mode == "missing") Require(!File.Exists(path), "missing target recreated");
                else if (mode is "directory" or "reparse") Require(Directory.Exists(path), "target kind changed");
                else Require(File.ReadAllText(path) == (mode == "changed" ? "external" : "original"), "target bytes changed");
                Require(Directory.GetFiles(root.FullName, ".nxpart-*").SequenceEqual(new[] { userPartial }) && File.ReadAllText(userPartial) == "user", "partial ownership cleanup");
                Require(File.ReadAllText(Path.Combine(outside.FullName, "sentinel")) == "outside", "outside changed");
            }
            catch (InvalidOperationException failure) { failures.Add(failure.Message); }
            finally {
                occupation?.Dispose();
                if (previousAcl is not null) RestoreAcl(path, previousAcl);
                if (mode == "reparse" && Directory.Exists(path)) Directory.Delete(path);
            }
        }
        Require(failures.Count == 0, string.Join("; ", failures));
    });
    public static Task ActualTerminalDiagnostics() => WithWorker(async (source, owned) => {
        var failures = new List<string>();
        foreach (var mode in new[] { "unsupported", "normal", "cancel_ready", "cancel_during", "unknown", "post_error", "post_changed", "reject_log" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var path = Path.Combine(root.FullName, "file.txt"); File.WriteAllText(path, "original");
            var expected = Confirm(root.FullName); var log = new CaptureLog { Reject = mode == "reject_log" };
            var events = new ConcurrentQueue<ArchiveWorkerEvent>();
            using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log); var id = Guid.NewGuid();
            runner.OutputCommitCheckpointProbe = stage => {
                if (stage == "commit_in_flight" && mode is "unsupported" or "unknown") throw new ArchiveOutputException("replace_file", mode == "unsupported" ? 50 : 64);
                if (mode == "cancel_ready" && stage == "target_rechecked" || mode == "cancel_during" && stage == "commit_in_flight") Require(runner.Cancel(id), "cancel admission");
                if (stage == "native_committed" && mode == "post_error") throw new ArchiveOutputException("inspect_full_identity", 5);
                if (stage == "native_committed" && mode == "post_changed") { File.Move(path, path + ".published"); File.WriteAllText(path, "external"); }
            };
            Require(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName,
                ConfirmedOverwrites: new Dictionary<string, string> { ["file.txt"] = expected })), "diagnostic admission");
            var terminal = await Finish(runner, events);
            try {
                var published = mode is "normal" or "cancel_during" or "post_error" or "post_changed" or "reject_log";
                var category = mode == "unsupported" ? "overwrite_not_supported" : mode == "unknown" ? "commit_outcome_unknown" : mode == "cancel_ready" ? "cancelled" :
                    mode == "cancel_during" ? "cancelled_with_committed_output" : mode == "post_error" ? "committed_but_verification_failed" : mode == "post_changed" ? "target_changed_after_commit" : "success";
                Require(terminal.Category == category && terminal.FilesCommitted == (published ? 1 : 0) && terminal.CleanupComplete, "diagnostic event semantics: " + mode);
                Require(File.ReadAllText(path) == (mode == "post_changed" ? "external" : published ? "this is a file\n" : "original"), "diagnostic output bytes");
                Require(Directory.GetFiles(root.FullName, ".nxpart-*").Length == (mode == "unknown" ? 1 : 0), "diagnostic evidence cleanup");
                if (mode == "reject_log") continue;
                var record = log.Records.Single(r => r.Name is "archive.worker.completed" or "archive.worker.cancelled" or "archive.worker.failed");
                Require(record.Id == terminal.OperationId && Equals(record.Fields["category"], terminal.Category) &&
                    Equals(record.Fields["native_error_code"], terminal.NativeErrorCode) && Equals(record.Fields["commit_state"], terminal.CommitState) &&
                    Equals(record.Fields["files_committed"], terminal.FilesCommitted) && Equals(record.Fields["cleanup_complete"], terminal.CleanupComplete) &&
                    Equals(record.Fields["elapsed_ms"], terminal.ElapsedMs), "terminal log/event fields disagree: " + mode);
                var serialized = JsonSerializer.Serialize(log.Records.Select(record => new { record.Name, record.Id, record.Fields }).ToArray());
                Require(!serialized.Contains(root.FullName, StringComparison.Ordinal) && !serialized.Contains("file.txt", StringComparison.Ordinal) && !serialized.Contains("original", StringComparison.Ordinal), "diagnostic payload leak");
            }
            catch (Exception failure) when (failure is InvalidOperationException or KeyNotFoundException) { failures.Add(mode + ": " + failure.Message); }
        }
        Require(failures.Count == 0, string.Join("; ", failures));
    });
    public static Task ActualOperationIsolation() => WithWorker(async (_, owned) => {
        var source = Path.Combine(owned.FullName, "independent.zip");
        using (var archive = System.IO.Compression.ZipFile.Open(source, System.IO.Compression.ZipArchiveMode.Create)) {
            foreach (var name in new[] { "first.txt", "file.txt" }) {
                using var stream = archive.CreateEntry(name, System.IO.Compression.CompressionLevel.NoCompression).Open();
                stream.Write("independent"u8);
            }
        }
        var events = new ConcurrentQueue<ArchiveWorkerEvent>(); var log = new CaptureLog();
        using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue, log);
        foreach (var mode in new[] { "begin", "commit", "cancel", "next_success" }) {
            var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            var path = Path.Combine(root.FullName, "file.txt"); File.WriteAllText(path, "original");
            var expected = Confirm(root.FullName); var id = Guid.NewGuid(); var commits = 0;
            if (mode == "begin") File.WriteAllText(path, "external");
            runner.OutputCommitCheckpointProbe = stage => {
                if (stage == "before_recheck" && commits == 1 && mode == "commit") File.WriteAllText(path, "external");
                if (stage == "native_committed") { commits++; if (mode == "cancel") Require(runner.Cancel(id), "isolation cancel"); }
            };
            Require(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName,
                ConfirmedOverwrites: new Dictionary<string, string> { ["file.txt"] = expected })), "isolation admission");
            var clock = Stopwatch.StartNew();
            while (!events.Any(e => e.OperationId == id && e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 15000, "isolation deadline"); await Task.Delay(20); }
            Require(await runner.DrainAsync(), "isolation drain");
            var terminal = events.Single(e => e.OperationId == id && e.Kind == "exited");
            var category = mode == "next_success" ? "success" : mode == "cancel" ? "cancelled_with_committed_output" : "target_changed";
            var state = mode is "begin" or "commit" ? "FailedBeforeCommit" : "Committed";
            Require(terminal.Category == category && terminal.CommitState == state && terminal.NativeErrorCode is null &&
                terminal.FilesCommitted == (mode == "next_success" ? 2 : 1) && terminal.CleanupComplete, "stale operation/file result: " + mode);
            Require(File.ReadAllText(Path.Combine(root.FullName, "first.txt")) == "independent", "first committed output removed");
            Require(File.ReadAllText(path) == (mode == "next_success" ? "independent" : mode == "cancel" ? "original" : "external"), "second output changed");
            Require(Directory.GetFiles(root.FullName, ".nxpart-*").Length == 0, "isolation partial leak");
            var record = log.Records.Single(r => r.Id == id && r.Name is "archive.worker.completed" or "archive.worker.failed" or "archive.worker.cancelled");
            Require(Equals(record.Fields["category"], category) && Equals(record.Fields["commit_state"], state) &&
                Equals(record.Fields["files_committed"], terminal.FilesCommitted), "isolation diagnostic mismatch");
        }
    });
}
