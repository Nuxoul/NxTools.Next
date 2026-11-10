using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows.Archive;

// A single native admission owns both the COM descriptor order and original member identities.
internal sealed class ArchiveVirtualPlan
{
    private readonly ArchiveVirtualDragItem[] items;
    private readonly ArchiveVirtualFile[] descriptors;
    internal int Count => items.Length;
    internal ArchiveVirtualDragItem this[int index] => items[index];
    internal ArchiveVirtualDragItem[] Items => items.ToArray();
    internal ArchiveVirtualFile[] Descriptors => descriptors.ToArray();
    internal ArchiveDragMember[] Members => items.Select((item, index) => (item, index))
        .Where(value => !value.item.Directory).Select(value => new ArchiveDragMember(value.index, value.item.Entry.Ordinal!.Value)).ToArray();
    private ArchiveVirtualPlan(ArchiveVirtualDragItem[] items, ArchiveVirtualFile[] descriptors)
    { this.items = items; this.descriptors = descriptors; }
    internal static ArchiveVirtualPlan Admit(ArchiveVirtualDragItem[] items)
    {
        var frozen = items.Select(item => item with { Entry = item.Entry with {
            Browser = item.Entry.Browser is { } browser ? browser with { Source = browser.Source with { Digest = browser.Source.Digest.ToArray() } } : null
        } }).ToArray();
        var files = frozen.Select(item => new ArchiveVirtualFile(item.Name, (long?)item.Size, item.Directory)).ToArray();
        if (!ArchiveVirtualFileDataObject.ValidTree(files) || frozen.Any(item => !item.Directory && item.Entry.Ordinal is null))
            throw new ArchiveOutputException("invalid_virtual_files");
        return new(frozen, files);
    }
}

internal sealed record ArchiveDragMember(int index, ulong ordinal);
internal sealed record ArchiveDragDemand(Guid RequestId, int Index, ulong Ordinal, ulong OutputLimit, string OutputRoot);
internal sealed record ArchiveDragMaterialization(ArchiveDragMember[] Members, ArchiveDragDemand First);
