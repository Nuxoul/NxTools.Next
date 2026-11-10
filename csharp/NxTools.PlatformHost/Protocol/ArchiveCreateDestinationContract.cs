using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveCreateDestinationCommand(uint Version, Guid OperationId, Guid SessionId, ulong Generation, string Destination)
{
    internal bool Valid => Version == 1 && OperationId != Guid.Empty && SessionId != Guid.Empty && Generation > 0 && ArchiveJobWire.PathValid(Destination);
    internal static ArchiveCreateDestinationCommand Parse(ProtocolEnvelope envelope)
    {
        var value = ArchiveJobWire.Data(envelope, "archive_create_destination").Deserialize<ArchiveCreateDestinationCommand>(ArchiveJobWire.Strict);
        if (value is null || !value.Valid || envelope.CorrelationId != value.OperationId) throw new InvalidDataException("invalid_request");
        return value;
    }
    public override string ToString() => "ArchiveCreateDestinationCommand([PRIVATE])";
}

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveCreateDestinationResult(uint Version, Guid OperationId, Guid SessionId, ulong Generation,
    bool DestinationMissing, string[] ExistingNames,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Code = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? NativeErrorCode = null);
