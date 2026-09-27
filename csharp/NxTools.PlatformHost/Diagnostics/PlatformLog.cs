using System.Text.Json;
using System.Text.Json.Nodes;
using System.Threading.Channels;

namespace NxTools.PlatformHost.Diagnostics;

internal interface IPlatformLog
{
    void Write(
        string level,
        string eventName,
        Guid? correlationId = null,
        IReadOnlyDictionary<string, object?>? fields = null);
}

internal sealed class PlatformLog : IPlatformLog, IDisposable
{
    private const int RetentionDays = 14;
    private static readonly HashSet<string> SensitiveFieldNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "text", "content", "clipboard_text", "payload", "message", "path", "file_path",
        "data_path", "image_path", "file_paths", "window_title", "source_window_title",
        "url", "cookie", "headers"
    };

    private readonly string _logDirectory;
    private readonly Channel<string> _entries;
    private readonly Task _writerTask;
    private int _disposed;
    private long _droppedEntries;

    private PlatformLog(string logDirectory)
    {
        _logDirectory = logDirectory;
        Directory.CreateDirectory(logDirectory);
        DeleteExpiredLogs();
        _entries = Channel.CreateBounded<string>(new BoundedChannelOptions(4096)
        {
            FullMode = BoundedChannelFullMode.Wait,
            SingleReader = true,
            SingleWriter = false
        });
        _writerTask = Task.Run(WriteLoopAsync);
    }

    public string CurrentLogPath => Path.Combine(
        _logDirectory,
        $"platform-host.log.{DateOnly.FromDateTime(DateTime.Now):yyyy-MM-dd}.jsonl");

    public static PlatformLog CreateDefault()
    {
        var localData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        return new PlatformLog(Path.Combine(localData, "NxTools", "NxTools.Next", "data", "logs"));
    }

    internal static PlatformLog CreateForDirectory(string directory) => new(directory);

    public void Write(
        string level,
        string eventName,
        Guid? correlationId = null,
        IReadOnlyDictionary<string, object?>? fields = null)
    {
        try
        {
            WriteCore(level, eventName, correlationId, fields);
        }
        catch
        {
            Interlocked.Increment(ref _droppedEntries);
        }
    }

    private void WriteCore(
        string level,
        string eventName,
        Guid? correlationId,
        IReadOnlyDictionary<string, object?>? fields)
    {
        if (Volatile.Read(ref _disposed) != 0)
        {
            return;
        }
        var entry = new Dictionary<string, object?>
        {
            ["timestamp"] = DateTimeOffset.Now.ToString("O"),
            ["level"] = level,
            ["process"] = "platform_host",
            ["platform_host_pid"] = Environment.ProcessId,
            ["thread_id"] = Environment.CurrentManagedThreadId,
            ["event"] = eventName,
            ["correlation_id"] = correlationId
        };
        if (fields is not null)
        {
            foreach (var field in fields)
            {
                entry[field.Key] = SensitiveFieldNames.Contains(field.Key)
                    ? "[REDACTED]"
                    : field.Value;
            }
        }
        var node = JsonSerializer.SerializeToNode(entry)
            ?? throw new JsonException("Platform log entry serialized to null.");
        RedactSensitiveFields(node);
        if (!_entries.Writer.TryWrite(node.ToJsonString()))
        {
            Interlocked.Increment(ref _droppedEntries);
        }
    }

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0)
        {
            return;
        }
        var dropped = Interlocked.Read(ref _droppedEntries);
        if (dropped > 0)
        {
            var warning = JsonSerializer.Serialize(new Dictionary<string, object?>
            {
                ["timestamp"] = DateTimeOffset.Now.ToString("O"),
                ["level"] = "warning",
                ["process"] = "platform_host",
                ["platform_host_pid"] = Environment.ProcessId,
                ["thread_id"] = Environment.CurrentManagedThreadId,
                ["event"] = "log.entries_dropped",
                ["correlation_id"] = null,
                ["count"] = dropped
            });
            _entries.Writer.WriteAsync(warning).AsTask().GetAwaiter().GetResult();
        }
        _entries.Writer.TryComplete();
        _writerTask.GetAwaiter().GetResult();
    }

    private async Task WriteLoopAsync()
    {
        StreamWriter? writer = null;
        DateOnly writerDate = default;
        try
        {
            await foreach (var entry in _entries.Reader.ReadAllAsync())
            {
                try
                {
                    var today = DateOnly.FromDateTime(DateTime.Now);
                    if (writer is null || writerDate != today)
                    {
                        writer?.Dispose();
                        writerDate = today;
                        writer = new StreamWriter(
                            new FileStream(
                                CurrentLogPath,
                                FileMode.Append,
                                FileAccess.Write,
                                FileShare.ReadWrite),
                            new System.Text.UTF8Encoding(false))
                        {
                            AutoFlush = true
                        };
                    }
                    await writer.WriteLineAsync(entry);
                }
                catch (Exception error) when (error is IOException or UnauthorizedAccessException)
                {
                    Console.Error.WriteLine($"PlatformHost diagnostic log failed: {error.Message}");
                }
            }
        }
        finally
        {
            if (writer is not null)
            {
                await writer.DisposeAsync();
            }
        }
    }

    private void DeleteExpiredLogs()
    {
        var cutoff = DateTime.UtcNow.AddDays(-RetentionDays);
        foreach (var path in Directory.EnumerateFiles(_logDirectory, "platform-host.log.*.jsonl"))
        {
            try
            {
                if (File.GetLastWriteTimeUtc(path) < cutoff)
                {
                    File.Delete(path);
                }
            }
            catch (IOException)
            {
            }
            catch (UnauthorizedAccessException)
            {
            }
        }
    }

    private static void RedactSensitiveFields(JsonNode node)
    {
        if (node is JsonObject jsonObject)
        {
            foreach (var property in jsonObject.ToList())
            {
                if (SensitiveFieldNames.Contains(property.Key))
                {
                    jsonObject[property.Key] = "[REDACTED]";
                }
                else if (property.Value is not null)
                {
                    RedactSensitiveFields(property.Value);
                }
            }
        }
        else if (node is JsonArray jsonArray)
        {
            foreach (var item in jsonArray)
            {
                if (item is not null)
                {
                    RedactSensitiveFields(item);
                }
            }
        }
    }
}

internal sealed class NullPlatformLog : IPlatformLog
{
    public static NullPlatformLog Instance { get; } = new();

    private NullPlatformLog()
    {
    }

    public void Write(
        string level,
        string eventName,
        Guid? correlationId = null,
        IReadOnlyDictionary<string, object?>? fields = null)
    {
    }
}
