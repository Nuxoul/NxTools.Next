using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// Effects: only the explicit Python-owned GUID scratch package, original Job/worker/native owners.
// No product, Shell, service, registry, ACL or path-based partial cleanup. HZ5 uses only hz5-GUID roots.
internal static class Hz4ProcessTests
{
    internal static bool Enabled
    {
        get
        {
#if DEBUG
            return true;
#else
            return false;
#endif
        }
    }
    private static void Require(bool value, string code) { if (!value) throw new InvalidOperationException(code); }
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    internal static Task CapabilityAndReleaseGate()
    {
        Require(Enabled == typeof(Hz4ProcessTests).Assembly.GetCustomAttributes(typeof(System.Diagnostics.DebuggableAttribute), false)
            .OfType<System.Diagnostics.DebuggableAttribute>().Any(value => value.IsJITTrackingEnabled && value.IsJITOptimizerDisabled), "compiled_debug_gate");
        if (!Enabled)
        {
            var riskRefused = false;
            try { ArchiveRiskTests.ActualZipRiskConfirmation().GetAwaiter().GetResult(); }
            catch (TestSkippedException failure) when (failure.Message == "Tiny risk fixture requires the DEBUG-only stricter threshold seam.") { riskRefused = true; }
            Require(riskRefused, "compiled_release_risk_refused_before_effects");
        }
        return Task.CompletedTask;
    }
    internal static async Task OriginalSessionHasNoRestartAuthority()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "archive-product-" + Guid.NewGuid());
        Directory.CreateDirectory(root);
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root);
        var connectorRefused = false;
        try { jobs.ProbeLifecycleForFixture(_ => { }); }
        catch (InvalidOperationException) { connectorRefused = true; }
        Require(connectorRefused == !Enabled, "compiled_owner_connector_gate");
        var command = new ArchiveJobCommand { Version = 1, Action = "resume", OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1 };
        var result = jobs.Dispatch(Wire(command));
        Require(!result.Success && result.Error == "stale_operation", "original_restart_refusal");
        Require(Directory.GetFileSystemEntries(root).Length == 0, "no_restart_effects");
    }
    internal static async Task<int> RunAsync(string root, string nonceText)
    {
#if !DEBUG
        // Compiled exclusion, before any path/capability read, session, worker or native output.
        await Console.Out.WriteLineAsync("{\"status\":\"refused\",\"code\":\"release_fixture_disabled\"}");
        return 2;
#else
        try { return await RunDebugAsync(root, nonceText); }
        catch (Exception failure)
        {
            // Private fixture output, fixed exception type only; no raw message/paths/payload.
            Console.WriteLine(JsonSerializer.Serialize(new { status = "fixture_failure", code = failure.GetType().Name }));
            return 3;
        }
#endif
    }
