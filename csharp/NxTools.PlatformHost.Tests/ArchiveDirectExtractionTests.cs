using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// These fixtures own only GUID-named temporary output and exercise the real Windows output session.
internal static class ArchiveDirectExtractionTests
{
    private static void Check(bool condition, string why) { if (!condition) throw new InvalidOperationException(why); }
    private static string ReadVisible(string path)
    {
        using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete);
        using var reader = new StreamReader(stream);
        return reader.ReadToEnd();
    }

    internal static Task DirectFilesStayVisibleAndClosedFilesSurvive()
    {
        var root = Directory.CreateTempSubdirectory("archive-direct-");
        try
        {
            using var output = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var nativeOperations = new List<string>();
            OutputNative.OperationProbe = nativeOperations.Add;
            try
            {
            for (var index = 0; index < 3; index++)
            {
                var name = $"nested/file-{index}.txt";
                var path = Path.Combine(root.FullName, "nested", $"file-{index}.txt");
                var token = output.BeginExtractedFile(name);
                Check(File.Exists(path) && Directory.GetFiles(root.FullName, ".nxpart-*", SearchOption.AllDirectories).Length == 0,
                    "final name must exist from begin; no temporary siblings");
                output.Write(token, Encoding.UTF8.GetBytes("part"));
                Check(ReadVisible(path) == "part", "bytes must be visible during write without commit");
                output.Write(token, Encoding.UTF8.GetBytes(index.ToString()));
                Check(output.Commit(token).Category == "committed", "direct close result");
                Check(output.Cancel(token, cleanupNewOutput: true).Category == "retained" && File.ReadAllText(path) == "part" + index,
                    "completed direct output survives optional cleanup");
            }
                Check(nativeOperations.Count(op => op == "open_file") == 3 &&
                    nativeOperations.Count(op => op == "identity") == 4 &&
                    !nativeOperations.Any(op => op is "flush" or "rename" or "verify_directory"),
                    "ordinary files require one native create/inspect each, no flush, rename or extra identity");
            }
            finally { OutputNative.OperationProbe = null; }
            var current = output.BeginExtractedFile("nested/incomplete.txt");
            output.Write(current, Encoding.UTF8.GetBytes("incomplete"));
            Check(output.Cancel(current).Category == "partial_removed", "abort removes current file");
            Check(!File.Exists(Path.Combine(root.FullName, "nested", "incomplete.txt")), "abort left current output");
            Check(Directory.GetFiles(Path.Combine(root.FullName, "nested")).Length == 3, "abort deleted completed files");
            var summary = output.CommitSummary();
            Check(summary.TotalRecords == 4 && output.CommitJournal().Count(f => f.Published == true) == 3,
                "direct files and abort must retain truthful journal counts");
            Check(output.CommitJournal().All(f => f.NativeIdentity is null), "direct journal should not inspect a file ID");
            Check(!output.ReadObservation.Snapshot().Any(c => c.Stage is "OutputFingerprint" or "OutputPostpublication"),
                "direct output must not hash or post-open files");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }

    internal static Task MetadataAndPreferredLocation()
    {
        var root = Directory.CreateTempSubdirectory("archive-direct-");
        try
        {
            using var output = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var stamp = DateTime.UtcNow.AddDays(-2).ToFileTimeUtc();
            var token = output.BeginExtractedFile("single.txt");
            output.Write(token, Encoding.UTF8.GetBytes("data"));
            output.BindZoneIdentifier(Encoding.UTF8.GetBytes("[ZoneTransfer]\r\nZoneId=3\r\n"));
            output.SetMetadata(token, new ArchiveOutputMetadata(stamp, ReadOnly: true, Hidden: true));
            Check(output.Commit(token).Category == "committed", "direct metadata commit");
            var path = Path.Combine(root.FullName, "single.txt");
            Check(File.ReadAllText(path) == "data" && File.GetLastWriteTimeUtc(path).ToFileTimeUtc() == stamp,
                "direct output bytes or timestamp changed");
            var attrs = File.GetAttributes(path);
            Check(attrs.HasFlag(FileAttributes.ReadOnly) && attrs.HasFlag(FileAttributes.Hidden), "direct file attributes");
            Check(File.ReadAllText(path + ":Zone.Identifier").Contains("ZoneId=3", StringComparison.Ordinal), "direct MOTW");
            var location = output.ResultLocation("single.txt");
            Check(location?.Target == path && location.Action == "select_in_explorer", "preferred direct result location");
            Check(output.Cancel(token, cleanupNewOutput: true).Category == "retained" && File.Exists(path), "completed metadata output removed");
            File.SetAttributes(path, FileAttributes.Normal);
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }

    internal static Task ConflictOverwriteAndFailure()
    {
        var root = Directory.CreateTempSubdirectory("archive-direct-");
        try
        {
            var path = Path.Combine(root.FullName, "existing.txt");
            File.WriteAllText(path, "old and longer than new");
            using var output = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            try { output.BeginExtractedFile("existing.txt"); throw new InvalidOperationException("existing target accepted"); }
            catch (ArchiveOutputException e) { Check(e.Stage == "target_exists" && e.NativeCode is 80 or 183, "native conflict result"); }
            Check(File.ReadAllText(path) == "old and longer than new", "conflict changed original");
            OutputNative.OperationProbe = stage => { if (stage == "identity") throw new ArchiveOutputException("inspect_injected"); };
            try
            {
                try { output.BeginExtractedFile("begin-failed.txt"); throw new InvalidOperationException("inspection unexpectedly succeeded"); }
                catch (ArchiveOutputException e) { Check(e.Stage == "inspect_injected", "inspection failure stage"); }
            }
            finally { OutputNative.OperationProbe = null; }
            Check(!File.Exists(Path.Combine(root.FullName, "begin-failed.txt")), "created file left on inspection failure");
            var overwrite = output.BeginExtractedFile("existing.txt", overwrite: true);
            Check(new FileInfo(path).Length == 0, "overwrite must truncate target directly at begin");
            output.Write(overwrite, Encoding.UTF8.GetBytes("new"));
            Check(output.Commit(overwrite).Category == "committed" && File.ReadAllText(path) == "new", "overwrite retained old tail");
            var cancelledPath = Path.Combine(root.FullName, "cancelled-overwrite.txt");
            File.WriteAllText(cancelledPath, "prior");
            var incomplete = output.BeginExtractedFile("cancelled-overwrite.txt", overwrite: true);
            output.Write(incomplete, Encoding.UTF8.GetBytes("partial"));
            using var cancelled = new CancellationTokenSource(); cancelled.Cancel();
            Check(output.Commit(incomplete, cancelled.Token).Category == "cancelled", "cancel must remove current file");
            Check(!File.Exists(cancelledPath) && File.ReadAllText(path) == "new", "failed current overwrite affected completed file");
            var other = output.BeginExtractedFile("finished.txt");
            output.Write(other, Encoding.UTF8.GetBytes("completed"));
            Check(output.Commit(other).Category == "committed", "failed overwrite poisoned following file");
            var failed = output.BeginExtractedFile("failed.txt");
            output.Write(failed, Encoding.UTF8.GetBytes("broken"));
            // A metadata stage failure is independent of payload CRC but must still delete this file.
            output.NamedStreamsFlagsProbe = (_, _) => 0;
            output.BindZoneIdentifier(Encoding.UTF8.GetBytes("[ZoneTransfer]\r\nZoneId=3\r\n"));
            try { output.Commit(failed); throw new InvalidOperationException("metadata failure accepted"); }
            catch (ArchiveOutputException) { }
            Check(!File.Exists(Path.Combine(root.FullName, "failed.txt")) && File.Exists(Path.Combine(root.FullName, "finished.txt")),
                "failed current file cleanup affected completed output");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }

    internal static Task RefusesUnsafeAndReparse()
    {
        var root = Directory.CreateTempSubdirectory("archive-direct-");
        var outside = Directory.CreateTempSubdirectory("archive-direct-outside-");
        try
        {
            using var output = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            foreach (var name in new[] { "../escaped.txt", "CON.txt", "a:stream", "sub/../bad", "a\\b", "a//b" })
            {
                try { output.BeginExtractedFile(name); throw new InvalidOperationException("unsafe path accepted"); }
                catch (ArchiveOutputException e) { Check(e.Stage == "invalid_component", "unsafe path stage"); }
            }
            Directory.CreateDirectory(Path.Combine(root.FullName, "directory"));
            try { output.BeginExtractedFile("directory", overwrite: true); throw new InvalidOperationException("directory truncated"); }
            catch (ArchiveOutputException e) { Check(e.Stage is "wrong_kind" or "open_component", "directory kind"); }
            var junction = Path.Combine(root.FullName, "junction");
            using (var mk = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{junction}\" \"{outside.FullName}\"")
            { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!)
            { mk.StandardOutput.ReadToEnd(); mk.StandardError.ReadToEnd(); Check(mk.WaitForExit(5000) && mk.ExitCode == 0, "junction fixture"); }
            try { output.BeginExtractedFile("junction/escaped.txt"); throw new InvalidOperationException("junction followed"); }
            catch (ArchiveOutputException e) { Check(e.Stage == "reparse_component", "junction stage"); }
            Check(!File.Exists(Path.Combine(outside.FullName, "escaped.txt")), "escaped into foreign directory");
            Directory.Delete(junction);
        }
        finally { root.Delete(true); outside.Delete(true); }
        return Task.CompletedTask;
    }
}
