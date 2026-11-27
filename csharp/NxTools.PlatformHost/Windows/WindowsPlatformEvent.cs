namespace NxTools.PlatformHost.Windows;

using NxTools.PlatformHost.Protocol;

internal abstract record WindowsPlatformEvent;
internal sealed record SystemMotionPreferenceChangedEvent(
    Guid CorrelationId, bool ReducedMotion, int NativeErrorCode) : WindowsPlatformEvent;
internal sealed record SystemThemePreferenceChangedEvent(
    Guid CorrelationId, bool Dark, int NativeErrorCode) : WindowsPlatformEvent;
internal sealed record SystemMaterialCapabilitiesChangedEvent(
    Guid CorrelationId, Guid SessionId, ulong Revision, bool Supported, bool TransparencyEnabled,
    bool HighContrast, bool EnergySaver, bool RemoteSession, int NativeErrorCode,
    bool SystemBackdrop, bool RoundedCorners) : WindowsPlatformEvent;

internal enum QuickLaunchKeyboardHealthState { InstallFailed, ThreadExited, Reinstalled }
internal sealed record QuickLaunchKeyboardHealthChangedEvent(
    Guid CorrelationId, QuickLaunchKeyboardHealthState State, int NativeErrorCode) : WindowsPlatformEvent;

internal sealed record TrayActionRequestedEvent(Guid CorrelationId, string Action) : WindowsPlatformEvent;

internal sealed record ForegroundChangedEvent(ForegroundContext Context) : WindowsPlatformEvent;

internal sealed record GlobalPointerPressedEvent(
    long WindowHandle,
    long RootWindowHandle,
    uint ProcessId,
    long SampledAtUnixMs = 0,
    ulong EventSequence = 0) : WindowsPlatformEvent;

internal sealed record ClipboardImageData(
    string StagingPath,
    uint Width,
    uint Height);

internal sealed record ClipboardCapturedEvent(
    string? Text,
    ClipboardImageData? Image,
    IReadOnlyList<string> Paths,
    string? SourceIconStagingPath,
    ForegroundContext Source,
    long CapturedAtUnixMs,
    IReadOnlyList<ClipboardNativeFormatData>? NativeFormats = null) : WindowsPlatformEvent;

internal sealed record CursorMonitorSnapshot(
    ulong DisplayId, int WorkLeft, int WorkTop, uint WorkWidth, uint WorkHeight, uint Dpi);

internal sealed record ClipboardPanelRequestedEvent(
    ForegroundContext Target,
    int CursorX,
    int CursorY,
    uint CursorDisplayIndex,
    uint CursorMonitorDpi,
    long RequestedAtUnixMs,
    ulong EventSequence = 0,
    Guid? CorrelationId = null,
    CursorMonitorSnapshot? CursorMonitor = null) : WindowsPlatformEvent;

internal sealed record ExplorerContext(
    long WindowHandle,
    string? CurrentDirectory);

internal sealed record DesktopContext(
    long WindowHandle,
    string? CurrentDirectory);

internal sealed record FileDialogContext(
    long WindowHandle,
    string? CurrentDirectory);

internal sealed record ThirdPartyFileManagerContext(
    long WindowHandle,
    string? CurrentDirectory,
    string HostContextId);

internal sealed record DirectoryNavigationContext(
    string Host,
    long WindowHandle,
    string? CurrentDirectory,
    string? HostContextId = null);

internal sealed record QuickLaunchPanelRequestedEvent(
    ForegroundContext Target,
    int CursorX,
    int CursorY,
    uint CursorDisplayIndex,
    uint CursorMonitorDpi,
    long RequestedAtUnixMs,
    DirectoryNavigationContext? DirectoryNavigationContext = null,
    Guid? CorrelationId = null,
    CursorMonitorSnapshot? CursorMonitor = null) : WindowsPlatformEvent;
internal sealed record QuickLaunchDirectoryContextResolvedEvent(
    long TargetWindowHandle,
    long RequestedAtUnixMs,
    DirectoryNavigationContext DirectoryNavigationContext) : WindowsPlatformEvent;

internal sealed record QuickLaunchAutoSwitchRequestedEvent(
    DirectoryNavigationContext Target,
    string SourceDirectory,
    long RequestedAtUnixMs) : WindowsPlatformEvent;

internal sealed record QuickLaunchFileDialogSearchHostChangedEvent(
    long WindowHandle,
    string? CurrentDirectory,
    int Left,
    int Top,
    int Width,
    int Height,
    uint MonitorDpi,
    bool Visible,
    long ChangedAtUnixMs,
    AutoQuickSwitchDirectorySource? QuickSwitchSource = null) : WindowsPlatformEvent;

internal sealed record TaskbarOverlayEditGestureFinishedEvent(long WindowHandle) : WindowsPlatformEvent;

internal sealed record ElevatedDirectoryHelperRequestedEvent(
    string PipeName,
    string AuthenticationToken) : WindowsPlatformEvent;
