namespace NxTools.PlatformHost.Windows;

using NxTools.PlatformHost.Protocol;

internal abstract record WindowsPlatformEvent;

internal sealed record ForegroundChangedEvent(ForegroundContext Context) : WindowsPlatformEvent;

internal sealed record GlobalPointerPressedEvent(
    long WindowHandle,
    long RootWindowHandle,
    uint ProcessId) : WindowsPlatformEvent;

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

internal sealed record ClipboardPanelRequestedEvent(
    ForegroundContext Target,
    int CursorX,
    int CursorY,
    uint CursorDisplayIndex,
    uint CursorMonitorDpi,
    long RequestedAtUnixMs) : WindowsPlatformEvent;

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
    DirectoryNavigationContext? DirectoryNavigationContext = null) : WindowsPlatformEvent;

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
