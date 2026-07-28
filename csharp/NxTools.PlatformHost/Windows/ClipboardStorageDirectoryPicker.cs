using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardStorageDirectoryPicker
{
    public static WindowsCommandResult Pick(PickClipboardStorageDirectoryCommand command)
        => StaDialogRunner.Run(
            "NxTools clipboard storage picker",
            () => PickOnStaThread(command));

    private static WindowsCommandResult PickOnStaThread(PickClipboardStorageDirectoryCommand command)
    {
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
            return dialog.ShowDialog() == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.SelectedPath)
                : new WindowsCommandResult(true, Data: string.Empty);
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or UnauthorizedAccessException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }
}
