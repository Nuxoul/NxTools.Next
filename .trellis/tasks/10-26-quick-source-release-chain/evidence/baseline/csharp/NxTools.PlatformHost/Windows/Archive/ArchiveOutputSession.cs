using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using Microsoft.Win32.SafeHandles;
using NxTools.PlatformHost.Diagnostics;

namespace NxTools.PlatformHost.Windows.Archive;

internal enum ArchiveCommitState
{ Preparing, Verified, CommitInFlight, FailedBeforeCommit, Committed, CommittedButVerificationFailed, TargetChangedAfterCommit, CommitOutcomeUnknown }
internal sealed class ArchiveOutputException(string stage, int? nativeCode = null,
    ArchiveCommitState state = ArchiveCommitState.FailedBeforeCommit) : Exception(stage + ":" + nativeCode)
{
    internal string Stage { get; } = stage;
    internal int? NativeCode { get; } = nativeCode;
    internal ArchiveCommitState State { get; } = state;
}
internal sealed record ArchiveOutputResult(string Category, int? NativeCode = null,
    ArchiveCommitState State = ArchiveCommitState.Preparing);

// Worker-thread-only. Parents and temporary are held throughout publication. Only the
// target recheck handle closes before the single rename; this accepted TOCTOU is not ID-CAS.
internal sealed class ArchiveOutputSession : IDisposable
{
    private readonly SafeFileHandle drive;
    private SafeFileHandle? root;
    private readonly List<SafeFileHandle> directories = [];
    private string[] pendingRootParts = [];
    private readonly Dictionary<Guid, Partial> entries = [];
    private readonly List<Partial> transactions = [];
    private sealed record OutputDirectory(SafeFileHandle Handle, bool Owned) { internal bool Finalized; }
    private readonly Dictionary<string, OutputDirectory> outputDirectories = new(StringComparer.OrdinalIgnoreCase);
    private bool directoryMetadataStarted;
    private ReadOnlyMemory<byte>? zoneIdentifier;
    private readonly Guid operationId;
    internal Guid OperationId => operationId;
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private readonly IPlatformLog? log;
    private bool disposed;
    private bool metadataStarted;
    internal Action<string>? CommitCheckpointProbe { get; set; }
    internal Func<bool, uint, uint>? NamedStreamsFlagsProbe { get; set; }
    private sealed class Partial(SafeFileHandle parent, string name, SafeFileHandle file)
    {
        internal readonly SafeFileHandle Parent = parent;
        internal readonly string Name = name;
        internal SafeFileHandle? File = file;
        internal FileStream? Stream;
        internal long Count;
        internal bool Committed;
        internal byte[]? Hash;
        internal uint Attributes;
        internal byte[]? ZoneIdentifierHash;
        internal ulong Volume, FileIdLow, FileIdHigh;
        internal long Size, Modified;
        internal ArchiveCommitState State = ArchiveCommitState.Preparing;
        internal ArchiveOutputMetadata? Metadata;
        internal string? Expected;
        internal bool Replaced;
        internal uint? EditAttributes;
        internal bool CandidateVerified;
        internal bool Journaled = true;
        internal string? NativeIdentity;
        internal string Cleanup = "not_requested";
    }
    private ArchiveOutputSession(SafeFileHandle drive, Guid id, IPlatformLog? log)
    { this.drive = drive; operationId = id; this.log = log; }
    // Historical transactions survive exact-token cleanup; safe to read after disposal.
    internal IReadOnlyList<Protocol.ArchiveJobCommit> CommitJournal() => transactions.Where(entry=>entry.Journaled).Select((entry,index) =>
        new Protocol.ArchiveJobCommit((ulong)index,entry.State.ToString(),entry.Replaced,(ulong)Math.Max(0,entry.Count),
            entry.NativeIdentity,entry.State is ArchiveCommitState.CommitInFlight or ArchiveCommitState.CommitOutcomeUnknown ? null : entry.Committed,entry.Cleanup)).ToArray();
    internal void ExcludeEncoderSpool(Guid token) => Entry(token).Journaled = false;

    internal static ArchiveOutputSession Open(string absoluteRoot, Guid operationId, IPlatformLog? log = null)
    {
        var session = Prepare(absoluteRoot, operationId, log);
        try { session.CreatePreparedRoot(); return session; }
        catch { session.Dispose(); throw; }
    }

