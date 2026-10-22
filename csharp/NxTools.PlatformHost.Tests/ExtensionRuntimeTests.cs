using System.Collections.Concurrent;
using System.ComponentModel;
using System.Security.Cryptography;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// No installed interpreter/native dialog: only injected picker callbacks and GUID-owned copies
// of this harness apphost, with early fixed-version argv handling before the test catalog.
internal static class ExtensionRuntimeTests
{
    public static Task TypedWirePickerAndStaFailures()
    {
        foreach (var program in new[] { "python", "node", "powershell", "pwsh" })
        {
            var pick = Request("pick_extension_runtime", new { Program = program, CurrentPath = @"C:\Synthetic\runtime.exe" });
            Assert(PlatformCommandParser.ParsePickExtensionRuntime(pick) == new PickExtensionRuntimeCommand(program, @"C:\Synthetic\runtime.exe"), "picker wire");
            var inspect = Request("inspect_extension_runtime", new { Program = program, Executable = @"C:\Synthetic\runtime.exe" });
            Assert(PlatformCommandParser.ParseInspectExtensionRuntime(inspect).Program == program, "inspect wire");
        }
        foreach (var data in new object[] { new { Program = "exe", CurrentPath = "" }, new { Program = "node" }, new { Program = 4, CurrentPath = "" } })
            Reject(() => PlatformCommandParser.ParsePickExtensionRuntime(Request("pick_extension_runtime", data)));
        Reject(() => PlatformCommandParser.ParseInspectExtensionRuntime(Request("inspect_extension_runtime", new { Program = "exe", Executable = "anything.exe" })));
        var result = ExtensionRuntimePicker.PickCore(new("python", @"C:\Synthetic\python.exe"), CancellationToken.None,
            path => path == @"C:\Synthetic\python.exe", path => path == @"C:\Synthetic", settings =>
            {
                Assert(settings.InitialDirectory == @"C:\Synthetic" && settings.FileName == "python.exe", "initial path");
                Assert(settings.Filter == "EXE (*.exe)|*.exe" && settings.CheckFileExists && settings.CheckPathExists && !settings.Multiselect, "EXE only");
                return @"C:\Synthetic\python.exe";
            });
        Assert(result.Success && result.Data == @"C:\Synthetic\python.exe", "full selected path");
        result = ExtensionRuntimePicker.PickCore(new("node", ""), CancellationToken.None, _ => true, _ => false, _ => null);
        Assert(result.Success && result.Data == "", "dismissal is empty success");
        result = ExtensionRuntimePicker.PickCore(new("node", ""), CancellationToken.None, _ => true, _ => false, _ => @"C:\Synthetic\run.cmd");
        Assert(!result.Success && result.Data is null, "scripts are not runtimes");
        var nativeFailure = ExtensionRuntimePicker.PickCore(new("node", ""), CancellationToken.None, _ => false, _ => false,
            _ => throw new Win32Exception(5, "PRIVATE executable path"));
        Assert(!nativeFailure.Success && nativeFailure.NativeErrorCode == 5 && !nativeFailure.Error!.Contains("PRIVATE"), "picker native code");
        using var lateCancellation = new CancellationTokenSource();
        result = ExtensionRuntimePicker.PickCore(new("node", ""), lateCancellation.Token, _ => true, _ => false,
            _ => { lateCancellation.Cancel(); return @"C:\Synthetic\late.exe"; });
        Assert(!result.Success && result.Data is null && result.NativeErrorCode == 995, "drop late selection");
        using var cancelled = new CancellationTokenSource(); cancelled.Cancel();
        Assert(ExtensionRuntimePicker.Pick(new("node", ""), cancelled.Token).NativeErrorCode == 995, "pre-cancel never shows UI");
        result = StaDialogRunner.Run("owned failure fixture", () => throw new EntryPointNotFoundException("PRIVATE path stdout"));
        Assert(!result.Success && result.NativeErrorCode != null && !result.Error!.Contains("PRIVATE"), "STA failure is fixed and code-bearing");
        result = ExtensionManifestPicker.PickCore(new(""), CancellationToken.None, _ => false, _ => false,
            _ => throw new Win32Exception(5, "PRIVATE payload"));
        Assert(!result.Success && result.NativeErrorCode == 5 && !result.Error!.Contains("PRIVATE"), "manifest failure privacy");
        return Task.CompletedTask;
    }

