using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows;
namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveSpaceCommand(uint Version, Guid OperationId, Guid SessionId, ulong Generation,
    string Source, string Destination, string? ExpectedSource, string? ExpectedTarget)
{
    private static bool Identity(string? value) => value is null || value.Length is >=3 and <=256 && value.StartsWith("v1:",StringComparison.Ordinal) && value.All(c=>c is >= ' ' and <= '~');
    internal bool Valid => Version is (1 or 2) && OperationId!=Guid.Empty && SessionId!=Guid.Empty && Generation>0 &&
        ArchiveJobWire.PathValid(Source) && ArchiveJobWire.PathValid(Destination) && Identity(ExpectedSource) && Identity(ExpectedTarget);
    internal static ArchiveSpaceCommand Parse(ProtocolEnvelope envelope)
    {
        var command=envelope.Payload.GetProperty("data").Deserialize<ArchiveSpaceCommand>(ArchiveJobWire.Strict);
        if(command is null || !command.Valid) throw new InvalidDataException("invalid_request");
        return command;
    }
    public override string ToString()=>"ArchiveSpaceCommand([PRIVATE])";
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveSpaceSourceSet(string Identity,string Primary,IReadOnlyList<string> Members){public override string ToString()=>"ArchiveSpaceSourceSet([PRIVATE])";internal bool Valid=>Identity.StartsWith("v1:",StringComparison.Ordinal)&&Identity.Length<=256&&Identity.All(c=>c is >= ' ' and <= '~')&&ArchiveJobWire.PathValid(Primary)&&Members.Count is >0 and <=1024&&Members.All(ArchiveJobWire.PathValid)&&Members.Contains(Primary)&&Members.Distinct(StringComparer.Ordinal).Count()==Members.Count&&Members.Sum(p=>(long)System.Text.Encoding.UTF8.GetByteCount(p))<=1048576;}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveSpaceObservation(uint Version, Guid OperationId, Guid SessionId, ulong Generation,
    ulong AvailableBytes, ulong AllocationBytes, string TargetIdentity, string VolumeIdentity, bool DestinationMissing, bool SourceMatches,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] uint? MissingDestinationDirectories=null,
    [property:JsonIgnore(Condition=JsonIgnoreCondition.WhenWritingNull)] ArchiveSpaceSourceSet? SourceSet=null)
{
    internal bool ValidFor(ArchiveSpaceCommand command) => Version==command.Version && (SourceSet is null || Version==2&&SourceSet.Valid&&SourceSet.Members.Contains(command.Source)) && OperationId==command.OperationId &&
        SessionId==command.SessionId && Generation==command.Generation &&
        AllocationBytes is >0 and <=16777216 && (AllocationBytes&(AllocationBytes-1))==0 &&
        (MissingDestinationDirectories is null || MissingDestinationDirectories<=64 && (MissingDestinationDirectories!=0)==DestinationMissing);
}
