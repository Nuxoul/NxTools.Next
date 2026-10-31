using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Tests;

// All native effects are confined to a newly created hz5-GUID in this session's scratch.
internal static class Hz5ReclamationTests
{
    private static void Require(bool value, string code) { if (!value) throw new InvalidOperationException(code); }
    private static ProtocolEnvelope Wire(ArchiveJobCommand command) => ProtocolEnvelope.Create("platform_command", new { command = "archive_job", data = command }, command.OperationId);
    internal static Task WireAndAdmission()
    {
        var recovery = Guid.NewGuid();
        var command = new ArchiveJobCommand { Version = 1, OperationId = Guid.NewGuid(), SessionId = Guid.NewGuid(), Generation = 1, Action = "start",
            Request = new() { Kind = "extract", Paths = [@"C:\fixture\source.zip"], Destination = @"C:\fixture\output",
                Extract = new() { Strategy = "here", Conflict = "ask", CopyMotw = false, SingleFileFolder = false, Reclamation = new("start", recovery) } } };
        Require(ArchiveJobWire.Parse(Wire(command)).Request!.Extract!.Reclamation!.RecoveryId == recovery, "start_roundtrip");
        var request = command.Request!; var extract = request.Extract!;
        foreach (var bad in new[] { extract with { Reclamation = new("resume", recovery) }, extract with { Reclamation = new("start", Guid.Empty) },
            extract with { Strategy = "smart" }, extract with { Conflict = "overwrite" }, extract with { Conflict = "skip" }, extract with { Conflict = "auto_rename" } })
        {
            var rejected = false;
            try { ArchiveJobWire.Parse(Wire(command with { Request = request with { Extract = bad } })); } catch { rejected = true; }
            Require(rejected, "invalid_reclamation_shape");
        }
        Require(ArchiveJobWire.Parse(Wire(command with { Request = request with { Extract = extract with { Reclamation = new("continue", recovery) } } })).Action == "start", "continue_is_fresh_start");
        Require(ArchiveJobWire.Parse(Wire(command with { Request = request with { Extract = extract with { SingleFileFolder = true } } })).Request!.Extract!.Strategy == "here", "here_destination_remains_explicit");
        var readmissionRejected = false;
        try { ArchiveJobWire.Parse(Wire(command with { Request = request with { Readmission = new(Guid.NewGuid(), null) } })); } catch { readmissionRejected = true; }
        Require(readmissionRejected, "no_ordinary_readmission");
        var limits = new ArchiveAdmittedLimits(4096, 64UL << 20, 64UL << 20, 1UL << 20, 4UL << 20);
        var worker = new ArchiveWorkerRequest(Guid.NewGuid(), "extract", request.Paths[0], 0, limits, OutputRoot: request.Destination, ConflictPolicy: "ask", JobExecution: true,
            JobExtract: extract, ExtractOptions: new(false, extract.Reclamation));
        Require(ArchiveWorkerAdmission.Capture(worker).ExtractOptions!.Reclamation == extract.Reclamation, "worker_admission");
        foreach (var invalid in new[] { worker with { DeleteSource = false }, worker with { OutputReadmission = JsonSerializer.SerializeToElement(new { }) },
            worker with { ConfirmedOverwrites = new Dictionary<string,string> { ["file"] = "identity" } }, worker with { JobExtract = extract with { Reclamation = null } },
            worker with { ExtractOptions = new(false, new("continue", Guid.NewGuid())) } }) {
            var refused = false; try { ArchiveWorkerAdmission.Capture(invalid); } catch (ArchiveOutputException) { refused = true; }
            Require(refused, "symmetric_worker_shape");
        }
        return Task.CompletedTask;
    }
    private static JsonElement Call(string action, object? body = null) => JsonSerializer.SerializeToElement(new
        { call_id = Guid.NewGuid(), action, data = body is null ? null : JsonSerializer.SerializeToUtf8Bytes(body, JsonDefaults.Options).Select(b => (int)b).ToArray() });
    private static JsonElement Execute(ArchiveWorkerOutputBridge bridge, Guid id, JsonElement call)
    {
        var reply = JsonSerializer.SerializeToElement(bridge.Execute(id, call, out var failure), JsonDefaults.Options);
        if (!reply.GetProperty("success").GetBoolean()) throw new InvalidOperationException(failure?.Category ?? "native_failure");
        return reply;
    }
    internal static Task PendingPunchRecoveryAndOutputProtection()
    {
#if !DEBUG
        throw new TestSkippedException("HZ5 destructive fixture requires Debug-only interruption seams.");
#else
        var scratch = Environment.GetEnvironmentVariable("PI_SCRATCH_DIR") ?? throw new InvalidOperationException("session_scratch_required");
        var root = Path.Combine(scratch, "hz5-" + Guid.NewGuid().ToString("D")); Directory.CreateDirectory(root);
        var source = Path.Combine(root, "source.zip"); var output = Path.Combine(root, "output"); Directory.CreateDirectory(output);
        var payload = Enumerable.Range(0, 262145).Select(i => (byte)(i * 31 + 7)).ToArray();
        File.WriteAllBytes(source, payload); var recovery = Guid.NewGuid(); var id = Guid.NewGuid();
        ulong originalAllocation;
        using (var observation = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
            originalAllocation = ArchiveReclamationNative.Allocation(observation.SafeFileHandle);
        var limits = new ArchiveAdmittedLimits(4096, 64UL << 20, 64UL << 20, 32UL << 20, 64UL << 20);
        ArchiveWorkerRequest Request(string mode) => new(id, "extract", source, 0, limits, OutputRoot: output, ConflictPolicy: "ask", JobExecution: true,
            ExtractOptions: new(false, new(mode, recovery)), JobExtract: new() { Strategy = "here", Conflict = "ask", CopyMotw = false, SingleFileFolder = false, Reclamation = new(mode, recovery) });
        using (var bridge = new ArchiveWorkerOutputBridge(Request("start"), null))
        {
            bridge.AcquireReclamation(Request("start"));
            var info = JsonDocument.Parse(Execute(bridge, id, Call("hz5_info")).GetProperty("read_data").GetString()!).RootElement;
            var cluster = info.GetProperty("cluster_bytes").GetUInt64();
            Require(cluster > 0 && cluster < (ulong)payload.Length / 2, "native_geometry");
            Require(cluster >= 65536 && (cluster & (cluster - 1)) == 0, "ntfs_sparse_release_quantum");
            Require(!File.Exists(source + ArchiveReclamationNative.MarkerName) && Directory.GetFiles(root, ".nxtools-hz5-*.journal").Length == 0, "geometry_does_not_mark_unqualified_source");
            var metadata = new { modified = (object?)null, attributes = (object?)null };
            Execute(bridge, id, Call("hz5_plan_entry", new { index = 0, ordinal = 0, path = "verified.bin", header_start = 0, payload_start = 1,
                payload_end = payload.Length, record_end = payload.Length, release_start = cluster, release_end = (ulong)payload.Length / cluster * cluster,
                size = payload.Length - 1, crc32 = ArchiveReclamationNative.Crc32(payload.AsSpan(1)), metadata }));
            Require(!File.Exists(source + ArchiveReclamationNative.MarkerName), "unsealed_plan_does_not_mark_intact_source");
            Execute(bridge, id, Call("hz5_seal", new { count = 1, structure = Convert.ToBase64String(new byte[] { 0x50, 0x4b }) }));
            Execute(bridge, id, Call("hz5_select", new { index = 0 }));
            var begin = Execute(bridge, id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "begin", path = "verified.bin" }));
            var token = begin.GetProperty("token").GetGuid();
            for (var offset = 1; offset < payload.Length; offset += 65536)
                Execute(bridge, id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "write", token, data = payload.Skip(offset).Take(65536).Select(b => (int)b).ToArray() }));
            Execute(bridge, id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "file_metadata", token, metadata }));
            Execute(bridge, id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "commit", token }));
            Require(File.ReadAllBytes(Path.Combine(output, "verified.bin")).AsSpan().SequenceEqual(payload.AsSpan(1)), "independent_output_oracle");
            Execute(bridge, id, Call("hz5_pending", new { index = 0 }));
            bridge.CommitCheckpointProbe = stage => { if (stage == "hz5_before_applied") throw new ArchiveOutputException("reclamation_fixture_applied_failure", 112); };
            var failed = JsonSerializer.SerializeToElement(bridge.Execute(id, Call("hz5_apply", new { index = 0 }), out var failure));
            Require(!failed.GetProperty("success").GetBoolean() && failure?.Category == "reclamation_fixture_applied_failure", "pending_survives_applied_failure");
            var refused = false; try { using var writer = new FileStream(Path.Combine(output, "verified.bin"), FileMode.Open, FileAccess.Write, FileShare.ReadWrite); } catch (IOException) { refused = true; }
            Require(refused, "verified_output_protected");
        }
        using (var bridge = new ArchiveWorkerOutputBridge(Request("continue"), null))
        {
            bridge.AcquireReclamation(Request("continue")); Execute(bridge, id, Call("hz5_info"));
            var observed = JsonDocument.Parse(Execute(bridge, id, Call("hz5_observe", new { index = 0 })).GetProperty("read_data").GetString()!).RootElement;
            Require(observed.GetProperty("status").GetString() == "published", "pending_reconciled_not_redecoded");
            var applied = Execute(bridge, id, Call("hz5_apply", new { index = 0 })).GetProperty("read_data").GetString();
            Require(JsonDocument.Parse(applied!).RootElement.GetProperty("released_bytes").GetUInt64() > 0, "actual_release");
            using (var observation = new FileStream(source, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
                Require(JsonDocument.Parse(applied!).RootElement.GetProperty("released_bytes").GetUInt64() == originalAllocation - ArchiveReclamationNative.Allocation(observation.SafeFileHandle), "net_actual_allocation_not_nominal_hole");
            Require(Execute(bridge, id, Call("hz5_apply", new { index = 0 })).GetProperty("read_data").GetString() == applied, "no_double_count");
            Execute(bridge, id, Call("hz5_complete"));
        }
        var ordinaryRefused = false; try { using var lease = ArchiveSourceLease.Open(source); } catch (ArchiveOutputException failure) { ordinaryRefused = failure.Stage == "reclamation_recovery_required"; }
        Require(ordinaryRefused, "consumed_shell_not_ordinary_source");
        return Task.CompletedTask;
#endif
    }
    internal static Task SourceActivityAndEditHandoff()
    {
        var root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "hz5-" + Guid.NewGuid().ToString("D")); Directory.CreateDirectory(root);
        var path = Path.Combine(root, "source.zip"); File.WriteAllBytes(path, [1, 2, 3, 4]);
        using (var deferred = ArchiveSourceLease.ReserveActivity(path, false, Guid.NewGuid())) {
            var refused = false; try { using var destructive = ArchiveSourceLease.ReserveActivity(path, true, Guid.NewGuid()); } catch (ArchiveOutputException failure) { refused = failure.Stage == "reclamation_source_busy"; }
            Require(refused, "deferred_activity_excluded_without_file_handle");
        }
        using (var edit = ArchiveSourceLease.OpenForEdit(path)) {
            edit.ReleaseEditFile(edit.CompleteEditReads());
            var refused = false; try { using var destructive = ArchiveSourceLease.ReserveActivity(path, true, Guid.NewGuid()); } catch (ArchiveOutputException failure) { refused = failure.Stage == "reclamation_source_busy"; }
            Require(refused, "edit_handoff_reservation_survives_file_close");
        }
        using (var admitted = ArchiveSourceLease.ReserveActivity(path, true, Guid.NewGuid())) Require(admitted.Identity.Length > 20, "actual_cleanup_releases_activity");
        using var cache = ArchiveEntryCache.ForOwnedTestCache(root);
        var cached = cache.Create(Guid.NewGuid(), Guid.NewGuid(), 1);
        cached.BindSourceActivity(ArchiveSourceLease.ReserveActivity(path, false, Guid.NewGuid()));
        var cachedRefused = false; try { using var destructive = ArchiveSourceLease.ReserveActivity(path, true, Guid.NewGuid()); } catch (ArchiveOutputException failure) { cachedRefused = failure.Stage == "reclamation_source_busy"; }
        Require(cachedRefused, "cached_preview_holds_source_activity");
        Require(cache.Release(cached.Id), "exact_cached_lease_cleanup");
        using var afterCache = ArchiveSourceLease.ReserveActivity(path, true, Guid.NewGuid());
        return Task.CompletedTask;
    }
    internal static Task ReadyRecoveryAndIncompleteRefusal()
    {
#if !DEBUG
        throw new TestSkippedException("HZ5 interruption fixtures require Debug-only seams.");
#else
        foreach (var ready in new[] { false, true }) {
            using var fixture = new NativeFixture(copyMotw: ready);
            using (var bridge = fixture.Open("start")) {
                fixture.Plan(bridge); Execute(bridge, fixture.Id, Call("hz5_select", new { index = 0 }));
                var token = Execute(bridge, fixture.Id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "begin", path = "verified.bin" })).GetProperty("token").GetGuid();
                for (var offset = 1; offset < fixture.Payload.Length; offset += 65536)
                    Execute(bridge, fixture.Id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "write", token, data = fixture.Payload.Skip(offset).Take(65536).Select(b => (int)b).ToArray() }));
                if (ready) {
                    Execute(bridge, fixture.Id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "file_metadata", token, metadata = fixture.Metadata }));
                    bridge.CommitCheckpointProbe = stage => { if (stage == "before_recheck") throw new ArchiveOutputException("reclamation_fixture_stop"); };
                    var reply = JsonSerializer.SerializeToElement(bridge.Execute(fixture.Id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "commit", token }), out _));
                    Require(!reply.GetProperty("success").GetBoolean(), "ready_interrupted_before_publication");
                }
            }
            Require(!File.Exists(Path.Combine(fixture.Output, "verified.bin")), "no_early_publication");
            using (var continuation = fixture.Open("continue")) {
                Execute(continuation, fixture.Id, Call("hz5_info"));
                var reply = JsonSerializer.SerializeToElement(continuation.Execute(fixture.Id, Call("hz5_observe", new { index = 0 }), out var failure));
                Require(reply.GetProperty("success").GetBoolean() == ready, "durable_ready_is_publication_authority");
                if (!ready) Require(failure?.Category == "reclamation_incomplete_partial_preserved", "incomplete_preserving_refusal");
            }
            Require(File.ReadAllBytes(fixture.Source).AsSpan().SequenceEqual(fixture.Payload), "observe_never_punches");
            if (ready) {
                var output = Path.Combine(fixture.Output, "verified.bin");
                Require(File.ReadAllBytes(output).AsSpan().SequenceEqual(fixture.Payload.AsSpan(1)), "ready_exact_content");
                Require((File.GetAttributes(output) & (FileAttributes.ReadOnly | FileAttributes.Hidden)) == (FileAttributes.ReadOnly | FileAttributes.Hidden), "ready_exact_metadata");
                Require(File.ReadAllText(output + ":Zone.Identifier") == "[ZoneTransfer]\r\nZoneId=3\r\n", "ready_exact_motw");
            } else Require(Directory.GetFiles(fixture.Output, ".nxpart-*").Length == 1, "indispensable_partial_retained");
        }
        return Task.CompletedTask;