#if DEBUG
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFileW(string name, uint access, uint share, IntPtr security, uint disposition, uint flags, IntPtr template);
    private static object AllocationSample(string root, string stage, int visit)
    {
        var files = Directory.GetFiles(Path.Combine(root, "archive"), "*", SearchOption.TopDirectoryOnly)
            .Concat(Directory.GetFiles(Path.Combine(root, "output"), "*", SearchOption.AllDirectories)).ToList();
        files.Add(Path.Combine(root, "archive", "source.zip") + ArchiveReclamationNative.MarkerName);
        var allocated = new SortedDictionary<string, ulong>(StringComparer.Ordinal);
        var unavailable = new List<object>();
        foreach (var path in files)
        {
            using var handle = CreateFileW(path, 0x80, 7, IntPtr.Zero, 3, 0x200000, IntPtr.Zero);
            var code = Marshal.GetLastWin32Error();
            var relative = Path.GetRelativePath(root, path).Replace('\\', '/');
            if (handle.IsInvalid) {
                if (code is not (2 or 3)) unavailable.Add(new { path = relative, native_code = code });
                continue;
            }
            try { allocated.Add(relative, ArchiveReclamationNative.Allocation(handle)); }
            catch (ArchiveOutputException failure) { unavailable.Add(new { path = relative, native_code = failure.NativeCode }); }
        }
        return new { stage, visit, allocated, total_allocated = allocated.Values.Aggregate(0UL, (sum, bytes) => checked(sum + bytes)),
            free_bytes = new DriveInfo(Path.GetPathRoot(root)!).TotalFreeSpace, unavailable, observer = "native_lifecycle" };
    }
    private sealed class Log(string path) : NxTools.PlatformHost.Diagnostics.IPlatformLog
    {
        private readonly object gate = new();
        private int count;
        public void Write(string level, string eventName, Guid? correlationId = null, IReadOnlyDictionary<string, object?>? fields = null)
        {
            lock (gate) if (++count <= 256) File.AppendAllText(path, JsonSerializer.Serialize(new { eventName, correlationId, fields }) + "\n");
        }
    }
    private static async Task<int> RunDebugAsync(string root, string nonceText)
    {
        Require(Guid.TryParseExact(nonceText, "D", out var nonce) && nonce != Guid.Empty, "invalid_nonce");
        ArchiveReadHelper.GuardOwnedRoot(root);
        root = Path.GetFullPath(root);
        var scratch = Path.GetFullPath(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!).TrimEnd('\\') + "\\";
        Require(root.StartsWith(scratch, StringComparison.OrdinalIgnoreCase), "session_scratch_required");
        for (var directory = new DirectoryInfo(root); directory is not null && directory.FullName.Length >= scratch.Length - 1; directory = directory.Parent)
            Require((directory.Attributes & FileAttributes.ReparsePoint) == 0, "capability_reparse");
        using var capabilityStream = new FileStream(Path.Combine(root, "capability-" + nonce + ".json"), FileMode.Open, FileAccess.Read, FileShare.None);
        Require(capabilityStream.Length is > 0 and <= 4096, "capability_bound");
        using var document = JsonDocument.Parse(capabilityStream);
        var cap = document.RootElement;
        Require(cap.GetProperty("schema").GetInt32() == 1 && cap.GetProperty("nonce").GetGuid() == nonce && cap.GetProperty("root").GetString() == root, "invalid_capability");
        using var driver = Process.GetProcessById(cap.GetProperty("driver_pid").GetInt32());
        Require(!driver.HasExited && driver.StartTime.ToUniversalTime().ToFileTimeUtc() == cap.GetProperty("driver_created").GetInt64(), "driver_identity");
        var reclamation = cap.TryGetProperty("kind", out var operationKind) && operationKind.GetString() == "reclamation";
        ArchiveReclamationOptions? reclamationIntent = null;
        if (reclamation) {
            var leaf = Path.GetFileName(root);
            Require(Path.GetDirectoryName(root)!.Equals(scratch.TrimEnd('\\'), StringComparison.OrdinalIgnoreCase) && leaf.StartsWith("hz5-", StringComparison.Ordinal) && Guid.TryParseExact(leaf[4..], "D", out _), "hz5_owned_root_required");
            reclamationIntent = new(cap.GetProperty("reclamation_mode").GetString()!, cap.GetProperty("recovery_id").GetGuid());
            Require(reclamationIntent.Valid, "reclamation_intent");
        }
        var mode = cap.GetProperty("mode").GetString();
        Require(mode is "crash" or "restart" or "run" or "native_failure" or "password_cancel" or "cancel", "invalid_mode");
        var format = cap.GetProperty("format").GetString();
        Require(format is "Zip" or "SevenZ" or "Tar" or "TarGz" or "TarXz", "invalid_format");
        Require(!reclamation || format == "Zip", "reclamation_stored_zip_only");
        var operation = cap.GetProperty("operation_id").GetGuid();
        var session = cap.GetProperty("session_id").GetGuid();
        var logical = cap.GetProperty("logical_id").GetGuid();
        Require(operation != Guid.Empty && session != Guid.Empty && logical != Guid.Empty, "invalid_identity");
        using (var claimed = new FileStream(Path.Combine(root, "claimed-" + nonce), FileMode.CreateNew, FileAccess.Write, FileShare.None)) claimed.Flush(true);
        var evidence = Path.Combine(root, "evidence");
        Require(Directory.Exists(evidence) && (File.GetAttributes(evidence) & FileAttributes.ReparsePoint) == 0, "owned_evidence");
        var clock = Stopwatch.StartNew();
        using var self = Process.GetCurrentProcess();
        var identity = new { process_id = self.Id, created = self.StartTime.ToUniversalTime().ToFileTimeUtc(), image = Environment.ProcessPath,
            tests_dll_sha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(typeof(Hz4ProcessTests).Assembly.Location))),
            host_dll_sha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(typeof(ArchiveJobSession).Assembly.Location))) };
        void Save(string suffix, object value)
        {
            using var file = new FileStream(Path.Combine(evidence, nonce + "." + suffix + ".json"), FileMode.CreateNew, FileAccess.Write, FileShare.Read);
            JsonSerializer.Serialize(file, value, JsonDefaults.Options); file.Flush(true);
        }
        Save("identity", identity);
        var logger = new Log(Path.Combine(evidence, nonce + ".diagnostics.jsonl"));
        await using var jobs = ArchiveJobSession.ForOwnedTestPackage(root, logger);
        var command = new ArchiveJobCommand { Version = 1, OperationId = operation, SessionId = session, Generation = 1, Action = "resume" };
        if (mode == "restart")
        {
            // Deliberately no journal/terminal/path ingestion, rehydration, start or cleanup.
            var result = jobs.Dispatch(Wire(command));
            Require(!result.Success && result.Error == "stale_operation", "restart_must_refuse");
            Save("restart", new { schema = 1, identity, logical_id = logical, operation_id = operation, session_id = session,
                status = "refused", code = result.Error, record_authority = "none_no_durable_archive_job_recovery", replayed = false, cleanup_attempted = false });
            return 0;
        }
        var helper = Path.Combine(root, ArchiveReadHelper.DeploymentName);
        Require(Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(helper))).Equals(cap.GetProperty("worker_sha256").GetString(), StringComparison.OrdinalIgnoreCase), "matching_worker");
        var extension = format switch { "Zip" => "zip", "SevenZ" => "7z", "TarGz" => "tgz", "TarXz" => "txz", _ => "tar" };
        var source = Path.Combine(root, "archive", "source." + extension);
        var destination = Path.Combine(root, reclamation && cap.TryGetProperty("destination_archive", out var destinationArchive) && destinationArchive.GetBoolean() ? "archive" : "output");
        var point = cap.GetProperty("point").GetString();
        var selectedVisit = cap.GetProperty("visit").GetInt32();
        Require(selectedVisit is >= 1 and <= 4, "bounded_visit");
        var visits = new Dictionary<string, int>(StringComparer.Ordinal);
        var hookGate = new object();
        void Hit(string stage)
        {
            int visit;
            lock (hookGate) { visit = visits.GetValueOrDefault(stage) + 1; visits[stage] = visit; }
            if (Path.GetFileName(root).StartsWith("hz5-", StringComparison.Ordinal) && stage is "before_flush" or "native_published" or "hz5_before_punch" or "hz5_after_punch" or "hz5_before_complete" or "hz5_after_complete" or "before_cleanup" or "after_cleanup")
            {
                var sample = JsonSerializer.SerializeToUtf8Bytes(AllocationSample(root, stage, visit), JsonDefaults.Options);
                using var metrics = new FileStream(Path.Combine(evidence, nonce + ".allocation.jsonl"), FileMode.Append, FileAccess.Write, FileShare.Read);
                metrics.Write(sample); metrics.WriteByte((byte)'\n'); metrics.Flush(true);
            }
            if (stage != point || visit != selectedVisit) return;
            bool sourceExclusive;
            try { using var opened = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.None); sourceExclusive = true; }
            catch (IOException) { sourceExclusive = false; }
            Save("receipt", new { schema = 1, nonce, identity, logical_id = logical, operation_id = operation, session_id = session,
                stage, visit, source_exclusive_read_open = sourceExclusive, elapsed_ms = clock.ElapsedMilliseconds, mechanism = "original_owner_probe" });
            if (mode == "native_failure") throw new ArchiveOutputException(reclamation ? "reclamation_fixture_failure" : stage == "before_flush" && cap.GetProperty("kind").GetString() == "extract" ? "flush_file" : "replace_file", cap.GetProperty("native_code").GetInt32());
            if (mode == "cancel") {
                Require(jobs.Dispatch(Wire(command with { Action = "cancel", Request = null })).Success, "original_cancel_admission");
                return;
            }
            if (mode != "crash") return;
            // No abort switch. Driver must validate receipt + retained exact handle and terminate it.
            Thread.Sleep(TimeSpan.FromSeconds(45));
            throw new ArchiveOutputException("fixture_checkpoint_deadline");
        }
        jobs.ProbeLifecycleForFixture(Hit);
        ArchiveJobRequest request;
        if (cap.GetProperty("kind").GetString() == "edit")
        {
            await using var reads = ArchiveReadSession.ForOwnedTestPackage(root, logger);
            var readWire = ArchiveProductReadTests.Command(path: source, version: 2, limits: new() { BrowserContext = true });
            var readCommand = ArchiveReadWire.Parse(readWire);
            Require(reads.Dispatch(readWire).Success, "browser_admission");
            ArchiveBrowserContext? context = null;
            while (context is null)
            {
                var item = (await reads.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(30))).Payload.GetProperty("data").Deserialize<ArchiveReadEvent>(JsonDefaults.Options)!;
                if (item.Batch is { } batch) Require(reads.Dispatch(ArchiveProductReadTests.Command("ack", readCommand.OperationId, readCommand.SessionId, readCommand.Generation, sequence: batch.Sequence, version: 2)).Success, "metadata_ack");
                if (item.Terminal is { } terminal) { Require(terminal.Status == "succeeded" && terminal.CleanupComplete && terminal.Browser?.Format == format, "browser_terminal"); context = terminal.Browser; }
            }
            request = new() { Kind = "modify", Paths = [source], Modify = new(context, "new_directory", [], "published/") };
        }
        else request = new() { Kind = "extract", Paths = [source], Destination = destination,
            Extract = new() { Strategy = "here", SingleFileFolder = false, Conflict = "ask", CopyMotw = reclamation && cap.TryGetProperty("copy_motw", out var copyMotw) && copyMotw.GetBoolean(), Reclamation = reclamationIntent } };
        command = command with { Action = "start", Request = request };
        Save("command", command);
        Require(jobs.Dispatch(Wire(command)).Success, "job_admission");
        while (true)
        {
            var item = (await jobs.Events.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(35))).Payload.GetProperty("data").Deserialize<ArchiveJobEvent>(JsonDefaults.Options)!;
            if (item.Prompt is { } prompt)
            {
                Require(mode == "password_cancel" && prompt.Kind == "password", "unexpected_prompt");
                Require(jobs.Dispatch(Wire(command with { Action = "reply", Request = null, Reply = new() { Ticket = prompt.Ticket, Kind = "password", ApplyToAll = false } })).Success, "password_cancel_receipt");
            }
            if (item.Terminal is not { } terminal) continue;
            Save("terminal", terminal);
            Hit("after_cleanup");
            Save("finished", new { schema = 1, identity, operation_id = operation, visits, elapsed_ms = clock.ElapsedMilliseconds });
            return 0;
        }
    }
#endif
}
