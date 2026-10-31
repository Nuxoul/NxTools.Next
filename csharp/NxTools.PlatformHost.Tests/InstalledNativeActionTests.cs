using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Synthetic only: no real Explorer, chooser, registry or service operations.
internal static class InstalledNativeActionTests
{
    public static Task ExplorerNativeFirstAndFallback()
    {
        var activationDispatch = typeof(ExplorerShellActivation).GetNestedType(
            "IExplorerBrowserDispatch", System.Reflection.BindingFlags.NonPublic)
            ?? typeof(IExplorerBrowserDispatch);
        Assert(activationDispatch.GUID == typeof(IExplorerBrowserDispatch).GUID,
            "Explorer readback must use the same IWebBrowser2 contract as navigation");
        foreach (var property in new[] { "WindowHandle", "LocationUrl" })
        {
            var expected = typeof(IExplorerBrowserDispatch).GetProperty(property)!;
            var actual = activationDispatch.GetProperty(property)!;
            var expectedId = System.Reflection.CustomAttributeExtensions.GetCustomAttribute<System.Runtime.InteropServices.DispIdAttribute>(expected)!.Value;
            var actualId = System.Reflection.CustomAttributeExtensions.GetCustomAttribute<System.Runtime.InteropServices.DispIdAttribute>(actual)!.Value;
            Assert(actualId == expectedId && actual.PropertyType == expected.PropertyType,
                $"Explorer {property} readback must use released dispatch ID and type");
        }
        WithTargets((directory, file) =>
        {
            var native = new ExplorerFixture();
            var result = ExplorerShellActivation.LaunchAndActivate(file, native);
            Assert(result.Success && result.Data == "foreground_confirmed", "native selection must confirm foreground");
            Assert(native.Calls[0] == "native_select" && !native.Calls.Contains("fallback"), "native selection precedes lookup; no executable on success");
            Assert(native.SelectedTarget == file, "existing file identity must be selected even with a reused window");
            Assert(native.Calls.IndexOf("restore") < native.Calls.IndexOf("activate"), "restore minimized window before activation");

            var fallback = new ExplorerFixture { NativeResult = unchecked((int)0x80004005) };
            result = ExplorerShellActivation.LaunchAndActivate(file, fallback);
            Assert(result.Success, "failed native selection must use executable fallback");
            Assert(fallback.Calls.Take(2).SequenceEqual(["native_select", "fallback"]), "fallback ordering");
            Assert(fallback.Arguments.Single() == $"/select,\"{file}\"", "fallback must preserve file selection identity");

            var folder = new ExplorerFixture();
            Assert(ExplorerShellActivation.LaunchAndActivate(directory, folder).Success, "native folder reveal");
            Assert(folder.Calls[0] == "native_open" && folder.SelectedTarget == directory, "native boundary retains directory type and exact target");
            folder = new ExplorerFixture { NativeResult = unchecked((int)0x80004005) };
            Assert(ExplorerShellActivation.LaunchAndActivate(directory, folder).Success, "folder fallback");
            Assert(folder.Arguments.Single() == $"/select,\"{directory}\"", "directory fallback selects the folder in its parent");
        });
        return Task.CompletedTask;
    }

