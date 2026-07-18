using System.Text;
using System.Text.Json;
using NxTools.PlatformHost.Diagnostics;
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
            ("image and files preserve capture and paste payloads", MultiFormatPayloadsRoundTrip),
            ("native clipboard format command preserves staged payload", NativeFormatPayloadRoundTrips),
            ("topmost command preserves native window", TopmostCommandPreservesNativeWindow),
            ("visibility command preserves native window", VisibilityCommandPreservesNativeWindow),
            ("paste retries transient target activation", PasteRetriesTransientTargetActivation),
            ("copy-only writes clipboard without activating target", CopyOnlyWritesWithoutActivatingTarget),
            ("four clipboard actions reach the expected Windows outcome", FourActionsReachExpectedWindowsOutcome),
            ("native input layout matches Windows ABI", NativeInputLayoutMatchesWindowsAbi),
            ("paste diagnostics preserve correlation and Win32 error", PasteDiagnosticsPreserveCorrelationAndWin32Error),
            ("platform log writes structured JSON lines", PlatformLogWritesStructuredJsonLines),
            ("unverified fallback is not reported as paste success", UnverifiedFallbackIsNotReportedAsPasteSuccess),
            ("pressed modifiers keep paste result unconfirmed", PressedModifiersKeepPasteResultUnconfirmed),
            ("incomplete input keeps paste result unconfirmed", IncompleteInputKeepsPasteResultUnconfirmed),
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
        Assert(command.PasteAfterWrite);
        Assert(command.TargetWindowHandle == 42L);
        Assert(command.TargetFocusWindowHandle == 43L);
        return Task.CompletedTask;
    }

    private static Task ClipboardEventPreservesSourceContext()
    {
        var envelope = PlatformEventEnvelopeFactory.Create(
            new ClipboardCapturedEvent(
                "captured text",
                null,
                [],
                "C:/source.png",
                new ForegroundContext(42, 43, 7, "notepad", "notes.txt"),
                1_700_000_000_000));
        var json = JsonSerializer.Serialize(envelope, JsonDefaults.Options);

        Assert(json.Contains("\"event\":\"clipboard_captured\"", StringComparison.Ordinal));
        Assert(json.Contains("\"text\":\"captured text\"", StringComparison.Ordinal));
        Assert(json.Contains("\"window_handle\":42", StringComparison.Ordinal));
        Assert(json.Contains("\"process_name\":\"notepad\"", StringComparison.Ordinal));
        Assert(json.Contains("C:/source.png", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task MultiFormatPayloadsRoundTrip()
    {
        var filesEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_files",
            Data = new { Paths = new[] { "C:/one.txt", "C:/two.txt" }, TargetWindowHandle = 4L, TargetFocusWindowHandle = 5L }
        });
        var files = PlatformCommandParser.ParsePasteFiles(filesEnvelope);
        Assert(files.Paths.Count == 2 && files.TargetWindowHandle == 4);

        var imageEnvelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_image",
            Data = new { ImagePath = "C:/image.png", TargetWindowHandle = 6L, TargetFocusWindowHandle = 7L }
        });
        var image = PlatformCommandParser.ParsePasteImage(imageEnvelope);
        Assert(image.ImagePath == "C:/image.png" && image.TargetFocusWindowHandle == 7);

        var captured = PlatformEventEnvelopeFactory.Create(new ClipboardCapturedEvent(
            "alternate representation",
            new ClipboardImageData("C:/staging.png", 640, 480),
            ["C:/one.txt"],
            "C:/source.png",
            new ForegroundContext(1, 2, 3, "capture", "window"), 10));
        var json = JsonSerializer.Serialize(captured, JsonDefaults.Options);
        Assert(json.Contains("clipboard_captured", StringComparison.Ordinal));
        Assert(json.Contains("\"width\":640", StringComparison.Ordinal));
        Assert(json.Contains("C:/one.txt", StringComparison.Ordinal));
        return Task.CompletedTask;
    }

    private static Task NativeFormatPayloadRoundTrips()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "paste_native_formats",
            Data = new
            {
                Formats = new[]
                {
                    new
                    {
                        FormatId = 13U,
                        FormatName = "",
                        DataPath = "C:/formats/unicode.bin",
                        SizeBytes = 24UL,
                        SortOrder = 0U
                    }
                },
                PasteAfterWrite = false,
                TargetWindowHandle = 0L,
                TargetFocusWindowHandle = 0L
            }
        });

        var command = PlatformCommandParser.ParsePasteNativeFormats(envelope);
        Assert(command.Formats.Count == 1);
        Assert(command.Formats[0].FormatId == 13);
        Assert(command.Formats[0].DataPath.EndsWith("unicode.bin", StringComparison.Ordinal));
        Assert(!command.PasteAfterWrite);
        return Task.CompletedTask;
    }

    private static Task TopmostCommandPreservesNativeWindow()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_topmost",
            Data = new
            {
                WindowHandle = 777L,
                Enabled = true
            }
        });

        var command = PlatformCommandParser.ParseSetWindowTopmost(envelope);
        Assert(command.WindowHandle == 777L);
        Assert(command.Enabled);
        return Task.CompletedTask;
    }

    private static Task VisibilityCommandPreservesNativeWindow()
    {
        var envelope = ProtocolEnvelope.Create("platform_command", new
        {
            Command = "set_window_visibility",
            Data = new
            {
                WindowHandle = 778L,
                Visible = false
            }
        });

        var command = PlatformCommandParser.ParseSetWindowVisibility(envelope);
        Assert(command.WindowHandle == 778L);
        Assert(!command.Visible);
        return Task.CompletedTask;
    }

    private static Task PasteRetriesTransientTargetActivation()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([false, true])
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(result.Success);
        Assert(operations.ActivationAttempts == 2);
        Assert(operations.SendPasteCalls == 1);
        Assert(operations.PostPasteCalls == 0);
        return Task.CompletedTask;
    }

    private static Task CopyOnlyWritesWithoutActivatingTarget()
    {
        var operations = new FakePasteOperations();
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("copy", 0, 0, false));

        Assert(result.Success);
        Assert(operations.ActivationAttempts == 0);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 0);
        return Task.CompletedTask;
    }

    private static Task FourActionsReachExpectedWindowsOutcome()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([true, true, true])
        };
        var coordinator = new WindowsPasteCoordinator(operations);
        var fixturePath = Path.Combine(
            FindRepositoryRoot(),
            "tests", "fixtures", "clipboard_action_commands.jsonl");
        foreach (var line in File.ReadLines(fixturePath))
        {
            var envelope = JsonSerializer.Deserialize<ProtocolEnvelope>(line, JsonDefaults.Options)
                ?? throw new InvalidDataException("Shared clipboard action fixture was empty.");
            var result = PlatformCommandParser.GetCommandName(envelope) switch
            {
                "paste_native_formats" => coordinator.Execute(
                    PlatformCommandParser.ParsePasteNativeFormats(envelope)),
                "paste_text" => coordinator.Execute(
                    PlatformCommandParser.ParsePasteText(envelope)),
                var command => throw new InvalidDataException(
                    $"Unexpected shared clipboard action fixture '{command}'.")
            };
            Assert(result.Success);
        }

        Assert(operations.WrittenPayloads.Count == 4);
        Assert(operations.WrittenPayloads[0] is NativeFormatsClipboardPastePayload
        {
            PasteAfterWrite: true
        });
        Assert(operations.WrittenPayloads[1] is NativeFormatsClipboardPastePayload
        {
            PasteAfterWrite: false
        });
        Assert(operations.WrittenPayloads[2] is TextClipboardPastePayload { Text: "first\r\nsecond" });
        Assert(operations.WrittenPayloads[3] is TextClipboardPastePayload { Text: "firstsecond" });
        Assert(operations.ActivationAttempts == 3);
        Assert(operations.SendPasteCalls == 3);
        return Task.CompletedTask;
    }

    private static string FindRepositoryRoot()
    {
        for (var directory = new DirectoryInfo(AppContext.BaseDirectory);
             directory is not null;
             directory = directory.Parent)
        {
            if (File.Exists(Path.Combine(directory.FullName, "Cargo.toml")))
            {
                return directory.FullName;
            }
        }
        throw new DirectoryNotFoundException("Could not locate the NxTools.Next repository root.");
    }

    private static Task NativeInputLayoutMatchesWindowsAbi()
    {
        var expectedSize = IntPtr.Size == 8 ? 40 : 28;
        Assert(WindowsPlatformAdapter.NativeInputSize == expectedSize);
        return Task.CompletedTask;
    }

    private static Task PasteDiagnosticsPreserveCorrelationAndWin32Error()
    {
        var correlationId = Guid.NewGuid();
        var log = new RecordingPlatformLog();
        var coordinator = new WindowsPasteCoordinator(
            new FakePasteOperations { SentInputCount = 0, PostPasteResult = true },
            log);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43), correlationId);

        Assert(!result.Success);
        var inputFailure = log.Entries.Single(entry =>
            entry.EventName == "paste.send_input_step" &&
            Equals(entry.Fields["step"], "ctrl_down"));
        Assert(inputFailure.CorrelationId == correlationId);
        Assert(Equals(inputFailure.Fields["win32_error"], 87));
        Assert(Equals(inputFailure.Fields["input_struct_size"], IntPtr.Size == 8 ? 40 : 28));
        return Task.CompletedTask;
    }

    private static Task PlatformLogWritesStructuredJsonLines()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"nxtools-log-test-{Guid.NewGuid():N}");
        var correlationId = Guid.NewGuid();
        string path;
        using (var log = PlatformLog.CreateForDirectory(directory))
        {
            path = log.CurrentLogPath;
            log.Write("info", "test.event", correlationId, new Dictionary<string, object?>
            {
                ["win32_error"] = 87,
                ["text"] = "must-not-appear",
                ["nested"] = new { Text = "nested-must-not-appear", Safe = 1 }
            });
        }
        var json = File.ReadAllText(path);
        using var document = JsonDocument.Parse(json);
        var root = document.RootElement;
        Assert(root.GetProperty("event").GetString() == "test.event");
        Assert(root.GetProperty("correlation_id").GetGuid() == correlationId);
        Assert(root.GetProperty("win32_error").GetInt32() == 87);
        Assert(root.GetProperty("text").GetString() == "[REDACTED]");
        Assert(root.GetProperty("nested").GetProperty("Text").GetString() == "[REDACTED]");
        Assert(!json.Contains("must-not-appear", StringComparison.Ordinal));
        Assert(!json.Contains("nested-must-not-appear", StringComparison.Ordinal));
        Directory.Delete(directory, recursive: true);
        return Task.CompletedTask;
    }

    private static Task UnverifiedFallbackIsNotReportedAsPasteSuccess()
    {
        var operations = new FakePasteOperations
        {
            ActivationResults = new Queue<bool>([false, false, false, false]),
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.ActivationAttempts == 4);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 1);
        Assert(operations.LastPostTarget == 43);
        return Task.CompletedTask;
    }

    private static Task PressedModifiersKeepPasteResultUnconfirmed()
    {
        var operations = new FakePasteOperations
        {
            ModifiersReleased = false,
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.SendPasteCalls == 0);
        Assert(operations.PostPasteCalls == 1);
        return Task.CompletedTask;
    }

    private static Task IncompleteInputKeepsPasteResultUnconfirmed()
    {
        var operations = new FakePasteOperations
        {
            SentInputCount = 2,
            PostPasteResult = true
        };
        var coordinator = new WindowsPasteCoordinator(operations);

        var result = coordinator.Execute(new PasteTextCommand("paste", 42, 43));

        Assert(!result.Success);
        Assert(result.Error?.Contains("could not be confirmed", StringComparison.OrdinalIgnoreCase) == true);
        Assert(operations.SendPasteCalls == 1);
        Assert(operations.PostPasteCalls == 1);
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
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.ClipboardNativeFormats));
        Assert(PlatformCapabilities.Create(false).Contains(PlatformCapabilities.WindowTopmost));
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

    private sealed class FakePasteOperations : IWindowsPasteOperations
    {
        public Queue<bool> ActivationResults { get; init; } = new([true]);
        public bool PostPasteResult { get; init; }
        public bool ModifiersReleased { get; init; } = true;
        public uint SentInputCount { get; init; } = 4;
        public int ActivationAttempts { get; private set; }
        public int SendPasteCalls { get; private set; }
        public int PostPasteCalls { get; private set; }
        public long LastPostTarget { get; private set; }
        public List<ClipboardPastePayload> WrittenPayloads { get; } = [];

        public bool IsWindow(long windowHandle) => windowHandle != 0;

        public uint GetClipboardSequenceNumber() => 7;

        public ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload)
        {
            WrittenPayloads.Add(payload);
            return new(true, null, [new ClipboardWriteAttempt(1, true, 0, null)]);
        }

        public bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle)
        {
            ActivationAttempts++;
            return ActivationResults.Count > 0 && ActivationResults.Dequeue();
        }

        public bool IsForegroundWindowForTarget(long targetWindowHandle) =>
            ActivationAttempts > 0 && ActivationResults.Count < 3;

        public PasteWindowSnapshot CaptureWindowState(long targetWindowHandle) =>
            new(targetWindowHandle, targetWindowHandle, targetWindowHandle, 1, 2, 2, 3, 3);

        public ModifierReleaseResult ReleaseActivationModifiers() =>
            new(ModifiersReleased, [], ModifiersReleased ? [] : [0x12], 1, []);

        public PasteShortcutResult SendPasteShortcut()
        {
            SendPasteCalls++;
            var steps = new[] { "ctrl_down", "v_down", "v_up", "ctrl_up" }
                .Select((name, index) => new KeyboardInputStep(
                    name,
                    index is 0 or 3 ? (ushort)0x11 : (ushort)0x56,
                    index >= 2 ? 2U : 0U,
                    index < SentInputCount ? 1U : 0U,
                    index < SentInputCount ? 0 : 87))
                .ToArray();
            return new PasteShortcutResult(SentInputCount, WindowsPlatformAdapter.NativeInputSize, steps);
        }

        public PostPasteResult PostPaste(long targetWindowHandle)
        {
            PostPasteCalls++;
            LastPostTarget = targetWindowHandle;
            return new PostPasteResult(
                PostPasteResult,
                PostPasteResult ? 0 : 5,
                PostPasteResult ? null : "post failed");
        }

        public void Delay(int milliseconds)
        {
        }
    }

    private sealed class RecordingPlatformLog : IPlatformLog
    {
        public List<RecordedLogEntry> Entries { get; } = [];

        public void Write(
            string level,
            string eventName,
            Guid? correlationId = null,
            IReadOnlyDictionary<string, object?>? fields = null) =>
            Entries.Add(new RecordedLogEntry(
                level,
                eventName,
                correlationId,
                fields is null
                    ? new Dictionary<string, object?>()
                    : new Dictionary<string, object?>(fields)));
    }

    private sealed record RecordedLogEntry(
        string Level,
        string EventName,
        Guid? CorrelationId,
        IReadOnlyDictionary<string, object?> Fields);
}
