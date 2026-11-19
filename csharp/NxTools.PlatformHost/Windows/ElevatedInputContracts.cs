namespace NxTools.PlatformHost.Windows;

internal sealed record ElevatedInputConfiguration(
    long DestinationWindow,
    QuickLaunchKeyboardConfiguration Keyboard,
    Guid CorrelationId,
    uint DesktopProcessId = 0);

internal sealed record ElevatedPasteInput(
    long TargetWindowHandle,
    long TargetFocusWindowHandle,
    Guid CorrelationId);

internal sealed record ElevatedWindowActivation(long WindowHandle, Guid CorrelationId);

internal interface IElevatedPasteInput
{
    // Null means no request was dispatched. Unknown outcomes must return failure, not null.
    WindowsCommandResult? TryPasteInput(ElevatedPasteInput input);
}

internal interface IElevatedInputChannel : IElevatedPasteInput
{
    void ConfigureInput(ElevatedInputConfiguration configuration);
    bool OwnsInput(int helperProcessId, long generation);
    void InputUnavailable(int helperProcessId, long generation);
    bool AcceptPointer(int helperProcessId, uint timestamp);
    WindowsCommandResult? TryActivateInput(ElevatedWindowActivation input, long foregroundWindow);
}

internal static class ElevatedInputMessages
{
    public const int Gesture = 0x8012;
    public const int Unavailable = 0x8013;
    public const int PointerPressed = 0x8018;
}
