using Microsoft.Win32.SafeHandles;
using System.Runtime.InteropServices;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// Only the current test apphost and GUID-owned files; never launches DOpus or Shell.
internal static class DirectoryOpusExportTests
{
    internal const string ControlVariable = "NXTOOLS_TEST_DOPUS_EXPORT_CONTROL";
    private const string Xml = "<results command='paths' result='1'/>";

    public static Task WriterAfterHelperExitIsEventuallyCleaned() => HeldWriter("exited");
    public static Task WriterAfterHelperTimeoutIsEventuallyCleaned() => HeldWriter("timeout");
    public static async Task UnconfirmedHelperExitRetainsCleanupOwner()
    {
        await using var fixture = new ExportFixture("unconfirmed");
        await fixture.StartWriter();
        var stages = new ConcurrentQueue<string>();
        var helperPid = 0;
        var platform = new NativeDirectoryOpusTargetPlatform(stages.Enqueue,
            terminateHelper: process => helperPid = process.Id);
        var result = await Task.Run(() => platform.Export(fixture.Host, "paths")).WaitAsync(TimeSpan.FromSeconds(5));
        Assert(result is null && stages.Contains("observation_dopus_helper_exit_unconfirmed"),
            "real bounded waits leave the controlled helper unconfirmed and output unaccepted");
        using var helper = Process.GetProcessById(helperPid);
        await fixture.ReleaseWriter();
        Assert(!helper.HasExited && File.Exists(fixture.Output) && Directory.Exists(fixture.ExportDirectory),
            "a live helper retains the directory even after the actual writer exits and releases every output handle");
        Assert(stages.Contains("observation_dopus_cleanup_deferred") && !stages.Contains("observation_dopus_cleanup_completed"),
            "helper lifetime keeps the original cleanup owner pending");
        File.WriteAllText(fixture.Marker("helper-release"), "");
        await helper.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5));
        await WaitUntil(() => stages.Contains("observation_dopus_cleanup_completed"), TimeSpan.FromSeconds(2));
        Assert(stages.Contains("observation_dopus_cleanup_completed") && !Directory.Exists(fixture.ExportDirectory),
            "actual helper exit permits final cleanup and capacity return");
    }

    public static async Task DirectoryDeleteFailureRetainsCleanupOwner()
    {
        await using var fixture = new ExportFixture("directory");
        await fixture.StartWriter();
        var stages = new ConcurrentQueue<string>();
        var result = await Task.Run(() => new NativeDirectoryOpusTargetPlatform(stages.Enqueue).Export(fixture.Host, "paths"));
        Assert(result == Xml, "directory cleanup failure cannot change a completed output result");
        Assert(!File.Exists(fixture.Output) && Directory.Exists(fixture.ExportDirectory),
            "File.Delete succeeds while the independent directory handle prevents Directory.Delete");
        Assert(stages.Contains("observation_dopus_cleanup_failed"), "the real directory sharing violation was observed");
        await fixture.ReleaseWriter();
        await WaitUntil(() => !Directory.Exists(fixture.ExportDirectory), TimeSpan.FromSeconds(2));
        Assert(!Directory.Exists(fixture.ExportDirectory), "directory deletion must resume after its actual handle closes");
    }

    public static async Task PendingCleanupBoundsAdmissionAndRecovers()
    {
        await using var first = new ExportFixture("exited");
        await first.StartWriter();
        var firstStages = new ConcurrentQueue<string>();
        Assert(new NativeDirectoryOpusTargetPlatform(firstStages.Enqueue).Export(first.Host, "paths") is null, "first held output is not evidence");
        await using var second = new ExportFixture("exited");
        await second.StartWriter();
        Assert(new NativeDirectoryOpusTargetPlatform().Export(second.Host, "paths") is null, "second held output is not evidence");
        await using var rejected = new ExportFixture("complete");
        var stages = new ConcurrentQueue<string>();
        var result = new NativeDirectoryOpusTargetPlatform(stages.Enqueue).Export(rejected.Host, "paths");
        Assert(result is null && stages.Contains("observation_dopus_cleanup_busy"), "two pending owners reject without an unbounded retry queue");
        Assert(!File.Exists(rejected.Marker("output-path")), "capacity exhaustion does not start another helper or create its output");
        await first.ReleaseWriter();
        await WaitUntil(() => firstStages.Contains("observation_dopus_cleanup_completed"), TimeSpan.FromSeconds(2));
        Assert(firstStages.Contains("observation_dopus_cleanup_completed"), "cleanup completion follows actual capacity return");
        Assert(!Directory.Exists(first.ExportDirectory) && Directory.Exists(second.ExportDirectory), "cleanup is scoped to the original directory");
        await using var recovered = new ExportFixture("complete");
        await recovered.StartWriter();
        Assert(new NativeDirectoryOpusTargetPlatform().Export(recovered.Host, "paths") == Xml, "actual cleanup returns admission capacity");
        await recovered.ReleaseWriter();
        await second.ReleaseWriter();
        await WaitUntil(() => !Directory.Exists(second.ExportDirectory), TimeSpan.FromSeconds(2));
        Assert(!Directory.Exists(second.ExportDirectory), "the remaining owner independently finishes cleanup");
    }

    public static async Task SuccessfulOutputAndNativeReadLimitArePreserved()
    {
        foreach (var mode in new[] { "complete", "oversized" })
        {
            await using var fixture = new ExportFixture(mode);
            await fixture.StartWriter();
            var stages = new ConcurrentQueue<string>();
            var result = new NativeDirectoryOpusTargetPlatform(stages.Enqueue).Export(fixture.Host, "paths");
            Assert(mode == "complete" ? result == Xml : result is null && stages.Contains("observation_dopus_output_budget"),
                "actual Export accepts complete XML and rejects a file larger than 2 MiB");
            Assert(!Directory.Exists(fixture.ExportDirectory), "already closed outputs are cleaned synchronously");
            await fixture.ReleaseWriter();
        }
    }

    private static async Task HeldWriter(string mode)
    {
        await using var fixture = new ExportFixture(mode);
        await fixture.StartWriter();
        var stages = new ConcurrentQueue<string>();
        var platform = new NativeDirectoryOpusTargetPlatform(stages.Enqueue);
        var clock = Stopwatch.StartNew();
        var result = await Task.Run(() => platform.Export(fixture.Host, "paths"))
            .WaitAsync(TimeSpan.FromSeconds(5));
        clock.Stop();
        Assert(result is null, "an unfinished output cannot become target evidence");
        Assert(clock.ElapsedMilliseconds < 1500, "cleanup must not extend the observation budget");
        Assert(File.Exists(fixture.Marker("writer-open")), "the independent writer actually opened the output");
        Assert(stages.Contains(mode == "exited" ? "observation_dopus_output_timeout" : "observation_dopus_helper_timeout"),
            "the fixture must exercise the requested helper-exit/timeout branch");
        Assert(stages.Contains("observation_dopus_cleanup_failed"), "a real output sharing violation reaches cleanup failure");
        Assert(Directory.Exists(fixture.ExportDirectory), "the writer still holds the exact export directory");
        Assert(stages.All(stage => !stage.Contains(fixture.Control) && !stage.Contains(fixture.ExportDirectory)),
            "diagnostic stages contain no private paths");
        await fixture.ReleaseWriter();
        await WaitUntil(() => stages.Contains("observation_dopus_cleanup_completed"), TimeSpan.FromSeconds(2));
        Assert(stages.Contains("observation_dopus_cleanup_completed"), "deferred cleanup has one terminal completion");
        Assert(!Directory.Exists(fixture.ExportDirectory), "the export directory must be reclaimed after the actual writer exits");
    }

    internal static int RunHelper(string argument)
    {
        var control = Environment.GetEnvironmentVariable(ControlVariable)!;
        var comma = argument.IndexOf(',');
        if (comma < 0 || !Directory.Exists(control)) return 2;
        var output = argument[..comma];
        var request = Path.Combine(control, "output-path");
        File.WriteAllText(request + ".tmp", output);
        File.Move(request + ".tmp", request);
        if (!WaitForFile(Path.Combine(control, "writer-open"))) return 3;
        var mode = File.ReadAllText(Path.Combine(control, "mode"));
        if (mode == "timeout") Thread.Sleep(5000);
        if (mode == "unconfirmed" && !WaitForFile(Path.Combine(control, "helper-release"))) return 7;
        return 0;
    }

    internal static int RunWriter(string control)
    {
        File.WriteAllText(Path.Combine(control, "writer-online"), "");
        var request = Path.Combine(control, "output-path");
        if (!WaitForFile(request)) return 4;
        var output = File.ReadAllText(request);
        var mode = File.ReadAllText(Path.Combine(control, "mode"));
        using var directory = mode == "directory" ? OpenDirectory(Path.GetDirectoryName(output)!) : null;
        if (directory?.IsInvalid == true) return 6;
        if (mode is "exited" or "timeout")
        {
            using var stream = new FileStream(output, FileMode.CreateNew, FileAccess.Write, FileShare.Read);
            var bytes = Encoding.UTF8.GetBytes(Xml);
            stream.Write(bytes.AsSpan(0, 8));
            stream.Flush();
            File.WriteAllText(Path.Combine(control, "writer-open"), "");
            if (!WaitForFile(Path.Combine(control, "release"))) return 5;
            stream.Write(bytes.AsSpan(8));
            stream.Flush();
        }
        else
        {
            File.WriteAllBytes(output, mode == "oversized" ? new byte[DirectoryOpusTargetObservation.MaxXmlSize + 1] : Encoding.UTF8.GetBytes(Xml));
            File.WriteAllText(Path.Combine(control, "writer-open"), "");
            if (!WaitForFile(Path.Combine(control, "release"))) return 5;
        }
        directory?.Dispose();
        File.WriteAllText(Path.Combine(control, "writer-closed"), "");
        return 0;
    }

    private static SafeFileHandle OpenDirectory(string path) => CreateFileW(path, 1, FileShare.Read | FileShare.Write,
        IntPtr.Zero, FileMode.Open, 0x02000000, IntPtr.Zero);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern SafeFileHandle CreateFileW(string path, uint access, FileShare share,
        IntPtr security, FileMode disposition, uint flags, IntPtr template);

    private static bool WaitForFile(string path)
    {
        var clock = Stopwatch.StartNew();
        while (!File.Exists(path) && clock.ElapsedMilliseconds < 5000) Thread.Sleep(1);
        return File.Exists(path);
    }

    private static async Task WaitUntil(Func<bool> predicate, TimeSpan budget)
    {
        var clock = Stopwatch.StartNew();
        while (!predicate() && clock.Elapsed < budget) await Task.Delay(10);
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }

    private sealed class ExportFixture : IAsyncDisposable
    {
        internal string Control { get; } = Path.Combine(Path.GetTempPath(), "nxtools-dopus-export-fixture-" + Guid.NewGuid().ToString("N"));
        internal string Marker(string name) => Path.Combine(Control, name);
        internal string Output => File.ReadAllText(Marker("output-path"));
        internal string ExportDirectory => Path.GetDirectoryName(Output)!;
        internal DirectoryOpusObservationHost Host { get; } = new(IntPtr.Zero, 0, 0,
            Path.Combine(AppContext.BaseDirectory, "NxTools.PlatformHost.Tests.exe"));
        private readonly string? _previousControl;
        private Process? _writer;

        internal ExportFixture(string mode)
        {
            Directory.CreateDirectory(Control);
            File.WriteAllText(Marker("mode"), mode);
            _previousControl = Environment.GetEnvironmentVariable(ControlVariable);
            Environment.SetEnvironmentVariable(ControlVariable, Control);
        }

        internal async Task StartWriter()
        {
            var info = new ProcessStartInfo(Host.Helper) { UseShellExecute = false, CreateNoWindow = true };
            info.ArgumentList.Add("--dopus-export-writer");
            info.ArgumentList.Add(Control);
            _writer = Process.Start(info)!;
            await WaitUntil(() => File.Exists(Marker("writer-online")) || _writer.HasExited, TimeSpan.FromSeconds(5));
            Assert(File.Exists(Marker("writer-online")) && !_writer.HasExited, "the self-owned writer is ready before the observation clock starts");
        }

        internal async Task ReleaseWriter()
        {
            File.WriteAllText(Marker("release"), "");
            await _writer!.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5));
            Assert(_writer.ExitCode == 0 && File.Exists(Marker("writer-closed")), "the independent writer completed and closed its actual output handle");
        }

        public async ValueTask DisposeAsync()
        {
            try
            {
                File.WriteAllText(Marker("release"), "");
                File.WriteAllText(Marker("helper-release"), "");
                if (_writer is not null)
                {
                    try { await _writer.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(6)); }
                    catch (TimeoutException) { _writer.Kill(); await _writer.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(5)); }
                    _writer.Dispose();
                }
                if (File.Exists(Marker("output-path")))
                {
                    var output = File.ReadAllText(Marker("output-path"));
                    if (File.Exists(output)) File.Delete(output);
                    if (Directory.Exists(Path.GetDirectoryName(output))) Directory.Delete(Path.GetDirectoryName(output)!);
                }
            }
            finally
            {
                Environment.SetEnvironmentVariable(ControlVariable, _previousControl);
                Directory.Delete(Control, true);
            }
        }
    }
}
