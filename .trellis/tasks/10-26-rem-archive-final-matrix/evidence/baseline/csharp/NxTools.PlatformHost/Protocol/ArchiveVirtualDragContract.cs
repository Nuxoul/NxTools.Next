using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveVirtualDragItem(string Name,ulong? Size,ArchiveEntryCommand Entry,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingDefault)] bool Directory=false);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveVirtualDragCommand(uint Version,Guid OperationId,Guid SessionId,ulong Generation,
    string Action,long SourceWindowHandle,ArchiveVirtualDragItem[]? Items=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] Guid? AuthorizationId=null);
internal static class ArchiveVirtualDragWire
{
    private static void Unique(JsonElement value)
    {
        if(value.ValueKind==JsonValueKind.Object) {
            var names=new HashSet<string>(StringComparer.Ordinal);
            foreach(var property in value.EnumerateObject()){if(!names.Add(property.Name))throw new InvalidDataException();Unique(property.Value);}
        }else if(value.ValueKind==JsonValueKind.Array)foreach(var item in value.EnumerateArray())Unique(item);
    }
    internal static ArchiveVirtualDragCommand Parse(ProtocolEnvelope envelope)
    {
        Unique(envelope.Payload);
        if(JsonSerializer.SerializeToUtf8Bytes(envelope.Payload).Length>262144||
            envelope.Payload.EnumerateObject().Any(p=>p.Name is not ("command" or "data"))||
            envelope.CorrelationId is not {} correlation || envelope.Payload.GetProperty("command").GetString() is not ("archive_virtual_drag" or "archive_virtual_drag_spike"))throw new InvalidDataException();
        var value=envelope.Payload.GetProperty("data").Deserialize<ArchiveVirtualDragCommand>(ArchiveJobWire.Strict)??throw new InvalidDataException();
        if(value.Version!=1||value.OperationId==Guid.Empty||value.SessionId==Guid.Empty||value.Generation==0||value.OperationId!=correlation||value.OperationId==envelope.Id||value.AuthorizationId==Guid.Empty)throw new InvalidDataException();
        if(value.Action=="cancel") {if(value.Items is not null||value.SourceWindowHandle!=0||value.AuthorizationId is not null)throw new InvalidDataException();return value;}
        if(value.Action!="start"||value.SourceWindowHandle<=0||value.Items is not {Length:>=1 and <=128})throw new InvalidDataException();
        if(value.Items.Any(item=>item is null||item.Size>536870912)||
            !ArchiveVirtualFileDataObject.ValidTree(value.Items.Select(item=>new ArchiveVirtualFile(item.Name,(long?)item.Size,item.Directory)).ToArray()))throw new InvalidDataException();
        var operations=new HashSet<Guid>();var ordinals=new HashSet<ulong>();
        ArchiveEntryCommand? first=null;
        var rawItems=envelope.Payload.GetProperty("data").GetProperty("items");var index=0;
        foreach(var item in value.Items) {
            var e=item.Entry??throw new InvalidDataException();
            // Preserve the entry parser's exact browser/source shape, including required zero-valued
            // source fields. Only this descriptor parser may validate an ordinal-less directory clone.
            var raw=rawItems[index++].GetProperty("entry");
            var browser=raw.GetProperty("browser");
            if(browser.ValueKind!=JsonValueKind.Object||!browser.TryGetProperty("source",out var source)||source.ValueKind!=JsonValueKind.Object)throw new InvalidDataException();
            foreach(var key in new[]{"snapshot_id","format","comment","source"})if(!browser.TryGetProperty(key,out _))throw new InvalidDataException();
            foreach(var key in new[]{"native","digest","size","modified_unix_ns"})if(!browser.GetProperty("source").TryGetProperty(key,out _))throw new InvalidDataException();
            var shape=item.Directory&&e.Ordinal is null?e with{Ordinal=0}:e;
            ArchiveEntryWire.Parse(ProtocolEnvelope.Create("platform_command",new{command="archive_entry",data=shape},e.OperationId));
            if(e.Action!="materialize"||e.SessionId!=value.SessionId||e.Generation!=value.Generation||e.LeaseId is not null||e.PasswordInteraction||!operations.Add(e.OperationId)||e.Ordinal is {} ordinal&&!ordinals.Add(ordinal))throw new InvalidDataException();
            if(first is not null&&(e.Path!=first.Path||e.Codepage!=first.Codepage||e.Depth!=first.Depth||e.CopyMotw!=first.CopyMotw||e.Browser!.SnapshotId!=first.Browser!.SnapshotId||e.Browser.Format!=first.Browser.Format||e.Browser.Comment!=first.Browser.Comment||e.Browser.Source.Native!=first.Browser.Source.Native||e.Browser.Source.Size!=first.Browser.Source.Size||e.Browser.Source.ModifiedUnixNs!=first.Browser.Source.ModifiedUnixNs||!e.Browser.Source.Digest.SequenceEqual(first.Browser.Source.Digest)))throw new InvalidDataException();
            first=e;
        }
        return value;
    }
}