    public static async Task OwnedVersionsHashAndBudgets()
    {
        using var fixture = new RuntimeFixture("version");
        var log = new FixtureLog();
        using var inspector = new ExtensionRuntimeInspector(log);
        foreach (var program in new[] { "python", "node", "powershell", "pwsh" })
        {
            var result = await inspector.InspectAsync(new(program, fixture.Executable), Guid.NewGuid(), CancellationToken.None);
            Assert(result.Success, "owned version failed: " + result.Error);
            using var data = JsonDocument.Parse(result.Data!);
            Assert(data.RootElement.GetProperty("program").GetString() == program &&
                   data.RootElement.GetProperty("executable").GetString() == fixture.Executable, "resolved identity");
            using var executable = File.OpenRead(fixture.Executable);
            Assert(data.RootElement.GetProperty("sha256").GetString() == Convert.ToHexString(SHA256.HashData(executable)).ToLowerInvariant(), "post-cleanup identity hash");
            Assert(data.RootElement.GetProperty("version").GetString()!.Length <= 128, "bounded version");
        }
        Assert(log.Entries.Any(e => e.Contains("extension.runtime.inspect.completed")) &&
               log.Entries.All(e => !e.Contains(fixture.Executable) && !e.Contains("Python 3.12.1") && !e.Contains("v22.1.2")), "diagnostic privacy");
    }

