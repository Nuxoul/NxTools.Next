using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Microsoft.Win32.SafeHandles;

namespace NxTools.PlatformHost.Windows.Archive;

// Worker-thread-only capability table for general archive creation. Rust owns traversal and the
// relative archive plan; this type exposes only IDs issued from held roots and never accepts paths.
internal sealed class ArchiveCreationSources : IDisposable
{
    private const int MaximumRoots = 32;
    private const int MaximumDepth = 64;
    private const int MaximumTrackedRecords = 65536;
    private const ulong DescendantMetadataReservation = 2048;
    private const ulong RootBaseMetadataReservation = 2048;
    private const ulong RootHandleMetadataReservation = 256;
    private const uint FileTypeDisk = 1;

    // Windows SDK 10.0.26100.0: FILE_INFO_BY_HANDLE_CLASS.FileBasicInfo and FILE_BASIC_INFO.
    private enum FileInfoByHandleClass
    {
        FileBasicInfo = 0,
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FileBasicInfo
    {
        internal long CreationTime;
        internal long LastAccessTime;
        internal long LastWriteTime;
        internal long ChangeTime;
        internal uint FileAttributes;
    }

    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", SetLastError = true, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetBasicInfo(SafeFileHandle handle, FileInfoByHandleClass kind,
        out FileBasicInfo info, uint size);

    [DllImport("kernel32.dll", EntryPoint = "GetFileType", SetLastError = true, ExactSpelling = true)]
    private static extern uint GetFileTypeNative(SafeFileHandle handle);

    private sealed class SourceRecord
    {
        internal required Guid Id { get; init; }
        internal required int RootIndex { get; init; }
        internal required Guid? ParentId { get; init; }
        internal required string Name { get; init; }
        internal required bool Directory { get; init; }
        internal required int Depth { get; init; }
        internal required ulong Volume { get; init; }
        internal required ulong IdLow { get; init; }
        internal required ulong IdHigh { get; init; }
        internal required ulong Size { get; init; }
        internal required long Modified { get; init; }
        internal required long ChangeTime { get; init; }
        internal required ushort[]? DosModified { get; init; }
    }

    private sealed class RootState(string path, SourceRecord record, List<SafeFileHandle> handles) : IDisposable
    {
        internal string Path { get; } = path;
        internal SourceRecord Record { get; } = record;
        internal List<SafeFileHandle> Handles { get; } = handles;
        internal SafeFileHandle RootHandle => Handles[^1];

        public void Dispose()
        {
            for (var index = Handles.Count - 1; index >= 0; index--) Handles[index].Dispose();
            Handles.Clear();
        }
    }

    private sealed class HeldNode(SourceRecord record, SafeFileHandle handle, bool owned)
    {
        internal SourceRecord Record { get; } = record;
        internal SafeFileHandle Handle { get; } = handle;
        internal bool Owned { get; } = owned;
    }

    private sealed class HeldChain(ArchiveCreationSources owner) : IDisposable
    {
        internal List<HeldNode> Nodes { get; } = [];
        internal SafeFileHandle TargetHandle => Nodes[^1].Handle;

        internal void VerifyAll()
        {
            foreach (var node in Nodes) owner.VerifyRecord(node.Record, node.Handle);
        }

        public void Dispose()
        {
            for (var index = Nodes.Count - 1; index >= 0; index--)
                if (Nodes[index].Owned) Nodes[index].Handle.Dispose();
            Nodes.Clear();
        }
    }

    private sealed class DirectoryEnumeration(SourceRecord record, HeldChain chain, IEnumerator<string> enumerator,
        int pathLength)
    {
        internal SourceRecord Record { get; } = record;
        internal HeldChain Chain { get; } = chain;
        internal IEnumerator<string> Enumerator { get; } = enumerator;
        internal int PathLength { get; } = pathLength;
        internal ulong NextOffset { get; set; }
    }

    private sealed class ActiveFile(SourceRecord record, HeldChain chain)
    {
        internal SourceRecord Record { get; } = record;
        internal HeldChain Chain { get; } = chain;
    }

    private readonly record struct Snapshot(ulong Volume, ulong IdLow, ulong IdHigh, ulong Size,
        long Modified, long ChangeTime, ushort[]? DosModified);

