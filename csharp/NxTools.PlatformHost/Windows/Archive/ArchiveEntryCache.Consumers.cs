namespace NxTools.PlatformHost.Windows.Archive;

internal sealed partial class ArchiveEntryCache
{
    private readonly Dictionary<Guid, int> consumerHolds = [];
    internal bool HoldConsumer(Guid id, Guid operation, Guid session, ulong generation, out long size)
    {
        lock (this) {
            size = 0;
            if (!Owns(id, operation, session, generation) || leases[id].SealedSize is not { } sealedSize) return false;
            consumerHolds[id] = consumerHolds.GetValueOrDefault(id) + 1;
            size = sealedSize; return true;
        }
    }
    internal IntPtr OpenConsumerVirtualStream(Guid id, Guid operation, Guid session, ulong generation, long? expected, CancellationToken stop, out long actual)
    {
        lock (this) {
            if (!consumerHolds.ContainsKey(id) || !leases.TryGetValue(id, out var lease) || lease.Operation != operation || lease.Session != session || lease.Generation != generation)
                throw new ArchiveOutputException("source_unavailable");
            return lease.OpenVirtualStream(expected, stop, out actual);
        }
    }
    internal void EndConsumer(Guid id)
    {
        lock (this) {
            if (!consumerHolds.TryGetValue(id, out var count)) throw new ArchiveOutputException("invalid_request");
            if (count > 1) consumerHolds[id] = count - 1;
            else { consumerHolds.Remove(id); if (released.Contains(id)) Release(id); }
        }
    }
}