    public static Task ExplorerReadbackRetryAndPending()
    {
        WithTargets((_, file) =>
        {
            var delayed = new ExplorerFixture { PublishAfter = 3, ForegroundAfter = 2 };
            Assert(ExplorerShellActivation.LaunchAndActivate(file, delayed).Success, "delayed matching child foreground succeeds");
            Assert(delayed.Waits >= 2 && delayed.Activations >= 2, "bounded lookup and activation retry");

            var denied = new ExplorerFixture { ForegroundAfter = int.MaxValue };
            var failed = ExplorerShellActivation.LaunchAndActivate(file, denied);
            Assert(!failed.Success && failed.Data == "foreground_not_confirmed", "API/launch acceptance is not terminal success");
            Assert(denied.Waits == 7 && denied.Arguments.Count == 1, "eight attempts and only one fallback");
            denied.ForegroundAfter = 1;
            Assert(ExplorerShellActivation.LaunchAndActivate(file, denied).Success, "failure must retain the ability to retry");

            var destroyed = new ExplorerFixture { DestroyAfterActivation = true };
            Assert(!ExplorerShellActivation.LaunchAndActivate(file, destroyed).Success, "destroyed candidate cannot authorize a close");

            WindowsCommandResult? terminal = null;
            var pending = new ExplorerFixture { OnReadback = () => Assert(terminal is null, "no terminal result before native readback") };
            terminal = ExplorerShellActivation.LaunchAndActivate(file, pending);
            Assert(terminal.Success && pending.Readbacks == 1, "one terminal result after confirmed readback");
            var missing = new ExplorerFixture();
            Assert(!ExplorerShellActivation.LaunchAndActivate(file + ".missing", missing).Success && missing.Calls.Count == 0,
                "missing targets fail without native/executable operations");
        });
        return Task.CompletedTask;
    }

    public static Task OpenWithCancellationAndFailure()
    {
        WithTargets((directory, file) =>
        {
            var chooser = new OpenWithFixture();
            Assert(OpenWithShellDialog.Show(file, IntPtr.Zero, chooser).Data == "open_with_completed", "chooser success");
            Assert(chooser.Target == file, "chooser receives exact file identity");
            chooser.Result = unchecked((int)0x800704C7);
            var cancelled = OpenWithShellDialog.Show(file, IntPtr.Zero, chooser);
            Assert(!cancelled.Success && cancelled.Error is null && cancelled.Data == "cancelled", "chooser cancellation is not failure or success");
            chooser.Result = 1; // S_FALSE: dismissal, not an execution acknowledgement.
            Assert(OpenWithShellDialog.Show(file, IntPtr.Zero, chooser).Data == "cancelled", "S_FALSE must not close completed window");
            chooser.Result = unchecked((int)0x80070005);
            var failed = OpenWithShellDialog.Show(file, IntPtr.Zero, chooser);
            Assert(!failed.Success && failed.Error is not null && failed.NativeErrorCode == chooser.Result, "native chooser failure code");
            var calls = chooser.Calls;
            Assert(!OpenWithShellDialog.Show(directory, IntPtr.Zero, chooser).Success && chooser.Calls == calls,
                "Open With must not open a directory or fallback parent");
            WindowsCommandResult? terminal = null;
            chooser.Result = 0;
            chooser.DuringShow = () => Assert(terminal is null, "chooser result stays pending until return");
            terminal = OpenWithShellDialog.Show(file, IntPtr.Zero, chooser);
            Assert(terminal.Success, "chooser publishes only terminal result");
            chooser.DuringShow = null;
            Assert(OpenWithShellDialog.Show(file, IntPtr.Zero, chooser, _ => throw new InvalidOperationException("synthetic log failure")).Success,
                "diagnostics must not alter chooser outcome");
        });
        return Task.CompletedTask;
    }

    public static Task ExplorerRelativePidlLifetime()
    {
        var fixture = new PidlFixture();
        Assert(ExplorerNativeFolder.Select("synthetic", fixture) == 0, "selection result");
        Assert(fixture.Freed.SequenceEqual([new IntPtr(200), new IntPtr(100)]), "free parent then absolute, never borrowed child");
        fixture = new PidlFixture { ThrowOnSelect = true };
        try { ExplorerNativeFolder.Select("synthetic", fixture); throw new InvalidOperationException("expected fixture failure"); }
        catch (SyntheticPidlException) { }
        Assert(fixture.Freed.SequenceEqual([new IntPtr(200), new IntPtr(100)]), "exception retains balanced ownership");
        fixture = new PidlFixture { CloneFails = true };
        Assert(ExplorerNativeFolder.Select("synthetic", fixture) < 0 && fixture.Freed.SequenceEqual([new IntPtr(100)]), "partial allocation cleanup");
        return Task.CompletedTask;
    }

