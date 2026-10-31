using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxTools.PlatformHost.Protocol;

internal static class ProtocolConstants
{
    public const ushort Version = 1;
    public const int MaxFrameBytes = 1024 * 1024;
    public const string DefaultPipeName = "nxtools-next-platform-v1";
}

internal sealed record ProtocolEnvelope
{
    [JsonPropertyName("protocol_version")]
    public required ushort ProtocolVersion { get; init; }

    [JsonPropertyName("id")]
    public required Guid Id { get; init; }

    [JsonPropertyName("correlation_id")]
    public Guid? CorrelationId { get; init; }

    [JsonPropertyName("type")]
    public required string Type { get; init; }

    [JsonPropertyName("payload")]
    public required JsonElement Payload { get; init; }

    public static ProtocolEnvelope Create<T>(string type, T payload, Guid? correlationId = null) =>
        new()
        {
            ProtocolVersion = ProtocolConstants.Version,
            Id = Guid.NewGuid(),
            CorrelationId = correlationId,
            Type = type,
            Payload = JsonSerializer.SerializeToElement(payload, JsonDefaults.Options)
        };

    public void Validate()
    {
        if (ProtocolVersion != ProtocolConstants.Version)
        {
            throw new InvalidDataException(
                $"Unsupported protocol version {ProtocolVersion}; expected {ProtocolConstants.Version}.");
        }
    }
}

internal static class JsonDefaults
{
    public static readonly JsonSerializerOptions Options = new(JsonSerializerDefaults.Web)
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower
    };
}
