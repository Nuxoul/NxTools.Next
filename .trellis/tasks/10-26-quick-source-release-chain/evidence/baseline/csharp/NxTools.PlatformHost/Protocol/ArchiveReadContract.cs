using System.Text.Json;
using System.Text.Json.Serialization;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveReadLimits
{
    public ulong MaxEntries { get; init; } = 4096;
    public ulong MaxMetadataBytes { get; init; } = 4194304;
    public ulong MaxWorkingBytes { get; init; } = 67108864;
    public ulong MaxEntryBytes { get; init; } = 34359738368;
    public ulong MaxOutputBytes { get; init; } = 107374182400;
    public ulong MaxRetainedBytes { get; init; } = 4194304;
    public uint AckTimeoutMs { get; init; } = 2000;
    public ulong MaxTransportBytes { get; init; } = 8388608;
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
    public bool BrowserContext { get; init; }
    internal bool ValidFor(uint version) => MaxEntries > 0 && MaxEntries <= (version == 2 ? 100000ul : 4096ul) &&
        MaxMetadataBytes > 0 && MaxMetadataBytes <= (version == 2 ? 134217728ul : 4194304ul) &&
        MaxWorkingBytes is >= 67108864 and <= 536870912 && MaxEntryBytes > 0 && MaxOutputBytes > 0 &&
        MaxRetainedBytes > 0 && MaxRetainedBytes <= (version == 2 ? 67108864ul : 4194304ul) &&
        (version == 2 ? MaxTransportBytes is > 0 and <= 67108864 : MaxTransportBytes == 8388608 && !BrowserContext) && AckTimeoutMs is >= 200 and <= 10000;
}

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveReadCommand
{
    public required uint Version { get; init; }
    public required Guid OperationId { get; init; }
    public required Guid SessionId { get; init; }
    public required ulong Generation { get; init; }
    public required string Action { get; init; }
    public string? Path { get; init; }
    public uint Codepage { get; init; }
    public ArchiveReadLimits? Limits { get; init; }
    public ulong? Sequence { get; init; }
}
internal sealed record ArchiveReadReceipt(bool Accepted, string? Code);
internal sealed record ArchiveReadTransfer(ulong PrivateBytes, ulong ReceivedBytes, ulong MaxBatchEntries,
    ulong MaxBatchBytes, ulong MaxInflightBatches, ulong ProcessBudgetBytes);
internal sealed record ArchiveReadEntry(ulong Ordinal, string Name, string Kind, ulong? Size, ulong? PackedSize,
    bool? Encrypted, string[] WarningCodes, string Provenance, uint? Codepage,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] JsonElement? Modified = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ushort? Method = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] uint? Crc32 = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ArchiveBrowserAttributes? Attributes = null);
internal sealed record ArchiveReadBatch(Guid SnapshotId, ulong Sequence, string Format, ArchiveReadEntry[] Entries);
internal sealed record ArchiveReadTerminal(string Status, string? Code, ulong FilesVerified = 0, ulong BytesVerified = 0,
    ulong EntryCount = 0, ulong BatchCount = 0, ulong ElapsedMs = 0, int? NativeErrorCode = null, bool CleanupComplete = true,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ArchiveReadTransfer? Transfer = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ArchiveBrowserContext? Browser = null);
internal sealed record ArchiveReadEvent(uint Version, Guid OperationId, Guid SessionId, ulong Generation, string Kind,
    string[] Capabilities, ArchiveReadBatch? Batch = null, ArchiveReadTerminal? Terminal = null)
{
    internal static ArchiveReadEvent For(ArchiveReadCommand command, string kind, string[]? capabilities = null,
        ArchiveReadBatch? batch = null, ArchiveReadTerminal? terminal = null) =>
        new(command.Version, command.OperationId, command.SessionId, command.Generation, kind, capabilities ?? [], batch, terminal);
    internal ProtocolEnvelope Envelope() => ProtocolEnvelope.Create("platform_event", new { @event = "archive_read", data = this }, OperationId);
}

