using System.Security;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Synthetic XML, identity snapshots and owned temporary files only; never launches DOpusRT/live Shell.
internal static class DirectoryOpusTargetObservationTests
{
    private static readonly IntPtr Root = new(10);
    private static readonly IntPtr Tab = new(20);

    public static Task ExactActiveSelectionFeedsSharedReveal()
    {
        WithTarget((directory, file) =>
        {
            var reader = new Platform(directory, file);
            var shell = new Shell(reader);
            var stages = new List<string>();
            var result = ExplorerShellActivation.LaunchAndActivate(file, shell, stages.Add);
            Assert(result.Success && result.Data == "target_confirmed_foreground_not_confirmed",
                "replacement-host exact active selection must succeed through shared Reveal even without foreground permission");
            Assert(reader.Commands.SequenceEqual(new[] { "paths", "listsel,0x14", "paths" }),
                "read-only exact-tab selection plus fresh paths revalidation; never mutation commands or active-tab zero");
            Assert(shell.Activations == 1 && shell.Fallbacks == 0 && shell.Waits == 0,
                "actual replacement host target proof prevents false failure/fallback without a second activation owner");
            reader = new Platform(directory, file) { Selection = Selection(directory, file.ToUpperInvariant()) };
            Assert(Observe(file, reader) == Root, "filesystem matching retains exact case-insensitive identity");
            reader = new Platform(directory, file) { Selection = Selection(directory, directory) };
            Assert(Observe(directory, reader, true) == IntPtr.Zero, "entering a directory is not selecting it in its parent");
            var parent = Path.GetDirectoryName(directory)!;
            reader = new Platform(parent, directory);
            Assert(Observe(directory, reader, true) == Root, "directory Reveal selects the exact directory in its parent");
        });
        return Task.CompletedTask;
    }

    public static Task WrongInactiveStaleAndForeignEvidenceRejects()
    {
        WithTarget((directory, file) =>
        {
            var cases = new Platform[]
            {
                new(directory, file) { Paths = Paths(directory, active: false) },
                new(directory, file) { Paths = Paths(directory, state: "2") },
                new(directory, file) { Paths = Paths(directory + "-other") },
                new(directory, file) { Selection = Selection(directory, file + ".wrong") },
                new(directory, file) { Selection = Selection(directory, file, tab: "0x15") },
                new(directory, file) { Selection = Selection(directory, file, root: "0xb") },
                new(directory, file) { Selection = Selection(directory + "-other", file) },
                new(directory, file) { Available = false },
                new(directory, file) { AvailableCalls = 1 },
                new(directory, file) { RecheckedPaths = Paths(directory + "-changed") },
                new(directory, file) { RecheckedPaths = Paths(directory, active: false) },
                new(directory, file) { Paths = Paths(directory) .Replace("</results>", Paths(directory).Split('>', 2)[1]) },
            };
            foreach (var candidate in cases)
                Assert(Observe(file, candidate) == IntPtr.Zero, "wrong/inactive/hidden/foreign/stale or duplicate tab cannot authorize target success");
            var empty = new Platform(directory, file) { Selection = Selection(directory, null) };
            Assert(Observe(file, empty) == IntPtr.Zero, "directory evidence without selection cannot confirm file");
        });
        return Task.CompletedTask;
    }

    public static Task MalformedBudgetsAndDiagnosticsFailClosed()
    {
        WithTarget((directory, file) =>
        {
            foreach (var malformed in new[]
            {
                "", "<results", "<results command='paths' result='0'/>",
                "<!DOCTYPE results [<!ENTITY x SYSTEM 'file:///private'>]><results command='paths' result='1'>&x;</results>",
                Paths(directory).Replace("0x14", "0xffffffffffffffff"),
                Paths(directory).Replace(Escape(directory), "relative"),
                "<results command='paths' result='1'>" + new string(' ', 2 * 1024 * 1024) + "</results>",
                "<results command='paths' result='1'>" + string.Concat(Enumerable.Repeat("<path/>", 257)) + "</results>",
            })
                Assert(Observe(file, new Platform(directory, file) { Paths = malformed }) == IntPtr.Zero,
                    "malformed/DTD/oversized/overcount/invalid identity fails closed");
            var manyItems = "<results command='listsel,0x14' result='1'><items path='" + Escape(directory) +
                "' lister='0xa' tab='0x14'>" + string.Concat(Enumerable.Repeat("<item path='" + Escape(file) + "'/>", 4097)) + "</items></results>";
            Assert(Observe(file, new Platform(directory, file) { Selection = manyItems }) == IntPtr.Zero, "selection count is bounded before acceptance");
            var stages = new List<string>();
            var reader = new Platform(directory, file) { ThrowExport = true };
            Assert(ExplorerShellActivation.ObserveTarget(file, false, new DirectoryOpusTargetObservation(file, false, reader, stages.Add)) == IntPtr.Zero,
                "helper/export errors preserve truthful failure");
            Assert(stages.Count > 0 && stages.All(stage => !stage.Contains(directory) && !stage.Contains("private")), "fixed metadata-only diagnostic stages");
            Assert(ExplorerShellActivation.ObserveTarget(file, false, new DirectoryOpusTargetObservation(file, false,
                new Platform(directory, file), _ => throw new IOException("private log"))) == Root, "diagnostic sink cannot change successful observation");
        });
        return Task.CompletedTask;
    }

