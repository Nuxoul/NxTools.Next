using System.ComponentModel;
using NxTools.PlatformHost.Diagnostics;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record PasteWindowSnapshot(
    long ForegroundWindow,
    long ForegroundRootWindow,
    long TargetRootWindow,
    uint CurrentThreadId,
    uint TargetThreadId,
    uint ForegroundThreadId,
    uint TargetProcessId,
    uint ForegroundProcessId);

internal sealed record ModifierReleaseResult(
    bool Success,
    IReadOnlyList<ushort> InitiallyPressed,
    IReadOnlyList<ushort> FinallyPressed,
    int Attempts,
    IReadOnlyList<KeyboardInputStep> InputSteps);

internal sealed record ClipboardWriteAttempt(
    int Attempt,
    bool Success,
    int HResult,
    string? Error);

internal sealed record ClipboardWriteResult(
    bool Success,
    string? Error,
    IReadOnlyList<ClipboardWriteAttempt> Attempts);

internal sealed record PostPasteResult(bool Posted, int Win32Error, string? Error);

internal sealed record KeyboardInputStep(
    string Name,
    ushort VirtualKey,
    uint Flags,
    uint Sent,
    int Win32Error);

internal sealed record PasteShortcutResult(
    uint Sent,
    int InputStructSize,
    IReadOnlyList<KeyboardInputStep> Steps);

internal abstract record ClipboardPastePayload(
    long TargetWindowHandle,
    long TargetFocusWindowHandle)
{
    public abstract string Kind { get; }
    public virtual int TextLength => 0;
    public virtual int PathCount => 0;
}

internal sealed record TextClipboardPastePayload(
    string Text,
    long TargetWindowHandle,
    long TargetFocusWindowHandle) : ClipboardPastePayload(TargetWindowHandle, TargetFocusWindowHandle)
{
    public override string Kind => "text";
    public override int TextLength => Text.Length;
}

internal sealed record FilesClipboardPastePayload(
    IReadOnlyList<string> Paths,
    long TargetWindowHandle,
    long TargetFocusWindowHandle) : ClipboardPastePayload(TargetWindowHandle, TargetFocusWindowHandle)
{
    public override string Kind => "files";
    public override int PathCount => Paths.Count;
}

internal sealed record ImageClipboardPastePayload(
    string ImagePath,
    long TargetWindowHandle,
    long TargetFocusWindowHandle) : ClipboardPastePayload(TargetWindowHandle, TargetFocusWindowHandle)
{
    public override string Kind => "image";
}

internal interface IWindowsPasteOperations
{
    bool IsWindow(long windowHandle);
    uint GetClipboardSequenceNumber();
    ClipboardWriteResult WriteClipboard(ClipboardPastePayload payload);
    PasteWindowSnapshot CaptureWindowState(long targetWindowHandle);
    bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle);
    bool IsForegroundWindowForTarget(long targetWindowHandle);
    ModifierReleaseResult ReleaseActivationModifiers();
    PasteShortcutResult SendPasteShortcut();
    PostPasteResult PostPaste(long targetWindowHandle);
    void Delay(int milliseconds);
}