    private sealed class SyntheticPidlException : Exception { }
    private sealed class PidlFixture : IExplorerPidlPlatform
    {
        public List<IntPtr> Freed { get; } = [];
        public bool ThrowOnSelect { get; set; }
        public bool CloneFails { get; set; }
        private bool _parentTrimmed;
        public IntPtr CreateAbsolute(string target) => new(100);
        public IntPtr Clone(IntPtr absolute)
        {
            Assert(absolute == new IntPtr(100) && Freed.Count == 0, "clone live absolute identity");
            return CloneFails ? IntPtr.Zero : new IntPtr(200);
        }
        public bool RemoveLast(IntPtr parent) { Assert(parent == new IntPtr(200), "trim only owning parent clone"); _parentTrimmed = true; return true; }
        public IntPtr FindLast(IntPtr absolute) { Assert(absolute == new IntPtr(100), "borrow child from absolute owner"); return new IntPtr(120); }
        public int OpenAndSelect(IntPtr parent, IntPtr relativeChild)
        {
            Assert(_parentTrimmed && parent == new IntPtr(200) && relativeChild == new IntPtr(120) && Freed.Count == 0,
                "relative child and parent owners must be live throughout native call");
            if (ThrowOnSelect) throw new SyntheticPidlException();
            return 0;
        }
        public void Free(IntPtr owned) { Assert(owned != new IntPtr(120), "never free borrowed child"); Freed.Add(owned); }
    }

    private static void WithTargets(Action<string, string> test)
    {
        var root = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? Path.GetTempPath();
        var directory = Path.Combine(root, $"nxtools-native-actions-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        var file = Path.Combine(directory, "synthetic.bin");
        File.WriteAllBytes(file, []);
        try { test(directory, file); }
        finally { Directory.Delete(directory, true); }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class OpenWithFixture : IOpenWithShellPlatform
    {
        public int Result { get; set; }
        public int Calls { get; private set; }
        public string? Target { get; private set; }
        public Action? DuringShow { get; set; }
        public int ShowChooser(IntPtr owner, string target)
        {
            Calls++;
            Target = target;
            DuringShow?.Invoke();
            return Result;
        }
    }

    private sealed class ExplorerFixture : IExplorerShellPlatform
    {
        private static readonly IntPtr Candidate = new(10);
        private static readonly IntPtr Child = new(11);
        public List<string> Calls { get; } = [];
        public List<string> Arguments { get; } = [];
        public string? SelectedTarget { get; private set; }
        public int NativeResult { get; set; }
        public int PublishAfter { get; set; } = 1;
        public int ForegroundAfter { get; set; } = 1;
        public bool DestroyAfterActivation { get; set; }
        public int Waits { get; private set; }
        public int Activations { get; private set; }
        public int Readbacks { get; private set; }
        private int _lookups;
        public Action? OnReadback { get; set; }
        public int LastErrorCode => 5;
        public int OpenNative(string target, bool isDirectory)
        {
            Calls.Add(isDirectory ? "native_open" : "native_select");
            SelectedTarget = target;
            return NativeResult;
        }
        public IntPtr FindExplorerWindow(string directory)
        {
            Calls.Add("lookup");
            return ++_lookups >= PublishAfter ? Candidate : IntPtr.Zero;
        }
        public void LaunchExplorer(string arguments) { Calls.Add("fallback"); Arguments.Add(arguments); }
        public bool IsWindow(IntPtr window) => window != IntPtr.Zero && !(DestroyAfterActivation && Activations > 0 && window == Candidate);
        public bool IsIconic(IntPtr window) => true;
        public void RestoreWindow(IntPtr window) => Calls.Add("restore");
        public void TryActivateWindow(IntPtr window) { Calls.Add("activate"); Activations++; }
        public IntPtr GetForegroundWindow()
        {
            Calls.Add("readback");
            Readbacks++;
            OnReadback?.Invoke();
            return Activations >= ForegroundAfter ? Child : new IntPtr(99);
        }
        public IntPtr GetRootWindow(IntPtr window) => window == Child ? Candidate : window;
        public void WaitForNextAttempt() => Waits++;
    }
}
