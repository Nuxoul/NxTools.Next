using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

internal static class Program
{
    public static async Task<int> Main()
    {
        var tests = new (string Name, Func<Task> Run)[]
        {
            ("envelope uses protocol v1 wire names", EnvelopeUsesWireNames),
            ("paste command preserves text and target window", PasteCommandPreservesPayload),
            ("paste result correlates to command", PasteResultCorrelatesToCommand),
            ("clipboard event preserves source context", ClipboardEventPreservesSourceContext),
            ("panel request preserves target and cursor", PanelRequestPreservesTargetAndCursor),
            ("hotkey capability reflects registration", HotKeyCapabilityReflectsRegistration),
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

    private static Task PasteCommandPreservesPayload()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_text",
            Data = new
            {
                Text = "paste exactly this",
                TargetWindowHandle = 42L,
                TargetFocusWindowHandle = 43L
            }
        });

        var command = PlatformCommandParser.ParsePasteText(envelope);
        Assert(command.Text == "paste exactly this");
        Assert(command.TargetWindowHandle == 42L);
        Assert(command.TargetFocusWindowHandle == 43L);
        return Task.CompletedTask;
    }

    private static Task ClipboardEventPreservesSourceContext()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ClipboardTextCapturedEvent(
                "captured text",
                new ForegroundContext(42, 43, 7, "notepad", "notes.txt"),
                1_700_000_000_000));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"clipboard_text_captured\"", StringComparison.Ordinal));
        Assert(json.Contains("\"text\":\"captured text\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":42", StringComparison.Ordinal));
        Assert(json.Contains("\"process_name\":\"notepad\"", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task PasteResultCorrelatesToCommand()
    {
        var commandId = Guid.NewGuid();
        var envelope = ProtocolEnvelope.Create(
            "platform_command_result",
            new
            {
                Success = false,
                Error = "target closed"
            },
            commandId);
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(envelope.CorrelationId == commandId);
        Assert(json.Contains("\"type\":\"platform_command_result\"", StringComparison.Ordinal));
        Assert(json.Contains("\"success\":false", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task PanelRequestPreservesTargetAndCursor()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ClipboardPanelRequestedEvent(
                new ForegroundContext(88, 89, 9, "editor", "draft.md"),
                1200,
                700,
                1,
                144,
                1_700_000_000_500));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"clipboard_panel_requested\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":88", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_x\":1200", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_y\":700", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_display_index\":1", StringComparison.Ordinal));
        Assert(json.Contains("\"cursor_monitor_dpi\":144", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task HotKeyCapabilityReflectsRegistration()
    {
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.PasteText));
        Assert(!PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardPanelHotKey));
        Assert(PlatformCapabilities.Create(true).Contains(PlatformCapabilities.ClipboardPanelHotKey));
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
