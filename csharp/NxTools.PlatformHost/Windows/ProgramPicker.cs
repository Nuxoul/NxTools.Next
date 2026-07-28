using System.ComponentModel;
using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ProgramPicker
{
    private const string Filter =
        "程序 (*.exe;*.com;*.bat;*.cmd;*.ps1;*.lnk)|*.exe;*.com;*.bat;*.cmd;*.ps1;*.lnk|所有文件 (*.*)|*.*";

    public static WindowsCommandResult Pick(PickProgramCommand command) =>
        StaDialogRunner.Run("NxTools program picker", () => PickOnStaThread(command));

    private static WindowsCommandResult PickOnStaThread(PickProgramCommand command)
    {
        try
        {
            var currentPath = Environment.ExpandEnvironmentVariables(command.CurrentPath.Trim());
            var initialDirectory = Directory.Exists(currentPath)
                ? currentPath
                : Path.GetDirectoryName(currentPath) ?? string.Empty;
            using var dialog = new OpenFileDialog
            {
                AutoUpgradeEnabled = true,
                CheckFileExists = true,
                CheckPathExists = true,
                FileName = File.Exists(currentPath) ? Path.GetFileName(currentPath) : string.Empty,
                Filter = Filter,
                InitialDirectory = Directory.Exists(initialDirectory) ? initialDirectory : string.Empty,
                Multiselect = false,
                RestoreDirectory = true,
                Title = "选择命令程序"
            };
            return dialog.ShowDialog() == DialogResult.OK
                ? new WindowsCommandResult(true, Data: dialog.FileName)
                : new WindowsCommandResult(true, Data: string.Empty);
        }
        catch (Exception error) when (error is InvalidOperationException
                                      or IOException
                                      or UnauthorizedAccessException
                                      or Win32Exception
                                      or ArgumentException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }
}
