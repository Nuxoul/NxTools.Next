using System.Text;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

internal static class ArchiveOutputTests
{
    private static void Assert(bool value, string why) { if (!value) throw new InvalidOperationException(why); }
    public static Task DiagnosticAndNativeFailure()
    {
        var root = Directory.CreateTempSubdirectory("archive-output-");
        try
        {
            var sentinel = "SECRET_PATH_" + Guid.NewGuid().ToString("N");
            var records = new List<string>();
            var id = Guid.NewGuid();
            using var session = ArchiveOutputSession.Open(root.FullName, id, new FailingLog(records));
            var token = session.BeginFile("secret-" + sentinel + ".txt");
            Assert(session.Write(token, Encoding.UTF8.GetBytes(sentinel)), "log write");
            Assert(session.Commit(token).Category == "committed", "log failed operation");
            var rejected = session.BeginFile("second.txt");
            Assert(session.Write(rejected, Encoding.UTF8.GetBytes("pending")), "failure write");
            File.WriteAllText(Path.Combine(root.FullName, "second.txt"), "old");
            var result = session.Commit(rejected);
            Assert(result.Category == "target_exists" && result.NativeCode is 80 or 183, "fixed native collision code");
            session.Cancel(rejected);
            Assert(File.ReadAllText(Path.Combine(root.FullName, "second.txt")) == "old", "failure clobbered original");
            Assert(records.Any(r => r.Contains(id.ToString(), StringComparison.Ordinal)) &&
                records.All(r => !r.Contains(sentinel, StringComparison.Ordinal) && !r.Contains(root.FullName, StringComparison.Ordinal)),
                "diagnostic leaked payload or lost correlation");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }
    private sealed class FailingLog(List<string> records) : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        { records.Add(eventName + correlationId + System.Text.Json.JsonSerializer.Serialize(fields)); throw new IOException("unavailable"); }
    }
    public static Task NestedAndUnsafe()
    {
        var root = Directory.CreateTempSubdirectory("archive-output-");
        try
        {
            using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var token = session.BeginFile("中文/日本語/hello.txt");
            Assert(session.Write(token, Encoding.UTF8.GetBytes("payload")), "write");
            Assert(session.Commit(token).Category == "committed", "commit");
            Assert(File.ReadAllText(Path.Combine(root.FullName, "中文", "日本語", "hello.txt")) == "payload", "byte compare");
            session.EnsureDirectory("中文/空目录");
            Assert(Directory.Exists(Path.Combine(root.FullName, "中文", "空目录")), "empty directory create");
            var nfd=session.BeginFile("e\u0301.txt");session.Write(nfd,Encoding.UTF8.GetBytes("preserved NFD"));Assert(session.Commit(nfd).Category=="committed","native must not force Unicode normalization");
            Assert(File.ReadAllText(Path.Combine(root.FullName,"e\u0301.txt"))=="preserved NFD","NFD name was altered");
            foreach (var name in new[] { "../bad", "CON.txt", "COM¹.txt", "a:stream", "a./b", "a\\b", "a//b", "/absolute", "a/../b" })
            {
                try { session.BeginFile(name); throw new InvalidOperationException("accepted unsafe component"); }
                catch (ArchiveOutputException e) { Assert(e.Stage == "invalid_component", "invalid stage"); }
            }
            foreach (var path in new[] { @"\\server\share\target", @"\\?\C:\target", @"C:\a\..\target", @"C:\a:stream" })
            {
                try { using var rejected = ArchiveOutputSession.Open(path, Guid.NewGuid()); throw new InvalidOperationException("unsafe root"); }
                catch (ArchiveOutputException) { }
            }
            try { session.EnsureDirectory("../unsafe"); throw new InvalidOperationException("unsafe directory"); }
            catch (ArchiveOutputException e) { Assert(e.Stage == "invalid_component", "directory stage"); }
            try { session.BeginFile("hello.txt", overwrite: true); throw new InvalidOperationException("unsafe overwrite"); }
            catch (ArchiveOutputException e) { Assert(e.Stage == "expected_identity_required", "overwrite requires identity"); }
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task PreserveExistingAndCancel()
    {
        var root = Directory.CreateTempSubdirectory("archive-output-");
        try
        {
            var original = Path.Combine(root.FullName, "target.bin");
            File.WriteAllText(original, "original");
            File.WriteAllText(Path.Combine(root.FullName, "target.bin.nxpart"), "user-owned");
            using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var partial = session.BeginFile("target.bin");
            Assert(session.Write(partial, Encoding.UTF8.GetBytes("new")), "write");
            Assert(session.Commit(partial).Category == "target_exists", "must never clobber existing target");
            Assert(File.ReadAllText(original) == "original", "original clobbered");
            session.Cancel(partial);
            Assert(File.ReadAllText(Path.Combine(root.FullName, "target.bin.nxpart")) == "user-owned", "user part removed");
            Assert(Directory.GetFiles(root.FullName).Length == 2, "partial not cleaned");
            var newToken = session.BeginFile("new.bin");
            Assert(session.Write(newToken, Encoding.UTF8.GetBytes("ours")), "write");
            Assert(session.Commit(newToken).Category == "committed", "commit new");
            session.Cancel(newToken, cleanupNewOutput: true);
            Assert(!File.Exists(Path.Combine(root.FullName, "new.bin")), "unchanged new output not cleaned");
            var edited = session.BeginFile("edited.bin");
            Assert(session.Write(edited, Encoding.UTF8.GetBytes("ours")), "write");
            Assert(session.Commit(edited).Category == "committed", "commit edited");
            File.WriteAllText(Path.Combine(root.FullName, "edited.bin"), "external");
            session.Cancel(edited, cleanupNewOutput: true);
            Assert(File.ReadAllText(Path.Combine(root.FullName, "edited.bin")) == "external", "external edit removed");
            var sameSize = session.BeginFile("same-size.bin");
            Assert(session.Write(sameSize, Encoding.UTF8.GetBytes("ours")), "same size write");
            Assert(session.Commit(sameSize).Category == "committed", "same size commit");
            var samePath = Path.Combine(root.FullName, "same-size.bin");
            var originalTime = File.GetLastWriteTimeUtc(samePath);
            File.WriteAllText(samePath, "edit");
            File.SetLastWriteTimeUtc(samePath, originalTime);
            Assert(session.Cancel(sameSize, cleanupNewOutput: true).Category == "changed_retained", "same-size external edit removed");
            Assert(File.ReadAllText(samePath) == "edit", "same-size content changed");
            var untouched = session.BeginFile("failed.bin");
            Assert(session.Write(untouched, Encoding.UTF8.GetBytes("partial")), "partial write");
            session.Cancel(untouched);
            Assert(!File.Exists(Path.Combine(root.FullName, "failed.bin")), "cancel published partial");
            Assert(Directory.GetFiles(root.FullName).Length == 4, "cancel left partial");
            Assert(File.ReadAllText(original) == "original", "cancel changed preexisting target");
            var swapped = session.BeginFile("swapped.bin");
            Assert(session.Write(swapped, Encoding.UTF8.GetBytes("owned")), "swapped write");
            Assert(session.Commit(swapped).Category == "committed", "swapped commit");
            var swappedPath = Path.Combine(root.FullName, "swapped.bin");
            File.Delete(swappedPath);
            File.WriteAllText(swappedPath, "external replacement");
            Assert(session.Cancel(swapped, cleanupNewOutput: true).Category == "changed_retained", "replacement removed");
            Assert(File.ReadAllText(swappedPath) == "external replacement", "replacement content");
        }
        finally { root.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task ConcurrentTargetAndReparse()
    {
        var root = Directory.CreateTempSubdirectory("archive-output-");
        var outside = Directory.CreateTempSubdirectory("archive-outside-");
        try
        {
            var rootJunction = Path.Combine(root.FullName, "redirect-root");
            using (var mk = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{rootJunction}\" \"{outside.FullName}\"")
            { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!)
            { mk.StandardOutput.ReadToEnd(); mk.StandardError.ReadToEnd(); Assert(mk.WaitForExit(5000) && mk.ExitCode == 0, "root junction fixture failed"); }
            try { using var redirected = ArchiveOutputSession.Open(rootJunction, Guid.NewGuid()); throw new InvalidOperationException("root followed junction"); }
            catch (ArchiveOutputException e) { Assert(e.Stage == "reparse_component", "root junction stage"); }
            Directory.Delete(rootJunction);
            using var session = ArchiveOutputSession.Open(root.FullName, Guid.NewGuid());
            var token = session.BeginFile("target");
            Assert(session.Write(token, Encoding.UTF8.GetBytes("owned")), "write");
            File.WriteAllText(Path.Combine(root.FullName, "target"), "competitor");
            Assert(session.Commit(token).Category == "target_exists", "concurrent target overwritten");
            Assert(File.ReadAllText(Path.Combine(root.FullName, "target")) == "competitor", "competitor clobbered");
            session.Cancel(token);
            // Windows directory junction, confined to temporary owned trees only.
            var junction = Path.Combine(root.FullName, "junction");
            using (var mk = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("cmd.exe", $"/d /c mklink /J \"{junction}\" \"{outside.FullName}\"")
            { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true })!)
            { mk.StandardOutput.ReadToEnd(); mk.StandardError.ReadToEnd(); Assert(mk.WaitForExit(5000) && mk.ExitCode == 0, "junction fixture failed"); }
            try { session.BeginFile("junction/escape.txt"); throw new InvalidOperationException("followed junction"); }
            catch (ArchiveOutputException e) { Assert(e.Stage == "reparse_component", "junction stage"); }
            Assert(!File.Exists(Path.Combine(outside.FullName, "escape.txt")), "escaped");
            Directory.Delete(junction);
            var parent = Path.Combine(root.FullName, "held");
            Directory.CreateDirectory(parent);
            var held = session.BeginFile("held/inside.txt");
            Assert(session.Write(held, Encoding.UTF8.GetBytes("inside")), "held write");
            var moved = Path.Combine(root.FullName, "moved");
            try { Directory.Move(parent, moved); throw new InvalidOperationException("held parent moved"); }
            catch (IOException) { /* Held directory disallows FILE_SHARE_DELETE. */ }
            Assert(session.Commit(held).Category == "committed", "held commit");
            Assert(!File.Exists(Path.Combine(outside.FullName, "inside.txt")), "held parent escaped");
            var rootMoved = Path.Combine(Path.GetDirectoryName(root.FullName)!, "archive-swapped-" + Guid.NewGuid().ToString("N"));
            try { Directory.Move(root.FullName, rootMoved); throw new InvalidOperationException("held root moved"); }
            catch (IOException) { /* Root ancestor remains leased. */ }
        }
        finally { root.Delete(true); outside.Delete(true); }
        return Task.CompletedTask;
    }
    public static Task RecheckedAtomicPathOverwrite()
    {
        var root=Directory.CreateTempSubdirectory("archive-overwrite-");
        try {
            var path=Path.Combine(root.FullName,"target.txt");File.WriteAllText(path,"original");
            using var session=ArchiveOutputSession.Open(root.FullName,Guid.NewGuid());
            var identity=session.CaptureTargetIdentity("target.txt");
            var token=session.BeginFile("target.txt",identity,overwrite:true);
            session.Write(token,Encoding.UTF8.GetBytes("replacement"));
            Assert(session.Commit(token).Category=="committed","rechecked atomic path replace");
            Assert(File.ReadAllText(path)=="replacement","replace bytes");
            Assert(Directory.GetFiles(root.FullName,".nxpart-*").Length==0,"published temporary leaked");
            Assert(session.Cancel(token,cleanupNewOutput:true).Category=="retained_replacement","cancel cannot delete replaced existing output");
            Assert(File.ReadAllText(path)=="replacement","replacement removed");
            var old=session.CaptureTargetIdentity("target.txt");
            File.Delete(path);File.WriteAllText(path,"external");
            try {session.BeginFile("target.txt",old,overwrite:true);throw new InvalidOperationException("accepted stale identity");}
            catch(ArchiveOutputException error){Assert(error.Stage=="target_changed","stale identity category");}
            Assert(File.ReadAllText(path)=="external","external target clobbered");
            var current=session.CaptureTargetIdentity("target.txt");
            var cancelled=session.BeginFile("target.txt",current,overwrite:true);session.Write(cancelled,Encoding.UTF8.GetBytes("partial"));session.Cancel(cancelled);
            Assert(File.ReadAllText(path)=="external","cancel changed original");
        } finally {root.Delete(true);} return Task.CompletedTask;
    }
}
