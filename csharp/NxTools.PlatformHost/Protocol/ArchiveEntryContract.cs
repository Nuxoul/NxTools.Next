using System.Text.Json;
using System.Text.Json.Serialization;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Protocol;

[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveBrowserAttributes(bool ReadOnly, bool Hidden);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveBrowserSource(string Native, uint[] Digest, ulong Size, ulong ModifiedUnixNs);
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveBrowserContext(Guid SnapshotId, string Format, string Comment, ArchiveBrowserSource Source)
{
    internal bool Valid => SnapshotId != Guid.Empty && Comment is not null && System.Text.Encoding.UTF8.GetByteCount(Comment) <= 65536 &&
        Format is "Zip" or "SevenZ" or "Rar4" or "Rar5" or "Tar" or "TarGz" or "TarBz2" or "TarXz" or "TarZstd" or "Gzip" or "Bzip2" or "Xz" or "Zstd" &&
        Source is { Native: { Length: > 3 and <= 256 }, Digest: { Length: 32 } } && Source.Native.StartsWith("v1:", StringComparison.Ordinal) &&
        Source.Native.All(c => c < 128 && !char.IsControl(c)) && Source.Digest.All(b => b <= 255);
}
[JsonUnmappedMemberHandling(JsonUnmappedMemberHandling.Disallow)]
internal sealed record ArchiveEntryCommand
{
    public required uint Version { get; init; }
    public required Guid OperationId { get; init; }
    public required Guid SessionId { get; init; }
    public required ulong Generation { get; init; }
    public required string Action { get; init; }
    public string? Path { get; init; }
    public uint Codepage { get; init; }
    public ulong? Ordinal { get; init; }
    public ArchiveBrowserContext? Browser { get; init; }
    public Guid? LeaseId { get; init; }
    public uint Depth { get; init; }
    public bool? CopyMotw { get; init; }
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingDefault)]
    public bool PasswordInteraction { get; init; }
}
internal sealed record ArchiveEntryResult(uint Version, Guid OperationId, Guid SessionId, ulong Generation, bool Success,
    Guid? LeaseId, string? Path, string? Code, int? NativeErrorCode = null, bool CleanupComplete = true, bool Retained = false);
internal static class ArchiveEntryWire
{
    private static readonly JsonSerializerOptions Strict = new(JsonDefaults.Options) { PropertyNameCaseInsensitive = false, NumberHandling = JsonNumberHandling.Strict };
    private static void Unique(JsonElement value) {
        if (value.ValueKind == JsonValueKind.Object) {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (var p in value.EnumerateObject()) { if (!names.Add(p.Name)) throw new InvalidDataException(); Unique(p.Value); }
        } else if (value.ValueKind == JsonValueKind.Array) foreach (var item in value.EnumerateArray()) Unique(item);
    }
    internal static ArchiveEntryCommand Parse(ProtocolEnvelope envelope)
    {
        Unique(envelope.Payload);
        if (JsonSerializer.SerializeToUtf8Bytes(envelope.Payload).Length > 262144 || envelope.Payload.GetProperty("command").GetString() != "archive_entry" ||
            envelope.Payload.EnumerateObject().Any(p => p.Name is not ("command" or "data"))) throw new InvalidDataException();
        var raw = envelope.Payload.GetProperty("data");
        if (raw.TryGetProperty("browser", out var browser) && browser.ValueKind != JsonValueKind.Null) {
            Require(browser, "snapshot_id", "format", "comment", "source");
            Require(browser.GetProperty("source"), "native", "digest", "size", "modified_unix_ns");
        }
        var command = envelope.Payload.GetProperty("data").Deserialize<ArchiveEntryCommand>(Strict) ?? throw new InvalidDataException();
        if (command.Version != 1 || command.OperationId == Guid.Empty || command.SessionId == Guid.Empty || command.Generation == 0 || envelope.CorrelationId != command.OperationId || envelope.Id == command.OperationId || command.LeaseId == Guid.Empty) throw new InvalidDataException();
        if (command.Action == "materialize") {
            var path = command.Path;
            if (path is null || path.Length is < 4 or > 32767 || !char.IsAsciiLetter(path[0]) || path[1] != ':' || path[2] != '\\' || path.Contains('/') || path.Any(char.IsControl) ||
                command.Codepage is not (0 or 65001 or 936 or 950 or 932 or 949 or 437 or 1250 or 1251 or 1252) || command.Ordinal is null || command.Browser?.Valid != true || command.Depth > 3) throw new InvalidDataException();
        } else if (command.Action is not ("cancel" or "release") || command.PasswordInteraction || command.Path is not null || command.Browser is not null || command.Ordinal is not null || command.Codepage != 0 || command.Depth != 0 || command.CopyMotw is not null || (command.Action == "release") != command.LeaseId.HasValue) throw new InvalidDataException();
        return command;
    }
    private static void Require(JsonElement value, params string[] keys) {
        if (value.ValueKind != JsonValueKind.Object || keys.Any(key => !value.TryGetProperty(key, out _))) throw new InvalidDataException();
    }
    internal static WindowsCommandResult Result(ArchiveEntryResult result) => new(result.Success, result.Code, NativeErrorCode: result.NativeErrorCode, Data: JsonSerializer.Serialize(result, JsonDefaults.Options));
}