    private static IntPtr Observe(string file, Platform platform, bool isDirectory = false) =>
        ExplorerShellActivation.ObserveTarget(file, isDirectory, new DirectoryOpusTargetObservation(file, isDirectory, platform));

    private static string Escape(string value) => SecurityElement.Escape(value)!;
    private static string Paths(string directory, bool active = true, string state = "1") =>
        "<results command='paths' result='1'><path lister='0xa' tab='0x14' active_tab='" + (active ? "1" : "0") +
        "' tab_state='" + state + "'>" + Escape(directory) + "</path></results>";
    private static string Selection(string directory, string? file, string tab = "0x14", string root = "0xa") =>
        "<results command='listsel,0x14' result='1'><items path='" + Escape(directory) + "' lister='" + root + "' tab='" + tab + "'>" +
        (file is null ? "" : "<item path='" + Escape(file) + "'/>") + "</items></results>";
    private static void WithTarget(Action<string, string> action)
    {
        var directory = Path.Combine(Path.GetTempPath(), "nxtools-dopus-observation-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "synthetic-owned.txt");
        File.WriteAllText(file, "synthetic");
        try { action(directory, file); }
        finally { Directory.Delete(directory, true); }
    }
    private static void Assert(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }

    private sealed class Platform : IDirectoryOpusTargetPlatform
    {
        public string Paths;
        public string Selection;
        public string? RecheckedPaths;
        public bool Available = true;
        public int AvailableCalls = int.MaxValue;
        public bool ThrowExport;
        public List<string> Commands = [];
        public Platform(string directory, string file)
        {
            Paths = DirectoryOpusTargetObservationTests.Paths(directory);
            Selection = DirectoryOpusTargetObservationTests.Selection(directory, file);
        }
        public IReadOnlyList<DirectoryOpusObservationHost> CaptureHosts() => [new(Root, 123, 456, "unused-fixture-helper")];
        public bool IsCurrent(DirectoryOpusObservationHost host, IntPtr tab) =>
            Available && AvailableCalls-- > 0 && host.Root == Root && tab == Tab;
        public string? Export(DirectoryOpusObservationHost host, string command)
        {
            Commands.Add(command);
            if (ThrowExport) throw new IOException("private helper payload");
            return command == "paths" ? Commands.Count > 1 ? RecheckedPaths ?? Paths : Paths : Selection;
        }
    }
    private sealed class Shell(Platform reader) : IExplorerShellPlatform
    {
        public int Activations, Fallbacks, Waits;
        public bool SupportsTargetObservation => true;
        public int OpenNative(string target, bool isDirectory) => 0;
        public IntPtr FindConfirmedTarget(string target, bool isDirectory) => Observe(target, reader, isDirectory);
        public IntPtr FindExplorerWindow(string directory) => IntPtr.Zero;
        public void LaunchExplorer(string arguments) => Fallbacks++;
        public bool IsWindow(IntPtr window) => window == Root;
        public bool IsIconic(IntPtr window) => false;
        public void RestoreWindow(IntPtr window) { }
        public void TryActivateWindow(IntPtr window) => Activations++;
        public IntPtr GetForegroundWindow() => IntPtr.Zero;
        public IntPtr GetRootWindow(IntPtr window) => window;
        public void WaitForNextAttempt() => Waits++;
        public int LastErrorCode => 0;
    }
}