    // Retain the actual existing namespace without creating any missing components. ZIP
    // source admission must succeed against these same handles before CreatePreparedRoot.
    internal static ArchiveOutputSession Prepare(string absoluteRoot, Guid operationId, IPlatformLog? log = null)
    {
        if (operationId == Guid.Empty || string.IsNullOrEmpty(absoluteRoot) || absoluteRoot.Length > 32767 ||
            absoluteRoot.Length < 4 || !char.IsAsciiLetter(absoluteRoot[0]) || absoluteRoot[1] != ':' ||
            absoluteRoot[2] != '\\' || absoluteRoot.IndexOfAny(['/', '\0']) >= 0)
            throw new ArchiveOutputException("invalid_root");
        var parts = absoluteRoot[3..].Split('\\');
        if (parts.Length > 64 || parts.Any(p => p.Length == 0)) throw new ArchiveOutputException("invalid_root");
        foreach (var part in parts) Validate(part);
        // The only absolute native name is a fixed drive device. No full-path filesystem create.
        var drive = OutputNative.Open(null, "\\??\\" + char.ToUpperInvariant(absoluteRoot[0]) + ":\\", directory: true);
        var session = new ArchiveOutputSession(drive, operationId, log);
        try
        {
            foreach (var part in parts) {
                try { session.directories.Add(OutputNative.Open(session.directories.LastOrDefault() ?? drive, part, directory: true)); }
                catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { break; }
            }
            session.pendingRootParts = parts[session.directories.Count..];
            if (session.pendingRootParts.Length == 0) session.root = session.directories[^1];
            session.Diagnostic("prepared", "prepared", null);
            return session;
        }
        catch { session.Dispose(); throw; }
    }
    internal void CreatePreparedRoot(Action? checkpoint = null)
    {
        Check();
        checkpoint?.Invoke();
        foreach (var part in pendingRootParts) {
            directories.Add(OutputNative.OpenDirectory(directories.LastOrDefault() ?? drive, part));
            checkpoint?.Invoke();
        }
        pendingRootParts = [];
        root = directories[^1];
        Diagnostic("accepted", "accepted", null);
        // Deliberately retain created directories on failure. Only exact owned unpublished
        // partials are cleaned; no path-based directory rollback or global transaction.
    }
    private SafeFileHandle Root => root ?? throw new ArchiveOutputException("session_closed");
    internal (bool Missing,ulong Available) InspectDestination()
    {
        Check();var parent=directories.LastOrDefault()??drive;
        OutputNative.RequireDirectoryWritable(parent);
        return (pendingRootParts.Length!=0,OutputNative.AvailableSpace(parent));
    }
    private void Check() { if (disposed) throw new ArchiveOutputException("session_closed"); }
    internal void RequireNewVolumeFamily(string name, bool sevenz)
    {
        Check(); Validate(name);
        if (root is null) return;
        var prefix = sevenz ? name + "." : Path.GetFileNameWithoutExtension(name) + ".z";
        var count = 0;
        // Held native directory identity defines enumeration. Never recurse or scan without bound.
        foreach (var candidate in Directory.EnumerateFileSystemEntries(OutputNative.StableDirectoryPath(root)))
        {
            if (++count > 16384) throw new ArchiveOutputException("session_entry_limit");
            var component = Path.GetFileName(candidate);
            if (component.Equals(name, StringComparison.OrdinalIgnoreCase)) throw new ArchiveOutputException("target_exists");
            if (component.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                var suffix = component[prefix.Length..];
                if (suffix.Length > 0 && suffix.All(char.IsAsciiDigit)) throw new ArchiveOutputException("target_exists");
            }
        }
    }
    internal void CheckCreationCollisions(string name, Func<SafeFileHandle, bool> directoryIsSource,
        Func<SafeFileHandle, bool> fileIsSource)
    {
        Check();
        if (directoryIsSource(drive) || directories.Any(directoryIsSource))
            throw new ArchiveOutputException("source_output_collision");
        if (root is null) return; // Missing suffix: existing ancestors were still identity-checked.
        SafeFileHandle final;
        try { final = OutputNative.Open(Root, name, directory: false, readOnly: true); }
        catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return; }
        using (final) if (fileIsSource(final)) throw new ArchiveOutputException("source_output_collision");
    }
    internal static void Validate(string name)
    {
        if (name.Length is < 1 or > 255 || name is "." or ".." || name.EndsWith('.') || name.EndsWith(' ') ||
            name.IndexOfAny(['\\', '/', ':', '<', '>', '"', '|', '?', '*']) >= 0 ||
            name.EnumerateRunes().Any(r => r.Value < 32 || r.Value is >= 0xD800 and <= 0xDFFF) ||
            !string.Equals(Encoding.Unicode.GetString(Encoding.Unicode.GetBytes(name)), name, StringComparison.Ordinal) ||
            new[] { "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³",
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³" }
                .Contains(name.Split('.')[0].TrimEnd(' '), StringComparer.OrdinalIgnoreCase))
            throw new ArchiveOutputException("invalid_component");
    }
    internal void EnsureDirectory(string relative)
    {
        Check();
        if (string.IsNullOrEmpty(relative)) throw new ArchiveOutputException("invalid_component");
        var parts = relative.Split('/');
        if (parts.Length > 64 || directories.Count > 16384 - parts.Length) throw new ArchiveOutputException("session_entry_limit");
        foreach (var part in parts) Validate(part);
        var parent = Root;
        var prefix = "";
        foreach (var part in parts) {
            prefix = prefix.Length == 0 ? part : prefix + "/" + part;
            parent = TrackDirectory(parent, part, prefix);
        }
    }
    internal void CreateExclusiveDirectory(string name)
    {
        Check(); Validate(name);
        if (outputDirectories.Count >= 16384) throw new ArchiveOutputException("session_entry_limit");
        var handle=OutputNative.Open(Root,name,directory:true,create:true,exclusive:true);
        directories.Add(handle);outputDirectories.Add(name,new(handle,true));
    }
    private SafeFileHandle TrackDirectory(SafeFileHandle parent, string component, string relative)
    {
        if (outputDirectories.TryGetValue(relative, out var existing)) return existing.Handle;
        if (outputDirectories.Count >= 16384) throw new ArchiveOutputException("session_entry_limit");
        var next = OutputNative.OpenDirectory(parent, component, out var created);
        directories.Add(next); outputDirectories.Add(relative, new(next, created)); return next;
    }
    internal void FinalizeDirectoryMetadata(string relative, ArchiveOutputMetadata metadata, CancellationToken cancellationToken)
    {
        Check();
        if (!outputDirectories.TryGetValue(relative, out var directory) || directory.Finalized ||
            entries.Values.Any(entry => !entry.Committed)) throw new ArchiveOutputException("metadata_invalid_state");
        cancellationToken.ThrowIfCancellationRequested();
        directory.Finalized = true;
        if (!directory.Owned) return; // Existing/raced-existing directories are layout only.
        if (!directoryMetadataStarted) { directoryMetadataStarted = true; Diagnostic("directory_metadata_started", "directory_metadata", null); }
        CommitCheckpointProbe?.Invoke("before_directory_metadata");
        cancellationToken.ThrowIfCancellationRequested();
        OutputNative.VerifyDirectory(directory.Handle);
        ArchiveMetadataNative.Apply(directory.Handle, metadata);
        CommitCheckpointProbe?.Invoke("directory_metadata_applied");
    }
    private (SafeFileHandle Parent, string Name) ResolveParent(string relative)
    {
        Check();
        if (string.IsNullOrEmpty(relative)) throw new ArchiveOutputException("invalid_component");
        var parts=relative.Split('/');
        if(parts.Length>64||directories.Count>16384-parts.Length)throw new ArchiveOutputException("session_entry_limit");
        foreach(var part in parts)Validate(part);
        var parent=Root;
        for (var index = 0; index < parts.Length - 1; index++) parent = TrackDirectory(parent, parts[index], string.Join('/', parts.Take(index + 1)));
        return(parent,parts[^1]);
    }
    internal string CaptureTargetIdentity(string relative)
    {
        var target=ResolveParent(relative);
        using var handle=OutputNative.Open(target.Parent,target.Name,directory:false,readOnly:true,anyKind:true);
        return TargetSignature(handle);
    }
    internal (string Signature, long Size)? ObservePreparedCreateTarget(string name, CancellationToken cancellationToken)
    {
        Check(); Validate(name); cancellationToken.ThrowIfCancellationRequested();
        if (root is null) return null; // Read-only Prepare: missing destination creates nothing.
        try { return ObserveOrdinaryTarget(name, cancellationToken); }
        catch (ArchiveOutputException failure) {
            var stage = failure.Stage == "reparse_component" ? "target_reparse" : failure.Stage == "wrong_kind" ? "target_directory" :
                failure.NativeCode is 32 or 33 ? "target_busy" : failure.NativeCode == 5 ? "target_denied" : failure.Stage;
            throw new ArchiveOutputException(stage, failure.NativeCode);
        }
    }
    internal (string Signature, long Size)? ObserveOrdinaryTarget(string relative, CancellationToken cancellationToken = default)
    {
        var target = ResolveParent(relative);
        try {
            using var handle = OutputNative.Open(target.Parent, target.Name, directory: false, readOnly: true, anyKind: true);
            return (TargetSignature(handle, cancellationToken), OutputNative.Identity(handle).Size);
        } catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return null; }
    }
    internal static string TargetIdentity(SafeFileHandle handle, CancellationToken cancellationToken = default)
    {
        var stamp=OutputNative.Identity(handle);
        using var borrowed=new SafeFileHandle(handle.DangerousGetHandle(),false);
        using var stream=new FileStream(borrowed,FileAccess.Read,65536,false);
        stream.Position=0;
        using var digest=IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
        var buffer=new byte[65536];
        while(true) { cancellationToken.ThrowIfCancellationRequested(); var count=stream.Read(buffer); if(count==0)break; digest.AppendData(buffer,0,count); }
        var hash=Convert.ToHexString(digest.GetHashAndReset());
        return $"v2:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}:{stamp.Size:X}:{stamp.Modified:X}:{hash}";
    }
    private static string TargetSignature(SafeFileHandle handle, CancellationToken cancellationToken = default) =>
        TargetIdentity(handle, cancellationToken);
    internal Guid BeginFile(string relative, string? expectedTargetIdentity = null, bool overwrite = false)
    {
        Check();
        if (overwrite != (expectedTargetIdentity is not null)) throw new ArchiveOutputException("expected_identity_required");
        if (string.IsNullOrEmpty(relative)) throw new ArchiveOutputException("invalid_component");
        var parts = relative.Split('/');
        if (parts.Length > 64 || directories.Count > 16384 - parts.Length) throw new ArchiveOutputException("session_entry_limit");
        foreach (var part in parts) Validate(part);
        if (transactions.Count >= 4096) throw new ArchiveOutputException("session_entry_limit");
        var parent = Root;
        // Child directory handles remain live until session disposal; their parents cannot be renamed.
        for (var i = 0; i < parts.Length - 1; i++)
        {
            parent = TrackDirectory(parent, parts[i], string.Join('/', parts.Take(i + 1)));
        }
        if (overwrite) RecheckTarget(parent, parts[^1], expectedTargetIdentity!);
        var token = Guid.NewGuid();
        SafeFileHandle? partial = null;
        for (var i = 0; i < 4 && partial is null; i++)
        {
            try { partial = OutputNative.Open(parent, ".nxpart-" + operationId.ToString("N") + "-" + Guid.NewGuid().ToString("N"),
                directory: false, exclusive: true); }
            catch (ArchiveOutputException e) when (e.NativeCode is 80 or 183) { }
        }
        if (partial is null) throw new ArchiveOutputException("partial_collision");
        var entry = new Partial(parent, parts[^1], partial) {Expected=expectedTargetIdentity};
        entries.Add(token, entry); transactions.Add(entry); // Cleanup ownership precedes fallible identity query.
        var stamp = OutputNative.Identity(partial);
        entry.NativeIdentity=$"v1:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}";
        Diagnostic("started", "started", null);
        return token;
    }
    private Partial Entry(Guid token) => entries.TryGetValue(token, out var entry) ? entry : throw new ArchiveOutputException("unknown_token");
    internal bool Write(Guid token, ReadOnlyMemory<byte> chunk)
    {
        Check(); var entry = Entry(token);
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing || entry.CandidateVerified ||
            chunk.Length is < 1 or > 65536 || entry.Count > 34359738368L - chunk.Length)
            throw new ArchiveOutputException("write_limit");
        try
        {
            entry.Stream ??= new FileStream(entry.File, FileAccess.ReadWrite, 65536, false);
            entry.Stream.Write(chunk.Span);
            entry.Count += chunk.Length;
            return true;
        }
        catch (IOException e) { throw new ArchiveOutputException("write_file", e.HResult & 0xffff); }
    }
    internal byte[] ReadPartial(Guid token, long offset, int length)
    {
        Check(); var entry = Entry(token);
        if (entry.State != ArchiveCommitState.Preparing || entry.Committed || entry.File is null ||
            offset < 0 || offset > entry.Count || length is < 1 or > 65536) throw new ArchiveOutputException("read_limit");
        entry.Stream?.Flush();
        var buffer = new byte[(int)Math.Min(length, entry.Count - offset)];
        var count = RandomAccess.Read(entry.Stream?.SafeFileHandle ?? entry.File, buffer, offset);
        if (count != buffer.Length) throw new ArchiveOutputException("read_partial");
        return buffer;
    }
    internal void MarkCandidateVerified(Guid token)
    {
        Check(); var entry = Entry(token);
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing || entry.CandidateVerified)
            throw new ArchiveOutputException("invalid_state", state: entry.State);
        entry.Stream?.Flush();
        OutputNative.Flush(entry.Stream?.SafeFileHandle ?? entry.File);
        entry.CandidateVerified = true;
    }
    private static void RecheckTarget(SafeFileHandle parent, string name, string expected)
    {
        try
        {
            using var target = OutputNative.Open(parent, name, directory: false, readOnly: true, anyKind: true);
            if (TargetSignature(target) != expected) throw new ArchiveOutputException("target_changed");
        }
        catch (ArchiveOutputException failure)
        {
            var stage = failure.Stage == "reparse_component" ? "target_reparse" : failure.Stage == "wrong_kind" ? "target_directory" :
                failure.NativeCode is 2 or 3 ? "target_missing" : failure.NativeCode is 32 or 33 ? "target_busy" :
                failure.NativeCode == 5 ? "target_denied" : failure.Stage;
            throw new ArchiveOutputException(stage, failure.NativeCode);
        }
    }
    private static void CloseOutput(Partial entry)
    {
        try { entry.Stream?.Dispose(); }
        finally { entry.Stream = null; entry.File?.Dispose(); entry.File = null; }
    }
    internal void BindZoneIdentifier(ReadOnlyMemory<byte>? value) { Check(); zoneIdentifier = value; }
    internal void SetMetadata(Guid token, ArchiveOutputMetadata metadata)
    {
        Check(); var entry = Entry(token);
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing || entry.Metadata is not null || entry.CandidateVerified)
            throw new ArchiveOutputException("metadata_invalid_state");
        entry.Metadata = metadata;
    }
    // Edit-only handoff: caller supplies single-use evidence created only after all worker
    // source readers closed. This method rechecks the original held source/target, observes
    // cancellation, then releases only the source file immediately before CommitInFlight.
    internal ArchiveOutputResult CommitVerifiedEdit(Guid token, ArchiveSourceLease source,
        ArchiveSourceLease.EditReadCompletion readCompletion, CancellationToken cancellationToken,
        string? expectedSourceTargetIdentity = null)
    {
        Check(); var entry = Entry(token);
        if (entry.Expected is null || !entry.CandidateVerified)
            throw new ArchiveOutputException("candidate_not_verified", state: entry.State);
        if (readCompletion is null || !readCompletion.BelongsTo(source))
            throw new ArchiveOutputException(readCompletion is null ? "edit_reads_incomplete" : "edit_source_mismatch", state: entry.State);
        if (cancellationToken.IsCancellationRequested)
            return CancelBeforeCommit(token);
        var targetIdentity = expectedSourceTargetIdentity ?? entry.Expected;
        source.VerifyEditSource(targetIdentity);
        RecheckTarget(entry.Parent, entry.Name, entry.Expected);
        CommitCheckpointProbe?.Invoke("edit_source_rechecked");
        if (cancellationToken.IsCancellationRequested)
            return CancelBeforeCommit(token);
        zoneIdentifier = source.EditZoneIdentifier;
        entry.EditAttributes = source.EditAttributes;
        return CommitCore(token, cancellationToken, () => {
            source.VerifyEditSource(targetIdentity);
            cancellationToken.ThrowIfCancellationRequested();
            source.ReleaseEditFile(readCompletion);
        });
    }
    private ArchiveOutputResult CancelBeforeCommit(Guid token)
    {
        var entry = Entry(token);
        entry.State = ArchiveCommitState.FailedBeforeCommit;
        var cleanup = Cancel(token);
        return new("cancelled", cleanup.NativeCode, ArchiveCommitState.FailedBeforeCommit);
    }
    internal ArchiveOutputResult Commit(Guid token, CancellationToken cancellationToken = default) =>
        CommitCore(token, cancellationToken, null);
    private ArchiveOutputResult CommitCore(Guid token, CancellationToken cancellationToken, Action? beforeCommitInFlight)
    {
        Check(); var entry = Entry(token);
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing)
            throw new ArchiveOutputException("invalid_state", state: entry.State);
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            CommitCheckpointProbe?.Invoke("before_flush");
            entry.Stream?.Flush();
            var file = entry.Stream?.SafeFileHandle ?? entry.File;
            if (zoneIdentifier is { } mark) {
                CommitCheckpointProbe?.Invoke("before_motw");
                ArchiveMotw.EnsureSupported(file, writing: true, NamedStreamsFlagsProbe);
                cancellationToken.ThrowIfCancellationRequested();
                ArchiveMotw.Write(file, mark.Span);
            }
            if (entry.Metadata is { } metadata) {
                if (!metadataStarted) { metadataStarted = true; Diagnostic("metadata_started", "metadata", null); }
                CommitCheckpointProbe?.Invoke("before_metadata");
                cancellationToken.ThrowIfCancellationRequested();
                ArchiveMetadataNative.Apply(file, metadata);
                CommitCheckpointProbe?.Invoke("metadata_applied");
            }
            if (entry.EditAttributes is { } editAttributes)
                ArchiveMetadataNative.PreserveEditAttributes(file, editAttributes);
            OutputNative.Flush(file);
            OutputNative.VerifyDirectory(entry.Parent);
            var stamp = OutputNative.Identity(file);
            var fingerprint = Fingerprint(entry);
            var attributes = ArchiveMetadataNative.Attributes(file);
            entry.State = ArchiveCommitState.Verified;
            CommitCheckpointProbe?.Invoke("before_recheck");
            if (entry.Expected is not null) RecheckTarget(entry.Parent, entry.Name, entry.Expected);
            // RecheckTarget disposed ONLY its target handle. Parents/source/temp remain protected.
            CommitCheckpointProbe?.Invoke("target_rechecked");
            cancellationToken.ThrowIfCancellationRequested();
            beforeCommitInFlight?.Invoke();
            entry.State = ArchiveCommitState.CommitInFlight;
            CommitCheckpointProbe?.Invoke("commit_in_flight");
            if (entry.Expected is null) OutputNative.RenameNoReplace(file, entry.Parent, entry.Name);
            else OutputNative.RenameReplace(file, entry.Parent, entry.Name);
            // Linearization: native success means published, even if close/postcheck/cancel fails.
            entry.Committed = true; entry.State = ArchiveCommitState.Committed;
            entry.Cleanup = entry.Expected is null ? "retained" : "retained_replacement";
            entry.Replaced = entry.Expected is not null;
            entry.Volume = stamp.Volume; entry.FileIdLow = stamp.IdLow; entry.FileIdHigh = stamp.IdHigh;
            entry.Size = stamp.Size; entry.Modified = stamp.Modified; entry.Hash = fingerprint;
            entry.Attributes = attributes;
            entry.ZoneIdentifierHash = zoneIdentifier is { } committedMark ? SHA256.HashData(committedMark.Span) : null;
            CommitCheckpointProbe?.Invoke("native_published"); // Native fact precedes close/hash.
            CloseOutput(entry);
            CommitCheckpointProbe?.Invoke("native_committed");
            using var observed = OutputNative.Open(entry.Parent, entry.Name, directory: false, readOnly: true, anyKind: true);
            var current = OutputNative.Identity(observed);
            if (current.Volume != stamp.Volume || current.IdLow != stamp.IdLow || current.IdHigh != stamp.IdHigh || current.Size != stamp.Size ||
                TargetSignature(observed) != $"v2:{current.Volume:X}:{current.IdHigh:X16}{current.IdLow:X16}:{current.Size:X}:{current.Modified:X}:{Convert.ToHexString(fingerprint)}")
            {
                entry.State = ArchiveCommitState.TargetChangedAfterCommit;
                Diagnostic("failed", "target_changed_after_commit", null);
                return new("target_changed_after_commit", State: entry.State);
            }
            entry.Modified = current.Modified; // Last-write time settles when the writer closes.
            Diagnostic("completed", "committed", null);
            return new("committed", State: entry.State);
        }
        catch (OperationCanceledException) when (entry.State is ArchiveCommitState.Preparing or ArchiveCommitState.Verified)
        {
            return CancelBeforeCommit(token);
        }
        catch (ArchiveOutputException failure)
        {
            if (entry.Committed) return PostCommitFailure(entry, failure.Stage, failure.NativeCode);
            var uncertain = entry.State == ArchiveCommitState.CommitInFlight && failure.NativeCode is 53 or 54 or 59 or 64 or 121 or 1231 or 1232 or 1236;
            entry.State = uncertain ? ArchiveCommitState.CommitOutcomeUnknown : ArchiveCommitState.FailedBeforeCommit;
            var unsupported = failure.Stage == "replace_file" && failure.NativeCode is 1 or 50 or 120 or 124;
            var stage = uncertain ? "commit_outcome_unknown" : unsupported ? "overwrite_not_supported" : failure.Stage;
            Diagnostic("failed", stage, failure.NativeCode);
            if (failure.Stage == "target_exists" || uncertain || unsupported) return new(stage, failure.NativeCode, entry.State);
            throw new ArchiveOutputException(stage, failure.NativeCode, entry.State);
        }
        catch (IOException failure)
        {
            if (entry.Committed) return PostCommitFailure(entry, "postcommit_io", failure.HResult & 0xffff);
            entry.State = entry.State == ArchiveCommitState.CommitInFlight ? ArchiveCommitState.CommitOutcomeUnknown : ArchiveCommitState.FailedBeforeCommit;
            throw new ArchiveOutputException(entry.State == ArchiveCommitState.CommitOutcomeUnknown ? "commit_outcome_unknown" : "flush_or_commit", failure.HResult & 0xffff, entry.State);
        }
    }
    private ArchiveOutputResult PostCommitFailure(Partial entry, string stage, int? native)
    {
        entry.State = native is 2 or 3 || stage is "wrong_kind" or "reparse_component" ?
            ArchiveCommitState.TargetChangedAfterCommit : ArchiveCommitState.CommittedButVerificationFailed;
        var category = entry.State == ArchiveCommitState.TargetChangedAfterCommit ? "target_changed_after_commit" : "committed_but_verification_failed";
        Diagnostic("failed", category, native);
        return new(category, native, entry.State);
    }
    private static byte[] Fingerprint(Partial entry)
    {
        var handle = entry.Stream?.SafeFileHandle ?? entry.File!;
        using var borrowed = new SafeFileHandle(handle.DangerousGetHandle(), false);
        using var reader = new FileStream(borrowed, FileAccess.Read, 65536, false);
        reader.Position = 0;
        return SHA256.HashData(reader);
    }
    // Optional cleanup is strictly per-file. Partial deletion uses the held handle. Committed
    // cleanup reacquires exclusive file access and verifies identity, size, mtime AND hash first.
    internal ArchiveOutputResult Cancel(Guid token, bool cleanupNewOutput = false)
    {
        Check(); var entry = Entry(token);
        ArchiveOutputResult Finish(string category, int? native = null) {
            entry.Cleanup = category;
            return new(category, native, entry.State);
        }
        if (entry.State is ArchiveCommitState.CommitOutcomeUnknown or ArchiveCommitState.CommitInFlight)
            return Finish("retained_uncertain");
        if (entry.State is ArchiveCommitState.CommittedButVerificationFailed or ArchiveCommitState.TargetChangedAfterCommit)
            return Finish("retained_published");
        if (!entry.Committed)
        {
            try
            {
                var handle = entry.Stream?.SafeFileHandle ?? entry.File!;
                ArchiveMetadataNative.ClearOwnedReadOnly(handle);
                OutputNative.DeleteHandle(handle);
                entry.Stream?.Dispose(); entry.File?.Dispose();
                // Exact unpublished handle only; source and parent protection are untouched.
                entry.Stream = null; entry.File = null;
                entries.Remove(token);
                Diagnostic("cancelled", "partial_removed", null); return Finish("partial_removed");
            }
            catch (ArchiveOutputException e) { entry.Cleanup="cleanup_failed"; Diagnostic("failed", e.Stage, e.NativeCode); return new(e.Stage, e.NativeCode); }
        }
        if(entry.Replaced)return Finish("retained_replacement");
        if (!cleanupNewOutput) return Finish("retained");
        try
        {
            using var current = OutputNative.Open(entry.Parent, entry.Name, directory: false, metadataCleanup: true);
            var stamp = OutputNative.Identity(current);
            if (stamp.Volume != entry.Volume || stamp.IdLow != entry.FileIdLow || stamp.IdHigh != entry.FileIdHigh || stamp.Size != entry.Size ||
                stamp.Modified != entry.Modified || entry.Hash is null) return Finish("changed_retained");
            using var snapshot = new FileStream(new SafeFileHandle(current.DangerousGetHandle(), false), FileAccess.Read);
            if (!SHA256.HashData(snapshot).AsSpan().SequenceEqual(entry.Hash)) return Finish("changed_retained");
            if (ArchiveMetadataNative.Attributes(current) != entry.Attributes ||
                !ArchiveMotw.Matches(current, entry.ZoneIdentifierHash)) return Finish("changed_retained");
            ArchiveMetadataNative.ClearOwnedReadOnly(current);
            OutputNative.DeleteHandle(current);
            entries.Remove(token);
            Diagnostic("cancelled", "new_output_removed", null); return Finish("new_output_removed");
        }
        catch (ArchiveOutputException e) { Diagnostic("failed", e.Stage, e.NativeCode); return Finish("changed_retained", e.NativeCode); }
        catch (IOException) { return Finish("changed_retained"); }
    }
    public void Dispose()
    {
        if (disposed) return;
        foreach (var token in entries.Keys.ToArray())
        {
            if (entries[token].Committed) continue;
            try { Cancel(token); }
            catch (Exception) { entries[token].Cleanup = "cleanup_failed"; /* Never delete by path. */ }
        }
        foreach(var entry in entries.Values){entry.Stream?.Dispose();entry.File?.Dispose();}
        disposed = true;
        foreach (var item in directories) item.Dispose();
        drive.Dispose();
    }
    private void Diagnostic(string phase, string stage, int? native)
    {
        try { log?.Write("info", "archive.output." + phase, operationId,
            new Dictionary<string, object?> { ["stage"] = stage, ["native_error_code"] = native,
                ["elapsed_ms"] = clock.ElapsedMilliseconds }); }
        catch (Exception) { /* Never change the file result for diagnostic failure. */ }
    }
}

