using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows.Archive;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record RawNumberedJoinCommand(uint Version, Guid OperationId, Guid SessionId, ulong Generation, string Action,
    string? SelectedPath = null, Guid? PreviewId = null, string? OutputPath = null)
{
    internal bool Valid => Version == 1 && OperationId != Guid.Empty && SessionId != Guid.Empty && Generation > 0 && (Action switch
    {
        "inspect" => ArchiveJobWire.PathValid(SelectedPath) && PreviewId is null && OutputPath is null,
        "start" => SelectedPath is null && PreviewId is {} id && id != Guid.Empty && ArchiveJobWire.PathValid(OutputPath),
        "cancel" => SelectedPath is null && PreviewId is null && OutputPath is null,
        _ => false
    });
    internal static RawNumberedJoinCommand Parse(ProtocolEnvelope envelope)
    {
        var command = ArchiveJobWire.Data(envelope, "raw_numbered_join").Deserialize<RawNumberedJoinCommand>(ArchiveJobWire.Strict);
        if (command is null || !command.Valid || envelope.CorrelationId != command.OperationId) throw new InvalidDataException("invalid_request");
        return command;
    }
    public override string ToString() => "RawNumberedJoinCommand([PRIVATE])";
}
internal sealed record RawJoinPreviewPart(string Name, long Length);
internal sealed record RawJoinPreviewData(Guid PreviewId, string SuggestedOutput, RawJoinPreviewPart[] Parts, long TotalBytes);
internal sealed record RawNumberedJoinReply(uint Version, Guid OperationId, Guid SessionId, ulong Generation,
    RawJoinPreviewData? Preview = null, RawJoinResult? Terminal = null);
