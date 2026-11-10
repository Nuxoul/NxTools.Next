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
internal sealed partial class ArchiveOutputSession : IDisposable
{
    private readonly SafeFileHandle drive;
    private SafeFileHandle? root;
    private readonly List<SafeFileHandle> directories = [];
    private string[] pendingRootParts = [];
    private readonly Dictionary<Guid, Partial> entries = [];
    private readonly object extractionGate = new();
    private int pendingExtractionBegins;
    private readonly List<Partial> transactions = [];
    private sealed record OutputDirectory(SafeFileHandle Handle, bool Owned) { internal bool Finalized; internal bool Completed; }
    private readonly Dictionary<string, OutputDirectory> outputDirectories = new(StringComparer.OrdinalIgnoreCase);
    private bool directoryMetadataStarted;
    private bool directoryPublished, directoryPublicationUnknown;
    // Read-only history after real cleanup; never authorizes commit/rollback or replaces the file journal.
    internal bool? OutputPublished() { lock (extractionGate) return directoryPublished || transactions.Any(entry => entry.Journaled && entry.Committed) ? true
        : !disposed || directoryPublicationUnknown || transactions.Any(entry => entry.DirectExtraction && entry.Cleanup == "cleanup_failed") ||
            transactions.Any(entry => entry.Journaled && entry.State is ArchiveCommitState.CommitInFlight or ArchiveCommitState.CommitOutcomeUnknown) ? null : false; }
    private void MarkDirectoryPublished() => directoryPublished = true;
    private SafeFileHandle OpenOutputDirectory(SafeFileHandle parent, string name, out bool created)
    {
        try { return OutputNative.OpenDirectory(parent, name, out created, MarkDirectoryPublished); }
        catch { directoryPublicationUnknown = true; throw; }
    }
    private ReadOnlyMemory<byte>? zoneIdentifier;
    private readonly Guid operationId;
    internal Guid OperationId => operationId;
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private readonly IPlatformLog? log;
    private bool disposed;
    private bool metadataStarted;
    internal Action<string>? CommitCheckpointProbe { get; set; }
    internal Action<string>? BeginCheckpointProbe { get; set; }  // Passive fixture observation before native final-name open.
    internal ArchiveNativeReadObservation ReadObservation { get; } = new();
    private long nativeWriteTicks, fingerprintTicks, postpublicationTicks;
    internal double NativeWriteMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref nativeWriteTicks)).TotalMilliseconds;
    internal double FingerprintMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref fingerprintTicks)).TotalMilliseconds;
    internal double PostpublicationMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref postpublicationTicks)).TotalMilliseconds;
    private long parentResolutionTicks, finalOpenTicks;
    internal double ParentResolutionMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref parentResolutionTicks)).TotalMilliseconds;
    internal double FinalOpenMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref finalOpenTicks)).TotalMilliseconds;
    private readonly OutputNative.OpenObservation extractionOpenObservation = new();
    internal double NativeCreateMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref extractionOpenObservation.CreateTicks)).TotalMilliseconds;
    internal double NativeOpenInspectionMs => Stopwatch.GetElapsedTime(0, Interlocked.Read(ref extractionOpenObservation.InspectionTicks)).TotalMilliseconds;
    internal Func<bool, uint, uint>? NamedStreamsFlagsProbe { get; set; }
    private sealed class Partial(SafeFileHandle parent, string name, SafeFileHandle file)
    {
        internal readonly SafeFileHandle Parent = parent;
        internal readonly string Name = name;
        internal string Relative = "";
        internal SafeFileHandle? File = file;
        internal bool DirectExtraction;
        internal SafeFileHandle? Retained;
        internal string PartialName = "";
        internal bool ReclamationReady;
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
        // SHA-256 of exactly the bytes appended through Write. The partial is opened without any
        // sharing, so for a new-file publication this equals a full read of the closed candidate.
        internal IncrementalHash? Digest;
        internal bool Journaled = true;
        internal string? NativeIdentity;
        internal string Cleanup = "not_requested";
    }
    private ArchiveOutputSession(SafeFileHandle drive, Guid id, IPlatformLog? log)
    { this.drive = drive; operationId = id; this.log = log; }
    // Historical transactions survive exact-token cleanup; safe to read after disposal.
    internal IReadOnlyList<Protocol.ArchiveJobCommit> CommitJournal(int maximum = int.MaxValue) { lock (extractionGate) return maximum == int.MaxValue
        ? JournalFacts().ToArray()
        : JournalFacts().Where(fact => !OrdinaryCommit(fact)).Concat(JournalFacts().Where(OrdinaryCommit))
            .Take(maximum).OrderBy(fact => fact.Ordinal).ToArray(); }
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
            directories.Add(OpenOutputDirectory(directories.LastOrDefault() ?? drive, part, out _));
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
    internal (bool Missing, ulong Available, ulong Allocation, string TargetIdentity, string VolumeIdentity, uint MissingDirectories) ObserveSpace()
    {
        Check(); var parent=directories.LastOrDefault()??drive;
        OutputNative.RequireDirectoryWritable(parent);
        var before=OutputNative.Identity(parent);
        var available=OutputNative.AvailableSpace(parent);
        var (allocation,volume)=OutputNative.AllocationSpace(parent);
        var after=OutputNative.Identity(parent);
        if(before.Volume!=after.Volume || before.IdLow!=after.IdLow || before.IdHigh!=after.IdHigh)
            throw new ArchiveOutputException("target_changed");
        // Bind the exact validated missing suffix, not merely its depth; no plaintext path
        // enters the opaque wire identity. Full held ancestor ID remains authoritative.
        var suffix=Convert.ToHexString(SHA256.HashData(Encoding.Unicode.GetBytes(string.Join('\\',pendingRootParts))));
        return (pendingRootParts.Length!=0,available,allocation,
            $"v1:{before.Volume:X}:{before.IdHigh:X16}{before.IdLow:X16}:{suffix}",volume,checked((uint)pendingRootParts.Length));
    }
    private void Check() { if (disposed) throw new ArchiveOutputException("session_closed"); }
    // Observe the held namespace only; a missing suffix does not authorize creating it.
    internal (bool Missing, string[] Names) ObserveCreateNames(CancellationToken cancellation = default)
    {
        Check();
        cancellation.ThrowIfCancellationRequested();
        if (root is null) { OutputNative.VerifyDirectory(directories.LastOrDefault() ?? drive); return (true, []); }
        var before = OutputNative.Identity(root);
        var names = new List<string>();
        long bytes = 0;
        foreach (var entry in Directory.EnumerateFileSystemEntries(OutputNative.StableDirectoryPath(root)))
        {
            cancellation.ThrowIfCancellationRequested();
            if (names.Count == 16384) throw new ArchiveOutputException("destination_limit");
            var name = Path.GetFileName(entry);
            bytes += Encoding.UTF8.GetByteCount(name);
            if (bytes > 262144) throw new ArchiveOutputException("destination_limit");
            names.Add(name);
        }
        var after = OutputNative.Identity(root);
        if (before.Volume != after.Volume || before.IdLow != after.IdLow || before.IdHigh != after.IdHigh)
            throw new ArchiveOutputException("destination_unavailable");
        return (false, names.ToArray());
    }
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
        // Only a completed worker directory request is output. Prepared roots/wrappers and
        // ancestry opened for an uncommitted file are not evidence of archive results.
        OutputNative.VerifyDirectory(parent);
        outputDirectories[relative].Completed = true;
        directoryPublished = true; // Completed directory output is real even without a file transaction.
    }
    internal void CreateExclusiveDirectory(string name)
    {
        Check(); Validate(name);
        if (outputDirectories.Count >= 16384) throw new ArchiveOutputException("session_entry_limit");
        SafeFileHandle handle;
        try { handle=OutputNative.Open(Root,name,directory:true,create:true,exclusive:true,createdCheckpoint:MarkDirectoryPublished); }
        catch { directoryPublicationUnknown=true; throw; }
        directories.Add(handle);outputDirectories.Add(name,new(handle,true));
    }
    private SafeFileHandle TrackDirectory(SafeFileHandle parent, string component, string relative)
    {
        if (outputDirectories.TryGetValue(relative, out var existing)) return existing.Handle;
        if (outputDirectories.Count >= 16384) throw new ArchiveOutputException("session_entry_limit");
        var next = OpenOutputDirectory(parent, component, out var created);
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
        var started = Stopwatch.GetTimestamp();
        try { lock (extractionGate) return ResolveParentCore(relative); }
        finally { Interlocked.Add(ref parentResolutionTicks, Stopwatch.GetTimestamp() - started); }
    }
    private (SafeFileHandle Parent, string Name) ResolveParentCore(string relative)
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
    internal string CaptureTargetIdentity(string relative, CancellationToken cancellationToken = default)
    {
        var target=ResolveParent(relative);
        using var handle=OutputNative.Open(target.Parent,target.Name,directory:false,readOnly:true,anyKind:true);
        return TargetSignature(handle, cancellationToken);
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
    internal static string TargetIdentity(SafeFileHandle handle, CancellationToken cancellationToken = default,
        ArchiveNativeReadObservation? observation = null, ArchiveNativeReadStage stage = ArchiveNativeReadStage.TargetAdmission,
        ArchiveNativeReadKind kind = ArchiveNativeReadKind.Target)
    {
        var stamp=OutputNative.Identity(handle);
        using var borrowed=new SafeFileHandle(handle.DangerousGetHandle(),false);
        using var stream=new FileStream(borrowed,FileAccess.Read,65536,false);
        stream.Position=0;
        var hash=Convert.ToHexString((observation ?? new ArchiveNativeReadObservation()).Hash(stream, stage, kind, cancellationToken));
        return $"v2:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}:{stamp.Size:X}:{stamp.Modified:X}:{hash}";
    }
    private string TargetSignature(SafeFileHandle handle, CancellationToken cancellationToken = default,
        ArchiveNativeReadStage stage = ArchiveNativeReadStage.TargetAdmission, ArchiveNativeReadKind kind = ArchiveNativeReadKind.Target)
    {
        if (stage != ArchiveNativeReadStage.OutputPostpublication)
            return TargetIdentity(handle, cancellationToken, ReadObservation, stage, kind);
        var started = Stopwatch.GetTimestamp();
        try { return TargetIdentity(handle, cancellationToken, ReadObservation, stage, kind); }
        finally { Interlocked.Add(ref postpublicationTicks, Stopwatch.GetTimestamp() - started); }
    }
    internal Guid BeginFile(string relative, string? expectedTargetIdentity = null, bool overwrite = false,
        CancellationToken cancellationToken = default)
    {
        Check();
        Reclamation?.ValidateBegin(relative, overwrite);
        if (overwrite != (expectedTargetIdentity is not null)) throw new ArchiveOutputException("expected_identity_required");
        if (string.IsNullOrEmpty(relative)) throw new ArchiveOutputException("invalid_component");
        var parts = relative.Split('/');
        if (parts.Length > 64 || directories.Count > 16384 - parts.Length) throw new ArchiveOutputException("session_entry_limit");
        foreach (var part in parts) Validate(part);
        AdmitTransaction(relative, expectedTargetIdentity);
        var parent = Root;
        // Child directory handles remain live until session disposal; their parents cannot be renamed.
        for (var i = 0; i < parts.Length - 1; i++)
        {
            parent = TrackDirectory(parent, parts[i], string.Join('/', parts.Take(i + 1)));
        }
        if (Reclamation is not null && ObserveOrdinaryTarget(relative) is not null) throw new ArchiveOutputException("reclamation_target_exists");
        if (overwrite) RecheckTarget(parent, parts[^1], expectedTargetIdentity!, cancellationToken: cancellationToken);
        var token = Guid.NewGuid();
        SafeFileHandle? partial = null;
        string partialName = "";
        for (var i = 0; i < 4 && partial is null; i++)
        {
            try { partialName = ".nxpart-" + operationId.ToString("N") + "-" + Guid.NewGuid().ToString("N");
                partial = OutputNative.Open(parent, partialName, directory: false, exclusive: true); }
            catch (ArchiveOutputException e) when (e.NativeCode is 80 or 183) { }
        }
        if (partial is null) throw new ArchiveOutputException("partial_collision");
        var entry = new Partial(parent, parts[^1], partial) {Expected=expectedTargetIdentity, Relative=relative, PartialName=partialName};
        entries.Add(token, entry); transactions.Add(entry); activeFiles++; // Cleanup ownership precedes fallible identity query.
        var stamp = OutputNative.Identity(partial);
        entry.NativeIdentity=$"v1:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}";
        Reclamation?.PartialCreated(relative, partialName, parent, partial);
        Diagnostic("started", "started", null);
        return token;
    }
    private Partial Entry(Guid token) { lock (extractionGate) return entries.TryGetValue(token, out var entry) ? entry : throw new ArchiveOutputException("unknown_token"); }
    internal bool Write(Guid token, ReadOnlyMemory<byte> chunk) { var entry = Entry(token); lock (entry) return WriteCore(token, chunk, entry); }
    private bool WriteCore(Guid token, ReadOnlyMemory<byte> chunk, Partial entry)
    {
        Check();
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing || entry.CandidateVerified ||
            chunk.Length is < 1 or > 65536 || entry.Count > 34359738368L - chunk.Length)
            throw new ArchiveOutputException("write_limit");
        Reclamation?.ValidateWrite(entry.Relative, entry.Count, chunk.Length);
        if (entry.DirectExtraction) return WriteExtractedFile(token, entry, chunk);
        try
        {
            entry.Stream ??= new FileStream(entry.File, FileAccess.ReadWrite, 65536, false);
            var started = Stopwatch.GetTimestamp();
            try { entry.Stream.Write(chunk.Span); (entry.Digest ??= IncrementalHash.CreateHash(HashAlgorithmName.SHA256)).AppendData(chunk.Span); }
            finally { Interlocked.Add(ref nativeWriteTicks, Stopwatch.GetTimestamp() - started); }
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
        ReadObservation.PartialRead(count);
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
    private void RecheckTarget(SafeFileHandle parent, string name, string expected,
        ArchiveNativeReadStage stage = ArchiveNativeReadStage.TargetBegin, CancellationToken cancellationToken = default,
        ArchiveSourceLease? editSource = null, string? expectedSourceTargetIdentity = null)
    {
        try
        {
            using var target = OutputNative.Open(parent, name, directory: false, readOnly: true, anyKind: true);
            if (editSource is not null)
                editSource.VerifyEditSource(expectedSourceTargetIdentity ?? expected, stage, cancellationToken, target, expected);
            else if (TargetSignature(target, cancellationToken, stage) != expected) throw new ArchiveOutputException("target_changed");
        }
        catch (ArchiveOutputException failure)
        {
            var failureStage = failure.Stage == "reparse_component" ? "target_reparse" : failure.Stage == "wrong_kind" ? "target_directory" :
                failure.NativeCode is 2 or 3 ? "target_missing" : failure.NativeCode is 32 or 33 ? "target_busy" :
                failure.NativeCode == 5 ? "target_denied" : failure.Stage;
            throw new ArchiveOutputException(failureStage, failure.NativeCode);
        }
    }
    private void CloseOutput(Partial entry)
    {
        if (entry.File is null) return;
        try { entry.Stream?.Dispose(); }
        finally {
            try { entry.Stream = null; entry.File?.Dispose(); entry.File = null; }
            finally { lock (extractionGate) activeFiles--; }
        }
    }
    internal void BindZoneIdentifier(ReadOnlyMemory<byte>? value) { Check(); zoneIdentifier = value; }
    internal void SetMetadata(Guid token, ArchiveOutputMetadata metadata) { var entry = Entry(token); lock (entry) SetMetadataCore(entry, metadata); }
    private void SetMetadataCore(Partial entry, ArchiveOutputMetadata metadata)
    {
        Check();
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
        try { RecheckTarget(entry.Parent, entry.Name, entry.Expected, ArchiveNativeReadStage.EditHandoff,
            cancellationToken, source, targetIdentity); }
        catch (OperationCanceledException) { return CancelBeforeCommit(token); }
        CommitCheckpointProbe?.Invoke("edit_source_rechecked");
        if (cancellationToken.IsCancellationRequested)
            return CancelBeforeCommit(token);
        zoneIdentifier = source.EditZoneIdentifier;
        entry.EditAttributes = source.EditAttributes;
        return CommitCore(token, cancellationToken, source, readCompletion, targetIdentity);
    }
    private ArchiveOutputResult CancelBeforeCommit(Guid token)
    {
        var entry = Entry(token);
        entry.State = ArchiveCommitState.FailedBeforeCommit;
        var cleanup = Cancel(token);
        return new("cancelled", cleanup.NativeCode, ArchiveCommitState.FailedBeforeCommit);
    }
    internal ArchiveOutputResult Commit(Guid token, CancellationToken cancellationToken = default)
    {
        var entry = Entry(token);
        lock (entry) return entry.DirectExtraction ? CommitExtractedFile(token, entry, cancellationToken) : CommitCore(token, cancellationToken);
    }
    private ArchiveOutputResult CommitCore(Guid token, CancellationToken cancellationToken, ArchiveSourceLease? editSource = null,
        ArchiveSourceLease.EditReadCompletion? readCompletion = null, string? expectedSourceTargetIdentity = null)
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
            if (Reclamation is not null && entry.Metadata?.Modified is null)
                ArchiveMetadataNative.Apply(file, new(OutputNative.Identity(file).Modified));
            if (!entry.ReclamationReady && zoneIdentifier is { } mark) {
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
            // New-file publication renames the exclusively held partial without replacement; its
            // written-bytes digest is the content fingerprint. Replacement, edit and reclamation
            // commits keep their independent full reads.
            var streamed = entry.Expected is null && editSource is null && Reclamation is null;
            var fingerprint = streamed && stamp.Size == entry.Count
                ? StreamedFingerprint(entry)
                : Fingerprint(entry, cancellationToken);
            var attributes = ArchiveMetadataNative.Attributes(file);
            var motwHash = zoneIdentifier is { } readyMark ? SHA256.HashData(readyMark.Span) : null;
            if (Reclamation is not null) {
                if (!ArchiveMotw.Matches(file, motwHash)) throw new ArchiveOutputException("reclamation_motw_mismatch");
                Reclamation.Ready(entry.Relative, file, entry.Metadata, fingerprint, attributes, motwHash, cancellationToken);
            }
            entry.State = ArchiveCommitState.Verified;
            CommitCheckpointProbe?.Invoke("before_recheck");
            if (entry.Expected is not null) RecheckTarget(entry.Parent, entry.Name, entry.Expected, ArchiveNativeReadStage.Prepublication,
                cancellationToken, editSource, expectedSourceTargetIdentity);
            // RecheckTarget disposed ONLY its target handle. Parents/source/temp remain protected.
            CommitCheckpointProbe?.Invoke("target_rechecked");
            cancellationToken.ThrowIfCancellationRequested();
            if (editSource is not null) editSource.ReleaseEditFile(readCompletion!);
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
            if (Reclamation is not null) OutputNative.Flush(file);
            CloseOutput(entry);
            CommitCheckpointProbe?.Invoke("native_committed");
            // Publication is a native fact. Complete the independent postpublication observation
            // even if the original token was cancelled after rename; it cannot undo that fact.
            using var observed = OutputNative.Open(entry.Parent, entry.Name, directory: false, readOnly: true, anyKind: true);
            var current = OutputNative.Identity(observed);
            // A new file is identified by volume/file ID/size of the object we wrote and renamed.
            if (current.Volume != stamp.Volume || current.IdLow != stamp.IdLow || current.IdHigh != stamp.IdHigh || current.Size != stamp.Size ||
                !streamed && TargetSignature(observed, stage: ArchiveNativeReadStage.OutputPostpublication, kind: ArchiveNativeReadKind.Output) != $"v2:{current.Volume:X}:{current.IdHigh:X16}{current.IdLow:X16}:{current.Size:X}:{current.Modified:X}:{Convert.ToHexString(fingerprint)}")
            {
                entry.State = ArchiveCommitState.TargetChangedAfterCommit;
                Diagnostic("failed", "target_changed_after_commit", null);
                return new("target_changed_after_commit", State: entry.State);
            }
            entry.Modified = current.Modified; // Last-write time settles when the writer closes.
            if (Reclamation is not null) {
                if (current.Modified != stamp.Modified || ArchiveMetadataNative.Attributes(observed) != attributes || !ArchiveMotw.Matches(observed, entry.ZoneIdentifierHash))
                    throw new ArchiveOutputException("reclamation_output_metadata_mismatch");
                entry.Retained = OutputNative.Open(entry.Parent, entry.Name, directory: false, readOnly: true);
                Reclamation.Published(entry.Relative);
            }
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
    private static byte[] StreamedFingerprint(Partial entry)
    {
        if (entry.Digest is not {} digest) return SHA256.HashData(ReadOnlySpan<byte>.Empty);
        try { return digest.GetHashAndReset(); }
        finally { digest.Dispose(); entry.Digest = null; }
    }
    private byte[] Fingerprint(Partial entry, CancellationToken cancellationToken)
    {
        var handle = entry.Stream?.SafeFileHandle ?? entry.File!;
        using var borrowed = new SafeFileHandle(handle.DangerousGetHandle(), false);
        using var reader = new FileStream(borrowed, FileAccess.Read, 65536, false);
        reader.Position = 0;
        var started = Stopwatch.GetTimestamp();
        try { return ReadObservation.Hash(reader, ArchiveNativeReadStage.OutputFingerprint, ArchiveNativeReadKind.Output, cancellationToken); }
        finally { Interlocked.Add(ref fingerprintTicks, Stopwatch.GetTimestamp() - started); }
    }
    // Optional cleanup is strictly per-file. Partial deletion uses the held handle. Committed
    // cleanup reacquires exclusive file access and verifies identity, size, mtime AND hash first.
    internal ArchiveOutputResult Cancel(Guid token, bool cleanupNewOutput = false) { var entry = Entry(token); lock (entry) return CancelCore(token, entry, cleanupNewOutput); }
    private ArchiveOutputResult CancelCore(Guid token, Partial entry, bool cleanupNewOutput)
    {
        Check();
        ArchiveOutputResult Finish(string category, int? native = null) {
            entry.Cleanup = category;
            return new(category, native, entry.State);
        }
        if (Reclamation is not null) return Finish("reclamation_retained");
        if (entry.State is ArchiveCommitState.CommitOutcomeUnknown or ArchiveCommitState.CommitInFlight)
            return Finish("retained_uncertain");
        if (entry.State is ArchiveCommitState.CommittedButVerificationFailed or ArchiveCommitState.TargetChangedAfterCommit)
            return Finish("retained_published");
        if (entry.DirectExtraction && entry.Committed) return Finish("retained");
        if (entry.DirectExtraction && !entry.Committed) entry.State = ArchiveCommitState.FailedBeforeCommit;
        if (!entry.Committed)
        {
            try
            {
                var handle = entry.Stream?.SafeFileHandle ?? entry.File!;
                ArchiveMetadataNative.ClearOwnedReadOnly(handle);
                OutputNative.DeleteHandle(handle);
                CloseOutput(entry);
                lock (extractionGate) entries.Remove(token);
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
            lock (extractionGate) entries.Remove(token);
            Diagnostic("cancelled", "new_output_removed", null); return Finish("new_output_removed");
        }
        catch (ArchiveOutputException e) { Diagnostic("failed", e.Stage, e.NativeCode); return Finish("changed_retained", e.NativeCode); }
        catch (IOException) { return Finish("changed_retained"); }
    }
    // Called before disposal, after worker drainage. Never reconstruct Name/Smart from the request.
    internal Protocol.ArchiveJobResultLocation? ResultLocation(string? preferredCommittedName = null)
    {
        Check();
        var committed = transactions.Where(entry=>entry.Journaled && entry.Committed).ToArray();
        if (preferredCommittedName is not null)
        {
            var primary = committed.FirstOrDefault(entry => entry.State == ArchiveCommitState.Committed &&
                entry.Name == preferredCommittedName && entry.Relative == preferredCommittedName && ReferenceEquals(entry.Parent, root));
            if (primary is null) return null;
            if (primary.DirectExtraction)
                return new(Path.Combine(OutputNative.DisplayDirectoryPath(primary.Parent), primary.Name), "select_in_explorer");
            using var observed = OutputNative.Open(primary.Parent, primary.Name, directory: false, readOnly: true, anyKind: true);
            var identity = OutputNative.Identity(observed);
            if (identity.Volume != primary.Volume || identity.IdLow != primary.FileIdLow || identity.IdHigh != primary.FileIdHigh)
                return null;
            return new(Path.Combine(OutputNative.DisplayDirectoryPath(primary.Parent), primary.Name), "select_in_explorer");
        }
        if (root is null) return null;
        var completedDirectories = outputDirectories.Where(pair=>pair.Value.Completed).ToArray();
        if (committed.Length == 0 && completedDirectories.Length == 0) return null;
        foreach (var pair in completedDirectories) OutputNative.VerifyDirectory(pair.Value.Handle);
        var roots = committed.Select(entry=>entry.Relative.Split('/')[0])
            .Concat(completedDirectories.Select(pair=>pair.Key.Split('/')[0]))
            .Distinct(StringComparer.OrdinalIgnoreCase).Take(2).ToArray();
        if (roots.Length == 1 && outputDirectories.TryGetValue(roots[0], out var directory))
            return new(OutputNative.DisplayDirectoryPath(directory.Handle), "open");
        if (committed.Length == 1 && roots.Length == 1)
            return new(Path.Combine(OutputNative.DisplayDirectoryPath(committed[0].Parent),committed[0].Name), "select_in_explorer");
        return new(OutputNative.DisplayDirectoryPath(root), "open");
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
        foreach(var entry in entries.Values){CloseOutput(entry);entry.Retained?.Dispose();entry.Digest?.Dispose();entry.Digest=null;}
        disposed = true;
        foreach (var item in directories) item.Dispose();
        drive.Dispose();
    }
    private void Diagnostic(string phase, string stage, int? native)
    {
        if (compactJournal && phase is "started" or "completed" or "cancelled") {
            int count;
            if (phase == "started") { lock (extractionGate) count = transactions.Count; }
            else count = Interlocked.Increment(ref diagnosticCompletions);
            if (count != 1 && (count < 8192 || count % 8192 != 0)) return;
        }
        try { log?.Write("info", "archive.output." + phase, operationId,
            new Dictionary<string, object?> { ["stage"] = stage, ["native_error_code"] = native,
                ["elapsed_ms"] = clock.ElapsedMilliseconds,
                ["native_full_reads"] = ReadObservation.FullReads, ["native_read_bytes"] = ReadObservation.BytesRead,
                ["partial_read_calls"] = ReadObservation.PartialReadCalls, ["partial_read_bytes"] = ReadObservation.PartialReadBytes }); }
        catch (Exception) { /* Never change the file result for diagnostic failure. */ }
    }
}

// SDK NT ABI: UNICODE_STRING (Length/MaximumLength in bytes), OBJECT_ATTRIBUTES (48 bytes
// x64), IO_STATUS_BLOCK (16 bytes x64). The unmanaged Unicode storage lives through each call.
// NtCreateFile with RootDirectory and one component is the filesystem security boundary.
internal static partial class OutputNative
{
    internal sealed class OpenObservation { internal long CreateTicks, InspectionTicks; }
    [ThreadStatic] internal static Action<string>? OperationProbe;
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
    [DllImport("kernel32.dll",EntryPoint="GetDiskFreeSpaceW",SetLastError=true,CharSet=CharSet.Unicode)]
    [return:MarshalAs(UnmanagedType.Bool)]
    private static extern bool DiskAllocation(string root,out uint sectors,out uint bytes,out uint free,out uint total);
    internal static (ulong Allocation,string Volume) AllocationSpace(SafeFileHandle directory)
    {
        var stable=StableDirectoryPath(directory); var end=stable.IndexOf('}');
        if(end<0)throw new ArchiveOutputException("invalid_commit_parent");
        var root=stable[..(end+1)]+"\\";
        if(!DiskAllocation(root,out var sectors,out var bytes,out _,out _))
            throw new ArchiveOutputException("space_allocation_failed",Marshal.GetLastWin32Error());
        var allocation=checked((ulong)sectors*bytes);
        if(allocation==0 || allocation>16777216 || (allocation&(allocation-1))!=0)
            throw new ArchiveOutputException("space_allocation_unknown");
        return (allocation,$"v1:{OutputNative.Identity(directory).Volume:X}:{stable[11..end]}");
    }
    internal static SafeFileHandle OpenDirectory(SafeFileHandle parent, string name) => OpenDirectory(parent, name, out _);
    internal static SafeFileHandle OpenDirectory(SafeFileHandle parent, string name, out bool created, Action? createdCheckpoint = null)
    {
        created = false;
        try { return Open(parent, name, directory: true); }
        catch (ArchiveOutputException e) when (e.NativeCode is 2 or 3) {
            try { var handle = Open(parent, name, directory: true, create: true, exclusive: true, createdCheckpoint: createdCheckpoint); created = true; return handle; }
            catch (ArchiveOutputException race) when (race.NativeCode is 80 or 183) { return Open(parent, name, directory: true); }
        }
    }
    internal static SafeFileHandle Open(SafeFileHandle? parent, string name, bool directory, bool create = false,
        bool exclusive = false, bool readOnly = false, bool anyKind = false, bool metadataCleanup = false, bool zoneStream = false, bool retireDirectory = false,bool requireWrite=false, Action? createdCheckpoint=null, bool reclamationWrite=false,
        Func<int, IntPtr>? nativeAllocate = null, Action<IntPtr>? nativeFree = null, bool directCreate = false,
        OpenObservation? observation = null)
    {
        var bytes = Encoding.Unicode.GetBytes(name);
        if (bytes.Length > ushort.MaxValue - 2) throw new ArchiveOutputException("invalid_component");
        var str = IntPtr.Zero;
        var unicode = IntPtr.Zero;
        try
        {
            str = nativeAllocate is null ? Marshal.AllocHGlobal(bytes.Length + 2) : nativeAllocate(bytes.Length + 2);
            unicode = nativeAllocate is null ? Marshal.AllocHGlobal(Marshal.SizeOf<UnicodeString>()) : nativeAllocate(Marshal.SizeOf<UnicodeString>());
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
            var access = reclamationWrite ? 0x100183u : zoneStream ? readOnly ? 0x100081u : 0x100183u : directory ? 0x100081u | (create ? 0x102u : 0u) | (requireWrite ? 0x6u : 0u) | (retireDirectory ? 0x10000u : 0u) : readOnly ? 0x100081u : metadataCleanup ? 0x110181u : 0x110183u;
            var createStarted = observation is null ? 0 : Stopwatch.GetTimestamp();
            var status = NtCreateFile(out var handle, access, ref attrs,
                out _, IntPtr.Zero, 0x80, directCreate ? 7u : reclamationWrite ? 1u : zoneStream ? readOnly && !metadataCleanup ? 1u : 7u : directory ? 3u : readOnly ? 1u : 0u,
                exclusive ? 2u : create ? 3u : 1u, options | (directory ? 0x1u : anyKind ? 0u : 0x40u),
                IntPtr.Zero, 0);
            if (observation is not null) Interlocked.Add(ref observation.CreateTicks, Stopwatch.GetTimestamp() - createStarted);
            if (status < 0)
            {
                handle?.Dispose();
                var error = (int)RtlNtStatusToDosError(status);
                throw new ArchiveOutputException(error is 80 or 183 ? "target_exists" : "open_component", error);
            }
            try
            {
                if (directory && create && exclusive) createdCheckpoint?.Invoke(); // Native success precedes fallible inspection.
                OperationProbe?.Invoke(directory ? "open_directory" : "open_file");
                var inspectionStarted = observation is null ? 0 : Stopwatch.GetTimestamp();
                try { Inspect(handle, directory); }
                finally { if (observation is not null) Interlocked.Add(ref observation.InspectionTicks, Stopwatch.GetTimestamp() - inspectionStarted); }
                return handle;
            }
            catch
            {
                try { if (directCreate) DeleteHandle(handle); }
                finally { handle.Dispose(); }
                throw;
            }
        }
        finally
        {
            if (unicode != IntPtr.Zero) { if (nativeFree is null) Marshal.FreeHGlobal(unicode); else nativeFree(unicode); }
            if (str != IntPtr.Zero) { if (nativeFree is null) Marshal.FreeHGlobal(str); else nativeFree(str); }
        }
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
    internal static void VerifyDirectory(SafeFileHandle handle) { OperationProbe?.Invoke("verify_directory"); Inspect(handle, true); }
    internal static (ulong Volume, ulong IdLow, ulong IdHigh, long Size, long Modified) Identity(SafeFileHandle handle)
    {
        OperationProbe?.Invoke("identity");
        if (!FileInfo(handle, out var info)) throw new ArchiveOutputException("inspect_identity", Marshal.GetLastWin32Error());
        if (!FullFileId(handle, FileInfoClass.FileIdInfo, out var id, (uint)Marshal.SizeOf<FileIdInfo>()))
            throw new ArchiveOutputException("inspect_full_identity", Marshal.GetLastWin32Error());
        return (id.VolumeSerialNumber, id.IdLow, id.IdHigh,
            ((long)info.SizeHigh << 32) | info.SizeLow, ((long)info.Write.dwHighDateTime << 32) | (uint)info.Write.dwLowDateTime);
    }
    internal static void Flush(SafeFileHandle handle)
    { OperationProbe?.Invoke("flush"); if (!FlushBuffers(handle)) throw new ArchiveOutputException("flush_file", Marshal.GetLastWin32Error()); }
    internal static void RenameNoReplace(SafeFileHandle source, SafeFileHandle parent, string name)
    {
        OperationProbe?.Invoke("rename");
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
    internal static string DisplayDirectoryPath(SafeFileHandle directory)
    {
        VerifyDirectory(directory);
        var buffer = new StringBuilder(32768);
        var length = FinalPath(directory,buffer,(uint)buffer.Capacity,0); // VOLUME_NAME_DOS; display fact only.
        if (length == 0 || length >= buffer.Capacity) throw new ArchiveOutputException("resolve_result_location",Marshal.GetLastWin32Error());
        var value=buffer.ToString();
        if (value.StartsWith(@"\\?\",StringComparison.Ordinal)) value=value[4..];
        if (!Protocol.ArchiveJobWire.PathValid(value)) throw new ArchiveOutputException("invalid_result_location");
        return value;
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
        OperationProbe?.Invoke("rename");
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
