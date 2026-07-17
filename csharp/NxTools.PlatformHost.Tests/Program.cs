using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Tests;

internal static class Program
{
    public static async Task<int> Main()
    {
        var tests = new (string Name, Func<Task> Run)[]
        {
            ("envelope uses protocol v1 wire names", EnvelopeUsesWireNames),
            ("bounded reader preserves adjacent frames", ReaderPreservesFrames),
            ("bounded reader rejects oversized frames", ReaderRejectsOversizedFrame)
        };

        foreach (var test in tests)
        {
            try
            {
                await test.Run();
                Console.WriteLine($"PASS {test.Name}");
            }
            catch (Exception error)
            {
                Console.Error.WriteLine($"FAIL {test.Name}: {error.Message}");
                return 1;
            }
        }

        return 0;
    }

    private static Task EnvelopeUsesWireNames()
    {
        var envelope = ProtocolEnvelope.Create("hello", new
        {
            ClientName = "platform_host",
            ClientVersion = "0.1.0",
            Capabilities = new[] { "foreground_context" }
        });
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"protocol_version\":1", StringComparison.Ordinal));
        Assert(json.Contains("\"type\":\"hello\"", StringComparison.Ordinal));
        Assert(json.Contains("\"client_name\":\"platform_host\"", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static async Task ReaderPreservesFrames()
    {
        await using var stream = new MemoryStream(Encoding.UTF8.GetBytes("first\nsecond\n"));
        var reader = new BoundedFrameReader(stream);
        Assert(await reader.ReadAsync(CancellationToken.None) == "first");
        Assert(await reader.ReadAsync(CancellationToken.None) == "second");
    }

    private static async Task ReaderRejectsOversizedFrame()
    {
        await using var stream = new MemoryStream(
            Enumerable.Repeat((byte)'x', ProtocolConstants.MaxFrameBytes + 1).ToArray());
        var reader = new BoundedFrameReader(stream);
        try
        {
            _ = await reader.ReadAsync(CancellationToken.None);
            throw new InvalidOperationException("oversized frame was accepted");
        }
        catch (InvalidDataException)
        {
        }
    }

    private static void Assert(bool condition)
    {
        if (!condition)
        {
            throw new InvalidOperationException("assertion failed");
        }
    }
}
