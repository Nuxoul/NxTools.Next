using System.Text.Json;
using System.Text.Json.Serialization;
namespace NxTools.PlatformHost.Protocol;
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchivePickerCommand {
    public required uint Version {get;init;}
    public required Guid OperationId {get;init;}
    public required Guid SessionId {get;init;}
    public required ulong Generation {get;init;}
    public required long OwnerWindowHandle {get;init;}
    public required string Purpose {get;init;}
    public string? CurrentDirectory {get;init;}
    public string? SuggestedName {get;init;}
    internal static ArchivePickerCommand Parse(ProtocolEnvelope envelope) {
        var c=ArchiveJobWire.Data(envelope,"archive_picker").Deserialize<ArchivePickerCommand>(ArchiveJobWire.Strict)??throw new InvalidDataException();
        if(c.Version!=1 || c.OperationId==Guid.Empty || c.SessionId==Guid.Empty || c.Generation==0 || c.OwnerWindowHandle==0 || envelope.CorrelationId!=c.OperationId || c.Purpose is not ("files" or "folders" or "destination" or "save") || c.CurrentDirectory is not null && !ArchiveJobWire.PathValid(c.CurrentDirectory) || c.SuggestedName is not null && (c.Purpose!="save" || !ArchiveJobWire.NameValid(c.SuggestedName)))throw new InvalidDataException();
        return c;
    }
}
internal sealed record ArchiveOwnedPickerResult(uint Version,Guid OperationId,Guid SessionId,ulong Generation,string[] Paths,bool Cancelled,string? Code=null,int? NativeErrorCode=null) {
    internal ProtocolEnvelope Envelope()=>ProtocolEnvelope.Create("platform_event",new{@event="archive_picker",data=this},OperationId);
}