internal sealed class WindowsPasteCoordinator(
    IWindowsPasteOperations operations,
    IPlatformLog? log = null)
{
    private const int ActivationAttempts = 4;
    private const uint PasteInputCount = 4;
    private readonly IPlatformLog _log = log ?? NullPlatformLog.Instance;

    public WindowsCommandResult Execute(PasteTextCommand command, Guid correlationId = default)
        => Execute(new TextClipboardPastePayload(
            command.Text, command.TargetWindowHandle,
            command.TargetFocusWindowHandle), correlationId);

    public WindowsCommandResult Execute(PasteFilesCommand command, Guid correlationId = default)
        => Execute(new FilesClipboardPastePayload(
            command.Paths, command.TargetWindowHandle,
            command.TargetFocusWindowHandle), correlationId);

    public WindowsCommandResult Execute(PasteImageCommand command, Guid correlationId = default)
        => Execute(new ImageClipboardPastePayload(
            command.ImagePath, command.TargetWindowHandle,
            command.TargetFocusWindowHandle), correlationId);

    internal WindowsCommandResult Execute(ClipboardPastePayload command, Guid correlationId)
    {
        Log("info", "paste.start", correlationId, new Dictionary<string, object?>()
        {
            ["target_window"] = Hex(command.TargetWindowHandle),
            ["target_focus_window"] = Hex(command.TargetFocusWindowHandle),
            ["content_kind"] = command.Kind,
            ["text_length"] = command.TextLength,
            ["path_count"] = command.PathCount,
            ["input_struct_size"] = WindowsPlatformAdapter.NativeInputSize
        });
        if (command.TargetWindowHandle == 0 || !operations.IsWindow(command.TargetWindowHandle))
        {
            return Fail(correlationId, "paste.target_invalid", "The target window no longer exists.");
        }

        var clipboardBefore = operations.GetClipboardSequenceNumber();
        var clipboardWrite = operations.WriteClipboard(command);
        foreach (var attempt in clipboardWrite.Attempts)
        {
            Log(attempt.Success ? "info" : "warning", "paste.clipboard_write_attempt",
                correlationId, new Dictionary<string, object?>
                {
                    ["attempt"] = attempt.Attempt,
                    ["success"] = attempt.Success,
                    ["hresult"] = $"0x{attempt.HResult:X8}",
                    ["error"] = attempt.Error
                });
        }
        if (!clipboardWrite.Success)
        {
            Log("error", "paste.clipboard_write", correlationId, new Dictionary<string, object?>()
            {
                ["success"] = false,
                ["sequence_before"] = clipboardBefore,
                ["sequence_after"] = operations.GetClipboardSequenceNumber(),
                ["error"] = clipboardWrite.Error
            });
            return Fail(
                correlationId,
                "paste.failed",
                clipboardWrite.Error ?? "Could not write the Windows clipboard.");
        }
        Log("info", "paste.clipboard_write", correlationId, new Dictionary<string, object?>()
        {
            ["success"] = true,
            ["sequence_before"] = clipboardBefore,
            ["sequence_after"] = operations.GetClipboardSequenceNumber()
        });

        var targetReady = false;
        for (var attempt = 0; attempt < ActivationAttempts; attempt++)
        {
            var before = operations.CaptureWindowState(command.TargetWindowHandle);
            var activated = operations.TryActivateWindow(
                command.TargetWindowHandle,
                command.TargetFocusWindowHandle);
            var after = operations.CaptureWindowState(command.TargetWindowHandle);
            targetReady = activated &&
                operations.IsForegroundWindowForTarget(command.TargetWindowHandle);
            Log(targetReady ? "info" : "warning", "paste.activation_attempt", correlationId,
                new Dictionary<string, object?>()
            {
                ["attempt"] = attempt + 1,
                ["activated"] = activated,
                ["target_ready"] = targetReady,
                ["before"] = before,
                ["after"] = after
            });
            if (targetReady)
            {
                break;
            }
            operations.Delay(25 + attempt * 20);
        }

        var pasteTarget = command.TargetFocusWindowHandle != 0 &&
            operations.IsWindow(command.TargetFocusWindowHandle)
                ? command.TargetFocusWindowHandle
                : command.TargetWindowHandle;
        if (!targetReady)
        {
            var post = operations.PostPaste(pasteTarget);
            Log("error", "paste.wm_paste_fallback", correlationId,
                new Dictionary<string, object?>()
            {
                ["reason"] = "target_activation_failed",
                ["target_window"] = Hex(pasteTarget),
                ["posted"] = post.Posted,
                ["win32_error"] = post.Win32Error,
                ["error"] = post.Error
            });
            return UnconfirmedFallback(
                correlationId,
                "Could not restore the target window",
                post.Posted,
                post.Error);
        }

        operations.Delay(40);
        var modifierResult = operations.ReleaseActivationModifiers();
        Log(modifierResult.Success ? "info" : "error", "paste.modifier_release", correlationId,
            new Dictionary<string, object?>
            {
                ["success"] = modifierResult.Success,
                ["initially_pressed"] = modifierResult.InitiallyPressed.Select(KeyName).ToArray(),
                ["finally_pressed"] = modifierResult.FinallyPressed.Select(KeyName).ToArray(),
                ["attempts"] = modifierResult.Attempts
            });
        foreach (var step in modifierResult.InputSteps)
        {
            Log(step.Sent == 1 ? "info" : "error", "paste.modifier_input_step", correlationId,
                InputStepFields(step, WindowsPlatformAdapter.NativeInputSize));
        }
        if (!modifierResult.Success)
        {
            var post = operations.PostPaste(pasteTarget);
            Log("error", "paste.wm_paste_fallback", correlationId,
                new Dictionary<string, object?>()
            {
                ["reason"] = "modifiers_remained_pressed",
                ["target_window"] = Hex(pasteTarget),
                ["posted"] = post.Posted,
                ["win32_error"] = post.Win32Error,
                ["error"] = post.Error
            });
            return UnconfirmedFallback(
                correlationId,
                "Activation modifiers remained pressed",
                post.Posted,
                post.Error);
        }

        var shortcut = operations.SendPasteShortcut();
        foreach (var step in shortcut.Steps)
        {
            Log(step.Sent == 1 ? "info" : "error", "paste.send_input_step", correlationId,
                InputStepFields(step, shortcut.InputStructSize));
        }
        if (shortcut.Sent == PasteInputCount)
        {
            Log("info", "paste.completed", correlationId, new Dictionary<string, object?>()
            {
                ["method"] = "send_input",
                ["sent"] = shortcut.Sent
            });
            return new WindowsCommandResult(true);
        }

        var inputPost = operations.PostPaste(pasteTarget);
        var firstInputError = shortcut.Steps.FirstOrDefault(step => step.Sent == 0)?.Win32Error ?? 0;
        Log("error", "paste.wm_paste_fallback", correlationId,
            new Dictionary<string, object?>()
        {
            ["reason"] = "send_input_incomplete",
            ["target_window"] = Hex(pasteTarget),
            ["posted"] = inputPost.Posted,
            ["win32_error"] = inputPost.Win32Error,
            ["error"] = inputPost.Error,
            ["sent"] = shortcut.Sent,
            ["expected"] = PasteInputCount
        });
        return UnconfirmedFallback(
            correlationId,
            $"SendInput accepted {shortcut.Sent}/{PasteInputCount} events (Win32={firstInputError})",
            inputPost.Posted,
            inputPost.Error);
    }

    private WindowsCommandResult UnconfirmedFallback(
        Guid correlationId,
        string reason,
        bool posted,
        string? postError) =>
        Fail(
            correlationId,
            "paste.failed",
            posted
                ? $"{reason}; WM_PASTE was dispatched but the paste could not be confirmed."
                : $"{reason}; WM_PASTE failed: {postError}");

    private WindowsCommandResult Fail(Guid correlationId, string eventName, string error)
    {
        Log("error", eventName, correlationId,
            new Dictionary<string, object?> { ["error"] = error });
        return new WindowsCommandResult(false, error);
    }

    private void Log(
        string level,
        string eventName,
        Guid correlationId,
        IReadOnlyDictionary<string, object?> fields) =>
        _log.Write(level, eventName, correlationId == default ? null : correlationId, fields);

    private static string Hex(long windowHandle) => $"0x{windowHandle:X}";

    private static string KeyName(ushort key) => $"VK_0x{key:X2}";

    private static IReadOnlyDictionary<string, object?> InputStepFields(
        KeyboardInputStep step,
        int inputStructSize) =>
        new Dictionary<string, object?>
        {
            ["step"] = step.Name,
            ["virtual_key"] = $"0x{step.VirtualKey:X2}",
            ["flags"] = $"0x{step.Flags:X8}",
            ["sent"] = step.Sent,
            ["win32_error"] = step.Win32Error,
            ["win32_message"] = step.Win32Error == 0
                ? null
                : new Win32Exception(step.Win32Error).Message,
            ["input_struct_size"] = inputStructSize
        };
}