internal sealed class ArchiveReadVersionException : Exception;
internal static class ArchiveReadWire
{
    internal static readonly string[] RequiredCapabilities = ["product_read_v1", "metadata_credit_v1", "read_open_v1", "read_test_v1"];
    internal static readonly string[] LargeCapabilities = ["product_read_v2", "bounded_metadata_v2", "metadata_credit_v1", "read_open_v1", "read_test_v1"];
    internal static string[] Capabilities(uint version) => version == 2 ? LargeCapabilities : RequiredCapabilities;
    private static readonly JsonSerializerOptions Strict = new(JsonDefaults.Options) {
        PropertyNameCaseInsensitive = false, NumberHandling = JsonNumberHandling.Strict };
    internal static ArchiveReadCommand Parse(ProtocolEnvelope envelope)
    {
        RejectDuplicates(envelope.Payload);
        if (JsonSerializer.SerializeToUtf8Bytes(envelope.Payload, JsonDefaults.Options).Length > 262144 ||
            envelope.Payload.EnumerateObject().Any(p => p.Name is not ("command" or "data")) ||
            envelope.Payload.GetProperty("command").GetString() != "archive_read") throw new InvalidDataException();
        var data = envelope.Payload.GetProperty("data");
        RejectDuplicates(data);
        if (data.EnumerateObject().Any(p => p.Name is not ("version" or "operation_id" or "session_id" or "generation" or "action" or "path" or "codepage" or "limits" or "sequence"))) throw new InvalidDataException();
        if (data.TryGetProperty("limits", out var limits) && limits.ValueKind == JsonValueKind.Object &&
            limits.EnumerateObject().Any(p => p.Name is not ("max_entries" or "max_metadata_bytes" or "max_working_bytes" or "max_entry_bytes" or "max_output_bytes" or "max_retained_bytes" or "ack_timeout_ms" or "max_transport_bytes" or "browser_context"))) throw new InvalidDataException();
        var command = data.Deserialize<ArchiveReadCommand>(Strict) ?? throw new InvalidDataException();
        if (command.Version is not (1 or 2)) throw new ArchiveReadVersionException();
        if (command.OperationId == Guid.Empty || command.SessionId == Guid.Empty || command.Generation == 0 ||
            envelope.CorrelationId != command.OperationId || envelope.Id == command.OperationId ||
            command.Action is not ("open" or "test" or "ack" or "cancel") ||
            command.Codepage is not (0 or 65001 or 936 or 950 or 932 or 949 or 437 or 1250 or 1251 or 1252)) throw new InvalidDataException();
        if (command.Action is "open" or "test")
        {
            var path = command.Path;
            if (path is null || path.Length is < 4 or > 32767 || !char.IsAsciiLetter(path[0]) || path[1] != ':' || path[2] != '\\' ||
                path.IndexOfAny(['/', '\0']) >= 0 || command.Sequence is not null || command.Version == 2 && command.Limits is null ||
                command.Limits is { } requested && !requested.ValidFor(command.Version)) throw new InvalidDataException();
        }
        else if (command.Path is not null || command.Limits is not null || command.Codepage != 0 ||
            (command.Action == "ack") != command.Sequence.HasValue) throw new InvalidDataException();
        return command;
    }
    private static void RejectDuplicates(JsonElement data)
    {
        if (data.ValueKind != JsonValueKind.Object) throw new InvalidDataException();
        var keys = new HashSet<string>(StringComparer.Ordinal);
        foreach (var property in data.EnumerateObject())
        {
            if (!keys.Add(property.Name)) throw new InvalidDataException();
            if (property.Name == "limits" && property.Value.ValueKind != JsonValueKind.Null) RejectDuplicates(property.Value);
        }
    }
    internal static ArchiveReadCommand? RejectionIdentity(ProtocolEnvelope envelope)
    {
        try {
            var data = envelope.Payload.GetProperty("data"); RejectDuplicates(data);
            var id = data.GetProperty("operation_id").GetGuid(); var session = data.GetProperty("session_id").GetGuid();
            var generation = data.GetProperty("generation").GetUInt64();
            var action = data.GetProperty("action").GetString();
            if (id == Guid.Empty || session == Guid.Empty || generation == 0 || envelope.CorrelationId != id || action is "ack" or "cancel") return null;
            return new() { Version = data.TryGetProperty("version", out var version) && version.TryGetUInt32(out var number) ? number : 1, OperationId = id, SessionId = session, Generation = generation, Action = action ?? "invalid" };
        } catch { return null; }
    }
    internal static Windows.WindowsCommandResult Result(ArchiveReadReceipt receipt) =>
        new(receipt.Accepted, receipt.Code, Data: JsonSerializer.Serialize(receipt, JsonDefaults.Options));
}
