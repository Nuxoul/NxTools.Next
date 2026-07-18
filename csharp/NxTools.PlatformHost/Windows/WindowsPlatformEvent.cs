namespace NxTools.PlatformHost.Windows;

internal abstract record WindowsPlatformEvent;

internal sealed record ForegroundChangedEvent(ForegroundContext Context) : WindowsPlatformEvent;

internal sealed record ClipboardTextCapturedEvent(
    string Text,
    ForegroundContext Source,
    long CapturedAtUnixMs) : WindowsPlatformEvent;

internal sealed record ClipboardImageCapturedEvent(
    string StagingPath,
    uint Width,
    uint Height,
    ForegroundContext Source,
    long CapturedAtUnixMs) : WindowsPlatformEvent;

internal sealed record ClipboardFilesCapturedEvent(
    IReadOnlyList<string> Paths,
    ForegroundContext Source,
    long CapturedAtUnixMs) : WindowsPlatformEvent;

internal sealed record ClipboardPanelRequestedEvent(
    ForegroundContext Target,
    int CursorX,
    int CursorY,
    uint CursorDisplayIndex,
    uint CursorMonitorDpi,
    long RequestedAtUnixMs) : WindowsPlatformEvent;
