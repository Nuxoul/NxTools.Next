using System.Windows.Forms;
using NxTools.PlatformHost.Protocol;

namespace NxTools.PlatformHost.Windows;

internal static class ClipboardArchiveFilePicker
{
    private const string Filter = "NxTools clipboard archive (*.nxtclip)|*.nxtclip";

    public static WindowsCommandResult Pick(PickClipboardArchivePathCommand command)
        => StaDialogRunner.Run(
            "NxTools clipboard archive picker",
            () => PickOnStaThread(command));

    private static WindowsCommandResult PickOnStaThread(PickClipboardArchivePathCommand command)
    {
        try
        {
            return command.Mode.ToLowerInvariant() switch
            {
                "export" => PickExport(command.SuggestedFileName),
                "import" => PickImport(),
                _ => new WindowsCommandResult(false, $"Unsupported clipboard archive picker mode '{command.Mode}'.")
            };
        }
        catch (Exception error) when (error is InvalidOperationException or IOException or UnauthorizedAccessException)
        {
            return new WindowsCommandResult(false, error.Message);
        }
    }

    private static WindowsCommandResult PickExport(string suggestedFileName)
    {
        using var dialog = new SaveFileDialog
        {
            AddExtension = true,
            AutoUpgradeEnabled = true,
            CheckPathExists = true,
            DefaultExt = "nxtclip",
            FileName = SafeSuggestedName(suggestedFileName),
            Filter = Filter,
            OverwritePrompt = true,
            RestoreDirectory = true,
            Title = "导出剪贴板归档"
        };
        return dialog.ShowDialog() == DialogResult.OK
            ? new WindowsCommandResult(true, Data: dialog.FileName)
            : new WindowsCommandResult(true, Data: string.Empty);
    }

    private static WindowsCommandResult PickImport()
    {
        using var dialog = new OpenFileDialog
        {
            AddExtension = true,
            AutoUpgradeEnabled = true,
            CheckFileExists = true,
            CheckPathExists = true,
            DefaultExt = "nxtclip",
            Filter = Filter,
            Multiselect = false,
            RestoreDirectory = true,
            Title = "导入剪贴板归档"
        };
        return dialog.ShowDialog() == DialogResult.OK
            ? new WindowsCommandResult(true, Data: dialog.FileName)
            : new WindowsCommandResult(true, Data: string.Empty);
    }

    private static string SafeSuggestedName(string value)
    {
        var name = Path.GetFileName(value.Trim());
        return string.IsNullOrWhiteSpace(name) ? "NxTools-clipboard.nxtclip" : name;
    }
}