    // TestRunner awaits fixtures serially. Only this process's PATH changes, always restored;
    // every executable reachable from the synthetic PATH is an owned harness copy.
    public static async Task OwnedPathDiscoveryAndPrecedence()
    {
        using var first = new RuntimeFixture("version");
        using var second = new RuntimeFixture("version");
        using var alias = new RuntimeFixture("private", "node.exe", "WindowsApps");
        var programs = new[] { "python", "node", "powershell", "pwsh" };
        var expected = programs.ToDictionary(program => program, program => first.AddExecutable(program + ".exe"));
        foreach (var program in programs) second.AddExecutable(program + ".exe");
        var originalPath = Environment.GetEnvironmentVariable("PATH", EnvironmentVariableTarget.Process);
        var log = new FixtureLog();
        using var inspector = new ExtensionRuntimeInspector(log);
        try
        {
            var malformed = first.DirectoryPath + "\"broken";
            var entries = new[] { "", ".", "relative-runtime", @"C:relative-runtime", malformed,
                first.DirectoryPath + ":invalid", @"\\?\invalid-runtime", alias.DirectoryPath.Replace('\\', '/'),
                Path.Combine(alias.DirectoryPath, "..", "WindowsApps"), "\"" + first.DirectoryPath + "\"", second.DirectoryPath };
            Environment.SetEnvironmentVariable("PATH", string.Join(Path.PathSeparator, entries), EnvironmentVariableTarget.Process);
            foreach (var program in programs)
            {
                var result = await inspector.InspectAsync(new(program, ""), Guid.NewGuid(), CancellationToken.None);
                Assert(result.Success, "PATH discovery failed: " + result.Error);
                using var data = JsonDocument.Parse(result.Data!);
                Assert(data.RootElement.GetProperty("program").GetString() == program &&
                       data.RootElement.GetProperty("executable").GetString() == expected[program], "first valid PATH entry wins");
                Assert(data.RootElement.GetProperty("version").GetString() ==
                       (program == "python" ? "Python 3.12.1" : program == "node" ? "v22.1.2" : "7.4.1"), "supported fixed version output");
                Assert(data.RootElement.GetProperty("sha256").GetString()!.Length == 64, "PATH probe retains identity hash");
            }
            var selected = await inspector.InspectAsync(new("node", Path.Combine(second.DirectoryPath, "node.exe")), Guid.NewGuid(), CancellationToken.None);
            Assert(selected.Success, "explicit valid selection");
            using (var data = JsonDocument.Parse(selected.Data!))
                Assert(data.RootElement.GetProperty("executable").GetString() == Path.Combine(second.DirectoryPath, "node.exe"), "explicit selection overrides earlier PATH entry");
            var canonical = await inspector.InspectAsync(new("node", @"\\?\" + Path.Combine(second.DirectoryPath, "node.exe")), Guid.NewGuid(), CancellationToken.None);
            Assert(canonical.Success, "persisted Rust-canonical local executable remains inspectable");
            Environment.SetEnvironmentVariable("PATH", second.DirectoryPath + Path.PathSeparator + first.DirectoryPath, EnvironmentVariableTarget.Process);
            var reversed = await inspector.InspectAsync(new("node", ""), Guid.NewGuid(), CancellationToken.None);
            Assert(reversed.Success, "reordered PATH discovery");
            using (var data = JsonDocument.Parse(reversed.Data!))
                Assert(data.RootElement.GetProperty("executable").GetString() == Path.Combine(second.DirectoryPath, "node.exe"), "PATH order is not cached");
            foreach (var configured in new[] { " ", "relative-runtime.exe", Path.Combine(first.DirectoryPath, "missing.exe"), alias.Executable })
            {
                var result = await inspector.InspectAsync(new("node", configured), Guid.NewGuid(), CancellationToken.None);
                Assert(!result.Success && result.Data is null && result.Error != "extension_runtime_not_found", "explicit invalid/missing selection never falls back to PATH");
            }
            foreach (var path in new[] { "", string.Join(Path.PathSeparator, entries.Take(9)),
                string.Join(Path.PathSeparator, Enumerable.Repeat("relative-runtime", 256).Append(first.DirectoryPath)) })
            {
                Environment.SetEnvironmentVariable("PATH", path, EnvironmentVariableTarget.Process);
                var result = await inspector.InspectAsync(new("node", ""), Guid.NewGuid(), CancellationToken.None);
                Assert(!result.Success && result.Error == "extension_runtime_not_found" && result.NativeErrorCode == 2 && result.Data is null,
                    "empty/excluded/over-budget PATH is not found without launching");
            }
            Assert(log.Entries.All(entry => !entry.Contains(first.DirectoryPath) && !entry.Contains(second.DirectoryPath) &&
                !entry.Contains(alias.DirectoryPath) && !entry.Contains("Python 3.12.1") && !entry.Contains("v22.1.2")), "PATH diagnostics remain private");
        }
        finally { Environment.SetEnvironmentVariable("PATH", originalPath, EnvironmentVariableTarget.Process); }
    }

    public static async Task MissingTimeoutCancelAndOutputPrivacy()
    {
        var log = new FixtureLog();
        using var inspector = new ExtensionRuntimeInspector(log);
        var missing = await inspector.InspectAsync(new("node", @"C:\NxTools-synthetic-missing-" + Guid.NewGuid() + ".exe"), Guid.NewGuid(), CancellationToken.None);
        Assert(!missing.Success && missing.Data is null && missing.NativeErrorCode != null, "missing executable fails without launch");
        foreach (var mode in new[] { "sleep", "flood", "private", "multiline" })
        {
            using var fixture = new RuntimeFixture(mode);
            var result = await inspector.InspectAsync(new("node", fixture.Executable), Guid.NewGuid(), CancellationToken.None);
            Assert(!result.Success && result.Data is null && !result.Error!.Contains("PRIVATE") && !result.Error.Contains(fixture.Executable), "failed probe never exposes output/path");
            if (mode == "sleep") Assert(result.Error == "extension_runtime_timeout", "3 second deadline");
        }
        using (var fixture = new RuntimeFixture("sleep"))
        using (var cancellation = new CancellationTokenSource(200))
        {
            var result = await inspector.InspectAsync(new("node", fixture.Executable), Guid.NewGuid(), cancellation.Token);
            Assert(!result.Success && result.NativeErrorCode == 995, "cancel terminates owned Job and reports cancellation");
        }
        Assert(log.Entries.All(e => !e.Contains("PRIVATE") && !e.Contains(".exe")), "output/path absent from all diagnostics");
    }

    public static async Task DetachedCommandsKeepReaderResponsive()
    {
        foreach (var command in new[] { "pick_extension_runtime", "inspect_extension_runtime" })
        {
            var dispatcher = new PlatformCommandSequenceDispatcher();
            var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
            try
            {
                await dispatcher.DispatchAsync(command, () => { entered.SetResult(); release.Task.GetAwaiter().GetResult(); return Task.CompletedTask; });
                await entered.Task.WaitAsync(TimeSpan.FromSeconds(2));
                var later = false;
                await dispatcher.DispatchAsync("set_window_visibility", () => { later = true; return Task.CompletedTask; });
                Assert(later && !release.Task.IsCompleted, "serial reader must not wait for picker/probe");
            }
            finally { release.TrySetResult(); await dispatcher.DrainAsync(); }
        }
    }

    // Called only by this test harness's early exact fixed-argv branch.
    internal static async Task<int> VersionFixtureAsync(string[] args)
    {
        Console.ReadLine(); // Scoped empty JSONL seed closes stdin without a racing payload write.
        var modeFile = Path.Combine(AppContext.BaseDirectory, "extension-runtime-fixture-mode.txt");
        var mode = File.Exists(modeFile) ? File.ReadAllText(modeFile) : "version";
        if (mode == "sleep") { await Task.Delay(Timeout.Infinite); return 0; }
        if (mode == "flood") { Console.WriteLine(new string('X', 5000)); return 0; }
        if (mode == "private") { Console.WriteLine("PRIVATE C:\\secret\\payload"); return 0; }
        if (mode == "multiline") { Console.WriteLine("v22.1.2\nv99.0.0"); return 0; }
        var limits = NxTools.PlatformHost.Windows.Extensions.ExtensionNative.CurrentJobLimits();
        if (limits.JobMemory.ToUInt64() != 256UL * 1024 * 1024 || limits.Basic.ActiveProcesses != 2 ||
            (limits.Basic.Flags & 0x2208) != 0x2208) return 8;
        Console.WriteLine(args[0] == "-I" ? "Python 3.12.1" : args[0] == "--version" ? "v22.1.2" : "7.4.1");
        return 0;
    }

    private sealed class RuntimeFixture : IDisposable
    {
        private readonly string directory = Path.Combine(Path.GetTempPath(), "NxTools-runtime-" + Guid.NewGuid());
        internal string DirectoryPath { get; }
        internal string Executable { get; }
        internal RuntimeFixture(string mode, string? executableName = null, string? subdirectory = null)
        {
            DirectoryPath = subdirectory is null ? directory : Path.Combine(directory, subdirectory);
            Directory.CreateDirectory(DirectoryPath);
            foreach (var source in Directory.GetFiles(AppContext.BaseDirectory))
                if (Path.GetExtension(source) is ".dll" or ".json") File.Copy(source, Path.Combine(DirectoryPath, Path.GetFileName(source)));
            Executable = AddExecutable(executableName ?? Path.GetFileName(Environment.ProcessPath!));
            File.WriteAllText(Path.Combine(DirectoryPath, "extension-runtime-fixture-mode.txt"), mode);
        }
        internal string AddExecutable(string name)
        {
            var target = Path.Combine(DirectoryPath, name);
            File.Copy(Environment.ProcessPath!, target);
            // Keep the embedded apphost DLL plus renamed-host companion names. Hostfxr
            // lookup must stay entirely in this owned directory, never an installed runtime tool.
            var sourceStem = Path.GetFileNameWithoutExtension(Environment.ProcessPath!);
            var targetStem = Path.GetFileNameWithoutExtension(name);
            if (sourceStem != targetStem)
                foreach (var suffix in new[] { ".dll", ".runtimeconfig.json", ".deps.json" })
                {
                    var source = Path.Combine(AppContext.BaseDirectory, sourceStem + suffix);
                    if (File.Exists(source)) File.Copy(source, Path.Combine(DirectoryPath, targetStem + suffix));
                }
            return target;
        }
        public void Dispose() => Directory.Delete(directory, true);
    }
    private sealed class FixtureLog : IPlatformLog
    {
        internal readonly ConcurrentQueue<string> Entries = new();
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null) =>
            Entries.Enqueue(eventName + " " + correlationId + " " + JsonSerializer.Serialize(fields));
    }
    private static ProtocolEnvelope Request(string command, object data) => ProtocolEnvelope.Create("platform_command", new { Command = command, Data = data });
    private static void Reject(Action action) { try { action(); } catch (InvalidDataException) { return; } throw new InvalidOperationException("invalid runtime request accepted"); }
    private static void Assert(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
}
