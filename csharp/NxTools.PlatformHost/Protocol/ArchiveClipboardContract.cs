using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveClipboardCommand(uint Version, Guid OperationId, Guid SessionId, ulong Generation)
{
    internal bool Valid => Version == 1 && OperationId != Guid.Empty && SessionId != Guid.Empty && Generation > 0;
    internal static ArchiveClipboardCommand Parse(ProtocolEnvelope envelope)
    {
        var value = ArchiveJobWire.Data(envelope, "read_clipboard_file_drop").Deserialize<ArchiveClipboardCommand>(ArchiveJobWire.Strict);
        if (value is null || !value.Valid || envelope.CorrelationId != value.OperationId) throw new InvalidDataException("invalid_request");
        return value;
    }
}

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveClipboardResult(uint Version, Guid OperationId, Guid SessionId, ulong Generation, string[] Paths,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Code = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? NativeErrorCode = null);
