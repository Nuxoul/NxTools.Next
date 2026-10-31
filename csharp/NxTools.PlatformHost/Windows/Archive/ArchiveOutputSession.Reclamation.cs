using System.Security.Cryptography;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveOutputSession
{
    internal ArchiveReclamationSession? Reclamation { get; set; }
    internal SafeFileHandle ReclamationAncestor => directories.LastOrDefault() ?? drive;
    internal SafeFileHandle ReclamationRoot => Root;
    internal void PrepareReclamationRoot(bool continuing)
    {
        Check();
        if (continuing && pendingRootParts.Length != 0) throw new ArchiveOutputException("reclamation_root_missing");
        if (root is null) CreatePreparedRoot();
        OutputNative.VerifyDirectory(Root); OutputNative.RequireDirectoryWritable(Root);
    }
    internal void PreflightReclamationTarget(JsonElement plan, string sourceParentIdentity, string sourceName)
    {
        Check();
        (SafeFileHandle Parent, string Name) target;
        try { target = ReclamationExistingParent(plan.GetProperty("path").GetString()!); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return; }
        if (ArchiveReclamationNative.Identity(target.Parent) == sourceParentIdentity && target.Name.Equals(sourceName, StringComparison.OrdinalIgnoreCase))
            throw new ArchiveOutputException("reclamation_source_output_overlap");
        try { using var present = OutputNative.Open(target.Parent, target.Name, directory: false, readOnly: true, anyKind: true); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return; }
        throw new ArchiveOutputException("reclamation_output_conflict");
    }
    private (SafeFileHandle Parent, string Name) ReclamationExistingParent(string relative)
    {
        var parts = relative.Split('/'); if (parts.Length > 64) throw new ArchiveOutputException("reclamation_output_path");
        foreach (var part in parts) Validate(part);
        var parent = Root;
        for (var index = 0; index < parts.Length - 1; index++)
        {
            var key = string.Join('/', parts.Take(index + 1));
            if (outputDirectories.TryGetValue(key, out var existing)) { parent = existing.Handle; continue; }
            if (directories.Count >= 16384 || outputDirectories.Count >= 16384) throw new ArchiveOutputException("reclamation_directory_limit");
            var handle = OutputNative.Open(parent, parts[index], directory: true);
            directories.Add(handle); outputDirectories.Add(key, new(handle, false)); parent = handle;
        }
        return (parent, parts[^1]);
    }
    private static void VerifyReclamationReceipt(SafeFileHandle file, JsonElement plan, JsonElement partial, JsonElement receipt, CancellationToken stop, bool allowClearedReadOnly = false)
    {
        var identity = ArchiveReclamationNative.Identity(file); var stamp = OutputNative.Identity(file);
        var expectedHash = receipt.GetProperty("fingerprint").GetString();
        var zone = receipt.GetProperty("zone_identifier_hash");
        var zoneHash = zone.ValueKind == JsonValueKind.Null ? null : Convert.FromHexString(zone.GetString()!);
        var metadata = ArchiveOutputMetadata.Parse(plan.GetProperty("metadata"));
        var attrs = ArchiveMetadataNative.Attributes(file);
        if (allowClearedReadOnly && (receipt.GetProperty("attributes").GetUInt32() & 1) != 0) attrs |= 1;
        if (expectedHash?.Length != 64 || zoneHash is { Length: not 32 } ||
            identity != partial.GetProperty("identity").GetString() || identity != receipt.GetProperty("identity").GetString() ||
            stamp.Size != receipt.GetProperty("size").GetInt64() || (ulong)stamp.Size != plan.GetProperty("size").GetUInt64() || stamp.Modified != receipt.GetProperty("modified").GetInt64() ||
            attrs != receipt.GetProperty("attributes").GetUInt32() || metadata.Modified is { } modified && stamp.Modified != modified ||
            metadata.ReadOnly is { } readOnly && ((attrs & 1) != 0) != readOnly || metadata.Hidden is { } hidden && ((attrs & 2) != 0) != hidden ||
            !ArchiveMotw.Matches(file, zoneHash) ||
            TargetIdentity(file, stop) != $"v2:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}:{stamp.Size:X}:{stamp.Modified:X}:{expectedHash}")
            throw new ArchiveOutputException("reclamation_output_changed");
    }
    private static SafeFileHandle OpenReclamationReady(SafeFileHandle parent, string name, JsonElement plan, JsonElement partial, JsonElement receipt, CancellationToken stop)
    {
        if ((receipt.GetProperty("attributes").GetUInt32() & 1) == 0) return OutputNative.Open(parent, name, directory: false);
        using (var owned = OutputNative.Open(parent, name, directory: false, metadataCleanup: true)) {
            VerifyReclamationReceipt(owned, plan, partial, receipt, stop);
            stop.ThrowIfCancellationRequested(); ArchiveMetadataNative.ClearOwnedReadOnly(owned);
        }
        SafeFileHandle? writable = null;
        try {
            writable = OutputNative.Open(parent, name, directory: false);
            VerifyReclamationReceipt(writable, plan, partial, receipt, stop, allowClearedReadOnly: true);
            ArchiveMetadataNative.Apply(writable, new(null, true, (receipt.GetProperty("attributes").GetUInt32() & 2) != 0));
            var result = writable; writable = null; return result;
        }
        catch {
            writable?.Dispose();
            // Only the original receipt authorizes attribute restoration; replacements are retained.
            using var restore = OutputNative.Open(parent, name, directory: false, metadataCleanup: true);
            VerifyReclamationReceipt(restore, plan, partial, receipt, CancellationToken.None, allowClearedReadOnly: true);
            ArchiveMetadataNative.Apply(restore, new(null, true, (receipt.GetProperty("attributes").GetUInt32() & 2) != 0));
            throw;
        }
    }
    internal void VerifyReclamationOutput(JsonElement plan, JsonElement partial, JsonElement receipt, CancellationToken stop, bool durablePublication)
    {
        Check(); stop.ThrowIfCancellationRequested();
        var relative = plan.GetProperty("path").GetString()!;
        var existing = transactions.FirstOrDefault(e => e.Relative == relative);
        if (existing is { Retained: { } retained }) { VerifyReclamationReceipt(retained, plan, partial, receipt, stop); return; }
        var target = ReclamationExistingParent(relative);
        if (ArchiveReclamationNative.Identity(target.Parent) != partial.GetProperty("parent_identity").GetString()) throw new ArchiveOutputException("reclamation_output_parent_changed");
        SafeFileHandle? final = null;
        try { final = OutputNative.Open(target.Parent, target.Name, directory: false, readOnly: true); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { }
        if (final is not null)
        {
            try
            {
                VerifyReclamationReceipt(final, plan, partial, receipt, stop);
                if (!durablePublication) {
                    final.Dispose(); final = null;
                    using (var writable = OpenReclamationReady(target.Parent, target.Name, plan, partial, receipt, stop)) {
                        VerifyReclamationReceipt(writable, plan, partial, receipt, stop); OutputNative.Flush(writable);
                    }
                    final = OutputNative.Open(target.Parent, target.Name, directory: false, readOnly: true);
                    VerifyReclamationReceipt(final, plan, partial, receipt, stop);
                }
                var stamp = OutputNative.Identity(final);
                var transaction = existing ?? new Partial(target.Parent, target.Name, final) { File = null, Relative = relative, Count = stamp.Size };
                transaction.Committed = true; transaction.State = ArchiveCommitState.Committed; transaction.Retained = final;
                transaction.NativeIdentity = ArchiveReclamationNative.Identity(final); transaction.Cleanup = "reclamation_retained";
                if (existing is null) { transactions.Add(transaction); entries.Add(Guid.NewGuid(), transaction); }
                final = null;
                return;
            }
            finally { final?.Dispose(); }
        }
        // Reuse CommitCore, including its original one no-replace rename. The exact recorded
        // ready partial is the only recovery input; incomplete or missing partials are retained/refused.
        var partialName = partial.GetProperty("partial_name").GetString()!; Validate(partialName);
        var file = OpenReclamationReady(target.Parent, partialName, plan, partial, receipt, stop);
        Partial recovered;
        try
        {
            VerifyReclamationReceipt(file, plan, partial, receipt, stop);
            recovered = new Partial(target.Parent, target.Name, file) { Relative = relative, PartialName = partialName, ReclamationReady = true,
                Count = OutputNative.Identity(file).Size, Metadata = ArchiveOutputMetadata.Parse(plan.GetProperty("metadata")), NativeIdentity = ArchiveReclamationNative.Identity(file) };
        }
        catch { file.Dispose(); throw; }
        var token = Guid.NewGuid(); entries.Add(token, recovered); transactions.Add(recovered);
        var result = Commit(token, stop);
        if (result.Category != "committed") throw new ArchiveOutputException("reclamation_ready_publication_failed", result.NativeCode, result.State);
        VerifyReclamationReceipt(recovered.Retained!, plan, partial, receipt, stop);
    }
}
