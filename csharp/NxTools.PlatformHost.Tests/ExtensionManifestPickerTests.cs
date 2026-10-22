using System.ComponentModel;
using System.Text.Json;
using NxTools.PlatformHost.Protocol;
using NxTools.PlatformHost.Windows;

namespace NxTools.PlatformHost.Tests;

// All picker/file-system callbacks are injected; no native dialog, user files or adapter hooks.
internal static class ExtensionManifestPickerTests
{
    public static Task ParserPreservesTypedWireAndRejectsInvalidPayloads()
    {
        var envelope = Request(new { CurrentPath = @"C:\Synthetic\扩展.json" });
        var wire = JsonSerializer.Serialize(envelope, JsonDefaults.Options);
        var decoded = JsonSerializer.Deserialize<ProtocolEnvelope>(wire, JsonDefaults.Options)!;
        Assert(PlatformCommandParser.ParsePickExtensionManifest(decoded).CurrentPath == @"C:\Synthetic\扩展.json",
            "typed snake-case picker payload must survive serialization");
        Assert(decoded.Payload.GetProperty("command").GetString() == "pick_extension_manifest" &&
               decoded.Payload.GetProperty("data").GetProperty("current_path").ValueKind == JsonValueKind.String,
            "picker uses its own command and current_path wire field");
        var wrong = ProtocolEnvelope.Create("platform_command", new { Command = "pick_program", Data = new { CurrentPath = "" } });
        Reject(() => PlatformCommandParser.ParsePickExtensionManifest(wrong));
        Reject(() => PlatformCommandParser.ParsePickExtensionManifest(Request(new { })));
        Reject(() => PlatformCommandParser.ParsePickExtensionManifest(Request(new { CurrentPath = (string?)null })));
        Reject(() => PlatformCommandParser.ParsePickExtensionManifest(Request(new { CurrentPath = 42 })));
        Assert(PlatformCommandParser.ParsePickExtensionManifest(Request(new { CurrentPath = "" })).CurrentPath == "",
            "empty initial path is valid");
        return Task.CompletedTask;
    }

    public static Task SettingsRequireOneExistingZipAndPreserveInitialPath()
    {
        const string manifest = @"C:\Synthetic\cleaner.zip";
        const string directory = @"C:\Synthetic";
        var calls = 0;
        var result = ExtensionManifestPicker.PickCore(new("  " + manifest + "  "), CancellationToken.None,
            path => path == manifest, path => path == directory, settings =>
            {
                calls++;
                Assert(settings.Title == "导入扩展包" && settings.Filter == "ZIP 扩展包 (*.zip)|*.zip",
                    "ordinary import selects only ZIP packages");
                Assert(settings.CheckFileExists && settings.CheckPathExists && !settings.Multiselect &&
                       settings.RestoreDirectory && settings.AutoUpgradeEnabled,
                    "native picker must require one existing file and restore working directory");
                Assert(settings.InitialDirectory == directory && settings.FileName == "cleaner.zip",
                    "existing ZIP initializes directory and filename");
                return manifest;
            });
        Assert(result.Success && result.Data == manifest && calls == 1, "selection returns only the chosen path");
        result = ExtensionManifestPicker.PickCore(new(directory), CancellationToken.None,
            _ => false, path => path == directory, settings =>
            {
                Assert(settings.InitialDirectory == directory && settings.FileName == "", "directory seed has no filename");
                return null;
            });
        Assert(result.Success && result.Data == "", "dialog dismissal returns successful empty path");
        result = ExtensionManifestPicker.PickCore(new(@"C:\Missing\runner.exe"), CancellationToken.None,
            _ => false, _ => false, settings =>
            {
                Assert(settings.InitialDirectory == "" && settings.FileName == "", "missing or non-ZIP seed does not preselect a program");
                return "";
            });
        Assert(result.Success && result.Data == "", "empty dialog selection is cancellation, not import");
        return Task.CompletedTask;
    }

    public static Task CancellationRejectsLateSelectionWithoutNativeWork()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();
        var result = ExtensionManifestPicker.Pick(new(""), cancellation.Token);
        Assert(!result.Success && result.Data is null, "pre-cancelled production entry must never enter native runner");
        result = ExtensionManifestPicker.PickCore(new(""), cancellation.Token,
            _ => throw new InvalidOperationException("file probe must not run"),
            _ => throw new InvalidOperationException("directory probe must not run"),
            _ => throw new InvalidOperationException("dialog must not run"));
        Assert(!result.Success && result.Data is null, "pre-cancel stops every callback");
        using var lateCancellation = new CancellationTokenSource();
        result = ExtensionManifestPicker.PickCore(new(""), lateCancellation.Token, _ => false, _ => false,
            _ => { lateCancellation.Cancel(); return @"C:\Synthetic\late.zip"; });
        Assert(!result.Success && result.Data is null, "session cancellation drops a late selected path");
        return Task.CompletedTask;
    }

    public static Task InvalidSelectionAndNativeFailuresArePayloadSafe()
    {
        foreach (var selection in new[] { @"C:\Synthetic\runner.exe", @"C:\Synthetic\missing.zip", @"C:\Synthetic\extension.json" })
        {
            var result = ExtensionManifestPicker.PickCore(new(""), CancellationToken.None,
                path => path.EndsWith(".exe", StringComparison.Ordinal), _ => false, _ => selection);
            Assert(!result.Success && result.Data is null && !result.Error!.Contains(selection),
                "non-ZIP or missing selection is rejected without exposing paths");
        }
        var failed = ExtensionManifestPicker.PickCore(new(""), CancellationToken.None, _ => false, _ => false,
            _ => throw new Win32Exception(5, "private path or payload"));
        Assert(!failed.Success && failed.NativeErrorCode == 5 && !failed.Error!.Contains("private"),
            "native failure preserves code, not raw exception payload");
        return Task.CompletedTask;
    }

    public static async Task DetachedPickerDoesNotBlockLaterCommands()
    {
        var dispatcher = new PlatformCommandSequenceDispatcher();
        var started = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        using var cancellation = new CancellationTokenSource();
        var laterRan = false;
        var processing = Task.Run(async () =>
        {
            await dispatcher.DispatchAsync("pick_extension_manifest", token =>
            {
                Assert(token == cancellation.Token, "detached picker retains session cancellation");
                started.TrySetResult();
                release.Task.GetAwaiter().GetResult();
                return Task.CompletedTask;
            }, cancellation.Token);
            await dispatcher.DispatchAsync("set_window_visibility", () =>
            {
                laterRan = true;
                return Task.CompletedTask;
            });
        });
        try
        {
            await started.Task.WaitAsync(TimeSpan.FromSeconds(2));
            await processing.WaitAsync(TimeSpan.FromSeconds(2));
            Assert(laterRan && !release.Task.IsCompleted, "blocking picker must not stall the serial reader");
        }
        finally
        {
            release.TrySetResult();
            await processing.WaitAsync(TimeSpan.FromSeconds(2));
            await dispatcher.DrainAsync().WaitAsync(TimeSpan.FromSeconds(2));
        }
    }

    private static ProtocolEnvelope Request<T>(T data) => ProtocolEnvelope.Create("platform_command",
        new { Command = "pick_extension_manifest", Data = data });

    private static void Reject(Action parse)
    {
        try { parse(); }
        catch (InvalidDataException) { return; }
        throw new InvalidOperationException("invalid picker payload must be rejected");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
