using System.Text;
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
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] Guid? AuthorizationId=null)
{
    [JsonIgnore] internal ArchiveVirtualPlan? Plan { get; init; }
}
// BytesDone counts verified sealed cache bytes; BytesProcessed includes only active decoded bytes.
// No target path or destination-write measurement is supplied by this native contract.
internal sealed record ArchiveVirtualDragProgress(uint Version,Guid OperationId,Guid SessionId,ulong Generation,
    string Phase,ulong BytesDone,ulong? BytesTotal,ulong FilesDone,ulong FilesTotal,
    int? CurrentIndex=null,string? CurrentMember=null,bool? CleanupComplete=null,string? Code=null,int? NativeErrorCode=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] Guid? CurrentRequestId=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ulong? CurrentBytesDone=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ulong? CurrentBytesTotal=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ulong? BytesProcessed=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ulong? Sequence=null)
{
    internal ProtocolEnvelope Envelope()=>ProtocolEnvelope.Create("platform_event",new{ @event="archive_virtual_drag_progress",data=this },OperationId);
}
internal static class ArchiveVirtualDragWire
{
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record Source(string Path,uint Codepage,ArchiveBrowserContext Browser,uint Depth,bool? CopyMotw);
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record Identity(Guid OperationId,ulong? Ordinal);
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record Item(string Name,ulong? Size,JsonElement Entry,bool Directory=false);
    [JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
    private sealed record Command(uint Version,Guid OperationId,Guid SessionId,ulong Generation,string Action,
        long SourceWindowHandle,Item[]? Items=null,Guid? AuthorizationId=null,JsonElement? Source=null);
    private static void Unique(JsonElement value)
    {
        if(value.ValueKind==JsonValueKind.Object) {
            var names=new HashSet<string>(StringComparer.Ordinal);
            foreach(var property in value.EnumerateObject()){if(!names.Add(property.Name))throw new InvalidDataException();Unique(property.Value);}
        }else if(value.ValueKind==JsonValueKind.Array)foreach(var item in value.EnumerateArray())Unique(item);
    }
    private static void BrowserShape(JsonElement browser)
    {
        if(browser.ValueKind!=JsonValueKind.Object)throw new InvalidDataException();
        foreach(var key in new[]{"snapshot_id","format","comment","source"})if(!browser.TryGetProperty(key,out _))throw new InvalidDataException();
        var source=browser.GetProperty("source");
        if(source.ValueKind!=JsonValueKind.Object)throw new InvalidDataException();
        foreach(var key in new[]{"native","digest","size","modified_unix_ns"})if(!source.TryGetProperty(key,out _))throw new InvalidDataException();
    }
    internal static ArchiveVirtualDragCommand Parse(ProtocolEnvelope envelope)
    {
        Unique(envelope.Payload);
        // The pipe owns the complete frame bound. Do not re-encode Unicode into a smaller business quota.
        if(Encoding.UTF8.GetByteCount(envelope.Payload.GetRawText())>ProtocolConstants.MaxFrameBytes||
            envelope.Payload.EnumerateObject().Any(p=>p.Name is not ("command" or "data"))||
            envelope.CorrelationId is not {} correlation || envelope.Payload.GetProperty("command").GetString() is not ("archive_virtual_drag" or "archive_virtual_drag_spike"))throw new InvalidDataException();
        var raw=envelope.Payload.GetProperty("data");
        var value=raw.Deserialize<Command>(ArchiveJobWire.Strict)??throw new InvalidDataException();
        if(value.Version!=1||value.OperationId==Guid.Empty||value.SessionId==Guid.Empty||value.Generation==0||value.OperationId!=correlation||value.OperationId==envelope.Id||value.AuthorizationId==Guid.Empty)throw new InvalidDataException();
        if(value.Action=="cancel") {
            if(value.Items is not null||value.SourceWindowHandle!=0||value.AuthorizationId is not null||value.Source is {ValueKind:not JsonValueKind.Null})throw new InvalidDataException();
            return new(value.Version,value.OperationId,value.SessionId,value.Generation,value.Action,0);
        }
        var locate=value.Action=="locate";
        if(value.Action is not ("start" or "locate")||value.SourceWindowHandle<=0||value.Items is not {Length:>=1})throw new InvalidDataException();
        // Destination location carries one descriptor; directories use a wrapper folder, while
        // file descriptors use a temporary marker file so the receiver's parent is the target.
        if(locate&&(value.Items.Length!=1||value.Items[0] is not {Name:{} name}||name.Contains('/')||
            value.AuthorizationId is not null||value.Source is {ValueKind:not JsonValueKind.Null}))throw new InvalidDataException();
        Source? frozen=null;
        if(value.Source is {ValueKind:not JsonValueKind.Null} shared) {
            if(shared.ValueKind!=JsonValueKind.Object)throw new InvalidDataException();
            foreach(var key in new[]{"path","codepage","browser","depth","copy_motw"})if(!shared.TryGetProperty(key,out _))throw new InvalidDataException();
            BrowserShape(shared.GetProperty("browser"));
            frozen=shared.Deserialize<Source>(ArchiveJobWire.Strict)??throw new InvalidDataException();
        }
        var items=new ArchiveVirtualDragItem[value.Items.Length];
        var operations=new HashSet<Guid>();var ordinals=new HashSet<ulong>();
        ArchiveEntryCommand? first=null;
        for(var index=0;index<items.Length;index++) {
            var item=value.Items[index]??throw new InvalidDataException();
            ArchiveEntryCommand e;
            if(frozen is not null) {
                var identity=item.Entry.Deserialize<Identity>(ArchiveJobWire.Strict)??throw new InvalidDataException();
                e=new(){Version=1,OperationId=identity.OperationId,SessionId=value.SessionId,Generation=value.Generation,Action="materialize",
                    Path=frozen.Path,Codepage=frozen.Codepage,Browser=frozen.Browser,Depth=frozen.Depth,CopyMotw=frozen.CopyMotw,Ordinal=identity.Ordinal};
            }else {
                if(item.Entry.ValueKind!=JsonValueKind.Object||!item.Entry.TryGetProperty("browser",out var browser))throw new InvalidDataException();
                BrowserShape(browser);
                e=item.Entry.Deserialize<ArchiveEntryCommand>(ArchiveJobWire.Strict)??throw new InvalidDataException();
            }
            var shape=item.Directory&&e.Ordinal is null?e with{Ordinal=0}:e;
            ArchiveEntryWire.Parse(ProtocolEnvelope.Create("platform_command",new{command="archive_entry",data=shape},e.OperationId));
            if(e.Action!="materialize"||e.SessionId!=value.SessionId||e.Generation!=value.Generation||e.LeaseId is not null||e.PasswordInteraction||e.TempPolicy is not null||!operations.Add(e.OperationId)||e.Ordinal is {} ordinal&&!ordinals.Add(ordinal))throw new InvalidDataException();
            if(first is not null&&(e.Path!=first.Path||e.Codepage!=first.Codepage||e.Depth!=first.Depth||e.CopyMotw!=first.CopyMotw||e.Browser!.SnapshotId!=first.Browser!.SnapshotId||e.Browser.Format!=first.Browser.Format||e.Browser.Comment!=first.Browser.Comment||e.Browser.SevenzEdit!=first.Browser.SevenzEdit||e.Browser.TarEdit!=first.Browser.TarEdit||e.Browser.Source.Native!=first.Browser.Source.Native||e.Browser.Source.Size!=first.Browser.Source.Size||e.Browser.Source.ModifiedUnixNs!=first.Browser.Source.ModifiedUnixNs||!e.Browser.Source.Digest.SequenceEqual(first.Browser.Source.Digest)))throw new InvalidDataException();
            if(item.Size>(ulong)long.MaxValue||item.Name is null||item.Name.Split('/').Any(part=>part.Length>255||part.Any(char.IsControl)))throw new InvalidDataException();
            items[index]=new(item.Name,item.Size,e,item.Directory);first=e;
        }
        ArchiveVirtualPlan plan;
        try { plan=ArchiveVirtualPlan.Admit(items); }
        catch(ArchiveOutputException failure) when(failure.Stage=="invalid_virtual_files") { throw new InvalidDataException(); }
        return new(value.Version,value.OperationId,value.SessionId,value.Generation,value.Action,value.SourceWindowHandle,items,value.AuthorizationId) { Plan=plan };
    }
}
