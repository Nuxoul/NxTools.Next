using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardStorageDirectoryPicker
{
    public static WindowsCommandResult Pick(PickClipboardStorageDirectoryCommand command) =>
        Pick(command, CancellationToken.None);

    public static WindowsCommandResult Pick(
        PickClipboardStorageDirectoryCommand command,
        CancellationToken cancellationToken)
        => StaDialogRunner.Run(
            "NxTools clipboard storage picker",
            () => PickOnStaThread(command, cancellationToken),
            cancellationToken);

    private static WindowsCommandResult PickOnStaThread(
        PickClipboardStorageDirectoryCommand command,
        CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        try
        {
            using var dialog = new FolderBrowserDialog
            {
                Description = "选择一个空目录迁移剪贴板数据库和资源",
                InitialDirectory = Directory.Exists(command.CurrentDirectory)
                    ? command.CurrentDirectory
                    : string.Empty,
                ShowNewFolderButton = true,
                UseDescriptionForTitle = true
            };
            var result = dialog.ShowDialog() == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.SelectedPath)
                : new WindowsCommandResult(true, Data: string.Empty);
            return cancellationToken.IsCancellationRequested ? CancelledResult() : result;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return CancelledResult();
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or UnauthorizedAccessException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    private static WindowsCommandResult CancelledResult() =>
        new(false, "Windows dialog operation was cancelled.");
}
