using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Reviewed effects: GUID-owned tiny trees/one owned junction/native file calls only.
internal static class ArchiveOverwriteTests
{
    private static void Require(bool value, string detail) { if (!value) throw new InvalidOperationException(detail); }
    private static Guid Prepare(ArchiveOutputSession session, string relative = "target.txt")
    {
        var confirmed = session.CaptureTargetIdentity(relative);
        Require(confirmed.StartsWith("v2:", StringComparison.Ordinal) && confirmed.Split(':')[2].Length == 32, "full128-bit identity snapshot");
        var token = session.BeginFile(relative, confirmed, overwrite: true);
        session.Write(token, Encoding.UTF8.GetBytes("verified-complete"));
        return token;
    }
    private static ArchiveOutputException Failure(Action action)
    {
        try { action(); } catch (ArchiveOutputException failure) { return failure; }
        throw new InvalidOperationException("expected native output failure");
    }
    private static void NoPartial(string root) => Require(Directory.GetFiles(root, ".nxpart-*", SearchOption.AllDirectories).Length == 0, "owned partial leaked");
    public static Task ChangesBeforeRecheck()
    {
        var owned = Directory.CreateTempSubdirectory("archive-overwrite-changes-");
        try {
            foreach (var change in new[] { "replace", "same_size_same_time", "missing", "directory", "reparse" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var path = Path.Combine(root.FullName, "target.txt"); File.WriteAllText(path, "original");
                using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid()); var token = Prepare(session);
                var timestamp = File.GetLastWriteTimeUtc(path);
                var expected = "target_changed";
                if (change == "replace") { File.Move(path, path + ".old"); File.WriteAllText(path, "external"); }
                if (change == "same_size_same_time") { File.WriteAllText(path, "external"); File.SetLastWriteTimeUtc(path, timestamp); }
                if (change == "missing") { File.Delete(path); expected = "target_missing"; }
                if (change == "directory") { File.Delete(path); Directory.CreateDirectory(path); expected = "target_directory"; }
                if (change == "reparse") {
                    File.Delete(path); expected = "target_reparse";
                    var outside = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                    File.WriteAllText(Path.Combine(outside.FullName, "sentinel"), "outside");
                    using var command = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("cmd.exe",
                        $"/d /c mklink /J \"{path}\" \"{outside.FullName}\"") { UseShellExecute = false, CreateNoWindow = true,
                        RedirectStandardOutput = true, RedirectStandardError = true })!;
                    command.StandardOutput.ReadToEnd(); command.StandardError.ReadToEnd();
                    Require(command.WaitForExit(5000) && command.ExitCode == 0, "owned junction creation");
                }
                var failure = Failure(() => session.Commit(token));
                Require(failure.Stage == expected && failure.State == ArchiveCommitState.FailedBeforeCommit, "recheck category: " + change + "/" + failure.Stage);
                Require(session.Cancel(token).Category == "partial_removed", "changed target exact cleanup"); NoPartial(root.FullName);
                if (change is "replace" or "same_size_same_time") Require(File.ReadAllText(path) == "external", "changed target overwritten");
                if (change == "missing") Require(!File.Exists(path), "missing target recreated");
                if (change is "directory" or "reparse") Require(Directory.Exists(path), "directory/reparse changed");
                if (change == "reparse") Directory.Delete(path);
            }
            var absentRoot = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
            using var absentSession = ArchiveOutputSession.Open(absentRoot.FullName, Guid.NewGuid());
            var absentToken = absentSession.BeginFile("new.txt"); absentSession.Write(absentToken, "complete"u8.ToArray());
            File.WriteAllText(Path.Combine(absentRoot.FullName, "new.txt"), "appeared");
            var appeared = absentSession.Commit(absentToken);
            Require(appeared.Category == "target_exists" && appeared.State == ArchiveCommitState.FailedBeforeCommit, "absent target appeared policy");
            absentSession.Cancel(absentToken); NoPartial(absentRoot.FullName);
            Require(File.ReadAllText(Path.Combine(absentRoot.FullName, "new.txt")) == "appeared", "appeared target clobbered");
        }
        finally { owned.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task FailuresAndNativeOccupation()
    {
        var owned = Directory.CreateTempSubdirectory("archive-overwrite-failure-");
        try {
            foreach (var mode in new[] { "write", "flush", "native_busy", "recheck_busy", "unsupported", "denied", "invalid_parameter", "unknown" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var path = Path.Combine(root.FullName, "target.txt"); File.WriteAllText(path, "original");
                using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid()); var token = Prepare(session);
                IDisposable? occupied = null; var calls = 0;
                try {
                    if (mode == "write") {
                        var failure = Failure(() => session.Write(token, new byte[65537]));
                        Require(failure.Stage == "write_limit", "bounded write failure");
                    } else {
                        session.CommitCheckpointProbe = stage => {
                            if (mode == "flush" && stage == "before_flush") throw new ArchiveOutputException("flush_file", 112);
                            if (mode == "native_busy" && stage == "target_rechecked") occupied = OutputNative.Open(null, @"\??\" + path, directory: false);
                            if (stage == "commit_in_flight") {
                                calls++;
                                if (mode == "unsupported") throw new ArchiveOutputException("replace_file", 50);
                                if (mode == "denied") throw new ArchiveOutputException("replace_file", 5);
                                if (mode == "invalid_parameter") throw new ArchiveOutputException("replace_file", 87);
                                if (mode == "unknown") throw new ArchiveOutputException("replace_file", 64);
                            }
                        };
                        if (mode == "recheck_busy") occupied = new FileStream(path, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
                        if (mode is "unsupported" or "unknown") {
                            var result = session.Commit(token);
                            Require(result.Category == (mode == "unsupported" ? "overwrite_not_supported" : "commit_outcome_unknown"), "native capability/unknown category");
                            Require(result.NativeCode == (mode == "unsupported" ? 50 : 64) && calls == 1, "native code or automatic retry/fallback");
                            if (mode == "unknown") Require(session.Cancel(token).Category == "retained_uncertain", "unknown outcome partial deleted");
                            if (mode == "unknown") Require(Failure(() => session.Write(token, "late"u8.ToArray())).Stage == "write_limit", "uncertain output remained writable");
                        } else {
                            ArchiveOutputException failure;
                            try { failure = Failure(() => session.Commit(token)); }
                            catch (InvalidOperationException) { throw new InvalidOperationException("expected native output failure: " + mode); }
                            var expected = mode == "flush" ? "flush_file" : mode == "recheck_busy" ? "target_busy" : "replace_file";
                            Require(failure.Stage == expected && failure.State == ArchiveCommitState.FailedBeforeCommit, "native failure category: " + mode + "/" + failure.Stage);
                            var code = mode == "flush" ? 112 : mode == "denied" ? 5 : mode == "invalid_parameter" ? 87 : 32;
                            Require(failure.NativeCode == code, "real native stage/code changed");
                            Require(calls == (mode is "native_busy" or "denied" or "invalid_parameter" ? 1 : 0), "failure retried/fell back");
                        }
                    }
                }
                finally { occupied?.Dispose(); }
                var actualBytes = File.ReadAllText(path);
                Require(actualBytes == "original", "precommit failure changed original: " + mode);
                if (mode != "unknown") { session.Cancel(token); NoPartial(root.FullName); }
                else {
                    session.Dispose();
                    // Uncertain evidence is deliberately retained; remove only this fixture's
                    // known uncommitted evidence after proving production cleanup retained it.
                    Require(Directory.GetFiles(root.FullName, ".nxpart-*").Length == 1, "unknown evidence not retained");
                }
            }
        }
        finally { owned.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task PostCommitAndCancellation()
    {
        var owned = Directory.CreateTempSubdirectory("archive-overwrite-post-");
        try {
            foreach (var mode in new[] { "cancel_before", "cancel_ready", "cancel_during", "cancel_after", "post_error", "post_replace", "accepted_toctou" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var path = Path.Combine(root.FullName, "target.txt"); File.WriteAllText(path, "original");
                using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid()); var token = Prepare(session);
                using var cancellation = new CancellationTokenSource(); var calls = 0;
                if (mode == "cancel_before") cancellation.Cancel();
                session.CommitCheckpointProbe = stage => {
                    if (stage == "target_rechecked") {
                        if (mode == "cancel_ready") cancellation.Cancel();
                        if (mode == "accepted_toctou") { File.Delete(path); File.WriteAllText(path, "racing-target"); }
                        // Only target handle was released: ancestors and exclusive partial stay live.
                        try { Directory.Move(root.FullName, root.FullName + ".moved"); throw new InvalidOperationException("parent lease released"); }
                        catch (IOException) { }
                        var partial = Directory.GetFiles(root.FullName, ".nxpart-*").Single();
                        try { File.Delete(partial); throw new InvalidOperationException("temporary protection released"); } catch (IOException) { }
                    }
                    if (stage == "commit_in_flight") { calls++; if (mode == "cancel_during") cancellation.Cancel(); }
                    if (stage == "native_committed") {
                        if (mode == "cancel_after") cancellation.Cancel();
                        if (mode == "post_error") throw new ArchiveOutputException("inspect_full_identity", 5);
                        if (mode == "post_replace") { File.Move(path, path + ".published"); File.WriteAllText(path, "external-after-commit"); }
                    }
                };
                var result = session.Commit(token, cancellation.Token);
                if (mode is "cancel_before" or "cancel_ready") {
                    Require(result.Category == "cancelled" && result.State == ArchiveCommitState.FailedBeforeCommit && calls == 0, "precommit cancellation result");
                    Require(File.ReadAllText(path) == "original", "precommit cancel replaced original");
                } else {
                    var expected = mode == "post_error" ? ArchiveCommitState.CommittedButVerificationFailed : mode == "post_replace" ? ArchiveCommitState.TargetChangedAfterCommit : ArchiveCommitState.Committed;
                    Require(result.State == expected && calls == 1, "postcommit state/call count: " + mode + "/" + result.State);
                    Require(File.ReadAllText(path) == (mode == "post_replace" ? "external-after-commit" : "verified-complete"), "postcommit content");
                    var cleanup = session.Cancel(token, cleanupNewOutput: true);
                    Require(cleanup.Category == (mode is "post_error" or "post_replace" ? "retained_published" : "retained_replacement"), "published handle treated as partial");
                    if (mode == "post_replace") Require(File.ReadAllText(path + ".published") == "verified-complete", "actual published bytes removed");
                    Require(Failure(() => session.Commit(token)).Stage == "invalid_state" && calls == 1, "postcommit retry allowed");
                }
                NoPartial(root.FullName);
            }
        }
        finally { owned.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task ExclusivePartialOwnership()
    {
        var root = Directory.CreateTempSubdirectory("archive-overwrite-ownership-");
        try {
            var user = Path.Combine(root.FullName, ".nxpart-user-owned"); File.WriteAllText(user, "user");
            using var first = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            using var second = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var one = first.BeginFile("one.txt"); first.Write(one, "one"u8.ToArray());
            var two = second.BeginFile("two.txt"); second.Write(two, "two"u8.ToArray());
            first.Cancel(one);
            Require(Directory.GetFiles(root.FullName, ".nxpart-*").Length == 2 && File.ReadAllText(user) == "user", "cancel removed other task/user partial");
            Require(second.Commit(two).State == ArchiveCommitState.Committed, "other task commit failed");
            first.Dispose(); Require(File.ReadAllText(Path.Combine(root.FullName, "two.txt")) == "two" && File.ReadAllText(user) == "user", "dispose removed published/user data");
            // CREATE_NEW proof at the real NT boundary, not just a random-name assertion.
            using var parent = OutputNative.Open(null, @"\??\" + root.FullName, directory: true);
            var collision = Failure(() => { using var unexpected = OutputNative.Open(parent, ".nxpart-user-owned", directory: false, exclusive: true); });
            Require(collision.NativeCode is 80 or 183 && File.ReadAllText(user) == "user", "existing temporary opened/truncated");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }
    public static async Task ActualWorkerOverwriteLifecycle()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var fixtures = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_CODEC_FIXTURES");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker) || string.IsNullOrEmpty(fixtures))
            throw new TestSkippedException("Explicit built worker and known independent7z fixture required.");
        var helper = Path.Combine(AppContext.BaseDirectory, "NxTools.ArchiveWorker.exe");
        Require(!File.Exists(helper), "fixture refuses existing helper");
        var owned = Directory.CreateTempSubdirectory("archive-overwrite-chain-"); var copied = false;
        try {
            File.Copy(worker, helper, false); copied = true;
            foreach (var mode in new[] { "normal", "crc", "cancel_ready", "cancel_during", "cancel_after", "post_error", "unsupported", "unknown" }) {
                var root = Directory.CreateDirectory(Path.Combine(owned.FullName, Guid.NewGuid().ToString("N")));
                var path = Path.Combine(root.FullName, "file.txt"); File.WriteAllText(path, "original");
                string expected; using (var capture = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid())) expected = capture.CaptureTargetIdentity("file.txt");
                var source = Path.Combine(fixtures, "lzma.7z");
                if (mode == "crc") {
                    source = Path.Combine(owned.FullName, "damaged.7z"); var bytes = File.ReadAllBytes(Path.Combine(fixtures, "lzma.7z"));
                    bytes[40] ^= 0x80; File.WriteAllBytes(source, bytes);
                }
                var events = new System.Collections.Concurrent.ConcurrentQueue<ArchiveWorkerEvent>();
                using var runner = ArchiveWorkerRunner.ForPackage(events.Enqueue);
                var id = Guid.NewGuid(); var calls = 0;
                runner.OutputCommitCheckpointProbe = stage => {
                    if (stage == "commit_in_flight") {
                        calls++;
                        if (mode == "unsupported") throw new ArchiveOutputException("replace_file", 50);
                        if (mode == "unknown") throw new ArchiveOutputException("replace_file", 64);
                    }
                    if (mode == "cancel_ready" && stage == "target_rechecked" || mode == "cancel_during" && stage == "commit_in_flight" || mode == "cancel_after" && stage == "native_committed")
                        Require(runner.Cancel(id), "actual native commit cancellation");
                    if (stage == "native_committed") {
                        var publication = runner.InspectOutputCommit(id);
                        Require(publication.FilesCommitted == 1 && publication.State == "Committed", "publication fact delayed until postcheck returns");
                    }
                    if (mode == "post_error" && stage == "native_committed") throw new ArchiveOutputException("inspect_full_identity", 5);
                };
                Require(runner.Start(new(id, "extract", source, 0, new { max_entries = 100 }, OutputRoot: root.FullName,
                    ConfirmedOverwrites: new Dictionary<string, string> { ["file.txt"] = expected })), "actual overwrite admission");
                var clock = System.Diagnostics.Stopwatch.StartNew();
                while (!events.Any(e => e.Kind == "exited")) { Require(clock.ElapsedMilliseconds < 15000, "actual overwrite deadline"); await Task.Delay(20); }
                Require(await runner.DrainAsync(), "actual overwrite drain");
                var terminal = events.Last(); Require(terminal.CleanupComplete, "actual overwrite cleanup incomplete");
                var published = mode is "normal" or "cancel_during" or "cancel_after" or "post_error";
                Require(terminal.FilesCommitted == (published ? 1 : 0), "actual publication count lost: " + mode);
                Require(File.ReadAllText(path) == (published ? "this is a file\n" : "original"), "actual overwrite bytes: " + mode);
                Require(calls == (mode is "crc" or "cancel_ready" ? 0 : 1), "actual commit before codec integrity/cancel");
                var category = mode == "normal" ? "success" : mode == "crc" ? "worker_failure" : mode == "post_error" ? "committed_but_verification_failed" :
                    mode == "unsupported" ? "overwrite_not_supported" : mode == "unknown" ? "commit_outcome_unknown" : published ? "cancelled_with_committed_output" : "cancelled";
                Require(terminal.Category == category, "actual overwrite terminal: " + mode + "/" + terminal.Category);
                var state = mode == "crc" ? null : mode is "cancel_ready" or "unsupported" ? "FailedBeforeCommit" : mode == "unknown" ? "CommitOutcomeUnknown" : mode == "post_error" ? "CommittedButVerificationFailed" : "Committed";
                var native = mode == "post_error" ? 5 : mode == "unsupported" ? 50 : mode == "unknown" ? 64 : (int?)null;
                Require(terminal.CommitState == state && terminal.NativeErrorCode == native, "actual state/native code lost: " + mode + "/" + terminal.CommitState + "/" + terminal.NativeErrorCode);
                if (mode == "unknown") Require(Directory.GetFiles(root.FullName, ".nxpart-*").Length == 1, "unknown actual evidence was deleted");
                else NoPartial(root.FullName);
            }
        }
        finally { if (copied) File.Delete(helper); owned.Delete(true); }
    }
}