    private readonly List<RootState> roots = [];
    private readonly Dictionary<Guid, SourceRecord> records = [];
    private readonly int maximumRecords;
    private readonly ulong maximumMetadata;
    private readonly ulong maximumEntryBytes;
    private readonly ulong maximumTotalBytes;
    private ulong metadataUsed;
    private ulong totalObservedBytes;
    private int trackedRecords;
    private DirectoryEnumeration? activeDirectory;
    private ActiveFile? activeFile;
    private bool disposed;
    internal Func<string, string>? NamespacePathProbe { get; set; }

    private ArchiveCreationSources(ulong maxEntries, ulong maxMetadata, ulong maxEntryBytes, ulong maxTotalBytes)
    {
        maximumRecords = (int)Math.Min(maxEntries, MaximumTrackedRecords);
        maximumMetadata = maxMetadata;
        maximumEntryBytes = maxEntryBytes;
        maximumTotalBytes = maxTotalBytes;
    }

    internal static ArchiveCreationSources Open(IReadOnlyList<string> paths, string outputRoot, string createName,
        ulong maxEntries, ulong maxMetadata, ulong maxEntryBytes, ulong maxTotalBytes, ArchiveOutputSession? heldOutput = null)
    {
        if (paths is null || paths.Count is < 1 or > MaximumRoots)
            throw new ArchiveOutputException(paths?.Count > MaximumRoots ? "session_entry_limit" : "invalid_source");

        ValidateAbsolutePath(outputRoot);
        try { ArchiveOutputSession.Validate(createName); }
        catch (ArchiveOutputException) { throw new ArchiveOutputException("invalid_component"); }

        for (var index = 0; index < paths.Count; index++)
        {
            ValidateAbsolutePath(paths[index]);
            for (var earlier = 0; earlier < index; earlier++)
                if (PathsOverlap(paths[index], paths[earlier]))
                    throw new ArchiveOutputException("source_output_collision");
        }

        var finalLength = (long)outputRoot.Length + 1 + createName.Length;
        if (finalLength > 32767) throw new ArchiveOutputException("invalid_component");
        var finalPath = outputRoot + "\\" + createName;
        foreach (var path in paths)
            if (string.Equals(path, finalPath, StringComparison.OrdinalIgnoreCase))
                throw new ArchiveOutputException("source_output_collision");

        var sources = new ArchiveCreationSources(maxEntries, maxMetadata, maxEntryBytes, maxTotalBytes);
        try
        {
            if (maxMetadata < 65536) throw new ArchiveOutputException("session_entry_limit");
            sources.metadataUsed = 65536; // One bounded enumerator/result path buffer, independent of retained records.
            for (var index = 0; index < paths.Count; index++) sources.AddRoot(paths[index]);
            sources.RejectOutputCollisions(outputRoot, createName, heldOutput);
            return sources;
        }
        catch
        {
            sources.Dispose();
            throw;
        }
    }

    // Replacement content is one regular held root, never a recursive source-tree add.
    internal void RequireSingleRegularFile()
    {
        CheckOpen();
        if (roots.Count != 1 || roots[0].Record.Directory) throw new ArchiveOutputException("invalid_source");
        RevalidateRoots();
    }

    internal object Execute(Guid operationId, JsonElement call)
    {
        Guid? callId = null;
        try
        {
            CheckOpen();
            if (call.ValueKind != JsonValueKind.Object ||
                !call.TryGetProperty("call_id", out var callIdElement) || !callIdElement.TryGetGuid(out var parsedCallId) ||
                parsedCallId == Guid.Empty || !call.TryGetProperty("action", out var actionElement) ||
                actionElement.ValueKind != JsonValueKind.String)
                throw new ArchiveOutputException("invalid_source_call");
            callId = parsedCallId;

            var action = actionElement.GetString();
            string? readData = null;
            switch (action)
            {
                case "source_roots":
                    readData = SourceRoots();
                    break;
                case "source_next":
                    readData = SourceNext(ReadToken(call), ReadNextOffset(call));
                    break;
                case "source_open":
                    SourceOpen(ReadToken(call));
                    break;
                case "source_read":
                    readData = SourceRead(ReadToken(call), ReadOffset(call), ReadLength(call));
                    break;
                case "source_close":
                    SourceClose(ReadToken(call));
                    break;
                case "source_check":
                    SourceCheck(ReadToken(call));
                    break;
                default:
                    throw new ArchiveOutputException("invalid_source_call");
            }

            return readData is null
                ? new { version = 1, operation_id = operationId, op = "native_result", call_id = callId.Value, success = true }
                : new { version = 1, operation_id = operationId, op = "native_result", call_id = callId.Value, success = true, read_data = readData };
        }
        catch (ArchiveOutputException failure)
        {
            var normalized = NormalizeFailure(failure);
            return FailureReply(operationId, callId, normalized.Category, normalized.NativeCode);
        }
        catch (UnauthorizedAccessException)
        {
            return FailureReply(operationId, callId, "source_unavailable", 5);
        }
        catch (IOException failure)
        {
            return FailureReply(operationId, callId, "source_unavailable", NativeCode(failure));
        }
        catch (Exception failure) when (failure is JsonException or InvalidOperationException or FormatException or
            OverflowException or ArgumentException or KeyNotFoundException or ObjectDisposedException)
        {
            return FailureReply(operationId, callId, "invalid_source_call", null);
        }
    }

