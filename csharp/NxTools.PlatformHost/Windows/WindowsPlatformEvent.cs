namespace NxTools.PlatformHost.Windows;

internal abstract record WindowsPlatformEvent;

internal sealed record ForegroundChangedEvent(ForegroundContext Context) : WindowsPlatformEvent;

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
    long CapturedAtUnixMs) : WindowsPlatformEvent;

internal sealed record ClipboardPanelRequestedEvent(
    ForegroundContext Target,
    int CursorX,
    int CursorY,
    uint CursorDisplayIndex,
    uint CursorMonitorDpi,
    long RequestedAtUnixMs) : WindowsPlatformEvent;
