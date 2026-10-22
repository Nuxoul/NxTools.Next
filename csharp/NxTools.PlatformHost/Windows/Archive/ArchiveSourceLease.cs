using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

// A stable namespace capability, not a before/after path stat. Every parent is held without
// FILE_SHARE_DELETE, and the file allows only FILE_SHARE_READ. Existing writable/delete
// handles make acquisition fail; subsequent opens (including UnRAR's reopen) resolve this object.
internal sealed class ArchiveSourceLease : IDisposable
{
    internal sealed class EditReadCompletion
    {
        private readonly ArchiveSourceLease owner;
        private readonly Guid nonce;
        internal bool IsConsumed { get; private set; }
        internal EditReadCompletion(ArchiveSourceLease owner, Guid nonce) { this.owner = owner; this.nonce = nonce; }
        internal bool BelongsTo(ArchiveSourceLease candidate) => ReferenceEquals(owner, candidate) && nonce != Guid.Empty;
        internal void Consume(ArchiveSourceLease candidate)
        {
            if (IsConsumed || !BelongsTo(candidate)) throw new ArchiveOutputException("edit_reads_incomplete");
            IsConsumed = true;
        }
    }
    private byte[]? zoneIdentifier;
    private bool zoneCaptured;
    internal ReadOnlyMemory<byte>? CaptureZoneIdentifier(Func<bool, uint, uint>? flagsProbe = null)
    {
        if (zoneCaptured) throw new ArchiveOutputException("motw_invalid_state");
        zoneCaptured = true;
        ArchiveMotw.EnsureSupported(handles[^1], writing: false, flagsProbe);
        zoneIdentifier = ArchiveMotw.Read(handles[^1]);
        return zoneIdentifier is null ? (ReadOnlyMemory<byte>?)null : new ReadOnlyMemory<byte>(zoneIdentifier);
    }
    private readonly List<SafeFileHandle> handles = [];
    private object? recycleVersion;
    private object? editSourceVersion;
    private byte[]? editSourceHash;
    private string? editSourceTargetSignature;
    private Guid pendingEditReadCompletion;
    private bool editFileReleased;
    internal uint EditAttributes { get; private set; }
    internal ReadOnlyMemory<byte>? EditZoneIdentifier => zoneIdentifier is null ? null : new ReadOnlyMemory<byte>(zoneIdentifier);
    private string? readmissionIdentity;
    internal ArchiveRecycleSource RecycleSource(Guid id)
    {
        void Verify() {
            if (handles.Count == 0 || recycleVersion is null || editFileReleased || handles[^1].IsClosed)
                throw new ArchiveOutputException("source_unavailable");
            if (!recycleVersion.Equals(ArchiveRecycleVersion.Capture(handles[^1]))) throw new ArchiveOutputException("source_changed");
        }
        Verify();
        return new(id, Verify);
    }
    private string? sourceName;
    private ArchiveSourceLease() { }
    internal static ArchiveSourceLease Open(string path) => OpenCore(path,false);
    internal static ArchiveSourceLease OpenForEdit(string path) => OpenCore(path,true);
    private static ArchiveSourceLease OpenCore(string path,bool editing)
    {
        if (string.IsNullOrEmpty(path) || path.Length is < 4 or > 32767 || !char.IsAsciiLetter(path[0]) ||
            path[1] != ':' || path[2] != '\\' || path.IndexOfAny(['/', '\0']) >= 0)
            throw new ArchiveOutputException("invalid_source");
        var parts = path[3..].Split('\\');
        if (parts.Length > 64 || parts.Any(p => p.Length is < 1 or > 255 || p is "." or ".." ||
            p.EndsWith('.') || p.EndsWith(' ') || p.IndexOfAny([':', '<', '>', '"', '|', '?', '*']) >= 0 || p.Any(char.IsControl)))
            throw new ArchiveOutputException("invalid_source");
        var lease = new ArchiveSourceLease();
        try
        {
            lease.handles.Add(OutputNative.Open(null, "\\??\\" + char.ToUpperInvariant(path[0]) + ":\\", directory: true));
            for (var index = 0; index < parts.Length; index++)
                lease.handles.Add(OutputNative.Open(lease.handles[^1], parts[index], directory: index != parts.Length - 1, readOnly: true));
            lease.sourceName = parts[^1];
            lease.recycleVersion = ArchiveRecycleVersion.Capture(lease.handles[^1]);
            lease.readmissionIdentity = lease.CaptureReadmissionIdentity();
            if (editing) {
                lease.EditAttributes = ArchiveMetadataNative.Attributes(lease.handles[^1]);
                lease.CaptureZoneIdentifier();
                (lease.editSourceVersion, lease.editSourceHash) = lease.CaptureEditSource();
                lease.editSourceTargetSignature = lease.CaptureTargetSignature();
            }
            return lease;
        }
        catch { lease.Dispose(); throw; }
    }
    internal string ReadmissionIdentity()
    {
        if (handles.Count == 0) throw new ArchiveOutputException("source_unavailable");
        var handle = handles[^1];
        if (editFileReleased || handle.IsClosed) return readmissionIdentity ?? throw new ArchiveOutputException("source_unavailable");
        return CaptureReadmissionIdentity();
    }
    private string CaptureReadmissionIdentity()
    {
        var stamp = OutputNative.Identity(handles[^1]);
        return $"v1:{stamp.Volume:X}:{stamp.IdHigh:X16}{stamp.IdLow:X16}:{stamp.Size:X}:{stamp.Modified:X}";
    }
    internal bool EditFileReleased => editFileReleased;
    // Future runner wiring may create this evidence only after its acknowledged edit-read-
    // complete message is owned and every worker-side source reader has actually closed.
    // The evidence is single-use and bound to this exact lease; it is not commit permission.
    internal EditReadCompletion CompleteEditReads()
    {
        if (editSourceHash is null || handles.Count < 2 || editFileReleased || handles[^1].IsClosed || pendingEditReadCompletion != Guid.Empty)
            throw new ArchiveOutputException("edit_reads_incomplete");
        VerifyEditSource(editSourceTargetSignature!);
        pendingEditReadCompletion = Guid.NewGuid();
        return new EditReadCompletion(this, pendingEditReadCompletion);
    }
    internal void VerifyEditSource(string expectedTargetIdentity)
    {
        if (handles.Count < 2 || editFileReleased || handles[^1].IsClosed) throw new ArchiveOutputException("source_unavailable");
        var (currentVersion, currentHash) = CaptureEditSource();
        try
        {
            if (editSourceVersion is null || editSourceHash is null || !editSourceVersion.Equals(currentVersion) ||
                !editSourceHash.AsSpan().SequenceEqual(currentHash)) throw new ArchiveOutputException("source_changed");
        }
        finally { System.Security.Cryptography.CryptographicOperations.ZeroMemory(currentHash); }
        var target = CaptureTargetSignature();
        if (!string.Equals(target, editSourceTargetSignature, StringComparison.Ordinal)) throw new ArchiveOutputException("source_changed");
        if (!string.Equals(target, expectedTargetIdentity, StringComparison.Ordinal)) throw new ArchiveOutputException("edit_target_mismatch");
    }
    internal void ReleaseEditFile(EditReadCompletion completion)
    {
        if (pendingEditReadCompletion == Guid.Empty || !completion.BelongsTo(this))
            throw new ArchiveOutputException("edit_reads_incomplete");
        completion.Consume(this);
        handles[^1].Dispose();
        editFileReleased = true;
        pendingEditReadCompletion = Guid.Empty;
    }
    private (object Version, byte[] Hash) CaptureEditSource()
    {
        var handle = handles[^1];
        var version = ArchiveRecycleVersion.Capture(handle);
        using var borrowed = new SafeFileHandle(handle.DangerousGetHandle(), false);
        using var stream = new FileStream(borrowed, FileAccess.Read, 65536, false);
        stream.Position = 0;
        return (version, System.Security.Cryptography.SHA256.HashData(stream));
    }
    private string CaptureTargetSignature()
    {
        if (handles.Count < 2) throw new ArchiveOutputException("source_unavailable");
        using var target = OutputNative.Open(handles[^2], sourceName!, directory: false, readOnly: true, anyKind: true);
        return ArchiveOutputSession.TargetIdentity(target);
    }
    public void Dispose()
    {
        if (zoneIdentifier is { } mark) System.Security.Cryptography.CryptographicOperations.ZeroMemory(mark);
        zoneIdentifier = null;
        if (editSourceHash is { } hash) System.Security.Cryptography.CryptographicOperations.ZeroMemory(hash);
        editSourceHash = null;
        editSourceVersion = null;
        pendingEditReadCompletion = Guid.Empty;
        for (var index = handles.Count - 1; index >= 0; index--) handles[index].Dispose();
        handles.Clear();
    }
}