    private static object FailureReply(Guid operationId, Guid? callId, string category, int? nativeCode) =>
        new { version = 1, operation_id = operationId, op = "native_result", call_id = callId,
            success = false, error_code = category, native_error_code = nativeCode };

    private static (string Category, int? NativeCode) NormalizeFailure(ArchiveOutputException failure)
    {
        var category = failure.Stage switch
        {
            "source_changed" or "source_unavailable" or "reparse_component" or "session_entry_limit" or
                "read_limit" or "invalid_source_call" => failure.Stage,
            "source_output_collision" => "invalid_source_call",
            "wrong_kind" => "source_changed",
            _ => "source_unavailable",
        };
        return (category, failure.NativeCode);
    }

    private string SourceRoots()
    {
        foreach (var root in roots) VerifyRecord(root.Record, root.RootHandle);
        var values = new object[roots.Count];
        for (var index = 0; index < roots.Count; index++) values[index] = ToDto(roots[index].Record);
        return JsonSerializer.Serialize(values);
    }

    private string SourceNext(Guid token, ulong offset)
    {
        if (activeFile is not null) throw new ArchiveOutputException("invalid_source_call");
        if (activeDirectory is null)
        {
            if (offset != 0) throw new ArchiveOutputException("invalid_source_call");
            var record = Record(token);
            if (!record.Directory) throw new ArchiveOutputException("invalid_source_call");
            activeDirectory = StartEnumeration(record);
        }
        else if (activeDirectory.Record.Id != token || activeDirectory.NextOffset != offset)
        {
            throw new ArchiveOutputException("invalid_source_call");
        }

        var state = activeDirectory!;
        var recordReserved = false;
        ulong reservedBytes = 0;
        try
        {
            bool moved;
            try { moved = state.Enumerator.MoveNext(); }
            catch (Exception failure) when (failure is IOException or UnauthorizedAccessException)
            {
                AbortEnumeration();
                throw MapIoFailure(failure, observed: true);
            }

            if (!moved)
            {
                // EOF does not consume a retained source record.
                FinishEnumeration();
                return "null";
            }
            ReserveRecord(DescendantMetadataReservation);
            recordReserved = true;

            var enumerated = state.Enumerator.Current;
            var name = Path.GetFileName(enumerated);
            try { ArchiveOutputSession.Validate(name); }
            catch (ArchiveOutputException failure)
            {
                throw new ArchiveOutputException("source_unavailable", failure.NativeCode);
            }

            if (state.Record.Depth >= MaximumDepth || (long)state.PathLength + 1 + name.Length > 32767)
                throw new ArchiveOutputException("session_entry_limit");

            Snapshot snapshot;
            bool directory;
            using (var handle = OpenUnknownRelative(state.Chain.TargetHandle, name, observed: true, out directory))
                snapshot = CaptureSnapshot(handle, directory, observed: true);

            if (!directory)
            {
                ReserveObservedBytes(snapshot.Size);
                reservedBytes = snapshot.Size;
            }

            var child = new SourceRecord
            {
                Id = NewRecordId(),
                RootIndex = state.Record.RootIndex,
                ParentId = state.Record.Id,
                Name = name,
                Directory = directory,
                Depth = state.Record.Depth + 1,
                Volume = snapshot.Volume,
                IdLow = snapshot.IdLow,
                IdHigh = snapshot.IdHigh,
                Size = snapshot.Size,
                Modified = snapshot.Modified,
                ChangeTime = snapshot.ChangeTime,
                DosModified = snapshot.DosModified,
            };
            records.Add(child.Id, child);
            recordReserved = false;
            reservedBytes = 0;
            state.NextOffset++;
            return JsonSerializer.Serialize(ToDto(child));
        }
        catch
        {
            if (reservedBytes != 0) totalObservedBytes -= reservedBytes;
            if (recordReserved) ReleaseRecord(DescendantMetadataReservation);
            AbortEnumeration();
            throw;
        }
    }