// SDK NT ABI: UNICODE_STRING (Length/MaximumLength in bytes), OBJECT_ATTRIBUTES (48 bytes
// x64), IO_STATUS_BLOCK (16 bytes x64). The unmanaged Unicode storage lives through each call.
// NtCreateFile with RootDirectory and one component is the filesystem security boundary.
internal static class OutputNative
{
    [StructLayout(LayoutKind.Sequential)] private struct UnicodeString
    { internal ushort Length, MaximumLength; internal IntPtr Buffer; }
    [StructLayout(LayoutKind.Sequential)] private struct ObjectAttributes
    { internal int Length; internal IntPtr RootDirectory, ObjectName; internal uint Attributes; internal IntPtr SecurityDescriptor, SecurityQualityOfService; }
    [StructLayout(LayoutKind.Sequential)] private struct IoStatusBlock
    { internal IntPtr Status, Information; }
    [StructLayout(LayoutKind.Sequential)] private struct ByHandleInfo
    {
        internal uint Attributes;
        internal System.Runtime.InteropServices.ComTypes.FILETIME Creation, Access, Write;
        internal uint Volume, SizeHigh, SizeLow, Links, IndexHigh, IndexLow;
    }
    [StructLayout(LayoutKind.Sequential)] private struct AttributeTag
    { internal uint Attributes, Tag; }
    // Windows SDK10.0.26100.0: minwinbase.h FILE_INFO_BY_HANDLE_CLASS, not NT FILE_INFORMATION_CLASS.
    private enum FileInfoClass { FileIdInfo = 18, FileRenameInfoEx = 22 }
    [Flags] private enum RenameFlags : uint { ReplaceIfExists = 1, PosixSemantics = 2 }
    [StructLayout(LayoutKind.Sequential)] private struct RenameInfo
    { internal RenameFlags Flags; internal IntPtr RootDirectory; internal uint FileNameLength; internal char FileName; }
    [StructLayout(LayoutKind.Sequential)] private struct FileIdInfo
    { internal ulong VolumeSerialNumber, IdLow, IdHigh; }
    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FullFileId(SafeFileHandle handle, FileInfoClass kind, out FileIdInfo info, uint size);
    [DllImport("kernel32.dll", EntryPoint = "SetFileInformationByHandle", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetFileInformation(SafeFileHandle handle, FileInfoClass kind, IntPtr info, uint size);
    [DllImport("kernel32.dll", EntryPoint = "GetFinalPathNameByHandleW", SetLastError = true, ExactSpelling = true, CharSet = CharSet.Unicode)]
    private static extern uint FinalPath(SafeFileHandle handle, StringBuilder path, uint length, uint flags);
    [DllImport("ntdll.dll", ExactSpelling = true)]
    private static extern int NtCreateFile(out SafeFileHandle handle, uint access, ref ObjectAttributes attributes,
        out IoStatusBlock io, IntPtr allocationSize, uint fileAttributes, uint shareAccess, uint disposition,
        uint options, IntPtr eaBuffer, uint eaLength);
    [DllImport("ntdll.dll", ExactSpelling = true)]
    private static extern int NtSetInformationFile(SafeFileHandle handle, out IoStatusBlock io, IntPtr information, uint length, int kind);
    [DllImport("ntdll.dll", ExactSpelling = true)]
    private static extern uint RtlNtStatusToDosError(int status);
    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FileTag(SafeFileHandle handle, int kind, out AttributeTag tag, uint size);
    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandle", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FileInfo(SafeFileHandle handle, out ByHandleInfo info);
    [DllImport("kernel32.dll", EntryPoint = "FlushFileBuffers", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool FlushBuffers(SafeFileHandle handle);
    [DllImport("kernel32.dll",EntryPoint="GetDiskFreeSpaceExW",SetLastError=true,CharSet=CharSet.Unicode)]
    [return:MarshalAs(UnmanagedType.Bool)]
    private static extern bool DiskSpace(string directory,out ulong available,out ulong total,out ulong free);
    internal static void RequireDirectoryWritable(SafeFileHandle directory)
    {
        try{var stable=StableDirectoryPath(directory);using var writable=Open(null,@"\??\"+stable[4..],directory:true,requireWrite:true);var before=Identity(directory);var after=Identity(writable);if(before.Volume!=after.Volume||before.IdLow!=after.IdLow||before.IdHigh!=after.IdHigh)throw new ArchiveOutputException("source_changed");}
        catch(ArchiveOutputException failure){throw new ArchiveOutputException("target_denied",failure.NativeCode);}
    }
    internal static ulong AvailableSpace(SafeFileHandle directory)
    {
        if(!DiskSpace(StableDirectoryPath(directory).TrimEnd('\\')+"\\",out var available,out _,out _))throw new ArchiveOutputException("target_denied",Marshal.GetLastWin32Error());
        return available;
    }
    internal static SafeFileHandle OpenDirectory(SafeFileHandle parent, string name) => OpenDirectory(parent, name, out _);
    internal static SafeFileHandle OpenDirectory(SafeFileHandle parent, string name, out bool created)
    {
        created = false;
        try { return Open(parent, name, directory: true); }
        catch (ArchiveOutputException e) when (e.NativeCode is 2 or 3) {
            try { var handle = Open(parent, name, directory: true, create: true, exclusive: true); created = true; return handle; }
            catch (ArchiveOutputException race) when (race.NativeCode is 80 or 183) { return Open(parent, name, directory: true); }
        }
    }
    internal static SafeFileHandle Open(SafeFileHandle? parent, string name, bool directory, bool create = false,
        bool exclusive = false, bool readOnly = false, bool anyKind = false, bool metadataCleanup = false, bool zoneStream = false, bool retireDirectory = false,bool requireWrite=false)
    {
        var bytes = Encoding.Unicode.GetBytes(name);
        if (bytes.Length > ushort.MaxValue - 2) throw new ArchiveOutputException("invalid_component");
        var str = Marshal.AllocHGlobal(bytes.Length + 2);
        var unicode = Marshal.AllocHGlobal(Marshal.SizeOf<UnicodeString>());
        try
        {
            Marshal.Copy(bytes, 0, str, bytes.Length); Marshal.WriteInt16(str, bytes.Length, 0);
            Marshal.StructureToPtr(new UnicodeString { Length = (ushort)bytes.Length,
                MaximumLength = (ushort)(bytes.Length + 2), Buffer = str }, unicode, false);
            var attrs = new ObjectAttributes { Length = Marshal.SizeOf<ObjectAttributes>(),
                RootDirectory = parent?.DangerousGetHandle() ?? IntPtr.Zero, ObjectName = unicode,
                Attributes = 0x40 }; // OBJ_CASE_INSENSITIVE
            // FILE_OPEN_REPARSE_POINT prevents resolving junctions on the opened component.
            const uint options = 0x00200000 | 0x00000020; // OPEN_REPARSE_POINT | SYNCHRONOUS_IO_NONALERT
            // Directory: SYNCHRONIZE | READ_ATTRIBUTES | LIST_DIRECTORY plus FILE_WRITE_DATA when creating;
            // file: SYNCHRONIZE | DELETE | READ_ATTRIBUTES | READ_DATA | WRITE_DATA | APPEND_DATA.
            var access = zoneStream ? readOnly ? 0x100081u : 0x100183u : directory ? 0x100081u | (create ? 0x102u : 0u) | (requireWrite ? 0x6u : 0u) | (retireDirectory ? 0x10000u : 0u) : readOnly ? 0x100081u : metadataCleanup ? 0x110181u : 0x110183u;
            var status = NtCreateFile(out var handle, access, ref attrs,
                out _, IntPtr.Zero, 0x80, zoneStream ? readOnly && !metadataCleanup ? 1u : 7u : directory ? 3u : readOnly ? 1u : 0u,
                exclusive ? 2u : create ? 3u : 1u, options | (directory ? 0x1u : anyKind ? 0u : 0x40u),
                IntPtr.Zero, 0);
            if (status < 0)
            {
                handle?.Dispose();
                var error = (int)RtlNtStatusToDosError(status);
                throw new ArchiveOutputException(error is 80 or 183 ? "target_exists" : "open_component", error);
            }
            try
            {
                Inspect(handle, directory);
                return handle;
            }
            catch { handle.Dispose(); throw; }
        }
        finally { Marshal.FreeHGlobal(unicode); Marshal.FreeHGlobal(str); }
    }
    internal static SafeFileHandle OpenZoneIdentifier(SafeFileHandle file, bool create, bool shareDelete = false)
    {
        var before = Identity(file);
        var stream = Open(file, ":Zone.Identifier", directory: false, exclusive: create, readOnly: !create, zoneStream: true, metadataCleanup: shareDelete);
        try {
            var observed = Identity(stream);
            if (before.Volume != observed.Volume || before.IdLow != observed.IdLow || before.IdHigh != observed.IdHigh)
                throw new ArchiveOutputException("motw_identity_changed");
            return stream;
        } catch { stream.Dispose(); throw; }
    }

    private static void Inspect(SafeFileHandle handle, bool directory)
    {
        if (!FileTag(handle, 9, out var tag, 8)) throw new ArchiveOutputException("inspect_tag", Marshal.GetLastWin32Error());
        if ((tag.Attributes & 0x400) != 0) throw new ArchiveOutputException("reparse_component");
        if (((tag.Attributes & 0x10) != 0) != directory) throw new ArchiveOutputException("wrong_kind");
        _ = Identity(handle);
    }
    internal static void VerifyDirectory(SafeFileHandle handle) => Inspect(handle, true);
    internal static (ulong Volume, ulong IdLow, ulong IdHigh, long Size, long Modified) Identity(SafeFileHandle handle)
    {
        if (!FileInfo(handle, out var info)) throw new ArchiveOutputException("inspect_identity", Marshal.GetLastWin32Error());
        if (!FullFileId(handle, FileInfoClass.FileIdInfo, out var id, (uint)Marshal.SizeOf<FileIdInfo>()))
            throw new ArchiveOutputException("inspect_full_identity", Marshal.GetLastWin32Error());
        return (id.VolumeSerialNumber, id.IdLow, id.IdHigh,
            ((long)info.SizeHigh << 32) | info.SizeLow, ((long)info.Write.dwHighDateTime << 32) | (uint)info.Write.dwLowDateTime);
    }
    internal static void Flush(SafeFileHandle handle)
    { if (!FlushBuffers(handle)) throw new ArchiveOutputException("flush_file", Marshal.GetLastWin32Error()); }
    internal static void RenameNoReplace(SafeFileHandle source, SafeFileHandle parent, string name)
    {
        var bytes = Encoding.Unicode.GetBytes(name);
        // FILE_RENAME_INFORMATION: BOOLEAN ReplaceIfExists, 7 pad, HANDLE RootDirectory,
        // ULONG FileNameLength, WCHAR FileName[1]. NtSetInformationFile class10.
        var block = Marshal.AllocHGlobal(20 + bytes.Length);
        try
        {
            Marshal.Copy(new byte[20 + bytes.Length], 0, block, 20 + bytes.Length);
            Marshal.WriteIntPtr(block, 8, parent.DangerousGetHandle());
            Marshal.WriteInt32(block, 16, bytes.Length);
            Marshal.Copy(bytes, 0, IntPtr.Add(block, 20), bytes.Length);
            var status = NtSetInformationFile(source, out _, block, (uint)(20 + bytes.Length), 10);
            if (status < 0)
            {
                var error = (int)RtlNtStatusToDosError(status);
                throw new ArchiveOutputException(error is 80 or 183 ? "target_exists" : "rename_file", error);
            }
        }
        finally { Marshal.FreeHGlobal(block); }
    }
    internal static string StableDirectoryPath(SafeFileHandle parent)
    {
        // Resolve only the held directory to a volume-GUID path; held ancestors cannot move/reparse.
        var parentPath = new StringBuilder(32768);
        var length = FinalPath(parent, parentPath, (uint)parentPath.Capacity, 1); // VOLUME_NAME_GUID
        if (length == 0) throw new ArchiveOutputException("resolve_commit_parent", Marshal.GetLastWin32Error());
        if (length >= parentPath.Capacity || !parentPath.ToString().StartsWith(@"\\?\Volume{", StringComparison.Ordinal))
            throw new ArchiveOutputException("invalid_commit_parent");
        return parentPath.ToString();
    }
    internal static void RenameReplace(SafeFileHandle source, SafeFileHandle parent, string name)
    {
        // Win32 wrapper on this workstation rejects relative RootDirectory with87.
        var bytes = Encoding.Unicode.GetBytes(StableDirectoryPath(parent).TrimEnd('\\') + "\\" + name);
        var nameOffset = Marshal.OffsetOf<RenameInfo>(nameof(RenameInfo.FileName)).ToInt32();
        // Win32's path conversion reads a terminated WCHAR name despite the explicit length.
        // Keep FileNameLength excluding NUL, but allocate/write the terminator inside the buffer.
        var size = Math.Max(Marshal.SizeOf<RenameInfo>(), checked(nameOffset + bytes.Length + sizeof(char)));
        var block = Marshal.AllocHGlobal(size);
        try
        {
            Marshal.Copy(new byte[size], 0, block, size);
            Marshal.StructureToPtr(new RenameInfo { Flags = RenameFlags.ReplaceIfExists | RenameFlags.PosixSemantics,
                RootDirectory = IntPtr.Zero, FileNameLength = (uint)bytes.Length }, block, false);
            Marshal.Copy(bytes, 0, IntPtr.Add(block, nameOffset), bytes.Length);
            Marshal.WriteInt16(block, nameOffset + bytes.Length, 0);
            if (!SetFileInformation(source, FileInfoClass.FileRenameInfoEx, block, (uint)size))
                throw new ArchiveOutputException("replace_file", Marshal.GetLastWin32Error());
        }
        finally { Marshal.FreeHGlobal(block); GC.KeepAlive(parent); }
    }
    internal static void DeleteHandle(SafeFileHandle file)
    {
        // FILE_DISPOSITION_INFORMATION class13; deletion targets this opened file ID, not a path.
        var memory = Marshal.AllocHGlobal(1);
        try
        {
            Marshal.WriteByte(memory, 1);
            var status = NtSetInformationFile(file, out _, memory, 1, 13);
            if (status < 0) throw new ArchiveOutputException("delete_file", (int)RtlNtStatusToDosError(status));
        }
        finally { Marshal.FreeHGlobal(memory); }
    }
}