#endif
    }
    internal static Task FramingBoundsAndRefusals()
    {
        using var fixture = new NativeFixture(false);
        using (var bridge = fixture.Open("start")) {
            fixture.Plan(bridge);
            var missingPending = JsonSerializer.SerializeToElement(bridge.Execute(fixture.Id, Call("hz5_apply", new { index = 0 }), out var pendingFailure));
            Require(!missingPending.GetProperty("success").GetBoolean() && pendingFailure?.Category == "reclamation_pending_required", "no_unlogged_punch");
            var info = JsonDocument.Parse(Execute(bridge, fixture.Id, Call("hz5_info")).GetProperty("read_data").GetString()!).RootElement;
            var length = info.GetProperty("journal_bytes").GetInt32();
            var read = JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "hz5_read", offset = 0, length });
            var bytes = Convert.FromBase64String(Execute(bridge, fixture.Id, read).GetProperty("read_data").GetString()!);
            var at = 0; ulong sequence = 0;
            while (at < bytes.Length) {
                var size = System.Buffers.Binary.BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(at));
                Require(size is > 0 and <= 65536 && at + size + 8 <= bytes.Length, "bounded_frame");
                Require(size == System.Buffers.Binary.BinaryPrimitives.ReadInt32LittleEndian(bytes.AsSpan(at + size + 4)), "matching_end_frame");
                using var payload = JsonDocument.Parse(bytes.AsMemory(at + 4, size));
                Require(payload.RootElement.GetProperty("sequence").GetUInt64() == ++sequence, "contiguous_real_records"); at += size + 8;
            }
            Require(sequence == 3, "header_plan_seal_written_once");
            var oversized = JsonSerializer.SerializeToElement(bridge.Execute(fixture.Id, JsonSerializer.SerializeToElement(new { call_id = Guid.NewGuid(), action = "hz5_read", offset = 0, length = 65537 }), out var boundFailure));
            Require(!oversized.GetProperty("success").GetBoolean() && boundFailure?.Category == "reclamation_read_limit", "read_bound");
        }
        var journal = Directory.GetFiles(fixture.Root, ".nxtools-hz5-*.journal").Single();
        using (var file = new FileStream(journal, FileMode.Open, FileAccess.Write, FileShare.None)) { file.Position = 4; file.WriteByte((byte)'!'); file.Flush(true); }
        using (var continuation = fixture.Open("continue")) {
            var reply = JsonSerializer.SerializeToElement(continuation.Execute(fixture.Id, Call("hz5_info"), out var failure));
            Require(!reply.GetProperty("success").GetBoolean() && failure?.Category == "reclamation_record_invalid", "complete_corruption_refused");
        }
        Require(File.ReadAllBytes(fixture.Source).AsSpan().SequenceEqual(fixture.Payload), "refusals_do_not_punch");
        return Task.CompletedTask;
    }
    private sealed class NativeFixture : IDisposable
    {
        internal readonly string Root, Source, Output;
        internal readonly Guid Id = Guid.NewGuid(), Recovery = Guid.NewGuid();
        internal readonly byte[] Payload = Enumerable.Range(0, 262144).Select(i => (byte)(i * 17 + 9)).ToArray();
        internal readonly object Metadata;
        private readonly bool copyMotw;
        internal NativeFixture(bool copyMotw) {
            this.copyMotw = copyMotw;
            Root = Path.Combine(Environment.GetEnvironmentVariable("PI_SCRATCH_DIR")!, "hz5-" + Guid.NewGuid().ToString("D")); Directory.CreateDirectory(Root);
            Source = Path.Combine(Root, "source.zip"); Output = Path.Combine(Root, "output"); Directory.CreateDirectory(Output); File.WriteAllBytes(Source, Payload);
            if (copyMotw) File.WriteAllText(Source + ":Zone.Identifier", "[ZoneTransfer]\r\nZoneId=3\r\n");
            Metadata = new { modified = new { kind = "filetime_utc", ticks = (ulong)new DateTime(2024, 3, 2, 1, 2, 4, DateTimeKind.Utc).ToFileTimeUtc() }, attributes = new { read_only = copyMotw, hidden = copyMotw } };
        }
        internal ArchiveWorkerOutputBridge Open(string mode) {
            var request = new ArchiveWorkerRequest(Id, "extract", Source, 0, new ArchiveAdmittedLimits(4096, 64UL << 20, 64UL << 20, 32UL << 20, 64UL << 20), OutputRoot: Output,
                ConflictPolicy: "ask", JobExecution: true, ExtractOptions: new(copyMotw, new(mode, Recovery)),
                JobExtract: new() { Strategy = "here", Conflict = "ask", CopyMotw = copyMotw, SingleFileFolder = false, Reclamation = new(mode, Recovery) });
            var bridge = new ArchiveWorkerOutputBridge(request, null); try { bridge.AcquireReclamation(request); return bridge; } catch { bridge.Dispose(); throw; }
        }
        internal void Plan(ArchiveWorkerOutputBridge bridge) {
            var info = JsonDocument.Parse(Execute(bridge, Id, Call("hz5_info")).GetProperty("read_data").GetString()!).RootElement;
            var cluster = info.GetProperty("cluster_bytes").GetUInt64();
            Execute(bridge, Id, Call("hz5_plan_entry", new { index = 0, ordinal = 0, path = "verified.bin", header_start = 0, payload_start = 1, payload_end = Payload.Length,
                record_end = Payload.Length, release_start = cluster, release_end = (ulong)Payload.Length / cluster * cluster, size = Payload.Length - 1,
                crc32 = ArchiveReclamationNative.Crc32(Payload.AsSpan(1)), metadata = Metadata }));
            Execute(bridge, Id, Call("hz5_seal", new { count = 1, structure = Convert.ToBase64String(new byte[] { 0x50, 0x4b }) }));
        }
        public void Dispose() { } // Persistent source, outputs and recovery authority are retained for the driver/oracle.
    }
}