    private void SourceOpen(Guid token)
    {
        if (activeFile is not null || activeDirectory is not null)
            throw new ArchiveOutputException("invalid_source_call");
        var record = Record(token);
        if (record.Directory) throw new ArchiveOutputException("invalid_source_call");
        activeFile = new ActiveFile(record, OpenRecordChain(record));
    }

    private string SourceRead(Guid token, ulong offset, int length)
    {
        var current = activeFile;
        if (current is null || current.Record.Id != token) throw new ArchiveOutputException("invalid_source_call");
        if (length is < 1 or > 65536 || offset > current.Record.Size || offset > long.MaxValue)
            throw new ArchiveOutputException("read_limit");

        VerifyRecord(current.Record, current.Chain.TargetHandle);
        var remaining = current.Record.Size - offset;
        var count = (int)Math.Min((ulong)length, remaining);
        var buffer = new byte[count];
        try
        {
            var read = 0;
            while (read < buffer.Length)
            {
                var currentRead = RandomAccess.Read(current.Chain.TargetHandle, buffer.AsSpan(read),
                    checked((long)offset + read));
                if (currentRead == 0) throw new ArchiveOutputException("source_changed");
                read += currentRead;
            }
            return Convert.ToBase64String(buffer);
        }
        catch (IOException failure)
        {
            throw MapIoFailure(failure, observed: true);
        }
        finally
        {
            CryptographicOperations.ZeroMemory(buffer);
        }
    }

    private void SourceClose(Guid token)
    {
        var current = activeFile;
        if (current is null || current.Record.Id != token) throw new ArchiveOutputException("invalid_source_call");
        activeFile = null;
        try { current.Chain.VerifyAll(); }
        finally { current.Chain.Dispose(); }
    }

    private void SourceCheck(Guid token)
    {
        var record = Record(token);
        if (activeFile is { } current)
        {
            if (current.Record.Id != token) throw new ArchiveOutputException("invalid_source_call");
            current.Chain.VerifyAll();
            return;
        }
        using var chain = OpenRecordChain(record);
        // OpenRecordChain verifies the root, every cached parent and the requested object.
    }

