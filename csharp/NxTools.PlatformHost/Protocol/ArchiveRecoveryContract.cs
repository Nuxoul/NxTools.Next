using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveRecoveryInspectCommand(uint Version, Guid OperationId, Guid SessionId, ulong Generation,
    string Source, string? Destination)
{
    internal bool Valid => Version==1 && OperationId!=Guid.Empty && SessionId!=Guid.Empty && Generation!=0 &&
        ArchiveJobWire.PathValid(Source) && ArchiveRecoveryPath.Valid(Source) &&
        (Destination is null || ArchiveJobWire.PathValid(Destination) && ArchiveRecoveryPath.Valid(Destination));
    internal static ArchiveRecoveryInspectCommand Parse(ProtocolEnvelope envelope)
    {
        var command=envelope.Payload.GetProperty("data").Deserialize<ArchiveRecoveryInspectCommand>(ArchiveJobWire.Strict);
        if(command is null || !command.Valid) throw new InvalidDataException("invalid_request");
        return command;
    }
    public override string ToString()=>"ArchiveRecoveryInspectCommand([PRIVATE])";
}
internal static class ArchiveRecoveryPath
{
    internal static bool Valid(string path) => path[3..].Split('\\').Length<=64 && path[3..].Split('\\').All(p=>
        p.Length is >=1 and <=255 && p is not ("." or "..") && !p.EndsWith('.') && !p.EndsWith(' ') &&
        p.IndexOfAny([':', '<', '>', '"', '|', '?', '*'])<0);
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveRecoveryInspection(uint Version,Guid OperationId,Guid SessionId,ulong Generation,
    Guid RecoveryId,string SourceIdentity,string? Destination,[property:JsonRequired] bool CopyMotw,[property:JsonRequired] bool Completed)
{
    private static bool IdentityValid(string identity) {
        var parts=identity.Split(':');
        return parts.Length==4 && parts[0]=="v1" && parts[1].Length is >=1 and <=16 &&
            parts[2].Length==32 && parts[3].Length is >=1 and <=16 &&
            parts.Skip(1).All(part=>part.All(Uri.IsHexDigit));
    }
    internal bool ValidFor(ArchiveRecoveryInspectCommand command) => command.Valid && Version==1 &&
        OperationId==command.OperationId && SessionId==command.SessionId && Generation==command.Generation &&
        RecoveryId!=Guid.Empty && IdentityValid(SourceIdentity) &&
        (Destination is null || ArchiveJobWire.PathValid(Destination) && ArchiveRecoveryPath.Valid(Destination) &&
            (command.Destination is null ? Destination==Path.GetDirectoryName(command.Source) : Destination==command.Destination));
    public override string ToString()=>"ArchiveRecoveryInspection([PRIVATE])";
}
