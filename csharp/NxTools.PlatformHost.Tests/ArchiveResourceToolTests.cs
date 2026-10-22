using System.Text.Json;

namespace NxTools.PlatformHost.Tests;

// Effects: synthetic files below the current session scratch only; no product/native mutation.
internal static class ArchiveResourceToolTests
{
    private static void Require(bool condition, string detail) { if (!condition) throw new InvalidOperationException(detail); }
    internal static string OwnRoot()
    {
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("session scratch required");
        var root = Path.Combine(scratch, "archive-resource-tool-" + Guid.NewGuid().ToString("D"));
        Directory.CreateDirectory(root); File.WriteAllText(Path.Combine(root, "owned-resource-fixtures"), "test"); return root;
    }
    public static async Task InitializationOwnsStatus()
    {
        var root = OwnRoot(); var control = Directory.CreateDirectory(Path.Combine(root, "control")).FullName;
        var previousControl = Environment.GetEnvironmentVariable("NXTOOLS_RESOURCE_CONTROL");
        var previousId = Environment.GetEnvironmentVariable("NXTOOLS_RESOURCE_RUN_ID");
        var previousWorker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        var runId = Guid.NewGuid();
        try {
            Environment.SetEnvironmentVariable("NXTOOLS_RESOURCE_CONTROL", control);
            Environment.SetEnvironmentVariable("NXTOOLS_RESOURCE_RUN_ID", runId.ToString("D"));
            Environment.SetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE", Path.Combine(root, "missing.exe"));
            Require(await ArchiveResourceProbe.RunAsync("runner100k", root) == 1, "initialization failure must remain failed");
            Require(File.Exists(Path.Combine(control, "status.json")), "initialization failure lost owned status/cleanup finally");
            using var status = JsonDocument.Parse(File.ReadAllText(Path.Combine(control, "status.json")));
            var value = status.RootElement;
            Require(value.GetProperty("schema").GetInt32() == 2 && value.GetProperty("run_id").GetGuid() == runId, "status identity/schema");
            Require(value.GetProperty("cleanup").GetProperty("status").GetString() == "Confirmed", "nothing acquired should confirm vacuous cleanup");
            Require(value.GetProperty("cleanup").GetProperty("native_terminal").ValueKind == JsonValueKind.Null &&
                value.GetProperty("cleanup").GetProperty("worker_exited").ValueKind == JsonValueKind.Null,
                "initialization failure fabricated worker/native cleanup evidence");
            Require(value.GetProperty("identities")[0].GetProperty("role").GetString() == "probe_owner", "root role");
        } finally {
            Environment.SetEnvironmentVariable("NXTOOLS_RESOURCE_CONTROL", previousControl);
            Environment.SetEnvironmentVariable("NXTOOLS_RESOURCE_RUN_ID", previousId);
            Environment.SetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE", previousWorker);
            // Only explicitly created files; uncertain residues are never swept.
            foreach (var name in new[] { "status.json", "status.tmp" }) File.Delete(Path.Combine(control, name));
            if (!Directory.EnumerateFileSystemEntries(control).Any()) Directory.Delete(control);
            File.Delete(Path.Combine(root, "owned-resource-fixtures"));
            if (!Directory.EnumerateFileSystemEntries(root).Any()) Directory.Delete(root);
        }
    }
    public static Task CounterReadsCommitTogether()
    {
        var deny = true;
        using var samples = new ArchiveResourceProbe.Samples(() => deny ? 999 : 12, () => deny ? throw new InvalidOperationException("denied") : 24);
        samples.Take();
        Require(samples.Count == 0, "failed private read was counted as successful sample");
        Require(samples.LastWorking == 0 && samples.PeakWorking == 0, "partial working read changed committed counters");
        var empty = JsonSerializer.SerializeToElement(samples.Result());
        Require(empty.GetProperty("status").GetString() == "Incomplete" && empty.GetProperty("PeakWorking").ValueKind == JsonValueKind.Null, "unavailable sample fabricated zero");
        deny = false; samples.Take();
        var result = JsonSerializer.SerializeToElement(samples.Result());
        Require(samples.Count == 1 && samples.PeakWorking == 12 && samples.PeakPrivate == 24, "failed read poisoned peaks");
        Require(result.GetProperty("attempts").GetInt32() == 2 && result.GetProperty("failures").GetInt32() == 1, "counter attempts/failures lost");
        Require(result.GetProperty("BaselineWorking").GetInt64() == 12, "baseline must be first valid pair");
        deny = true; samples.Take();
        Require(samples.Count == 1 && samples.LastWorking == 12 && samples.PeakWorking == 12 && samples.LastPrivate == 24,
            "later partial pair committed a fabricated peak");
        using var denied = new ArchiveResourceProbe.Samples(() => throw new System.ComponentModel.Win32Exception(5), () => 1);
        for (var index = 0; index < 20; index++) denied.Take();
        var deniedResult = JsonSerializer.SerializeToElement(denied.Result());
        Require(denied.Count == 0 && deniedResult.GetProperty("errors").GetArrayLength() <= 16 &&
            deniedResult.GetProperty("errors")[0].GetProperty("native_code").GetInt32() == 5, "bounded native failures");
        return Task.CompletedTask;
    }
    public static Task DispatchExcludesSetup()
    {
        double now = 100;
        var first = new ArchiveResourceProbe.DispatchClock(() => now);
        now += 10; first.Dispatch(); now += 7;
        var firstStamp = first.OperationMs;
        var second = new ArchiveResourceProbe.DispatchClock(() => now);
        // Virtual setup barrier, not a wallclock/performance threshold.
        now += 9000; second.Dispatch(); now += 7;
        Require(first.SetupMs == 10 && second.SetupMs == 9000, "setup interval missing");
        Require(firstStamp == 7 && second.OperationMs == 7, "setup advance leaked into runner dispatch operation stamps");
        return Task.CompletedTask;
    }
    public static Task ControlBoundedAndIdempotent()
    {
        var root = OwnRoot();
        try {
            foreach (var mode in new[] { "match", "wrong_id", "oversize", "malformed", "read_failure" }) {
                var directory = Directory.CreateDirectory(Path.Combine(root, mode)).FullName; var runId = Guid.NewGuid().ToString("D");
                var control = new ArchiveResourceProbe.Control(directory, runId, Path.Combine(root, "helper.exe"), Guid.NewGuid());
                control.Publish(); var stopPath = Path.Combine(directory, "stop.json");
                if (mode == "read_failure") Directory.CreateDirectory(stopPath);
                else File.WriteAllText(stopPath, mode == "oversize" ? new string(' ', 16385) : mode == "malformed" ? "{" :
                    JsonSerializer.Serialize(new { run_id = mode == "wrong_id" ? Guid.NewGuid().ToString("D") : runId, reason = "injected_threshold" }));
                Require(control.Poll() == (mode != "wrong_id"), "control run identity/bound/fail-safe");
                var calls = 0; control.Cancel(() => calls++); control.Poll(); control.Cancel(() => calls++);
                Require(calls == (mode == "wrong_id" ? 0 : 1), "duplicate stop dispatched again");
                control.Draining(); control.Finish(new("Confirmed", true, null, null, null, 0, null, true, true));
                control.Request("late"); control.Cancel(() => calls++); control.Publish();
                using var status = JsonDocument.Parse(File.ReadAllText(Path.Combine(directory, "status.json")));
                Require(status.RootElement.GetProperty("state").GetString() == "Cleaned" && calls <= 1, "late stop changed cleanup");
                if (mode == "read_failure") Directory.Delete(stopPath); else File.Delete(stopPath);
                File.Delete(Path.Combine(directory, "status.json")); Directory.Delete(directory);
            }
            var incompleteDirectory = Directory.CreateDirectory(Path.Combine(root, "output_cleanup_failure")).FullName;
            var incomplete = new ArchiveResourceProbe.Control(incompleteDirectory, Guid.NewGuid().ToString("D"), Path.Combine(root, "helper.exe"), Guid.NewGuid());
            incomplete.Finish(new("Confirmed", true, true, true, true, 0, true, true, true, false));
            incomplete.Publish();
            Require(!incomplete.CleanupFacts.confirmed && incomplete.CleanupFacts.status == "Incomplete", "owned empty output removal failure falsely confirmed cleanup");
            File.Delete(Path.Combine(incompleteDirectory, "status.json")); Directory.Delete(incompleteDirectory);
        } finally { File.Delete(Path.Combine(root, "owned-resource-fixtures")); if (!Directory.EnumerateFileSystemEntries(root).Any()) Directory.Delete(root); }
        return Task.CompletedTask;
    }
    public static async Task ActualSmallStopConverges()
    {
        var worker = Environment.GetEnvironmentVariable("NXTOOLS_ARCHIVE_WORKER_FIXTURE");
        if (string.IsNullOrEmpty(worker) || !File.Exists(worker)) throw new TestSkippedException("Explicit built actual worker required; coordinate native lane.");
        var root = OwnRoot(); var control = Directory.CreateDirectory(Path.Combine(root, "control")).FullName;
        var runId = Guid.NewGuid().ToString("D");
        // Independent framework ZIP writer, not the product writer. Exactly one Stored 256KiB member.
        using (var zip = System.IO.Compression.ZipFile.Open(Path.Combine(root, "small.zip"), System.IO.Compression.ZipArchiveMode.Create)) {
            var entry = zip.CreateEntry("synthetic.bin", System.IO.Compression.CompressionLevel.NoCompression);
            using var stream = entry.Open(); var block = Enumerable.Range(0, 256).Select(index => (byte)index).ToArray();
            for (var index = 0; index < 1024; index++) stream.Write(block);
        }
        File.WriteAllText(Path.Combine(root, "entries.jsonl"), JsonSerializer.Serialize(new { ordinal = 0, name = "synthetic.bin", kind = "file", size = 262144, packed_size = 262144 }) + "\n");
        var start = new System.Diagnostics.ProcessStartInfo(Environment.ProcessPath!) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true, CreateNoWindow = true };
        start.ArgumentList.Add("--archive-resource-probe"); start.ArgumentList.Add("tool-stop-small"); start.ArgumentList.Add(root);
        start.Environment["NXTOOLS_RESOURCE_CONTROL"] = control; start.Environment["NXTOOLS_RESOURCE_RUN_ID"] = runId;
        using var child = System.Diagnostics.Process.Start(start)!;
        var stdout = child.StandardOutput.ReadToEndAsync(); var stderr = child.StandardError.ReadToEndAsync();
        var cleanupConfirmed = false;
        try {
            var deadline = System.Diagnostics.Stopwatch.StartNew(); var held = false;
            while (!held && !child.HasExited && deadline.ElapsedMilliseconds < 10000) {
                if (File.Exists(Path.Combine(control, "status.json"))) {
                    using var status = ReadStatus(Path.Combine(control, "status.json"));
                    var value = status.RootElement;
                    if (value.TryGetProperty("barrier", out var barrier) && barrier.ValueKind == JsonValueKind.Object && barrier.TryGetProperty("partial_bytes", out _)) {
                        Require(barrier.GetProperty("partial_bytes").GetInt64() is > 0 and < 262144 && barrier.GetProperty("source_held").GetBoolean() && barrier.GetProperty("slot_held").GetBoolean(), "actual native partial/source/slot barrier");
                        Require(value.GetProperty("identities").EnumerateArray().Any(identity => identity.GetProperty("role").GetString() == "actual_worker"), "actual native handle identity required at barrier");
                        held = true;
                    }
                }
                if (!held) await Task.Delay(10);
            }
            Require(held, "actual tool did not hold first native data boundary");
            WriteStop(control, runId);
            await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(12));
            using var final = ReadStatus(Path.Combine(control, "status.json"));
            var facts = final.RootElement.GetProperty("cleanup");
            cleanupConfirmed = facts.GetProperty("confirmed").GetBoolean();
            Require(child.ExitCode == 1 && cleanupConfirmed && facts.GetProperty("native_terminal").GetBoolean() &&
                facts.GetProperty("worker_exited").GetBoolean() && facts.GetProperty("job_and_pumps_drained").GetBoolean() &&
                facts.GetProperty("active_count").GetInt32() == 0 && facts.GetProperty("source_writable").GetBoolean() &&
                facts.GetProperty("helper_removed").GetBoolean() && facts.GetProperty("partial_absent").GetBoolean(), "original runner/Job/pumps/source/helper/partial/slot failed convergence");
            var lines = (await stdout).Split('\n');
            using var result = JsonDocument.Parse(lines.Single(line => line.StartsWith("RESOURCE ", StringComparison.Ordinal))[9..]);
            Require(result.RootElement.GetProperty("operation").GetProperty("category").GetString() == "cancelled" &&
                result.RootElement.GetProperty("stop").GetProperty("requested").GetBoolean() &&
                result.RootElement.GetProperty("measurement").GetProperty("status").GetString() == "Complete", "operation/measurement/stop facts conflated");
            Require((await stderr).Length == 0 && !Directory.EnumerateDirectories(root, "output-*").Any(), "owned empty output not released");
        } finally {
            if (!child.HasExited) {
                WriteStop(control, runId);
                // No root kill: the coordinating parent owns any forced fallback.
                await child.WaitForExitAsync().WaitAsync(TimeSpan.FromSeconds(12));
            }
            if (cleanupConfirmed) {
                foreach (var name in new[] { "stop.json", "status.json" }) File.Delete(Path.Combine(control, name));
                Directory.Delete(control, false);
                foreach (var name in new[] { "small.zip", "entries.jsonl", "owned-resource-fixtures" }) File.Delete(Path.Combine(root, name));
                if (!Directory.EnumerateFileSystemEntries(root).Any()) Directory.Delete(root);
            }
        }
    }
    private static JsonDocument ReadStatus(string path)
    {
        using var input = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read | FileShare.Delete);
        return JsonDocument.Parse(input);
    }
    private static void WriteStop(string directory, string runId)
    {
        var temporary = Path.Combine(directory, "stop.tmp");
        File.WriteAllText(temporary, JsonSerializer.Serialize(new { run_id = runId, reason = "injected_threshold" }));
        File.Move(temporary, Path.Combine(directory, "stop.json"), true);
    }
}