    private DirectoryEnumeration StartEnumeration(SourceRecord record)
    {
        var chain = OpenRecordChain(record);
        try
        {
            var path = OutputNative.StableDirectoryPath(chain.TargetHandle);
            path = NamespacePathProbe?.Invoke(path) ?? path;
            if (path.Length > 32767 || !path.StartsWith(@"\\?\", StringComparison.Ordinal))
                throw new ArchiveOutputException("source_unavailable");
            // Reopen the actual enumerated namespace, not the original DOS path. The original
            // no-delete root/lineage chain pins every ancestor for this enumerator's lifetime.
            using (var enumeratedDirectory = OpenRelative(null, @"\??\" + path[4..], directory: true, observed: true))
                VerifyRecord(record, enumeratedDirectory);
            var pathLength = path.Length;
            var options = new EnumerationOptions
            {
                AttributesToSkip = 0,
                BufferSize = 4096,
                IgnoreInaccessible = false,
                RecurseSubdirectories = false,
                ReturnSpecialDirectories = false,
            };
            var enumerator = Directory.EnumerateFileSystemEntries(path, "*", options).GetEnumerator();
            return new DirectoryEnumeration(record, chain, enumerator, pathLength);
        }
        catch (Exception failure) when (failure is IOException or UnauthorizedAccessException)
        {
            chain.Dispose();
            throw MapIoFailure(failure, observed: true);
        }
        catch
        {
            chain.Dispose();
            throw;
        }
    }

    private void FinishEnumeration()
    {
        var state = activeDirectory ?? throw new ArchiveOutputException("invalid_source_call");
        activeDirectory = null;
        try
        {
            state.Chain.VerifyAll();
        }
        finally
        {
            try { state.Enumerator.Dispose(); }
            finally { state.Chain.Dispose(); }
        }
    }

    private void AbortEnumeration()
    {
        var state = activeDirectory;
        activeDirectory = null;
        if (state is null) return;
        try { state.Enumerator.Dispose(); }
        finally { state.Chain.Dispose(); }
    }

    private HeldChain OpenRecordChain(SourceRecord record)
    {
        var root = roots[record.RootIndex];
        VerifyRecord(root.Record, root.RootHandle);
        var chain = new HeldChain(this);
        try
        {
            var lineage = DescendantLineage(record, root.Record);
            if (lineage.Count == 0)
            {
                var duplicate = OpenRelative(root.Handles[^2], record.Name, record.Directory, observed: true);
                try { VerifyRecord(record, duplicate); }
                catch { duplicate.Dispose(); throw; }
                chain.Nodes.Add(new HeldNode(record, duplicate, owned: true));
                return chain;
            }

            chain.Nodes.Add(new HeldNode(root.Record, root.RootHandle, owned: false));
            var parent = root.RootHandle;
            foreach (var item in lineage)
            {
                var handle = OpenRelative(parent, item.Name, item.Directory, observed: true);
                try { VerifyRecord(item, handle); }
                catch { handle.Dispose(); throw; }
                chain.Nodes.Add(new HeldNode(item, handle, owned: true));
                parent = handle;
            }
            return chain;
        }
        catch
        {
            chain.Dispose();
            throw;
        }
    }

    private List<SourceRecord> DescendantLineage(SourceRecord record, SourceRecord root)
    {
        var lineage = new List<SourceRecord>(record.Depth);
        var current = record;
        while (current.ParentId is { } parentId)
        {
            lineage.Add(current);
            if (!records.TryGetValue(parentId, out current) || lineage.Count > MaximumDepth)
                throw new ArchiveOutputException("invalid_source_call");
        }
        if (current.Id != root.Id || current.RootIndex != record.RootIndex)
            throw new ArchiveOutputException("invalid_source_call");
        lineage.Reverse();
        return lineage;
    }

    private static SafeFileHandle OpenUnknownRelative(SafeFileHandle parent, string name, bool observed, out bool directory)
    {
        // FILE_DIRECTORY_FILE identifies a regular file with ERROR_DIRECTORY(267). Never
        // interpret denied/busy/reparse as a reason to retry with weaker protection.
        try { var handle = OpenRelative(parent, name, directory: true, observed); directory = true; return handle; }
        catch (ArchiveOutputException failure) when (failure.NativeCode == 267) {
            var handle = OpenRelative(parent, name, directory: false, observed); directory = false; return handle;
        }
    }

    private void AddRoot(string path)
    {
        var parts = path[3..].Split('\\');
        var reservation = RootReservation(path.Length, parts.Length);
        ReserveRecord(reservation);
        var recordReserved = true;
        ulong reservedBytes = 0;
        var handles = new List<SafeFileHandle>(parts.Length + 1);
        try
        {
            handles.Add(OpenRelative(null, "\\??\\" + char.ToUpperInvariant(path[0]) + ":\\",
                directory: true, observed: false));
            for (var index = 0; index < parts.Length; index++)
            {
                handles.Add(index == parts.Length - 1
                    ? OpenUnknownRelative(handles[^1], parts[index], observed: false, out _)
                    : OpenRelative(handles[^1], parts[index], directory: true, observed: false));
            }

            var finalDirectory = (ReadBasicInfo(handles[^1], observed: false).FileAttributes &
                (uint)FileAttributes.Directory) != 0;
            var snapshot = CaptureSnapshot(handles[^1], finalDirectory, observed: false);
            if (!finalDirectory)
            {
                ReserveObservedBytes(snapshot.Size);
                reservedBytes = snapshot.Size;
            }

            var record = new SourceRecord
            {
                Id = NewRecordId(),
                RootIndex = roots.Count,
                ParentId = null,
                Name = parts[^1],
                Directory = finalDirectory,
                Depth = parts.Length,
                Volume = snapshot.Volume,
                IdLow = snapshot.IdLow,
                IdHigh = snapshot.IdHigh,
                Size = snapshot.Size,
                Modified = snapshot.Modified,
                ChangeTime = snapshot.ChangeTime,
                DosModified = snapshot.DosModified,
            };
            var state = new RootState(path, record, handles);
            RejectRootOverlap(state);
            records.Add(record.Id, record);
            roots.Add(state);
            recordReserved = false;
            reservedBytes = 0;
        }
        catch
        {
            if (reservedBytes != 0) totalObservedBytes -= reservedBytes;
            if (recordReserved) ReleaseRecord(reservation);
            for (var index = handles.Count - 1; index >= 0; index--) handles[index].Dispose();
            throw;
        }
    }

    private void RejectRootOverlap(RootState candidate)
    {
        foreach (var existing in roots)
        {
            for (var index = 1; index < existing.Handles.Count; index++)
                if (SameIdentity(candidate.Record, existing.Handles[index]))
                    throw new ArchiveOutputException("invalid_source");
            for (var index = 1; index < candidate.Handles.Count; index++)
                if (SameIdentity(existing.Record, candidate.Handles[index]))
                    throw new ArchiveOutputException("invalid_source");
        }
    }

    internal void RevalidateRoots()
    {
        CheckOpen();
        foreach (var root in roots) VerifyRecord(root.Record, root.RootHandle);
    }
    internal IReadOnlyList<ArchiveRecycleSource> RecycleSources()
    {
        RevalidateRoots();
        if (roots.Any(root => root.Record.Directory)) return [];
        return roots.Select(root => new ArchiveRecycleSource(root.Record.Id, () => {
            CheckOpen(); VerifyRecord(root.Record, root.RootHandle);
        })).ToArray();
    }

    internal void RejectOutputCollisions(string outputRoot, string createName, ArchiveOutputSession? heldOutput)
    {
        foreach (var root in roots)
        {
            if (root.Record.Directory && IsSameOrDescendant(outputRoot, root.Path))
                throw new ArchiveOutputException("source_output_collision");
            if (!root.Record.Directory && string.Equals(root.Path, outputRoot + "\\" + createName,
                    StringComparison.OrdinalIgnoreCase))
                throw new ArchiveOutputException("source_output_collision");
        }

        if (heldOutput is not null) {
            heldOutput.CheckCreationCollisions(createName,
                handle => roots.Any(root => root.Record.Directory && SameIdentity(root.Record, handle)),
                handle => roots.Any(root => !root.Record.Directory && SameIdentity(root.Record, handle)));
            return;
        }
        // Standalone native-source fixtures have no publishing session. Runner always supplies
        // its actual held output session, so a DOS remap cannot redirect publication admission.
        var parts = outputRoot[3..].Split('\\');
        var handles = new List<SafeFileHandle>(parts.Length + 1);
        try
        {
            handles.Add(OpenRelative(null, "\\??\\" + char.ToUpperInvariant(outputRoot[0]) + ":\\",
                directory: true, observed: false));
            foreach (var part in parts)
            {
                SafeFileHandle handle;
                try { handle = OpenRelative(handles[^1], part, directory: true, observed: false); }
                catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return; }
                handles.Add(handle);
                foreach (var root in roots)
                    if (root.Record.Directory && SameIdentity(root.Record, handle))
                        throw new ArchiveOutputException("source_output_collision");
            }

            SafeFileHandle final;
            try { final = OpenRelative(handles[^1], createName, directory: false, observed: false); }
            catch (ArchiveOutputException failure) when (failure.NativeCode is 2 or 3) { return; }
            using (final)
            {
                foreach (var root in roots)
                    if (!root.Record.Directory && SameIdentity(root.Record, final))
                        throw new ArchiveOutputException("source_output_collision");
            }
        }
        finally
        {
            for (var index = handles.Count - 1; index >= 0; index--) handles[index].Dispose();
        }
    }

    private bool SameIdentity(SourceRecord record, SafeFileHandle handle)
    {
        try
        {
            var identity = OutputNative.Identity(handle);
            return record.Volume == identity.Volume && record.IdLow == identity.IdLow &&
                record.IdHigh == identity.IdHigh;
        }
        catch (ArchiveOutputException failure)
        {
            throw MapNativeFailure(failure, observed: false);
        }
    }

    private Snapshot CaptureSnapshot(SafeFileHandle handle, bool directory, bool observed)
    {
        try
        {
            RequireDisk(handle);
            var before = ReadBasicInfo(handle, observed);
            ValidateBasicKind(before, directory, observed);
            var identity = OutputNative.Identity(handle);
            var after = ReadBasicInfo(handle, observed);
            ValidateBasicKind(after, directory, observed);
            if (before.LastWriteTime != after.LastWriteTime || before.ChangeTime != after.ChangeTime ||
                before.FileAttributes != after.FileAttributes || identity.Modified != after.LastWriteTime || identity.Size < 0)
                throw new ArchiveOutputException("source_changed");
            return new Snapshot(identity.Volume, identity.IdLow, identity.IdHigh, (ulong)identity.Size,
                identity.Modified, after.ChangeTime, DosTime(identity.Modified));
        }
        catch (ArchiveOutputException failure)
        {
            throw MapNativeFailure(failure, observed);
        }
    }

    private void VerifyRecord(SourceRecord record, SafeFileHandle handle)
    {
        var current = CaptureSnapshot(handle, record.Directory, observed: true);
        if (record.Volume != current.Volume || record.IdLow != current.IdLow || record.IdHigh != current.IdHigh ||
            record.Size != current.Size || record.Modified != current.Modified || record.ChangeTime != current.ChangeTime)
            throw new ArchiveOutputException("source_changed");
    }

    private static void RequireDisk(SafeFileHandle handle)
    {
        Marshal.SetLastPInvokeError(0);
        var kind = GetFileTypeNative(handle);
        if (kind == FileTypeDisk) return;
        var error = Marshal.GetLastWin32Error();
        throw new ArchiveOutputException("source_unavailable", error == 0 ? null : error);
    }

    private static FileBasicInfo ReadBasicInfo(SafeFileHandle handle, bool observed)
    {
        if (GetBasicInfo(handle, FileInfoByHandleClass.FileBasicInfo, out var info,
                (uint)Marshal.SizeOf<FileBasicInfo>()))
            return info;
        var failure = new ArchiveOutputException("source_unavailable", Marshal.GetLastWin32Error());
        throw MapNativeFailure(failure, observed);
    }

    private static void ValidateBasicKind(FileBasicInfo info, bool directory, bool observed)
    {
        if ((info.FileAttributes & (uint)FileAttributes.ReparsePoint) != 0)
            throw new ArchiveOutputException("reparse_component");
        if (((info.FileAttributes & (uint)FileAttributes.Directory) != 0) != directory)
            throw new ArchiveOutputException(observed ? "source_changed" : "source_unavailable");
    }

    private static SafeFileHandle OpenRelative(SafeFileHandle? parent, string name, bool directory, bool observed)
    {
        try { return OutputNative.Open(parent, name, directory, readOnly: true); }
        catch (ArchiveOutputException failure) { throw MapNativeFailure(failure, observed); }
    }

    private static ArchiveOutputException MapNativeFailure(ArchiveOutputException failure, bool observed)
    {
        if (failure.Stage is "source_changed" or "source_unavailable" or "reparse_component" or
            "session_entry_limit" or "read_limit" or "invalid_source_call" or "source_output_collision")
            return failure;
        if (observed && (failure.Stage == "wrong_kind" || failure.NativeCode is 2 or 3 or 32 or 33))
            return new ArchiveOutputException("source_changed", failure.NativeCode);
        return new ArchiveOutputException("source_unavailable", failure.NativeCode);
    }

    private static ArchiveOutputException MapIoFailure(Exception failure, bool observed)
    {
        var native = failure is UnauthorizedAccessException ? 5 : NativeCode(failure);
        return observed && native is 2 or 3 or 32 or 33
            ? new ArchiveOutputException("source_changed", native)
            : new ArchiveOutputException("source_unavailable", native);
    }


    private static ushort[]? DosTime(long fileTime)
    {
        try
        {
            var local = DateTime.FromFileTimeUtc(fileTime).ToLocalTime();
            if (local.Year is < 1980 or > 2107) return null;
            var date = (ushort)(((local.Year - 1980) << 9) | (local.Month << 5) | local.Day);
            var time = (ushort)((local.Hour << 11) | (local.Minute << 5) | (local.Second / 2));
            return [date, time];
        }
        catch (ArgumentOutOfRangeException)
        {
            return null;
        }
    }

    private object ToDto(SourceRecord record) => new
    {
        id = record.Id,
        name = record.Name,
        directory = record.Directory,
        size = record.Size,
        dos_modified = record.DosModified,
    };

    private SourceRecord Record(Guid token)
    {
        if (token == Guid.Empty || !records.TryGetValue(token, out var record))
            throw new ArchiveOutputException("invalid_source_call");
        return record;
    }

    private Guid NewRecordId()
    {
        for (var attempt = 0; attempt < 4; attempt++)
        {
            var id = Guid.NewGuid();
            if (id != Guid.Empty && !records.ContainsKey(id)) return id;
        }
        throw new ArchiveOutputException("source_unavailable");
    }

    private void ReserveRecord(ulong amount)
    {
        if (trackedRecords >= maximumRecords || amount > maximumMetadata - Math.Min(metadataUsed, maximumMetadata))
            throw new ArchiveOutputException("session_entry_limit");
        trackedRecords++;
        metadataUsed += amount;
    }

    private void ReleaseRecord(ulong amount)
    {
        trackedRecords--;
        metadataUsed -= amount;
    }

    private void ReserveObservedBytes(ulong size)
    {
        if (size > maximumEntryBytes || size > maximumTotalBytes - Math.Min(totalObservedBytes, maximumTotalBytes))
            throw new ArchiveOutputException("read_limit");
        totalObservedBytes += size;
    }

    private static ulong RootReservation(int pathLength, int depth)
    {
        try
        {
            return checked(RootBaseMetadataReservation + (ulong)pathLength * sizeof(char) +
                (ulong)(depth + 1) * RootHandleMetadataReservation);
        }
        catch (OverflowException)
        {
            throw new ArchiveOutputException("session_entry_limit");
        }
    }

    private static Guid ReadToken(JsonElement call)
    {
        if (!call.TryGetProperty("token", out var token) || !token.TryGetGuid(out var value) || value == Guid.Empty)
            throw new ArchiveOutputException("invalid_source_call");
        return value;
    }

    private static ulong ReadNextOffset(JsonElement call)
    {
        if (!call.TryGetProperty("offset", out var offset) || !offset.TryGetUInt64(out var value))
            throw new ArchiveOutputException("invalid_source_call");
        return value;
    }

    private static ulong ReadOffset(JsonElement call)
    {
        if (!call.TryGetProperty("offset", out var offset) || !offset.TryGetUInt64(out var value))
            throw new ArchiveOutputException("read_limit");
        return value;
    }

    private static int ReadLength(JsonElement call)
    {
        if (!call.TryGetProperty("length", out var length) || !length.TryGetInt32(out var value))
            throw new ArchiveOutputException("read_limit");
        return value;
    }

    private static int ValidateAbsolutePath(string path)
    {
        if (string.IsNullOrEmpty(path) || path.Length is < 4 or > 32767 || !char.IsAsciiLetter(path[0]) ||
            path[1] != ':' || path[2] != '\\' || path.IndexOfAny(['/', '\0']) >= 0)
            throw new ArchiveOutputException("invalid_source");
        var parts = path[3..].Split('\\');
        if (parts.Length is < 1 or > MaximumDepth)
            throw new ArchiveOutputException("invalid_source");
        try
        {
            foreach (var part in parts) ArchiveOutputSession.Validate(part);
        }
        catch (ArchiveOutputException)
        {
            throw new ArchiveOutputException("invalid_source");
        }
        return parts.Length;
    }

    private static bool PathsOverlap(string left, string right) =>
        string.Equals(left, right, StringComparison.OrdinalIgnoreCase) || IsSameOrDescendant(left, right) ||
        IsSameOrDescendant(right, left);

    private static bool IsSameOrDescendant(string path, string possibleAncestor) =>
        string.Equals(path, possibleAncestor, StringComparison.OrdinalIgnoreCase) ||
        path.Length > possibleAncestor.Length && path[possibleAncestor.Length] == '\\' &&
        path.StartsWith(possibleAncestor, StringComparison.OrdinalIgnoreCase);

    private static int? NativeCode(Exception failure)
    {
        var value = failure.HResult & 0xffff;
        return value == 0 ? null : value;
    }

    private void CheckOpen()
    {
        if (disposed) throw new ArchiveOutputException("invalid_source_call");
    }

    public void Dispose()
    {
        if (disposed) return;
        disposed = true;
        try { AbortEnumeration(); }
        catch (Exception) { /* Disposal only releases owned capabilities. */ }
        if (activeFile is { } file)
        {
            activeFile = null;
            try { file.Chain.Dispose(); }
            catch (Exception) { /* Continue releasing root chains. */ }
        }
        for (var index = roots.Count - 1; index >= 0; index--)
        {
            try { roots[index].Dispose(); }
            catch (Exception) { /* Safe handles are independently released. */ }
        }
        roots.Clear();
        records.Clear();
    }
}
