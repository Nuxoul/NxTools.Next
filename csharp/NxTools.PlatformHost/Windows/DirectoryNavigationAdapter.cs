using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal sealed record DirectoryNavigationCaptureAttempt(
    DirectoryNavigationContext? Context,
    WindowsCommandResult? Failure = null);

internal static class DirectoryNavigationAdapter
{
    internal const string ExplorerHost = "explorer";
    internal const string DesktopHost = "desktop";
    internal const string FileDialogHost = "file_dialog";
    internal const string ThirdPartyFileManagerHost =
        "third_party_file_manager";

    public static DirectoryNavigationContext? TryCapture(
        ForegroundContext foreground,
        IElevatedDirectoryNavigationChannel? elevatedChannel = null)
    {
        if (elevatedChannel is not null &&
            RequiresElevatedHelper(foreground))
        {
            return elevatedChannel.TryCapture(foreground);
        }
        var local = TryCaptureLocal(foreground);
        if (local is not null)
        {
            return local;
        }
        return elevatedChannel is not null &&
            IsPotentialElevatedHost(foreground)
            ? elevatedChannel.TryCapture(foreground)
            : null;
    }

    internal static DirectoryNavigationContext? TryCaptureLocal(
        ForegroundContext foreground) =>
        TryCaptureLocalAttempt(foreground).Context;

    internal static DirectoryNavigationCaptureAttempt TryCaptureLocalAttempt(
        ForegroundContext foreground)
    {
        var desktop = DesktopWindowAdapter.TryCapture(foreground);
        if (desktop is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    DesktopHost,
                    desktop.WindowHandle,
                    desktop.CurrentDirectory));
        }
        var explorer = ExplorerWindowAdapter.TryCapture(foreground);
        if (explorer is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    ExplorerHost,
                    explorer.WindowHandle,
                    explorer.CurrentDirectory));
        }
        var fileDialog = FileDialogWindowAdapter.TryCapture(foreground);
        if (fileDialog is not null)
        {
            return new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    FileDialogHost,
                    fileDialog.WindowHandle,
                    fileDialog.CurrentDirectory));
        }
        var thirdPartyAttempt =
            ThirdPartyFileManagerAdapter.TryCapture(foreground);
        var thirdParty = thirdPartyAttempt.Context;
        return thirdParty is null
            ? new DirectoryNavigationCaptureAttempt(
                null,
                thirdPartyAttempt.Failure)
            : new DirectoryNavigationCaptureAttempt(
                new DirectoryNavigationContext(
                    ThirdPartyFileManagerHost,
                    thirdParty.WindowHandle,
                    thirdParty.CurrentDirectory,
                    thirdParty.HostContextId));
    }

    public static WindowsCommandResult Navigate(
        NavigateDirectoryCommand command,
        IElevatedDirectoryNavigationChannel? elevatedChannel = null) =>
        ElevatedDirectoryNavigationClient.IsElevatedContext(
            command.HostContextId)
            ? elevatedChannel?.Navigate(command) ??
                new WindowsCommandResult(
                    false,
                    "The same-integrity directory helper is unavailable.")
            : NavigateLocal(command);

    internal static WindowsCommandResult NavigateLocal(
        NavigateDirectoryCommand command) =>
        command.Host switch
        {
            ExplorerHost => ExplorerWindowAdapter.Navigate(
                new NavigateExplorerCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            DesktopHost => DesktopWindowAdapter.Navigate(
                new NavigateDesktopCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            FileDialogHost => FileDialogWindowAdapter.Navigate(
                new NavigateFileDialogCommand(
                    command.WindowHandle,
                    command.TargetDirectory)),
            ThirdPartyFileManagerHost => ThirdPartyFileManagerAdapter.Navigate(
                new NavigateThirdPartyFileManagerCommand(
                    command.WindowHandle,
                    command.TargetDirectory,
                    command.HostContextId)),
            _ => new WindowsCommandResult(
                false,
                $"Unsupported directory navigation host '{command.Host}'.")
        };

    internal static bool RequiresElevatedHelper(
        ForegroundContext foreground) =>
        IsPotentialElevatedHost(foreground) &&
        ElevatedDirectoryHelperPeerVerifier.IsProcessElevated(
            foreground.ProcessId);

    internal static bool IsPotentialElevatedHost(
        ForegroundContext foreground)
    {
        var processName = Path.GetFileNameWithoutExtension(
            foreground.ProcessName);
        return processName.Equals("explorer", StringComparison.OrdinalIgnoreCase) ||
            ThirdPartyFileManagerAdapter.IsPotentialHost(foreground) ||
            FileDialogWindowAdapter.IsPotentialHost(foreground);
    }
}
