using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveOutputSession
{
    // Ordinary extraction owns the final name from Begin through close. Creation/edit and
    // entry-cache callers continue to use BeginFile's staged publication lifecycle.
    internal Action<int>? ExtractionWriteCheckpointProbe { get; set; }
    internal Guid BeginExtractedFile(string relative, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        Check();
        if (Reclamation is not null) throw new ArchiveOutputException("invalid_state");
        cancellationToken.ThrowIfCancellationRequested();
        var target = ResolveParent(relative);
        lock (extractionGate) { AdmitTransaction(relative, null); pendingExtractionBegins++; }
        SafeFileHandle file;
        var replaced = false;
        var started = System.Diagnostics.Stopwatch.GetTimestamp();
        try
        {
            BeginCheckpointProbe?.Invoke(relative);
            try { file = OutputNative.Open(target.Parent, target.Name, directory: false, exclusive: true, directCreate: true, observation: extractionOpenObservation); }
            catch (ArchiveOutputException failure) when (overwrite && failure.Stage == "target_exists" && failure.NativeCode is 80 or 183)
            {
                // FILE_OPEN_REPARSE_POINT and Inspect run before any truncation.
                file = OutputNative.Open(target.Parent, target.Name, directory: false, anyKind: true, observation: extractionOpenObservation);
                replaced = true;
            }
        }
        catch
        {
            lock (extractionGate) { journalReservation -= 2048UL + (ulong)relative.Length * 4; pendingExtractionBegins--; }
            throw;
        }
        finally { Interlocked.Add(ref finalOpenTicks, System.Diagnostics.Stopwatch.GetTimestamp() - started); }
        var token = Guid.NewGuid();
        var entry = new Partial(target.Parent, target.Name, file)
        { Relative = relative, DirectExtraction = true, Replaced = replaced };
        bool first;
        lock (extractionGate) {
            entries.Add(token, entry); transactions.Add(entry); activeFiles++; pendingExtractionBegins--;
            first = transactions.Count == 1;
        }
        if (replaced)
        {
            try { RandomAccess.SetLength(file, 0); }
            catch (IOException failure)
            {
                entry.State = ArchiveCommitState.FailedBeforeCommit;
                var cleanup = Cancel(token);
                if (cleanup.Category != "partial_removed") throw new ArchiveOutputException("cleanup_failed", cleanup.NativeCode);
                throw new ArchiveOutputException("truncate_file", failure.HResult & 0xffff);
            }
        }
        if (first) Diagnostic("started", "started", null);
        return token;
    }

    private bool WriteExtractedFile(Guid token, Partial entry, ReadOnlyMemory<byte> chunk)
    {
        try
        {
            var started = System.Diagnostics.Stopwatch.GetTimestamp();
            try {
                RandomAccess.Write(entry.File!, chunk.Span, entry.Count);
            }
            finally { Interlocked.Add(ref nativeWriteTicks, System.Diagnostics.Stopwatch.GetTimestamp() - started); }
            entry.Count += chunk.Length;
            ExtractionWriteCheckpointProbe?.Invoke(chunk.Length);
            return true;
        }
        catch (IOException failure)
        {
            entry.State = ArchiveCommitState.FailedBeforeCommit;
            throw new ArchiveOutputException("write_file", failure.HResult & 0xffff);
        }
    }

    private ArchiveOutputResult CommitExtractedFile(Guid token, Partial entry, CancellationToken cancellationToken)
    {
        Check();
        if (entry.Committed || entry.File is null || entry.State != ArchiveCommitState.Preparing)
            throw new ArchiveOutputException("invalid_state", state: entry.State);
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var file = entry.File!;
            if (zoneIdentifier is { } mark)
            {
                CommitCheckpointProbe?.Invoke("before_motw");
                ArchiveMotw.EnsureSupported(file, writing: true, NamedStreamsFlagsProbe);
                cancellationToken.ThrowIfCancellationRequested();
                ArchiveMotw.Write(file, mark.Span);
            }
            if (entry.Metadata is { } metadata)
            {
                bool startedMetadata;
                lock (extractionGate) { startedMetadata = !metadataStarted; metadataStarted = true; }
                if (startedMetadata) Diagnostic("metadata_started", "metadata", null);
                CommitCheckpointProbe?.Invoke("before_metadata");
                cancellationToken.ThrowIfCancellationRequested();
                ArchiveMetadataNative.Apply(file, metadata);
                CommitCheckpointProbe?.Invoke("metadata_applied");
            }
            cancellationToken.ThrowIfCancellationRequested();
            CloseOutput(entry);
            entry.Committed = true;
            entry.State = ArchiveCommitState.Committed;
            entry.Cleanup = entry.Replaced ? "retained_replacement" : "retained";
            return new("committed", State: entry.State);
        }
        catch (OperationCanceledException)
        {
            entry.State = ArchiveCommitState.FailedBeforeCommit;
            var cleanup = Cancel(token);
            return new(cleanup.Category == "partial_removed" ? "cancelled" : "cleanup_failed", cleanup.NativeCode, entry.State);
        }
        catch (ArchiveOutputException failure)
        {
            entry.State = ArchiveCommitState.FailedBeforeCommit;
            var cleanup = Cancel(token);
            if (cleanup.Category != "partial_removed") throw new ArchiveOutputException("cleanup_failed", cleanup.NativeCode);
            Diagnostic("failed", failure.Stage, failure.NativeCode);
            throw;
        }
        catch (IOException failure)
        {
            entry.State = ArchiveCommitState.FailedBeforeCommit;
            var cleanup = Cancel(token);
            if (cleanup.Category != "partial_removed") throw new ArchiveOutputException("cleanup_failed", cleanup.NativeCode);
            Diagnostic("failed", "close_file", failure.HResult & 0xffff);
            throw new ArchiveOutputException("close_file", failure.HResult & 0xffff);
        }
    }
}
