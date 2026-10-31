using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Encodings.Web;

namespace NxTools.PlatformHost.Windows.Extensions;

internal sealed record ExtensionStartRequest
{
    public int Version { get; init; } = 1;
    public string Op { get; init; } = "start";
    public Guid RunId { get; init; }
    public string PluginId { get; init; } = "";
    public string Executable { get; init; } = "";
    public string[] Arguments { get; init; } = [];
    public string WorkingDirectory { get; init; } = "";
    public int TimeoutMs { get; init; } = 30000;
    public long MemoryBytes { get; init; } = 512L * 1024 * 1024;
    public int MaxProcesses { get; init; } = 8;
    public string Stdin { get; init; } = "";
    // Host-owned opt-in; the child never chooses its native outer budget.
    public int? DataPlane { get; init; }
}

internal sealed record ExtensionProcessEvent(
    string Op, Guid RunId, bool Success = true, string? Reason = null,
    int? NativeErrorCode = null, uint? ExitCode = null, string? Data = null,
    long ElapsedMs = 0, uint? ProcessId = null, int Version = 1, bool CleanupComplete = true);

internal sealed record ExtensionControlRequest(int Version, string Op, Guid RunId, string? Data = null);

internal sealed class ExtensionFailure(string reason, int? nativeCode = null) : Exception(reason)
{
    internal string Reason { get; } = reason;
    internal int? NativeCode { get; } = nativeCode;
}

internal static class ExtensionProcessProtocol
{
    internal const int FrameLimit = 256 * 1024;
    internal const int InputLimit = 1024 * 1024;
    internal const int StdoutLimit = 4 * 1024 * 1024;
    internal const int StderrLimit = 64 * 1024;
    internal const int DataWireLimit = 24 * 1024 * 1024;
    internal static int InputBudget(ExtensionStartRequest request) => InputLimit + (request.DataPlane == 2 ? DataWireLimit : 0);
    internal static int StdoutBudget(ExtensionStartRequest request) => StdoutLimit + (request.DataPlane == 2 ? DataWireLimit : 0);
    internal static readonly UTF8Encoding Utf8 = new(false, true);
    internal static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
        MaxDepth = 32
    };

    internal static (ExtensionStartRequest? Start, ExtensionControlRequest? Control, CredentialRequest? Credential) Parse(string frame)
    {
        try
        {
            if (Utf8.GetByteCount(frame) > FrameLimit) throw new ExtensionFailure("control_frame_limit");
            using var doc = JsonDocument.Parse(frame, new JsonDocumentOptions { MaxDepth = 32 });
            CheckKeys(doc.RootElement);
            if (doc.RootElement.GetProperty("version").GetInt32() != 1) throw new ExtensionFailure("unsupported_version");
            var op = doc.RootElement.GetProperty("op").GetString();
            if (op == "start")
            {
                var start = JsonSerializer.Deserialize<ExtensionStartRequest>(frame, Json)!;
                Validate(start);
                return (start, null, null);
            }
            if (op is "credential_get" or "credential_set" or "credential_delete")
            {
                var credential = JsonSerializer.Deserialize<CredentialRequest>(frame, Json)!;
                CredentialAdapter.Validate(credential);
                return (null, null, credential);
            }
            var control = JsonSerializer.Deserialize<ExtensionControlRequest>(frame, Json)!;
            if (control.RunId == Guid.Empty || op is not ("cancel" or "write") ||
                (op == "write" && control.Data is null) || (op == "cancel" && control.Data is not null))
                throw new ExtensionFailure("invalid_control");
            if (control.Data is not null) InputFrame(control.Data);
            return (null, control, null);
        }
        catch (ExtensionFailure) { throw; }
        catch (Exception e) when (e is JsonException or InvalidOperationException or KeyNotFoundException or EncoderFallbackException)
        { throw new ExtensionFailure("invalid_control"); }
    }

    private static void CheckKeys(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            var keys = new HashSet<string>(StringComparer.Ordinal);
            foreach (var property in element.EnumerateObject())
            {
                if (!keys.Add(property.Name)) throw new ExtensionFailure("duplicate_control_key");
                CheckKeys(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
            foreach (var item in element.EnumerateArray()) CheckKeys(item);
    }

    internal static void Validate(ExtensionStartRequest request)
    {
        if (request.Version != 1 || request.Op != "start" || request.RunId == Guid.Empty ||
            string.IsNullOrWhiteSpace(request.PluginId) || request.PluginId.Length > 128 ||
            request.PluginId.Any(char.IsControl) || request.TimeoutMs is < 1 or > 120000 ||
            request.MemoryBytes is < 1 or > 1073741824 || request.MaxProcesses is < 1 or > 8 ||
            !LocalFullPath(request.Executable) || !LocalFullPath(request.WorkingDirectory) ||
            request.Arguments is null || request.Arguments.Length > 128 ||
            request.Arguments.Any(arg => arg is null || arg.Contains('\0')) || request.Stdin is null ||
            request.DataPlane is not (null or 2))
            throw new ExtensionFailure("invalid_start");
        var line = ExtensionNative.CommandLine(request.Executable, request.Arguments);
        if (line.Length >= 32767) throw new ExtensionFailure("arguments_limit");
        InputFrame(request.Stdin);
    }

    private static bool LocalFullPath(string? path) => !string.IsNullOrEmpty(path) &&
        path.Length < 32767 && path.Length >= 3 && char.IsAsciiLetter(path[0]) && path[1] == ':' &&
        (path[2] == '\\' || path[2] == '/') && !path.Contains('\0') && Path.IsPathFullyQualified(path);

    // Each write is exactly one protocol line. Count the trailing newline against input budget.
    internal static byte[] InputFrame(string data)
    {
        if (data.Contains('\n') || data.Contains('\r')) throw new ExtensionFailure("invalid_input_frame");
        try
        {
            var bytes = Utf8.GetBytes(data + "\n");
            if (bytes.Length > FrameLimit) throw new ExtensionFailure("input_frame_limit");
            return bytes;
        }
        catch (EncoderFallbackException) { throw new ExtensionFailure("invalid_input_utf8"); }
    }
}

// One fixed byte buffer per pipe. EOF without LF is a protocol failure, never silent truncation.
internal sealed class ExtensionLineReader(Stream stream, int totalLimit, string prefix, Action<int>? observeBytes = null)
{
    private readonly byte[] chunk = new byte[Math.Min(8192, totalLimit)];
    private readonly byte[] line = new byte[Math.Min(ExtensionProcessProtocol.FrameLimit, totalLimit)];
    private int begin, end, length, total;

    internal async ValueTask<string?> ReadAsync(CancellationToken cancellation = default)
    {
        while (true)
        {
            if (begin == end)
            {
                end = await stream.ReadAsync(chunk, cancellation);
                begin = 0;
                if (end == 0)
                {
                    if (length != 0) throw new ExtensionFailure(prefix + "_truncated_line");
                    return null;
                }
                if (end > totalLimit - total) throw new ExtensionFailure(prefix + "_total_limit");
                observeBytes?.Invoke(end);
                total += end;
            }
            var value = chunk[begin++];
            if (value == 10)
            {
                var count = length > 0 && line[length - 1] == 13 ? length - 1 : length;
                length = 0;
                try { return ExtensionProcessProtocol.Utf8.GetString(line, 0, count); }
                catch (DecoderFallbackException) { throw new ExtensionFailure(prefix + "_invalid_utf8"); }
            }
            if (length == line.Length) throw new ExtensionFailure(prefix + "_line_limit");
            line[length++] = value;
        }
    }
}
