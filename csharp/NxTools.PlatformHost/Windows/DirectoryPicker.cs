using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class DirectoryPicker
{
    public static WindowsCommandResult Pick(PickDirectoryCommand command) =>
        StaDialogRunner.Run("NxTools directory picker", () => PickOnStaThread(command));

    private static WindowsCommandResult PickOnStaThread(PickDirectoryCommand command)
    {
        try
        {
            using var dialog = new FolderBrowserDialog
            {
                Description = string.IsNullOrWhiteSpace(command.Title)
                    ? "选择目录"
                    : command.Title.Trim(),
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
        catch (Exception error) when (error is InvalidOperationException
                                      or IOException
                                      or UnauthorizedAccessException
                                      or Win32Exception)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }
}
