using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal interface IWindowsPasteOperations
{
    bool IsWindow(long windowHandle);
    bool TrySetClipboardText(string text, out string? error);
    bool TryActivateWindow(long targetWindowHandle, long focusWindowHandle);
    bool IsForegroundWindowForTarget(long targetWindowHandle);
    bool TryReleaseActivationModifiers();
    uint SendPasteShortcut();
    bool TryPostPaste(long targetWindowHandle, out string? error);
    void Delay(int milliseconds);
}

internal sealed class WindowsPasteCoordinator(IWindowsPasteOperations operations)
{
    private const int ActivationAttempts = 4;
    private const uint PasteInputCount = 4;

    public WindowsCommandResult Execute(PasteTextCommand command)
    {
        if (command.TargetWindowHandle == 0 || !operations.IsWindow(command.TargetWindowHandle))
        {
            return new WindowsCommandResult(false, "The target window no longer exists.");
        }
        if (!operations.TrySetClipboardText(command.Text, out var clipboardError))
        {
            return new WindowsCommandResult(false, clipboardError ?? "Could not write the Windows clipboard.");
        }

        var targetReady = false;
        for (var attempt = 0; attempt < ActivationAttempts; attempt++)
        {
            var activated = operations.TryActivateWindow(
                command.TargetWindowHandle,
                command.TargetFocusWindowHandle);
            targetReady = activated &&
                operations.IsForegroundWindowForTarget(command.TargetWindowHandle);
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
            var posted = operations.TryPostPaste(pasteTarget, out var postError);
            return UnconfirmedFallback(
                "Could not restore the target window",
                posted,
                postError);
        }

        operations.Delay(40);
        if (!operations.TryReleaseActivationModifiers())
        {
            var posted = operations.TryPostPaste(pasteTarget, out var modifierPostError);
            return UnconfirmedFallback(
                "Activation modifiers remained pressed",
                posted,
                modifierPostError);
        }

        var sent = operations.SendPasteShortcut();
        if (sent == PasteInputCount)
        {
            return new WindowsCommandResult(true);
        }

        var inputPosted = operations.TryPostPaste(pasteTarget, out var inputPostError);
        return UnconfirmedFallback(
            $"SendInput accepted {sent}/{PasteInputCount} events",
            inputPosted,
            inputPostError);
    }

    private static WindowsCommandResult UnconfirmedFallback(
        string reason,
        bool posted,
        string? postError) =>
        new(
            false,
            posted
                ? $"{reason}; WM_PASTE was dispatched but the paste could not be confirmed."
                : $"{reason}; WM_PASTE failed: {postError}");
}
