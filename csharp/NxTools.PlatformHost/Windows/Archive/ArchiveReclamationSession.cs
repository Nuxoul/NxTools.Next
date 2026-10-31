using System.Buffers.Binary;
using System.Diagnostics;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

// Native authority only: Rust supplies the qualified immutable ZIP plan and every next-unit decision.
// The writable source, parent chain, append object and output receipts never leave this owner.
internal sealed class ArchiveReclamationSession : IDisposable
{
    internal const int MaximumJournalBytes = 32 << 20;
    private const int MaximumRecordBytes = 65536;
    private readonly ArchiveWorkerRequest request;
    private readonly ArchiveOutputSession output;
    private readonly ArchiveReclamationOptions intent;
    private readonly List<SafeFileHandle> sourceHandles = [];
    private readonly List<JsonElement> plans = [];
    private readonly Dictionary<int, JsonElement> partials = [], ready = [], pending = [], applied = [];
    private readonly HashSet<int> observed = [];
    private readonly IPlatformLog? log;
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private IDisposable? activity;
    private SafeFileHandle? journal, marker;
    private string sourceIdentity = "", rootIdentity = "", parentIdentity = "", companionIdentity = "";
    private byte[]? sourceZone;
    private ulong retainedBytes;
    private bool tornTail, failureReported;
    private ulong cluster, sequence;
    private long journalLength;
    private int frontier, selected = -1;
    private bool initialized, sealedPlan, completed, poisoned;
    internal Action<string>? Checkpoint {
#if DEBUG
        get; set;
#else
        get => null;
        set { if (value is not null) throw Refuse("release_fault_hook_disabled"); }
#endif
    }
    private SafeFileHandle Source => sourceHandles[^1];
    private SafeFileHandle Parent => sourceHandles[^2];
    internal ArchiveReclamationSession(ArchiveWorkerRequest request, ArchiveOutputSession output, IPlatformLog? log, string? expectedIdentity = null)
    {
        this.request = request; this.output = output; this.log = log;
        intent = request.ExtractOptions?.Reclamation ?? throw Refuse("not_admitted");
        var sourcePath = request.Path; var destination = request.OutputRoot!;
        if (!ArchiveJobWire.PathValid(sourcePath) || !ArchiveJobWire.PathValid(destination) ||
            sourcePath.Equals(destination, StringComparison.OrdinalIgnoreCase) || destination.StartsWith(sourcePath + "\\", StringComparison.OrdinalIgnoreCase)) throw Refuse("source_output_overlap");
        var parts = sourcePath[3..].Split('\\');
        if (parts.Length is < 1 or > 64 || parts.Any(p => !ValidComponent(p))) throw Refuse("source_path");
        try
        {
            sourceHandles.Add(OutputNative.Open(null, "\\??\\" + char.ToUpperInvariant(sourcePath[0]) + ":\\", directory: true));
            for (var index = 0; index < parts.Length - 1; index++) sourceHandles.Add(OutputNative.Open(sourceHandles[^1], parts[index], directory: true));
            sourceHandles.Add(OutputNative.Open(sourceHandles[^1], parts[^1], directory: false, reclamationWrite: true));
            if (intent.Mode == "start") ArchiveReclamationNative.RefuseRecoverySource(Source);
            ArchiveReclamationNative.RequireCapability(Source, intent.Mode == "continue");
            if (expectedIdentity is not null && ArchiveReclamationNative.Identity(Source) != expectedIdentity) throw Refuse("source_identity");
            activity = ArchiveSourceActivity.Acquire(ArchiveReclamationNative.Identity(Source), true, request.OperationId);
            sourceIdentity = ArchiveReclamationNative.Identity(Source, true); parentIdentity = ArchiveReclamationNative.Identity(Parent);
            // NTFS sparse deallocation uses 64KiB units even on a smaller-cluster volume.
            cluster = Math.Max(65536UL, OutputNative.AllocationSpace(Parent).Allocation);
            if (OutputNative.Identity(output.ReclamationAncestor).Volume != OutputNative.Identity(Source).Volume) throw Refuse("different_volume");
            if (request.ExtractOptions!.CopyMotw) { sourceZone = ArchiveMotw.Read(Source); output.BindZoneIdentifier(sourceZone); }
            output.Reclamation = this;
            Diagnostic("accepted");
        }
        catch { Dispose(); throw; }
    }
    internal void VerifySource()
    {
        if (sourceHandles.Count < 2 || ArchiveReclamationNative.Identity(Source, true) != sourceIdentity || ArchiveReclamationNative.Identity(Parent) != parentIdentity) throw Refuse("source_identity");
        ArchiveReclamationNative.RequireCapability(Source, true);
    }
    private string CompanionName => ".nxtools-hz5-" + intent.RecoveryId.ToString("N") + ".journal";
    private void Initialize()
    {
        if (initialized) return;
        VerifySource();
        output.PrepareReclamationRoot(intent.Mode == "continue");
        rootIdentity = ArchiveReclamationNative.Identity(output.ReclamationRoot);
        if (OutputNative.Identity(output.ReclamationRoot).Volume != OutputNative.Identity(Source).Volume) throw Refuse("different_volume");
        if (intent.Mode == "continue")
        {
            OpenContinuation();
        }
        output.Reclamation = this;
        initialized = true;
    }
    private void CreateAuthority()
    {
        if (journal is not null) return;
        journal = OutputNative.Open(Parent, CompanionName, directory: false, exclusive: true);
        companionIdentity = ArchiveReclamationNative.Identity(journal);
        Append("header", JsonSerializer.SerializeToElement(new { mode = "stored_zip_whole_file_sparse", recovery_id = intent.RecoveryId,
            logical_id = intent.RecoveryId, source_identity = sourceIdentity, root_identity = rootIdentity, parent_identity = parentIdentity,
            companion_identity = companionIdentity, original_modified = OutputNative.Identity(Source).Modified, original_attributes = ArchiveMetadataNative.Attributes(Source),
            copy_motw = request.ExtractOptions!.CopyMotw, source_zone_hash = SourceZoneHash, cluster_bytes = cluster }));
    }
    private void BindMarker()
    {
        try {
            var bytes = JsonSerializer.SerializeToUtf8Bytes(new { schema = 1, recovery_id = intent.RecoveryId, companion_identity = companionIdentity, source_identity = sourceIdentity });
            if (bytes.Length > 1024) throw Refuse("marker_limit");
            Hit("hz5_before_marker");
            marker = ArchiveReclamationNative.OpenMarker(Source, true);
            RandomAccess.Write(marker, bytes, 0); OutputNative.Flush(marker);
            if (!ArchiveReclamationNative.ReadExact(marker, 0, bytes.Length).AsSpan().SequenceEqual(bytes)) throw Refuse("marker_readback");
            OutputNative.Flush(Source); Hit("hz5_marker_durable");
        }
        catch { poisoned = true; throw; }
    }
    private void OpenContinuation()
    {
        marker = ArchiveReclamationNative.OpenMarker(Source, false);
        var length = RandomAccess.GetLength(marker); if (length is < 1 or > 1024) throw Refuse("marker_invalid");
        var binding = Parse(ArchiveReclamationNative.ReadExact(marker, 0, (int)length));
        if (binding.GetProperty("schema").GetInt32() != 1 || binding.GetProperty("recovery_id").GetGuid() != intent.RecoveryId || binding.GetProperty("source_identity").GetString() != sourceIdentity) throw Refuse("marker_mismatch");
        journal = OutputNative.Open(Parent, CompanionName, directory: false, reclamationWrite: true);
        companionIdentity = ArchiveReclamationNative.Identity(journal);
        if (binding.GetProperty("companion_identity").GetString() != companionIdentity) throw Refuse("companion_identity");
        Load(); Diagnostic("recovery_accepted");
    }
    private string? SourceZoneHash => sourceZone is null ? null : Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(sourceZone));
    internal object? Execute(string action, JsonElement call, CancellationToken stop)
    {
        if (poisoned) throw Refuse("journal_unavailable");
        if (action == "hz5_info")
        {
            NoBody(call); Initialize();
            return new { schema = 1, recovery_id = intent.RecoveryId, cluster_bytes = cluster, journal_bytes = journalLength, source_identity = sourceIdentity, root_identity = rootIdentity };
        }
        if (!initialized) throw Refuse("info_required");
        if (action == "hz5_read")
        {
            var offset = call.GetProperty("offset").GetInt64(); var length = call.GetProperty("length").GetInt32();
            if (offset < 0 || length is < 1 or > 65536 || offset > journalLength || length > journalLength - offset) throw Refuse("read_limit");
            return Convert.ToBase64String(ArchiveReclamationNative.ReadExact(journal!, offset, length));
        }
        if (action == "hz5_complete")
        {
            NoBody(call);
            if (!sealedPlan || frontier != plans.Count || observed.Count != plans.Count) throw Refuse("outputs_unverified");
            foreach (var index in Enumerable.Range(0, plans.Count)) VerifyOutput(index, stop);
            Hit("hz5_before_complete");
            if (!completed) Append("completed_consumed", JsonSerializer.SerializeToElement(new { count = plans.Count }));
            Hit("hz5_after_complete"); Diagnostic("completed"); return new { };
        }
        var body = Body(call);
        if (action == "hz5_plan_entry")
        {
            if (intent.Mode != "start" || sealedPlan || completed) throw Refuse("plan_state");
            ValidatePlan(body); CreateAuthority(); Append("plan_entry", body); return new { };
        }
        if (action == "hz5_seal")
        {
            Fields(body, "count", "structure");
            if (intent.Mode != "start" || sealedPlan || body.GetProperty("count").GetInt32() != plans.Count || plans.Count == 0) throw Refuse("plan_state");
            var structure = Convert.FromBase64String(body.GetProperty("structure").GetString()!);
            if (structure.Length == 0 || structure.Length > 49152) throw Refuse("structure_limit");
            if (!plans.Any(p => U(p, "release_end") > U(p, "release_start"))) throw Refuse("no_reclaimable_payload");
            var future = plans.Sum(p => (long)JsonSerializer.SerializeToUtf8Bytes(p.GetProperty("path")).Length + 2048);
            if (journalLength + future + structure.Length * 2L + 1024 > MaximumJournalBytes) throw Refuse("journal_limit");
            if (!plans.Any(p => ArchiveReclamationNative.AllocatedIn(Source, U(p, "release_start"), U(p, "release_end")) != 0)) throw Refuse("no_reclaimable_allocation");
            foreach (var unitPlan in plans) {
                stop.ThrowIfCancellationRequested();
                output.PreflightReclamationTarget(unitPlan, parentIdentity, Path.GetFileName(request.Path));
            }
            Append("sealed", body); BindMarker(); return new { };
        }
        Fields(body, "index"); var unit = body.GetProperty("index").GetInt32();
        if (!sealedPlan || unit < 0 || unit >= plans.Count) throw Refuse("unit_invalid");
        switch (action)
        {
            case "hz5_select":
                if (completed || unit != frontier || partials.ContainsKey(unit) || ready.ContainsKey(unit) || pending.ContainsKey(unit) || Enumerable.Range(0, frontier).Any(i => !observed.Contains(i))) throw Refuse("select_state");
                selected = unit; return new { };
            case "hz5_observe":
                if (unit > frontier) throw Refuse("observe_state");
                if (!ready.ContainsKey(unit))
                {
                    if (partials.ContainsKey(unit)) throw Refuse("incomplete_partial_preserved");
                    return new { status = "absent", released_bytes = 0UL };
                }
                VerifyOutput(unit, stop); observed.Add(unit);
                if (pending.TryGetValue(unit, out var intentRecord) && !applied.ContainsKey(unit)) Reconcile(unit, intentRecord);
                return new { status = applied.ContainsKey(unit) ? "applied" : "published", released_bytes = applied.TryGetValue(unit, out var result) ? U(result, "released_bytes") : 0UL };
            case "hz5_pending":
                if (completed || unit != frontier || pending.ContainsKey(unit) || !ready.ContainsKey(unit)) throw Refuse("pending_state");
                RequirePriorObserved(); VerifyOutput(unit, stop); observed.Add(unit); VerifySource(); stop.ThrowIfCancellationRequested();
                var plan = plans[unit]; var before = ArchiveReclamationNative.AllocatedIn(Source, U(plan, "release_start"), U(plan, "release_end"));
                Append("pending", JsonSerializer.SerializeToElement(new { index = unit, interval_allocated_before = before,
                    source_allocated_before = ArchiveReclamationNative.Allocation(Source), free_before = OutputNative.AvailableSpace(Parent) }));
                Diagnostic("release_pending"); return new { };
            case "hz5_apply":
                if (applied.TryGetValue(unit, out var previous)) { VerifyOutput(unit, stop); return new { released_bytes = U(previous, "released_bytes") }; }
                if (completed || unit != frontier || !pending.TryGetValue(unit, out var durable)) throw Refuse("pending_required");
                RequirePriorObserved(); VerifyOutput(unit, stop); observed.Add(unit); VerifySource();
                var current = Reconcile(unit, durable); var range = plans[unit];
                stop.ThrowIfCancellationRequested(); Hit("hz5_before_punch"); stop.ThrowIfCancellationRequested();
                if (current != 0) ArchiveReclamationNative.Punch(Source, U(range, "release_start"), U(range, "release_end"));
                OutputNative.Flush(Source); Hit("hz5_after_punch");
                var after = ArchiveReclamationNative.AllocatedIn(Source, U(range, "release_start"), U(range, "release_end"));
                if (after != 0) throw Refuse("allocation_not_released");
                var sourceAfter = ArchiveReclamationNative.Allocation(Source);
                var sourceBefore = U(durable, "source_allocated_before");
                if (sourceAfter > sourceBefore || sourceBefore - sourceAfter > U(durable, "interval_allocated_before")) throw Refuse("allocation_observation_changed");
                var released = sourceBefore - sourceAfter;
                Hit("hz5_before_applied");
                Append("applied", JsonSerializer.SerializeToElement(new { index = unit, released_bytes = released, interval_allocated_after = after,
                    source_allocated_after = sourceAfter, free_after = OutputNative.AvailableSpace(Parent) }));
                Diagnostic("release_applied", released); return new { released_bytes = released };
            default: throw Refuse("action_invalid");
        }
    }
    private ulong Reconcile(int index, JsonElement record)
    {
        VerifySource(); var plan = plans[index];
        var actual = ArchiveReclamationNative.AllocatedIn(Source, U(plan, "release_start"), U(plan, "release_end"));
        var before = U(record, "interval_allocated_before");
        if (actual > before || actual != 0 && actual != before) throw Refuse("pending_allocation_changed");
        return actual;
    }
    internal void ValidateWrite(string path, long count, int length) {
        if (selected < 0 || selected != frontier || plans[selected].GetProperty("path").GetString() != path || count < 0 ||
            (ulong)count > U(plans[selected], "size") || (ulong)length > U(plans[selected], "size") - (ulong)count) throw Refuse("output_size");
    }
    private void RequirePriorObserved() { if (Enumerable.Range(0, frontier).Any(i => !observed.Contains(i))) throw Refuse("outputs_unverified"); }
    private void VerifyOutput(int index, CancellationToken stop) {
        if (index == frontier) selected = index;
        output.VerifyReclamationOutput(plans[index], partials[index], ready[index], stop, pending.ContainsKey(index));
    }
    internal void ValidateOutputAction(string action) {
        if (!initialized || !sealedPlan || action is "begin_overwrite" or "observe_target") throw Refuse("output_state");
    }
    internal void ValidateBegin(string path, bool overwrite)
    {
        if (!initialized || !sealedPlan || selected != frontier || selected < 0 || overwrite || partials.ContainsKey(selected) || plans[selected].GetProperty("path").GetString() != path) throw Refuse("output_plan_mismatch");
    }
    internal void PartialCreated(string path, string partialName, SafeFileHandle parent, SafeFileHandle file)
    {
        ValidateBegin(path, false);
        Append("partial", JsonSerializer.SerializeToElement(new { index = selected, path, partial_name = partialName,
            parent_identity = ArchiveReclamationNative.Identity(parent), identity = ArchiveReclamationNative.Identity(file) }));
    }
    internal void Ready(string path, SafeFileHandle file, ArchiveOutputMetadata? metadata, byte[] fingerprint, uint attributes, byte[]? motw, CancellationToken stop)
    {
        if (selected < 0 || selected != frontier || plans[selected].GetProperty("path").GetString() != path || !partials.ContainsKey(selected)) throw Refuse("ready_state");
        var plan = plans[selected]; var required = ArchiveOutputMetadata.Parse(plan.GetProperty("metadata"));
        if (required.Modified is { } modified && OutputNative.Identity(file).Modified != modified || required.ReadOnly is { } readOnly && ((attributes & 1) != 0) != readOnly ||
            required.Hidden is { } hidden && ((attributes & 2) != 0) != hidden) throw Refuse("output_metadata_mismatch");
        if (metadata != required || OutputNative.Identity(file).Size != checked((long)U(plan, "size")) || ArchiveReclamationNative.FileCrc32(file, stop) != plan.GetProperty("crc32").GetUInt32()) throw Refuse("output_integrity");
        var receipt = JsonSerializer.SerializeToElement(new { index = selected, identity = ArchiveReclamationNative.Identity(file), size = OutputNative.Identity(file).Size,
            modified = OutputNative.Identity(file).Modified, fingerprint = Convert.ToHexString(fingerprint), attributes,
            zone_identifier_hash = motw is null ? null : Convert.ToHexString(motw) });
        if (ready.TryGetValue(selected, out var saved)) { if (saved.GetRawText() != receipt.GetRawText()) throw Refuse("ready_mismatch"); return; }
        Append("ready", receipt);
    }
    internal void Published(string path) { if (selected >= 0 && plans[selected].GetProperty("path").GetString() == path) observed.Add(selected); }
    private static bool ValidComponent(string? name) {
        if (name is null) return false;
        try { ArchiveOutputSession.Validate(name); return true; } catch (ArchiveOutputException) { return false; }
    }
    private void ValidatePlan(JsonElement plan)
    {
        Fields(plan, "index", "ordinal", "path", "header_start", "payload_start", "payload_end", "record_end", "release_start", "release_end", "size", "crc32", "metadata");
        if (plans.Count >= 4096 || (ulong)plans.Count >= ((ArchiveAdmittedLimits)request.Limits).max_entries || plan.GetProperty("index").GetInt32() != plans.Count) throw Refuse("plan_limit");
        var path = plan.GetProperty("path").GetString();
        if (path is null || path.Length > 32767 || path.Split('/').Length > 64 || path.Split('/').Any(p => !ValidComponent(p))) throw Refuse("output_path");
        var hs = U(plan, "header_start"); var ps = U(plan, "payload_start"); var pe = U(plan, "payload_end"); var re = U(plan, "record_end");
        var rs = U(plan, "release_start"); var rend = U(plan, "release_end"); var size = U(plan, "size");
        if (hs > ps || ps > pe || pe > re || re > (ulong)OutputNative.Identity(Source).Size || size != pe - ps || size > ((ArchiveAdmittedLimits)request.Limits).max_entry_bytes ||
            rs > rend || rs % cluster != 0 || rend % cluster != 0 || (rs != rend && (rs < ps || rend > pe)) || rend > (ulong)OutputNative.Identity(Source).Size) throw Refuse("range_invalid");
        _ = plan.GetProperty("crc32").GetUInt32(); _ = ArchiveOutputMetadata.Parse(plan.GetProperty("metadata"));
        ulong total = size;
        foreach (var prior in plans)
        {
            total = checked(total + U(prior, "size")); var other = prior.GetProperty("path").GetString()!;
            if (path.Equals(other, StringComparison.OrdinalIgnoreCase) || path.StartsWith(other + "/", StringComparison.OrdinalIgnoreCase) || other.StartsWith(path + "/", StringComparison.OrdinalIgnoreCase) ||
                U(prior, "ordinal") == U(plan, "ordinal") || hs < U(prior, "record_end") && U(prior, "header_start") < re) throw Refuse("plan_overlap");
        }
        if (total > ((ArchiveAdmittedLimits)request.Limits).max_output_bytes) throw Refuse("plan_limit");
    }
    private void Append(string kind, JsonElement value)
    {
        if (poisoned || journal is null) throw Refuse("journal_unavailable");
        var bytes = JsonSerializer.SerializeToUtf8Bytes(new { schema = 1, sequence = sequence + 1, kind, value });
        if (retainedBytes + (ulong)bytes.Length + 512 > ((ArchiveAdmittedLimits)request.Limits).max_metadata_bytes - MaximumJournalBytes) throw Refuse("metadata_limit");
        if (bytes.Length is < 1 or > MaximumRecordBytes || journalLength > MaximumJournalBytes - bytes.Length - 8) throw Refuse("journal_limit");
        var frame = new byte[bytes.Length + 8]; BinaryPrimitives.WriteInt32LittleEndian(frame, bytes.Length); bytes.CopyTo(frame, 4); BinaryPrimitives.WriteInt32LittleEndian(frame.AsSpan(frame.Length - 4), bytes.Length);
        try
        {
            if (tornTail) {
                if (kind != "applied" || !pending.ContainsKey(frontier)) throw Refuse("torn_pending_required");
                RandomAccess.SetLength(journal, journalLength); OutputNative.Flush(journal); tornTail = false;
            }
            Hit("hz5_journal_" + kind + "_before_append");
            Hit("hz5_before_journal_append"); RandomAccess.Write(journal, frame, journalLength); Hit("hz5_after_journal_append");
            Hit("hz5_journal_" + kind + "_after_append");
            OutputNative.Flush(journal); Hit("hz5_after_journal_flush");
            Hit("hz5_journal_" + kind + "_after_flush");
            if (!ArchiveReclamationNative.ReadExact(journal, journalLength, frame.Length).AsSpan().SequenceEqual(frame)) throw Refuse("journal_readback");
            Hit("hz5_after_journal_readback");
            Hit("hz5_journal_" + kind + "_after_readback");
            Accept(kind, value, ++sequence); journalLength += frame.Length; Diagnostic("journal_durable");
        }
        catch { poisoned = true; throw; }
    }
    private void Load()
    {
        var length = RandomAccess.GetLength(journal!); if (length is < 8 or > MaximumJournalBytes) throw Refuse("journal_limit");
        while (journalLength < length)
        {
            if (length - journalLength < 8) { tornTail = true; break; }
            var size = BinaryPrimitives.ReadInt32LittleEndian(ArchiveReclamationNative.ReadExact(journal!, journalLength, 4));
            if (size is < 1 or > MaximumRecordBytes) throw Refuse("journal_frame");
            if (size + 8 > length - journalLength) { tornTail = true; break; }
            var frame = ArchiveReclamationNative.ReadExact(journal!, journalLength + 4, size + 4);
            if (BinaryPrimitives.ReadInt32LittleEndian(frame.AsSpan(size)) != size) throw Refuse("journal_frame");
            var envelope = Parse(frame.AsMemory(0, size)); Fields(envelope, "schema", "sequence", "kind", "value");
            if (envelope.GetProperty("schema").GetInt32() != 1 || envelope.GetProperty("sequence").GetUInt64() != sequence + 1) throw Refuse("journal_sequence");
            Accept(envelope.GetProperty("kind").GetString()!, envelope.GetProperty("value"), ++sequence); journalLength += size + 8;
        }
        if (!sealedPlan) throw Refuse("unsealed_preserved");
        if (tornTail && !pending.ContainsKey(frontier)) throw Refuse("journal_torn_tail_preserved");
    }
    private void Accept(string kind, JsonElement value, ulong recordSequence)
    {
        retainedBytes = checked(retainedBytes + (ulong)JsonSerializer.SerializeToUtf8Bytes(value).Length + 512);
        if (retainedBytes > ((ArchiveAdmittedLimits)request.Limits).max_metadata_bytes - MaximumJournalBytes) throw Refuse("metadata_limit");
        if (completed || value.ValueKind != JsonValueKind.Object) throw Refuse("journal_transition");
        if (recordSequence == 1)
        {
            if (kind != "header" || value.GetProperty("mode").GetString() != "stored_zip_whole_file_sparse" || value.GetProperty("recovery_id").GetGuid() != intent.RecoveryId ||
                value.GetProperty("source_identity").GetString() != sourceIdentity || value.GetProperty("root_identity").GetString() != rootIdentity || value.GetProperty("parent_identity").GetString() != parentIdentity ||
                value.GetProperty("companion_identity").GetString() != companionIdentity || U(value, "cluster_bytes") != cluster || value.GetProperty("copy_motw").GetBoolean() != request.ExtractOptions!.CopyMotw || value.GetProperty("source_zone_hash").GetString() != SourceZoneHash) throw Refuse("header_mismatch");
            return;
        }
        if (kind == "plan_entry") { if (sealedPlan) throw Refuse("journal_transition"); ValidatePlan(value); plans.Add(value.Clone()); return; }
        if (kind == "sealed")
        {
            if (sealedPlan || plans.Count == 0 || value.GetProperty("count").GetInt32() != plans.Count || !plans.Any(p => U(p, "release_end") > U(p, "release_start"))) throw Refuse("journal_transition");
            var structure = Convert.FromBase64String(value.GetProperty("structure").GetString()!); if (structure.Length is < 1 or > 49152) throw Refuse("structure_limit");
            sealedPlan = true; return;
        }
        if (!sealedPlan) throw Refuse("journal_transition");
        if (kind == "completed_consumed") { if (frontier != plans.Count) throw Refuse("journal_transition"); completed = true; return; }
        var index = value.GetProperty("index").GetInt32(); if (index != frontier || index >= plans.Count) throw Refuse("journal_transition");
        switch (kind)
        {
            case "partial":
                if (partials.ContainsKey(index) || value.GetProperty("path").GetString() != plans[index].GetProperty("path").GetString() || !ValidComponent(value.GetProperty("partial_name").GetString()) || !value.GetProperty("partial_name").GetString()!.StartsWith(".nxpart-", StringComparison.Ordinal)) throw Refuse("journal_transition");
                partials.Add(index, value.Clone()); break;
            case "ready":
                if (!partials.ContainsKey(index) || ready.ContainsKey(index) || value.GetProperty("identity").GetString() != partials[index].GetProperty("identity").GetString() || U(value, "size") != U(plans[index], "size")) throw Refuse("journal_transition");
                ready.Add(index, value.Clone()); break;
            case "pending":
                if (!ready.ContainsKey(index) || pending.ContainsKey(index) || U(value, "interval_allocated_before") > U(plans[index], "release_end") - U(plans[index], "release_start")) throw Refuse("journal_transition");
                pending.Add(index, value.Clone()); break;
            case "applied":
                if (!pending.ContainsKey(index) || applied.ContainsKey(index) || U(value, "interval_allocated_after") != 0 ||
                    U(value, "source_allocated_after") > U(pending[index], "source_allocated_before") ||
                    U(value, "released_bytes") != U(pending[index], "source_allocated_before") - U(value, "source_allocated_after") ||
                    U(value, "released_bytes") > U(pending[index], "interval_allocated_before")) throw Refuse("journal_transition");
                applied.Add(index, value.Clone()); frontier++; selected = -1; break;
            default: throw Refuse("journal_kind");
        }
    }
    private static JsonElement Body(JsonElement call)
    {
        var data = call.GetProperty("data");
        if (data.ValueKind != JsonValueKind.Array || data.GetArrayLength() is < 1 or > MaximumRecordBytes) throw Refuse("call_limit");
        var bytes = new byte[data.GetArrayLength()]; var i = 0; foreach (var b in data.EnumerateArray()) bytes[i++] = b.GetByte();
        return Parse(bytes);
    }
    private static void NoBody(JsonElement call) { if (call.TryGetProperty("data", out var data) && data.ValueKind != JsonValueKind.Null && !(data.ValueKind == JsonValueKind.Array && data.GetArrayLength() == 0)) throw Refuse("call_shape"); }
    private static JsonElement Parse(ReadOnlyMemory<byte> bytes)
    {
        using var document = JsonDocument.Parse(bytes, new JsonDocumentOptions { MaxDepth = 32 });
        void Unique(JsonElement e) { if (e.ValueKind == JsonValueKind.Object) { var names = new HashSet<string>(); foreach (var p in e.EnumerateObject()) { if (!names.Add(p.Name)) throw Refuse("record_duplicate"); Unique(p.Value); } } else if (e.ValueKind == JsonValueKind.Array) foreach (var item in e.EnumerateArray()) Unique(item); }
        Unique(document.RootElement); return document.RootElement.Clone();
    }
    private static void Fields(JsonElement value, params string[] fields) { if (value.ValueKind != JsonValueKind.Object || value.EnumerateObject().Count() != fields.Length || fields.Any(f => !value.TryGetProperty(f, out _))) throw Refuse("call_shape"); }
    private static ulong U(JsonElement e, string name) => e.GetProperty(name).GetUInt64();
    private static ArchiveOutputException Refuse(string stage) => new("reclamation_" + stage);
    private void Hit(string stage)
    {
#if DEBUG
        Checkpoint?.Invoke(stage);
#endif
    }
    private void Diagnostic(string stage, ulong? allocation = null)
    {
        try { log?.Write("info", "archive.reclamation." + stage, request.OperationId, new Dictionary<string, object?> { ["recovery_id"] = intent.RecoveryId,
            ["stage"] = stage, ["sequence"] = sequence, ["released_bytes"] = allocation, ["elapsed_ms"] = clock.ElapsedMilliseconds }); } catch { }
    }
    internal void ReportFailure(string stage, int? native) {
        if (failureReported) return; failureReported = true;
        try { log?.Write("warning", "archive.reclamation.failed", request.OperationId, new Dictionary<string, object?> {
            ["recovery_id"] = intent.RecoveryId, ["stage"] = stage, ["sequence"] = sequence, ["native_error_code"] = native, ["elapsed_ms"] = clock.ElapsedMilliseconds }); } catch { }
    }
    internal void ReportEnd(bool cancelled) { if (!completed && !failureReported) Diagnostic(cancelled ? "cancelled" : "incomplete"); }
    public void Dispose()
    {
        marker?.Dispose(); journal?.Dispose();
        if (sourceZone is not null) System.Security.Cryptography.CryptographicOperations.ZeroMemory(sourceZone); sourceZone = null;
        for (var i = sourceHandles.Count - 1; i >= 0; i--) sourceHandles[i].Dispose(); sourceHandles.Clear();
        activity?.Dispose(); activity = null;
    }
}
